//! Publication consumes trusted verified compiler exports, without replaying producers.
use cpg_core::artifact::VerifiedExport;
use futures::TryStreamExt;
use lctx_model::domain::{
    KeySink, ModelError,
    completion::{Completion, RemoteState, StorageState, complete},
    serving::{Name, SnapshotHandle},
};
use lctx_surrealdb::{Loader, RuntimeConfig, reader};
mod native_publication;
use native_publication::{Admission, Publication};
pub async fn publish(
    export: &VerifiedExport,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let native = export.native();
    let publication = match Publication::new(Admission::Export(export), config).await {
        Ok(publication) => publication,
        Err(error) => {
            let result = Err(error);
            let completion = publication_completion(native, &result).await;
            return complete(result, completion);
        }
    };
    let result = async {
        publication.install_definitions(native_definitions).await?;
        let phase = lctx_surrealdb::phase::Phase::begin("publication_references");
        let references = async {
            let mut entities = Vec::new();
            let mut rows = export.native_entities().await?;
            while let Some(row) = rows.try_next().await? {
                entities.push(row);
                if entities.len() >= 128 {
                    publication.entity_references(&entities).await?;
                    entities.clear();
                }
            }
            publication.entity_references(&entities).await?;
            let mut assertions = Vec::new();
            let mut rows = export.native_assertions().await?;
            while let Some(row) = rows.try_next().await? {
                assertions.push(row);
                if assertions.len() >= 128 {
                    publication.assertion_references(&assertions).await?;
                    assertions.clear();
                }
            }
            publication.assertion_references(&assertions).await?;
            Ok::<(), ModelError>(())
        }
        .await;
        phase.finish_result(&references);
        references?;
        publication.materialize_search().await?;
        publication.seal(config, native_definitions).await
    }
    .await;
    let mut completion = Completion::default();
    if result.is_err() {
        completion.step(
            "private publication session invalidation",
            publication.invalidate().await,
        );
    }
    let result = complete(result, completion);
    let completion = publication_completion(native, &result).await;
    complete(result, completion)
}
/// Ordinary compilation retains its admitted native authority; no portable self-import.
pub async fn seal_completed(
    artifact: &cpg_core::artifact::AdmittedArtifact,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let native = artifact.native();
    let publication = match Publication::new(Admission::Completed(artifact), config).await {
        Ok(publication) => publication,
        Err(error) => {
            let result = Err(error);
            let completion = publication_completion(native, &result).await;
            return complete(result, completion);
        }
    };
    let result = async {
        publication.install_definitions(native_definitions).await?;
        let phase = lctx_surrealdb::phase::Phase::begin("publication_references");
        let references = async {
            let mut entities = Vec::new();
            let mut rows = artifact.entities().await?;
            while let Some(row) = rows.try_next().await? {
                entities.push(row);
                if entities.len() >= 128 {
                    publication.entity_references(&entities).await?;
                    entities.clear();
                }
            }
            publication.entity_references(&entities).await?;
            let mut assertions = Vec::new();
            let mut rows = artifact.assertions().await?;
            while let Some(row) = rows.try_next().await? {
                assertions.push(row);
                if assertions.len() >= 128 {
                    publication.assertion_references(&assertions).await?;
                    assertions.clear();
                }
            }
            publication.assertion_references(&assertions).await?;
            Ok::<(), ModelError>(())
        }
        .await;
        phase.finish_result(&references);
        references?;
        publication.materialize_search().await?;
        publication.seal(config, native_definitions).await
    }
    .await;
    let mut completion = Completion::default();
    if result.is_err() {
        completion.step(
            "private publication session invalidation",
            publication.invalidate().await,
        );
    }
    let result = complete(result, completion);
    let completion = publication_completion(native, &result).await;
    complete(result, completion)
}
async fn publication_completion<T>(
    native: &std::sync::Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    result: &Result<T, ModelError>,
) -> Completion {
    let mut completion = Completion::default();
    if let Err(error) = result {
        native.fail();
        completion = native.drain_report().await;
        if !error.has_committed_effect() {
            if error.permits_storage_cleanup() {
                completion.step("publication abandon", native.abandon().await);
            } else {
                completion.remote = RemoteState::Unknown;
                completion
                    .storage
                    .push(StorageState::Orphan(native.database().as_str().into()));
            }
        }
    }
    completion
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
    let attempt = PrivatePublication {
        loader: Loader::new(client),
        database: Name::new(database).map_err(ModelError::codec)?,
    };
    let mut creation_attempted = false;
    let setup = async {
        let version = attempt
            .loader
            .client()
            .version()
            .await
            .map_err(ModelError::codec)?
            .to_string();
        if !version.starts_with("3.3.") {
            return Err(ModelError::Invalid(
                "native realization requires reviewed SurrealDB 3.3 engine".into(),
            ));
        }
        attempt
            .loader
            .client()
            .query(format!(
                "DEFINE NAMESPACE IF NOT EXISTS `{}`",
                config.namespace.as_str()
            ))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        attempt
            .loader
            .client()
            .use_ns(config.namespace.as_str())
            .await
            .map_err(ModelError::codec)?;
        creation_attempted = true;
        attempt
            .loader
            .client()
            .query(format!(
                "DEFINE DATABASE `{}` STRICT",
                attempt.database.as_str()
            ))
            .await
            .map_err(lctx_surrealdb::loader::write_failure)?
            .check()
            .map_err(ModelError::codec)?;
        attempt
            .loader
            .client()
            .use_db(attempt.database.as_str())
            .await
            .map_err(ModelError::codec)?;
        Ok(())
    }
    .await;
    if setup.is_err() {
        let mut completion = Completion::default();
        if creation_attempted
            && setup
                .as_ref()
                .err()
                .is_some_and(|error| !error.permits_storage_cleanup())
        {
            completion.remote = RemoteState::Unknown;
            completion
                .storage
                .push(StorageState::Orphan(attempt.database.as_str().into()));
            completion.step(
                "publication setup session invalidation",
                attempt
                    .loader
                    .client()
                    .invalidate()
                    .await
                    .map_err(|error| ModelError::Cause(Box::new(error))),
            );
        } else if creation_attempted {
            completion.step("publication setup abandon", abandon(&attempt).await);
        } else {
            completion.step(
                "publication setup session invalidation",
                attempt
                    .loader
                    .client()
                    .invalidate()
                    .await
                    .map_err(|error| ModelError::Cause(Box::new(error))),
            );
        }
        complete(setup, completion)?;
    }
    Ok(attempt)
}
pub(crate) async fn abandon(attempt: &PrivatePublication) -> Result<(), ModelError> {
    let result = async {
        attempt
            .loader
            .client()
            .query(format!(
                "REMOVE DATABASE IF EXISTS `{}`",
                attempt.database.as_str()
            ))
            .await
            .map_err(lctx_surrealdb::loader::write_failure)?
            .check()
            .map_err(|error| ModelError::Cause(Box::new(error)))?;
        Ok::<(), ModelError>(())
    }
    .await;
    let mut completion = Completion::default();
    completion.cleanup(attempt.database.as_str(), result);
    completion.step(
        "publication abandon session invalidation",
        attempt
            .loader
            .client()
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error))),
    );
    complete(Ok(()), completion)
}

mod search;
pub use search::{materialize_search, reconcile_search};

pub mod backup;
mod backup_import;

mod definitions;
pub(crate) use definitions::verify_realization;

pub mod inspection;
