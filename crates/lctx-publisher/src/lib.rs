//! Publication consumes trusted verified compiler exports, without replaying producers.
use cpg_core::artifact::VerifiedExport;
use futures::TryStreamExt;
use lctx_model::domain::{
    KeySink, ModelError,
    serving::{DatabaseIdentity, Name, SnapshotHandle},
};
use lctx_surrealdb::surrealdb::types::{Bytes, Object, RecordId, ToSql, Value, Variables};
use lctx_surrealdb::{Loader, NativeReader, RuntimeConfig, reader};
pub async fn publish(
    export: &VerifiedExport,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let native=export.native();
    let attempt=PrivatePublication {loader:Loader::new(native.shared_client()),database:native.database().clone()};
    let result=async {
        load(export,&attempt.loader,native_definitions).await?;
        if native.completed_state().await? != export.manifest().completed_state {return Err(ModelError::Conflict("published completed state"));}
        native.end_writes().await?;
        seal(&attempt,export.manifest(),config,native_definitions).await
    }.await;
    if result.is_err() {native.fail();let drained=native.drain().await;abandon(&attempt).await?;drained?;}
    result
}
/// Ordinary compilation retains its admitted native authority; no portable self-import.
pub async fn seal_completed(
    artifact:&cpg_core::artifact::AdmittedArtifact,
    config:&RuntimeConfig,native_definitions:&str,
)->Result<SnapshotHandle,ModelError> {
    let native=artifact.native();
    let attempt=PrivatePublication {loader:Loader::new(native.shared_client()),database:native.database().clone()};
    let result=async {
        let loader=&attempt.loader;
        loader.client().query(native_definitions).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let mut entities=Vec::new();
        let mut assertions=Vec::new();
        let mut rows=artifact.entities().await?;
        while let Some(row)=rows.try_next().await? {entities.push(row);if entities.len()>=128 {loader.entity_references(&entities).await?;entities.clear();}}
        loader.entity_references(&entities).await?;
        let mut rows=artifact.assertions().await?;
        while let Some(row)=rows.try_next().await? {assertions.push(row);if assertions.len()>=128 {loader.assertion_references(&assertions).await?;assertions.clear();}}
        loader.assertion_references(&assertions).await?;
        materialize_search(loader).await?;
        loader.reconcile(artifact.manifest()).await?;
        if native.completed_state().await? != artifact.manifest().completed_state {return Err(ModelError::Conflict("sealed completed state"));}
        native.end_writes().await?;
        seal(&attempt,artifact.manifest(),config,native_definitions).await
    }.await;
    if result.is_err() {native.fail();let drained=native.drain().await;abandon(&attempt).await?;drained?;}
    result
}

