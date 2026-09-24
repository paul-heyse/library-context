//! The behavior model's Stage 1 tables (ADR-0021, ADR-0022; DESIGN §3.2, §3.9; the plan
//! `docs/plans/behavioral-model-pivot-plan_2026-09-24.md`, Stage 1): the whole public surface,
//! not the seeds.
//!
//! - **Persisted relations.** Pass B's and Pass C's declared relations (`cpg_schema::flows`),
//!   computed over the whole release on every compile, are written as analysis tables:
//!   `argument_flows`, `guards`, `parameter_reads`, `handoffs`. Each is the relation's own SQL,
//!   wrapped only to carry the snapshot id, so the relations (and Pass B's and C's digests) are
//!   unchanged.
//! - **`delegations`**: every depth-1 call arc of a public callable (the invocation projection's
//!   accepted evidence, `flows::arcs`).
//! - **`operations`**: one row per public node, at its preferred path, with its docstring summary.
//! - **`operation_facets`**: `find_operations`'s typed facets per public node, each with the best
//!   verdict among the rows it comes from.
//! - **`operation_facet_status`**: per public node and facet, whether its facet rows are complete
//!   (`established`) or why not: the served authority for `find_operations`' `complete` and its
//!   `unknown` list (increment 3's deep review, F3 and F4).
//! - **`behaviors`**: what each public operation does with its parameters, whom it delegates to and
//!   what official usage hands it, each row with a [`Verdict`](crate::codebook::Verdict) and, when
//!   it is not established, its boundary reason.
//! - **`behavior_steps`**: each behavior's call path, hop by hop with its modality (the review's
//!   F6): what `explain` returns and what `semantic:established-needs-definite-path` reads.
//! - **`operation_documents`**: the texts `search_operations` embeds, one row per view and chunk.
//!
//! All are analysis tables (ADR-0019): computed from the snapshot, the analytics config and the
//! compiler, never extracted, so they carry no `fact_id` and are not coverage units. What stands in
//! for coverage is `operations.behavior_status`: every public callable is analyzed or says why not
//! (`semantic:behavior-covers-public`).

use crate::codebook::{
    BehaviorKind, BoundaryReason, Codebook, DeclarationKind, EmbeddingView, InvocationPhase,
    Modality, OperationFacet, SourceRole, ValueClass, Verdict,
};
use crate::id::{Digest, Id, IdHasher};
use crate::table::table;

table!(
    /// Pass B's argument flows (§9.2), persisted: each argument of a `call` or `init` arc mapped
    /// to one formal of its target, with how its value arises and where the call sits.
    ArgumentFlows, ArgumentFlowsRow = "argument_flows",
    family = Findings,
    key = [snapshot_id, edge_id, argument_node_id, formal_node_id],
    checks = [],
    {
        snapshot_id: Id,
        caller_node_id: Id,
        call_site_node_id: Id,
        target_node_id: Id,
        edge_id: Id,
        modality: Modality,
        phase: InvocationPhase,
        argument_node_id: Id,
        formal_node_id: Id,
        formal_name: String,
        value_class: ValueClass,
        source_parameter_node_id: Option<Id>,
        alias_name: Option<String>,
        value_text: Option<String>,
        may_catch: bool,
        conditional: bool,
        value_tested: bool,
    }
);

table!(
    /// Pass B's guards (§9.2), persisted: an `if` directly in a function's body whose test is a
    /// supported predicate over a parameter, with a `raise` directly in its branch.
    Guards, GuardsRow = "guards",
    family = Findings,
    key = [snapshot_id, if_node_id, raise_node_id, parameter_node_id],
    checks = [],
    {
        snapshot_id: Id,
        function_node_id: Id,
        if_node_id: Id,
        test_node_id: Id,
        raise_node_id: Id,
        parameter_node_id: Id,
        module_node_id: Id,
        test_start_byte: i64,
        test_end_byte: i64,
        raise_start_byte: i64,
        raise_end_byte: i64,
    }
);

