//! Trusted local logical transport. Imported metadata never becomes a serving realization.
use lctx_model::domain::{
    ModelError,
    completion::{Completion, RemoteState, complete},
    serving::SnapshotHandle,
};
use lctx_surrealdb::surrealdb::types::RecordId;
use lctx_surrealdb::{NativeReader, RuntimeConfig};
use std::path::Path;

/// Write a logical dump after the gRPC file export consumes successful terminal completion.
/// Database users/access credentials and historical versions are excluded.
/// A parent-directory sync failure after publication leaves the dump in place and reports
/// uncertain durability; callers must inspect the destination before retrying.
pub async fn backup(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    output: &Path,
) -> Result<(), ModelError> {
    if handle.database.namespace != config.namespace
        || handle.database.database != config.database
        || handle.service_generation != config.service_generation
    {
        return Err(ModelError::Conflict("backup installation identity"));
    }
    if output.exists() {
        return Err(ModelError::Conflict("backup destination already exists"));
    }
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await?;
    let client = viewer.shared_client();
    let staged = match tempfile::NamedTempFile::new_in(parent) {
        Ok(file) => file,
        Err(error) => {
            let mut completion = Completion::default();
            completion.step("backup setup reader pin release", viewer.close().await);
            completion.step(
                "backup setup session invalidation",
                client.invalidate().await.map_err(ModelError::codec),
            );
            return complete(Err(ModelError::codec(error)), completion);
        }
    };
    // The retained publication pin protects the requested manifest. SurrealDB exports all
    // tables through one read transaction; RocksDB snapshot reads retain even concurrently
    // retired rows. Ordinary logical export needs no database-wide retirement barrier.
    let result = async {
        let metadata = recovery_metadata(config, handle)?;
        export_main(&client, staged.path()).await?;
        // Comments carry references and obligations only. The data-only decoder never grants
        // authority from them or imports live attempts, credentials, pins or executable code.
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(staged.path())
            .map_err(ModelError::codec)?;
        writeln!(
            file,
            "\n-- lctx-backup-recovery: {}",
            serde_json::to_string(&metadata).map_err(ModelError::codec)?
        )
        .map_err(ModelError::codec)
    }
    .await;
    let mut completion = Completion::default();
    completion.step("backup retained manifest pin release", viewer.close().await);
    completion.step(
        "backup export session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    complete_backup(
        staged,
        output,
        result,
        complete(Ok(()), completion),
        |parent| std::fs::File::open(parent)?.sync_all(),
    )
}

fn recovery_metadata(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
) -> Result<serde_json::Value, ModelError> {
    let installation = std::env::var_os("LCTX_SURREAL_SERVICE_CONFIG")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_STATE_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|home| std::path::PathBuf::from(home).join(".local/state"))
                })
                .map(|root| root.join("library-context/surrealdb/installation.json"))
        });
    let mut protected = serde_json::Value::Null;
    if let Some(path) = installation.filter(|path| path.exists()) {
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).map_err(ModelError::codec)?)
                .map_err(ModelError::codec)?;
        if record.get("grpc_endpoint") == Some(&serde_json::json!(config.endpoint))
            && record.get("namespace") == Some(&serde_json::json!(config.namespace.as_str()))
            && record.get("service_generation")
                == Some(
                    &serde_json::to_value(config.service_generation).map_err(ModelError::codec)?,
                )
        {
            if let Some(pointer) = record.get("recovery").filter(|pointer| pointer.is_object()) {
                let mut fields = serde_json::Map::new();
                for name in [
                    "schema",
                    "archive",
                    "sha256",
                    "installation_id",
                    "service_generation",
                    "native_schemas",
                    "obligations",
                ] {
                    if let Some(value) = pointer.get(name) {
                        fields.insert(name.into(), value.clone());
                    }
                }
                protected = serde_json::Value::Object(fields);
            }
        }
    }
    Ok(
        serde_json::json!({"schema":1,"kind":"logical-content","publication":handle.publication,"view":handle.view,"definition_epoch":handle.definition_epoch,"service_generation":config.service_generation,"namespace":config.namespace,"database":config.database,"protected_recovery_asset":protected,
        "obligations":["logical content restore performs independent admission and never restores live runtime authority","whole-service recovery requires a separately protected cold archive, pinned executables, private configuration and credentials","a referenced cold archive is an independent checkpoint, not a snapshot of this later logical export","reconcile durable effects, attempts, pins and retirement records under maintenance before reopening a recovered service","selection, serving receipts and canonical coordination must be restored only by their owning installation"]}),
    )
}

