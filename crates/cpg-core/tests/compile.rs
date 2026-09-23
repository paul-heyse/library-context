//! Slice 2: a compile attempt writes the raw tables, derives Stage C/D, validates and publishes
//! (DESIGN §4.1, §6, §8; ADR-0009).

use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch};
use cpg_core::CoreError;
use cpg_core::attempt::{compile, publish};
use cpg_core::delta::read_at;
use cpg_core::snapshot::{published, resolve};
use cpg_core::sql;
use cpg_extract::{ExtractInput, extract};
use cpg_schema::codebook::Codebook;
use cpg_schema::id::Id;
use cpg_schema::table::Table;
use cpg_schema::tables::{Declarations, SnapshotsRow};
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

/// The extractor's raw batches for a fixture, under `snapshot_id`.
fn raw(fixture: &str, snapshot_id: Id) -> Vec<(&'static str, RecordBatch)> {
    let dir = tempfile::tempdir().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let release = dir.path().join("release");
    copy(&repo.join("fixtures/python").join(fixture), &release);
    let site = dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(std::fs::canonicalize(&release).unwrap(), fixture)
            .unwrap(),
        venv_root: std::fs::canonicalize(dir.path().join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(&site).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap()
    .tables
}

async fn text(ctx: &SessionContext, statement: &str) -> String {
    let batches = sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    pretty_format_batches(&batches).unwrap().to_string()
}

async fn count(ctx: &SessionContext, statement: &str) -> i64 {
    let batches = sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let a = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<arrow_array::Int64Array>()
        .unwrap();
    a.value(0)
}

#[tokio::test]
async fn an_attempt_publishes_every_table_and_readers_see_only_published_rows() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (Id([1; 16]), Id([2; 16]));
    let raw_a = raw("pysa_variants", a);
    let out = compile(root.path(), a, &raw_a).await.unwrap();
    let versions = resolve(root.path(), a).await.unwrap().expect("published");
    assert_eq!(versions, out.versions);
    assert_eq!(versions.len(), 36 + 22, "every raw and derived table");

    // A second attempt fails validation after writing its rows: it publishes nothing.
    let mut raw_b = raw("pysa_variants", b);
    drop_rows(&mut raw_b, "coverage", 1);
    assert!(matches!(
        compile(root.path(), b, &raw_b).await,
        Err(CoreError::Invalid(_))
    ));
    assert!(resolve(root.path(), b).await.unwrap().is_none());

    // A's reader still sees exactly A's rows, at A's versions.
    let declarations = raw_a.iter().find(|(n, _)| *n == "declarations").unwrap();
    let back = read_at::<Declarations>(root.path(), versions["declarations"], a)
        .await
        .unwrap();
    assert_eq!(&back, &declarations.1);
    let (_, ctx) = published(root.path(), a).await.unwrap().unwrap();
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM declarations").await,
        declarations.1.num_rows() as i64
    );
}

