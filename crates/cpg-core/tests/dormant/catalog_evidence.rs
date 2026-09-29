// DORMANT (cutover plan P1.3): not a test target. It drives the removed Delta runtime
// (`snapshot`/`attempt`/`delta`), so it cannot compile until its layer is rebuilt on generations.
// Owner: P4 (catalog evidence). Its known answers stay here as expectations to re-express, never to reuse as-is.
//! Original evidence is useful without brief seeds and survives canonical publication.
#[path = "catalog/selection_evidence.rs"]
mod selection_evidence;
use cpg_schema::{Id, Table, catalog::CompileProfile, evidence::*, query::QueryRow};
use std::path::Path;
async fn rows<T: Table>(ctx: &datafusion::prelude::SessionContext) -> Vec<T::Row>
where
    T::Row: QueryRow,
{
    cpg_core::sql::fetch(
        ctx,
        &cpg_schema::query::Relation {
            name: T::NAME,
            deps: T::DEPS,
            sql: format!("SELECT * FROM {}", T::NAME),
        },
        cpg_core::sql::Params::new(),
    )
    .await
    .unwrap()
}
async fn compiled() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Id,
    datafusion::prelude::SessionContext,
) {
    let temp = tempfile::tempdir().unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/catalog_evidence")
        .canonicalize()
        .unwrap();
    // Corpus materialization writes only into a disposable copy of fixture source.
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let dst = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy(&path, &dst);
            } else {
                std::fs::copy(path, dst).unwrap();
            }
        }
    }
    copy(&root.join("corpus"), &temp.path().join("corpus"));
    let snapshot = Id([93; 16]);
    let mut input = cpg_extract::ExtractInput {
        profile: CompileProfile::Catalog,
        release: cpg_extract::Release::from_tree(root.join("release"), "pr3").unwrap(),
        venv_root: temp.path().join("venv"),
        site_packages: vec![site],
        python_version: (3, 14, 7),
        python_platform: "linux".into(),
        snapshot_id: snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    };
    // Add the fixture library to the explicit corpus dependency path.
    input.site_packages.push(root.join("release"));
    // A long Unicode paragraph exercises continuation through original UTF-8 boundaries.
    std::fs::write(
        temp.path().join("corpus/docs/large.mdx"),
        format!("# Long\n\n{}", "λ🦀\n".repeat(5000)),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("corpus/docs/large-launch.mdx"),
        format!("# Launch\n\n```sh\n{}\n```\n", "echo λ\n".repeat(25000)),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("corpus/examples/large.py"),
        format!("from pr3pkg import run\nrun(\"{}\")\n", "λ".repeat(70000)),
    )
    .unwrap();
    input.corpus = Some(
        cpg_extract::library::corpus(
            &temp.path().join("corpus"),
            &cpg_extract::library::Source {
                repository: "https://example.invalid/pr3".into(),
                tag: "v1".into(),
                commit: "a".repeat(40),
                documents: vec!["docs/*.mdx".into()],
                documents_exclude: vec![],
                examples: vec!["examples/*.py".into()],
                examples_exclude: vec![],
                tests: vec!["tests/**/*.py".into()],
                tests_exclude: vec![],
                assets: vec![],
                assets_exclude: vec![],
            },
            &input,
        )
        .unwrap(),
    );
    let extracted = cpg_extract::extract(&input).unwrap();
    let store = temp.path().join("store");
    cpg_core::attempt::compile_catalog(
        &store,
        snapshot,
        extracted.tables,
        &cpg_core::catalog::CompileInputs {
            public_roots: vec!["pr3pkg".into()],
            profile: CompileProfile::Catalog,
            embedder: None,
            embedding_cache: None,
        },
        None,
    )
    .await
    .unwrap();
    let (_, ctx) = cpg_core::snapshot::published(&store, snapshot)
        .await
        .unwrap()
        .unwrap();
    (temp, store, snapshot, ctx)
}
#[tokio::test]
async fn original_contexts_independent_roots_and_pure_rebuild() {
    let (_temp, _store, snapshot, ctx) = compiled().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/catalog_evidence");
    let artifacts = rows::<CatalogArtifacts>(&ctx).await;
    let spans = rows::<CatalogSpans>(&ctx).await;
    let scenarios = rows::<CatalogScenarios>(&ctx).await;
    let associations = rows::<CatalogAssociations>(&ctx).await;
    assert!(!scenarios.is_empty());
    let docs = artifacts
        .iter()
        .find(|a| a.path == "docs/example.mdx")
        .unwrap();
    assert_eq!(
        docs.body.0,
        std::fs::read(root.join("corpus/docs/example.mdx")).unwrap()
    );
    assert_eq!(docs.alignment, "mapped_with_evidence");
    let fence = spans
        .iter()
        .find(|s| s.artifact_id == docs.artifact_id && s.extraction == "original_document_fence")
        .unwrap();
    assert!(
        std::str::from_utf8(&docs.body.0[fence.start_byte as usize..fence.end_byte as usize])
            .unwrap()
            .contains("```")
    );
    assert!(
        associations.iter().any(|a| a.intent == "expected_failure"),
        "{associations:?}"
    );
    assert!(associations.iter().any(|a| a.intent == "assertion_test"));
    assert!(associations.iter().any(|a| a.intent == "skip_xfail"));
    assert!(scenarios.iter().any(|s| {
        serde_json::from_str::<ScenarioDetail>(&s.detail)
            .unwrap()
            .requirements
            .iter()
            .any(|r| r.expression == "fixture")
    }));
    let facts = cpg_core::catalog::load_facts(&ctx).await.unwrap();
    let inputs = cpg_core::evidence::load(&ctx).await.unwrap();
    let public = cpg_core::sql::fetch(
        &ctx,
        &cpg_schema::public::public_paths(),
        cpg_core::sql::Params::new().texts("roots", ["pr3pkg"]),
    )
    .await
    .unwrap();
    let catalog =
        cpg_core::catalog::derive_contracts(&facts, snapshot, &["pr3pkg".into()], &public).unwrap();
    let original = cpg_core::evidence::PreparedEvidence::new(&facts, &inputs)
        .derive(snapshot, &catalog)
        .unwrap();
    let broken = facts
        .source
        .iter()
        .find(|s| s.path == "tests/_invalid/partial.py")
        .unwrap();
    assert!(scenarios.iter().any(|s| {
        let d: ScenarioDetail = serde_json::from_str(&s.detail).unwrap();
        d.analysis_module == Some(broken.module_node_id) && d.checks.parse == CheckStatus::Failed
    }));
    assert!(scenarios.iter().any(|s| {
        let d: ScenarioDetail = serde_json::from_str(&s.detail).unwrap();
        d.intent == Intent::Mixed
    }));
    for (expression, intent) in [
        ("run(\"header\")", "assertion_test"),
        ("run(\"protected\")", "expected_failure"),
        ("run(\"deferred\")", "assertion_test"),
        ("run(\"unit-bad\")", "assertion_test"),
        ("run(\"generator-deferred\")", "assertion_test"),
        ("run(\"generator-eager\")", "expected_failure"),
        ("run(\"generator-filter\")", "assertion_test"),
        ("run(\"overridden\")", "assertion_test"),
    ] {
        let source = facts
            .source
            .iter()
            .find(|s| s.path == "tests/test_scope.py")
            .unwrap();
        let start = source.text.as_ref().unwrap().find(expression).unwrap() as i64;
        assert!(
            associations.iter().any(|a| a.intent == intent
                && serde_json::from_str::<Vec<AssociationSupport>>(&a.support)
                    .unwrap()
                    .iter()
                    .any(|s| s.analysis_module == source.module_node_id && s.start_byte == start)),
            "{expression}: expected {intent}"
        );
    }
    let site = associations
        .iter()
        .find(|a| a.basis == "resolved_target")
        .unwrap();
    let supported: Vec<AssociationSupport> = serde_json::from_str(&site.support).unwrap();
    let fact = supported[0].fact_id;
    let mut candidate = inputs.clone();
    candidate
        .facts
        .iter_mut()
        .find(|f| f.fact_id == fact)
        .unwrap()
        .modality = cpg_schema::codebook::Modality::Candidate;
    let candidate = cpg_core::evidence::PreparedEvidence::new(&facts, &candidate)
        .derive(snapshot, &catalog)
        .unwrap();
    assert!(
        candidate
            .associations
            .iter()
            .filter(|a| a.site_id == site.site_id && a.evidence_kind == "scenario")
            .all(|a| a.basis == "candidate_targets")
    );

    // A release package declaration is one scoped association regardless of API count.
    let mut metadata_facts = facts.clone();
    let release = metadata_facts
        .source
        .iter()
        .find(|s| s.role == cpg_schema::codebook::SourceRole::Release)
        .unwrap()
        .release_id;
    metadata_facts
        .releases
        .iter_mut()
        .find(|r| r.release_id == release)
        .unwrap()
        .distributions = vec!["pr3pkg==1".into()];
    let bytes =
        b"Metadata-Version: 2.5\nName: pr3pkg\nVersion: 1\nRequires-Dist: dependency[server]>=1\n";
    let digest = cpg_schema::id::content_digest(bytes);
    let mut with_metadata = inputs.clone();
    with_metadata.captured.push(CapturedArtifactsRow {
        snapshot_id: snapshot,
        artifact_id: artifact_id(release, "pr3pkg-1.dist-info/METADATA", digest),
        release_id: release,
        context_id: Id::ZERO,
        path: "pr3pkg-1.dist-info/METADATA".into(),
        source_kind: "distribution_metadata".into(),
        source_digest: digest,
        byte_len: bytes.len() as i64,
        body: cpg_schema::column::Blob(bytes.to_vec()),
        alignment: "exact".into(),
        provenance: "explicit fixture metadata input".into(),
        observations: serde_json::to_string(&cpg_extract::metadata::metadata(bytes)).unwrap(),
    });
    let normalized = cpg_core::evidence::PreparedEvidence::new(&metadata_facts, &with_metadata)
        .derive(snapshot, &catalog)
        .unwrap();
    selection_evidence::check(
        &metadata_facts,
        &with_metadata,
        &catalog,
        &normalized,
        snapshot,
    );
    let scoped: Vec<_> = normalized
        .associations
        .iter()
        .filter(|a| a.release_id == Some(release))
        .collect();
    assert_eq!(scoped.len(), 4);
    assert!(
        scoped
            .iter()
            .all(|a| a.member_id.is_none() && a.basis == "release_distribution")
    );

    let mut malformed = normalized.associations.clone();
    malformed
        .iter_mut()
        .find(|a| a.release_id == Some(release))
        .unwrap()
        .role = "invokes".into();
    assert!(
        cpg_schema::evidence::validate(
            &normalized.artifacts,
            &normalized.spans,
            &normalized.scenarios,
            &normalized.deployments,
            &malformed
        )
        .is_err()
    );

    let mut reordered = inputs.clone();
    reordered.blocks.reverse();
    reordered.mentions.reverse();
    reordered.edges.reverse();
    let mut reordered_facts = facts.clone();
    reordered_facts.lexical_bindings.reverse();
    reordered_facts.nodes.reverse();
    assert_eq!(
        original,
        cpg_core::evidence::PreparedEvidence::new(&reordered_facts, &reordered)
            .derive(snapshot, &catalog)
            .unwrap()
    );
    let member = catalog
        .members
        .iter()
        .find(|m| m.access_path == "pr3pkg.unseeded")
        .unwrap();
    assert!(
        associations
            .iter()
            .any(|a| a.member_id == Some(member.member_id))
    );
}

