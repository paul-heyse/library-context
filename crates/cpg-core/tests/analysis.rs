//! Stage E on `analysis_shapes` (ADR-0019; DESIGN §5, §9.1, §12): Pass A from a method seed over
//! the invocation projection, with parallel call sites, two paths to one helper, a property
//! getter, an override-open dispatch, a dependency, a synthetic constructor, a callee outside the
//! subsystem, an unresolved call, a callable value never invoked, a module-level caller and a
//! chain deeper than the budget. Plus determinism, and each new rule rejecting a violation.

use std::path::Path;

use arrow_array::RecordBatch;
use cpg_core::analyze::{Analysis, project};
use cpg_core::attempt::compile_analyzed;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::codebook::{
    Codebook, FindingKind, InvocationPhase, Modality, NodeKind, StopReason,
};
use cpg_schema::id::Id;
use cpg_schema::projection::invocation;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;
use lctx_analytics::config::AnalyticsConfig;

const CONFIG: &str = r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 2
serve_unreviewed = true
"#;

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

/// The fixture extracted and compiled with the analytics config; the session reads the published
/// snapshot.
async fn analyzed(sub: &str, reverse: bool) -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    let s = Id([7; 16]);
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
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks {
            reverse_module_order: reverse,
            ..Default::default()
        },
    })
    .unwrap();
    let analysis = Analysis {
        config: AnalyticsConfig::parse(CONFIG).unwrap(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, s, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    (ctx, dir)
}

async fn batches(ctx: &SessionContext, statement: &str) -> Vec<RecordBatch> {
    sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap()
}

async fn text(ctx: &SessionContext, statement: &str) -> String {
    pretty_format_batches(&batches(ctx, statement).await)
        .unwrap()
        .to_string()
}

/// A readable label for any node a finding names.
const LABEL: &str = "COALESCE(d.qualified_name, cd.module_name || '.' || cd.qualified_name, \
     'synthetic ' || sc.function_key, CASE WHEN c.node_id IS NOT NULL THEN 'call site' END)";

fn labelled(column: &str) -> String {
    format!(
        "LEFT JOIN declarations d ON d.node_id = {column} \
         LEFT JOIN context_definitions cd ON cd.symbol_node_id = {column} \
         LEFT JOIN synthetic_callables sc ON sc.node_id = {column} \
         LEFT JOIN call_syntax c ON c.node_id = {column}"
    )
}

/// The seed's findings: kind, target, depth, stop reason, omitted flag and witness count.
fn findings_query(seed: &str) -> String {
    format!(
        "SELECT f.finding_kind AS kind, {LABEL} AS target, f.depth, f.stop_reason AS stop, \
                f.witnesses_omitted AS omitted, \
                (SELECT count(DISTINCT w.path) FROM witnesses w \
                 WHERE w.finding_id = f.finding_id) AS paths \
         FROM findings f JOIN declarations s ON s.node_id = f.subject_node_id \
         {} \
         WHERE s.qualified_name = '{seed}' \
         ORDER BY kind, target, f.depth, stop",
        labelled("f.related_node_id")
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn pass_a_finds_the_known_answers_on_analysis_shapes() {
    let (ctx, _dir) = analyzed("one", false).await;
    let seed = "pkg.server.Server.tool";
    insta::assert_snapshot!("pass_a_tool", text(&ctx, &findings_query(seed)).await);

    let rows = |kind: FindingKind, target: &str| {
        format!(
            "SELECT count(*) FROM findings f JOIN declarations s ON s.node_id = f.subject_node_id \
             {} WHERE s.qualified_name = '{seed}' AND f.finding_kind = {} AND {LABEL} = '{target}'",
            labelled("f.related_node_id"),
            kind.code()
        )
    };
    let count = |q: String| {
        let ctx = &ctx;
        async move {
            let b = batches(ctx, &q).await;
            b[0].column(0)
                .as_any()
                .downcast_ref::<arrow_array::Int64Array>()
                .unwrap()
                .value(0)
        }
    };
    // Two parallel call sites to `helper`: one direct delegation, two witnesses.
    assert_eq!(
        count(rows(FindingKind::DirectDelegation, "pkg.helpers.helper")).await,
        1
    );
    // The override-open dispatch is followed, never a direct delegation (§3.6).
    assert_eq!(
        count(rows(
            FindingKind::DirectDelegation,
            "pkg.handlers.Handler.handle"
        ))
        .await,
        0
    );
    assert_eq!(
        count(rows(
            FindingKind::BoundedDelegationPath,
            "pkg.handlers.Handler.handle"
        ))
        .await,
        1
    );
    // Boundaries: a dependency, a release callee outside the subsystem.
    assert_eq!(
        count(rows(FindingKind::ImplementationBoundary, "extdep.external")).await,
        1
    );
    assert_eq!(
        count(rows(
            FindingKind::ImplementationBoundary,
            "pkg.other.outside"
        ))
        .await,
        1
    );
    // The chain below `dispatch` stops at the depth budget.
    assert_eq!(
        count(rows(
            FindingKind::BoundedDelegationPath,
            "pkg.helpers.deep_one"
        ))
        .await,
        1
    );
    assert_eq!(
        count(rows(
            FindingKind::BoundedDelegationPath,
            "pkg.helpers.deep_two"
        ))
        .await,
        0
    );
    // The unresolved `getattr(fn, "x")()` result call.
    let unresolved = format!(
        "SELECT count(*) FROM findings WHERE finding_kind = {} AND stop_reason = {}",
        FindingKind::IncompleteResolution.code(),
        StopReason::UnresolvedSite.code()
    );
    assert!(count(unresolved).await >= 1);
    // The property getter is reached through a `property_get` arc.
    let getter = format!(
        "SELECT count(*) FROM witnesses w JOIN declarations d ON d.node_id = w.callee_node_id \
         WHERE d.qualified_name = 'pkg.server.Server.label' AND w.phase = {}",
        InvocationPhase::PropertyGet.code()
    );
    assert!(
        count(getter).await >= 1,
        "the getter arc is in the projection"
    );
    // A candidate step is recorded as such on its witness.
    let candidate = format!(
        "SELECT count(*) FROM witnesses w JOIN declarations d ON d.node_id = w.callee_node_id \
         WHERE d.qualified_name = 'pkg.handlers.Handler.handle' AND w.modality = {}",
        Modality::Candidate.code()
    );
    assert!(count(candidate).await >= 1);
    // Each seed has one invocation, with its parameters, projection and libraries recorded.
    insta::assert_snapshot!(
        "pass_a_invocations",
        text(
            &ctx,
            &format!(
                "SELECT {LABEL} AS seed, i.method, i.parameters, i.library_versions, \
                        i.completion, i.stop_reason, i.vertices_examined, i.arcs_examined \
                 FROM analysis_invocations i {} ORDER BY seed",
                labelled("i.subject_node_id")
            )
        )
        .await
    );
    // The public alias lists the access paths naming the seed.
    insta::assert_snapshot!(
        "pass_a_aliases",
        text(
            &ctx,
            "SELECT m.label FROM finding_members m ORDER BY m.label"
        )
        .await
    );

    // The projection keeps a module-level caller as a typed vertex.
    let p = project(&ctx, &invocation()).await.unwrap();
    assert!(
        p.arcs
            .iter()
            .any(|a| p.kinds[a.src as usize] == NodeKind::Module),
        "the module-level call in pkg.boot is an arc from its module"
    );
    // Every published rule passes on the analyzed snapshot.
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn pass_a_is_identical_across_module_order_and_location() {
    let (a, _da) = analyzed("one", false).await;
    let (b, _db) = analyzed("elsewhere", true).await;
    for table in [
        "findings",
        "witnesses",
        "finding_members",
        "analysis_invocations",
    ] {
        let q = format!("SELECT * EXCLUDE (snapshot_id) FROM {table} ORDER BY 1, 2, 3, 4");
        assert_eq!(text(&a, &q).await, text(&b, &q).await, "{table}");
    }
}

/// Each rule the analysis tables add rejects an injected violation (C6 review F1's meta-test).
#[tokio::test(flavor = "multi_thread")]
async fn the_analysis_rules_reject_their_violations() {
    let (ctx, _dir) = analyzed("one", false).await;
    for table in ["analysis_invocations", "findings", "witnesses"] {
        let published = ctx.table(table).await.unwrap();
        ctx.register_table(format!("{table}_published").as_str(), published.into_view())
            .unwrap();
    }
    let cases = [
        (
            "semantic:invocation-model-producer",
            "analysis_invocations",
            "SELECT snapshot_id, invocation_id, run_id, 'ffff/pass-a' AS model_id, extraction_mode, \
                    method, parameters, parameters_digest, projection_digest, library_versions, \
                    subject_node_id, seed, iterations, residual, converged, quality_history, \
                    candidate_set_size, vertices_examined, arcs_examined, completion, stop_reason \
             FROM analysis_invocations_published",
        ),
        (
            "semantic:witness-chain",
            "witnesses",
            "SELECT snapshot_id, finding_id, path, step, \
                    CASE WHEN step > 0 THEN callee_node_id ELSE caller_node_id END \
                      AS caller_node_id, \
                    call_site_node_id, callee_node_id, edge_id, modality, phase \
             FROM witnesses_published",
        ),
        (
            "finite:findings.score",
            "findings",
            "SELECT snapshot_id, finding_id, invocation_id, finding_kind, subject_node_id, \
                    related_node_id, evidence_status, depth, stop_reason, witnesses_omitted, \
                    CAST('NaN' AS DOUBLE) AS score, condition_node_id \
             FROM findings_published",
        ),
        (
            "semantic:finding-status-policy",
            "findings",
            "SELECT snapshot_id, finding_id, invocation_id, finding_kind, subject_node_id, \
                    related_node_id, CAST(2 AS SMALLINT) AS evidence_status, depth, stop_reason, \
                    witnesses_omitted, score, condition_node_id \
             FROM findings_published",
        ),
        (
            "semantic:invocation-run-is-compiler",
            "analysis_invocations",
            "SELECT snapshot_id, invocation_id, \
                    (SELECT min(run_id) FROM runs WHERE cardinality(families) > 0) AS run_id, \
                    model_id, extraction_mode, method, parameters, parameters_digest, \
                    projection_digest, library_versions, subject_node_id, seed, iterations, \
                    residual, converged, quality_history, candidate_set_size, vertices_examined, \
                    arcs_examined, completion, stop_reason \
             FROM analysis_invocations_published",
        ),
        (
            "ref:witnesses.edge_id->edges",
            "witnesses",
            "SELECT snapshot_id, finding_id, path, step, caller_node_id, call_site_node_id, \
                    callee_node_id, CAST(X'00000000000000000000000000000000' AS BYTEA) AS edge_id, \
                    modality, phase \
             FROM witnesses_published",
        ),
    ];
    let rules = cpg_schema::rules::rules();
    for (rule, table, view) in cases {
        let doctored = sql::query(&ctx, view).await.unwrap().into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, doctored).unwrap();
        let query = &rules.iter().find(|r| r.name == rule).expect(rule).sql;
        let rows: usize = batches(&ctx, query)
            .await
            .iter()
            .map(RecordBatch::num_rows)
            .sum();
        assert!(rows > 0, "{rule} accepted its violation");
        let original = ctx
            .table(format!("{table}_published").as_str())
            .await
            .unwrap()
            .into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}