/// Derived tables rendered by name, for review (the ids are opaque). The temp dir holds the tables
/// the session reads.
async fn derived_text(fixture: &str) -> (String, SessionContext, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let s = Id([3; 16]);
    compile(root.path(), s, &raw(fixture, s)).await.unwrap();
    let (_, ctx) = published(root.path(), s).await.unwrap().unwrap();
    let mut out = String::new();
    for (title, statement) in [
        (
            "exports",
            "SELECT e.access_path, d.qualified_name AS declaration, d.is_overload \
             FROM exports e LEFT JOIN declarations d ON d.node_id = e.declaration_node_id \
             ORDER BY e.access_path, declaration NULLS FIRST",
        ),
        (
            "signatures",
            "SELECT d.qualified_name AS def, d.start_byte, c.start_byte AS callable_start, \
                    s.function_key, s.signature_index, s.form, s.reason \
             FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
             JOIN declarations c ON c.node_id = s.callable_node_id \
             ORDER BY d.start_byte, def",
        ),
        (
            "parameters",
            "SELECT d.qualified_name AS def, d.start_byte, p.ordinal, x.name AS syntax, \
                    q.name AS semantics, q.required, p.reason \
             FROM parameters p JOIN declarations d ON d.node_id = p.signature_node_id \
             LEFT JOIN parameter_syntax x ON x.fact_id = p.syntax_fact_id \
             LEFT JOIN parameter_semantics q ON q.fact_id = p.semantics_fact_id \
             ORDER BY d.start_byte, p.ordinal",
        ),
        (
            "resolutions",
            "SELECT c.start_byte, c.end_byte, r.status, r.has_unresolved_remainder AS remainder, \
                    r.candidate_set_complete_under_model AS complete, r.unresolved_reason, \
                    r.target_count, r.reason \
             FROM resolutions r JOIN call_syntax c ON c.node_id = r.call_site_node_id \
             ORDER BY c.start_byte, c.end_byte",
        ),
        (
            "call_targets",
            "SELECT c.start_byte, c.end_byte, p.phase, p.higher_order_index AS ho, \
                    p.target_module, p.target_name, \
                    COALESCE(d.qualified_name, x.qualified_name) AS target, n.node_kind AS kind, \
                    f.modality, t.reason \
             FROM call_targets t JOIN call_syntax c ON c.node_id = t.call_site_node_id \
             JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
             JOIN facts f ON f.fact_id = t.pysa_fact_id \
             LEFT JOIN declarations d ON d.node_id = t.target_node_id \
             LEFT JOIN context_definitions x ON x.symbol_node_id = t.target_node_id \
             LEFT JOIN nodes n ON n.node_id = t.target_node_id \
             ORDER BY c.start_byte, c.end_byte, p.phase, ho NULLS FIRST, p.target_name",
        ),
        (
            "nodes",
            "SELECT node_kind, count(*) AS n FROM nodes GROUP BY node_kind ORDER BY node_kind",
        ),
        (
            "edges",
            "SELECT edge_kind, src_kind, dst_kind, count(*) AS n FROM edges \
             GROUP BY edge_kind, src_kind, dst_kind ORDER BY edge_kind, src_kind, dst_kind",
        ),
    ] {
        out.push_str(&format!("## {title}\n{}\n", text(&ctx, statement).await));
    }
    (out, ctx, root)
}

#[tokio::test]
async fn derived_tables_on_the_variant_fixture() {
    let (out, ctx, _root) = derived_text("pysa_variants").await;
    insta::assert_snapshot!(out);
    // Resolutions with no Pysa record agree with the extractor's call boundaries, reason by reason.
    for reason in [7, 10] {
        let resolutions = count(
            &ctx,
            &format!("SELECT count(*) FROM resolutions WHERE reason = {reason}"),
        )
        .await;
        let boundaries = count(
            &ctx,
            &format!("SELECT count(*) FROM boundaries WHERE fact_family = 3 AND reason = {reason}"),
        )
        .await;
        assert_eq!(resolutions, boundaries, "reason {reason}");
    }
}

