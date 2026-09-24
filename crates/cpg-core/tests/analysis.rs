//! Stage E on `analysis_shapes` (ADR-0019; DESIGN §5, §9.1, §12): Pass A from a method seed over
//! the invocation projection, with parallel call sites, two paths to one helper, a property
//! getter, an override-open dispatch, a dependency, a synthetic constructor, a callee outside the
//! subsystem, an unresolved call, a callable value never invoked, a module-level caller and a
//! chain deeper than the budget. Plus determinism, and each new rule rejecting a violation.

use std::path::Path;

use arrow_array::RecordBatch;
use cpg_core::analyze::{Analysis, Techniques, project};
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
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot", "pkg.shadow", "pkg.controls"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe", "pkg.configure", "pkg.Catalog.add_tool"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 6
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
    compile_variant(sub, config, platform, kernels()).await
}

/// [`compile_config`] with an analytics variant (§9.8).
async fn compile_variant(
    sub: &str,
    config: &str,
    platform: &str,
    techniques: Techniques,
) -> (
    Result<cpg_core::attempt::Published, cpg_core::CoreError>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let result = compile_at(dir.path(), sub, Id([7; 16]), config, platform, techniques).await;
    (result, dir)
}

/// The fixture extracted under `dir/sub` as snapshot `s` and compiled into `dir/store`.
async fn compile_at(
    dir: &Path,
    sub: &str,
    s: Id,
    config: &str,
    platform: &str,
    techniques: Techniques,
) -> Result<cpg_core::attempt::Published, cpg_core::CoreError> {
    let base = dir.join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
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
        techniques,
    };
    compile_analyzed(&dir.join("store"), s, &out.tables, Some(&analysis)).await
}

