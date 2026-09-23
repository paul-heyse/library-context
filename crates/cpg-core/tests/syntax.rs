//! CPG slice C2 (DESIGN §3.2 `syntax`): the placed syntax tree on the `syntax_shapes` fixture, and
//! Pysa's non-call sites resolved to its nodes.

use std::collections::BTreeMap;
use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch, StringArray};
use cpg_core::attempt::compile;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, extract};
use cpg_schema::codebook::{Codebook, EdgeKind, SyntaxField, SyntaxKind};
use cpg_schema::id::Id;
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

async fn published_fixture(fixture: &str) -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    copy(
        &repo.join("fixtures/python").join(fixture),
        &dir.path().join("release"),
    );
    std::fs::create_dir_all(dir.path().join("venv/site-packages")).unwrap();
    let s = Id([6; 16]);
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(dir.path().join("release")).unwrap(),
            fixture,
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(dir.path().join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(dir.path().join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: s,
        keep_pysa_json: false,
        test_hooks: Default::default(),
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

fn ids(b: &RecordBatch, c: usize) -> Vec<Option<String>> {
    let binary = arrow_cast::cast(b.column(c), &arrow_schema::DataType::Binary).unwrap();
    let fixed = arrow_cast::cast(&binary, &arrow_schema::DataType::FixedSizeBinary(16)).unwrap();
    let a = fixed
        .as_any()
        .downcast_ref::<FixedSizeBinaryArray>()
        .unwrap();
    (0..a.len())
        .map(|i| (!a.is_null(i)).then(|| Id(a.value(i).try_into().unwrap()).hex()))
        .collect()
}

/// Each module's placed tree, indented by depth: kind, field, ordinal and detail. Declarations
/// and calls appear under their own ids, so the tree is one piece.
async fn tree(ctx: &SessionContext) -> String {
    struct Row {
        parent: String,
        kind: i16,
        field: i16,
        ordinal: i64,
        detail: Option<String>,
        start: i64,
    }
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for b in batches(
        ctx,
        "SELECT node_id, parent_node_id, kind, field, ordinal, detail, start_byte FROM syntax_nodes",
    )
    .await
    {
        let (node, parent) = (ids(&b, 0), ids(&b, 1));
        let i16s = |c: usize| b.column(c).as_any().downcast_ref::<Int16Array>().unwrap().clone();
        let (kind, field) = (i16s(2), i16s(3));
        let ordinal = b.column(4).as_any().downcast_ref::<Int64Array>().unwrap().clone();
        let detail = arrow_cast::cast(b.column(5), &arrow_schema::DataType::Utf8).unwrap();
        let detail = detail.as_any().downcast_ref::<StringArray>().unwrap().clone();
        let start = b.column(6).as_any().downcast_ref::<Int64Array>().unwrap().clone();
        for i in 0..b.num_rows() {
            rows.insert(
                node[i].clone().unwrap(),
                Row {
                    parent: parent[i].clone().unwrap(),
                    kind: kind.value(i),
                    field: field.value(i),
                    ordinal: ordinal.value(i),
                    detail: (!detail.is_null(i)).then(|| detail.value(i).to_owned()),
                    start: start.value(i),
                },
            );
        }
    }
    let mut children: BTreeMap<String, Vec<(i64, i16, i64, String)>> = BTreeMap::new();
    for (id, r) in &rows {
        children.entry(r.parent.clone()).or_default().push((
            r.start,
            r.field,
            r.ordinal,
            id.clone(),
        ));
    }
    for v in children.values_mut() {
        v.sort();
    }
    let roots: Vec<String> = children
        .keys()
        .filter(|p| !rows.contains_key(*p))
        .cloned()
        .collect();
    fn render(
        out: &mut String,
        id: &str,
        depth: usize,
        rows: &BTreeMap<String, Row>,
        children: &BTreeMap<String, Vec<(i64, i16, i64, String)>>,
    ) {
        let r = &rows[id];
        out.push_str(&format!(
            "{}{} [{} {}]{}\n",
            "  ".repeat(depth),
            SyntaxKind::from_code(r.kind).unwrap().text(),
            SyntaxField::from_code(r.field).unwrap().text(),
            r.ordinal,
            r.detail
                .as_ref()
                .map(|d| format!(" {d}"))
                .unwrap_or_default()
        ));
        for (_, _, _, c) in children.get(id).into_iter().flatten() {
            render(out, c, depth + 1, rows, children);
        }
    }
    let mut out = String::new();
    for root in roots {
        for (_, _, _, c) in &children[&root] {
            render(&mut out, c, 0, &rows, &children);
        }
    }
    out
}

#[tokio::test]
async fn the_syntax_tree_places_what_the_passes_read() {
    let (ctx, _dir) = published_fixture("syntax_shapes").await;
    insta::assert_snapshot!(tree(&ctx).await);
    // Pysa's `for`/`with` protocol, operator and comparison sites land on placed nodes: a site
    // edge from a syntax node for each resolved record.
    let sites = batches(
        &ctx,
        &format!(
            "SELECT count(*) FROM site_targets t WHERE t.site_node_id IS NULL \
             UNION ALL SELECT count(*) FROM edges WHERE edge_kind = {}",
            EdgeKind::SiteTarget.code()
        ),
    )
    .await;
    let counts: Vec<i64> = sites
        .iter()
        .flat_map(|b| {
            let a = b.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
            (0..a.len()).map(|i| a.value(i)).collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(counts[0], 0, "every site has its node");
    assert!(counts[1] > 0, "site edges exist");
}

/// C3 (DESIGN §3.2 `lexical`): every name reference of the `lexical_shapes` fixture and what the
/// recognizer resolves it to, rendered by name, line and scope.
#[tokio::test]
async fn names_resolve_under_python_scoping() {
    let (ctx, _dir) = published_fixture("lexical_shapes").await;
    let text = |b: &RecordBatch, c: usize| -> Vec<Option<String>> {
        let a = arrow_cast::cast(b.column(c), &arrow_schema::DataType::Utf8).unwrap();
        let a = a.as_any().downcast_ref::<StringArray>().unwrap().clone();
        (0..a.len())
            .map(|i| (!a.is_null(i)).then(|| a.value(i).to_owned()))
            .collect()
    };
    let mut out = String::new();
    for b in batches(
        &ctx,
        "SELECT f.path || ':' || CAST(r.start_byte AS VARCHAR) || ' ' || r.name AS reference, \
                CAST(s.kind AS VARCHAR) AS scope, \
                CASE WHEN x.binding_id IS NOT NULL \
                     THEN 'binding ' || CAST(b.kind AS VARCHAR) || ' in scope ' \
                          || CAST(bs.kind AS VARCHAR) || ' at ' || CAST(b.start_byte AS VARCHAR) \
                          || CASE WHEN b.static_branch IS NOT NULL \
                                  THEN ' [static ' || CAST(b.static_branch AS VARCHAR) || ' ' \
                                       || CAST(b.static_polarity AS VARCHAR) || ']' ELSE '' END \
                          || CASE WHEN x.captured THEN ' (captured)' ELSE '' END \
                     WHEN x.builtin_name IS NOT NULL THEN 'builtin ' || x.builtin_name \
                     ELSE 'reason ' || CAST(x.reason AS VARCHAR) END AS resolves_to \
         FROM references r \
         JOIN source_files f ON f.module_node_id = r.module_node_id \
         JOIN scopes s ON s.node_id = r.scope_id \
         JOIN reference_resolutions x ON x.reference_id = r.node_id \
         LEFT JOIN bindings b ON b.node_id = x.binding_id \
         LEFT JOIN scopes bs ON bs.node_id = b.scope_id \
         ORDER BY f.path, r.start_byte, resolves_to",
    )
    .await
    {
        let (a, s, t) = (text(&b, 0), text(&b, 1), text(&b, 2));
        for i in 0..b.num_rows() {
            out.push_str(&format!(
                "{} (scope {}) -> {}\n",
                a[i].as_deref().unwrap_or(""),
                s[i].as_deref().unwrap_or(""),
                t[i].as_deref().unwrap_or("")
            ));
        }
    }
    out.push_str("## bindings\n");
    for b in batches(
        &ctx,
        "SELECT f.path || ':' || CAST(b.start_byte AS VARCHAR) || ' ' || b.name || ' kind ' \
                || CAST(b.kind AS VARCHAR) || ' in scope ' || CAST(s.kind AS VARCHAR) \
                || ' #' || CAST(b.ordinal AS VARCHAR) \
                || CASE WHEN b.static_branch IS NOT NULL \
                        THEN ' [static ' || CAST(b.static_branch AS VARCHAR) || ' ' \
                             || CAST(b.static_polarity AS VARCHAR) || ']' ELSE '' END AS line \
         FROM bindings b JOIN scopes s ON s.node_id = b.scope_id \
         JOIN source_files f ON f.module_node_id = b.module_node_id \
         ORDER BY f.path, b.start_byte, b.name",
    )
    .await
    {
        for line in text(&b, 0) {
            out.push_str(&format!("{}\n", line.unwrap_or_default()));
        }
    }
    insta::assert_snapshot!(out);
}
