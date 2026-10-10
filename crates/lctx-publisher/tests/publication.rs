//! Actual compiled finite fixture, independent export verification, native publication and VIEWER.
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
#[path = "../../cpg-core/tests/fixtures/native.rs"]
mod native_fixture;
#[path = "../src/backup_import.rs"]
mod backup_decode;
use cpg_core::{
    artifact, compilation,
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    admission::Frontier,
    serving::SnapshotHandle,
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
        serde_json::from_value::<SnapshotHandle>(serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["handle"].clone()).unwrap(),
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
        serde_json::from_value::<SnapshotHandle>(serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["handle"].clone()).unwrap(),
        *handle
    );
}
// Derived rows linked to selected anchors are part of this publication's exact cold
// comparison. Unrelated rows in the shared database are deliberately outside its authority.
async fn restored_derived_excess_is_refused(config:&RuntimeConfig,handle:&SnapshotHandle,definitions:&str){
    use lctx_surrealdb::surrealdb::types::{Object,RecordId,Value,Variables};
    let reader=NativeReader::connect(&config.endpoint,&config.writer_credentials(),handle.clone()).await.unwrap();
    let details=lctx_publisher::inspection::show(&reader).await.unwrap();
    let loader=lctx_surrealdb::Loader::for_views(reader.shared_client(),details.bindings.iter().filter(|binding|binding.boundary.is_none()).map(|binding|binding.view.identity).collect());
    let mut response=reader.client().query("SELECT VALUE node.anchor FROM compiler_view_member WHERE view IN $lctx_views AND node.semantic_type='source_artifacts' LIMIT 1").bind(reader.view_bindings()).await.unwrap().check().unwrap();
    let targets:Vec<RecordId>=response.take(0).unwrap();let target=targets.first().unwrap().clone();
    let suffix=format!("{}-{}",handle.publication.hex(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    for table in ["search_api_options","search_documentation_deployment","search_scenario","search_source","vector","lex_occurs","vec_occurs"] {
        let relation=table.ends_with("occurs");let vector=table=="vector"||table=="vec_occurs";
        let source_table=if vector{"vector"}else if relation{"search_source"}else{table};
        let source=RecordId::new(source_table,format!("{suffix}-{table}"));
        let mut endpoint=Object::new();endpoint.insert("id",source.clone());
        if vector {let mut embedding=vec![0f32;1024];embedding[0]=1.;endpoint.insert("encoder_hash",suffix.clone());endpoint.insert("policy_key",suffix.clone());endpoint.insert("library_input",suffix.clone());endpoint.insert("family",3i64);endpoint.insert("full_key",suffix.clone());endpoint.insert("projection_key",format!("{suffix}-{table}"));endpoint.insert("embedding",embedding);}
        else {let text=format!("restored extra discovery row {suffix}-{table}");endpoint.insert("text",text.clone());endpoint.insert("digest",ContentHash::of(text.as_bytes()).0.to_vec());}
        let edge_table=if vector{"vec_occurs"}else{"lex_occurs"};let edge_id=RecordId::new(edge_table,format!("{suffix}-{table}"));
        let mut edge=Object::new();edge.insert("id",edge_id.clone());edge.insert("in",source.clone());edge.insert("out",target.clone());edge.insert("unit_node",target.clone());edge.insert("family",3i64);
        for (field,byte) in [("unit",1i64),("window",2),("part",3),("context",4),("input",5)]{edge.insert(field,vec![byte;16]);}
        for field in ["binding","member","anchor"]{edge.insert(field,Value::Null);}
        for field in ["exact_name","exact_path","exact_option"]{edge.insert(field,"");}
        edge.insert("eligible",true);edge.insert("occurrence_key",format!("{suffix}-{table}"));
        for (name,relation,row) in [(source_table,false,Value::Object(endpoint)),(edge_table,true,Value::Object(edge))] {
            let mut vars=Variables::new();vars.insert("rows",vec![row]);reader.client().query(format!("INSERT {}INTO {name} $rows",if relation{"RELATION "}else{""})).bind(vars).await.unwrap().check().unwrap();
        }
        for _ in 0..2 {assert!(lctx_publisher::reconcile_search(&loader).await.is_err(),"scoped excess {table}");assert!(lctx_publisher::inspection::audit(config,handle,definitions).await.is_err());}
        let mut vars=Variables::new();vars.insert("ids",vec![edge_id,source]);reader.client().query("DELETE $ids").bind(vars).await.unwrap().check().unwrap();
        lctx_publisher::inspection::audit(config,handle,definitions).await.unwrap();
    }
    reader.close().await.unwrap();reader.client().invalidate().await.unwrap();
}

#[tokio::test]
async fn compiled_export_publishes_unselected_and_viewer_is_immutable() {publication_control(false).await;}
#[tokio::test]
#[ignore="requires explicit exclusive maintenance admission"]
async fn actual_installed_analyzer_drift_is_refused(){
    assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),"run through just service maintenance --native-clients");
    publication_control(true).await;
}
async fn publication_control(maintenance:bool) {
    let scratch=tempfile::tempdir().unwrap();
    let mut config=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"))).unwrap();config.selection=scratch.path().join("selected.json");
    let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Normalized).await.unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions::default(),
        native.clone(),
    )
    .unwrap();
    let captured = runtime::capture("catalog_core", Profile::Catalog, workspace.budget());
    let settings=ContentHash::of(format!("native-publication-control-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()).as_bytes());
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
    let restored = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Normalized).await.unwrap();
    let workspace = Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions::default(), restored).unwrap();
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();

    let definitions = lctx_surrealdb::materialization::native_definitions();
    let handle = lctx_publisher::publish(&verified, &config, &definitions)
        .await
        .unwrap();
    assert!(!config.selection.exists());
    assert_eq!(handle.semantic, verified.manifest().content());
    let listed = lctx_publisher::inspection::list(&config).await.unwrap();
    assert!(listed.contains(&handle));
    let inspector=NativeReader::connect(&config.endpoint,&config.writer_credentials(),handle.clone()).await.unwrap();
    let details=lctx_publisher::inspection::show(&inspector).await.unwrap();
    assert_eq!(details.manifest.completed_state,verified.manifest().completed_state);
    assert!(!details.contributions.is_empty());
    assert!(!details.bindings.is_empty());
    lctx_publisher::inspection::audit(&config, &handle, &definitions)
        .await
        .unwrap();
    inspector.close().await.unwrap();inspector.client().invalidate().await.unwrap();
    if maintenance {
        let installer=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("maintenance installer config"))).unwrap();
        let admin=lctx_surrealdb::reader::connect(&installer.endpoint,&installer.writer_credentials(),installer.namespace.as_str(),installer.database.as_str()).await.unwrap();
        admin.query("DEFINE ANALYZER OVERWRITE lctx_discovery TOKENIZERS class FILTERS uppercase").await.unwrap().check().unwrap();
        let refused=lctx_publisher::inspection::audit(&config,&handle,&definitions).await;
        let analyzer=definitions.lines().find(|line|line.starts_with("DEFINE ANALYZER lctx_discovery ")).unwrap().replacen("DEFINE ANALYZER ","DEFINE ANALYZER OVERWRITE ",1);
        admin.query(analyzer).await.unwrap().check().unwrap();
        admin.invalidate().await.unwrap();
        assert!(refused.is_err(),"actual analyzer drift must be refused");
        lctx_publisher::inspection::audit(&config,&handle,&definitions).await.unwrap();
    }
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap();
    let marker: Vec<String> = viewer
        .query(
            format!("SELECT VALUE handle FROM publication:`{}`",handle.publication.hex()),
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
    let mut response=viewer.client().query("SELECT * FROM entity WHERE id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) LIMIT 1").bind(viewer.view_bindings()).await.unwrap().check().unwrap();
    let mut rows:Vec<lctx_surrealdb::surrealdb::types::Value>=response.take(0).unwrap();
    let lctx_surrealdb::surrealdb::types::Value::Object(ref mut row)=rows[0] else{panic!("canonical payload")};
    row.insert("canonical",lctx_surrealdb::surrealdb::types::Bytes::from(b"foreign canonical bytes".to_vec()));
    assert!(lctx_surrealdb::control::ensure_rows(viewer.client(),None,rows).await.is_err());
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
    omitted_backup_payloads_are_refused(&config,&handle,&backup,&definitions).await;
    let restored = lctx_publisher::backup::restore_publication(&config, &backup, handle.publication, &definitions)
        .await
        .unwrap();
    assert_eq!(restored.semantic, handle.semantic);
    assert_eq!(restored.database,handle.database);
    assert_eq!(restored,handle,"restored immutable publication identity is content-based");
    lctx_publisher::inspection::audit(&config, &restored, &definitions)
        .await
        .unwrap();
    assert_eq!(config.selected().unwrap(), handle); // Restore never selects its imported handle.
    restored_derived_excess_is_refused(&config, &restored, &definitions).await;
    let restored_viewer = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
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
    viewer.close().await.unwrap();viewer.client().invalidate().await.unwrap();
    restored_viewer.close().await.unwrap();restored_viewer.client().invalidate().await.unwrap();
    std::fs::remove_file(&config.selection).unwrap();
    std::fs::remove_file(config.selection.with_extension("serving.json")).unwrap();
    lctx_publisher::backup::retire(&config,&handle,true).await.unwrap();
    assert!(NativeReader::connect(&config.endpoint,&config.writer_credentials(),handle).await.is_err());
}
fn surrealdb_vars()->lctx_surrealdb::surrealdb::types::Variables{Default::default()}

