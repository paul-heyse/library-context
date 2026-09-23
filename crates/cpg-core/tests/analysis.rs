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
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot", "pkg.shadow"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe"]
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

/// Compile the fixture under `config` for `platform`, returning the attempt's result and the
/// store's directory.
async fn compile_config(
    sub: &str,
    config: &str,
    platform: &str,
) -> (
    Result<cpg_core::attempt::Published, cpg_core::CoreError>,
    tempfile::TempDir,
) {
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
        python_platform: platform.to_owned(),
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks::default(),
    })
    .unwrap();
    let analysis = Analysis {
        config: AnalyticsConfig::parse(config).unwrap(),
        embedder: None,
    };
    let result = compile_analyzed(&dir.path().join("store"), s, &out.tables, Some(&analysis)).await;
    (result, dir)
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
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    // A decorator factory reaches its nested callable by a definition arc, and what that calls
    // (slice 1.4 review F1).
    let nested = format!(
        "SELECT count(*) FROM findings f JOIN declarations s ON s.node_id = f.subject_node_id \
         {} WHERE s.qualified_name = 'pkg.server.Server.route' \
           AND {LABEL} IN ('pkg.server.Server.route.decorator', 'pkg.helpers.helper')",
        labelled("f.related_node_id")
    );
    assert_eq!(count(nested).await, 2);
    // Every published rule passes on the analyzed snapshot.
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}

