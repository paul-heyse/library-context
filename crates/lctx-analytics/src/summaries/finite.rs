//! Pure finite source-to-return summary composition over explicit, typed inputs.
//! DataFusion acquisition and Delta publication belong to `cpg-core`.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use cpg_schema::behavior::{ModeledArgumentEvaluationsRow, SummaryBoundariesRow, SummaryComponentsRow, SummaryFlowStepsRow, SummaryFlowsRow};
use cpg_schema::codebook::{BoundaryReason, ModeledArgumentEvaluationStatus, SummaryFlowKind, SummaryFlowStepKind, Verdict};
use cpg_schema::condition::Atom;
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
        source_origin_id: Id,
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
        source_origin_id: Id,
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
        return_start_byte: i64,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    pub struct ModeledChainArgument {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        source_origin_id: Id,
        condition_id: Id,
        step: i64,
        raw_step_count: i64,
        call_start_byte: i64,
        call_end_byte: i64,
        operand_start_byte: i64,
        operand_end_byte: i64,
        sink_start_byte: i64,
        sink_end_byte: i64,
        call_site_node_id: Id,
        call_fact_id: Id,
        source_argument_fact_id: Id,
        source_value_start_byte: i64,
        source_value_end_byte: i64,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        callee_resolution_fact_id: Id,
        argument_count: i64,
        argument_ordinal: i64,
        argument_fact_id: Id,
        evaluation_status: ModeledArgumentEvaluationStatus,
        evaluation_evidence_id: Option<Id>,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        return_start_byte: i64,
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
        source_origin_id: Id,
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
        return_start_byte: i64,
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
        source_origin_id: Id,
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
        return_start_byte: i64,
        approximated: bool,
        source_argument_ordinal: i64,
        control_argument_fact_id: Option<Id>,
        control_argument_ordinal: Option<i64>,
        control_evaluation_fact_id: Option<Id>,
        control_formal_node_id: Option<Id>,
        control_link_id: Option<Id>,
        control_atom: Option<String>,
        control_value: Option<bool>,
        control_source_link_id: Option<Id>,
        control_source_atom: Option<String>,
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
        source_origin_id: Id,
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

type BoundaryKey = (Id, Id, Id, Id, Id, Id);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRefusal {
    pub snapshot_id: Id,
    pub function_node_id: Id,
    pub parameter_node_id: Id,
    pub source_flow_fact_id: Id,
    pub source_origin_id: Id,
    pub condition_id: Id,
    pub reason: BoundaryReason,
}

impl SummaryRefusal {
    fn key(&self) -> BoundaryKey {
        (self.snapshot_id, self.function_node_id, self.parameter_node_id,
         self.source_flow_fact_id, self.condition_id, self.source_origin_id)
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
        source_origin_id: key.5,
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
        BoundaryReason::SummaryPairWorkLimit => 1,
        BoundaryReason::ConditionNodeLimit => 2,
        BoundaryReason::ConditionWorkLimit => 3,
        BoundaryReason::ConditionAtomLimit => 4,
        BoundaryReason::BudgetReached => 5,
        BoundaryReason::OutsideProviderModel => 6,
        BoundaryReason::MissingEvidence => 7,
        _ => 7,
    }
}

/// Complete the positive/refusal decision for every parameter-origin return contribution.
/// A proof and refusal belong to one stable source contribution, even when multiple origins
/// share a raw fact and condition.
fn summarize_boundaries(
    candidates: &[SummaryBoundaryCandidate],
    flows: &[SummaryFlowsRow],
    refusals: &[SummaryRefusal],
) -> Vec<SummaryBoundariesRow> {
    let proved: HashSet<BoundaryKey> = flows.iter()
        .filter(|flow| flow.verdict != Verdict::Unknown)
        .map(|flow| (
        flow.snapshot_id, flow.function_node_id, flow.parameter_node_id,
        flow.source_flow_fact_id, flow.condition_id, flow.source_origin_id,
    )).collect();
    let mut refused: HashMap<BoundaryKey, Vec<BoundaryReason>> = HashMap::new();
    for refusal in refusals {
        refused.entry(refusal.key()).or_default().push(refusal.reason);
    }
    let mut grouped: BTreeMap<BoundaryKey, Vec<&SummaryBoundaryCandidate>> = BTreeMap::new();
    for candidate in candidates {
        let key = (candidate.snapshot_id, candidate.function_node_id,
            candidate.parameter_node_id, candidate.source_flow_fact_id,
            candidate.condition_id, candidate.source_origin_id);
        grouped.entry(key).or_default().push(candidate);
    }
    let mut rows = Vec::new();
    for (key, origins) in grouped {
        if proved.contains(&key) && !refused.contains_key(&key) {
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
        } else {
            refused.get(&key).and_then(|reasons| reasons.iter()
                .filter(|reason| !matches!(reason,
                    BoundaryReason::MissingEvidence | BoundaryReason::CallTransfer))
                .min_by_key(|reason| (refusal_priority(**reason), **reason))
                .copied()).unwrap_or(fallback)
        };
        rows.push(SummaryBoundariesRow {
            snapshot_id: key.0,
            function_node_id: key.1,
            parameter_node_id: key.2,
            source_flow_fact_id: key.3,
            condition_id: key.4,
            source_origin_id: key.5,
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
    source_origin_id: Id,
    condition_id: Id,
    condition_is_true: bool,
    return_site_fact_id: Id,
    return_region_fact_id: Id,
    approximated: bool,
    path_depth: i64,
    proof: Vec<recipe::SummaryFlowProofStep>,
}

/// The second argument is a literal or a directly read caller formal with an exact tested
/// value in the caller's path condition. A partial row is never the one-argument case.
#[derive(Clone, Copy)]
struct ControlEvidence<'a> {
    evaluation: Id,
    callee_link: Id,
    callee_atom: &'a str,
    literal: Option<bool>,
    caller_link: Option<Id>,
    caller_atom: Option<&'a str>,
}

fn control_evidence(seed: &LocalCallSummaryFlowSeed)
    -> Result<Option<ControlEvidence<'_>>, BoundaryReason> {
    if !matches!(seed.source_argument_ordinal, 0 | 1)
        || (seed.control_argument_fact_id.is_none() && seed.source_argument_ordinal != 0)
        || seed.control_argument_fact_id.is_some() != seed.control_argument_ordinal.is_some()
        || seed.control_argument_ordinal.is_some_and(|ordinal|
            !matches!(ordinal, 0 | 1) || ordinal == seed.source_argument_ordinal)
    {
        return Err(BoundaryReason::MissingEvidence);
    }
    match (
        seed.control_argument_fact_id, seed.control_evaluation_fact_id,
        seed.control_formal_node_id, seed.control_link_id,
        seed.control_atom.as_deref(), seed.control_value,
        seed.control_source_link_id, seed.control_source_atom.as_deref(),
    ) {
        (None, None, None, None, None, None, None, None) => Ok(None),
        (Some(_), Some(evaluation), Some(_), Some(link), Some(atom), Some(value), None, None)
            if matches!(Atom::parse_encoded(atom), Ok(Atom::Evaluated { atom, .. })
                if matches!(*atom, Atom::Truthy { .. })) => Ok(Some(ControlEvidence {
                    evaluation, callee_link: link, callee_atom: atom,
                    literal: Some(value), caller_link: None, caller_atom: None,
                })),
        (Some(_), Some(evaluation), Some(_), Some(link), Some(atom), None,
         Some(source_link), Some(source_atom))
            if matches!(Atom::parse_encoded(atom), Ok(Atom::Evaluated { atom, .. })
                if matches!(*atom, Atom::Truthy { .. }))
            && matches!(Atom::parse_encoded(source_atom), Ok(Atom::Evaluated { atom, .. })
                if matches!(*atom, Atom::Truthy { .. })) => Ok(Some(ControlEvidence {
                    evaluation, callee_link: link, callee_atom: atom,
                    literal: None, caller_link: Some(source_link),
                    caller_atom: Some(source_atom),
                })),
        _ => Err(BoundaryReason::MissingEvidence),
    }
}

fn fixed_caller_truth(diagram: &Diagram, encoded_atom: &str)
    -> Result<Option<bool>, KernelBoundary> {
    let atom = Atom::parse_encoded(encoded_atom).expect("validated caller control atom");
    let predicate = Diagram::from_atom(&atom)?;
    if diagram.implies(&predicate)? { return Ok(Some(true)); }
    if diagram.implies(&predicate.not()?)? { return Ok(Some(false)); }
    Ok(None)
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

#[derive(Clone)]
pub struct FiniteSummaryInputs {
    pub diagrams: HashMap<Id, Diagram>,
    pub boundaries: HashMap<Id, BoundaryReason>,
    pub pass_steps: HashMap<Id, Vec<ReturnPassStep>>,
    pub preceding_calls: HashMap<Id, Vec<PrecedingCallRegion>>,
    pub normal_predecessors: Vec<PrecedingNormalCallArgument>,
    pub components: Vec<SummaryComponentsRow>,
    pub direct_seeds: Vec<SummaryFlowSeed>,
    pub modeled_seeds: Vec<ModeledSummaryFlowSeed>,
    pub chain_arguments: Vec<ModeledChainArgument>,
    pub evaluations: Vec<ModeledArgumentEvaluationsRow>,
    pub assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed>,
    pub local_seeds: Vec<LocalCallSummaryFlowSeed>,
    pub boundary_candidates: Vec<SummaryBoundaryCandidate>,
}

fn preceding_call_steps(
    function_node_id: Id,
    return_start_byte: i64,
    condition_id: Id,
    calls: &PrecedingCallIndex,
    normal: &NormalPredecessorIndex,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
) -> Result<Vec<recipe::SummaryFlowProofStep>, BoundaryReason> {
    let mut proof = Vec::new();
    for call in calls.get(&function_node_id).into_iter().flatten()
        .take_while(|call| call.call_start_byte < return_start_byte) {
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
        let Some(return_condition) = diagrams.get(&condition_id) else {
            return Err(boundaries.get(&condition_id).copied()
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
                seed.parameter_node_id, seed.source_flow_fact_id, seed.condition_id,
                seed.source_origin_id),
                boundaries.get(&seed.condition_id).copied()
                    .unwrap_or(BoundaryReason::MissingEvidence));
            continue;
        };
        let mut proof = match preceding_call_steps(seed.function_node_id,
            seed.return_start_byte, seed.condition_id, preceding_calls,
            normal_predecessors, diagrams, boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(refusals, (seed.snapshot_id, seed.function_node_id,
                    seed.parameter_node_id, seed.source_flow_fact_id, seed.condition_id,
                    seed.source_origin_id), reason);
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
            source_origin: seed.source_origin_id,
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
            source_origin_id: seed.source_origin_id,
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

const MAX_MODELED_CHAIN_STEPS: usize = 8;
const MAX_MODELED_CHAIN_ARGUMENTS: usize = 128;

/// Compose an exact chain from the returned outer call into its source argument call, ending
/// at the raw formal use. A row missing at any step means no normal-completion proof.
fn modeled_chain_proof(rows: &[ModeledChainArgument])
    -> Result<Vec<recipe::SummaryFlowProofStep>, BoundaryReason>
{
    let first = rows.first().ok_or(BoundaryReason::MissingEvidence)?;
    let depth = usize::try_from(first.raw_step_count)
        .map_err(|_| BoundaryReason::MissingEvidence)?;
    if depth < 2 { return Err(BoundaryReason::MissingEvidence); }
    if depth > MAX_MODELED_CHAIN_STEPS {
        return Err(BoundaryReason::SummaryDepthLimit);
    }
    if rows.len() > MAX_MODELED_CHAIN_ARGUMENTS {
        return Err(BoundaryReason::BudgetReached);
    }
    let mut steps: Vec<Vec<&ModeledChainArgument>> = vec![Vec::new(); depth];
    for row in rows {
        let step = usize::try_from(row.step).map_err(|_| BoundaryReason::MissingEvidence)?;
        if step >= depth || row.raw_step_count != first.raw_step_count
            || row.snapshot_id != first.snapshot_id
            || row.function_node_id != first.function_node_id
            || row.parameter_node_id != first.parameter_node_id
            || row.parameter_name != first.parameter_name
            || row.source_flow_fact_id != first.source_flow_fact_id
            || row.source_origin_id != first.source_origin_id
            || row.condition_id != first.condition_id
            || row.return_site_fact_id != first.return_site_fact_id
            || row.return_region_fact_id != first.return_region_fact_id
            || row.return_condition_id != first.return_condition_id
            || row.return_start_byte != first.return_start_byte
            || row.approximated != first.approximated
        {
            return Err(BoundaryReason::MissingEvidence);
        }
        steps[step].push(row);
    }
    for arguments in &mut steps {
        arguments.sort_by_key(|row| (row.argument_ordinal, row.argument_fact_id));
        let Some(&call) = arguments.first() else {
            return Err(BoundaryReason::CallTransfer);
        };
        if usize::try_from(call.argument_count).ok() != Some(arguments.len())
            || arguments.iter().enumerate().any(|(ordinal, row)| {
                row.argument_ordinal != ordinal as i64
                    || row.call_site_node_id != call.call_site_node_id
                    || row.call_fact_id != call.call_fact_id
                    || row.source_argument_fact_id != call.source_argument_fact_id
                    || row.pysa_fact_id != call.pysa_fact_id
                    || row.model_id != call.model_id || row.rule_id != call.rule_id
                    || row.callee_resolution_fact_id != call.callee_resolution_fact_id
                    || row.call_start_byte != call.call_start_byte
                    || row.call_end_byte != call.call_end_byte
                    || row.source_value_start_byte != call.source_value_start_byte
                    || row.source_value_end_byte != call.source_value_end_byte
            })
            || arguments.iter().filter(|row|
                row.argument_fact_id == call.source_argument_fact_id).count() != 1
            || call.operand_start_byte != call.source_value_start_byte
            || call.operand_end_byte != call.source_value_end_byte
        {
            return Err(BoundaryReason::MissingEvidence);
        }
    }
    let outer = steps[0][0];
    if outer.call_start_byte != outer.sink_start_byte
        || outer.call_end_byte != outer.sink_end_byte
    {
        return Err(BoundaryReason::MissingEvidence);
    }
    for pair in steps.windows(2) {
        let outer = pair[0][0];
        let inner = pair[1][0];
        if outer.source_value_start_byte != inner.call_start_byte
            || outer.source_value_end_byte != inner.call_end_byte
        {
            return Err(BoundaryReason::MissingEvidence);
        }
    }
    fn visit(
        index: usize,
        steps: &[Vec<&ModeledChainArgument>],
        raw: Id,
        proof: &mut Vec<recipe::SummaryFlowProofStep>,
    ) -> Result<(), BoundaryReason> {
        let call = steps[index][0];
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeResolution,
            evidence_id: call.callee_resolution_fact_id,
            condition_id: call.condition_id,
        });
        for argument in &steps[index] {
            let evidence = if argument.argument_fact_id == call.source_argument_fact_id {
                if index + 1 < steps.len() {
                    visit(index + 1, steps, raw, proof)?;
                    steps[index + 1][0].call_fact_id
                } else {
                    raw
                }
            } else {
                if argument.evaluation_status == ModeledArgumentEvaluationStatus::Unknown
                    || argument.evaluation_status == ModeledArgumentEvaluationStatus::SourceOperand
                {
                    return Err(BoundaryReason::MissingEvidence);
                }
                argument.evaluation_evidence_id.ok_or(BoundaryReason::MissingEvidence)?
            };
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::ArgumentEvaluation,
                evidence_id: evidence,
                condition_id: call.condition_id,
            });
        }
        for (kind, evidence_id) in [
            (SummaryFlowStepKind::CallSite, call.call_fact_id),
            (SummaryFlowStepKind::CallTarget, call.pysa_fact_id),
            (SummaryFlowStepKind::ModelRule, call.rule_id),
        ] {
            proof.push(recipe::SummaryFlowProofStep {
                kind, evidence_id, condition_id: call.condition_id,
            });
        }
        Ok(())
    }
    let mut proof = Vec::new();
    visit(0, &steps, first.source_flow_fact_id, &mut proof)?;
    Ok(proof)
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
        source_origin: path.source_origin_id,
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
        source_origin_id: path.source_origin_id,
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
    finite_flows_with_pair_limit(inputs, 1_000_000)
}

// The override gives the pure cap control a small, deterministic state space. Production
// always uses the same bounded producer and its one-million-pair limit.
fn finite_flows_with_pair_limit(inputs: FiniteSummaryInputs, max_pair_work: usize)
    -> FiniteSummaryOutcome
{
    let FiniteSummaryInputs {
        diagrams, boundaries, mut pass_steps, mut preceding_calls, normal_predecessors, components,
        direct_seeds, modeled_seeds, chain_arguments, evaluations, assignment_seeds, local_seeds,
        boundary_candidates,
    } = inputs;
    let mut refusals = Vec::new();
    // The pure producer must own source order. A caller can supply rows in any order, and
    // `take_while` below must never skip an earlier call after seeing a later one.
    for calls in preceding_calls.values_mut() {
        calls.sort_by_key(|call| (call.call_start_byte, call.call_fact_id));
    }
    for passes in pass_steps.values_mut() {
        passes.sort_by_key(|pass| (pass.ordinal, pass.pass_fact_id));
    }
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
            seed.source_flow_fact_id, seed.condition_id, seed.source_origin_id);
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
        let mut proof = match preceding_call_steps(seed.function_node_id,
            seed.return_start_byte, seed.condition_id, &preceding_calls,
            &normal_by_call, &diagrams, &boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            }
        };
        let Some(model_proof) = modeled_call_proof(
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
        proof.extend(model_proof);
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
                source_origin_id: seed.source_origin_id,
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
    let mut chains: BTreeMap<BoundaryKey, Vec<ModeledChainArgument>> = BTreeMap::new();
    for row in chain_arguments {
        let key = (row.snapshot_id, row.function_node_id, row.parameter_node_id,
            row.source_flow_fact_id, row.condition_id, row.source_origin_id);
        chains.entry(key).or_default().push(row);
    }
    for (key, rows) in chains {
        let seed = &rows[0];
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
        if condition.is_false() { continue; }
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
        let mut proof = match preceding_call_steps(seed.function_node_id,
            seed.return_start_byte, seed.condition_id, &preceding_calls,
            &normal_by_call, &diagrams, &boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            },
        };
        match modeled_chain_proof(&rows) {
            Ok(chain) => proof.extend(chain),
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            },
        }
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ReturnExit,
            evidence_id: seed.return_site_fact_id,
            condition_id: seed.return_condition_id,
        });
        push_finite_path(&mut flows, &mut steps, &pass_steps, FinitePath {
            snapshot_id: seed.snapshot_id,
            function_node_id: seed.function_node_id,
            parameter_node_id: seed.parameter_node_id,
            parameter_name: seed.parameter_name.clone(),
            source_flow_fact_id: seed.source_flow_fact_id,
            source_origin_id: seed.source_origin_id,
            condition_id: seed.condition_id,
            condition_is_true: condition.is_true(),
            return_site_fact_id: seed.return_site_fact_id,
            return_region_fact_id: seed.return_region_fact_id,
            approximated: seed.approximated,
            path_depth: seed.raw_step_count,
            proof,
        });
    }
    for seed in assignment_seeds {
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.successor_condition_id, seed.source_origin_id);
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
        let mut proof = match preceding_call_steps(seed.function_node_id,
            seed.return_start_byte, seed.predecessor_condition_id, &preceding_calls,
            &normal_by_call, &diagrams, &boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            }
        };
        let Some(model_proof) = modeled_call_proof(
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
        proof.extend(model_proof);
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
                source_origin_id: seed.source_origin_id,
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
    // Component order is callee-first. A recursive component consumes each newly cited value
    // path through its local call edges until a least finite fixed point or a typed cap.
    let component_by_function: HashMap<Id, (i64, Id)> = components
        .into_iter()
        .map(|row| (row.function_node_id, (row.component_order, row.component_id)))
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
            seed.source_origin_id,
            seed.pysa_fact_id,
            seed.callee_node_id,
            seed.callee_parameter_node_id,
            seed.return_site_fact_id,
            seed.condition_id,
        )
    });
    let mut by_formal: HashMap<(Id, Id), Vec<SummaryFlowsRow>> = HashMap::new();
    for flow in &flows {
        by_formal
            .entry((flow.function_node_id, flow.parameter_node_id))
            .or_default()
            .push(flow.clone());
    }
    let mut by_id: HashMap<Id, SummaryFlowsRow> = flows.iter()
        .cloned().map(|row| (row.summary_id, row)).collect();
    let mut known_summary_ids: HashSet<Id> = by_id.keys().copied().collect();
    struct PreparedLocalSeed {
        seed: LocalCallSummaryFlowSeed,
        condition_is_true: bool,
        predecessor_proof: Vec<recipe::SummaryFlowProofStep>,
        admitted: bool,
        depth_limited: bool,
        condition_refusal: Option<BoundaryReason>,
    }
    let mut groups: BTreeMap<(i64, Id), Vec<PreparedLocalSeed>> = BTreeMap::new();
    for seed in local_seeds {
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.condition_id, seed.source_origin_id);
        let Some(&component) = component_by_function.get(&seed.function_node_id) else {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        };
        if let Err(reason) = control_evidence(&seed) {
            refuse(&mut refusals, key, reason);
            continue;
        }
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
        let predecessor_proof = match preceding_call_steps(seed.function_node_id,
            seed.return_start_byte, seed.condition_id, &preceding_calls,
            &normal_by_call, &diagrams, &boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            }
        };
        groups.entry(component).or_default().push(PreparedLocalSeed {
            condition_is_true: condition.is_true(), seed, predecessor_proof,
            admitted: false, depth_limited: false, condition_refusal: None,
        });
    }
    const MAX_LOCAL_PATH_DEPTH: i64 = 8;
    for prepared in groups.values_mut() {
        let mut dependent: HashMap<(Id, Id), Vec<usize>> = HashMap::new();
        let mut pending: BTreeSet<(usize, Id)> = BTreeSet::new();
        let mut capped = false;
        for (index, path) in prepared.iter().enumerate() {
            let formal = (path.seed.callee_node_id, path.seed.callee_parameter_node_id);
            dependent.entry(formal).or_default().push(index);
            if let Some(callees) = by_formal.get(&formal) {
                for callee in callees {
                    let pair = (index, callee.summary_id);
                    if !pending.contains(&pair) && pending.len() >= max_pair_work {
                        capped = true;
                        break;
                    }
                    pending.insert(pair);
                }
            }
            if capped { break; }
        }
        let mut pair_work = 0;
        while !capped {
            let Some((index, callee_id)) = pending.pop_first() else { break };
            if pair_work >= max_pair_work {
                capped = true;
                break;
            }
            pair_work += 1;
            let callee = &by_id[&callee_id];
            if callee.verdict == Verdict::Unknown || callee.boundary_reason.is_some()
                || callee.kind != SummaryFlowKind::Value
                || callee.output_path != OutputPath::ReturnValue.render()
            {
                continue;
            }
            let path = &mut prepared[index];
            let seed = &path.seed;
            let Some(callee_condition) = diagrams.get(&callee.condition_id) else {
                path.condition_refusal = Some(boundaries.get(&callee.condition_id).copied()
                    .unwrap_or(BoundaryReason::MissingEvidence));
                continue;
            };
            if (callee_condition.is_true() && callee.verdict != Verdict::Established)
                || (!callee_condition.is_true() && callee.verdict != Verdict::Conditional)
            {
                continue;
            }
            let control = control_evidence(seed).expect("validated local control");
            let specialized = if callee_condition.is_true() {
                None
            } else {
                let Some(control) = control else { continue };
                if !callee_condition.support().iter().any(|candidate|
                    candidate == control.callee_atom) {
                    continue;
                }
                let value = if let Some(value) = control.literal {
                    value
                } else {
                    let caller = diagrams.get(&seed.condition_id)
                        .expect("validated caller condition");
                    let source_atom = control.caller_atom.expect("validated caller atom");
                    match fixed_caller_truth(caller, source_atom) {
                        Ok(Some(value)) => value,
                        Ok(None) => continue,
                        Err(limit) => {
                            path.condition_refusal = Some(condition_limit(limit));
                            continue;
                        }
                    }
                };
                match callee_condition.restrict_atoms(&[(control.callee_atom, value)]) {
                    Ok(restricted) if restricted.is_true() => Some(control.callee_link),
                    Ok(_) => continue,
                    Err(limit) => {
                        path.condition_refusal = Some(condition_limit(limit));
                        continue;
                    }
                }
            };
            if callee.path_depth >= MAX_LOCAL_PATH_DEPTH {
                path.depth_limited = true;
                continue;
            }
            path.admitted = true;
            let mut proof = path.predecessor_proof.clone();
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::CalleeResolution,
                evidence_id: seed.callee_resolution_fact_id,
                condition_id: seed.condition_id,
            });
            let mut evaluations = vec![(seed.source_argument_ordinal, seed.source_flow_fact_id)];
            if let Some(control) = control {
                evaluations.push((seed.control_argument_ordinal
                    .expect("validated control ordinal"), control.evaluation));
            }
            evaluations.sort_by_key(|(ordinal, _)| *ordinal);
            proof.extend(evaluations.into_iter().map(|(_, evidence_id)| {
                recipe::SummaryFlowProofStep {
                    kind: SummaryFlowStepKind::ArgumentEvaluation,
                    evidence_id,
                    condition_id: seed.condition_id,
                }
            }));
            proof.extend([
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
            ].into_iter().map(|(kind, evidence_id, condition_id)|
                recipe::SummaryFlowProofStep { kind, evidence_id, condition_id }));
            if let Some(link) = specialized {
                if let Some(source_link) = control.and_then(|c| c.caller_link) {
                    proof.push(recipe::SummaryFlowProofStep {
                        kind: SummaryFlowStepKind::CallerConditionLink,
                        evidence_id: source_link,
                        condition_id: seed.condition_id,
                    });
                }
                proof.push(recipe::SummaryFlowProofStep {
                    kind: SummaryFlowStepKind::CalleeConditionLink,
                    evidence_id: link,
                    condition_id: callee.condition_id,
                });
            }
            proof.extend([
                (SummaryFlowStepKind::CalleeSummary, callee.summary_id, callee.condition_id),
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
            );
            let previous_steps_len = steps.len();
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
                    source_origin_id: seed.source_origin_id,
                    condition_id: seed.condition_id,
                    condition_is_true: path.condition_is_true,
                    return_site_fact_id: seed.return_site_fact_id,
                    return_region_fact_id: seed.return_region_fact_id,
                    approximated: seed.approximated || callee.approximated,
                    path_depth: callee.path_depth + 1,
                    proof,
                },
            );
            let flow = flows.last().expect("pushed finite local path").clone();
            if !known_summary_ids.insert(flow.summary_id) {
                flows.pop();
                steps.truncate(previous_steps_len);
                continue;
            }
            let formal = (flow.function_node_id, flow.parameter_node_id);
            by_id.insert(flow.summary_id, flow.clone());
            by_formal.entry(formal).or_default().push(flow.clone());
            if let Some(callers) = dependent.get(&formal) {
                for &caller in callers {
                    let pair = (caller, flow.summary_id);
                    if !pending.contains(&pair) && pending.len() >= max_pair_work {
                        capped = true;
                        break;
                    }
                    pending.insert(pair);
                }
            }
        }
        for path in prepared {
            let seed = &path.seed;
            let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
                seed.source_flow_fact_id, seed.condition_id, seed.source_origin_id);
            if capped {
                refuse(&mut refusals, key, BoundaryReason::SummaryPairWorkLimit);
            } else if path.depth_limited {
                refuse(&mut refusals, key, BoundaryReason::SummaryDepthLimit);
            } else if let Some(reason) = path.condition_refusal {
                refuse(&mut refusals, key, reason);
            } else if !path.admitted {
                refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            }
        }
    }
    flows.sort_by_key(|row| row.summary_id);
    steps.sort_by_key(|row| (row.summary_id, row.ordinal));
    refusals.sort_by_key(|row| (row.key(), row.reason));
    refusals.dedup();
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
            source_origin_id: id(9),
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
            chain_arguments: Vec::new(),
            evaluations: Vec::new(),
            assignment_seeds: Vec::new(),
            local_seeds: Vec::new(),
            boundary_candidates: vec![SummaryBoundaryCandidate {
                snapshot_id: id(1),
                function_node_id: id(2),
                parameter_node_id: id(3),
                source_flow_fact_id: id(4),
                source_origin_id: id(9),
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

    #[test]
    fn modeled_chain_proof_is_depth_generic_source_ordered_and_conservative() {
        fn row(step: u8, ordinal: i64) -> ModeledChainArgument {
            let spans = [(10, 40, 20, 39), (20, 39, 30, 38), (30, 38, 35, 36)];
            let (call_start, call_end, source_start, source_end) = spans[usize::from(step)];
            ModeledChainArgument {
                snapshot_id: id(1), function_node_id: id(2), parameter_node_id: id(3),
                parameter_name: "value".to_owned(), source_flow_fact_id: id(4),
                source_origin_id: id(9), condition_id: Diagram::always().id(),
                step: i64::from(step), raw_step_count: 3,
                call_start_byte: call_start, call_end_byte: call_end,
                operand_start_byte: source_start, operand_end_byte: source_end,
                sink_start_byte: 10, sink_end_byte: 40,
                call_site_node_id: id(10 + step), call_fact_id: id(20 + step),
                source_argument_fact_id: id(30 + step),
                source_value_start_byte: source_start, source_value_end_byte: source_end,
                pysa_fact_id: id(60 + step), model_id: id(70 + step),
                rule_id: id(80 + step), callee_resolution_fact_id: id(90 + step),
                argument_count: 2, argument_ordinal: ordinal,
                argument_fact_id: if ordinal == 0 { id(40 + step) } else { id(30 + step) },
                evaluation_status: if ordinal == 0 {
                    ModeledArgumentEvaluationStatus::LiteralNormal
                } else {
                    ModeledArgumentEvaluationStatus::SourceOperand
                },
                evaluation_evidence_id: (ordinal == 0).then(|| id(50 + step)),
                return_site_fact_id: id(5), return_region_fact_id: id(6),
                return_condition_id: Diagram::always().id(), return_start_byte: 8,
                approximated: false,
            }
        }
        let rows: Vec<_> = (0..3).flat_map(|step| [row(step, 0), row(step, 1)]).collect();
        let proof = modeled_chain_proof(&rows).unwrap();
        assert_eq!(proof.iter().filter(|step| step.kind == SummaryFlowStepKind::ModelRule).count(), 3);
        let mut shuffled = rows.clone();
        shuffled.reverse();
        let identity = |steps: Vec<recipe::SummaryFlowProofStep>| {
            steps.into_iter().map(|step| (step.kind, step.evidence_id, step.condition_id))
                .collect::<Vec<_>>()
        };
        assert_eq!(identity(modeled_chain_proof(&shuffled).unwrap()), identity(proof.clone()));
        assert_eq!(proof[0].kind, SummaryFlowStepKind::CalleeResolution);
        assert_eq!(proof[1].evidence_id, id(50));
        assert_eq!(proof[2].kind, SummaryFlowStepKind::CalleeResolution);
        let mut missing = rows.clone();
        missing.retain(|row| !(row.step == 1 && row.argument_ordinal == 0));
        assert_eq!(modeled_chain_proof(&missing).err(), Some(BoundaryReason::MissingEvidence));
        let mut raising = rows;
        raising[0].evaluation_status = ModeledArgumentEvaluationStatus::Unknown;
        assert_eq!(modeled_chain_proof(&raising).err(), Some(BoundaryReason::MissingEvidence));
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

    fn recursive_member(function: u8, ordinal: i64) -> SummaryComponentsRow {
        SummaryComponentsRow {
            snapshot_id: id(1), component_id: id(80), function_node_id: id(function),
            component_order: 0, member_ordinal: ordinal, member_count: 2,
            recursive: true,
        }
    }

    fn recursive_edge(caller: u8, formal: u8, callee: u8, callee_formal: u8,
        raw: u8, origin: u8, call: u8) -> LocalCallSummaryFlowSeed
    {
        LocalCallSummaryFlowSeed {
            snapshot_id: id(1), function_node_id: id(caller), parameter_node_id: id(formal),
            parameter_name: "value".to_owned(), source_flow_fact_id: id(raw),
            source_origin_id: id(origin), condition_id: Diagram::always().id(),
            call_site_node_id: id(call), call_fact_id: id(call + 1),
            pysa_fact_id: id(call + 2), callee_node_id: id(callee),
            callee_parameter_node_id: id(callee_formal),
            callee_resolution_fact_id: id(call + 3), return_site_fact_id: id(call + 4),
            return_region_fact_id: id(call + 5),
            return_condition_id: Diagram::always().id(), return_start_byte: 30,
            approximated: false,
            source_argument_ordinal: 0,
            control_argument_fact_id: None, control_argument_ordinal: None,
            control_evaluation_fact_id: None,
            control_formal_node_id: None, control_link_id: None,
            control_atom: None, control_value: None,
            control_source_link_id: None, control_source_atom: None,
        }
    }

    fn recursive_candidate(seed: &LocalCallSummaryFlowSeed) -> SummaryBoundaryCandidate {
        SummaryBoundaryCandidate {
            snapshot_id: seed.snapshot_id, function_node_id: seed.function_node_id,
            parameter_node_id: seed.parameter_node_id,
            source_flow_fact_id: seed.source_flow_fact_id,
            source_origin_id: seed.source_origin_id, condition_id: seed.condition_id,
            use_id: seed.call_site_node_id, source_key: "Parameter[value] via call".to_owned(),
            through_call: true, local_through_call: true, upstream_through_call: false,
            raw_through_call: true, raw_approximated: false, reach_budget: false,
        }
    }

    #[test]
    fn self_recursive_finite_base_proves_paths_but_keeps_depth_cut_open() {
        let mut input = inputs();
        input.components.push(recursive_member(2, 0));
        let edge = recursive_edge(2, 3, 2, 3, 40, 41, 50);
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.local_seeds.push(edge);
        let result = finite_flows(input);
        let mut depths = result.flows.iter().map(|row| row.path_depth).collect::<Vec<_>>();
        depths.sort_unstable();
        assert_eq!(depths, (0..=8).collect::<Vec<_>>());
        assert!(result.flows.iter().all(|row| row.verdict == Verdict::Established));
        assert_eq!(result.boundaries.len(), 1);
        assert_eq!(result.boundaries[0].source_origin_id, id(41));
        assert_eq!(result.boundaries[0].reason, BoundaryReason::SummaryDepthLimit);
        assert_eq!(result.steps.iter().filter(|step|
            step.kind == SummaryFlowStepKind::CalleeSummary).count(), 8);
    }

    #[test]
    fn exact_literal_specializes_one_conditional_recursive_base() {
        use cpg_schema::condition::EvaluationIdentity;

        let atom = Atom::Truthy { place: "stop".to_owned() }.evaluated(
            EvaluationIdentity::Synthetic {
                module: "00".repeat(16), predicate: "11".repeat(16),
            });
        let guard = Diagram::from_atom(&atom).unwrap();
        let else_path = guard.not().unwrap();
        let make_input = |literal: bool| {
            let mut input = inputs();
            input.diagrams.insert(guard.id(), guard.clone());
            input.diagrams.insert(else_path.id(), else_path.clone());
            input.direct_seeds[0].condition_id = guard.id();
            input.boundary_candidates[0].condition_id = guard.id();
            input.components.push(recursive_member(2, 0));
            let mut edge = recursive_edge(2, 3, 2, 3, 40, 41, 50);
            edge.condition_id = else_path.id();
            edge.control_argument_fact_id = Some(id(70));
            edge.control_argument_ordinal = Some(1);
            edge.control_evaluation_fact_id = Some(id(71));
            edge.control_formal_node_id = Some(id(72));
            edge.control_link_id = Some(id(73));
            edge.control_atom = Some(atom.encode());
            edge.control_value = Some(literal);
            edge.control_source_link_id = None;
            edge.control_source_atom = None;
            input.boundary_candidates.push(recursive_candidate(&edge));
            input.local_seeds.push(edge);
            input
        };
        let positive = finite_flows(make_input(true));
        let local = positive.flows.iter().find(|row| row.source_origin_id == id(41)).unwrap();
        assert_eq!(local.path_depth, 1);
        assert_eq!(local.condition_id, else_path.id());
        assert_eq!(local.verdict, Verdict::Conditional);
        assert!(positive.boundaries.is_empty());
        assert_eq!(positive.steps.iter().filter(|step| step.summary_id == local.summary_id
            && step.kind == SummaryFlowStepKind::ArgumentEvaluation).count(), 2);
        assert!(positive.steps.iter().any(|step| step.summary_id == local.summary_id
            && step.kind == SummaryFlowStepKind::CalleeConditionLink
            && step.evidence_id == id(73)));

        let mut reversed = make_input(true);
        reversed.local_seeds[0].source_argument_ordinal = 1;
        reversed.local_seeds[0].control_argument_ordinal = Some(0);
        let reversed = finite_flows(reversed);
        let reversed_path = reversed.flows.iter()
            .find(|row| row.source_origin_id == id(41)).unwrap();
        let evaluations: Vec<_> = reversed.steps.iter()
            .filter(|step| step.summary_id == reversed_path.summary_id
                && step.kind == SummaryFlowStepKind::ArgumentEvaluation)
            .map(|step| step.evidence_id)
            .collect();
        assert_eq!(evaluations, vec![id(71), id(40)],
            "proof evaluations follow syntax order, not the tracked source argument");

        let withheld = finite_flows(make_input(false));
        assert!(!withheld.flows.iter().any(|row| row.source_origin_id == id(41)));
        assert!(withheld.boundaries.iter().any(|row| row.source_origin_id == id(41)
            && row.reason == BoundaryReason::CallTransfer));
    }

    #[test]
    fn exact_forwarded_control_requires_the_caller_guard() {
        use cpg_schema::condition::EvaluationIdentity;

        let callee_atom = Atom::Truthy { place: "stop".to_owned() }.evaluated(
            EvaluationIdentity::Synthetic {
                module: "00".repeat(16), predicate: "11".repeat(16),
            });
        let caller_atom = Atom::Truthy { place: "stop".to_owned() }.evaluated(
            EvaluationIdentity::Synthetic {
                module: "00".repeat(16), predicate: "22".repeat(16),
            });
        let callee_guard = Diagram::from_atom(&callee_atom).unwrap();
        let caller_guard = Diagram::from_atom(&caller_atom).unwrap();
        let mut input = inputs();
        for root in [&callee_guard, &caller_guard] {
            input.diagrams.insert(root.id(), root.clone());
        }
        input.direct_seeds[0].condition_id = callee_guard.id();
        input.boundary_candidates[0].condition_id = callee_guard.id();
        input.components.extend([recursive_member(2, 0), recursive_member(4, 1)]);
        let mut edge = recursive_edge(4, 5, 2, 3, 40, 41, 50);
        edge.condition_id = caller_guard.id();
        edge.control_argument_fact_id = Some(id(70));
        edge.control_argument_ordinal = Some(1);
        edge.control_evaluation_fact_id = Some(id(71));
        edge.control_formal_node_id = Some(id(72));
        edge.control_link_id = Some(id(73));
        edge.control_atom = Some(callee_atom.encode());
        edge.control_source_link_id = Some(id(74));
        edge.control_source_atom = Some(caller_atom.encode());
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.local_seeds.push(edge);
        let positive = finite_flows(input.clone());
        let local = positive.flows.iter().find(|row| row.source_origin_id == id(41)).unwrap();
        assert_eq!(local.condition_id, caller_guard.id());
        assert_eq!(local.verdict, Verdict::Conditional);
        assert!(positive.steps.iter().any(|step| step.summary_id == local.summary_id
            && step.kind == SummaryFlowStepKind::CallerConditionLink
            && step.evidence_id == id(74)));
        assert!(positive.steps.iter().any(|step| step.summary_id == local.summary_id
            && step.kind == SummaryFlowStepKind::CalleeConditionLink
            && step.evidence_id == id(73)));

        let opposite = caller_guard.not().unwrap();
        input.diagrams.insert(opposite.id(), opposite.clone());
        input.local_seeds[0].condition_id = opposite.id();
        input.boundary_candidates[1].condition_id = opposite.id();
        let withheld = finite_flows(input);
        assert!(!withheld.flows.iter().any(|row| row.source_origin_id == id(41)));
        assert!(withheld.boundaries.iter().any(|row| row.source_origin_id == id(41)
            && row.reason == BoundaryReason::CallTransfer));
    }

    #[test]
    fn mutual_recursion_parallel_origins_and_shuffle_have_stable_proofs() {
        let mut forward = inputs();
        forward.components = vec![recursive_member(2, 0), recursive_member(7, 1)];
        let a = recursive_edge(7, 8, 2, 3, 40, 41, 50);
        let parallel = recursive_edge(7, 8, 2, 3, 42, 43, 60);
        let back = recursive_edge(2, 3, 7, 8, 44, 45, 70);
        let open = recursive_edge(7, 8, 2, 3, 46, 47, 90);
        for edge in [&a, &parallel, &back] {
            forward.boundary_candidates.push(recursive_candidate(edge));
        }
        forward.boundary_candidates.push(recursive_candidate(&open));
        forward.local_seeds = vec![a, parallel, back];
        let mut reversed = inputs();
        reversed.components = forward.components.iter().rev().cloned().collect();
        reversed.boundary_candidates = forward.boundary_candidates.iter().rev().cloned().collect();
        reversed.local_seeds = forward.local_seeds.iter().rev().cloned().collect();
        let first = finite_flows(forward);
        let second = finite_flows(reversed);
        assert_eq!(first.flows, second.flows);
        assert_eq!(first.steps, second.steps);
        assert_eq!(first.refusals, second.refusals);
        assert_eq!(first.boundaries, second.boundaries);
        assert!(first.flows.iter().any(|row| row.source_origin_id == id(41)));
        assert!(first.flows.iter().any(|row| row.source_origin_id == id(43)));
        assert!(first.flows.iter().any(|row| row.source_origin_id == id(45)));
        assert!(!first.flows.iter().any(|row| row.source_origin_id == id(47)));
        assert!(first.boundaries.iter().any(|row| row.source_origin_id == id(47)
            && row.reason == BoundaryReason::CallTransfer));
        assert!(first.flows.iter().all(|row| row.path_depth <= 8));
    }

    #[test]
    fn base_free_cycle_and_pair_cap_remain_origin_specific_unknowns() {
        let mut no_base = inputs();
        no_base.direct_seeds.clear();
        no_base.boundary_candidates.clear();
        no_base.components = vec![recursive_member(2, 0), recursive_member(7, 1)];
        let a = recursive_edge(2, 3, 7, 8, 40, 41, 50);
        let b = recursive_edge(7, 8, 2, 3, 42, 43, 60);
        for edge in [&a, &b] { no_base.boundary_candidates.push(recursive_candidate(edge)); }
        no_base.local_seeds = vec![a, b];
        let result = finite_flows(no_base);
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries.len(), 2);
        assert!(result.boundaries.iter().all(|row| row.reason == BoundaryReason::CallTransfer));

        let mut capped = inputs();
        capped.components.push(recursive_member(2, 0));
        let edge = recursive_edge(2, 3, 2, 3, 40, 41, 50);
        capped.boundary_candidates.push(recursive_candidate(&edge));
        capped.local_seeds.push(edge);
        let result = finite_flows_with_pair_limit(capped, 1);
        assert!(result.flows.iter().any(|row| row.source_origin_id == id(41)));
        assert_eq!(result.boundaries.len(), 1);
        assert_eq!(result.boundaries[0].source_origin_id, id(41));
        assert_eq!(result.boundaries[0].reason, BoundaryReason::SummaryPairWorkLimit);

        let capped_parallel = |reverse: bool| {
            let mut input = inputs();
            input.components.push(recursive_member(2, 0));
            let mut edges = vec![
                recursive_edge(2, 3, 2, 3, 40, 41, 50),
                recursive_edge(2, 3, 2, 3, 42, 43, 60),
            ];
            if reverse { edges.reverse(); }
            input.boundary_candidates.extend(edges.iter().map(recursive_candidate));
            input.local_seeds = edges;
            finite_flows_with_pair_limit(input, 1)
        };
        let first = capped_parallel(false);
        let reversed = capped_parallel(true);
        assert_eq!(first.flows, reversed.flows);
        assert_eq!(first.boundaries, reversed.boundaries);
        assert_eq!(first.boundaries.len(), 2);
        assert!(first.boundaries.iter().all(|row|
            row.reason == BoundaryReason::SummaryPairWorkLimit));
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
    fn proved_origin_does_not_erase_an_unproved_sibling_on_the_same_raw_fact() {
        let mut input = inputs();
        let mut sibling = input.boundary_candidates[0].clone();
        sibling.source_origin_id = id(10);
        sibling.source_key = "Parameter[value] via call".to_owned();
        sibling.through_call = true;
        sibling.local_through_call = true;
        input.boundary_candidates.push(sibling);

        let outcome = finite_flows(input);
        assert_eq!(outcome.flows.len(), 1);
        assert_eq!(outcome.flows[0].source_origin_id, id(9));
        assert_eq!(outcome.boundaries.len(), 1);
        assert_eq!(outcome.boundaries[0].source_origin_id, id(10));
        assert_eq!(outcome.boundaries[0].reason, BoundaryReason::CallTransfer);
    }

    #[test]
    fn shuffled_predecessors_cannot_hide_an_earlier_unproved_call() {
        let later = PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(40), call_start_byte: 40,
            condition_id: None, approximated: None,
        };
        let earlier = PrecedingCallRegion {
            function_node_id: id(2), call_fact_id: id(7), call_start_byte: 10,
            condition_id: None, approximated: None,
        };
        let mut forward = inputs();
        forward.preceding_calls.insert(id(2), vec![earlier.clone(), later.clone()]);
        let mut reverse = inputs();
        reverse.preceding_calls.insert(id(2), vec![later, earlier]);
        let first = finite_flows(forward);
        let second = finite_flows(reverse);
        assert!(first.flows.is_empty() && second.flows.is_empty());
        assert_eq!(first.boundaries, second.boundaries);
        assert_eq!(first.boundaries[0].reason, BoundaryReason::UnsupportedControlFlow);
    }

    #[test]
    fn shuffled_finalizer_steps_keep_canonical_proof_identity() {
        let first_pass = ReturnPassStep {
            return_site_fact_id: id(5), pass_fact_id: id(50),
            condition_id: Diagram::always().id(), ordinal: 0,
        };
        let second_pass = ReturnPassStep {
            return_site_fact_id: id(5), pass_fact_id: id(51),
            condition_id: Diagram::always().id(), ordinal: 1,
        };
        let mut forward = inputs();
        forward.pass_steps.insert(id(5), vec![first_pass.clone(), second_pass.clone()]);
        let mut reverse = inputs();
        reverse.pass_steps.insert(id(5), vec![second_pass, first_pass]);
        let first = finite_flows(forward);
        let second = finite_flows(reverse);
        assert_eq!(first.flows, second.flows);
        assert_eq!(first.steps, second.steps);
        assert_eq!(first.steps.iter().map(|step| step.evidence_id).collect::<Vec<_>>(),
            [id(4), id(50), id(51)]);
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
                source_origin_id: id(n + 61),
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
                return_start_byte: 30,
                approximated: false,
                source_argument_ordinal: 0,
                control_argument_fact_id: None, control_argument_ordinal: None,
                control_evaluation_fact_id: None,
                control_formal_node_id: None, control_link_id: None,
                control_atom: None, control_value: None,
                control_source_link_id: None, control_source_atom: None,
            });
            input.boundary_candidates.push(SummaryBoundaryCandidate {
                snapshot_id: id(1),
                function_node_id: id(n),
                parameter_node_id: id(n + 30),
                source_flow_fact_id: id(n + 60),
                source_origin_id: id(n + 61),
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
            source_origin_id: id(9),
            condition_id: id(200), call_site_node_id: id(10), call_fact_id: id(11),
            pysa_fact_id: id(12), callee_node_id: id(13),
            callee_parameter_node_id: id(14), callee_resolution_fact_id: id(15),
            return_site_fact_id: id(5), return_region_fact_id: id(6),
            return_condition_id: always, return_start_byte: 30, approximated: false,
            source_argument_ordinal: 0,
            control_argument_fact_id: None, control_argument_ordinal: None,
            control_evaluation_fact_id: None,
            control_formal_node_id: None, control_link_id: None,
            control_atom: None, control_value: None,
            control_source_link_id: None, control_source_atom: None,
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
            source_origin_id: id(9),
            condition_id: source_id, call_site_node_id: id(10), call_fact_id: id(11),
            pysa_fact_id: id(12), callee_node_id: id(13),
            callee_parameter_node_id: id(14), callee_resolution_fact_id: id(15),
            return_site_fact_id: id(5), return_region_fact_id: id(6),
            return_condition_id: exit_id, return_start_byte: 30, approximated: false,
            source_argument_ordinal: 0,
            control_argument_fact_id: None, control_argument_ordinal: None,
            control_evaluation_fact_id: None,
            control_formal_node_id: None, control_link_id: None,
            control_atom: None, control_value: None,
            control_source_link_id: None, control_source_atom: None,
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
