//! The graph catalog and edge registry (DESIGN §3.8; ADR-0014).
//!
//! The typed family tables stay the only authority. `nodes` and `edges` are Stage-D derived tables
//! generated from the registry below: identity, kind, endpoints and evidence, never a payload.
//! - **Per node kind:** one existence source, a relation independent of every column that
//!   references the node, so an isolate (a public function nobody calls) is present.
//! - **Per edge kind:** the endpoint kinds, its direction meaning, whether parallel edges are
//!   allowed, its derivation class, the table its evidence fact belongs to, and its lineage from
//!   raw rows.
//!
//! Generated from the registry: the catalog SQL, the endpoint-kind rules, the evidence rules, the
//! lineage rules (each raw row yields its edge, or its derived row carries a reason), the
//! single-parent rules for kinds that forbid parallel edges, and the typed-target rules. No rule
//! reads a relation built from the column it checks.

use crate::codebook::{
    AncestryRelation, Codebook, DeclarationKind, EdgeKind, NodeKind, PysaCalleeKind, PysaSiteKind,
    PysaTargetKind,
};
use crate::id::Id;
use crate::rules::Rule;
use crate::table::table;

/// How an edge kind is asserted (DESIGN §3.8): the guidelines' "extracted, resolved, derived and
/// heuristic stay distinguishable".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derivation {
    /// One provider row states it.
    Extracted,
    /// A provider's own resolution states it (Pysa, Pyrefly's public names).
    Analyzer,
    /// A Stage-C/D join decides it.
    Joined,
    /// Our own analysis decides it.
    Recognizer,
}

impl Derivation {
    pub fn text(self) -> &'static str {
        match self {
            Derivation::Extracted => "extracted",
            Derivation::Analyzer => "analyzer",
            Derivation::Joined => "joined",
            Derivation::Recognizer => "recognizer",
        }
    }
}

/// A node kind's existence source: a query yielding `node_id`, `module_node_id` and
/// `existence_fact_id`.
pub struct NodeSource {
    pub kind: NodeKind,
    pub sql: String,
}

/// Raw rows each edge of a kind is accountable to: every `expected` fact id is an edge's evidence
/// or an `explained` fact id (a derived row that carries a reason, or a declared exclusion).
pub struct Lineage {
    pub expected: String,
    pub explained: Option<String>,
}

/// One edge kind of the registry.
pub struct EdgeSource {
    pub kind: EdgeKind,
    pub src: &'static [NodeKind],
    pub dst: &'static [NodeKind],
    /// What `src → dst` means.
    pub direction: &'static str,
    /// Whether two edges of the kind may join the same pair of nodes.
    pub parallel: bool,
    pub derivation: Derivation,
    /// The raw table the evidence fact belongs to.
    pub evidence_table: &'static str,
    /// Yields `src_node_id, dst_node_id, ordinal, evidence_fact_id, support_fact_id,
    /// discriminator` (ids as `BYTEA`, the ordinal as `BIGINT`).
    pub sql: String,
    /// At most one edge per evidence row.
    pub one_per_evidence: bool,
    pub lineage: Option<Lineage>,
}

fn c(v: impl Codebook) -> i16 {
    v.code()
}

const NULL_ID: &str = "CAST(NULL AS BYTEA)";
const NULL_ORDINAL: &str = "CAST(NULL AS BIGINT)";

fn id(expr: &str) -> String {
    format!("CAST({expr} AS BYTEA)")
}

/// `src, dst, ordinal, evidence, support, discriminator` in the catalog's column types.
fn row(
    src: &str,
    dst: &str,
    ordinal: Option<&str>,
    evidence: &str,
    support: Option<&str>,
    disc: Option<&str>,
) -> String {
    format!(
        "{} AS src_node_id, {} AS dst_node_id, {} AS ordinal, {} AS evidence_fact_id, \
         {} AS support_fact_id, {} AS discriminator",
        id(src),
        id(dst),
        ordinal.map_or(NULL_ORDINAL.to_owned(), |o| format!("CAST({o} AS BIGINT)")),
        id(evidence),
        support.map_or(NULL_ID.to_owned(), id),
        disc.map_or(NULL_ID.to_owned(), id),
    )
}

