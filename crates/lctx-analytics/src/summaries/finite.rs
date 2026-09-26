//! Pure finite source-to-return summary composition over explicit, typed inputs.
//! DataFusion acquisition and Delta publication belong to `cpg-core`.
use std::collections::{BTreeMap, HashMap, HashSet};
use cpg_schema::behavior::{ModeledArgumentEvaluationsRow, SummaryBoundariesRow, SummaryComponentsRow, SummaryFlowStepsRow, SummaryFlowsRow};
use cpg_schema::codebook::{BoundaryReason, ModeledArgumentEvaluationStatus, SummaryFlowKind, SummaryFlowStepKind, Verdict};
use cpg_schema::condition_kernel::{Diagram, KernelBoundary};
use cpg_schema::id::{Id, recipe};
use cpg_schema::models::{InputPath, OutputPath};

cpg_schema::query_row! {
    pub struct SummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_start_byte: i64,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    pub struct ModeledSummaryFlowSeed {
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
    pub struct ModeledAssignmentSummaryFlowSeed {
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

cpg_schema::query_row! {
    pub struct LocalCallSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        call_site_node_id: Id,
        call_fact_id: Id,
        pysa_fact_id: Id,
        callee_node_id: Id,
        callee_parameter_node_id: Id,
        callee_resolution_fact_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    /// One source origin requiring a summary decision. The origin count protects an unproved
    /// sibling from a positive proof over a different contribution to the same raw fact.
    pub struct SummaryBoundaryCandidate {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        source_flow_fact_id: Id,
        condition_id: Id,
        use_id: Id,
        source_key: String,
        through_call: bool,
        local_through_call: bool,
        upstream_through_call: bool,
        raw_through_call: bool,
        raw_approximated: bool,
        reach_budget: bool,
    }
}

type BoundaryKey = (Id, Id, Id, Id, Id);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRefusal {
    pub snapshot_id: Id,
    pub function_node_id: Id,
    pub parameter_node_id: Id,
    pub source_flow_fact_id: Id,
    pub condition_id: Id,
    pub reason: BoundaryReason,
}

impl SummaryRefusal {
    fn key(&self) -> BoundaryKey {
        (self.snapshot_id, self.function_node_id, self.parameter_node_id,
         self.source_flow_fact_id, self.condition_id)
    }
}

pub struct FiniteSummaryOutcome {
    pub flows: Vec<SummaryFlowsRow>,
    pub steps: Vec<SummaryFlowStepsRow>,
    pub refusals: Vec<SummaryRefusal>,
    pub boundaries: Vec<SummaryBoundariesRow>,
}

fn refuse(
    refusals: &mut Vec<SummaryRefusal>,
    key: BoundaryKey,
    reason: BoundaryReason,
) {
    refusals.push(SummaryRefusal {
        snapshot_id: key.0,
        function_node_id: key.1,
        parameter_node_id: key.2,
        source_flow_fact_id: key.3,
        condition_id: key.4,
        reason,
    });
}

/// One summary-facing classification for a bounded condition operation or stored root.
pub fn condition_limit(reason: KernelBoundary) -> BoundaryReason {
    match reason {
        KernelBoundary::AtomLimit => BoundaryReason::ConditionAtomLimit,
        KernelBoundary::WorkPreflight => BoundaryReason::ConditionWorkLimit,
        KernelBoundary::NodeLimit => BoundaryReason::ConditionNodeLimit,
        KernelBoundary::SourceOverBudget => BoundaryReason::BudgetReached,
        KernelBoundary::TransferUnsupported => BoundaryReason::OutsideProviderModel,
        KernelBoundary::AtomNameCollision => BoundaryReason::MissingEvidence,
    }
}

fn refusal_priority(reason: BoundaryReason) -> u8 {
    match reason {
        BoundaryReason::SummaryDepthLimit => 0,
        BoundaryReason::ConditionNodeLimit => 1,
        BoundaryReason::ConditionWorkLimit => 2,
        BoundaryReason::ConditionAtomLimit => 3,
        BoundaryReason::BudgetReached => 4,
        BoundaryReason::OutsideProviderModel => 5,
        BoundaryReason::MissingEvidence => 6,
        _ => 7,
    }
}

/// Complete the positive/refusal decision for every parameter-origin return contribution.
/// A specific producer refusal replaces the generic control/call fallback only for a unique
/// origin; a second origin with the same raw-fact key must stay open independently.
fn summarize_boundaries(
    candidates: &[SummaryBoundaryCandidate],
    flows: &[SummaryFlowsRow],
    refusals: &[SummaryRefusal],
) -> Vec<SummaryBoundariesRow> {
    let proved: HashSet<BoundaryKey> = flows.iter()
        .filter(|flow| flow.verdict != Verdict::Unknown)
        .map(|flow| (
        flow.snapshot_id, flow.function_node_id, flow.parameter_node_id,
        flow.source_flow_fact_id, flow.condition_id,
    )).collect();
    let mut refused: HashMap<BoundaryKey, Vec<BoundaryReason>> = HashMap::new();
    for refusal in refusals {
        refused.entry(refusal.key()).or_default().push(refusal.reason);
    }
    let mut grouped: BTreeMap<BoundaryKey, Vec<&SummaryBoundaryCandidate>> = BTreeMap::new();
    for candidate in candidates {
        let key = (candidate.snapshot_id, candidate.function_node_id,
            candidate.parameter_node_id, candidate.source_flow_fact_id,
            candidate.condition_id);
        grouped.entry(key).or_default().push(candidate);
    }
    let mut rows = Vec::new();
    for (key, origins) in grouped {
        if origins.len() == 1 && proved.contains(&key) && !refused.contains_key(&key) {
            continue;
        }
        let crossed_call = origins.iter().any(|o| o.through_call || o.local_through_call
            || o.upstream_through_call || o.raw_through_call);
        let fallback = if origins.iter().any(|o| o.reach_budget) {
            BoundaryReason::BudgetReached
        } else if crossed_call { BoundaryReason::CallTransfer }
            else { BoundaryReason::UnsupportedControlFlow };
        let reason = if origins.iter().any(|o| o.reach_budget) {
            BoundaryReason::BudgetReached
        } else if origins.len() == 1 {
            refused.get(&key).and_then(|reasons| reasons.iter()
                .filter(|reason| !matches!(reason,
                    BoundaryReason::MissingEvidence | BoundaryReason::CallTransfer))
                .min_by_key(|reason| (refusal_priority(**reason), **reason))
                .copied()).unwrap_or(fallback)
        } else {
            fallback
        };
        rows.push(SummaryBoundariesRow {
            snapshot_id: key.0,
            function_node_id: key.1,
            parameter_node_id: key.2,
            source_flow_fact_id: key.3,
            condition_id: key.4,
            reason,
            local_through_call: origins.iter().any(|o| o.local_through_call),
            upstream_through_call: origins.iter().any(|o| o.upstream_through_call),
            raw_approximated: origins.iter().any(|o| o.raw_approximated),
        });
    }
    rows
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
    pub struct ReturnPassStep {
        return_site_fact_id: Id,
        pass_fact_id: Id,
        condition_id: Id,
        ordinal: i64,
    }
}

cpg_schema::query_row! {
    pub struct PrecedingCallRegion {
        function_node_id: Id,
        call_fact_id: Id,
        call_start_byte: i64,
        condition_id: Option<Id>,
        approximated: Option<bool>,
    }
}

cpg_schema::query_row! {
    /// A simple argument of a sole pinned normal-return call. The SQL relation requires an
    /// argument witness; the producer checks dense order and the import's true condition.
    pub struct PrecedingNormalCallArgument {
        call_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        callee_resolution_fact_id: Id,
        import_binding_fact_id: Id,
        import_region_fact_id: Id,
        import_condition_id: Id,
        argument_fact_id: Id,
        evaluation_evidence_id: Id,
        ordinal: i64,
        argument_count: i64,
    }
}

type PrecedingCallIndex = HashMap<Id, Vec<PrecedingCallRegion>>;
type NormalPredecessorIndex = HashMap<Id, Vec<PrecedingNormalCallArgument>>;
type ReturnPassIndex = HashMap<Id, Vec<ReturnPassStep>>;

pub struct FiniteSummaryInputs {
    pub diagrams: HashMap<Id, Diagram>,
    pub boundaries: HashMap<Id, BoundaryReason>,
    pub pass_steps: HashMap<Id, Vec<ReturnPassStep>>,
    pub preceding_calls: HashMap<Id, Vec<PrecedingCallRegion>>,
    pub normal_predecessors: Vec<PrecedingNormalCallArgument>,
    pub components: Vec<SummaryComponentsRow>,
    pub direct_seeds: Vec<SummaryFlowSeed>,
    pub modeled_seeds: Vec<ModeledSummaryFlowSeed>,
    pub evaluations: Vec<ModeledArgumentEvaluationsRow>,
    pub assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed>,
    pub local_seeds: Vec<LocalCallSummaryFlowSeed>,
    pub boundary_candidates: Vec<SummaryBoundaryCandidate>,
}

fn preceding_call_steps(
    seed: &SummaryFlowSeed,
    calls: &PrecedingCallIndex,
    normal: &NormalPredecessorIndex,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
) -> Result<Vec<recipe::SummaryFlowProofStep>, BoundaryReason> {
    let mut proof = Vec::new();
    for call in calls.get(&seed.function_node_id).into_iter().flatten()
        .take_while(|call| call.call_start_byte < seed.return_start_byte) {
        if call.approximated != Some(false) {
            return Err(BoundaryReason::UnsupportedControlFlow);
        }
        let Some(call_condition_id) = call.condition_id else {
            return Err(BoundaryReason::MissingEvidence);
        };
        let Some(call_condition) = diagrams.get(&call_condition_id) else {
            return Err(boundaries.get(&call_condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
        };
        let Some(return_condition) = diagrams.get(&seed.condition_id) else {
            return Err(boundaries.get(&seed.condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
        };
        match return_condition.and(call_condition) {
            Ok(both) if both.is_false() => continue,
            Ok(_) => {},
            Err(reason) => return Err(condition_limit(reason)),
        }
        let Some(arguments) = normal.get(&call.call_fact_id) else {
            return Err(BoundaryReason::UnsupportedControlFlow);
        };
        if arguments.is_empty() || usize::try_from(arguments[0].argument_count).ok()
            != Some(arguments.len()) {
            return Err(BoundaryReason::UnsupportedControlFlow);
        }
        if !diagrams.get(&arguments[0].import_condition_id).is_some_and(Diagram::is_true) {
            return Err(boundaries.get(&arguments[0].import_condition_id).copied()
                .unwrap_or(BoundaryReason::UnsupportedControlFlow));
        }
        if arguments.iter().enumerate().any(|(ordinal, argument)|
            argument.ordinal != ordinal as i64 || argument.argument_count != arguments[0].argument_count
            || argument.pysa_fact_id != arguments[0].pysa_fact_id
            || argument.model_id != arguments[0].model_id
            || argument.callee_resolution_fact_id != arguments[0].callee_resolution_fact_id
            || argument.import_binding_fact_id != arguments[0].import_binding_fact_id
            || argument.import_region_fact_id != arguments[0].import_region_fact_id
            || argument.import_condition_id != arguments[0].import_condition_id) {
            return Err(BoundaryReason::UnsupportedControlFlow);
        }
        for (kind, evidence_id) in [
            (SummaryFlowStepKind::ModuleImportBinding, arguments[0].import_binding_fact_id),
            (SummaryFlowStepKind::ModuleImportRegion, arguments[0].import_region_fact_id),
        ] {
            proof.push(recipe::SummaryFlowProofStep {
                kind, evidence_id, condition_id: arguments[0].import_condition_id,
            });
        }
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeResolution,
            evidence_id: arguments[0].callee_resolution_fact_id,
            condition_id: call_condition_id,
        });
        for argument in arguments {
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::ArgumentEvaluation,
                evidence_id: argument.evaluation_evidence_id,
                condition_id: call_condition_id,
            });
        }
        for (kind, evidence_id) in [
            (SummaryFlowStepKind::CallSite, call.call_fact_id),
            (SummaryFlowStepKind::CallTarget, arguments[0].pysa_fact_id),
            (SummaryFlowStepKind::PrecedingCallNormal, arguments[0].model_id),
        ] {
            proof.push(recipe::SummaryFlowProofStep { kind, evidence_id, condition_id: call_condition_id });
        }
    }
    Ok(proof)
}

/// The first finite summary case: a direct, synchronous body return of a local parameter.
/// A bounded/missing condition is a named unknown, never an admitted positive flow.
fn direct_flows(
    seeds: Vec<SummaryFlowSeed>,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
    pass_steps: &ReturnPassIndex,
    preceding_calls: &PrecedingCallIndex,
    normal_predecessors: &NormalPredecessorIndex,
    refusals: &mut Vec<SummaryRefusal>,
) -> (Vec<SummaryFlowsRow>, Vec<SummaryFlowStepsRow>) {
    let mut flows = Vec::new();
    let mut steps = Vec::new();
    for seed in seeds {
        let Some(return_diagram) = diagrams.get(&seed.condition_id) else {
            refuse(refusals, (seed.snapshot_id, seed.function_node_id,
                seed.parameter_node_id, seed.source_flow_fact_id, seed.condition_id),
                boundaries.get(&seed.condition_id).copied()
                    .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        let mut proof = match preceding_call_steps(&seed, preceding_calls,
            normal_predecessors, diagrams, boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(refusals, (seed.snapshot_id, seed.function_node_id,
                    seed.parameter_node_id, seed.source_flow_fact_id, seed.condition_id), reason);
                continue;
            }
        };
        let (verdict, boundary_reason) = if return_diagram.is_false() {
            continue;
        } else if return_diagram.is_true() {
            (Verdict::Established, None)
        } else {
            (Verdict::Conditional, None)
        };
        let input_path = InputPath::Parameter {
            name: seed.parameter_name,
        }
        .render();
        let output_path = OutputPath::ReturnValue.render();
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::RawIdentity,
            evidence_id: seed.source_flow_fact_id,
            condition_id: seed.condition_id,
        });
        let finalizers = pass_steps
            .get(&seed.return_site_fact_id)
            .map_or(&[][..], Vec::as_slice);
        for finalizer in finalizers {
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::FinalizerPass,
                evidence_id: finalizer.pass_fact_id,
                condition_id: finalizer.condition_id,
            });
        }
        let summary_id = recipe::summary_flow(&recipe::SummaryFlowIdentity {
            function: seed.function_node_id,
            parameter: seed.parameter_node_id,
            input_path: &input_path,
            output_path: &output_path,
            transfer_kind: SummaryFlowKind::Value,
            condition: seed.condition_id,
            return_site: seed.return_site_fact_id,
            return_region: seed.return_region_fact_id,
            steps: &proof,
        });
        flows.push(SummaryFlowsRow {
            snapshot_id: seed.snapshot_id,
            summary_id,
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
        });
        steps.extend(
            proof
                .into_iter()
                .enumerate()
                .map(|(ordinal, step)| SummaryFlowStepsRow {
                    snapshot_id: seed.snapshot_id,
                    summary_id,
                    ordinal: ordinal as i64,
                    kind: step.kind,
                    evidence_id: step.evidence_id,
                    condition_id: step.condition_id,
                }),
        );
    }
    (flows, steps)
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
    }) || ordered
        .iter()
        .filter(|argument| {
            argument.status == ModeledArgumentEvaluationStatus::SourceOperand
                && argument.argument_fact_id == seed.source_argument_fact_id
        })
        .count()
        != 1
    {
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
    pass_steps: &ReturnPassIndex,
    path: FinitePath,
) {
    let input_path = InputPath::Parameter {
        name: path.parameter_name,
    }
    .render();
    let output_path = OutputPath::ReturnValue.render();
    let mut proof = path.proof;
    let finalizers = pass_steps
        .get(&path.return_site_fact_id)
        .map_or(&[][..], Vec::as_slice);
    if !finalizers.is_empty() {
        let index = proof
            .iter()
            .position(|step| step.kind == SummaryFlowStepKind::ReturnExit)
            .unwrap_or(proof.len());
        for (offset, finalizer) in finalizers.iter().enumerate() {
            proof.insert(
                index + offset,
                recipe::SummaryFlowProofStep {
                    kind: SummaryFlowStepKind::FinalizerPass,
                    evidence_id: finalizer.pass_fact_id,
                    condition_id: finalizer.condition_id,
                },
            );
        }
    }
    let summary_id = recipe::summary_flow(&recipe::SummaryFlowIdentity {
        function: path.function_node_id,
        parameter: path.parameter_node_id,
        input_path: &input_path,
        output_path: &output_path,
        transfer_kind: SummaryFlowKind::Value,
        condition: path.condition_id,
        return_site: path.return_site_fact_id,
        return_region: path.return_region_fact_id,
        steps: &proof,
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
        verdict: if path.condition_is_true {
            Verdict::Established
        } else {
            Verdict::Conditional
        },
        boundary_reason: None,
        source_flow_fact_id: path.source_flow_fact_id,
        return_site_fact_id: path.return_site_fact_id,
        return_region_fact_id: path.return_region_fact_id,
        approximated: path.approximated,
        path_depth: path.path_depth,
    });
    steps.extend(
        proof
            .into_iter()
            .enumerate()
            .map(|(ordinal, step)| SummaryFlowStepsRow {
                snapshot_id: path.snapshot_id,
                summary_id,
                ordinal: ordinal as i64,
                kind: step.kind,
                evidence_id: step.evidence_id,
                condition_id: step.condition_id,
            }),
    );
}

