//! Pure finite source-to-return summary composition over explicit, typed inputs.
//! DataFusion acquisition and Delta publication belong to `cpg-core`.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use cpg_schema::behavior::{ModeledArgumentEvaluationsRow, ReturnEntryStatusesRow, ReturnEntryStepsRow, SummaryBoundariesRow, SummaryComponentsRow, SummaryFlowStepsRow, SummaryFlowsRow};
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
        argument_count: i64,
        binding_complete: bool,
    }
}

pub use cpg_schema::summary_contract::LocalCallArgument;

pub use cpg_schema::summary_contract::DirectValueLink as LocalCallValueLink;
use cpg_schema::summary_contract::{fixed_truth, validate_fixed_control_proof};

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
    pub coverage: Vec<cpg_schema::behavior::SummaryOriginCoverageRow>,
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
        BoundaryReason::SummaryDepthLimit | BoundaryReason::ExpressionDepthLimit => 0,
        BoundaryReason::SummaryPairWorkLimit | BoundaryReason::ExpressionWorkLimit => 1,
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

/// Semantic progress excludes proof identity and path length. Source origins and return sites
/// remain separate: equality here never merges parallel calls or discharges another origin.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SemanticFlow {
    snapshot: Id,
    function: Id,
    parameter: Id,
    source: Id,
    origin: Id,
    condition: Id,
    return_site: Id,
    return_region: Id,
    input: String,
    output: String,
    kind: SummaryFlowKind,
    verdict: Verdict,
    refusal: Option<BoundaryReason>,
    approximated: bool,
}

impl From<&SummaryFlowsRow> for SemanticFlow {
    fn from(row: &SummaryFlowsRow) -> Self {
        Self { snapshot: row.snapshot_id, function: row.function_node_id,
            parameter: row.parameter_node_id, source: row.source_flow_fact_id,
            origin: row.source_origin_id, condition: row.condition_id,
            return_site: row.return_site_fact_id, return_region: row.return_region_fact_id,
            input: row.input_path.clone(), output: row.output_path.clone(), kind: row.kind,
            verdict: row.verdict, refusal: row.boundary_reason, approximated: row.approximated }
    }
}

/// Only a new semantic fact or a shorter bounded proof can enable a caller. Longer witnesses
/// are retained locally, but never become fresh recursive work merely because their IDs differ.
#[derive(Default)]
struct SemanticFrontier {
    representatives: HashMap<SemanticFlow, (i64, Id)>,
    by_formal: HashMap<(Id, Id), BTreeSet<SemanticFlow>>,
}

impl SemanticFrontier {
    fn insert(&mut self, row: &SummaryFlowsRow) -> bool {
        let key = SemanticFlow::from(row);
        if self.representatives.get(&key).is_some_and(|(depth,_)| *depth <= row.path_depth) {
            return false;
        }
        self.by_formal.entry((row.function_node_id,row.parameter_node_id))
            .or_default().insert(key.clone());
        self.representatives.insert(key,(row.path_depth,row.summary_id));
        true
    }

    fn current(&self, row: &SummaryFlowsRow) -> bool {
        self.representatives.get(&SemanticFlow::from(row))
            .is_some_and(|(_,id)| *id == row.summary_id)
    }
}

/// Retaining a representative is not an exhaustive list of proofs. Mark suppressed witness
/// expansion and propagate that disclosure to callers citing any representative of that fact.
fn unexpanded_witnesses(flows: &[SummaryFlowsRow], steps: &[SummaryFlowStepsRow]) -> BTreeSet<Id> {
    let mut groups: HashMap<SemanticFlow,BTreeSet<Id>> = HashMap::new();
    for row in flows { groups.entry(SemanticFlow::from(row)).or_default().insert(row.summary_id); }
    let mut omitted: BTreeSet<_> = groups.values().filter(|ids| ids.len()>1)
        .flat_map(|ids|ids.iter().copied()).collect();
    let mut callers: HashMap<Id,Vec<Id>> = HashMap::new();
    for step in steps.iter().filter(|s|s.kind==SummaryFlowStepKind::CalleeSummary) {
        callers.entry(step.evidence_id).or_default().push(step.summary_id);
    }
    let mut pending=omitted.clone();
    while let Some(id)=pending.pop_first() {
        for caller in callers.get(&id).into_iter().flatten() {
            if omitted.insert(*caller) {pending.insert(*caller);}
        }
    }
    omitted
}

/// Validate a complete ordered argument group before any summary uses its values.
fn ordered_arguments<'a>(seed: &LocalCallSummaryFlowSeed, rows: &'a [LocalCallArgument])
    -> Result<Vec<&'a LocalCallArgument>, BoundaryReason> {
    if !seed.binding_complete || !(1..=128).contains(&seed.argument_count) {
        return Err(BoundaryReason::UnsupportedControlFlow);
    }
    if let Some(reason) = rows.iter().filter(|row| row.evaluation_fact_id.is_none())
        .filter_map(|row| row.evaluation_reason)
        .filter(|reason| matches!(reason, BoundaryReason::ExpressionDepthLimit | BoundaryReason::ExpressionWorkLimit))
        .min() { return Err(reason); }
    let mut arguments: Vec<_> = rows.iter().collect();
    arguments.sort_by_key(|row| (row.ordinal, row.argument_fact_id));
    if arguments.len() != seed.argument_count as usize
        || arguments.iter().enumerate().any(|(ordinal, row)|
            row.ordinal != ordinal as i64 || row.formal_node_id.is_none()
                || row.evaluation_fact_id.is_none())
        || arguments.iter().filter_map(|row| row.formal_node_id).collect::<BTreeSet<_>>().len()
            != arguments.len()
        || arguments.iter().map(|row| row.argument_fact_id).collect::<BTreeSet<_>>().len()
            != arguments.len()
        || !arguments.iter().any(|row| row.ordinal == seed.source_argument_ordinal
            && row.formal_node_id == Some(seed.callee_parameter_node_id)) {
        return Err(BoundaryReason::MissingEvidence);
    }
    Ok(arguments)
}

struct LocalControlLinks {
    by_id: HashMap<Id, LocalCallValueLink>,
    by_atom: HashMap<(Id, String), Vec<Id>>,
    by_formal: HashMap<(Id, Id), Vec<Id>>,
    valid: bool,
}

impl LocalControlLinks {
    fn new(rows: Vec<LocalCallValueLink>) -> Self {
        let mut result = Self { by_id: HashMap::new(), by_atom: HashMap::new(),
            by_formal: HashMap::new(), valid: true };
        for row in rows {
            result.by_atom.entry((row.operation_node_id, row.atom.clone())).or_default().push(row.link_id);
            result.by_formal.entry((row.operation_node_id, row.formal_node_id)).or_default().push(row.link_id);
            if result.by_id.insert(row.link_id, row).is_some() { result.valid = false; }
        }
        for ids in result.by_atom.values_mut().chain(result.by_formal.values_mut()) { ids.sort(); }
        result
    }
}

