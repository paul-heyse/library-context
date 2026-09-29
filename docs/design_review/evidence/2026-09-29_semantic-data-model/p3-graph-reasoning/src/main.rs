//! L1 probe: petgraph page_rank vs the weighted power iteration, Csr parallel edges,
//! condensation weight loss, BDD implication order as an ascent lattice.
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use petgraph::algo::{condensation, page_rank};
use petgraph::csr::Csr;
use petgraph::graph::{DiGraph, NodeIndex};
use biodivine_lib_bdd::{Bdd, BddVariableSet, op_function};
use ascent::ascent_run;
use fixedbitset::FixedBitSet;

fn repo_pagerank(n: usize, arcs: &BTreeMap<(u32, u32), u64>, d: f64, iters: usize) -> Vec<f64> {
    let mut out_w = vec![0u64; n];
    for (&(s, _), &w) in arcs { out_w[s as usize] += w; }
    let u = 1.0 / n as f64;
    let mut r = vec![u; n];
    for _ in 0..iters {
        let dang: f64 = r.iter().zip(&out_w).filter(|(_, w)| **w == 0).map(|(x, _)| *x).sum();
        let mut flow = vec![0.0; n];
        for (&(s, t), &w) in arcs { flow[t as usize] += r[s as usize] * w as f64 / out_w[s as usize] as f64; }
        let base = (1.0 - d) * u + d * dang * u;
        r = flow.iter().map(|f| base + d * f).collect();
    }
    r
}

/// A condition value ordered by implication; join = OR. All values share ONE variable set.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Cond { Known(Bdd), Unknown }
impl PartialOrd for Cond {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;
        match (self, o) {
            (Cond::Unknown, Cond::Unknown) => Some(Equal),
            (Cond::Unknown, _) => Some(Greater),
            (_, Cond::Unknown) => Some(Less),
            (Cond::Known(a), Cond::Known(b)) => Bdd::cmp_implies(a, b),
        }
    }
}
impl ascent::Lattice for Cond {
    fn join_mut(&mut self, other: Self) -> bool {
        let next = match (&*self, &other) {
            (Cond::Unknown, _) => return false,
            (_, Cond::Unknown) => Cond::Unknown,
            (Cond::Known(a), Cond::Known(b)) => match Bdd::binary_op_with_limit(64, a, b, op_function::or) {
                Some(c) => Cond::Known(c),
                None => Cond::Unknown, // refusal is top, never false
            },
        };
        let changed = next != *self;
        *self = next;
        changed
    }
    fn meet_mut(&mut self, _other: Self) -> bool { unimplemented!("not used") }
}

