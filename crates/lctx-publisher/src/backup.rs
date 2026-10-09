//! Trusted local logical transport. Imported metadata never becomes a serving realization.
use crate::{
    PrivatePublication, abandon, begin,
    native_publication::{Admission, Publication},
};
use lctx_model::domain::{
    ModelError,
    completion::{Completion, RemoteState, StorageState, complete},
    graph::{Assertion, Entity, Manifest, Target, semantic_contract},
    serving::SnapshotHandle,
};
use lctx_surrealdb::surrealdb::{
    Surreal,
    engine::remote::http::{Client, Http},
    opt::auth::Root,
    types::{Bytes, RecordId, SurrealValue, Variables},
};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
use std::{
    io::{Seek, Write},
    path::Path,
};

pub(crate) async fn http(
    config: &RuntimeConfig,
    database: &str,
) -> Result<Surreal<Client>, ModelError> {
    // The managed server exposes HTTP and gRPC on the same configured authority.
    let authority = config
        .endpoint
        .strip_prefix("grpc://")
        .ok_or(ModelError::Schema("managed gRPC endpoint"))?;
    let client = Surreal::new::<Http>(authority)
        .await
        .map_err(ModelError::codec)?;
    let setup = async {
        client
            .signin(Root {
                username: config.username.clone(),
                password: config.password.clone(),
            })
            .await
            .map_err(ModelError::codec)?;
        client
            .use_ns(config.namespace.as_str())
            .use_db(database)
            .await
            .map_err(ModelError::codec)?;
        Ok::<(), ModelError>(())
    }
    .await;
    if let Err(error) = setup {
        let mut completion = Completion::default();
        completion.step(
            "restore HTTP setup session invalidation",
            client
                .invalidate()
                .await
                .map_err(|error| ModelError::Cause(Box::new(error))),
        );
        return complete(Err(error), completion);
    }
    Ok(client)
}

/// Write a logical dump after the gRPC file export consumes successful terminal completion.
/// Database users/access credentials and historical versions are excluded.
/// A parent-directory sync failure after publication leaves the dump in place and reports
/// uncertain durability; callers must inspect the destination before retrying.
pub async fn backup(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    output: &Path,
) -> Result<(), ModelError> {
    if handle.database.namespace != config.namespace {
        return Err(ModelError::Conflict("backup namespace"));
    }
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await?;
    viewer
        .client()
        .invalidate()
        .await
        .map_err(ModelError::codec)?;
    drop(viewer);
    if output.exists() {
        return Err(ModelError::Conflict("backup destination already exists"));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let client = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        handle.database.database.as_str(),
    )
    .await?;
    let staged = match tempfile::NamedTempFile::new_in(parent) {
        Ok(staged) => staged,
        Err(error) => {
            let mut completion = Completion::default();
            completion.step(
                "backup setup session invalidation",
                client
                    .invalidate()
                    .await
                    .map_err(|error| ModelError::Cause(Box::new(error))),
            );
            return complete(Err(ModelError::Cause(Box::new(error))), completion);
        }
    };
    // Transport only canonical families, originals and their native role arcs. Derived search
    // is rebuilt on restore; its optional array fields have a 3.3 export/import DDL mismatch.
    let tables = [
        "entity",
        "assertion",
        "participant",
        "reference",
        "external",
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
    // SDK 3.3's gRPC file route awaits the export copy and requires a terminal trailer
    // with the received byte count. The server sends it only after engine success.
    // Its optional BLAKE3 trailer digest is not verified by the SDK.
    let result = client
        .export(staged.path())
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
        .map_err(ModelError::codec);
    let drained = client
        .invalidate()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)));
    drop(client);
    complete_backup(staged, output, result, drained, |parent| {
        std::fs::File::open(parent)?.sync_all()
    })
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

