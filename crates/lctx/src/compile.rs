//! A cumulative compile publishes one unselected generation. Configuration precedes acquisition and database effects.
use cpg_core::postgres::generations::{GenerationAttempt, GenerationStore};
use lctx_model::domain::{
    admission::{Frontier, FrontierContract},
    stages::{Profile, Schedule},
};
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
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit CLI acquisition paths and facts options form this entry point"
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
    database: Option<&Path>,
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
    let runtime =
        cpg_core::model_runtime::AttemptRuntime::new(cpg_core::model_runtime::RuntimeOptions {
            memory_bytes,
            ..Default::default()
        })?;
    let budget = runtime.budget().clone();
    let library = libraries.join(name);
    let upper = options.prepare(frontier.name(), &library)?;
    let native = cpg_extract::native_context::NativeContextConfig::committed(profile, &budget)?;
    let prepared = upper
        .as_ref()
        .map(|upper| {
            cpg_core::compilation::PreparedCompilation::new(
                frontier,
                upper.settings.clone(),
                native.catalog(),
                upper.embedder.as_deref(),
                &budget,
            )
        })
        .transpose()?;
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
        &inventory, &budget, receipts, native,
    )?);
    let model = crate::database::model()?;
    let providers = cpg_core::facts::providers::<GenerationAttempt>(configuration);
    let schedule = if let Some(prepared) = &prepared {
        prepared.schedule(&model, &providers, profile)?
    } else if frontier == Frontier::Normalized {
        cpg_core::normalize::schedule(&model, &providers, profile)?
    } else {
        Schedule::build(
            &model,
            providers.iter().map(|p| p.declaration(profile)).collect(),
            &[],
            profile,
        )?
    };
    FrontierContract::for_frontier(&model, profile, frontier)?.preflight(&schedule)?;
    let database = crate::database::Database::discover(database)?;
    let store = GenerationStore::open(database.owner().await?, model).await?;
    let importer = database.importer()?;
    let cache = if upper.as_ref().is_some_and(|u| u.embedder.is_some()) {
        Some(database.application().await?)
    } else {
        None
    };
    let writer = match database.writer().await {
        Ok(writer) => writer,
        Err(error) => {
            if let Some(cache) = &cache {
                cache.close().await;
            }
            return Err(error);
        }
    };
    let published = match frontier {
        Frontier::Analysis | Frontier::Catalog => {
            cpg_core::compilation::publish(
                &store,
                &importer,
                writer.clone(),
                captured,
                &runtime,
                profile,
                configuration,
                prepared.as_ref().expect("upper configuration"),
                upper.as_ref().and_then(|u| u.embedder.as_deref()),
                cache.clone(),
            )
            .await
        }
        Frontier::Normalized => {
            cpg_core::normalize::publish(
                &store,
                &importer,
                writer.clone(),
                captured,
                &runtime,
                profile,
                configuration,
            )
            .await
        }
        Frontier::Facts => {
            cpg_core::facts::publish(
                &store,
                writer.clone(),
                captured,
                budget.clone(),
                profile,
                configuration,
            )
            .await
        }
        Frontier::Conformance => unreachable!("not a CLI frontier"),
    };
    writer.close().await;
    if let Some(cache) = &cache {
        cache.close().await;
    }
    let published = published?;
    let report = if prepared.is_some() {
        let serving = database.serving()?;
        let model = crate::database::model()?;
        let session = cpg_core::generation_read::GenerationSession::open(
            &serving,
            model.clone(),
            published.generation,
            cpg_core::generation_read::ProviderOptions {
                connections: serving.provider_connections,
                ..Default::default()
            },
        )
        .await?;
        Some(cpg_core::analysis_report::read(session, &model).await?)
    } else {
        None
    };
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"generation":published.generation.hex(),"frontier":frontier.name(),"profile":profile.name(),"content_digest":published.content.hex(),"selected":false,"analysis":report,"stage_measurements":published.measurements,"rss_sampling_interval_ms":20,"families":published.availability.iter().map(|(family,availability)|serde_json::json!({"family":format!("{family:?}"),"availability":format!("{availability:?}")})).collect::<Vec<_>>(),"peak_reservation_bytes":budget.peak()})
        )?
    );
    Ok(())
}