/// Reconstruct all admitted finite paths and their ordered proof steps from published inputs.
/// The same producer is called at write time and by the shared publication validator.
pub fn finite_flows(inputs: FiniteSummaryInputs) -> FiniteSummaryOutcome {
    let FiniteSummaryInputs {
        diagrams, boundaries, pass_steps, preceding_calls, normal_predecessors, components,
        direct_seeds, modeled_seeds, evaluations, assignment_seeds, local_seeds,
        boundary_candidates,
    } = inputs;
    let mut refusals = Vec::new();
    let recursive_functions: HashSet<Id> = components
        .iter()
        .filter(|row| row.recursive)
        .map(|row| row.function_node_id)
        .collect();
    let mut normal_by_call: NormalPredecessorIndex = HashMap::new();
    for row in normal_predecessors {
        normal_by_call.entry(row.call_fact_id).or_default().push(row);
    }
    for rows in normal_by_call.values_mut() { rows.sort_by_key(|row| row.ordinal); }
    let (mut flows, mut steps) =
        direct_flows(direct_seeds, &diagrams, &boundaries, &pass_steps, &preceding_calls,
            &normal_by_call,
            &mut refusals);
    let seeds = modeled_seeds;
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
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.condition_id);
        if recursive_functions.contains(&seed.function_node_id) {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        }
        let Some(condition) = diagrams.get(&seed.condition_id) else {
            refuse(&mut refusals, key, boundaries.get(&seed.condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        let Some(return_condition) = diagrams.get(&seed.return_condition_id) else {
            refuse(&mut refusals, key, boundaries.get(&seed.return_condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        if condition.is_false() {
            continue;
        }
        match condition.implies(return_condition) {
            Ok(true) => {},
            Ok(false) => {
                refuse(&mut refusals, key, BoundaryReason::UnsupportedControlFlow);
                continue;
            },
            Err(limit) => {
                refuse(&mut refusals, key, condition_limit(limit));
                continue;
            },
        }
        let Some(mut proof) = modeled_call_proof(
            &CallEvidence {
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
            },
            &by_candidate,
        ) else {
            refuse(&mut refusals, key, BoundaryReason::MissingEvidence);
            continue;
        };
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ReturnExit,
            evidence_id: seed.return_site_fact_id,
            condition_id: seed.return_condition_id,
        });
        push_finite_path(
            &mut flows,
            &mut steps,
            &pass_steps,
            FinitePath {
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
            },
        );
    }
    for seed in assignment_seeds {
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.successor_condition_id);
        if recursive_functions.contains(&seed.function_node_id) {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        }
        let Some(condition) = diagrams.get(&seed.predecessor_condition_id) else {
            refuse(&mut refusals, key, boundaries.get(&seed.predecessor_condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        if condition.is_false() {
            continue;
        }
        let mut denied = None;
        for id in [seed.reaching_condition_id, seed.successor_condition_id,
            seed.return_condition_id] {
            denied = match diagrams.get(&id) {
                Some(other) => match condition.implies(other) {
                    Ok(true) => None,
                    Ok(false) => Some(BoundaryReason::UnsupportedControlFlow),
                    Err(limit) => Some(condition_limit(limit)),
                },
                None => Some(boundaries.get(&id).copied()
                    .unwrap_or(BoundaryReason::MissingEvidence)),
            };
            if denied.is_some() { break; }
        }
        if let Some(reason) = denied {
            refuse(&mut refusals, key, reason);
            continue;
        }
        let Some(mut proof) = modeled_call_proof(
            &CallEvidence {
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
            },
            &by_candidate,
        ) else {
            refuse(&mut refusals, key, BoundaryReason::MissingEvidence);
            continue;
        };
        for (kind, evidence_id, condition_id) in [
            (
                SummaryFlowStepKind::DefinitionReaching,
                seed.reaching_fact_id,
                seed.reaching_condition_id,
            ),
            (
                SummaryFlowStepKind::ReturnSource,
                seed.source_flow_fact_id,
                seed.successor_condition_id,
            ),
            (
                SummaryFlowStepKind::ReturnExit,
                seed.return_site_fact_id,
                seed.return_condition_id,
            ),
        ] {
            proof.push(recipe::SummaryFlowProofStep {
                kind,
                evidence_id,
                condition_id,
            });
        }
        push_finite_path(
            &mut flows,
            &mut steps,
            &pass_steps,
            FinitePath {
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
            },
        );
    }
    // The first interprocedural case needs no cross-scope atom substitution: the callee's
    // admitted path is unconditional. Acyclic components run after their callees, while
    // recursive components retain their existing unknown boundary until the bounded worklist.
    let component_by_function: HashMap<Id, (i64, bool)> = components
        .into_iter()
        .map(|row| (row.function_node_id, (row.component_order, row.recursive)))
        .collect();
    let mut local_seeds = local_seeds;
    local_seeds.sort_by_key(|seed| {
        (
            component_by_function
                .get(&seed.function_node_id)
                .map_or(i64::MAX, |c| c.0),
            seed.function_node_id,
            seed.call_site_node_id,
            seed.source_flow_fact_id,
            seed.pysa_fact_id,
        )
    });
    let mut by_formal: HashMap<(Id, Id), Vec<SummaryFlowsRow>> = HashMap::new();
    for flow in &flows {
        by_formal
            .entry((flow.function_node_id, flow.parameter_node_id))
            .or_default()
            .push(flow.clone());
    }
    const MAX_LOCAL_PATH_DEPTH: i64 = 8;
    for seed in local_seeds {
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.condition_id);
        let Some((_, false)) = component_by_function.get(&seed.function_node_id) else {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        };
        let Some(condition) = diagrams.get(&seed.condition_id) else {
            refuse(&mut refusals, key, boundaries.get(&seed.condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        let Some(exit_condition) = diagrams.get(&seed.return_condition_id) else {
            refuse(&mut refusals, key, boundaries.get(&seed.return_condition_id).copied()
                .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        if condition.is_false() {
            continue;
        }
        match condition.implies(exit_condition) {
            Ok(true) => {},
            Ok(false) => {
                refuse(&mut refusals, key, BoundaryReason::UnsupportedControlFlow);
                continue;
            },
            Err(limit) => {
                refuse(&mut refusals, key, condition_limit(limit));
                continue;
            },
        }
        let Some(callee_paths) = by_formal
            .get(&(seed.callee_node_id, seed.callee_parameter_node_id))
            .cloned()
        else {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        };
        let mut admitted = false;
        let mut depth_limited = false;
        for callee in callee_paths {
            if callee.path_depth >= MAX_LOCAL_PATH_DEPTH {
                depth_limited = true;
                continue;
            }
            if callee.verdict != Verdict::Established
                || callee.boundary_reason.is_some()
                || callee.kind != SummaryFlowKind::Value
                || callee.output_path != OutputPath::ReturnValue.render()
                || !diagrams
                    .get(&callee.condition_id)
                    .is_some_and(Diagram::is_true)
            {
                continue;
            }
            admitted = true;
            let proof = [
                (
                    SummaryFlowStepKind::CalleeResolution,
                    seed.callee_resolution_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::ArgumentEvaluation,
                    seed.source_flow_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CallSite,
                    seed.call_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CallTarget,
                    seed.pysa_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CalleeSummary,
                    callee.summary_id,
                    callee.condition_id,
                ),
                (
                    SummaryFlowStepKind::ReturnExit,
                    seed.return_site_fact_id,
                    seed.return_condition_id,
                ),
            ]
            .into_iter()
            .map(
                |(kind, evidence_id, condition_id)| recipe::SummaryFlowProofStep {
                    kind,
                    evidence_id,
                    condition_id,
                },
            )
            .collect();
            push_finite_path(
                &mut flows,
                &mut steps,
                &pass_steps,
                FinitePath {
                    snapshot_id: seed.snapshot_id,
                    function_node_id: seed.function_node_id,
                    parameter_node_id: seed.parameter_node_id,
                    parameter_name: seed.parameter_name.clone(),
                    source_flow_fact_id: seed.source_flow_fact_id,
                    condition_id: seed.condition_id,
                    condition_is_true: condition.is_true(),
                    return_site_fact_id: seed.return_site_fact_id,
                    return_region_fact_id: seed.return_region_fact_id,
                    approximated: seed.approximated || callee.approximated,
                    path_depth: callee.path_depth + 1,
                    proof,
                },
            );
            if let Some(flow) = flows.last() {
                by_formal
                    .entry((flow.function_node_id, flow.parameter_node_id))
                    .or_default()
                    .push(flow.clone());
            }
        }
        if depth_limited {
            refuse(&mut refusals, key, BoundaryReason::SummaryDepthLimit);
        } else if !admitted {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
        }
    }
    let boundaries = summarize_boundaries(&boundary_candidates, &flows, &refusals);
    FiniteSummaryOutcome { flows, steps, refusals, boundaries }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> Id {
        Id([byte; 16])
    }

    fn direct_seed() -> SummaryFlowSeed {
        SummaryFlowSeed {
            snapshot_id: id(1),
            function_node_id: id(2),
            parameter_node_id: id(3),
            parameter_name: "value".to_owned(),
            source_flow_fact_id: id(4),
            condition_id: Diagram::always().id(),
            return_site_fact_id: id(5),
            return_region_fact_id: id(6),
            return_start_byte: 30,
            approximated: false,
        }
    }

    fn inputs() -> FiniteSummaryInputs {
        let always = Diagram::always();
        FiniteSummaryInputs {
            diagrams: HashMap::from([(always.id(), always)]),
            boundaries: HashMap::new(),
            pass_steps: HashMap::new(),
            preceding_calls: HashMap::new(),
            normal_predecessors: Vec::new(),
            components: Vec::new(),
            direct_seeds: vec![direct_seed()],
            modeled_seeds: Vec::new(),
            evaluations: Vec::new(),
            assignment_seeds: Vec::new(),
            local_seeds: Vec::new(),
            boundary_candidates: vec![SummaryBoundaryCandidate {
                snapshot_id: id(1),
                function_node_id: id(2),
                parameter_node_id: id(3),
                source_flow_fact_id: id(4),
                condition_id: Diagram::always().id(),
                use_id: id(8),
                source_key: "Parameter[value]".to_owned(),
                through_call: false,
                local_through_call: false,
                upstream_through_call: false,
                raw_through_call: false,
                raw_approximated: false,
                reach_budget: false,
            }],
        }
    }

    fn split_equalities(bits_per_half: usize) -> (Diagram, Diagram) {
        use cpg_schema::condition::Atom;

        let mut halves = [Diagram::always(), Diagram::always()];
        for bit in 0..bits_per_half * 2 {
            let a = Diagram::from_atom(&Atom::Truthy { place: format!("a{bit:02}") }).unwrap();
            let b = Diagram::from_atom(&Atom::Truthy { place: format!("b{bit:02}") }).unwrap();
            let both = a.and(&b).unwrap();
            let neither = a.not().unwrap().and(&b.not().unwrap()).unwrap();
            let equal = both.or(&neither).unwrap();
            let half = usize::from(bit >= bits_per_half);
            halves[half] = halves[half].and(&equal).unwrap();
        }
        let [left, right] = halves;
        (left, right)
    }

    fn direct_with_bounded_predecessor(return_condition: Diagram, call_condition: Diagram)
        -> FiniteSummaryOutcome
    {
        let mut input = inputs();
        let return_id = return_condition.id();
        let call_id = call_condition.id();
        input.diagrams.insert(return_id, return_condition);
        input.diagrams.insert(call_id, call_condition);
        input.direct_seeds[0].condition_id = return_id;
        input.boundary_candidates[0].condition_id = return_id;
        input.preceding_calls.insert(id(2), vec![PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: Some(call_id), approximated: Some(false),
        }]);
        finite_flows(input)
    }

    #[test]
    fn actual_predecessor_work_and_node_caps_keep_specific_unknown_causes() {
        for (bits, expected) in [
            (8, BoundaryReason::ConditionNodeLimit),
            (10, BoundaryReason::ConditionWorkLimit),
        ] {
            let (return_condition, call_condition) = split_equalities(bits);
            let outcome = direct_with_bounded_predecessor(return_condition, call_condition);
            assert!(outcome.flows.is_empty());
            assert_eq!(outcome.boundaries.len(), 1);
            assert_eq!(outcome.boundaries[0].reason, expected);
            assert_eq!(outcome.refusals[0].reason, expected);
        }
    }

    #[test]
    fn direct_admission_and_independent_preceding_call_control_use_pure_inputs() {
        let outcome = finite_flows(inputs());
        assert_eq!(outcome.flows.len(), 1);
        assert_eq!(outcome.flows[0].verdict, Verdict::Established);
        assert_eq!(outcome.steps.len(), 1);
        assert_eq!(outcome.steps[0].kind, SummaryFlowStepKind::RawIdentity);
        assert!(outcome.boundaries.is_empty());

        let mut withheld = inputs();
        withheld.preceding_calls.insert(
            id(2),
            vec![PrecedingCallRegion {
                function_node_id: id(2),
                call_fact_id: id(7),
                call_start_byte: 10,
                condition_id: None,
                approximated: None,
            }],
        );
        let outcome = finite_flows(withheld);
        assert!(outcome.flows.is_empty());
        assert!(outcome.steps.is_empty());
        assert_eq!(outcome.refusals[0].reason, BoundaryReason::UnsupportedControlFlow);
        assert_eq!(outcome.boundaries[0].reason, BoundaryReason::UnsupportedControlFlow);
    }

    #[test]
    fn cited_normal_predecessor_precedes_the_direct_return_proof() {
        let mut input = inputs();
        input.preceding_calls.insert(id(2), vec![PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: Some(Diagram::always().id()), approximated: Some(false),
        }]);
        for (ordinal, evidence) in [(0, id(20)), (1, id(21))] {
            input.normal_predecessors.push(PrecedingNormalCallArgument {
                call_fact_id: id(7), pysa_fact_id: id(22), model_id: id(23),
                callee_resolution_fact_id: id(24), argument_fact_id: id(25 + ordinal),
                import_binding_fact_id: id(30), import_region_fact_id: id(31),
                import_condition_id: Diagram::always().id(),
                evaluation_evidence_id: evidence, ordinal: i64::from(ordinal),
                argument_count: 2,
            });
        }
        let result = finite_flows(input);
        assert_eq!(result.flows.len(), 1);
        assert!(result.boundaries.is_empty());
        assert_eq!(result.steps.iter().map(|step| step.kind).collect::<Vec<_>>(), [
            SummaryFlowStepKind::ModuleImportBinding,
            SummaryFlowStepKind::ModuleImportRegion,
            SummaryFlowStepKind::CalleeResolution,
            SummaryFlowStepKind::ArgumentEvaluation,
            SummaryFlowStepKind::ArgumentEvaluation,
            SummaryFlowStepKind::CallSite,
            SummaryFlowStepKind::CallTarget,
            SummaryFlowStepKind::PrecedingCallNormal,
            SummaryFlowStepKind::RawIdentity,
        ]);
        assert_eq!(result.steps[7].evidence_id, id(23));

        let mut incomplete = inputs();
        incomplete.preceding_calls.insert(id(2), vec![PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: Some(Diagram::always().id()), approximated: Some(false),
        }]);
        incomplete.normal_predecessors.push(PrecedingNormalCallArgument {
            call_fact_id: id(7), pysa_fact_id: id(22), model_id: id(23),
            callee_resolution_fact_id: id(24), argument_fact_id: id(25),
            import_binding_fact_id: id(30), import_region_fact_id: id(31),
            import_condition_id: Diagram::always().id(),
            evaluation_evidence_id: id(20), ordinal: 0, argument_count: 2,
        });
        let result = finite_flows(incomplete);
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries[0].reason, BoundaryReason::UnsupportedControlFlow);

        let mut guarded = inputs();
        guarded.preceding_calls.insert(id(2), vec![PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: Some(Diagram::always().id()), approximated: Some(false),
        }]);
        let guard = Diagram::from_atom(&cpg_schema::condition::Atom::Truthy {
            place: "module_import_guard".to_owned(),
        }).unwrap();
        let guard_id = guard.id();
        guarded.diagrams.insert(guard_id, guard);
        for (ordinal, evidence) in [(0, id(20)), (1, id(21))] {
            guarded.normal_predecessors.push(PrecedingNormalCallArgument {
                call_fact_id: id(7), pysa_fact_id: id(22), model_id: id(23),
                callee_resolution_fact_id: id(24), argument_fact_id: id(25 + ordinal),
                import_binding_fact_id: id(30), import_region_fact_id: id(31),
                import_condition_id: guard_id, evaluation_evidence_id: evidence,
                ordinal: i64::from(ordinal), argument_count: 2,
            });
        }
        let result = finite_flows(guarded);
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries[0].reason, BoundaryReason::UnsupportedControlFlow);
    }

    #[test]
    fn local_depth_cap_and_condition_work_refusal_survive_boundary_decision() {
        let mut input = inputs();
        let always = Diagram::always().id();
        for n in 3_u8..=11 {
            input.components.push(SummaryComponentsRow {
                snapshot_id: id(1),
                component_id: id(n),
                function_node_id: id(n),
                component_order: i64::from(n),
                member_ordinal: 0,
                member_count: 1,
                recursive: false,
            });
            input.local_seeds.push(LocalCallSummaryFlowSeed {
                snapshot_id: id(1),
                function_node_id: id(n),
                parameter_node_id: id(n + 30),
                parameter_name: "value".to_owned(),
                source_flow_fact_id: id(n + 60),
                condition_id: always,
                call_site_node_id: id(n + 90),
                call_fact_id: id(n + 100),
                pysa_fact_id: id(n + 110),
                callee_node_id: id(n - 1),
                callee_parameter_node_id: if n == 3 { id(3) } else { id(n + 29) },
                callee_resolution_fact_id: id(n + 120),
                return_site_fact_id: id(n + 130),
                return_region_fact_id: id(n + 140),
                return_condition_id: always,
                approximated: false,
            });
            input.boundary_candidates.push(SummaryBoundaryCandidate {
                snapshot_id: id(1),
                function_node_id: id(n),
                parameter_node_id: id(n + 30),
                source_flow_fact_id: id(n + 60),
                condition_id: always,
                use_id: id(n + 160),
                source_key: "Parameter[value]".to_owned(),
                through_call: true,
                local_through_call: true,
                upstream_through_call: false,
                raw_through_call: true,
                raw_approximated: false,
                reach_budget: false,
            });
        }
        let result = finite_flows(input);
        assert!(result.flows.iter().any(|row| row.function_node_id == id(10)
            && row.path_depth == 8));
        assert!(!result.flows.iter().any(|row| row.function_node_id == id(11)));
        assert_eq!(result.boundaries.iter().find(|row| row.function_node_id == id(11))
            .unwrap().reason, BoundaryReason::SummaryDepthLimit);

        let mut bounded = inputs();
        bounded.direct_seeds.clear();
        bounded.boundary_candidates[0].through_call = true;
        bounded.boundaries.insert(id(200), condition_limit(KernelBoundary::WorkPreflight));
        bounded.local_seeds.push(LocalCallSummaryFlowSeed {
            snapshot_id: id(1), function_node_id: id(2), parameter_node_id: id(3),
            parameter_name: "value".to_owned(), source_flow_fact_id: id(4),
            condition_id: id(200), call_site_node_id: id(10), call_fact_id: id(11),
            pysa_fact_id: id(12), callee_node_id: id(13),
            callee_parameter_node_id: id(14), callee_resolution_fact_id: id(15),
            return_site_fact_id: id(5), return_region_fact_id: id(6),
            return_condition_id: always, approximated: false,
        });
        bounded.boundary_candidates[0].condition_id = id(200);
        bounded.components.push(SummaryComponentsRow {
            snapshot_id: id(1), component_id: id(2), function_node_id: id(2),
            component_order: 0, member_ordinal: 0, member_count: 1, recursive: false,
        });
        let result = finite_flows(bounded);
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionWorkLimit);

        let mut capped_reach = inputs();
        capped_reach.direct_seeds.clear();
        capped_reach.boundary_candidates[0].reach_budget = true;
        let result = finite_flows(capped_reach);
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries[0].reason, BoundaryReason::BudgetReached);
    }

    #[test]
    fn actual_condition_atom_cap_is_a_typed_finite_refusal() {
        use cpg_schema::condition::Atom;

        let mut input = inputs();
        input.direct_seeds.clear();
        let mut source = Diagram::always();
        for number in 0..128 {
            let atom = Diagram::from_atom(&Atom::Truthy {
                place: format!("source_{number:03}"),
            }).unwrap();
            source = source.and(&atom).unwrap();
        }
        let exit = Diagram::from_atom(&Atom::Truthy {
            place: "exit_only".to_owned(),
        }).unwrap();
        let source_id = source.id();
        let exit_id = exit.id();
        input.diagrams.insert(source_id, source);
        input.diagrams.insert(exit_id, exit);
        input.boundary_candidates[0].condition_id = source_id;
        input.boundary_candidates[0].through_call = true;
        input.components.push(SummaryComponentsRow {
            snapshot_id: id(1), component_id: id(2), function_node_id: id(2),
            component_order: 0, member_ordinal: 0, member_count: 1, recursive: false,
        });
        input.local_seeds.push(LocalCallSummaryFlowSeed {
            snapshot_id: id(1), function_node_id: id(2), parameter_node_id: id(3),
            parameter_name: "value".to_owned(), source_flow_fact_id: id(4),
            condition_id: source_id, call_site_node_id: id(10), call_fact_id: id(11),
            pysa_fact_id: id(12), callee_node_id: id(13),
            callee_parameter_node_id: id(14), callee_resolution_fact_id: id(15),
            return_site_fact_id: id(5), return_region_fact_id: id(6),
            return_condition_id: exit_id, approximated: false,
        });

        let result = finite_flows(input);
        assert!(result.flows.is_empty());
        assert!(result.steps.is_empty());
        assert_eq!(result.refusals.len(), 1);
        assert_eq!(result.refusals[0].reason, BoundaryReason::ConditionAtomLimit);
        assert_eq!(result.boundaries.len(), 1);
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionAtomLimit);
    }

    #[test]
    fn direct_return_preserves_stored_and_predecessor_atom_caps() {
        use cpg_schema::condition::Atom;

        let mut stored = inputs();
        stored.direct_seeds[0].condition_id = id(200);
        stored.boundary_candidates[0].condition_id = id(200);
        stored.boundaries.insert(id(200), BoundaryReason::ConditionAtomLimit);
        let result = finite_flows(stored);
        assert!(result.flows.is_empty());
        assert_eq!(result.refusals[0].reason, BoundaryReason::ConditionAtomLimit);
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionAtomLimit);

        let mut composed = inputs();
        let mut return_condition = Diagram::always();
        for number in 0..128 {
            let atom = Diagram::from_atom(&Atom::Truthy {
                place: format!("return_{number:03}"),
            }).unwrap();
            return_condition = return_condition.and(&atom).unwrap();
        }
        let call_condition = Diagram::from_atom(&Atom::Truthy {
            place: "call_only".to_owned(),
        }).unwrap();
        let return_id = return_condition.id();
        let call_id = call_condition.id();
        composed.diagrams.insert(return_id, return_condition);
        composed.diagrams.insert(call_id, call_condition);
        composed.direct_seeds[0].condition_id = return_id;
        composed.boundary_candidates[0].condition_id = return_id;
        composed.preceding_calls.insert(id(2), vec![PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: Some(call_id), approximated: Some(false),
        }]);
        let result = finite_flows(composed);
        assert!(result.flows.is_empty());
        assert_eq!(result.refusals[0].reason, BoundaryReason::ConditionAtomLimit);
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionAtomLimit);
    }
}
