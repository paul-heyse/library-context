//! Trusted local logical transport. Imported metadata never becomes a serving realization.
use lctx_model::domain::{
    ModelError,
    completed_transport::BackupReceipt,
    completion::{Completion, RemoteState, complete},
    serving::SnapshotHandle,
};
use lctx_native::surrealdb::types::RecordId;
use lctx_native::{NativeReader, RuntimeConfig};
use std::path::Path;

/// Write only the exact recoverable closure through one transaction-bound native snapshot.
/// Staged bytes become visible only after checked EOF, explicit cancel and session invalidation.
pub async fn backup(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    output: &Path,
    native_definitions: &str,
) -> Result<BackupReceipt, ModelError> {
    if handle.database.namespace != config.namespace
        || handle.database.database != config.database
        || handle.service_generation != config.service_generation
    {
        return Err(ModelError::Conflict("backup installation identity"));
    }
    if output.exists() {
        return Err(ModelError::Conflict("backup destination already exists"));
    }
    let config = config.clone();
    let handle = handle.clone();
    let output = output.to_owned();
    let definitions = native_definitions.to_owned();
    // Caller cancellation transfers the whole read/export finalizer to its owned driver.
    crate::owned_read::run("selected backup", move |cancel| async move {
        backup_owned(&config, &handle, &output, &definitions, &cancel).await
    })
    .await
}
async fn backup_owned(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    output: &Path,
    native_definitions: &str,
    cancel: &crate::owned_read::Cancellation,
) -> Result<BackupReceipt, ModelError> {
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
    let mut pin_guard = viewer.protect_terminal_close();
    let staged = match tempfile::NamedTempFile::new_in(parent) {
        Ok(file) => file,
        Err(error) => {
            let mut completion = Completion::default();
            completion.step(
                "backup setup reader pin release",
                pin_guard.close(&viewer).await,
            );
            return complete(Err(ModelError::codec(error)), completion);
        }
    };
    let mut transaction = None;
    let mut exporter = None;
    let mut sources = None;
    let result = async {
        cancel.check()?;
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(
            lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
        )?;
        let completed =
            lctx_native::completed_state::published_completed_state(&viewer, &budget).await?;
        let metadata = recovery_metadata(config, handle)?;
        // One dedicated exporter session, with one external transaction across all metadata,
        // closure and content reads. There is no independent export or per-page snapshot.
        exporter = Some(
            lctx_native::reader::connect(
                &config.endpoint,
                &config.writer_credentials(),
                config.namespace.as_str(),
                config.database.as_str(),
            )
            .await?,
        );
        cancel.check()?;
        let version = exporter
            .as_ref()
            .expect("owned exporter")
            .version()
            .await
            .map_err(ModelError::codec)?
            .to_string();
        cancel.check()?;
        let session = match std::sync::Arc::try_unwrap(exporter.take().expect("owned exporter")) {
            Ok(session) => session,
            Err(session) => {
                exporter = Some(session);
                return Err(ModelError::Conflict("backup export session ownership"));
            }
        };
        let snapshot = match session.begin().retain_on_error().await {
            Ok(snapshot) => std::sync::Arc::new(snapshot),
            Err((error, session)) => {
                let mut completion = Completion::default();
                let invalidated = session.invalidate().await.map_err(ModelError::codec);
                if invalidated.is_err() {
                    viewer.retain_unknown();
                    completion.remote = RemoteState::Unknown;
                }
                completion.step("backup failed-begin session invalidation", invalidated);
                return complete(Err(ModelError::codec(error)), completion);
            }
        };
        transaction = Some(snapshot.clone());
        sources = Some(tempfile::tempdir().map_err(ModelError::codec)?);
        let directory = sources.as_ref().expect("owned backup source spools");
        let sql = directory.path().join("claims.sql");
        let state = directory.path().join("completed-state");
        std::fs::File::create(&sql).map_err(ModelError::codec)?;
        let (manifest, state_receipt) = crate::selected_backup::export(
            &crate::selected_backup::Snapshot {
                transaction: &snapshot,
                cancel,
            },
            &completed,
            &version,
            handle,
            native_definitions,
            crate::selected_backup::BackupSpools {
                claims: &sql,
                completed_state: &state,
            },
            &budget,
        )
        .await?;
        cancel.check()?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&sql)
            .map_err(ModelError::codec)?;
        writeln!(
            file,
            "\n-- lctx-backup-recovery: {}",
            serde_json::to_string(&metadata).map_err(ModelError::codec)?
        )
        .map_err(ModelError::codec)?;
        file.flush().map_err(ModelError::codec)?;
        drop(file);
        let carrier = crate::backup_import::write_carrier(&sql, &state, staged.path(), &budget)?;
        let receipt = BackupReceipt {
            recipe: 1,
            source: handle.clone(),
            manifest,
            definitions: lctx_model::domain::ContentHash::of(native_definitions.as_bytes()),
            state: state_receipt,
            carrier,
        };
        receipt.validate()?;
        Ok(receipt)
    }
    .await;
    let mut completion = Completion::default();
    if let Some(session) = exporter {
        let invalidated = session.invalidate().await.map_err(ModelError::codec);
        if invalidated.is_err() {
            viewer.retain_unknown();
            completion.remote = RemoteState::Unknown;
        }
        completion.step(
            "backup pre-begin exporter session invalidation",
            invalidated,
        );
    }
    if let Some(transaction) = transaction {
        let cancelled = transaction.cancel_ref().await.map_err(ModelError::codec);
        if cancelled.is_err() {
            viewer.retain_unknown();
            completion.remote = RemoteState::Unknown;
        }
        completion.step("backup snapshot explicit cancellation", cancelled);
        let invalidated = transaction
            .invalidate_session()
            .await
            .map_err(ModelError::codec);
        if invalidated.is_err() {
            viewer.retain_unknown();
            completion.remote = RemoteState::Unknown;
        }
        completion.step("backup snapshot session invalidation", invalidated);
    }
    completion.step(
        "backup retained manifest pin release",
        pin_guard.close(&viewer).await,
    );
    if let Some(sources) = sources {
        completion.cleanup(
            sources.path().display().to_string(),
            sources.close().map_err(ModelError::codec),
        );
    }
    let mut receipt = None;
    let result = result.and_then(|value| {
        cancel.check().map(|()| {
            receipt = Some(value);
        })
    });
    complete_backup(
        staged,
        output,
        result,
        complete(Ok(()), completion),
        |parent| std::fs::File::open(parent)?.sync_all(),
    )?;
    receipt.ok_or(ModelError::Conflict(
        "backup checked completion receipt absent",
    ))
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
            && let Some(pointer) = record.get("recovery").filter(|pointer| pointer.is_object())
        {
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
    Ok(
        serde_json::json!({"schema":1,"kind":"logical-content","publication":handle.publication,"view":handle.view,"definition_epoch":handle.definition_epoch,"service_generation":config.service_generation,"namespace":config.namespace,"database":config.database,"protected_recovery_asset":protected,
        "obligations":["logical content restore performs independent admission and never restores live runtime authority","whole-service recovery requires a separately protected cold archive, pinned executables, private configuration and credentials","a referenced cold archive is an independent checkpoint, not a snapshot of this later logical export","reconcile durable effects, attempts, pins and retirement records under maintenance before reopening a recovered service","selection, serving receipts and canonical coordination must be restored only by their owning installation"]}),
    )
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

/// Restore the exact publication bound by an independently retained backup receipt.
/// Incoming claims earn fresh typed admission; imported metadata grants no runtime authority.
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    native_definitions: &str,
    expected: &BackupReceipt,
) -> Result<SnapshotHandle, ModelError> {
    crate::restore::restore(config, input, None, native_definitions, expected).await
}
pub async fn restore_publication(
    config: &RuntimeConfig,
    input: &Path,
    publication: lctx_model::domain::ContentHash,
    native_definitions: &str,
    expected: &BackupReceipt,
) -> Result<SnapshotHandle, ModelError> {
    crate::restore::restore(
        config,
        input,
        Some(publication),
        native_definitions,
        expected,
    )
    .await
}

