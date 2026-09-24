//! Seed selection within the brief budget, and the other choices of what a brief shows (DESIGN
//! §9.4, §9.5, §10.3; ADR-0011's increment-2 amendment). The configured seeds come first. While
//! the budget allows, the eligible public APIs follow by rank: the most direct official-usage calls
//! first (PageRank in the `+pagerank` variant), ties to the smaller id. An API is skipped once its
//! community holds its share of the budget, configured seeds included; an API in no community is
//! capped by nothing. Communities keep the briefs diverse; they no longer choose them (the
//! increment-2 review's U1). Statistical output decides only which briefs exist, never what one
//! says.

use std::collections::BTreeMap;

use cpg_schema::id::{Digest, Id, content_digest};
use serde::Serialize;

/// The selection rule, recorded in the selection invocation's parameters.
pub const RULE: &str = "configured seeds first; then the eligible public APIs (with a docstring \
                        summary and at least one direct official-usage call) by rank, then id, \
                        each skipped once its community holds ceil(budget / community_share) \
                        seeds, until the budget is reached";

/// The pre-registered choices (the increment-2 review's F7): frozen with the analytics parameters
/// in `eval/gold/analytics-freeze.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    pub rule: &'static str,
    pub usage_policy: &'static str,
    /// A community holds at most ⌈budget / community_share⌉ seeds.
    pub community_share: usize,
    /// A shared signature needs a concept whose intent has at least this many attributes
    /// (slice 2.5, D32).
    pub shared_signature_min_attributes: usize,
    /// The other APIs a shared signature names; the rest are counted.
    pub shared_signature_names: usize,
    /// The implications a brief states, the best supported first.
    pub implications: usize,
    /// The co-members a Related line names.
    pub related_names: usize,
}

impl Params {
    pub fn preregistered() -> Self {
        Params {
            rule: RULE,
            usage_policy: crate::ranking::USAGE_POLICY,
            community_share: 3,
            shared_signature_min_attributes: 2,
            shared_signature_names: 5,
            implications: 3,
            related_names: 5,
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("parameters serialize")
    }

    pub fn digest(&self) -> Digest {
        content_digest(self.json().as_bytes())
    }
}

/// The selected seeds, in selection order, at most `budget − configured.len()` of them.
/// `community_of` maps a public API to its community's finding.
pub fn select(
    configured: &[Id],
    budget: usize,
    eligible: &[Id],
    rank: &BTreeMap<Id, f64>,
    community_of: &BTreeMap<Id, Id>,
    params: &Params,
) -> Vec<Id> {
    let score = |n: &Id| rank.get(n).copied().unwrap_or(0.0);
    let cap = budget.div_ceil(params.community_share.max(1)).max(1);
    let mut held: BTreeMap<Id, usize> = BTreeMap::new();
    for c in configured.iter().filter_map(|n| community_of.get(n)) {
        *held.entry(*c).or_insert(0) += 1;
    }
    let mut order: Vec<Id> = eligible
        .iter()
        .copied()
        .filter(|n| !configured.contains(n))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    order.sort_by(|a, b| score(b).total_cmp(&score(a)).then(a.cmp(b)));
    let room = budget.saturating_sub(configured.len());
    let mut out = Vec::new();
    for m in order {
        if out.len() >= room {
            break;
        }
        if let Some(c) = community_of.get(&m) {
            let n = held.entry(*c).or_insert(0);
            if *n >= cap {
                continue;
            }
            *n += 1;
        }
        out.push(m);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Id {
        Id([n; 16])
    }

    #[test]
    fn rank_orders_and_communities_cap() {
        // Community 100: 1 (configured), 2, 3, 4; community 101: 5, 6; 7 in none.
        let community_of: BTreeMap<Id, Id> = [
            (id(1), id(100)),
            (id(2), id(100)),
            (id(3), id(100)),
            (id(4), id(100)),
            (id(5), id(101)),
            (id(6), id(101)),
        ]
        .into_iter()
        .collect();
        let rank: BTreeMap<Id, f64> = [
            (id(2), 50.0),
            (id(3), 40.0),
            (id(4), 30.0),
            (id(5), 20.0),
            (id(6), 20.0),
            (id(7), 1.0),
        ]
        .into_iter()
        .collect();
        let eligible: Vec<Id> = (1..=7).map(id).collect();
        let p = Params::preregistered();
        // Budget 6: a community holds at most 2. Community 100 has the configured 1, so it gives
        // only 2; then 5 and 6 (ties to the smaller id), then 7, which no community caps.
        assert_eq!(
            select(&[id(1)], 6, &eligible, &rank, &community_of, &p),
            vec![id(2), id(5), id(6), id(7)]
        );
        // Budget 3: a community holds at most 1, which the configured 1 already fills for
        // community 100.
        assert_eq!(
            select(&[id(1)], 3, &eligible, &rank, &community_of, &p),
            vec![id(5), id(7)]
        );
        assert!(select(&[id(1)], 1, &eligible, &rank, &community_of, &p).is_empty());
        // No communities: rank alone.
        assert_eq!(
            select(&[id(1)], 4, &eligible, &rank, &BTreeMap::new(), &p),
            vec![id(2), id(3), id(4)]
        );
        // Nothing eligible beyond the configured: nothing selected.
        assert!(select(&[id(1)], 9, &[id(1)], &rank, &community_of, &p).is_empty());
    }
}
