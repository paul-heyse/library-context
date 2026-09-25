//! The first L3 source-path check: hydrate published-form analysis conditions, then ask the
//! bounded BDD kernel whether a cited predecessor edge is propositionally compatible.

use std::collections::HashMap;

use cpg_schema::behavior::{
    AnalysisConditionNodesRow, AnalysisConditionsRow, ModeledArgumentEvaluationsRow,
    SummaryFlowStepsRow, SummaryFlowsRow,
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::{BoundaryReason, ModeledArgumentEvaluationStatus, SummaryFlowKind, SummaryFlowStepKind, Verdict};
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
    struct ModeledSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        call_fact_id: Id,
        source_argument_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        callee_resolution_fact_id: Id,
        argument_count: i64,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    struct ModeledAssignmentSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        predecessor_flow_fact_id: Id,
        reaching_fact_id: Id,
        predecessor_condition_id: Id,
        reaching_condition_id: Id,
        successor_condition_id: Id,
        source_argument_fact_id: Id,
        call_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        callee_resolution_fact_id: Id,
        argument_count: i64,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

struct CallEvidence {
    flow_fact_id: Id,
    parameter_node_id: Id,
    source_argument_fact_id: Id,
    call_fact_id: Id,
    pysa_fact_id: Id,
    model_id: Id,
    rule_id: Id,
    callee_resolution_fact_id: Id,
    argument_count: i64,
    condition_id: Id,
}

type ArgumentIndex = HashMap<(Id, Id, Id, Id, Id), Vec<ModeledArgumentEvaluationsRow>>;

struct FinitePath {
    snapshot_id: Id,
    function_node_id: Id,
    parameter_node_id: Id,
    parameter_name: String,
    source_flow_fact_id: Id,
    condition_id: Id,
    condition_is_true: bool,
    return_site_fact_id: Id,
    return_region_fact_id: Id,
    approximated: bool,
    path_depth: i64,
    proof: Vec<recipe::SummaryFlowProofStep>,
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
async fn direct_flows(
    ctx: &SessionContext,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
) -> Result<Vec<SummaryFlowsRow>, CoreError> {
    let seeds: Vec<SummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
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

/// A direct-flow proof has one raw local-identity step; modeled paths use a separate typed
/// sequence. The canonical summary id hashes the ordered list.
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

/// One completed candidate call path, shared by direct returns and the first assignment hop.
/// This is a may-path under the source/model abstraction, not universal normal execution.
fn modeled_call_proof(
    seed: &CallEvidence,
    by_candidate: &ArgumentIndex,
) -> Option<Vec<recipe::SummaryFlowProofStep>> {
    let arguments = by_candidate.get(&(
        seed.flow_fact_id,
        seed.parameter_node_id,
        seed.pysa_fact_id,
        seed.model_id,
        seed.rule_id,
    ))?;
    if arguments.len() != usize::try_from(seed.argument_count).ok()? {
        return None;
    }
    let mut ordered = arguments.clone();
    ordered.sort_by_key(|argument| argument.ordinal);
    if ordered.iter().enumerate().any(|(ordinal, argument)| {
        argument.ordinal != ordinal as i64
            || argument.status == ModeledArgumentEvaluationStatus::Unknown
            || argument.evidence_id.is_none()
            || argument.condition_id != seed.condition_id
    }) || ordered.iter().filter(|argument| {
        argument.status == ModeledArgumentEvaluationStatus::SourceOperand
            && argument.argument_fact_id == seed.source_argument_fact_id
    }).count() != 1 {
        return None;
    }
    let mut proof = Vec::with_capacity(ordered.len() + 4);
    proof.push(recipe::SummaryFlowProofStep {
        kind: SummaryFlowStepKind::CalleeResolution,
        evidence_id: seed.callee_resolution_fact_id,
        condition_id: seed.condition_id,
    });
    for argument in &ordered {
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ArgumentEvaluation,
            evidence_id: argument.evidence_id?,
            condition_id: argument.condition_id,
        });
    }
    for (kind, evidence_id) in [
        (SummaryFlowStepKind::CallSite, seed.call_fact_id),
        (SummaryFlowStepKind::CallTarget, seed.pysa_fact_id),
        (SummaryFlowStepKind::ModelRule, seed.rule_id),
    ] {
        proof.push(recipe::SummaryFlowProofStep {
            kind,
            evidence_id,
            condition_id: seed.condition_id,
        });
    }
    Some(proof)
}

fn push_finite_path(
    flows: &mut Vec<SummaryFlowsRow>,
    steps: &mut Vec<SummaryFlowStepsRow>,
    path: FinitePath,
) {
    let input_path = InputPath::Parameter { name: path.parameter_name }.render();
    let output_path = OutputPath::ReturnValue.render();
    let summary_id = recipe::summary_flow(&recipe::SummaryFlowIdentity {
        function: path.function_node_id,
        parameter: path.parameter_node_id,
        input_path: &input_path,
        output_path: &output_path,
        transfer_kind: SummaryFlowKind::Value,
        condition: path.condition_id,
        return_site: path.return_site_fact_id,
        return_region: path.return_region_fact_id,
        steps: &path.proof,
    });
    flows.push(SummaryFlowsRow {
        snapshot_id: path.snapshot_id,
        summary_id,
        function_node_id: path.function_node_id,
        parameter_node_id: path.parameter_node_id,
        input_path,
        output_path,
        kind: SummaryFlowKind::Value,
        condition_id: path.condition_id,
        verdict: if path.condition_is_true { Verdict::Established } else { Verdict::Conditional },
        boundary_reason: None,
        source_flow_fact_id: path.source_flow_fact_id,
        return_site_fact_id: path.return_site_fact_id,
        return_region_fact_id: path.return_region_fact_id,
        approximated: path.approximated,
        path_depth: path.path_depth,
    });
    steps.extend(path.proof.into_iter().enumerate().map(|(ordinal, step)| SummaryFlowStepsRow {
        snapshot_id: path.snapshot_id,
        summary_id,
        ordinal: ordinal as i64,
        kind: step.kind,
        evidence_id: step.evidence_id,
        condition_id: step.condition_id,
    }));
}

/// Reconstruct all admitted finite paths and their ordered proof steps from published inputs.
/// The same producer is called at write time and by the shared publication validator.
pub async fn finite_flows(
    ctx: &SessionContext,
) -> Result<(Vec<SummaryFlowsRow>, Vec<SummaryFlowStepsRow>), CoreError> {
    let (diagrams, boundaries) = load_conditions(ctx).await?;
    let mut flows = direct_flows(ctx, &diagrams, &boundaries).await?;
    let mut steps = direct_flow_steps(&flows);
    let seeds: Vec<ModeledSummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    let evaluations: Vec<ModeledArgumentEvaluationsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_argument_evaluations(),
        sql::Params::new(),
    )
    .await?;
    let mut by_candidate: ArgumentIndex = HashMap::new();
    for evaluation in evaluations {
        by_candidate
            .entry((
                evaluation.candidate_flow_fact_id,
                evaluation.parameter_node_id,
                evaluation.pysa_fact_id,
                evaluation.model_id,
                evaluation.rule_id,
            ))
            .or_default()
            .push(evaluation);
    }
    for seed in seeds {
        let Some(condition) = diagrams.get(&seed.condition_id) else {
            continue;
        };
        let Some(return_condition) = diagrams.get(&seed.return_condition_id) else {
            continue;
        };
        if condition.is_false() || condition.implies(return_condition) != Ok(true) {
            continue;
        }
        let Some(mut proof) = modeled_call_proof(&CallEvidence {
            flow_fact_id: seed.source_flow_fact_id,
            parameter_node_id: seed.parameter_node_id,
            source_argument_fact_id: seed.source_argument_fact_id,
            call_fact_id: seed.call_fact_id,
            pysa_fact_id: seed.pysa_fact_id,
            model_id: seed.model_id,
            rule_id: seed.rule_id,
            callee_resolution_fact_id: seed.callee_resolution_fact_id,
            argument_count: seed.argument_count,
            condition_id: seed.condition_id,
        }, &by_candidate) else {
            continue;
        };
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ReturnExit,
            evidence_id: seed.return_site_fact_id,
            condition_id: seed.return_condition_id,
        });
        push_finite_path(&mut flows, &mut steps, FinitePath {
            snapshot_id: seed.snapshot_id,
            function_node_id: seed.function_node_id,
            parameter_node_id: seed.parameter_node_id,
            parameter_name: seed.parameter_name,
            source_flow_fact_id: seed.source_flow_fact_id,
            condition_id: seed.condition_id,
            condition_is_true: condition.is_true(),
            return_site_fact_id: seed.return_site_fact_id,
            return_region_fact_id: seed.return_region_fact_id,
            approximated: seed.approximated,
            path_depth: 1,
            proof,
        });
    }
    let assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_assignment_summary_flow_seeds(),
        sql::Params::new(),
    )
    .await?;
    for seed in assignment_seeds {
        let Some(condition) = diagrams.get(&seed.predecessor_condition_id) else {
            continue;
        };
        if condition.is_false() || [
            seed.reaching_condition_id,
            seed.successor_condition_id,
            seed.return_condition_id,
        ].iter().any(|id| {
            diagrams.get(id).is_none_or(|other| condition.implies(other) != Ok(true))
        }) {
            continue;
        }
        let Some(mut proof) = modeled_call_proof(&CallEvidence {
            flow_fact_id: seed.predecessor_flow_fact_id,
            parameter_node_id: seed.parameter_node_id,
            source_argument_fact_id: seed.source_argument_fact_id,
            call_fact_id: seed.call_fact_id,
            pysa_fact_id: seed.pysa_fact_id,
            model_id: seed.model_id,
            rule_id: seed.rule_id,
            callee_resolution_fact_id: seed.callee_resolution_fact_id,
            argument_count: seed.argument_count,
            condition_id: seed.predecessor_condition_id,
        }, &by_candidate) else {
            continue;
        };
        for (kind, evidence_id, condition_id) in [
            (SummaryFlowStepKind::DefinitionReaching, seed.reaching_fact_id, seed.reaching_condition_id),
            (SummaryFlowStepKind::ReturnSource, seed.source_flow_fact_id, seed.successor_condition_id),
            (SummaryFlowStepKind::ReturnExit, seed.return_site_fact_id, seed.return_condition_id),
        ] {
            proof.push(recipe::SummaryFlowProofStep { kind, evidence_id, condition_id });
        }
        push_finite_path(&mut flows, &mut steps, FinitePath {
            snapshot_id: seed.snapshot_id,
            function_node_id: seed.function_node_id,
            parameter_node_id: seed.parameter_node_id,
            parameter_name: seed.parameter_name,
            source_flow_fact_id: seed.source_flow_fact_id,
            condition_id: seed.predecessor_condition_id,
            condition_is_true: condition.is_true(),
            return_site_fact_id: seed.return_site_fact_id,
            return_region_fact_id: seed.return_region_fact_id,
            approximated: seed.approximated,
            path_depth: 2,
            proof,
        });
    }
    Ok((flows, steps))
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
