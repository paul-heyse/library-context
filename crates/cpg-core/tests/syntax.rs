//! CPG slices C2–C5 (DESIGN §3.2): the placed syntax tree on the `syntax_shapes` fixture and
//! Pysa's non-call sites resolved to its nodes; name resolution on `lexical_shapes`; type terms,
//! observations and record fields on `type_shapes`; a corpus's documents and mentions on
//! `docs_shapes`.

use std::collections::BTreeMap;
use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch, StringArray};
use cpg_core::attempt::compile;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, extract};
use cpg_schema::codebook::{Codebook, EdgeKind, SourceRole, SyntaxField, SyntaxKind};
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
        corpus: None,
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

#[tokio::test(flavor = "multi_thread")]
async fn the_syntax_tree_places_what_the_passes_read() {
    let (ctx, _dir) = published_fixture("syntax_shapes").await;
    insta::assert_snapshot!(tree(&ctx).await);
    // Pysa's `for`/`with` protocol, operator and comparison sites land on placed nodes: a site
    // edge from a syntax node for each resolved record. (Two queries: a UNION ALL's branch order
    // is not an output order once plans run on several partitions.)
    let count = |statement: String| {
        let ctx = &ctx;
        async move {
            let b = batches(ctx, &statement).await;
            b[0].column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .value(0)
        }
    };
    let unplaced =
        count("SELECT count(*) FROM site_targets t WHERE t.site_node_id IS NULL".to_owned()).await;
    let site_edges = count(format!(
        "SELECT count(*) FROM edges WHERE edge_kind = {}",
        EdgeKind::SiteTarget.code()
    ))
    .await;
    assert_eq!(unplaced, 0, "every site has its node");
    assert!(site_edges > 0, "site edges exist");
}

/// C3 (DESIGN §3.2 `lexical`): every name reference of the `lexical_shapes` fixture and what the
/// recognizer resolves it to, rendered by name, line and scope.
#[tokio::test(flavor = "multi_thread")]
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
                COALESCE(sd.qualified_name, \
                         CAST(s.kind AS VARCHAR) || '@' || CAST(s.start_byte AS VARCHAR)) AS scope, \
                CASE WHEN x.binding_id IS NOT NULL \
                     THEN 'binding ' || CAST(b.kind AS VARCHAR) || ' in scope ' \
                          || COALESCE(bd.qualified_name, CAST(bs.kind AS VARCHAR) || '@' \
                                      || CAST(bs.start_byte AS VARCHAR)) \
                          || ' at ' || CAST(b.start_byte AS VARCHAR) \
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
         LEFT JOIN declarations sd ON sd.node_id = s.owner_node_id \
         LEFT JOIN bindings b ON b.node_id = x.binding_id \
         LEFT JOIN scopes bs ON bs.node_id = b.scope_id \
         LEFT JOIN declarations bd ON bd.node_id = bs.owner_node_id \
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
                || CAST(b.kind AS VARCHAR) || ' in scope ' \
                || COALESCE(d.qualified_name, \
                            CAST(s.kind AS VARCHAR) || '@' || CAST(s.start_byte AS VARCHAR)) \
                || ' #' || CAST(b.ordinal AS VARCHAR) \
                || CASE WHEN b.static_branch IS NOT NULL \
                        THEN ' [static ' || CAST(b.static_branch AS VARCHAR) || ' ' \
                             || CAST(b.static_polarity AS VARCHAR) || ']' ELSE '' END AS line \
         FROM bindings b JOIN scopes s ON s.node_id = b.scope_id \
         LEFT JOIN declarations d ON d.node_id = s.owner_node_id \
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

/// Text of each row's columns, joined by ` | ` (nulls as `-`).
async fn lines(ctx: &SessionContext, statement: &str) -> String {
    let mut out = String::new();
    for b in batches(ctx, statement).await {
        let cols: Vec<StringArray> = (0..b.num_columns())
            .map(|c| {
                let a = arrow_cast::cast(b.column(c), &arrow_schema::DataType::Utf8).unwrap();
                a.as_any().downcast_ref::<StringArray>().unwrap().clone()
            })
            .collect();
        for i in 0..b.num_rows() {
            let row: Vec<&str> = cols
                .iter()
                .map(|c| if c.is_null(i) { "-" } else { c.value(i) })
                .collect();
            out.push_str(&row.join(" | "));
            out.push('\n');
        }
    }
    out
}

