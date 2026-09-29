//! Compile + run: which algorithms accept filtered/reversed views over Graph<(), u32, Directed, u32>.
use fixedbitset::FixedBitSet;
use petgraph::algo::{dominators::simple_fast, has_path_connecting, kosaraju_scc, tarjan_scc, toposort, page_rank, DfsSpace, dijkstra};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeFiltered, EdgeRef, NodeFiltered, Reversed};
type G = DiGraph<(), u32, u32>;
fn main() {
    let mut g: G = DiGraph::default();
    for _ in 0..4 { g.add_node(()); }
    // arc ids 0..: 0->1 (kind A), 1->2 (kind B), 0->2 (kind A), 2->3 (kind A), 3->1 (kind B)
    let kinds = ['A', 'B', 'A', 'A', 'B'];
    for (k, &(s, t)) in [(0usize, 1usize), (1, 2), (0, 2), (2, 3), (3, 1)].iter().enumerate() {
        g.add_edge(NodeIndex::new(s), NodeIndex::new(t), k as u32);
    }
    let mut mask = FixedBitSet::with_capacity(4); mask.insert_range(0..3); // hide node 3
    let nf = NodeFiltered(&g, &mask);
    let ef = EdgeFiltered::from_fn(&g, |e: petgraph::graph::EdgeReference<'_, u32, u32>| kinds[*e.weight() as usize] == 'A');
    // NodeFiltered: tarjan, kosaraju, toposort, has_path_connecting, simple_fast, dijkstra
    let a = tarjan_scc(&nf).len();
    let b = kosaraju_scc(&nf).len();
    let c = toposort(&nf, None).map(|v| v.len()).unwrap_or(0);
    let mut space = DfsSpace::new(&nf);
    let d = has_path_connecting(&nf, NodeIndex::new(0), NodeIndex::new(2), Some(&mut space));
    let dom = simple_fast(&nf, NodeIndex::new(0));
    let dj = dijkstra(&nf, NodeIndex::new(0), None, |_| 1u32).len();
    // EdgeFiltered by arc kind (weight = arc id -> side table), plus page_rank (needs NodeCount)
    let e1 = toposort(&ef, None).map(|v| v.len()).unwrap_or(0);
    let pr = page_rank(&ef, 0.85f64, 10).len();
    // Reversed over a filtered view: post-dominators / reverse reachability
    let rev = Reversed(&ef);
    let e2 = has_path_connecting(rev, NodeIndex::new(3), NodeIndex::new(0), None);
    let pdom = simple_fast(Reversed(&nf), NodeIndex::new(2));
    let rn = kosaraju_scc(Reversed(&nf)).len();
    println!("nf tarjan={a} kosaraju={b} toposort_len={c} path0to2={d} idom(2)={:?} dijkstra={dj}", dom.immediate_dominator(NodeIndex::new(2)));
    println!("ef(kind A) toposort_len={e1} page_rank_len={pr} rev_path3to0={e2} postdom idom(0)={:?} rev_nf_scc={rn}", pdom.immediate_dominator(NodeIndex::new(0)));
}