table!(
    /// Pass B's parameter reads (§9.2 review F4), persisted: each argument of an arc whose value
    /// reads a name the caller binds under one of its parameters' names.
    ParameterReads, ParameterReadsRow = "parameter_reads",
    family = Findings,
    key = [snapshot_id, edge_id, argument_node_id, parameter_node_id],
    checks = [],
    {
        snapshot_id: Id,
        caller_node_id: Id,
        call_site_node_id: Id,
        target_node_id: Id,
        edge_id: Id,
        modality: Modality,
        phase: InvocationPhase,
        argument_node_id: Id,
        parameter_node_id: Id,
        rebound: bool,
        bare: bool,
        unpacked: bool,
    }
);

table!(
    /// Pass C's handoff occurrences (§9.3), persisted: in official usage code, `x = producer(...)`
    /// then `consumer(..., x)`, or `consumer(..., producer(...))`.
    Handoffs, HandoffsRow = "handoffs",
    family = Findings,
    key = [
        snapshot_id,
        consumer_site_node_id,
        consumer_node_id,
        producer_site_node_id,
        producer_node_id,
        formal_node_id,
    ],
    checks = [],
    {
        snapshot_id: Id,
        consumer_node_id: Id,
        producer_node_id: Id,
        formal_node_id: Id,
        formal_name: String,
        path: String,
        role: SourceRole,
        consumer_start_byte: i64,
        consumer_site_node_id: Id,
        producer_site_node_id: Id,
        named: bool,
        consumer_modality: Modality,
    }
);

table!(
    /// Every depth-1 call arc of a public callable: the invocation projection's accepted call and
    /// `init` arcs into release functions, candidates (override-open dispatch) included with their
    /// modality.
    Delegations, DelegationsRow = "delegations",
    family = Findings,
    key = [snapshot_id, caller_node_id, edge_id],
    checks = [],
    {
        snapshot_id: Id,
        caller_node_id: Id,
        call_site_node_id: Id,
        target_node_id: Id,
        edge_id: Id,
        modality: Modality,
        phase: InvocationPhase,
    }
);

table!(
    /// One row per public node (`public_paths`), at its preferred path: what `get_operation` and
    /// `find_operations` start from.
    Operations, OperationsRow = "operations",
    family = Findings,
    key = [snapshot_id, node_id],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        access_path: String,
        kind: DeclarationKind,
        /// A `def` inside a class.
        is_method: bool,
        qualified_name: String,
        module: String,
        /// The docstring's first paragraph, whitespace collapsed; none without a docstring.
        docstring_summary: Option<String>,
        /// What the behavior scan's rows can support: `established` when the scan from this
        /// callable met no boundary in its **region** (the callables it reached): no depth cut and
        /// no read of a tracked formal at the frontier, no override-open call in the region or of
        /// its own, no open call site taking a tracked value or of its own, and no read it does
        /// not follow; otherwise `unknown` (then negative answers about it are unknown);
        /// `not_analyzed` for a class (its controls are its `__init__`'s). Increment 3's deep
        /// review, F2.
        behavior_status: Verdict,
        /// The first boundary met, in the order budget, override dispatch, open site, unfollowed
        /// read.
        boundary_reason: Option<BoundaryReason>,
        /// Why the status is not `established`, every boundary met, in words.
        status_reason: Option<String>,
    }
);

table!(
    /// A typed facet of a public operation, for `find_operations`.
    OperationFacets, OperationFacetsRow = "operation_facets",
    family = Findings,
    key = [snapshot_id, node_id, facet, value],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        facet: OperationFacet,
        value: String,
        /// `established` for a declared facet; for a behavioral one, the best verdict among the
        /// behaviors it comes from (`established`, then `conditional`, then `unknown`). Only
        /// `established` and `conditional` rows match; an `unknown` row puts its operation in the
        /// `unknown` list.
        verdict: Verdict,
    }
);

table!(
    /// Whether a public node's rows for a facet are complete (increment 3's deep review, F3, F4):
    /// one row per public node and facet. `established`: every value it has is a row;
    /// otherwise the verdict and why (a class's constructor is not public, a behavior's region is
    /// not closed, a facet is never complete). `find_operations` is `complete` only when every
    /// operation in its universe that does not match is `established` for every facet it asks.
    OperationFacetStatus, OperationFacetStatusRow = "operation_facet_status",
    family = Findings,
    key = [snapshot_id, node_id, facet],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        facet: OperationFacet,
        verdict: Verdict,
        reason: Option<String>,
    }
);

