//! C1 (ADR-0014; guidelines §12): the node and edge catalogs on known-answer graph shapes, their
//! determinism, and a graph-readiness reader that builds a petgraph projection from them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, Int16Array, RecordBatch};
use cpg_core::attempt::compile;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::codebook::{Codebook, EdgeKind, NodeKind};
use cpg_schema::id::Id;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;

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

/// The `graph_shapes` release with `depmod` installed in site-packages, extracted and compiled
/// under `snapshot`; the session reads the published snapshot.
async fn shapes(sub: &str, reverse: bool) -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/graph_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    let s = Id([5; 16]);
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(),
            "graph_shapes",
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
    let store = dir.path().join("store");
    compile(&store, s, &out.tables).await.unwrap();
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

async fn count(ctx: &SessionContext, statement: &str) -> i64 {
    let b = batches(ctx, statement).await;
    b[0].column(0)
        .as_any()
        .downcast_ref::<arrow_array::Int64Array>()
        .unwrap()
        .value(0)
}

/// Rows of (id, id) as hex pairs.
async fn id_pairs(ctx: &SessionContext, statement: &str) -> Vec<(String, String)> {
    let hex = |a: &FixedSizeBinaryArray, i: usize| Id(a.value(i).try_into().unwrap()).hex();
    let mut out = Vec::new();
    for b in batches(ctx, statement).await {
        let cast = |c: usize| {
            let binary = arrow_cast::cast(b.column(c), &arrow_schema::DataType::Binary).unwrap();
            arrow_cast::cast(&binary, &arrow_schema::DataType::FixedSizeBinary(16)).unwrap()
        };
        let (x, y) = (cast(0), cast(1));
        let (x, y) = (
            x.as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap(),
            y.as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap(),
        );
        for i in 0..b.num_rows() {
            out.push((hex(x, i), hex(y, i)));
        }
    }
    out
}

/// Joins that name any node: its qualified name, path, access path or external symbol.
fn labelled(alias: &str, column: &str) -> String {
    format!(
        "LEFT JOIN declarations d{alias} ON d{alias}.node_id = {column} \
         LEFT JOIN context_definitions x{alias} ON x{alias}.symbol_node_id = {column} \
         LEFT JOIN synthetic_callables sc{alias} ON sc{alias}.node_id = {column} \
         LEFT JOIN source_files s{alias} ON s{alias}.module_node_id = {column} \
         LEFT JOIN context_modules m{alias} ON m{alias}.module_node_id = {column} \
         LEFT JOIN (SELECT DISTINCT export_node_id, access_path FROM exports) ex{alias} \
           ON ex{alias}.export_node_id = {column}"
    )
}

/// The label the `labelled` joins give.
fn label(a: &str) -> String {
    format!(
        "COALESCE(d{a}.qualified_name, x{a}.module_name || ':' || x{a}.qualified_name, \
                  'synthetic:' || sc{a}.function_key, s{a}.path, m{a}.module_name, \
                  ex{a}.access_path)"
    )
}

