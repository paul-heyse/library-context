//! Exact endpoint accounting after the ordinary pair queue is permanently exhausted.
//! This index never merges proof inventory, semantic states, costs or queue admissions.
use super::*;
type CallerEndpoint = (Id<EntityRef>, ([u8; 16], [u8; 16]));
type CalleeEndpoint = (Id<EntityRef>, [u8; 16]);
type GroupPair = (Id<CallBindingAttempt>, CallerEndpoint, CalleeEndpoint);
fn caller_endpoint(branch: &WorkBranch) -> CallerEndpoint {
    (branch.descriptor().owner, (*branch.origin.bytes(), *branch.descriptor().input.bytes()))
}
fn callee_endpoint(branch: &WorkBranch) -> CalleeEndpoint {
    (branch.descriptor().owner, *branch.descriptor().output.bytes())
}
pub(super) fn caller_eligible(vocabulary: &Vocabulary, branch: &WorkBranch) -> bool {
    vocabulary.places.get(&branch.descriptor().output)
        .and_then(|p| vocabulary.roots.get(&p.root))
        .is_some_and(|r| matches!(r, PlaceRoot::Occurrence { .. }))
}
struct RemoveProof(ProofId);
impl charged::AdmittedMutation<BTreeSet<ProofId>> for RemoveProof {
    type Output = ();
    fn additional(&self, _: &BTreeSet<ProofId>) -> Result<usize, ModelError> { Ok(0) }
    fn apply(self, proofs: &mut BTreeSet<ProofId>) { proofs.remove(&self.0); }
}
#[derive(Default)]
pub(super) struct EndpointIndex {
    semantics: ChargedMap<ContentHash, BTreeSet<ProofId>>,
    callers: ChargedMap<CallerEndpoint, BTreeSet<ProofId>>,
    callees: ChargedMap<CalleeEndpoint, BTreeSet<ProofId>>,
}
impl EndpointIndex {
    pub(super) fn track(
        &mut self, branch: &WorkBranch, vocabulary: &Vocabulary,
        branches: &ChargedMap<ProofId, WorkBranch>,
        frontier: &summary_worklist::Frontier<ContentHash, ProofId>, charge: &mut StateCharge,
    ) -> Result<(), ModelError> {
        let semantic = branch.semantic();
        // A Pareto replacement has identical semantic endpoints. Retire its old witnesses,
        // retaining every proof in branches/by_owner. Each endpoint keeps a current witness
        // after this atomic local update, even across recursive generations.
        if let Some(old) = self.semantics.remove(charge, &semantic)? {
            for proof in old.iter().copied() {
                if frontier.current(&semantic, proof) {
                    self.semantics.insert_set(charge, semantic, proof)?;
                } else {
                    let old = &branches[&proof];
                    self.callees.mutate(charge, callee_endpoint(old), RemoveProof(proof))?;
                    if caller_eligible(vocabulary, old) {
                        self.callers.mutate(charge, caller_endpoint(old), RemoveProof(proof))?;
                    }
                }
            }
        }
        if frontier.current(&semantic, branch.id()) {
            self.semantics.insert_set(charge, semantic, branch.id())?;
            self.callees.insert_set(charge, callee_endpoint(branch), branch.id())?;
            if caller_eligible(vocabulary, branch) {
                self.callers.insert_set(charge, caller_endpoint(branch), branch.id())?;
            }
        }
        Ok(())
    }
}
/// Completed groups have an actual refused witness, not merely equivalent endpoints.
/// Deferred groups contain only pending/processed exceptions and are reconsidered when a
/// genuinely new proof arrives. Their attempted witnesses remain bounded by ordinary work.
pub(super) struct ExhaustedGroups {
    callers: charged::ChargedSet<(Id<CallBindingAttempt>, CallerEndpoint)>,
    callees: charged::ChargedSet<(Id<CallBindingAttempt>, CalleeEndpoint)>,
    complete: charged::ChargedSet<GroupPair>,
    deferred: ChargedMap<GroupPair, BTreeSet<(bool, ProofId)>>,
    charge: StateCharge,
    pub(super) examined: usize,
}
impl ExhaustedGroups {
    pub(super) fn new(budget: &ResourceBudget) -> Self {
        Self { callers: Default::default(), callees: Default::default(), complete: Default::default(),
            deferred: Default::default(), charge: StateCharge::new(budget, "summary-exhausted-endpoints"), examined: 0 }
    }
    fn group(
        &mut self, context: &EnqueueContext<'_>, key: GroupPair, witness: (ProofId, bool),
        queue: &summary_worklist::WorkQueue<WorkItem>, processed: &charged::ChargedSet<WorkItem>,
        out: &mut SummaryRecords,
    ) -> Result<usize, ModelError> {
        let (new, caller_new) = witness;
        if self.complete.contains(&key) || self.deferred.get(&key).is_some_and(|old| old.contains(&(caller_new, new))) {
            return Ok(0);
        }
        let endpoints = &context.progress.endpoints;
        let opposite = if caller_new { &endpoints.callees[&key.2] } else { &endpoints.callers[&key.1] };
        // Examine at most the actual pending/processed exceptions plus one refused witness.
        // No Cartesian product is constructed and no exception is relabelled as refused.
        for other in opposite {
            let item = if caller_new { WorkItem::Call(key.0, new, *other) } else { WorkItem::Call(key.0, *other, new) };
            self.examined += 1;
            if processed.contains(&item) || queue.contains(&item) { continue; }
            let made = materialize_residual(ResidualContext { discovery: context.discovery, data: context.data,
                invocation: context.invocation, component: context.component_id, progress: context.progress,
                seeds: context.seeds }, item, obligation::ObligationKind::SummaryPairWorkLimit, out)?;
            self.complete.insert(&mut self.charge, key)?;
            self.deferred.remove(&mut self.charge, &key)?;
            return Ok(usize::from(made));
        }
        self.deferred.insert_set(&mut self.charge, key, (caller_new, new))?;
        Ok(0)
    }
    fn deferred_for(
        &self, attempt: Id<CallBindingAttempt>, endpoint: CallerEndpoint, callee: CalleeEndpoint, caller_new: bool,
    ) -> Result<(Vec<GroupPair>, Box<dyn Reservation>), ModelError> {
        let charge = self.charge.budget().expect("exhausted endpoint owner").reserve(
            "summary-exhausted-exceptions", self.deferred.len() * size_of::<GroupPair>())?;
        let keys = self.deferred.keys().filter(|key| key.0 == attempt && if caller_new { key.1 == endpoint } else { key.2 == callee }).copied().collect();
        Ok((keys, charge))
    }
    pub(super) fn calls(
        &mut self, context: &EnqueueContext<'_>, new: ProofId,
        queue: &summary_worklist::WorkQueue<WorkItem>, processed: &charged::ChargedSet<WorkItem>, out: &mut SummaryRecords,
    ) -> Result<usize, ModelError> {
        debug_assert!(queue.exhausted());
        let branch = &context.progress.branches[&new];
        let caller_key = caller_endpoint(branch);
        let callee_key = callee_endpoint(branch);
        let mut made = 0;
        // Calls keep their existing canonical order. Only permanently refused work is grouped;
        // the original queue prefix and mutable proof production are never reordered.
        for index in context.discovery.callers.get(&branch.descriptor().owner).into_iter().flatten() {
            let call = &context.calls[*index];
            if context.component.binary_search(&call.owner).is_err() || !caller_eligible(context.vocabulary, branch) { continue; }
            if self.callers.insert(&mut self.charge, (call.attempt, caller_key))? {
                for (callee, _) in context.progress.endpoints.callees.range((call.callee, [0;16])..=(call.callee, [255;16])) {
                    made += self.group(context, (call.attempt, caller_key, *callee), (new, true), queue, processed, out)?;
                }
            } else {
                let (keys, _charge) = self.deferred_for(call.attempt, caller_key, callee_key, true)?;
                for key in keys { made += self.group(context, key, (new, true), queue, processed, out)?; }
            }
        }
        for index in context.discovery.callees.get(&branch.descriptor().owner).into_iter().flatten() {
            let call = &context.calls[*index];
            if context.component.binary_search(&call.owner).is_err() { continue; }
            if self.callees.insert(&mut self.charge, (call.attempt, callee_key))? {
                for (caller, _) in context.progress.endpoints.callers.range((call.owner, ([0;16],[0;16]))..=(call.owner, ([255;16],[255;16]))) {
                    made += self.group(context, (call.attempt, *caller, callee_key), (new, false), queue, processed, out)?;
                }
            } else {
                let (keys, _charge) = self.deferred_for(call.attempt, caller_key, callee_key, false)?;
                for key in keys { made += self.group(context, key, (new, false), queue, processed, out)?; }
            }
        }
        Ok(made)
    }
}
