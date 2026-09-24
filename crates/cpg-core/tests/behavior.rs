//! The behavior model on `behavior_shapes` (ADR-0021, ADR-0022; DESIGN §3.9; increment 3's deep
//! review F1–F3, F6): one verdict policy per arc, the scan's status over its region, persisted
//! steps, and facets whose completeness is data.

use std::path::Path;

use arrow_array::RecordBatch;
use cpg_core::analyze::{Analysis, Techniques};
use cpg_core::attempt::compile_analyzed;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::id::Id;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;
use lctx_analytics::config::AnalyticsConfig;

const CONFIG: &str = r#"
version = 1
[subsystem]
module_prefixes = ["bpkg"]
public_roots = ["bpkg"]
[seeds]
primary = ["bpkg.top"]
distractors = ["bpkg.forward_open"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 2
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

/// `behavior_shapes` extracted and compiled with the default techniques; the session reads the
/// published snapshot.
pub async fn compiled() -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/behavior_shapes");
    copy(&fixture.join("release"), &dir.path().join("release"));
    std::fs::create_dir_all(dir.path().join("venv/site-packages")).unwrap();
    let s = Id([9; 16]);
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(dir.path().join("release")).unwrap(),
            "behavior_shapes",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(dir.path().join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(dir.path().join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks::default(),
    })
    .unwrap();
    let analysis = Analysis {
        config: AnalyticsConfig::parse(CONFIG).unwrap(),
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, s, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    (ctx, dir)
}

async fn table(ctx: &SessionContext, statement: &str) -> String {
    let batches: Vec<RecordBatch> = sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    pretty_format_batches(&batches).unwrap().to_string()
}

/// An operation's status and first boundary, by preferred path.
async fn status(ctx: &SessionContext, path: &str) -> String {
    table(
        ctx,
        &format!(
            "SELECT behavior_status, boundary_reason FROM operations WHERE access_path = '{path}'"
        ),
    )
    .await
}

#[tokio::test]
async fn one_arc_has_one_verdict_and_the_region_decides_the_status() {
    let (ctx, _dir) = compiled().await;
    // F1: a forward through `self.render(...)`, which a subclass overrides, is unknown
    // (`override_dispatch`, code 17), like the delegation over the same arc.
    let forwards = table(
        &ctx,
        "SELECT b.kind, b.target_name, b.verdict, b.boundary_reason FROM behaviors b \
         JOIN operations o ON o.node_id = b.operation_node_id \
         WHERE o.access_path = 'bpkg.Base.handle' AND b.kind IN (0, 4) ORDER BY b.kind",
    )
    .await;
    assert!(
        forwards.contains("| 0    | value       | 3       | 17"),
        "{forwards}"
    );
    assert!(
        !forwards.contains("| 0       |"),
        "no established row: {forwards}"
    );
    // F2: the status is over the region.
    let handle = status(&ctx, "bpkg.Base.handle").await;
    assert!(handle.contains("| 3               | 17"), "{handle}");
    let forward_open = status(&ctx, "bpkg.forward_open").await;
    assert!(
        forward_open.contains("| 3               | 1 "),
        "a forwarded value reaches an open site in a callee: {forward_open}"
    );
    let own_open = status(&ctx, "bpkg.own_open").await;
    assert!(own_open.contains("| 3               | 1 "), "{own_open}");
    let top = status(&ctx, "bpkg.top").await;
    assert!(
        top.contains("| 3               | 6 "),
        "a formal read at the frontier is a depth cut: {top}"
    );
}

#[tokio::test]
async fn every_forward_carries_its_path_hop_by_hop() {
    let (ctx, _dir) = compiled().await;
    // F6: a row at depth d has exactly d steps, numbered from 0 (a call outside the release and
    // a stored field's later read have no arc to step through).
    let bad = table(
        &ctx,
        "SELECT count(*) AS bad FROM behaviors b \
         LEFT JOIN (SELECT behavior_id, count(*) AS n, max(step) AS last FROM behavior_steps \
                    GROUP BY behavior_id) s ON s.behavior_id = b.behavior_id \
         WHERE b.kind IN (0, 1, 4) AND b.callee_text IS NULL \
           AND (b.value IS NULL OR b.value NOT LIKE 'via %') \
           AND (COALESCE(s.n, 0) <> b.depth OR s.last <> b.depth - 1)",
    )
    .await;
    assert!(
        bad.contains("| 0   |"),
        "every row's steps match its depth:\n{bad}"
    );
}

#[tokio::test]
async fn class_facets_and_their_completeness_are_data() {
    let (ctx, _dir) = compiled().await;
    // F3: a class's decorators and its public constructor's parameters are facets.
    let facets = table(
        &ctx,
        "SELECT o.access_path, f.facet, f.value, f.verdict FROM operation_facets f \
         JOIN operations o ON o.node_id = f.node_id \
         WHERE o.kind = 2 AND f.facet IN (0, 4) ORDER BY 1, 2, 3",
    )
    .await;
    assert!(
        facets.contains("| bpkg.Point      | 4     | dataclass"),
        "{facets}"
    );
    assert!(
        facets.contains("| bpkg.Plain      | 0     | name"),
        "{facets}"
    );
    assert!(
        facets.contains("| bpkg.Configured | 0     | name"),
        "an inherited public __init__ counts: {facets}"
    );
    // A class with no public constructor says its parameters are not analyzed (4), never
    // complete by silence.
    let point = table(
        &ctx,
        "SELECT s.facet, s.verdict FROM operation_facet_status s \
         JOIN operations o ON o.node_id = s.node_id \
         WHERE o.access_path = 'bpkg.Point' AND s.facet IN (0, 1, 4) ORDER BY 1",
    )
    .await;
    assert!(point.contains("| 0     | 4       |"), "{point}");
    assert!(point.contains("| 4     | 0       |"), "{point}");
}

/// One operation's behaviors as `kind | parameter | target | callee text | value | verdict |
/// reason | condition | phase` lines.
async fn behaviors_of(ctx: &SessionContext, path: &str) -> String {
    table(
        ctx,
        &format!(
            "SELECT b.kind, b.parameter_name, b.target_name, b.callee_text, b.value, b.verdict, \
                    b.boundary_reason, b.condition, b.phase \
             FROM behaviors b JOIN operations o ON o.node_id = b.operation_node_id \
             WHERE o.access_path = '{path}' ORDER BY 1, 2, 3, 5"
        ),
    )
    .await
}

#[tokio::test]
async fn the_flow_ir_follows_fallbacks_settings_and_fields() {
    let (ctx, _dir) = compiled().await;
    // A rebound fallback is followed: `host` reaches `_bind` unchanged when it is not None and the
    // transport is http or sse (Stage 1 declined it as "rebound").
    let serve = behaviors_of(&ctx, "bpkg.serve").await;
    assert!(
        serve.contains(
            "| 0    | host           | host                      |             |                 \
             | 1       |                 | !is_none(host) & member_of(transport,{\"http\",\"sse\"}) |"
        ) || serve.contains("!is_none(host) & member_of(transport,{\"http\",\"sse\"})"),
        "{serve}"
    );
    // The setting it falls back to is read per call (phase 3), only when `host` is None.
    assert!(
        serve.contains("| bpkg.config.settings.host |") && serve.contains("| is_none(host)"),
        "{serve}"
    );
    // One setting, two spellings, one resolved key.
    let debug = behaviors_of(&ctx, "bpkg.debug_enabled").await;
    assert!(debug.contains("bpkg.config.settings.debug"), "{debug}");
    let reads = table(
        &ctx,
        "SELECT field, phase FROM ambient_reads WHERE global = 'bpkg.config.settings' ORDER BY 1",
    )
    .await;
    assert!(
        reads.contains("| log_level | 0     |"),
        "an import-time read: {reads}"
    );
    // A constructor parameter stored to a field reaches what another method does with it.
    let session = behaviors_of(&ctx, "bpkg.Session.__init__").await;
    assert!(
        session.contains("via self.name in bpkg.service.Session.call, into logger.info"),
        "{session}"
    );
    assert!(
        session.contains("via self._prior in bpkg.service.Session.adopt")
            && session.contains("equals(self._mode,\"pinned\")"),
        "{session}"
    );
    // A log-only parameter derives into the logging call, and nothing else.
    let start = behaviors_of(&ctx, "bpkg.start").await;
    assert!(
        start.contains("| 7    | stateless") && start.contains("logger.info"),
        "{start}"
    );
    // `**kwargs` raises when truthy.
    let make = behaviors_of(&ctx, "bpkg.make").await;
    assert!(
        make.contains("| 2    | kwargs") && make.contains("truthy(kwargs)"),
        "{make}"
    );
}

#[tokio::test]
async fn a_negative_claim_is_refuted_only_where_its_premise_holds() {
    let (ctx, _dir) = compiled().await;
    // `ignore(a, b)` never reads `b`: refuted under the model (2), on a premise that holds.
    let ignore = table(
        &ctx,
        "SELECT b.parameter_name, b.verdict, p.holds FROM behaviors b \
         JOIN operations o ON o.node_id = b.operation_node_id \
         JOIN negative_premises p ON p.place_key = b.premise_key \
         WHERE o.access_path = 'bpkg.ignore' AND b.kind = 10",
    )
    .await;
    assert!(
        ignore.contains("| b              | 2       | true  |"),
        "{ignore}"
    );
    let premises = table(
        &ctx,
        "SELECT place_key, holds, boundary_reason FROM negative_premises \
         WHERE place_key IN ('Field[bpkg.config.Plain.never_read]', \
                             'Global[bpkg.config.settings].unused_option') ORDER BY 1",
    )
    .await;
    // No load named `never_read` anywhere and no dynamic access reaches `Plain`: the premise
    // holds.
    let line = |key: &str, rest: &str| {
        premises
            .lines()
            .any(|l| l.contains(key) && l.contains(rest))
    };
    assert!(
        line("Field[bpkg.config.Plain.never_read]", "| true"),
        "{premises}"
    );
    // `get_setting` reads by name through `settings = self` (the aliasing shape): unknown,
    // `dynamic_access` (16).
    assert!(
        line("Global[bpkg.config.settings].unused_option", "| false | 16"),
        "{premises}"
    );
}
