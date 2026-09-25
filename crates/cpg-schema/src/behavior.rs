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
    ArgumentKind, BehaviorKind, BoundaryReason, Codebook, DeclarationKind, DynamicKind,
    EmbeddingView, ExactValueOrigin, ExitSiteKind, FlowCallLinkStatus, FlowSink, HandlerTypeStatus, ImplicitReceiver,
    InvocationPhase, Modality, ModelArgumentStatus, ModeledArgumentEvaluationStatus, ModelCallbackAction, ModelChannelCoverage,
    ModelEffectKind, ModelEffectSubjectStatus, ModelExceptionAction, ModelExit, ModelPathKind,
    ModelPathRole, ModelResourceAction, ModelResourceSourceStatus, ModelTransferEndpointStatus,
    ModelTransferKind, ModeledHandlerClassMatch, OperationFacet, Origin, ParameterKind,
    PremiseKind, ReadPhase, SourceRole, SummaryFlowKind, SummaryFlowStepKind, SyntaxKind, TestValueLinkOrigin, ValueClass, Verdict,
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
        subject_path_kind: Option<ModelPathKind>,
        subject_path: Option<String>,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// One candidate authored effect at a source call. A subjectless rule remains explicitly
    /// unqualified; a parameter subject is cited only through exact signature binding.
    ModeledEffectSites, ModeledEffectSitesRow = "modeled_effect_sites",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        effect: ModelEffectKind,
        argument: Option<String>,
        subject_path_id: Option<Id>,
        subject_path_kind: Option<ModelPathKind>,
        subject_expression_node_id: Option<Id>,
        subject_expression_fact_id: Option<Id>,
        subject_status: ModelEffectSubjectStatus,
        subject_reason: Option<BoundaryReason>,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        origin: Origin,
    }
);

table!(
    /// A call in the analyzed release that reaches a pinned modeled target. This joins source
    /// call evidence to a model assertion without claiming any transfer, effect or callback
    /// fate. Candidate-set openness and both origins remain visible to later L2/L3 consumers.
    ModelApplications, ModelApplicationsRow = "model_applications",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id],
    checks = [("revision_positive", "revision > 0"), ("target_count_nonnegative", "target_count >= 0")],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        module_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        target_module_fact_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        target_modality: Modality,
        target_origin: Origin,
        phase: InvocationPhase,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        /// Number of resolved call targets, including those without an authored model.
        target_count: i64,
        /// The selected pinned model target asserts normal return after argument evaluation;
        /// this alone does not establish completion of this source call.
        target_normal_return: bool,
        model_origin: Origin,
    }
);

table!(
    /// A formal named by a typed model path. The catalog compiler emits this from its input or
    /// output path AST, never by parsing the path's display spelling. It permits later source
    /// argument binding without introducing a second model grammar.
    ModelFormalPaths, ModelFormalPathsRow = "model_formal_paths",
    family = Findings,
    key = [snapshot_id, model_id, target_node_id, rule_id, path_role, path_id],
    checks = [],
    {
        snapshot_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        revision: i64,
        path_id: Id,
        path_role: ModelPathRole,
        formal_name: String,
        origin: Origin,
    }
);

table!(
    /// A model formal mapped to one source argument only when every pinned signature agrees.
    /// A starred call, implicit receiver, missing argument or overloaded disagreement remains
    /// `unknown` with a boundary. A bound argument is local to this candidate model target.
    ModelArgumentBindings, ModelArgumentBindingsRow = "model_argument_bindings",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id, path_role, path_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        target_node_id: Id,
        rule_id: Id,
        path_role: ModelPathRole,
        path_id: Id,
        formal_name: String,
        signature_count: i64,
        matched_signatures: i64,
        argument_node_id: Option<Id>,
        argument_fact_id: Option<Id>,
        status: ModelArgumentStatus,
        reason: Option<BoundaryReason>,
    }
);

table!(
    /// A source call site and an authored callback action on one candidate modeled target.
    /// The binding status names whether the callback value is an exact source argument;
    /// the target's openness and rule exit/modalities remain separate from that identity.
    ModeledCallbackSites, ModeledCallbackSitesRow = "modeled_callback_sites",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        callback_path_id: Id,
        argument_node_id: Option<Id>,
        argument_fact_id: Option<Id>,
        binding_status: ModelArgumentStatus,
        binding_reason: Option<BoundaryReason>,
        action: ModelCallbackAction,
        exit: ModelExit,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
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
        resource_path_kind: ModelPathKind,
        action: ModelResourceAction,
        exit: ModelExit,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// A candidate modeled resource action at a source call. `source_expression_node_id`
    /// identifies the call expression for a return value or an exact argument expression; it
    /// is never a runtime resource identity or proof of acquisition/release completion.
    ModeledResourceSites, ModeledResourceSitesRow = "modeled_resource_sites",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        resource_path_id: Id,
        resource_role: ModelPathRole,
        resource_path_kind: ModelPathKind,
        source_expression_node_id: Option<Id>,
        source_expression_fact_id: Option<Id>,
        source_status: ModelResourceSourceStatus,
        source_reason: Option<BoundaryReason>,
        action: ModelResourceAction,
        exit: ModelExit,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        origin: Origin,
    }
);

table!(
    /// Authored exception action of a pinned external definition. A conversion must name its
    /// replacement class. Referenced models resolve both names to unique pinned context class
    /// facts before publication; a display string is never the class identity.
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
        class_node_id: Id,
        class_fact_id: Id,
        action: ModelExceptionAction,
        to_class: Option<String>,
        to_class_node_id: Option<Id>,
        to_class_fact_id: Option<Id>,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// One candidate modeled exception action at a source call. The class is pinned, but a
    /// potential raise is not an observed exceptional exit and open dispatch stays visible.
    ModeledExceptionSites, ModeledExceptionSitesRow = "modeled_exception_sites",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        class: String,
        class_node_id: Id,
        class_fact_id: Id,
        action: ModelExceptionAction,
        to_class: Option<String>,
        to_class_node_id: Option<Id>,
        to_class_fact_id: Option<Id>,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
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
    /// A handler body's sole direct statement is `return None`, with an exact literal child
    /// and cited ty region. If this handler is entered and its region is reached, it returns
    /// normally before any enclosing `finally` action. The region's approximation remains
    /// explicit; this does not prove handler selection, final completion or suppression.
    HandlerReturnNoneSites, HandlerReturnNoneSitesRow = "handler_return_none_sites",
    family = Findings,
    key = [snapshot_id, handler_node_id],
    checks = [],
    {
        snapshot_id: Id,
        function_node_id: Id,
        handler_node_id: Id,
        handler_fact_id: Id,
        return_node_id: Id,
        return_fact_id: Id,
        none_node_id: Id,
        none_fact_id: Id,
        region_fact_id: Id,
        condition_id: Id,
        approximated: bool,
    }
);

table!(
    /// A modeled potential raise lies in the body of an enclosing try with this handler.
    /// A class comparison nominates a possible catch; neither it nor syntactic nesting proves
    /// that the call raises, that this handler is selected, or that the handler completes.
    ModeledExceptionHandlerCandidates, ModeledExceptionHandlerCandidatesRow = "modeled_exception_handler_candidates",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id, handler_node_id],
    checks = [("nonnegative_depth", "frame_depth >= 0")],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        call_fact_id: Id,
        raised_class_node_id: Id,
        raised_class_fact_id: Id,
        try_node_id: Id,
        handler_node_id: Id,
        handler_fact_id: Id,
        frame_depth: i64,
        handler_ordinal: i64,
        handler_type_status: HandlerTypeStatus,
        handler_class_node_id: Option<Id>,
        handler_class_fact_id: Option<Id>,
        class_mro_fact_id: Option<Id>,
        class_match: ModeledHandlerClassMatch,
        /// Clause order within this try frame admits this handler if the modeled raise occurs.
        /// An inner frame may still intercept it; this is not an operation-level catch.
        frame_possible: bool,
        /// Positive class match with no earlier possible clause, conditional on this modeled
        /// raise reaching this frame. Handler-body completion remains a separate question.
        frame_first_match_if_raised: bool,
    }
);

table!(
    /// Coverage of the bounded syntax-ancestor walk for one modeled potential raise. A missing
    /// source syntax node or a depth cap is explicit, even when no handler candidate was found.
    ModeledExceptionHandlerWalks, ModeledExceptionHandlerWalksRow = "modeled_exception_handler_walks",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [("nonnegative_depth", "max_depth >= 0")],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        source_syntax_fact_id: Option<Id>,
        max_depth: i64,
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
    /// One candidate modeled raise has a conditional path through the first matching handler
    /// of a direct `try` to its sole `return None`. There is no intervening `try`/`with` and
    /// the frame has no `finally`. The modeled call/raise and target remain candidates; this
    /// is not an operation-level catch, absence-of-escape, or completed-effect verdict.
    ModeledExceptionReturnNonePaths, ModeledExceptionReturnNonePathsRow = "modeled_exception_return_none_paths",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [("nonnegative_depth", "frame_depth >= 0")],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        call_fact_id: Id,
        source_syntax_fact_id: Id,
        raised_class_node_id: Id,
        raised_class_fact_id: Id,
        try_node_id: Id,
        handler_node_id: Id,
        handler_fact_id: Id,
        handler_ordinal: i64,
        frame_depth: i64,
        class_match: ModeledHandlerClassMatch,
        class_mro_fact_id: Option<Id>,
        return_node_id: Id,
        return_fact_id: Id,
        none_node_id: Id,
        none_fact_id: Id,
        handler_region_fact_id: Id,
        handler_condition_id: Id,
        handler_region_approximated: bool,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
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
        /// A pinned authored claim of total normal completion after argument evaluation;
        /// absence remains unknown. It never resolves call-site dispatch by itself.
        normal_return: bool,
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
        input_path_kind: ModelPathKind,
        input_path: String,
        output_path_id: Id,
        output_path_kind: ModelPathKind,
        output_path: String,
        transfer: ModelTransferKind,
        modality: Modality,
        origin: Origin,
    }
);

