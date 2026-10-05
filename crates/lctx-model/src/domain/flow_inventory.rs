//! Per-use native enumeration and exact retained membership. This does not certify Python completeness.
use super::{
    assertion::*,
    attribution::*,
    charged::{ChargedMap, StateCharge},
    conditions::{Condition, ConditionNode, EvaluationAtom},
    flow::*,
    lexical::LexicalScope,
    source::Occurrence,
    syntax::SubjectBoundary,
    *,
};
use crate::{Assertion, Domain, DomainCode};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_USE_CANDIDATES: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowCandidateKind {
    Bound = 0,
    Undefined = 1,
    Deleted = 2,
    Nested = 3,
    LoopHeader = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="flow_use_candidates", validate=validate_candidate)]
pub struct FlowUseCandidate {
    #[model(key)]
    pub inventory: Id<FlowUseInventoryObservation>,
    #[model(key)]
    pub ordinal: i64,
    pub kind: FlowCandidateKind,
    pub pruned: bool,
    pub loop_expanded: bool,
    pub unattached: bool,
    pub reachability: Option<Id<AssertionQualification>>,
    pub narrowing: Option<Id<AssertionQualification>>,
    pub narrowing_unavailable: bool,
    pub narrowing_precision_lost: bool,
    pub condition_unavailable: bool,
    pub reachability_lost: bool,
    pub mapped_count: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_use_inventory_members")]
pub struct FlowUseInventoryMember {
    #[model(key)]
    pub inventory: Id<FlowUseInventoryObservation>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub reaching: Id<FlowReachingObservation>,
    #[model(key)]
    pub support: Id<FlowReachingSupport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name="flow_use_inventory_observations", validate=validate_inventory, invariant_refs=inventory_invariants_refs)]