#[tokio::test]
async fn derived_tables_on_the_unicode_fixture() {
    let (out, ctx, _root) = derived_text("unicode_bom").await;
    insta::assert_snapshot!(out);
    // `lcfix.build` re-exports the implementation, not an `@overload` stub; the two stubs carry
    // Pysa's two signatures in source order and the implementation none.
    let seed = text(
        &ctx,
        "SELECT d.qualified_name, d.is_overload FROM exports e \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         WHERE e.access_path = 'lcfix.build'",
    )
    .await;
    assert!(
        seed.contains("lcfix.core.build") && seed.contains("false"),
        "{seed}"
    );
    let indexes = text(
        &ctx,
        "SELECT d.is_overload, s.signature_index FROM signatures s \
         JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name = 'lcfix.core.build' ORDER BY d.start_byte",
    )
    .await;
    let lines: Vec<&str> = indexes
        .lines()
        .filter(|l| l.starts_with("| "))
        .skip(1)
        .collect();
    assert_eq!(
        lines
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>(),
        ["| true | 0 |", "| true | 1 |", "| false | |"]
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM signatures WHERE reason IS NOT NULL"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn derived_tables_on_the_keys_fixture() {
    let (out, ctx, _root) = derived_text("pysa_keys").await;
    insta::assert_snapshot!(out);
    // From outside, `dual.g()` targets the stub's `g`; inside `dual.py`, `f(1)` targets the
    // source's `f` (review F3).
    let targets = text(
        &ctx,
        "SELECT f.path AS caller, g.path AS target FROM call_targets t \
         JOIN call_syntax c ON c.node_id = t.call_site_node_id \
         JOIN source_files f ON f.module_node_id = c.module_node_id \
         JOIN declarations d ON d.node_id = t.target_node_id \
         JOIN source_files g ON g.module_node_id = d.module_node_id \
         ORDER BY caller, target",
    )
    .await;
    assert!(
        targets.contains("keys/use_dual.py | keys/dual.pyi"),
        "{targets}"
    );
    assert!(
        targets.contains("keys/dual.py     | keys/dual.py"),
        "{targets}"
    );
    // Slice-2 review F5: one `exports` row per file of a `.py`/`.pyi` pair, each seeding the
    // declaration in its own file.
    let seeds = text(
        &ctx,
        "SELECT e.access_path, o.path AS origin, g.path AS seed FROM exports e \
         JOIN public_names p ON p.fact_id = e.public_fact_id \
         JOIN source_files o ON o.module_node_id = p.origin_module_node_id \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         JOIN source_files g ON g.module_node_id = d.module_node_id \
         WHERE e.access_path = 'keys.dual.f' ORDER BY origin",
    )
    .await;
    let rows: Vec<String> = seeds
        .lines()
        .filter(|l| l.starts_with("| keys"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    assert_eq!(
        rows,
        [
            "| keys.dual.f | keys/dual.py | keys/dual.py |",
            "| keys.dual.f | keys/dual.pyi | keys/dual.pyi |"
        ]
    );
}

#[tokio::test]
async fn derived_tables_on_the_derive_cases_fixture() {
    let (out, ctx, _root) = derived_text("derive_cases").await;
    insta::assert_snapshot!(out);
    // F1, then ADR-0014: constructing a release dataclass calls its synthesized `__init__`, which
    // has no declaration: the target is a typed `synthetic_callable` node, with no reason left.
    let init = text(
        &ctx,
        "SELECT n.node_kind, t.reason FROM call_targets t \
         JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
         JOIN nodes n ON n.node_id = t.target_node_id \
         WHERE p.target_name = '__init__' AND p.target_module = '@dc/impl.py'",
    )
    .await;
    assert!(init.contains("| 9         |"), "synthetic_callable: {init}");
    // F2: the public `load` is the definition Pyrefly binds under Python 3.14 (the `if` branch);
    // the `else` branch is unreachable in the context, not missing evidence.
    let load = text(
        &ctx,
        "SELECT d.start_byte, s.reason FROM exports e \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         JOIN signatures s ON s.signature_node_id = d.node_id \
         WHERE e.access_path = 'dc.load'",
    )
    .await;
    let live = count(
        &ctx,
        "SELECT min(start_byte) FROM declarations WHERE qualified_name = 'dc.compat.load'",
    )
    .await;
    assert!(load.contains(&format!("| {live} ")), "{load}");
    let dead = count(
        &ctx,
        "SELECT count(*) FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name IN ('dc.compat.load', 'dc.ov.f') AND s.reason = 14",
    )
    .await;
    assert_eq!(dead, 1 + 2, "the dead `load` and the two dead `f` stubs");
    let placed = count(
        &ctx,
        "SELECT count(*) FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name = 'dc.ov.f' AND s.signature_index IS NOT NULL",
    )
    .await;
    assert_eq!(placed, 2, "the live stubs carry Pysa's two signatures");
}

fn replace(batch: &RecordBatch, column: &str, array: ArrayRef) -> RecordBatch {
    let i = batch.schema().index_of(column).unwrap();
    let mut columns = batch.columns().to_vec();
    columns[i] = array;
    RecordBatch::try_new(batch.schema(), columns).unwrap()
}

fn table<'a>(raw: &'a mut [(&'static str, RecordBatch)], name: &str) -> &'a mut RecordBatch {
    &mut raw.iter_mut().find(|(n, _)| *n == name).unwrap().1
}

fn drop_rows(raw: &mut [(&'static str, RecordBatch)], name: &str, n: usize) {
    let b = table(raw, name);
    *b = b.slice(n, b.num_rows() - n);
}

/// Keep the rows of `name` whose `column` (as text) satisfies `keep`.
fn keep_rows(
    raw: &mut [(&'static str, RecordBatch)],
    name: &str,
    column: &str,
    keep: impl Fn(&str) -> bool,
) {
    let b = table(raw, name);
    let values = arrow_cast::cast(
        b.column(b.schema().index_of(column).unwrap()),
        &arrow_schema::DataType::Utf8,
    )
    .unwrap();
    let values = values
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap();
    let mask: arrow_array::BooleanArray =
        values.iter().map(|v| Some(keep(v.unwrap_or("")))).collect();
    *b = arrow_select::filter::filter_record_batch(b, &mask).unwrap();
}

/// Each rule kind rejects a snapshot that breaks it, and nothing is published (DM-53).
#[tokio::test]
async fn every_rule_kind_rejects_its_violation() {
    let s = Id([4; 16]);
    let base = raw("pysa_variants", s);
    type Mutation = fn(&mut Vec<(&'static str, RecordBatch)>);
    let cases: [(&str, Mutation); 23] = [
        ("key:declarations", |raw| {
            let b = table(raw, "declarations");
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &b.slice(0, 1)]).unwrap();
        }),
        ("ref:call_syntax.owner_node_id", |raw| {
            let b = table(raw, "call_syntax");
            let owners =
                FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n([9u8; 16], b.num_rows()))
                    .unwrap();
            *b = replace(b, "owner_node_id", Arc::new(owners));
        }),
        ("fact:", |raw| drop_rows(raw, "facts", 1)),
        ("codebook:boundaries.reason", |raw| {
            let b = table(raw, "boundaries");
            let reasons = Int16Array::from(vec![99_i16; b.num_rows()]);
            *b = replace(b, "reason", Arc::new(reasons));
        }),
        ("coverage:complete", |raw| drop_rows(raw, "coverage", 1)),
        ("fact-payload:declarations", |raw| {
            drop_rows(raw, "declarations", 1)
        }),
        ("coverage:declared-family", |raw| {
            let b = table(raw, "runs");
            let field = match b.schema().field_with_name("families").unwrap().data_type() {
                arrow_schema::DataType::List(f) => f.clone(),
                other => panic!("{other}"),
            };
            let mut list =
                arrow_array::builder::ListBuilder::new(arrow_array::builder::StringBuilder::new())
                    .with_field(field);
            for _ in 0..b.num_rows() {
                list.values().append_value("bogus");
                list.append(true);
            }
            *b = replace(b, "families", Arc::new(list.finish()));
        }),
        // Slice-2 review F3: a `resolutions` reason with no matching call boundary.
        ("semantic:resolution-has-boundary", |raw| {
            keep_rows(raw, "boundaries", "fact_family", |f| f != "3");
            keep_rows(raw, "facts", "table_name", |t| t != "boundaries");
        }),
        // Slice-2 F1, then ADR-0014 review F1: a release target that names nothing publishes
        // with no reason, and the typed rule rejects it (no catch-all reason hides it).
        ("typed:call_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let i = b.schema().index_of("target_module").unwrap();
            let modules = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            let moved: arrow_array::StringArray = modules
                .iter()
                .map(|m| m.map(|m| if m.starts_with('@') { "@nowhere.py" } else { m }))
                .collect();
            *b = replace(b, "target_module", Arc::new(moved));
        }),
        // Review F1: a dependency target whose definition is missing is our failure, too.
        ("typed:call_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let (m, k) = (
                b.schema().index_of("target_module").unwrap(),
                b.schema().index_of("target_key").unwrap(),
            );
            let modules = b
                .column(m)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .clone();
            let keys: arrow_array::StringArray = b
                .column(k)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .zip(modules.iter())
                .map(|(key, module)| match (key, module) {
                    (Some(_), Some(m)) if !m.starts_with('@') => Some("F:999999"),
                    (key, _) => key,
                })
                .collect();
            *b = replace(b, "target_key", Arc::new(keys));
        }),
        // Review O2: two rows asserting one argument id are a collision, never merged.
        ("key:nodes", |raw| {
            let b = table(raw, "arguments");
            let dup = b.slice(0, 1);
            let fact = FixedSizeBinaryArray::try_from_iter(std::iter::once([7u8; 16])).unwrap();
            let dup = replace(&dup, "fact_id", Arc::new(fact));
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &dup]).unwrap();
        }),
        // C2 review F3: a declaration without its placement row.
        ("placed:declarations", |raw| {
            let b = table(raw, "syntax_nodes");
            *b = b.slice(0, 0);
        }),
        // C2 review F3: a `def` placed on a span its body does not fit in.
        ("contained:syntax_nodes", |raw| {
            let b = table(raw, "syntax_nodes");
            let def = cpg_schema::codebook::SyntaxKind::StmtFunctionDef.code();
            let kinds = b
                .column(b.schema().index_of("kind").unwrap())
                .as_any()
                .downcast_ref::<Int16Array>()
                .unwrap()
                .clone();
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .zip(kinds.iter())
                    .map(|(v, k)| if k == Some(def) { Some(0) } else { v })
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C2 review F2/F3: a Pysa site no node sits at is our failure, never a catch-all reason.
        ("typed:site_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let sites = b
                .column(b.schema().index_of("site_kind").unwrap())
                .as_any()
                .downcast_ref::<Int16Array>()
                .unwrap()
                .clone();
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .zip(sites.iter())
                    .map(|(v, k)| {
                        if k == Some(0) {
                            v
                        } else {
                            v.map(|v| v + 1_000_000)
                        }
                    })
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C2 review F3: one node placed twice.
        ("unique:syntax_nodes", |raw| {
            let b = table(raw, "syntax_nodes");
            let dup = b.slice(0, 1);
            let fact = FixedSizeBinaryArray::try_from_iter(std::iter::once([8u8; 16])).unwrap();
            let dup = replace(&dup, "fact_id", Arc::new(fact));
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &dup]).unwrap();
        }),
        // C3: a resolution with no binding, no builtin and no reason.
        ("typed:reference_resolutions", |raw| {
            let b = table(raw, "reference_resolutions");
            let none: FixedSizeBinaryArray = FixedSizeBinaryArray::try_from_sparse_iter_with_size(
                std::iter::repeat_n(None::<[u8; 16]>, b.num_rows()),
                16,
            )
            .unwrap();
            let no_text = arrow_array::StringArray::from(vec![None::<&str>; b.num_rows()]);
            let no_reason = Int16Array::from(vec![None::<i16>; b.num_rows()]);
            *b = replace(b, "binding_id", Arc::new(none));
            *b = replace(b, "builtin_name", Arc::new(no_text));
            *b = replace(b, "reason", Arc::new(no_reason));
        }),
        // C3: a binding whose id is not its recipe.
        ("id:bindings", |raw| {
            let b = table(raw, "bindings");
            let names: arrow_array::StringArray = b
                .column(b.schema().index_of("name").unwrap())
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|n| n.map(|n| format!("{n}_renamed")))
                .collect();
            *b = replace(b, "name", Arc::new(names));
        }),
        // C3 review F7: a scope whose id is not its owner's recipe.
        ("id:scopes", |raw| {
            let b = table(raw, "scopes");
            let module = b
                .column(b.schema().index_of("module_node_id").unwrap())
                .clone();
            *b = replace(b, "owner_node_id", module);
        }),
        // C3 review F7: a reference whose id is not its name's recipe.
        ("id:references", |raw| {
            let b = table(raw, "references");
            let names = b
                .column(b.schema().index_of("name_node_id").unwrap())
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
                .clone();
            let first = FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(
                names.value(0),
                b.num_rows(),
            ))
            .unwrap();
            *b = replace(b, "name_node_id", Arc::new(first));
        }),
        // C3 review F7: a Pysa identifier site with no reference at its span is our failure.
        ("typed:identifier_targets", |raw| {
            let b = table(raw, "references");
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C3 review F2 (P10): an import our own naming got wrong is never read as the provider's
        // "not found".
        ("typed:import_targets", |raw| {
            let b = table(raw, "export_syntax");
            let i = b.schema().index_of("resolved_module").unwrap();
            let moved: arrow_array::StringArray = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|m| m.map(|m| format!("{m}_misnamed")))
                .collect();
            *b = replace(b, "resolved_module", Arc::new(moved));
        }),
        // ADR-0014 lineage: a Pysa call record that matches no call site is not silently dropped.
        ("lineage:call_target", |raw| {
            let b = table(raw, "pysa_calls");
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // ADR-0014 endpoint kinds: a declaration whose parent is a call site.
        ("endpoint:declares", |raw| {
            let call = {
                let c = table(raw, "call_syntax");
                let ids = c
                    .column(c.schema().index_of("node_id").unwrap())
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .unwrap();
                ids.value(0).to_vec()
            };
            let b = table(raw, "declarations");
            let parents = FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(
                call.as_slice(),
                b.num_rows(),
            ))
            .unwrap();
            *b = replace(b, "parent_node_id", Arc::new(parents));
        }),
    ];
    for (rule, mutate) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut raw = base.clone();
        mutate(&mut raw);
        match compile(root.path(), s, &raw).await {
            Err(CoreError::Invalid(violations)) => assert!(
                violations.iter().any(|v| v.rule.starts_with(rule)),
                "{rule}: {violations:?}"
            ),
            other => panic!("{rule}: expected a validation failure, got {other:?}"),
        }
        assert!(resolve(root.path(), s).await.unwrap().is_none(), "{rule}");
    }
    // The unmutated snapshot passes every rule.
    let root = tempfile::tempdir().unwrap();
    compile(root.path(), s, &base).await.unwrap();
}

/// The C4 rules reject their violations on `type_shapes`, which has records and type variables
/// (C4 review F1, F3, and the `id:record_fields` case it asked for).
#[tokio::test]
async fn the_types_rules_reject_their_violations() {
    let s = Id([9; 16]);
    let base = raw("type_shapes", s);
    type Mutation = fn(&mut Vec<(&'static str, RecordBatch)>);
    let cases: [(&str, Mutation); 3] = [
        // A class reference nothing defines is our failure, never a catch-all reason.
        ("typed:type_class_targets", |raw| {
            let b = table(raw, "type_terms");
            let i = b.schema().index_of("class_key").unwrap();
            let moved: arrow_array::StringArray = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|k| k.map(|_| "999999"))
                .collect();
            *b = replace(b, "class_key", Arc::new(moved));
        }),
        // An anchor inside a release module that nothing holds is our failure too.
        ("typed:type_binders", |raw| {
            let b = table(raw, "type_terms");
            for column in ["anchor_start", "anchor_end"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        ("id:record_fields", |raw| {
            let b = table(raw, "record_fields");
            let names: arrow_array::StringArray = b
                .column(b.schema().index_of("name").unwrap())
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|n| n.map(|n| format!("{n}_renamed")))
                .collect();
            *b = replace(b, "name", Arc::new(names));
        }),
    ];
    for (rule, mutate) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut raw = base.clone();
        mutate(&mut raw);
        match compile(root.path(), s, &raw).await {
            Err(CoreError::Invalid(violations)) => assert!(
                violations.iter().any(|v| v.rule.starts_with(rule)),
                "{rule}: {violations:?}"
            ),
            other => panic!("{rule}: expected a validation failure, got {other:?}"),
        }
    }
    let root = tempfile::tempdir().unwrap();
    compile(root.path(), s, &base).await.unwrap();
}

#[tokio::test]
async fn a_failed_snapshots_append_is_classified_by_rereading() {
    // ADR-0009 P3 in-repo: the append is rejected (a CHECK), and re-reading `snapshots` shows the
    // attempt unpublished. A published attempt reads back as published.
    let root = tempfile::tempdir().unwrap();
    let s = Id([5; 16]);
    compile(root.path(), s, &raw("unicode_bom", s))
        .await
        .unwrap();
    let bad = SnapshotsRow {
        snapshot_id: Id([6; 16]),
        content_digest: cpg_schema::id::Digest([0; 32]),
        table_name: "declarations".to_owned(),
        table_version: -1,
        schema_digest: cpg_schema::id::Digest([0; 32]),
        compiler_digest: cpg_schema::id::Digest([0; 32]),
        row_count: 0,
    };
    assert!(matches!(
        publish(root.path(), Id([6; 16]), &[bad]).await,
        Err(CoreError::Unpublished(_))
    ));
    assert!(resolve(root.path(), Id([6; 16])).await.unwrap().is_none());
    assert!(resolve(root.path(), s).await.unwrap().is_some());
    assert_eq!(Declarations::NAME, "declarations");
}
