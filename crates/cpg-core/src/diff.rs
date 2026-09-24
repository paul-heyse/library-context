//! `lctx diff` (DESIGN §9.8, slice 3.3): two published snapshots compared by content id. Each is
//! read through its own snapshot-scoped session, and the join on ids happens here, in memory, so no
//! reader reads a table across snapshots (ADR-0017). Finding, assertion, evidence and brief ids
//! carry no run or config, so a variant's unchanged output keeps its ids and the diff is its
//! change.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use datafusion::arrow::array::{Array, StringArray};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::DataType;
use datafusion::prelude::SessionContext;
use serde::Serialize;

use crate::CoreError;
use crate::snapshot::published;
use crate::sql;
use cpg_schema::codebook::{AssertionKind, EvidenceStatus};
use cpg_schema::id::Id;

/// One table's ids compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableDiff {
    pub table: &'static str,
    pub only_from: usize,
    pub only_to: usize,
    pub common: usize,
}

/// One brief (by title) whose statements differ: `(kind, status, text)` rows on one side only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BriefChange {
    pub title: String,
    pub removed: Vec<(String, String, String)>,
    pub added: Vec<(String, String, String)>,
}

/// The whole comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diff {
    pub from: String,
    pub to: String,
    pub tables: Vec<TableDiff>,
    /// Brief titles on one side only.
    pub briefs_only_from: Vec<String>,
    pub briefs_only_to: Vec<String>,
    pub changed: Vec<BriefChange>,
}

/// The ids compared: each table's content id, as text. A document is keyed by its brief's seed,
/// its chunk and its text (the ADR-0020 review's F7), not by its brief's id, which moves with any
/// assertion: a document no technique reaches compares equal.
fn id_queries() -> [(&'static str, &'static str); 5] {
    [
        (
            "findings",
            "SELECT DISTINCT encode(finding_id, 'hex') FROM findings",
        ),
        (
            "assertions",
            "SELECT DISTINCT encode(assertion_id, 'hex') FROM assertions",
        ),
        (
            "evidence",
            "SELECT DISTINCT encode(evidence_id, 'hex') FROM evidence",
        ),
        (
            "briefs",
            "SELECT DISTINCT encode(brief_id, 'hex') FROM briefs",
        ),
        (
            "brief_documents",
            "SELECT DISTINCT encode(b.seed_node_id, 'hex') || ':' || CAST(d.chunk AS VARCHAR) \
             || ':' || encode(sha256(d.text), 'hex') \
             FROM brief_documents d JOIN briefs b ON b.brief_id = d.brief_id",
        ),
    ]
}

/// What a brief states, per title, its kind and status by codebook name (the ADR-0020 review's O3).
fn statements_query() -> String {
    format!(
        "SELECT b.title, {kind} AS kind, {status} AS status, COALESCE(a.text, '') AS text \
         FROM briefs b JOIN brief_assertions ba ON ba.brief_id = b.brief_id \
         JOIN assertions a ON a.assertion_id = ba.assertion_id",
        kind = crate::bundle::text_of::<AssertionKind>("a.assertion_kind"),
        status = crate::bundle::text_of::<EvidenceStatus>("a.evidence_status"),
    )
}

async fn strings(ctx: &SessionContext, query: &str) -> Result<Vec<Vec<String>>, CoreError> {
    let batches = sql::query(ctx, query).await?.collect().await?;
    let mut rows = Vec::new();
    for b in &batches {
        let columns: Vec<StringArray> = b
            .columns()
            .iter()
            .map(|c| {
                cast(c, &DataType::Utf8).map(|a| {
                    a.as_any()
                        .downcast_ref::<StringArray>()
                        .expect("cast to Utf8")
                        .clone()
                })
            })
            .collect::<Result<_, _>>()?;
        for i in 0..b.num_rows() {
            rows.push(
                columns
                    .iter()
                    .map(|c| {
                        if c.is_null(i) {
                            String::new()
                        } else {
                            c.value(i).to_owned()
                        }
                    })
                    .collect(),
            );
        }
    }
    Ok(rows)
}