/// Import a trusted current-format local dump privately, then copy canonical graph, exact completed state and
/// original bytes into a fresh realization. No imported marker, permission or function is served.
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let phase = lctx_surrealdb::phase::Phase::begin("restore");
    let outcome = async {
        let staging = begin(config).await?;
        let result = async {
            let admission_phase = lctx_surrealdb::phase::Phase::begin("restore_staging_admission");
            let admission = import(config, input, &staging)
                .await
                .map_err(|error| restore_phase("staging admission", error));
            admission_phase.finish_result(&admission);
            let manifest = admission?;
            let fresh =
                lctx_surrealdb::compiler::NativeCompilerStore::begin(config, manifest.frontier)
                    .await?;
            let result = async {
                let copy_phase = lctx_surrealdb::phase::Phase::begin("restore_canonical_copy");
                let copied = copy(config, &staging, &fresh, &manifest)
                    .await
                    .map_err(|error| restore_phase("canonical copy", error));
                copy_phase.finish_result(&copied);
                let admitted = copied?;
                let publication = Publication::new(Admission::Restored(&admitted), config).await?;
                let published = async {
                    publication.install_definitions(native_definitions).await?;
                    let reference_phase =
                        lctx_surrealdb::phase::Phase::begin("restore_derived_references");
                    let references = async {
                        let mut after = RecordId::new("entity", "");
                        loop {
                            let (rows, last) = page::<Entity>(&staging, "entity", after).await?;
                            if rows.is_empty() {
                                break;
                            }
                            publication.entity_references(&rows).await?;
                            after = last;
                        }
                        let mut after = RecordId::new("assertion", "");
                        loop {
                            let (rows, last) =
                                page::<Assertion>(&staging, "assertion", after).await?;
                            if rows.is_empty() {
                                break;
                            }
                            publication.assertion_references(&rows).await?;
                            after = last;
                        }
                        Ok::<(), ModelError>(())
                    }
                    .await;
                    reference_phase.finish_result(&references);
                    references?;
                    publication
                        .materialize_search()
                        .await
                        .map_err(|error| restore_phase("search reconstruction", error))?;
                    publication
                        .seal(config, native_definitions)
                        .await
                        .map_err(|error| restore_phase("final sealing", error))
                }
                .await;
                let mut completion = Completion::default();
                if published.is_err() {
                    completion.step(
                        "restore publication session invalidation",
                        publication.invalidate().await,
                    );
                }
                complete(published, completion)
            }
            .await;
            let mut completion = Completion::default();
            if let Err(error) = &result
                && !error.has_committed_effect()
            {
                if error.permits_storage_cleanup() {
                    retain_abandon_outcome(
                        &mut completion,
                        fresh.database().as_str(),
                        "fresh realization abandon",
                        fresh.abandon().await,
                    );
                } else {
                    completion
                        .storage
                        .push(StorageState::Orphan(fresh.database().as_str().into()));
                }
            }
            complete(result, completion)
        }
        .await;
        let mut completion = Completion::default();
        if let Ok(handle) = &result {
            completion.committed(
                "sealed unselected database",
                serde_json::to_string(handle).map_err(ModelError::codec)?,
            );
        }
        if result
            .as_ref()
            .err()
            .is_none_or(ModelError::permits_storage_cleanup)
        {
            let cleanup_phase = lctx_surrealdb::phase::Phase::begin("restore_staging_cleanup");
            let cleanup = abandon(&staging).await;
            cleanup_phase.finish_result(&cleanup);
            retain_abandon_outcome(
                &mut completion,
                staging.database.as_str(),
                "restore staging abandon",
                cleanup,
            );
        } else {
            completion
                .storage
                .push(StorageState::Orphan(staging.database.as_str().into()));
        }
        complete(result, completion)
    }
    .await;
    phase.finish_result(&outcome);
    outcome
}

fn retain_abandon_outcome(
    completion: &mut Completion,
    database: &str,
    step: &'static str,
    result: Result<(), ModelError>,
) {
    // Abandon may remove the database and then fail session invalidation. Preserve its
    // known storage outcome instead of classifying that later failure as an orphan.
    let storage = match &result {
        Err(ModelError::Completion(outcome)) => outcome
            .completion
            .storage
            .iter()
            .find(|state| match state {
                StorageState::Removed(identity) | StorageState::Orphan(identity) => {
                    identity == database
                }
            })
            .cloned(),
        _ => None,
    }
    .unwrap_or_else(|| {
        if result.is_ok() {
            StorageState::Removed(database.into())
        } else {
            StorageState::Orphan(database.into())
        }
    });
    completion.storage.push(storage);
    completion.step(step, result);
}

