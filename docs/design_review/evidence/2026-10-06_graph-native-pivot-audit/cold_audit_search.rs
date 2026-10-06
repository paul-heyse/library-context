//! Actual admitted Catalog compilation, native publication, ten tools and original bytes.
#[path = "../../../../crates/cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact,
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{admission::Frontier, serving::*, stages::Profile, *};
use lctx_surrealdb::{RuntimeConfig, reader};
use std::sync::Arc;
const LIBRARY: &str = "synthesis-sources";
/// The source bytes remain the captured fixture. Explicit distribution ownership is the
/// library admission premise; a labelled, unowned tree supplies no such authority.
fn library_fixture(
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    use cpg_extract::{
        acquisition::{
            AcquiredInput, Acquisition, InventoryDistribution, InventoryFile, LibraryInventory,
            derive_blocks,
        },
        bundle::CapturedInputs,
        capture::CapturedInput,
    };
    let tree = runtime::capture("synthesis_sources", Profile::Catalog, budget);
    let original = tree.inputs()[0].captured();
    let derived = original
        .derivations()
        .iter()
        .map(|d| d.path())
        .collect::<std::collections::BTreeSet<_>>();
    let paths = original
        .artifacts()
        .iter()
        .filter(|a| !derived.contains(a.path.as_str()))
        .map(|a| a.path.clone())
        .collect::<Vec<_>>();
    let documents = paths
        .iter()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    let captured =
        CapturedInput::capture_derived(original.root(), &paths, budget, &documents, derive_blocks)
            .unwrap();
    let files = paths
        .into_iter()
        .filter_map(|path| {
            admission::ArtifactClass::of(&path).map(|class| InventoryFile {
                path,
                owners: vec![LIBRARY.into()],
                role: match class {
                    admission::ArtifactClass::PythonSource => input::SourceRole::Release,
                    admission::ArtifactClass::Document => input::SourceRole::Document,
                },
                record_sha256: None,
            })
        })
        .collect();
    let inventory = LibraryInventory {
        name: LIBRARY.into(),
        requirement: format!("{LIBRARY}==0.0.0"),
        lock_digest: ContentHash::of(b"native-serving-first-party-fixture"),
        installer: None,
        python_version: "3.14.7".into(),
        platform: "linux".into(),
        site_packages: captured.root().to_owned(),
        distributions: vec![InventoryDistribution {
            name: LIBRARY.into(),
            version: "0.0.0".into(),
            first_party: true,
            artifact_sha256: vec![],
            record_digest: captured.revision().manifest,
        }],
        files,
        configuration: ContentHash::of(b"native-serving-first-party-fixture/v1"),
    };
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::new(
            captured,
            Acquisition::Installed(inventory),
        )],
        tree.config().clone(),
    ))
}
#[tokio::main]
async fn main() {
    let file = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("owned disposable SurrealDB fixture required");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
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
    let captured = library_fixture(workspace.budget());
    let settings = ContentHash::of(b"native-serving-journey");
    let prepared = PreparedCompilation::new(
        Frontier::Catalog,
        runtime::settings("api"),
        captured.config().catalog(),
        None,
        workspace.budget(),
    )
    .unwrap();
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        settings,
        Frontier::Catalog,
        Some(&prepared),
        None,
        None,
    )
    .await
    .unwrap();
    drop(prepared);
    let admitted = artifact::admit(
        &workspace,
        &captured,
        Frontier::Catalog,
        Profile::Catalog,
        settings,
    )
    .await
    .unwrap();
    let export = scratch.path().join("export");
    admitted.export(&export).unwrap();
    drop(admitted);
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();
    let config = RuntimeConfig {
        endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
        username: fixture["admin_user"].as_str().unwrap().into(),
        password: fixture["admin_password"].as_str().unwrap().into(),
        viewer_username: "serving_fixture".into(),
        viewer_password: format!("serving-fixture-{}", std::process::id()),
        namespace: Name::new("gn_serving_journeys").unwrap(),
        cache_database: Name::new("cache").unwrap(),
        selection: scratch.path().join("selection.json"),
    };
    let handle = lctx_publisher::publish(&verified, &config, &lctx_serving::native_definitions())
        .await
        .unwrap();
    assert!(!config.selection.exists());
    let definitions = lctx_serving::native_definitions();
    lctx_publisher::inspection::audit(&config, &handle, &definitions).await.unwrap();
    let admin = reader::connect(&config.endpoint, &config.root_credentials(), config.namespace.as_str(), handle.database.database.as_str()).await.unwrap();
    let mut result = admin.query("SELECT VALUE text FROM search_api_options").await.unwrap().check().unwrap();
    let original_texts: Vec<String> = result.take(0).unwrap();
    assert!(!original_texts.is_empty(), "Probe must mutate actual derived search rows");
    admin.query("UPDATE search_api_options SET text='audit_corruption_unrelated_document' RETURN NONE").await.unwrap().check().unwrap();
    let mut result = admin.query("SELECT VALUE text FROM search_api_options").await.unwrap().check().unwrap();
    let changed_texts: Vec<String> = result.take(0).unwrap();
    assert!(changed_texts.iter().all(|s| s == "audit_corruption_unrelated_document"));
    assert_ne!(original_texts, changed_texts);
    let audit_accepted = lctx_publisher::inspection::audit(&config, &handle, &definitions).await.is_ok();
    println!("{}", serde_json::json!({"probe":"cold_audit_search_corruption", "changed_rows":changed_texts.len(),"canonical_content_unchanged":true,"audit_accepted":audit_accepted,"defect_reproduced":audit_accepted}));
    assert!(audit_accepted, "The anticipated defect was not reproduced; revisit the finding");
}
