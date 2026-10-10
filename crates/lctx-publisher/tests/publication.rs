//! Actual compiled finite fixture, independent export verification, native publication and VIEWER.
#[path = "../src/backup_import.rs"]
mod backup_decode;
#[path = "../../cpg-core/tests/fixtures/native.rs"]
mod native_fixture;
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact, compilation,
    workspace::{Workspace, WorkspaceOptions},
};
use futures::FutureExt;
use lctx_model::domain::{admission::Frontier, serving::SnapshotHandle, stages::Profile, *};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
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
) {
    use lctx_surrealdb::surrealdb::types::{Object, RecordId, Value, Variables};
    assert_eq!(
        reader.handle(),
        handle,
        "derived control borrows the exact publication"
    );
    let details = lctx_publisher::inspection::show(reader).await.unwrap();
    let loader = lctx_surrealdb::Loader::for_views(
        reader.shared_client(),
        details
            .bindings
            .iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view.identity)
            .collect(),
    );
    let mut response=reader.client().query("SELECT node AS payload,node.anchor AS anchor FROM compiler_view_member WHERE view IN $lctx_views AND node.semantic_type='source_artifacts' LIMIT 1").bind(reader.view_bindings()).await.unwrap().check().unwrap();
    let targets: Vec<Object> = response.take(0).unwrap();
    let target = match targets[0].get("anchor").unwrap() {
        Value::RecordId(id) => id.clone(),
        _ => panic!("selected artifact anchor"),
    };
    let target_payload = match targets[0].get("payload").unwrap() {
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
    for table in [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
        "vector",
        "lex_occurs",
        "vec_occurs",
    ] {
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
        // Remove only this control's fresh rows before evaluating negative assertions.
        let mut vars = Variables::new();
        vars.insert("ids", vec![edge_id, source]);
        reader
            .client()
            .query("DELETE $ids")
            .bind(vars)
            .await
            .unwrap()
            .check()
            .unwrap();
        let clean = lctx_publisher::inspection::audit(config, handle, definitions).await;
        checks.push((table, clean.is_ok(), clean.is_ok()));
    }
    for (table, reconciliation, audit) in checks {
        assert!(
            reconciliation && audit,
            "scoped excess refusal and cleaned admission: {table}"
        );
    }
}

