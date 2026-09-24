//! Seed selection within the brief budget (DESIGN §9.4, §9.5; ADR-0011; slice 2.6): communities
//! choose which entry points get briefs, centrality orders them. The configured seeds come first;
//! while the budget allows, each round takes from every community, in order, its most central
//! public API not yet chosen. Communities holding no chosen seed come first, then by their most
//! central member's rank, then by the community's finding id; ties between members go to the
//! smaller id. Statistical output decides only which briefs exist, never what one says.

use std::collections::{BTreeMap, BTreeSet};

use cpg_schema::id::Id;

/// The selection rule, recorded in the selection invocation's parameters.
pub const RULE: &str = "configured seeds first; then rounds over the communities (those holding no \
                        chosen seed first, then by their most central member), each taking its \
                        most central unchosen documented public API, until the budget is reached";

/// One community: its finding id and its public members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Community {
    pub finding: Id,
    pub members: Vec<Id>,
}

/// The selected seeds, in selection order, at most `budget − configured.len()` of them.
pub fn select(
    configured: &[Id],
    budget: usize,
    communities: &[Community],
    centrality: &BTreeMap<Id, f64>,
) -> Vec<Id> {
    let rank = |n: &Id| centrality.get(n).copied().unwrap_or(0.0);
    let chosen: BTreeSet<Id> = configured.iter().copied().collect();
    // Each community's members, most central first.
    let mut ordered: Vec<(bool, f64, Id, Vec<Id>)> = communities
        .iter()
        .map(|c| {
            let mut members = c.members.clone();
            members.sort_by(|a, b| rank(b).total_cmp(&rank(a)).then(a.cmp(b)));
            let seeded = members.iter().any(|m| chosen.contains(m));
            let top = members.first().map_or(0.0, rank);
            (seeded, top, c.finding, members)
        })
        .collect();
    ordered.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)).then(a.2.cmp(&b.2)));
    let mut chosen = chosen;
    let mut out = Vec::new();
    let room = budget.saturating_sub(configured.len());
    let mut cursors = vec![0usize; ordered.len()];
    while out.len() < room {
        let mut progressed = false;
        for (k, (_, _, _, members)) in ordered.iter().enumerate() {
            if out.len() >= room {
                break;
            }
            while cursors[k] < members.len() && chosen.contains(&members[cursors[k]]) {
                cursors[k] += 1;
            }
            if let Some(&m) = members.get(cursors[k]) {
                chosen.insert(m);
                out.push(m);
                cursors[k] += 1;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
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
    fn unseeded_communities_come_first_and_centrality_orders_members() {
        let communities = vec![
            Community {
                finding: id(100),
                members: vec![id(1), id(2), id(3)],
            },
            Community {
                finding: id(101),
                members: vec![id(4), id(5)],
            },
            Community {
                finding: id(102),
                members: vec![id(6), id(7)],
            },
        ];
        let centrality: BTreeMap<Id, f64> = [
            (id(1), 0.5),
            (id(2), 0.4),
            (id(3), 0.1),
            (id(4), 0.2),
            (id(5), 0.3),
            (id(6), 0.05),
            (id(7), 0.01),
        ]
        .into_iter()
        .collect();
        // Seed 1 is configured: communities 101 (top 0.3) and 102 (top 0.05) come first, then
        // 100; each gives its most central unchosen member per round.
        assert_eq!(
            select(&[id(1)], 5, &communities, &centrality),
            vec![id(5), id(6), id(2), id(4)]
        );
        // The budget binds; no room, no selection.
        assert_eq!(select(&[id(1)], 2, &communities, &centrality), vec![id(5)]);
        assert!(select(&[id(1)], 1, &communities, &centrality).is_empty());
        // Every member chosen: the rounds stop.
        assert_eq!(select(&[id(1)], 100, &communities, &centrality).len(), 6);
    }
}
