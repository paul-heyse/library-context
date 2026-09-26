//! W12: compare only the finite-base reachability kernel on one attributed call state.
//! The product's conditions, source proofs and typed refusals are deliberately separate.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use ascent::ascent_run;
use datafrog::{Iteration, Relation};
use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};

type Pair = (u32, u32); // (function, cited base origin)

fn ascent_reach(calls: &[Pair], bases: &[Pair]) -> BTreeSet<Pair> {
    ascent_run! {
        relation call(u32, u32) = calls.to_vec();
        relation base(u32, u32) = bases.to_vec();
        relation reach(u32, u32);
        reach(f, origin) <-- base(f, origin);
        reach(caller, origin) <-- call(caller, callee), reach(callee, origin);
    }
    .reach
    .into_iter()
    .collect()
}

fn datafrog_reach(calls: &[Pair], bases: &[Pair]) -> BTreeSet<Pair> {
    let by_callee: Relation<Pair> = calls.iter().map(|&(caller, callee)| (callee, caller)).collect();
    let mut iteration = Iteration::new();
    let reach = iteration.variable::<Pair>("reach");
    reach.insert(Relation::from_vec(bases.to_vec()));
    while iteration.changed() {
        reach.from_join(&reach, &by_callee, |_, &origin, &caller| (caller, origin));
    }
    reach.complete().elements.into_iter().collect()
}

/// SCC-local deterministic worklist with the same base-origin relation. A budget refusal is
/// explicit: missing tuples after a cap cannot be interpreted as absent paths.
fn scc_reach(
    functions: &[u32], calls: &[Pair], bases: &[Pair], pair_budget: usize,
) -> (BTreeSet<Pair>, BTreeMap<u32, &'static str>, usize) {
    let mut graph = DiGraph::<(), ()>::new();
    for _ in functions { graph.add_node(()); }
    for &(caller, callee) in calls {
        graph.add_edge(NodeIndex::new(caller as usize), NodeIndex::new(callee as usize), ());
    }
    let mut components: Vec<Vec<u32>> = kosaraju_scc(&graph).into_iter().map(|nodes| {
        let mut ids: Vec<_> = nodes.into_iter().map(|n| n.index() as u32).collect();
        ids.sort_unstable();
        ids
    }).collect();
    components.sort();
    let owner: HashMap<u32, usize> = components.iter().enumerate()
        .flat_map(|(index, members)| members.iter().map(move |&f| (f, index))).collect();
    let mut pending: BTreeSet<usize> = (0..components.len()).collect();
    let mut reached: BTreeSet<Pair> = BTreeSet::new();
    let mut work = 0;
    let mut capped = false;
    while !pending.is_empty() {
        let ready = *pending.iter().find(|&&component| calls.iter().all(|&(caller, callee)|
            owner[&caller] != component || owner[&callee] == component
                || !pending.contains(&owner[&callee]))).expect("condensation DAG has a sink");
        let mut local: BTreeSet<Pair> = bases.iter().copied()
            .filter(|(f, _)| owner[f] == ready).collect();
        for &(caller, callee) in calls {
            if owner[&caller] == ready && owner[&callee] != ready {
                for &(_, origin) in reached.iter().filter(|(f, _)| *f == callee) {
                    work += 1;
                    if work > pair_budget { capped = true; break; }
                    local.insert((caller, origin));
                }
            }
            if capped { break; }
        }
        if capped { break; }
        loop {
            let mut next = local.clone();
            for &(caller, callee) in calls {
                if owner[&caller] != ready || owner[&callee] != ready { continue; }
                for &(_, origin) in local.iter().filter(|(f, _)| *f == callee) {
                    work += 1;
                    if work > pair_budget { capped = true; break; }
                    next.insert((caller, origin));
                }
                if capped { break; }
            }
            if capped || next == local { break; }
            local = next;
        }
        if capped { break; }
        reached.extend(local);
        pending.remove(&ready);
    }
    let reason = functions.iter().copied().map(|function| {
        (function, if capped { "budget_reached" }
            else if reached.iter().any(|(f, _)| *f == function) { "proved" }
            else { "call_transfer" })
    }).collect();
    (reached, reason, work)
}

fn main() {
    // 0 -> 1, 1 <-> 2 has a finite base at 1; 3 self-recurses, 4 -> 3; 5 has
    // an independent base. Only 0,1,2,5 have a finite path on this state.
    let functions = [0, 1, 2, 3, 4, 5];
    let calls = [(0, 1), (1, 2), (2, 1), (3, 3), (4, 3)];
    let bases = [(1, 101), (5, 105)];
    let expected: BTreeSet<_> = [(0, 101), (1, 101), (2, 101), (5, 105)].into();
    let ascent = ascent_reach(&calls, &bases);
    let datafrog = datafrog_reach(&calls, &bases);
    let (native, reasons, work) = scc_reach(&functions, &calls, &bases, 100);
    assert_eq!(ascent, expected);
    assert_eq!(datafrog, expected);
    assert_eq!(native, expected);
    assert_eq!(reasons[&3], "call_transfer");
    assert_eq!(reasons[&4], "call_transfer");
    let mut reversed_calls = calls.to_vec();
    reversed_calls.reverse();
    let mut reversed_bases = bases.to_vec();
    reversed_bases.reverse();
    assert_eq!(ascent_reach(&reversed_calls, &reversed_bases), expected);
    assert_eq!(datafrog_reach(&reversed_calls, &reversed_bases), expected);
    assert_eq!(scc_reach(&functions, &reversed_calls, &reversed_bases, 100).0, expected);
    assert!(ascent_reach(&calls, &[]).is_empty());
    assert!(datafrog_reach(&calls, &[]).is_empty());
    assert!(scc_reach(&functions, &calls, &[], 100).0.is_empty());
    let (capped, capped_reasons, capped_work) = scc_reach(&functions, &calls, &bases, 0);
    assert!(capped.len() < expected.len());
    assert!(capped_reasons.values().all(|reason| *reason == "budget_reached"));
    println!("agreement={expected:?} full_work={work} capped_work={capped_work} cap=budget_reached");
}