table!(
    /// An authored transfer applied to one pinned candidate source call. Endpoint expressions
    /// are identified only when the argument binding or call-result path is exact. This is not
    /// yet a composed value-flow summary or a whole-operation verdict.
    ModeledTransferSites, ModeledTransferSitesRow = "modeled_transfer_sites",
    family = Findings,
    key = [snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        function_node_id: Option<Id>,
        call_fact_id: Id,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        input_path_id: Id,
        input_path_kind: ModelPathKind,
        input_expression_node_id: Option<Id>,
        input_expression_fact_id: Option<Id>,
        input_status: ModelTransferEndpointStatus,
        input_reason: Option<BoundaryReason>,
        output_path_id: Id,
        output_path_kind: ModelPathKind,
        output_expression_node_id: Option<Id>,
        output_expression_fact_id: Option<Id>,
        output_status: ModelTransferEndpointStatus,
        output_reason: Option<BoundaryReason>,
        transfer: ModelTransferKind,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        origin: Origin,
    }
);

table!(
    /// A one-call source-to-value candidate through a pinned model transfer. The call must
    /// occupy the entire return or definition value expression; the modeled result remains
    /// conditional on target choice, normal completion and model modality.
    ModeledExactValueTransfers, ModeledExactValueTransfersRow = "modeled_exact_value_transfers",
    family = Findings,
    key = [snapshot_id, flow_value_fact_id, parameter_node_id, pysa_fact_id, model_id, rule_id],
    checks = [("sink_span_order", "sink_start_byte >= 0 AND sink_end_byte > sink_start_byte")],
    {
        snapshot_id: Id,
        flow_value_fact_id: Id,
        flow_value_call_fact_id: Id,
        call_site_node_id: Id,
        call_fact_id: Id,
        argument_node_id: Id,
        argument_fact_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        use_id: Id,
        sink: FlowSink,
        sink_start_byte: i64,
        sink_end_byte: i64,
        /// Recomposition by the flow analysis; the provider `conditions` table may omit it.
        condition_id: Id,
        condition: String,
        /// Approximation on this raw flow fact only; predecessor reachability is not closed.
        raw_flow_approximated: bool,
        pysa_fact_id: Id,
        target_node_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        transfer: ModelTransferKind,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        origin: Origin,
    }
);

