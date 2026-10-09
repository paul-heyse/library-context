//! Publication consumes trusted verified compiler exports, without replaying producers.
use cpg_core::artifact::VerifiedExport;
use futures::TryStreamExt;
use lctx_model::domain::{
    KeySink, ModelError,
    completion::{complete,Completion,RemoteState,StorageState},
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
    let mut completion=Completion::default();
    if let Err(error)=&result {
        native.fail();
        completion=native.drain_report().await;
        if !error.has_committed_effect() {
            if error.permits_storage_cleanup() {completion.step("publication abandon",native.abandon().await);}
            else {completion.remote=RemoteState::Unknown;completion.storage.push(StorageState::Orphan(native.database().as_str().into()));}
        }
    }
    complete(result,completion)
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
        loader.client().query(native_definitions).await.map_err(lctx_surrealdb::loader::write_failure)?.check().map_err(ModelError::codec)?;
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
    let mut completion=Completion::default();
    if let Err(error)=&result {
        native.fail();
        completion=native.drain_report().await;
        if !error.has_committed_effect() {
            if error.permits_storage_cleanup() {completion.step("publication abandon",native.abandon().await);}
            else {completion.remote=RemoteState::Unknown;completion.storage.push(StorageState::Orphan(native.database().as_str().into()));}
        }
    }
    complete(result,completion)
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
    let attempt=PrivatePublication {loader:Loader::new(client),database:Name::new(database).map_err(ModelError::codec)?};
    let mut creation_attempted=false;
    let setup=async {
    let version = attempt.loader.client().version().await.map_err(ModelError::codec)?.to_string();
    if !version.starts_with("3.3.") {
        return Err(ModelError::Invalid("native realization requires reviewed SurrealDB 3.3 engine".into()));
    }
    attempt.loader.client().query(format!("DEFINE NAMESPACE IF NOT EXISTS `{}`",config.namespace.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    attempt.loader.client().use_ns(config.namespace.as_str()).await.map_err(ModelError::codec)?;
        creation_attempted=true;
        attempt.loader.client().query(format!("DEFINE DATABASE `{}` STRICT",attempt.database.as_str())).await.map_err(lctx_surrealdb::loader::write_failure)?.check().map_err(ModelError::codec)?;
        attempt.loader.client().use_db(attempt.database.as_str()).await.map_err(ModelError::codec)?;
        Ok(())
    }.await;
    if setup.is_err() {
        let mut completion=Completion::default();
        if creation_attempted && setup.as_ref().err().is_some_and(|error|!error.permits_storage_cleanup()) {
            completion.remote=RemoteState::Unknown;completion.storage.push(StorageState::Orphan(attempt.database.as_str().into()));
            completion.step("publication setup session invalidation",attempt.loader.client().invalidate().await.map_err(|error|ModelError::Cause(Box::new(error))));
        } else if creation_attempted {completion.step("publication setup abandon",abandon(&attempt).await);}
        else {completion.step("publication setup session invalidation",attempt.loader.client().invalidate().await.map_err(|error|ModelError::Cause(Box::new(error))));}
        complete(setup,completion)?;
    }
    Ok(attempt)
}
pub(crate) async fn abandon(attempt: &PrivatePublication)->Result<(),ModelError> {
    let result=async {attempt
        .loader
        .client()
        .query(format!("REMOVE DATABASE IF EXISTS `{}`", attempt.database.as_str()))
        .await.map_err(lctx_surrealdb::loader::write_failure)?.check().map_err(|error|ModelError::Cause(Box::new(error)))?;Ok::<(),ModelError>(())}.await;
    let mut completion=Completion::default();
    completion.cleanup(attempt.database.as_str(),result);
    completion.step("publication abandon session invalidation",attempt.loader.client().invalidate().await.map_err(|error|ModelError::Cause(Box::new(error))));
    complete(Ok(()),completion)
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
        .map_err(lctx_surrealdb::loader::write_failure)?
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
        .map_err(lctx_surrealdb::loader::write_failure)?
        .check()
        .map_err(ModelError::codec)?;
    // The marker is committed. Session/readback failure cannot authorize deleting it.
    let mut completion=Completion::default();
    completion.committed("sealed unselected database",serde_json::to_string(&handle).map_err(ModelError::codec)?);
    completion.step("publication session invalidation",client.invalidate().await.map_err(|error|ModelError::Cause(Box::new(error))));
    let readback=NativeReader::connect(
        &config.endpoint,&config.viewer_credentials(),handle.clone(),
    ).await.map(|_|handle);
    complete(readback,completion)
}
async fn load(
    export: &VerifiedExport,
    loader: &Loader,
    native_definitions: &str,
) -> Result<(), ModelError> {
    loader.client().query(native_definitions).await.map_err(lctx_surrealdb::loader::write_failure)?.check().map_err(ModelError::codec)?;
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
    crate::materialize_search(loader).await?;
    loader.reconcile(export.manifest()).await
}
mod search;
pub use search::{materialize_search, reconcile_search};

pub mod backup;
mod backup_import;

mod definitions;
pub(crate) use definitions::verify_realization;

pub mod inspection;
