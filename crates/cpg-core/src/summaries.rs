//! The first L3 source-path check: hydrate published-form analysis conditions, then ask the
//! bounded BDD kernel whether a cited predecessor edge is propositionally compatible.

use std::collections::HashMap;

use cpg_schema::behavior::{
    AnalysisConditionNodesRow, AnalysisConditionsRow, SummaryComponentsRow,
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::{BoundaryReason, Codebook};
use cpg_schema::condition_kernel::{
    ConditionRoot, Diagram, DiagramNode, KernelBoundary, hydrate_catalog,
};
use cpg_schema::id::{Id, recipe};
use datafusion::prelude::SessionContext;

use crate::{CoreError, sql};
use lctx_analytics::summaries::finite::{
    FiniteSummaryInputs, FiniteSummaryOutcome, LocalCallArgument, LocalCallSummaryFlowSeed,
    LocalCallValueLink, ModeledAssignmentSummaryFlowSeed, ModeledChainArgument,
    ModeledSummaryFlowSeed, ReturnPassStep, SummaryBoundaryCandidate, SummaryFlowSeed,
};

cpg_schema::relations! {
    inventory relations;
    admitted_source_call_bindings = "summary_source_call_bindings", deps = ["source_call_bindings"], sql = "SELECT * FROM source_call_bindings".to_owned();
    source_call_normals = "summary_source_call_normals", deps = ["source_call_normals"], sql = "SELECT * FROM source_call_normals".to_owned();
    source_call_targets = "summary_source_call_targets", deps = ["call_targets"], sql = "SELECT * FROM call_targets".to_owned();
    source_call_resolutions = "summary_source_call_resolutions", deps = ["resolutions"], sql = "SELECT * FROM resolutions".to_owned();
    source_call_functions = "summary_source_call_functions", deps = ["pysa_functions"], sql = "SELECT * FROM pysa_functions".to_owned();
    source_call_node_map = "summary_source_call_node_map", deps = ["provider_node_map"], sql = "SELECT * FROM provider_node_map".to_owned();
    source_call_parameters = "summary_source_call_parameters", deps = ["parameter_semantics"], sql = "SELECT * FROM parameter_semantics".to_owned();
    model_frames = "summary_model_frames", deps = ["model_frame_exits"], sql = "SELECT * FROM model_frame_exits".to_owned();
    model_frame_arguments = "summary_model_frame_arguments", deps = ["model_frame_exit_arguments"], sql = "SELECT * FROM model_frame_exit_arguments".to_owned();
    action_bindings = "action_bindings", deps = ["model_argument_bindings"], sql = "SELECT * FROM model_argument_bindings".to_owned();
    action_candidates_effect = "action_candidates_effect", deps = ["modeled_effect_sites"], sql = "SELECT * FROM modeled_effect_sites".to_owned();
    action_candidates_callback = "action_candidates_callback", deps = ["modeled_callback_sites"], sql = "SELECT * FROM modeled_callback_sites".to_owned();
    action_candidates_resource = "action_candidates_resource", deps = ["modeled_resource_sites"], sql = "SELECT * FROM modeled_resource_sites".to_owned();
    action_applications = "action_applications", deps = ["model_applications"], sql = "SELECT * FROM model_applications".to_owned();
    action_executions = "action_executions", deps = ["call_executions"], sql = "SELECT * FROM call_executions".to_owned();
    action_execution_steps = "action_execution_steps", deps = ["call_execution_steps"], sql = "SELECT * FROM call_execution_steps".to_owned();

    return_certificates = "summary_return_certificates", deps = ["return_completion_certificates"], sql = "SELECT * FROM return_completion_certificates".to_owned();
    context_protocols = "summary_context_protocols", deps = ["model_context_protocols"], sql = "SELECT * FROM model_context_protocols".to_owned();
    context_sites = "summary_context_sites", deps = ["source_context_sites"], sql = "SELECT * FROM source_context_sites".to_owned();
    context_arguments = "summary_context_arguments", deps = ["source_context_arguments"], sql = "SELECT * FROM source_context_arguments".to_owned();
    context_calls = "summary_context_calls", deps = ["call_syntax"], sql = "SELECT * FROM call_syntax".to_owned();
    context_provider_calls = "summary_context_provider_calls", deps = ["pysa_calls"], sql = "SELECT * FROM pysa_calls".to_owned();
    context_exports = "summary_context_exports", deps = ["export_syntax"], sql = "SELECT * FROM export_syntax".to_owned();
    context_regions = "summary_context_regions", deps = ["flow_regions"], sql = "SELECT * FROM flow_regions".to_owned();
    expression_syntax = "summary_expression_syntax", deps = ["syntax_nodes"],
        sql = "SELECT * FROM syntax_nodes".to_owned();
    expression_reads = "summary_expression_reads", deps = ["syntax_nodes", "declarations", "references", "reference_resolutions",
        "bindings", "scopes", "flow_uses", "flow_reaching", "flow_definitions", "analysis_conditions"],
        sql = cpg_schema::behavior::expression_reads_sql();
    pinned_call_targets = "summary_pinned_call_targets", deps = ["model_applications", "call_syntax", "context_definitions",
        "syntax_nodes", "references", "reference_resolutions", "bindings", "scopes", "declarations", "flow_regions"],
        sql = cpg_schema::behavior::pinned_call_targets_sql();
    expression_call_arguments = "summary_expression_arguments", deps = ["arguments"], sql = "SELECT * FROM arguments".to_owned();
    expression_parameters = "summary_expression_parameters", deps = ["context_parameters"], sql = "SELECT * FROM context_parameters".to_owned();
    expression_transfers = "summary_expression_transfers", deps = ["modeled_transfer_sites"], sql = "SELECT * FROM modeled_transfer_sites".to_owned();
    completion_declarations = "summary_completion_declarations", deps = ["declarations"], sql = "SELECT * FROM declarations".to_owned();
    completion_parameters = "summary_completion_parameters", deps = ["parameter_syntax"], sql = "SELECT * FROM parameter_syntax".to_owned();
    completion_expressions = "summary_completion_expressions", deps = ["expression_evaluations"], sql = "SELECT * FROM expression_evaluations".to_owned();
    completion_expression_steps = "summary_completion_expression_steps", deps = ["expression_evaluation_steps"], sql = "SELECT * FROM expression_evaluation_steps".to_owned();
    completion_outcomes = "summary_completion_outcomes", deps = ["statement_completions", "declarations", "syntax_nodes", "analysis_conditions"],
        sql = format!("SELECT s.* FROM statement_completions s JOIN declarations d \
            ON d.snapshot_id=s.snapshot_id AND d.node_id=s.function_node_id AND d.kind={} \
            WHERE EXISTS (SELECT 1 FROM analysis_conditions premise WHERE premise.snapshot_id=s.snapshot_id \
                AND array_has($entry_condition,premise.condition_id) AND premise.root_id IS NOT NULL \
                AND premise.boundary_reason IS NULL) \
            AND NOT EXISTS (SELECT 1 FROM syntax_nodes n WHERE n.snapshot_id=d.snapshot_id \
                AND n.owner_node_id=d.node_id AND n.kind IN ({},{}))",
            cpg_schema::codebook::DeclarationKind::Function.code(),
            cpg_schema::codebook::SyntaxKind::ExprYield.code(),cpg_schema::codebook::SyntaxKind::ExprYieldFrom.code());
    completion_bindings = "summary_completion_bindings", deps = ["bindings"], sql = "SELECT * FROM bindings".to_owned();
    completion_scopes = "summary_completion_scopes", deps = ["scopes"], sql = "SELECT * FROM scopes".to_owned();
    completion_exits = "summary_completion_exits", deps = ["exit_sites"], sql = "SELECT * FROM exit_sites".to_owned();
    return_entries = "summary_return_entries", deps = ["return_entry_statuses"], sql = "SELECT * FROM return_entry_statuses".to_owned();
    return_entry_steps = "summary_return_entry_steps", deps = ["return_entry_steps"], sql = "SELECT * FROM return_entry_steps".to_owned();
    completion_entry_conditions = "summary_completion_entry_conditions",
        deps = ["value_flow_contributions", "flow_values", "syntax_nodes", "exit_sites", "value_flow_predecessor_candidates"],
        sql = format!("WITH returned AS ( \
            SELECT DISTINCT c.snapshot_id, c.flow_value_fact_id, c.condition_id, e.source_fact_id AS return_site_fact_id \
            FROM value_flow_contributions c JOIN flow_values v ON v.snapshot_id = c.snapshot_id AND v.fact_id = c.flow_value_fact_id \
            JOIN syntax_nodes r ON r.snapshot_id = c.snapshot_id AND r.owner_node_id = c.sink_function_node_id \
              AND r.module_node_id = v.module_node_id AND r.kind = {} \
              AND r.start_byte <= v.sink_start_byte AND r.end_byte >= v.sink_end_byte \
            JOIN exit_sites e ON e.snapshot_id = r.snapshot_id AND e.site_node_id = r.node_id AND e.kind = {} \
            WHERE v.sink = {} ) \
            SELECT snapshot_id, return_site_fact_id, condition_id FROM returned \
            UNION SELECT r.snapshot_id, r.return_site_fact_id, p.predecessor_condition_id AS condition_id \
            FROM returned r JOIN value_flow_predecessor_candidates p ON p.snapshot_id = r.snapshot_id AND p.successor_fact_id = r.flow_value_fact_id",
            cpg_schema::codebook::SyntaxKind::StmtReturn.code(),cpg_schema::codebook::ExitSiteKind::Return.code(),cpg_schema::codebook::FlowSink::Return.code());
    completion_handler_types = "summary_completion_handler_types", deps = ["handler_types"], sql = "SELECT * FROM handler_types".to_owned();
    completion_classes = "summary_completion_classes", deps = ["context_definitions"], sql = "SELECT * FROM context_definitions".to_owned();
    completion_mro = "summary_completion_mro", deps = ["context_class_mro"], sql = "SELECT * FROM context_class_mro".to_owned();
    completion_modules = "summary_completion_modules", deps = ["context_modules"], sql = "SELECT * FROM context_modules".to_owned();
    completion_tests = "summary_completion_tests", deps = ["flow_tests"], sql = "SELECT * FROM flow_tests".to_owned();
    binding_references = "summary_binding_references", deps = ["references"], sql = "SELECT * FROM references".to_owned();
    binding_resolutions = "summary_binding_resolutions", deps = ["reference_resolutions"], sql = "SELECT * FROM reference_resolutions".to_owned();
    identity_values = "summary_identity_values", deps = ["flow_values"], sql = "SELECT * FROM flow_values".to_owned();
    identity_contributions = "summary_identity_contributions", deps = ["value_flow_contributions"], sql = "SELECT * FROM value_flow_contributions".to_owned();
    binding_statements = "summary_binding_statements", deps = ["statement_completions"], sql = "SELECT * FROM statement_completions".to_owned();
    binding_statement_steps = "summary_binding_statement_steps", deps = ["statement_completion_steps"], sql = "SELECT * FROM statement_completion_steps".to_owned();
    provider_conditions = "summary_provider_conditions", deps = ["conditions"],
        sql = "SELECT DISTINCT condition_id, root_id, boundary_reason FROM conditions".to_owned();
    provider_nodes = "summary_provider_nodes", deps = ["condition_nodes"],
        sql = "SELECT DISTINCT node_id, atom, low_id, high_id FROM condition_nodes".to_owned();
    analysis_conditions = "summary_analysis_conditions", deps = ["analysis_conditions"],
        sql = "SELECT * FROM analysis_conditions".to_owned();
    analysis_condition_nodes = "summary_analysis_condition_nodes", deps = ["analysis_condition_nodes"],
        sql = "SELECT * FROM analysis_condition_nodes".to_owned();
    predecessors = "summary_predecessors", deps = ["value_flow_predecessor_candidates"],
        sql = "SELECT * FROM value_flow_predecessor_candidates".to_owned();
    call_functions = "summary_call_functions", deps = ["declarations"],
        sql = format!("SELECT snapshot_id, node_id AS function_node_id FROM declarations \
                       WHERE kind IN ({}, {})",
                      cpg_schema::codebook::DeclarationKind::Function.code(),
                      cpg_schema::codebook::DeclarationKind::AsyncFunction.code());
    call_arcs = "summary_call_arcs", deps = ["call_syntax", "call_targets", "declarations"],
        sql = format!("SELECT DISTINCT c.owner_node_id AS caller_node_id, \
                             t.target_node_id AS callee_node_id \
                       FROM call_targets t JOIN call_syntax c \
                         ON c.node_id = t.call_site_node_id \
                       JOIN declarations caller ON caller.node_id = c.owner_node_id \
                         AND caller.kind IN ({f}, {af}) \
                       JOIN declarations callee ON callee.node_id = t.target_node_id \
                         AND callee.kind IN ({f}, {af}) \
                       WHERE t.argument_node_id IS NULL AND NOT c.in_annotation",
                      f = cpg_schema::codebook::DeclarationKind::Function.code(),
                      af = cpg_schema::codebook::DeclarationKind::AsyncFunction.code());
    published_components = "summary_published_components", deps = ["summary_components"],
        sql = "SELECT * FROM summary_components".to_owned();
}

cpg_schema::query_row! {
    struct EntryCondition {
        snapshot_id: Id,
        return_site_fact_id: Id,
        condition_id: Id,
    }
}

cpg_schema::query_row! {
    struct CallFunction {
        snapshot_id: Id,
        function_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct CallArc {
        caller_node_id: Id,
        callee_node_id: Id,
    }
}

cpg_schema::query_row! {
    struct ProviderCondition {
        condition_id: Id,
        root_id: Option<Id>,
        boundary_reason: Option<String>,
    }
}

cpg_schema::query_row! {
    struct ProviderNode {
        node_id: Id,
        atom: String,
        low_id: Id,
        high_id: Id,
    }
}

type ReturnPassIndex = HashMap<Id, Vec<ReturnPassStep>>;

async fn return_pass_steps(ctx: &SessionContext) -> Result<ReturnPassIndex, CoreError> {
    let rows: Vec<ReturnPassStep> = sql::fetch(
        ctx,
        &cpg_schema::behavior::return_exit_pass_steps(),
        sql::Params::new(),
    )
    .await?;
    let mut by_return: ReturnPassIndex = HashMap::new();
    for row in rows {
        by_return
            .entry(row.return_site_fact_id)
            .or_default()
            .push(row);
    }
    for steps in by_return.values_mut() {
        steps.sort_by_key(|step| step.ordinal);
        if steps
            .iter()
            .enumerate()
            .any(|(ordinal, step)| step.ordinal != ordinal as i64)
        {
            return Err(CoreError::Analysis(
                "non-dense return finalizer proof".to_owned(),
            ));
        }
    }
    Ok(by_return)
}

pub async fn predecessor_compatibility(
    ctx: &SessionContext,
) -> Result<Vec<ValueFlowPredecessorCompatibilityRow>, CoreError> {
    let edges: Vec<ValueFlowPredecessorCandidatesRow> =
        sql::fetch(ctx, &predecessors(), sql::Params::new()).await?;
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    Ok(lctx_analytics::summaries::predecessor_compatibility(
        &edges,
        &diagrams,
        &boundaries,
    ))
}

/// Materialize the deterministic SCC schedule over attributed release-to-release calls.
/// Candidate/open targets are topology only and confer no behavior verdict here.
pub async fn call_components(ctx: &SessionContext) -> Result<Vec<SummaryComponentsRow>, CoreError> {
    let functions: Vec<CallFunction> =
        sql::fetch(ctx, &call_functions(), sql::Params::new()).await?;
    let arcs: Vec<CallArc> = sql::fetch(ctx, &call_arcs(), sql::Params::new()).await?;
    let Some(snapshot_id) = functions.first().map(|row| row.snapshot_id) else {
        return Ok(Vec::new());
    };
    if functions.iter().any(|row| row.snapshot_id != snapshot_id) {
        return Err(CoreError::Analysis(
            "mixed snapshots in call component inputs".to_owned(),
        ));
    }
    let components = lctx_analytics::summaries::call_components(
        &functions
            .iter()
            .map(|row| row.function_node_id)
            .collect::<Vec<_>>(),
        &arcs
            .iter()
            .map(|row| (row.caller_node_id, row.callee_node_id))
            .collect::<Vec<_>>(),
    )
    .map_err(|error| CoreError::Analysis(error.to_string()))?;
    let mut rows = Vec::with_capacity(functions.len());
    for (component_order, component) in components.iter().enumerate() {
        let component_id = recipe::summary_component(&component.members);
        for (member_ordinal, &function_node_id) in component.members.iter().enumerate() {
            rows.push(SummaryComponentsRow {
                snapshot_id,
                component_id,
                function_node_id,
                component_order: component_order as i64,
                member_ordinal: member_ordinal as i64,
                member_count: component.members.len() as i64,
                recursive: component.recursive,
            });
        }
    }
    Ok(rows)
}

/// Acquire typed relations; finite composition itself owns no session or store.
pub async fn finite_flows(ctx: &SessionContext) -> Result<FiniteSummaryOutcome, CoreError> {
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    let pass_steps = return_pass_steps(ctx).await?;
    let entries = sql::fetch(ctx, &return_entries(), sql::Params::new()).await?;
    let entry_steps = sql::fetch(ctx, &return_entry_steps(), sql::Params::new()).await?;
    let components: Vec<SummaryComponentsRow> =
        sql::fetch(ctx, &published_components(), sql::Params::new()).await?;
    let direct_seeds: Vec<SummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let modeled_seeds: Vec<ModeledSummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let chain_arguments: Vec<ModeledChainArgument> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_chain_arguments(),
        sql::Params::new(),
    )
    .await?;
    let evaluations: Vec<cpg_schema::behavior::ModeledArgumentEvaluationsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_argument_evaluations(),
        sql::Params::new(),
    )
    .await?;
    let assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_assignment_summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let local_seeds: Vec<LocalCallSummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::local_call_summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let local_arguments: Vec<LocalCallArgument> = sql::fetch(
        ctx,
        &cpg_schema::behavior::local_call_arguments(),
        sql::Params::new(),
    )
    .await?;
    let local_value_links: Vec<LocalCallValueLink> = sql::fetch(
        ctx,
        &cpg_schema::behavior::local_call_value_links(),
        sql::Params::new(),
    )
    .await?;
    let boundary_candidates: Vec<SummaryBoundaryCandidate> = sql::fetch(
        ctx,
        &cpg_schema::behavior::summary_boundary_candidates(),
        sql::Params::new(),
    )
    .await?;
    let local_bindings = source_call_bindings(ctx, &local_seeds, &local_arguments).await?;
    let (identities, context_identities, modeled_identities) =
        source_value_identities(ctx, &modeled_seeds, &evaluations).await?;
    let context_arguments = sql::fetch(ctx, &context_arguments(), sql::Params::new()).await?;
    let context_sites = sql::fetch(ctx, &context_sites(), sql::Params::new()).await?;
    let return_certificates = sql::fetch(ctx, &return_certificates(), sql::Params::new()).await?;
    let model_frames = sql::fetch(ctx, &model_frames(), sql::Params::new()).await?;
    let source_bindings =
        sql::fetch(ctx, &admitted_source_call_bindings(), sql::Params::new()).await?;
    let source_calls = sql::fetch(ctx, &source_call_normals(), sql::Params::new()).await?;
    let mut result = lctx_analytics::summaries::finite::finite_flows(FiniteSummaryInputs {
        source_bindings,
        source_calls,
        model_frames,
        modeled_identities,
        diagrams,
        boundaries,
        pass_steps,
        entries,
        entry_steps,
        components,
        context_sites,
        return_certificates,
        context_identities,
        context_arguments,
        direct_seeds,
        modeled_seeds,
        chain_arguments,
        evaluations,
        assignment_seeds,
        local_seeds,
        local_arguments,
        local_value_links,
        local_bindings,
        boundary_candidates,
        identities,
    });
    // Require the published premise, not a root available only in raw provider conditions.
    let statements = sql::fetch(
        ctx,
        &completion_outcomes(),
        sql::Params::new().ids("entry_condition", [Diagram::always().id()]),
    )
    .await?;
    result
        .coverage
        .extend(lctx_analytics::completion::coverage(&statements));
    Ok(result)
}

