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
    AncestryRelation, BindingKind, BoundaryReason, Codebook, DeclarationKind, DerivationClass,
    EdgeKind, ExportSyntaxKind, Modality, ModuleOrigin, NodeKind, PysaCalleeKind, PysaSiteKind,
    PysaTargetKind, SyntaxField, SyntaxKind,
};
use crate::id::Id;
use crate::rules::Rule;
use crate::table::table;

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
    /// How the kind is asserted: the guidelines' extracted, resolved, derived and heuristic stay
    /// distinguishable (published in `edge_kinds`).
    pub derivation: DerivationClass,
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

/// The node whose body holds a placed call or site: its owning declaration, else its module.
/// One expression for `encloses_call` and the invocation projection (slice 1.4 review O7).
pub fn owner_of(alias: &str) -> String {
    format!("COALESCE({alias}.owner_node_id, {alias}.module_node_id)")
}

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
        // A module Pyrefly cannot find has a row (the provider's answer) and no node.
        n(
            NodeKind::ExternalModule,
            format!(
                "SELECT module_node_id AS node_id, CAST(NULL AS BYTEA) AS module_node_id, \
                        fact_id AS existence_fact_id FROM context_modules WHERE origin <> {}",
                c(ModuleOrigin::NotFound)
            ),
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
        // C2: placed syntax other than declarations and calls, whose existence sources are
        // `declarations` and `call_syntax` (their placement rows share their ids).
        n(
            NodeKind::SyntaxNode,
            format!(
                "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM syntax_nodes \
                 WHERE kind NOT IN ({}, {}, {})",
                c(SyntaxKind::StmtFunctionDef),
                c(SyntaxKind::StmtClassDef),
                c(SyntaxKind::ExprCall)
            ),
        ),
        // C3: the lexical family's own nodes.
        n(
            NodeKind::Scope,
            "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM scopes".to_owned(),
        ),
        n(
            NodeKind::Binding,
            "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM bindings".to_owned(),
        ),
        n(
            NodeKind::Reference,
            "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM references"
                .to_owned(),
        ),
        // C4: a term belongs to no module (one term serves every module that observes it).
        n(
            NodeKind::Type,
            "SELECT node_id, CAST(NULL AS BYTEA) AS module_node_id, fact_id AS existence_fact_id \
             FROM type_terms"
                .to_owned(),
        ),
        n(
            NodeKind::Field,
            "SELECT node_id, module_node_id, fact_id AS existence_fact_id FROM record_fields"
                .to_owned(),
        ),
        // C5: the source corpus. A document is not a module, so these have no module.
        n(
            NodeKind::Document,
            "SELECT node_id, CAST(NULL AS BYTEA) AS module_node_id, fact_id AS existence_fact_id \
             FROM documents"
                .to_owned(),
        ),
        n(
            NodeKind::Passage,
            "SELECT node_id, CAST(NULL AS BYTEA) AS module_node_id, fact_id AS existence_fact_id \
             FROM passages"
                .to_owned(),
        ),
        n(
            NodeKind::CodeBlock,
            "SELECT node_id, CAST(NULL AS BYTEA) AS module_node_id, fact_id AS existence_fact_id \
             FROM code_blocks"
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
            derivation: DerivationClass::Extracted,
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
            derivation: DerivationClass::Joined,
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
            derivation: DerivationClass::Joined,
            evidence_table: "declarations",
            sql: format!(
                "WITH {keyed}, \
                 py AS ( \
                   SELECT d.node_id, d.fact_id, s.release_id, s.module_name, d.qualified_name, \
                          {rank} AS pick \
                   FROM declarations d \
                   JOIN source_files s ON s.module_node_id = d.module_node_id AND NOT s.is_stub \
                   LEFT JOIN keyed k ON k.node_id = d.node_id) \
                 SELECT {} FROM declarations d \
                 JOIN source_files s ON s.module_node_id = d.module_node_id AND s.is_stub \
                 JOIN py p ON p.pick = 1 AND p.release_id = s.release_id \
                  AND p.module_name = s.module_name \
                  AND p.qualified_name = d.qualified_name",
                row(
                    "d.node_id",
                    "p.node_id",
                    None,
                    "d.fact_id",
                    Some("p.fact_id"),
                    None
                ),
                keyed = crate::derived::KEYED,
                rank = crate::derived::seed_rank("s.release_id, s.module_name, d.qualified_name"),
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
            derivation: DerivationClass::Extracted,
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
            dst: &[
                N::Class,
                N::Function,
                N::ExternalSymbol,
                N::Module,
                N::ExternalModule,
                N::Binding,
            ],
            direction: "the public access path, read from one file, names the target",
            // A `.py` and its `.pyi` both publish the path: one edge per access file, told apart by
            // the file (review F2).
            parallel: true,
            derivation: DerivationClass::Analyzer,
            evidence_table: "public_names",
            sql: format!(
                "SELECT {} FROM exports e JOIN public_names p ON p.fact_id = e.public_fact_id \
                 WHERE e.target_node_id IS NOT NULL",
                row(
                    "e.export_node_id",
                    "e.target_node_id",
                    None,
                    "e.public_fact_id",
                    Some("e.declaration_fact_id"),
                    Some("p.access_module_node_id")
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
            derivation: DerivationClass::Extracted,
            evidence_table: "call_syntax",
            sql: format!(
                "SELECT {} FROM call_syntax s",
                row(&owner_of("s"), "s.node_id", None, "s.fact_id", None, None)
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
            derivation: DerivationClass::Extracted,
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
            derivation: DerivationClass::Analyzer,
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
            derivation: DerivationClass::Analyzer,
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
            derivation: DerivationClass::Analyzer,
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
            derivation: DerivationClass::Joined,
            evidence_table: "context_definitions",
            // Two runs of one attempt may both describe a symbol: one edge, from the first fact.
            sql: format!(
                "SELECT {} FROM {} d WHERE d.pick = 1",
                row(
                    "d.symbol_node_id",
                    "d.module_node_id",
                    None,
                    "d.fact_id",
                    None,
                    None
                ),
                first_fact("context_definitions", "symbol_node_id")
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM context_definitions".to_owned(),
                explained: Some(format!(
                    "SELECT fact_id FROM {} d WHERE d.pick > 1",
                    first_fact("context_definitions", "symbol_node_id")
                )),
            }),
        },
        // C2: the syntax tree and Pysa's non-call sites.
        EdgeSource {
            kind: EdgeKind::AstChild,
            src: &[N::Module, N::Class, N::Function, N::CallSite, N::SyntaxNode],
            dst: &[N::Class, N::Function, N::CallSite, N::SyntaxNode],
            direction: "the child is placed in the parent's field at the ordinal (source order)",
            parallel: false,
            derivation: DerivationClass::Extracted,
            evidence_table: "syntax_nodes",
            sql: format!(
                "SELECT {} FROM syntax_nodes s",
                row(
                    "s.parent_node_id",
                    "s.node_id",
                    Some("s.ordinal"),
                    "s.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM syntax_nodes".to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::ArgumentValue,
            src: &[N::Argument],
            dst: &[N::CallSite, N::SyntaxNode],
            direction: "the argument's value is the placed expression",
            parallel: false,
            derivation: DerivationClass::Joined,
            evidence_table: "arguments",
            sql: format!(
                "SELECT {} FROM arguments a JOIN syntax_nodes s \
                   ON s.parent_node_id = a.call_node_id AND s.field = {} \
                  AND s.start_byte >= a.start_byte AND s.end_byte <= a.end_byte",
                row(
                    "a.node_id",
                    "s.node_id",
                    None,
                    "a.fact_id",
                    Some("s.fact_id"),
                    None
                ),
                c(SyntaxField::Argument)
            ),
            // Every expression outside annotations is placed, so every argument of a call outside
            // an annotation has exactly one value node.
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT a.fact_id FROM arguments a \
                           JOIN call_syntax c ON c.node_id = a.call_node_id \
                           WHERE NOT c.in_annotation"
                    .to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::SiteTarget,
            src: &[N::SyntaxNode, N::CallSite],
            dst: callables,
            direction: "the site (an attribute access, an operator, a `for`/`with` protocol, a \
                        format string) may invoke the target (phase and modality on the evidence \
                        row; artificial sites are `synthetic_model`)",
            parallel: true,
            derivation: DerivationClass::Analyzer,
            evidence_table: "pysa_calls",
            sql: format!(
                "SELECT {} FROM site_targets t JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
                 JOIN facts f ON f.fact_id = t.pysa_fact_id \
                 WHERE t.site_node_id IS NOT NULL AND t.target_node_id IS NOT NULL \
                   AND f.modality <> {potential}",
                row(
                    "t.site_node_id",
                    "t.target_node_id",
                    None,
                    "t.pysa_fact_id",
                    None,
                    Some("p.payload_id")
                ),
                potential = c(Modality::Potential),
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT p.fact_id FROM pysa_calls p JOIN facts f ON f.fact_id = p.fact_id \
                     WHERE NOT ({}) AND p.callee_kind <> {} AND f.modality <> {}",
                    call_site_rows(),
                    c(PysaCalleeKind::Identifier),
                    c(Modality::Potential)
                ),
                explained: Some(
                    "SELECT pysa_fact_id AS fact_id FROM site_targets WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        // C3: the lexical family.
        simple(
            EdgeKind::OwnsScope,
            &[N::Module, N::Class, N::Function, N::SyntaxNode],
            &[N::Scope],
            "the module, declaration, lambda or comprehension opens the scope",
            DerivationClass::Recognizer,
            "scopes",
            "SELECT {} FROM scopes s",
            ("s.owner_node_id", "s.node_id", None, "s.fact_id"),
            Some("SELECT fact_id FROM scopes"),
        ),
        simple(
            EdgeKind::LexicalParent,
            &[N::Scope],
            &[N::Scope],
            "the scope is nested in its parent scope",
            DerivationClass::Recognizer,
            "scopes",
            "SELECT {} FROM scopes s WHERE s.parent_scope_id IS NOT NULL",
            ("s.node_id", "s.parent_scope_id", None, "s.fact_id"),
            Some("SELECT fact_id FROM scopes WHERE parent_scope_id IS NOT NULL"),
        ),
        simple(
            EdgeKind::Binds,
            &[N::Scope],
            &[N::Binding],
            "the scope holds the binding event, at its ordinal (source order)",
            DerivationClass::Recognizer,
            "bindings",
            "SELECT {} FROM bindings b",
            ("b.scope_id", "b.node_id", Some("b.ordinal"), "b.fact_id"),
            Some("SELECT fact_id FROM bindings"),
        ),
        simple(
            EdgeKind::Introduces,
            &[N::Binding],
            &[N::Class, N::Function, N::Parameter, N::SyntaxNode],
            "the binding event is made by the declaration, parameter or placed statement",
            DerivationClass::Recognizer,
            "bindings",
            &format!(
                "SELECT {{}} FROM bindings b JOIN nodes n ON n.node_id = b.site_node_id \
                 WHERE b.kind <> {}",
                c(BindingKind::Implicit)
            ),
            ("b.node_id", "b.site_node_id", None, "b.fact_id"),
            // An import alias, a match pattern and a lambda parameter have no node of their own
            // (review O1, deferred); an implicit name has no statement.
            None,
        ),
        resolution(EdgeKind::ReadsBinding, false),
        resolution(EdgeKind::Captures, true),
        EdgeSource {
            kind: EdgeKind::ReadsBuiltin,
            src: &[N::Reference],
            dst: &[N::ExternalSymbol],
            direction: "the free name reads the builtin function or class",
            parallel: false,
            derivation: DerivationClass::Recognizer,
            evidence_table: "reference_resolutions",
            sql: format!(
                "WITH builtins AS ( \
                   SELECT d.name, d.symbol_node_id, \
                          row_number() OVER (PARTITION BY d.name ORDER BY d.kind, d.key, d.fact_id) \
                            AS pick \
                   FROM context_definitions d \
                   JOIN context_modules m ON m.module_node_id = d.module_node_id \
                   WHERE m.module_name = 'builtins' AND d.is_top_level) \
                 SELECT {} FROM reference_resolutions r \
                 JOIN builtins b ON b.pick = 1 AND b.name = r.builtin_name \
                 WHERE r.reason IS NULL",
                row(
                    "r.reference_id",
                    "b.symbol_node_id",
                    None,
                    "r.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM reference_resolutions \
                           WHERE builtin_name IS NOT NULL AND reason IS NULL"
                    .to_owned(),
                explained: None,
            }),
        },
        EdgeSource {
            kind: EdgeKind::Shadows,
            src: &[N::Binding],
            dst: &[N::Binding],
            direction: "the binding event follows the previous event of its name in its scope",
            parallel: false,
            derivation: DerivationClass::Joined,
            evidence_table: "bindings",
            sql: format!(
                "WITH ordered AS ( \
                   SELECT node_id, fact_id, \
                          lag(node_id) OVER (PARTITION BY scope_id, name ORDER BY ordinal) \
                            AS previous \
                   FROM bindings) \
                 SELECT {} FROM ordered o WHERE o.previous IS NOT NULL",
                row("o.node_id", "o.previous", None, "o.fact_id", None, None)
            ),
            one_per_evidence: true,
            lineage: None,
        },
        EdgeSource {
            kind: EdgeKind::PotentialTarget,
            src: &[N::Reference, N::SyntaxNode, N::CallSite],
            dst: callables,
            direction: "the referenced callable value may be invoked as the target (`if_called`; \
                        `potential`)",
            parallel: true,
            derivation: DerivationClass::Analyzer,
            evidence_table: "pysa_calls",
            sql: format!(
                "SELECT {} FROM identifier_targets t \
                 JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
                 WHERE t.reference_node_id IS NOT NULL AND t.target_node_id IS NOT NULL \
                 UNION ALL \
                 SELECT {} FROM site_targets t \
                 JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
                 JOIN facts f ON f.fact_id = t.pysa_fact_id \
                 WHERE t.site_node_id IS NOT NULL AND t.target_node_id IS NOT NULL \
                   AND f.modality = {potential}",
                row(
                    "t.reference_node_id",
                    "t.target_node_id",
                    None,
                    "t.pysa_fact_id",
                    None,
                    Some("p.payload_id")
                ),
                row(
                    "t.site_node_id",
                    "t.target_node_id",
                    None,
                    "t.pysa_fact_id",
                    None,
                    Some("p.payload_id")
                ),
                potential = c(Modality::Potential),
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT p.fact_id FROM pysa_calls p JOIN facts f ON f.fact_id = p.fact_id \
                     WHERE NOT ({}) AND (p.callee_kind = {} OR f.modality = {})",
                    call_site_rows(),
                    c(PysaCalleeKind::Identifier),
                    c(Modality::Potential)
                ),
                explained: Some(
                    "SELECT pysa_fact_id AS fact_id FROM identifier_targets \
                     WHERE reason IS NOT NULL \
                     UNION ALL SELECT pysa_fact_id AS fact_id FROM site_targets \
                     WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        EdgeSource {
            kind: EdgeKind::ImportsModule,
            src: &[N::Module],
            dst: &[N::Module, N::ExternalModule],
            direction: "the module imports the module (one edge per imported alias)",
            parallel: true,
            derivation: DerivationClass::Joined,
            evidence_table: "export_syntax",
            sql: format!(
                "SELECT {} FROM import_targets t JOIN export_syntax x ON x.fact_id = t.import_fact_id \
                 WHERE t.target_node_id IS NOT NULL",
                row(
                    "t.module_node_id",
                    "t.target_node_id",
                    None,
                    "t.import_fact_id",
                    None,
                    Some("x.node_id")
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: format!(
                    "SELECT fact_id FROM export_syntax WHERE kind IN ({}, {})",
                    c(ExportSyntaxKind::Import),
                    c(ExportSyntaxKind::ImportFrom)
                ),
                explained: Some(
                    "SELECT import_fact_id AS fact_id FROM import_targets WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        // C4: the types family.
        simple(
            EdgeKind::HasType,
            &[
                N::Parameter,
                N::Function,
                N::CallSite,
                N::Argument,
                N::SyntaxNode,
            ],
            &[N::Type],
            "the element has the type, in the role its observation states (§3.5.1)",
            DerivationClass::Analyzer,
            "type_observations",
            "SELECT {} FROM type_observations o",
            ("o.subject_node_id", "o.term_node_id", None, "o.fact_id"),
            Some("SELECT fact_id FROM type_observations"),
        ),
        EdgeSource {
            kind: EdgeKind::TypeArg,
            src: &[N::Type],
            dst: &[N::Type],
            direction: "the child term is part of the parent term, at its role and ordinal",
            // A callable returning the type of its first parameter joins one pair twice at
            // ordinal 0, told apart by the role.
            parallel: true,
            derivation: DerivationClass::Analyzer,
            evidence_table: "type_term_args",
            // A term both runs of an attempt observe has its children twice: one edge each.
            sql: format!(
                "SELECT {} FROM {} a WHERE a.pick = 1",
                row(
                    "a.parent_node_id",
                    "a.child_node_id",
                    Some("a.ordinal"),
                    "a.fact_id",
                    None,
                    Some("lctx_id('type_arg_role', a.role)")
                ),
                first_fact("type_term_args", "parent_node_id, role, ordinal")
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM type_term_args".to_owned(),
                explained: Some(format!(
                    "SELECT fact_id FROM {} a WHERE a.pick > 1",
                    first_fact("type_term_args", "parent_node_id, role, ordinal")
                )),
            }),
        },
        EdgeSource {
            kind: EdgeKind::TypeClass,
            src: &[N::Type],
            dst: &[N::Class, N::ExternalSymbol],
            direction: "the term is an instance, object or `TypedDict` of the class",
            parallel: false,
            derivation: DerivationClass::Analyzer,
            evidence_table: "type_terms",
            sql: format!(
                "SELECT {} FROM type_class_targets t WHERE t.class_node_id IS NOT NULL",
                row(
                    "t.term_node_id",
                    "t.class_node_id",
                    None,
                    "t.term_fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM type_terms WHERE class_module IS NOT NULL"
                    .to_owned(),
                explained: Some(format!(
                    "SELECT term_fact_id AS fact_id FROM type_class_targets \
                     WHERE reason IS NOT NULL \
                     UNION ALL SELECT fact_id FROM {} t WHERE t.pick > 1",
                    first_fact("type_terms", "node_id")
                )),
            }),
        },
        simple(
            EdgeKind::HasField,
            &[N::Class],
            &[N::Field],
            "the class declares the record field, at its ordinal in the record's field order",
            DerivationClass::Analyzer,
            "record_fields",
            "SELECT {} FROM record_fields r",
            (
                "r.class_node_id",
                "r.node_id",
                Some("r.ordinal"),
                "r.fact_id",
            ),
            Some("SELECT fact_id FROM record_fields"),
        ),
        simple(
            EdgeKind::FieldType,
            &[N::Field],
            &[N::Type],
            "the record field has the type",
            DerivationClass::Analyzer,
            "record_fields",
            "SELECT {} FROM record_fields r",
            ("r.node_id", "r.term_node_id", None, "r.fact_id"),
            Some("SELECT fact_id FROM record_fields"),
        ),
        // C5: the source corpus.
        simple(
            EdgeKind::ContainsPassage,
            &[N::Document],
            &[N::Passage],
            "the document holds the passage, at its ordinal",
            DerivationClass::Extracted,
            "passages",
            "SELECT {} FROM passages p",
            (
                "p.document_node_id",
                "p.node_id",
                Some("p.ordinal"),
                "p.fact_id",
            ),
            Some("SELECT fact_id FROM passages"),
        ),
        simple(
            EdgeKind::ContainsBlock,
            &[N::Passage],
            &[N::CodeBlock],
            "the passage holds the code block, at its ordinal in the document",
            DerivationClass::Extracted,
            "code_blocks",
            "SELECT {} FROM code_blocks b",
            (
                "b.passage_node_id",
                "b.node_id",
                Some("b.ordinal"),
                "b.fact_id",
            ),
            Some("SELECT fact_id FROM code_blocks"),
        ),
        EdgeSource {
            kind: EdgeKind::Mentions,
            src: &[N::Passage],
            dst: &[N::Export, N::Class, N::Function],
            direction: "the passage names the API element at the byte offset (`exact` or \
                        `lexical` on the evidence row; a lexical mention is `candidate`)",
            // One passage names one export at several places.
            parallel: true,
            derivation: DerivationClass::Recognizer,
            evidence_table: "mentions",
            sql: format!(
                "SELECT {} FROM mention_targets t JOIN mentions m ON m.fact_id = t.mention_fact_id \
                 WHERE t.target_node_id IS NOT NULL",
                row(
                    "t.passage_node_id",
                    "t.target_node_id",
                    Some("m.start_byte"),
                    "t.mention_fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM mentions".to_owned(),
                explained: Some(
                    "SELECT mention_fact_id AS fact_id FROM mention_targets \
                     WHERE reason IS NOT NULL"
                        .to_owned(),
                ),
            }),
        },
        // C5b: the usage run.
        EdgeSource {
            kind: EdgeKind::BlockModule,
            src: &[N::CodeBlock],
            dst: &[N::Module],
            direction: "the Python code block is compiled as the module (materialized in the usage run)",
            parallel: false,
            derivation: DerivationClass::Joined,
            evidence_table: "code_blocks",
            sql: format!(
                "SELECT {} FROM code_blocks b JOIN documents d ON d.node_id = b.document_node_id \
                 JOIN source_files f ON f.release_id = d.release_id AND f.path = b.module_path",
                row(
                    "b.node_id",
                    "f.module_node_id",
                    None,
                    "b.fact_id",
                    None,
                    None
                )
            ),
            one_per_evidence: true,
            lineage: Some(Lineage {
                expected: "SELECT fact_id FROM code_blocks WHERE module_path IS NOT NULL"
                    .to_owned(),
                explained: None,
            }),
        },
    ]
}

/// An edge kind read straight off one table: `sql` has one `{}` for the row, and each lineage row
/// yields one edge.
#[allow(
    clippy::too_many_arguments,
    reason = "a registry row or binding event is these fields"
)]
fn simple(
    kind: EdgeKind,
    src: &'static [NodeKind],
    dst: &'static [NodeKind],
    direction: &'static str,
    derivation: DerivationClass,
    evidence_table: &'static str,
    sql: &str,
    (from, to, ordinal, evidence): (&str, &str, Option<&str>, &str),
    lineage: Option<&str>,
) -> EdgeSource {
    EdgeSource {
        kind,
        src,
        dst,
        direction,
        parallel: false,
        derivation,
        evidence_table,
        sql: sql.replacen("{}", &row(from, to, ordinal, evidence, None, None), 1),
        one_per_evidence: true,
        lineage: lineage.map(|expected| Lineage {
            expected: expected.to_owned(),
            explained: None,
        }),
    }
}

/// A reference reads a binding (`reads_binding`), or a binding of an enclosing function scope
/// (`captures`): one row of the recognizer's resolution each.
fn resolution(kind: EdgeKind, captured: bool) -> EdgeSource {
    use NodeKind as N;
    let not = if captured { "" } else { "NOT " };
    EdgeSource {
        kind,
        src: &[N::Reference],
        dst: &[N::Binding],
        direction: if captured {
            "the free name reads a binding of an enclosing function scope (a closure)"
        } else {
            "the name reads the binding event (flow-insensitive; `candidate` when several)"
        },
        parallel: false,
        derivation: DerivationClass::Recognizer,
        evidence_table: "reference_resolutions",
        sql: format!(
            "SELECT {} FROM reference_resolutions r \
             WHERE r.binding_id IS NOT NULL AND {not}r.captured",
            row(
                "r.reference_id",
                "r.binding_id",
                None,
                "r.fact_id",
                None,
                None
            )
        ),
        one_per_evidence: true,
        lineage: Some(Lineage {
            expected: format!(
                "SELECT fact_id FROM reference_resolutions \
                 WHERE binding_id IS NOT NULL AND {not}captured"
            ),
            explained: None,
        }),
    }
}

fn ancestry(kind: EdgeKind, relation: AncestryRelation, direction: &'static str) -> EdgeSource {
    use NodeKind as N;
    EdgeSource {
        kind,
        src: &[N::Class],
        dst: &[N::Class, N::ExternalSymbol],
        direction,
        parallel: false,
        derivation: DerivationClass::Analyzer,
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

/// `table` with each row's rank among the rows sharing `partition` (by fact id): what two runs of
/// one attempt both assert is written once, from its first fact (`pick = 1`).
fn first_fact(table: &str, partition: &str) -> String {
    format!(
        "(SELECT *, row_number() OVER (PARTITION BY {partition} ORDER BY fact_id \
                                       ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS pick \
          FROM {table})"
    )
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
        // Only the kinds several existence rows legitimately assert keep one row: an export read
        // from a `.py` and its `.pyi`, and a dependency module, symbol or type term two runs of
        // one attempt both reference (a term's collision check is `unique:type_terms`). Any other repeated id stays twice, and `key:nodes` rejects it: a
        // collision is never merged (§3.4.1; review O2).
        let merged = [
            NodeKind::Export,
            NodeKind::ExternalModule,
            NodeKind::ExternalSymbol,
            // One structure is one term, whichever runs observe it.
            NodeKind::Type,
        ];
        format!(
            "SELECT node_id, node_kind, module_node_id, existence_fact_id FROM ( \
               SELECT *, row_number() OVER (PARTITION BY node_id, node_kind \
                                            ORDER BY existence_fact_id \
                                            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) \
                        AS pick \
               FROM ({union}) u) r \
             WHERE pick = 1 OR node_kind NOT IN ({})",
            kinds(&merged)
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

table!(
    /// Raw rows the graph does not represent yet, each with its reason (DESIGN §3.7; review F5).
    /// C1 published Pysa's non-call sites here until C2's syntax nodes and C3's references carried
    /// them; since C3 it is empty by construction, kept as the published place a future
    /// unrepresented class lands, so nothing is omitted silently. Its partition rule is an edit
    /// guard until then ([`crate::rules::EDIT_GUARDS`]).
    GraphGaps, GraphGapsRow = "graph_gaps",
    family = Graph,
    key = [snapshot_id, gap_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        /// The raw row's fact.
        gap_fact_id: Id,
        table_name: String,
        reason: BoundaryReason,
        /// The site class and the slice that will represent it.
        detail: String,
    }
);

impl crate::derived::Derived for GraphGaps {
    fn sql() -> String {
        format!(
            "SELECT fact_id AS gap_fact_id, 'pysa_calls' AS table_name, \
                    CAST({not_requested} AS SMALLINT) AS reason, \
                    'identifier site: C3 lexical references' AS detail \
             FROM pysa_calls \
             WHERE NOT ({call_site}) AND callee_kind = {identifier} AND FALSE",
            not_requested = c(BoundaryReason::NotRequested),
            identifier = c(PysaCalleeKind::Identifier),
            call_site = call_site_rows(),
        )
    }
}

table!(
    /// The edge registry as data (DESIGN §3.8; review F6): each kind's derivation class, direction
    /// meaning, parallel policy, evidence table and endpoint kinds, published with every snapshot
    /// so a projection selects by them from the store, not from its own build.
    EdgeKinds, EdgeKindsRow = "edge_kinds",
    family = Graph,
    key = [snapshot_id, edge_kind],
    checks = [],
    {
        snapshot_id: Id,
        edge_kind: EdgeKind,
        derivation: DerivationClass,
        direction: String,
        parallel: bool,
        evidence_table: String,
        /// Allowed endpoint kinds, as `node_kind` texts joined by `,`.
        src_kinds: String,
        dst_kinds: String,
    }
);

impl crate::derived::Derived for EdgeKinds {
    fn sql() -> String {
        let names = |ks: &[NodeKind]| ks.iter().map(|k| k.text()).collect::<Vec<_>>().join(",");
        let rows = edge_sources()
            .iter()
            .map(|e| {
                format!(
                    "SELECT CAST({} AS SMALLINT) AS edge_kind, CAST({} AS SMALLINT) AS derivation, \
                            '{}' AS direction, {} AS parallel, '{}' AS evidence_table, \
                            '{}' AS src_kinds, '{}' AS dst_kinds",
                    e.kind.code(),
                    e.derivation.code(),
                    e.direction.replace('\'', "''"),
                    e.parallel,
                    e.evidence_table,
                    names(e.src),
                    names(e.dst),
                )
            })
            .collect::<Vec<_>>();
        rows.join(" UNION ALL ")
    }
}

/// Pysa's records at call sites: a regular site with a call callee.
fn call_site_rows() -> String {
    format!(
        "site_kind = {} AND callee_kind = {}",
        c(PysaSiteKind::Regular),
        c(PysaCalleeKind::Call)
    )
}

/// A node-valued column of a family table, and the node kinds it may name (DESIGN §3.8, §8):
/// the references are generated from here, checked against `nodes`, so the registry and the
/// references are one authority (slice-2 review O8; ADR-0014 review F4).
pub struct NodeColumn {
    pub table: &'static str,
    pub column: &'static str,
    pub kinds: &'static [NodeKind],
}

/// Every node-valued column, except each kind's own existence column.
pub fn node_columns() -> Vec<NodeColumn> {
    use NodeKind as N;
    let nc = |table, column, kinds| NodeColumn {
        table,
        column,
        kinds,
    };
    const MODULE: &[NodeKind] = &[NodeKind::Module];
    const DECL: &[NodeKind] = &[NodeKind::Class, NodeKind::Function];
    const CALLABLE: &[NodeKind] = &[
        NodeKind::Function,
        NodeKind::SyntheticCallable,
        NodeKind::ExternalSymbol,
    ];
    vec![
        nc("declarations", "module_node_id", MODULE),
        nc("declarations", "parent_node_id", DECL),
        nc("export_syntax", "module_node_id", MODULE),
        nc("public_names", "origin_module_node_id", MODULE),
        nc("public_names", "access_module_node_id", MODULE),
        nc("parameter_syntax", "function_node_id", &[N::Function]),
        nc("parameter_docs", "function_node_id", &[N::Function]),
        nc("parameter_docs", "module_node_id", &[N::Module]),
        nc("pysa_functions", "module_node_id", MODULE),
        nc("parameter_semantics", "module_node_id", MODULE),
        nc("class_ancestry", "module_node_id", MODULE),
        nc("pysa_classes", "module_node_id", MODULE),
        nc("call_syntax", "module_node_id", MODULE),
        nc("call_syntax", "owner_node_id", DECL),
        nc("arguments", "call_node_id", &[N::CallSite]),
        nc("pysa_calls", "module_node_id", MODULE),
        nc("coverage", "scope_node_id", &[N::Module, N::Document]),
        nc("boundaries", "module_node_id", MODULE),
        nc(
            "boundaries",
            "subject_node_id",
            &[N::Class, N::Function, N::CallSite],
        ),
        nc(
            "context_definitions",
            "module_node_id",
            &[N::ExternalModule],
        ),
        nc("provider_node_map", "module_node_id", MODULE),
        nc("provider_node_map", "node_id", &[N::Function]),
        nc("provider_class_map", "module_node_id", MODULE),
        nc("provider_class_map", "node_id", &[N::Class]),
        nc("synthetic_callables", "module_node_id", MODULE),
        nc("exports", "declaration_node_id", DECL),
        nc(
            "exports",
            "target_node_id",
            &[
                N::Class,
                N::Function,
                N::ExternalSymbol,
                N::Module,
                N::ExternalModule,
                N::Binding,
            ],
        ),
        nc("signatures", "signature_node_id", &[N::Function]),
        nc("signatures", "callable_node_id", &[N::Function]),
        nc("signatures", "module_node_id", MODULE),
        nc("parameters", "signature_node_id", &[N::Function]),
        nc("resolutions", "call_site_node_id", &[N::CallSite]),
        nc("argument_resolutions", "argument_node_id", &[N::Argument]),
        nc("call_targets", "call_site_node_id", &[N::CallSite]),
        nc("call_targets", "argument_node_id", &[N::Argument]),
        nc("call_targets", "target_node_id", CALLABLE),
        nc("ancestry_targets", "class_node_id", &[N::Class]),
        nc(
            "ancestry_targets",
            "ancestor_node_id",
            &[N::Class, N::ExternalSymbol],
        ),
        nc(
            "override_targets",
            "function_node_id",
            &[N::Function, N::SyntheticCallable],
        ),
        nc("override_targets", "overridden_node_id", CALLABLE),
        nc("nodes", "module_node_id", &[N::Module, N::ExternalModule]),
        // C2
        nc("syntax_nodes", "module_node_id", MODULE),
        nc("syntax_nodes", "owner_node_id", DECL),
        nc(
            "syntax_nodes",
            "parent_node_id",
            &[N::Module, N::Class, N::Function, N::CallSite, N::SyntaxNode],
        ),
        nc(
            "site_targets",
            "site_node_id",
            &[N::SyntaxNode, N::CallSite],
        ),
        nc("site_targets", "target_node_id", CALLABLE),
        // C3
        nc("scopes", "module_node_id", MODULE),
        nc(
            "scopes",
            "owner_node_id",
            &[N::Module, N::Class, N::Function, N::SyntaxNode],
        ),
        nc("scopes", "parent_scope_id", &[N::Scope]),
        nc("bindings", "scope_id", &[N::Scope]),
        nc("bindings", "module_node_id", MODULE),
        nc("references", "scope_id", &[N::Scope]),
        // The reference is a role of its placed name (annotation names are C4's, not references).
        nc("references", "name_node_id", &[N::SyntaxNode]),
        nc("references", "module_node_id", MODULE),
        nc(
            "references",
            "parent_node_id",
            &[N::Module, N::Class, N::Function, N::CallSite, N::SyntaxNode],
        ),
        nc("reference_resolutions", "reference_id", &[N::Reference]),
        nc("reference_resolutions", "binding_id", &[N::Binding]),
        nc("identifier_targets", "reference_node_id", &[N::Reference]),
        nc("identifier_targets", "target_node_id", CALLABLE),
        nc("import_targets", "module_node_id", MODULE),
        nc(
            "import_targets",
            "target_node_id",
            &[N::Module, N::ExternalModule],
        ),
        // C4
        nc("type_binders", "term_node_id", &[N::Type]),
        nc(
            "type_binders",
            "binder_node_id",
            &[N::Class, N::Function, N::SyntaxNode],
        ),
        nc("type_term_args", "parent_node_id", &[N::Type]),
        nc("type_term_args", "child_node_id", &[N::Type]),
        nc("type_observations", "module_node_id", MODULE),
        nc(
            "type_observations",
            "subject_node_id",
            &[
                N::Parameter,
                N::Function,
                N::CallSite,
                N::Argument,
                N::SyntaxNode,
            ],
        ),
        nc("type_observations", "term_node_id", &[N::Type]),
        nc("record_fields", "class_node_id", &[N::Class]),
        nc("record_fields", "module_node_id", MODULE),
        nc("record_fields", "term_node_id", &[N::Type]),
        nc("type_class_targets", "term_node_id", &[N::Type]),
        nc(
            "type_class_targets",
            "class_node_id",
            &[N::Class, N::ExternalSymbol],
        ),
        // C5
        nc("passages", "document_node_id", &[N::Document]),
        nc("code_blocks", "document_node_id", &[N::Document]),
        nc("code_blocks", "passage_node_id", &[N::Passage]),
        nc("doc_links", "passage_node_id", &[N::Passage]),
        nc("mentions", "passage_node_id", &[N::Passage]),
        nc("mention_targets", "passage_node_id", &[N::Passage]),
        nc(
            "mention_targets",
            "target_node_id",
            &[N::Export, N::Class, N::Function],
        ),
    ]
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
    for n in node_columns() {
        out.push(rule(
            format!("ref:{}.{}->nodes", n.table, n.column),
            format!(
                "SELECT f.{c} AS value FROM {t} f LEFT ANTI JOIN \
                   (SELECT node_id FROM nodes WHERE node_kind IN ({k})) r ON f.{c} = r.node_id \
                 WHERE f.{c} IS NOT NULL",
                t = n.table,
                c = n.column,
                k = kinds(n.kinds)
            ),
        ));
    }
    // The ids the extractor computes in Rust equal the same recipe in SQL (review F3).
    for (name, sql) in [
        (
            "id:arguments",
            "SELECT node_id FROM arguments \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('argument', call_node_id, ordinal) AS BYTEA)",
        ),
        (
            "id:context_definitions",
            "SELECT symbol_node_id FROM context_definitions \
             WHERE CAST(symbol_node_id AS BYTEA) \
                <> CAST(lctx_id('external_symbol', module_node_id, kind, key) AS BYTEA)",
        ),
        (
            "id:scopes",
            "SELECT node_id FROM scopes \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('scope', owner_node_id) AS BYTEA)",
        ),
        (
            "id:bindings",
            "SELECT node_id FROM bindings \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('binding', site_node_id, name) AS BYTEA)",
        ),
        (
            "id:references",
            "SELECT node_id FROM references \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('reference', name_node_id) AS BYTEA)",
        ),
        (
            "id:record_fields",
            "SELECT node_id FROM record_fields \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('field', class_node_id, name) AS BYTEA)",
        ),
        (
            "id:documents",
            "SELECT node_id FROM documents \
             WHERE CAST(node_id AS BYTEA) <> CAST(lctx_id('document', release_id, path) AS BYTEA)",
        ),
        (
            "id:passages",
            "SELECT node_id FROM passages \
             WHERE CAST(node_id AS BYTEA) \
                <> CAST(lctx_id('passage', document_node_id, ordinal) AS BYTEA)",
        ),
        (
            "id:code_blocks",
            "SELECT node_id FROM code_blocks \
             WHERE CAST(node_id AS BYTEA) \
                <> CAST(lctx_id('code_block', document_node_id, ordinal) AS BYTEA)",
        ),
        (
            "id:context_modules",
            "SELECT module_node_id FROM context_modules WHERE distribution IS NOT NULL \
               AND CAST(module_node_id AS BYTEA) \
                <> CAST(lctx_id('external_module', distribution, version, module_name) AS BYTEA)",
        ),
    ] {
        out.push(rule(name.to_owned(), sql.to_owned()));
    }
    // Every Pysa row is in exactly one place (review F5): a call-site row an edge or a reason
    // accounts for (the lineage rules), an unresolved remainder counted on its resolution, or a
    // gap the snapshot publishes.
    out.push(rule(
        "partition:pysa_calls-gaps".to_owned(),
        "SELECT g.gap_fact_id FROM graph_gaps g \
         LEFT ANTI JOIN pysa_calls p ON p.fact_id = g.gap_fact_id \
         WHERE g.table_name = 'pysa_calls'"
            .to_owned(),
    ));
    out.push(rule(
        "partition:pysa_calls-remainders".to_owned(),
        format!(
            "WITH rem AS ( \
               SELECT p.fact_id, p.higher_order_index, c.node_id AS call_node_id \
               FROM pysa_calls p LEFT JOIN call_syntax c \
                 ON c.module_node_id = p.module_node_id AND c.start_byte = p.start_byte \
                AND c.end_byte = p.end_byte \
               WHERE {rows} AND p.target_kind = {unresolved}), \
             counted AS ( \
               SELECT call_site_node_id AS call_node_id, CAST(NULL AS BIGINT) AS ordinal \
               FROM resolutions WHERE has_unresolved_remainder \
               UNION ALL \
               SELECT a.call_node_id, a.ordinal FROM argument_resolutions x \
               JOIN arguments a ON a.node_id = x.argument_node_id \
               WHERE x.has_unresolved_remainder) \
             SELECT r.fact_id FROM rem r LEFT ANTI JOIN counted k \
               ON k.call_node_id = r.call_node_id \
              AND ((r.higher_order_index IS NULL AND k.ordinal IS NULL) \
                   OR k.ordinal = r.higher_order_index)",
            rows = call_site_rows(),
            unresolved = c(PysaTargetKind::Unresolved),
        ),
    ));
    // C2: every declaration, and every call outside an annotation, is placed; a child lies within
    // its placed parent, in the same module.
    out.push(rule(
        "placed:declarations".to_owned(),
        "SELECT d.fact_id FROM declarations d LEFT ANTI JOIN syntax_nodes s ON s.node_id = d.node_id"
            .to_owned(),
    ));
    out.push(rule(
        "placed:call_syntax".to_owned(),
        "SELECT c.fact_id FROM call_syntax c LEFT ANTI JOIN syntax_nodes s ON s.node_id = c.node_id \
         WHERE NOT c.in_annotation"
            .to_owned(),
    ));
    out.push(rule(
        "unique:syntax_nodes".to_owned(),
        "SELECT node_id, count(*) AS n FROM syntax_nodes GROUP BY node_id HAVING count(*) > 1"
            .to_owned(),
    ));
    out.push(rule(
        "contained:syntax_nodes".to_owned(),
        "SELECT c.node_id FROM syntax_nodes c JOIN syntax_nodes p ON p.node_id = c.parent_node_id \
         WHERE c.start_byte < p.start_byte OR c.end_byte > p.end_byte \
            OR c.module_node_id <> p.module_node_id"
            .to_owned(),
    ));
    // C5b: an attempt's releases never share a release-relative path, so a `@path` module reference
    // names one file.
    // One type-term id is one term: two runs may both emit it, but never with a different kind,
    // detail or display (a Merkle-id collision is rejected, never merged; C4 review).
    out.push(rule(
        "unique:type_terms".to_owned(),
        "SELECT node_id FROM type_terms GROUP BY node_id \
         HAVING count(DISTINCT kind) > 1 OR count(DISTINCT display) > 1 \
             OR count(DISTINCT COALESCE(detail, '')) > 1"
            .to_owned(),
    ));
    out.push(rule(
        "unique:release-paths".to_owned(),
        "SELECT path, count(DISTINCT release_id) AS n FROM source_files GROUP BY path \
         HAVING count(DISTINCT release_id) > 1"
            .to_owned(),
    ));
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
        ("exports", "target_node_id IS NULL"),
        (
            "site_targets",
            "(site_node_id IS NULL OR target_node_id IS NULL)",
        ),
        (
            "identifier_targets",
            "(reference_node_id IS NULL OR target_node_id IS NULL)",
        ),
        ("import_targets", "target_node_id IS NULL"),
        (
            "reference_resolutions",
            "binding_id IS NULL AND builtin_name IS NULL",
        ),
        ("type_class_targets", "class_node_id IS NULL"),
        ("type_binders", "binder_node_id IS NULL"),
        ("mention_targets", "target_node_id IS NULL"),
    ] {
        out.push(rule(
            format!("typed:{table}"),
            format!("SELECT * FROM {table} WHERE {columns} AND reason IS NULL"),
        ));
    }
    out
}