/// Retire an explicitly named, unselected manifest and a bounded reachable closure.
/// Native owner holds preserve content shared by other publications and active readers.
pub async fn retire(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    readers_stopped: bool,
) -> Result<lctx_model::domain::completed::RetirementProgress, ModelError> {
    retire_named(config, handle, readers_stopped, false, None).await
}
async fn retirement_selection_guard(
    config: &RuntimeConfig,
    cancel: &crate::owned_read::Cancellation,
    retry: &lctx_native::control::RetryCancellation,
) -> Result<lctx_native::config::SelectionGuard, ModelError> {
    let guard = config.lock_selection().await?;
    cancel.check()?;
    if retry.stopped() {
        return Err(ModelError::Conflict("retirement admission cancelled"));
    }
    Ok(guard)
}
/// Explicit selection clearance and durable invocation addresses remain separate
/// from native authority; every pass validates the actual installed retirement job.
pub async fn retire_named(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    readers_stopped: bool,
    clear_selection: bool,
    job: Option<lctx_model::domain::ContentHash>,
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
    let config = config.clone();
    let handle = handle.clone();
    crate::owned_read::run("native publication retirement", move |cancel| async move {
        let retry = lctx_native::control::RetryCancellation::default();
        let signal = lctx_native::signal::SignalRegistration::retry(
            "native publication retirement",
            retry.clone(),
        )?;
        let mut completion = Completion::default();
        let result = lctx_native::control::with_retry_cancellation(retry.clone(), async {
            cancel.check()?;
            if retry.stopped() {
                return Err(ModelError::Conflict("retirement admission cancelled"));
            }
            let _guard = retirement_selection_guard(&config, &cancel, &retry).await?;
            // This validates the actual marker, not merely equality between two
            // portable config/handle fields, before clearing any local authority.
            let client = lctx_native::installation::check_installation(&config).await?;
            let session = lctx_native::reader::SessionFinalizationGuard::new(client.clone());
            let result = async {
                cancel.check()?;
                if retry.stopped() {
                    return Err(ModelError::Conflict("retirement admission cancelled"));
                }
                if clear_selection {
                    config.clear_selection_locked(&handle, &_guard)?;
                } else if config.selection.try_exists().map_err(ModelError::codec)?
                    && config.selected()? == handle
                {
                    return Err(ModelError::Conflict("cannot retire selected publication"));
                }
                let root = RecordId::new("publication", handle.publication.hex());
                let root_key = serde_json::to_string(&root).map_err(ModelError::codec)?;
                let identity = match job {
                    Some(identity) => identity,
                    None => lctx_native::control::fresh_identity("retirement-invocation")?,
                };
                let mut progress = lctx_native::control::retire_reachable_named(
                    &client,
                    identity,
                    config.service_generation,
                    vec![root],
                    4096,
                )
                .await?;
                completion.committed("native retirement invocation", progress.identity.hex());
                loop {
                    if progress.retained.contains(&root_key) {
                        return Err(ModelError::Conflict(
                            "publication remains reachable or pinned",
                        ));
                    }
                    if progress.remaining.is_empty() {
                        return Ok(progress);
                    }
                    cancel.check()?;
                    if retry.stopped() {
                        return Err(ModelError::Conflict("retirement continuation cancelled"));
                    }
                    progress =
                        lctx_native::control::resume_retirement(&client, progress.identity, 4096)
                            .await?;
                }
            }
            .await;
            if result
                .as_ref()
                .err()
                .is_some_and(|error| !error.permits_storage_cleanup())
            {
                completion.local = lctx_model::domain::completion::LocalState::Outstanding;
                completion.remote = RemoteState::Unknown;
            } else {
                let closed = session.close().await;
                if closed.is_err() {
                    completion.local = lctx_model::domain::completion::LocalState::Outstanding;
                }
                completion.step("retirement session invalidation", closed);
            }
            result
        })
        .await;
        if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            completion.local = lctx_model::domain::completion::LocalState::Outstanding;
            completion.remote = RemoteState::Unknown;
        }
        // The receipt describes actual owner finality, independently of a known
        // refusal or successful committed retirement pages.
        if completion.local == lctx_model::domain::completion::LocalState::Terminal
            && completion.remote == RemoteState::Confirmed
            && completion.failures.is_empty()
        {
            completion.step(
                "retirement signal owner",
                signal.complete(&Completion::default()).await,
            );
        }
        complete(result, completion)
    })
    .await
}

