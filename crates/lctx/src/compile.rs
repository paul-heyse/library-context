//! Cumulative graph compilation, store-free artifact export and unselected native publication.
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
    Artifact(&'a Path),
    Native(&'a Path),
}
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit CLI captures, configuration and destination"
)]
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
    if name.is_empty()
        || Path::new(name).components().count() != 1
        || !matches!(
            Path::new(name).components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        anyhow::bail!("library name must be one normalized path component");
    }
    let runtime = match &target {
        Target::Artifact(_) => None,
        Target::Native(path) => Some(crate::newnative::config(path)?),
    };
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace = cpg_core::workspace::Workspace::new(
        model,
        cpg_core::workspace::WorkspaceOptions {
            memory_bytes,
            ..Default::default()
        },
    )?;
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
    let native_client = match &runtime {
        Some(config) => Some(crate::newnative::ready(config).await?),
        None => None,
    };
    let cache = match native_client {
        Some(client) if upper.as_ref().is_some_and(|upper| upper.embedder.is_some()) => Some(
            Arc::new(lctx_surrealdb::NativeEmbeddingCache::install(client).await?)
                as Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>,
        ),
        _ => None,
    };
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
    match target {
        Target::Artifact(destination) => {
            artifact.export(destination)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "artifact":destination, "frontier":frontier.name(), "profile":profile.name(),
                    "content":artifact.manifest().content().hex(), "published":false,
                }))?
            );
        }
        Target::Native(_) => {
            let staged = tempfile::tempdir()?;
            let destination = staged.path().join("artifact");
            artifact.export(&destination)?;
            let exported = cpg_core::artifact::verify_export(&destination, &workspace).await?;
            let handle = lctx_publisher::publish(
                &exported,
                runtime.as_ref().expect("native target configuration"),
                &lctx_serving::native_definitions(),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&handle)?);
        }
    }
    Ok(())
}