table!(
    /// What a public operation does (ADR-0022): with its parameters (Pass B's worklist from the
    /// operation), whom it calls, and what official usage hands it. Every row carries a verdict.
    Behaviors, BehaviorsRow = "behaviors",
    family = Findings,
    key = [snapshot_id, behavior_id],
    checks = [("depth_nonnegative", "depth >= 0")],
    {
        snapshot_id: Id,
        behavior_id: Id,
        operation_node_id: Id,
        kind: BehaviorKind,
        /// The operation's parameter the row is about (`forwards`, `raises_when`, `unfollowed`).
        parameter_node_id: Option<Id>,
        parameter_name: Option<String>,
        /// The callable the path ends in (a callee, or the handoff partner).
        callee_node_id: Option<Id>,
        /// What the path reaches there: a formal, a `raise`, or none.
        target_node_id: Option<Id>,
        /// The formal's name, or the handoff's formal.
        target_name: Option<String>,
        /// A literal as written (`supplies_literal`), the unfollowed reason, or a delegation's
        /// modality when it is not definite. A guard's test text is `site_text`.
        value: Option<String>,
        /// Call steps from the operation (0 for a handoff).
        depth: i64,
        /// A call on the path is made only on some paths of its caller.
        conditional: bool,
        verdict: Verdict,
        /// Why the verdict is `unknown`: `override_dispatch` (a hop through a candidate arc),
        /// `ambiguous_binding` (a potential arc), `outside_provider_model` (a read in a form not
        /// followed). Null when established or conditional.
        boundary_reason: Option<BoundaryReason>,
        /// The syntax node that shows it: the last call site, the `if` test, or the usage site.
        site_node_id: Option<Id>,
        /// Where the site is: its module, byte span, 1-based line and verbatim text.
        site_module_node_id: Option<Id>,
        site_start_byte: Option<i64>,
        site_end_byte: Option<i64>,
        site_line: Option<i64>,
        site_text: Option<String>,
        /// How many occurrences stand behind a handoff row (1 otherwise).
        occurrences: i64,
        invocation_id: Option<Id>,
    }
);

table!(
    /// Each behavior's call path from its operation, one row per hop (increment 3's deep review,
    /// F6): Pass B's witness path, or a delegation's one arc. A handoff has none.
    BehaviorSteps, BehaviorStepsRow = "behavior_steps",
    family = Findings,
    key = [snapshot_id, behavior_id, step],
    checks = [("step_nonnegative", "step >= 0")],
    {
        snapshot_id: Id,
        behavior_id: Id,
        step: i64,
        caller_node_id: Id,
        call_site_node_id: Id,
        callee_node_id: Id,
        modality: Modality,
        /// The caller makes this call only on some of its paths.
        conditional: bool,
    }
);

table!(
    /// The texts `search_operations` embeds (ADR-0010 amendment, 2026-09-24): one row per view and
    /// chunk, with the cache key once embedded. Every view shares the snapshot's one spec.
    OperationDocuments, OperationDocumentsRow = "operation_documents",
    family = Findings,
    key = [snapshot_id, node_id, embedding_view, chunk],
    checks = [("chunk_nonnegative", "chunk >= 0")],
    {
        snapshot_id: Id,
        node_id: Id,
        embedding_view: EmbeddingView,
        chunk: i64,
        text: String,
        spec_hash: Option<Digest>,
        input_hash: Option<Digest>,
    }
);

/// A behavior row's content id: the operation, what it states and where, never the run (§3.4.1).
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per identity column of a behavior row"
)]
pub fn behavior_id(
    operation: Id,
    kind: BehaviorKind,
    parameter: Option<Id>,
    callee: Option<Id>,
    target: Option<Id>,
    value: Option<&str>,
    site: Option<Id>,
) -> Id {
    IdHasher::new("behavior")
        .id(operation)
        .i64(i64::from(kind.code()))
        .opt_id(parameter)
        .opt_id(callee)
        .opt_id(target)
        .opt_str(value)
        .opt_id(site)
        .finish_id()
}

/// A relation's rows with the attempt's snapshot id: every table a session registers holds one
/// snapshot (§6.2), so `source_files` names it.
fn with_snapshot(sql: &str) -> String {
    format!(
        "SELECT s.snapshot_id, r.* FROM ({sql}) r \
         CROSS JOIN (SELECT DISTINCT snapshot_id FROM source_files) s"
    )
}

