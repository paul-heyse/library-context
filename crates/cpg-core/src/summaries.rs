//! The first L3 source-path check: hydrate published-form analysis conditions, then ask the
//! bounded BDD kernel whether a cited predecessor edge is propositionally compatible.

use std::collections::HashMap;

use cpg_schema::behavior::{
    AnalysisConditionNodesRow, AnalysisConditionsRow, ValueFlowPredecessorCandidatesRow,
    ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::BoundaryReason;
use cpg_schema::condition_kernel::{ConditionRoot, DiagramNode, KernelBoundary, hydrate_catalog};
use cpg_schema::id::Id;
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
    let roots: Vec<AnalysisConditionsRow> =
        sql::fetch(ctx, &analysis_conditions(), sql::Params::new()).await?;
    let nodes: Vec<AnalysisConditionNodesRow> =
        sql::fetch(ctx, &analysis_condition_nodes(), sql::Params::new()).await?;
    let edges: Vec<ValueFlowPredecessorCandidatesRow> =
        sql::fetch(ctx, &predecessors(), sql::Params::new()).await?;
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
    Ok(lctx_analytics::summaries::predecessor_compatibility(
        &edges, &diagrams, &boundaries,
    ))
}
