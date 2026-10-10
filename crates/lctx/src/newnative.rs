//! Explicit native operator actions and fixed-snapshot viewer journeys.
use anyhow::Context as _;
use lctx_model::domain::serving::{ResourceLimits, SnapshotHandle};
use lctx_surrealdb::surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{SerdeWrapper, SurrealValue},
};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
use std::{path::Path, sync::Arc, time::Duration};

pub fn default_config() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG") {
        return path.into();
    }
    if let Some(path) = std::env::var_os("LCTX_SURREAL_SERVICE_CONFIG") {
        return std::path::PathBuf::from(path).with_file_name("main-runtime.json");
    }
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/state")
        });
    root.join("library-context/surrealdb/main-runtime.json")
}

pub fn config(path: &Path) -> anyhow::Result<RuntimeConfig> {
    RuntimeConfig::read(path)
        .with_context(|| format!("native runtime configuration {}", path.display()))
}

/// Observe the installed generation before acquisition. This never provisions storage.
pub async fn ready(config: &RuntimeConfig) -> anyhow::Result<Arc<Surreal<Client>>> {
    tokio::time::timeout(deadline(), async {
        Ok(lctx_surrealdb::compiler::check_installation(config).await?)
    })
    .await
    .context("native runtime readiness deadline exceeded")?
}

fn deadline() -> Duration {
    Duration::from_millis(ResourceLimits::default().request_deadline_ms)
}
pub(crate) fn operation_error(error: anyhow::Error) -> lctx_model::domain::ModelError {
    match error.downcast::<lctx_model::domain::ModelError>() {
        Ok(error) => error,
        Err(error) => lctx_model::domain::ModelError::Cause(error.into_boxed_dyn_error()),
    }
}

pub async fn publish(
    path: &Path,
    config: &RuntimeConfig,
    memory_bytes: usize,
) -> anyhow::Result<SnapshotHandle> {
    let manifest =
        lctx_model::domain::graph::Manifest::decode(&std::fs::read(path.join("manifest.json"))?)?;
    let model = Arc::new(lctx_model::domain::model()?);
    let native =
        lctx_surrealdb::compiler::NativeCompilerStore::begin(config, manifest.frontier).await?;
    let workspace = match cpg_core::workspace::Workspace::new(
        model,
        cpg_core::workspace::WorkspaceOptions {
            memory_bytes,
            ..Default::default()
        },
        native.clone(),
    ) {
        Ok(workspace) => workspace,
        Err(error) => {
            native.fail();
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step("import setup abandon", native.abandon().await);
            return lctx_model::domain::completion::complete::<SnapshotHandle>(
                Err(error),
                completion,
            )
            .map_err(Into::into);
        }
    };
    let result = async {
        let export = cpg_core::artifact::verify_export(path, &workspace).await?;
        lctx_publisher::publish(&export, config, &lctx_serving::native_definitions()).await
    }
    .await;
    let mut completion = workspace.drain_report().await;
    if let Ok(handle) = &result {
        completion.committed(
            "published unselected manifest",
            serde_json::to_string(handle)?,
        );
    }
    if result.is_err() || !completion.failures.is_empty() {
        native.fail();
        if !result
            .as_ref()
            .err()
            .is_some_and(lctx_model::domain::ModelError::has_committed_effect)
            && result.is_err()
        {
            if result
                .as_ref()
                .err()
                .is_none_or(lctx_model::domain::ModelError::permits_storage_cleanup)
            {
                completion.step("import abandon", native.abandon().await);
            } else {
                completion
                    .storage
                    .push(lctx_model::domain::completion::StorageState::Orphan(
                        format!("attempt:{}", native.attempt().hex()),
                    ));
            }
        }
    }
    lctx_model::domain::completion::complete(result, completion).map_err(Into::into)
}

fn handle(config: &RuntimeConfig, path: Option<&Path>) -> anyhow::Result<SnapshotHandle> {
    match path {
        Some(path) => serde_json::from_slice(&std::fs::read(path)?)
            .with_context(|| format!("snapshot handle {}", path.display())),
        None => Ok(config.selected()?),
    }
}

pub async fn pin(config: &RuntimeConfig, path: Option<&Path>) -> anyhow::Result<NativeReader> {
    let handle = handle(config, path)?;
    Ok(tokio::time::timeout(
        deadline(),
        NativeReader::connect(&config.endpoint, &config.writer_credentials(), handle),
    )
    .await
    .context("snapshot readiness deadline exceeded")??)
}

pub async fn select(config: &RuntimeConfig, path: &Path) -> anyhow::Result<SnapshotHandle> {
    let guard = config.lock_selection().await?;
    let reader = pin(config, Some(path)).await?;
    let result = config
        .select_locked(reader.handle(), &guard)
        .map(|()| reader.handle().clone())
        .map_err(Into::into);
    finish_reader(&reader, result).await
}

pub async fn show(
    config: &RuntimeConfig,
    path: Option<&Path>,
) -> anyhow::Result<lctx_publisher::inspection::SnapshotDetails> {
    let reader = pin(config, path).await?;
    let result = lctx_publisher::inspection::show(&reader)
        .await
        .map_err(Into::into);
    finish_reader(&reader, result).await
}

