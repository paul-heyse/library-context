//! Cross-table validation (DESIGN §8): one DataFusion query per rule, generated from the
//! contracts. A rule passes when its query returns no rows. The rules are:
//! - `key`: every table's declared total key is unique in the snapshot;
//! - `ref`: every reference below names an existing row (null passes);
//! - `fact`: every raw row has its `facts` row, and every `facts` row has its raw row;
//! - `codebook`: every `Int16` codebook column holds a code of its codebook;
//! - `coverage`: every family a run declares has a row for every module of its release;
//! - `semantic`: hand-written rules no declaration generates (listed in [`semantic`]).
//!
//! The queries read tables registered under their own names and filtered to one snapshot.

use crate::codebook::{Codebook, FactFamily, registry};
use crate::column::CODEBOOK_KEY;
use crate::table::Table;

/// One validation query and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    pub sql: String,
}

/// A column whose non-null values must name a row of one of `to`.
pub struct Reference {
    pub table: &'static str,
    pub column: &'static str,
    pub to: &'static [(&'static str, &'static str)],
}

const MODULE: &[(&str, &str)] = &[("source_files", "module_node_id")];
const DECLARATION: &[(&str, &str)] = &[("declarations", "node_id")];
const CALL: &[(&str, &str)] = &[("call_syntax", "node_id")];
const FACT: &[(&str, &str)] = &[("facts", "fact_id")];

const fn r(
    table: &'static str,
    column: &'static str,
    to: &'static [(&'static str, &'static str)],
) -> Reference {
    Reference { table, column, to }
}

/// The declared references of every stored table (keys and fact links are generated).
pub const REFERENCES: &[Reference] = &[
    r("facts", "run_id", &[("runs", "run_id")]),
    r("runs", "release_id", &[("releases", "release_id")]),
    // A run's release has modules, so `coverage:complete` cannot pass vacuously.
    r("runs", "release_id", &[("source_files", "release_id")]),
    r("releases", "release_id", &[("runs", "release_id")]),
    r("distributions", "release_id", &[("releases", "release_id")]),
    r("source_files", "release_id", &[("runs", "release_id")]),
    r("runs", "context_id", &[("contexts", "context_id")]),
    r("runs", "producer_id", &[("producers", "producer_id")]),
    r("declarations", "module_node_id", MODULE),
    r("declarations", "parent_node_id", DECLARATION),
    r("export_syntax", "module_node_id", MODULE),
    r("public_names", "origin_module_node_id", MODULE),
    r("parameter_syntax", "function_node_id", DECLARATION),
    r("pysa_functions", "module_node_id", MODULE),
    r("parameter_semantics", "module_node_id", MODULE),
    r("class_ancestry", "module_node_id", MODULE),
    r("call_syntax", "module_node_id", MODULE),
    r("call_syntax", "owner_node_id", DECLARATION),
    r("arguments", "call_node_id", CALL),
    r("pysa_calls", "module_node_id", MODULE),
    r("coverage", "run_id", &[("runs", "run_id")]),
    r("coverage", "scope_node_id", MODULE),
    r("boundaries", "module_node_id", MODULE),
    r(
        "boundaries",
        "subject_node_id",
        &[("declarations", "node_id"), ("call_syntax", "node_id")],
    ),
    r("provider_node_map", "module_node_id", MODULE),
    r("provider_node_map", "node_id", DECLARATION),
    r("provider_node_map", "pysa_fact_id", FACT),
    r("provider_node_map", "declaration_fact_id", FACT),
    r("exports", "declaration_node_id", DECLARATION),
    r("exports", "public_fact_id", FACT),
    r("exports", "declaration_fact_id", FACT),
    r("signatures", "signature_node_id", DECLARATION),
    r("signatures", "callable_node_id", DECLARATION),
    r("signatures", "module_node_id", MODULE),
    r("signatures", "declaration_fact_id", FACT),
    r(
        "parameters",
        "signature_node_id",
        &[("signatures", "signature_node_id")],
    ),
    r("parameters", "syntax_fact_id", FACT),
    r("parameters", "semantics_fact_id", FACT),
    r("resolutions", "call_site_node_id", CALL),
    r("resolutions", "call_fact_id", FACT),
    r(
        "call_targets",
        "call_site_node_id",
        &[("resolutions", "call_site_node_id")],
    ),
    r("call_targets", "pysa_fact_id", FACT),
    r("call_targets", "target_node_id", DECLARATION),
];

