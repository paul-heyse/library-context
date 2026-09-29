//! rustworkx-core 0.18.1 on petgraph 0.8.3: one petgraph? lexicographic callee-first SCC schedule,
//! iterative DFS with edge weights and Control::Prune.
use std::convert::Infallible;
use petgraph::algo::condensation;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Control;
use rustworkx_core::dag_algo::lexicographical_topological_sort;
use rustworkx_core::traversal::{depth_first_search, DfsEvent};
fn main() {
    // caller -> callee; node weight = canonical id. 1<->2 SCC, 0->1, 3 self-loop, 4->3, 5 alone.
    let ids = [50u64, 10, 20, 40, 30, 60];
    let mut g: DiGraph<u64, u32, u32> = DiGraph::default();
    for &id in &ids { g.add_node(id); }
    for (k, &(s, t)) in [(0usize, 1usize), (1, 2), (2, 1), (3, 3), (4, 3)].iter().enumerate() {
        g.add_edge(NodeIndex::new(s), NodeIndex::new(t), k as u32);
    }
    let c = condensation(g.clone(), true);
    // callee-first: reverse=true sorts the reversed graph; tie key = min member id
    let order = lexicographical_topological_sort(&c, |n| Ok::<u64, Infallible>(*c[n].iter().min().unwrap()), true, None).unwrap();
    let sched: Vec<Vec<u64>> = order.iter().map(|&n| { let mut m = c[n].clone(); m.sort(); m }).collect();
    println!("callee-first schedule by min member id: {sched:?}");
    // iterative DFS: TreeEdge carries the edge weight (arc id); prune below node 1
    let mut events = Vec::new();
    let _: Control<()> = depth_first_search(&g, [NodeIndex::new(0), NodeIndex::new(4)], |e| match e {
        DfsEvent::TreeEdge(u, v, w) => { events.push(format!("tree {}->{} arc{}", u.index(), v.index(), w)); if v.index() == 1 { Control::Prune } else { Control::Continue } }
        DfsEvent::BackEdge(u, v, w) => { events.push(format!("back {}->{} arc{}", u.index(), v.index(), w)); Control::Continue }
        _ => Control::Continue,
    });
    println!("dfs events: {events:?}");
}