/// A linked callee Boolean predicate is replaced only by an exact argument value or a
/// caller-linked direct formal fixed by the caller condition. All mappings apply simultaneously.
fn specialize_local_condition(seed: &LocalCallSummaryFlowSeed,
    arguments: &[&LocalCallArgument], links: &LocalControlLinks,
    caller: &Diagram, callee: &Diagram,
) -> Result<Option<Vec<recipe::SummaryFlowProofStep>>, BoundaryReason> {
    if !links.valid { return Err(BoundaryReason::MissingEvidence); }
    let mut replacements = Vec::new();
    let mut proof = Vec::new();
    for atom in callee.support() {
        if !matches!(Atom::parse_encoded(atom), Ok(Atom::Evaluated { atom, .. })
            if matches!(*atom, Atom::Truthy { .. })) { return Ok(None); }
        let Some(candidates) = links.by_atom.get(&(seed.callee_node_id, atom.clone()))
            else { return Ok(None) };
        let link = &links.by_id[&candidates[0]];
        if candidates.iter().any(|id| links.by_id[id].formal_node_id != link.formal_node_id) {
            return Err(BoundaryReason::MissingEvidence);
        }
        let Some(argument) = arguments.iter().find(|row|
            row.formal_node_id == Some(link.formal_node_id)) else { return Ok(None) };
        let value = if let Some(value) = argument.boolean_value {
            value
        } else {
            let Some(formal) = argument.source_formal_node_id else { return Ok(None) };
            let sources = links.by_formal.get(&(seed.function_node_id, formal));
            let mut fixed = None;
            for source_id in sources.into_iter().flatten() {
                let source = &links.by_id[source_id];
                if !matches!(Atom::parse_encoded(&source.atom), Ok(Atom::Evaluated { atom, .. })
                    if matches!(*atom, Atom::Truthy { .. })) { continue; }
                if let Some(value) = fixed_truth(caller, &source.atom).map_err(condition_limit)? {
                    fixed = Some((source, value));
                    break;
                }
            }
            let Some((source, value)) = fixed else { return Ok(None) };
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::CallerConditionLink, evidence_id: source.link_id,
                condition_id: seed.condition_id,
            });
            value
        };
        replacements.push((atom.as_str(), if value { Diagram::always() } else { Diagram::never() }));
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeConditionLink, evidence_id: link.link_id,
            condition_id: callee.id(),
        });
    }
    let refs: Vec<_> = replacements.iter().map(|(atom, value)| (*atom, value)).collect();
    let result = callee.substitute_atoms(&refs).map_err(condition_limit)?;
    if !result.is_true() { return Ok(None); }
    validate_fixed_control_proof(seed.function_node_id, caller, seed.callee_node_id, callee,
        &proof, &links.by_id).map_err(|_| BoundaryReason::MissingEvidence)?;
    Ok(Some(proof))
}

cpg_schema::query_row! {
    pub struct ReturnPassStep {
        return_site_fact_id: Id,
        pass_fact_id: Id,
        kind: SummaryFlowStepKind,
        condition_id: Id,
        ordinal: i64,
    }
}

type EntryIndex = HashMap<Id, Vec<ReturnEntryStatusesRow>>;
type EntryStepIndex = HashMap<Id, Vec<ReturnEntryStepsRow>>;
type ReturnPassIndex = HashMap<Id, Vec<ReturnPassStep>>;

#[derive(Clone)]
pub struct FiniteSummaryInputs {
    pub diagrams: HashMap<Id, Diagram>,
    pub boundaries: HashMap<Id, BoundaryReason>,
    pub pass_steps: HashMap<Id, Vec<ReturnPassStep>>,
    pub entries: Vec<ReturnEntryStatusesRow>,
    pub entry_steps: Vec<ReturnEntryStepsRow>,
    pub components: Vec<SummaryComponentsRow>,
    pub direct_seeds: Vec<SummaryFlowSeed>,
    pub modeled_seeds: Vec<ModeledSummaryFlowSeed>,
    pub chain_arguments: Vec<ModeledChainArgument>,
    pub evaluations: Vec<ModeledArgumentEvaluationsRow>,
    pub assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed>,
    pub local_seeds: Vec<LocalCallSummaryFlowSeed>,
    pub local_arguments: Vec<LocalCallArgument>,
    pub local_value_links: Vec<LocalCallValueLink>,
    pub boundary_candidates: Vec<SummaryBoundaryCandidate>,
}

fn entry_proof(
    key: (Id,Id,Id,Id),
    entries: &EntryIndex,
    steps: &EntryStepIndex,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
) -> Result<Vec<recipe::SummaryFlowProofStep>, BoundaryReason> {
    let (snapshot_id,function_node_id,return_site_fact_id,condition_id)=key;
    let all=entries.get(&return_site_fact_id).ok_or(BoundaryReason::MissingEvidence)?;
    let exact:Vec<_>=all.iter().filter(|r|r.condition_id==condition_id).collect();
    let rows=if exact.is_empty() {all.iter().collect::<Vec<_>>()} else {exact};
    if rows.len()!=1 || rows[0].function_node_id!=function_node_id || rows[0].snapshot_id!=snapshot_id || rows[0].work<=0 {
        return Err(BoundaryReason::MissingEvidence);
    }
    let entry=&rows[0];
    if let Some(reason)=entry.reason { return Err(reason); }
    let condition=diagrams.get(&condition_id).ok_or_else(||boundaries.get(&condition_id).copied().unwrap_or(BoundaryReason::MissingEvidence))?;
    let required=diagrams.get(&entry.condition_id).ok_or_else(||boundaries.get(&entry.condition_id).copied().unwrap_or(BoundaryReason::MissingEvidence))?;
    if !condition.implies(required).map_err(condition_limit)? { return Err(BoundaryReason::UnsupportedControlFlow); }
    let steps:Vec<_>=steps.get(&return_site_fact_id).into_iter().flatten().filter(|s|s.condition_id==entry.condition_id).collect();
    if steps.len()>cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS { return Err(BoundaryReason::SummaryProofLimit); }
    if steps.iter().enumerate().any(|(ordinal,s)|s.ordinal!=ordinal as i64
        || s.snapshot_id!=entry.snapshot_id || s.condition_id!=entry.condition_id) {
        return Err(BoundaryReason::MissingEvidence);
    }
    Ok(steps.iter().map(|s|recipe::SummaryFlowProofStep {kind:s.kind,evidence_id:s.evidence_id,condition_id}).collect())
}

