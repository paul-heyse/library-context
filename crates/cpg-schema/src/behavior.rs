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
    BehaviorKind, BoundaryReason, Codebook, DeclarationKind, DynamicKind, EmbeddingView,
    ExactValueOrigin, ExitSiteKind, FlowSink, HandlerTypeStatus, InvocationPhase, Modality,
    ModelCallbackAction, ModelChannelCoverage, ModelEffectKind, ModelExceptionAction, ModelExit,
    ModelPathRole, ModelResourceAction, ModelTransferKind, OperationFacet, Origin, PremiseKind,
    ReadPhase, SourceRole, SyntaxKind, TestValueLinkOrigin, ValueClass, Verdict,
};
use crate::id::{Digest, Id, IdHasher};
use crate::table::table;

table!(
    /// An authored effect for a pinned external definition. Optional argument and subject path
    /// keep variant data separate from the closed effect kind. This is a model assertion, not a
    /// source-observed or composed effect of a release operation.
    ModelEffects, ModelEffectsRow = "model_effects",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        effect: ModelEffectKind,
        argument: Option<String>,
        subject_path_id: Option<Id>,
        subject_path: Option<String>,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// Authored callback action of a pinned external definition. `callback_path_id` is the
    /// canonical typed path's identity; `callback_path` is display only.
    ModelCallbacks, ModelCallbacksRow = "model_callbacks",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        callback_path_id: Id,
        callback_path: String,
        action: ModelCallbackAction,
        exit: ModelExit,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// Authored resource action of a pinned external definition. The typed path may refer to an
    /// input or a returned resource, and the exit determines when the action is promised.
    ModelResources, ModelResourcesRow = "model_resources",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        resource_path_id: Id,
        resource_path: String,
        resource_role: ModelPathRole,
        action: ModelResourceAction,
        exit: ModelExit,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// Authored exception action of a pinned external definition. A conversion must name its
    /// replacement class; the class names remain unresolved until summary application.
    ModelExceptions, ModelExceptionsRow = "model_exceptions",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        class: String,
        action: ModelExceptionAction,
        to_class: Option<String>,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// A syntactic except clause and its type expression (null for bare except). The condition
    /// reaches the try statement, not the handler: this row does not prove a catch.
    HandlerClauses, HandlerClausesRow = "handler_clauses",
    family = Findings,
    key = [snapshot_id, handler_node_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        function_node_id: Id,
        try_node_id: Id,
        handler_node_id: Id,
        ordinal: i64,
        module_node_id: Id,
        try_fact_id: Id,
        handler_fact_id: Id,
        type_node_id: Option<Id>,
        type_fact_id: Option<Id>,
        try_region_fact_id: Id,
        start_byte: i64,
        end_byte: i64,
        entry_condition_id: Id,
        entry_approximated: bool,
    }
);

table!(
    /// A direct statement in a handler body, with ty's local region. Nested actions belong to
    /// their own syntax parent. A listed action is not evidence it runs to completion.
    HandlerActions, HandlerActionsRow = "handler_actions",
    family = Findings,
    key = [snapshot_id, handler_node_id, action_node_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        function_node_id: Id,
        handler_node_id: Id,
        action_node_id: Id,
        action_kind: SyntaxKind,
        module_node_id: Id,
        action_fact_id: Id,
        region_fact_id: Id,
        start_byte: i64,
        end_byte: i64,
        condition_id: Id,
        approximated: bool,
    }
);

table!(
    /// Resolution of an authored except type. Only an exact lexical builtin name bound to one
    /// pinned context class gets `pinned_builtin`. This is class identity, not a catch verdict.
    HandlerTypes, HandlerTypesRow = "handler_types",
    family = Findings,
    key = [snapshot_id, handler_node_id],
    checks = [],
    {
        snapshot_id: Id,
        handler_node_id: Id,
        function_node_id: Id,
        type_node_id: Option<Id>,
        type_fact_id: Option<Id>,
        reference_fact_id: Option<Id>,
        resolution_fact_id: Option<Id>,
        class_node_id: Option<Id>,
        class_fact_id: Option<Id>,
        class_module_fact_id: Option<Id>,
        class_name: Option<String>,
        status: HandlerTypeStatus,
        reason: Option<BoundaryReason>,
    }
);