/// C4 (DESIGN §3.2 `types`, §3.5.1): what each element's type is, the terms' structure and
/// binders, the classes they name, and the record fields, on the `type_shapes` fixture.
#[tokio::test(flavor = "multi_thread")]
async fn types_keep_structure_binders_and_record_fields() {
    let (ctx, _dir) = published_fixture("type_shapes").await;
    let role = "CASE o.role WHEN 0 THEN 'parameter' WHEN 1 THEN 'return' \
                WHEN 2 THEN 'call_result' WHEN 3 THEN 'argument' ELSE 'raised' END";
    let mut out = String::from("## observations: subject | role | declared | type\n");
    out += &lines(
        &ctx,
        &format!(
            "SELECT COALESCE(pd.qualified_name || '.' || p.name, d.qualified_name || '()', \
                             'call@' || CAST(c.start_byte AS VARCHAR), \
                             'arg@' || CAST(a.start_byte AS VARCHAR), \
                             'raise@' || CAST(r.start_byte AS VARCHAR)) AS subject, \
                    {role} AS role, CAST(o.declared AS VARCHAR), t.display \
             FROM type_observations o JOIN type_terms t ON t.node_id = o.term_node_id \
             LEFT JOIN parameter_syntax p ON p.node_id = o.subject_node_id \
             LEFT JOIN declarations pd ON pd.node_id = p.function_node_id \
             LEFT JOIN declarations d ON d.node_id = o.subject_node_id \
             LEFT JOIN call_syntax c ON c.node_id = o.subject_node_id \
             LEFT JOIN arguments a ON a.node_id = o.subject_node_id \
             LEFT JOIN syntax_nodes r ON r.node_id = o.subject_node_id \
             ORDER BY subject, role, 3, t.display"
        ),
    )
    .await;
    out += "## type variables: display | kind | variable | binder | reason\n";
    out += &lines(
        &ctx,
        "SELECT t.display, CAST(t.kind AS VARCHAR), t.variable, \
                COALESCE(d.qualified_name, CAST(sn.kind AS VARCHAR)), CAST(b.reason AS VARCHAR) \
         FROM type_terms t LEFT JOIN type_binders b ON b.term_node_id = t.node_id \
         LEFT JOIN declarations d ON d.node_id = b.binder_node_id \
         LEFT JOIN syntax_nodes sn ON sn.node_id = b.binder_node_id AND d.node_id IS NULL \
         WHERE t.variable IS NOT NULL ORDER BY t.variable, t.display",
    )
    .await;
    out += "## variable children: variable | role | child\n";
    out += &lines(
        &ctx,
        "SELECT p.display, CAST(a.role AS VARCHAR), c.display \
         FROM type_term_args a JOIN type_terms p ON p.node_id = a.parent_node_id \
         JOIN type_terms c ON c.node_id = a.child_node_id \
         WHERE p.variable IS NOT NULL ORDER BY 1, 2, 3",
    )
    .await;
    out += "## types boundaries: reason | span | detail\n";
    out += &lines(
        &ctx,
        "SELECT CAST(reason AS VARCHAR), CAST(start_byte AS VARCHAR) AS s, detail \
         FROM boundaries WHERE fact_family = 9 ORDER BY start_byte, detail",
    )
    .await;
    out += "## classes: term | class\n";
    out += &lines(
        &ctx,
        &format!(
            "SELECT t.display, COALESCE(d.qualified_name, m.module_name || '#' || x.key) \
             FROM edges e JOIN type_terms t ON t.node_id = e.src_node_id \
             LEFT JOIN declarations d ON d.node_id = e.dst_node_id \
             LEFT JOIN context_definitions x ON x.symbol_node_id = e.dst_node_id \
             LEFT JOIN context_modules m ON m.module_node_id = x.module_node_id \
             WHERE e.edge_kind = {} ORDER BY t.display",
            EdgeKind::TypeClass.code()
        ),
    )
    .await;
    out += "## record fields: class | kind | # | name | type | declared | default | init | alias \
            | kw_only | required | read_only\n";
    out += &lines(
        &ctx,
        "SELECT d.qualified_name, CAST(f.record_kind AS VARCHAR) AS kind, CAST(f.ordinal AS VARCHAR) AS ord, \
                f.name, t.display, CAST(f.declared AS VARCHAR), CAST(f.has_default AS VARCHAR), \
                CAST(f.init AS VARCHAR), f.alias, CAST(f.kw_only AS VARCHAR), \
                CAST(f.required AS VARCHAR), CAST(f.read_only AS VARCHAR) \
         FROM record_fields f JOIN declarations d ON d.node_id = f.class_node_id \
         JOIN type_terms t ON t.node_id = f.term_node_id \
         ORDER BY d.qualified_name, f.ordinal",
    )
    .await;
    // The recursive alias is one finite term, and nothing was cut at the depth cap.
    out += "## structure of `leaves`'s parameter type: parent | role | # | child\n";
    out += &lines(
        &ctx,
        "WITH RECURSIVE s(node_id, depth) AS ( \
           SELECT o.term_node_id, 0 FROM type_observations o \
           JOIN parameter_syntax p ON p.node_id = o.subject_node_id \
           JOIN declarations d ON d.node_id = p.function_node_id \
           WHERE d.qualified_name = 'ts.leaves' \
           UNION ALL SELECT a.child_node_id, s.depth + 1 FROM type_term_args a \
           JOIN s ON s.node_id = a.parent_node_id WHERE s.depth < 10) \
         SELECT DISTINCT pt.display, CAST(a.role AS VARCHAR), CAST(a.ordinal AS VARCHAR), \
                ct.display \
         FROM s JOIN type_term_args a ON a.parent_node_id = s.node_id \
         JOIN type_terms pt ON pt.node_id = a.parent_node_id \
         JOIN type_terms ct ON ct.node_id = a.child_node_id ORDER BY 1, 2, 3, 4",
    )
    .await;
    insta::assert_snapshot!(out);

    // Two unrelated `T`s are two terms with two binders.
    let ts = lines(
        &ctx,
        "SELECT DISTINCT d.qualified_name FROM type_observations o \
         JOIN type_terms t ON t.node_id = o.term_node_id \
         JOIN type_binders b ON b.term_node_id = t.node_id \
         JOIN declarations d ON d.node_id = b.binder_node_id \
         JOIN declarations f ON f.node_id = o.subject_node_id \
         WHERE o.role = 1 AND t.display = 'T' AND f.qualified_name IN ('ts.first', 'ts.ident') \
         ORDER BY 1",
    )
    .await;
    assert_eq!(ts, "ts.first\nts.ident\n");
    let truncated = lines(&ctx, "SELECT count(*) FROM type_terms WHERE kind = 27").await;
    assert_eq!(truncated, "0\n");
    // One variable is one term wherever it is observed (C4 review F3); `P`, `P.args` and
    // `P.kwargs` are three forms of one variable.
    let split = lines(
        &ctx,
        "SELECT variable FROM type_terms WHERE variable IS NOT NULL \
         GROUP BY variable, kind, detail HAVING count(DISTINCT node_id) > 1",
    )
    .await;
    assert_eq!(split, "");
    // Two same-named enums are two literal terms, each with its class (C4 review F5).
    let enums = lines(
        &ctx,
        &format!(
            "SELECT count(DISTINCT t.node_id) FROM type_terms t JOIN edges e \
               ON e.src_node_id = t.node_id AND e.edge_kind = {} \
             WHERE t.kind = 11 AND t.detail = 'RED'",
            EdgeKind::TypeClass.code()
        ),
    )
    .await;
    assert_eq!(enums, "2\n");
}