#[assertion(support=FlowUseInventorySupport, name="flow_use_inventory_supports", family=FactFamily::Flow, subjects(use_,scope))]
pub struct FlowUseInventoryObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub scope: Id<LexicalScope>,
    #[model(key)]
    pub view: Id<FlowSourceViewObservation>,
    #[model(key)]
    pub native_count: i64,
    #[model(key)]
    pub mapped_count: i64,
    #[model(key)]
    pub candidate_digest: ContentHash,
    #[model(key)]
    pub member_digest: ContentHash,
    #[model(key)]
    pub complete: bool,
}
/// Adapter construction data; no provider indices or synthetic definition IDs are retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateState {
    pub kind: FlowCandidateKind,
    pub pruned: bool,
    pub loop_expanded: bool,
    pub unattached: bool,
    pub reachability: Option<Id<AssertionQualification>>,
    pub narrowing: Option<Id<AssertionQualification>>,
    pub narrowing_unavailable: bool,
    pub narrowing_precision_lost: bool,
    pub condition_unavailable: bool,
    pub reachability_lost: bool,
    pub mapped_count: i64,
}
impl CandidateState {
    fn complete(&self) -> bool {
        !self.pruned
            && !self.loop_expanded
            && !self.unattached
            && !self.condition_unavailable
            && !self.reachability_lost
            && self.mapped_count == 1
    }
    fn encode(&self, key: &mut KeySink) {
        self.kind.encode(key);
        self.pruned.encode(key);
        self.loop_expanded.encode(key);
        self.unattached.encode(key);
        self.reachability.encode(key);
        self.narrowing.encode(key);
        self.narrowing_unavailable.encode(key);
        self.narrowing_precision_lost.encode(key);
        self.condition_unavailable.encode(key);
        self.reachability_lost.encode(key);
        self.mapped_count.encode(key);
    }
}
impl FlowUseCandidate {
    pub fn state(&self) -> CandidateState {
        CandidateState {
            kind: self.kind,
            pruned: self.pruned,
            loop_expanded: self.loop_expanded,
            unattached: self.unattached,
            reachability: self.reachability,
            narrowing: self.narrowing,
            narrowing_unavailable: self.narrowing_unavailable,
            narrowing_precision_lost: self.narrowing_precision_lost,
            condition_unavailable: self.condition_unavailable,
            reachability_lost: self.reachability_lost,
            mapped_count: self.mapped_count,
        }
    }
}
fn candidate_digest(values: &[CandidateState]) -> ContentHash {
    let mut key = KeySink::new("flow-use-candidate-inventory");
    (values.len() as i64).encode(&mut key);
    for (i, value) in values.iter().enumerate() {
        (i as i64).encode(&mut key);
        value.encode(&mut key);
    }
    key.finish()
}
fn member_digest(
    values: &[(i64, Id<FlowReachingObservation>, Id<FlowReachingSupport>)],
) -> ContentHash {
    let mut key = KeySink::new("flow-use-mapped-inventory");
    (values.len() as i64).encode(&mut key);
    for (ordinal, row, support) in values {
        ordinal.encode(&mut key);
        row.encode(&mut key);
        support.encode(&mut key);
    }
    key.finish()
}
impl FlowUseInventoryObservation {
    pub fn new(
        qualification: Id<AssertionQualification>,
        use_: Id<FlowUse>,
        scope: Id<LexicalScope>,
        view: Id<FlowSourceViewObservation>,
        candidates: &[CandidateState],
        members: &[(i64, Id<FlowReachingObservation>, Id<FlowReachingSupport>)],
    ) -> Result<(Self, Vec<FlowUseCandidate>, Vec<FlowUseInventoryMember>), ModelError> {
        if candidates.len() > MAX_USE_CANDIDATES || members.len() > MAX_USE_CANDIDATES {
            return Err(invalid("per-use inventory work bound"));
        }
        let mut members = members.to_vec();
        members.sort();
        members.dedup();
        let row = Self {
            qualification,
            use_,
            scope,
            view,
            native_count: candidates.len() as i64,
            mapped_count: members.len() as i64,
            candidate_digest: candidate_digest(candidates),
            member_digest: member_digest(&members),
            complete: !candidates.is_empty() && candidates.iter().all(CandidateState::complete),
        };
        row.validate()?;
        let candidates: Vec<FlowUseCandidate> = candidates
            .iter()
            .enumerate()
            .map(|(i, v)| FlowUseCandidate {
                inventory: row.id(),
                ordinal: i as i64,
                kind: v.kind,
                pruned: v.pruned,
                loop_expanded: v.loop_expanded,
                unattached: v.unattached,
                reachability: v.reachability,
                narrowing: v.narrowing,
                narrowing_unavailable: v.narrowing_unavailable,
                narrowing_precision_lost: v.narrowing_precision_lost,
                condition_unavailable: v.condition_unavailable,
                reachability_lost: v.reachability_lost,
                mapped_count: v.mapped_count,
            })
            .collect();
        for candidate in &candidates {
            candidate.validate()?;
        }
        let members = members
            .into_iter()
            .map(|(ordinal, reaching, support)| FlowUseInventoryMember {
                inventory: row.id(),
                ordinal,
                reaching,
                support,
            })
            .collect();
        Ok((row, candidates, members))
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_candidate(row: &FlowUseCandidate) -> Result<(), ModelError> {
    if !(0..MAX_USE_CANDIDATES as i64).contains(&row.ordinal)
        || !(0..=MAX_USE_CANDIDATES as i64).contains(&row.mapped_count)
        || row.loop_expanded != (row.kind == FlowCandidateKind::LoopHeader)
        || (row.pruned && row.mapped_count != 0)
        || (row.kind != FlowCandidateKind::LoopHeader && row.mapped_count > 1)
        || (row.kind == FlowCandidateKind::Bound && row.unattached && row.mapped_count != 0)
        || row.condition_unavailable != row.reachability.is_none()
        || row.narrowing_unavailable != row.narrowing.is_none()
    {
        return Err(invalid("inconsistent native use candidate"));
    }
    Ok(())
}
fn validate_inventory(row: &FlowUseInventoryObservation) -> Result<(), ModelError> {
    if !(0..=MAX_USE_CANDIDATES as i64).contains(&row.native_count)
        || !(0..=MAX_USE_CANDIDATES as i64).contains(&row.mapped_count)
        || (row.complete && (row.native_count == 0 || row.native_count != row.mapped_count))
    {
        return Err(invalid("invalid native use inventory cardinality"));
    }
    Ok(())
}
pub(crate) fn inventory_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "flow_use_inventory_replay",
        inputs: vec![
            ValidationInput::of::<ProviderRun>(&["id"]),
            ValidationInput::of::<SubjectBoundary>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<FlowUse>(&["id"]),
            ValidationInput::of::<FlowUseObservation>(&["id"]),
            ValidationInput::of::<FlowUseSupport>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<ConditionNode>(&["id"]),
            ValidationInput::of::<Condition>(&["id"]),
            ValidationInput::of::<EvaluationAtom>(&["id"]),
            ValidationInput::of::<FlowSourceViewObservation>(&["id"]),
            ValidationInput::of::<FlowSourceViewSupport>(&["id"]),
            ValidationInput::of::<ReachingDefinition>(&["id"]),
            ValidationInput::of::<FlowReachingObservation>(&["id"]),
            ValidationInput::of::<FlowReachingSupport>(&["id"]),
            ValidationInput::of::<FlowUseInventoryObservation>(&["id"]),
            ValidationInput::of::<FlowUseInventorySupport>(&["id"]),
            ValidationInput::of::<FlowUseCandidate>(&["inventory", "ordinal"]),
            ValidationInput::of::<FlowUseInventoryMember>(&[
                "inventory",
                "ordinal",
                "reaching",
                "support",
            ]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InventoryCheck {
                charge: StateCharge::new(budget, "flow_use_inventory_replay"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct InventoryCheck {
    charge: StateCharge,
    runs: ChargedMap<Id<ProviderRun>, ProviderRun>,
    boundaries: ChargedMap<Id<SubjectBoundary>, SubjectBoundary>,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    uses: ChargedMap<Id<FlowUse>, FlowUse>,
    use_observations: ChargedMap<Id<FlowUseObservation>, FlowUseObservation>,
    use_supports: ChargedMap<Id<FlowUseSupport>, FlowUseSupport>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    conditions: ChargedMap<Id<Condition>, Condition>,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    atoms: ChargedMap<Id<EvaluationAtom>, EvaluationAtom>,
    views: ChargedMap<Id<FlowSourceViewObservation>, FlowSourceViewObservation>,
    view_supports: ChargedMap<Id<FlowSourceViewSupport>, FlowSourceViewSupport>,
    targets: ChargedMap<Id<ReachingDefinition>, ReachingDefinition>,
    reaching: ChargedMap<Id<FlowReachingObservation>, FlowReachingObservation>,
    supports: ChargedMap<Id<FlowReachingSupport>, FlowReachingSupport>,
    inventories: ChargedMap<Id<FlowUseInventoryObservation>, FlowUseInventoryObservation>,
    inventory_supports: ChargedMap<Id<FlowUseInventorySupport>, FlowUseInventorySupport>,
    candidates: ChargedMap<(Id<FlowUseInventoryObservation>, i64), FlowUseCandidate>,
    members: ChargedMap<Id<FlowUseInventoryMember>, FlowUseInventoryMember>,
}
impl InvariantCheck for InventoryCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        macro_rules! collect {
            ($ty:ty,$field:ident) => {
                if relation == <$ty>::NAME {
                    for row in <$ty>::decode(batch)? {
                        self.$field.insert(&mut self.charge, row.id(), row)?;
                    }
                    return Ok(());
                }
            };
        }
        collect!(ProviderRun, runs);
        collect!(SubjectBoundary, boundaries);
        collect!(Occurrence, occurrences);
        collect!(FlowUse, uses);
        collect!(FlowUseObservation, use_observations);
        collect!(FlowUseSupport, use_supports);
        collect!(AssertionQualification, qualifications);
        collect!(Condition, conditions);
        collect!(ConditionNode, nodes);
        collect!(EvaluationAtom, atoms);
        collect!(FlowSourceViewObservation, views);
        collect!(FlowSourceViewSupport, view_supports);
        collect!(ReachingDefinition, targets);
        collect!(FlowReachingObservation, reaching);
        collect!(FlowReachingSupport, supports);
        collect!(FlowUseInventoryObservation, inventories);
        collect!(FlowUseInventorySupport, inventory_supports);
        collect!(FlowUseInventoryMember, members);
        if relation == FlowUseCandidate::NAME {
            for row in FlowUseCandidate::decode(batch)? {
                row.validate()?;
                self.candidates
                    .insert(&mut self.charge, (row.inventory, row.ordinal), row)?;
            }
            return Ok(());
        }
        Err(invalid("undeclared use inventory input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for row in self.candidates.values() {
            if !self.inventories.contains_key(&row.inventory) {
                return Err(invalid("orphan native use candidate"));
            }
        }
        for row in self.members.values() {
            if !self.inventories.contains_key(&row.inventory) {
                return Err(invalid("orphan mapped use member"));
            }
        }
        let mut charge = self.charge;
        charge.grow(
            (self.members.len()
                + self.inventory_supports.len()
                + self.supports.len()
                + self.view_supports.len()
                + self.use_observations.len())
                * 256,
        )?;
        let mut member_index = BTreeMap::<_, Vec<_>>::new();
        for member in self.members.values() {
            member_index
                .entry(member.inventory)
                .or_default()
                .push(member);
        }
        let mut inventory_support_index = BTreeMap::<_, Vec<_>>::new();
        for support in self.inventory_supports.values() {
            inventory_support_index
                .entry(support.assertion)
                .or_default()
                .push(support);
        }
        let view_runs = self
            .view_supports
            .values()
            .map(|s| (s.assertion, s.run, s.surface))
            .collect::<BTreeSet<_>>();
        let use_frames = self
            .use_observations
            .values()
            .map(|u| (u.use_, u.scope, u.qualification))
            .collect::<BTreeSet<_>>();
        let mut reaching_index = BTreeMap::<_, BTreeSet<_>>::new();
        for support in self.supports.values() {
            if let Some(reach) = self.reaching.get(&support.assertion) {
                reaching_index
                    .entry((reach.use_, support.run, support.surface))
                    .or_default()
                    .insert(reach.id());
            }
        }
        for row in self.inventories.values() {
            row.validate()?;
            charge.grow((row.native_count as usize + row.mapped_count as usize) * 256 + 128)?;
            let candidates = self
                .candidates
                .range((row.id(), 0)..=(row.id(), i64::MAX))
                .map(|(_, c)| c)
                .collect::<Vec<_>>();
            let mut states = Vec::new();
            for (i, c) in candidates.iter().enumerate() {
                if c.ordinal != i as i64 {
                    return Err(invalid("native inventory missing ordinal"));
                }
                states.push(c.state());
            }
            if states.len() as i64 != row.native_count
                || candidate_digest(&states) != row.candidate_digest
                || row.complete
                    != (!states.is_empty() && states.iter().all(CandidateState::complete))
            {
                return Err(invalid("native inventory status/count/digest mismatch"));
            }
            let q = self
                .qualifications
                .get(&row.qualification)
                .ok_or_else(|| invalid("inventory qualification missing"))?;
            let use_ = self
                .uses
                .get(&row.use_)
                .ok_or_else(|| invalid("inventory use missing"))?;
            let site = self
                .occurrences
                .get(&use_.occurrence)
                .ok_or_else(|| invalid("inventory occurrence missing"))?;
            let view = self
                .views
                .get(&row.view)
                .ok_or_else(|| invalid("inventory source view missing"))?;
            let vq = self
                .qualifications
                .get(&view.qualification)
                .ok_or_else(|| invalid("view qualification missing"))?;
            if view.source != site.source
                || vq.scope != q.scope
                || vq.context != q.context
                || !use_frames.contains(&(row.use_, row.scope, row.qualification))
            {
                return Err(invalid("inventory source/view/context/scope mismatch"));
            }
            for candidate in &candidates {
                for formula in [candidate.reachability, candidate.narrowing]
                    .into_iter()
                    .flatten()
                {
                    let cq = self
                        .qualifications
                        .get(&formula)
                        .ok_or_else(|| invalid("candidate formula qualification missing"))?;
                    if cq.scope != q.scope
                        || cq.context != q.context
                        || cq.assumptions != q.assumptions
                    {
                        return Err(invalid(
                            "candidate formula source/context/scope/assumptions mismatch",
                        ));
                    }
                    let condition = self
                        .conditions
                        .get(&cq.condition)
                        .ok_or_else(|| invalid("candidate condition missing"))?;
                    let closure = super::conditions::kernel::closure(condition.root, &self.nodes)?;
                    for node in closure {
                        if let ConditionNode::Branch { atom, .. } = &self.nodes[&node] {
                            let atom = self
                                .atoms
                                .get(atom)
                                .ok_or_else(|| invalid("candidate formula atom missing"))?;
                            let evaluation = self
                                .occurrences
                                .get(&atom.evaluation)
                                .ok_or_else(|| invalid("candidate formula evaluation missing"))?;
                            if atom.context != q.context || evaluation.source != site.source {
                                return Err(invalid(
                                    "candidate formula crosses native source/context",
                                ));
                            }
                        }
                    }
                }
                if let Some(formula) = candidate.reachability {
                    let cq = self
                        .qualifications
                        .get(&formula)
                        .expect("candidate qualification checked");
                    if candidate.reachability_lost != (cq.approximation != Approximation::Exact) {
                        return Err(invalid("candidate reachability precision mismatch"));
                    }
                }
                if let Some(formula) = candidate.narrowing {
                    let cq = self
                        .qualifications
                        .get(&formula)
                        .expect("candidate qualification checked");
                    if candidate.narrowing_precision_lost
                        != (cq.approximation != Approximation::Exact)
                    {
                        return Err(invalid("candidate narrowing precision mismatch"));
                    }
                }
            }
            let inventory_supports = inventory_support_index
                .get(&row.id())
                .cloned()
                .unwrap_or_default();
            if inventory_supports.len() != 1 {
                return Err(invalid("inventory needs its exact provider support"));
            }
            let is = inventory_supports[0];
            if !view_runs.contains(&(row.view, is.run, is.surface)) {
                return Err(invalid("inventory/view provider run mismatch"));
            }
            let mut members = Vec::new();
            let mut observed = BTreeSet::new();
            for member in member_index.get(&row.id()).into_iter().flatten() {
                let reach = self
                    .reaching
                    .get(&member.reaching)
                    .ok_or_else(|| invalid("inventory reaching missing"))?;
                let support = self
                    .supports
                    .get(&member.support)
                    .ok_or_else(|| invalid("inventory reaching support missing"))?;
                let rq = self
                    .qualifications
                    .get(&reach.qualification)
                    .ok_or_else(|| invalid("reaching qualification missing"))?;
                if reach.use_ != row.use_
                    || support.assertion != reach.id()
                    || support.run != is.run
                    || support.surface != is.surface
                    || rq.context != q.context
                    || rq.scope != q.scope
                {
                    return Err(invalid("foreign inventory member or support"));
                }
                if row.complete && (reach.loop_carried || rq.approximation != Approximation::Exact)
                {
                    return Err(invalid("incomplete native reach cannot certify closure"));
                }
                let candidate = self
                    .candidates
                    .get(&(row.id(), member.ordinal))
                    .ok_or_else(|| invalid("mapped member has no native candidate"))?;
                let target = self
                    .targets
                    .get(&reach.target)
                    .ok_or_else(|| invalid("mapped candidate target missing"))?;
                let valid = match candidate.kind {
                    FlowCandidateKind::Bound => matches!(target, ReachingDefinition::Bound { .. }),
                    FlowCandidateKind::Undefined | FlowCandidateKind::Deleted => {
                        matches!(target, ReachingDefinition::Unbound)
                    }
                    FlowCandidateKind::Nested => matches!(target, ReachingDefinition::Nested),
                    FlowCandidateKind::LoopHeader => true,
                };
                if !valid
                    || candidate.reachability != Some(reach.qualification)
                    || reach.loop_carried != (candidate.kind == FlowCandidateKind::LoopHeader)
                {
                    return Err(invalid(
                        "native candidate state disagrees with mapped target",
                    ));
                }
                observed.insert(reach.id());
                members.push((member.ordinal, reach.id(), support.id()));
            }
            members.sort();
            let mut counts = vec![0i64; candidates.len()];
            for member in &members {
                counts[member.0 as usize] += 1;
            }
            for candidate in &candidates {
                if counts[candidate.ordinal as usize] != candidate.mapped_count {
                    return Err(invalid("native candidate mapped cardinality mismatch"));
                }
            }
            let expected = reaching_index
                .get(&(row.use_, is.run, is.surface))
                .cloned()
                .unwrap_or_default();
            if observed != expected
                || members.len() as i64 != row.mapped_count
                || member_digest(&members) != row.member_digest
            {
                return Err(invalid("mapped inventory membership/count/digest mismatch"));
            }
        }
        for support in self.use_supports.values() {
            let observation = self
                .use_observations
                .get(&support.assertion)
                .ok_or_else(|| invalid("inventory use support has no observation"))?;
            let use_ = self
                .uses
                .get(&observation.use_)
                .ok_or_else(|| invalid("inventory use observation missing use"))?;
            let source = self
                .occurrences
                .get(&use_.occurrence)
                .ok_or_else(|| invalid("inventory use occurrence missing"))?
                .source;
            // A runtime-view observation identifies a native enumeration run.
            // Independently authored facts without that view make no enumeration claim.
            let native = self.view_supports.values().any(|v| {
                v.run == support.run
                    && v.surface == support.surface
                    && self
                        .views
                        .get(&v.assertion)
                        .is_some_and(|view| view.source == source)
            });
            let q = self
                .qualifications
                .get(&observation.qualification)
                .ok_or_else(|| invalid("native use qualification missing"))?;
            let refused = self.runs.get(&support.run).is_some_and(|run| {
                self.boundaries.values().any(|b| {
                    b.subject == Some(use_.occurrence)
                        && b.scope == q.scope
                        && b.context == q.context
                        && b.provider == run.provider
                        && b.family == FactFamily::Flow
                        && b.reason == obligation::ObligationKind::ResourceRefused
                })
            });
            if native
                && !refused
                && !self.inventories.values().any(|i| {
                    i.use_ == observation.use_
                        && i.scope == observation.scope
                        && i.qualification == observation.qualification
                        && self.inventory_supports.values().any(|s| {
                            s.assertion == i.id()
                                && s.run == support.run
                                && s.surface == support.surface
                        })
                })
            {
                return Err(invalid("native use is missing its enumeration inventory"));
            }
        }
        Ok(())
    }
}

/// Bounded explanation over fully hydrated, publication-validated inventory. Paging comes afterward.
pub struct UseInventoryExplanation<'a> {
    pub inventory: &'a FlowUseInventoryObservation,
    pub candidates: Vec<&'a FlowUseCandidate>,
    pub members: Vec<&'a FlowUseInventoryMember>,
    _charge: StateCharge,
}
pub fn explain<'a>(
    use_: Id<FlowUse>,
    inventory: &'a FlowUseInventoryObservation,
    candidates: &'a [FlowUseCandidate],
    members: &'a [FlowUseInventoryMember],
    budget: &resources::ResourceBudget,
) -> Result<UseInventoryExplanation<'a>, ModelError> {
    inventory.validate()?;
    if inventory.use_ != use_ {
        return Err(invalid("explanation inventory names a different read"));
    }
    let mut charge = StateCharge::new(budget, "flow_use_inventory_explanation");
    charge.grow((candidates.len() + members.len()) * 128)?;
    let mut selected = candidates
        .iter()
        .filter(|c| c.inventory == inventory.id())
        .collect::<Vec<_>>();
    selected.sort_by_key(|c| c.ordinal);
    let mut mapped = members
        .iter()
        .filter(|m| m.inventory == inventory.id())
        .collect::<Vec<_>>();
    mapped.sort_by_key(|m| (m.ordinal, m.reaching, m.support));
    let states = selected.iter().map(|c| c.state()).collect::<Vec<_>>();
    let pairs = mapped
        .iter()
        .map(|m| (m.ordinal, m.reaching, m.support))
        .collect::<Vec<_>>();
    if inventory.complete != (!states.is_empty() && states.iter().all(CandidateState::complete))
        || selected
            .iter()
            .enumerate()
            .any(|(i, c)| c.ordinal != i as i64)
        || states.len() as i64 != inventory.native_count
        || pairs.len() as i64 != inventory.mapped_count
        || candidate_digest(&states) != inventory.candidate_digest
        || member_digest(&pairs) != inventory.member_digest
    {
        return Err(invalid("explanation requires full inventory hydration"));
    }
    Ok(UseInventoryExplanation {
        inventory,
        candidates: selected,
        members: mapped,
        _charge: charge,
    })
}
/// A view of already publication-replayed singleton outcomes, never fresh entry authority.
pub struct StoredEntryOutcomes<'a> {
    pub witnesses: Vec<&'a conditions::entry::EntryValueWitness>,
    _charge: StateCharge,
}
impl StoredEntryOutcomes<'_> {
    pub fn reason(&self) -> Option<obligation::ObligationKind> {
        self.witnesses
            .is_empty()
            .then_some(obligation::ObligationKind::EntryValueUnknown)
    }
}
impl UseInventoryExplanation<'_> {
    pub fn entry_outcomes<'a>(
        &self,
        use_: &FlowUse,
        context: Id<AnalysisContext>,
        runs: &[Id<ProviderRun>],
        witnesses: &'a [conditions::entry::EntryValueWitness],
        budget: &resources::ResourceBudget,
    ) -> Result<StoredEntryOutcomes<'a>, ModelError> {
        if use_.id() != self.inventory.use_ {
            return Err(invalid("singleton lookup names a different read"));
        }
        if witnesses.len() > MAX_USE_CANDIDATES {
            return Err(invalid("stored per-read singleton outcome bound"));
        }
        let mut charge = StateCharge::new(budget, "flow-inventory-stored-entry-outcomes");
        charge.grow((witnesses.len() + 1) * 128)?;
        let mut selected = witnesses
            .iter()
            .filter(|w| {
                w.access == use_.occurrence
                    && w.context == context
                    && w.inventory == self.inventory.id()
                    && runs.contains(&w.run)
            })
            .collect::<Vec<_>>();
        selected.sort_by_key(|w| w.id());
        Ok(StoredEntryOutcomes {
            witnesses: selected,
            _charge: charge,
        })
    }
}

