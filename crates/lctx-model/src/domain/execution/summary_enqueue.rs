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
struct RemoveCaller(CallerEndpoint);
impl charged::AdmittedMutation<BTreeSet<CallerEndpoint>> for RemoveCaller {
    type Output = ();
    fn additional(&self, _: &BTreeSet<CallerEndpoint>) -> Result<usize, ModelError> { Ok(0) }
    fn apply(self, callers: &mut BTreeSet<CallerEndpoint>) { callers.remove(&self.0); }
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
    deferred_callees: ChargedMap<(Id<CallBindingAttempt>, CalleeEndpoint), BTreeSet<CallerEndpoint>>,
    charge: StateCharge,
    pub(super) examined: usize,
    pub(super) deferred_visited: usize,
}
impl ExhaustedGroups {
    pub(super) fn new(budget: &ResourceBudget) -> Self {
        Self { callers: Default::default(), callees: Default::default(), complete: Default::default(),
            deferred: Default::default(), deferred_callees: Default::default(), charge: StateCharge::new(budget, "summary-exhausted-endpoints"), examined: 0, deferred_visited: 0 }
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
            self.forget_deferred(key)?;
            return Ok(usize::from(made));
        }
        self.defer(key, (caller_new, new))?;
        Ok(0)
    }
    fn defer(&mut self, key: GroupPair, witness: (bool, ProofId)) -> Result<(), ModelError> {
        self.deferred.insert_set(&mut self.charge, key, witness)?;
        self.deferred_callees.insert_set(&mut self.charge, (key.0, key.2), key.1)?;
        Ok(())
    }
    fn forget_deferred(&mut self, key: GroupPair) -> Result<(), ModelError> {
        if self.deferred.remove(&mut self.charge, &key)?.is_some() {
            let reverse = (key.0, key.2);
            self.deferred_callees.mutate(&mut self.charge, reverse, RemoveCaller(key.1))?;
            if self.deferred_callees[&reverse].is_empty() { self.deferred_callees.remove(&mut self.charge, &reverse)?; }
        }
        Ok(())
    }
    /// Caller prefixes use the primary ordered key; reverse lookups use the charged
    /// callee index. Empty unrelated endpoints require neither scanning nor allocation.
    fn next_deferred(
        &mut self, attempt: Id<CallBindingAttempt>, endpoint: CallerEndpoint, callee: CalleeEndpoint,
        caller_new: bool, after: Option<GroupPair>,
    ) -> Option<GroupPair> {
        use std::ops::Bound::{Excluded, Included};
        let next = if caller_new {
            let first = (attempt, endpoint, (callee.0, [0;16]));
            let last = (attempt, endpoint, (callee.0, [255;16]));
            self.deferred.range((after.map_or(Included(first), Excluded), Included(last))).next().map(|(key, _)| *key)
        } else {
            let keys = self.deferred_callees.get(&(attempt, callee))?;
            let first = (endpoint.0, ([0;16],[0;16]));
            let last = (endpoint.0, ([255;16],[255;16]));
            keys.range((after.map_or(Included(first), |key| Excluded(key.1)), Included(last)))
                .next().map(|caller| (attempt, *caller, callee))
        };
        self.deferred_visited += usize::from(next.is_some());
        next
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
                let mut after = None;
                while let Some(key) = self.next_deferred(call.attempt, caller_key, (call.callee, callee_key.1), true, after) {
                    made += self.group(context, key, (new, true), queue, processed, out)?;
                    after = Some(key);
                }
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
                let mut after = None;
                while let Some(key) = self.next_deferred(call.attempt, (call.owner, caller_key.1), callee_key, false, after) {
                    made += self.group(context, key, (new, false), queue, processed, out)?;
                    after = Some(key);
                }
            }
        }
        Ok(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id<R>(n: u8) -> Id<R> { serde_json::from_value(serde_json::json!(vec![n;16])).unwrap() }
    #[test]
    fn disjoint_deferred_regions_require_no_visits_or_allocation_for_completed_endpoints() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        {
            let mut groups = ExhaustedGroups::new(&budget);
            let attempt = id(1);
            let caller = id(2);
            let callee = id(4);
            for n in 1..=128u8 {
                groups.defer((attempt, (caller, ([3;16],[n;16])), (callee,[n;16])), (true,(id(3),id(n)))).unwrap();
            }
            groups.defer((attempt,(caller,([3;16],[7;16])),(callee,[200;16])), (true,(id(3),id(7)))).unwrap();
            groups.defer((attempt,(caller,([3;16],[200;16])),(callee,[7;16])), (false,(id(3),id(7)))).unwrap();
            let retained = budget.reserved();
            let pressure = budget.reserve("deferred-unrelated-singleton", budget.limit() - retained).unwrap();
            for _ in 0..1024 {
                assert!(groups.next_deferred(attempt,(caller,([3;16],[250;16])),(callee,[250;16]),true,None).is_none());
                assert!(groups.next_deferred(attempt,(caller,([3;16],[250;16])),(callee,[250;16]),false,None).is_none());
            }
            assert_eq!(groups.deferred_visited,0);
            assert_eq!(budget.reserved(),retained + pressure.size());
            let mut after = None;
            let mut caller_matches = Vec::new();
            while let Some(key) = groups.next_deferred(attempt,(caller,([3;16],[7;16])),(callee,[7;16]),true,after) {
                caller_matches.push(key); after=Some(key);
            }
            assert_eq!(caller_matches.len(),2);
            after=None;
            let mut callee_matches = Vec::new();
            while let Some(key) = groups.next_deferred(attempt,(caller,([3;16],[7;16])),(callee,[7;16]),false,after) {
                callee_matches.push(key); after=Some(key);
            }
            assert_eq!(callee_matches.len(),2);
            assert_eq!(groups.deferred_visited,4);
            drop(pressure);
            groups.forget_deferred((attempt,(caller,([3;16],[7;16])),(callee,[7;16]))).unwrap();
            let remaining = groups.next_deferred(attempt,(caller,([3;16],[7;16])),(callee,[7;16]),false,None).unwrap();
            assert_eq!(remaining.1.1.1,[200;16]);
            assert!(groups.next_deferred(attempt,(caller,([3;16],[7;16])),(callee,[7;16]),false,Some(remaining)).is_none());
        }
        assert_eq!(budget.reserved(),0);
    }
}