fn restore_phase(phase: &'static str, error: ModelError) -> ModelError {
    match error {
        ModelError::Codec(detail) => ModelError::Codec(format!("restore {phase}: {detail}")),
        ModelError::Invalid(detail) => ModelError::Invalid(format!("restore {phase}: {detail}")),
        ModelError::Frontier(detail) => ModelError::Frontier(format!("restore {phase}: {detail}")),
        ModelError::Infrastructure { class, detail } => ModelError::Infrastructure {
            class,
            detail: format!("restore {phase}: {detail}"),
        },
        other => other,
    }
}

async fn import(
    config: &RuntimeConfig,
    input: &Path,
    staging: &PrivatePublication,
) -> Result<Manifest, ModelError> {
    let client = http(config, staging.database.as_str()).await?;
    let imported = import_units(&client, input).await;
    let drained = client
        .invalidate()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)));
    drop(client);
    let mut completion = Completion::default();
    completion.step("restore import session invalidation", drained);
    complete(imported, completion)?;
    let reader = NativeReader::private(staging.loader.shared_client());
    let bytes: Vec<Bytes> = reader
        .query(
            "SELECT VALUE manifest FROM publication:current",
            Variables::new(),
        )
        .await?;
    let manifest = Manifest::decode(
        bytes
            .first()
            .filter(|_| bytes.len() == 1)
            .ok_or(ModelError::Schema("logical dump publication manifest"))?,
    )?;
    manifest.validate()?;
    if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
        return Err(ModelError::Conflict("restore semantic contract"));
    }
    staging
        .loader
        .reconcile(&manifest)
        .await
        .map_err(|error| restore_phase("staging graph reconciliation", error))?;
    let native = lctx_surrealdb::compiler::NativeCompilerStore::from_existing(
        staging.loader.shared_client(),
        config.namespace.clone(),
        staging.database.clone(),
    );
    if native
        .completed_state()
        .await
        .map_err(|error| restore_phase("staging completed state", error))?
        != manifest.completed_state
    {
        return Err(ModelError::Conflict("restore completed state"));
    }
    Ok(manifest)
}

async fn import_units(client: &Surreal<Client>, input: &Path) -> Result<(), ModelError> {
    crate::backup_import::import(client, input).await
}

