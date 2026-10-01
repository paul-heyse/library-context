//! Bounded finite-summary progress. Semantic keys exclude proof identity and proof cost.
//! Equal-cost evidence is retained by the publication owner without reopening recursive work.
use super::configuration::SummaryLimits;
use crate::domain::{
    HeapSize, ModelError, charged::StateCharge, obligation::ObligationKind,
    resources::ResourceBudget,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofCost {
    pub depth: u32,
    pub steps: u32,
    pub rank: u32,
}
impl ProofCost {
    pub const SOURCE: Self = Self {
        depth: 0,
        steps: 1,
        rank: 0,
    };
    /// Sequential caller evidence and one nested callee proof share a finite construction DAG.
    pub fn through_call(caller: Self, callee: Self) -> Result<Self, ObligationKind> {
        Ok(Self {
            depth: caller.depth.max(
                callee
                    .depth
                    .checked_add(1)
                    .ok_or(ObligationKind::SummaryDepthLimit)?,
            ),
            steps: caller
                .steps
                .checked_add(callee.steps)
                .and_then(|n| n.checked_add(1))
                .ok_or(ObligationKind::SummaryProofLimit)?,
            rank: caller
                .rank
                .max(callee.rank)
                .checked_add(1)
                .ok_or(ObligationKind::SummaryProofLimit)?,
        })
    }
    pub fn check(self, limits: SummaryLimits) -> Result<(), ObligationKind> {
        if self.depth > limits.depth {
            return Err(ObligationKind::SummaryDepthLimit);
        }
        if self.steps == 0 || self.steps > limits.proof_steps {
            return Err(ObligationKind::SummaryProofLimit);
        }
        Ok(())
    }
    fn dominates(self, other: Self) -> bool {
        self.depth <= other.depth && self.steps <= other.steps
    }
}
impl HeapSize for ProofCost {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Representative<W> {
    pub witness: W,
    pub cost: ProofCost,
}
impl<W: HeapSize> HeapSize for Representative<W> {
    fn heap_bytes(&self) -> usize {
        self.witness.heap_bytes()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Advanced,
    Existing,
    Refused(ObligationKind),
}

/// Pareto representatives remain charged for their whole lifetime. The owner retains every
/// admitted finite proof separately; this index selects only proofs that can improve callers.
pub struct Frontier<K, W> {
    states: BTreeMap<K, Vec<Representative<W>>>,
    limits: SummaryLimits,
    charge: StateCharge,
}
impl<K: Ord + Clone + HeapSize, W: Ord + Copy + HeapSize> Frontier<K, W> {
    pub fn new(limits: SummaryLimits, budget: &ResourceBudget) -> Self {
        Self {
            states: BTreeMap::new(),
            limits,
            charge: StateCharge::new(budget, "summary-pareto-frontier"),
        }
    }
    pub fn insert(
        &mut self,
        key: K,
        representative: Representative<W>,
    ) -> Result<Admission, ModelError> {
        // An unconditional semantic repeat already has an admissible witness. An exhausted
        // redundant derivation cannot manufacture a residual for that established state.
        if self.states.get(&key).is_some_and(|rows| {
            rows.iter()
                .any(|old| old.cost.dominates(representative.cost))
        }) {
            return Ok(Admission::Existing);
        }
        if let Err(reason) = representative.cost.check(self.limits) {
            return Ok(Admission::Refused(reason));
        }
        if !self.states.contains_key(&key) && self.states.len() >= self.limits.members as usize {
            return Ok(Admission::Refused(ObligationKind::SummaryProofLimit));
        }
        let old = self.states.get(&key);
        let retained = old.map_or(0, |rows| {
            rows.iter()
                .filter(|old| !representative.cost.dominates(old.cost))
                .count()
        });
        let bytes = (retained + 1)
            .checked_mul(size_of::<Representative<W>>() + 64)
            .and_then(|n| n.checked_add(size_of::<K>() + key.heap_bytes() + 64))
            .ok_or_else(|| ModelError::Invalid("summary frontier allowance overflow".into()))?;
        let previous_bytes = old.map_or(0, |rows| {
            rows.capacity()
                .saturating_mul(size_of::<Representative<W>>() + 64)
                .saturating_add(size_of::<K>() + key.heap_bytes() + 64)
        });
        self.charge.grow(bytes)?;
        let mut rows = Vec::with_capacity(retained + 1);
        if let Some(old) = old {
            rows.extend(
                old.iter()
                    .copied()
                    .filter(|old| !representative.cost.dominates(old.cost)),
            );
        }
        rows.push(representative);
        rows.sort_by_key(|row| (row.cost.depth, row.cost.steps, row.witness));
        self.states.insert(key, rows);
        self.charge.release(previous_bytes);
        Ok(Admission::Advanced)
    }
    pub fn current(&self, key: &K, witness: W) -> bool {
        self.states
            .get(key)
            .is_some_and(|rows| rows.iter().any(|row| row.witness == witness))
    }
    pub fn representatives(&self, key: &K) -> impl Iterator<Item = &Representative<W>> {
        self.states.get(key).into_iter().flatten()
    }
    pub fn states(&self) -> impl Iterator<Item = (&K, &[Representative<W>])> {
        self.states.iter().map(|(key, rows)| (key, rows.as_slice()))
    }
}

/// Canonical pair queue. Both a new queue admission and each processing step consume work;
/// duplicate pending pairs consume neither allocation nor work. Exhaustion is explicit.
pub struct WorkQueue<T> {
    pending: BTreeSet<T>,
    work: u32,
    limit: u32,
    charge: StateCharge,
}
impl<T: Ord + HeapSize> WorkQueue<T> {
    pub fn new(limit: u32, budget: &ResourceBudget) -> Self {
        Self {
            pending: BTreeSet::new(),
            work: 0,
            limit,
            charge: StateCharge::new(budget, "summary-component-queue"),
        }
    }
    pub fn insert(&mut self, pair: T) -> Result<Result<bool, ObligationKind>, ModelError> {
        if self.pending.contains(&pair) {
            return Ok(Ok(false));
        }
        if self.work >= self.limit {
            return Ok(Err(ObligationKind::SummaryPairWorkLimit));
        }
        self.charge.grow(
            size_of::<T>()
                .saturating_add(pair.heap_bytes())
                .saturating_add(64),
        )?;
        self.pending.insert(pair);
        self.work += 1;
        Ok(Ok(true))
    }
    pub fn pop(&mut self) -> Result<Option<T>, ObligationKind> {
        if self.pending.is_empty() {
            return Ok(None);
        }
        if self.work >= self.limit {
            return Err(ObligationKind::SummaryPairWorkLimit);
        }
        let pair = self.pending.pop_first().expect("nonempty queue");
        self.work += 1;
        self.charge.release(
            size_of::<T>()
                .saturating_add(pair.heap_bytes())
                .saturating_add(64),
        );
        Ok(Some(pair))
    }
    pub fn pending(&self) -> impl Iterator<Item = &T> {
        self.pending.iter()
    }
    pub fn work(&self) -> u32 {
        self.work
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn budget() -> ResourceBudget {
        ResourceBudget::fixed(1 << 20).unwrap()
    }
    fn proof(witness: u64, depth: u32, steps: u32) -> Representative<u64> {
        Representative {
            witness,
            cost: ProofCost {
                depth,
                steps,
                rank: depth,
            },
        }
    }
    #[test]
    fn independent_cost_improvements_and_equal_evidence_do_not_lose_paths() {
        let budget = budget();
        let mut frontier = Frontier::new(SummaryLimits::default(), &budget);
        assert_eq!(
            frontier.insert(1u64, proof(1, 7, 50)).unwrap(),
            Admission::Advanced
        );
        assert_eq!(
            frontier.insert(1, proof(2, 7, 50)).unwrap(),
            Admission::Existing
        );
        assert_eq!(
            frontier.insert(1, proof(3, 7, 20)).unwrap(),
            Admission::Advanced
        );
        assert!(!frontier.current(&1, 1));
        assert_eq!(
            frontier.insert(1, proof(4, 2, 40)).unwrap(),
            Admission::Advanced
        );
        assert_eq!(
            frontier
                .representatives(&1)
                .map(|r| r.witness)
                .collect::<Vec<_>>(),
            vec![4, 3]
        );
        assert_eq!(
            frontier.insert(1, proof(5, 8, 60)).unwrap(),
            Admission::Existing
        );
        assert_eq!(
            frontier.insert(1, proof(6, 1, 10)).unwrap(),
            Admission::Advanced
        );
        assert_eq!(
            frontier.insert(2, proof(7, 1, 10)).unwrap(),
            Admission::Advanced,
            "distinct origin/condition/channel is a distinct key"
        );
        assert!(budget.reserved() > 0);
        drop(frontier);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn repeated_semantics_converge_but_distinct_guard_evaluations_reach_the_depth_boundary() {
        let budget = budget();
        for limit in [0, 1, 2] {
            let limits = SummaryLimits {
                depth: limit,
                ..Default::default()
            };
            let mut frontier = Frontier::new(limits, &budget);
            assert_eq!(
                frontier.insert(0u64, proof(0, 0, 1)).unwrap(),
                Admission::Advanced
            );
            let mut cost = ProofCost::SOURCE;
            for depth in 1..=limit + 1 {
                cost = ProofCost::through_call(ProofCost::SOURCE, cost).unwrap();
                let row = Representative {
                    witness: depth as u64,
                    cost,
                };
                assert_eq!(frontier.insert(0, row).unwrap(), Admission::Existing);
                assert_eq!(
                    frontier.insert(depth as u64, row).unwrap(),
                    if depth <= limit {
                        Admission::Advanced
                    } else {
                        Admission::Refused(ObligationKind::SummaryDepthLimit)
                    }
                );
            }
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn queue_charges_admission_and_processing_and_preserves_parallel_members() {
        let budget = budget();
        let mut queue = WorkQueue::new(4, &budget);
        assert_eq!(queue.insert((1u64, 2u64)).unwrap(), Ok(true));
        assert_eq!(queue.insert((1, 2)).unwrap(), Ok(false));
        assert_eq!(queue.insert((0, 2)).unwrap(), Ok(true));
        assert_eq!(queue.pop(), Ok(Some((0, 2))));
        assert_eq!(queue.pop(), Ok(Some((1, 2))));
        assert_eq!(queue.work(), 4);
        assert_eq!(
            queue.insert((2, 2)).unwrap(),
            Err(ObligationKind::SummaryPairWorkLimit)
        );
        drop(queue);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn resource_refusal_precedes_mutation() {
        let budget = ResourceBudget::fixed(1).unwrap();
        let mut frontier = Frontier::new(SummaryLimits::default(), &budget);
        assert!(frontier.insert(1u64, proof(1, 0, 1)).is_err());
        assert_eq!(frontier.states().count(), 0);
        let mut queue = WorkQueue::new(5, &budget);
        assert!(queue.insert(1u64).is_err());
        assert_eq!(queue.work(), 0);
        assert_eq!(queue.pending().count(), 0);
    }
}