/// The variant these tests exercise the kernels through: the techniques the keep rule turned off
/// by default (ADR-0020) are still reviewed code, frozen and reachable as variants.
fn kernels() -> Techniques {
    Techniques::parse("+communities,+fca,+knn").unwrap()
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
        techniques: kernels(),
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
         WHERE s.qualified_name = '{seed}' AND f.finding_kind <= 5 \
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
                 FROM analysis_invocations i {} WHERE i.method <= 2 \
                 ORDER BY seed, i.method",
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
            "SELECT m.label FROM finding_members m \
             JOIN findings f ON f.finding_id = m.finding_id \
             WHERE f.finding_kind <> 11 ORDER BY m.label"
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
/// base outside the release, or is rebound by an assignment in a class body. Since the holistic
/// assessment's A1 a seed is a `public_paths` row, and the relation holds none for either
/// (`public_paths_agree_with_the_seed_resolution`), so the compile stops naming the seed.
#[tokio::test(flavor = "multi_thread")]
async fn a_seed_that_could_name_another_method_is_refused() {
    for (seed, why) in [
        ("pkg.Shadowed.run", "not a public path"),
        ("pkg.Aliased.tool", "not a public path"),
    ] {
        let config = CONFIG.replace(
            r#"distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe", "pkg.configure", "pkg.Catalog.add_tool"]"#,
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

/// The holistic assessment's A1: `public_paths` agrees with the seed resolution (`member()`) on
/// every fixture seed: each brief's seed path is a row naming its seed node. It refuses where
/// `member()` refuses: `pkg.Shadowed.run` (a base outside the release) and `pkg.Aliased.tool` (an
/// assignment) have no row.
#[tokio::test(flavor = "multi_thread")]
async fn public_paths_agree_with_the_seed_resolution() {
    let (ctx, _dir) = analyzed("public", false).await;
    let disagree = text(
        &ctx,
        "SELECT b.access_path FROM briefs b LEFT ANTI JOIN public_paths p \
           ON p.access_path = b.access_path AND p.node_id = b.seed_node_id",
    )
    .await;
    assert_eq!(
        disagree.lines().filter(|l| l.contains("pkg")).count(),
        0,
        "{disagree}"
    );
    let briefs = text(&ctx, "SELECT access_path FROM briefs").await;
    assert!(
        briefs.lines().filter(|l| l.contains("pkg")).count() >= 3,
        "{briefs}"
    );
    let refused = text(
        &ctx,
        "SELECT access_path FROM public_paths \
         WHERE access_path IN ('pkg.Shadowed.run', 'pkg.Aliased.tool')",
    )
    .await;
    assert_eq!(
        refused.lines().filter(|l| l.contains("pkg")).count(),
        0,
        "{refused}"
    );
    // Each brief member is a public path of its seed (step 5 serves them from the table).
    let members = text(
        &ctx,
        "SELECT m.access_path FROM brief_members m JOIN briefs b ON b.brief_id = m.brief_id \
         LEFT ANTI JOIN public_paths p ON p.access_path = m.access_path \
           AND p.node_id = b.seed_node_id",
    )
    .await;
    assert_eq!(
        members.lines().filter(|l| l.contains("pkg")).count(),
        0,
        "{members}"
    );
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
    // Slice 2.1 review F1: a restriction carries its path's qualifiers, and none is published
    // that the seed cannot trigger.
    let configure = text(
        &ctx,
        "SELECT a.text FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE b.title = 'pkg.configure' AND a.assertion_kind = 8 ORDER BY ba.ordinal",
    )
    .await;
    let raises = |callee: &str| {
        configure
            .lines()
            .map(|l| l.trim_matches(|c| c == '|' || c == ' '))
            .find(|l| l.starts_with(&format!("`{callee}` raises")))
            .map(str::to_owned)
    };
    assert!(
        raises("pkg.controls.Registry.add").is_some_and(|l| l
            .ends_with("(`pkg.configure`'s call to `pkg.controls.Registry.add` is overridable).")),
        "{configure}"
    );
    assert!(
        raises("pkg.controls.slow").is_some_and(
            |l| l.ends_with("(`pkg.configure` calls `pkg.controls.slow` only on some paths).")
        ),
        "{configure}"
    );
    for never in [
        "pkg.controls.strict",
        "pkg.controls.muted",
        "pkg.controls.probe_attr",
    ] {
        assert_eq!(raises(never), None, "{configure}");
    }
}

/// Pass B (DESIGN §9.2, §12) on `pkg.configure`: forwarding directly, through one alias and over a
/// bound receiver; one parameter to two mappings of one callee; a literal; guards reached through
/// forwarding. Never followed: `**options`, a guard behind the caller's `try`, a guard after the
/// parameter is rebound.
#[tokio::test(flavor = "multi_thread")]
async fn pass_b_finds_the_known_answers_on_analysis_shapes() {
    let (ctx, _dir) = analyzed("one", false).await;
    let rows = text(
        &ctx,
        "SELECT f.finding_kind AS kind, src.label AS source, d.qualified_name AS callee, \
                ps.name AS formal, COALESCE(v.label, why.label) AS value, f.depth, \
                (SELECT count(*) FROM finding_members a WHERE a.finding_id = f.finding_id \
                 AND a.role = 3) AS aliases, \
                (SELECT count(*) FROM finding_members a WHERE a.finding_id = f.finding_id \
                 AND a.role = 7) AS conditional \
         FROM findings f JOIN declarations s ON s.node_id = f.subject_node_id \
         LEFT JOIN finding_members src ON src.finding_id = f.finding_id AND src.role = 1 \
         LEFT JOIN finding_members v ON v.finding_id = f.finding_id AND v.role = 2 \
         LEFT JOIN finding_members why ON why.finding_id = f.finding_id AND why.role = 8 \
         LEFT JOIN finding_members fm ON fm.finding_id = f.finding_id AND fm.role = 4 \
         LEFT JOIN parameter_syntax ps ON ps.node_id = COALESCE(fm.node_id, f.related_node_id) \
         LEFT JOIN declarations d ON d.node_id = COALESCE(ps.function_node_id, \
           CASE WHEN f.finding_kind = 10 THEN f.related_node_id END) \
         WHERE s.qualified_name = 'pkg.controls.configure' AND f.finding_kind IN (6, 7, 8, 10) \
         ORDER BY kind, source, callee, formal, value",
    )
    .await;
    insta::assert_snapshot!("pass_b_configure", rows);
    // No restriction: behind a `try` or a suppressing `with`, under the caller's own test of the
    // value, on an attribute, after rebinding, from an unmapped `**` argument (review F1, F5).
    for absent in [
        "pkg.controls.passthrough",
        "pkg.controls.checked",
        "pkg.controls.muted",
        "pkg.controls.strict",
        "pkg.controls.probe_attr",
        "pkg.controls.rebinding",
    ] {
        let raised = rows
            .lines()
            .filter(|l| l.starts_with("| 8 ") && l.contains(absent))
            .count();
        assert_eq!(raised, 0, "a guard in {absent} is reported:\n{rows}");
    }
    // Declines leave a trace (review F4): rebound, computed, unpacked or taken by no formal.
    for (source, callee, why) in [
        ("note", "pkg.controls.annotate", "rebound"),
        ("name", "pkg.controls.annotate", "computed"),
        ("name", "pkg.controls.swap", "unmapped"),
        ("extra", "pkg.controls.swap", "unmapped"),
        ("options", "pkg.controls.passthrough", "unmapped"),
    ] {
        assert!(
            rows.lines().any(|l| l.starts_with("| 10 ")
                && l.contains(&format!("| {source} "))
                && l.contains(callee)
                && l.contains(why)),
            "no {why} trace of {source} into {callee}:\n{rows}"
        );
    }
    // The chain's third hop (`finish` → `record`) is past the depth bound, and says so.
    let stop = text(
        &ctx,
        "SELECT i.stop_reason FROM analysis_invocations i \
         JOIN declarations s ON s.node_id = i.subject_node_id \
         WHERE s.qualified_name = 'pkg.controls.configure' AND i.method = 1",
    )
    .await;
    assert!(stop.contains("| 0 "), "{stop}");
}

/// Communities (DESIGN §9.4; ADR-0011) on `analysis_shapes`: every pre-registered resolution
/// runs every seed, each recorded with its seed, iterations and quality history; the consensus
/// records its stability per resolution and its choice; each reported community lists its public
/// APIs, its agreement as score, and the sites behind its strongest pairs.
#[tokio::test(flavor = "multi_thread")]
async fn communities_are_stable_and_projected_onto_public_apis() {
    let (ctx, _dir) = analyzed("one", false).await;
    // The increment-2 review's F4: `Alpha` inherits `Server`'s methods and sorts first, but a
    // method is named through the class that declares it.
    let named = text(
        &ctx,
        "SELECT count(*) AS n FROM finding_members WHERE starts_with(label, 'pkg.Alpha.')",
    )
    .await;
    assert!(named.contains("| 0 |"), "{named}");
    let runs = text(
        &ctx,
        "SELECT count(*) AS runs, count(DISTINCT seed) AS seeds, \
                count(DISTINCT parameters) AS parameter_sets, \
                sum(CASE WHEN converged THEN 1 ELSE 0 END) AS converged, \
                min(cardinality(quality_history)) AS least_history \
         FROM analysis_invocations WHERE method = 3",
    )
    .await;
    insta::assert_snapshot!("community_runs", runs);
    insta::assert_snapshot!(
        "community_consensus",
        text(
            &ctx,
            "SELECT completion, candidate_set_size, arcs_examined, diagnostics \
             FROM analysis_invocations WHERE method = 4",
        )
        .await
    );
    insta::assert_snapshot!(
        "communities",
        text(
            &ctx,
            "SELECT s.label AS subject, f.score, f.evidence_status, f.witnesses_omitted, \
                    m.ordinal, m.role, m.label \
             FROM findings f JOIN finding_members m ON m.finding_id = f.finding_id \
             JOIN finding_members s ON s.finding_id = f.finding_id AND s.role = 9 \
               AND s.node_id = f.subject_node_id \
             WHERE f.finding_kind = 11 ORDER BY subject, m.ordinal",
        )
        .await
    );
}

/// Centrality (DESIGN §9.5) in the `+pagerank` variant: one PageRank invocation over the usage
/// projection, converged, with its iterations, residual and diagnostics; each public API's rank is
/// a `centrality` finding. The default runs no PageRank (the increment-2 review's U1).
#[tokio::test(flavor = "multi_thread")]
async fn public_apis_are_ranked_over_the_usage_projection() {
    let (default, _dd) = analyzed("one", false).await;
    assert!(
        text(
            &default,
            "SELECT count(*) AS n FROM analysis_invocations WHERE method = 5"
        )
        .await
        .contains("| 0 |")
    );
    let (result, dir) = compile_variant(
        "pagerank",
        CONFIG,
        "linux",
        Techniques::parse("+communities,+fca,+knn,+pagerank").unwrap(),
    )
    .await;
    result.unwrap();
    let (_, ctx) = published(&dir.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    insta::assert_snapshot!(
        "pagerank",
        text(
            &ctx,
            "SELECT iterations, converged, residual < 1e-10 AS below_tolerance, completion, \
                    candidate_set_size, arcs_examined, diagnostics \
             FROM analysis_invocations WHERE method = 5",
        )
        .await
    );
    insta::assert_snapshot!(
        "centrality_top",
        text(
            &ctx,
            &format!(
                "SELECT {LABEL} AS api, round(f.score, 6) AS rank, f.evidence_status \
                 FROM findings f {} WHERE f.finding_kind = 12 \
                 ORDER BY f.score DESC, api LIMIT 8",
                labelled("f.subject_node_id")
            )
        )
        .await
    );
}

/// Formal concept analysis (DESIGN §9.6): one invocation per seed scope (the exported class or
/// namespace its access path names), its concepts and implications as findings, and the seed's
/// applicable case and the implications it meets as assertions.
#[tokio::test(flavor = "multi_thread")]
async fn concepts_come_from_each_seeds_structural_scope() {
    let (ctx, _dir) = analyzed("one", false).await;
    // The increment-2 review's F2: an undetermined return type is no attribute, and a class raised
    // as a class or as an instance is one.
    let labels = text(
        &ctx,
        "SELECT DISTINCT label FROM finding_members WHERE role IN (12, 13, 14) ORDER BY label",
    )
    .await;
    assert!(labels.contains("raises KeyError"), "{labels}");
    assert!(
        !labels.contains("Unknown") && !labels.contains("type[KeyError]"),
        "{labels}"
    );
    insta::assert_snapshot!(
        "fca_invocations",
        text(
            &ctx,
            "SELECT parameters, completion, stop_reason, diagnostics \
             FROM analysis_invocations WHERE method = 6 ORDER BY parameters",
        )
        .await
    );
    insta::assert_snapshot!(
        "fca_concepts",
        text(
            &ctx,
            "SELECT f.finding_kind, CAST(f.score AS BIGINT) AS score, \
                    (SELECT string_agg(m.label, ', ' ORDER BY m.ordinal) FROM finding_members m \
                     WHERE m.finding_id = f.finding_id AND m.role IN (11, 13)) AS objects_or_premise, \
                    (SELECT string_agg(m.label, ', ' ORDER BY m.ordinal) FROM finding_members m \
                     WHERE m.finding_id = f.finding_id AND m.role IN (12, 14)) AS intent_or_conclusion \
             FROM findings f WHERE f.finding_kind IN (13, 14) \
             ORDER BY f.finding_kind, objects_or_premise, intent_or_conclusion",
        )
        .await
    );
    insta::assert_snapshot!(
        "fca_assertions",
        text(
            &ctx,
            "SELECT b.title, a.assertion_kind, a.evidence_status, a.text FROM briefs b \
             JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
             JOIN assertions a ON a.assertion_id = ba.assertion_id \
             WHERE a.assertion_kind IN (12, 13) ORDER BY b.title, ba.ordinal",
        )
        .await
    );
}

/// Seed selection (the increment-2 review's U1): with room in the brief budget but no official usage
/// code, nothing is eligible, so nothing is selected, and the invocation says so. (`docs_shapes`
/// selects what its usage code calls: `selection_takes_what_usage_calls_within_the_budget`.)
#[tokio::test(flavor = "multi_thread")]
async fn selection_needs_official_usage() {
    let (result, dir) = compile_config(
        "select",
        &CONFIG.replace("budget = 6", "budget = 8"),
        "linux",
    )
    .await;
    result.unwrap();
    let (_, ctx) = published(&dir.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let selection = text(
        &ctx,
        "SELECT candidate_set_size, diagnostics FROM analysis_invocations WHERE method = 7",
    )
    .await;
    assert!(
        selection.contains("\"eligible\":0,\"selected\":[],\"dropped\":[]"),
        "{selection}"
    );
    let briefs = text(&ctx, "SELECT count(*) AS briefs FROM briefs").await;
    assert!(briefs.contains("| 6 "), "{briefs}");
}

/// The increment-2 review's F1 (its probes 3 and 4): an FCA scope is a node. Two paths naming
/// `Catalog` are one scope, and `Widgets`, which inherits `Catalog`'s methods, is another; a seed's
/// shared signature and implications come from its own scope only, and name only its APIs.
#[tokio::test(flavor = "multi_thread")]
async fn fca_scopes_are_nodes_and_state_only_their_own_apis() {
    let (result, dir) = compile_config(
        "scopes",
        &CONFIG
            .replace(
                "\"pkg.Catalog.add_tool\"]",
                "\"pkg.Catalog.add_tool\", \"pkg.registry.Catalog.add_prompt\", \
                 \"pkg.Widgets.remove\", \"pkg.Widgets.add_gadget\"]",
            )
            .replace("budget = 6", "budget = 9"),
        "linux",
    )
    .await;
    result.unwrap();
    let (_, ctx) = published(&dir.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let scopes = text(
        &ctx,
        "SELECT parameters FROM analysis_invocations WHERE method = 6 ORDER BY parameters",
    )
    .await;
    assert!(scopes.contains("\"scope\":\"pkg.Catalog\""), "{scopes}");
    assert!(scopes.contains("\"scope\":\"pkg.Widgets\""), "{scopes}");
    assert!(!scopes.contains("pkg.registry.Catalog"), "{scopes}");
    let stated = text(
        &ctx,
        "SELECT b.title, a.assertion_kind, a.text FROM briefs b \
         JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE a.assertion_kind IN (13, 15) ORDER BY b.title, ba.ordinal",
    )
    .await;
    insta::assert_snapshot!("fca_scopes", stated);
    for line in stated
        .lines()
        .filter(|l| l.contains("pkg.Catalog.add_tool |"))
    {
        assert!(
            !line.contains("add_gadget") && !line.contains("add_widget"),
            "{line}"
        );
        assert!(!line.contains("pkg.Widgets"), "{line}");
    }
    // The same method under `Widgets` does state `Widgets`' own APIs.
    assert!(stated.contains("add_gadget"), "{stated}");
}

/// Slice 3.2's variants (§9.6, §9.4, §9.8): `+rca` adds relational attributes to the same FCA and
/// `+type-layer,+mention-layer` add community layers, each recorded in its invocation's parameters
/// and diagnostics. A variant's snapshot is another content (its label joins the config digest),
/// while a finding the variant does not touch keeps its id, so an ablation diff is a join.
#[tokio::test(flavor = "multi_thread")]
async fn variants_add_relational_attributes_and_layers() {
    let (default, dd) = compile_config("default", CONFIG, "linux").await;
    let (variant, dv) = compile_variant(
        "variant",
        CONFIG,
        "linux",
        Techniques::parse("+communities,+fca,+knn,+rca,+type-layer,+mention-layer").unwrap(),
    )
    .await;
    let (default, variant) = (default.unwrap(), variant.unwrap());
    assert_ne!(default.content_digest, variant.content_digest);
    let (_, a) = published(&dd.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    let (_, b) = published(&dv.path().join("store"), Id([7; 16]))
        .await
        .unwrap()
        .unwrap();
    // Pass A is untouched: identical finding ids.
    let pass_a = "SELECT finding_id FROM findings WHERE finding_kind <= 5 ORDER BY 1";
    assert_eq!(text(&a, pass_a).await, text(&b, pass_a).await);
    let fca = text(
        &b,
        "SELECT parameters, diagnostics FROM analysis_invocations WHERE method = 6 \
         ORDER BY parameters",
    )
    .await;
    assert!(
        fca.contains("\"rca\":\"rca: one existential-scaling step"),
        "{fca}"
    );
    let relational = text(
        &b,
        "SELECT DISTINCT label FROM finding_members WHERE role IN (12, 13, 14) \
           AND (starts_with(label, 'calls ') OR starts_with(label, 'hands off to ') \
                OR starts_with(label, 'takes from ')) ORDER BY label",
    )
    .await;
    insta::assert_snapshot!("rca_attributes", relational);
    let consensus = text(
        &b,
        "SELECT diagnostics FROM analysis_invocations WHERE method = 4",
    )
    .await;
    assert!(
        consensus.contains("\"extra_layers\":[[\"type\","),
        "{consensus}"
    );
    assert!(consensus.contains("[\"mention\",0,"), "{consensus}");
    assert!(
        !text(
            &a,
            "SELECT diagnostics FROM analysis_invocations WHERE method = 4"
        )
        .await
        .contains("extra_layers")
    );
    assert!(cpg_core::validate::validate(&b).await.unwrap().is_empty());
}

/// `lctx diff` (§9.8, slice 3.3): the kernels variant with and without FCA, compiled into one
/// store, differ exactly in FCA's findings and what the briefs state from them; Pass A's findings
/// are common to both, and a snapshot compared with itself changes nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_diff_is_a_join_on_content_ids() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (Id([7; 16]), Id([8; 16]));
    compile_at(dir.path(), "a", a, CONFIG, "linux", kernels())
        .await
        .unwrap();
    compile_at(
        dir.path(),
        "b",
        b,
        CONFIG,
        "linux",
        Techniques::parse("+communities,+knn").unwrap(),
    )
    .await
    .unwrap();
    let store = dir.path().join("store");
    let diff = cpg_core::diff::diff(&store, a, b).await.unwrap();
    assert!(diff.changes_published_output());
    let findings = &diff.tables[0];
    assert!(findings.only_from > 0 && findings.only_to == 0 && findings.common > 0);
    // Only FCA's statements go: implications and shared signatures, by codebook name (the ADR-0020
    // review's O3).
    for change in &diff.changed {
        assert!(change.added.is_empty(), "{change:?}");
        assert!(
            change
                .removed
                .iter()
                .all(|(kind, _, _)| kind == "implication" || kind == "shared_signature"),
            "{change:?}"
        );
    }
    // FCA's text never enters a brief document (U2, D14), so no document is one-sided, however
    // the briefs' ids move (the ADR-0020 review's F7: documents keyed by seed, chunk and text).
    let documents = diff
        .tables
        .iter()
        .find(|t| t.table == "brief_documents")
        .unwrap();
    assert_eq!(
        (documents.only_from, documents.only_to),
        (0, 0),
        "{documents:?}"
    );
    insta::assert_snapshot!("diff_minus_fca", diff.render());
    let same = cpg_core::diff::diff(&store, a, a).await.unwrap();
    assert!(!same.changes_published_output());
    assert!(
        same.tables
            .iter()
            .all(|t| t.only_from == 0 && t.only_to == 0)
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
                 FROM analysis_invocations i {} WHERE i.method <= 2 \
                 ORDER BY seed, i.method",
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
    // Increment-1 deep review F5: a definition is no call site; a boundary reached over an
    // override-open arc is one the code may call; a nested definition claims nothing more.
    let configure = text(
        &ctx,
        "SELECT a.text FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE b.title IN ('pkg.configure', 'pkg.Server.route') ORDER BY b.title, ba.ordinal",
    )
    .await;
    for expected in [
        "`pkg.configure` already calls `pkg.controls.configure.audit` (1 call site).",
        "Callables `pkg.configure` reaches may call into code outside the analyzed release, \
         which is not analyzed further: `builtins.list.append`.",
        "`pkg.Server.route` defines the nested callable `pkg.server.Server.route.decorator`.",
    ] {
        assert!(configure.contains(expected), "{expected}\n{configure}");
    }
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
        (
            4,
            4,
            "7500a12012d91a6b563b21c4fef99943592326800bd6fd5a995a8b0e6ce6dec6",
        ),
        (
            5,
            5,
            "b9370ca547aa34bdc91d68b9f24a8ad05f1b911d8dc478ee9495777c7fc95d7f",
        ),
        (
            5,
            6,
            "9a7d3a54b54907211f83642e6e76fd006ffbb714e873d810fe0651a1087f8c85",
        ),
        (
            6,
            7,
            "6a58ea464f733da304e251dfd905a0e54f1399577e8158cf2d87673574a4a4b1",
        ),
        (
            7,
            8,
            "a1059c90f44d1e3d9fcac8d70043dfbea917a6facf056fb890e833cad70d610c",
        ),
        (
            8,
            8,
            "a1059c90f44d1e3d9fcac8d70043dfbea917a6facf056fb890e833cad70d610c",
        ),
        (
            9,
            8,
            "a1059c90f44d1e3d9fcac8d70043dfbea917a6facf056fb890e833cad70d610c",
        ),
        (
            10,
            9,
            "cd3ea8534cc3db47dc97be8727bbb4510fc3e4fa9a235cfe9aec2ff7b0c2c1af",
        ),
        (
            11,
            9,
            "cd3ea8534cc3db47dc97be8727bbb4510fc3e4fa9a235cfe9aec2ff7b0c2c1af",
        ),
        (
            12,
            10,
            "982bc3a8b44d4c4fe033b665edaf19081a44e1926b43f19287e5583e12974b1f",
        ),
        (
            13,
            11,
            "44252165621a2305caebdef9f2bac256f6e0fd6c57397a3e629e1e7080ccaa1f",
        ),
        (
            14,
            12,
            "02cbec86553c6e4fb4e4d098a21d0dc9539f271090fd9c96825e0d823b45bf9b",
        ),
        (
            15,
            13,
            "bc3a98ff16fbcb0a3a355db8e827e7859dea812b3a2d84abc6ce4bbfd2a41bb8",
        ),
        (
            15,
            14,
            "80eea5dfd50a644872fe345833563c7d4658604d2a7544f3629c17c8ed72d20a",
        ),
        (
            15,
            15,
            "12c4893c49d07d6758fce5f2753cf0ba87d3af7f1f005f1a2e15a443712186a6",
        ),
        (
            16,
            15,
            "12c4893c49d07d6758fce5f2753cf0ba87d3af7f1f005f1a2e15a443712186a6",
        ),
        (
            16,
            16,
            "bfe1425ba1c615cb7e668e433ed3b9e484853eb3af71d0b9d7995e159ee35255",
        ),
        (
            16,
            17,
            "f5ffbc46253fa37731e7d752ea3091591b4576f9c3e2bfc0bc016af34ba02ce2",
        ),
        (
            16,
            18,
            "2e8c9be4c961e42a4a860b3b08fc81b89d41fd0787ed9ba6d1eb86522bd40259",
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
        "public_paths",
        "analysis_invocations",
        "findings",
        "finding_members",
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
        // Slice 2.6's policy case: statistical output never states a control. A control stated
        // `statistically_derived` breaks the kind policy...
        (
            "semantic:assertion-policy",
            "assertions",
            "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, assertion_kind, \
                    subject_node_id, applicable_case, \
                    CASE WHEN assertion_kind = 6 THEN CAST(2 AS SMALLINT) ELSE evidence_status END \
                      AS evidence_status, \
                    text, conditions, limitations, template_version \
             FROM assertions_published",
        ),
        // ...and a control citing a community finding, left `structurally_observed`, is not the
        // status its supports derive.
        (
            "semantic:assertion-status-derived",
            "assertion_support",
            "SELECT * FROM assertion_support_published UNION ALL \
             SELECT a.snapshot_id, a.assertion_id, s.role, 999 AS ordinal, f.finding_id, \
                    CAST(NULL AS BYTEA) AS evidence_id \
             FROM assertions_published a \
             JOIN assertion_support_published s ON s.assertion_id = a.assertion_id \
               AND s.ordinal = 0 \
             CROSS JOIN (SELECT finding_id FROM findings WHERE finding_kind = 11 \
                         ORDER BY finding_id LIMIT 1) f \
             WHERE a.assertion_kind = 6",
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
        // The holistic assessment's A1: a path its export does not spell, a node with two
        // preferred paths, and an inherited path claimed as the node's own.
        (
            "semantic:public-path-exported",
            "public_paths",
            "SELECT snapshot_id, node_id, access_path || '_elsewhere' AS access_path, \
                    export_node_id, kind, own, preferred FROM public_paths_published",
        ),
        (
            "semantic:public-path-preferred",
            "public_paths",
            "SELECT snapshot_id, node_id, access_path, export_node_id, kind, own, \
                    true AS preferred FROM public_paths_published",
        ),
        (
            "semantic:public-path-own",
            "public_paths",
            "SELECT snapshot_id, node_id, access_path, export_node_id, kind, NOT own AS own, \
                    preferred FROM public_paths_published",
        ),
        // A member path that is no public path of the seed; one whose `own` flag is wrong; and
        // a seed's public path missing from its brief (the holistic assessment's A1).
        (
            "semantic:brief-member-public",
            "brief_members",
            "SELECT snapshot_id, brief_id, 'elsewhere.' || access_path AS access_path, \
                    export_node_id, declaration_node_id, own \
             FROM brief_members_published",
        ),
        (
            "semantic:brief-member-public",
            "brief_members",
            "SELECT snapshot_id, brief_id, access_path, export_node_id, declaration_node_id, \
                    NOT own AS own FROM brief_members_published",
        ),
        (
            "semantic:brief-member-public",
            "brief_members",
            "SELECT * FROM brief_members_published WHERE own",
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
        // A documented parameter whose span is not its own description.
        (
            "semantic:documented-parameter-cites-its-doc",
            "evidence",
            "SELECT snapshot_id, evidence_id, evidence_kind, cited_fact_id, node_id, \
                    module_node_id, start_byte + 1 AS start_byte, end_byte + 1 AS end_byte, text \
             FROM evidence_published",
        ),
        // Review F3: each documented parameter cites another parameter's description.
        (
            "semantic:documented-parameter-cites-its-doc",
            "evidence",
            "WITH others AS ( \
               SELECT a.function_node_id, a.name AS own, min(b.name) AS other \
               FROM parameter_docs a JOIN parameter_docs b \
                 ON b.function_node_id = a.function_node_id AND b.name <> a.name \
               GROUP BY a.function_node_id, a.name) \
             SELECT e.snapshot_id, e.evidence_id, e.evidence_kind, e.cited_fact_id, e.node_id, \
                    e.module_node_id, COALESCE(d.start_byte, e.start_byte) AS start_byte, \
                    COALESCE(d.end_byte, e.end_byte) AS end_byte, e.text \
             FROM evidence_published e \
             LEFT JOIN parameter_syntax ps ON ps.node_id = e.node_id \
             LEFT JOIN others o ON o.function_node_id = ps.function_node_id AND o.own = ps.name \
             LEFT JOIN parameter_docs d ON d.function_node_id = o.function_node_id \
               AND d.name = o.other",
        ),
        // The increment-2 review's deferred row: every finding credited to one Pass A invocation.
        (
            "semantic:finding-kind-by-method",
            "findings",
            "SELECT snapshot_id, finding_id, \
                    (SELECT min(invocation_id) FROM analysis_invocations WHERE method = 0) \
                      AS invocation_id, \
                    finding_kind, subject_node_id, related_node_id, evidence_status, depth, \
                    stop_reason, witnesses_omitted, score, condition_node_id \
             FROM findings_published",
        ),
        // The increment-2 review's F2: a concept about an undetermined type.
        (
            "semantic:concept-attribute-known",
            "finding_members",
            "SELECT * EXCLUDE (label), \
                    CASE WHEN role = 12 THEN 'returns Unknown' ELSE label END AS label \
             FROM finding_members_published",
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
