//! Stage G (DESIGN §6.4; ADR-0017, ADR-0019): a serving generation from a published snapshot.
//! Rebuilding it gives the same bytes. A changed file is refused. One generation never mixes
//! vector spaces. The serving schema digests are the known answers Python checks too.

use std::path::{Path, PathBuf};

use cpg_core::analyze::Analysis;
use cpg_core::attempt::compile_analyzed;
use cpg_core::bundle::{build, bundle, verify};
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::codebook::{
    BoundaryReason, Codebook, FlowSink, ModeledArgumentEvaluationStatus, SummaryFlowStepKind,
};
use cpg_schema::id::Id;
use lctx_analytics::config::AnalyticsConfig;

const CONFIG: &str = r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot", "pkg.shadow", "pkg.controls"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe", "pkg.configure"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 6
"#;

const SNAPSHOT: Id = Id([7; 16]);

fn copy(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let p = e.unwrap().path();
        let t = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy(&p, &t);
        } else {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

/// `analysis_shapes` compiled under `sub` with the fake embedder; returns the directory and store.
async fn compiled(sub: &str, reverse: bool, finalizer: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    if finalizer {
        let path = base.join("release/pkg/controls.py");
        let source = std::fs::read_to_string(&path).unwrap();
        let before = "def passthrough(**options):\n    return options";
        let after = "def passthrough(**options):\n    try:\n        return options\n    finally:\n        pass\n        pass";
        assert!(source.contains(before));
        std::fs::write(path, source.replace(before, after)).unwrap();
    }
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(),
            "analysis_shapes",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(base.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(base.join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: SNAPSHOT,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks {
            reverse_module_order: reverse,
            ..Default::default()
        },
    })
    .unwrap();
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(CONFIG).unwrap(),
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
        techniques: Default::default(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, SNAPSHOT, &out.tables, Some(&analysis))
        .await
        .unwrap();
    (dir, store)
}

/// A real local-call chain reaches the finite composition depth cap.
async fn compiled_summary_caps() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("caps");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/summary_caps");
    copy(&fixture, &base.join("release"));
    std::fs::create_dir_all(base.join("venv/site-packages")).unwrap();
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(),
            "summary_caps",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(base.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(base.join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 7),
        python_platform: "linux".to_owned(),
        snapshot_id: SNAPSHOT,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks::default(),
    })
    .unwrap();
    let config = AnalyticsConfig::parse(
        r#"
version = 1
[subsystem]
module_prefixes = ["capspkg"]
public_roots = ["capspkg"]
[seeds]
primary = ["capspkg.f9"]
distractors = ["capspkg.f0", "capspkg.unsupported"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 3
"#,
    )
    .unwrap();
    let analysis = cpg_core::analyze::Analysis {
        embedding_cache: None,
        config,
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
        techniques: Default::default(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, SNAPSHOT, &out.tables, Some(&analysis))
        .await
        .unwrap();
    (dir, store)
}

#[tokio::test(flavor = "multi_thread")]
async fn finite_depth_and_unsupported_refusals_reach_the_native_response() {
    let (dir, store) = compiled_summary_caps().await;
    let (_, ctx) = published(&store, SNAPSHOT).await.unwrap().unwrap();
    let identity_rows = sql::query(
        &ctx,
        "SELECT p.identity_id FROM source_parameter_identities p \
        JOIN declarations d ON d.node_id=p.function_node_id \
        JOIN value_flow_contributions c ON c.origin_id=p.source_origin_id \
        WHERE d.name='handler_entry_identity' AND c.approximated",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    assert_eq!(
        identity_rows
            .iter()
            .map(|batch| batch.num_rows())
            .sum::<usize>(),
        1,
        "the independent proof retains the provider's approximation"
    );
    let rejected = sql::query(
        &ctx,
        "SELECT p.identity_id FROM source_parameter_identities p \
        JOIN declarations d ON d.node_id=p.function_node_id WHERE d.name IN \
        ('handler_entry_rebound', 'handler_entry_deleted', 'handler_entry_nested_mutation')",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    assert_eq!(
        rejected.iter().map(|batch| batch.num_rows()).sum::<usize>(),
        0
    );
    for (name, expected) in [
        ("fresh_default_true", 1),
        ("fresh_default_false", 0),
        ("fresh_keyword_default", 1),
        ("fresh_unused_default", 1),
        ("fresh_skipped_default", 1),
        ("fresh_missing_default", 0),
        ("fresh_removed_default", 0),
        ("fresh_removed_keyword_default", 0),
        ("fresh_escaped_default", 0),
        ("fresh_intervening_default", 0),
        ("fresh_effectful_argument", 0),
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id=f.function_node_id WHERE d.name='{name}'"
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(counts.value(0),expected,"{name} fresh default admission: {}",
            sql::render(&ctx,&format!("SELECT d.qualified_name,b.reason,v.approximated AS raw_approx, \
                c.approximated AS contribution_approx, c.condition_id, v.identity, v.through_call \
                FROM summary_boundaries b JOIN declarations d ON d.node_id=b.function_node_id \
                JOIN value_flow_contributions c ON c.origin_id=b.source_origin_id \
                JOIN flow_values v ON v.fact_id=c.flow_value_fact_id WHERE d.qualified_name LIKE '%{name}%'"))
                .await.unwrap());
    }
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) FROM ( \
        SELECT c.flow_value_fact_id FROM value_flow_contributions c \
        JOIN declarations d ON d.node_id = c.sink_function_node_id \
        JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
        LEFT JOIN summary_flows f ON f.source_origin_id = c.origin_id \
          AND f.source_flow_fact_id = c.flow_value_fact_id \
        LEFT JOIN summary_boundaries b ON b.source_origin_id = c.origin_id \
          AND b.source_flow_fact_id = c.flow_value_fact_id \
        WHERE d.name = 'mixed_origin' AND v.sink = {} \
        GROUP BY c.flow_value_fact_id HAVING count(DISTINCT c.origin_id) = 2 \
          AND count(DISTINCT f.summary_id) = 1 \
          AND count(DISTINCT CASE WHEN b.reason = {} THEN b.source_origin_id END) = 1 \
    )",
            FlowSink::Return.code(),
            BoundaryReason::CallTransfer.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "one raw returned use keeps a proved value and an unproved call origin"
    );
    for (name, reason) in [
        ("f9", 21),
        ("unsupported", BoundaryReason::ScopeBoundary.code()),
        ("condition_atom_cap", 24),
        ("summary_proof_cap", 30),
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) AS n FROM summary_boundaries b \
            JOIN declarations d ON d.node_id = b.function_node_id \
            WHERE d.name = '{name}' AND b.reason = {reason}"
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(
            counts.value(0),
            1,
            "{name} must retain its typed boundary: {}",
            sql::render(
                &ctx,
                &format!(
                    "SELECT b.reason FROM summary_boundaries b JOIN declarations d \
                ON d.node_id=b.function_node_id WHERE d.name='{name}'"
                )
            )
            .await
            .unwrap()
        );
    }
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'condition_atom_cap' AND f.boundary_reason IS NULL",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "a bounded condition cannot publish a positive summary"
    );
    for (name, expected) in [
        ("completed_predecessor", 1),
        ("parameter_predecessor", 1),
        ("signed_literal_predecessor", 1),
        ("boolean_not_predecessor", 1),
        ("binary_numeric_predecessor", 1),
        ("raising_binary_predecessor", 0),
        ("short_circuit_and_predecessor", 1),
        ("raising_and_predecessor", 0),
        ("selected_true_predecessor", 1),
        ("raising_false_predecessor", 0),
        ("two_completed_predecessors", 2),
        ("raising_unary_predecessor", 0),
        ("raising_not_predecessor", 0),
        ("completed_then_raising", 0),
        ("assigned_local_argument", 1),
        ("raising_predecessor", 0),
        ("possibly_unbound_argument", 0),
        ("possibly_unbound_local_argument", 0),
        ("conditional_callee", 0),
        ("guarded_module_callee", 0),
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) AS n FROM model_applications a \
             JOIN declarations d ON d.node_id = a.function_node_id \
             WHERE d.name = '{name}' AND a.target_body_return_parameter IS NOT NULL \
               AND a.target_count = 1 AND a.candidate_set_complete_under_model \
               AND NOT a.has_unresolved_remainder",
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        if name != "conditional_callee" && name != "guarded_module_callee" {
            let expected_applications =
                if name == "two_completed_predecessors" || name == "completed_then_raising" {
                    2
                } else {
                    1
                };
            assert_eq!(
                counts.value(0),
                expected_applications,
                "{name} must have closed normal-return targets"
            );
        }
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) AS n FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
               AND p.kind = {} \
             WHERE d.name = '{name}' AND f.path_depth = 0",
                SummaryFlowStepKind::PrecedingCallNormal.code(),
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(
            counts.value(0),
            expected,
            "{name} normal predecessor admission"
        );
    }
    let rows = sql::query(&ctx, "SELECT count(*) AS n FROM summary_boundaries b \
        JOIN declarations d ON d.node_id = b.function_node_id \
        WHERE d.name IN ('raising_predecessor', 'raising_unary_predecessor', 'raising_not_predecessor', 'raising_binary_predecessor', 'raising_and_predecessor', 'raising_false_predecessor', 'completed_then_raising', 'possibly_unbound_argument', \
          'possibly_unbound_local_argument', 'conditional_callee', \
          'guarded_module_callee') AND b.reason = 4")
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        11,
        "uncertain arguments and callees must stay unknown"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps first ON first.summary_id = f.summary_id \
        JOIN summary_flow_steps second ON second.summary_id = f.summary_id \
        JOIN call_syntax c1 ON c1.fact_id = first.evidence_id \
        JOIN call_syntax c2 ON c2.fact_id = second.evidence_id \
        WHERE d.name = 'two_completed_predecessors' AND f.path_depth = 0 \
          AND first.kind = {call} AND second.kind = {call} \
          AND first.ordinal < second.ordinal AND c1.start_byte < c2.start_byte",
            call = SummaryFlowStepKind::CallSite.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "both completed predecessor calls must retain source order"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
        WHERE d.name = 'modeled_after_completed' AND f.path_depth = 1 \
          AND p.kind = {}",
            SummaryFlowStepKind::PrecedingCallNormal.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        2,
        "modeled return cites its predecessor and returned call completion"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flow_steps p \
        JOIN summary_flows f ON f.summary_id = p.summary_id \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'assigned_modeled_after_completed' AND f.path_depth = 2 \
          AND p.kind = {}",
            SummaryFlowStepKind::PrecedingCallNormal.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        3,
        "assignment return cites its source invocation, later call and modeled value group"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
        WHERE d.name = 'local_after_completed' AND f.path_depth = 1 \
          AND p.kind = {}",
            SummaryFlowStepKind::PrecedingCallNormal.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "local wrapper must cite earlier normal completion"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'modeled_after_opaque'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "opaque predecessor cannot certify modeled return"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_boundaries b \
        JOIN declarations d ON d.node_id = b.function_node_id \
        WHERE d.name = 'modeled_after_opaque' AND b.reason = 5",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "opaque predecessor retains an explicit control boundary"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'assigned_modeled_after_opaque'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "opaque call cannot certify assignment result"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_boundaries b \
        JOIN declarations d ON d.node_id = b.function_node_id \
        WHERE d.name = 'assigned_modeled_after_opaque' AND b.reason = 5",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "opaque assignment predecessor retains control boundary"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'local_after_opaque'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "opaque predecessor cannot certify local wrapper"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_boundaries b \
        JOIN declarations d ON d.node_id = b.function_node_id \
        WHERE d.name = 'local_after_opaque' AND b.reason = 5",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "opaque local predecessor retains control boundary"
    );
    let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        JOIN syntax_nodes s ON s.fact_id = p.evidence_id \
        WHERE d.name = 'signed_literal_predecessor' \
          AND s.kind = {} AND s.detail = '-'",
        cpg_schema::codebook::SyntaxKind::ExprUnaryOp.code()))
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the signed literal must cite its unary syntax fact"
    );
    let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        JOIN syntax_nodes s ON s.fact_id = p.evidence_id \
        WHERE d.name = 'boolean_not_predecessor' \
          AND s.kind = {} AND s.detail = 'not'",
        cpg_schema::codebook::SyntaxKind::ExprUnaryOp.code()))
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the Boolean negation must cite its unary syntax fact"
    );
    let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        JOIN syntax_nodes s ON s.fact_id = p.evidence_id \
        WHERE d.name = 'binary_numeric_predecessor' \
          AND s.kind = {} AND s.detail = '+'",
        cpg_schema::codebook::SyntaxKind::ExprBinOp.code()))
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the binary literal proof cites the outer addition syntax fact"
    );
    let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        JOIN syntax_nodes s ON s.fact_id = p.evidence_id \
        WHERE d.name = 'short_circuit_and_predecessor' \
          AND s.kind = {} AND s.detail = 'and'",
        cpg_schema::codebook::SyntaxKind::ExprBoolOp.code()))
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the safe short circuit cites the outer Boolean syntax fact"
    );
    let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        JOIN syntax_nodes s ON s.fact_id = p.evidence_id \
        WHERE d.name = 'selected_true_predecessor' AND s.kind = {}",
        cpg_schema::codebook::SyntaxKind::ExprIf.code()))
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the selected literal cites the whole conditional syntax fact"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps proof ON proof.summary_id = f.summary_id \
          AND proof.kind = {} \
        JOIN syntax_nodes expression ON expression.fact_id = proof.evidence_id \
          AND expression.kind = {} AND expression.detail = '+' \
        WHERE d.name = 'binary_sibling_identity' AND f.path_depth = 1 \
          AND f.boundary_reason IS NULL",
            SummaryFlowStepKind::ArgumentEvaluation.code(),
            cpg_schema::codebook::SyntaxKind::ExprBinOp.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the modeled return admits a normally evaluated binary sibling"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'raising_binary_sibling'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "a raising binary sibling cannot complete the modeled return"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps proof ON proof.summary_id = f.summary_id \
          AND proof.kind = {} \
        JOIN syntax_nodes expression ON expression.fact_id = proof.evidence_id \
          AND expression.kind = {} AND expression.detail = 'or' \
        WHERE d.name = 'short_circuit_or_sibling' AND f.path_depth = 1 \
          AND f.boundary_reason IS NULL",
            SummaryFlowStepKind::ArgumentEvaluation.code(),
            cpg_schema::codebook::SyntaxKind::ExprBoolOp.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the modeled return cites its safe short-circuit sibling"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM modeled_argument_evaluations e \
        JOIN call_syntax c ON c.node_id = e.call_site_node_id \
        JOIN declarations d ON d.node_id = c.owner_node_id \
        JOIN syntax_nodes expression ON expression.fact_id = e.evidence_id \
        WHERE d.name IN ('short_circuit_or_sibling', 'binary_sibling_identity') \
          AND e.status = {} AND expression.kind IN ({}, {})",
            ModeledArgumentEvaluationStatus::ComposedExpressionNormal.code(),
            cpg_schema::codebook::SyntaxKind::ExprBoolOp.code(),
            cpg_schema::codebook::SyntaxKind::ExprBinOp.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        2,
        "closed expressions have their own typed evaluation status"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'raising_or_sibling'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "a raising Boolean sibling cannot complete the modeled return"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM modeled_argument_evaluations e \
        JOIN call_syntax c ON c.node_id = e.call_site_node_id \
        JOIN declarations d ON d.node_id = c.owner_node_id \
        JOIN syntax_nodes expression ON expression.fact_id = e.evidence_id \
        WHERE d.name = 'selected_false_sibling' AND e.status = {} \
          AND expression.kind = {}",
            ModeledArgumentEvaluationStatus::ComposedExpressionNormal.code(),
            cpg_schema::codebook::SyntaxKind::ExprIf.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the selected else literal has a typed whole-expression witness"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'raising_true_sibling'",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "the selected raising branch cannot complete the modeled return"
    );
    let rows = sql::query(&ctx, "SELECT count(*) AS n FROM return_entry_steps p \
        JOIN return_entry_statuses entry ON entry.return_site_fact_id = p.return_site_fact_id AND entry.condition_id = p.condition_id \
        JOIN declarations d ON d.node_id = entry.function_node_id \
        WHERE d.name = 'assigned_local_argument' \
          AND p.evidence_id IN (SELECT fact_id FROM flow_reaching)")
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the local argument must cite ty reaching evidence"
    );
    for (name, expected) in [
        ("terminating_branch_before_recursion", 1),
        ("unconditional_self_call", 0),
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) AS n FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             WHERE d.name = '{name}' AND f.path_depth = 0 AND f.boundary_reason IS NULL",
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(
            counts.value(0),
            expected,
            "{name} recursive predecessor admission"
        );
    }
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
         JOIN declarations d ON d.node_id = f.function_node_id \
         JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
           AND s.kind = {} \
         JOIN flow_test_value_links l ON l.link_id = s.evidence_id \
         WHERE d.name = 'recursive_literal_return' AND f.path_depth = 1",
            SummaryFlowStepKind::CalleeConditionLink.code(),
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the real recursive literal cites its exact test link"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'recursive_literal_false' AND f.path_depth > 0",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "the false control must not derive a recursive path"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        JOIN summary_flow_steps evaluation ON evaluation.summary_id = f.summary_id \
          AND evaluation.kind = {} \
        JOIN syntax_nodes expression ON expression.fact_id = evaluation.evidence_id \
          AND expression.kind = {} AND expression.detail = 'not' \
        JOIN summary_flow_steps link ON link.summary_id = f.summary_id \
          AND link.kind = {} AND evaluation.ordinal < link.ordinal \
        WHERE d.name = 'recursive_closed_true' AND f.path_depth = 1",
            SummaryFlowStepKind::ArgumentEvaluation.code(),
            cpg_schema::codebook::SyntaxKind::ExprUnaryOp.code(),
            SummaryFlowStepKind::CalleeConditionLink.code()
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the closed unary value and callee guard have ordered proof links"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'recursive_closed_false' AND f.path_depth > 0",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "the closed false value cannot select the recursive base"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'recursive_raising_control' AND f.path_depth > 0",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "a retained unknown argument cannot fix a recursive guard"
    );
    for (name, kind) in [
        (
            "recursive_short_circuit_true",
            cpg_schema::codebook::SyntaxKind::ExprBoolOp,
        ),
        (
            "recursive_selected_true",
            cpg_schema::codebook::SyntaxKind::ExprIf,
        ),
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) AS n FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            JOIN summary_flow_steps evaluation ON evaluation.summary_id = f.summary_id \
              AND evaluation.kind = {} \
            JOIN syntax_nodes expression ON expression.fact_id = evaluation.evidence_id \
              AND expression.kind = {} \
            JOIN summary_flow_steps link ON link.summary_id = f.summary_id \
              AND link.kind = {} AND evaluation.ordinal < link.ordinal \
            WHERE d.name = '{name}' AND f.path_depth = 1",
                SummaryFlowStepKind::ArgumentEvaluation.code(),
                kind.code(),
                SummaryFlowStepKind::CalleeConditionLink.code()
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(
            counts.value(0),
            1,
            "{name} must cite its closed control before the callee link"
        );
    }
    for name in [
        "recursive_short_circuit_false",
        "recursive_selected_false",
        "recursive_selected_raising",
    ] {
        let rows = sql::query(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            WHERE d.name = '{name}' AND f.path_depth > 0"
            ),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let counts = rows[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap();
        assert_eq!(
            counts.value(0),
            0,
            "{name} cannot reach a recursive positive"
        );
    }
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM ({}) seed \
        JOIN declarations d ON d.node_id = seed.function_node_id \
        JOIN ({}) arg ON arg.call_fact_id = seed.call_fact_id \
          AND arg.callee_node_id = seed.callee_node_id AND arg.evaluation_fact_id IS NULL \
        WHERE d.name = 'recursive_selected_raising'",
            cpg_schema::behavior::local_call_summary_flow_seeds().sql,
            cpg_schema::behavior::local_call_arguments().sql
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the selected raising branch retains a candidate with an unknown evaluation"
    );
    let rows = sql::query(
        &ctx,
        &format!(
            "SELECT count(*) AS n FROM summary_flows f \
         JOIN declarations d ON d.node_id = f.function_node_id \
         JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
           AND s.kind = {} \
         JOIN flow_test_value_links l ON l.link_id = s.evidence_id \
         WHERE d.name = 'recursive_keyword_true' AND f.path_depth = 1",
            SummaryFlowStepKind::CalleeConditionLink.code(),
        ),
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        1,
        "the keyword control must cite its callee guard link"
    );
    let rows = sql::query(
        &ctx,
        "SELECT count(*) FROM summary_flows f \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'recursive_keyword_false' AND f.path_depth > 0",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let counts = rows[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap();
    assert_eq!(
        counts.value(0),
        0,
        "the false keyword control cannot derive a recursive path"
    );
    let generation = bundle(&store, SNAPSHOT, &dir.path().join("generations"))
        .await
        .unwrap();
    let script = r#"
import sys
import asyncio
from pathlib import Path
from fastmcp import Client
import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_semantics import SemanticExecutor, kernel_format
from lctx_mcp.generation import NATIVE_IPC_FILES
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
generation = load(Path(sys.argv[1]), None)
index = generation.condition_graph
public = {row["node_id"] for row in generation.tables["operations"].to_pylist()}
private_link = next(row for row in generation.tables["flow_test_value_links"].to_pylist()
                    if row["operation_node_id"] not in public)
files = []
for name in NATIVE_IPC_FILES:
    table = generation.tables[name]
    if name == "callable_parameters":
        kept = [row for row in table.to_pylist() if row["formal_node_id"] != private_link["formal_node_id"]]
        table = pa.Table.from_pylist(kept, schema=table.schema)
    sink = pa.BufferOutputStream()
    with ipc.new_file(sink, table.schema) as writer:
        writer.write_table(table)
    files.append((name, sink.getvalue().to_pybytes()))
try:
    SemanticExecutor.from_ipc(kernel_format(), generation.snapshot_id,
                              generation.manifest["entry_value_effect_digest"], files)
except ValueError as error:
    assert "invalid value link" in str(error), str(error)
else:
    raise AssertionError("missing private callable formal admitted")
def inspect(operation, formal):
    return index.inspect_value_paths(operation, formal, "none", "", True, 0, 20)
def altered_index(changes):
    files = []
    for name in NATIVE_IPC_FILES:
        table = generation.tables[name]
        if name in changes:
            table = pa.Table.from_pylist(changes[name], schema=table.schema)
        sink = pa.BufferOutputStream()
        with ipc.new_file(sink, table.schema) as writer:
            writer.write_table(table)
        files.append((name, sink.getvalue().to_pybytes()))
    return SemanticExecutor.from_ipc(kernel_format(), generation.snapshot_id,
                                    generation.manifest["entry_value_effect_digest"], files)
# Raw direct, nested and assignment-return paths need their root normal-call obligation
# even when no SourceModeledIdentity commitment protects that group.
for operation in ("capspkg.modeled_after_completed", "capspkg.nested_total_identity",
                  "capspkg.assigned_modeled_after_completed"):
    paths, boundaries, _, truncated, _ = inspect(operation, "value")
    assert paths and not truncated, (operation, boundaries)
    target = bytes.fromhex(paths[0][0])
    rows = generation.tables["summary_flow_steps"].to_pylist()
    group = sorted((r for r in rows if r["summary_id"] == target), key=lambda r: r["ordinal"])
    assert any(r["kind"] == "raw_identity" for r in group), operation
    assert not any(r["kind"] == "source_modeled_identity" for r in group), operation
    last = max(i for i, r in enumerate(group) if r["kind"] == "model_rule")
    assert [r["kind"] for r in group[last-2:last]] == ["model_frame_exit", "preceding_call_normal"]
    # Both pair omission and whole conclusion omission retain the call occurrence anchors.
    for end in (last, last + 1):
        changed_group = group[:last-2] + group[end:]
        for ordinal, row in enumerate(changed_group):
            row["ordinal"] = ordinal
        changed = [r for r in rows if r["summary_id"] != target] + changed_group
        try:
            altered_index({"summary_flow_steps": changed})
        except ValueError as error:
            assert ("modeled call omitted" in str(error)
                    or "value basis" in str(error)), (operation, str(error))
        else:
            raise AssertionError(("root release obligation was erased", operation, end))
for name in ("model_frame_exit_arguments", "model_frame_exit_steps"):
    try:
        altered_index({name: []})
    except ValueError as error:
        assert "frame release" in str(error), str(error)
    else:
        raise AssertionError(("frame evidence disappeared", name))
try:
    inspect("capspkg.fresh_default_true.inner", "enabled")
except ValueError:
    pass
else:
    raise AssertionError("private callee metadata became a public operation")
for operation in ("capspkg.optional_unused_explicit", "capspkg.optional_keyword_explicit"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert paths and not boundaries and not truncated, (operation, paths, boundaries)
for operation in ("capspkg.optional_unused_omitted", "capspkg.optional_keyword_omitted"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not paths and boundaries and not truncated, (operation, paths, boundaries)
    assert any(boundary[3] == "default_stability_unknown" for boundary in boundaries), boundaries
for name in ("fresh_default_true", "fresh_keyword_default", "fresh_unused_default", "fresh_skipped_default"):
    paths, boundaries, total, truncated, work = inspect("capspkg." + name, "value")
    assert paths and not boundaries and not truncated, (name, paths, boundaries)
    assert all(any(step[0] == "default_availability_evidence" for step in path[3]) for path in paths), paths
    assert all(any(step[0] == "default_stability_evidence" for step in path[3]) for path in paths), paths
    assert all(sum(step[0] == "argument_evaluation" for step in path[3]) == 1 for path in paths), paths
for name in ("fresh_default_false", "fresh_missing_default", "fresh_removed_default",
             "fresh_removed_keyword_default", "fresh_escaped_default", "fresh_intervening_default",
             "fresh_effectful_argument"):
    paths, boundaries, total, truncated, work = inspect("capspkg." + name, "value")
    assert not paths and boundaries and not truncated, (name, paths, boundaries)
# Independently generated programs, never compiler fixtures, challenge value identity.
import json
import subprocess
oracle = subprocess.run([sys.executable, "docs/design_review/evidence/2026-09-27_modeled-identity/runtime_oracle.py"],
                        capture_output=True, text=True, check=True, timeout=45)
observed = json.loads(oracle.stdout)["cases"]
for name in ("framed", "nested_frames", "keyword", "rebound", "deleted", "overridden", "nested_mutation"):
    paths, boundaries, total, truncated, work = inspect("capspkg.modeled_" + name, "value")
    assert bool(paths) == observed[name]["same_identity"], (name, paths, boundaries)
    if paths:
        assert all(any(step[0] == "source_modeled_identity" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.context_value_after_model", "value")
assert paths and all(any(s[0] == "context_entry_value_identity" for s in p[3]) for p in paths), (paths, boundaries)
assert any(any(s[0] == "model_rule" for s in p[3]) for p in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.modeled_framed", "value")
modeled_rows = generation.tables["source_modeled_identities"].to_pylist()
own = next(r for r in modeled_rows if r["source_origin_id"].hex() == paths[0][9])
foreign = next(r for r in modeled_rows if r["function_node_id"] != own["function_node_id"])
summary = next(r for r in generation.tables["summary_flows"].to_pylist() if r["source_origin_id"] == own["source_origin_id"])
steps = [r for r in generation.tables["summary_flow_steps"].to_pylist() if r["summary_id"] == summary["summary_id"]]
start = next(r["ordinal"] for r in steps if r["kind"] == "callee_resolution" and r["evidence_id"] == own["callee_resolution_fact_id"])
for mutation in ("missing", "foreign", "removed_step", "duplicate_step", "reordered", "entire_group"):
    files = []
    for name in NATIVE_IPC_FILES:
        table = generation.tables[name]
        rows = table.to_pylist()
        if name == "source_modeled_identities" and mutation in ("missing", "entire_group"):
            rows = [r for r in rows if r["identity_id"] != own["identity_id"]]
        if name == "summary_flow_steps":
            changed = []
            ordinals = {}
            for row in rows:
                selected = row["summary_id"] == summary["summary_id"]
                own_step = selected and row["kind"] == "source_modeled_identity"
                if own_step and mutation == "foreign": row = dict(row, evidence_id=foreign["identity_id"])
                if selected and mutation == "reordered" and row["ordinal"] in (start, start + 1):
                    other = next(s for s in steps if s["ordinal"] == start + 1 - (row["ordinal"] - start))
                    row = dict(other, ordinal=row["ordinal"])
                remove = (own_step and mutation == "removed_step") or (selected and mutation == "entire_group"
                    and start <= row["ordinal"] < start + own["model_proof_count"])
                copies = 0 if remove else 2 if own_step and mutation == "duplicate_step" else 1
                for _ in range(copies):
                    ordinal = ordinals.get(row["summary_id"], 0)
                    changed.append(dict(row, ordinal=ordinal))
                    ordinals[row["summary_id"]] = ordinal + 1
            rows = changed
        table = pa.Table.from_pylist(rows, schema=table.schema)
        sink = pa.BufferOutputStream()
        with ipc.new_file(sink, table.schema) as writer: writer.write_table(table)
        files.append((name, sink.getvalue().to_pybytes()))
    try:
        SemanticExecutor.from_ipc(kernel_format(), generation.snapshot_id,
                                  generation.manifest["entry_value_effect_digest"], files)
    except ValueError as error:
        assert "modeled" in str(error), (mutation, str(error))
        if mutation == "entire_group": assert "requires a value basis" in str(error), str(error)
    else:
        raise AssertionError((mutation, "invalid modeled identity admitted"))
paths, boundaries, total, truncated, work = inspect("capspkg.handler_entry_identity", "value")
assert paths and not boundaries and not truncated, (paths, boundaries)
assert any(any(step[0] == "handler_class_evidence" for step in path[3]) for path in paths), paths
assert any(any(step[0] == "source_parameter_identity" for step in path[3]) for path in paths), paths
certificate_rows = generation.tables["source_parameter_identities"].to_pylist()
own_certificate = next(row for row in certificate_rows if row["source_origin_id"].hex() == paths[0][9])
foreign_certificate = next(row for row in certificate_rows if row["function_node_id"] != own_certificate["function_node_id"])
for mutation in ("missing", "foreign", "removed_step", "duplicate_step"):
    files = []
    for name in NATIVE_IPC_FILES:
        table = generation.tables[name]
        rows = table.to_pylist()
        if name == "source_parameter_identities" and mutation == "missing":
            rows = [row for row in rows if row["identity_id"] != own_certificate["identity_id"]]
        if name == "summary_flow_steps" and mutation == "foreign":
            rows = [dict(row, evidence_id=foreign_certificate["identity_id"])
                    if row["kind"] == "source_parameter_identity" and row["evidence_id"] == own_certificate["identity_id"]
                    else row for row in rows]
        if name == "summary_flow_steps" and mutation in ("removed_step", "duplicate_step"):
            changed = []
            ordinals = {}
            for row in rows:
                own_step = row["kind"] == "source_parameter_identity" and row["evidence_id"] == own_certificate["identity_id"]
                copies = 0 if own_step and mutation == "removed_step" else 2 if own_step else 1
                for _ in range(copies):
                    ordinal = ordinals.get(row["summary_id"], 0)
                    changed.append(dict(row, ordinal=ordinal))
                    ordinals[row["summary_id"]] = ordinal + 1
            rows = changed
        table = pa.Table.from_pylist(rows, schema=table.schema)
        sink = pa.BufferOutputStream()
        with ipc.new_file(sink, table.schema) as writer:
            writer.write_table(table)
        files.append((name, sink.getvalue().to_pybytes()))
    try:
        SemanticExecutor.from_ipc(kernel_format(), generation.snapshot_id,
                                  generation.manifest["entry_value_effect_digest"], files)
    except ValueError as error:
        assert "source parameter identity" in str(error), (mutation, str(error))
    else:
        raise AssertionError((mutation, "source identity mismatch admitted"))
for operation in ("capspkg.handler_entry_nonmatch", "capspkg.handler_entry_cleanup"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not paths and boundaries and not truncated, (operation, paths, boundaries)
for operation in ("capspkg.handler_entry_cleanup", "capspkg.handler_predecessor_cleanup"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not paths and not truncated and any(b[3] == "handler_name_cleanup" for b in boundaries), (operation, paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.handler_cleanup_unselected", "value")
assert paths and not truncated, (paths, boundaries)
assert not any(b[3] == "handler_name_cleanup" for b in boundaries), boundaries
for operation, expected in (
    ("capspkg.f9", "summary_depth_limit"),
    ("capspkg.condition_atom_cap", "condition_atom_limit"),
    ("capspkg.unsupported", "scope_boundary"),
    ("capspkg.raising_predecessor", "unsupported_control_flow"),
    ("capspkg.raising_unary_predecessor", "unsupported_control_flow"),
    ("capspkg.raising_not_predecessor", "unsupported_control_flow"),
    ("capspkg.raising_binary_predecessor", "unsupported_control_flow"),
    ("capspkg.raising_and_predecessor", "unsupported_control_flow"),
    ("capspkg.raising_false_predecessor", "unsupported_control_flow"),
    ("capspkg.completed_then_raising", "unsupported_control_flow"),
    ("capspkg.possibly_unbound_argument", "unsupported_control_flow"),
    ("capspkg.possibly_unbound_local_argument", "unsupported_control_flow"),
    ("capspkg.conditional_callee", "unsupported_control_flow"),
    ("capspkg.guarded_module_callee", "unsupported_control_flow"),
    ("capspkg.unconditional_self_call", "scope_boundary"),
    ("capspkg.modeled_after_opaque", "scope_boundary"),
    ("capspkg.assigned_modeled_after_opaque", "scope_boundary"),
    ("capspkg.local_after_opaque", "scope_boundary"),
):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated, (operation, paths, boundaries)
    assert any(boundary[3] == expected for boundary in boundaries), (operation, paths, boundaries)
for operation in ("capspkg.completed_predecessor", "capspkg.parameter_predecessor", "capspkg.signed_literal_predecessor", "capspkg.boolean_not_predecessor", "capspkg.binary_numeric_predecessor", "capspkg.short_circuit_and_predecessor", "capspkg.selected_true_predecessor", "capspkg.two_completed_predecessors", "capspkg.assigned_local_argument"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and not boundaries, (operation, paths, boundaries)
    assert any(any(step[0] == "preceding_call_normal" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.two_completed_predecessors", "value")
assert any(sum(step[0] == "preceding_call_normal" for step in path[3]) == 2 for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.local_after_completed", "value")
assert not truncated and not boundaries, (paths, boundaries)
assert any(any(step[0] == "preceding_call_normal" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.modeled_after_completed", "value")
assert not truncated and not boundaries, (paths, boundaries)
assert any(any(step[0] == "preceding_call_normal" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.assigned_modeled_after_completed", "value")
assert not truncated and not boundaries, (paths, boundaries)
assert any(sum(step[0] == "preceding_call_normal" for step in path[3]) == 3 for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.keyword_local_wrapper", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
assert any(any(step[0] == "callee_summary" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_closed_true", "value")
assert not truncated and paths, (paths, boundaries)
assert any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_closed_false", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_raising_control", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
for operation in ("capspkg.recursive_short_circuit_true", "capspkg.recursive_selected_true"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and paths, (operation, paths, boundaries)
    assert any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
for operation in ("capspkg.recursive_short_circuit_false", "capspkg.recursive_selected_false", "capspkg.recursive_selected_raising"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and paths and boundaries, (operation, paths, boundaries)
    assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
    assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.unpacked_local_wrapper", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.nested_total_identity", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
assert any(
    next(row for row in generation.tables["summary_flows"].to_pylist()
         if row["summary_id"].hex() == path[0])["path_depth"] == 2
    and [step[0] for step in path[3]].count("model_rule") == 2
    and [step[0] for step in path[3]][:2]
        == ["callee_resolution", "argument_evaluation"]
    and [step[0] for step in path[3]].index("model_rule")
        < max(i for i, step in enumerate(path[3]) if step[0] == "argument_evaluation")
    for path in paths
), paths
paths, boundaries, total, truncated, work = inspect("capspkg.nested_raising_identity", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
for operation, reason in (("capspkg.expression_depth_cap", "expression_depth_limit"),
                          ("capspkg.expression_work_cap", "expression_work_limit"),
                          ("capspkg.summary_proof_cap", "summary_proof_limit")):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and not paths and boundaries, (operation, paths, boundaries)
    assert any(boundary[3] == reason for boundary in boundaries), (operation, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.summary_proof_boundary", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
assert any(len(path[3]) == 64 for path in paths), paths
for operation in ("capspkg.composed_boolean_control", "capspkg.composed_expression_predecessor", "capspkg.composed_expression_sibling"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and paths and not boundaries, (operation, paths, boundaries)
for operation in ("capspkg.composed_raising_control", "capspkg.composed_expression_raising_sibling"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and not paths and boundaries, (operation, paths, boundaries)
for operation in ("capspkg.typed_finalizer_identity", "capspkg.reraised_finalizer_identity"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and paths and not boundaries, (operation, paths, boundaries)
    assert any(any(step[0] == "handler_class_evidence" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.unmatched_finalizer_identity", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.multiple_argument_true", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
assert any(sum(step[0] == "callee_condition_link" for step in path[3]) == 1 for path in paths), paths
for operation in ("capspkg.multiple_argument_false", "capspkg.multiple_argument_raising",
                  "capspkg.multiple_argument_missing",
                  "capspkg.multiple_control_true", "capspkg.multiple_control_false",
                  "capspkg.multiple_control_raising"):
    paths, boundaries, total, truncated, work = inspect(operation, "value")
    assert not truncated and not paths and boundaries, (operation, paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.nested_deleted_identity", "value")
assert not truncated and not paths and not boundaries, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.binary_sibling_identity", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.raising_binary_sibling", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.short_circuit_or_sibling", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.raising_or_sibling", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.selected_false_sibling", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.raising_true_sibling", "value")
assert not truncated and not paths and boundaries, (paths, boundaries)
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.framed_maybe_deleted_identity", "value")
assert not truncated and not paths, (paths, boundaries)
paths, boundaries, total, truncated, work = inspect("capspkg.nested_three_total_identity", "value")
assert not truncated and paths and not boundaries, (paths, boundaries)
assert any(
    next(row for row in generation.tables["summary_flows"].to_pylist()
         if row["summary_id"].hex() == path[0])["path_depth"] == 3
    and [step[0] for step in path[3]].count("model_rule") == 3
    for path in paths
), paths
for operation, formal, kind, value, expected in (
    ("capspkg.string_member_identity", "transport", "str", "stdio", "refuted_under_model"),
    ("capspkg.string_member_identity", "transport", "str", "http", "compatible_under_model"),
    ("capspkg.integer_equal_identity", "level", "int", "3", "refuted_under_model"),
    ("capspkg.integer_equal_identity", "level", "int", "2", "compatible_under_model"),
    ("capspkg.integer_equal_identity", "level", "bool", "true", "unknown"),
):
    paths, boundaries, total, truncated, work = index.inspect_value_paths(
        operation, formal, kind, value, True, 0, 10
    )
    assert not truncated and paths and not boundaries, (operation, paths, boundaries)
    assert any(path[4] == expected for path in paths), (operation, expected, paths)
async def check_mcp(server):
    async with Client(server) as client:
        for operation, formal, kind, value, expected in (
            ("capspkg.string_member_identity", "transport", "str", "stdio", "refuted_under_model"),
            ("capspkg.string_member_identity", "transport", "str", "http", "compatible_under_model"),
            ("capspkg.integer_equal_identity", "level", "int", 3, "refuted_under_model"),
            ("capspkg.integer_equal_identity", "level", "int", 2, "compatible_under_model"),
            ("capspkg.integer_equal_identity", "level", "bool", True, "unknown"),
        ):
            result = await client.call_tool("inspect_value_paths", {
                "snapshot_id": generation.snapshot_id,
                "operation": operation,
                "formal": formal,
                "exact_input": {"kind": kind, "value": value},
                "standard_builtins": True,
            })
            page = result.structured_content
            assert page and page["paths"] and not page["boundaries"], (operation, page)
            assert any(path["exact_input_result"] == expected for path in page["paths"]), (operation, expected, page)
with served_bundle(Path(sys.argv[1])) as pg:
    asyncio.run(check_mcp(pg.server(None)))
paths, boundaries, total, truncated, work = inspect("capspkg.terminating_branch_before_recursion", "value")
assert not truncated and paths, (paths, boundaries)
assert any(boundary[3] == "scope_boundary" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = index.inspect_value_paths(
    "capspkg.recursive_literal_return", "value", "none", "", True, 0, 20
)
assert not truncated and paths, (paths, boundaries)
assert any(path[1] == "conditional" and any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_literal_false", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_keyword_true", "value")
assert not truncated and paths, (paths, boundaries)
assert any(path[1] == "conditional"
           and any(step[0] == "callee_condition_link" for step in path[3])
           for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_keyword_false", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_all_keyword_true", "value")
assert not truncated and paths, (paths, boundaries)
assert any(path[1] == "conditional"
           and any(step[0] == "callee_condition_link" for step in path[3])
           for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_all_keyword_false", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_reversed_keyword_true", "value")
assert not truncated and paths, (paths, boundaries)
assert any(path[1] == "conditional"
           and any(step[0] == "callee_condition_link" for step in path[3])
           for path in paths), paths
assert any(len(evaluations) == 2 and evaluations[-1] != path[8]
           and any(step[0] == "return_source" and step[1] == path[8]
                   and path[3][index + 1][0] == "argument_evaluation"
                   and path[3][index + 1][1] == evaluations[-1]
                   for index, step in enumerate(path[3][:-1]))
           for path in paths
           if any(step[0] == "callee_condition_link" for step in path[3])
           for resolution in [max(i for i, step in enumerate(path[3]) if step[0] == "callee_resolution")]
           for evaluations in [[step[1] for step in path[3][resolution + 1:]
                                if step[0] == "argument_evaluation"]]), paths
paths, boundaries, total, truncated, work = inspect("capspkg.recursive_reversed_keyword_false", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "callee_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.guarded_symbolic_recursive", "value")
assert not truncated and paths, (paths, boundaries)
assert any(path[1] == "conditional"
           and any(step[0] == "caller_condition_link" for step in path[3])
           and any(step[0] == "callee_condition_link" for step in path[3])
           for path in paths), paths
paths, boundaries, total, truncated, work = inspect("capspkg.guarded_symbolic_base", "value")
assert not truncated and paths and boundaries, (paths, boundaries)
assert not any(any(step[0] == "caller_condition_link" for step in path[3]) for path in paths), paths
assert any(boundary[3] == "call_transfer" for boundary in boundaries), boundaries
paths, boundaries, total, truncated, work = inspect("capspkg.mixed_origin", "value")
assert not truncated and len(paths) == 1 and not boundaries, (paths, boundaries)
positive = next(row for row in generation.tables["summary_flows"].to_pylist()
                if row["summary_id"].hex() == paths[0][0])
assert paths[0][8] == positive["source_flow_fact_id"].hex()
assert paths[0][9] == positive["source_origin_id"].hex()
paths, boundaries, total, truncated, work = inspect("capspkg.mixed_origin", "other")
assert not truncated and not paths and len(boundaries) == 1, (paths, boundaries)
assert boundaries[0][3] == "call_transfer", boundaries
withheld = next(row for row in generation.tables["summary_boundaries"].to_pylist()
                if row["source_origin_id"].hex() == boundaries[0][1])
assert positive["source_flow_fact_id"] == withheld["source_flow_fact_id"]
assert positive["source_origin_id"] != withheld["source_origin_id"]
assert paths == [] and boundaries[0][0] == positive["source_flow_fact_id"].hex()
"#;
    let output = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "-c",
            script,
            generation.dir.to_str().unwrap(),
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original_identities = sql::query(&ctx, "SELECT * FROM source_parameter_identities")
        .await
        .unwrap()
        .into_view();
    let missing_identities = sql::query(
        &ctx,
        "SELECT * FROM source_parameter_identities WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("source_parameter_identities").unwrap();
    ctx.register_table("source_parameter_identities", missing_identities)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "source-parameter-identity-equality"),
        "missing source identity passed reconstruction: {violations:?}"
    );
    ctx.deregister_table("source_parameter_identities").unwrap();
    ctx.register_table("source_parameter_identities", original_identities)
        .unwrap();
    let original_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await
        .unwrap()
        .into_view();
    let omitted_completion = sql::query(
        &ctx,
        &format!(
            "SELECT * FROM summary_flow_steps WHERE kind <> {}",
            SummaryFlowStepKind::PrecedingCallNormal.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", omitted_completion)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "missing normal-completion witness passed validation: {violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps.clone())
        .unwrap();
    let missing_link = sql::query(
        &ctx,
        &format!(
            "SELECT * FROM summary_flow_steps WHERE kind <> {}",
            SummaryFlowStepKind::CalleeConditionLink.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", missing_link)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "missing recursive condition link passed validation: {violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps)
        .unwrap();
}

/// W1: a real finalizer proof survives extraction, Delta publication, IPC generation and the
/// native reader. The Python process only loads the generation and issues the semantic query.
#[tokio::test(flavor = "multi_thread")]
async fn finalizer_proof_round_trips_through_the_native_generation_reader() {
    let (dir, store) = compiled("finalizer", false, true).await;
    let generation = bundle(&store, SNAPSHOT, &dir.path().join("generations"))
        .await
        .unwrap();
    let mut tables = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&generation.dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "arrow") {
            let reader =
                arrow_ipc::reader::FileReader::try_new(std::fs::File::open(&path).unwrap(), None)
                    .unwrap();
            let batches = reader.collect::<Result<Vec<_>, _>>().unwrap();
            tables.insert(
                path.file_stem().unwrap().to_str().unwrap().to_owned(),
                batches,
            );
        }
    }
    cpg_schema::serving_support::validate(&tables).unwrap();
    let findings = tables.get_mut("support_findings").unwrap();
    let first = findings
        .iter_mut()
        .find(|batch| batch.num_rows() > 0)
        .unwrap();
    *first = first.slice(1, first.num_rows() - 1);
    assert!(
        cpg_schema::serving_support::validate(&tables).is_err(),
        "a missing finding closure was admitted"
    );
    let script = r#"
import sys
from pathlib import Path
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
from lctx_mcp.server import markdown
generation = load(Path(sys.argv[1]), None)
index = generation.condition_graph
paths, boundaries, total, truncated, work = index.inspect_value_paths(
    "pkg.controls.passthrough", "options", "none", "", True, 0, 20
)
assert not truncated and not boundaries, (paths, boundaries)
assert any(
    path[1] in ("established", "conditional")
    and sum(step[0] == "finalizer_pass" for step in path[3]) == 2
    for path in paths
), paths
brief_id = next(
    brief["brief_id"] for brief in generation.tables["briefs"].to_pylist()
    if brief["access_path"] == "pkg.configure"
)
with served_bundle(Path(sys.argv[1])) as pg:
    capability = pg.capability(brief_id.hex())
coordinates = next(a for a in capability.assertions if a.kind == "coordinates")
finding = next(s.finding for s in coordinates.supports if s.finding is not None)
assert finding.model_id and finding.invocation_id and finding.witnesses, finding
assert finding.source_resolution == "source_span", finding
source = finding.witnesses[0]
assert source.source_path.endswith("pkg/controls.py") and source.end_byte > source.start_byte
rendered = markdown(capability)
assert finding.finding_id in rendered
assert finding.invocation_id in rendered and finding.model_id in rendered
assert f"{source.source_path}:{source.start_byte}-{source.end_byte}" in rendered
"#;
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", script])
        .arg(&generation.dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native finalizer query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn files_of(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// §6.4: rebuilding a generation from the store gives byte-identical files. A second compile of the
/// same library, from another location and module order, gives the same generation.
#[tokio::test(flavor = "multi_thread")]
async fn a_generation_rebuilds_to_the_same_bytes() {
    let (dir, store) = compiled("one", false, false).await;
    let a = bundle(&store, SNAPSHOT, &dir.path().join("gen-a"))
        .await
        .unwrap();
    let b = bundle(&store, SNAPSHOT, &dir.path().join("gen-b"))
        .await
        .unwrap();
    assert_eq!(a.key, b.key);
    assert_eq!(files_of(&a.dir), files_of(&b.dir));
    // Building into an existing generation of the same key is idempotent.
    let again = bundle(&store, SNAPSHOT, &dir.path().join("gen-a"))
        .await
        .unwrap();
    assert_eq!(again.key, a.key);
    let manifest = verify(&a.dir).unwrap();
    // ADR-0010's R2 F1 amendment: a brief's lexical names are its seed's own spellings.
    // `pkg.Server.tool` is also spelled `pkg.Alpha.tool` through a public subclass, which promotes
    // the brief (`symbol_map`) but names nothing in its lexical text.
    let served = |file: &str| {
        let bytes = std::fs::read(a.dir.join(file)).unwrap();
        let reader =
            arrow_ipc::reader::FileReader::try_new(std::io::Cursor::new(bytes), None).unwrap();
        reader.map(|b| b.unwrap()).collect::<Vec<_>>()
    };
    let strings = |b: &arrow_array::RecordBatch, column: &str| -> Vec<String> {
        let c = b.column_by_name(column).unwrap();
        let c = c
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();
        (0..arrow_array::Array::len(c))
            .map(|i| c.value(i).to_owned())
            .collect()
    };
    let symbols: Vec<String> = served("symbol_map.arrow")
        .iter()
        .flat_map(|b| strings(b, "symbol"))
        .collect();
    assert!(
        symbols.contains(&"pkg.Alpha.tool".to_owned()),
        "{symbols:?}"
    );
    let names: Vec<String> = served("lexical_text.arrow")
        .iter()
        .flat_map(|b| strings(b, "text"))
        .map(|t| t.rsplit('\n').next().unwrap_or_default().to_owned())
        .collect();
    assert!(
        names.iter().any(|n| n.split(' ').any(|t| t == "tool")),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|n| n.split(' ').any(|t| t == "alpha")),
        "an inherited spelling named a brief: {names:?}"
    );
    let rows = |f: &str| manifest["files"][f]["rows"].as_u64().unwrap();
    // Five configured seeds; the fixture has no official usage code, so nothing is eligible to
    // fill the budget of six (the increment-2 review's U1).
    assert_eq!(rows("briefs"), 5);
    assert_eq!(rows("vectors"), 5);
    assert_eq!(rows("lexical_text"), 5);
    assert_eq!(rows("embedding_spec"), 1);
    assert!(
        rows("symbol_map") >= 8,
        "every public access path of each brief"
    );
    // `pkg.describe`, which calls nothing.
    assert_eq!(
        manifest["summary"]["briefs"]["unreviewed"]["documentation_only"],
        1
    );
    assert_eq!(manifest["spec_hash"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["condition_kernel_format"], 1);
    let names: Vec<&String> = manifest["files"].as_object().unwrap().keys().collect();
    assert_eq!(
        names,
        [
            "ambient_reads",
            "analysis_condition_nodes",
            "analysis_conditions",
            "assertions",
            "behavior_discharges",
            "behaviors",
            "brief_members",
            "briefs",
            "callable_parameters",
            "condition_nodes",
            "conditions",
            "embedding_spec",
            "evidence",
            "flow_test_leaves",
            "flow_test_value_links",
            "lexical_text",
            "model_context_protocols",
            "model_frame_exit_arguments",
            "model_frame_exit_steps",
            "model_frame_exits",
            "operation_facet_status",
            "operation_facets",
            "operation_text",
            "operation_vectors",
            "operations",
            "place_claims",
            "public_paths",
            "return_completion_certificates",
            "singletons",
            "source_body_completions",
            "source_body_release_inputs",
            "source_body_steps",
            "source_call_bindings",
            "source_call_header_steps",
            "source_call_normals",
            "source_context_arguments",
            "source_context_sites",
            "source_context_value_identities",
            "source_modeled_identities",
            "source_parameter_identities",
            "summary_boundaries",
            "summary_flow_steps",
            "summary_flows",
            "support_attribute_incidences",
            "support_attributes",
            "support_findings",
            "support_members",
            "support_witnesses",
            "supports",
            "symbol_map",
            "vectors",
        ]
    );

    let (other_dir, other_store) = compiled("elsewhere", true, false).await;
    let c = bundle(&other_store, SNAPSHOT, &other_dir.path().join("gen"))
        .await
        .unwrap();
    assert_eq!(c.key, a.key, "the same library, compiled again elsewhere");
    assert_eq!(files_of(&c.dir), files_of(&a.dir));
}

/// A changed file, or a generation under another name, is refused against its manifest.
#[tokio::test(flavor = "multi_thread")]
async fn a_changed_generation_is_refused() {
    let (dir, store) = compiled("one", false, false).await;
    let g = bundle(&store, SNAPSHOT, &dir.path().join("gen"))
        .await
        .unwrap();
    let renamed = dir.path().join("gen").join("0000000000000000");
    copy(&g.dir, &renamed);
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(err.contains("generation key"), "{err}");
    let manifest_path = renamed.join("MANIFEST.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"]["conditions"]["file"] = serde_json::json!("../conditions.arrow");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(err.contains("unexpected served file path"), "{err}");

    let mut mismatched = g.manifest.clone();
    mismatched["projection"]["snapshot_id"] = serde_json::json!("ff".repeat(16));
    let projection: cpg_schema::serving_projection::Manifest =
        serde_json::from_value(mismatched["projection"].clone()).unwrap();
    mismatched["projection_generation"] = serde_json::json!(projection.generation().unwrap());
    std::fs::write(&manifest_path, serde_json::to_vec(&mismatched).unwrap()).unwrap();
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(
        err.contains("projection envelope mismatch: snapshot_id"),
        "{err}"
    );

    let path = g.dir.join("briefs.arrow");
    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 20;
    bytes[last] ^= 0xff;
    std::fs::write(&path, bytes).unwrap();
    let err = verify(&g.dir).unwrap_err().to_string();
    assert!(err.contains("briefs.arrow: its sha256"), "{err}");
}

/// §6.4: a generation never mixes vector spaces: two specs in the snapshot refuse the build.
#[tokio::test(flavor = "multi_thread")]
async fn mixed_embedding_specs_are_refused() {
    let (dir, store) = compiled("one", false, false).await;
    let (_, ctx) = published(&store, SNAPSHOT).await.unwrap().unwrap();
    let doctored = sql::query(
        &ctx,
        "SELECT * FROM embedding_specs UNION ALL \
         SELECT snapshot_id, CAST(X'00000000000000000000000000000000000000000000000000000000000000ff' \
                AS BYTEA) AS spec_hash, spec FROM embedding_specs",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("embedding_specs").unwrap();
    ctx.register_table("embedding_specs", doctored).unwrap();
    let err = build(&ctx, &dir.path().join("gen"))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("mixed embedding specs"), "{err}");
}

/// The serving schema digests are known answers shared with Python (`specs/serving/`, ADR-0019):
/// Python recomputes each from a generation's own files. `LCTX_WRITE_KNOWN_ANSWERS=1` rewrites
/// the file after a declared change of a served schema.
#[test]
fn serving_schema_digests_are_the_shared_known_answers() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/serving/schema_digests.json");
    let mut answers = serde_json::Map::new();
    for f in cpg_schema::bundle::files(1024) {
        answers.insert(
            f.name.to_owned(),
            serde_json::json!({
                "form": cpg_schema::bundle::canonical_form(&f.schema).unwrap(),
                "digest": cpg_schema::bundle::schema_digest(&f.schema).unwrap(),
            }),
        );
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(answers)).unwrap() + "\n";
    if std::env::var_os("LCTX_WRITE_KNOWN_ANSWERS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "a served schema changed: rewrite the known answers deliberately"
    );
}

/// The generation the Python tests serve (`just py-fixture`, DESIGN §11.3): `analysis_shapes` with
/// the fake embedder, built into `$LCTX_PY_FIXTURE`, its key in `CURRENT`. A no-op without it.
#[tokio::test(flavor = "multi_thread")]
async fn writes_the_python_fixture_generation() {
    let Some(out) = std::env::var_os("LCTX_PY_FIXTURE") else {
        return;
    };
    let out = PathBuf::from(out);
    let (_dir, store) = compiled("fixture", false, false).await;
    let g = bundle(&store, SNAPSHOT, &out).await.unwrap();
    std::fs::write(out.join("CURRENT"), &g.key).unwrap();
}