#[tokio::test]
async fn the_catalogs_hold_every_graph_shape() {
    let (ctx, _dir) = shapes("one", false).await;
    // The edges between named nodes (declarations, exports, modules, external and synthetic
    // symbols); call sites, arguments and parameters appear through the call graph below.
    let named = format!(
        "SELECT e.edge_kind, {src} AS src, {dst} AS dst, e.ordinal FROM edges e {js} {jd} \
         WHERE e.src_kind NOT IN ({cs}, {arg}, {par}) AND e.dst_kind NOT IN ({cs}, {arg}, {par}) \
         ORDER BY e.edge_kind, src, dst, e.ordinal",
        src = label("s"),
        dst = label("t"),
        js = labelled("s", "e.src_node_id"),
        jd = labelled("t", "e.dst_node_id"),
        cs = NodeKind::CallSite.code(),
        arg = NodeKind::Argument.code(),
        par = NodeKind::Parameter.code(),
    );
    // The call graph as a projection would read it: owner → target, per call site.
    let calls = format!(
        "SELECT {owner} AS caller, c.start_byte AS site, {dst} AS callee, e.dst_kind \
         FROM edges e JOIN call_syntax c ON c.node_id = e.src_node_id {jo} {jd} \
         WHERE e.edge_kind = {k} ORDER BY caller, site, callee",
        owner = label("o"),
        dst = label("t"),
        jo = labelled("o", "COALESCE(c.owner_node_id, c.module_node_id)"),
        jd = labelled("t", "e.dst_node_id"),
        k = EdgeKind::CallTarget.code(),
    );
    let out = format!(
        "## nodes\n{}\n## named edges\n{}\n## call graph\n{}\n",
        text(
            &ctx,
            "SELECT node_kind, count(*) AS n FROM nodes GROUP BY node_kind ORDER BY node_kind"
        )
        .await,
        text(&ctx, &named).await,
        text(&ctx, &calls).await
    );
    insta::assert_snapshot!(out);

    // The isolate is a node and has no incoming call edge.
    let lonely = "SELECT node_id FROM declarations WHERE qualified_name = 'shapes.core.lonely'";
    assert_eq!(
        count(
            &ctx,
            &format!("SELECT count(*) FROM nodes WHERE node_id IN ({lonely})")
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM edges WHERE edge_kind = {} AND dst_node_id IN ({lonely})",
                EdgeKind::CallTarget.code()
            )
        )
        .await,
        0
    );
    // Two call sites to one callee: two edges, two ids, two sources.
    let parallel = id_pairs(
        &ctx,
        &format!(
            "SELECT e.edge_id, e.src_node_id FROM edges e \
             JOIN call_syntax c ON c.node_id = e.src_node_id \
             JOIN declarations o ON o.node_id = c.owner_node_id \
             JOIN declarations t ON t.node_id = e.dst_node_id \
             WHERE e.edge_kind = {} AND o.qualified_name = 'shapes.core.twice' \
               AND t.qualified_name = 'shapes.core.callee'",
            EdgeKind::CallTarget.code()
        ),
    )
    .await;
    assert_eq!(parallel.len(), 2);
    assert_ne!(parallel[0].0, parallel[1].0, "distinct edge ids");
    assert_ne!(parallel[0].1, parallel[1].1, "distinct call sites");
    // The computed call has no edge, and its resolution says why.
    let dispatch = text(
        &ctx,
        &format!(
            "SELECT r.status, r.has_unresolved_remainder, r.target_count, \
                    count(e.edge_id) AS edges \
             FROM resolutions r JOIN call_syntax c ON c.node_id = r.call_site_node_id \
             JOIN declarations o ON o.node_id = c.owner_node_id \
             LEFT JOIN edges e ON e.src_node_id = c.node_id AND e.edge_kind = {} \
             WHERE o.qualified_name = 'shapes.core.dispatch' \
             GROUP BY c.start_byte, r.status, r.has_unresolved_remainder, r.target_count \
             ORDER BY c.start_byte",
            EdgeKind::CallTarget.code()
        ),
    )
    .await;
    assert!(
        dispatch.contains("| 2      | true                     | 0            | 0     |"),
        "{dispatch}"
    );
    // `f(g(x))`: the argument is not the inner call site.
    let arg_vs_call = count(
        &ctx,
        &format!(
            "SELECT count(*) FROM edges a JOIN edges c ON c.dst_node_id = a.dst_node_id \
             WHERE a.edge_kind = {} AND c.edge_kind = {}",
            EdgeKind::HasArgument.code(),
            EdgeKind::EnclosesCall.code()
        ),
    )
    .await;
    assert_eq!(arg_vs_call, 0, "no id is both an argument and a call site");
    // `map(double, xs)`: the argument may be invoked as `double` (a potential target).
    let higher = count(
        &ctx,
        &format!(
            "SELECT count(*) FROM edges e JOIN declarations t ON t.node_id = e.dst_node_id \
             JOIN arguments a ON a.node_id = e.src_node_id \
             JOIN facts f ON f.fact_id = e.evidence_fact_id \
             WHERE e.edge_kind = {} AND t.qualified_name = 'shapes.core.double' \
               AND f.modality = 2",
            EdgeKind::HigherOrderTarget.code()
        ),
    )
    .await;
    assert_eq!(higher, 1, "one potential higher-order target");
    // Review F2: a `.py`/`.pyi` pair publishing one path to one origin gives two edges, one per
    // access file, with two ids.
    let dual = count(
        &ctx,
        &format!(
            "SELECT count(DISTINCT e.edge_id) FROM edges e \
             JOIN exports x ON x.public_fact_id = e.evidence_fact_id \
             WHERE e.edge_kind = {} AND x.access_path = 'shapes.dual.lonely'",
            EdgeKind::Exports.code()
        ),
    )
    .await;
    assert_eq!(dual, 2, "one export edge per access file");
    // C3: the variable export targets its module-level binding (no longer `variable_origin`).
    let version = text(
        &ctx,
        "SELECT n.node_kind, e.reason FROM exports e JOIN nodes n ON n.node_id = e.target_node_id \
         WHERE e.access_path = 'shapes.VERSION'",
    )
    .await;
    assert!(version.contains("| 12        |"), "a binding: {version}");
}

