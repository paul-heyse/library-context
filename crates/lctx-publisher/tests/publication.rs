//! Actual compiled finite fixture, independent export verification, native publication and VIEWER.
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact, compilation,
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    admission::Frontier,
    serving::{Name, SnapshotHandle},
    stages::Profile,
    *,
};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
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
        serde_json::from_slice::<SnapshotHandle>(&shown.stdout).unwrap(),
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
        serde_json::from_slice::<SnapshotHandle>(&shown.stdout).unwrap(),
        *handle
    );
}
// The normalized publication has no search corpus. Exercise actual excess rows in a fresh
// restore without inventing canonical retrieval premises; populated field mutations live in
// stream_search. Relation cases need injected endpoint nodes because ENFORCED rejects dangling
// relations; every injected row is read back before audit and removed before the clean audit.
async fn restored_derived_excess_is_refused(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    definitions: &str,
) {
    use lctx_surrealdb::surrealdb::types::{Object, RecordId, Value, Variables};
    let admin = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        handle.database.database.as_str(),
    )
    .await
    .unwrap();
    async fn query(
        admin: &lctx_surrealdb::surrealdb::Surreal<
            lctx_surrealdb::surrealdb::engine::remote::grpc::Client,
        >,
        sql: &str,
    ) -> Vec<Value> {
        admin
            .query(sql)
            .await
            .unwrap()
            .check()
            .unwrap()
            .take(0)
            .unwrap()
    }
    let restored_loader = lctx_surrealdb::Loader::new(admin.clone());
    let canonical_entities = query(&admin, "SELECT id,canonical FROM entity ORDER BY id").await;
    let canonical_assertions =
        query(&admin, "SELECT id,canonical FROM assertion ORDER BY id").await;
    let selected = std::fs::read(&config.selection).unwrap();
    let targets: Vec<RecordId> = admin
        .query("SELECT VALUE id FROM entity ORDER BY id LIMIT 1")
        .await
        .unwrap()
        .check()
        .unwrap()
        .take(0)
        .unwrap();
    let target = targets.first().expect("actual canonical endpoint").clone();
    let tables = [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
        "vector",
        "lex_occurs",
        "vec_occurs",
    ];
    let initial = {
        let mut rows = Vec::new();
        for table in tables {
            rows.push(query(&admin, &format!("SELECT * FROM {table} ORDER BY id")).await);
        }
        rows
    };
    assert!(
        initial.iter().all(Vec::is_empty),
        "normalized frontier deliberately has no retrieval derivations"
    );
    fn text_row(table: &str) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new(table, "restored_excess"));
        row.insert("text", "restored extra discovery row");
        row.insert(
            "digest",
            ContentHash::of(b"restored extra discovery row").0.to_vec(),
        );
        Value::Object(row)
    }
    fn vector_row() -> Value {
        let mut embedding = vec![0.0f32; 1024];
        embedding[0] = 1.0;
        let mut row = Object::new();
        row.insert("id", RecordId::new("vector", "restored_excess"));
        row.insert("encoder_hash", "restored-excess-encoder");
        row.insert("policy_key", "restored-excess-policy");
        row.insert("library_input", "restored-excess-library");
        row.insert("family", 3i64);
        row.insert("full_key", "restored-excess-full");
        row.insert("projection_key", "restored-excess-projection");
        row.insert("embedding", embedding);
        Value::Object(row)
    }
    for table in tables {
        let mut additions = Vec::new();
        let row = if table.starts_with("search_") {
            text_row(table)
        } else if table == "vector" {
            vector_row()
        } else {
            let endpoint = if table == "lex_occurs" {
                text_row("search_source")
            } else {
                vector_row()
            };
            let endpoint_id = endpoint.as_object().unwrap().get("id").unwrap().clone();
            additions.push(endpoint);
            let mut row = Object::new();
            row.insert("id", RecordId::new(table, "restored_excess"));
            row.insert("in", endpoint_id);
            row.insert("out", target.clone());
            row.insert("family", 3i64);
            row.insert("unit", vec![1i64; 16]);
            row.insert("window", vec![2i64; 16]);
            row.insert("part", vec![5i64; 16]);
            row.insert("binding", Value::Null);
            row.insert("exact_name", "");
            row.insert("exact_path", "");
            row.insert("exact_option", "");
            row.insert("context", vec![3i64; 16]);
            row.insert("input", vec![4i64; 16]);
            row.insert("member", Value::Null);
            row.insert("anchor", Value::Null);
            row.insert("eligible", true);
            row.insert("occurrence_key", "restored_excess");
            Value::Object(row)
        };
        additions.push(row);
        for row in &additions {
            let name = row
                .as_object()
                .unwrap()
                .get("id")
                .unwrap()
                .as_record()
                .unwrap()
                .table
                .as_str();
            let mut bindings = Variables::new();
            bindings.insert("rows", vec![row.clone()]);
            admin
                .query(format!(
                    "INSERT {}INTO {name} $rows",
                    if name.ends_with("occurs") {
                        "RELATION "
                    } else {
                        ""
                    }
                ))
                .bind(bindings)
                .await
                .unwrap()
                .check()
                .unwrap();
        }
        let persisted = query(&admin, &format!("SELECT * FROM {table} ORDER BY id")).await;
        assert_eq!(
            persisted.len(),
            1,
            "injected restored {table} row must actually persist"
        );
        let mut corrupted_families = Vec::new();
        for name in tables {
            corrupted_families
                .push(query(&admin, &format!("SELECT * FROM {name} ORDER BY id")).await);
        }
        // Reach the derived comparator directly; another cold-audit phase cannot supply
        // this refusal. Only search/vector endpoint nodes were injected, never entity rows.
        for _ in 0..2 {
            assert!(
                matches!(
                    lctx_publisher::reconcile_search(&restored_loader).await,
                    Err(ModelError::Serving(serving::FailureKind::Corrupt))
                ),
                "restored derived mismatch {table}"
            );
        }
        assert!(
            lctx_publisher::inspection::audit(config, handle, definitions)
                .await
                .is_err(),
            "restored excess {table}"
        );
        assert!(
            lctx_publisher::inspection::audit(config, handle, definitions)
                .await
                .is_err()
        );
        assert_eq!(
            query(&admin, &format!("SELECT * FROM {table} ORDER BY id")).await,
            persisted,
            "cold refusal does not repair {table}"
        );
        for (name, before) in tables.into_iter().zip(&corrupted_families) {
            assert_eq!(
                &query(&admin, &format!("SELECT * FROM {name} ORDER BY id")).await,
                before,
                "refusal does not repair derived family {name}"
            );
        }
        assert_eq!(
            query(&admin, "SELECT id,canonical FROM entity ORDER BY id").await,
            canonical_entities
        );
        assert_eq!(
            query(&admin, "SELECT id,canonical FROM assertion ORDER BY id").await,
            canonical_assertions
        );
        assert_eq!(std::fs::read(&config.selection).unwrap(), selected);
        for row in additions.iter().rev() {
            let mut bindings = Variables::new();
            bindings.insert("id", row.as_object().unwrap().get("id").unwrap().clone());
            admin
                .query("DELETE $id")
                .bind(bindings)
                .await
                .unwrap()
                .check()
                .unwrap();
        }
        lctx_publisher::reconcile_search(&restored_loader)
            .await
            .unwrap();
        lctx_publisher::inspection::audit(config, handle, definitions)
            .await
            .unwrap();
        for name in tables {
            assert!(
                query(&admin, &format!("SELECT * FROM {name} ORDER BY id"))
                    .await
                    .is_empty()
            );
        }
    }
    admin.invalidate().await.unwrap();
}

