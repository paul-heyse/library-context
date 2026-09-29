// DORMANT (cutover plan P1.3): not a test target. It drives the removed Delta runtime
// (`snapshot`/`attempt`/`delta`), so it cannot compile until its layer is rebuilt on generations.
// Owner: P4 (catalog). Its known answers stay here as expectations to re-express, never to reuse as-is.
//! Catalog-only publication must not need a brief, flow pass or native semantic closure.
use cpg_schema::{Id, catalog::CompileProfile};
use std::path::Path;
#[path = "catalog/pr4.rs"]
mod pr4;

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
    if std::env::var_os("LCTX_CATALOG_FIXTURE").is_some() {
        let _ = attempt.keep();
        std::fs::write(
            destination.join("STORE"),
            store.to_string_lossy().as_bytes(),
        )
        .unwrap();
    }
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
    let facts = cpg_core::catalog::load_facts(&ctx).await.unwrap();
    let public: Vec<cpg_schema::findings::PublicPathsRow> = cpg_core::sql::fetch(
        &ctx,
        &cpg_schema::public::public_paths(),
        cpg_core::sql::Params::new().texts("roots", ["catalogpkg"]),
    )
    .await
    .unwrap();
    let clean =
        cpg_core::catalog::derive_contracts(&facts, snapshot, &["catalogpkg".into()], &public)
            .unwrap();
    let prepared = cpg_core::catalog::PreparedCatalog::new(facts);
    assert_eq!(
        clean,
        prepared
            .derive(snapshot, &["catalogpkg".into()], &public)
            .unwrap()
    );
    input_change_controls(prepared.facts(), snapshot, &public, &clean);
    let by_name = |name: &str| {
        prepared
            .facts()
            .declarations
            .iter()
            .find(|d| d.qualified_name == name || d.qualified_name.ends_with(&format!(".{name}")))
            .unwrap_or_else(|| {
                panic!(
                    "missing {name}: {:?}",
                    prepared
                        .facts()
                        .declarations
                        .iter()
                        .map(|d| &d.qualified_name)
                        .collect::<Vec<_>>()
                )
            })
            .node_id
    };
    let options = by_name("Options");
    let left = clean
        .configurations
        .iter()
        .find(|f| f.class_node_id == options && f.name == "left")
        .unwrap();
    assert_eq!(left.literal_json.as_deref(), Some("\"http\""));
    assert!(
        clean
            .configurations
            .iter()
            .any(|f| f.class_node_id == options
                && f.name == "cache"
                && f.factory_text.as_deref() == Some("list"))
    );
    let left_read = by_name("Options.read_left");
    let right_read = by_name("Options.read_right");
    let unrelated = by_name("Options.unrelated");
    assert!(clean.field_links.iter().any(|l| l.field_id == left.field_id
        && l.reader_node_id == Some(left_read)
        && l.kind == "exact_reader"));
    assert!(
        !clean
            .field_links
            .iter()
            .any(|l| l.field_id == left.field_id && l.reader_node_id == Some(right_read))
    );
    assert!(
        !clean
            .field_links
            .iter()
            .any(|l| l.reader_node_id == Some(unrelated))
    );
    for name in [
        "CustomAllocation",
        "CustomMeta",
        "ReplacedRecord",
        "DescriptorField",
        "SetterMutation",
        "OperatorMutation",
        "StaticConstructor",
    ] {
        let class = by_name(name);
        assert!(
            !clean
                .field_links
                .iter()
                .any(|l| l.class_node_id == class && l.kind == "exact_storage"),
            "{name}"
        );
    }
    assert!(
        clean
            .field_links
            .iter()
            .any(|l| l.class_node_id == by_name("DirectOptions") && l.kind == "exact_storage")
    );
    let replaced_properties: std::collections::BTreeSet<_> = prepared
        .facts()
        .declarations
        .iter()
        .filter(|d| d.parent_node_id == Some(by_name("ReplacedProperty")))
        .map(|d| d.node_id)
        .collect();
    assert!(
        !clean
            .surfaces
            .iter()
            .any(|s| replaced_properties.contains(&s.declaration_node_id)
                && s.accessor_role.as_deref() == Some("setter"))
    );
    assert!(
        clean
            .surfaces
            .iter()
            .any(|s| s.accessor_role.as_deref() == Some("setter") && s.related_node_id.is_some())
    );
    assert!(
        clean
            .surfaces
            .iter()
            .any(|s| s.protocol.as_deref() == Some("context_manager") && s.admission == "withheld")
    );
    assert!(
        clean
            .surfaces
            .iter()
            .any(|s| s.resolved_target.as_deref() == Some("functools.wraps")
                && s.related_node_id.is_some())
    );
    assert!(
        clean
            .evidence
            .iter()
            .any(|e| e.subject_node_id == left.field_id && e.text.contains("'http'"))
    );
    assert!(clean.surfaces.iter().any(|s| s.declaration_node_id
        == by_name("AliasDescriptor.create")
        && s.binding_mode.as_deref() == Some("class")
        && s.admission == "withheld"));
    assert!(!clean.surfaces.iter().any(|s| s.registration.is_some()));
    // Same immutable source observations, explicit pinned-provider input change. This control
    // tests the normalizer's policy boundary, not execution of FastMCP or an analyzed fixture.
    let mut pilot_facts = prepared.facts().clone();
    pilot_facts.releases[0]
        .distributions
        .push("fastmcp==4.0.5".into());
    let pilot_clean = cpg_core::catalog::derive_contracts(
        &pilot_facts,
        snapshot,
        &["catalogpkg".into()],
        &public,
    )
    .unwrap();
    let pilot_prepared = cpg_core::catalog::PreparedCatalog::new(pilot_facts);
    assert_eq!(
        pilot_clean,
        pilot_prepared
            .derive(snapshot, &["catalogpkg".into()], &public)
            .unwrap()
    );
    for registration in ["tool", "resource", "prompt"] {
        assert!(
            pilot_clean
                .surfaces
                .iter()
                .any(|s| s.registration.as_deref() == Some(registration)
                    && s.admission == "withheld"),
            "{registration}"
        );
    }

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
    pr4::check(&ctx, snapshot, &generation, &destination).await;
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

