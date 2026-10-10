//! Publication consumes trusted verified compiler exports, without replaying producers.
use cpg_core::artifact::VerifiedExport;
use futures::TryStreamExt;
use lctx_model::domain::{
    ModelError,
    completion::{Completion, RemoteState, StorageState, complete},
    serving::SnapshotHandle,
};
use lctx_surrealdb::RuntimeConfig;
mod native_publication;
use native_publication::{Admission, Publication};
/// Install additive immutable executable definitions under explicit root maintenance authority.
/// Normal compilation/publication only verifies this epoch and cannot create schema or users.
pub async fn install_definitions(config:&RuntimeConfig,blueprint:&str)->Result<(),ModelError>{
    if config.authentication!=lctx_surrealdb::AuthenticationScope::Root {return Err(ModelError::Conflict("definition installation requires root maintenance authority"));}
    let client=lctx_surrealdb::reader::connect(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str()).await?;
    let result=async {
        lctx_surrealdb::control::check_installation(&client,config.service_generation).await?;
        let loader=lctx_surrealdb::Loader::new(client.clone());
        crate::definitions::install_epoch(&loader,blueprint).await.map(|_|())
    }.await;
    let mut completion=Completion::default();completion.step("definition installation session invalidation",client.invalidate().await.map_err(ModelError::codec));
    complete(result,completion)
}
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
            "publication session invalidation",
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
            "publication session invalidation",
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
                    .push(StorageState::Orphan(format!("native_attempt:{}",native.attempt().hex())));
            }
        }
    }
    completion
}

mod search;
pub use search::{materialize_search, reconcile_search};

pub mod backup;
mod backup_import;
mod restore;

mod definitions;
pub(crate) use definitions::verify_realization;

pub mod inspection;