table!(
    /// Explicit return/raise statements and direct finally-body actions in release functions.
    /// These are attributed source/control sites, not a proof that an exception is handled or
    /// that a return is the only path. The flow region supplies the path condition.
    ExitSites, ExitSitesRow = "exit_sites",
    family = Findings,
    key = [snapshot_id, function_node_id, site_node_id, kind],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        function_node_id: Id,
        site_node_id: Id,
        kind: ExitSiteKind,
        module_node_id: Id,
        source_fact_id: Id,
        region_fact_id: Id,
        start_byte: i64,
        end_byte: i64,
        condition_id: Id,
        approximated: bool,
    }
);

table!(
    /// An authored model bound to a definition in the pinned analysis context. This is a target
    /// proof, not yet an applied transfer or effect summary. Overloads have distinct target nodes.
    ModelTargets, ModelTargetsRow = "model_targets",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id],
    checks = [],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        target_module_fact_id: Id,
        target_definition_fact_id: Id,
        target_key: String,
        revision: i64,
        transfer_coverage: ModelChannelCoverage,
        effect_coverage: ModelChannelCoverage,
        callback_coverage: ModelChannelCoverage,
        resource_coverage: ModelChannelCoverage,
        exception_coverage: ModelChannelCoverage,
        origin: Origin,
    }
);

table!(
    /// An authored unconditional transfer for a pinned target. Paths are written only by the
    /// typed catalog renderer; the original TOML rule and its revision determine `rule_id`.
    /// This is model evidence, not a source-observed flow or a composed summary.
    ModelTransfers, ModelTransfersRow = "model_transfers",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        input_path_id: Id,
        input_path: String,
        output_path_id: Id,
        output_path: String,
        transfer: ModelTransferKind,
        modality: Modality,
        origin: Origin,
    }
);

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
        /// followed), `dynamic_access` or `missing_evidence` (a negative claim's premise fails).
        /// Null when established or conditional.
        boundary_reason: Option<BoundaryReason>,
        /// The condition it holds under, in the operation's own places (Stage 2; the first hop's
        /// for a path); null for `true`.
        condition: Option<String>,
        /// A call's callee as written, when it is outside the release (`logger.info`).
        callee_text: Option<String>,
        /// When a setting is read (`reads_setting`).
        phase: Option<ReadPhase>,
        /// The negative premise a refuted or unknown `is_read` rests on (`negative_premises`).
        premise_key: Option<String>,
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
        /// The condition this hop's value reaches its callee under, in its caller's places.
        condition: Option<String>,
    }
);

table!(
    /// Where a function's parameter goes (Stage 2.6; ADR-0022): per sink (a call argument, a
    /// `return`, a `raise`, a stored field or dict entry), the parameter whose value reaches it
    /// through the flow IR's reaching definitions and value sources, **identity** (unchanged),
    /// derived, or only through a call (`through_call`: the callee's result may not carry it),
    /// under the condition it does so (a path condition in the function's own places). Per sink and
    /// source, the strongest transfer is kept.
    /// A read of an enclosing function's parameter inside a lambda or comprehension is followed
    /// through our resolution, flow-insensitively (`captured`).
    ValueFlows, ValueFlowsRow = "value_flows",
    family = Findings,
    key = [snapshot_id, module_node_id, sink_start_byte, sink_end_byte, source_key, identity],
    checks = [
        ("sink_span_order", "sink_end_byte >= sink_start_byte"),
        ("identity_not_through_call", "NOT (identity AND through_call)"),
    ],
    {
        snapshot_id: Id,
        /// The function the sink is in (for a field source, the reading method).
        function_node_id: Id,
        /// `Parameter[<node>]` or `Field[<class>.<field>]` (a method's read of its receiver's
        /// field with no local definition: the value stored there by any method of a relative).
        source_key: String,
        parameter_node_id: Option<Id>,
        /// The parameter's name, or the field's.
        source_name: String,
        class_node_id: Option<Id>,
        sink: FlowSink,
        module_node_id: Id,
        sink_start_byte: i64,
        sink_end_byte: i64,
        identity: bool,
        through_call: bool,
        captured: bool,
        condition_id: Id,
        condition: String,
        /// For an argument: its node and call site; for a stored value: the place written.
        argument_node_id: Option<Id>,
        call_site_node_id: Option<Id>,
        place: Option<String>,
    }
);

