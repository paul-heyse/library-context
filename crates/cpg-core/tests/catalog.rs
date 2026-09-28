//! Catalog-only publication must not need a brief, flow pass or native semantic closure.
use cpg_schema::{Id, catalog::CompileProfile};
use std::path::Path;

#[tokio::test]
async fn catalog_profile_publishes_original_contracts_without_native_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/catalog").canonicalize().unwrap();
    let snapshot = Id([47; 16]);
    let extracted = cpg_extract::extract(&cpg_extract::ExtractInput {
        profile: CompileProfile::Catalog,
        release: cpg_extract::Release::from_tree(root, "catalog-fixture").unwrap(),
        venv_root: temp.path().join("venv"), site_packages: vec![site],
        python_version: (3,14,7), python_platform: "linux".into(), snapshot_id: snapshot,
        corpus: None, keep_pysa_json: false, test_hooks: Default::default(),
    }).unwrap();
    let destination = std::env::var_os("LCTX_CATALOG_FIXTURE").map(std::path::PathBuf::from)
        .unwrap_or_else(|| temp.path().to_path_buf());
    std::fs::create_dir_all(&destination).unwrap();
    let attempt = tempfile::tempdir_in(&destination).unwrap();
    let store = attempt.path().join("store");
    let result = cpg_core::attempt::compile_catalog(&store, snapshot, extracted.tables,
        &cpg_core::catalog::CompileInputs { public_roots: vec!["catalogpkg".into()],
            profile: CompileProfile::Catalog, embedder: None, embedding_cache: None }, None).await.unwrap();
    for name in ["flow_values", "summary_flows", "briefs", "behaviors", "model_targets"] {
        assert_eq!(result.rows.iter().find(|r| r.0 == name).unwrap().1, 0, "{name}");
    }
    for name in ["operations", "catalog_members", "catalog_bindings", "catalog_signatures", "catalog_parameters", "catalog_evidence"] {
        assert!(result.rows.iter().find(|r| r.0 == name).unwrap().1 > 0, "{name}");
    }
    let (_, ctx) = cpg_core::snapshot::published(&store, snapshot).await.unwrap().unwrap();
    let check = cpg_core::sql::render(&ctx,
        "SELECT ordinal,name,kind,default_state FROM catalog_parameters WHERE signature_id IN (SELECT signature_id FROM catalog_signatures WHERE callable_node_id IN (SELECT node_id FROM operations WHERE access_path='catalogpkg.ordinary')) ORDER BY ordinal").await.unwrap();
    for value in ["positional_only", "var_positional", "keyword_only", "var_keyword", "literal"] {
        assert!(check.contains(value), "missing {value}: {check}");
    }
    let signatures = cpg_core::sql::render(&ctx, "SELECT role,form,constructor_class_id FROM catalog_signatures").await.unwrap();
    assert!(signatures.contains("provider_constructor"), "{signatures}");
    assert!(signatures.contains("overload"), "{signatures}");
    let aliases = cpg_core::sql::render(&ctx, "SELECT access_path FROM catalog_members ORDER BY access_path").await.unwrap();
    for path in ["catalogpkg.alias", "catalogpkg.undocumented", "catalogpkg.ordinary", "catalogpkg.Config", "catalogpkg.Child.method"] {
        assert!(aliases.contains(path), "{aliases}");
    }
    let dotted: Vec<cpg_schema::findings::PublicPathsRow> = cpg_core::sql::fetch(&ctx,
        &cpg_schema::public::public_paths(), cpg_core::sql::Params::new().texts("roots",
            &["catalogpkg.Child.method".into(), "catalogpkg.Child".into()])).await.unwrap();
    assert!(dotted.iter().any(|p| p.access_path == "catalogpkg.Child.method"));
    assert!(dotted.iter().all(|p| p.access_path == "catalogpkg.Child" || p.access_path.starts_with("catalogpkg.Child.")));
    let generation = cpg_core::bundle::build(&ctx, &destination.join("generations")).await.unwrap();
    std::fs::write(destination.join("CURRENT"), generation.dir.to_string_lossy().as_bytes()).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(generation.dir.join("MANIFEST.json")).unwrap()).unwrap();
    assert_eq!(manifest["capabilities"]["catalog"], true);
    assert_eq!(manifest["capabilities"]["native_value_paths"], false);
    assert!(manifest["projection"]["artifacts"].get("conditions.arrow").is_none());
    assert!(manifest["condition_kernel_format"].is_null());
    // The shared source-equality validator rejects a forged default, even with a valid fact FK.
    let altered = cpg_core::sql::query(&ctx, "SELECT * EXCLUDE(default_text), 'forged()' AS default_text FROM catalog_parameters").await.unwrap().into_view();
    ctx.deregister_table("catalog_parameters").unwrap();
    ctx.register_table("catalog_parameters", altered).unwrap();
    let failures = cpg_core::catalog::validate(&ctx, &cpg_schema::catalog::CatalogCompilationRow {
        snapshot_id: snapshot, profile: "catalog".into(), public_roots: vec!["catalogpkg".into()],
        input_digest: cpg_schema::id::Digest([0;32]),
    }).await.unwrap();
    assert!(failures.iter().any(|f| f.rule == "catalog-source-equality:catalog_parameters"));
}