#[tokio::test]
#[ignore = "explicit PostgreSQL evidence functional check"]
async fn postgres_original_bytes_bounded_pages_and_typed_refs() {
    use cpg_core::postgres::{
        Config,
        serving::{Role, RoleConfig, TEST_IMAGE},
    };
    use testcontainers_modules::{
        postgres::Postgres,
        testcontainers::{ImageExt, runners::AsyncRunner},
    };
    let (name, tag) = TEST_IMAGE.trim().split_once(':').unwrap();
    let db = Postgres::default()
        .with_fsync_enabled()
        .with_name(name)
        .with_tag(tag)
        .start()
        .await
        .unwrap();
    let port = db.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let admin = sqlx::PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_app LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_migrator LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'; GRANT CREATE ON DATABASE postgres TO lctx_migrator; GRANT CREATE ON SCHEMA public TO lctx_migrator; GRANT TEMP ON DATABASE postgres TO lctx_importer; CREATE SCHEMA lctx_ext; REVOKE ALL ON SCHEMA lctx_ext FROM PUBLIC; CREATE EXTENSION vector WITH SCHEMA lctx_ext VERSION '0.8.6'; GRANT USAGE ON SCHEMA lctx_ext TO lctx_app,lctx_migrator,lctx_importer,lctx_serving;").execute(&admin).await.unwrap();
    let config = Config {
        application_url: url("lctx_app"),
        migration_url: url("lctx_migrator"),
        migration_config: None,
        max_connections: 2,
        acquire_timeout_seconds: 2,
        statement_timeout_seconds: 30,
        lock_timeout_seconds: 2,
        max_receipt_bytes: 268435456,
    };
    let migrator = config.connect_migrator().await.unwrap();
    migrator.migrate().await.unwrap();
    migrator.check().await.unwrap();
    migrator.close().await;
    let (temp, store, snapshot, ctx) = compiled().await;
    let generation = cpg_core::bundle::bundle(&store, snapshot, &temp.path().join("generations"))
        .await
        .unwrap();
    let source = cpg_core::postgres::import::Source::open(&generation.dir).unwrap();
    let mut role = RoleConfig {
        format: 1,
        role: Role::Importer,
        url: url("lctx_importer"),
        max_connections: 2,
        provider_connections: 0,
        acquire_timeout_seconds: 2,
        statement_timeout_seconds: 30,
        lock_timeout_seconds: 2,
    };
    let importer = role.open_importer().await.unwrap();
    importer
        .import(source.clone(), temp.path().join("artifacts"))
        .await
        .unwrap();
    role.role = Role::Serving;
    role.url = url("lctx_serving");
    let serving = role.open_serving().await.unwrap();
    let pinned = serving
        .pin(
            &source.manifest().context.library,
            Some(source.generation()),
            None,
        )
        .await
        .unwrap();
    let artifacts = rows::<CatalogArtifacts>(&ctx).await;
    let spans = rows::<CatalogSpans>(&ctx).await;
    let artifact = artifacts
        .iter()
        .find(|a| a.path == "docs/large.mdx")
        .unwrap();
    let span = spans
        .iter()
        .filter(|s| s.artifact_id == artifact.artifact_id)
        .max_by_key(|s| s.end_byte - s.start_byte)
        .unwrap();
    let reference = EvidenceRef::new(EvidenceKind::Span, span.span_id);
    let mut cursor = None;
    let mut collected = String::new();
    let mut pages = 0;
    loop {
        let page = serving
            .get_evidence(
                &pinned,
                &snapshot.hex(),
                reference.clone(),
                cursor.as_deref(),
                false,
            )
            .await
            .unwrap();
        let encoded =
            cpg_schema::wire::tool_result("get_evidence", &page.to_string(), false).unwrap();
        assert!(encoded.len() <= 32768);
        let text = page["content"][0]["text"].as_str().unwrap();
        collected.push_str(text);
        pages += 1;
        cursor = page["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
        assert!(
            serving
                .get_evidence(
                    &pinned,
                    &snapshot.hex(),
                    EvidenceRef::new(EvidenceKind::Scenario, span.span_id),
                    cursor.as_deref(),
                    false
                )
                .await
                .is_err()
        );
        assert!(
            serving
                .get_evidence(
                    &pinned,
                    &snapshot.hex(),
                    reference.clone(),
                    cursor.as_deref(),
                    true
                )
                .await
                .is_err()
        );
    }
    assert!(pages > 1);
    assert_eq!(
        collected.as_bytes(),
        &artifact.body.0[span.start_byte as usize..span.end_byte as usize]
    );
    let packet = serving
        .operation_packet(
            &pinned,
            &serde_json::from_value(serde_json::json!({
                "snapshot_id":snapshot.hex(),"operation":"pr3pkg.run",
                "view":{"kind":"section","section":"evidence"}
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    cpg_schema::wire::tool_result("get_operation", &packet.to_string(), false).unwrap();
    assert!(!packet["items"].as_array().unwrap().is_empty());
    let reference: EvidenceRef = serde_json::from_value(
        packet["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["record"]["evidence"]["kind"] == "scenario")
            .unwrap()["record"]["evidence"]
            .clone(),
    )
    .unwrap();
    let page = serving
        .get_evidence(&pinned, &snapshot.hex(), reference, None, false)
        .await
        .unwrap();
    assert_eq!(page["scenario"]["checks"]["execution"], "not_run");
    for path in ["examples/large.py", "docs/large-launch.mdx"] {
        let artifact = artifacts.iter().find(|a| a.path == path).unwrap();
        let span_ids: std::collections::BTreeSet<_> = spans
            .iter()
            .filter(|s| s.artifact_id == artifact.artifact_id)
            .map(|s| s.span_id)
            .collect();
        let reference = if path.ends_with(".py") {
            let scenario = rows::<CatalogScenarios>(&ctx)
                .await
                .into_iter()
                .find(|s| span_ids.contains(&s.primary_span_id))
                .unwrap();
            EvidenceRef::new(EvidenceKind::Scenario, scenario.scenario_id)
        } else {
            let deployment = rows::<CatalogDeployments>(&ctx)
                .await
                .into_iter()
                .find(|d| span_ids.contains(&d.span_id))
                .unwrap();
            EvidenceRef::new(EvidenceKind::Deployment, deployment.deployment_id)
        };
        for expanded in [false, true] {
            let page = serving
                .get_evidence(&pinned, &snapshot.hex(), reference.clone(), None, expanded)
                .await
                .unwrap();
            assert_eq!(page["metadata_omitted"], true);
            assert!(page["content"][0]["text"].is_string());
            assert!(page["next_cursor"].is_string());
            let encoded =
                cpg_schema::wire::tool_result("get_evidence", &page.to_string(), expanded).unwrap();
            assert!(encoded.len() <= if expanded { 262144 } else { 32768 });
        }
    }
    serving.close().await;
    importer.close().await;
    admin.close().await;
}
