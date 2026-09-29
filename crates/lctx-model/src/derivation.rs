//! The derivation index (DESIGN §15.9).
//!
//! Each proof or step relation declares itself a [`DerivationSource`]: which of its columns is
//! the derivation's own id, which is the conclusion, the rule, and its premise columns with their
//! roles. The PostgreSQL views `derivations` and `derivation_premises` are generated from these
//! declarations, so "why" and "why unresolved" traverse one index. Alternatives are separate
//! derivations of one conclusion; premise graphs are acyclic ([`acyclic`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::decl::relation::{GENERATION_COLUMN, RelationDecl};
use crate::ddl::{DdlConfig, quote};
use crate::id::Id;

/// One premise column of a step relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Premise {
    /// The column holding the premise's id.
    pub column: &'static str,
    /// What the premise contributes (`binding`, `callee_transfer`, `condition`, …).
    pub role: &'static str,
    /// The relation the premise id names.
    pub relation: &'static str,
}

/// A step or proof relation, declared as a derivation source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerivationSource {
    /// The step relation.
    pub relation: &'static str,
    /// The column holding the derivation's own id (the step row's id).
    pub id_column: &'static str,
    /// The relation and column of what it concludes.
    pub conclusion_relation: &'static str,
    pub conclusion_column: &'static str,
    /// The rule's stable name.
    pub rule: &'static str,
    pub premises: &'static [Premise],
}

/// A derivation-source defect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceError(pub String);

/// Check every source against the declared model: the step relation, its id, conclusion and
/// premise columns, and the relations they name, all exist.
pub fn validate(sources: &[DerivationSource], model: &[&RelationDecl]) -> Result<(), Vec<SourceError>> {
    let find = |name: &str| model.iter().find(|d| d.name == name);
    let mut errors = Vec::new();
    let mut rules = BTreeSet::new();
    for s in sources {
        let mut err = |m: String| errors.push(SourceError(format!("{}: {m}", s.rule)));
        if !rules.insert((s.relation, s.rule)) {
            err("rule declared twice for one relation".into());
        }
        let Some(step) = find(s.relation) else {
            err(format!("step relation {} is not declared", s.relation));
            continue;
        };
        for column in [s.id_column, s.conclusion_column]
            .into_iter()
            .chain(s.premises.iter().map(|p| p.column))
        {
            if step.column(column).is_none() {
                err(format!("{}.{column} is not declared", s.relation));
            }
        }
        for relation in std::iter::once(s.conclusion_relation).chain(s.premises.iter().map(|p| p.relation)) {
            if find(relation).is_none() {
                err(format!("relation {relation} is not declared"));
            }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The `derivations` view: one row per derivation, naming its rule and conclusion.
pub fn derivations_view(sources: &[DerivationSource], cfg: &DdlConfig) -> String {
    let parts: Vec<String> = sources
        .iter()
        .map(|s| {
            format!(
                "SELECT {g}, {id} AS derivation_id, {rule} AS rule, {rel} AS source_relation, \
                 {crel} AS conclusion_relation, {c} AS conclusion_id FROM {schema}.{table}",
                g = quote(GENERATION_COLUMN),
                id = quote(s.id_column),
                rule = literal(s.rule),
                rel = literal(s.relation),
                crel = literal(s.conclusion_relation),
                c = quote(s.conclusion_column),
                schema = quote(cfg.schema),
                table = quote(s.relation),
            )
        })
        .collect();
    crate::ddl::view("derivations", &body(parts, "derivation_id, rule, source_relation, conclusion_relation, conclusion_id"), cfg)
}

/// The `derivation_premises` view: one row per present premise of each derivation.
pub fn premises_view(sources: &[DerivationSource], cfg: &DdlConfig) -> String {
    let parts: Vec<String> = sources
        .iter()
        .flat_map(|s| {
            s.premises.iter().map(move |p| {
                format!(
                    "SELECT {g}, {id} AS derivation_id, {role} AS role, {rel} AS premise_relation, \
                     {col} AS premise_id FROM {schema}.{table} WHERE {col} IS NOT NULL",
                    g = quote(GENERATION_COLUMN),
                    id = quote(s.id_column),
                    role = literal(p.role),
                    rel = literal(p.relation),
                    col = quote(p.column),
                    schema = quote(cfg.schema),
                    table = quote(s.relation),
                )
            })
        })
        .collect();
    crate::ddl::view("derivation_premises", &body(parts, "derivation_id, role, premise_relation, premise_id"), cfg)
}

fn body(parts: Vec<String>, columns: &str) -> String {
    if parts.is_empty() {
        // An empty index still has its columns, so a reader never fails on a missing view.
        let nulls: Vec<String> = columns
            .split(", ")
            .map(|c| format!("NULL::{} AS {c}", if c.ends_with("_id") { "bytea" } else { "text" }))
            .collect();
        return format!(
            "SELECT NULL::bytea AS {}, {} WHERE FALSE",
            quote(GENERATION_COLUMN),
            nulls.join(", ")
        );
    }
    parts.join("\nUNION ALL\n")
}

/// Whether a set of `(derivation's conclusion, premise)` edges over row ids is acyclic. The
/// premise graph of every generation must be; the store's validator runs this over the views.
pub fn acyclic(edges: &[(Id, Id)]) -> bool {
    let mut indegree: BTreeMap<Id, usize> = BTreeMap::new();
    let mut out: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for &(conclusion, premise) in edges {
        indegree.entry(conclusion).or_default();
        *indegree.entry(premise).or_default() += 1;
        out.entry(conclusion).or_default().push(premise);
    }
    let mut ready: Vec<Id> = indegree.iter().filter(|(_, d)| **d == 0).map(|(n, _)| *n).collect();
    let mut seen = 0;
    while let Some(node) = ready.pop() {
        seen += 1;
        for next in out.get(&node).into_iter().flatten() {
            let d = indegree.get_mut(next).expect("every endpoint counted");
            *d -= 1;
            if *d == 0 {
                ready.push(*next);
            }
        }
    }
    seen == indegree.len()
}
