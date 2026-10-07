//! Explicit native operator actions and fixed-snapshot viewer journeys.
use anyhow::Context as _;
use lctx_model::domain::serving::{ResourceLimits, SnapshotHandle};
use lctx_surrealdb::surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{SerdeWrapper, SurrealValue, Value},
};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
use std::{path::Path, sync::Arc, time::Duration};

pub const DEFAULT_CONFIG: &str = "build/native/runtime.json";

pub fn config(path: &Path) -> anyhow::Result<RuntimeConfig> {
    RuntimeConfig::read(path)
        .with_context(|| format!("native runtime configuration {}", path.display()))
}

/// Establish version, authentication and the explicit configured control database before acquisition.
pub async fn ready(config: &RuntimeConfig) -> anyhow::Result<Arc<Surreal<Client>>> {
    tokio::time::timeout(deadline(), async {
        let client = lctx_surrealdb::reader::authenticated(
            &config.endpoint, &config.root_credentials(), None,
        ).await?;
        require_version(&client).await?;
        // Root USE can implicitly create storage. Readiness requires an installed control
        // database and must not create operator state as a side effect of a failed compile.
        require_member(&client, "INFO FOR ROOT", "namespaces", config.namespace.as_str()).await?;
        client.use_ns(config.namespace.as_str()).await?;
        require_member(&client, "INFO FOR NS", "databases", config.cache_database.as_str()).await?;
        client.use_db(config.cache_database.as_str()).await?;
        client.query("INFO FOR DB").await?.check()?;
        Ok(client)
    })
    .await
    .context("native runtime readiness deadline exceeded")?
}

async fn require_member(client:&Surreal<Client>,sql:&str,group:&str,name:&str)->anyhow::Result<()> {
    let mut response=client.query(sql).await?.check()?;
    let value:Value=response.take(0)?;
    let Value::Object(object)=value else{anyhow::bail!("native readiness inventory unavailable");};
    let Some(Value::Object(members))=object.get(group) else{anyhow::bail!("native readiness inventory unavailable");};
    anyhow::ensure!(members.contains_key(name),"native runtime storage is not installed; run store install explicitly");
    Ok(())
}

fn deadline() -> Duration {
    Duration::from_millis(ResourceLimits::default().request_deadline_ms)
}
async fn require_version(client: &Surreal<Client>) -> anyhow::Result<String> {
    let version = client.version().await?.to_string();
    anyhow::ensure!(
        version.starts_with("3.3."),
        "native runtime requires SurrealDB 3.3"
    );
    Ok(version)
}

pub async fn publish(
    path: &Path,
    config: &RuntimeConfig,
    memory_bytes: usize,
) -> anyhow::Result<SnapshotHandle> {
    let manifest:lctx_model::domain::graph::Manifest=serde_json::from_slice(&std::fs::read(path.join("manifest.json"))?)?;
    let model=Arc::new(lctx_model::domain::model()?);
    let native=lctx_surrealdb::compiler::NativeCompilerStore::begin(config,manifest.frontier).await?;
    let workspace = match cpg_core::workspace::Workspace::new(
        model,
        cpg_core::workspace::WorkspaceOptions {memory_bytes,..Default::default()},native.clone(),
    ) {Ok(workspace)=>workspace,Err(error)=>{native.fail();native.abandon().await?;return Err(error.into());}};
    let result=async {
        let export = cpg_core::artifact::verify_export(path, &workspace).await?;
        Ok(lctx_publisher::publish(&export, config, &lctx_serving::native_definitions()).await?)
    }.await;
    let drained=workspace.drain().await;
    if result.is_err() || drained.is_err() {
        native.fail();
        if native.abandon().await.is_err() {
            return Err(lctx_model::domain::ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                format!("failed import left owned unselected database {}",native.database().as_str()),
            ).into());
        }
    }
    drained?;
    result
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
        NativeReader::connect(&config.endpoint, &config.viewer_credentials(), handle),
    )
    .await
    .context("snapshot readiness deadline exceeded")??)
}

pub async fn select(config: &RuntimeConfig, path: &Path) -> anyhow::Result<SnapshotHandle> {
    let guard = config.lock_selection().await?;
    let reader = pin(config, Some(path)).await?;
    config.select_locked(reader.handle(), &guard)?;
    Ok(reader.handle().clone())
}

pub async fn query(
    config: &RuntimeConfig,
    path: Option<&Path>,
    sql: &str,
) -> anyhow::Result<serde_json::Value> {
    let reader = pin(config, path).await?;
    tokio::time::timeout(deadline(), async {
        // Only a database VIEWER is used. The checked response includes every statement, and
        // this process owns this dedicated session for the entire query.
        let mut response = reader.client().query(sql).await?.check()?;
        let mut results = Vec::with_capacity(response.num_statements());
        for index in 0..response.num_statements() {
            let value: Value = response.take(index)?;
            results.push(SerdeWrapper::<serde_json::Value>::from_value(value)?.0);
        }
        Ok(serde_json::Value::Array(results))
    })
    .await
    .context("snapshot query deadline exceeded")?
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

pub async fn tool(
    config: &RuntimeConfig,
    path: Option<&Path>,
    tool: &str,
    raw: &str,
) -> anyhow::Result<String> {
    let limits = ResourceLimits::default();
    lctx_model::domain::serving::decode_request(tool, raw, &limits)?;
    let reader = pin(config, path).await?;
    let service = lctx_serving::NativeService::new(reader, limits)?;
    Ok(service.execute(tool, raw).await?)
}

/// Explicitly initialize only the namespace/control database named by the runtime configuration.
pub async fn install(config: &RuntimeConfig) -> anyhow::Result<()> {
    tokio::time::timeout(deadline(), async {
        let client = lctx_surrealdb::reader::authenticated(
            &config.endpoint,&config.root_credentials(),None,
        ).await?;
        require_version(&client).await?;
        client.query(format!("DEFINE NAMESPACE IF NOT EXISTS `{}`",config.namespace.as_str())).await?.check()?;
        client.use_ns(config.namespace.as_str()).await?;
        client.query(format!("DEFINE DATABASE IF NOT EXISTS `{}` STRICT",config.cache_database.as_str())).await?.check()?;
        client.use_db(config.cache_database.as_str()).await?;
        lctx_surrealdb::NativeEmbeddingCache::install(client).await?;
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
pub async fn restore(config: &RuntimeConfig, input: &Path) -> anyhow::Result<SnapshotHandle> {
    Ok(lctx_publisher::backup::restore(config, input, &lctx_serving::native_definitions()).await?)
}
pub async fn retire(
    config: &RuntimeConfig,
    path: &Path,
    readers_stopped: bool,
) -> anyhow::Result<()> {
    let snapshot = handle(config, Some(path))?;
    lctx_publisher::backup::retire(config, &snapshot, readers_stopped).await?;
    Ok(())
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