async fn copy(
    config: &RuntimeConfig,
    staging: &PrivatePublication,
    native: &std::sync::Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    manifest: &Manifest,
) -> Result<cpg_core::artifact::RestoredAdmission, ModelError> {
    let setup = async {
        cpg_core::workspace::Workspace::new(
            std::sync::Arc::new(lctx_model::domain::model()?),
            cpg_core::workspace::WorkspaceOptions::default(),
            native.clone(),
        )
    }
    .await;
    let runtime = match setup {
        Ok(runtime) => runtime,
        Err(error) => {
            native.fail();
            return complete(Err(error), native.drain_report().await);
        }
    };
    let source_state = lctx_surrealdb::compiler::NativeCompilerStore::from_existing(
        staging.loader.shared_client(),
        config.namespace.clone(),
        staging.database.clone(),
    );
    let result = async {
        let state = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        if source_state
            .export_state(state.path())
            .await
            .map_err(|error| restore_phase("completed-state export", error))?
            != manifest.completed_state
        {
            return Err(ModelError::Conflict("restore source state"));
        }

        // External reconstruction owns content mutation. Derived publication begins only
        // after independent semantic admission returns its nominal owner.
        {
            let mut after = RecordId::new("entity", "");
            loop {
                let (rows, last) = page::<Entity>(staging, "entity", after).await?;
                if rows.is_empty() {
                    break;
                }
                native.import_entities(&rows).await?;
                after = last;
            }
            let mut after = RecordId::new("assertion", "");
            loop {
                let (rows, last) = page::<Assertion>(staging, "assertion", after).await?;
                if rows.is_empty() {
                    break;
                }
                native.import_assertions(&rows).await?;
                after = last;
            }
        }
        let source = NativeReader::private(staging.loader.shared_client());
        for original in &manifest.originals {
            let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
            let mut start = 0;
            while start < original.byte_len {
                let mut ranges = Vec::new();
                for _ in 0..4 {
                    if start >= original.byte_len {
                        break;
                    }
                    let length = (original.byte_len - start).min(65536) as usize;
                    ranges.push((original.source, start, length));
                    start += length as u64;
                }
                for bytes in source.original_bytes_batch(&ranges).await? {
                    file.write_all(&bytes).map_err(ModelError::codec)?;
                }
            }
            file.rewind().map_err(ModelError::codec)?;
            let mut bindings = Variables::new();
            bindings.insert(
                "source",
                lctx_surrealdb::reader::target_id(Target::Entity(original.source)),
            );
            let headers: Vec<Bytes> = source
                .query(
                    "SELECT VALUE canonical FROM entity WHERE id=$source",
                    bindings,
                )
                .await?;
            let entity: Entity = serde_json::from_slice(
                headers
                    .first()
                    .filter(|_| headers.len() == 1)
                    .ok_or(ModelError::Schema("restored original source"))?,
            )
            .map_err(ModelError::codec)?;
            let Entity::Source(header) = entity else {
                return Err(ModelError::Schema("restored original source"));
            };
            if header.content != original.content
                || u64::try_from(header.byte_len).map_err(ModelError::codec)? != original.byte_len
            {
                return Err(ModelError::Conflict("restored original source metadata"));
            }
            file.rewind().map_err(ModelError::codec)?;
            native
                .import_original_stream(
                    original.source.0,
                    original.content,
                    original.byte_len,
                    &mut file,
                )
                .await?;
        }
        native
            .import_state(state.path(), &manifest.completed_state)
            .await
            .map_err(|error| restore_phase("completed-state import", error))?;
        runtime.restore(manifest.profile).await?;
        cpg_core::artifact::verify_restored(&runtime, manifest)
            .await
            .map_err(|error| restore_phase("restored artifact admission", error))
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "restore source native drain",
        complete(Ok(()), source_state.drain_report().await),
    );
    if result.is_err() || !completion.failures.is_empty() {
        native.fail();
        let drained = runtime.drain_report().await;
        completion.step("restore reconstruction drain", complete(Ok(()), drained));
    }

    complete(result, completion)
}

async fn page<T: serde::de::DeserializeOwned>(
    staging: &PrivatePublication,
    table: &str,
    after: RecordId,
) -> Result<(Vec<T>, RecordId), ModelError> {
    let mut vars = Variables::new();
    vars.insert("after", after.clone());
    let mut response = staging
        .loader
        .client()
        .query(format!(
            "SELECT id,canonical FROM {table} WHERE id>$after ORDER BY id LIMIT 128"
        ))
        .bind(vars)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    #[derive(lctx_surrealdb::surrealdb::types::SurrealValue)]
    #[surreal(crate = "lctx_surrealdb::surrealdb::types")]
    struct Row {
        id: RecordId,
        canonical: Bytes,
    }
    let rows: Vec<Row> = response.take(0).map_err(ModelError::codec)?;
    let last = rows.last().map(|r| r.id.clone()).unwrap_or(after);
    let values = rows
        .into_iter()
        .map(|r| serde_json::from_slice(&r.canonical).map_err(ModelError::codec))
        .collect::<Result<_, _>>()?;
    Ok((values, last))
}

/// Retire an explicitly named, unselected published database after the operator has stopped
/// every known reader. There is no automatic lease/history or selected-handle replacement.
pub async fn retire(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    readers_stopped: bool,
) -> Result<(), ModelError> {
    if !readers_stopped {
        return Err(ModelError::Invalid(
            "retirement requires stopped readers".into(),
        ));
    }
    if handle.database.namespace != config.namespace {
        return Err(ModelError::Conflict("retirement namespace"));
    }
    let _guard = config.lock_selection().await?;
    let database = handle.database.database.as_str();
    let suffix = database
        .strip_prefix("snapshot_")
        .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or(ModelError::Schema("published private database name"))?;
    if config.selection.try_exists().map_err(ModelError::codec)?
        && config.selected()?.database == handle.database
    {
        return Err(ModelError::Conflict("cannot retire selected snapshot"));
    }
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await?;
    viewer
        .client()
        .invalidate()
        .await
        .map_err(ModelError::codec)?;
    drop(viewer);
    let client = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        database,
    )
    .await?;
    client
        .query(format!("REMOVE DATABASE `snapshot_{suffix}`"))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client
        .invalidate()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/fixtures/grpc_export.rs"]
