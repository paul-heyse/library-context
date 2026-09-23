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
#[tokio::test]
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
             ORDER BY subject, role"
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
         JOIN type_terms ct ON ct.node_id = a.child_node_id ORDER BY 1, 2, 3",
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

/// C5 (DESIGN §3.2 `docs`): a corpus run beside the library run. The documents the selection
/// keeps, their passages, code blocks and links, the mentions of the library's API by class, and
/// each document's coverage.
#[tokio::test]
async fn a_corpus_documents_its_library() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join("fixtures/python/docs_shapes");
    // The library installed as an acquired one is: its files in site-packages, owned by its
    // distribution's `RECORD`, so the usage run reaches it as a dependency (C5b).
    copy(
        &fixture.join("release"),
        &dir.path().join("venv/site-packages"),
    );
    copy(&fixture.join("corpus"), &dir.path().join("corpus"));
    let site = std::fs::canonicalize(dir.path().join("venv/site-packages")).unwrap();
    let s = Id([7; 16]);
    let files = vec![site.join("pkg/__init__.py"), site.join("pkg/core.py")];
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
            files,
            release_id: Id([8; 16]),
            origin: cpg_extract::ReleaseOrigin::Library(library),
        },
        venv_root: std::fs::canonicalize(dir.path().join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(dir.path().join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    };
    let source = cpg_extract::library::Source {
        repository: "https://example.invalid/pkg".to_owned(),
        tag: "v1".to_owned(),
        commit: "0".repeat(40),
        documents: vec!["docs/**/*.mdx".to_owned()],
        documents_exclude: vec!["docs/old/**".to_owned()],
        usage: vec!["tests/**/*.py".to_owned()],
    };
    let tree = std::fs::canonicalize(dir.path().join("corpus")).unwrap();
    input.corpus = Some(cpg_extract::library::corpus(&tree, &source, &input).unwrap());
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
    text += "## usage targets: dependency definition | release declaration | reason\n";
    text += &lines(
        &ctx,
        "SELECT m.module_name || ':' || x.qualified_name, d.qualified_name, \
                CAST(u.reason AS VARCHAR) \
         FROM usage_targets u JOIN context_definitions x ON x.symbol_node_id = u.symbol_node_id \
         JOIN context_modules m ON m.module_node_id = x.module_node_id \
         LEFT JOIN declarations d ON d.node_id = u.target_node_id ORDER BY 1",
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
}