/// The `docs_shapes` library installed as an acquired one is (its files in site-packages, owned by
/// its distribution's `RECORD`), with its corpus from `corpus/` under `dir`, and each of `extra`
/// (source, destination under the tree) copied into the tree too.
fn corpus_input(dir: &Path, s: Id, extra: &[(&str, &str)]) -> ExtractInput {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join("fixtures/python/docs_shapes");
    copy(&fixture.join("release"), &dir.join("venv/site-packages"));
    copy(&fixture.join("corpus"), &dir.join("corpus"));
    for (from, to) in extra {
        copy(&fixture.join(from), &dir.join("corpus").join(to));
    }
    let site = std::fs::canonicalize(dir.join("venv/site-packages")).unwrap();
    let library = cpg_extract::library::AcquiredLibrary {
        name: "pkg".to_owned(),
        requirement: "pkg==1.0".to_owned(),
        lock_digest: cpg_schema::id::Digest::ZERO,
        release: vec!["pkg==1.0".to_owned()],
        installer: None,
        distributions: vec![cpg_extract::library::Distribution {
            name: "pkg".to_owned(),
            version: "1.0".to_owned(),
            artifact_sha256: Vec::new(),
            record_digest: cpg_schema::id::Digest::ZERO,
        }],
        owners: ["pkg/__init__.py", "pkg/core.py"]
            .into_iter()
            .map(|p| (p.to_owned(), "pkg".to_owned()))
            .collect(),
    };
    let mut input = ExtractInput {
        release: cpg_extract::Release {
            root: site.clone(),
            files: vec![site.join("pkg/__init__.py"), site.join("pkg/core.py")],
            release_id: Id([8; 16]),
            origin: cpg_extract::ReleaseOrigin::Library(library),
        },
        venv_root: std::fs::canonicalize(dir.join("venv")).unwrap(),
        site_packages: vec![site],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    };
    let tree = std::fs::canonicalize(dir.join("corpus")).unwrap();
    input.corpus = Some(cpg_extract::library::corpus(&tree, &source(), &input).unwrap());
    input
}

fn source() -> cpg_extract::library::Source {
    cpg_extract::library::Source {
        repository: "https://example.invalid/pkg".to_owned(),
        tag: "v1".to_owned(),
        commit: "0".repeat(40),
        documents: vec!["docs/**/*.mdx".to_owned()],
        documents_exclude: vec!["docs/old/**".to_owned()],
        examples: vec!["examples/**/*.py".to_owned()],
        examples_exclude: vec![],
        tests: vec!["tests/**/*.py".to_owned()],
        tests_exclude: vec![],
    }
}

