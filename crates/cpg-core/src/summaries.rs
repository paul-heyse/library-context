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
    FiniteSummaryInputs, LocalCallSummaryFlowSeed, ModeledAssignmentSummaryFlowSeed,
    ModeledSummaryFlowSeed, PrecedingCallRegion, ReturnPassStep, SummaryFlowSeed,
};

cpg_schema::relations! {
    inventory relations;
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

type PrecedingCallIndex = HashMap<Id, Vec<PrecedingCallRegion>>;

async fn preceding_call_regions(ctx: &SessionContext) -> Result<PrecedingCallIndex, CoreError> {
    let rows: Vec<PrecedingCallRegion> = sql::fetch(
        ctx,
        &cpg_schema::behavior::preceding_call_regions(),
        sql::Params::new(),
    )
    .await?;
    let mut by_function: PrecedingCallIndex = HashMap::new();
    for row in rows {
        by_function
            .entry(row.function_node_id)
            .or_default()
            .push(row);
    }
    for calls in by_function.values_mut() {
        calls.sort_by_key(|call| (call.call_start_byte, call.call_fact_id));
    }
    Ok(by_function)
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
pub async fn finite_flows(
    ctx: &SessionContext,
) -> Result<(Vec<cpg_schema::behavior::SummaryFlowsRow>, Vec<cpg_schema::behavior::SummaryFlowStepsRow>), CoreError> {
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    let pass_steps = return_pass_steps(ctx).await?;
    let preceding_calls = preceding_call_regions(ctx).await?;
    let components: Vec<SummaryComponentsRow> = sql::fetch(ctx, &published_components(), sql::Params::new()).await?;
    let direct_seeds: Vec<SummaryFlowSeed> = sql::fetch(ctx, &cpg_schema::behavior::summary_flow_seeds(), sql::Params::new()).await?;
    let modeled_seeds: Vec<ModeledSummaryFlowSeed> = sql::fetch(ctx, &cpg_schema::behavior::modeled_summary_flow_seeds(), sql::Params::new()).await?;
    let evaluations: Vec<cpg_schema::behavior::ModeledArgumentEvaluationsRow> = sql::fetch(ctx, &cpg_schema::behavior::modeled_argument_evaluations(), sql::Params::new()).await?;
    let assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed> = sql::fetch(ctx, &cpg_schema::behavior::modeled_assignment_summary_flow_seeds(), sql::Params::new()).await?;
    let local_seeds: Vec<LocalCallSummaryFlowSeed> = sql::fetch(ctx, &cpg_schema::behavior::local_call_summary_flow_seeds(), sql::Params::new()).await?;
    Ok(lctx_analytics::summaries::finite::finite_flows(FiniteSummaryInputs {
        diagrams, boundaries, pass_steps, preceding_calls, components,
        direct_seeds, modeled_seeds, evaluations, assignment_seeds, local_seeds,
    }))
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
    let classify = |code: &str| match KernelBoundary::from_code(code) {
        Some(
            KernelBoundary::SourceOverBudget
            | KernelBoundary::AtomLimit
            | KernelBoundary::WorkPreflight
            | KernelBoundary::NodeLimit,
        ) => BoundaryReason::BudgetReached,
        Some(KernelBoundary::TransferUnsupported) => BoundaryReason::OutsideProviderModel,
        Some(KernelBoundary::AtomNameCollision) | None => BoundaryReason::MissingEvidence,
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