#[tokio::test]
async fn compiled_export_publishes_unselected_and_viewer_is_immutable() {
    let path =
        std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable native server required");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            partitions: 1,
            batch_rows: 128,
        },
    )
    .unwrap();
    let captured = runtime::capture("catalog_core", Profile::Catalog, workspace.budget());
    let settings = ContentHash::of(b"native-publication-control");
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
    admitted.export(&export).unwrap();
    drop(admitted);
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();
    let config = RuntimeConfig {
        endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
        username: fixture["admin_user"].as_str().unwrap().into(),
        password: fixture["admin_password"].as_str().unwrap().into(),
        viewer_username: "fixture_viewer".into(),
        viewer_password: format!("fixture-viewer-{}", std::process::id()),
        namespace: Name::new(format!("gn_publication_{}", std::process::id())).unwrap(),
        cache_database: Name::new("cache").unwrap(),
        selection: scratch.path().join("selected.json"),
    };
    let definitions = lctx_surrealdb::materialization::native_definitions();
    let handle = lctx_publisher::publish(&verified, &config, &definitions)
        .await
        .unwrap();
    assert!(!config.selection.exists());
    assert_eq!(handle.semantic, verified.manifest().content());
    let listed = lctx_publisher::inspection::list(&config).await.unwrap();
    assert!(listed.contains(&handle));
    lctx_publisher::inspection::audit(&config, &handle, &definitions)
        .await
        .unwrap();
    // Actual effective analyzer drift cannot hide behind the original publication marker.
    let admin = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        handle.database.database.as_str(),
    )
    .await
    .unwrap();
    admin
        .query("DEFINE ANALYZER OVERWRITE lctx_discovery TOKENIZERS class FILTERS uppercase")
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        lctx_publisher::inspection::audit(&config, &handle, &definitions)
            .await
            .is_err()
    );
    let analyzer = definitions.lines().find(|line|line.starts_with("DEFINE ANALYZER lctx_discovery ")).expect("selected discovery analyzer").replacen("DEFINE ANALYZER ","DEFINE ANALYZER OVERWRITE ",1);
    admin
        .query(analyzer)
        .await
        .unwrap()
        .check()
        .unwrap();
    lctx_publisher::inspection::audit(&config, &handle, &definitions)
        .await
        .unwrap();
    admin.invalidate().await.unwrap();
    drop(admin);

    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap();
    let marker: Vec<String> = viewer
        .query(
            "SELECT VALUE handle FROM publication:current",
            surrealdb_vars(),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<SnapshotHandle>(&hex::decode(&marker[0]).unwrap()).unwrap(),
        handle
    );
    // VIEWER can read and cannot modify either graph data or executable definitions.
    let mut denied = viewer
        .client()
        .query("CREATE entity:denied; SELECT VALUE id FROM entity:denied")
        .await
        .unwrap()
        .check()
        .unwrap();
    let changed: Vec<lctx_surrealdb::surrealdb::types::RecordId> = denied.take(1).unwrap();
    assert!(changed.is_empty());
    assert!(
        viewer
            .client()
            .query("DEFINE TABLE forbidden")
            .await
            .unwrap()
            .check()
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
    let restored = lctx_publisher::backup::restore(&config, &backup, &definitions)
        .await
        .unwrap();
    assert_eq!(restored.semantic, handle.semantic);
    assert_ne!(restored.database, handle.database);
    lctx_publisher::inspection::audit(&config, &restored, &definitions)
        .await
        .unwrap();
    assert_eq!(config.selected().unwrap(), handle); // Restore never selects its imported handle.
    restored_derived_excess_is_refused(&config, &restored, &definitions).await;
    let restored_viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        restored.clone(),
    )
    .await
    .unwrap();
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
    if let Some(binary) = &cli {
        select_and_show_cli(binary, &config, &restored, scratch.path());
    } else {
        config.select(&restored).unwrap();
    }
    assert_eq!(serving.selected().unwrap(), restored);
    let new_launch = NativeReader::connect(
        &serving.endpoint,
        &serving.credentials(),
        serving.selected().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(new_launch.handle(), &restored);
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
    new_launch.client().invalidate().await.unwrap();
    drop(new_launch);
    viewer.client().invalidate().await.unwrap();
    drop(viewer);
    lctx_publisher::backup::retire(&config, &handle, true)
        .await
        .unwrap();
    assert!(
        NativeReader::connect(&config.endpoint, &config.viewer_credentials(), handle)
            .await
            .is_err()
    );
    restored_viewer.client().invalidate().await.unwrap();
    drop(restored_viewer);
    std::fs::remove_file(&config.selection).unwrap();
    std::fs::remove_file(config.selection.with_extension("serving.json")).unwrap();
    lctx_publisher::backup::retire(&config, &restored, true)
        .await
        .unwrap();
}
fn surrealdb_vars() -> lctx_surrealdb::surrealdb::types::Variables {
    Default::default()
}