// Omitted content must be refused even though the identical publication already exists
// in this shared store. An intact same-store round trip alone cannot establish that.
async fn omitted_backup_payloads_are_refused(config:&RuntimeConfig,handle:&SnapshotHandle,backup:&std::path::Path,definitions:&str){
    use std::io::Write;
    use lctx_surrealdb::surrealdb::types::ToSql;
    for table in ["entity","assertion","compiler_record"] {
        let input=backup.with_file_name(format!("omitted-{table}.surql"));
        let mut altered=std::io::BufWriter::new(std::fs::File::create(&input).unwrap());
        writeln!(altered,"OPTION IMPORT;").unwrap();
        let mut dump=backup_decode::DataDump::new(std::fs::File::open(backup).unwrap());let mut removed=0;
        while let Some(item)=dump.next().unwrap() {
            match item {
                backup_decode::Item::Definition(definition)=>writeln!(altered,"{definition};").unwrap(),
                backup_decode::Item::Rows(rows)=>{
                    assert_eq!(rows.len(),1,"managed native export must emit one bounded record per statement");
                    let mut kept=Vec::new();
                    for row in rows {
                        let omit=matches!(&row,lctx_surrealdb::surrealdb::types::Value::Object(object) if matches!(object.get("id"),Some(lctx_surrealdb::surrealdb::types::Value::RecordId(id)) if id.table.as_str()==table));
                        if omit{removed+=1;}else{kept.push(row);}
                    }
                    if !kept.is_empty(){writeln!(altered,"INSERT {};",lctx_surrealdb::surrealdb::types::Value::Array(kept.into()).to_sql()).unwrap();}
                },
            }
        }
        altered.flush().unwrap();drop(altered);
        if removed==0{continue;}
        let error=lctx_publisher::backup::restore_publication(config,&input,handle.publication,definitions).await.unwrap_err();
        assert!(error.to_string().contains("restore selected payload absent from dump"),"{table}: {error}");
    }
}