mod grpc_export_fixture;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn restore_failed_http_request_retains_primary_and_removed_staging_outcome() {
        use lctx_model::domain::completion::{LocalState, RemoteState, StorageState};
        use lctx_model::domain::serving::Name;
        use lctx_surrealdb::{reader, surrealdb::types::Value};
        let mut config = RuntimeConfig::read(&std::path::PathBuf::from(
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"),
        ))
        .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        config.namespace = Name::new(format!("failed_restore_{}", std::process::id())).unwrap();
        config.selection = scratch.path().join("selection.json");
        let selected = b"selection must survive failed private import";
        std::fs::write(&config.selection, selected).unwrap();
        let input = scratch.path().join("failed.surql");
        // The first default-sized request has independent effects on both sides of a
        // checked failure. The trailing CREATE belongs to a separate, forbidden request.
        let dump = format!(
            "OPTION IMPORT; DEFINE TABLE import_probe SCHEMALESS; CREATE import_probe:before; THROW 'restore-owned-sentinel'; CREATE import_probe:within; {} CREATE import_probe:following;",
            "RETURN 1;".repeat(lctx_model::domain::resources::TRANSFER_ROWS - 4)
        );
        std::fs::write(&input, &dump).unwrap();
        let mut requests = crate::backup_import::Requests::new(dump.as_bytes());
        assert!(
            !requests
                .next_request()
                .unwrap()
                .unwrap()
                .contains("import_probe:following")
        );
        assert!(
            requests
                .next_request()
                .unwrap()
                .unwrap()
                .contains("import_probe:following")
        );
        assert!(requests.next_request().unwrap().is_none());