crate::relations! {
    inventory all;

    /// `argument_flows`: `flows::argument_flows_sql`, with the snapshot id.
    argument_flows = "behavior:argument_flows",
        deps = ["bindings", "edges", "pysa_calls", "facts", "arguments", "parameter_syntax",
                "syntax_nodes", "declarations", "source_files"],
        sql = with_snapshot(&crate::flows::argument_flows_sql());

    /// `guards`: `flows::guards_sql`, with the snapshot id.
    guards = "behavior:guards",
        deps = ["bindings", "syntax_nodes", "references", "reference_resolutions", "declarations",
                "source_files"],
        sql = with_snapshot(&crate::flows::guards_sql());

    /// `parameter_reads`: `flows::parameter_reads_sql`, with the snapshot id.
    parameter_reads = "behavior:parameter_reads",
        deps = ["bindings", "edges", "pysa_calls", "facts", "arguments", "references",
                "reference_resolutions", "syntax_nodes", "source_files"],
        sql = with_snapshot(&crate::flows::parameter_reads_sql());

    /// `handoffs`: `flows::handoffs_sql`, with the snapshot id.
    handoffs = "behavior:handoffs",
        deps = ["bindings", "edges", "pysa_calls", "facts", "arguments", "parameter_syntax",
                "references", "reference_resolutions", "syntax_nodes", "declarations",
                "source_files", "code_blocks", "documents"],
        sql = with_snapshot(&crate::flows::handoffs_sql());

    /// `delegations`: the depth-1 arcs of every public callable.
    delegations = "behavior:delegations",
        deps = ["edges", "pysa_calls", "facts", "public_paths", "source_files"],
        sql = with_snapshot(&format!(
            "SELECT a.caller_node_id, a.call_site_node_id, a.target_node_id, a.edge_id, \
                    a.modality, a.phase \
             FROM ({arcs}) a \
             JOIN (SELECT DISTINCT node_id FROM public_paths) p ON p.node_id = a.caller_node_id",
            arcs = crate::flows::arcs(),
        ));

    /// The public nodes at their preferred path, with what `operations` is built from.
    operation_sources = "behavior:operation_sources",
        deps = ["public_paths", "declarations", "source_files"],
        sql = format!(
            "SELECT p.node_id, p.access_path, d.kind, d.qualified_name, sf.module_name AS module, \
                    d.docstring, (pd.kind IS NOT NULL AND pd.kind = {class}) AS is_method, \
                    d.module_node_id, d.start_byte, d.end_byte, d.decorators \
             FROM public_paths p \
             JOIN declarations d ON d.node_id = p.node_id \
             JOIN source_files sf ON sf.module_node_id = d.module_node_id AND sf.role = {release} \
             LEFT JOIN declarations pd ON pd.node_id = d.parent_node_id \
             WHERE p.preferred",
            class = DeclarationKind::Class.code(),
            release = SourceRole::Release.code(),
        );
}

crate::relations! {
    inventory boundaries;

    /// Each caller's call sites that resolution leaves open: unresolved, partial, or with an
    /// unresolved remainder (§3.6).
    open_sites = "behavior:open_sites",
        deps = ["edges", "resolutions"],
        sql = format!(
            "SELECT ec.src_node_id AS node_id, count(*) AS sites FROM edges ec \
             JOIN resolutions r ON r.call_site_node_id = ec.dst_node_id \
             WHERE ec.edge_kind = {encloses} \
               AND (r.status <> {resolved} OR r.has_unresolved_remainder) \
             GROUP BY ec.src_node_id ORDER BY ec.src_node_id",
            encloses = crate::codebook::EdgeKind::EnclosesCall.code(),
            resolved = crate::codebook::ResolutionStatus::Resolved.code(),
        );
}