/// C5 (DESIGN §3.2 `docs`): a corpus run beside the library run. The documents the selection
/// keeps, their passages, code blocks and links, the mentions of the library's API by class, and
/// each document's coverage.
#[tokio::test(flavor = "multi_thread")]
async fn a_corpus_documents_its_library() {
    let dir = tempfile::tempdir().unwrap();
    let s = Id([7; 16]);
    let input = corpus_input(dir.path(), s, &[]);
    let out = extract(&input).unwrap();
    let store = dir.path().join("store");
    compile(&store, s, &out.tables).await.unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();

    let mut text = String::from("## documents: path | title | parsed\n");
    text += &lines(
        &ctx,
        "SELECT path, title, CAST(parsed AS VARCHAR) FROM documents ORDER BY path",
    )
    .await;
    text += "## passages: ordinal | level | heading | path | span\n";
    text += &lines(
        &ctx,
        "SELECT CAST(p.ordinal AS VARCHAR) AS ord, CAST(p.level AS VARCHAR) AS lvl, p.heading, \
                array_to_string(p.heading_path, ' > '), \
                CAST(p.start_byte AS VARCHAR) || '-' || CAST(p.end_byte AS VARCHAR) \
         FROM passages p ORDER BY p.ordinal",
    )
    .await;
    text += "## code blocks: passage | language | code\n";
    text += &lines(
        &ctx,
        "SELECT p.heading, b.language, replace(b.code, chr(10), ' / ') \
         FROM code_blocks b JOIN passages p ON p.node_id = b.passage_node_id ORDER BY b.ordinal",
    )
    .await;
    text += "## links: passage | url | text\n";
    text += &lines(
        &ctx,
        "SELECT p.heading, l.url, l.text \
         FROM doc_links l JOIN passages p ON p.node_id = l.passage_node_id ORDER BY l.start_byte",
    )
    .await;
    text += "## mentions: passage | class | source | form | target | modality | edge target\n";
    text += &lines(
        &ctx,
        &format!(
            "SELECT COALESCE(p.heading, '(preamble)'), \
                    CASE m.class WHEN 0 THEN 'exact' ELSE 'lexical' END, \
                    CASE m.source WHEN 0 THEN 'code' ELSE 'prose' END, m.form, \
                    COALESCE(m.access_path, m.qualified_name), CAST(f.modality AS VARCHAR), \
                    COALESCE(x.access_path, d.qualified_name) \
             FROM mentions m JOIN passages p ON p.node_id = m.passage_node_id \
             JOIN facts f ON f.fact_id = m.fact_id \
             LEFT JOIN edges e ON e.evidence_fact_id = m.fact_id AND e.edge_kind = {} \
             LEFT JOIN (SELECT DISTINCT access_path, export_node_id FROM exports) x \
               ON x.export_node_id = e.dst_node_id \
             LEFT JOIN declarations d ON d.node_id = e.dst_node_id \
             ORDER BY m.start_byte, 5",
            EdgeKind::Mentions.code()
        ),
    )
    .await;
    text += "## modules: path | role (ADR-0015)\n";
    text += &lines(
        &ctx,
        "SELECT f.path, CAST(f.role AS VARCHAR) FROM source_files f ORDER BY f.path",
    )
    .await;
    text += "## corpus modules: path | coverage (family: status)\n";
    text += &lines(
        &ctx,
        "SELECT f.path, string_agg(CAST(c.fact_family AS VARCHAR) || ':' \
                                   || CAST(c.status AS VARCHAR), ' ' ORDER BY c.fact_family) \
         FROM source_files f JOIN releases r ON r.release_id = f.release_id \
         JOIN coverage c ON c.scope_node_id = f.module_node_id \
         WHERE r.label IS NOT NULL GROUP BY f.path ORDER BY f.path",
    )
    .await;
    text += "## code block → module\n";
    text += &lines(
        &ctx,
        &format!(
            "SELECT b.module_path, f.module_name FROM edges e \
             JOIN code_blocks b ON b.node_id = e.src_node_id \
             JOIN source_files f ON f.module_node_id = e.dst_node_id \
             WHERE e.edge_kind = {} ORDER BY 1",
            EdgeKind::BlockModule.code()
        ),
    )
    .await;
    text += "## usage calls reaching the release: call site | release callable\n";
    text += &lines(
        &ctx,
        &format!(
            "SELECT f.path || ':' || CAST(c.start_byte AS VARCHAR) AS site, d.qualified_name \
             FROM edges e JOIN call_syntax c ON c.node_id = e.src_node_id \
             JOIN source_files f ON f.module_node_id = c.module_node_id \
             JOIN declarations d ON d.node_id = e.dst_node_id \
             JOIN releases r ON r.release_id = f.release_id \
             WHERE e.edge_kind = {} AND r.label IS NOT NULL ORDER BY 1, 2",
            EdgeKind::CallTarget.code()
        ),
    )
    .await;
    text += "## coverage: path | status | reason\n";
    text += &lines(
        &ctx,
        "SELECT d.path, CAST(c.status AS VARCHAR), CAST(c.reason AS VARCHAR) \
         FROM coverage c JOIN documents d ON d.node_id = c.scope_node_id ORDER BY d.path",
    )
    .await;
    insta::assert_snapshot!(text);
    // The library's own class is one term whichever run observes it (C5 review F2).
    let server = lines(
        &ctx,
        "SELECT count(DISTINCT node_id) FROM type_terms WHERE display = 'Server' AND kind = 0",
    )
    .await;
    assert_eq!(server, "1\n");

    // ADR-0015: a snippet is read from Delta alone. With the fetched tree and the environment
    // gone, each example and test call is sliced from its module's stored text by its span.
    std::fs::remove_dir_all(dir.path().join("corpus")).unwrap();
    std::fs::remove_dir_all(dir.path().join("venv")).unwrap();
    let rows = batches(
        &ctx,
        &format!(
            "SELECT f.text, c.start_byte, c.end_byte FROM call_syntax c \
             JOIN source_files f ON f.module_node_id = c.module_node_id \
             WHERE f.role IN ({}, {}) ORDER BY f.path, c.start_byte",
            SourceRole::Example.code(),
            SourceRole::Test.code()
        ),
    )
    .await;
    let mut snippets = Vec::new();
    for b in &rows {
        let texts = arrow_cast::cast(b.column(0), &arrow_schema::DataType::Utf8).unwrap();
        let texts = texts.as_any().downcast_ref::<StringArray>().unwrap();
        let int = |c: usize| b.column(c).as_any().downcast_ref::<Int64Array>().unwrap();
        let (starts, ends) = (int(1), int(2));
        for i in 0..b.num_rows() {
            let span = starts.value(i) as usize..ends.value(i) as usize;
            snippets.push(texts.value(i)[span].to_owned());
        }
    }
    assert_eq!(
        snippets,
        [
            "make_server(\"demo\")",
            "server.run()",
            "make_server(\"x\")",
            "server.tool(print)",
            "isinstance(server, Server)",
        ]
    );
}

/// The corpus's identity is a function of its inputs, not of where they sit (C5 review F1): the
/// same corpus compiled in two places is one context and one run, and it names the library release.
#[test]
fn a_corpus_run_does_not_depend_on_its_location() {
    let run = |dir: &Path| {
        let out = extract(&corpus_input(dir, Id([7; 16]), &[])).unwrap();
        let runs = out
            .tables
            .iter()
            .find(|(n, _)| *n == "runs")
            .map(|(_, b)| b.clone())
            .unwrap();
        ids(&runs, runs.schema().index_of("run_id").unwrap())
            .into_iter()
            .chain(ids(&runs, runs.schema().index_of("context_id").unwrap()))
            .collect::<Vec<_>>()
    };
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert_eq!(run(a.path()), run(b.path()));
    // Another library release is another corpus: its vocabulary and the files it reaches differ.
    let input = corpus_input(a.path(), Id([7; 16]), &[]);
    let mut other = input.clone();
    other.release.release_id = Id([9; 16]);
    let tree = std::fs::canonicalize(a.path().join("corpus")).unwrap();
    other.corpus = Some(cpg_extract::library::corpus(&tree, &source(), &other).unwrap());
    let corpus_id = |i: &ExtractInput| i.corpus.as_ref().unwrap().release.release_id;
    assert_ne!(corpus_id(&input), corpus_id(&other));
}

/// A tree holding its own copy of the package (a flat layout) would cut the usage code off the
/// release: the compile fails and names it (C5 review F3).
#[test]
fn a_tree_that_shadows_the_release_fails() {
    let dir = tempfile::tempdir().unwrap();
    let input = corpus_input(dir.path(), Id([7; 16]), &[("release/pkg", "pkg")]);
    let err = extract(&input).unwrap_err().to_string();
    assert!(err.contains("shadows the release"), "{err}");
}

/// A glob that selects nothing is refused at Stage A, not published as an empty corpus (C5
/// review F5).
#[test]
fn a_glob_that_selects_nothing_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let input = corpus_input(dir.path(), Id([7; 16]), &[]);
    let tree = std::fs::canonicalize(dir.path().join("corpus")).unwrap();
    let mut moved = source();
    moved.documents = vec!["documentation/**/*.mdx".to_owned()];
    let err = cpg_extract::library::corpus(&tree, &moved, &input)
        .unwrap_err()
        .to_string();
    assert!(err.contains("selects nothing"), "{err}");
}

