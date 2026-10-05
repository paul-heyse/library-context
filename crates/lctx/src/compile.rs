//! Store-free cumulative compilation and explicit artifact export.
use lctx_model::domain::{admission::Frontier, stages::Profile};
use std::{path::{Path, PathBuf}, sync::Arc};
pub fn profile(text: &str) -> Result<Profile, String> {
    Profile::ALL.into_iter().find(|p| p.name() == text)
        .ok_or_else(|| format!("{text:?} is not catalog or behavioral"))
}
#[allow(clippy::too_many_arguments, reason = "Explicit CLI captures, configuration and destination")]
pub async fn compile(
    name: &str, profile: Profile, frontier: Frontier, receipts: &[PathBuf], memory_bytes: usize,
    options: &crate::compile_options::Options, libraries: &Path, envs: &Path, sources: &Path,
    destination: &Path,
) -> anyhow::Result<()> {
    if name.is_empty() || Path::new(name).components().count() != 1
        || !matches!(Path::new(name).components().next(), Some(std::path::Component::Normal(_))) {
        anyhow::bail!("library name must be one normalized path component");
    }
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace = cpg_core::workspace::Workspace::new(model, cpg_core::workspace::WorkspaceOptions {
        memory_bytes, ..Default::default()
    })?;
    let budget = workspace.budget();
    let library = libraries.join(name);
    let upper = options.prepare(frontier.name(), &library)?;
    let native = cpg_extract::native_context::NativeContextConfig::committed(profile, budget)?;
    let prepared = upper.as_ref().map(|upper| cpg_core::compilation::PreparedCompilation::new(
        frontier, upper.settings.clone(), native.catalog(), upper.embedder.as_deref(), budget,
    )).transpose()?;
    let environment = envs.join(name);
    crate::acquire(&library, &environment, false)?;
    let source = crate::fetch_source(&library, &sources.join(name))?;
    let inventory = cpg_extract::acquisition::inventory(&library, &environment,
        source.as_ref().map(|(path, _)| path.as_path()))?;
    let configuration = inventory.library.configuration;
    let captured = Arc::new(cpg_extract::acquisition::capture_receipts(&inventory, budget, receipts, native)?);
    cpg_core::compilation::compile(&workspace, captured.clone(), profile, configuration, frontier,
        prepared.as_ref(), upper.as_ref().and_then(|u| u.embedder.as_deref()), None).await?;
    let artifact = cpg_core::artifact::admit(&workspace, &captured, frontier, profile, configuration).await?;
    artifact.export(destination)?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "artifact":destination, "frontier":frontier.name(), "profile":profile.name(),
        "content":artifact.manifest().content().hex(), "published":false,
    }))?);
    Ok(())
}