/// Slice 1.4 review F2: seed resolution refuses rather than guesses when a member may come from a
/// base outside the release, or is rebound by an assignment in a class body.
#[tokio::test(flavor = "multi_thread")]
async fn a_seed_that_could_name_another_method_is_refused() {
    for (seed, why) in [
        ("pkg.Shadowed.run", "outside the analyzed release"),
        ("pkg.Aliased.tool", "assignment"),
    ] {
        let config = CONFIG.replace(
            r#"distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe"]"#,
            &format!(r#"distractors = ["{seed}"]"#),
        );
        let err = compile_config("refuse", &config, "linux")
            .await
            .0
            .unwrap_err()
            .to_string();
        assert!(err.contains(why), "{seed}: {err}");
    }
}

/// Slice 1.5 review F3: each template says what its finding's fields show. Under a witness cap of
/// one, a call-site count is a floor. A boundary below a callee is not the seed's own call. The
/// override-open hop is the one named.
#[tokio::test(flavor = "multi_thread")]
async fn templates_say_what_the_findings_show() {
    let (result, dir) = compile_config(
        "cap",
        &CONFIG.replace("max_witnesses = 3", "max_witnesses = 1"),
        "linux",
    )
    .await;
    result.unwrap();
    let (_, ctx) = published(&dir.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let texts = text(
        &ctx,
        "SELECT a.text FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE b.title = 'pkg.Server.tool' ORDER BY ba.ordinal",
    )
    .await;
    let line = |start: &str| {
        texts
            .lines()
            .map(|l| l.trim_matches(|c| c == '|' || c == ' '))
            .find(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no line starts {start:?} in\n{texts}"))
            .to_owned()
    };
    assert_eq!(
        line("`pkg.Server.tool` already calls `pkg.helpers.helper`"),
        "`pkg.Server.tool` already calls `pkg.helpers.helper` (1 or more call sites)."
    );
    let deep = line("Callables `pkg.Server.tool` reaches call into code outside");
    assert!(deep.contains("`builtins.isinstance`"), "{deep}");
    assert!(!deep.contains("`extdep.external`"), "{deep}");
    assert!(
        line("`pkg.Server.tool` calls into code outside").contains("`extdep.external`"),
        "{texts}"
    );
    assert!(
        line("`pkg.Server.tool` already reaches `pkg.handlers.Handler.handle`").ends_with(
            "(`pkg.handlers.run`'s call to `pkg.handlers.Handler.handle` is overridable)."
        ),
        "{texts}"
    );
}

/// ADR-0019 review F4 through the whole attempt: a vertex budget truncates the invocation, which is
/// `partial` with its stop reason, and Stage F states it as a limit of the brief.
#[tokio::test(flavor = "multi_thread")]
async fn a_vertex_budget_is_partial_and_a_stated_limit() {
    let (result, dir) = compile_config(
        "budget",
        &CONFIG.replace("max_vertices = 128", "max_vertices = 1"),
        "linux",
    )
    .await;
    result.unwrap();
    let (_, ctx) = published(&dir.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    insta::assert_snapshot!(
        "vertex_budget",
        text(
            &ctx,
            &format!(
                "SELECT {LABEL} AS seed, i.completion, i.stop_reason, i.vertices_examined \
                 FROM analysis_invocations i {} ORDER BY seed",
                labelled("i.subject_node_id")
            )
        )
        .await
    );
    let limits = text(
        &ctx,
        "SELECT a.text FROM assertions a WHERE a.text LIKE '%stopped at its budget%' ORDER BY a.text",
    )
    .await;
    assert!(limits.contains("`pkg.Server.tool`"), "{limits}");
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
    // Stage F too (slice 1.5 review F6): each table's leading columns are its key.
    for table in SYNTHESIS {
        let q = format!("SELECT * EXCLUDE (snapshot_id) FROM {table} ORDER BY 1, 2, 3");
        assert_eq!(text(&a, &q).await, text(&b, &q).await, "{table}");
    }
}

/// Stage F's tables. An output pin leaves out their lineage columns: a run's and a model's ids move
/// with the compiler digest, and a cited fact's with its run (ADR-0019).
const SYNTHESIS: [&str; 7] = [
    "evidence",
    "assertions",
    "assertion_support",
    "briefs",
    "brief_assertions",
    "brief_members",
    "brief_documents",
];

fn identity_of(table: &str) -> String {
    let lineage = match table {
        "assertions" | "briefs" => "snapshot_id, run_id, model_id",
        "evidence" => "snapshot_id, cited_fact_id",
        _ => "snapshot_id",
    };
    format!("SELECT * EXCLUDE ({lineage}) FROM {table} ORDER BY 1, 2, 3")
}

/// ADR-0019 review F7, slice 1.5 review F6: findings, evidence, assertions and briefs are named
/// by content, never by the run-scoped facts they cite. Another target platform changes every run,
/// so every fact id, and renames none of them. (A dependency file's own change is a changed
/// dependency: an unowned module's id is its content, DESIGN §3.4.1.)
#[tokio::test(flavor = "multi_thread")]
async fn synthesis_ids_ignore_the_runs_they_cite() {
    let (a, da) = compile_config("ctx", CONFIG, "linux").await;
    let (b, db) = compile_config("ctx", CONFIG, "darwin").await;
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.content_digest, b.content_digest, "the runs changed");
    let (_, ctx_a) = published(&da.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let (_, ctx_b) = published(&db.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let facts = "SELECT cited_fact_id FROM evidence WHERE cited_fact_id IS NOT NULL ORDER BY 1";
    assert_ne!(
        text(&ctx_a, facts).await,
        text(&ctx_b, facts).await,
        "the cited facts are the runs'"
    );
    for q in [
        "SELECT finding_id FROM findings ORDER BY 1",
        "SELECT evidence_id FROM evidence ORDER BY 1",
        "SELECT assertion_id FROM assertions ORDER BY 1",
        "SELECT brief_id FROM briefs ORDER BY 1",
    ] {
        assert_eq!(text(&ctx_a, q).await, text(&ctx_b, q).await, "{q}");
    }
}

/// Stage F on the fixture (DESIGN §10): each seed's brief, its assertions in order with their
/// kind, status and text, and every evidence text equal to the exact bytes of its span.
#[tokio::test(flavor = "multi_thread")]
async fn briefs_are_synthesized_from_findings_and_verbatim_evidence() {
    let (ctx, _dir) = analyzed("one", false).await;
    insta::assert_snapshot!(
        "briefs",
        text(
            &ctx,
            "SELECT b.title, ba.ordinal, a.assertion_kind AS kind, a.evidence_status AS status, \
                    a.text \
             FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
             JOIN assertions a ON a.assertion_id = ba.assertion_id \
             ORDER BY b.title, ba.ordinal"
        )
        .await
    );
    insta::assert_snapshot!(
        "brief_documents",
        text(
            &ctx,
            "SELECT b.title, d.chunk, d.text FROM brief_documents d \
             JOIN briefs b ON b.brief_id = d.brief_id ORDER BY b.title, d.chunk"
        )
        .await
    );
    // Every span evidence is its source's exact bytes, past a non-ASCII byte (review F8).
    let rows = batches(
        &ctx,
        "SELECT e.start_byte, e.end_byte, e.text, s.text AS source FROM evidence e \
         JOIN source_files s ON s.module_node_id = e.module_node_id \
         WHERE e.start_byte IS NOT NULL",
    )
    .await;
    let mut checked = 0;
    let mut past_non_ascii = 0;
    for b in &rows {
        let col = |n: &str| b.column_by_name(n).unwrap().clone();
        let start = arrow_array::cast::AsArray::as_primitive::<arrow_array::types::Int64Type>(
            &col("start_byte"),
        )
        .clone();
        let end = arrow_array::cast::AsArray::as_primitive::<arrow_array::types::Int64Type>(&col(
            "end_byte",
        ))
        .clone();
        let texts = arrow_cast::cast(&col("text"), &arrow_schema::DataType::Utf8).unwrap();
        let sources = arrow_cast::cast(&col("source"), &arrow_schema::DataType::Utf8).unwrap();
        let texts = arrow_array::cast::AsArray::as_string::<i32>(&texts);
        let sources = arrow_array::cast::AsArray::as_string::<i32>(&sources);
        for i in 0..b.num_rows() {
            let (s, e) = (start.value(i) as usize, end.value(i) as usize);
            assert_eq!(
                &sources.value(i).as_bytes()[s..e],
                texts.value(i).as_bytes()
            );
            checked += 1;
            past_non_ascii += usize::from(!sources.value(i).as_bytes()[..s].is_ascii());
        }
    }
    assert!(checked >= 3, "the docstring and parameter evidence");
    // Slice 1.5 review F5: some of it lies past a non-ASCII byte, where chars and bytes differ.
    assert!(past_non_ascii >= 1, "a span past a non-ASCII byte");
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());

    // ADR-0019 review O4: the analysis output is pinned to the versions that name it, so an
    // output change without a version bump fails here (the compiler digest would not move).
    // To change the output: bump COMPILER_OUTPUT_VERSION or TEMPLATE_VERSION and append a row.
    const LEDGER: &[(u32, i64, &str)] = &[
        (
            2,
            1,
            "d07a8be1d8d04d7ac762799bfc9f93546e9aaf08dce497cebfc6e0283e70b4d6",
        ),
        (
            3,
            2,
            "1a936cb57baca6b2e5171b16edd2511185ebf488aaecafa0b4ff24851de063d8",
        ),
        (
            3,
            3,
            "9a1f41e215fbb48772b9caf40a8ab75615eda7bcb0c030dc1298973591600ef8",
        ),
        (
            4,
            3,
            "e962da02699a07ec2aa584c0ed2f0c974ab351cd1bd712fcfa587b1e00506407",
        ),
    ];
    // Texts, and every identity column of Stage F's tables (slice 1.5 review F6).
    let mut output = format!(
        "{}\n{}",
        text(&ctx, &findings_query("pkg.server.Server.tool")).await,
        text(
            &ctx,
            "SELECT b.title, ba.ordinal, a.text FROM briefs b \
             JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
             JOIN assertions a ON a.assertion_id = ba.assertion_id ORDER BY b.title, ba.ordinal"
        )
        .await
    );
    for table in SYNTHESIS {
        output += &text(&ctx, &identity_of(table)).await;
    }
    let digest = cpg_schema::id::content_digest(output.as_bytes()).hex();
    let now = (
        cpg_core::attempt::COMPILER_OUTPUT_VERSION,
        cpg_core::synth::TEMPLATE_VERSION,
        digest.as_str(),
    );
    assert_eq!(
        LEDGER.last(),
        Some(&now),
        "append {now:?} with bumped versions"
    );
    for (a, b) in LEDGER.iter().zip(LEDGER.iter().skip(1)) {
        assert!(
            (a.0, a.1) < (b.0, b.1),
            "a ledger row reuses the versions of the one before"
        );
    }
}

/// Each rule the analysis tables add rejects an injected violation (C6 review F1's meta-test).
#[tokio::test(flavor = "multi_thread")]
async fn the_analysis_rules_reject_their_violations() {
    let (ctx, _dir) = analyzed("one", false).await;
    for table in [
        "analysis_invocations",
        "findings",
        "witnesses",
        "assertions",
        "assertion_policy",
        "assertion_support",
        "evidence",
        "briefs",
        "brief_assertions",
        "brief_members",
        "brief_documents",
        "embedding_specs",
        "embedding_cache",
    ] {
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
                    call_site_node_id, callee_node_id, edge_id, modality, arc_kind, phase \
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
            "semantic:assertion-policy",
            "assertions",
            "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, assertion_kind, \
                    subject_node_id, applicable_case, \
                    CASE WHEN assertion_kind = 1 THEN CAST(1 AS SMALLINT) ELSE evidence_status END \
                      AS evidence_status, \
                    text, conditions, limitations, template_version \
             FROM assertions_published",
        ),
        // An Outcome whose evidence support is gone states more than it cites.
        (
            "semantic:assertion-status-derived",
            "assertion_support",
            "SELECT * FROM assertion_support_published WHERE evidence_id IS NULL",
        ),
        // A parameter relabelled `documented` on fact evidence alone (the policy permits it).
        (
            "semantic:assertion-status-derived",
            "assertions",
            "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, assertion_kind, \
                    subject_node_id, applicable_case, \
                    CASE WHEN assertion_kind = 3 THEN CAST(1 AS SMALLINT) ELSE evidence_status END \
                      AS evidence_status, \
                    text, conditions, limitations, template_version \
             FROM assertions_published",
        ),
        // A statistical finding under a structural assertion.
        (
            "semantic:assertion-status-derived",
            "findings",
            "SELECT snapshot_id, finding_id, invocation_id, finding_kind, subject_node_id, \
                    related_node_id, CAST(2 AS SMALLINT) AS evidence_status, depth, stop_reason, \
                    witnesses_omitted, score, condition_node_id \
             FROM findings_published",
        ),
        (
            "semantic:assertion-text-iff-resolved",
            "assertions",
            "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, assertion_kind, \
                    subject_node_id, applicable_case, evidence_status, \
                    CAST(NULL AS VARCHAR) AS text, conditions, limitations, template_version \
             FROM assertions_published",
        ),
        (
            "semantic:support-cites-one",
            "assertion_support",
            "SELECT snapshot_id, assertion_id, role, ordinal, finding_id, \
                    CAST(NULL AS BYTEA) AS evidence_id \
             FROM assertion_support_published WHERE finding_id IS NULL \
             UNION ALL SELECT * FROM assertion_support_published WHERE finding_id IS NOT NULL",
        ),
        (
            "semantic:evidence-text-bytes",
            "evidence",
            "SELECT snapshot_id, evidence_id, evidence_kind, cited_fact_id, node_id, \
                    module_node_id, start_byte, end_byte + 1 AS end_byte, text \
             FROM evidence_published",
        ),
        (
            "semantic:brief-member-exported",
            "brief_members",
            "SELECT snapshot_id, brief_id, 'elsewhere.' || access_path AS access_path, \
                    export_node_id, declaration_node_id \
             FROM brief_members_published",
        ),
        // Slice 1.5 review O3: a member path naming another member of an exported class.
        (
            "semantic:brief-member-exported",
            "brief_members",
            "SELECT DISTINCT m.snapshot_id, m.brief_id, \
                    CASE WHEN m.access_path = e.access_path THEN m.access_path \
                         ELSE m.access_path || '_elsewhere' END AS access_path, \
                    m.export_node_id, m.declaration_node_id \
             FROM brief_members_published m JOIN exports e ON e.export_node_id = m.export_node_id",
        ),
        // An analysis-backed brief whose finding supports are gone.
        (
            "semantic:brief-cites-analysis",
            "assertion_support",
            "SELECT * FROM assertion_support_published WHERE finding_id IS NULL",
        ),
        // Slice 1.5 review F4, the other direction: an analysis-backed brief labelled
        // documentation-only.
        (
            "semantic:brief-cites-analysis",
            "briefs",
            "SELECT snapshot_id, brief_id, run_id, model_id, seed_node_id, access_path, title, \
                    applicable_case, true AS documentation_only, review_state \
             FROM briefs_published",
        ),
        // `pkg.describe`'s brief is documentation-only; without its Outcome it says nothing.
        (
            "semantic:documentation-only-has-outcome",
            "brief_assertions",
            "SELECT ba.* FROM brief_assertions_published ba \
             JOIN assertions a ON a.assertion_id = ba.assertion_id WHERE a.assertion_kind <> 0",
        ),
        (
            "semantic:embedding-dimensions",
            "embedding_cache",
            "SELECT * FROM embedding_cache_published UNION ALL \
             (SELECT spec_hash, input_hash, array_slice(vector, 1, 8) AS vector, model \
              FROM embedding_cache_published LIMIT 1)",
        ),
        (
            "semantic:direct-delegation-is-definite",
            "witnesses",
            "SELECT snapshot_id, finding_id, path, step, caller_node_id, call_site_node_id, \
                    callee_node_id, edge_id, CAST(1 AS SMALLINT) AS modality, arc_kind, phase \
             FROM witnesses_published",
        ),
        (
            "semantic:witness-edge",
            "witnesses",
            "SELECT snapshot_id, finding_id, path, step, caller_node_id, call_site_node_id, \
                    caller_node_id AS callee_node_id, edge_id, modality, arc_kind, phase \
             FROM witnesses_published",
        ),
        (
            "ref:witnesses.edge_id->edges",
            "witnesses",
            "SELECT snapshot_id, finding_id, path, step, caller_node_id, call_site_node_id, \
                    callee_node_id, CAST(X'00000000000000000000000000000000' AS BYTEA) AS edge_id, \
                    modality, arc_kind, phase \
             FROM witnesses_published",
        ),
    ];
    let cases = cases.into_iter().chain([
        // Slice 1.7: a document's cache key is whole.
        (
            "semantic:document-key-whole",
            "brief_documents",
            "SELECT snapshot_id, brief_id, chunk, text, CAST(NULL AS BYTEA) AS spec_hash, \
                    input_hash \
             FROM brief_documents_published",
        ),
        // An embedded document whose vector the cache lacks.
        (
            "semantic:document-vector-cached",
            "brief_documents",
            "SELECT snapshot_id, brief_id, chunk, text, spec_hash, \
                    CAST(X'00000000000000000000000000000000000000000000000000000000000000ff' \
                         AS BYTEA) AS input_hash \
             FROM brief_documents_published",
        ),
        // Two specs in one snapshot.
        (
            "semantic:one-embedding-spec",
            "embedding_specs",
            "SELECT * FROM embedding_specs_published UNION ALL \
             SELECT snapshot_id, \
                    CAST(X'00000000000000000000000000000000000000000000000000000000000000ff' \
                         AS BYTEA) AS spec_hash, spec \
             FROM embedding_specs_published",
        ),
    ]);
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