/// A fetched tree is read without following links (H1 C2): a symlink a glob selects is refused,
/// naming it, and so is a directory link that could hold a selection; a link an exclude covers is
/// neither followed nor refused, so a loop there is harmless. A source tree refuses any link.
#[test]
fn symlinks_in_a_tree_are_refused_unless_excluded() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let input = corpus_input(dir.path(), Id([7; 16]), &[]);
    let tree = std::fs::canonicalize(dir.path().join("corpus")).unwrap();
    let corpus = || {
        cpg_extract::library::corpus(&tree, &source(), &input)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    let outside = dir.path().join("outside.mdx");
    std::fs::write(&outside, "# Outside\n").unwrap();
    symlink(&outside, tree.join("docs/linked.mdx")).unwrap();
    let err = corpus().unwrap_err();
    assert!(err.contains("symlink docs/linked.mdx"), "{err}");
    std::fs::remove_file(tree.join("docs/linked.mdx")).unwrap();
    symlink(dir.path(), tree.join("docs/more")).unwrap();
    let err = corpus().unwrap_err();
    assert!(err.contains("symlink docs/more"), "{err}");
    std::fs::remove_file(tree.join("docs/more")).unwrap();
    // An exclude matching only some names under a link does not cover it (H1 review F5).
    symlink(dir.path(), tree.join("docs/more")).unwrap();
    let mut partial = source();
    partial.documents_exclude.push("docs/**/_*".to_owned());
    let err = cpg_extract::library::corpus(&tree, &partial, &input)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(err.contains("symlink docs/more"), "{err}");
    std::fs::remove_file(tree.join("docs/more")).unwrap();
    // A link under the tests is refused, unless `tests_exclude` covers it.
    symlink(dir.path(), tree.join("tests/fixtures")).unwrap();
    assert!(corpus().unwrap_err().contains("symlink tests/fixtures"));
    let mut excluded = source();
    excluded.tests_exclude.push("tests/fixtures/**".to_owned());
    cpg_extract::library::corpus(&tree, &excluded, &input).unwrap();
    std::fs::remove_file(tree.join("tests/fixtures")).unwrap();
    symlink(&tree, tree.join("docs/old/loop")).unwrap();
    corpus().unwrap();
    let err = cpg_extract::Release::from_tree(tree.clone(), "t")
        .unwrap_err()
        .to_string();
    assert!(err.contains("loop is a symlink"), "{err}");
}

/// A module has one role: a file `examples` and `tests` both select is refused (ADR-0015).
#[test]
fn a_file_with_two_roles_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let input = corpus_input(dir.path(), Id([7; 16]), &[]);
    let tree = std::fs::canonicalize(dir.path().join("corpus")).unwrap();
    let mut both = source();
    both.examples.push("tests/**/*.py".to_owned());
    let err = cpg_extract::library::corpus(&tree, &both, &input)
        .unwrap_err()
        .to_string();
    assert!(err.contains("both select"), "{err}");
}

/// Each C5 rule rejects an injected violation on the `docs_shapes` corpus, and nothing is
/// published (C5 review F7).
#[tokio::test(flavor = "multi_thread")]
async fn the_corpus_rules_reject_their_violations() {
    type Raw = Vec<(&'static str, RecordBatch)>;
    fn batch<'a>(raw: &'a mut Raw, name: &str) -> &'a mut RecordBatch {
        &mut raw.iter_mut().find(|(n, _)| *n == name).unwrap().1
    }
    fn set(b: &mut RecordBatch, column: &str, array: std::sync::Arc<dyn Array>) {
        let i = b.schema().index_of(column).unwrap();
        let mut columns = b.columns().to_vec();
        columns[i] = array;
        *b = RecordBatch::try_new(b.schema(), columns).unwrap();
    }
    fn shift(b: &mut RecordBatch, column: &str) {
        let moved: Int64Array = b
            .column(b.schema().index_of(column).unwrap())
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .iter()
            .map(|v| v.map(|v| v + 100))
            .collect();
        set(b, column, std::sync::Arc::new(moved));
    }
    fn texts(b: &RecordBatch, column: &str, f: impl Fn(&str) -> String) -> StringArray {
        let a = arrow_cast::cast(
            b.column(b.schema().index_of(column).unwrap()),
            &arrow_schema::DataType::Utf8,
        )
        .unwrap();
        a.as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .iter()
            .map(|v| v.map(&f))
            .collect()
    }
    fn keep(b: &mut RecordBatch, column: &str, keep: impl Fn(&str) -> bool) {
        let values = texts(b, column, str::to_owned);
        let mask: arrow_array::BooleanArray =
            values.iter().map(|v| Some(v.is_none_or(&keep))).collect();
        *b = arrow_select::filter::filter_record_batch(b, &mask).unwrap();
    }
    type Mutation = fn(&mut Raw);
    let cases: [(&str, Mutation); 10] = [
        // C6 review O6: a document of a release no run compiled escapes nothing.
        ("ref:documents.release_id->runs", |raw| {
            let b = batch(raw, "documents");
            let other = arrow_array::FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(
                [9u8; 16],
                b.num_rows(),
            ))
            .unwrap();
            set(b, "release_id", std::sync::Arc::new(other));
        }),
        ("id:documents", |raw| {
            let b = batch(raw, "documents");
            let paths = texts(b, "path", |p| format!("{p}.moved"));
            set(b, "path", std::sync::Arc::new(paths));
        }),
        ("id:passages", |raw| {
            shift(batch(raw, "passages"), "ordinal")
        }),
        ("id:code_blocks", |raw| {
            shift(batch(raw, "code_blocks"), "ordinal")
        }),
        ("typed:mention_targets", |raw| {
            let b = batch(raw, "mentions");
            let paths = texts(b, "access_path", |_| "nowhere.at.all".to_owned());
            set(b, "access_path", std::sync::Arc::new(paths));
        }),
        // One id, two displays: a collision is rejected, never merged.
        ("unique:type_terms", |raw| {
            let b = batch(raw, "type_terms");
            let twin = b.slice(0, 1);
            let mut twin = twin;
            let display = texts(&twin, "display", |d| format!("{d} (twin)"));
            set(&mut twin, "display", std::sync::Arc::new(display));
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &twin]).unwrap();
        }),
        // Two releases of one attempt claiming one path.
        ("unique:release-paths", |raw| {
            let b = batch(raw, "source_files");
            let paths = texts(b, "path", |p| {
                if p.starts_with("tests/") {
                    "pkg/core.py".to_owned()
                } else {
                    p.to_owned()
                }
            });
            set(b, "path", std::sync::Arc::new(paths));
        }),
        ("coverage:complete", |raw| {
            keep(batch(raw, "coverage"), "fact_family", |f| f != "10");
        }),
        ("coverage:family-has-scope", |raw| {
            keep(batch(raw, "documents"), "path", |_| false);
            keep(batch(raw, "coverage"), "fact_family", |f| f != "10");
        }),
        ("lineage:block_module", |raw| {
            let b = batch(raw, "code_blocks");
            let paths = texts(b, "module_path", |_| "_lctx_blocks/nowhere.py".to_owned());
            set(b, "module_path", std::sync::Arc::new(paths));
        }),
    ];
    let dir = tempfile::tempdir().unwrap();
    let s = Id([7; 16]);
    let base = extract(&corpus_input(dir.path(), s, &[])).unwrap().tables;
    for (rule, mutate) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut raw = base.clone();
        mutate(&mut raw);
        match compile(root.path(), s, &raw).await {
            Err(cpg_core::CoreError::Invalid(violations)) => assert!(
                violations.iter().any(|v| v.rule.starts_with(rule)),
                "{rule}: {:?}",
                violations.iter().map(|v| &v.rule).collect::<Vec<_>>()
            ),
            other => panic!("{rule}: expected a validation failure, got {other:?}"),
        }
    }
}