        let error = restore(&config, &input, "").await.unwrap_err();
        assert!(
            matches!(error.primary(), Some(ModelError::Cause(cause))
            if cause.downcast_ref::<lctx_surrealdb::surrealdb::Error>()
                .is_some_and(|error| error.is_thrown() && error.to_string().contains("restore-owned-sentinel"))),
            "structured restore failure: {error:?}"
        );
        assert!(error.permits_storage_cleanup());
        assert!(!error.has_committed_effect());
        let ModelError::Completion(outcome) = error else {
            panic!("restore completion outcome")
        };
        assert_eq!(outcome.completion.local, LocalState::Terminal);
        assert_eq!(outcome.completion.remote, RemoteState::Confirmed);
        assert!(outcome.completion.failures.is_empty());
        let [StorageState::Removed(database)] = outcome.completion.storage.as_slice() else {
            panic!("failed restore must report its removed private database")
        };
        let client = reader::authenticated(&config.endpoint, &config.root_credentials(), None)
            .await
            .unwrap();
        client.use_ns(config.namespace.as_str()).await.unwrap();
        let info = client
            .query("INFO FOR NS;")
            .await
            .unwrap()
            .check()
            .unwrap()
            .take::<Option<Value>>(0)
            .unwrap()
            .unwrap();
        let Value::Object(info) = info else {
            panic!("namespace info")
        };
        let Some(Value::Object(databases)) = info.get("databases") else {
            panic!("namespace databases")
        };
        assert!(
            databases.is_empty(),
            "failed restore left a database: {databases:?}"
        );
        assert!(!databases.contains_key(database));
        assert_eq!(std::fs::read(&config.selection).unwrap(), selected);
        client
            .query(format!("REMOVE NAMESPACE `{}`", config.namespace.as_str()))
            .await
            .unwrap()
            .check()
            .unwrap();
        client.invalidate().await.unwrap();
    }

    #[test]
    fn restore_primary_and_cleanup_errors_are_structured() {
        let mut completion = Completion::default();
        completion.cleanup("staging", Err(ModelError::Codec("cleanup failure".into())));
        let error = complete::<()>(
            Err(ModelError::Schema("terminal import failure")),
            completion,
        )
        .unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Schema("terminal import failure"))
        ));
        let ModelError::Completion(outcome) = error else {
            panic!()
        };
        assert!(
            matches!(&outcome.completion.failures[0].error,ModelError::Codec(detail) if detail=="cleanup failure")
        );
    }

    #[test]
    fn restore_abandon_retains_known_removal_after_session_failure() {
        let mut abandoned = Completion::default();
        abandoned.cleanup("staging", Ok(()));
        abandoned.step(
            "publication abandon session invalidation",
            Err(ModelError::Codec("invalidation failure".into())),
        );
        let mut completion = Completion::default();
        retain_abandon_outcome(
            &mut completion,
            "staging",
            "restore staging abandon",
            complete(Ok(()), abandoned),
        );
        let error = complete::<()>(
            Err(ModelError::Schema("terminal import failure")),
            completion,
        )
        .unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Schema("terminal import failure"))
        ));
        let ModelError::Completion(outcome) = error else {
            panic!("completion")
        };
        assert_eq!(
            outcome.completion.storage,
            vec![StorageState::Removed("staging".into())]
        );
        let ModelError::Completion(abandoned) = &outcome.completion.failures[0].error else {
            panic!("abandon outcome")
        };
        assert!(
            matches!(&abandoned.completion.failures[0].error,ModelError::Codec(detail) if detail=="invalidation failure")
        );
    }

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
        use lctx_model::domain::{
            ContentHash,
            serving::{DatabaseIdentity, Name},
        };

        let fixture_path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
            .expect("owned disposable native server required");
        let fixture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let config = RuntimeConfig {
            endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
            username: fixture["admin_user"].as_str().unwrap().into(),
            password: fixture["admin_password"].as_str().unwrap().into(),
            viewer_username: "backup_viewer".into(),
            viewer_password: "owned-backup-fixture-password".into(),
            namespace: Name::new(format!("backup_control_{}", std::process::id())).unwrap(),
            cache_database: Name::new("cache").unwrap(),
            selection: scratch.path().join("selected.json"),
        };
        let handle = SnapshotHandle {
            semantic: ContentHash::of(b"backup-control-semantic"),
            realization: ContentHash::of(b"backup-control-realization"),
            database: DatabaseIdentity {
                namespace: config.namespace.clone(),
                database: Name::new("backup_fixture").unwrap(),
            },
        };
        let client = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.root_credentials(),
            config.namespace.as_str(),
            handle.database.database.as_str(),
        )
        .await
        .unwrap();
        let mut vars = Variables::new();
        vars.insert("handle", hex::encode(serde_json::to_vec(&handle).unwrap()));
        client
            .query("DEFINE USER backup_viewer ON DATABASE PASSWORD 'owned-backup-fixture-password' ROLES VIEWER;
                DEFINE TABLE publication SCHEMALESS PERMISSIONS FOR select FULL;
                CREATE publication:current SET handle = $handle;
                CREATE entity:example SET canonical = 'canonical-backup-sentinel';
                CREATE discovery_document:example SET text = 'derived-search-sentinel';")
            .bind(vars)
            .await
            .unwrap()
            .check()
            .unwrap();
        let output = scratch.path().join("snapshot.surql");
        backup(&config, &handle, &output).await.unwrap();
        let dump = std::fs::read_to_string(&output).unwrap();
        assert!(dump.contains("canonical-backup-sentinel"));
        assert!(dump.contains("publication"));
        assert!(!dump.contains("derived-search-sentinel"));
        assert!(!dump.contains("backup_viewer"));
        assert!(!dump.contains("owned-backup-fixture-password"));
        assert!(!config.selection.exists());
        assert!(backup(&config, &handle, &output).await.is_err());
        assert_eq!(std::fs::read_to_string(output).unwrap(), dump);
        client
            .query("REMOVE DATABASE backup_fixture")
            .await
            .unwrap()
            .check()
            .unwrap();
        client.invalidate().await.unwrap();
    }
}