fn main() {
    // --- (1) PageRank: 0->1 twice (parallel), 0->2, 1->2, 2 dangling, 3 isolated.
    let mut g: DiGraph<(), u32, u32> = DiGraph::default();
    for _ in 0..4 { g.add_node(()); }
    let arcs_list = [(0u32, 1u32), (0, 1), (0, 2), (1, 2)];
    for (k, &(s, t)) in arcs_list.iter().enumerate() { g.add_edge(NodeIndex::new(s as usize), NodeIndex::new(t as usize), k as u32); }
    let pg = page_rank(&g, 0.85_f64, 100);
    let mut w: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    for &(s, t) in &arcs_list { *w.entry((s, t)).or_insert(0) += 1; }
    let ours = repo_pagerank(4, &w, 0.85, 100);
    // simple graph (dedup), both
    let mut gs: DiGraph<(), (), u32> = DiGraph::default();
    for _ in 0..4 { gs.add_node(()); }
    for &(s, t) in &[(0u32, 1u32), (0, 2), (1, 2)] { gs.add_edge(NodeIndex::new(s as usize), NodeIndex::new(t as usize), ()); }
    let pg_simple = page_rank(&gs, 0.85_f64, 100);
    let mut ws: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    for &(s, t) in &[(0u32, 1u32), (0, 2), (1, 2)] { ws.insert((s, t), 1); }
    let ours_simple = repo_pagerank(4, &ws, 0.85, 100);
    let maxdiff = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max);
    println!("pagerank multigraph petgraph={pg:.6?} repo={ours:.6?} maxdiff={:.3e}", maxdiff(&pg, &ours));
    println!("pagerank simple     petgraph={pg_simple:.6?} repo={ours_simple:.6?} maxdiff={:.3e}", maxdiff(&pg_simple, &ours_simple));
    // control: a graph with no dangling nodes and no parallel edges (a 3-cycle) agrees by symmetry
    let mut gc: DiGraph<(), (), u32> = DiGraph::default();
    for _ in 0..3 { gc.add_node(()); }
    for &(s, t) in &[(0u32, 1u32), (1, 2), (2, 0)] { gc.add_edge(NodeIndex::new(s as usize), NodeIndex::new(t as usize), ()); }
    let mut wc: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    for &(s, t) in &[(0u32, 1u32), (1, 2), (2, 0)] { wc.insert((s, t), 1); }
    println!("pagerank 3-cycle control maxdiff={:.3e}", maxdiff(&page_rank(&gc, 0.85_f64, 100), &repo_pagerank(3, &wc, 0.85, 100)));

    // --- (2) Csr refuses parallel edges
    let dup: Result<Csr<(), u32, petgraph::Directed, u32>, _> = Csr::from_sorted_edges(&[(0u32, 1u32, 7u32), (0, 1, 8)]);
    println!("csr from_sorted_edges duplicate pair -> is_err={}", dup.is_err());
    let mut c: Csr<(), u32, petgraph::Directed, u32> = Csr::with_nodes(2);
    let a = c.add_edge(0, 1, 7); let b = c.add_edge(0, 1, 8);
    println!("csr add_edge first={a} second={b} edge_count={} weight={:?}", c.edge_count(), c.edges_slice(0));

    // --- (3) condensation make_acyclic=true keeps one weight per SCC pair (update_edge: last wins)
    let mut h: DiGraph<(), u32, u32> = DiGraph::default();
    for _ in 0..3 { h.add_node(()); }
    for (k, &(s, t)) in [(0u32, 1u32), (1, 0), (0, 2), (1, 2)].iter().enumerate() { h.add_edge(NodeIndex::new(s as usize), NodeIndex::new(t as usize), k as u32); }
    let keep = condensation(h.clone(), false);
    let acyc = condensation(h, true);
    let ws_keep: Vec<u32> = keep.edge_weights().copied().collect();
    let ws_acyc: Vec<u32> = acyc.edge_weights().copied().collect();
    println!("condensation make_acyclic=false weights={ws_keep:?}; make_acyclic=true weights={ws_acyc:?}");

    // --- (4) BDD implication order as an ascent lattice (one shared variable set)
    let vars = BddVariableSet::new(&["a", "b"]);
    let va = vars.mk_var_by_name("a");
    let vb = vars.mk_var_by_name("b");
    let ab = va.and(&vb);
    println!("cmp_implies(a&b, a)={:?} cmp_implies(a, b)={:?}", Bdd::cmp_implies(&ab, &va), Bdd::cmp_implies(&va, &vb));
    // reach(node, cond): cond under which node is reached; edges carry guard conditions.
    let edges: Vec<(u32, u32, Cond)> = vec![
        (0, 1, Cond::Known(va.clone())),
        (0, 2, Cond::Known(vb.clone())),
        (1, 3, Cond::Known(vars.mk_true())),
        (2, 3, Cond::Known(vars.mk_true())),
        (3, 1, Cond::Known(vars.mk_true())), // cycle 1->3->1
    ];
    let start = vec![(0u32, Cond::Known(vars.mk_true()))];
    let res = ascent_run! {
        relation edge(u32, u32, Cond) = edges.clone();
        lattice reach(u32, Cond) = start.clone();
        relation derived_by(u32, u32, u32); // (rule id, head node, premise node): one row per distinct binding
        reach(y, Cond::Known(match (c, g) { (Cond::Known(c), Cond::Known(g)) => c.and(g), _ => unreachable!() })),
        derived_by(1, y, x)
            <-- reach(x, ?c @ Cond::Known(_)), edge(x, y, ?g @ Cond::Known(_));
        reach(y, Cond::Unknown) <-- reach(x, Cond::Unknown), edge(x, y, _);
    };
    let mut rows: Vec<(u32, String)> = res.reach.iter().map(|(n, c)| (*n, match c { Cond::Known(b) => b.to_boolean_expression(&vars).to_string(), Cond::Unknown => "unknown".into() })).collect();
    rows.sort();
    let mut d = res.derived_by.clone(); d.sort();
    println!("ascent lattice reach={rows:?} rows={} derived_by={d:?}", res.reach.len());

    // --- (5) FixedBitSet equality includes capacity
    let mut x = FixedBitSet::with_capacity(8); x.insert(1);
    let mut y = FixedBitSet::with_capacity(16); y.insert(1);
    let mut h1 = std::collections::hash_map::DefaultHasher::new(); x.hash(&mut h1);
    println!("fixedbitset same members, capacities 8 vs 16: eq={} ones_eq={}", x == y, x.ones().eq(y.ones()));
}