fn functions() -> String {
    format!(
        "{}, {}",
        c(DeclarationKind::Function),
        c(DeclarationKind::AsyncFunction)
    )
}

/// Pysa's regular call records, the rows `call_targets` resolves.
fn regular_calls() -> String {
    format!(
        "site_kind = {} AND callee_kind = {} AND target_kind <> {}",
        c(PysaSiteKind::Regular),
        c(PysaCalleeKind::Call),
        c(PysaTargetKind::Unresolved)
    )
}

/// Every node kind's existence source (C1).
pub fn node_sources() -> Vec<NodeSource> {
    let n = |kind, sql: String| NodeSource { kind, sql };
    vec![
        n(
            NodeKind::Module,
            "SELECT module_node_id AS node_id, module_node_id, fact_id AS existence_fact_id \
             FROM source_files"
                .to_owned(),
        ),
        n(
            NodeKind::Class,
            format!(
                "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM declarations \
                 WHERE kind = {}",
                c(DeclarationKind::Class)
            ),
        ),
        n(
            NodeKind::Function,
            format!(
                "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM declarations \
                 WHERE kind IN ({})",
                functions()
            ),
        ),
        n(
            NodeKind::Parameter,
            "SELECT p.node_id, d.module_node_id, p.fact_id AS existence_fact_id \
             FROM parameter_syntax p JOIN declarations d ON d.node_id = p.function_node_id"
                .to_owned(),
        ),
        n(
            NodeKind::CallSite,
            "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM call_syntax"
                .to_owned(),
        ),
        n(
            NodeKind::Argument,
            "SELECT a.node_id, s.module_node_id, a.fact_id AS existence_fact_id \
             FROM arguments a JOIN call_syntax s ON s.node_id = a.call_node_id"
                .to_owned(),
        ),
        n(
            NodeKind::Export,
            "SELECT export_node_id AS node_id, CAST(NULL AS BYTEA) AS module_node_id, \
                    public_fact_id AS existence_fact_id FROM exports"
                .to_owned(),
        ),
        n(
            NodeKind::ExternalModule,
            "SELECT module_node_id AS node_id, CAST(NULL AS BYTEA) AS module_node_id, \
                    fact_id AS existence_fact_id FROM context_modules"
                .to_owned(),
        ),
        n(
            NodeKind::ExternalSymbol,
            "SELECT symbol_node_id AS node_id, module_node_id, fact_id AS existence_fact_id \
             FROM context_definitions"
                .to_owned(),
        ),
        n(
            NodeKind::SyntheticCallable,
            "SELECT node_id, module_node_id, pysa_fact_id AS existence_fact_id \
             FROM synthetic_callables"
                .to_owned(),
        ),
    ]
}

