//! Shared bounded progress state for the attributed SCC schedule. Semantic keys and
//! applicability subjects belong to channel adapters; witness IDs never define semantic equality.
use cpg_schema::id::Id;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_PATH_DEPTH: i64 = 8;
pub(crate) const MAX_PROOF_COST: usize = cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Witness {
    pub id: Id,
    pub depth: i64,
    pub cost: usize,
}

/// Keep nondominated depth/cost representatives. Either dimension can enable a caller:
/// replacing the entire frontier with only a shallow or only a cheap proof loses valid paths.
/// At most MAX_PATH_DEPTH+1 representatives survive for one semantic key. Equal-cost witness
/// alternatives are retained by the channel owner, but create no recursive work. Producers
/// submit initial witnesses in canonical order and use the deterministic component queue.
pub(crate) struct Frontier<K, A> {
    representatives: BTreeMap<K, Vec<Witness>>,
    by_subject: BTreeMap<A, BTreeSet<K>>,
}
impl<K, A> Default for Frontier<K, A> {
    fn default() -> Self {
        Self {
            representatives: BTreeMap::new(),
            by_subject: BTreeMap::new(),
        }
    }
}
impl<K: Ord + Clone, A: Ord> Frontier<K, A> {
    pub fn insert(&mut self, key: K, subject: A, witness: Witness) -> bool {
        if !(0..=MAX_PATH_DEPTH).contains(&witness.depth) || witness.cost > MAX_PROOF_COST {
            return false;
        }
        let representatives = self.representatives.entry(key.clone()).or_default();
        if representatives
            .iter()
            .any(|old| old.depth <= witness.depth && old.cost <= witness.cost)
        {
            return false;
        }
        representatives.retain(|old| !(witness.depth <= old.depth && witness.cost <= old.cost));
        representatives.push(witness);
        representatives.sort_by_key(|w| (w.depth, w.cost, w.id));
        self.by_subject.entry(subject).or_default().insert(key);
        true
    }
    pub fn current(&self, key: &K, id: Id) -> bool {
        self.representatives
            .get(key)
            .is_some_and(|rows| rows.iter().any(|w| w.id == id))
    }
    pub fn witnesses(&self, subject: &A) -> impl Iterator<Item = Id> + '_ {
        self.by_subject
            .get(subject)
            .into_iter()
            .flatten()
            .flat_map(|key| self.representatives[key].iter().map(|w| w.id))
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.representatives.len()
    }
}

/// Refusals belong to semantic alternatives, not to individual witness attempts. A later
/// limited witness cannot undo a successful proof of the same alternative; open siblings remain.
pub(crate) struct AlternativeProgress<K> {
    admitted: BTreeSet<K>,
    refused: BTreeMap<K, cpg_schema::codebook::BoundaryReason>,
}
impl<K> Default for AlternativeProgress<K> {
    fn default() -> Self {
        Self {
            admitted: BTreeSet::new(),
            refused: BTreeMap::new(),
        }
    }
}
impl<K: Ord> AlternativeProgress<K> {
    pub fn begin(&mut self, key: &K) {
        self.refused.remove(key);
    }
    pub fn admit(&mut self, key: K) {
        self.refused.remove(&key);
        self.admitted.insert(key);
    }
    pub fn refuse(&mut self, key: K, reason: cpg_schema::codebook::BoundaryReason) {
        if !self.admitted.contains(&key) {
            self.refused.insert(key, reason);
        }
    }
    pub fn unexamined(&self) -> bool {
        self.admitted.is_empty() && self.refused.is_empty()
    }
    pub fn reasons(&self) -> impl Iterator<Item = cpg_schema::codebook::BoundaryReason> + '_ {
        self.refused.values().copied()
    }
}

/// The original per-component pair bound applies both to queued and processed candidates.
/// Stale representatives are popped without charge; callers charge only current semantic work.
#[derive(Default)]
pub(crate) struct ComponentQueue {
    pending: BTreeSet<(usize, Id)>,
    work: usize,
    limit: usize,
}
impl ComponentQueue {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            ..Self::default()
        }
    }
    pub fn insert(&mut self, edge: usize, witness: Id) -> bool {
        let pair = (edge, witness);
        if !self.pending.contains(&pair) && self.pending.len() >= self.limit {
            return false;
        }
        self.pending.insert(pair);
        true
    }
    pub fn pop(&mut self) -> Option<(usize, Id)> {
        self.pending.pop_first()
    }
    pub fn charge(&mut self) -> bool {
        if self.work >= self.limit {
            return false;
        }
        self.work += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u8) -> Id {
        Id([n; 16])
    }
    #[test]
    fn success_recovers_one_alternative_and_survives_a_later_limited_witness() {
        use cpg_schema::codebook::BoundaryReason as R;
        let mut progress = AlternativeProgress::default();
        progress.refuse(1, R::SummaryDepthLimit);
        progress.refuse(2, R::SummaryProofLimit);
        progress.begin(&1);
        progress.admit(1);
        progress.refuse(1, R::SummaryDepthLimit);
        assert_eq!(
            progress.reasons().collect::<Vec<_>>(),
            [R::SummaryProofLimit]
        );
        assert!(!progress.unexamined());
        progress.begin(&2);
        progress.admit(2);
        assert_eq!(progress.reasons().count(), 0);
        assert!(!progress.unexamined());
    }
    #[test]
    fn independent_cost_improvements_and_tradeoffs_reopen_work() {
        let mut frontier = Frontier::default();
        let add = |f: &mut Frontier<u8, u8>, i, d, c| {
            f.insert(
                1,
                2,
                Witness {
                    id: id(i),
                    depth: d,
                    cost: c,
                },
            )
        };
        assert!(add(&mut frontier, 1, 7, 50));
        assert!(!add(&mut frontier, 2, 7, 50));
        assert!(add(&mut frontier, 3, 7, 20));
        assert!(!frontier.current(&1, id(1)));
        assert!(add(&mut frontier, 4, 2, 40));
        assert_eq!(frontier.witnesses(&2).collect::<Vec<_>>(), [id(4), id(3)]);
        assert!(!add(&mut frontier, 5, 8, 60));
        assert!(add(&mut frontier, 6, 1, 10));
        assert_eq!(frontier.witnesses(&2).collect::<Vec<_>>(), [id(6)]);
        assert!(!add(&mut frontier, 7, 9, 1));
        assert!(!add(&mut frontier, 8, 0, 65));
        assert!(
            frontier.insert(
                3,
                2,
                Witness {
                    id: id(9),
                    depth: 1,
                    cost: 10
                }
            ),
            "distinct origin or channel survives"
        );
        assert_eq!(frontier.len(), 2);
    }
    #[test]
    fn component_queue_preserves_parallel_edges_deduplicates_pairs_and_keeps_original_cap() {
        let mut q = ComponentQueue::new(2);
        assert!(q.insert(1, id(2)));
        assert!(q.insert(1, id(2)));
        assert!(q.insert(0, id(2)));
        assert!(!q.insert(2, id(2)));
        assert_eq!(q.pop(), Some((0, id(2))));
        assert!(q.charge());
        assert_eq!(q.pop(), Some((1, id(2))));
        assert!(q.charge());
        assert!(q.insert(0, id(3)));
        assert_eq!(q.pop(), Some((0, id(3))));
        assert!(!q.charge());
    }
}
