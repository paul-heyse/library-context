//! The first L3 source-path check: hydrate published-form analysis conditions, then ask the
//! bounded BDD kernel whether a cited predecessor edge is propositionally compatible.

use std::collections::HashMap;

use cpg_schema::behavior::{
    AnalysisConditionNodesRow, AnalysisConditionsRow, SummaryFlowStepsRow, SummaryFlowsRow,
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::{BoundaryReason, SummaryFlowKind, SummaryFlowStepKind, Verdict};
use cpg_schema::condition_kernel::{ConditionRoot, Diagram, DiagramNode, KernelBoundary, hydrate_catalog};
use cpg_schema::id::{Id, recipe};
use cpg_schema::models::{InputPath, OutputPath};
use datafusion::prelude::SessionContext;

use crate::{CoreError, sql};

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
}

cpg_schema::query_row! {
    struct SummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        approximated: bool,
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

pub async fn predecessor_compatibility(
    ctx: &SessionContext,
) -> Result<Vec<ValueFlowPredecessorCompatibilityRow>, CoreError> {
    let edges: Vec<ValueFlowPredecessorCandidatesRow> =
        sql::fetch(ctx, &predecessors(), sql::Params::new()).await?;
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    Ok(lctx_analytics::summaries::predecessor_compatibility(
        &edges, &diagrams, &boundaries,
    ))
}

/// The first finite summary case: a direct, synchronous body return of a local parameter.
/// A bounded/missing condition is a named unknown, never an admitted positive flow.
pub async fn direct_flows(ctx: &SessionContext) -> Result<Vec<SummaryFlowsRow>, CoreError> {
    let seeds: Vec<SummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    Ok(seeds
        .into_iter()
        .filter_map(|seed| {
            let (verdict, boundary_reason) = match diagrams.get(&seed.condition_id) {
                Some(diagram) if diagram.is_false() => return None,
                Some(diagram) if diagram.is_true() => (Verdict::Established, None),
                Some(_) => (Verdict::Conditional, None),
                None => (
                    Verdict::Unknown,
                    Some(
                        boundaries
                            .get(&seed.condition_id)
                            .copied()
                            .unwrap_or(BoundaryReason::MissingEvidence),
                    ),
                ),
            };
            let input_path = InputPath::Parameter {
                name: seed.parameter_name,
            }
            .render();
            let output_path = OutputPath::ReturnValue.render();
            Some(SummaryFlowsRow {
                snapshot_id: seed.snapshot_id,
                summary_id: recipe::summary_flow(&recipe::SummaryFlowIdentity {
                    function: seed.function_node_id,
                    parameter: seed.parameter_node_id,
                    input_path: &input_path,
                    output_path: &output_path,
                    transfer_kind: SummaryFlowKind::Value,
                    condition: seed.condition_id,
                    return_site: seed.return_site_fact_id,
                    return_region: seed.return_region_fact_id,
                    steps: &[recipe::SummaryFlowProofStep {
                        kind: SummaryFlowStepKind::RawIdentity,
                        evidence_id: seed.source_flow_fact_id,
                        condition_id: seed.condition_id,
                    }],
                }),
                function_node_id: seed.function_node_id,
                parameter_node_id: seed.parameter_node_id,
                input_path,
                output_path,
                kind: SummaryFlowKind::Value,
                condition_id: seed.condition_id,
                verdict,
                boundary_reason,
                source_flow_fact_id: seed.source_flow_fact_id,
                return_site_fact_id: seed.return_site_fact_id,
                return_region_fact_id: seed.return_region_fact_id,
                approximated: seed.approximated,
                path_depth: 0,
            })
        })
        .collect())
}

/// The initial direct-flow proof has one raw local-identity step. Later producers append
/// typed steps to their own summary paths; the canonical summary id hashes the ordered list.
pub fn direct_flow_steps(flows: &[SummaryFlowsRow]) -> Vec<SummaryFlowStepsRow> {
    flows
        .iter()
        .map(|flow| SummaryFlowStepsRow {
            snapshot_id: flow.snapshot_id,
            summary_id: flow.summary_id,
            ordinal: 0,
            kind: SummaryFlowStepKind::RawIdentity,
            evidence_id: flow.source_flow_fact_id,
            condition_id: flow.condition_id,
        })
        .collect()
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
        Some(KernelBoundary::SourceOverBudget | KernelBoundary::AtomLimit |
             KernelBoundary::WorkPreflight | KernelBoundary::NodeLimit) => BoundaryReason::BudgetReached,
        Some(KernelBoundary::TransferUnsupported) => BoundaryReason::OutsideProviderModel,
        Some(KernelBoundary::AtomNameCollision) | None => BoundaryReason::MissingEvidence,
    };
    for root in &roots {
        if let Some(code) = root.boundary_reason.as_deref() {
            boundaries.insert(root.condition_id, classify(code));
        }
    }
    let mut diagrams = hydrate_catalog(&roots, &nodes).map_err(CoreError::Analysis)?;
    let provider_roots: Vec<ConditionRoot> = provider_roots.into_iter().map(|r| ConditionRoot {
        condition_id: r.condition_id,
        root_id: r.root_id,
        boundary_reason: r.boundary_reason,
    }).collect();
    let provider_nodes: Vec<DiagramNode> = provider_nodes.into_iter().map(|r| DiagramNode {
        node_id: r.node_id,
        atom: r.atom,
        low: r.low_id,
        high: r.high_id,
    }).collect();
    for root in &provider_roots {
        if let Some(code) = root.boundary_reason.as_deref() {
            boundaries.insert(root.condition_id, classify(code));
        }
    }
    diagrams.extend(hydrate_catalog(&provider_roots, &provider_nodes).map_err(CoreError::Analysis)?);
    Ok((diagrams, boundaries))
}