#[tokio::test]
async fn compiled_export_publishes_unselected_and_viewer_is_immutable() {
    publication_control(false).await;
}
#[tokio::test]
#[ignore = "requires explicit exclusive maintenance admission"]
async fn actual_installed_analyzer_drift_is_refused() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "run through just service maintenance --native-clients"
    );
    publication_control(true).await;
}
async fn publication_control(maintenance: bool) {
    cpg_extract::logging::init_logging();
    // Keep durable reader owners outside the assertion future. Tokio shutdown after a
    // panic cannot be relied on to finish ReaderPin's best-effort asynchronous Drop.
    let mut inspector_owner = None;
    let mut viewer_owner = None;
    let mut restored_viewer_owner = None;
    let outcome = AssertUnwindSafe(async {
    let scratch = tempfile::tempdir().unwrap();
    let mut config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap();
    config.selection = scratch.path().join("selected.json");
    let native =
        lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Normalized)
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
    let restored =
        lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Normalized)
            .await
            .unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions::default(),
        restored,
    )
    .unwrap();
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();

    let definitions = lctx_surrealdb::materialization::native_definitions();
    let handle = lctx_publisher::publish(&verified, &config, &definitions)
        .await
        .unwrap();
    assert!(!config.selection.exists());
    assert_eq!(handle.semantic, verified.manifest().content());
    let listed = lctx_publisher::inspection::list(&config).await.unwrap();
    assert!(listed.contains(&handle));
    inspector_owner = Some(NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap());
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
    if maintenance {
        let installer = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
                .expect("maintenance installer config"),
        ))
        .unwrap();
        let admin = lctx_surrealdb::reader::connect(
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
    }
    viewer_owner = Some(NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap());
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
    let selected_entity = lctx_surrealdb::prepared::PreparedQuery::new(
        surrealdb_vars(),
        vec!["LET $nodes=SELECT VALUE node FROM compiler_view_member WITH INDEX view_nodes WHERE view IN $lctx_views".into()],
        vec!["SELECT * FROM $nodes WHERE record::table(id)='entity' LIMIT 1".into()],
    ).unwrap();
    let mut rows: Vec<lctx_surrealdb::surrealdb::types::Value> =
        viewer.query_prepared_native(selected_entity).await.unwrap();
    assert_eq!(
        rows.len(),
        1,
        "immutable collision control requires one selected entity"
    );
    let selected_id = rows[0].as_object().unwrap().get("id").unwrap().clone();
    let mut selected = surrealdb_vars();
    selected.insert("node", selected_id);
    let membership: Vec<lctx_surrealdb::surrealdb::types::RecordId> = viewer
        .query_native("SELECT VALUE id FROM compiler_view_member WITH INDEX view_nodes WHERE view IN $lctx_views AND node=$node LIMIT 1", selected)
        .await
        .unwrap();
    assert_eq!(
        membership.len(),
        1,
        "tampered payload belongs to this exact pinned view"
    );
    let lctx_surrealdb::surrealdb::types::Value::Object(ref mut row) = rows[0] else {
        panic!("canonical payload")
    };
    row.insert(
        "canonical",
        lctx_surrealdb::surrealdb::types::Bytes::from(b"foreign canonical bytes".to_vec()),
    );
    assert!(
        lctx_surrealdb::control::ensure_rows(viewer.client(), None, rows)
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
        assert_eq!(bytes, &expected[..bytes.len()]);
    }
    let cli = std::env::var_os("LCTX_REMEDIATION_CLI_BIN").map(std::path::PathBuf::from);
    if let Some(binary) = &cli {
        select_and_show_cli(binary, &config, &handle, scratch.path());
    } else {
        config.select(&handle).unwrap();
    }
    assert_eq!(config.selected().unwrap(), handle);
    let serving = lctx_surrealdb::config::ViewerConfig::read(
        &config.selection.with_extension("serving.json"),
    )
    .unwrap();
    assert_eq!(serving.selected().unwrap(), handle);
    let backup = scratch.path().join("snapshot.surql");
    lctx_publisher::backup::backup(&config, &handle, &backup)
        .await
        .unwrap();
    assert!(
        lctx_publisher::backup::backup(&config, &handle, &backup)
            .await
            .is_err()
    );
    omitted_backup_payloads_are_refused(&config, &handle, &backup, &definitions).await;
    let selected_before_restore = std::fs::read(&config.selection).unwrap();
    let serving_before_restore =
        std::fs::read(config.selection.with_extension("serving.json")).unwrap();
    let mut ownership = surrealdb_vars();
    ownership.insert(
        "publication",
        lctx_surrealdb::surrealdb::types::RecordId::new("publication", handle.publication.hex()),
    );
    // Publication roots retain each physical contributor. Observe only this exact
    // publication's owners, so unrelated concurrent validation attempts cannot enter the check.
    let ownership_query = "SELECT VALUE object.attempt FROM native_hold WITH INDEX owner_holds WHERE owner=$publication AND record::table(object)='compiler_contribution'";
    let before: Vec<lctx_surrealdb::surrealdb::types::RecordId> = viewer
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
    let after: Vec<lctx_surrealdb::surrealdb::types::RecordId> = viewer
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
    let attempts: Vec<lctx_surrealdb::surrealdb::types::Object> = viewer
        .query_native("SELECT * FROM $attempts", ownership)
        .await
        .unwrap();
    assert_eq!(attempts.len(), 2);
    let mut epochs = std::collections::BTreeSet::new();
    for attempt in attempts {
        use lctx_surrealdb::surrealdb::types::{Number, Value};
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
    restored_derived_excess_is_refused(viewer, &config, &restored, &definitions).await;
    restored_viewer_owner = Some(NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        restored.clone(),
    )
    .await
    .unwrap());
    let restored_viewer = restored_viewer_owner.as_ref().unwrap();
    for original in &verified.manifest().originals {
        let bytes = restored_viewer
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
        assert_eq!(bytes, &expected[..bytes.len()]);
    }
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
    // Restore of identical content does not invent a second publication identity.
    // Already pinned readers retain the original immutable realization after selection changes.
    assert_eq!(viewer.handle(), &handle);
    viewer
        .records::<source::SourceArtifact>(lctx_surrealdb::RecordSelection::Scope {
            field: "input".into(),
            values: vec![
                serde_json::to_value(captured.inputs()[0].captured().revision().id()).unwrap(),
            ],
        })
        .await
        .unwrap();
    close_reader_owner(&mut viewer_owner).await.unwrap();
    close_reader_owner(&mut restored_viewer_owner).await.unwrap();
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
    let completion = finalize_reader_owners([
        &mut inspector_owner,
        &mut viewer_owner,
        &mut restored_viewer_owner,
    ])
    .await;
    let cleanup = completion::complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(secondary) = cleanup {
            eprintln!("publication assertion failed; reader cleanup also failed: {secondary:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
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
            let close = reader.close().await;
            let invalidate = reader
                .client()
                .invalidate()
                .await
                .map_err(ModelError::codec);
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
    use lctx_surrealdb::surrealdb::types::RecordId;
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
    let observer = lctx_surrealdb::reader::connect(
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

fn surrealdb_vars() -> lctx_surrealdb::surrealdb::types::Variables {
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
    use lctx_surrealdb::surrealdb::types::ToSql;
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
                        let omit = matches!(&row,lctx_surrealdb::surrealdb::types::Value::Object(object) if matches!(object.get("id"),Some(lctx_surrealdb::surrealdb::types::Value::RecordId(id)) if id.table.as_str()==table));
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
                            lctx_surrealdb::surrealdb::types::Value::Array(kept.into()).to_sql()
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