#[tokio::test]
async fn a_projection_reads_the_catalogs_without_losing_shape_or_lineage() {
    use petgraph::algo::tarjan_scc;
    use petgraph::graph::{DiGraph, NodeIndex};

    let (ctx, _dir) = shapes("one", false).await;
    // Vertex universe: every function node (isolates included), selected apart from the edges.
    let functions = id_pairs(
        &ctx,
        &format!(
            "SELECT n.node_id AS a, n.node_id AS b FROM nodes n WHERE n.node_kind = {} \
             ORDER BY n.node_id",
            NodeKind::Function.code()
        ),
    )
    .await;
    // Arcs: caller → callee per call site, keeping the edge id (parallel sites stay distinct).
    let arcs = id_pairs(
        &ctx,
        &format!(
            "SELECT c.src_node_id, t.dst_node_id FROM edges t \
             JOIN edges c ON c.dst_node_id = t.src_node_id AND c.edge_kind = {} \
             WHERE t.edge_kind = {} AND t.dst_kind = {} ORDER BY t.edge_id",
            EdgeKind::EnclosesCall.code(),
            EdgeKind::CallTarget.code(),
            NodeKind::Function.code()
        ),
    )
    .await;
    let mut g: DiGraph<String, ()> = DiGraph::new();
    let mut index: BTreeMap<String, NodeIndex> = BTreeMap::new();
    for (id, _) in &functions {
        index.insert(id.clone(), g.add_node(id.clone()));
    }
    for (caller, callee) in &arcs {
        if let (Some(a), Some(b)) = (index.get(caller), index.get(callee)) {
            g.add_edge(*a, *b, ());
        }
    }
    let names: BTreeMap<String, String> = {
        let mut m = BTreeMap::new();
        for b in batches(&ctx, "SELECT node_id, qualified_name FROM declarations").await {
            let ids = arrow_cast::cast(
                &arrow_cast::cast(b.column(0), &arrow_schema::DataType::Binary).unwrap(),
                &arrow_schema::DataType::FixedSizeBinary(16),
            )
            .unwrap();
            let ids = ids.as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
            let q = arrow_cast::cast(b.column(1), &arrow_schema::DataType::Utf8).unwrap();
            let q = q
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            for i in 0..b.num_rows() {
                m.insert(
                    Id(ids.value(i).try_into().unwrap()).hex(),
                    q.value(i).to_owned(),
                );
            }
        }
        m
    };
    let named = |ix: NodeIndex| names[&g[ix]].clone();
    // The isolate survives the projection.
    let lonely = g
        .node_indices()
        .find(|&i| named(i) == "shapes.core.lonely")
        .expect("isolate");
    assert_eq!(g.neighbors_undirected(lonely).count(), 0);
    // Parallel call sites stay two arcs.
    let (twice, callee) = (
        g.node_indices()
            .find(|&i| named(i) == "shapes.core.twice")
            .unwrap(),
        g.node_indices()
            .find(|&i| named(i) == "shapes.core.callee")
            .unwrap(),
    );
    assert_eq!(g.edges_connecting(twice, callee).count(), 2);
    // The self-loop and the cross-file cycle are found by SCC over the whole release.
    let fact = g
        .node_indices()
        .find(|&i| named(i) == "shapes.core.fact")
        .unwrap();
    assert!(g.contains_edge(fact, fact));
    let cycles: BTreeSet<BTreeSet<String>> = tarjan_scc(&g)
        .into_iter()
        .filter(|c| c.len() > 1)
        .map(|c| c.into_iter().map(named).collect())
        .collect();
    assert_eq!(
        cycles,
        [["shapes.ping.ping", "shapes.pong.pong"]
            .into_iter()
            .map(str::to_owned)
            .collect()]
        .into_iter()
        .collect()
    );
    // The diamond reconverges on `bottom`.
    let bottom = g
        .node_indices()
        .find(|&i| named(i) == "shapes.core.bottom")
        .unwrap();
    assert_eq!(
        g.neighbors_directed(bottom, petgraph::Direction::Incoming)
            .count(),
        2
    );
    // Lineage: every call edge cites the Pysa row its `call_targets` row cites.
    let unmatched = count(
        &ctx,
        &format!(
            "SELECT count(*) FROM edges e LEFT ANTI JOIN call_targets t \
               ON t.pysa_fact_id = e.evidence_fact_id AND t.target_node_id = e.dst_node_id \
             WHERE e.edge_kind = {}",
            EdgeKind::CallTarget.code()
        ),
    )
    .await;
    assert_eq!(unmatched, 0);
}