table!(
    /// A conservative, cited identity bridge from a public operation's entry formal to the
    /// exact operand use of one source test. A Pyrefly type observation is not this proof.
    /// The initial origin permits only one direct reaching formal and no intervening effect.
    FlowTestValueLinks, FlowTestValueLinksRow = "flow_test_value_links",
    family = Findings,
    key = [snapshot_id, operation_node_id, formal_node_id, leaf_fact_id, use_id],
    checks = [
        ("operand_span_order", "operand_start_byte >= 0 AND operand_end_byte > operand_start_byte"),
        ("stability_witness", "(origin = 2 AND stability_origin_id IS NOT NULL AND stability_condition_id IS NOT NULL) OR (origin <> 2 AND stability_origin_id IS NULL AND stability_condition_id IS NULL)"),
    ],
    {
        snapshot_id: Id,
        link_id: Id,
        operation_node_id: Id,
        formal_node_id: Id,
        module_node_id: Id,
        leaf_fact_id: Id,
        atom_id: Id,
        use_id: Id,
        use_fact_id: Id,
        reaching_fact_id: Id,
        definition_fact_id: Id,
        operand_start_byte: i64,
        operand_end_byte: i64,
        place: String,
        condition_id: Id,
        origin: TestValueLinkOrigin,
        effect_model_digest: Digest,
        /// Exact guard origin used to prove stability to a later test; null for origins 0 and 1.
        stability_origin_id: Option<Id>,
        /// The reaching fact's path condition that entails the exact guard atom.
        stability_condition_id: Option<Id>,
    }
);

table!(
    /// Under this leaf atom's TRUE assignment, the entry formal has exactly the named builtin
    /// runtime class under the standard-builtin-namespace model. The cited value link proves
    /// the guard's inner operand is that entry value;
    /// a later operand needs its own stability proof. The test root may include other atoms.
    FlowTestExactOrigins, FlowTestExactOriginsRow = "flow_test_exact_origins",
    family = Findings,
    key = [snapshot_id, operation_node_id, formal_node_id, leaf_fact_id, use_id],
    checks = [("builtin_class", "builtin_class IN ('str', 'int', 'bool')")],
    {
        snapshot_id: Id,
        origin_id: Id,
        operation_node_id: Id,
        formal_node_id: Id,
        module_node_id: Id,
        leaf_fact_id: Id,
        atom_id: Id,
        use_id: Id,
        value_link_id: Id,
        test_condition_id: Id,
        builtin_class: String,
        origin: ExactValueOrigin,
    }
);

table!(
    /// Field accesses (Stage 2.6): every `x.f` definition (a write) or load (a read) in the
    /// release, by the field's name. `class_node_id` is the method's class when the receiver is
    /// the method's own (`self`); the premise for "never read" is name-based, whatever the
    /// receiver (ADR-0022 §Verdicts).
    FieldAccesses, FieldAccessesRow = "field_accesses",
    family = Findings,
    key = [snapshot_id, module_node_id, start_byte, end_byte, write],
    checks = [("span_order", "end_byte >= start_byte")],
    {
        snapshot_id: Id,
        field: String,
        write: bool,
        receiver_self: bool,
        class_node_id: Option<Id>,
        function_node_id: Option<Id>,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        place: String,
        condition_id: Id,
        condition: String,
    }
);

table!(
    /// Reads of a module-global singleton's fields (Stage 2.6; ADR-0022 §Places and the read
    /// phase): each read at its resolved key (`Global[module.name]`, the field), whatever its
    /// spelling, with the reading site's phase and the condition it is read under.
    AmbientReads, AmbientReadsRow = "ambient_reads",
    family = Findings,
    key = [snapshot_id, module_node_id, start_byte, end_byte],
    checks = [("span_order", "end_byte >= start_byte")],
    {
        snapshot_id: Id,
        /// `module.name` of the global the read resolves to.
        global: String,
        class_node_id: Id,
        field: String,
        /// The reading function; null for a module or class body.
        reader_node_id: Option<Id>,
        phase: ReadPhase,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        line: i64,
        spelled: String,
        condition_id: Id,
        condition: String,
    }
);

