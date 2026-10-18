//! Cumulative graph compilation, explicit artifact export and unselected native publication.
use lctx_model::domain::{admission::Frontier, stages::Profile};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
pub fn profile(text: &str) -> Result<Profile, String> {
    Profile::ALL
        .into_iter()
        .find(|p| p.name() == text)
        .ok_or_else(|| format!("{text:?} is not catalog or behavioral"))
}
pub enum Target<'a> {
    Artifact(&'a Path, &'a Path),
    Native(&'a Path),
}
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit CLI captures, configuration and destination"
)]
#[tracing::instrument(target = "lctx_phase", name = "compile_cli", skip_all, fields(library = name, profile = profile.name(), frontier = frontier.name()))]
pub async fn compile(
    name: &str,
    profile: Profile,
    frontier: Frontier,
    receipts: &[PathBuf],
    memory_bytes: usize,
    options: &crate::compile_options::Options,
    libraries: &Path,
    envs: &Path,
    sources: &Path,
    target: Target<'_>,
) -> anyhow::Result<()> {
    use lctx_native::phase::{Phase, Terminal};
    let operation_phase = Phase::begin("compile_cli");
    let setup_phase = Phase::begin("cli_setup");
    let setup: anyhow::Result<_> = async {
        if name.is_empty()
            || Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            anyhow::bail!("library name must be one normalized path component");
        }
        options.validate_frontier(frontier.name())?;
        let runtime = crate::newnative::config(match &target {
            Target::Artifact(_, config) | Target::Native(config) => config,
        })?;
        crate::newnative::ready(&runtime).await?;
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(memory_bytes)?;
        let model = Arc::new(lctx_model::domain::model()?);
        let factory = lctx_compiler_store::compiler::recovery_factory(&runtime, budget.clone())?;
        let workspace = cpg_core::workspace::Workspace::private_with_budget(
            model,
            cpg_core::workspace::WorkspaceOptions {
                memory_bytes,
                ..Default::default()
            },
            budget.clone(),
        )?;
        let cache_setup: Result<(), lctx_model::domain::ModelError> = async {
            let cache = lctx_compiler_store::NativeProductCache::connect(&runtime, &factory)
                .await?
                .map(Arc::new);
            if let Err(error) = workspace.set_product_cache(cache.clone()) {
                let mut terminal = lctx_model::domain::completion::Completion::default();
                if let Some(cache) = cache {
                    if error.permits_storage_cleanup() {
                        terminal.step("rejected product cache session close", cache.close().await);
                    } else {
                        cache.retain_unknown();
                    }
                }
                return lctx_model::domain::completion::complete(Err(error), terminal);
            }
            Ok(())
        }
        .await;
        if let Err(error) = cache_setup {
            let result: Result<_, lctx_model::domain::ModelError> = Err(error);
            let completion =
                crate::newnative::finalize_compilation(&workspace, None, &result, None).await;
            return lctx_model::domain::completion::complete(result, completion)
                .map_err(Into::into);
        }
        Ok((runtime, factory, workspace))
    }
    .await;
    setup_phase.finish_result(&setup);
    let (runtime, factory, workspace) = match setup {
        Ok(tuple) => tuple,
        Err(error) => {
            operation_phase.finish(Terminal::Failed);
            return Err(error);
        }
    };

    let mut committed = None;
    let mut materialized = None;
    let result: anyhow::Result<String>=async {
    let acquisition_phase = Phase::begin("configuration_and_acquisition");
    let acquired: anyhow::Result<_>=async {
    let budget = workspace.budget();
    let library = libraries.join(name);
    let upper = options.prepare(frontier.name(), &library)?;
    let native = cpg_extract::native_context::NativeContextConfig::committed(profile, budget)?;
    let prepared = upper
        .as_ref()
        .map(|upper| {
            cpg_core::compilation::PreparedCompilation::new(
                frontier,
                upper.settings.clone(),
                native.catalog(),
                upper.embedder.as_deref(),
                budget,
            ).map(std::sync::Arc::new)
        })
        .transpose()?;
    let cache = if upper.as_ref().is_some_and(|upper| upper.embedder.is_some()) {
        Some(Arc::new(lctx_native::NativeEmbeddingCache::connect(crate::newnative::ready(&runtime).await?).await?)
            as Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>)
    } else {None};
    let mut acquired = crate::acquisition::Lease::open(&library, &envs.join(name), &sources.join(name), true, false, true)?;
    let inventory = cpg_extract::acquisition::inventory(
        &library,
        &acquired.environment,
        acquired.source.as_deref(),
    )?;
    let configuration = inventory.library.configuration;
    let captured = Arc::new(cpg_extract::acquisition::capture_receipts(
        &inventory, budget, receipts, native,
    )?);
    acquired.finish()?;
        Ok((upper,prepared,cache,captured,configuration))
    }.await;
    acquisition_phase.finish_result(&acquired);
    let (upper,prepared,cache,captured,configuration)=acquired?;

    cpg_core::compilation::compile(
        &workspace,
        captured.clone(),
        profile,
        configuration,
        frontier,
        prepared.clone(),
        upper.as_ref().and_then(|u| u.embedder.clone()),
        cache,
    )
    .await?;
    // This is the first compiler-target allocation. Private stages have already completed;
    // independently configured cache/effect sessions retain their separate real owners.
    let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(&runtime, frontier, &factory).await?;
    materialized = Some(store.clone());
    let artifact = cpg_core::artifact::materialize_and_admit(
        &workspace, &captured, frontier, profile, configuration, store,
    ).await?;
    let output=match target {
        Target::Artifact(destination,_)=>{
            let receipt=artifact.export(destination).await?;
            serde_json::to_string_pretty(&serde_json::json!({"artifact":destination,"frontier":frontier.name(),"profile":profile.name(),"content":artifact.manifest().content().hex(),"published":false,"receipt":receipt}))?
        }
        Target::Native(_)=>{
            let handle=lctx_publisher::seal_completed(&artifact,&runtime,&lctx_serving::native_definitions()).await?;
            committed=Some(format!("{handle:?}"));
            serde_json::to_string_pretty(&handle)?
        }
    };
    Ok(output)
    }.await;
    let result = result.map_err(crate::newnative::operation_error);
    let completion = crate::newnative::finalize_compilation(
        &workspace,
        materialized.as_deref(),
        &result,
        committed,
    )
    .await;
    let output = lctx_model::domain::completion::complete(result, completion);
    operation_phase.finish_result(&output);
    let output = output?;
    println!("{output}");
    Ok(())
}