pub async fn query(
    config: &RuntimeConfig,
    path: Option<&Path>,
    relation: &str,
    limit: usize,
) -> anyhow::Result<serde_json::Value> {
    if limit == 0 {
        return Err(crate::Refused("query limit must be positive".into()).into());
    }
    let model = lctx_model::domain::model()?;
    if model.relation(relation).is_none() {
        return Err(crate::Refused("query requires a model-declared relation".into()).into());
    }
    let reader = pin(config, path).await?;
    let result = tokio::time::timeout(deadline(), async {
        let mut stream = reader.relation_bodies(relation, limit)?;
        let mut rows = Vec::new();
        while let Some(value) = stream.next().await? {
            rows.push(SerdeWrapper::<serde_json::Value>::from_value(value)?.0);
        }
        Ok::<_, anyhow::Error>(serde_json::Value::Array(rows))
    })
    .await
    .context("snapshot query deadline exceeded");
    finish_reader(&reader, result.and_then(|value| value)).await
}

async fn finish_reader<T>(reader: &NativeReader, result: anyhow::Result<T>) -> anyhow::Result<T> {
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("published reader pin release", reader.close().await);
    completion.step(
        "published reader session close",
        reader
            .client()
            .invalidate()
            .await
            .map_err(lctx_model::domain::ModelError::codec),
    );
    lctx_model::domain::completion::complete(result.map_err(operation_error), completion)
        .map_err(Into::into)
}

pub async fn export(
    config: &RuntimeConfig,
    handle: Option<&Path>,
    key: lctx_model::domain::projection::normalization::ProjectionKey,
    output: &Path,
    memory_bytes: usize,
) -> anyhow::Result<()> {
    if output.exists() {
        return Err(crate::Refused("projection destination already exists".into()).into());
    }
    let reader = pin(config, handle).await?;
    let result = async {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(memory_bytes)?;
        let projection = tokio::time::timeout(
            deadline(),
            lctx_surrealdb::projections::materialize(&reader, key, &budget),
        )
        .await
        .context("projection materialization deadline exceeded")??;
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut writer = std::io::BufWriter::new(staged.as_file_mut());
            projection.write_json(&mut writer)?;
            use std::io::Write;
            writer.flush()?;
        }
        staged.as_file().sync_all()?;
        staged
            .persist_noclobber(output)
            .map_err(|error| error.error)?;
        Ok(())
    }
    .await;
    finish_reader(&reader, result).await
}

pub async fn tool(
    config: &RuntimeConfig,
    path: Option<&Path>,
    tool: &str,
    raw: &str,
) -> anyhow::Result<String> {
    let limits = config.serving_limits.clone().unwrap_or_default();
    lctx_model::domain::serving::decode_request(tool, raw, &limits)?;
    let reader = pin(config, path).await?;
    let service = lctx_serving::NativeService::new(reader.clone(), limits)?;
    let result = service
        .execute(tool, raw)
        .await
        .map_err(anyhow::Error::from);
    service.close().await;
    drop(service);
    finish_reader(&reader, result).await
}

/// Explicit maintenance installation, never called by ordinary compilation or readiness.
pub async fn install(config: &RuntimeConfig, keep_closed: bool) -> anyhow::Result<()> {
    tokio::time::timeout(deadline(), async {
        anyhow::ensure!(
            config.authentication == lctx_surrealdb::config::AuthenticationScope::Root,
            "installation requires explicit maintenance credentials"
        );
        lctx_surrealdb::compiler::install_shared(config, &lctx_serving::native_definitions())
            .await?;
        let client = lctx_surrealdb::compiler::check_installation(config).await?;
        lctx_surrealdb::NativeEmbeddingCache::install(client).await?;
        lctx_surrealdb::NativeProductCache::install(config).await?;
        lctx_publisher::install_definitions(config, &lctx_serving::native_definitions()).await?;
        if !keep_closed {
            lctx_surrealdb::compiler::open_admission(config).await?;
        }
        Ok(())
    })
    .await
    .context("native installation deadline exceeded")?
}

/// Explicit lifecycle operations never update the selected snapshot pointer.
pub async fn backup(
    config: &RuntimeConfig,
    path: Option<&Path>,
    output: &Path,
) -> anyhow::Result<()> {
    let selected = handle(config, path)?;
    lctx_publisher::backup::backup(config, &selected, output).await?;
    Ok(())
}
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    publication: Option<lctx_model::domain::ContentHash>,
) -> anyhow::Result<SnapshotHandle> {
    let definitions = lctx_serving::native_definitions();
    Ok(match publication {
        Some(publication) => {
            lctx_publisher::backup::restore_publication(config, input, publication, &definitions)
                .await?
        }
        None => lctx_publisher::backup::restore(config, input, &definitions).await?,
    })
}
pub async fn retire(
    config: &RuntimeConfig,
    path: &Path,
    readers_stopped: bool,
) -> anyhow::Result<lctx_model::domain::completed::RetirementProgress> {
    let snapshot = handle(config, Some(path))?;
    Ok(lctx_publisher::backup::retire(config, &snapshot, readers_stopped).await?)
}

pub async fn list(config: &RuntimeConfig) -> anyhow::Result<Vec<SnapshotHandle>> {
    Ok(lctx_publisher::inspection::list(config).await?)
}
pub async fn audit(config: &RuntimeConfig, path: &Path) -> anyhow::Result<SnapshotHandle> {
    let snapshot = handle(config, Some(path))?;
    lctx_publisher::inspection::audit(config, &snapshot, &lctx_serving::native_definitions())
        .await?;
    Ok(snapshot)
}
