//! Publication consumes trusted verified compiler exports, without replaying producers.
use lctx_compiler_store::artifact::VerifiedExport;
use lctx_model::domain::{
    ModelError,
    completion::{Completion, RemoteState, StorageState, complete},
    serving::SnapshotHandle,
};
use lctx_native::RuntimeConfig;
mod native_publication;
use native_publication::{Admission, Publication};
/// Independently compare the installed current executable definitions without DDL.
pub async fn check_definitions(
    config: &RuntimeConfig,
    blueprint: &str,
) -> Result<lctx_model::domain::ContentHash, ModelError> {
    let client = lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let result = async {
        lctx_native::control::check_installation(&client, config.service_generation).await?;
        let loader = lctx_native::Loader::new(client.clone());
        crate::definitions::verify_epoch(&loader, blueprint).await
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "definition verification session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    complete(result, completion)
}
/// Install additive immutable executable definitions under explicit root maintenance authority.
/// Normal compilation/publication only verifies this epoch and cannot create schema or users.
pub async fn install_definitions(
    config: &RuntimeConfig,
    blueprint: &str,
) -> Result<(), ModelError> {
    if config.authentication != lctx_native::AuthenticationScope::Root {
        return Err(ModelError::Conflict(
            "definition installation requires root maintenance authority",
        ));
    }
    let client = lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let result = async {
        lctx_native::control::check_installation(&client, config.service_generation).await?;
        let loader = lctx_native::Loader::new(client.clone());
        crate::definitions::install_epoch(&loader, blueprint)
            .await
            .map(|_| ())
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "definition installation session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    complete(result, completion)
}
pub async fn publish(
    export: &VerifiedExport,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let native = export.native();
    let mut publication =
        match Publication::new(Admission::Export(export), config, native_definitions).await {
            Ok(publication) => publication,
            Err(error) => {
                let result = Err(error);
                let completion = publication_completion(native, &result).await;
                return complete(result, completion);
            }
        };
    let result = async {
        if publication.is_unchanged() {
            return publication.seal(config, native_definitions).await;
        }
        publication.materialize_references().await?;
        publication.materialize_search().await?;
        publication.seal(config, native_definitions).await
    }
    .await;
    let mut completion = Completion::default();
    if let Err(error) = &result {
        completion.step(
            "publication session invalidation",
            publication.invalidate(error).await,
        );
    }
    let result = complete(result, completion);
    let completion = publication_completion(native, &result).await;
    complete(result, completion)
}
/// Ordinary compilation retains its admitted native authority; no portable self-import.
pub async fn seal_completed(
    artifact: &lctx_compiler_store::artifact::AdmittedArtifact,
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let native = artifact.native();
    let mut publication =
        match Publication::new(Admission::Completed(artifact), config, native_definitions).await {
            Ok(publication) => publication,
            Err(error) => {
                let result = Err(error);
                let completion = publication_completion(native, &result).await;
                return complete(result, completion);
            }
        };
    let result = async {
        if publication.is_unchanged() {
            return publication.seal(config, native_definitions).await;
        }
        publication.materialize_references().await?;
        publication.materialize_search().await?;
        publication.seal(config, native_definitions).await
    }
    .await;
    let mut completion = Completion::default();
    if let Err(error) = &result {
        completion.step(
            "publication session invalidation",
            publication.invalidate(error).await,
        );
    }
    let result = complete(result, completion);
    let completion = publication_completion(native, &result).await;
    complete(result, completion)
}
async fn publication_completion<T>(
    native: &std::sync::Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    result: &Result<T, ModelError>,
) -> Completion {
    let mut completion = Completion::default();
    if let Err(error) = result {
        native.fail();
        completion = native.drain_report().await;
        // Drainage can recover an acknowledged marker that cancellation or a
        // finalizer failure kept out of the immediate error. Both owners count.
        if !error.has_committed_effect() && completion.committed.is_empty() {
            if error.permits_storage_cleanup() {
                completion.step("publication abandon", native.abandon().await);
            } else {
                completion.remote = RemoteState::Unknown;
                completion.storage.push(StorageState::Orphan(format!(
                    "native_attempt:{}",
                    native.attempt().hex()
                )));
            }
        }
    }
    completion
}

#[cfg(test)]
mod publication_completion_controls {
    use super::*;
    use lctx_compiler_store::compiler::NativeCompilerStore;
    use lctx_model::domain::{admission::Frontier, resources::ResourceBudget};
    use lctx_native::NativeReader;
    use lctx_native::surrealdb::types::{RecordId, Variables};

    #[tokio::test(flavor = "multi_thread")]
    async fn retained_publication_acknowledgement_prevents_abandon_on_confirmed_finalization_error()
    {
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let native = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::compiler::check_installation(&config)
            .await
            .unwrap();
        let reader = NativeReader::private(client.clone());
        let result = async {
            let budget = ResourceBudget::fixed(256 << 20)?;
            native.freeze_content(&budget).await?;
            let lease = native.begin_finalization().await?;
            let identity = native.attempt().hex();
            // Model the confirmed failure after the marker acknowledgement has
            // reached its real lease owner, before the caller receives metadata.
            lease.record_committed(identity.clone());
            lease.finish_with(Ok::<(), ModelError>(()))?;
            let failed: Result<(), ModelError> = Err(ModelError::Conflict(
                "controlled confirmed post-marker close failure",
            ));
            assert!(!failed.as_ref().unwrap_err().has_committed_effect());
            let completion = publication_completion(&native, &failed).await;
            let mut variables = Variables::new();
            variables.insert(
                "attempt",
                RecordId::new("native_attempt", native.attempt().hex()),
            );
            let states: Vec<String> = reader
                .query_native("SELECT VALUE state FROM $attempt", variables)
                .await?;
            assert_eq!(states, ["open"], "acknowledgement excludes abandonment");
            assert_eq!(
                completion.local,
                lctx_model::domain::completion::LocalState::Terminal
            );
            assert_eq!(completion.remote, RemoteState::Confirmed);
            assert_eq!(
                completion.committed,
                [lctx_model::domain::completion::CommittedEffect {
                    kind: "published unselected manifest",
                    identity,
                }]
            );
            assert!(
                completion
                    .failures
                    .iter()
                    .all(|failure| failure.step != "publication abandon")
            );
            let error = complete(failed, completion).unwrap_err();
            assert!(error.has_committed_effect());
            assert!(matches!(
                error.primary(),
                Some(ModelError::Conflict(
                    "controlled confirmed post-marker close failure",
                ))
            ));
            Ok::<_, ModelError>(())
        }
        .await;
        // This nonce attempt has no publication content; its explicit test owner
        // releases it only after observing the production finalizer's decision.
        let mut cleanup = Completion::default();
        cleanup.step(
            "controlled publication attempt cleanup",
            native.abandon().await,
        );
        cleanup.step("controlled publication reader close", reader.close().await);
        cleanup.step(
            "controlled publication session invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        complete(result, cleanup).unwrap();
    }
}

mod search;
pub use search::{materialize_search, reconcile_search};

pub mod backup;
mod backup_import;
mod owned_read;
mod restore;
mod selected_backup;

mod definitions;
pub(crate) use definitions::verify_realization;

pub mod inspection;
