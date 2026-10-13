//! Actual compiled finite fixture, independent export verification, native publication and VIEWER.
#[path = "../src/backup_import.rs"]
mod backup_decode;
#[path = "../../cpg-core/tests/fixtures/native.rs"]
mod native_fixture;
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
#[allow(
    dead_code,
    reason = "The shared scoped fixture also serves standalone native controls"
)]
#[path = "../../lctx-serving/tests/fixtures/scoped.rs"]
mod scoped_search_fixture;
use cpg_core::{
    artifact, compilation,
    workspace::{Workspace, WorkspaceOptions},
};
use futures::FutureExt;
use lctx_model::domain::{admission::Frontier, serving::SnapshotHandle, stages::Profile, *};
use lctx_native::{NativeReader, RuntimeConfig};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
fn select_and_show_cli(
    binary: &std::path::Path,
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    root: &std::path::Path,
) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let runtime = root.join("runtime.json");
    if !runtime.exists() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&runtime)
            .unwrap();
        file.write_all(&serde_json::to_vec(config).unwrap())
            .unwrap();
    }
    let candidate = root.join("candidate.json");
    std::fs::write(&candidate, serde_json::to_vec(handle).unwrap()).unwrap();
    let selected = std::process::Command::new(binary)
        .args(["snapshot", "--runtime-config"])
        .arg(&runtime)
        .arg("select")
        .arg(&candidate)
        .output()
        .unwrap();
    assert!(
        selected.status.success(),
        "selection CLI: {}",
        String::from_utf8_lossy(&selected.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<SnapshotHandle>(&selected.stdout).unwrap(),
        *handle
    );
    let shown = std::process::Command::new(binary)
        .args(["snapshot", "--runtime-config"])
        .arg(&runtime)
        .arg("show")
        .output()
        .unwrap();
    assert!(
        shown.status.success(),
        "show CLI: {}",
        String::from_utf8_lossy(&shown.stderr)
    );
    assert_eq!(
        serde_json::from_value::<SnapshotHandle>(
            serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["handle"].clone()
        )
        .unwrap(),
        *handle
    );
    // A valid-shaped foreign handle must be refused by the actual CLI before the one
    // selection authority is replaced. The configured viewer follows this same file.
    let before = std::fs::read(&config.selection).unwrap();
    let mut foreign = handle.clone();
    foreign.realization = ContentHash::of(b"unpublished CLI selection candidate");
    std::fs::write(&candidate, serde_json::to_vec(&foreign).unwrap()).unwrap();
    let refused = std::process::Command::new(binary)
        .args(["snapshot", "--runtime-config"])
        .arg(&runtime)
        .arg("select")
        .arg(&candidate)
        .output()
        .unwrap();
    assert!(
        !refused.status.success(),
        "unpublished candidate was selected by the CLI"
    );
    assert_eq!(std::fs::read(&config.selection).unwrap(), before);
    assert_eq!(config.selected().unwrap(), *handle);
    let shown = std::process::Command::new(binary)
        .args(["snapshot", "--runtime-config"])
        .arg(&runtime)
        .arg("show")
        .output()
        .unwrap();
    assert!(shown.status.success());
    assert_eq!(
        serde_json::from_value::<SnapshotHandle>(
            serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["handle"].clone()
        )
        .unwrap(),
        *handle
    );
}
// Derived rows linked to selected anchors are part of this publication's exact cold
// comparison. Unrelated rows in the shared database are deliberately outside its authority.
async fn restored_derived_excess_is_refused(
    reader: &NativeReader<SnapshotHandle>,
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    definitions: &str,
    table: &'static str,
) {
    let native_read_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_native::surrealdb::types::{Object, RecordId, Value, Variables};
    assert_eq!(
        reader.handle(),
        handle,
        "derived control borrows the exact publication"
    );
    let details = lctx_publisher::inspection::show(reader).await.unwrap();
    let loader = lctx_native::Loader::for_views(
        reader.shared_client(),
        details
            .bindings
            .iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view.identity)
            .collect(),
    )
    .with_budget(&native_read_budget);
    let selected = reader
        .prepare_selection()
        .await
        .unwrap()
        .expect("exact publication selection");
    let mut source_rows = selected
        .rows_with_budget(
            "entity",
            "semantic_type='source_artifacts'",
            Variables::new(),
            vec![],
            "id",
            Some(1),
            &native_read_budget,
        )
        .unwrap();
    let source_row = source_rows
        .next()
        .await
        .unwrap()
        .expect("selected source artifact");
    source_rows.drain_transport().await.unwrap();
    let source_row = source_row.as_object().unwrap();
    let target = match source_row.get("anchor").unwrap() {
        Value::RecordId(id) => id.clone(),
        _ => panic!("selected artifact anchor"),
    };
    let target_payload = match source_row.get("id").unwrap() {
        Value::RecordId(id) => id.clone(),
        _ => panic!("selected artifact payload"),
    };
    let suffix = format!(
        "{}-{}",
        handle.publication.hex(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut checks = Vec::new();
    let relation = table.ends_with("occurs");
    let vector = table == "vector" || table == "vec_occurs";
    let source_table = if vector {
        "vector"
    } else if relation {
        "search_source"
    } else {
        table
    };
    let source = RecordId::new(source_table, format!("{suffix}-{table}"));
    let mut endpoint = Object::new();
    endpoint.insert("id", source.clone());
    if vector {
        endpoint.insert("dependencies", vec![target_payload.clone()]);
        let mut embedding = vec![0f32; 1024];
        embedding[0] = 1.;
        endpoint.insert("encoder_hash", suffix.clone());
        endpoint.insert("policy_key", suffix.clone());
        endpoint.insert("library_input", suffix.clone());
        endpoint.insert("family", 3i64);
        endpoint.insert("full_key", suffix.clone());
        endpoint.insert("projection_key", format!("{suffix}-{table}"));
        endpoint.insert("embedding", embedding);
    } else {
        let text = format!("restored extra discovery row {suffix}-{table}");
        endpoint.insert("text", text.clone());
        endpoint.insert("digest", ContentHash::of(text.as_bytes()).0.to_vec());
    }
    let edge_table = if vector { "vec_occurs" } else { "lex_occurs" };
    let edge_id = RecordId::new(edge_table, format!("{suffix}-{table}"));
    let mut edge = Object::new();
    edge.insert("id", edge_id.clone());
    edge.insert("in", source.clone());
    edge.insert("out", target.clone());
    edge.insert("unit_node", target.clone());
    edge.insert("unit_payload", target_payload.clone());
    edge.insert("dependencies", vec![target_payload.clone()]);
    edge.insert("family", 3i64);
    for (field, byte) in [
        ("unit", 1i64),
        ("window", 2),
        ("part", 3),
        ("context", 4),
        ("input", 5),
    ] {
        edge.insert(field, vec![byte; 16]);
    }
    for field in ["binding", "member", "anchor"] {
        edge.insert(field, Value::Null);
    }
    for field in ["exact_name", "exact_path", "exact_option"] {
        edge.insert(field, "");
    }
    edge.insert("eligible", true);
    edge.insert("occurrence_key", format!("{suffix}-{table}"));
    let observed = AssertUnwindSafe(async {
        for (name, relation, row) in [
            (source_table, false, Value::Object(endpoint)),
            (edge_table, true, Value::Object(edge)),
        ] {
            let mut vars = Variables::new();
            vars.insert("rows", vec![row]);
            reader
                .client()
                .query(format!(
                    "INSERT {}INTO {name} $rows",
                    if relation { "RELATION " } else { "" }
                ))
                .bind(vars)
                .await
                .unwrap()
                .check()
                .unwrap();
        }
        for _ in 0..2 {
            let reconciliation = lctx_publisher::reconcile_search(&loader).await;
            let audit = lctx_publisher::inspection::audit(config, handle, definitions).await;
            checks.push((table, reconciliation.is_err(), audit.is_err()));
        }
    })
    .catch_unwind()
    .await;
    // Remove only this control's fresh rows before evaluating negative assertions.
    let mut vars = Variables::new();
    vars.insert("ids", vec![edge_id, source]);
    let cleaned = async {
        reader
            .client()
            .query("DELETE $ids")
            .bind(vars)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        Ok::<(), ModelError>(())
    }
    .await;
    if let Err(primary) = observed {
        if let Err(secondary) = cleaned {
            eprintln!(
                "derived excess observation panicked; exact mutation cleanup failed: {secondary:#?}"
            );
        }
        std::panic::resume_unwind(primary);
    }
    cleaned.unwrap();
    let clean = lctx_publisher::inspection::audit(config, handle, definitions).await;
    checks.push((table, clean.is_ok(), clean.is_ok()));
    for (table, reconciliation, audit) in checks {
        assert!(
            reconciliation && audit,
            "scoped excess refusal and cleaned admission: {table}"
        );
    }
}

#[tokio::test]
async fn compiled_export_publishes_unselected_and_viewer_is_immutable() {
    publication_control().await;
}
async fn publication_control() {
    cpg_extract::logging::init_logging();
    // Keep durable reader owners outside the assertion future. Tokio shutdown after a
    // panic cannot be relied on to finish ReaderPin's best-effort asynchronous Drop.
    let mut inspector_owner = None;
    let mut viewer_owner = None;
    let outcome = AssertUnwindSafe(async {
        let scratch = tempfile::tempdir().unwrap();
        let mut config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        config.selection = scratch.path().join("selected.json");
        let native = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            Frontier::Normalized,
        )
        .await
        .unwrap();
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            native.clone(),
        )
        .unwrap();
        let captured = runtime::capture("catalog_core", Profile::Catalog, workspace.budget());
        let settings = ContentHash::of(
            format!(
                "native-publication-control-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )
            .as_bytes(),
        );
        compilation::compile(
            &workspace,
            captured.clone(),
            Profile::Catalog,
            settings,
            Frontier::Normalized,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        let admitted = artifact::admit(
            &workspace,
            &captured,
            Frontier::Normalized,
            Profile::Catalog,
            settings,
        )
        .await
        .unwrap();
        let export = scratch.path().join("artifact");
        admitted.export(&export).await.unwrap();
        drop(admitted);
        workspace.drain().await.unwrap();
        native.abandon().await.unwrap();
        let restored = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            Frontier::Normalized,
        )
        .await
        .unwrap();
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            restored,
        )
        .unwrap();
        let verified = artifact::verify_export(&export, &workspace).await.unwrap();

        let definitions = lctx_native::materialization::native_definitions();
        let handle = lctx_publisher::publish(&verified, &config, &definitions)
            .await
            .unwrap();
        assert!(!config.selection.exists());
        assert_eq!(handle.semantic, verified.manifest().content());
        let listed = lctx_publisher::inspection::list(&config).await.unwrap();
        assert!(listed.contains(&handle));
        inspector_owner = Some(
            NativeReader::connect(
                &config.endpoint,
                &config.writer_credentials(),
                handle.clone(),
            )
            .await
            .unwrap(),
        );
        let inspector = inspector_owner.as_ref().unwrap();
        let details = lctx_publisher::inspection::show(&inspector).await.unwrap();
        assert_eq!(
            details.manifest.completed_state,
            verified.manifest().completed_state
        );
        assert!(!details.contributions.is_empty());
        assert!(!details.bindings.is_empty());
        lctx_publisher::inspection::audit(&config, &handle, &definitions)
            .await
            .unwrap();
        assert_reader_panic_cleanup(&mut inspector_owner, &config, &handle).await;
        viewer_owner = Some(
            NativeReader::connect(
                &config.endpoint,
                &config.writer_credentials(),
                handle.clone(),
            )
            .await
            .unwrap()
            .with_budget(workspace.budget()),
        );
        let viewer = viewer_owner.as_ref().unwrap();
        let marker: Vec<String> = viewer
            .query(
                format!(
                    "SELECT VALUE handle FROM publication:`{}`",
                    handle.publication.hex()
                ),
                surrealdb_vars(),
            )
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<SnapshotHandle>(&hex::decode(&marker[0]).unwrap()).unwrap(),
            handle
        );
        // The private pin owner is not a serialized read-only ACL grant. Native immutable
        // ingress refuses an existing payload address with different canonical bytes.
        let selected = viewer
            .prepare_selection()
            .await
            .unwrap()
            .expect("exact publication selection");
        let mut selected_rows = selected
            .rows_with_budget(
                "entity",
                "true",
                surrealdb_vars(),
                vec![],
                "id",
                Some(1),
                workspace.budget(),
            )
            .unwrap();
        let mut rows = vec![
            selected_rows
                .next()
                .await
                .unwrap()
                .expect("selected entity"),
        ];
        selected_rows.drain_transport().await.unwrap();
        assert_eq!(
            rows.len(),
            1,
            "immutable collision control requires one selected entity"
        );
        let selected_id = rows[0].as_object().unwrap().get("id").unwrap().clone();
        let lctx_native::surrealdb::types::Value::RecordId(selected_id) = selected_id else {
            panic!("selected entity address")
        };
        assert!(
            selected
                .contains_all_with_budget(&[selected_id], workspace.budget())
                .unwrap(),
            "tampered payload belongs to this exact pinned view"
        );
        let lctx_native::surrealdb::types::Value::Object(ref mut row) = rows[0] else {
            panic!("canonical payload")
        };
        row.insert(
            "canonical",
            lctx_native::surrealdb::types::Bytes::from(b"foreign canonical bytes".to_vec()),
        );
        assert!(
            lctx_native::control::ensure_rows(viewer.client(), None, rows)
                .await
                .is_err()
        );
        for original in &verified.manifest().originals {
            let bytes = viewer
                .original_bytes(
                    original.source,
                    0,
                    usize::try_from(original.byte_len.min(256 << 10)).unwrap(),
                )
                .await
                .unwrap();
            let expected =
                std::fs::read(export.join(format!("original-{}.bin", original.source.0.hex())))
                    .unwrap();
            assert_eq!(bytes.as_ref(), &expected[..bytes.len()]);
        }
        let cli = std::env::var_os("LCTX_REMEDIATION_CLI_BIN").map(std::path::PathBuf::from);
        if let Some(binary) = &cli {
            select_and_show_cli(binary, &config, &handle, scratch.path());
        } else {
            config.select(&handle).unwrap();
        }
        assert_eq!(config.selected().unwrap(), handle);
        let serving = lctx_native::config::ViewerConfig::read(
            &config.selection.with_extension("serving.json"),
        )
        .unwrap();
        assert_eq!(serving.selected().unwrap(), handle);
        assert!(
            lctx_publisher::backup::retire(&config, &handle, false)
                .await
                .is_err()
        );
        assert!(
            lctx_publisher::backup::retire(&config, &handle, true)
                .await
                .is_err()
        );
        // Already pinned readers retain the original immutable realization after selection changes.
        assert_eq!(viewer.handle(), &handle);
        viewer
            .records::<source::SourceArtifact>(lctx_native::RecordSelection::Scope {
                field: "input".into(),
                values: vec![
                    serde_json::to_value(captured.inputs()[0].captured().revision().id()).unwrap(),
                ],
            })
            .await
            .unwrap();
        close_reader_owner(&mut viewer_owner).await.unwrap();
        std::fs::remove_file(&config.selection).unwrap();
        std::fs::remove_file(config.selection.with_extension("serving.json")).unwrap();
        lctx_publisher::backup::retire(&config, &handle, true)
            .await
            .unwrap();
        assert!(
            NativeReader::connect(&config.endpoint, &config.writer_credentials(), handle)
                .await
                .is_err()
        );
    })
    .catch_unwind()
    .await;
    let completion = finalize_reader_owners([&mut inspector_owner, &mut viewer_owner]).await;
    let cleanup = completion::complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(secondary) = cleanup {
            eprintln!("publication assertion failed; reader cleanup also failed: {secondary:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}

/// Small controls still compile and admit real provider outputs. Distinct bytes AND paths
/// give every parallel case its own input revision and physical SourceArtifact anchors.
struct SmallPublication {
    scratch: tempfile::TempDir,
    config: RuntimeConfig,
    handle: SnapshotHandle,
    manifest: graph::Manifest,
    source: Vec<u8>,
}
async fn small_publication(case: &str) -> Result<SmallPublication, ModelError> {
    small_publication_from_source(case, Frontier::Normalized, None).await
}
async fn small_publication_from_source(
    case: &str,
    frontier: Frontier,
    source_tail: Option<&[u8]>,
) -> Result<SmallPublication, ModelError> {
    cpg_extract::logging::init_logging();
    let scratch = tempfile::tempdir().map_err(ModelError::codec)?;
    let mut config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))?;
    config.selection = scratch.path().join("selected.json");
    let native =
        lctx_compiler_store::compiler::NativeCompilerStore::begin(&config, frontier).await?;
    let mut publication_owns_completion = false;
    let built = async {
        let workspace = Workspace::new(
            Arc::new(model()?),
            WorkspaceOptions::default(),
            native.clone(),
        )?;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = format!(
            "{case}_{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(ModelError::codec)?
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let root = scratch.path().join("sources");
        std::fs::create_dir(&root).map_err(ModelError::codec)?;
        let path = format!("{nonce}.py");
        let mut source =
            format!("# Independently owned publication control {nonce}\n").into_bytes();
        match source_tail {
            Some(bytes) => source.extend_from_slice(bytes),
            None => source.extend(
                std::fs::read(runtime::root("native_signature").join("native_signature.py"))
                    .map_err(ModelError::codec)?,
            ),
        }
        std::fs::write(root.join(&path), &source).map_err(ModelError::codec)?;
        let captured = Arc::new(cpg_extract::bundle::CapturedInputs::new(
            vec![cpg_extract::acquisition::AcquiredInput::tree(
                cpg_extract::capture::CapturedInput::capture(&root, &[path], workspace.budget())
                    .map_err(ModelError::codec)?,
                &nonce,
            )],
            cpg_extract::native_context::NativeContextConfig::committed(
                Profile::Catalog,
                workspace.budget(),
            )
            .map_err(ModelError::codec)?,
        ));
        let settings = ContentHash::of(nonce.as_bytes());
        let prepared = if frontier == Frontier::Catalog {
            Some(compilation::PreparedCompilation::new(
                frontier,
                runtime::settings(&nonce),
                captured.config().catalog(),
                None,
                workspace.budget(),
            )?)
        } else {
            None
        };
        compilation::compile(
            &workspace,
            captured.clone(),
            Profile::Catalog,
            settings,
            frontier,
            prepared.as_ref(),
            None,
            None,
        )
        .await?;
        drop(prepared);
        let admitted =
            artifact::admit(&workspace, &captured, frontier, Profile::Catalog, settings).await?;
        let manifest = admitted.manifest().clone();
        // Once entered, publication owns attempt/session completion on either outcome.
        publication_owns_completion = true;
        let handle = lctx_publisher::seal_completed(
            &admitted,
            &config,
            &lctx_native::materialization::native_definitions(),
        )
        .await?;
        Ok((handle, manifest, source))
    }
    .await;
    let mut completion = completion::Completion::default();
    if built.is_err() && !publication_owns_completion {
        completion.step(
            "small publication attempt abandonment",
            native.abandon().await,
        );
    }
    let (handle, manifest, source) = completion::complete(built, completion)?;
    Ok(SmallPublication {
        scratch,
        config,
        handle,
        manifest,
        source,
    })
}

async fn finish_small_publication<const N: usize>(
    fixture: &SmallPublication,
    outcome: std::thread::Result<()>,
    owners: [&mut Option<NativeReader<SnapshotHandle>>; N],
) {
    let reader_phase = lctx_native::phase::Phase::begin("small_publication_reader_finalization");
    let mut completion = finalize_reader_owners(owners).await;
    reader_phase.finish(if completion.failures.is_empty() {
        lctx_native::phase::Terminal::Passed
    } else {
        lctx_native::phase::Terminal::Failed
    });
    for path in [
        &fixture.config.selection,
        &fixture.config.selection.with_extension("serving.json"),
    ] {
        let removed = match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ModelError::codec(error)),
        };
        completion.cleanup(path.display().to_string(), removed);
    }
    let retirement_phase = lctx_native::phase::Phase::begin("small_publication_retirement");
    let retired = lctx_publisher::backup::retire(&fixture.config, &fixture.handle, true)
        .await
        .map(|_| ());
    retirement_phase.finish_result(&retired);
    completion.step("small publication retirement", retired);
    let cleanup = completion::complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(secondary) = cleanup {
            eprintln!("publication control failed; owned cleanup also failed: {secondary:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}

fn logical_backup_rows(
    path: &std::path::Path,
) -> std::collections::BTreeMap<
    lctx_native::surrealdb::types::RecordId,
    lctx_native::surrealdb::types::Value,
> {
    use lctx_native::surrealdb::types::Value;
    let mut dump = backup_decode::DataDump::new(std::fs::File::open(path).unwrap());
    let mut inventory = std::collections::BTreeMap::new();
    while let Some(item) = dump.next().unwrap() {
        if let backup_decode::Item::Rows(rows) = item {
            for row in rows {
                let Value::Object(object) = &row else {
                    panic!("logical backup object");
                };
                let Some(Value::RecordId(id)) = object.get("id") else {
                    panic!("logical backup identity");
                };
                assert!(
                    inventory.insert(id.clone(), row).is_none(),
                    "duplicate logical backup row"
                );
            }
        }
    }
    inventory
}

// Spend successive production retirement passes on the same durable invocation; a
// successful first bounded pass alone does not establish that the source is cold.
// Shared content may remain retained by other publications; the test separately
// establishes absence of this publication's source ownership and original bytes.
async fn retire_publication_completely(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
) -> Result<(), ModelError> {
    let client = lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let result = async {
        let mut progress = lctx_publisher::backup::retire(config, handle, true).await?;
        loop {
            if progress.remaining.is_empty() {
                return Ok(());
            }
            let next =
                lctx_native::control::resume_retirement(&client, progress.identity, 4096).await?;
            if next == progress {
                return Err(ModelError::Conflict(
                    "composed backup retirement made no progress",
                ));
            }
            progress = next;
        }
    }
    .await;
    let mut completion = completion::Completion::default();
    completion.step(
        "composed backup retirement session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    completion::complete(result, completion)
}

#[tokio::test]
async fn selected_backup_excludes_unrelated_retention_and_restores_after_source_retirement() {
    use lctx_native::surrealdb::types::{RecordId, Value};
    let fixture = small_publication("selected_backup_composed").await.unwrap();
    let definitions = lctx_native::materialization::native_definitions();
    let reader_budget =
        resources::ResourceBudget::fixed(WorkspaceOptions::default().memory_bytes).unwrap();
    let mut unrelated_owner = None;
    let mut churn_owner = None;
    let mut restored_owner = None;
    let observer = lctx_native::reader::connect(
        &fixture.config.endpoint,
        &fixture.config.writer_credentials(),
        fixture.config.namespace.as_str(),
        fixture.config.database.as_str(),
    )
    .await
    .unwrap();
    let outcome = AssertUnwindSafe(async {
        let baseline = fixture.scratch.path().join("baseline.surql");
        lctx_publisher::backup::backup(&fixture.config, &fixture.handle, &baseline, &definitions).await.unwrap();
        let expected = logical_backup_rows(&baseline);
        for table in ["entity", "original", "original_blob", "original_chunk", "lex_occurs"] {
            assert!(expected.keys().any(|id| id.table.as_str() == table), "selected fixture must exercise {table}");
        }
        unrelated_owner = Some(small_publication("selected_backup_unrelated_retained").await.unwrap());
        let unrelated = unrelated_owner.as_ref().unwrap();
        assert_ne!(fixture.handle.publication, unrelated.handle.publication);
        for original in &unrelated.manifest.originals {
            assert!(!expected.contains_key(&RecordId::new("original", original.source.0.hex())));
        }
        let output = fixture.scratch.path().join("composed.surql");
        // These are actual independent production owners, not synthetic data mutations.
        // join exercises concurrent calls; it does not establish which snapshot read
        // overlaps a mutation. The external-transaction control covers that ordering.
        let (backed_up, churned) = tokio::join!(
            lctx_publisher::backup::backup(&fixture.config, &fixture.handle, &output, &definitions),
            async {
                churn_owner = Some(small_publication("selected_backup_concurrent_publication").await.unwrap());
                let churn = churn_owner.as_ref().unwrap();
                retire_publication_completely(&churn.config, &churn.handle).await
            },
        );
        backed_up.unwrap();
        churned.unwrap();
        assert_eq!(logical_backup_rows(&output), expected,
            "unrelated retained payloads, publications and actual derived claims cannot enter the selected dump");
        // The unrelated publication remains live across export and selected retirement.
        lctx_publisher::inspection::audit(&unrelated.config, &unrelated.handle, &definitions).await.unwrap();
        retire_publication_completely(&fixture.config, &fixture.handle).await.unwrap();
        let mut absent = vec![RecordId::new("publication", fixture.handle.publication.hex())];
        absent.extend(expected.keys().filter(|id| matches!(id.table.as_str(), "original" | "original_blob" | "original_chunk" | "compiler_contribution" | "compiler_membership")).cloned());
        for window in absent.chunks(128) {
            let mut response = observer.query("SELECT VALUE id FROM $rows")
                .bind(("rows", window.to_vec())).await.unwrap().check().unwrap();
            let remaining: Vec<RecordId> = response.take(0).unwrap();
            assert!(remaining.is_empty(), "cold restore cannot borrow source publication, contributions, memberships or original bytes: {remaining:?}");
        }
        assert!(NativeReader::connect(&fixture.config.endpoint, &fixture.config.writer_credentials(), fixture.handle.clone()).await.is_err(),
            "source publication must be unavailable before independent admission");
        let restored = lctx_publisher::backup::restore_publication(
            &fixture.config, &output, fixture.handle.publication, &definitions,
        ).await.unwrap();
        assert_eq!(restored, fixture.handle);
        assert!(!fixture.config.selection.exists(), "restore does not adopt an operator selection");
        lctx_publisher::inspection::audit(&fixture.config, &restored, &definitions).await.unwrap();
        restored_owner = Some(NativeReader::connect(&fixture.config.endpoint,
            &fixture.config.writer_credentials(), restored).await.unwrap().with_budget(&reader_budget));
        let reader = restored_owner.as_ref().unwrap();
        assert_eq!(lctx_publisher::inspection::show(reader).await.unwrap().manifest.content(), fixture.manifest.content());
        assert_eq!(fixture.manifest.originals.len(), 1);
        for original in &fixture.manifest.originals {
            let bytes = reader.original_bytes(original.source, 0, usize::try_from(original.byte_len).unwrap()).await.unwrap();
            assert_eq!(bytes, fixture.source, "independent cold restore recovers every original byte");
        }
        let restored_dump = fixture.scratch.path().join("restored.surql");
        lctx_publisher::backup::backup(&fixture.config, &fixture.handle, &restored_dump, &definitions).await.unwrap();
        // Fresh physical contributors/bindings are intentionally new. Graph payloads,
        // originals and actual derived integrity claims retain their exact source values.
        let content = |rows: std::collections::BTreeMap<RecordId, Value>| rows.into_iter()
            .filter(|(id, _)| !id.table.as_str().starts_with("compiler_")).collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(content(logical_backup_rows(&restored_dump)), content(expected));
        lctx_publisher::inspection::audit(&unrelated.config, &unrelated.handle, &definitions).await.unwrap();
    }).catch_unwind().await;
    let mut completion = finalize_reader_owners([&mut restored_owner]).await;
    completion.step(
        "composed backup observer invalidation",
        observer.invalidate().await.map_err(ModelError::codec),
    );
    for owned in [
        Some(&fixture),
        unrelated_owner.as_ref(),
        churn_owner.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        completion.step(
            "composed backup publication retirement",
            retire_publication_completely(&owned.config, &owned.handle).await,
        );
    }
    let cleanup = completion::complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(secondary) = cleanup {
            eprintln!(
                "composed backup assertion failed; owned cleanup also failed: {secondary:#?}"
            );
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}

#[tokio::test]
async fn selected_backup_restores_fresh_ownership_while_foreign_window_binding_coexists() {
    use lctx_native::surrealdb::types::{RecordId, RecordIdKey, Value, Variables};
    use retrieval::{BindingBasis, WindowBinding};
    let fixture = small_publication_from_source(
        "selected_backup_foreign_binding",
        Frontier::Catalog,
        Some(b"pass\n"),
    )
    .await
    .unwrap();
    let definitions = lctx_native::materialization::native_definitions();
    let budget =
        resources::ResourceBudget::fixed(WorkspaceOptions::default().memory_bytes).unwrap();
    let mut selected_owner = None;
    let mut restored_owner = None;
    let mut foreign_owner = None;
    let outcome = AssertUnwindSafe(async {
        selected_owner = Some(NativeReader::connect(&fixture.config.endpoint,
            &fixture.config.writer_credentials(), fixture.handle.clone()).await.unwrap().with_budget(&budget));
        let reader = selected_owner.as_ref().unwrap();
        let mut publication = Variables::new();
        publication.insert("publication", RecordId::new("publication", fixture.handle.publication.hex()));
        let pins_query = "SELECT VALUE owner FROM native_hold WITH INDEX object_holds WHERE object=$publication AND record::table(owner)='native_pin' ORDER BY owner";
        let selected_pins: Vec<RecordId> = reader.query_native(pins_query, publication.clone()).await.unwrap();
        assert_eq!(selected_pins.len(), 1, "the selected reader owns one exact publication pin");
        let owners_query = "SELECT VALUE object.attempt FROM native_hold WITH INDEX owner_holds WHERE owner=$publication AND record::table(object)='compiler_contribution'";
        let original_owners: Vec<RecordId> = reader.query_native(owners_query, publication.clone()).await.unwrap();
        let original_owners = original_owners.into_iter().collect::<std::collections::BTreeSet<_>>();
        assert!(!original_owners.is_empty());
        let baseline = fixture.scratch.path().join("before-foreign.surql");
        lctx_publisher::backup::backup(&fixture.config, &fixture.handle, &baseline, &definitions).await.unwrap();
        let expected = logical_backup_rows(&baseline);
        let mut entities = Vec::new();
        let mut assertions = Vec::new();
        for (id, row) in &expected {
            if !matches!(id.table.as_str(), "entity" | "assertion") { continue; }
            let Value::Object(row) = row else { panic!("selected canonical graph row"); };
            let Some(Value::Bytes(bytes)) = row.get("canonical") else { panic!("selected canonical graph bytes"); };
            if id.table.as_str() == "entity" {
                entities.push(serde_json::from_slice::<graph::Entity>(bytes).unwrap());
            } else {
                let assertion = serde_json::from_slice::<graph::Assertion>(bytes).unwrap();
                if assertion.source.is_some() { assertions.push(assertion); }
            }
        }
        let binding = entities.iter().find_map(|entity| match entity {
            graph::Entity::RetrievalWindowBinding(binding) => Some(binding.clone()), _ => None,
        }).expect("real published fixture has a window binding");
        let foreign_binding = WindowBinding {
            basis: if binding.basis == BindingBasis::Source { BindingBasis::DirectDefinition } else { BindingBasis::Source },
            ..binding.clone()
        };
        let foreign_payload = lctx_native::loader::entity_payload_id(&graph::Entity::from(foreign_binding.clone())).unwrap();
        assert!(!expected.contains_key(&foreign_payload), "B's additional binding is absent from A's exact view");
        entities.push(graph::Entity::from(foreign_binding.clone()));
        foreign_owner = Some(scoped_search_fixture::reader(&fixture.config, &entities, &assertions).await.unwrap());
        let foreign = foreign_owner.as_ref().unwrap();
        let foreign_vars = foreign.reader.view_bindings();
        let Some(Value::Array(views)) = foreign_vars.get("lctx_views") else { panic!("foreign exact view IDs"); };
        let views = views.iter().map(|view| {
            let Value::RecordId(view) = view else { panic!("foreign view record ID"); };
            let RecordIdKey::String(hash) = &view.key else { panic!("foreign view identity"); };
            ContentHash(hex::decode(hash).unwrap().try_into().unwrap())
        }).collect::<Vec<_>>();
        let foreign_loader = lctx_native::Loader::for_attempt_views(foreign.reader.shared_client(),
            foreign.store.attempt(), views).with_budget(&budget);
        lctx_publisher::materialize_search(&foreign_loader).await.unwrap();
        lctx_publisher::reconcile_search(&foreign_loader).await.unwrap();
        lctx_publisher::inspection::audit(&fixture.config, &fixture.handle, &definitions).await.unwrap();
        let mut binding_vars = Variables::new();
        binding_vars.insert("binding", lctx_native::loader::json_value(serde_json::json!(foreign_binding.id().bytes())).unwrap());
        let foreign_rows: Vec<Value> = foreign.reader.query_native(
            "SELECT * FROM lex_occurs WHERE binding=$binding ORDER BY id", binding_vars.clone()).await.unwrap();
        assert_eq!(foreign_rows.len(), 1, "the extra B binding creates one distinct lexical claim for A's shared window/unit");
        let Value::Object(foreign_claim) = &foreign_rows[0] else { panic!("foreign lexical claim"); };
        let Some(Value::RecordId(foreign_claim_id)) = foreign_claim.get("id") else { panic!("foreign lexical claim identity"); };
        assert!(!expected.contains_key(foreign_claim_id));
        assert!(foreign_claim.get("dependencies").unwrap().as_array().unwrap().contains(&Value::RecordId(foreign_payload.clone())));
        let Some(Value::RecordId(unit)) = foreign_claim.get("unit_payload") else { panic!("shared unit payload"); };
        assert!(expected.contains_key(unit), "B's occurrence shares A's exact unit payload");
        assert!(expected.values().any(|row| row.as_object().is_some_and(|row|
            row.get("window") == foreign_claim.get("window") && row.get("unit_payload") == Some(&Value::RecordId(unit.clone())))),
            "A already has an occurrence for the exact shared unit and window");
        let output = fixture.scratch.path().join("with-foreign.surql");
        lctx_publisher::backup::backup(&fixture.config, &fixture.handle, &output, &definitions).await.unwrap();
        let backed_up = logical_backup_rows(&output);
        assert_eq!(backed_up, expected, "B's binding and its actual search claim cannot enter A's selected backup");
        assert!(!backed_up.contains_key(&foreign_payload));
        assert!(!backed_up.contains_key(foreign_claim_id));
        let restored = lctx_publisher::backup::restore_publication(&fixture.config, &output,
            fixture.handle.publication, &definitions).await.unwrap();
        assert_eq!(restored, fixture.handle);
        assert!(!fixture.config.selection.exists(), "independent restore does not select the publication");
        lctx_publisher::inspection::audit(&fixture.config, &restored, &definitions).await.unwrap();
        let after_owners: Vec<RecordId> = reader.query_native(owners_query, publication.clone()).await.unwrap();
        let after_owners = after_owners.into_iter().collect::<std::collections::BTreeSet<_>>();
        let fresh = after_owners.difference(&original_owners).cloned().collect::<Vec<_>>();
        assert_eq!(fresh.len(), 1, "restore must independently admit a fresh physical attempt owner");
        let mut owner_vars = Variables::new(); owner_vars.insert("attempts", fresh);
        let attempts: Vec<lctx_native::surrealdb::types::Object> = reader.query_native("SELECT * FROM $attempts", owner_vars).await.unwrap();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].get("state"), Some(&Value::String("closed".into())));
        assert_eq!(attempts[0].get("admitted"), Some(&Value::Bool(true)));
        assert_eq!(attempts[0].get("generation"), Some(&Value::String(fixture.config.service_generation.hex())));
        let pins_after_restore: Vec<RecordId> = reader.query_native(pins_query, publication.clone()).await.unwrap();
        assert_eq!(pins_after_restore, selected_pins, "backup and restore release their exact internal pins");
        restored_owner = Some(NativeReader::connect(&fixture.config.endpoint,
            &fixture.config.writer_credentials(), restored).await.unwrap().with_budget(&budget));
        let restored_reader = restored_owner.as_ref().unwrap();
        let details = lctx_publisher::inspection::show(restored_reader).await.unwrap();
        assert_eq!(details.manifest.content(), fixture.manifest.content());
        assert_eq!(details.manifest.originals, fixture.manifest.originals);
        assert_eq!(fixture.manifest.originals.len(), 1);
        for original in &fixture.manifest.originals {
            assert_eq!(restored_reader.original_bytes(original.source, 0, usize::try_from(original.byte_len).unwrap()).await.unwrap(), fixture.source);
        }
        close_reader_owner(&mut restored_owner).await.unwrap();
        let pins_after_close: Vec<RecordId> = reader.query_native(pins_query, publication).await.unwrap();
        assert_eq!(pins_after_close, selected_pins, "the restored viewer releases its pin while A's original reader stays live");
        let foreign_after: Vec<Value> = foreign.reader.query_native(
            "SELECT * FROM lex_occurs WHERE binding=$binding ORDER BY id", binding_vars).await.unwrap();
        assert_eq!(foreign_after, foreign_rows, "A backup/restore leaves B's foreign search claim unchanged");
        lctx_publisher::reconcile_search(&foreign_loader).await.unwrap();
        let foreign_binding_after: Vec<Value> = foreign.reader.query_native("SELECT * FROM $id", {
            let mut vars=Variables::new(); vars.insert("id", foreign_payload.clone()); vars
        }).await.unwrap();
        assert_eq!(foreign_binding_after.len(), 1, "B's exact foreign payload remains present through restore");
    }).catch_unwind().await;
    let mut completion = completion::Completion::default();
    if let Some(foreign) = foreign_owner.as_ref() {
        completion.step("foreign shared search view cleanup", foreign.close().await);
        completion.step(
            "foreign shared search reader invalidation",
            foreign
                .reader
                .client()
                .invalidate()
                .await
                .map_err(ModelError::codec),
        );
    }
    let cleanup = completion::complete(Ok(()), completion);
    let outcome = match outcome {
        Err(primary) => {
            if let Err(secondary) = cleanup {
                eprintln!(
                    "shared-window backup assertion failed; foreign cleanup also failed: {secondary:#?}"
                );
            }
            Err(primary)
        }
        Ok(()) => {
            AssertUnwindSafe(async {
                cleanup.unwrap();
            })
            .catch_unwind()
            .await
        }
    };
    finish_small_publication(
        &fixture,
        outcome,
        [&mut selected_owner, &mut restored_owner],
    )
    .await;
}

#[tokio::test]
async fn backup_omissions_and_concurrent_cold_restores_preserve_fresh_ownership() {
    cpg_extract::logging::init_logging();
    let fixture = small_publication("backup_cold_restore").await.unwrap();
    let config = &fixture.config;
    let handle = fixture.handle.clone();
    let definitions = lctx_native::materialization::native_definitions();
    let mut viewer_owner = None;
    let mut restored_viewer_owner = None;
    let outcome = AssertUnwindSafe(async {
        viewer_owner = Some(NativeReader::connect(&config.endpoint,
            &config.writer_credentials(), handle.clone()).await.unwrap());
        let viewer = viewer_owner.as_ref().unwrap();
        config.select(&handle).unwrap();
    let backup = fixture.scratch.path().join("snapshot.surql");
    lctx_publisher::backup::backup(&config, &handle, &backup, &definitions)
        .await
        .unwrap();
    assert!(
        lctx_publisher::backup::backup(&config, &handle, &backup, &definitions)
            .await
            .is_err()
    );
    // A logical backup closes exactly the requested publication, even though the
    // engine snapshot includes unrelated shared-service content.
    let mut dump = backup_decode::DataDump::new(std::fs::File::open(&backup).unwrap());
    let mut exported_publications = Vec::new();
    while let Some(item) = dump.next().unwrap() {
        if let backup_decode::Item::Rows(rows) = item {
            for row in rows {
                if let lctx_native::surrealdb::types::Value::Object(object) = row {
                    if let Some(lctx_native::surrealdb::types::Value::RecordId(id)) = object.get("id") {
                        if id.table.as_str() == "publication" {
                            exported_publications.push(id.clone());
                        }
                    }
                }
            }
        }
    }
    assert_eq!(exported_publications, vec![lctx_native::surrealdb::types::RecordId::new("publication", handle.publication.hex())]);
    omitted_backup_payloads_are_refused(&config, &handle, &backup, &definitions).await;
    let selected_before_restore = std::fs::read(&config.selection).unwrap();
    let serving_before_restore =
        std::fs::read(config.selection.with_extension("serving.json")).unwrap();
    let mut ownership = surrealdb_vars();
    ownership.insert(
        "publication",
        lctx_native::surrealdb::types::RecordId::new("publication", handle.publication.hex()),
    );
    // Publication roots retain each physical contributor. Observe only this exact
    // publication's owners, so unrelated concurrent validation attempts cannot enter the check.
    let ownership_query = "SELECT VALUE object.attempt FROM native_hold WITH INDEX owner_holds WHERE owner=$publication AND record::table(object)='compiler_contribution'";
    let before: Vec<lctx_native::surrealdb::types::RecordId> = viewer
        .query_native(ownership_query, ownership.clone())
        .await
        .unwrap();
    let before = before
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let (first, second) = tokio::join!(
        lctx_publisher::backup::restore_publication(
            &config,
            &backup,
            handle.publication,
            &definitions,
        ),
        lctx_publisher::backup::restore_publication(
            &config,
            &backup,
            handle.publication,
            &definitions,
        ),
    );
    let restored = first.unwrap();
    let concurrent = second.unwrap();
    assert_eq!(
        restored, handle,
        "restore retains the immutable publication identity"
    );
    assert_eq!(
        concurrent, restored,
        "concurrent equal restores converge on one handle"
    );
    let (first_audit, second_audit) = tokio::join!(
        lctx_publisher::inspection::audit(&config, &restored, &definitions),
        lctx_publisher::inspection::audit(&config, &concurrent, &definitions),
    );
    first_audit.unwrap();
    second_audit.unwrap();
    assert_eq!(
        std::fs::read(&config.selection).unwrap(),
        selected_before_restore
    );
    assert_eq!(
        std::fs::read(config.selection.with_extension("serving.json")).unwrap(),
        serving_before_restore,
    );
    assert_eq!(config.selected().unwrap(), handle); // Restore never selects its imported handle.
    let after: Vec<lctx_native::surrealdb::types::RecordId> = viewer
        .query_native(ownership_query, ownership)
        .await
        .unwrap();
    let after = after.into_iter().collect::<std::collections::BTreeSet<_>>();
    let fresh = after.difference(&before).cloned().collect::<Vec<_>>();
    assert_eq!(
        fresh.len(),
        2,
        "each restore creates a fresh mutable attempt owner"
    );
    let mut ownership = surrealdb_vars();
    ownership.insert("attempts", fresh);
    let attempts: Vec<lctx_native::surrealdb::types::Object> = viewer
        .query_native("SELECT * FROM $attempts", ownership)
        .await
        .unwrap();
    assert_eq!(attempts.len(), 2);
    let mut epochs = std::collections::BTreeSet::new();
    for attempt in attempts {
        use lctx_native::surrealdb::types::{Number, Value};
        assert_eq!(attempt.get("state"), Some(&Value::String("closed".into())));
        assert_eq!(attempt.get("admitted"), Some(&Value::Bool(true)));
        assert_eq!(
            attempt.get("generation"),
            Some(&Value::String(config.service_generation.hex()))
        );
        let Some(Value::Number(Number::Int(epoch))) = attempt.get("epoch") else {
            panic!("fresh restore attempt epoch");
        };
        assert!(*epoch > 0);
        assert!(
            epochs.insert(*epoch),
            "restores cannot borrow another attempt's epoch"
        );
    }

        restored_viewer_owner = Some(NativeReader::connect(&config.endpoint,
            &config.writer_credentials(), restored.clone()).await.unwrap());
        let restored_viewer = restored_viewer_owner.as_ref().unwrap();
        assert_eq!(fixture.manifest.originals.len(), 1);
        for original in &fixture.manifest.originals {
            let bytes = restored_viewer.original_bytes(original.source, 0,
                usize::try_from(original.byte_len.min(256 << 10)).unwrap()).await.unwrap();
            assert_eq!(bytes.as_ref(), &fixture.source[..bytes.len()]);
        }
        assert_eq!(viewer.handle(), &handle);
    }).catch_unwind().await;
    finish_small_publication(
        &fixture,
        outcome,
        [&mut viewer_owner, &mut restored_viewer_owner],
    )
    .await;
}

macro_rules! derived_excess_controls {
    ($($name:ident: $table:literal,)*) => {$(
        #[tokio::test]
        async fn $name() {
            let fixture = small_publication(stringify!($name)).await.unwrap();
            let mut owner = None;
            let outcome = AssertUnwindSafe(async {
                owner = Some(NativeReader::connect(&fixture.config.endpoint,
                    &fixture.config.writer_credentials(), fixture.handle.clone()).await.unwrap());
                restored_derived_excess_is_refused(owner.as_ref().unwrap(), &fixture.config,
                    &fixture.handle, &lctx_native::materialization::native_definitions(),
                    $table).await;
            }).catch_unwind().await;
            finish_small_publication(&fixture, outcome, [&mut owner]).await;
        }
    )*};
}
derived_excess_controls! {
    cold_audit_refuses_excess_api_options: "search_api_options",
    cold_audit_refuses_excess_documentation_deployment: "search_documentation_deployment",
    cold_audit_refuses_excess_scenario: "search_scenario",
    cold_audit_refuses_excess_source: "search_source",
    cold_audit_refuses_excess_vector: "vector",
    cold_audit_refuses_excess_lexical_occurrence: "lex_occurs",
    cold_audit_refuses_excess_vector_occurrence: "vec_occurs",
}

#[tokio::test]
async fn retirement_matching_stale_generation_refuses_before_selection_clear_or_job_creation() {
    use lctx_native::surrealdb::types::{RecordId, Value};
    let fixture = small_publication("retirement_stale_generation")
        .await
        .unwrap();
    let mut observer_owner = None;
    let outcome = AssertUnwindSafe(async {
        let budget = resources::ResourceBudget::fixed(resources::DEFAULT_MEMORY_BYTES).unwrap();
        observer_owner = Some(NativeReader::connect(
            &fixture.config.endpoint,
            &fixture.config.writer_credentials(),
            fixture.handle.clone(),
        ).await.unwrap().with_budget(&budget));
        let observer = observer_owner.as_ref().unwrap();
        let publication = RecordId::new("publication", fixture.handle.publication.hex());
        let job = lctx_native::control::fresh_identity("stale-generation-retirement-control").unwrap();
        let job_record = RecordId::new("native_retirement", job.hex());
        let mut records = surrealdb_vars();
        records.insert("records", vec![publication.clone(), job_record]);
        let before: Vec<Value> = observer.query_native("SELECT * FROM $records", records.clone()).await.unwrap();
        assert_eq!(before.len(), 1, "actual publication exists and the exact chosen job does not");
        let Value::Object(row) = &before[0] else { panic!("actual publication must be a native row"); };
        assert_eq!(row.get("id"), Some(&Value::RecordId(publication)));

        // Both portable fields agree with each other; only the actual installed
        // generation distinguishes these stale addresses from the live publication.
        let generation_a = ContentHash::of(b"stale-generation-retirement-control");
        assert_ne!(generation_a, fixture.config.service_generation);
        let mut stale_config = fixture.config.clone();
        stale_config.service_generation = generation_a;
        let mut stale_handle = fixture.handle.clone();
        stale_handle.service_generation = generation_a;
        stale_config.select(&stale_handle).unwrap();
        let pointer = std::fs::read(&fixture.config.selection).unwrap();
        let refused = lctx_publisher::backup::retire_named(
            &stale_config, &stale_handle, true, true, Some(job),
        ).await;
        let error = refused.expect_err("matching stale config/handle cannot grant retirement in the actual installation");
        assert!(format!("{error:#?}").contains("installed native service generation/schema"),
            "refusal must come from the actual installed generation, not unrelated infrastructure: {error:#?}");
        assert_eq!(std::fs::read(&fixture.config.selection).unwrap(), pointer,
            "the exact stale selection must remain intact on actual-generation refusal");
        let after: Vec<Value> = observer.query_native("SELECT * FROM $records", records).await.unwrap();
        assert_eq!(after, before, "the actual publication is unchanged and no retirement job was created");
    }).catch_unwind().await;
    finish_small_publication(&fixture, outcome, [&mut observer_owner]).await;
}

#[tokio::test]
#[ignore = "requires explicit exclusive maintenance admission"]
async fn actual_installed_analyzer_drift_is_refused() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "run through just service maintenance --native-clients"
    );
    let fixture = small_publication("installed_analyzer_drift").await.unwrap();
    let config = &fixture.config;
    let handle = fixture.handle.clone();
    let definitions = lctx_native::materialization::native_definitions();
    let outcome = AssertUnwindSafe(async {
        let installer = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
                .expect("maintenance installer config"),
        ))
        .unwrap();
        let admin = lctx_native::reader::connect(
            &installer.endpoint,
            &installer.writer_credentials(),
            installer.namespace.as_str(),
            installer.database.as_str(),
        )
        .await
        .unwrap();
        admin
            .query("DEFINE ANALYZER OVERWRITE lctx_discovery TOKENIZERS class FILTERS uppercase")
            .await
            .unwrap()
            .check()
            .unwrap();
        let refused = lctx_publisher::inspection::audit(&config, &handle, &definitions).await;
        let analyzer = definitions
            .lines()
            .find(|line| line.starts_with("DEFINE ANALYZER lctx_discovery "))
            .unwrap()
            .replacen("DEFINE ANALYZER ", "DEFINE ANALYZER OVERWRITE ", 1);
        admin.query(analyzer).await.unwrap().check().unwrap();
        admin.invalidate().await.unwrap();
        assert!(refused.is_err(), "actual analyzer drift must be refused");
        lctx_publisher::inspection::audit(&config, &handle, &definitions)
            .await
            .unwrap();
    })
    .catch_unwind()
    .await;
    finish_small_publication(&fixture, outcome, []).await;
}