crate::relations! {
    inventory open_reads;

    /// Call sites resolution leaves open (unresolved, partial, or with an unresolved remainder)
    /// whose arguments read a name the caller binds as a parameter (increment 3's deep review,
    /// F2): a tracked value that leaves the analysis there. The mapping from an argument's value
    /// to the names it reads is `flows::parameter_reads_sql`'s.
    open_site_reads = "behavior:open_site_reads",
        deps = ["edges", "resolutions", "arguments", "syntax_nodes", "references",
                "reference_resolutions", "bindings"],
        sql = format!(
            "WITH open AS ( \
               SELECT ec.src_node_id AS caller_node_id, r.call_site_node_id FROM edges ec \
               JOIN resolutions r ON r.call_site_node_id = ec.dst_node_id \
               WHERE ec.edge_kind = {encloses} \
                 AND (r.status <> {resolved} OR r.has_unresolved_remainder)), \
             declared AS ( \
               SELECT b.site_node_id AS parameter_node_id, b.scope_id, b.name FROM bindings b \
               WHERE b.kind = {parameter}), \
             valued AS ( \
               SELECT o.caller_node_id, o.call_site_node_id, s.module_node_id, s.owner_node_id, \
                      s.start_byte, s.end_byte \
               FROM open o JOIN arguments a ON a.call_node_id = o.call_site_node_id \
               JOIN edges v ON v.edge_kind = {argument_value} AND v.src_node_id = a.node_id \
               JOIN syntax_nodes s ON s.node_id = v.dst_node_id) \
             SELECT DISTINCT v.caller_node_id, v.call_site_node_id, d.parameter_node_id \
             FROM valued v \
             JOIN syntax_nodes n ON n.module_node_id = v.module_node_id \
                  AND n.owner_node_id = v.owner_node_id AND n.kind = {name} \
                  AND n.start_byte >= v.start_byte AND n.end_byte <= v.end_byte \
             JOIN references rf ON rf.name_node_id = n.node_id \
             JOIN reference_resolutions rr ON rr.reference_id = rf.node_id AND NOT rr.captured \
             JOIN bindings b ON b.node_id = rr.binding_id \
             JOIN declared d ON d.scope_id = b.scope_id AND d.name = b.name \
             ORDER BY 1, 2, 3",
            encloses = crate::codebook::EdgeKind::EnclosesCall.code(),
            resolved = crate::codebook::ResolutionStatus::Resolved.code(),
            parameter = crate::codebook::BindingKind::Parameter.code(),
            argument_value = crate::codebook::EdgeKind::ArgumentValue.code(),
            name = crate::codebook::SyntaxKind::ExprName.code(),
        );
}

crate::query_row! {
    /// An open call site taking a value the caller binds as a parameter.
    pub struct OpenSiteReadRow {
        caller_node_id: Id,
        call_site_node_id: Id,
        parameter_node_id: Id,
    }
}

crate::query_row! {
    /// A caller with call sites resolution leaves open.
    pub struct OpenSitesRow {
        node_id: Id,
        sites: i64,
    }
}

crate::query_row! {
    /// A public node with what `operations` and the embedded views are built from.
    pub struct OperationSourceRow {
        node_id: Id,
        access_path: String,
        kind: DeclarationKind,
        qualified_name: String,
        module: String,
        docstring: Option<String>,
        is_method: bool,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        /// The declaration's decorators' trailing names, in source order.
        decorators: Vec<String>,
    }
}

/// A docstring's first paragraph, whitespace collapsed; none when it is empty.
pub fn docstring_summary(docstring: &str) -> Option<String> {
    let first = docstring
        .trim()
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!first.is_empty()).then_some(first)
}

/// The relations' identity, for the compiler digest and the behavior scan's invocation.
pub fn digest() -> Digest {
    let mut h = IdHasher::new("behavior-relations");
    for r in all().into_iter().chain(boundaries()).chain(open_reads()) {
        h.str(r.name).str(&r.sql);
    }
    h.finish_digest()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_is_the_first_paragraph_collapsed() {
        assert_eq!(
            docstring_summary("  Add a tool.\n\n    Args:\n  x: y").as_deref(),
            Some("Add a tool.")
        );
        assert_eq!(
            docstring_summary("Two\n   lines.\n\nMore.").as_deref(),
            Some("Two lines.")
        );
        assert_eq!(docstring_summary("  \n "), None);
    }

    #[test]
    fn behavior_ids_follow_their_content() {
        let op = Id([1; 16]);
        let a = behavior_id(op, BehaviorKind::Forwards, None, None, None, None, None);
        let b = behavior_id(op, BehaviorKind::Delegates, None, None, None, None, None);
        let c = behavior_id(op, BehaviorKind::Forwards, None, None, None, Some(""), None);
        assert_ne!(a, b);
        assert_ne!(a, c, "an absent value is not an empty one");
    }
}