table!(
    /// Name- or string-driven accesses (ADR-0022 §Verdicts, `dynamic_access`) and what they
    /// reach under the stated model: a class through a receiver whose reaching definitions,
    /// through local copies, include a method's own receiver or a global bound to an instance;
    /// modules for `import_module`/`__import__` with a computed name; every place for `exec`
    /// and `eval`. `reaches_class_node_id` and `reaches_all` are both unset when the receiver is
    /// outside the model (named in every negative answer).
    DynamicAccesses, DynamicAccessesRow = "dynamic_accesses",
    family = Findings,
    key = [snapshot_id, call_site_node_id],
    checks = [],
    {
        snapshot_id: Id,
        /// The call's site id; for a `__dict__` load, the flow use's id.
        call_site_node_id: Id,
        kind: DynamicKind,
        function_node_id: Option<Id>,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        reaches_class_node_id: Option<Id>,
        reaches_modules: bool,
        reaches_all: bool,
    }
);

table!(
    /// Every `raise` statement in a release function, with the condition it is reached under
    /// (its region, relative to the function's entry) and the function's parameters that
    /// condition tests (Stage 2.6: guards by the flow IR's reachability, not by syntax).
    RaiseSites, RaiseSitesRow = "raise_sites",
    family = Findings,
    key = [snapshot_id, module_node_id, start_byte, end_byte],
    checks = [("span_order", "end_byte >= start_byte")],
    {
        snapshot_id: Id,
        function_node_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        line: i64,
        /// The statement's first line, as written.
        text: String,
        condition_id: Id,
        condition: String,
        /// The function's parameters whose value reaches a use inside a test of the condition
        /// (through the flow IR's reaching definitions), sorted.
        parameters: Vec<String>,
        /// Whether the raise may leave its function: no enclosing `try` of the same function has
        /// a handler that may catch it, and no enclosing `with` is over `suppress(...)`. Only an
        /// escaping raise is a guard or a `raises_when` fate (ADR-0022 §Conditions).
        escapes: bool,
    }
);