async fn close_reader_owner(
    owner: &mut Option<NativeReader<SnapshotHandle>>,
) -> Result<(), ModelError> {
    let reader = owner.as_ref().expect("live reader owner");
    reader.close().await?;
    reader
        .client()
        .invalidate()
        .await
        .map_err(ModelError::codec)?;
    // Retain ownership on either failure so the outer finalizer can report/finish it.
    owner.take();
    Ok(())
}

async fn finalize_reader_owners<const N: usize>(
    owners: [&mut Option<NativeReader<SnapshotHandle>>; N],
) -> completion::Completion {
    let mut completion = completion::Completion::default();
    for owner in owners {
        if let Some(reader) = owner.as_ref() {
            let close_phase = lctx_native::phase::Phase::begin("publication_reader_close");
            let close = reader.close().await;
            close_phase.finish_result(&close);
            let invalidate_phase =
                lctx_native::phase::Phase::begin("publication_reader_invalidation");
            let invalidate = reader
                .client()
                .invalidate()
                .await
                .map_err(ModelError::codec);
            invalidate_phase.finish_result(&invalidate);
            completion.step("publication reader close", close);
            completion.step("publication reader session invalidation", invalidate);
            // Keep the object alive through finalization observation. Normal explicit
            // close points remove their Option only after close/invalidation succeed.
        }
    }
    completion
}