/// Every edge kind (C1).
pub fn edge_sources() -> Vec<EdgeSource> {
    use NodeKind as N;
    let callables: &[NodeKind] = &[N::Function, N::SyntheticCallable, N::ExternalSymbol];
    let declarations: &[NodeKind] = &[N::Class, N::Function];
    vec![
        EdgeSource {
            kind: EdgeKind::Declares,
            src: &[N::Module, N::Class, N::Function],
            dst: declarations,
            direction: "the module, class or function body defines the declaration",
            parallel: false,
            derivation: Derivation::Extracted,
            evidence_table: "declarations",
            sql: format!(
                "SELECT {} FROM declarations d",
                row(
                    "COALESCE(d.parent_node_id, d.module_node_id)",
                    "d.node_id",
                    None,
                    "d.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM declarations".to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::OverloadOf,
            src: &[N::Function],
            dst: &[N::Function],
            direction: "the `@overload` stub describes a signature of the callable",
            parallel: false,
            derivation: Derivation::Joined,
            evidence_table: "declarations",
            sql: format!(
                "SELECT {} FROM signatures s WHERE s.signature_node_id <> s.callable_node_id",
                row(
                    "s.signature_node_id",
                    "s.callable_node_id",
                    None,
                    "s.declaration_fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT fact_id FROM declarations WHERE is_overload AND kind IN ({})",
                    functions()
                ),
                // A stub-only group's last stub is its own callable.
                explained: Some(
                    "SELECT declaration_fact_id AS fact_id FROM signatures \
                     WHERE signature_node_id = callable_node_id"
                        .to_owned(),
                ),
            }),
        },
        EdgeSource {
            kind: EdgeKind::StubFor,
            src: declarations,
            dst: declarations,
            direction: "the `.pyi` declaration types the `.py` declaration of its module and name",
            parallel: false,
            derivation: Derivation::Joined,
            evidence_table: "declarations",
            sql: format!(
                "WITH keyed AS ( \
                   SELECT DISTINCT node_id FROM provider_node_map WHERE node_id IS NOT NULL), \
                 py AS ( \
                   SELECT d.node_id, d.fact_id, s.module_name, d.qualified_name, \
                          row_number() OVER (PARTITION BY s.module_name, d.qualified_name \
                                             ORDER BY d.is_overload, k.node_id IS NULL, \
                                                      d.start_byte DESC, d.node_id) AS pick \
                   FROM declarations d \
                   JOIN source_files s ON s.module_node_id = d.module_node_id AND NOT s.is_stub \
                   LEFT JOIN keyed k ON k.node_id = d.node_id) \
                 SELECT {} FROM declarations d \
                 JOIN source_files s ON s.module_node_id = d.module_node_id AND s.is_stub \
                 JOIN py p ON p.pick = 1 AND p.module_name = s.module_name \
                  AND p.qualified_name = d.qualified_name",
                row(
                    "d.node_id",
                    "p.node_id",
                    None,
                    "d.fact_id",
                    Some("p.fact_id"),
                    None
                )
            ),
            one_per_evidence: true,
            // A stub may type a module with no `.py` in the release: no lineage obligation.
            lineage: None,
        },
        EdgeSource {
            kind: EdgeKind::HasParameter,
            src: &[N::Function],
            dst: &[N::Parameter],
            direction: "the `def` declares the parameter at the ordinal",
            parallel: false,
            derivation: Derivation::Extracted,
            evidence_table: "parameter_syntax",
            sql: format!(
                "SELECT {} FROM parameter_syntax p",
                row(
                    "p.function_node_id",
                    "p.node_id",
                    Some("p.ordinal"),
                    "p.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM parameter_syntax".to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::Exports,
            src: &[N::Export],
            dst: &[N::Class, N::Function, N::ExternalSymbol],
            direction: "the public access path names the declaration",
            parallel: false,
            derivation: Derivation::Analyzer,
            evidence_table: "public_names",
            sql: format!(
                "SELECT {} FROM exports e \
                 WHERE COALESCE(e.declaration_node_id, e.external_node_id) IS NOT NULL",
                row(
                    "e.export_node_id",
                    "COALESCE(e.declaration_node_id, e.external_node_id)",
                    None,
                    "e.public_fact_id",
                    Some("e.declaration_fact_id"),
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM public_names".to_owned(),
                explained: Some(
                    "SELECT public_fact_id AS fact_id FROM exports WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        EdgeSource {
            kind: EdgeKind::EnclosesCall,
            src: &[N::Module, N::Class, N::Function],
            dst: &[N::CallSite],
            direction: "the call is in the module's or declaration's body (its owner)",
            parallel: false,
            derivation: Derivation::Extracted,
            evidence_table: "call_syntax",
            sql: format!(
                "SELECT {} FROM call_syntax s",
                row(
                    "COALESCE(s.owner_node_id, s.module_node_id)",
                    "s.node_id",
                    None,
                    "s.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM call_syntax".to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::HasArgument,
            src: &[N::CallSite],
            dst: &[N::Argument],
            direction: "the call passes the argument at the ordinal (source order)",
            parallel: false,
            derivation: Derivation::Extracted,
            evidence_table: "arguments",
            sql: format!(
                "SELECT {} FROM arguments a",
                row(
                    "a.call_node_id",
                    "a.node_id",
                    Some("a.ordinal"),
                    "a.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM arguments".to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::CallTarget,
            src: &[N::CallSite],
            dst: callables,
            direction: "the call site may invoke the target (phase, receiver and modality on the \
                        evidence row)",
            parallel: true,
            derivation: Derivation::Analyzer,
            evidence_table: "pysa_calls",
            sql: format!(
                "SELECT {} FROM call_targets t \
                 JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
                 JOIN call_syntax s ON s.node_id = t.call_site_node_id \
                 WHERE p.higher_order_index IS NULL AND t.target_node_id IS NOT NULL",
                row(
                    "t.call_site_node_id",
                    "t.target_node_id",
                    None,
                    "t.pysa_fact_id",
                    Some("s.fact_id"),
                    Some("p.payload_id")
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT fact_id FROM pysa_calls WHERE {} AND higher_order_index IS NULL",
                    regular_calls()
                ),
                explained: Some(
                    "SELECT pysa_fact_id AS fact_id FROM call_targets WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        EdgeSource {
            kind: EdgeKind::HigherOrderTarget,
            src: &[N::Argument],
            dst: callables,
            direction: "the argument's value may be invoked as the target (`potential`)",
            parallel: true,
            derivation: Derivation::Analyzer,
            evidence_table: "pysa_calls",
            sql: format!(
                "SELECT {} FROM call_targets t \
                 JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
                 WHERE t.argument_node_id IS NOT NULL AND t.target_node_id IS NOT NULL",
                row(
                    "t.argument_node_id",
                    "t.target_node_id",
                    None,
                    "t.pysa_fact_id",
                    None,
                    Some("p.payload_id")
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT fact_id FROM pysa_calls WHERE {} AND higher_order_index IS NOT NULL",
                    regular_calls()
                ),
                explained: Some(
                    "SELECT pysa_fact_id AS fact_id FROM call_targets WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        ancestry(
            EdgeKind::BaseClass,
            AncestryRelation::Base,
            "the class lists the ancestor as a base, at the ordinal",
        ),
        ancestry(
            EdgeKind::MroEntry,
            AncestryRelation::Mro,
            "the ancestor is at the ordinal of the class's MRO as Pyrefly reports it (the class and `object` excluded)",
        ),
        EdgeSource {
            kind: EdgeKind::Overrides,
            src: &[N::Function, N::SyntheticCallable],
            dst: callables,
            direction: "the method overrides the base-class method",
            parallel: false,
            derivation: Derivation::Analyzer,
            evidence_table: "pysa_functions",
            sql: format!(
                "SELECT {} FROM override_targets o \
                 WHERE o.function_node_id IS NOT NULL AND o.overridden_node_id IS NOT NULL",
                row(
                    "o.function_node_id",
                    "o.overridden_node_id",
                    None,
                    "o.function_fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM pysa_functions WHERE overridden_module IS NOT NULL"
                    .to_owned(),
                explained: Some(
                    "SELECT function_fact_id AS fact_id FROM override_targets \
                     WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        EdgeSource {
            kind: EdgeKind::DeclaredIn,
            src: &[N::ExternalSymbol],
            dst: &[N::ExternalModule],
            direction: "the dependency module defines the symbol",
            parallel: false,
            derivation: Derivation::Joined,
            evidence_table: "context_definitions",
            sql: format!(
                "SELECT {} FROM context_definitions d",
                row(
                    "d.symbol_node_id",
                    "d.module_node_id",
                    None,
                    "d.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM context_definitions".to_owned(),
                explained: None,
            }),
        },
    ]
}

fn ancestry(kind: EdgeKind, relation: AncestryRelation, direction: &'static str) -> EdgeSource {
    use NodeKind as N;
    EdgeSource {
        kind,
        src: &[N::Class],
        dst: &[N::Class, N::ExternalSymbol],
        direction,
        parallel: false,
        derivation: Derivation::Analyzer,
        evidence_table: "class_ancestry",
        sql: format!(
            "SELECT {} FROM ancestry_targets t \
             JOIN class_ancestry a ON a.fact_id = t.ancestry_fact_id \
             WHERE a.relation = {} AND t.class_node_id IS NOT NULL \
               AND t.ancestor_node_id IS NOT NULL",
            row(
                "t.class_node_id",
                "t.ancestor_node_id",
                Some("a.ordinal"),
                "t.ancestry_fact_id",
                None,
                None
            ),
            c(relation)
        ),
        one_per_evidence: true,
        lineage: Some(Lineage {
            expected: format!(
                "SELECT fact_id FROM class_ancestry WHERE relation = {} AND ancestor_module IS NOT NULL",
                c(relation)
            ),
            explained: Some(
                "SELECT ancestry_fact_id AS fact_id FROM ancestry_targets WHERE reason IS NOT NULL"
                    .to_owned(),
            ),
        }),
    }
}

fn kinds(ks: &[NodeKind]) -> String {
    ks.iter()
        .map(|k| k.code().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

table!(
    /// Every node of the snapshot, one row per node (DESIGN §3.8), from each kind's existence
    /// source. One id has one kind: a collision fails `key:nodes`.
    Nodes, NodesRow = "nodes",
    family = Graph,
    key = [snapshot_id, node_id],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        node_kind: NodeKind,
        /// The release module, or for an external symbol its external module; null for an export
        /// and an external module.
        module_node_id: Option<Id>,
        existence_fact_id: Id,
    }
);

impl crate::derived::Derived for Nodes {
    fn sql() -> String {
        let union = node_sources()
            .iter()
            .map(|n| {
                format!(
                    "SELECT CAST(node_id AS BYTEA) AS node_id, CAST({} AS SMALLINT) AS node_kind, \
                            CAST(module_node_id AS BYTEA) AS module_node_id, \
                            CAST(existence_fact_id AS BYTEA) AS existence_fact_id \
                     FROM ({}) s",
                    n.kind.code(),
                    n.sql
                )
            })
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        // One row per (node, kind): a node several existence rows assert (an export seen from a
        // `.py` and its `.pyi`) keeps its least fact; the same id under two kinds stays twice.
        format!(
            "SELECT node_id, node_kind, module_node_id, existence_fact_id FROM ( \
               SELECT *, row_number() OVER (PARTITION BY node_id, node_kind \
                                            ORDER BY existence_fact_id \
                                            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) \
                        AS pick \
               FROM ({union}) u) r \
             WHERE pick = 1"
        )
    }
}

table!(
    /// Every relationship of the snapshot (DESIGN §3.8): a persistent `edge_id` =
    /// `H(edge, kind, src, dst, ordinal, discriminator)`, typed endpoints, and the evidence fact
    /// whose row carries the payload. An endpoint's kind is what the join with `nodes` decides; a
    /// dangling endpoint reads null and fails its endpoint rule.
    Edges, EdgesRow = "edges",
    family = Graph,
    key = [snapshot_id, edge_id],
    checks = [("ordinal_nonnegative", "ordinal IS NULL OR ordinal >= 0")],
    {
        snapshot_id: Id,
        edge_id: Id,
        edge_kind: EdgeKind,
        src_node_id: Id,
        src_kind: Option<NodeKind>,
        dst_node_id: Id,
        dst_kind: Option<NodeKind>,
        ordinal: Option<i64>,
        evidence_fact_id: Id,
        support_fact_id: Option<Id>,
    }
);

impl crate::derived::Derived for Edges {
    fn sql() -> String {
        let union = edge_sources()
            .iter()
            .map(|e| {
                format!(
                    "SELECT CAST({} AS SMALLINT) AS edge_kind, * FROM ({}) k{}",
                    e.kind.code(),
                    e.sql,
                    e.kind.code()
                )
            })
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        format!(
            "WITH e AS ({union}) \
             SELECT lctx_id('edge', e.edge_kind, e.src_node_id, e.dst_node_id, e.ordinal, \
                            e.discriminator) AS edge_id, \
                    e.edge_kind, e.src_node_id, s.node_kind AS src_kind, \
                    e.dst_node_id, t.node_kind AS dst_kind, e.ordinal, \
                    e.evidence_fact_id, e.support_fact_id \
             FROM e \
             LEFT JOIN nodes s ON s.node_id = e.src_node_id \
             LEFT JOIN nodes t ON t.node_id = e.dst_node_id"
        )
    }
}

/// The graph rules (DESIGN §3.8, §8), one query each, generated from the registry.
pub fn rules() -> Vec<Rule> {
    let mut out = Vec::new();
    let rule = |name: String, sql: String| Rule { name, sql };
    for e in edge_sources() {
        let k = e.kind.code();
        let name = e.kind.text();
        out.push(rule(
            format!("endpoint:{name}"),
            format!(
                "SELECT edge_id FROM edges WHERE edge_kind = {k} \
                 AND (src_kind IS NULL OR src_kind NOT IN ({}) \
                      OR dst_kind IS NULL OR dst_kind NOT IN ({}))",
                kinds(e.src),
                kinds(e.dst)
            ),
        ));
        out.push(rule(
            format!("evidence:{name}"),
            format!(
                "SELECT e.edge_id FROM edges e LEFT ANTI JOIN facts f \
                   ON f.fact_id = e.evidence_fact_id AND f.table_name = '{}' \
                 WHERE e.edge_kind = {k}",
                e.evidence_table
            ),
        ));
        if e.one_per_evidence {
            out.push(rule(
                format!("one-per-evidence:{name}"),
                format!(
                    "SELECT evidence_fact_id, count(*) AS n FROM edges WHERE edge_kind = {k} \
                     GROUP BY evidence_fact_id HAVING count(*) > 1"
                ),
            ));
        }
        if !e.parallel {
            out.push(rule(
                format!("no-parallel:{name}"),
                format!(
                    "SELECT src_node_id, dst_node_id, count(*) AS n FROM edges \
                     WHERE edge_kind = {k} GROUP BY src_node_id, dst_node_id, ordinal \
                     HAVING count(*) > 1"
                ),
            ));
        }
        if let Some(l) = &e.lineage {
            let explained = l
                .explained
                .as_ref()
                .map(|x| format!(" UNION ALL SELECT fact_id FROM ({x}) x"))
                .unwrap_or_default();
            out.push(rule(
                format!("lineage:{name}"),
                format!(
                    "WITH expected AS ({}), \
                     accounted AS (SELECT evidence_fact_id AS fact_id FROM edges \
                                   WHERE edge_kind = {k}{explained}) \
                     SELECT x.fact_id FROM expected x \
                     LEFT ANTI JOIN accounted a ON a.fact_id = x.fact_id",
                    l.expected
                ),
            ));
        }
    }
    out.push(rule(
        "support:edges".to_owned(),
        "SELECT e.edge_id FROM edges e LEFT ANTI JOIN facts f ON f.fact_id = e.support_fact_id \
         WHERE e.support_fact_id IS NOT NULL"
            .to_owned(),
    ));
    // A null target always says why (ADR-0014): the retired "null node, null reason" meanings.
    for (table, columns) in [
        ("call_targets", "target_node_id IS NULL"),
        (
            "ancestry_targets",
            "(class_node_id IS NULL OR ancestor_node_id IS NULL)",
        ),
        (
            "override_targets",
            "(function_node_id IS NULL OR overridden_node_id IS NULL)",
        ),
        (
            "exports",
            "declaration_node_id IS NULL AND external_node_id IS NULL",
        ),
    ] {
        out.push(rule(
            format!("typed:{table}"),
            format!("SELECT * FROM {table} WHERE {columns} AND reason IS NULL"),
        ));
    }
    out
}