fn native_traversal<S: Support>(support: &S) -> bool {
    support.attribution().is_some_and(|a| {
        a.fidelity == Fidelity::NativeStructural
            && a.origin == Origin::AnalyzerAssertion
            && a.mode == ExtractionMode::NativeTraversal
    })
}
/// Closure authority for one already attributed native read. Family coverage is separate evidence.
/// Full candidate hydration is checked by the same kernel used by original-evidence explanations.
pub fn complete_native_singleton<'a>(
    data: &'a conditions::entry::EntryData,
    observation: &FlowUseObservation,
    support: &FlowUseSupport,
    reaching: &FlowReachingObservation,
    reaching_support: &FlowReachingSupport,
    budget: &resources::ResourceBudget,
) -> Result<Option<(&'a FlowUseInventoryObservation, &'a FlowUseInventorySupport)>, ModelError> {
    let Some(use_) = data.uses.get(observation.use_) else {
        return Ok(None);
    };
    let Some(site) = data.occurrences.get(use_.occurrence) else {
        return Ok(None);
    };
    let Some(q) = data.qualifications.get(observation.qualification) else {
        return Ok(None);
    };
    if support.assertion != observation.id()
        || !native_traversal(support)
        || !native_traversal(reaching_support)
        || reaching.use_ != use_.id()
        || reaching_support.assertion != reaching.id()
        || reaching_support.run != support.run
        || reaching_support.surface != support.surface
        || reaching.loop_carried
    {
        return Ok(None);
    }
    let Some(run) = data.runs.get(support.run) else {
        return Ok(None);
    };
    let Some(artifact) = data.artifacts.get(site.source) else {
        return Ok(None);
    };
    let Some(rq) = data.qualifications.get(reaching.qualification) else {
        return Ok(None);
    };
    if run.context != q.context
        || run.input != artifact.input
        || rq.context != q.context
        || rq.scope != q.scope
        || rq.approximation != Approximation::Exact
        || q.approximation != Approximation::Exact
    {
        return Ok(None);
    }
    let frame = conditions::entry::SupportFrame {
        access: use_.occurrence,
        context: q.context,
        run: support.run,
    };
    if conditions::entry::exact(data, observation.qualification, frame).is_err()
        || conditions::entry::exact(data, reaching.qualification, frame).is_err()
        || !matches!(
            data.targets.get(reaching.target),
            Some(ReachingDefinition::Bound { .. })
        )
    {
        return Ok(None);
    }
    if !conditions::entry::supported(data, support, frame).unwrap_or(false)
        || !conditions::entry::supported(data, reaching_support, frame).unwrap_or(false)
    {
        return Ok(None);
    }
    let mut inventories = data.inventories.iter().filter(|i| {
        i.use_ == use_.id()
            && i.scope == observation.scope
            && i.qualification == observation.qualification
    });
    let Some(inventory) = inventories.next() else {
        return Ok(None);
    };
    if inventories.next().is_some()
        || !inventory.complete
        || inventory.native_count != 1
        || inventory.mapped_count != 1
    {
        return Ok(None);
    }
    let mut supports = data
        .inventory_supports
        .iter()
        .filter(|s| s.assertion == inventory.id());
    let Some(inventory_support) = supports.next() else {
        return Ok(None);
    };
    if supports.next().is_some()
        || inventory_support.run != support.run
        || inventory_support.surface != support.surface
        || !native_traversal(inventory_support)
        || !conditions::entry::supported(data, inventory_support, frame).unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(view) = data.source_views.get(inventory.view) else {
        return Ok(None);
    };
    let Some(vq) = data.qualifications.get(view.qualification) else {
        return Ok(None);
    };
    if view.source != site.source
        || view.original_content != artifact.content
        || view.byte_len != artifact.byte_len
        || conditions::entry::exact(data, view.qualification, frame).is_err()
        || vq.context != q.context
        || vq.scope != q.scope
        || !data.source_view_supports.iter().any(|s| {
            s.assertion == view.id()
                && s.run == support.run
                && s.surface == support.surface
                && native_traversal(s)
                && conditions::entry::supported(data, s, frame).unwrap_or(false)
        })
    {
        return Ok(None);
    }
    let candidate_count = data
        .inventory_candidates
        .iter()
        .filter(|c| c.inventory == inventory.id())
        .count();
    let member_count = data
        .inventory_members
        .iter()
        .filter(|m| m.inventory == inventory.id())
        .count();
    if candidate_count > MAX_USE_CANDIDATES || member_count > MAX_USE_CANDIDATES {
        return Err(invalid("singleton native inventory work bound"));
    }
    let _hydration = budget.reserve(
        "native-singleton-inventory-hydration",
        (candidate_count + member_count + 1) * 512,
    )?;
    let candidates = data
        .inventory_candidates
        .iter()
        .filter(|c| c.inventory == inventory.id())
        .cloned()
        .collect::<Vec<_>>();
    let members = data
        .inventory_members
        .iter()
        .filter(|m| m.inventory == inventory.id())
        .cloned()
        .collect::<Vec<_>>();
    let explanation = explain(use_.id(), inventory, &candidates, &members, budget)?;
    if explanation.candidates.len() != 1
        || explanation.candidates[0].kind != FlowCandidateKind::Bound
        || explanation.members.len() != 1
        || explanation.members[0].reaching != reaching.id()
        || explanation.members[0].support != reaching_support.id()
    {
        return Ok(None);
    }
    if data.reaching_supports.iter().any(|s| {
        s.run == support.run
            && s.surface == support.surface
            && data
                .reaching
                .get(s.assertion)
                .is_some_and(|r| r.use_ == use_.id() && r.id() != reaching.id())
    }) {
        return Ok(None);
    }
    Ok(Some((inventory, inventory_support)))
}

pub(crate) fn inventory_invariants_refs() -> Vec<&'static str> { vec!["flow_use_inventory_replay"] }