/// Name, key and schema of a stored table.
struct Shape {
    name: &'static str,
    key: &'static [&'static str],
    schema: arrow_schema::SchemaRef,
}

fn shapes() -> Vec<Shape> {
    macro_rules! all {
        ($($t:ty),+) => {
            vec![$(Shape {
                name: <$t as Table>::NAME,
                key: <$t as Table>::key(),
                schema: <$t as Table>::schema(),
            }),+]
        };
    }
    let mut out = crate::for_each_table!(all);
    out.extend(crate::for_each_derived_table!(all));
    out
}

fn quoted(values: impl IntoIterator<Item = impl std::fmt::Display>) -> String {
    values
        .into_iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rules that span tables in ways no declaration captures.
fn semantic() -> Vec<Rule> {
    let calls = FactFamily::Calls.code();
    [
        (
            // Slice-2 review F1: a target in the release either names a declaration or says why not.
            "semantic:release-target-explained",
            "SELECT t.pysa_fact_id FROM call_targets t              JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id              WHERE p.target_module LIKE '@%' AND t.target_node_id IS NULL AND t.reason IS NULL"
                .to_owned(),
        ),
        (
            // F2: every Pysa function with signatures is some signature row's callable.
            "semantic:pysa-signatures-placed",
            "SELECT m.module_node_id, m.function_key FROM provider_node_map m              JOIN pysa_functions f                ON f.module_node_id = m.module_node_id AND f.function_key = m.function_key              LEFT ANTI JOIN signatures s                ON s.module_node_id = m.module_node_id AND s.function_key = m.function_key              WHERE m.node_id IS NOT NULL AND f.signature_count > 0"
                .to_owned(),
        ),
        (
            // F3: the derived "no Pysa record" reason and the extractor's call boundary agree, per
            // call site, in both directions.
            "semantic:resolution-has-boundary",
            format!(
                "SELECT r.call_site_node_id FROM resolutions r LEFT ANTI JOIN boundaries b                    ON b.subject_node_id = r.call_site_node_id AND b.fact_family = {calls}                   AND b.reason = r.reason                  WHERE r.reason IS NOT NULL"
            ),
        ),
        (
            "semantic:boundary-has-resolution",
            format!(
                "SELECT b.subject_node_id FROM boundaries b LEFT ANTI JOIN resolutions r                    ON r.call_site_node_id = b.subject_node_id AND r.reason = b.reason                  WHERE b.fact_family = {calls} AND b.subject_node_id IS NOT NULL"
            ),
        ),
        (
            // O4: Stage C maps at most one Pysa key to a declaration.
            "semantic:stage-c-injective",
            "SELECT node_id, count(*) AS n FROM provider_node_map WHERE node_id IS NOT NULL              GROUP BY node_id HAVING count(*) > 1"
                .to_owned(),
        ),
        (
            // O2: a provider-local composite reference.
            "semantic:parameter-semantics-function",
            "SELECT q.module_node_id, q.function_key FROM parameter_semantics q              LEFT ANTI JOIN pysa_functions f                ON f.module_node_id = q.module_node_id AND f.function_key = q.function_key"
                .to_owned(),
        ),
        (
            // O2: `model_id` is `<producer_id hex>/<surface>` of the fact's own run (§3.5).
            "semantic:model-id-producer",
            "SELECT f.fact_id FROM facts f JOIN runs r ON r.run_id = f.run_id              WHERE split_part(f.model_id, '/', 1) <> encode(r.producer_id, 'hex')"
                .to_owned(),
        ),
    ]
    .into_iter()
    .map(|(name, sql)| Rule {
        name: name.to_owned(),
        sql,
    })
    .collect()
}

/// Every rule, in a fixed order: keys, references, fact links, codebooks, coverage, semantic.
pub fn rules() -> Vec<Rule> {
    let shapes = shapes();
    let mut out = Vec::new();
    for s in &shapes {
        let key = s.key.join(", ");
        out.push(Rule {
            name: format!("key:{}", s.name),
            sql: format!(
                "SELECT {key}, count(*) AS n FROM {} GROUP BY {key} HAVING count(*) > 1",
                s.name
            ),
        });
    }
    for r in REFERENCES {
        let targets =
            r.to.iter()
                .map(|(t, c)| format!("SELECT {c} AS k FROM {t}"))
                .collect::<Vec<_>>()
                .join(" UNION ALL ");
        out.push(Rule {
            name: format!(
                "ref:{}.{}->{}",
                r.table,
                r.column,
                r.to.iter().map(|(t, _)| *t).collect::<Vec<_>>().join("|")
            ),
            sql: format!(
                "SELECT f.{c} AS value FROM {t} f LEFT ANTI JOIN ({targets}) r ON f.{c} = r.k \
                 WHERE f.{c} IS NOT NULL",
                t = r.table,
                c = r.column
            ),
        });
    }
    let fact_tables: Vec<&str> = shapes
        .iter()
        .filter(|s| s.name != "facts" && s.schema.index_of("fact_id").is_ok())
        .map(|s| s.name)
        .collect();
    for t in &fact_tables {
        out.push(Rule {
            name: format!("fact:{t}"),
            sql: format!(
                "SELECT t.fact_id FROM {t} t LEFT ANTI JOIN facts f \
                 ON f.fact_id = t.fact_id AND f.table_name = '{t}'"
            ),
        });
        out.push(Rule {
            name: format!("fact-payload:{t}"),
            sql: format!(
                "SELECT f.fact_id FROM facts f LEFT ANTI JOIN {t} t ON t.fact_id = f.fact_id \
                 WHERE f.table_name = '{t}'"
            ),
        });
    }
    out.push(Rule {
        name: "fact:table-name".to_owned(),
        sql: format!(
            "SELECT table_name FROM facts WHERE table_name NOT IN ({})",
            quoted(&fact_tables)
        ),
    });
    let books = registry();
    for s in &shapes {
        for f in s.schema.fields() {
            let Some(book) = f.metadata().get(CODEBOOK_KEY) else {
                continue;
            };
            let codes = books
                .iter()
                .find(|b| b.name == book)
                .map(|b| {
                    b.values
                        .iter()
                        .map(|(code, _)| code.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
                .join(", ");
            out.push(Rule {
                name: format!("codebook:{}.{}", s.name, f.name()),
                sql: format!(
                    "SELECT {c} FROM {t} WHERE {c} IS NOT NULL AND {c} NOT IN ({codes})",
                    t = s.name,
                    c = f.name()
                ),
            });
        }
    }
    let families = FactFamily::all()
        .iter()
        .map(|f| format!("('{}', {})", f.text(), f.code()))
        .collect::<Vec<_>>()
        .join(", ");
    out.push(Rule {
        name: "coverage:declared-family".to_owned(),
        sql: format!(
            "WITH declared AS (SELECT unnest(families) AS family FROM runs) \
             SELECT family FROM declared WHERE family NOT IN ({})",
            quoted(FactFamily::all().iter().map(|f| f.text()))
        ),
    });
    out.push(Rule {
        name: "coverage:complete".to_owned(),
        sql: format!(
            "WITH declared AS (SELECT run_id, release_id, unnest(families) AS family FROM runs), \
             codes AS (SELECT * FROM (VALUES {families}) AS c(family, code)), \
             expected AS ( \
               SELECT d.run_id, s.module_node_id, c.code \
               FROM declared d JOIN codes c ON c.family = d.family \
               JOIN source_files s ON s.release_id = d.release_id) \
             SELECT e.run_id, e.module_node_id, e.code FROM expected e \
             LEFT ANTI JOIN coverage v \
               ON v.run_id = e.run_id AND v.scope_node_id = e.module_node_id \
              AND v.fact_family = e.code"
        ),
    });
    out.extend(semantic());
    out
}