/// Mechanical acquisition for the independent source identity proof.
async fn source_value_identities(
    ctx: &SessionContext,
    seeds: &[ModeledSummaryFlowSeed],
    evaluations: &[cpg_schema::behavior::ModeledArgumentEvaluationsRow],
) -> Result<
    (
        Vec<cpg_schema::parameter_identity::SourceParameterIdentitiesRow>,
        Vec<cpg_schema::context_value::SourceContextValueIdentitiesRow>,
        Vec<cpg_schema::modeled_identity::SourceModeledIdentitiesRow>,
    ),
    CoreError,
> {
    let contributions = sql::fetch(ctx, &identity_contributions(), sql::Params::new()).await?;
    if contributions.is_empty() {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let declarations = sql::fetch(ctx, &completion_declarations(), sql::Params::new()).await?;
    let parameters = sql::fetch(ctx, &completion_parameters(), sql::Params::new()).await?;
    let syntax = sql::fetch(ctx, &expression_syntax(), sql::Params::new()).await?;
    let bindings = sql::fetch(ctx, &completion_bindings(), sql::Params::new()).await?;
    let scopes = sql::fetch(ctx, &completion_scopes(), sql::Params::new()).await?;
    let references = sql::fetch(ctx, &binding_references(), sql::Params::new()).await?;
    let resolutions = sql::fetch(ctx, &binding_resolutions(), sql::Params::new()).await?;
    let values = sql::fetch(ctx, &identity_values(), sql::Params::new()).await?;
    let exits = sql::fetch(ctx, &completion_exits(), sql::Params::new()).await?;
    let sites = sql::fetch(ctx, &context_sites(), sql::Params::new()).await?;
    let arguments = sql::fetch(ctx, &context_arguments(), sql::Params::new()).await?;
    let protocols = sql::fetch(ctx, &context_protocols(), sql::Params::new()).await?;
    let context_identities =
        lctx_analytics::context_value::prove(lctx_analytics::context_value::Inputs {
            declarations: &declarations,
            parameters: &parameters,
            syntax: &syntax,
            bindings: &bindings,
            scopes: &scopes,
            references: &references,
            resolutions: &resolutions,
            values: &values,
            contributions: &contributions,
            exits: &exits,
            sites: &sites,
            arguments: &arguments,
            protocols: &protocols,
        });
    let identities =
        lctx_analytics::parameter_identity::prove(lctx_analytics::parameter_identity::Inputs {
            declarations: &declarations,
            parameters: &parameters,
            syntax: &syntax,
            bindings: &bindings,
            scopes: &scopes,
            references: &references,
            resolutions: &resolutions,
            values: &values,
            contributions: &contributions,
            exits: &exits,
        });
    let calls = sql::fetch(ctx, &context_calls(), sql::Params::new()).await?;
    let call_arguments = sql::fetch(ctx, &expression_call_arguments(), sql::Params::new()).await?;
    let modeled_identities =
        lctx_analytics::modeled_identity::prove(lctx_analytics::modeled_identity::Inputs {
            declarations: &declarations,
            parameters: &parameters,
            syntax: &syntax,
            bindings: &bindings,
            scopes: &scopes,
            references: &references,
            resolutions: &resolutions,
            values: &values,
            exits: &exits,
            calls: &calls,
            arguments: &call_arguments,
            seeds,
            evaluations,
        });
    Ok((identities, context_identities, modeled_identities))
}

async fn source_call_bindings(
    ctx: &SessionContext,
    seeds: &[LocalCallSummaryFlowSeed],
    mappings: &[LocalCallArgument],
) -> Result<Vec<lctx_analytics::call_binding::SourceCallBinding>, CoreError> {
    if seeds.is_empty() {
        return Ok(Vec::new());
    }
    let requests: Vec<_> = seeds
        .iter()
        .map(|s| lctx_analytics::call_binding::Request {
            snapshot: s.snapshot_id,
            caller: s.function_node_id,
            call_node: s.call_site_node_id,
            call_fact: s.call_fact_id,
            callee: s.callee_node_id,
        })
        .collect();
    let syntax = sql::fetch(ctx, &expression_syntax(), sql::Params::new()).await?;
    let declarations = sql::fetch(ctx, &completion_declarations(), sql::Params::new()).await?;
    let parameters = sql::fetch(ctx, &completion_parameters(), sql::Params::new()).await?;
    let arguments = sql::fetch(ctx, &expression_call_arguments(), sql::Params::new()).await?;
    let bindings = sql::fetch(ctx, &completion_bindings(), sql::Params::new()).await?;
    let references = sql::fetch(ctx, &binding_references(), sql::Params::new()).await?;
    let resolutions = sql::fetch(ctx, &binding_resolutions(), sql::Params::new()).await?;
    let statements = sql::fetch(ctx, &binding_statements(), sql::Params::new()).await?;
    let statement_steps = sql::fetch(ctx, &binding_statement_steps(), sql::Params::new()).await?;
    let expressions = sql::fetch(ctx, &completion_expressions(), sql::Params::new()).await?;
    Ok(lctx_analytics::call_binding::bind(
        lctx_analytics::call_binding::Inputs {
            requests: &requests,
            mappings,
            syntax: &syntax,
            declarations: &declarations,
            parameters: &parameters,
            arguments: &arguments,
            bindings: &bindings,
            references: &references,
            resolutions: &resolutions,
            statements: &statements,
            statement_steps: &statement_steps,
            expressions: &expressions,
        },
    ))
}

async fn load_conditions(
    ctx: &SessionContext,
) -> Result<(HashMap<Id, Diagram>, HashMap<Id, BoundaryReason>), CoreError> {
    let roots: Vec<AnalysisConditionsRow> =
        sql::fetch(ctx, &analysis_conditions(), sql::Params::new()).await?;
    let nodes: Vec<AnalysisConditionNodesRow> =
        sql::fetch(ctx, &analysis_condition_nodes(), sql::Params::new()).await?;
    let provider_roots: Vec<ProviderCondition> =
        sql::fetch(ctx, &provider_conditions(), sql::Params::new()).await?;
    let provider_nodes: Vec<ProviderNode> =
        sql::fetch(ctx, &provider_nodes(), sql::Params::new()).await?;
    let roots: Vec<ConditionRoot> = roots
        .into_iter()
        .map(|r| ConditionRoot {
            condition_id: r.condition_id,
            root_id: r.root_id,
            boundary_reason: r.boundary_reason,
        })
        .collect();
    let nodes: Vec<DiagramNode> = nodes
        .into_iter()
        .map(|r| DiagramNode {
            node_id: r.node_id,
            atom: r.atom,
            low: r.low_id,
            high: r.high_id,
        })
        .collect();
    let mut boundaries = HashMap::new();
    let classify = |code: &str| {
        KernelBoundary::from_code(code)
            .map(lctx_analytics::summaries::finite::condition_limit)
            .unwrap_or(BoundaryReason::MissingEvidence)
    };
    for root in &roots {
        if let Some(code) = root.boundary_reason.as_deref() {
            boundaries.insert(root.condition_id, classify(code));
        }
    }
    let mut diagrams = hydrate_catalog(&roots, &nodes).map_err(CoreError::Analysis)?;
    let provider_roots: Vec<ConditionRoot> = provider_roots
        .into_iter()
        .map(|r| ConditionRoot {
            condition_id: r.condition_id,
            root_id: r.root_id,
            boundary_reason: r.boundary_reason,
        })
        .collect();
    let provider_nodes: Vec<DiagramNode> = provider_nodes
        .into_iter()
        .map(|r| DiagramNode {
            node_id: r.node_id,
            atom: r.atom,
            low: r.low_id,
            high: r.high_id,
        })
        .collect();
    for root in &provider_roots {
        if let Some(code) = root.boundary_reason.as_deref() {
            boundaries.insert(root.condition_id, classify(code));
        }
    }
    diagrams
        .extend(hydrate_catalog(&provider_roots, &provider_nodes).map_err(CoreError::Analysis)?);
    Ok((diagrams, boundaries))
}

/// One source acquisition and pure staged preparation owns publication and reconstruction.
/// Persisted expressions, source bodies and source-call certificates never feed this operator.
pub async fn execution(
    ctx: &SessionContext,
) -> Result<lctx_analytics::execution::Outcome, CoreError> {
    let nodes = sql::fetch(ctx, &expression_syntax(), sql::Params::new()).await?;
    let reads = sql::fetch(ctx, &expression_reads(), sql::Params::new()).await?;
    let targets = sql::fetch(ctx, &pinned_call_targets(), sql::Params::new()).await?;
    let call_arguments = sql::fetch(ctx, &expression_call_arguments(), sql::Params::new()).await?;
    let parameters = sql::fetch(ctx, &expression_parameters(), sql::Params::new()).await?;
    let transfers = sql::fetch(ctx, &expression_transfers(), sql::Params::new()).await?;
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    let unconditional_conditions: Vec<_> = diagrams
        .iter()
        .filter(|(_, diagram)| diagram.is_true())
        .map(|(id, _)| *id)
        .collect();
    let references = sql::fetch(ctx, &binding_references(), sql::Params::new()).await?;
    let resolutions = sql::fetch(ctx, &binding_resolutions(), sql::Params::new()).await?;
    let context_protocols = sql::fetch(ctx, &context_protocols(), sql::Params::new()).await?;
    let context_sites = sql::fetch(ctx, &context_sites(), sql::Params::new()).await?;
    let context_arguments = sql::fetch(ctx, &context_arguments(), sql::Params::new()).await?;
    let declarations = sql::fetch(ctx, &completion_declarations(), sql::Params::new()).await?;
    let source_parameters = sql::fetch(ctx, &completion_parameters(), sql::Params::new()).await?;
    let bindings = sql::fetch(ctx, &completion_bindings(), sql::Params::new()).await?;
    let scopes = sql::fetch(ctx, &completion_scopes(), sql::Params::new()).await?;
    let exits = sql::fetch(ctx, &completion_exits(), sql::Params::new()).await?;
    let handler_types = sql::fetch(ctx, &completion_handler_types(), sql::Params::new()).await?;
    let classes = sql::fetch(ctx, &completion_classes(), sql::Params::new()).await?;
    let mro = sql::fetch(ctx, &completion_mro(), sql::Params::new()).await?;
    let modules = sql::fetch(ctx, &completion_modules(), sql::Params::new()).await?;
    let tests = sql::fetch(ctx, &completion_tests(), sql::Params::new()).await?;
    let requests: Vec<EntryCondition> =
        sql::fetch(ctx, &completion_entry_conditions(), sql::Params::new()).await?;
    let entry_conditions: Vec<_> = requests
        .iter()
        .map(|r| (r.snapshot_id, r.return_site_fact_id, r.condition_id))
        .collect();
    let calls = sql::fetch(ctx, &context_calls(), sql::Params::new()).await?;
    let targets_source = sql::fetch(ctx, &source_call_targets(), sql::Params::new()).await?;
    let resolutions_source =
        sql::fetch(ctx, &source_call_resolutions(), sql::Params::new()).await?;
    let providers = sql::fetch(ctx, &context_provider_calls(), sql::Params::new()).await?;
    let functions = sql::fetch(ctx, &source_call_functions(), sql::Params::new()).await?;
    let node_map = sql::fetch(ctx, &source_call_node_map(), sql::Params::new()).await?;
    let parameters_source = sql::fetch(ctx, &source_call_parameters(), sql::Params::new()).await?;
    Ok(lctx_analytics::execution::prepare(
        lctx_analytics::evaluation::EvaluationInputs {
            syntax: &nodes,
            reads: &reads,
            targets: &targets,
            call_arguments: &call_arguments,
            parameters: &parameters,
            transfers: &transfers,
            unconditional_conditions: &unconditional_conditions,
            ..Default::default()
        },
        lctx_analytics::completion::Inputs {
            model_frames: &[],
            source_bindings: &[],
            source_calls: &[],
            references: &references,
            resolutions: &resolutions,
            invocations: &[],
            context_protocols: &context_protocols,
            context_sites: &context_sites,
            context_arguments: &context_arguments,
            declarations: &declarations,
            parameters: &source_parameters,
            syntax: &nodes,
            expressions: &[],
            expression_steps: &[],
            bindings: &bindings,
            scopes: &scopes,
            exits: &exits,
            handler_types: &handler_types,
            classes: &classes,
            mro: &mro,
            modules: &modules,
            tests: &tests,
            diagrams: &diagrams,
            boundaries: &boundaries,
            entry_conditions: &entry_conditions,
        },
        lctx_analytics::source_call::Targets {
            arguments: &call_arguments,
            calls: &calls,
            targets: &targets_source,
            resolutions: &resolutions_source,
            providers: &providers,
            functions: &functions,
            node_map: &node_map,
            parameters: &parameters_source,
        },
    ))
}

/// Source admission and publication reconstruction use the same pure operator. No persisted
/// site or completion result is an input to its own reconstruction.
pub async fn source_contexts(
    ctx: &SessionContext,
) -> Result<lctx_analytics::context_protocol::Outcome, CoreError> {
    let protocols = sql::fetch(ctx, &context_protocols(), sql::Params::new()).await?;
    let syntax = sql::fetch(ctx, &expression_syntax(), sql::Params::new()).await?;
    let calls = sql::fetch(ctx, &context_calls(), sql::Params::new()).await?;
    let provider_calls = sql::fetch(ctx, &context_provider_calls(), sql::Params::new()).await?;
    let arguments = sql::fetch(ctx, &expression_call_arguments(), sql::Params::new()).await?;
    let declarations = sql::fetch(ctx, &completion_declarations(), sql::Params::new()).await?;
    let definitions = sql::fetch(ctx, &completion_classes(), sql::Params::new()).await?;
    let parameters = sql::fetch(ctx, &expression_parameters(), sql::Params::new()).await?;
    let modules = sql::fetch(ctx, &completion_modules(), sql::Params::new()).await?;
    let bindings = sql::fetch(ctx, &completion_bindings(), sql::Params::new()).await?;
    let references = sql::fetch(ctx, &binding_references(), sql::Params::new()).await?;
    let resolutions = sql::fetch(ctx, &binding_resolutions(), sql::Params::new()).await?;
    let scopes = sql::fetch(ctx, &completion_scopes(), sql::Params::new()).await?;
    let exports = sql::fetch(ctx, &context_exports(), sql::Params::new()).await?;
    let regions = sql::fetch(ctx, &context_regions(), sql::Params::new()).await?;
    let (diagrams, _) = load_conditions(ctx).await?;
    let unconditional_conditions: Vec<_> = diagrams
        .iter()
        .filter(|(_, d)| d.is_true())
        .map(|(id, _)| *id)
        .collect();
    Ok(lctx_analytics::context_protocol::admit(
        lctx_analytics::context_protocol::Inputs {
            protocols: &protocols,
            syntax: &syntax,
            calls: &calls,
            provider_calls: &provider_calls,
            arguments: &arguments,
            declarations: &declarations,
            definitions: &definitions,
            parameters: &parameters,
            modules: &modules,
            bindings: &bindings,
            references: &references,
            resolutions: &resolutions,
            scopes: &scopes,
            exports: &exports,
            regions: &regions,
            unconditional_conditions: &unconditional_conditions,
        },
    ))
}

/// Acquire already-owned candidates and independent execution/outcome proofs; no action
/// meaning or timing policy lives in orchestration.
pub async fn action_assessments(
    ctx: &SessionContext,
) -> Result<lctx_analytics::actions::Outcome, CoreError> {
    let source_bindings =
        sql::fetch(ctx, &admitted_source_call_bindings(), sql::Params::new()).await?;
    let source_calls = sql::fetch(ctx, &source_call_normals(), sql::Params::new()).await?;
    let frames = sql::fetch(ctx, &model_frames(), sql::Params::new()).await?;
    let frame_arguments = sql::fetch(ctx, &model_frame_arguments(), sql::Params::new()).await?;
    let bindings = sql::fetch(ctx, &action_bindings(), sql::Params::new()).await?;
    let arguments = sql::fetch(ctx, &expression_call_arguments(), sql::Params::new()).await?;
    let effects = sql::fetch(ctx, &action_candidates_effect(), sql::Params::new()).await?;
    let callbacks = sql::fetch(ctx, &action_candidates_callback(), sql::Params::new()).await?;
    let resources = sql::fetch(ctx, &action_candidates_resource(), sql::Params::new()).await?;
    let applications = sql::fetch(ctx, &action_applications(), sql::Params::new()).await?;
    let executions = sql::fetch(ctx, &action_executions(), sql::Params::new()).await?;
    let execution_steps = sql::fetch(ctx, &action_execution_steps(), sql::Params::new()).await?;
    let expressions = sql::fetch(ctx, &completion_expressions(), sql::Params::new()).await?;
    let expression_steps =
        sql::fetch(ctx, &completion_expression_steps(), sql::Params::new()).await?;
    Ok(lctx_analytics::actions::assess(
        lctx_analytics::actions::Inputs {
            source_bindings: &source_bindings,
            source_calls: &source_calls,
            frames: &frames,
            frame_arguments: &frame_arguments,
            bindings: &bindings,
            arguments: &arguments,
            effects: &effects,
            callbacks: &callbacks,
            resources: &resources,
            applications: &applications,
            executions: &executions,
            execution_steps: &execution_steps,
            expressions: &expressions,
            expression_steps: &expression_steps,
        },
    ))
}
