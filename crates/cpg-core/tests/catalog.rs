//! Catalog-only publication must not need a brief, flow pass or native semantic closure.
use cpg_schema::{Id, catalog::CompileProfile};
use std::path::Path;

#[tokio::test]
async fn catalog_profile_publishes_original_contracts_without_native_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/catalog")
        .canonicalize()
        .unwrap();
    let snapshot = Id([47; 16]);
    let extracted = cpg_extract::extract(&cpg_extract::ExtractInput {
        profile: CompileProfile::Catalog,
        release: cpg_extract::Release::from_tree(root, "catalog-fixture").unwrap(),
        venv_root: temp.path().join("venv"),
        site_packages: vec![site],
        python_version: (3, 14, 7),
        python_platform: "linux".into(),
        snapshot_id: snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap();
    let destination = std::env::var_os("LCTX_CATALOG_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temp.path().to_path_buf());
    std::fs::create_dir_all(&destination).unwrap();
    let attempt = tempfile::tempdir_in(&destination).unwrap();
    let store = attempt.path().join("store");
    let result = cpg_core::attempt::compile_catalog(
        &store,
        snapshot,
        extracted.tables,
        &cpg_core::catalog::CompileInputs {
            public_roots: vec!["catalogpkg".into()],
            profile: CompileProfile::Catalog,
            embedder: None,
            embedding_cache: None,
        },
        None,
    )
    .await
    .unwrap();
    for name in [
        "flow_values",
        "summary_flows",
        "briefs",
        "behaviors",
        "model_targets",
    ] {
        assert_eq!(
            result.rows.iter().find(|r| r.0 == name).unwrap().1,
            0,
            "{name}"
        );
    }
    for name in [
        "operations",
        "catalog_members",
        "catalog_bindings",
        "catalog_signatures",
        "catalog_parameters",
        "catalog_evidence",
    ] {
        assert!(
            result.rows.iter().find(|r| r.0 == name).unwrap().1 > 0,
            "{name}"
        );
    }
    let (_, ctx) = cpg_core::snapshot::published(&store, snapshot)
        .await
        .unwrap()
        .unwrap();
    let check = cpg_core::sql::render(&ctx,
        "SELECT ordinal,name,kind,default_state FROM catalog_parameters WHERE signature_id IN (SELECT signature_id FROM catalog_signatures WHERE callable_node_id IN (SELECT node_id FROM operations WHERE access_path='catalogpkg.ordinary')) ORDER BY ordinal").await.unwrap();
    for value in [
        "positional_only",
        "var_positional",
        "keyword_only",
        "var_keyword",
        "literal",
    ] {
        assert!(check.contains(value), "missing {value}: {check}");
    }
    let signatures = cpg_core::sql::render(
        &ctx,
        "SELECT role,form,constructor_class_id FROM catalog_signatures",
    )
    .await
    .unwrap();
    assert!(signatures.contains("provider_constructor"), "{signatures}");
    assert!(signatures.contains("overload"), "{signatures}");
    let aliases = cpg_core::sql::render(
        &ctx,
        "SELECT access_path FROM catalog_members ORDER BY access_path",
    )
    .await
    .unwrap();
    for path in [
        "catalogpkg.alias",
        "catalogpkg.undocumented",
        "catalogpkg.ordinary",
        "catalogpkg.Config",
        "catalogpkg.Child.method",
        "catalogpkg.variants.Shared.source_member",
        "catalogpkg.variants.Shared.stub_member",
        "catalogpkg.variants.Shared.inherited_stub",
        "catalogpkg.variants.Rebound.earlier",
    ] {
        assert!(aliases.contains(path), "{aliases}");
    }
    let dotted: Vec<cpg_schema::findings::PublicPathsRow> = cpg_core::sql::fetch(
        &ctx,
        &cpg_schema::public::public_paths(),
        cpg_core::sql::Params::new()
            .texts("roots", ["catalogpkg.Child.method", "catalogpkg.Child"]),
    )
    .await
    .unwrap();
    assert!(
        dotted
            .iter()
            .any(|p| p.access_path == "catalogpkg.Child.method")
    );
    assert!(
        dotted.iter().all(|p| p.access_path == "catalogpkg.Child"
            || p.access_path.starts_with("catalogpkg.Child."))
    );
    for path in ["catalogpkg.ChildConfig", "catalogpkg.PublicConfig"] {
        let roots = vec![path.to_owned()];
        let paths = cpg_core::sql::fetch(
            &ctx,
            &cpg_schema::public::public_paths(),
            cpg_core::sql::Params::new().texts("roots", &roots),
        )
        .await
        .unwrap();
        let contracts = cpg_core::catalog::contracts(&ctx, snapshot, &roots, &paths)
            .await
            .unwrap();
        let class = contracts
            .members
            .iter()
            .find(|m| m.access_path == path)
            .unwrap()
            .operation_node_id
            .unwrap();
        assert!(
            contracts
                .constructors
                .iter()
                .any(|c| c.class_node_id == class && !c.own && c.ancestry_fact_id.is_some()),
            "{path}: {:?}",
            contracts.constructors
        );
        assert!(
            contracts
                .signatures
                .iter()
                .any(|s| s.role == "provider_constructor"),
            "{path}"
        );
        assert!(
            contracts
                .members
                .iter()
                .all(|m| m.access_path == path || m.access_path.starts_with(&format!("{path}.")))
        );
    }
    let generation = cpg_core::bundle::build(&ctx, &destination.join("generations"))
        .await
        .unwrap();
    std::fs::write(
        destination.join("CURRENT"),
        generation.dir.to_string_lossy().as_bytes(),
    )
    .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.dir.join("MANIFEST.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["capabilities"]["catalog"], true);
    assert_eq!(manifest["capabilities"]["native_value_paths"], false);
    assert!(
        manifest["projection"]["artifacts"]
            .get("conditions.arrow")
            .is_none()
    );
    assert!(manifest["condition_kernel_format"].is_null());
    assert_eq!(cpg_core::bundle::verify(&generation.dir).unwrap(), manifest);
    // Exercise each generated catalog rule family and the optional-rendering invariant over
    // the production validators. Restore the original provider after each forged relation.
    for (rule_name, table, query) in [
        (
            "catalog-code:catalog_members.brief_status",
            "catalog_members",
            "SELECT * EXCLUDE(brief_status), 'invented' AS brief_status FROM catalog_members",
        ),
        (
            "semantic:catalog-brief-state",
            "catalog_members",
            "SELECT * EXCLUDE(brief_status), 'available' AS brief_status FROM catalog_members",
        ),
        (
            "catalog-unselected:summary_boundaries",
            "summary_boundaries",
            "SELECT snapshot_id, operation_node_id AS function_node_id, operation_node_id AS parameter_node_id, operation_node_id AS source_flow_fact_id, operation_node_id AS source_origin_id, operation_node_id AS condition_id, CAST(0 AS SMALLINT) AS reason, false AS local_through_call, false AS upstream_through_call, false AS raw_approximated FROM catalog_members WHERE operation_node_id IS NOT NULL LIMIT 1",
        ),
    ] {
        let forged = cpg_core::sql::query(&ctx, query).await.unwrap().into_view();
        let original = ctx.deregister_table(table).unwrap().unwrap();
        ctx.register_table(table, forged).unwrap();
        let rule = cpg_schema::rules::rules()
            .into_iter()
            .find(|r| r.name == rule_name)
            .unwrap();
        let bad = cpg_core::sql::query(&ctx, &rule.sql)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert!(
            bad.iter().map(|b| b.num_rows()).sum::<usize>() > 0,
            "{rule_name}"
        );
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
    // The shared source-equality validator rejects a forged default, even with a valid fact FK.
    let altered = cpg_core::sql::query(
        &ctx,
        "SELECT * EXCLUDE(default_text), 'forged()' AS default_text FROM catalog_parameters",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("catalog_parameters").unwrap();
    ctx.register_table("catalog_parameters", altered).unwrap();
    let failures = cpg_core::catalog::validate(
        &ctx,
        &cpg_schema::catalog::CatalogCompilationRow {
            snapshot_id: snapshot,
            profile: "catalog".into(),
            public_roots: vec!["catalogpkg".into()],
            input_digest: cpg_schema::id::Digest([0; 32]),
        },
    )
    .await
    .unwrap();
    assert!(
        failures
            .iter()
            .any(|f| f.rule == "catalog-source-equality:catalog_parameters")
    );
}