async fn export_main(
    client: &lctx_surrealdb::surrealdb::Surreal<
        lctx_surrealdb::surrealdb::engine::remote::grpc::Client,
    >,
    output: &Path,
) -> Result<(), ModelError> {
    let tables = [
        "entity",
        "assertion",
        "participant",
        "reference",
        "external",
        "entity_anchor",
        "assertion_anchor",
        "compiler_view_member",
        "original",
        "original_chunk",
        "publication",
        "compiler_contribution",
        "compiler_membership",
        "compiler_view",
        "compiler_record",
        "compiler_binding",
        "compiler_alias",
    ]
    .map(str::to_owned)
    .to_vec();
    client
        .export(output)
        .with_config()
        .users(false)
        .accesses(false)
        .versions(false)
        .params(false)
        .functions(false)
        .analyzers(false)
        .apis(false)
        .buckets(false)
        .modules(false)
        .configs(false)
        .tables(tables)
        .await
        .map_err(ModelError::codec)
}

fn complete_backup(
    staged: tempfile::NamedTempFile,
    output: &Path,
    result: Result<(), ModelError>,
    drained: Result<(), ModelError>,
    sync_parent: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), ModelError> {
    let mut completion = Completion::default();
    completion.step("backup export session invalidation", drained);
    let result = complete(result, completion);
    if result.is_err() {
        let mut completion = Completion::default();
        let identity = staged.path().display().to_string();
        completion.cleanup(
            identity,
            staged
                .close()
                .map_err(|error| ModelError::Cause(Box::new(error))),
        );
        return complete(result, completion);
    }
    if let Err(error) = staged.as_file().sync_all() {
        let mut completion = Completion::default();
        let identity = staged.path().display().to_string();
        completion.cleanup(
            identity,
            staged
                .close()
                .map_err(|error| ModelError::Cause(Box::new(error))),
        );
        return complete(Err(ModelError::Cause(Box::new(error))), completion);
    }
    if let Err(error) = staged.persist_noclobber(output) {
        let mut completion = Completion::default();
        let identity = error.file.path().display().to_string();
        completion.cleanup(
            identity,
            error
                .file
                .close()
                .map_err(|error| ModelError::Cause(Box::new(error))),
        );
        return complete(Err(ModelError::Cause(Box::new(error.error))), completion);
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut completion = Completion::default();
    completion.committed("published backup", output.display().to_string());
    let result = sync_parent(parent).map_err(|error| ModelError::Cause(Box::new(error)));
    if result.is_err() {
        completion.remote = RemoteState::Unknown;
    }
    complete(result, completion)
}

/// Restore exactly one claimed publication through current typed admission. A dump with
/// multiple publications requires `restore_publication`; no imported runtime authority survives.
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    crate::restore::restore(config, input, None, native_definitions).await
}
pub async fn restore_publication(
    config: &RuntimeConfig,
    input: &Path,
    publication: lctx_model::domain::ContentHash,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    crate::restore::restore(config, input, Some(publication), native_definitions).await
}