/// The first finite summary case: a direct, synchronous body return of a local parameter.
/// A bounded/missing condition is a named unknown, never an admitted positive flow.
fn direct_flows(
    seeds: Vec<SummaryFlowSeed>,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
    pass_steps: &ReturnPassIndex,
    entries: &EntryIndex,
    entry_steps: &EntryStepIndex,
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
        let mut proof = match entry_proof((seed.snapshot_id, seed.function_node_id,
            seed.return_site_fact_id, seed.condition_id), entries,
            entry_steps, diagrams, boundaries) {
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
                kind: finalizer.kind,
                evidence_id: finalizer.pass_fact_id,
                condition_id: finalizer.condition_id,
            });
        }
        if proof.len() > cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS {
            refuse(refusals, (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
                seed.source_flow_fact_id, seed.condition_id, seed.source_origin_id), BoundaryReason::SummaryProofLimit);
            continue;
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
            || (argument.argument_fact_id == seed.source_argument_fact_id
                && (argument.status != ModeledArgumentEvaluationStatus::SourceOperand
                    || argument.evidence_id != Some(seed.flow_fact_id)
                    || argument.source_normal_evidence_id.is_none()))
            || (argument.argument_fact_id != seed.source_argument_fact_id
                && argument.source_normal_evidence_id.is_some())
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
    let mut proof = Vec::with_capacity(ordered.len() + 5);
    proof.push(recipe::SummaryFlowProofStep {
        kind: SummaryFlowStepKind::CalleeResolution,
        evidence_id: seed.callee_resolution_fact_id,
        condition_id: seed.condition_id,
    });
    for argument in &ordered {
        if argument.argument_fact_id == seed.source_argument_fact_id {
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::RawIdentity,
                evidence_id: seed.flow_fact_id,
                condition_id: seed.condition_id,
            });
        }
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ArgumentEvaluation,
            evidence_id: if argument.argument_fact_id == seed.source_argument_fact_id {
                argument.source_normal_evidence_id?
            } else {
                argument.evidence_id?
            },
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
                    if !matches!(argument.evaluation_status,
                        ModeledArgumentEvaluationStatus::ParameterNameNormal
                            | ModeledArgumentEvaluationStatus::LexicalParameterNormal)
                    {
                        return Err(BoundaryReason::MissingEvidence);
                    }
                    proof.push(recipe::SummaryFlowProofStep {
                        kind: SummaryFlowStepKind::RawIdentity,
                        evidence_id: raw,
                        condition_id: call.condition_id,
                    });
                    argument.evaluation_evidence_id.ok_or(BoundaryReason::MissingEvidence)?
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
    refusals: &mut Vec<SummaryRefusal>,
    path: FinitePath,
) -> bool {
    let input_path = InputPath::Parameter {
        name: path.parameter_name,
    }
    .render();
    let output_path = OutputPath::ReturnValue.render();
    let mut proof = path.proof;
    let finalizers = pass_steps
        .get(&path.return_site_fact_id)
        .map_or(&[][..], Vec::as_slice);
    if proof.len().saturating_add(finalizers.len()) > cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS {
        refuse(refusals, (path.snapshot_id, path.function_node_id, path.parameter_node_id,
            path.source_flow_fact_id, path.condition_id, path.source_origin_id), BoundaryReason::SummaryProofLimit);
        return false;
    }
    if !finalizers.is_empty() {
        let index = proof
            .iter()
            .position(|step| step.kind == SummaryFlowStepKind::ReturnExit)
            .unwrap_or(proof.len());
        for (offset, finalizer) in finalizers.iter().enumerate() {
            proof.insert(
                index + offset,
                recipe::SummaryFlowProofStep {
                    kind: finalizer.kind,
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
    true
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
        diagrams, boundaries, mut pass_steps, entries, entry_steps, components,
        direct_seeds, modeled_seeds, chain_arguments, evaluations, assignment_seeds, local_seeds,
        local_arguments, local_value_links, boundary_candidates,
    } = inputs;
    let mut refusals = Vec::new();
    for passes in pass_steps.values_mut() { passes.sort_by_key(|pass| (pass.ordinal, pass.pass_fact_id)); }
    let mut entries_by_return:EntryIndex=HashMap::new();
    for row in entries { entries_by_return.entry(row.return_site_fact_id).or_default().push(row); }
    let mut steps_by_return:EntryStepIndex=HashMap::new();
    for row in entry_steps { steps_by_return.entry(row.return_site_fact_id).or_default().push(row); }
    for steps in steps_by_return.values_mut() { steps.sort_by_key(|s|s.ordinal); }
    let (mut flows,mut steps)=direct_flows(direct_seeds,&diagrams,&boundaries,&pass_steps,
        &entries_by_return,&steps_by_return,&mut refusals);
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
        let mut proof = match entry_proof((seed.snapshot_id, seed.function_node_id,
            seed.return_site_fact_id, seed.condition_id), &entries_by_return,
            &steps_by_return, &diagrams, &boundaries) {
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
            &mut refusals,
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
        let mut proof = match entry_proof((seed.snapshot_id, seed.function_node_id,
            seed.return_site_fact_id, seed.condition_id), &entries_by_return,
            &steps_by_return, &diagrams, &boundaries) {
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
        push_finite_path(&mut flows, &mut steps, &pass_steps, &mut refusals, FinitePath {
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
        let mut proof = match entry_proof((seed.snapshot_id, seed.function_node_id,
            seed.return_site_fact_id, seed.predecessor_condition_id), &entries_by_return,
            &steps_by_return, &diagrams, &boundaries) {
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
            &mut refusals,
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
    // Component order is callee-first. Only semantic progress (or a shorter proof) drives
    // recursion; alternate witness identities do not restart the fixed point.
    let component_by_function: HashMap<Id, (i64, Id)> = components
        .into_iter()
        .map(|row| (row.function_node_id, (row.component_order, row.component_id)))
        .collect();
    let local_value_links = LocalControlLinks::new(local_value_links);
    let mut arguments_by_call: HashMap<(Id, Id), Vec<LocalCallArgument>> = HashMap::new();
    for row in local_arguments {
        arguments_by_call.entry((row.call_fact_id, row.callee_node_id)).or_default().push(row);
    }
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
    let mut frontier = SemanticFrontier::default();
    // Stable initial representatives, independent of input order and hash-map iteration.
    flows.sort_by_key(|row| (row.path_depth,row.summary_id));
    for flow in &flows { frontier.insert(flow); }
    let mut by_id: HashMap<Id, SummaryFlowsRow> = flows.iter()
        .cloned().map(|row| (row.summary_id, row)).collect();
    let mut known_summary_ids: HashSet<Id> = by_id.keys().copied().collect();
    struct PreparedLocalSeed<'a> {
        arguments: Vec<&'a LocalCallArgument>,
        seed: LocalCallSummaryFlowSeed,
        condition_is_true: bool,
        predecessor_proof: Vec<recipe::SummaryFlowProofStep>,
        admitted: bool,
        refused: BTreeMap<SemanticFlow, BoundaryReason>,
    }
    let mut groups: BTreeMap<(i64, Id), Vec<PreparedLocalSeed<'_>>> = BTreeMap::new();
    for seed in local_seeds {
        let key = (seed.snapshot_id, seed.function_node_id, seed.parameter_node_id,
            seed.source_flow_fact_id, seed.condition_id, seed.source_origin_id);
        let Some(&component) = component_by_function.get(&seed.function_node_id) else {
            refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            continue;
        };
        let arguments = match ordered_arguments(&seed, arguments_by_call
            .get(&(seed.call_fact_id, seed.callee_node_id)).map_or(&[], Vec::as_slice)) {
            Ok(arguments) => arguments,
            Err(reason) => { refuse(&mut refusals, key, reason); continue; },
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
        let predecessor_proof = match entry_proof((seed.snapshot_id, seed.function_node_id,
            seed.return_site_fact_id, seed.condition_id), &entries_by_return,
            &steps_by_return, &diagrams, &boundaries) {
            Ok(proof) => proof,
            Err(reason) => {
                refuse(&mut refusals, key, reason);
                continue;
            }
        };
        groups.entry(component).or_default().push(PreparedLocalSeed {
            arguments, condition_is_true: condition.is_true(), seed, predecessor_proof,
            admitted: false, refused: BTreeMap::new(),
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
            if let Some(callees) = frontier.by_formal.get(&formal) {
                for callee in callees {
                    let pair = (index, frontier.representatives[callee].1);
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
            let callee = &by_id[&callee_id];
            if !frontier.current(callee) { continue; }
            if pair_work >= max_pair_work {
                capped = true;
                break;
            }
            pair_work += 1;
            if callee.verdict == Verdict::Unknown || callee.boundary_reason.is_some()
                || callee.kind != SummaryFlowKind::Value
                || callee.output_path != OutputPath::ReturnValue.render()
            {
                continue;
            }
            let path = &mut prepared[index];
            let seed = &path.seed;
            let semantic = SemanticFlow::from(callee);
            // A shorter representative replaces this alternative's earlier refusal. Refusals
            // for other semantic alternatives survive even when this path finds a witness.
            path.refused.remove(&semantic);
            let Some(callee_condition) = diagrams.get(&callee.condition_id) else {
                path.refused.insert(semantic, boundaries.get(&callee.condition_id).copied()
                    .unwrap_or(BoundaryReason::MissingEvidence));
                continue;
            };
            if (callee_condition.is_true() && callee.verdict != Verdict::Established)
                || (!callee_condition.is_true() && callee.verdict != Verdict::Conditional)
            {
                continue;
            }
            let arguments = &path.arguments;
            let specialized = if callee_condition.is_true() {
                Vec::new()
            } else {
                match specialize_local_condition(seed, arguments, &local_value_links,
                    &diagrams[&seed.condition_id], callee_condition) {
                    Ok(Some(proof)) => proof,
                    Ok(None) => continue,
                    Err(reason) => { path.refused.insert(semantic, reason); continue; },
                }
            };
            if callee.path_depth >= MAX_LOCAL_PATH_DEPTH {
                path.refused.insert(semantic, BoundaryReason::SummaryDepthLimit);
                continue;
            }
            let mut proof = path.predecessor_proof.clone();
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::CalleeResolution,
                evidence_id: seed.callee_resolution_fact_id,
                condition_id: seed.condition_id,
            });
            for argument in arguments {
                if argument.ordinal == seed.source_argument_ordinal {
                    proof.push(recipe::SummaryFlowProofStep {
                        kind: SummaryFlowStepKind::ReturnSource,
                        evidence_id: seed.source_flow_fact_id, condition_id: seed.condition_id,
                    });
                }
                proof.push(recipe::SummaryFlowProofStep {
                    kind: SummaryFlowStepKind::ArgumentEvaluation,
                    evidence_id: argument.evaluation_fact_id.expect("validated normal evaluation"),
                    condition_id: seed.condition_id,
                });
            }
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
            proof.extend(specialized);
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
            if let Err(error)=cpg_schema::summary_contract::admit_callee_proof(seed.function_node_id,
                &diagrams[&seed.condition_id],callee.path_depth+1,&proof,&local_value_links.by_id,
                |id|by_id.get(&id).and_then(|target|Some(cpg_schema::summary_contract::CalleeProofTarget {
                    function_node_id:target.function_node_id,condition:diagrams.get(&target.condition_id)?,
                    verdict:target.verdict,path_depth:target.path_depth,
                }))) {
                path.refused.insert(semantic,error.reason);
                continue;
            }
            let previous_steps_len = steps.len();
            if !push_finite_path(
                &mut flows,
                &mut steps,
                &pass_steps,
                &mut refusals,
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
            ) { path.refused.insert(semantic, BoundaryReason::SummaryProofLimit); continue; }
            path.admitted = true;
            let flow = flows.last().expect("pushed finite local path").clone();
            if !known_summary_ids.insert(flow.summary_id) {
                flows.pop();
                steps.truncate(previous_steps_len);
                continue;
            }
            let formal = (flow.function_node_id, flow.parameter_node_id);
            by_id.insert(flow.summary_id, flow.clone());
            if !frontier.insert(&flow) { continue; }
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
            }
            if !capped && !path.admitted && path.refused.is_empty() {
                refuse(&mut refusals, key, BoundaryReason::CallTransfer);
            }
            for reason in path.refused.values().copied() { refuse(&mut refusals, key, reason); }
        }
    }
    flows.sort_by_key(|row| row.summary_id);
    steps.sort_by_key(|row| (row.summary_id, row.ordinal));
    refusals.sort_by_key(|row| (row.key(), row.reason));
    refusals.dedup();
    let boundaries = summarize_boundaries(&boundary_candidates, &flows, &refusals);
    let mut coverage = origin_coverage(&boundary_candidates, &flows, &refusals, &boundaries);
    let omitted = unexpanded_witnesses(&flows, &steps);
    let omitted_origins: BTreeSet<_> = flows.iter().filter(|row|omitted.contains(&row.summary_id))
        .map(|row|(row.snapshot_id,row.function_node_id,row.parameter_node_id,
            row.source_flow_fact_id,row.condition_id,row.source_origin_id)).collect();
    for row in &mut coverage {
        row.witnesses_omitted |= row.parameter_node_id.is_some_and(|parameter|
            omitted_origins.contains(&(row.snapshot_id,row.function_node_id,
                parameter,row.source_fact_id,row.condition_id,row.subject_id)));
    }
    FiniteSummaryOutcome { flows, steps, refusals, boundaries, coverage }
}

/// Coverage deliberately reads refusals even where a positive path means there is no
/// summary_boundaries row. A retained witness is not evidence that all alternatives finished.
fn origin_coverage(candidates: &[SummaryBoundaryCandidate], flows: &[SummaryFlowsRow],
    refusals: &[SummaryRefusal], boundaries: &[SummaryBoundariesRow],
) -> Vec<cpg_schema::behavior::SummaryOriginCoverageRow> {
    use cpg_schema::codebook::{InvocationPhase, SummaryChannel};
    let mut witnesses: HashMap<BoundaryKey,BTreeSet<Id>> = HashMap::new();
    let mut reasons: HashMap<BoundaryKey,BTreeSet<BoundaryReason>> = HashMap::new();
    for flow in flows.iter().filter(|f|matches!(f.verdict,Verdict::Established|Verdict::Conditional)) {
        let key=(flow.snapshot_id,flow.function_node_id,flow.parameter_node_id,
            flow.source_flow_fact_id,flow.condition_id,flow.source_origin_id);
        witnesses.entry(key).or_default().insert(flow.summary_id);
        if flow.approximated {reasons.entry(key).or_default().insert(BoundaryReason::OutsideProviderModel);}
    }
    for refusal in refusals {reasons.entry(refusal.key()).or_default().insert(refusal.reason);}
    for b in boundaries {
        reasons.entry((b.snapshot_id,b.function_node_id,b.parameter_node_id,b.source_flow_fact_id,
            b.condition_id,b.source_origin_id)).or_default().insert(b.reason);
    }
    let mut rows = BTreeMap::new();
    for c in candidates {
        let key=(c.snapshot_id,c.function_node_id,c.parameter_node_id,c.source_flow_fact_id,c.condition_id,c.source_origin_id);
        let witness_count=witnesses.get(&key).map_or(0, BTreeSet::len) as i64;
        let explicit=reasons.get(&key);
        let reason=explicit.and_then(|r|r.iter().min_by_key(|r|(refusal_priority(**r),**r))).copied()
            .or(if c.reach_budget {Some(BoundaryReason::BudgetReached)}
                else if c.raw_approximated {Some(BoundaryReason::OutsideProviderModel)}
                else if c.through_call || c.local_through_call || c.upstream_through_call || c.raw_through_call {
                    // The value worklist proves individual finite paths. Until it composes
                    // callee/model channel coverage, a witness cannot close a call origin.
                    Some(BoundaryReason::CallTransfer)
                }
                else if witness_count==0 {Some(BoundaryReason::CallTransfer)} else {None});
        let (subject_kind,subject_id,source_fact_id,parameter_node_id)=cpg_schema::summary_contract::SummarySubject::ValueOrigin {
            origin_id:c.source_origin_id,flow_fact_id:c.source_flow_fact_id,parameter_node_id:c.parameter_node_id,
        }.columns();
        let row=cpg_schema::behavior::SummaryOriginCoverageRow {
            snapshot_id:c.snapshot_id,function_node_id:c.function_node_id,parameter_node_id,
            subject_kind,subject_id,source_fact_id,condition_id:c.condition_id,
            channel:SummaryChannel::Value,phase:InvocationPhase::Call,complete:reason.is_none(),reason,witness_count,
            witnesses_omitted:explicit.is_some_and(|r|r.contains(&BoundaryReason::SummaryProofLimit)),
        };
        rows.entry(key).and_modify(|existing:&mut cpg_schema::behavior::SummaryOriginCoverageRow| {
            // Duplicate observations may weaken coverage, never make it complete by row order.
            existing.reason=existing.reason.into_iter().chain(row.reason)
                .min_by_key(|r|(refusal_priority(*r),*r));
            existing.complete &= row.complete;
            existing.witnesses_omitted |= row.witnesses_omitted;
        }).or_insert(row);
    }
    rows.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry_inputs(mut input:FiniteSummaryInputs) -> FiniteSummaryInputs {
        let mut seeds=Vec::new();
        macro_rules! collect { ($rows:expr) => { for seed in $rows { seeds.push((seed.snapshot_id,seed.function_node_id,seed.return_site_fact_id)); } }; }
        collect!(&input.direct_seeds);collect!(&input.modeled_seeds);collect!(&input.assignment_seeds);
        collect!(&input.local_seeds);collect!(&input.chain_arguments);
        for (snapshot_id,function_node_id,return_site_fact_id) in seeds {
            if !input.entries.iter().any(|e|e.return_site_fact_id==return_site_fact_id) {
                input.entries.push(ReturnEntryStatusesRow {snapshot_id,function_node_id,return_site_fact_id,
                    condition_id:Diagram::always().id(),reason:None,work:1});
            }
        }
        input
    }
    fn finite_flows(input:FiniteSummaryInputs) -> FiniteSummaryOutcome { super::finite_flows(entry_inputs(input)) }
    fn finite_flows_with_pair_limit(input:FiniteSummaryInputs,limit:usize) -> FiniteSummaryOutcome {
        super::finite_flows_with_pair_limit(entry_inputs(input),limit)
    }


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
            entries: vec![ReturnEntryStatusesRow { snapshot_id:id(1),function_node_id:id(2),return_site_fact_id:id(5),condition_id:Diagram::always().id(),reason:None,work:1 }],
            entry_steps: Vec::new(),
            components: Vec::new(),
            direct_seeds: vec![direct_seed()],
            modeled_seeds: Vec::new(),
            chain_arguments: Vec::new(),
            evaluations: Vec::new(),
            assignment_seeds: Vec::new(),
            local_seeds: Vec::new(),
            local_arguments: Vec::new(),
            local_value_links: Vec::new(),
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
    fn positive_origin_does_not_close_siblings_or_an_unfinished_alternative() {
        let mut input=inputs();
        let mut approximate_exit=inputs();
        approximate_exit.direct_seeds[0].approximated=true;
        let approximate_exit=finite_flows(approximate_exit);
        assert!(!approximate_exit.coverage[0].complete);
        assert_eq!(approximate_exit.coverage[0].reason,Some(BoundaryReason::OutsideProviderModel));
        let mut sibling=input.boundary_candidates[0].clone();
        sibling.source_origin_id=id(99);
        sibling.through_call=true;sibling.local_through_call=true;
        input.boundary_candidates.push(sibling);
        let outcome=finite_flows(input.clone());
        assert!(outcome.coverage.iter().find(|c|c.subject_id==id(9)).unwrap().complete);
        assert!(!outcome.coverage.iter().find(|c|c.subject_id==id(99)).unwrap().complete);
        let refusal=SummaryRefusal {snapshot_id:id(1),function_node_id:id(2),parameter_node_id:id(3),
            source_flow_fact_id:id(4),source_origin_id:id(9),condition_id:Diagram::always().id(),
            reason:BoundaryReason::SummaryProofLimit};
        let coverage=origin_coverage(&input.boundary_candidates,&outcome.flows,&[refusal],&outcome.boundaries);
        let positive=coverage.iter().find(|c|c.subject_id==id(9)).unwrap();
        assert_eq!(positive.witness_count,1);
        assert!(!positive.complete && positive.witnesses_omitted);
        assert_eq!(positive.reason,Some(BoundaryReason::SummaryProofLimit));
        let mut called=input.boundary_candidates[0].clone();called.local_through_call=true;
        let called=origin_coverage(&[called],&outcome.flows,&[],&[]);
        assert_eq!(called[0].witness_count,1);
        assert!(!called[0].complete);
        let mut approximate=input.boundary_candidates[0].clone();
        approximate.raw_approximated=true;
        input.boundary_candidates.push(approximate);
        let first=origin_coverage(&input.boundary_candidates,&outcome.flows,&[],&outcome.boundaries);
        input.boundary_candidates.reverse();
        assert_eq!(first,origin_coverage(&input.boundary_candidates,&outcome.flows,&[],&outcome.boundaries));
        assert!(!first.iter().find(|c|c.subject_id==id(9)).unwrap().complete);
    }

    #[test]
    fn one_call_model_needs_a_separate_normal_source_read_witness() {
        let condition = Diagram::always().id();
        let seed = CallEvidence {
            flow_fact_id: id(4), parameter_node_id: id(3),
            source_argument_fact_id: id(12), call_fact_id: id(20),
            pysa_fact_id: id(21), model_id: id(22), rule_id: id(23),
            callee_resolution_fact_id: id(24), argument_count: 2, condition_id: condition,
        };
        let source = ModeledArgumentEvaluationsRow {
            snapshot_id: id(1), candidate_flow_fact_id: id(4), parameter_node_id: id(3),
            pysa_fact_id: id(21), model_id: id(22), rule_id: id(23),
            call_site_node_id: id(19), argument_node_id: id(11),
            argument_fact_id: id(12), ordinal: 1,
            status: ModeledArgumentEvaluationStatus::SourceOperand,
            evidence_id: Some(id(4)), source_normal_evidence_id: Some(id(30)),
            reason: None, condition_id: condition,
        };
        let literal = ModeledArgumentEvaluationsRow {
            argument_node_id: id(13), argument_fact_id: id(14), ordinal: 0,
            status: ModeledArgumentEvaluationStatus::LiteralNormal,
            evidence_id: Some(id(15)), source_normal_evidence_id: None,
            ..source.clone()
        };
        let key = (id(4), id(3), id(21), id(22), id(23));
        let mut candidates = HashMap::from([(key, vec![source.clone(), literal])]);
        let proof = modeled_call_proof(&seed, &candidates).unwrap();
        let source_eval = proof.iter().position(|step| step.kind == SummaryFlowStepKind::ArgumentEvaluation
            && step.evidence_id == id(30)).unwrap();
        assert_eq!(proof[source_eval - 1].kind, SummaryFlowStepKind::RawIdentity);
        assert_eq!(proof[source_eval - 1].evidence_id, id(4));
        candidates.get_mut(&key).unwrap()[0].source_normal_evidence_id = None;
        assert!(modeled_call_proof(&seed, &candidates).is_none());
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
                } else if step == 2 {
                    ModeledArgumentEvaluationStatus::ParameterNameNormal
                } else {
                    ModeledArgumentEvaluationStatus::SourceOperand
                },
                evaluation_evidence_id: if ordinal == 0 { Some(id(50 + step)) }
                    else if step == 2 { Some(id(100 + step)) } else { None },
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
        assert_eq!(proof.iter().filter(|step| step.kind == SummaryFlowStepKind::RawIdentity).count(), 1);
        assert!(proof.iter().any(|step| step.kind == SummaryFlowStepKind::ArgumentEvaluation
            && step.evidence_id == id(102)));
        let mut missing = rows.clone();
        missing.retain(|row| !(row.step == 1 && row.argument_ordinal == 0));
        assert_eq!(modeled_chain_proof(&missing).err(), Some(BoundaryReason::MissingEvidence));
        let mut raising = rows;
        raising[0].evaluation_status = ModeledArgumentEvaluationStatus::Unknown;
        assert_eq!(modeled_chain_proof(&raising).err(), Some(BoundaryReason::MissingEvidence));
        raising[0].evaluation_status = ModeledArgumentEvaluationStatus::LiteralNormal;
        raising[5].evaluation_status = ModeledArgumentEvaluationStatus::Unknown;
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
            binding_complete: true,
            argument_count: 1,
        }
    }

    fn with_local_arguments(mut input: FiniteSummaryInputs) -> FiniteSummaryInputs {
        for seed in &input.local_seeds {
            if !input.local_arguments.iter().any(|row| row.call_fact_id == seed.call_fact_id
                && row.callee_node_id == seed.callee_node_id
                && row.ordinal == seed.source_argument_ordinal) {
                input.local_arguments.push(LocalCallArgument {
                    evaluation_reason: None,
                    call_fact_id: seed.call_fact_id, callee_node_id: seed.callee_node_id,
                    argument_fact_id: seed.source_flow_fact_id, ordinal: seed.source_argument_ordinal,
                    formal_node_id: Some(seed.callee_parameter_node_id),
                    evaluation_fact_id: Some(seed.source_flow_fact_id), boolean_value: None,
                    source_formal_node_id: Some(seed.parameter_node_id),
                });
            }
        }
        input
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
    fn self_recursive_finite_base_stops_at_semantic_fixed_point() {
        let mut input = inputs();
        input.components.push(recursive_member(2, 0));
        let edge = recursive_edge(2, 3, 2, 3, 40, 41, 50);
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.local_seeds.push(edge);
        let result = finite_flows(with_local_arguments(input));
        let mut depths = result.flows.iter().map(|row| row.path_depth).collect::<Vec<_>>();
        depths.sort_unstable();
        assert_eq!(depths, vec![0, 1, 2]);
        assert!(result.flows.iter().all(|row| row.verdict == Verdict::Established));
        assert!(result.boundaries.is_empty());
        assert_eq!(result.steps.iter().filter(|step|
            step.kind == SummaryFlowStepKind::CalleeSummary).count(), 2);
        let coverage=result.coverage.iter().find(|row|row.subject_id==id(41)).unwrap();
        assert!(coverage.witnesses_omitted);
        assert!(!coverage.complete); // Finite witnesses alone do not close source coverage.
    }

    #[test]
    fn shorter_representative_reopens_depth_limited_callers_without_erasing_other_origins() {
        // The low-ordered chain can reach `middle` at depth eight before its shallow
        // alternative is processed. A caller processed between those alternatives must
        // replace the obsolete cap, while the genuinely deep sibling remains refused.
        for salt in 0..16 {
            let mut input=inputs();
            let mut shallow=direct_seed();
            shallow.function_node_id=id(24); shallow.parameter_node_id=id(25);
            shallow.source_flow_fact_id=id(100+salt); shallow.source_origin_id=id(120+salt);
            shallow.return_site_fact_id=id(140+salt); shallow.return_region_fact_id=id(160+salt);
            input.direct_seeds.push(shallow);
            let chain=[2,10,12,14,16,18,20,24];
            for (index,pair) in chain.windows(2).enumerate() {
                input.local_seeds.push(recursive_edge(pair[1],pair[1]+1,pair[0],pair[0]+1,
                    50+index as u8,70+index as u8,90+index as u8));
            }
            input.local_seeds.push(recursive_edge(40,41,24,25,180,181,182));
            input.local_seeds.push(recursive_edge(30,31,40,41,183,184,185));
            // A separate too-deep route is still a real boundary.
            input.local_seeds.push(recursive_edge(42,43,20,21,186,187,188));
            if salt % 2 == 1 {
                // A different semantic result of the same callee remains depth limited
                // for the very same caller, even after its other result recovers.
                input.local_seeds.push(recursive_edge(40,41,42,43,195,196,197));
            }
            input.local_seeds.push(recursive_edge(44,45,42,43,189,190,191));
            input.local_seeds.push(recursive_edge(46,47,44,45,192,193,194));
            for function in chain.into_iter().chain([30,40,42,44,46]) {
                input.components.push(recursive_member(function,0));
            }
            input.boundary_candidates.extend(input.local_seeds.iter().map(recursive_candidate));
            let outcome=finite_flows(with_local_arguments(input));
            assert!(outcome.flows.iter().any(|f|f.function_node_id==id(30) && f.path_depth==2));
            assert_eq!(outcome.refusals.iter().any(|r|r.function_node_id==id(30)
                && r.reason==BoundaryReason::SummaryDepthLimit), salt % 2 == 1);
            assert!(outcome.refusals.iter().any(|r|r.function_node_id==id(46)
                && r.reason==BoundaryReason::SummaryDepthLimit));
        }
    }

    #[test]
    fn semantic_frontier_retains_distinct_origins_and_shorter_proof_progress() {
        let out=finite_flows(inputs());
        let mut row=out.flows[0].clone();
        row.path_depth=8;
        let mut frontier=SemanticFrontier::default();
        assert!(frontier.insert(&row));
        let mut alternate=row.clone(); alternate.summary_id=id(100);
        assert!(!frontier.insert(&alternate));
        alternate.path_depth=1;
        assert!(frontier.insert(&alternate));
        assert!(!frontier.current(&row));
        let mut sibling=alternate.clone(); sibling.source_origin_id=id(101);
        assert!(frontier.insert(&sibling));
        assert_eq!(frontier.representatives.len(),2);
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
            edge.argument_count = 2;
            input.local_arguments.push(LocalCallArgument {
                    evaluation_reason: None,
                call_fact_id: edge.call_fact_id, callee_node_id: edge.callee_node_id,
                argument_fact_id: id(70), ordinal: 1, formal_node_id: Some(id(72)),
                evaluation_fact_id: Some(id(71)), boolean_value: Some(literal),
                source_formal_node_id: None,
            });
            input.local_value_links.push(LocalCallValueLink {
                operation_node_id: edge.callee_node_id, formal_node_id: id(72),
                link_id: id(73), atom: atom.encode(),
            });
            input.boundary_candidates.push(recursive_candidate(&edge));
            input.local_seeds.push(edge);
            input
        };
        let positive = finite_flows(with_local_arguments(make_input(true)));
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
        reversed.local_arguments[0].ordinal = 0;
        let reversed = finite_flows(with_local_arguments(reversed));
        let reversed_path = reversed.flows.iter()
            .find(|row| row.source_origin_id == id(41)).unwrap();
        let evaluations: Vec<_> = reversed.steps.iter()
            .filter(|step| step.summary_id == reversed_path.summary_id
                && step.kind == SummaryFlowStepKind::ArgumentEvaluation)
            .map(|step| step.evidence_id)
            .collect();
        assert_eq!(evaluations, vec![id(71), id(40)],
            "proof evaluations follow syntax order, not the tracked source argument");

        let withheld = finite_flows(with_local_arguments(make_input(false)));
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
        edge.argument_count = 2;
        input.local_arguments.push(LocalCallArgument {
                    evaluation_reason: None,
            call_fact_id: edge.call_fact_id, callee_node_id: edge.callee_node_id,
            argument_fact_id: id(70), ordinal: 1, formal_node_id: Some(id(72)),
            evaluation_fact_id: Some(id(71)), boolean_value: None,
            source_formal_node_id: Some(id(75)),
        });
        input.local_value_links.extend([
            LocalCallValueLink { operation_node_id: edge.callee_node_id,
                formal_node_id: id(72), link_id: id(73), atom: callee_atom.encode() },
            LocalCallValueLink { operation_node_id: edge.function_node_id,
                formal_node_id: id(75), link_id: id(74), atom: caller_atom.encode() },
        ]);
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.local_seeds.push(edge);
        let positive = finite_flows(with_local_arguments(input.clone()));
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
        let withheld = finite_flows(with_local_arguments(input));
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
        let first = finite_flows(with_local_arguments(forward));
        let second = finite_flows(with_local_arguments(reversed));
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

    fn multi_control_inputs(second: bool) -> FiniteSummaryInputs {
        use cpg_schema::condition::EvaluationIdentity;
        let mut input = inputs();
        let atoms: Vec<_> = ["first", "second"].into_iter().enumerate().map(|(index, name)|
            Atom::Truthy { place: name.to_owned() }.evaluated(EvaluationIdentity::Synthetic {
                module: "00".repeat(16), predicate: format!("{:02x}", index + 1).repeat(16),
            })).collect();
        let guard = Diagram::from_atom(&atoms[0]).unwrap()
            .and(&Diagram::from_atom(&atoms[1]).unwrap()).unwrap();
        input.diagrams.insert(guard.id(), guard.clone());
        input.direct_seeds[0].condition_id = guard.id();
        input.boundary_candidates[0].condition_id = guard.id();
        input.components.extend([recursive_member(2, 0), recursive_member(4, 1)]);
        let mut edge = recursive_edge(4, 5, 2, 3, 40, 41, 50);
        edge.argument_count = 3;
        edge.source_argument_ordinal = 1;
        for (index, value) in [true, second].into_iter().enumerate() {
            let formal = id(70 + index as u8);
            input.local_arguments.push(LocalCallArgument {
                    evaluation_reason: None,
                call_fact_id: edge.call_fact_id, callee_node_id: edge.callee_node_id,
                argument_fact_id: id(80 + index as u8), ordinal: (index * 2) as i64,
                formal_node_id: Some(formal), evaluation_fact_id: Some(id(90 + index as u8)),
                boolean_value: Some(value), source_formal_node_id: None,
            });
            input.local_value_links.push(LocalCallValueLink {
                operation_node_id: edge.callee_node_id, formal_node_id: formal,
                link_id: id(100 + index as u8), atom: atoms[index].encode(),
            });
        }
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.local_seeds.push(edge);
        with_local_arguments(input)
    }

    #[test]
    fn multiple_controls_preserve_order_and_require_all_normal_evaluations() {
        let input = multi_control_inputs(true);
        let result = finite_flows(input.clone());
        let local = result.flows.iter().find(|row| row.source_origin_id == id(41)).unwrap();
        let evaluations: Vec<_> = result.steps.iter().filter(|step|
            step.summary_id == local.summary_id && step.kind == SummaryFlowStepKind::ArgumentEvaluation)
            .map(|step| step.evidence_id).collect();
        assert_eq!(evaluations, vec![id(90), id(40), id(91)]);
        assert_eq!(result.steps.iter().filter(|step| step.summary_id == local.summary_id
            && step.kind == SummaryFlowStepKind::CalleeConditionLink).count(), 2);
        let mut shuffled = input.clone();
        shuffled.local_arguments.reverse();
        shuffled.local_value_links.reverse();
        assert_eq!(finite_flows(shuffled).flows, result.flows);
        for variant in 0..5 {
            let mut broken = input.clone();
            match variant {
                0 => { broken.local_arguments.pop(); },
                1 => broken.local_arguments[0].evaluation_fact_id = None,
                2 => broken.local_arguments[0].formal_node_id = Some(id(3)),
                3 => broken.local_arguments[0].ordinal = 1,
                _ => broken.local_seeds[0].binding_complete = false,
            }
            let refused = finite_flows(broken);
            assert!(!refused.flows.iter().any(|row| row.source_origin_id == id(41)));
            assert!(refused.boundaries.iter().any(|row| row.source_origin_id == id(41)));
        }
        for reason in [BoundaryReason::ExpressionDepthLimit, BoundaryReason::ExpressionWorkLimit] {
            let mut capped = input.clone();
            capped.local_arguments[0].evaluation_fact_id = None;
            capped.local_arguments[0].evaluation_reason = Some(reason);
            let result = finite_flows(capped);
            assert!(result.boundaries.iter().any(|row| row.source_origin_id == id(41) && row.reason == reason));
        }
        assert!(!finite_flows(multi_control_inputs(false)).flows.iter()
            .any(|row| row.source_origin_id == id(41)));
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
        let result = finite_flows(with_local_arguments(no_base));
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries.len(), 2);
        assert!(result.boundaries.iter().all(|row| row.reason == BoundaryReason::CallTransfer));

        let mut capped = inputs();
        capped.components.push(recursive_member(2, 0));
        let edge = recursive_edge(2, 3, 2, 3, 40, 41, 50);
        capped.boundary_candidates.push(recursive_candidate(&edge));
        capped.local_seeds.push(edge);
        let result = finite_flows_with_pair_limit(with_local_arguments(capped), 1);
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
            finite_flows_with_pair_limit(with_local_arguments(input), 1)
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
        input.entries[0].condition_id=call_id;
        finite_flows(with_local_arguments(input))
    }

    #[test]
    fn entry_guard_requires_implication_and_preserves_producer_cap_causes() {
        let (path,required)=split_equalities(8);
        let outcome=direct_with_bounded_predecessor(path,required);
        assert_eq!(outcome.refusals[0].reason,BoundaryReason::UnsupportedControlFlow);
        // The completion producer now owns the actual guard conjunction and its cap tests.
        for expected in [BoundaryReason::ConditionNodeLimit,BoundaryReason::ConditionWorkLimit] {
            let mut input=inputs();input.entries[0].reason=Some(expected);
            let outcome = finite_flows(input);
            assert!(outcome.flows.is_empty());
            assert_eq!(outcome.boundaries.len(), 1);
            assert_eq!(outcome.boundaries[0].reason, expected);
            assert_eq!(outcome.refusals[0].reason, expected);
        }
    }

    #[test]
    fn direct_admission_and_independent_preceding_call_control_use_pure_inputs() {
        let outcome = finite_flows(with_local_arguments(inputs()));
        assert_eq!(outcome.flows.len(), 1);
        assert_eq!(outcome.flows[0].verdict, Verdict::Established);
        assert_eq!(outcome.steps.len(), 1);
        assert_eq!(outcome.steps[0].kind, SummaryFlowStepKind::RawIdentity);
        assert!(outcome.boundaries.is_empty());

        let mut withheld = inputs();
        withheld.entries[0].reason=Some(BoundaryReason::UnsupportedControlFlow);
        let outcome = finite_flows(with_local_arguments(withheld));
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

        let outcome = finite_flows(with_local_arguments(input));
        assert_eq!(outcome.flows.len(), 1);
        assert_eq!(outcome.flows[0].source_origin_id, id(9));
        assert_eq!(outcome.boundaries.len(), 1);
        assert_eq!(outcome.boundaries[0].source_origin_id, id(10));
        assert_eq!(outcome.boundaries[0].reason, BoundaryReason::CallTransfer);
    }

    #[test]
    fn entry_proofs_reject_mixed_origins_missing_rows_and_nondense_steps() {
        let mut missing=inputs();missing.entries.clear();
        assert!(super::finite_flows(missing).flows.is_empty());
        for variant in 0..3 {
            let mut input=inputs();
            match variant {
                0=>input.entries[0].function_node_id=id(99),
                1=>input.entries[0].snapshot_id=id(99),
                _=>input.entry_steps.push(ReturnEntryStepsRow {snapshot_id:id(1),return_site_fact_id:id(5),
                    ordinal:1,evidence_id:id(70),kind:SummaryFlowStepKind::CompletionStatement,condition_id:Diagram::always().id()}),
            }
            let out=super::finite_flows(input);
            assert!(out.flows.is_empty());assert_eq!(out.refusals[0].reason,BoundaryReason::MissingEvidence);
        }
    }

    #[test]
    fn shuffled_finalizer_steps_keep_canonical_proof_identity() {
        let first_pass = ReturnPassStep {
            return_site_fact_id: id(5), pass_fact_id: id(50),
            kind: SummaryFlowStepKind::FinalizerPass,
            condition_id: Diagram::always().id(), ordinal: 0,
        };
        let second_pass = ReturnPassStep {
            return_site_fact_id: id(5), pass_fact_id: id(51),
            kind: SummaryFlowStepKind::FinalizerPass,
            condition_id: Diagram::always().id(), ordinal: 1,
        };
        let mut forward = inputs();
        forward.pass_steps.insert(id(5), vec![first_pass.clone(), second_pass.clone()]);
        let mut reverse = inputs();
        reverse.pass_steps.insert(id(5), vec![second_pass, first_pass]);
        let first = finite_flows(with_local_arguments(forward));
        let second = finite_flows(with_local_arguments(reverse));
        assert_eq!(first.flows, second.flows);
        assert_eq!(first.steps, second.steps);
        assert_eq!(first.steps.iter().map(|step| step.evidence_id).collect::<Vec<_>>(),
            [id(4), id(50), id(51)]);
    }

    #[test]
    fn producer_and_native_share_the_proof_limit() {
        let mut input=inputs();
        let limit=cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS;
        let steps:Vec<_>=(0..limit).map(|i| ReturnPassStep {
            return_site_fact_id:id(5),pass_fact_id:id(50),kind:SummaryFlowStepKind::FinalizerPass,
            condition_id:Diagram::always().id(),ordinal:i as i64 }).collect();
        input.pass_steps.insert(id(5),steps[..limit-1].to_vec());
        let admitted=finite_flows(input.clone());
        assert_eq!(admitted.steps.len(),limit);
        assert_eq!(admitted.flows.len(),1);
        input.pass_steps.insert(id(5),steps);
        let refused=finite_flows(input);
        assert!(refused.flows.is_empty());
        assert!(refused.boundaries.iter().any(|r|r.reason==BoundaryReason::SummaryProofLimit));
    }

    #[test]
    fn oversized_local_application_retains_the_named_proof_limit() {
        let mut input=inputs();
        let edge=recursive_edge(30,31,2,3,40,41,50);
        input.components.push(recursive_member(30,1));
        input.boundary_candidates.push(recursive_candidate(&edge));
        input.entry_steps=(0..cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS).map(|ordinal|
            ReturnEntryStepsRow {snapshot_id:edge.snapshot_id,return_site_fact_id:edge.return_site_fact_id,
                ordinal:ordinal as i64,evidence_id:id(70),kind:SummaryFlowStepKind::CompletionStatement,
                condition_id:edge.condition_id}).collect();
        input.local_seeds.push(edge);
        let result=finite_flows(with_local_arguments(input));
        assert!(!result.flows.iter().any(|f|f.source_origin_id==id(41)));
        assert_eq!(result.boundaries.iter().find(|b|b.source_origin_id==id(41)).unwrap().reason,
            BoundaryReason::SummaryProofLimit);
        assert_eq!(result.coverage.iter().find(|c|c.subject_id==id(41)).unwrap().reason,
            Some(BoundaryReason::SummaryProofLimit));
    }

    #[test]
    fn cited_entry_steps_precede_the_return_and_are_canonical_under_shuffle() {
        let mut input=inputs();
        input.entry_steps=(0..2).map(|ordinal|ReturnEntryStepsRow {snapshot_id:id(1),return_site_fact_id:id(5),
            ordinal,evidence_id:id(70+ordinal as u8),kind:SummaryFlowStepKind::CompletionStatement,condition_id:Diagram::always().id()}).collect();
        let out=finite_flows(input.clone());input.entry_steps.reverse();
        let shuffled=finite_flows(input);
        assert_eq!(out.steps,shuffled.steps);assert_eq!(out.flows,shuffled.flows);
        assert_eq!(out.steps.iter().map(|s|s.evidence_id).collect::<Vec<_>>(),[id(70),id(71),id(4)]);
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
            binding_complete: true,
                argument_count: 1,
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
        let result = finite_flows(with_local_arguments(input));
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
            binding_complete: true,
            argument_count: 1,
        });
        bounded.boundary_candidates[0].condition_id = id(200);
        bounded.components.push(SummaryComponentsRow {
            snapshot_id: id(1), component_id: id(2), function_node_id: id(2),
            component_order: 0, member_ordinal: 0, member_count: 1, recursive: false,
        });
        let result = finite_flows(with_local_arguments(bounded));
        assert!(result.flows.is_empty());
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionWorkLimit);

        let mut capped_reach = inputs();
        capped_reach.direct_seeds.clear();
        capped_reach.boundary_candidates[0].reach_budget = true;
        let result = finite_flows(with_local_arguments(capped_reach));
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
            binding_complete: true,
            argument_count: 1,
        });

        let result = finite_flows(with_local_arguments(input));
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
        let result = finite_flows(with_local_arguments(stored));
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
        composed.entries[0].condition_id=call_id;
        let result = finite_flows(with_local_arguments(composed));
        assert!(result.flows.is_empty());
        assert_eq!(result.refusals[0].reason, BoundaryReason::ConditionAtomLimit);
        assert_eq!(result.boundaries[0].reason, BoundaryReason::ConditionAtomLimit);
    }
}