table!(
    /// Module-global singletons (Stage 2.6; ADR-0022 §Places): a module-level `N = C(...)` of a
    /// release class, the key its fields' reads resolve to.
    Singletons, SingletonsRow = "singletons",
    family = Findings,
    key = [snapshot_id, global],
    checks = [],
    {
        snapshot_id: Id,
        /// `module.name`.
        global: String,
        class_node_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// Whether a negative claim about a place can be refuted under the model (ADR-0022
    /// §Verdicts): one row per place a claim is made about, with the premise's kind, whether it
    /// holds, and why not.
    NegativePremises, NegativePremisesRow = "negative_premises",
    family = Findings,
    key = [snapshot_id, place_key],
    checks = [],
    {
        snapshot_id: Id,
        /// `Parameter[<node>]`, `Field[<class>.<field>]` or `Global[<module>.<name>].<field>`.
        place_key: String,
        kind: PremiseKind,
        subject_node_id: Option<Id>,
        holds: bool,
        boundary_reason: Option<BoundaryReason>,
        reason: Option<String>,
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

    /// Except clauses with their authored type expression and the try-entry region.
    handler_clauses = "behavior:handler_clauses",
        deps = ["syntax_nodes", "flow_regions", "declarations"],
        sql = format!(
            "SELECT t.snapshot_id, t.owner_node_id AS function_node_id, \
                    t.node_id AS try_node_id, h.node_id AS handler_node_id, h.ordinal, \
                    h.module_node_id, t.fact_id AS try_fact_id, \
                    h.fact_id AS handler_fact_id, ty.node_id AS type_node_id, \
                    ty.fact_id AS type_fact_id, r.fact_id AS try_region_fact_id, \
                    h.start_byte, h.end_byte, r.condition_id AS entry_condition_id, \
                    r.approximated AS entry_approximated \
             FROM syntax_nodes t JOIN syntax_nodes h ON h.parent_node_id = t.node_id \
               AND h.field = {handler_field} AND h.kind = {handler_kind} \
             LEFT JOIN syntax_nodes ty ON ty.parent_node_id = h.node_id \
               AND ty.field = {test_field} \
             JOIN flow_regions r ON r.module_node_id = t.module_node_id \
               AND r.start_byte = t.start_byte AND r.end_byte = t.end_byte \
               AND r.scope_kind = {function_scope} \
             JOIN declarations d ON d.node_id = t.owner_node_id \
               AND d.kind IN ({function}, {async_function}) \
               AND r.scope_start_byte = d.name_start_byte \
               AND r.scope_end_byte = d.name_end_byte \
             WHERE t.kind = {try_}",
            handler_field = crate::codebook::SyntaxField::Handler.code(),
            handler_kind = SyntaxKind::ExceptHandlerExceptHandler.code(),
            test_field = crate::codebook::SyntaxField::Test.code(),
            function_scope = crate::codebook::LexicalScopeKind::Function.code(),
            function = DeclarationKind::Function.code(),
            async_function = DeclarationKind::AsyncFunction.code(),
            try_ = SyntaxKind::StmtTry.code(),
        );

    /// Keep every clause, including bare and unresolved types. Only one exact builtin
    /// reference and one pinned class can become a class identity; a tuple, attribute, alias
    /// or shadowed name stays unknown until a broader resolver proves it.
    handler_types = "behavior:handler_types",
        deps = ["handler_clauses", "syntax_nodes", "references", "reference_resolutions", "context_definitions", "context_modules"],
        sql = format!(
            "WITH candidates AS ( \
               SELECT h.snapshot_id, h.handler_node_id, h.function_node_id, \
                      h.type_node_id, h.type_fact_id, r.fact_id AS reference_fact_id, \
                      rr.fact_id AS resolution_fact_id, d.symbol_node_id AS class_node_id, \
                      d.fact_id AS class_fact_id, m.fact_id AS class_module_fact_id, \
                      rr.builtin_name AS class_name \
               FROM handler_clauses h \
               LEFT JOIN syntax_nodes t ON t.node_id = h.type_node_id AND t.kind = {name} \
               LEFT JOIN references r ON r.name_node_id = t.node_id \
                 AND r.module_node_id = h.module_node_id \
               LEFT JOIN reference_resolutions rr ON rr.reference_id = r.node_id \
                 AND rr.builtin_name = r.name AND rr.binding_id IS NULL \
               LEFT JOIN context_definitions d ON d.module_name = 'builtins' \
                 AND d.qualified_name = rr.builtin_name AND d.kind = {class_kind} \
               LEFT JOIN context_modules m ON m.module_node_id = d.module_node_id \
                 AND m.module_name = 'builtins' AND m.origin = {typeshed} \
             ), ranked AS ( \
               SELECT *, COUNT(reference_fact_id) OVER (PARTITION BY handler_node_id) AS refs, \
                      COUNT(resolution_fact_id) OVER (PARTITION BY handler_node_id) AS resolutions, \
                      COUNT(class_module_fact_id) OVER (PARTITION BY handler_node_id) AS classes, \
                      ROW_NUMBER() OVER (PARTITION BY handler_node_id \
                        ORDER BY class_name, class_fact_id) AS pick \
               FROM candidates \
             ) \
             SELECT snapshot_id, handler_node_id, function_node_id, type_node_id, type_fact_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN reference_fact_id END AS reference_fact_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN resolution_fact_id END AS resolution_fact_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN class_node_id END AS class_node_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN class_fact_id END AS class_fact_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN class_module_fact_id END AS class_module_fact_id, \
                    CASE WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                         THEN class_name END AS class_name, \
                    CAST(CASE WHEN type_node_id IS NULL THEN {bare} \
                              WHEN refs = 1 AND resolutions = 1 AND classes = 1 \
                              THEN {pinned} ELSE {unknown} END AS SMALLINT) AS status, \
                    CAST(CASE WHEN type_node_id IS NULL \
                                 OR (refs = 1 AND resolutions = 1 AND classes = 1) THEN NULL \
                                 WHEN refs > 1 OR resolutions > 1 OR classes > 1 THEN {ambiguous} \
                                 WHEN refs = 0 THEN {unsupported} \
                                 WHEN resolutions = 0 THEN {unresolved} \
                                 ELSE {missing} END AS SMALLINT) AS reason \
             FROM ranked WHERE pick = 1",
            name = SyntaxKind::ExprName.code(),
            class_kind = crate::codebook::DefinitionKind::Class.code(),
            typeshed = crate::codebook::ModuleOrigin::BundledTypeshed.code(),
            bare = HandlerTypeStatus::Bare.code(),
            pinned = HandlerTypeStatus::PinnedBuiltin.code(),
            unknown = HandlerTypeStatus::Unknown.code(),
            ambiguous = BoundaryReason::AmbiguousBinding.code(),
            unsupported = BoundaryReason::OutsideProviderModel.code(),
            unresolved = BoundaryReason::UnresolvedTarget.code(),
            missing = BoundaryReason::MissingEvidence.code(),
        );

    /// Direct handler-body statements with their own reachability region.
    handler_actions = "behavior:handler_actions",
        deps = ["handler_clauses", "syntax_nodes", "flow_regions", "declarations"],
        sql = format!(
            "SELECT h.snapshot_id, h.function_node_id, h.handler_node_id, \
                    a.node_id AS action_node_id, a.kind AS action_kind, a.module_node_id, \
                    a.fact_id AS action_fact_id, r.fact_id AS region_fact_id, \
                    a.start_byte, a.end_byte, r.condition_id, r.approximated \
             FROM handler_clauses h JOIN syntax_nodes a \
               ON a.parent_node_id = h.handler_node_id AND a.field = {body} \
             JOIN flow_regions r ON r.module_node_id = a.module_node_id \
               AND r.start_byte = a.start_byte AND r.end_byte = a.end_byte \
               AND r.scope_kind = {function_scope} \
             JOIN declarations d ON d.node_id = h.function_node_id \
               AND r.scope_start_byte = d.name_start_byte \
               AND r.scope_end_byte = d.name_end_byte",
            body = crate::codebook::SyntaxField::Body.code(),
            function_scope = crate::codebook::LexicalScopeKind::Function.code(),
        );

    /// Source-observed explicit exits and finally-body actions with ty's statement region.
    exit_sites = "behavior:exit_sites",
        deps = ["syntax_nodes", "flow_regions", "declarations"],
        sql = format!(
            "WITH sites AS ( \
               SELECT s.snapshot_id, s.owner_node_id AS function_node_id, \
                      s.node_id AS site_node_id, s.kind AS syntax_kind, s.field, \
                      p.kind AS parent_kind, s.module_node_id, \
                      s.fact_id AS source_fact_id, r.fact_id AS region_fact_id, \
                      s.start_byte, s.end_byte, r.condition_id, r.approximated \
               FROM syntax_nodes s JOIN flow_regions r \
               ON r.module_node_id = s.module_node_id \
              AND r.start_byte = s.start_byte AND r.end_byte = s.end_byte \
              AND r.scope_kind = {function_scope} \
             JOIN declarations d ON d.node_id = s.owner_node_id \
               AND d.kind IN ({function}, {async_function}) \
               AND r.scope_start_byte = d.name_start_byte \
               AND r.scope_end_byte = d.name_end_byte \
             LEFT JOIN syntax_nodes p ON p.node_id = s.parent_node_id \
               AND p.module_node_id = s.module_node_id \
             WHERE s.kind IN ({ret}, {raise}) \
                OR (s.field = {finalbody} AND p.kind = {try_}) \
             ) \
             SELECT snapshot_id, function_node_id, site_node_id, \
                    CAST(CASE WHEN syntax_kind = {ret} THEN {return_kind} \
                              ELSE {raise_kind} END AS SMALLINT) AS kind, \
                    module_node_id, source_fact_id, region_fact_id, start_byte, end_byte, \
                    condition_id, approximated \
             FROM sites WHERE syntax_kind IN ({ret}, {raise}) \
             UNION ALL \
             SELECT snapshot_id, function_node_id, site_node_id, \
                    CAST({finally_kind} AS SMALLINT) AS kind, \
                    module_node_id, source_fact_id, region_fact_id, start_byte, end_byte, \
                    condition_id, approximated \
             FROM sites WHERE field = {finalbody} AND parent_kind = {try_}",
            ret = crate::codebook::SyntaxKind::StmtReturn.code(),
            raise = crate::codebook::SyntaxKind::StmtRaise.code(),
            return_kind = ExitSiteKind::Return.code(),
            raise_kind = ExitSiteKind::Raise.code(),
            finally_kind = ExitSiteKind::FinallyBody.code(),
            function_scope = crate::codebook::LexicalScopeKind::Function.code(),
            function = DeclarationKind::Function.code(),
            async_function = DeclarationKind::AsyncFunction.code(),
            finalbody = crate::codebook::SyntaxField::Finalbody.code(),
            try_ = crate::codebook::SyntaxKind::StmtTry.code(),
        );

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