/// DESIGN §10.3's second Outcome leg on a corpus (slice 1.5 review F2, F5). A seed without a
/// docstring takes the lead sentence of a paragraph whose exact mention lies inside that sentence,
/// segmented across a soft line break. A mention past the lead sentence gives nothing, and a
/// changelog is never read. The passage evidence is its passage's exact bytes.
#[tokio::test(flavor = "multi_thread")]
async fn an_outcome_from_the_docs_is_the_sentence_that_mentions_the_seed() {
    let dir = tempfile::tempdir().unwrap();
    let s = Id([7; 16]);
    let input = corpus_input(dir.path(), s, &[("outcome", "docs/outcome")]);
    let out = extract(&input).unwrap();
    let analysis = cpg_core::analyze::Analysis {
        config: lctx_analytics::config::AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["pkg.core"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.make_server"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 2
"#,
        )
        .unwrap(),
        embedder: None,
        techniques: Default::default(),
    };
    let store = dir.path().join("store");
    cpg_core::attempt::compile_analyzed(&store, s, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    let outcomes = lines(
        &ctx,
        "SELECT b.title, CAST(b.documentation_only AS VARCHAR), a.evidence_status, a.text \
         FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE a.assertion_kind = 0 ORDER BY b.title",
    )
    .await;
    assert_eq!(
        outcomes,
        "pkg.Server.tool | true | 1 | Call `make_server()`, then `Server.tool` to register a tool \
         (`pkg.Server.tool` in full).\n\
         pkg.make_server | false | 1 | The `pkg.make_server` factory builds a configured server \
         from its name.\n"
    );
    // Each passage evidence is the bytes of its passage at its span; one crosses a line break.
    let rows = batches(
        &ctx,
        "SELECT e.start_byte - p.start_byte AS at, e.end_byte - e.start_byte AS len, e.text, \
                p.text AS passage \
         FROM evidence e JOIN passages p ON p.node_id = e.node_id WHERE e.evidence_kind = 2",
    )
    .await;
    let mut checked = Vec::new();
    for b in &rows {
        let at = b.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
        let len = b.column(1).as_any().downcast_ref::<Int64Array>().unwrap();
        let cast = |c: usize| {
            arrow_cast::cast(b.column(c), &arrow_schema::DataType::Utf8)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .clone()
        };
        let (text, passage) = (cast(2), cast(3));
        for i in 0..b.num_rows() {
            let (a, n) = (at.value(i) as usize, len.value(i) as usize);
            assert_eq!(
                &passage.value(i).as_bytes()[a..a + n],
                text.value(i).as_bytes()
            );
            checked.push(text.value(i).to_owned());
        }
    }
    assert_eq!(checked.len(), 2, "{checked:?}");
    assert!(checked.iter().any(|t| t.contains('\n')), "{checked:?}");
}

/// `docs_shapes` with its usage modules as official examples, compiled with the analytics config
/// at `sub` under a temporary directory, module order reversed or not.
async fn docs_shapes_analyzed(sub: &str, reverse: bool) -> (SessionContext, tempfile::TempDir) {
    docs_shapes_embedded(sub, reverse, None, 3).await
}

/// A bag-of-words embedder for tests: each lowercase word adds one to a hashed dimension of 64,
/// then the vector is normalized, so texts sharing words are near (slice 3.1).
struct WordsEmbedder {
    spec: cpg_core::embed::Spec,
}

impl WordsEmbedder {
    fn new() -> Self {
        WordsEmbedder {
            spec: cpg_core::embed::Spec {
                model: "test-words".to_owned(),
                revision: "1".to_owned(),
                tokenizer_revision: "bytes/4".to_owned(),
                server: "in-process".to_owned(),
                served_dtype: "float32".to_owned(),
                pooling: "none".to_owned(),
                query_template: "{query}".to_owned(),
                query_task: "test".to_owned(),
                document_template: "{text}".to_owned(),
                dimensions: 64,
                output_dtype: "float32".to_owned(),
                normalization: "l2".to_owned(),
                max_document_tokens: 2048,
            },
        }
    }

    fn vector(text: &str) -> Vec<f32> {
        let mut v = vec![0f32; 64];
        for word in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
        {
            let h = word
                .to_lowercase()
                .bytes()
                .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
                    (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
                });
            v[(h % 64) as usize] += 1.0;
        }
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm == 0.0 {
            v[0] = 1.0;
            return v;
        }
        v.iter().map(|x| x / norm).collect()
    }
}