/// Pure transform controls after one fixture load: no store, embedding or PG calls inside.
fn input_change_controls(
    facts: &cpg_core::catalog::CatalogFacts,
    snapshot: Id,
    public: &[cpg_schema::findings::PublicPathsRow],
    baseline: &cpg_core::catalog::Contracts,
) {
    use cpg_core::catalog::{Contracts, PreparedCatalog, derive_contracts};
    // Canonical publication treats rows as multisets; explicit ordinal fields retain ordering.
    fn rows(c: &Contracts) -> Vec<String> {
        let mut out = Vec::new();
        macro_rules! include {
            ($($field:ident),+) => { $(for row in &c.$field {
                out.push(format!("{}:{row:?}", stringify!($field)));
            })+ };
        }
        include!(
            members,
            constructors,
            bindings,
            signatures,
            parameters,
            evidence,
            types,
            type_args,
            type_observations,
            surfaces,
            configurations,
            field_links
        );
        out.sort();
        out
    }
    let compare = |facts: &cpg_core::catalog::CatalogFacts,
                   roots: &[String],
                   public: &[cpg_schema::findings::PublicPathsRow]| {
        let clean = derive_contracts(facts, snapshot, roots, public).unwrap();
        let indexed = PreparedCatalog::new(facts.clone())
            .derive(snapshot, roots, public)
            .unwrap();
        assert_eq!(rows(&clean), rows(&indexed));
        clean
    };
    let roots = ["catalogpkg".to_owned()];
    let mut shuffled = facts.clone();
    macro_rules! reverse {
        ($($field:ident),+) => { $(shuffled.$field.reverse();)+ };
    }
    reverse!(
        declarations,
        source,
        names,
        signatures,
        parameters,
        syntax,
        semantics,
        docs,
        functions,
        synthetic,
        classes,
        class_map,
        ancestry,
        ancestry_targets,
        scopes,
        lexical_bindings,
        observations,
        terms,
        args,
        candidates,
        member_observations,
        releases,
        field_syntax,
        record_fields,
        nodes,
        references,
        resolutions,
        roots,
        descriptors
    );
    let mut reversed_public = public.to_vec();
    reversed_public.reverse();
    assert_eq!(
        rows(baseline),
        rows(&compare(&shuffled, &roots, &reversed_public))
    );

    // A previously absent public spelling appears, then is removed again. No negative lookup
    // can survive the changed membership. Stable declaration/signature identity is retained.
    let missing = ["catalogpkg.new_alias".to_owned()];
    let empty = compare(facts, &missing, &[]);
    assert!(empty.members.is_empty());
    let mut alias = public
        .iter()
        .find(|p| p.access_path == "catalogpkg.ordinary")
        .unwrap()
        .clone();
    alias.access_path = missing[0].clone();
    let added = compare(facts, &missing, &[alias]);
    assert_eq!(added.members.len(), 1);
    assert_eq!(added.members[0].access_path, missing[0]);
    assert!(!added.signatures.is_empty());
    assert_eq!(rows(&empty), rows(&compare(facts, &missing, &[])));

    // Absent source syntax remains unknown; restoring the observation restores its exact default.
    let parameter = baseline
        .parameters
        .iter()
        .find(|p| p.default_text.is_some() && p.syntax_fact_id.is_some())
        .unwrap();
    let mut absent = facts.clone();
    let syntax_id = parameter.syntax_fact_id.unwrap();
    absent.syntax.retain(|p| p.fact_id != syntax_id);
    for p in &mut absent.parameters {
        if p.syntax_fact_id == Some(syntax_id) {
            p.syntax_fact_id = None;
        }
    }
    let without = compare(&absent, &roots, public);
    let p = without
        .parameters
        .iter()
        .find(|p| p.signature_id == parameter.signature_id && p.ordinal == parameter.ordinal)
        .unwrap();
    assert!(p.default_text.is_none());
    assert_ne!(p.default_state, "literal");
    assert_eq!(rows(baseline), rows(&compare(facts, &roots, public)));

    // Appending whitespace leaves source spans/signatures intact, but every cited file digest
    // must be rebound. This input perturbation is not published as a new canonical snapshot.
    let mut evidence = facts.clone();
    for file in &mut evidence.source {
        if let Some(text) = &mut file.text {
            text.push('\n');
            file.byte_len = text.len() as i64;
            file.content_digest = cpg_schema::id::content_digest(text.as_bytes());
        }
    }
    let refreshed = compare(&evidence, &roots, public);
    assert_eq!(baseline.signatures, refreshed.signatures);
    assert_ne!(baseline.evidence, refreshed.evidence);
    for citation in &refreshed.evidence {
        let prior = baseline
            .evidence
            .iter()
            .find(|e| e.evidence_id == citation.evidence_id)
            .unwrap();
        assert_eq!(prior.text, citation.text);
        assert_ne!(prior.source_digest, citation.source_digest);
    }
}