/// Retire an explicitly named, unselected manifest and a bounded reachable closure.
/// Native owner holds preserve content shared by other publications and active readers.
pub async fn retire(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    readers_stopped: bool,
) -> Result<lctx_model::domain::completed::RetirementProgress, ModelError> {
    if !readers_stopped {
        return Err(ModelError::Invalid(
            "retirement requires stopped readers".into(),
        ));
    }
    if handle.database.namespace != config.namespace {
        return Err(ModelError::Conflict("retirement namespace"));
    }
    if handle.database.database != config.database
        || handle.service_generation != config.service_generation
    {
        return Err(ModelError::Conflict("retirement installation identity"));
    }
    let _guard = config.lock_selection().await?;
    if config.selection.try_exists().map_err(ModelError::codec)? && config.selected()? == *handle {
        return Err(ModelError::Conflict("cannot retire selected publication"));
    }
    let client = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let result = async {
        let progress = lctx_surrealdb::control::retire_reachable(
            &client,
            vec![RecordId::new("publication", handle.publication.hex())],
            4096,
        )
        .await?;
        if progress.retired == 0 && !progress.retained.is_empty() {
            return Err(ModelError::Conflict(
                "publication remains reachable or pinned",
            ));
        }
        Ok(progress)
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "retirement session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    complete(result, completion)
}

#[cfg(test)]
#[path = "../tests/fixtures/grpc_export.rs"]
mod grpc_export_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn backup_sdk_file_export_faults_never_publish_provisional_bytes() {
        use super::grpc_export_fixture::{Fault, Fixture, PARTIAL};
        use lctx_surrealdb::Credentials;
        for fault in [
            Fault::LateEngineError,
            Fault::LateTaskError,
            Fault::MissingTrailer,
            Fault::ByteCountMismatch,
            Fault::TransportClose,
        ] {
            let fixture = Fixture::start(fault).await;
            let scratch = tempfile::tempdir().unwrap();
            let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
            let provisional = staged.path().to_owned();
            let output = scratch.path().join("snapshot.surql");
            let client = lctx_surrealdb::reader::connect(
                &fixture.endpoint,
                &Credentials::Root {
                    username: "fixture".into(),
                    password: "fixture".into(),
                },
                "injected_export",
                "fixture",
            )
            .await
            .unwrap();
            let exporting = client.clone();
            let path = provisional.clone();
            let mut export =
                tokio::spawn(
                    async move { exporting.export(&path).await.map_err(ModelError::codec) },
                );
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if tokio::fs::read(&provisional).await.unwrap() == PARTIAL {
                        break;
                    }
                    assert!(
                        !export.is_finished(),
                        "{fault:?} must be injected after actual partial file bytes"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            assert!(!export.is_finished());
            if matches!(fault, Fault::TransportClose) {
                fixture.disconnect();
            } else {
                fixture.release_terminal();
            }
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), &mut export)
                .await
                .unwrap()
                .unwrap();
            assert!(result.is_err(), "real SDK must refuse {fault:?}");
            assert_eq!(tokio::fs::read(&provisional).await.unwrap(), PARTIAL);
            let drained = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                client.invalidate().await
            })
            .await
            .unwrap()
            .map_err(ModelError::codec);
            drop(client);
            assert!(
                complete_backup(staged, &output, result, drained, |_| panic!(
                    "failed protocol export must not publish"
                ))
                .is_err()
            );
            assert!(!output.exists(), "{fault:?} left a completed destination");
            assert!(
                !provisional.exists(),
                "{fault:?} retained provisional bytes"
            );
            fixture.close().await;
        }
    }

    #[tokio::test]
    async fn backup_sdk_destination_write_failure_discards_provisional_path() {
        use super::grpc_export_fixture::{Fault, Fixture};
        use lctx_surrealdb::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        // The SDK follows this owned symlink and encounters actual ENOSPC while writing.
        // /dev/full is a device, not operator data; no production injection hook is involved.
        std::fs::remove_file(&provisional).unwrap();
        std::os::unix::fs::symlink("/dev/full", &provisional).unwrap();
        let client = lctx_surrealdb::reader::connect(
            &fixture.endpoint,
            &Credentials::Root {
                username: "fixture".into(),
                password: "fixture".into(),
            },
            "injected_export",
            "fixture",
        )
        .await
        .unwrap();
        fixture.release_terminal();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            client.export(&provisional).await
        })
        .await
        .unwrap()
        .map_err(ModelError::codec);
        assert!(result.is_err(), "actual SDK destination write must fail");
        let drained = client
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error)));
        drop(client);
        assert!(
            complete_backup(staged, &output, result, drained, |_| panic!(
                "failed destination write must not publish"
            ))
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
        fixture.close().await;
    }

    #[tokio::test]
    async fn backup_sdk_terminal_success_publishes_exact_completed_bytes() {
        use super::grpc_export_fixture::{Fault, Fixture, PARTIAL};
        use lctx_surrealdb::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let output = scratch.path().join("snapshot.surql");
        let client = lctx_surrealdb::reader::connect(
            &fixture.endpoint,
            &Credentials::Root {
                username: "fixture".into(),
                password: "fixture".into(),
            },
            "injected_export",
            "fixture",
        )
        .await
        .unwrap();
        fixture.release_terminal();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            client.export(staged.path()).await
        })
        .await
        .unwrap()
        .map_err(ModelError::codec);
        let drained = client
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error)));
        drop(client);
        complete_backup(staged, &output, result, drained, |parent| {
            std::fs::File::open(parent)?.sync_all()
        })
        .unwrap();
        assert_eq!(std::fs::read(output).unwrap(), PARTIAL);
        fixture.close().await;
    }

    fn partial_dump(parent: &Path) -> tempfile::NamedTempFile {
        let mut staged = tempfile::NamedTempFile::new_in(parent).unwrap();
        staged.write_all(b"-- partial streamed dump\n").unwrap();
        staged
    }

    #[test]
    fn backup_export_failure_discards_partial_dump_and_preserves_primary_error() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        let error = complete_backup(
            staged,
            &output,
            Err(ModelError::Codec("late export failure".into())),
            Err(ModelError::Codec("session cleanup failure".into())),
            |_| panic!("failed export must not publish or sync directory"),
        )
        .unwrap_err();
        assert!(
            matches!(error.primary(), Some(ModelError::Codec(message)) if message == "late export failure")
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_session_drain_failure_discards_completed_provisional_dump() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        assert!(
            complete_backup(
                staged,
                &output,
                Ok(()),
                Err(ModelError::Codec("session cleanup failure".into())),
                |_| panic!("failed session drain must not publish or sync directory"),
            )
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_publication_never_clobbers_a_destination_created_during_export() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        std::fs::write(&output, b"preexisting backup").unwrap();
        assert!(
            complete_backup(staged, &output, Ok(()), Ok(()), |_| {
                panic!("failed publication must not sync directory")
            })
            .is_err()
        );
        assert_eq!(std::fs::read(output).unwrap(), b"preexisting backup");
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_destination_publication_failure_discards_provisional_dump() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("missing-parent/snapshot.surql");
        assert!(
            complete_backup(staged, &output, Ok(()), Ok(()), |_| {
                panic!("failed publication must not sync directory")
            })
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_postpublication_sync_failure_retains_dump_and_reports_uncertain_durability() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        let error = complete_backup(staged, &output, Ok(()), Ok(()), |_| {
            Err(std::io::Error::other("injected directory sync failure"))
        })
        .unwrap_err();
        assert!(error.has_committed_effect());
        assert!(matches!(error.primary(), Some(ModelError::Cause(_))));
        let ModelError::Completion(outcome) = error else {
            panic!()
        };
        assert_eq!(outcome.completion.remote, RemoteState::Unknown);
        assert_eq!(
            outcome.completion.committed[0].identity,
            output.display().to_string()
        );
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"-- partial streamed dump\n"
        );
        assert!(!provisional.exists());
        assert!(
            complete_backup(
                partial_dump(scratch.path()),
                &output,
                Ok(()),
                Ok(()),
                |_| { panic!("retry must not overwrite the committed destination") }
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"-- partial streamed dump\n"
        );
    }

    #[tokio::test]
    async fn backup_grpc_file_export_on_owned_persistent_fixture() {
        use lctx_model::domain::{graph::Entity, input::Package};
        let config = RuntimeConfig::read(Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let client = lctx_surrealdb::compiler::check_installation(&config)
            .await
            .unwrap();
        let package = Package {
            name: format!(
                "canonical-backup-sentinel-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ),
        };
        let loader = lctx_surrealdb::Loader::new(client.clone());
        loader
            .entities(&[Entity::from(package.clone())])
            .await
            .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let output = scratch.path().join("snapshot.surql");
        export_main(&client, &output).await.unwrap();
        let dump = std::fs::read_to_string(&output).unwrap();
        assert!(dump.contains(&package.name));
        assert!(dump.contains("publication"));
        assert!(!dump.contains("DEFINE USER"));
        for table in [
            "native_installation",
            "native_attempt",
            "native_pin",
            "native_effect",
            "native_hold",
            "native_backup_hold",
            "native_retirement",
            "native_product",
            "native_guard",
        ] {
            assert!(
                !dump.contains(&format!("DEFINE TABLE {table} ")),
                "runtime control table exported: {table}"
            );
        }
        // The exported grammar is accepted without submitting any SQL back to the service.
        let mut parsed = crate::backup_import::DataDump::new(std::fs::File::open(&output).unwrap());
        while parsed.next().unwrap().is_some() {}
        assert!(!config.selection.exists());
        client.invalidate().await.unwrap();
    }
}