#[cfg(test)]
#[path = "../tests/fixtures/grpc_export.rs"]
pub(crate) mod grpc_export_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn retirement_cancelled_during_selection_lock_wait_preserves_pointer() {
        use std::sync::atomic::Ordering;
        let scratch = tempfile::tempdir().unwrap();
        let config: RuntimeConfig = serde_json::from_value(serde_json::json!({
            "endpoint":"grpc://127.0.0.1:1", "username":"fixture", "password":"private",
            "viewer_username":"viewer", "viewer_password":"private", "namespace":"fixture",
            "database":"validation", "cache_database":"validation", "authentication":"database",
            "service_generation": lctx_model::domain::ContentHash::of(b"fixture"),
            "selection":scratch.path().join("selected.json"), "reuse":null, "serving_limits":null,
        }))
        .unwrap();
        std::fs::write(&config.selection, b"exact selected pointer bytes").unwrap();
        let held = config.lock_selection().await.unwrap();
        let cancel = crate::owned_read::Cancellation::default();
        let retry = lctx_native::control::RetryCancellation::default();
        let mut waiting = Box::pin(retirement_selection_guard(&config, &cancel, &retry));
        std::future::poll_fn(|context| {
            assert!(std::future::Future::poll(waiting.as_mut(), context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        cancel.flag().store(true, Ordering::Release);
        drop(held);
        assert!(waiting.await.is_err());
        assert_eq!(
            std::fs::read(&config.selection).unwrap(),
            b"exact selected pointer bytes"
        );
        config.lock_selection().await.unwrap(); // The refused operation released its guard.
    }

    #[tokio::test]
    async fn backup_sdk_file_export_faults_never_publish_provisional_bytes() {
        use super::grpc_export_fixture::{Fault, Fixture, PARTIAL};
        use lctx_native::Credentials;
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
            let client = lctx_native::reader::connect(
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
        use lctx_native::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        // The SDK follows this owned symlink and encounters actual ENOSPC while writing.
        // /dev/full is a device, not operator data; no production injection hook is involved.
        std::fs::remove_file(&provisional).unwrap();
        std::os::unix::fs::symlink("/dev/full", &provisional).unwrap();
        let client = lctx_native::reader::connect(
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
        use lctx_native::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let output = scratch.path().join("snapshot.surql");
        let client = lctx_native::reader::connect(
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
}
