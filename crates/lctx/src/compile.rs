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
    use lctx_surrealdb::phase::{Phase, Terminal};
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
        let model = Arc::new(lctx_model::domain::model()?);
        let store =
            lctx_surrealdb::compiler::NativeCompilerStore::begin(&runtime, frontier).await?;
        let workspace = match cpg_core::workspace::Workspace::new(
            model,
            cpg_core::workspace::WorkspaceOptions {
                memory_bytes,
                ..Default::default()
            },
            store.clone(),
        ) {
            Ok(workspace) => workspace,
            Err(error) => {
                store.fail();
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.step("compile setup abandon", store.abandon().await);
                return lctx_model::domain::completion::complete::<_>(Err(error), completion)
                    .map_err(Into::into);
            }
        };
        workspace.set_product_cache(
            lctx_surrealdb::NativeProductCache::connect(&runtime).await?.map(Arc::new),
        )?;
        Ok((runtime, store, workspace))
    }
    .await;
    setup_phase.finish_result(&setup);
    let (runtime, store, workspace) = match setup {
        Ok(tuple) => tuple,
        Err(error) => {
            operation_phase.finish(Terminal::Failed);
            return Err(error);
        }
    };

    let artifact_target = matches!(&target, Target::Artifact(..));
    let mut committed = None;
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
            )
        })
        .transpose()?;
    let cache = if upper.as_ref().is_some_and(|upper| upper.embedder.is_some()) {
        Some(Arc::new(lctx_surrealdb::NativeEmbeddingCache::install(crate::newnative::ready(&runtime).await?).await?)
            as Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>)
    } else {None};
    let environment = envs.join(name);
    crate::acquire(&library, &environment, false)?;
    let source = crate::fetch_source(&library, &sources.join(name))?;
    let inventory = cpg_extract::acquisition::inventory(
        &library,
        &environment,
        source.as_ref().map(|(path, _)| path.as_path()),
    )?;
    let configuration = inventory.library.configuration;
    let captured = Arc::new(cpg_extract::acquisition::capture_receipts(
        &inventory, budget, receipts, native,
    )?);
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
        prepared.as_ref(),
        upper.as_ref().and_then(|u| u.embedder.as_deref()),
        cache,
    )
    .await?;
    let artifact =
        cpg_core::artifact::admit(&workspace, &captured, frontier, profile, configuration).await?;
    let output=match target {
        Target::Artifact(destination,_)=>{
            artifact.export(destination).await?;
            serde_json::to_string_pretty(&serde_json::json!({"artifact":destination,"frontier":frontier.name(),"profile":profile.name(),"content":artifact.manifest().content().hex(),"published":false}))?
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
    let mut completion = workspace.drain_report().await;
    if let Some(identity) = committed {
        completion.committed("sealed unselected database", identity);
    }
    if artifact_target || result.is_err() || !completion.failures.is_empty() {
        store.fail();
        if completion.committed.is_empty()
            && !result
                .as_ref()
                .err()
                .is_some_and(lctx_model::domain::ModelError::has_committed_effect)
        {
            if result
                .as_ref()
                .err()
                .is_none_or(lctx_model::domain::ModelError::permits_storage_cleanup)
            {
                completion.step("compile abandon", store.abandon().await);
            } else {
                completion
                    .storage
                    .push(lctx_model::domain::completion::StorageState::Orphan(
                        store.database().as_str().into(),
                    ));
            }
        }
    }
    let output = lctx_model::domain::completion::complete(result, completion);
    operation_phase.finish_result(&output);
    let output = output?;
    println!("{output}");
    Ok(())
}