async fn session(store: &Path, id: Id) -> Result<SessionContext, CoreError> {
    published(store, id)
        .await?
        .map(|(_, ctx)| ctx)
        .ok_or_else(|| CoreError::Analysis(format!("snapshot {} is not published", id.hex())))
}

type Statements = BTreeMap<String, BTreeSet<(String, String, String)>>;

async fn statements(ctx: &SessionContext) -> Result<Statements, CoreError> {
    let mut out: Statements = BTreeMap::new();
    for row in strings(ctx, &statements_query()).await? {
        let [title, kind, status, text]: [String; 4] = row.try_into().expect("four columns");
        out.entry(title).or_default().insert((kind, status, text));
    }
    Ok(out)
}

/// Compare two published snapshots of one store.
pub async fn diff(store: &Path, from: Id, to: Id) -> Result<Diff, CoreError> {
    let (a, b) = (session(store, from).await?, session(store, to).await?);
    let mut tables = Vec::new();
    for (table, query) in id_queries() {
        let ids = |rows: Vec<Vec<String>>| -> BTreeSet<String> {
            rows.into_iter()
                .filter_map(|r| r.into_iter().next())
                .collect()
        };
        let (x, y) = (
            ids(strings(&a, query).await?),
            ids(strings(&b, query).await?),
        );
        tables.push(TableDiff {
            table,
            only_from: x.difference(&y).count(),
            only_to: y.difference(&x).count(),
            common: x.intersection(&y).count(),
        });
    }
    let (sa, sb) = (statements(&a).await?, statements(&b).await?);
    let titles = |s: &Statements| s.keys().cloned().collect::<BTreeSet<_>>();
    let (ta, tb) = (titles(&sa), titles(&sb));
    let changed = ta
        .intersection(&tb)
        .filter_map(|title| {
            let (x, y) = (&sa[title], &sb[title]);
            let removed: Vec<_> = x.difference(y).cloned().collect();
            let added: Vec<_> = y.difference(x).cloned().collect();
            (!removed.is_empty() || !added.is_empty()).then(|| BriefChange {
                title: title.clone(),
                removed,
                added,
            })
        })
        .collect();
    Ok(Diff {
        from: from.hex(),
        to: to.hex(),
        tables,
        briefs_only_from: ta.difference(&tb).cloned().collect(),
        briefs_only_to: tb.difference(&ta).cloned().collect(),
        changed,
    })
}

impl Diff {
    /// §9.8's first keep condition: the published briefs or what they state differ.
    pub fn changes_published_output(&self) -> bool {
        !self.briefs_only_from.is_empty()
            || !self.briefs_only_to.is_empty()
            || !self.changed.is_empty()
    }

    /// The comparison as pretty JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a diff serializes") + "\n"
    }

    /// A readable report.
    pub fn render(&self) -> String {
        let mut out = format!("diff {} -> {}\n", self.from, self.to);
        for t in &self.tables {
            out += &format!(
                "  {:<16} only in from {:>6}, only in to {:>6}, common {:>6}\n",
                t.table, t.only_from, t.only_to, t.common
            );
        }
        for title in &self.briefs_only_from {
            out += &format!("- brief {title}\n");
        }
        for title in &self.briefs_only_to {
            out += &format!("+ brief {title}\n");
        }
        for c in &self.changed {
            out += &format!("~ brief {}\n", c.title);
            for (kind, status, text) in &c.removed {
                out += &format!("    - [{kind}/{status}] {text}\n");
            }
            for (kind, status, text) in &c.added {
                out += &format!("    + [{kind}/{status}] {text}\n");
            }
        }
        out += &format!(
            "published output {}\n",
            if self.changes_published_output() {
                "changed"
            } else {
                "unchanged"
            }
        );
        out
    }
}