impl cpg_core::embed::Embedder for WordsEmbedder {
    fn spec(&self) -> &cpg_core::embed::Spec {
        &self.spec
    }
    fn count_tokens<'a>(
        &'a self,
        request_text: &'a str,
    ) -> cpg_core::embed::EmbedFuture<'a, usize> {
        Box::pin(async move { Ok(request_text.len() / 4) })
    }
    fn embed<'a>(
        &'a self,
        request_texts: &'a [String],
    ) -> cpg_core::embed::EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move { Ok(request_texts.iter().map(|t| Self::vector(t)).collect()) })
    }
}

async fn docs_shapes_embedded(
    sub: &str,
    reverse: bool,
    embedder: Option<std::sync::Arc<dyn cpg_core::embed::Embedder>>,
    budget: u32,
) -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    std::fs::create_dir_all(&base).unwrap();
    let s = Id([7; 16]);
    let mut input = corpus_input(&base, s, &[("usage", "examples")]);
    input.test_hooks.reverse_module_order = reverse;
    let out = extract(&input).unwrap();
    let analysis = cpg_core::analyze::Analysis {
        config: lctx_analytics::config::AnalyticsConfig::parse(&format!(
            r#"
version = 1
[subsystem]
module_prefixes = ["pkg.core"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.make_server", "pkg.Server.stop"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = {budget}
"#,
        ))
        .unwrap(),
        embedder,
        techniques: cpg_core::analyze::Techniques::parse("+communities,+fca,+knn").unwrap(),
    };
    let store = base.join("store");
    cpg_core::attempt::compile_analyzed(&store, s, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    (ctx, dir)
}

/// Pass C and usage patterns (DESIGN §9.3, §10.5) on a corpus. A named handoff with a receiver
/// use before it, and a nested one, are counted; a binding read by two consumers, a reassigned
/// one, a helper the usage code defines, a chained assignment and a binding whose attribute is
/// passed on are not (slice 2.2 review F2, F4). The pattern is the smallest self-contained
/// official example that shows a whole handoff, with its import, and it cites only a handoff it
/// shows (F3). A seed whose only uses sit under a `with` or read an unbound name has no pattern
/// (F1).
#[tokio::test(flavor = "multi_thread")]
async fn handoffs_and_usage_patterns_come_from_official_code() {
    let (ctx, _dir) = docs_shapes_analyzed("one", false).await;
    let handoffs = lines(
        &ctx,
        "SELECT sd.qualified_name, od.qualified_name, CAST(f.score AS BIGINT), m.label \
         FROM findings f JOIN declarations sd ON sd.node_id = f.subject_node_id \
         JOIN declarations od ON od.node_id = f.related_node_id \
         JOIN finding_members m ON m.finding_id = f.finding_id AND m.role = 4 \
         WHERE f.finding_kind = 9 ORDER BY 1, 2",
    )
    .await;
    assert_eq!(
        handoffs,
        "pkg.core.Server.tool | pkg.core.make_server | 2 | fn\n\
         pkg.core.make_server | pkg.core.Server.tool | 2 | fn\n"
    );
    let patterns = lines(
        &ctx,
        "SELECT b.title, a.evidence_status, a.text, count(s.finding_id) \
         FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         LEFT JOIN assertion_support s ON s.assertion_id = a.assertion_id \
         WHERE a.assertion_kind = 9 GROUP BY b.title, a.evidence_status, a.text ORDER BY b.title",
    )
    .await;
    // The smallest pattern that shows a whole handoff: the nested one, with its setup. The
    // producer's pattern is the same, since a pattern cut at a named producer would not show its
    // consumer.
    for seed in ["pkg.Server.tool", "pkg.make_server"] {
        assert!(
            patterns.contains(&format!(
                "{seed} | 1 | From `examples/handoff.py`:\n```python\n\
                 from pkg import make_server\nprimary = make_server(\"primary\")\n\
                 primary.tool(make_server(\"nested\"))\n``` | 1"
            )),
            "{patterns}"
        );
    }
    assert!(!patterns.contains("pkg.Server.stop"), "{patterns}");
    // The community layers (§9.4): the usage code co-uses the subsystem callables, so the co-use
    // layer has their pairs; four vertices give only degenerate partitions, so nothing is chosen
    // or reported.
    let consensus = lines(
        &ctx,
        "SELECT diagnostics FROM analysis_invocations WHERE method = 4",
    )
    .await;
    assert!(consensus.contains("\"chosen_gamma\":null,"), "{consensus}");
    insta::assert_snapshot!("docs_shapes_consensus", consensus);
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());

    // Each rule the review added rejects an injected violation.
    for table in ["assertions", "evidence"] {
        let published = ctx.table(table).await.unwrap();
        ctx.register_table(format!("{table}_published").as_str(), published.into_view())
            .unwrap();
    }
    let rules = cpg_schema::rules::rules();
    for (rule, table, view) in [
        // The pattern's statements moved away from the consumer site it cites.
        (
            "semantic:usage-pattern-shows-its-handoff",
            "evidence",
            "SELECT snapshot_id, evidence_id, evidence_kind, cited_fact_id, node_id, \
                    module_node_id, start_byte + 100000 AS start_byte, \
                    end_byte + 100000 AS end_byte, text FROM evidence_published",
        ),
        (
            "semantic:no-materialized-block-path",
            "assertions",
            "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, \
                    assertion_kind, subject_node_id, applicable_case, evidence_status, \
                    text || ' (_lctx_blocks/d_x/block_0.py)' AS text, conditions, limitations, \
                    template_version FROM assertions_published",
        ),
    ] {
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
}

/// Embeddings in analytics (slice 3.1): E0 embeds the corpus passages and the subsystem's public
/// APIs through the cache; kNN links an API to the passages near it, and the brief says so,
/// `statistically_derived`.
#[tokio::test(flavor = "multi_thread")]
async fn doc_links_come_from_embedding_similarity() {
    let (ctx, _dir) = docs_shapes_embedded(
        "knn",
        false,
        Some(std::sync::Arc::new(WordsEmbedder::new())),
        3,
    )
    .await;
    let knn = lines(
        &ctx,
        "SELECT parameters, candidate_set_size, diagnostics FROM analysis_invocations \
         WHERE method = 8",
    )
    .await;
    insta::assert_snapshot!("knn_invocation", knn);
    let links = lines(
        &ctx,
        "SELECT d.qualified_name, m.label, round(f.score, 3) FROM findings f \
         JOIN declarations d ON d.node_id = f.subject_node_id \
         JOIN finding_members m ON m.finding_id = f.finding_id AND m.role = 15 \
         WHERE f.finding_kind = 15 ORDER BY 1, 3 DESC, 2",
    )
    .await;
    insta::assert_snapshot!("doc_links", links);
    let briefs = lines(
        &ctx,
        "SELECT b.title, a.evidence_status, a.text FROM briefs b \
         JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id \
         WHERE a.assertion_kind = 14 ORDER BY b.title",
    )
    .await;
    assert!(
        briefs.contains("pkg.Server.tool | 2 | Documentation near this operation"),
        "{briefs}"
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}

/// Slice 3.2's layers on a corpus: `+mention-layer` pairs what one passage names exactly, and
/// `+knn-layer` each API with its nearest APIs by embedding; both are recorded in the consensus's
/// diagnostics. The kNN layer without an embedder is refused.
#[tokio::test(flavor = "multi_thread")]
async fn mention_and_knn_layers_come_from_the_corpus() {
    let (ctx, _dir) = docs_shapes_variant(
        "layers",
        Some(std::sync::Arc::new(WordsEmbedder::new())),
        cpg_core::analyze::Techniques::parse("+communities,+mention-layer,+knn-layer").unwrap(),
    )
    .await
    .unwrap();
    let consensus = lines(
        &ctx,
        "SELECT diagnostics FROM analysis_invocations WHERE method = 4",
    )
    .await;
    insta::assert_snapshot!("docs_shapes_layers", consensus);
    assert!(consensus.contains("[\"mention\","), "{consensus}");
    assert!(consensus.contains("[\"knn\","), "{consensus}");
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let refused = docs_shapes_variant(
        "refused",
        None,
        cpg_core::analyze::Techniques::parse("+communities,+knn-layer").unwrap(),
    )
    .await;
    assert!(
        refused
            .err()
            .is_some_and(|e| e.to_string().contains("needs an embedder"))
    );
}

/// `docs_shapes` compiled with an analytics variant (slice 3.2).
async fn docs_shapes_variant(
    sub: &str,
    embedder: Option<std::sync::Arc<dyn cpg_core::embed::Embedder>>,
    techniques: cpg_core::analyze::Techniques,
) -> Result<(SessionContext, tempfile::TempDir), cpg_core::CoreError> {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    std::fs::create_dir_all(&base).unwrap();
    let s = Id([7; 16]);
    let out = extract(&corpus_input(&base, s, &[("usage", "examples")])).unwrap();
    let analysis = cpg_core::analyze::Analysis {
        config: lctx_analytics::config::AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["pkg.core"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.make_server", "pkg.Server.stop"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 3
"#,
        )
        .unwrap(),
        embedder,
        techniques,
    };
    let store = base.join("store");
    cpg_core::attempt::compile_analyzed(&store, s, &out.tables, Some(&analysis)).await?;
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    Ok((ctx, dir))
}

/// Direct usage and seed selection (DESIGN §9.5; the increment-2 review's U1 and F6(a)): each public
/// API the official usage code calls is counted, one per definite call site; with room in the
/// budget, the most called eligible API (one with a docstring summary) becomes a seed, named by its
/// preferred path, with its brief.
#[tokio::test(flavor = "multi_thread")]
async fn selection_takes_what_usage_calls_within_the_budget() {
    let (ctx, _dir) = docs_shapes_embedded("select", false, None, 4).await;
    let counts = lines(
        &ctx,
        "SELECT d.qualified_name, CAST(f.score AS BIGINT), f.evidence_status FROM findings f \
         JOIN declarations d ON d.node_id = f.subject_node_id \
         WHERE f.finding_kind = 17 ORDER BY 1",
    )
    .await;
    insta::assert_snapshot!("direct_usage", counts);
    let selection = lines(
        &ctx,
        "SELECT parameters, candidate_set_size, diagnostics FROM analysis_invocations \
         WHERE method = 7",
    )
    .await;
    insta::assert_snapshot!("docs_shapes_selection", selection);
    assert!(
        selection.contains("\"selected\":[\"pkg.Server.run\"]"),
        "{selection}"
    );
    let briefs = lines(&ctx, "SELECT title FROM briefs ORDER BY title").await;
    assert!(briefs.contains("pkg.Server.run\n"), "{briefs}");
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}

/// Slice 2.2 review F6: Pass C, the usage patterns and the rest of Stage E and F are identical
/// across the corpus's location and module order.
#[tokio::test(flavor = "multi_thread")]
async fn pass_c_and_usage_patterns_are_identical_across_location_and_module_order() {
    let (a, _da) = docs_shapes_analyzed("one", false).await;
    let (b, _db) = docs_shapes_analyzed("elsewhere/deeper", true).await;
    for table in [
        "findings",
        "finding_members",
        "witnesses",
        "analysis_invocations",
        "assertions",
        "assertion_support",
        "evidence",
        "briefs",
    ] {
        let q = format!("SELECT * EXCLUDE (snapshot_id) FROM {table} ORDER BY 1, 2, 3, 4");
        assert_eq!(lines(&a, &q).await, lines(&b, &q).await, "{table}");
    }
}