table!(
    /// Every explicit argument of an exact one-call modeled value candidate, in source order.
    /// The selected source operand cites its raw value fact; a sibling literal cites its Ruff
    /// expression fact; an exact unshadowed builtin name cites lexical resolution. Other
    /// evaluations remain explicit unknowns. This records local
    /// evaluation evidence, not a completed call, callee dispatch or enclosing return.
    ModeledArgumentEvaluations, ModeledArgumentEvaluationsRow = "modeled_argument_evaluations",
    family = Findings,
    key = [snapshot_id, candidate_flow_fact_id, parameter_node_id, pysa_fact_id, model_id, rule_id, argument_fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0"),
        ("evidence_iff_known", "(status IN (0, 1, 3) AND evidence_id IS NOT NULL AND reason IS NULL) OR (status = 2 AND evidence_id IS NULL AND reason IS NOT NULL)"),
    ],
    {
        snapshot_id: Id,
        candidate_flow_fact_id: Id,
        parameter_node_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        call_site_node_id: Id,
        argument_node_id: Id,
        argument_fact_id: Id,
        ordinal: i64,
        status: ModeledArgumentEvaluationStatus,
        evidence_id: Option<Id>,
        reason: Option<BoundaryReason>,
        condition_id: Id,
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
    /// One source origin contributing to one raw `flow_values` fact before the presentation
    /// view merges origins and keeps only the strongest transfer. L3 must join its parent fact
    /// to `flow_value_call_links`; a sink span or `through_call` flag is not a call identity.
    ValueFlowContributions, ValueFlowContributionsRow = "value_flow_contributions",
    family = Findings,
    key = [snapshot_id, flow_value_fact_id, use_id, source_key, identity, through_call],
    checks = [
        ("identity_not_through_call", "NOT (identity AND through_call)"),
        ("local_call_is_a_call", "NOT local_through_call OR through_call"),
        ("upstream_transfer_exclusive", "NOT (upstream_identity AND upstream_through_call)"),
        ("upstream_call_is_a_call", "NOT upstream_through_call OR through_call"),
        ("one_origin_kind", "(parameter_node_id IS NULL AND class_node_id IS NOT NULL) OR (parameter_node_id IS NOT NULL AND class_node_id IS NULL)"),
    ],
    {
        snapshot_id: Id,
        flow_value_fact_id: Id,
        use_id: Id,
        /// Function owning the source parameter (or reading a source field).
        function_node_id: Id,
        /// Function containing this raw sink use; a captured parameter may belong to a
        /// different function. Null when the use is not in a function.
        sink_function_node_id: Option<Id>,
        source_key: String,
        parameter_node_id: Option<Id>,
        class_node_id: Option<Id>,
        identity: bool,
        through_call: bool,
        /// The parent `flow_values` fact itself crosses a call; inherited call transfer through
        /// a reaching definition must be followed at that definition's distinct fact.
        local_through_call: bool,
        /// The transfer from the source origin to this fact's use, before the local fact.
        /// False/false is a derived transfer; true/false is identity; false/true crosses
        /// an earlier call and needs that earlier fact's ordered call path.
        upstream_identity: bool,
        upstream_through_call: bool,
        captured: bool,
        condition_id: Id,
        condition: String,
    }
);

table!(
    /// The structural BDD authority for a condition recomposed by the flow analysis. These
    /// identities may not occur in the provider's `conditions` table.
    AnalysisConditions, AnalysisConditionsRow = "analysis_conditions",
    family = Findings,
    key = [snapshot_id, condition_id],
    checks = [("root_iff_stated", "(root_id IS NOT NULL AND boundary_reason IS NULL) OR (root_id IS NULL AND boundary_reason IS NOT NULL)")],
    {
        snapshot_id: Id,
        condition_id: Id,
        root_id: Option<Id>,
        boundary_reason: Option<String>,
    }
);

table!(
    /// Deduplicated content-addressed nodes for `analysis_conditions`; terminals are implicit.
    AnalysisConditionNodes, AnalysisConditionNodesRow = "analysis_condition_nodes",
    family = Findings,
    key = [snapshot_id, node_id],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        atom: String,
        low_id: Id,
        high_id: Id,
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
    /// A source-preserving predecessor candidate for a raw value fact whose source already
    /// crossed a call before this fact. The reaching definition identifies the earlier value
    /// expression; condition compatibility and complete path selection remain for L3.
    ValueFlowPredecessorCandidates, ValueFlowPredecessorCandidatesRow = "value_flow_predecessor_candidates",
    family = Findings,
    key = [snapshot_id, successor_fact_id, predecessor_fact_id, source_key, reaching_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        successor_fact_id: Id,
        predecessor_fact_id: Id,
        source_key: String,
        parameter_node_id: Id,
        function_node_id: Id,
        successor_use_id: Id,
        predecessor_use_id: Id,
        reaching_fact_id: Id,
        reaching_condition_id: Id,
        reaching_approximated: bool,
        loop_carried: bool,
        definition_id: Id,
        definition_fact_id: Id,
        /// Flow-analysis conditions; separate from the provider reaching condition above.
        successor_condition_id: Id,
        predecessor_condition_id: Id,
        successor_raw_approximated: bool,
        predecessor_raw_approximated: bool,
        predecessor_local_through_call: bool,
        predecessor_upstream_through_call: bool,
    }
);

table!(
    /// Bounded propositional compatibility of the three cited conditions on one predecessor
    /// candidate. `true` admits only a may-path; it does not prove value transfer or completion.
    ValueFlowPredecessorCompatibility, ValueFlowPredecessorCompatibilityRow = "value_flow_predecessor_compatibility",
    family = Findings,
    key = [snapshot_id, successor_fact_id, predecessor_fact_id, source_key, reaching_fact_id],
    checks = [("decision_or_boundary", "(compatible_under_atoms IS NULL AND boundary_reason IS NOT NULL) OR (compatible_under_atoms IS NOT NULL AND boundary_reason IS NULL)")],
    {
        snapshot_id: Id,
        successor_fact_id: Id,
        predecessor_fact_id: Id,
        source_key: String,
        reaching_fact_id: Id,
        predecessor_condition_id: Id,
        reaching_condition_id: Id,
        successor_condition_id: Id,
        compatible_under_atoms: Option<bool>,
        boundary_reason: Option<BoundaryReason>,
    }
);

table!(
    /// A two-step source-to-return candidate: a pinned whole-assignment model result reaches
    /// an identity return through one cited, condition-checked definition. The call's normal
    /// completion, target closure and any enclosing exit actions remain unresolved here.
    ModeledAssignmentReturnPaths, ModeledAssignmentReturnPathsRow = "modeled_assignment_return_paths",
    family = Findings,
    key = [snapshot_id, successor_fact_id, predecessor_fact_id, reaching_fact_id, parameter_node_id, pysa_fact_id, model_id, rule_id],
    checks = [("decision_or_boundary", "(compatible_under_atoms IS NULL AND boundary_reason IS NOT NULL) OR (compatible_under_atoms IS NOT NULL AND boundary_reason IS NULL)")],
    {
        snapshot_id: Id,
        successor_fact_id: Id,
        predecessor_fact_id: Id,
        reaching_fact_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        call_site_node_id: Id,
        call_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        target_definition_fact_id: Id,
        predecessor_condition_id: Id,
        reaching_condition_id: Id,
        successor_condition_id: Id,
        compatible_under_atoms: Option<bool>,
        boundary_reason: Option<BoundaryReason>,
        transfer: ModelTransferKind,
        target_modality: Modality,
        model_modality: Modality,
        candidate_set_complete_under_model: bool,
        has_unresolved_remainder: bool,
        predecessor_raw_approximated: bool,
        reaching_approximated: bool,
        successor_raw_approximated: bool,
    }
);

table!(
    /// Finite source-to-output may-flow of one callable, with a lossless condition root and
    /// source witness. Producers admit direct synchronous parameter identities, exact pinned
    /// identity-model calls and unique condition-compatible assignment predecessors only with
    /// their respective call and exit proofs.
    SummaryFlows, SummaryFlowsRow = "summary_flows",
    family = Findings,
    key = [snapshot_id, summary_id],
    checks = [
        ("boundary_iff_unknown", "(verdict = 3 AND boundary_reason IS NOT NULL) OR (verdict IN (0, 1) AND boundary_reason IS NULL)"),
        ("depth_nonnegative", "path_depth >= 0"),
    ],
    {
        snapshot_id: Id,
        /// Canonical identity of the finite path and its ordered evidence, independent of row order.
        summary_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        input_path: String,
        output_path: String,
        kind: SummaryFlowKind,
        condition_id: Id,
        verdict: Verdict,
        boundary_reason: Option<BoundaryReason>,
        source_flow_fact_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        approximated: bool,
        path_depth: i64,
    }
);

table!(
    /// Ordered proof steps for a finite summary. Raw identity and the first exact pinned-model
    /// path use disjoint typed step sequences reconstructed by the shared validator.
    SummaryFlowSteps, SummaryFlowStepsRow = "summary_flow_steps",
    family = Findings,
    key = [snapshot_id, summary_id, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        summary_id: Id,
        ordinal: i64,
        kind: SummaryFlowStepKind,
        /// The source relation is selected by `kind`; the shared validator checks the exact
        /// ordered sequence and the generated reference rule checks evidence closure.
        evidence_id: Id,
        condition_id: Id,
    }
);

table!(
    /// A parameter-to-return raw path for which the finite summary producer has no positive
    /// proof. Absence of a `summary_flows` row is never a negative transfer conclusion.
    SummaryBoundaries, SummaryBoundariesRow = "summary_boundaries",
    family = Findings,
    key = [snapshot_id, function_node_id, parameter_node_id, source_flow_fact_id, condition_id],
    checks = [],
    {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        source_flow_fact_id: Id,
        condition_id: Id,
        reason: BoundaryReason,
        local_through_call: bool,
        upstream_through_call: bool,
        raw_approximated: bool,
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
        /// A conservative escape witness: no enclosing `try` or `with` body in the same function
        /// can alter this raise's fate. False means unknown, not caught. Only a witnessed escape
        /// can become a guard or a `raises_when` fate (ADR-0022 §Conditions).
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

const MODELED_HANDLER_MAX_ANCESTOR_DEPTH: i64 = 128;

/// Reused CTE because derived relations are queries, not automatically registered views.
fn modeled_callee_candidates_sql() -> String {
    format!(
        "callee_candidates AS ( \
           SELECT c.node_id AS call_node_id, rr.fact_id AS resolution_fact_id, \
                  count(*) OVER (PARTITION BY c.node_id) AS candidate_count \
           FROM call_syntax c \
           JOIN syntax_nodes s ON s.parent_node_id = c.node_id \
             AND s.module_node_id = c.module_node_id AND s.field = {callee_field} \
             AND s.kind = {name_expr} AND s.start_byte = c.callee_start_byte \
             AND s.end_byte = c.callee_end_byte \
           JOIN references ref ON ref.name_node_id = s.node_id \
             AND ref.module_node_id = s.module_node_id \
           JOIN reference_resolutions rr ON rr.reference_id = ref.node_id \
             AND rr.reason IS NULL AND (rr.binding_id IS NOT NULL OR rr.builtin_name IS NOT NULL) \
         )",
        callee_field = crate::codebook::SyntaxField::Callee.code(),
        name_expr = SyntaxKind::ExprName.code(),
    )
}

fn modeled_handler_climb_sql() -> String {
    format!(
        "WITH RECURSIVE climb AS ( \
           SELECT e.snapshot_id, e.call_site_node_id, e.pysa_fact_id, e.model_id, e.rule_id, \
                  e.function_node_id, s.node_id, s.parent_node_id, s.field, \
                  CAST(0 AS BIGINT) AS depth \
           FROM modeled_exception_sites e JOIN syntax_nodes s \
             ON s.node_id = e.call_site_node_id \
           WHERE e.action = {raise} \
           UNION ALL \
           SELECT c.snapshot_id, c.call_site_node_id, c.pysa_fact_id, c.model_id, c.rule_id, \
                  c.function_node_id, p.node_id, p.parent_node_id, p.field, c.depth + 1 \
           FROM climb c JOIN syntax_nodes p ON p.node_id = c.parent_node_id \
           WHERE c.depth < {max_depth} AND p.owner_node_id = c.function_node_id \
         ) ",
        raise = ModelExceptionAction::Raise.code(),
        max_depth = MODELED_HANDLER_MAX_ANCESTOR_DEPTH,
    )
}

crate::relations! {
    inventory all;

    /// One evidence-preserving application per Pysa call-target fact and pinned model. A
    /// higher-order argument target is not the call's callee; it is excluded here.
    model_applications = "behavior:model_applications",
        deps = ["call_targets", "call_syntax", "pysa_calls", "facts", "resolutions", "model_targets"],
        sql = format!(
            "SELECT c.snapshot_id, c.node_id AS call_site_node_id, c.module_node_id, \
                    c.owner_node_id AS function_node_id, c.fact_id AS call_fact_id, \
                    t.pysa_fact_id, t.target_node_id, m.model_id, \
                    m.target_module_fact_id, m.target_definition_fact_id, m.revision, \
                    f.modality AS target_modality, f.origin AS target_origin, p.phase, \
                    r.candidate_set_complete_under_model, r.has_unresolved_remainder, \
                    r.target_count, m.normal_return AS target_normal_return, \
                    m.origin AS model_origin \
             FROM call_targets t \
             JOIN model_targets m ON m.target_node_id = t.target_node_id \
             JOIN call_syntax c ON c.node_id = t.call_site_node_id \
             JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
             JOIN facts f ON f.fact_id = p.fact_id \
             JOIN resolutions r ON r.call_site_node_id = c.node_id \
             WHERE t.reason IS NULL AND t.argument_node_id IS NULL \
               AND p.higher_order_index IS NULL AND p.phase IN ({call}, {init}) \
               AND NOT c.in_annotation",
            call = InvocationPhase::Call.code(),
            init = InvocationPhase::Init.code(),
        );

    /// Bind a modeled formal only when each pinned signature selects the same one explicit
    /// argument. Any unpacking or implicit receiver withholds this first positive mapping.
    model_argument_bindings = "behavior:model_argument_bindings",
        deps = ["model_applications", "model_formal_paths", "context_definitions", "context_parameters", "pysa_calls", "arguments"],
        sql = format!(
            "WITH unpacked AS ( \
               SELECT call_node_id, COUNT(*) AS n FROM arguments \
               WHERE kind IN ({starred}, {double_starred}) GROUP BY call_node_id \
             ), base AS ( \
               SELECT a.snapshot_id, a.call_site_node_id, a.pysa_fact_id, a.model_id, \
                      a.target_node_id, f.rule_id, f.path_role, f.path_id, f.formal_name, \
                      d.signature_count, p.implicit_receiver, \
                      COALESCE(u.n, 0) AS unpacked_count \
               FROM model_applications a \
               JOIN model_formal_paths f ON f.model_id = a.model_id \
                 AND f.target_node_id = a.target_node_id \
               JOIN context_definitions d ON d.fact_id = a.target_definition_fact_id \
                 AND d.symbol_node_id = a.target_node_id \
               JOIN pysa_calls p ON p.fact_id = a.pysa_fact_id \
               LEFT JOIN unpacked u ON u.call_node_id = a.call_site_node_id \
             ), matches AS ( \
               SELECT b.*, cp.signature_index, arg.node_id AS matched_node_id, \
                      arg.ordinal AS matched_ordinal \
               FROM base b \
               JOIN context_parameters cp ON cp.symbol_node_id = b.target_node_id \
                 AND cp.name = b.formal_name \
               LEFT JOIN arguments arg ON arg.call_node_id = b.call_site_node_id \
                 AND ((arg.kind = {positional} AND arg.ordinal = cp.ordinal \
                       AND cp.kind IN ({pos_only}, {pos_or_keyword})) \
                   OR (arg.kind = {keyword} AND arg.keyword = b.formal_name \
                       AND cp.kind IN ({pos_or_keyword}, {keyword_only}))) \
             ), grouped AS ( \
               SELECT snapshot_id, call_site_node_id, pysa_fact_id, model_id, \
                      target_node_id, rule_id, path_role, path_id, formal_name, \
                      signature_count, implicit_receiver, unpacked_count, \
                      COUNT(DISTINCT signature_index) AS signatures_seen, \
                      COUNT(matched_node_id) AS matched_signatures, \
                      COUNT(DISTINCT matched_ordinal) AS distinct_arguments, \
                      MIN(matched_ordinal) AS selected_ordinal \
               FROM matches GROUP BY snapshot_id, call_site_node_id, pysa_fact_id, \
                    model_id, target_node_id, rule_id, path_role, path_id, formal_name, \
                    signature_count, implicit_receiver, unpacked_count \
             ) \
             SELECT g.snapshot_id, g.call_site_node_id, g.pysa_fact_id, g.model_id, \
                    g.target_node_id, g.rule_id, g.path_role, g.path_id, g.formal_name, \
                    g.signature_count, g.matched_signatures, \
                    CASE WHEN {bound_condition} THEN chosen.node_id END AS argument_node_id, \
                    CASE WHEN {bound_condition} THEN chosen.fact_id END AS argument_fact_id, \
                    CAST(CASE WHEN {bound_condition} THEN {bound} ELSE {unknown} END \
                      AS SMALLINT) AS status, \
                    CAST(CASE WHEN {bound_condition} THEN NULL \
                              WHEN g.unpacked_count > 0 THEN {unsupported_unpacking} \
                              WHEN g.implicit_receiver IS NOT NULL \
                                AND g.implicit_receiver <> {receiver_false} THEN {outside} \
                              WHEN g.signatures_seen <> g.signature_count \
                                OR g.matched_signatures = 0 THEN {missing} \
                              ELSE {ambiguous} END AS SMALLINT) AS reason \
             FROM grouped g LEFT JOIN arguments chosen \
               ON chosen.call_node_id = g.call_site_node_id \
              AND chosen.ordinal = g.selected_ordinal",
            starred = ArgumentKind::Starred.code(),
            double_starred = ArgumentKind::DoubleStarred.code(),
            positional = ArgumentKind::Positional.code(),
            keyword = ArgumentKind::Keyword.code(),
            pos_only = ParameterKind::PositionalOnly.code(),
            pos_or_keyword = ParameterKind::PositionalOrKeyword.code(),
            keyword_only = ParameterKind::KeywordOnly.code(),
            bound_condition = format!(
                "g.unpacked_count = 0 AND (g.implicit_receiver IS NULL OR g.implicit_receiver = {}) \
                 AND g.signature_count = g.signatures_seen \
                 AND g.matched_signatures = g.signature_count \
                 AND g.distinct_arguments = 1",
                ImplicitReceiver::False.code()
            ),
            bound = ModelArgumentStatus::Bound.code(),
            unknown = ModelArgumentStatus::Unknown.code(),
            unsupported_unpacking = BoundaryReason::UnsupportedUnpacking.code(),
            receiver_false = ImplicitReceiver::False.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
            missing = BoundaryReason::MissingEvidence.code(),
            ambiguous = BoundaryReason::AmbiguousBinding.code(),
        );

    /// Applying a callback model does not prove its source argument or that the call took its
    /// modeled exit. Preserve both unknown binding and candidate/open dispatch in every row.
    modeled_callback_sites = "behavior:modeled_callback_sites",
        deps = ["model_applications", "model_callbacks", "model_argument_bindings"],
        sql = format!(
            "SELECT a.snapshot_id, a.call_site_node_id, a.function_node_id, \
                    a.call_fact_id, a.pysa_fact_id, a.target_node_id, a.model_id, \
                    m.rule_id, m.target_definition_fact_id, m.callback_path_id, \
                    b.argument_node_id, b.argument_fact_id, \
                    CAST(COALESCE(b.status, {unknown}) AS SMALLINT) AS binding_status, \
                    CAST(CASE WHEN b.rule_id IS NULL THEN {outside} ELSE b.reason END \
                      AS SMALLINT) AS binding_reason, \
                    m.action, m.exit, a.target_modality, m.modality AS model_modality, \
                    a.candidate_set_complete_under_model, a.has_unresolved_remainder, \
                    m.origin \
             FROM model_applications a \
             JOIN model_callbacks m ON m.model_id = a.model_id \
               AND m.target_node_id = a.target_node_id \
               AND m.target_definition_fact_id = a.target_definition_fact_id \
               AND m.revision = a.revision \
             LEFT JOIN model_argument_bindings b \
               ON b.call_site_node_id = a.call_site_node_id \
              AND b.pysa_fact_id = a.pysa_fact_id \
              AND b.model_id = a.model_id AND b.rule_id = m.rule_id \
              AND b.path_id = m.callback_path_id AND b.path_role = {input}",
            unknown = ModelArgumentStatus::Unknown.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
            input = ModelPathRole::Input.code(),
        );

    /// Model a resource at a cited call. A return path identifies the call expression; a
    /// parameter path uses only an exact pinned-signature argument binding. Field and global
    /// paths need a separate value proof and remain explicit unknowns.
    modeled_resource_sites = "behavior:modeled_resource_sites",
        deps = ["model_applications", "model_resources", "model_argument_bindings"],
        sql = format!(
            "SELECT a.snapshot_id, a.call_site_node_id, a.function_node_id, \
                    a.call_fact_id, a.pysa_fact_id, a.target_node_id, a.model_id, \
                    m.rule_id, m.target_definition_fact_id, m.resource_path_id, \
                    m.resource_role, m.resource_path_kind, \
                    CASE WHEN m.resource_path_kind = {return_value} THEN a.call_site_node_id \
                         WHEN m.resource_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_node_id END AS source_expression_node_id, \
                    CASE WHEN m.resource_path_kind = {return_value} THEN a.call_fact_id \
                         WHEN m.resource_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_fact_id END AS source_expression_fact_id, \
                    CAST(CASE WHEN m.resource_path_kind = {return_value} THEN {call_result} \
                              WHEN m.resource_path_kind = {parameter} AND b.status = {bound} \
                                THEN {bound_argument} ELSE {unknown} END AS SMALLINT) AS source_status, \
                    CAST(CASE WHEN m.resource_path_kind = {return_value} \
                               OR (m.resource_path_kind = {parameter} AND b.status = {bound}) \
                                THEN NULL ELSE COALESCE(b.reason, {outside}) END \
                      AS SMALLINT) AS source_reason, \
                    m.action, m.exit, a.target_modality, m.modality AS model_modality, \
                    a.candidate_set_complete_under_model, a.has_unresolved_remainder, m.origin \
             FROM model_applications a \
             JOIN model_resources m ON m.model_id = a.model_id \
               AND m.target_node_id = a.target_node_id \
               AND m.target_definition_fact_id = a.target_definition_fact_id \
               AND m.revision = a.revision \
             LEFT JOIN model_argument_bindings b \
               ON b.call_site_node_id = a.call_site_node_id \
              AND b.pysa_fact_id = a.pysa_fact_id \
              AND b.model_id = a.model_id AND b.rule_id = m.rule_id \
              AND b.path_id = m.resource_path_id AND b.path_role = m.resource_role",
            return_value = ModelPathKind::ReturnValue.code(),
            parameter = ModelPathKind::Parameter.code(),
            bound = ModelArgumentStatus::Bound.code(),
            call_result = ModelResourceSourceStatus::CallResult.code(),
            bound_argument = ModelResourceSourceStatus::BoundArgument.code(),
            unknown = ModelResourceSourceStatus::Unknown.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
        );

    /// A typed transfer model at one candidate call. For now only an exact formal argument
    /// and the call-result expression identify endpoint sources; other paths remain unknown.
    modeled_transfer_sites = "behavior:modeled_transfer_sites",
        deps = ["model_applications", "model_transfers", "model_argument_bindings"],
        sql = format!(
            "SELECT a.snapshot_id, a.call_site_node_id, a.function_node_id, \
                    a.call_fact_id, a.pysa_fact_id, a.target_node_id, a.model_id, \
                    m.rule_id, m.target_definition_fact_id, \
                    m.input_path_id, m.input_path_kind, \
                    CASE WHEN m.input_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_node_id END AS input_expression_node_id, \
                    CASE WHEN m.input_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_fact_id END AS input_expression_fact_id, \
                    CAST(CASE WHEN m.input_path_kind = {parameter} AND b.status = {bound} \
                                THEN {bound_argument} ELSE {unknown} END AS SMALLINT) AS input_status, \
                    CAST(CASE WHEN m.input_path_kind = {parameter} AND b.status = {bound} \
                                THEN NULL ELSE COALESCE(b.reason, {outside}) END \
                      AS SMALLINT) AS input_reason, \
                    m.output_path_id, m.output_path_kind, \
                    CASE WHEN m.output_path_kind = {return_value} \
                           THEN a.call_site_node_id END AS output_expression_node_id, \
                    CASE WHEN m.output_path_kind = {return_value} \
                           THEN a.call_fact_id END AS output_expression_fact_id, \
                    CAST(CASE WHEN m.output_path_kind = {return_value} \
                                THEN {call_result} ELSE {unknown} END AS SMALLINT) AS output_status, \
                    CAST(CASE WHEN m.output_path_kind = {return_value} \
                                THEN NULL ELSE {outside} END AS SMALLINT) AS output_reason, \
                    m.transfer, a.target_modality, m.modality AS model_modality, \
                    a.candidate_set_complete_under_model, a.has_unresolved_remainder, m.origin \
             FROM model_applications a \
             JOIN model_transfers m ON m.model_id = a.model_id \
               AND m.target_node_id = a.target_node_id \
               AND m.target_definition_fact_id = a.target_definition_fact_id \
               AND m.revision = a.revision \
             LEFT JOIN model_argument_bindings b \
               ON b.call_site_node_id = a.call_site_node_id \
              AND b.pysa_fact_id = a.pysa_fact_id \
              AND b.model_id = a.model_id AND b.rule_id = m.rule_id \
              AND b.path_id = m.input_path_id AND b.path_role = {input}",
            parameter = ModelPathKind::Parameter.code(),
            return_value = ModelPathKind::ReturnValue.code(),
            bound = ModelArgumentStatus::Bound.code(),
            bound_argument = ModelTransferEndpointStatus::BoundArgument.code(),
            call_result = ModelTransferEndpointStatus::CallResult.code(),
            unknown = ModelTransferEndpointStatus::Unknown.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
            input = ModelPathRole::Input.code(),
        );

    /// Only an exact one-call path can currently identify a modeled return or definition
    /// value step. Nested calls and inherited call transfer need predecessor-path evidence;
    /// neither is inferred from a merged flow row or shared source span.
    modeled_exact_value_transfers = "behavior:modeled_exact_value_transfers",
        deps = ["value_flow_contributions", "flow_values", "flow_value_calls", "flow_value_call_links", "modeled_transfer_sites"],
        sql = format!(
            "WITH step_counts AS ( \
               SELECT flow_value_fact_id, count(*) AS n FROM flow_value_calls \
               GROUP BY flow_value_fact_id) \
             SELECT v.snapshot_id, v.flow_value_fact_id, fc.fact_id AS flow_value_call_fact_id, \
                    l.call_node_id AS call_site_node_id, l.call_fact_id, \
                    l.argument_node_id, l.argument_fact_id, v.sink_function_node_id AS function_node_id, \
                    v.parameter_node_id, v.use_id, f.sink, f.sink_start_byte, f.sink_end_byte, \
                    v.condition_id, v.condition, \
                    f.approximated AS raw_flow_approximated, m.pysa_fact_id, m.target_node_id, \
                    m.model_id, m.rule_id, m.target_definition_fact_id, m.transfer, \
                    m.target_modality, m.model_modality, m.candidate_set_complete_under_model, \
                    m.has_unresolved_remainder, m.origin \
             FROM value_flow_contributions v \
             JOIN flow_values f ON f.fact_id = v.flow_value_fact_id \
             JOIN step_counts sc ON sc.flow_value_fact_id = v.flow_value_fact_id AND sc.n = 1 \
             JOIN flow_value_calls fc ON fc.flow_value_fact_id = v.flow_value_fact_id \
               AND fc.step = 0 AND fc.use_id = v.use_id \
               AND fc.call_start_byte = f.sink_start_byte \
               AND fc.call_end_byte = f.sink_end_byte \
             JOIN flow_value_call_links l ON l.flow_value_call_fact_id = fc.fact_id \
               AND l.status = {bound_argument} \
             JOIN modeled_transfer_sites m ON m.call_site_node_id = l.call_node_id \
               AND m.input_expression_node_id = l.argument_node_id \
               AND m.output_expression_node_id = l.call_node_id \
               AND m.input_status = {bound_input} AND m.output_status = {call_result} \
               AND m.function_node_id = v.sink_function_node_id \
             WHERE f.sink IN ({return_sink}, {definition_sink}) \
               AND v.parameter_node_id IS NOT NULL \
               AND v.function_node_id = v.sink_function_node_id \
               AND v.upstream_identity AND v.local_through_call",
            bound_argument = FlowCallLinkStatus::BoundArgument.code(),
            bound_input = ModelTransferEndpointStatus::BoundArgument.code(),
            call_result = ModelTransferEndpointStatus::CallResult.code(),
            return_sink = FlowSink::Return.code(),
            definition_sink = FlowSink::Definition.code(),
        );

    /// Preserve argument evaluation order for each exact modeled value candidate. The source
    /// operand has an observed raw value path; direct literal and exact builtin-name siblings
    /// complete normally without a further effect. All other siblings are named unknowns.
    modeled_argument_evaluations = "behavior:modeled_argument_evaluations",
        deps = ["modeled_exact_value_transfers", "call_syntax", "arguments", "syntax_nodes", "references", "reference_resolutions"],
        sql = format!(
            "WITH literal_candidates AS ( \
               SELECT a.fact_id AS argument_fact_id, s.fact_id AS syntax_fact_id, \
                      count(*) OVER (PARTITION BY a.fact_id) AS candidate_count \
               FROM arguments a \
               JOIN call_syntax c ON c.node_id = a.call_node_id \
               JOIN syntax_nodes s ON s.module_node_id = c.module_node_id \
                 AND s.parent_node_id = c.node_id AND s.field = {argument_field} \
                 AND s.start_byte = a.value_start_byte AND s.end_byte = a.value_end_byte \
                 AND s.kind IN ({string_literal}, {bytes_literal}, {number_literal}, \
                                {boolean_literal}, {none_literal}, {ellipsis_literal}) \
               WHERE a.kind IN ({positional}, {keyword}) \
             ), builtin_candidates AS ( \
               SELECT a.fact_id AS argument_fact_id, rr.fact_id AS resolution_fact_id, \
                      count(*) OVER (PARTITION BY a.fact_id) AS candidate_count \
               FROM arguments a \
               JOIN call_syntax c ON c.node_id = a.call_node_id \
               JOIN syntax_nodes s ON s.module_node_id = c.module_node_id \
                 AND s.parent_node_id = c.node_id AND s.field = {argument_field} \
                 AND s.start_byte = a.value_start_byte AND s.end_byte = a.value_end_byte \
                 AND s.kind = {name_expr} \
               JOIN references r ON r.name_node_id = s.node_id \
                 AND r.module_node_id = s.module_node_id \
               JOIN reference_resolutions rr ON rr.reference_id = r.node_id \
                 AND rr.builtin_name = r.name AND rr.binding_id IS NULL AND rr.reason IS NULL \
               WHERE a.kind IN ({positional}, {keyword}) \
             ) \
             SELECT m.snapshot_id, m.flow_value_fact_id AS candidate_flow_fact_id, \
                    m.parameter_node_id, m.pysa_fact_id, m.model_id, m.rule_id, \
                    m.call_site_node_id, a.node_id AS argument_node_id, \
                    a.fact_id AS argument_fact_id, a.ordinal, \
                    CAST(CASE WHEN a.fact_id = m.argument_fact_id THEN {source_operand} \
                              WHEN l.syntax_fact_id IS NOT NULL THEN {literal_normal} \
                              WHEN b.resolution_fact_id IS NOT NULL THEN {builtin_normal} \
                              ELSE {unknown} END AS SMALLINT) AS status, \
                    CASE WHEN a.fact_id = m.argument_fact_id THEN m.flow_value_fact_id \
                         ELSE COALESCE(l.syntax_fact_id, b.resolution_fact_id) END AS evidence_id, \
                    CAST(CASE WHEN a.fact_id = m.argument_fact_id OR l.syntax_fact_id IS NOT NULL \
                                   OR b.resolution_fact_id IS NOT NULL \
                              THEN NULL ELSE {outside} END AS SMALLINT) AS reason, \
                    m.condition_id \
             FROM modeled_exact_value_transfers m \
             JOIN arguments a ON a.call_node_id = m.call_site_node_id \
             LEFT JOIN literal_candidates l ON l.argument_fact_id = a.fact_id \
               AND l.candidate_count = 1 \
             LEFT JOIN builtin_candidates b ON b.argument_fact_id = a.fact_id \
               AND b.candidate_count = 1",
            source_operand = ModeledArgumentEvaluationStatus::SourceOperand.code(),
            literal_normal = ModeledArgumentEvaluationStatus::LiteralNormal.code(),
            builtin_normal = ModeledArgumentEvaluationStatus::BuiltinNameNormal.code(),
            unknown = ModeledArgumentEvaluationStatus::Unknown.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
            argument_field = crate::codebook::SyntaxField::Argument.code(),
            positional = ArgumentKind::Positional.code(),
            keyword = ArgumentKind::Keyword.code(),
            name_expr = SyntaxKind::ExprName.code(),
            string_literal = SyntaxKind::ExprStringLiteral.code(),
            bytes_literal = SyntaxKind::ExprBytesLiteral.code(),
            number_literal = SyntaxKind::ExprNumberLiteral.code(),
            boolean_literal = SyntaxKind::ExprBooleanLiteral.code(),
            none_literal = SyntaxKind::ExprNoneLiteral.code(),
            ellipsis_literal = SyntaxKind::ExprEllipsisLiteral.code(),
        );

    /// The provider's reaching definition gives a predecessor value expression, but its
    /// possible value facts and their path conditions are retained separately. This is a
    /// candidate edge, not a proof that exactly one earlier call supplied the successor.
    value_flow_predecessor_candidates = "behavior:value_flow_predecessor_candidates",
        deps = ["value_flow_contributions", "flow_values", "flow_reaching", "flow_definitions"],
        sql = format!(
            "SELECT c.snapshot_id, c.flow_value_fact_id AS successor_fact_id, \
                    p.flow_value_fact_id AS predecessor_fact_id, c.source_key, \
                    c.parameter_node_id, c.sink_function_node_id AS function_node_id, \
                    c.use_id AS successor_use_id, p.use_id AS predecessor_use_id, \
                    r.fact_id AS reaching_fact_id, r.condition_id AS reaching_condition_id, \
                    r.approximated AS reaching_approximated, r.loop_carried, \
                    d.definition_id, d.fact_id AS definition_fact_id, \
                    c.condition_id AS successor_condition_id, \
                    p.condition_id AS predecessor_condition_id, \
                    sf.approximated AS successor_raw_approximated, \
                    pf.approximated AS predecessor_raw_approximated, \
                    p.local_through_call AS predecessor_local_through_call, \
                    p.upstream_through_call AS predecessor_upstream_through_call \
             FROM value_flow_contributions c \
             JOIN flow_values sf ON sf.fact_id = c.flow_value_fact_id \
             JOIN flow_reaching r ON r.use_id = c.use_id \
               AND r.definition_id IS NOT NULL \
             JOIN flow_definitions d ON d.definition_id = r.definition_id \
               AND d.module_node_id = sf.module_node_id \
               AND d.value_start_byte IS NOT NULL AND d.value_end_byte IS NOT NULL \
             JOIN flow_values pf ON pf.module_node_id = d.module_node_id \
               AND pf.sink = {definition_sink} \
               AND pf.sink_start_byte = d.value_start_byte \
               AND pf.sink_end_byte = d.value_end_byte \
             JOIN value_flow_contributions p ON p.flow_value_fact_id = pf.fact_id \
               AND p.source_key = c.source_key \
               AND p.parameter_node_id = c.parameter_node_id \
               AND p.sink_function_node_id = c.sink_function_node_id \
             WHERE c.upstream_through_call AND c.parameter_node_id IS NOT NULL \
               AND c.function_node_id = c.sink_function_node_id",
            definition_sink = FlowSink::Definition.code(),
        );

    /// Preserve each cited assignment-to-return path separately. An identity raw return use
    /// excludes computed outer expressions; the predecessor's whole value has the exact pinned
    /// model step. Compatibility is only a may-path check, not normal completion.
    modeled_assignment_return_paths = "behavior:modeled_assignment_return_paths",
        deps = ["value_flow_predecessor_candidates", "value_flow_predecessor_compatibility",
                "modeled_exact_value_transfers", "flow_values"],
        sql = format!(
            "SELECT p.snapshot_id, p.successor_fact_id, p.predecessor_fact_id, \
                    p.reaching_fact_id, p.function_node_id, p.parameter_node_id, \
                    m.call_site_node_id, m.call_fact_id, m.pysa_fact_id, m.model_id, m.rule_id, \
                    m.target_definition_fact_id, p.predecessor_condition_id, \
                    p.reaching_condition_id, p.successor_condition_id, \
                    c.compatible_under_atoms, c.boundary_reason, m.transfer, \
                    m.target_modality, m.model_modality, \
                    m.candidate_set_complete_under_model, m.has_unresolved_remainder, \
                    p.predecessor_raw_approximated, p.reaching_approximated, \
                    p.successor_raw_approximated \
             FROM value_flow_predecessor_candidates p \
             JOIN value_flow_predecessor_compatibility c \
               ON c.snapshot_id = p.snapshot_id \
              AND c.successor_fact_id = p.successor_fact_id \
              AND c.predecessor_fact_id = p.predecessor_fact_id \
              AND c.source_key = p.source_key \
              AND c.reaching_fact_id = p.reaching_fact_id \
             JOIN modeled_exact_value_transfers m \
               ON m.snapshot_id = p.snapshot_id \
              AND m.flow_value_fact_id = p.predecessor_fact_id \
              AND m.parameter_node_id = p.parameter_node_id \
              AND m.function_node_id = p.function_node_id \
              AND m.use_id = p.predecessor_use_id \
              AND m.condition_id = p.predecessor_condition_id \
              AND m.sink = {definition_sink} \
             JOIN flow_values s ON s.fact_id = p.successor_fact_id \
               AND s.sink = {return_sink} AND s.identity AND NOT s.through_call \
               AND s.use_id = p.successor_use_id",
            definition_sink = FlowSink::Definition.code(),
            return_sink = FlowSink::Return.code(),
        );

    /// Initial finite summary seeds: an identity raw value returned directly from a
    /// synchronous function body, with no local/inherited call crossing or generator yield.
    /// The condition kernel decides whether each seed is admitted or bounded.
    summary_flow_seeds = "behavior:summary_flow_seeds",
        deps = ["value_flow_contributions", "flow_values", "flow_value_calls", "parameter_syntax",
                "syntax_nodes", "declarations", "exit_sites"],
        sql = format!(
            "SELECT DISTINCT v.snapshot_id, v.sink_function_node_id AS function_node_id, \
                    v.parameter_node_id, p.name AS parameter_name, \
                    v.flow_value_fact_id AS source_flow_fact_id, v.condition_id, \
                    e.source_fact_id AS return_site_fact_id, \
                    e.region_fact_id AS return_region_fact_id, \
                    (f.approximated OR e.approximated) AS approximated \
             FROM value_flow_contributions v \
             JOIN flow_values f ON f.fact_id = v.flow_value_fact_id \
             JOIN parameter_syntax p ON p.node_id = v.parameter_node_id \
               AND p.function_node_id = v.sink_function_node_id \
             JOIN declarations d ON d.node_id = v.sink_function_node_id \
               AND d.kind = {function_kind} \
             JOIN syntax_nodes r ON r.owner_node_id = d.node_id \
               AND r.parent_node_id = d.node_id AND r.field = {body_field} \
               AND r.kind = {return_kind} AND r.module_node_id = f.module_node_id \
               AND r.start_byte <= f.sink_start_byte AND r.end_byte >= f.sink_end_byte \
             JOIN exit_sites e ON e.site_node_id = r.node_id \
               AND e.function_node_id = d.node_id AND e.kind = {exit_return} \
             WHERE f.sink = {return_sink} AND f.identity AND NOT f.through_call \
               AND v.identity AND v.upstream_identity AND NOT v.through_call \
               AND NOT v.local_through_call AND NOT v.upstream_through_call \
               AND NOT v.captured AND v.function_node_id = v.sink_function_node_id \
               AND NOT EXISTS (SELECT 1 FROM flow_value_calls c \
                 WHERE c.flow_value_fact_id = f.fact_id) \
               AND NOT EXISTS (SELECT 1 FROM syntax_nodes y \
                 WHERE y.owner_node_id = d.node_id \
                   AND y.kind IN ({yield_kind}, {yield_from_kind}))",
            function_kind = DeclarationKind::Function.code(),
            body_field = crate::codebook::SyntaxField::Body.code(),
            return_kind = SyntaxKind::StmtReturn.code(),
            exit_return = ExitSiteKind::Return.code(),
            return_sink = FlowSink::Return.code(),
            yield_kind = SyntaxKind::ExprYield.code(),
            yield_from_kind = SyntaxKind::ExprYieldFrom.code(),
        );

    /// A direct return whose entire expression is one exact pinned identity-model call.
    /// This relation selects attributed source/model facts; the bounded kernel and ordered
    /// argument-evaluation proof still decide admission in `cpg-core::summaries`.
    modeled_summary_flow_seeds = "behavior:modeled_summary_flow_seeds",
        deps = ["modeled_exact_value_transfers", "model_applications", "model_transfers",
                "call_syntax", "syntax_nodes", "references", "reference_resolutions",
                "parameter_syntax", "declarations", "exit_sites", "flow_values"],
        sql = format!(
            "WITH {callee_candidates} \
             SELECT DISTINCT m.snapshot_id, m.function_node_id, m.parameter_node_id, \
                    p.name AS parameter_name, m.flow_value_fact_id AS source_flow_fact_id, \
                    m.condition_id, m.call_fact_id, \
                    m.argument_fact_id AS source_argument_fact_id, m.pysa_fact_id, \
                    m.model_id, m.rule_id, cc.resolution_fact_id AS callee_resolution_fact_id, \
                    c.positional_count + c.keyword_count AS argument_count, \
                    e.source_fact_id AS return_site_fact_id, \
                    e.region_fact_id AS return_region_fact_id, \
                    e.condition_id AS return_condition_id, \
                    (f.approximated OR e.approximated) AS approximated \
             FROM modeled_exact_value_transfers m \
             JOIN model_applications a ON a.call_site_node_id = m.call_site_node_id \
               AND a.pysa_fact_id = m.pysa_fact_id AND a.model_id = m.model_id \
               AND a.target_node_id = m.target_node_id \
             JOIN model_transfers t ON t.rule_id = m.rule_id AND t.model_id = m.model_id \
               AND t.target_node_id = m.target_node_id \
             JOIN call_syntax c ON c.node_id = m.call_site_node_id \
             JOIN callee_candidates cc ON cc.call_node_id = c.node_id \
               AND cc.candidate_count = 1 \
             JOIN parameter_syntax p ON p.node_id = m.parameter_node_id \
               AND p.function_node_id = m.function_node_id \
             JOIN declarations d ON d.node_id = m.function_node_id \
               AND d.kind = {function_kind} \
             JOIN syntax_nodes r ON r.owner_node_id = d.node_id \
               AND r.parent_node_id = d.node_id AND r.field = {body_field} \
               AND r.kind = {return_kind} AND r.module_node_id = c.module_node_id \
               AND r.start_byte <= m.sink_start_byte AND r.end_byte >= m.sink_end_byte \
             JOIN exit_sites e ON e.site_node_id = r.node_id \
               AND e.function_node_id = d.node_id AND e.kind = {exit_return} \
             JOIN flow_values f ON f.fact_id = m.flow_value_fact_id \
             WHERE m.sink = {return_sink} AND m.transfer = {identity} \
               AND m.target_modality = {definite} AND m.model_modality = {definite} \
               AND m.candidate_set_complete_under_model AND NOT m.has_unresolved_remainder \
               AND a.target_count = 1 AND a.target_normal_return \
               AND a.phase = {call_phase} \
               AND NOT EXISTS (SELECT 1 FROM syntax_nodes y \
                 WHERE y.owner_node_id = d.node_id \
                   AND y.kind IN ({yield_kind}, {yield_from_kind}))",
            callee_candidates = modeled_callee_candidates_sql(),
            function_kind = DeclarationKind::Function.code(),
            body_field = crate::codebook::SyntaxField::Body.code(),
            return_kind = SyntaxKind::StmtReturn.code(),
            exit_return = ExitSiteKind::Return.code(),
            return_sink = FlowSink::Return.code(),
            identity = ModelTransferKind::Identity.code(),
            definite = Modality::Definite.code(),
            call_phase = InvocationPhase::Call.code(),
            yield_kind = SyntaxKind::ExprYield.code(),
            yield_from_kind = SyntaxKind::ExprYieldFrom.code(),
        );

    /// The two-hop assignment form of the same pinned identity call. The successor must have
    /// exactly one provider reaching row, and the independent predecessor compatibility check
    /// must admit it. The kernel still verifies all four condition implications at admission.
    modeled_assignment_summary_flow_seeds = "behavior:modeled_assignment_summary_flow_seeds",
        deps = ["modeled_assignment_return_paths", "modeled_exact_value_transfers",
                "model_applications", "model_transfers", "call_syntax", "references",
                "reference_resolutions", "parameter_syntax", "declarations",
                "syntax_nodes", "exit_sites", "flow_values", "flow_reaching"],
        sql = format!(
            "WITH {callee_candidates}, reaching_counts AS ( \
               SELECT use_id, count(*) AS n FROM flow_reaching GROUP BY use_id \
             ) \
             SELECT DISTINCT q.snapshot_id, q.function_node_id, q.parameter_node_id, \
                    p.name AS parameter_name, q.successor_fact_id AS source_flow_fact_id, \
                    q.predecessor_fact_id AS predecessor_flow_fact_id, \
                    q.reaching_fact_id, q.predecessor_condition_id, \
                    q.reaching_condition_id, q.successor_condition_id, \
                    m.argument_fact_id AS source_argument_fact_id, m.call_fact_id, \
                    q.pysa_fact_id, q.model_id, q.rule_id, \
                    cc.resolution_fact_id AS callee_resolution_fact_id, \
                    c.positional_count + c.keyword_count AS argument_count, \
                    e.source_fact_id AS return_site_fact_id, \
                    e.region_fact_id AS return_region_fact_id, \
                    e.condition_id AS return_condition_id, \
                    (q.predecessor_raw_approximated OR q.reaching_approximated \
                     OR q.successor_raw_approximated OR e.approximated) AS approximated \
             FROM modeled_assignment_return_paths q \
             JOIN modeled_exact_value_transfers m ON m.flow_value_fact_id = q.predecessor_fact_id \
               AND m.parameter_node_id = q.parameter_node_id \
               AND m.pysa_fact_id = q.pysa_fact_id AND m.model_id = q.model_id \
               AND m.rule_id = q.rule_id AND m.sink = {definition_sink} \
             JOIN model_applications a ON a.call_site_node_id = m.call_site_node_id \
               AND a.pysa_fact_id = m.pysa_fact_id AND a.model_id = m.model_id \
               AND a.target_node_id = m.target_node_id \
             JOIN model_transfers t ON t.rule_id = m.rule_id AND t.model_id = m.model_id \
               AND t.target_node_id = m.target_node_id \
             JOIN call_syntax c ON c.node_id = m.call_site_node_id \
             JOIN callee_candidates cc ON cc.call_node_id = c.node_id \
               AND cc.candidate_count = 1 \
             JOIN parameter_syntax p ON p.node_id = q.parameter_node_id \
               AND p.function_node_id = q.function_node_id \
             JOIN declarations d ON d.node_id = q.function_node_id \
               AND d.kind = {function_kind} \
             JOIN flow_values s ON s.fact_id = q.successor_fact_id \
               AND s.sink = {return_sink} \
             JOIN flow_reaching h ON h.fact_id = q.reaching_fact_id \
               AND h.use_id = s.use_id \
             JOIN reaching_counts rc ON rc.use_id = h.use_id AND rc.n = 1 \
             JOIN syntax_nodes r ON r.owner_node_id = d.node_id \
               AND r.parent_node_id = d.node_id AND r.field = {body_field} \
               AND r.kind = {return_kind} AND r.module_node_id = s.module_node_id \
               AND r.start_byte <= s.sink_start_byte AND r.end_byte >= s.sink_end_byte \
             JOIN exit_sites e ON e.site_node_id = r.node_id \
               AND e.function_node_id = d.node_id AND e.kind = {exit_return} \
             WHERE q.compatible_under_atoms AND q.boundary_reason IS NULL \
               AND q.transfer = {identity} AND q.target_modality = {definite} \
               AND q.model_modality = {definite} \
               AND q.candidate_set_complete_under_model AND NOT q.has_unresolved_remainder \
               AND a.target_count = 1 AND a.target_normal_return \
               AND a.phase = {call_phase} \
               AND NOT EXISTS (SELECT 1 FROM syntax_nodes y \
                 WHERE y.owner_node_id = d.node_id \
                   AND y.kind IN ({yield_kind}, {yield_from_kind}))",
            callee_candidates = modeled_callee_candidates_sql(),
            definition_sink = FlowSink::Definition.code(),
            function_kind = DeclarationKind::Function.code(),
            return_sink = FlowSink::Return.code(),
            body_field = crate::codebook::SyntaxField::Body.code(),
            return_kind = SyntaxKind::StmtReturn.code(),
            exit_return = ExitSiteKind::Return.code(),
            identity = ModelTransferKind::Identity.code(),
            definite = Modality::Definite.code(),
            call_phase = InvocationPhase::Call.code(),
            yield_kind = SyntaxKind::ExprYield.code(),
            yield_from_kind = SyntaxKind::ExprYieldFrom.code(),
        );

    /// Unknown-is-not-absent for every same-callable parameter-origin return fact not admitted
    /// by the finite direct producer. A crossed call takes the more specific transfer reason;
    /// other shapes await their control/execution proof.
    summary_boundaries = "behavior:summary_boundaries",
        deps = ["value_flow_contributions", "flow_values", "summary_flows"],
        sql = format!(
            "WITH remaining AS ( \
               SELECT v.snapshot_id, v.sink_function_node_id AS function_node_id, \
                      v.parameter_node_id, v.flow_value_fact_id AS source_flow_fact_id, \
                      v.condition_id, v.through_call, v.local_through_call, \
                      v.upstream_through_call, f.through_call AS raw_through_call, \
                      f.approximated AS raw_approximated \
               FROM value_flow_contributions v \
               JOIN flow_values f ON f.fact_id = v.flow_value_fact_id \
               WHERE f.sink = {return_sink} AND v.parameter_node_id IS NOT NULL \
                 AND v.function_node_id = v.sink_function_node_id \
                 AND NOT EXISTS (SELECT 1 FROM summary_flows s \
                     WHERE s.function_node_id = v.sink_function_node_id \
                       AND s.parameter_node_id = v.parameter_node_id \
                       AND s.source_flow_fact_id = v.flow_value_fact_id \
                       AND s.condition_id = v.condition_id) \
             ) SELECT snapshot_id, function_node_id, parameter_node_id, \
                      source_flow_fact_id, condition_id, \
                    CAST(CASE WHEN MAX(CASE WHEN through_call OR local_through_call \
                             OR upstream_through_call OR raw_through_call THEN 1 ELSE 0 END) > 0 \
                              THEN {call_transfer} ELSE {control} END AS SMALLINT) AS reason, \
                    MAX(CASE WHEN local_through_call THEN 1 ELSE 0 END) > 0 AS local_through_call, \
                    MAX(CASE WHEN upstream_through_call THEN 1 ELSE 0 END) > 0 AS upstream_through_call, \
                    MAX(CASE WHEN raw_approximated THEN 1 ELSE 0 END) > 0 AS raw_approximated \
               FROM remaining GROUP BY snapshot_id, function_node_id, parameter_node_id, \
                    source_flow_fact_id, condition_id",
            call_transfer = BoundaryReason::CallTransfer.code(),
            control = BoundaryReason::UnsupportedControlFlow.code(),
            return_sink = FlowSink::Return.code(),
        );

    /// A subjectless effect stays unqualified; a parameter subject needs an exact binding.
    /// The candidate call's condition/exit is composed later.
    modeled_effect_sites = "behavior:modeled_effect_sites",
        deps = ["model_applications", "model_effects", "model_argument_bindings"],
        sql = format!(
            "SELECT a.snapshot_id, a.call_site_node_id, a.function_node_id, \
                    a.call_fact_id, a.pysa_fact_id, a.target_node_id, a.model_id, \
                    m.rule_id, m.target_definition_fact_id, m.effect, m.argument, \
                    m.subject_path_id, m.subject_path_kind, \
                    CASE WHEN m.subject_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_node_id END AS subject_expression_node_id, \
                    CASE WHEN m.subject_path_kind = {parameter} AND b.status = {bound} \
                           THEN b.argument_fact_id END AS subject_expression_fact_id, \
                    CAST(CASE WHEN m.subject_path_id IS NULL THEN {unqualified} \
                              WHEN m.subject_path_kind = {parameter} AND b.status = {bound} \
                                THEN {bound_argument} ELSE {unknown} END \
                      AS SMALLINT) AS subject_status, \
                    CAST(CASE WHEN m.subject_path_id IS NULL \
                               OR (m.subject_path_kind = {parameter} AND b.status = {bound}) \
                                THEN NULL ELSE COALESCE(b.reason, {outside}) END \
                      AS SMALLINT) AS subject_reason, \
                    a.target_modality, m.modality AS model_modality, \
                    a.candidate_set_complete_under_model, a.has_unresolved_remainder, m.origin \
             FROM model_applications a \
             JOIN model_effects m ON m.model_id = a.model_id \
               AND m.target_node_id = a.target_node_id \
               AND m.target_definition_fact_id = a.target_definition_fact_id \
               AND m.revision = a.revision \
             LEFT JOIN model_argument_bindings b \
               ON b.call_site_node_id = a.call_site_node_id \
              AND b.pysa_fact_id = a.pysa_fact_id \
              AND b.model_id = a.model_id AND b.rule_id = m.rule_id \
              AND b.path_id = m.subject_path_id AND b.path_role = {input}",
            parameter = ModelPathKind::Parameter.code(),
            bound = ModelArgumentStatus::Bound.code(),
            unqualified = ModelEffectSubjectStatus::Unqualified.code(),
            bound_argument = ModelEffectSubjectStatus::BoundArgument.code(),
            unknown = ModelEffectSubjectStatus::Unknown.code(),
            outside = BoundaryReason::OutsideProviderModel.code(),
            input = ModelPathRole::Input.code(),
        );

    /// A pinned exception class and action applied at a candidate source call. The call may
    /// raise under the model; this row does not yet decide a handler or exceptional exit.
    modeled_exception_sites = "behavior:modeled_exception_sites",
        deps = ["model_applications", "model_exceptions"],
        sql = "SELECT a.snapshot_id, a.call_site_node_id, a.function_node_id, \
                     a.call_fact_id, a.pysa_fact_id, a.target_node_id, a.model_id, \
                     m.rule_id, m.target_definition_fact_id, m.class, m.class_node_id, \
                     m.class_fact_id, m.action, m.to_class, m.to_class_node_id, \
                     m.to_class_fact_id, a.target_modality, m.modality AS model_modality, \
                     a.candidate_set_complete_under_model, a.has_unresolved_remainder, m.origin \
              FROM model_applications a JOIN model_exceptions m \
                ON m.model_id = a.model_id AND m.target_node_id = a.target_node_id \
               AND m.target_definition_fact_id = a.target_definition_fact_id \
               AND m.revision = a.revision".to_owned();

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

    /// Walk only syntax ancestors in the same innermost function. A child whose parent is a
    /// `try` and whose field is its body is inside that frame. The explicit depth cap keeps a
    /// malformed/cyclic syntax relation finite; absence of a row never proves no handler.
    modeled_exception_handler_candidates = "behavior:modeled_exception_handler_candidates",
        deps = ["modeled_exception_sites", "syntax_nodes", "handler_clauses", "handler_types", "context_definitions", "context_class_mro"],
        sql = format!(
            "{climb}, candidates AS ( \
             SELECT e.snapshot_id, e.call_site_node_id, e.pysa_fact_id, e.model_id, e.rule_id, \
                    e.call_fact_id, e.class_node_id AS raised_class_node_id, \
                    e.class_fact_id AS raised_class_fact_id, h.try_node_id, h.handler_node_id, \
                    h.handler_fact_id, c.depth AS frame_depth, h.ordinal AS handler_ordinal, \
                    t.status AS handler_type_status, t.class_node_id AS handler_class_node_id, \
                    t.class_fact_id AS handler_class_fact_id, cm.fact_id AS class_mro_fact_id, \
                    CAST(CASE WHEN t.status = {bare} THEN {bare_match} \
                              WHEN t.status <> {pinned} THEN {unknown_type} \
                              WHEN t.class_node_id = e.class_node_id THEN {same_class} \
                              WHEN cm.fact_id IS NOT NULL THEN {pinned_ancestor} \
                              ELSE {unknown_relation} END AS SMALLINT) AS class_match \
             FROM climb c JOIN handler_clauses h \
               ON h.try_node_id = c.parent_node_id AND h.function_node_id = c.function_node_id \
             JOIN handler_types t ON t.handler_node_id = h.handler_node_id \
             JOIN modeled_exception_sites e \
               ON e.call_site_node_id = c.call_site_node_id AND e.pysa_fact_id = c.pysa_fact_id \
              AND e.model_id = c.model_id AND e.rule_id = c.rule_id \
             LEFT JOIN context_definitions d ON d.symbol_node_id = t.class_node_id \
               AND d.fact_id = t.class_fact_id \
             LEFT JOIN context_class_mro cm ON cm.class_node_id = e.class_node_id \
               AND cm.ancestor_module = d.module_name AND cm.ancestor_key = d.key \
               AND NOT cm.cyclic \
             WHERE c.field = {body} \
             ), ranked AS ( \
               SELECT *, \
                 COALESCE(SUM(CASE WHEN class_match IN ({same_class}, {bare_match}, {pinned_ancestor}) \
                   THEN 1 ELSE 0 END) OVER (PARTITION BY call_site_node_id, pysa_fact_id, \
                     model_id, rule_id, try_node_id ORDER BY handler_ordinal \
                     ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING), 0) AS earlier_positive, \
                 COALESCE(SUM(CASE WHEN class_match IN ({unknown_type}, {unknown_relation}) \
                   THEN 1 ELSE 0 END) OVER (PARTITION BY call_site_node_id, pysa_fact_id, \
                     model_id, rule_id, try_node_id ORDER BY handler_ordinal \
                     ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING), 0) AS earlier_unknown \
               FROM candidates \
             ) \
             SELECT snapshot_id, call_site_node_id, pysa_fact_id, model_id, rule_id, \
                    call_fact_id, raised_class_node_id, raised_class_fact_id, try_node_id, \
                    handler_node_id, handler_fact_id, frame_depth, handler_ordinal, \
                    handler_type_status, handler_class_node_id, handler_class_fact_id, \
                    class_mro_fact_id, class_match, earlier_positive = 0 AS frame_possible, \
                    class_match IN ({same_class}, {bare_match}, {pinned_ancestor}) \
                      AND earlier_positive = 0 AND earlier_unknown = 0 \
                      AS frame_first_match_if_raised \
             FROM ranked",
            climb = modeled_handler_climb_sql(),
            bare = HandlerTypeStatus::Bare.code(),
            pinned = HandlerTypeStatus::PinnedBuiltin.code(),
            bare_match = ModeledHandlerClassMatch::Bare.code(),
            unknown_type = ModeledHandlerClassMatch::HandlerTypeUnknown.code(),
            same_class = ModeledHandlerClassMatch::SameClass.code(),
            unknown_relation = ModeledHandlerClassMatch::ClassRelationUnknown.code(),
            pinned_ancestor = ModeledHandlerClassMatch::PinnedAncestor.code(),
            body = crate::codebook::SyntaxField::Body.code(),
        );

    /// One coverage row per modeled raise, including a missing syntax anchor or a capped walk.
    /// The candidate relation can only be interpreted as complete for a row with null reason.
    modeled_exception_handler_walks = "behavior:modeled_exception_handler_walks",
        deps = ["modeled_exception_sites", "syntax_nodes"],
        sql = format!(
            "{climb}, stats AS ( \
               SELECT c.call_site_node_id, c.pysa_fact_id, c.model_id, c.rule_id, \
                      MAX(c.depth) AS max_depth, \
                      MAX(CASE WHEN c.depth = {max_depth} AND p.node_id IS NOT NULL \
                               THEN 1 ELSE 0 END) AS capped \
               FROM climb c LEFT JOIN syntax_nodes p ON p.node_id = c.parent_node_id \
                 AND p.owner_node_id = c.function_node_id \
               GROUP BY c.call_site_node_id, c.pysa_fact_id, c.model_id, c.rule_id \
             ) \
             SELECT e.snapshot_id, e.call_site_node_id, e.pysa_fact_id, e.model_id, e.rule_id, \
                    s.fact_id AS source_syntax_fact_id, \
                    COALESCE(st.max_depth, CAST(0 AS BIGINT)) AS max_depth, \
                    CAST(CASE WHEN s.fact_id IS NULL THEN {missing} \
                              WHEN st.capped > 0 THEN {budget} ELSE NULL END AS SMALLINT) AS reason \
             FROM modeled_exception_sites e \
             LEFT JOIN syntax_nodes s ON s.node_id = e.call_site_node_id \
             LEFT JOIN stats st ON st.call_site_node_id = e.call_site_node_id \
               AND st.pysa_fact_id = e.pysa_fact_id AND st.model_id = e.model_id \
               AND st.rule_id = e.rule_id \
             WHERE e.action = {raise}",
            climb = modeled_handler_climb_sql(),
            max_depth = MODELED_HANDLER_MAX_ANCESTOR_DEPTH,
            missing = BoundaryReason::MissingEvidence.code(),
            budget = BoundaryReason::BudgetReached.code(),
            raise = ModelExceptionAction::Raise.code(),
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

    /// Positive, site-local normal-return witness. Count authored direct statements from
    /// syntax, not from `handler_actions`, so a missing flow region cannot make a two-statement
    /// body look like a sole return. The exact `None` child avoids interpreting source text.
    handler_return_none_sites = "behavior:handler_return_none_sites",
        deps = ["handler_clauses", "handler_actions", "syntax_nodes"],
        sql = format!(
            "WITH body_counts AS ( \
               SELECT h.handler_node_id, count(s.node_id) AS actions \
               FROM handler_clauses h LEFT JOIN syntax_nodes s \
                 ON s.parent_node_id = h.handler_node_id AND s.field = {body} \
               GROUP BY h.handler_node_id \
             ), value_counts AS ( \
               SELECT parent_node_id, count(*) AS values \
               FROM syntax_nodes WHERE field = {value} GROUP BY parent_node_id \
             ) \
             SELECT h.snapshot_id, h.function_node_id, h.handler_node_id, \
                    h.handler_fact_id, a.action_node_id AS return_node_id, \
                    a.action_fact_id AS return_fact_id, n.node_id AS none_node_id, \
                    n.fact_id AS none_fact_id, a.region_fact_id, a.condition_id, \
                    a.approximated \
             FROM handler_clauses h \
             JOIN body_counts b ON b.handler_node_id = h.handler_node_id AND b.actions = 1 \
             JOIN handler_actions a ON a.handler_node_id = h.handler_node_id \
               AND a.action_kind = {return_kind} \
             JOIN value_counts v ON v.parent_node_id = a.action_node_id AND v.values = 1 \
             JOIN syntax_nodes n ON n.parent_node_id = a.action_node_id \
               AND n.field = {value} AND n.kind = {none_kind}",
            body = crate::codebook::SyntaxField::Body.code(),
            value = crate::codebook::SyntaxField::Value.code(),
            return_kind = SyntaxKind::StmtReturn.code(),
            none_kind = SyntaxKind::ExprNoneLiteral.code(),
        );

    /// Compose one exact local candidate path only. A complete ancestry walk is required;
    /// an inner `try`/`with` or an outer/finally controller is withheld rather than assumed
    /// to propagate or preserve the handler's return. The target/model modalities remain
    /// in the result, so this relation cannot itself become a definite operation verdict.
    modeled_exception_return_none_paths = "behavior:modeled_exception_return_none_paths",
        deps = ["modeled_exception_handler_candidates", "modeled_exception_handler_walks", "handler_return_none_sites", "modeled_exception_sites", "syntax_nodes"],
        sql = format!(
            "{climb} SELECT c.snapshot_id, c.call_site_node_id, c.pysa_fact_id, c.model_id, \
                    c.rule_id, c.call_fact_id, w.source_syntax_fact_id, \
                    c.raised_class_node_id, c.raised_class_fact_id, c.try_node_id, \
                    c.handler_node_id, c.handler_fact_id, c.handler_ordinal, c.frame_depth, \
                    c.class_match, c.class_mro_fact_id, r.return_node_id, r.return_fact_id, \
                    r.none_node_id, r.none_fact_id, r.region_fact_id AS handler_region_fact_id, \
                    r.condition_id AS handler_condition_id, \
                    r.approximated AS handler_region_approximated, e.target_modality, \
                    e.model_modality, e.candidate_set_complete_under_model, \
                    e.has_unresolved_remainder \
             FROM modeled_exception_handler_candidates c \
             JOIN modeled_exception_handler_walks w \
               ON w.call_site_node_id = c.call_site_node_id \
              AND w.pysa_fact_id = c.pysa_fact_id AND w.model_id = c.model_id \
              AND w.rule_id = c.rule_id AND w.reason IS NULL \
             JOIN modeled_exception_sites e \
               ON e.call_site_node_id = c.call_site_node_id \
              AND e.pysa_fact_id = c.pysa_fact_id AND e.model_id = c.model_id \
              AND e.rule_id = c.rule_id \
             JOIN handler_return_none_sites r ON r.handler_node_id = c.handler_node_id \
             JOIN syntax_nodes t ON t.node_id = c.try_node_id \
               AND t.parent_node_id = e.function_node_id AND t.field = {body} \
             WHERE c.frame_first_match_if_raised \
               AND NOT EXISTS (SELECT 1 FROM syntax_nodes f \
                 WHERE f.parent_node_id = c.try_node_id AND f.field = {finalbody}) \
               AND NOT EXISTS (SELECT 1 FROM climb b \
                 JOIN syntax_nodes s ON s.node_id = b.node_id \
                 WHERE b.call_site_node_id = c.call_site_node_id \
                   AND b.pysa_fact_id = c.pysa_fact_id AND b.model_id = c.model_id \
                   AND b.rule_id = c.rule_id AND b.depth <= c.frame_depth \
                   AND s.kind IN ({try_kind}, {with_kind}))",
            climb = modeled_handler_climb_sql(),
            body = crate::codebook::SyntaxField::Body.code(),
            finalbody = crate::codebook::SyntaxField::Finalbody.code(),
            try_kind = SyntaxKind::StmtTry.code(),
            with_kind = SyntaxKind::StmtWith.code(),
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