pub(crate) struct PrivatePublication {
    pub(crate) loader: Loader,
    pub(crate) database: Name,
}
pub(crate) async fn begin(config: &RuntimeConfig) -> Result<PrivatePublication, ModelError> {
    let mut identity = KeySink::new("native-database-attempt/v1");
    identity.part(
        b"clock",
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(ModelError::codec)?
            .as_nanos()
            .to_le_bytes(),
    );
    identity.part(b"process", &std::process::id().to_le_bytes());
    let database = format!("snapshot_{}", identity.finish().hex());
    let client = reader::authenticated(&config.endpoint, &config.root_credentials(), None).await?;
    let version = client.version().await.map_err(ModelError::codec)?.to_string();
    if !version.starts_with("3.3.") {
        return Err(ModelError::Invalid("native realization requires reviewed SurrealDB 3.3 engine".into()));
    }
    client.query(format!("DEFINE NAMESPACE IF NOT EXISTS `{}`",config.namespace.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    client.use_ns(config.namespace.as_str()).await.map_err(ModelError::codec)?;
    client.query(format!("DEFINE DATABASE `{database}` STRICT")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    if let Err(error)=client.use_db(database.as_str()).await {
        let cleaned=client.query(format!("REMOVE DATABASE IF EXISTS `{database}`")).await.and_then(|response|response.check());
        if cleaned.is_err(){return Err(ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed,format!("setup left owned unselected database {database}")));}
        return Err(ModelError::codec(error));
    }
    Ok(PrivatePublication {
        loader: Loader::new(client),
        database: Name::new(database).map_err(ModelError::codec)?,
    })
}
pub(crate) async fn abandon(attempt: &PrivatePublication)->Result<(),ModelError> {
    let result=async {attempt
        .loader
        .client()
        .query(format!("REMOVE DATABASE IF EXISTS `{}`", attempt.database.as_str()))
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok::<(),ModelError>(())}.await;
    result.map_err(|_|ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed,
        format!("cleanup left owned unselected database {}",attempt.database.as_str())))
}
pub(crate) async fn seal(
    attempt: &PrivatePublication,
    manifest: &lctx_model::domain::graph::Manifest,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let client = attempt.loader.client();
    let realization = verify_realization(&attempt.loader, native_definitions).await?;
    let handle = SnapshotHandle {
        semantic: manifest.content(),
        realization,
        database: DatabaseIdentity {
            namespace: config.namespace.clone(),
            database: attempt.database.clone(),
        },
    };
    // Loader has no outstanding tasks and every statement has a successful final response.
    // A distinct database VIEWER is the only credential emitted for serving.
    let username = Name::new(config.viewer_username.clone()).map_err(ModelError::codec)?;
    // DEFINE USER requires a literal strand at 3.3; use the SDK SQL string codec, never raw interpolation.
    let password = Value::String(config.viewer_password.clone()).to_sql();
    client
        .query(format!(
            "DEFINE USER `{}` ON DATABASE PASSWORD {password} ROLES VIEWER",
            username.as_str()
        ))
        .await
        .map_err(|_| ModelError::Invalid("native viewer definition failed".into()))?
        .check()
        .map_err(|_| ModelError::Invalid("native viewer definition failed".into()))?;
    let mut marker = Object::new();
    marker.insert("id", RecordId::new("publication", "current"));
    marker.insert(
        "handle",
        hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?),
    );
    marker.insert(
        "manifest",
        Bytes::from(serde_json::to_vec(manifest).map_err(ModelError::codec)?),
    );
    let mut bindings = Variables::new();
    bindings.insert("marker", marker);
    client
        .query("INSERT INTO publication $marker RETURN NONE")
        .bind(bindings)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client.invalidate().await.map_err(ModelError::codec)?;
    NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await?;
    Ok(handle)
}
async fn load(
    export: &VerifiedExport,
    loader: &Loader,
    native_definitions: &str,
) -> Result<(), ModelError> {
    loader.client().query(native_definitions).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let mut entities = Vec::new();
    let mut bytes = 0;
    for row in export.entities()? {
        let row = row?;
        bytes += serde_json::to_vec(&row).map_err(ModelError::codec)?.len();
        entities.push(row);
        if entities.len() >= 128 || bytes >= 4 << 20 {
            loader.ensure_entities(&entities).await?;
            entities.clear();
            bytes = 0;
        }
    }
    loader.ensure_entities(&entities).await?;
    let mut assertions = Vec::new();
    bytes = 0;
    for row in export.assertions()? {
        let row = row?;
        bytes += serde_json::to_vec(&row).map_err(ModelError::codec)?.len();
        assertions.push(row);
        if assertions.len() >= 128 || bytes >= 4 << 20 {
            loader.ensure_assertions(&assertions).await?;
            assertions.clear();
            bytes = 0;
        }
    }
    loader.ensure_assertions(&assertions).await?;
    entities.clear();
    for row in export.entities()? {
        entities.push(row?);
        if entities.len() >= 128 {
            loader.entity_references(&entities).await?;
            entities.clear();
        }
    }
    loader.entity_references(&entities).await?;
    assertions.clear();
    for row in export.assertions()? {
        assertions.push(row?);
        if assertions.len() >= 128 {
            loader.assertion_references(&assertions).await?;
            assertions.clear();
        }
    }
    loader.assertion_references(&assertions).await?;
    for original in export.originals() {
        let (original, mut file) = original?;
        loader
            .ensure_original_stream(
                original.source.0,
                original.content,
                original.byte_len,
                &mut file,
            )
            .await?;
    }
    crate::materialize_search(loader)
        .await
        .map_err(|error| ModelError::Invalid(format!("native search materialization: {error}")))?;
    loader.reconcile(export.manifest()).await.map_err(|error| {
        ModelError::Invalid(format!("canonical publication reconciliation: {error}"))
    })
}
mod search;
pub use search::{materialize_search, reconcile_search};

pub mod backup;

mod definitions;
pub(crate) use definitions::verify_realization;

pub mod inspection;