#[tokio::test]
async fn the_catalogs_are_the_same_across_runs_order_and_location() {
    let ids = |ctx: SessionContext| async move {
        let mut out = BTreeSet::new();
        for b in batches(
            &ctx,
            "SELECT edge_id, edge_kind FROM edges UNION ALL SELECT node_id, node_kind FROM nodes",
        )
        .await
        {
            let ids = arrow_cast::cast(
                &arrow_cast::cast(b.column(0), &arrow_schema::DataType::Binary).unwrap(),
                &arrow_schema::DataType::FixedSizeBinary(16),
            )
            .unwrap();
            let ids = ids.as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
            let kinds = b.column(1).as_any().downcast_ref::<Int16Array>().unwrap();
            for i in 0..b.num_rows() {
                out.insert((Id(ids.value(i).try_into().unwrap()).hex(), kinds.value(i)));
            }
        }
        out
    };
    let (a, _da) = shapes("one", false).await;
    let (b, _db) = shapes("two/deeper", true).await;
    let (a, b) = (ids(a).await, ids(b).await);
    assert!(!a.is_empty());
    assert_eq!(
        a, b,
        "another location and the reversed module order give the same catalogs"
    );
}

/// The registry rules that guard the derivations themselves (evidence, support, one edge per
/// evidence row, no parallel edges where the kind forbids them, typed targets), and the rules no
/// other case reached (C6 review F1): each rejects a published table replaced by a doctored view.
/// A case runs its own rule's query, the shared validator (§8); the restored snapshot then passes
/// every rule.
#[tokio::test]
async fn each_graph_rule_rejects_a_doctored_catalog() {
    let (ctx, _dir) = shapes("one", false).await;
    let doctored = [
        "edges",
        "call_targets",
        "resolutions",
        "exports",
        "ancestry_targets",
        "override_targets",
        "signatures",
        "provider_node_map",
        "pysa_functions",
        "facts",
        "arguments",
        "context_definitions",
        "context_modules",
        "syntax_nodes",
        "source_files",
    ];
    for table in doctored {
        let published = ctx.table(table).await.unwrap();
        ctx.register_table(format!("{table}_published").as_str(), published.into_view())
            .unwrap();
    }
    let (declares, call) = (EdgeKind::Declares.code(), EdgeKind::CallTarget.code());
    let cases = [
        (
            "one-per-evidence:declares",
            "edges",
            format!(
                "SELECT * FROM edges_published UNION ALL \
                 (SELECT * FROM edges_published WHERE edge_kind = {declares} LIMIT 1)"
            ),
        ),
        (
            "no-parallel:declares",
            "edges",
            format!(
                "SELECT * FROM edges_published UNION ALL \
                 (SELECT * FROM edges_published WHERE edge_kind = {declares} LIMIT 1)"
            ),
        ),
        (
            "evidence:call_target",
            "edges",
            format!(
                "SELECT snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, \
                        dst_kind, ordinal, \
                        CASE WHEN edge_kind = {call} THEN support_fact_id \
                             ELSE evidence_fact_id END AS evidence_fact_id, support_fact_id \
                 FROM edges_published"
            ),
        ),
        (
            "support:edges",
            "edges",
            "SELECT snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, \
                    dst_kind, ordinal, evidence_fact_id, \
                    CAST(X'00000000000000000000000000000000' AS BYTEA) AS support_fact_id \
             FROM edges_published"
                .to_owned(),
        ),
        (
            "endpoint:call_target",
            "edges",
            format!(
                "SELECT snapshot_id, edge_id, edge_kind, src_node_id, \
                        CASE WHEN edge_kind = {call} THEN NULL ELSE src_kind END AS src_kind, \
                        dst_node_id, dst_kind, ordinal, evidence_fact_id, support_fact_id \
                 FROM edges_published"
            ),
        ),
        (
            "typed:call_targets",
            "call_targets",
            "SELECT snapshot_id, call_site_node_id, pysa_fact_id, argument_node_id, \
                    CAST(NULL AS BYTEA) AS target_node_id, reason \
             FROM call_targets_published"
                .to_owned(),
        ),
        // C6 review F1: an unresolved remainder no resolution counts (the `getattr(...)()` site).
        (
            "partition:pysa_calls-remainders",
            "resolutions",
            "SELECT * REPLACE (false AS has_unresolved_remainder) FROM resolutions_published"
                .to_owned(),
        ),
        (
            "typed:exports",
            "exports",
            "SELECT * REPLACE (CAST(NULL AS BYTEA) AS target_node_id, \
                               CAST(NULL AS SMALLINT) AS reason) FROM exports_published"
                .to_owned(),
        ),
        (
            "typed:ancestry_targets",
            "ancestry_targets",
            "SELECT * REPLACE (CAST(NULL AS BYTEA) AS ancestor_node_id, \
                               CAST(NULL AS SMALLINT) AS reason) FROM ancestry_targets_published"
                .to_owned(),
        ),
        (
            "typed:override_targets",
            "override_targets",
            "SELECT * REPLACE (CAST(NULL AS BYTEA) AS overridden_node_id, \
                               CAST(NULL AS SMALLINT) AS reason) FROM override_targets_published"
                .to_owned(),
        ),
        // A Pysa function with signatures that the signature table lost.
        (
            "semantic:pysa-signatures-placed",
            "signatures",
            "SELECT * FROM signatures_published WHERE false".to_owned(),
        ),
        // Two provider functions mapped onto one declaration.
        (
            "semantic:stage-c-injective",
            "provider_node_map",
            "SELECT * FROM provider_node_map_published UNION ALL \
             (SELECT * REPLACE (function_key || '#twin' AS function_key) \
              FROM provider_node_map_published WHERE node_id IS NOT NULL LIMIT 1)"
                .to_owned(),
        ),
        // Parameter semantics for a function Pysa never reported.
        (
            "semantic:parameter-semantics-function",
            "pysa_functions",
            "SELECT * FROM pysa_functions_published WHERE false".to_owned(),
        ),
        // A fact whose model is not its run's producer.
        (
            "semantic:model-id-producer",
            "facts",
            "SELECT * REPLACE ('00000000000000000000000000000000/x' AS model_id) \
             FROM facts_published"
                .to_owned(),
        ),
        (
            "id:arguments",
            "arguments",
            "SELECT * REPLACE (ordinal + 1 AS ordinal) FROM arguments_published".to_owned(),
        ),
        (
            "id:context_definitions",
            "context_definitions",
            "SELECT * REPLACE (key || 'x' AS key) FROM context_definitions_published".to_owned(),
        ),
        (
            "id:context_modules",
            "context_modules",
            "SELECT * REPLACE (coalesce(distribution, 'dep') AS distribution, \
                               module_name || 'x' AS module_name) \
             FROM context_modules_published"
                .to_owned(),
        ),
        // A call site with no placement row.
        (
            "placed:call_syntax",
            "syntax_nodes",
            "SELECT * FROM syntax_nodes_published WHERE false".to_owned(),
        ),
        // A run whose release has neither modules nor documents.
        (
            "ref:runs.release_id->source_files|documents",
            "source_files",
            "SELECT * FROM source_files_published WHERE false".to_owned(),
        ),
    ];
    let rules = cpg_schema::rules::rules();
    for (rule, table, view) in cases {
        let doctored = sql::query(&ctx, &view).await.unwrap().into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, doctored).unwrap();
        let query = &rules.iter().find(|r| r.name == rule).expect(rule).sql;
        let rows: usize = sql::query(&ctx, query)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap()
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
    // Restored, the snapshot passes every rule.
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}