// Exercise the actual panic path using the already compiled fixture and its inspector.
// This simulated assertion panic is contained here; an enclosing real failure still
// propagates through publication_control's finalizer unchanged.
async fn assert_reader_panic_cleanup(
    owner: &mut Option<NativeReader<SnapshotHandle>>,
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
) {
    use lctx_native::surrealdb::types::RecordId;
    let reader = owner.as_ref().unwrap();
    let mut vars = surrealdb_vars();
    vars.insert(
        "publication",
        RecordId::new("publication", handle.publication.hex()),
    );
    let pins: Vec<RecordId> = reader.query_native(
        "SELECT VALUE owner FROM native_hold WITH INDEX object_holds WHERE object=$publication AND record::table(owner)='native_pin'",
        vars,
    ).await.unwrap();
    assert_eq!(
        pins.len(),
        1,
        "the unique publication has one inspector pin"
    );
    let observer = lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await
    .unwrap();
    let simulated = AssertUnwindSafe(async {
        assert_eq!(reader.handle(), handle);
        panic!("simulated publication assertion failure for awaited reader cleanup");
    })
    .catch_unwind()
    .await;
    let mut completion = finalize_reader_owners([owner]).await;
    // The reader object remains alive here. An independent connection establishes
    // that awaited finalization, rather than eventual Drop, released this exact pin.
    let released = async {
        let mut result = observer
            .query("SELECT VALUE released FROM ONLY $pin")
            .bind(("pin", pins[0].clone()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        result.take::<Option<bool>>(0).map_err(ModelError::codec)
    }
    .await;
    completion.step(
        "publication cleanup observer invalidation",
        observer.invalidate().await.map_err(ModelError::codec),
    );
    let cleanup = completion::complete(Ok(()), completion);
    if cleanup.is_ok() {
        owner.take();
    }
    cleanup.unwrap();
    assert!(
        simulated.is_err(),
        "simulated assertion must exercise unwind cleanup"
    );
    assert_eq!(
        released.unwrap(),
        Some(true),
        "the exact pin is released while its reader object is alive"
    );
}

fn surrealdb_vars() -> lctx_native::surrealdb::types::Variables {
    Default::default()
}

// Omitted content must be refused even though the identical publication already exists
// in this shared store. An intact same-store round trip alone cannot establish that.
async fn omitted_backup_payloads_are_refused(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    backup: &std::path::Path,
    definitions: &str,
) {
    use lctx_native::surrealdb::types::ToSql;
    use std::io::Write;
    for table in ["entity", "assertion", "compiler_record"] {
        let input = backup.with_file_name(format!("omitted-{table}.surql"));
        let mut altered = std::io::BufWriter::new(std::fs::File::create(&input).unwrap());
        writeln!(altered, "OPTION IMPORT;").unwrap();
        let mut dump = backup_decode::DataDump::new(std::fs::File::open(backup).unwrap());
        let mut removed = 0;
        while let Some(item) = dump.next().unwrap() {
            match item {
                backup_decode::Item::Definition(definition) => {
                    writeln!(altered, "{definition};").unwrap()
                }
                backup_decode::Item::Rows(rows) => {
                    // The bounded parser can return empty or multirow export frames.
                    // Omission is per physical row; empty survivors emit no INSERT.
                    let mut kept = Vec::new();
                    for row in rows {
                        let omit = matches!(&row,lctx_native::surrealdb::types::Value::Object(object) if matches!(object.get("id"),Some(lctx_native::surrealdb::types::Value::RecordId(id)) if id.table.as_str()==table));
                        if omit {
                            removed += 1;
                        } else {
                            kept.push(row);
                        }
                    }
                    if !kept.is_empty() {
                        writeln!(
                            altered,
                            "INSERT {};",
                            lctx_native::surrealdb::types::Value::Array(kept.into()).to_sql()
                        )
                        .unwrap();
                    }
                }
            }
        }
        altered.flush().unwrap();
        drop(altered);
        assert!(
            removed > 0,
            "normalized fixture must exercise omitted {table} payloads"
        );
        let error = lctx_publisher::backup::restore_publication(
            config,
            &input,
            handle.publication,
            definitions,
        )
        .await
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("restore selected payload absent from dump"),
            "{table}: {error}"
        );
    }
}
