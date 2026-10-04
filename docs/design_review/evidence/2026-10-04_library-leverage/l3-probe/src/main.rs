//! L3 probes (library-leverage review, 2026-10-04). Each section prints facts; no assertion hides output.
use biodivine_lib_bdd::{op_function, Bdd, BddVariable, BddVariableSet};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn random_dnf(ctx: &BddVariableSet, rng: &mut Lcg, nvars: usize, terms: usize, lits: usize) -> Bdd {
    let mut f = ctx.mk_false();
    for _ in 0..terms {
        let mut t = ctx.mk_true();
        for _ in 0..lits {
            let v = BddVariable::from_index((rng.next() as usize) % nvars);
            let pol = rng.next() % 2 == 0;
            t = t.and(&ctx.mk_literal(v, pol));
        }
        f = f.or(&t);
    }
    f
}

fn p1_biodivine() {
    println!("== P1 biodivine 0.6.3: bounded exists/substitute through limit-taking ops");
    let nvars = 24;
    let ctx = BddVariableSet::new_anonymous(nvars as u16);
    let mut rng = Lcg(7);
    let (mut agree, mut total, mut none_agree, mut refusals) = (0, 0, 0, 0);
    let (mut tasks_le_product, mut tasks_total) = (0, 0);
    let mut sample_ratio = vec![];
    for _ in 0..200 {
        let f = random_dnf(&ctx, &mut rng, nvars, 16, 5);
        for v in f.support_set() {
            total += 1;
            // bespoke form: restrict both cofactors, then a capped or
            let low = f.restrict(&[(v, false)]);
            let high = f.restrict(&[(v, true)]);
            let ours = Bdd::binary_op_with_limit(50_000, &low, &high, op_function::or);
            // library form: one fused flip apply with the same result-node limit
            let lib = Bdd::fused_binary_flip_op_with_limit(50_000, (&f, None), (&f, Some(v)), None, op_function::or);
            if ours.as_ref() == lib.as_ref() && lib.as_ref() == Some(&f.var_exists(v)) { agree += 1; }
            // tight limit: refusal decisions must coincide (limit counts result nodes in both)
            let cap = f.var_exists(v).size().saturating_sub(1).max(1);
            let o2 = Bdd::binary_op_with_limit(cap, &low, &high, op_function::or);
            let l2 = Bdd::fused_binary_flip_op_with_limit(cap, (&f, None), (&f, Some(v)), None, op_function::or);
            if o2.is_none() { refusals += 1; }
            if o2.is_none() == l2.is_none() { none_agree += 1; }
            // work: check_fused_binary_flip_op counts low-level tasks; compare to the |low|x|high| preflight
            let (_nonempty, tasks) = Bdd::check_fused_binary_flip_op(usize::MAX, (&f, None), (&f, Some(v)), None, op_function::or).unwrap();
            let product = low.size() * high.size();
            tasks_total += 1;
            if tasks <= product { tasks_le_product += 1; }
            if sample_ratio.len() < 5 { sample_ratio.push((tasks, product, f.size())); }
        }
    }
    println!("exists one var: fused_flip_with_limit == restrict+or == var_exists: {agree}/{total}");
    println!("tight limit (result size - 1): refusal agreement {none_agree}/{total}, refusals {refusals}");
    println!("check_fused_binary_flip_op tasks <= |low|*|high|: {tasks_le_product}/{tasks_total}; samples (tasks, product, |f|): {sample_ratio:?}");
    // check limit on tasks gives a work refusal
    let f = random_dnf(&ctx, &mut Lcg(99), nvars, 16, 5);
    let v = *f.support_set().iter().min().unwrap();
    let (_, tasks) = Bdd::check_fused_binary_flip_op(usize::MAX, (&f, None), (&f, Some(v)), None, op_function::or).unwrap();
    println!("check_fused_binary_flip_op(limit=tasks-1) -> {:?}; limit=tasks -> is_some {}", Bdd::check_fused_binary_flip_op(tasks - 1, (&f, None), (&f, Some(v)), None, op_function::or).map(|x| x.1), Bdd::check_fused_binary_flip_op(tasks, (&f, None), (&f, Some(v)), None, op_function::or).is_some());
    // multi-variable exists: library exists(&vars) (unbounded) vs iterated capped fused flip
    let mut rng = Lcg(3);
    let (mut m_agree, mut m_total) = (0, 0);
    for _ in 0..100 {
        let f = random_dnf(&ctx, &mut rng, nvars, 16, 5);
        let mut vars: Vec<_> = f.support_set().into_iter().collect();
        vars.sort();
        vars.truncate(4);
        let mut g = f.clone();
        for v in &vars {
            g = Bdd::fused_binary_flip_op_with_limit(50_000, (&g, None), (&g, Some(*v)), None, op_function::or).unwrap();
        }
        m_total += 1;
        if g == f.exists(&vars) { m_agree += 1; }
    }
    println!("exists 4 vars: iterated capped fused flip == Bdd::exists: {m_agree}/{m_total}");
    // substitution v := g (v not in g) through limit-taking ops vs library substitute (unbounded)
    let mut rng = Lcg(11);
    let (mut s_agree, mut s_total) = (0, 0);
    for _ in 0..100 {
        let f = random_dnf(&ctx, &mut rng, nvars, 12, 4);
        let v = BddVariable::from_index(0);
        if !f.support_set().contains(&v) { continue; }
        let g = loop {
            let g = random_dnf(&ctx, &mut rng, nvars, 3, 3);
            if !g.support_set().contains(&v) { break g; }
        };
        let hi = f.restrict(&[(v, true)]);
        let lo = f.restrict(&[(v, false)]);
        let yes = Bdd::binary_op_with_limit(50_000, &g, &hi, op_function::and).unwrap();
        let no = Bdd::binary_op_with_limit(50_000, &g, &lo, op_function::and_not).map(|_| ()).and(Bdd::binary_op_with_limit(50_000, &lo, &g, op_function::and_not)).unwrap();
        let ours = Bdd::binary_op_with_limit(50_000, &yes, &no, op_function::or).unwrap();
        // fused: iff(v,g) and f, then exists v -- with limits
        let lit = ctx.mk_literal(v, true);
        let iff = Bdd::binary_op_with_limit(50_000, &lit, &g, op_function::iff).unwrap();
        let conj = Bdd::binary_op_with_limit(50_000, &iff, &f, op_function::and).unwrap();
        let fused = Bdd::fused_binary_flip_op_with_limit(50_000, (&conj, None), (&conj, Some(v)), None, op_function::or).unwrap();
        s_total += 1;
        if ours == f.substitute(v, &g) && fused == ours { s_agree += 1; }
    }
    println!("substitute v:=g: cofactor form == iff/and/flip-exists form == Bdd::substitute: {s_agree}/{s_total}");
}

fn p2_rustworkx_lex_topo() {
    use petgraph::graph::{DiGraph, NodeIndex};
    use rustworkx_core::dag_algo::lexicographical_topological_sort;
    use std::convert::Infallible;
    println!("== P2 rustworkx-core 0.18.1 lexicographical_topological_sort");
    // DAG: names inserted out of order; expect min-name-first among ready nodes
    let mut g: DiGraph<&str, ()> = DiGraph::new();
    let n: Vec<NodeIndex> = ["d", "b", "a", "c", "e"].iter().map(|s| g.add_node(*s)).collect();
    g.add_edge(n[2], n[4], ()); // a -> e
    g.add_edge(n[1], n[4], ()); // b -> e
    g.add_edge(n[0], n[3], ()); // d -> c
    g.add_edge(n[0], n[3], ()); // parallel d -> c
    let order = lexicographical_topological_sort(&g, |i| Ok::<_, Infallible>(g[i]), false, None).unwrap();
    println!("DAG order by name key: {:?}", order.iter().map(|i| g[*i]).collect::<Vec<_>>());
    // equal keys: tie broken by NodeIndex
    let order = lexicographical_topological_sort(&g, |_| Ok::<_, Infallible>(0u8), false, None).unwrap();
    println!("all-equal keys -> NodeIndex order: {:?}", order.iter().map(|i| g[*i]).collect::<Vec<_>>());
    // cycle: b <-> c plus a -> b, d isolated
    let mut h: DiGraph<&str, ()> = DiGraph::new();
    let m: Vec<NodeIndex> = ["a", "b", "c", "d"].iter().map(|s| h.add_node(*s)).collect();
    h.add_edge(m[0], m[1], ());
    h.add_edge(m[1], m[2], ());
    h.add_edge(m[2], m[1], ());
    let r = lexicographical_topological_sort(&h, |i| Ok::<_, Infallible>(h[i]), false, None);
    println!("cyclic graph: {:?} (node_count {})", r.as_ref().map(|v| v.iter().map(|i| h[*i]).collect::<Vec<_>>()).map_err(|e| e.to_string()), h.node_count());
    // petgraph toposort on the same cyclic graph for contrast
    println!("petgraph toposort on cyclic graph is_err: {}", petgraph::algo::toposort(&h, None).is_err());
}

fn p3_petgraph_condensation() {
    use petgraph::algo::condensation;
    use petgraph::graph::DiGraph;
    println!("== P3 petgraph 0.8.3 condensation parallel arcs");
    let mut g: DiGraph<&str, u32> = DiGraph::new();
    let a = g.add_node("a");
    let b = g.add_node("b");
    let c = g.add_node("c");
    g.add_edge(a, b, 1);
    g.add_edge(b, a, 2);
    g.add_edge(a, c, 3);
    g.add_edge(a, c, 4);
    g.add_edge(b, c, 5);
    let keep = condensation(g.clone(), false);
    let mut arcs: Vec<_> = keep.raw_edges().iter().map(|e| (keep[e.source()].clone(), keep[e.target()].clone(), e.weight)).collect();
    arcs.sort();
    println!("make_acyclic=false: nodes {:?}; arcs {:?}", keep.node_weights().collect::<Vec<_>>(), arcs);
    let acyc = condensation(g, true);
    println!("make_acyclic=true: arcs {:?}", acyc.raw_edges().iter().map(|e| e.weight).collect::<Vec<_>>());
}

mod p4 {
    use ascent::ascent;
    use ascent::Lattice;
    use std::collections::BTreeSet;

    /// Pareto antichain of (depth, steps) under component-wise dominance (smaller is better).
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Front(pub BTreeSet<(u32, u32)>);
    fn dominated(x: &(u32, u32), y: &(u32, u32)) -> bool { y.0 <= x.0 && y.1 <= x.1 }
    fn minimal(s: BTreeSet<(u32, u32)>) -> BTreeSet<(u32, u32)> {
        s.iter().filter(|x| !s.iter().any(|y| y != *x && dominated(x, y))).copied().collect()
    }
    impl PartialOrd for Front {
        // a <= b iff every element of b is weakly dominated by an element of a (a is "better or equal" is top)
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            use std::cmp::Ordering::*;
            let le = other.0.iter().all(|y| self.0.iter().any(|x| dominated(y, x)));
            let ge = self.0.iter().all(|x| other.0.iter().any(|y| dominated(x, y)));
            match (le, ge) { (true, true) => Some(Equal), (true, false) => Some(Less), (false, true) => Some(Greater), _ => None }
        }
    }
    impl Lattice for Front {
        fn meet_mut(&mut self, _other: Self) -> bool { unimplemented!("meet unused by ascent lattice relations") }
        fn join_mut(&mut self, other: Self) -> bool {
            let joined = minimal(self.0.union(&other.0).copied().collect());
            let changed = joined != self.0;
            self.0 = joined;
            changed
        }
    }

    ascent! {
        #![generate_run_timeout]
        struct Summary;
        relation edge(u32, u32, u32);           // (from, to, steps)
        relation base(u32);
        lattice best(u32, Front);               // per node: nondominated (depth, steps)
        relation refused(u32, u32);             // derivation refused by a fallible side computation
        best(n, Front([(0u32, 0u32)].into_iter().collect())) <-- base(n);
        best(x, Front(f.0.iter().filter(|c| c.0 < 6).map(|c| (c.0 + 1, c.1 + s)).collect())) <--
            edge(x, y, s), best(y, f), if f.0.iter().any(|c| c.0 < 6) && conj_ok(*x, *y);
        refused(x, y) <-- edge(x, y, _), best(y, _), if !conj_ok(*x, *y);
    }
    fn conj_ok(x: u32, y: u32) -> bool { !(x == 3 && y == 1) }

    pub fn run() {
        println!("== P4 ascent 0.8.1 (default-features=false): Pareto antichain lattice, fallible rule, run_timeout");
        let mut p = Summary::default();
        p.base = vec![(0,)];
        // cycle 1<->2, two routes 1->0 (cheap deep vs. expensive shallow)
        p.edge = vec![(1, 0, 10), (2, 0, 1), (1, 2, 1), (2, 1, 1), (3, 1, 1), (3, 2, 5)];
        let finished = p.run_timeout(std::time::Duration::MAX);
        let mut best: Vec<_> = p.best.iter().map(|(n, f)| (*n, f.0.iter().copied().collect::<Vec<_>>())).collect();
        best.sort();
        println!("finished={finished}; best: {best:?}; refused: {:?}; scc_iters {:?}", p.refused, p.scc_iters);
        let mut q = Summary::default();
        q.base = vec![(0,)];
        q.edge = p.edge.clone();
        let finished = q.run_timeout(std::time::Duration::ZERO);
        println!("run_timeout(ZERO): finished={finished}; best rows {}; scc_iters {:?}", q.best.len(), q.scc_iters);
    }
}

fn p5_graphops_pagerank() {
    use graphops::{pagerank_weighted_run, Graph, PageRankConfig, WeightedGraph};
    println!("== P5 graphops 0.5.1 pagerank_weighted_run vs leiden-rs 0.8.1 compute_flow");
    // the existing native_ranking oracle fixture
    let pairs = [(0, 1, 2u64), (0, 1, 1), (0, 2, 1), (1, 2, 2), (2, 0, 1), (3, 2, 1), (4, 0, 2), (4, 3, 1)];
    let n = 6;
    struct Agg { n: usize, arcs: Vec<(usize, usize, f64)>, dup: bool }
    impl Graph for Agg {
        fn node_count(&self) -> usize { self.n }
        fn neighbors(&self, u: usize) -> Vec<usize> {
            let mut v: Vec<usize> = self.arcs.iter().filter(|a| a.0 == u).map(|a| a.1).collect();
            if !self.dup { v.dedup(); }
            v
        }
    }
    impl WeightedGraph for Agg {
        fn edge_weight(&self, s: usize, t: usize) -> f64 { self.arcs.iter().filter(|a| a.0 == s && a.1 == t).map(|a| a.2).sum() }
    }
    let mut raw: Vec<(usize, usize, f64)> = pairs.iter().map(|p| (p.0, p.1, p.2 as f64)).collect();
    raw.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let cfg = PageRankConfig { damping: 0.85, max_iterations: 1000, tolerance: 1e-14 };
    let agg = pagerank_weighted_run(&Agg { n, arcs: raw.clone(), dup: false }, cfg);
    let dup = pagerank_weighted_run(&Agg { n, arcs: raw.clone(), dup: true }, cfg);
    let mut b = leiden_rs::graph::GraphDataBuilder::new(n).directed();
    for (s, t, w) in &raw { b.add_edge(*s, *t, *w).unwrap(); }
    let flow = leiden_rs::infomap::compute_flow(&b.build().unwrap(), 0.15, 1e-14, 1000);
    let maxdiff = agg.scores.iter().zip(&flow).map(|(a, f)| (a - f.flow).abs()).fold(0.0, f64::max);
    let maxdiff_dup = dup.scores.iter().zip(&flow).map(|(a, f)| (a - f.flow).abs()).fold(0.0, f64::max);
    println!("deduplicated neighbours: iterations {}, diff_l1 {:e}, converged {}, max |graphops - compute_flow| {:e}", agg.iterations, agg.diff_l1, agg.converged, maxdiff);
    println!("parallel arcs listed twice in neighbors(): max |graphops - compute_flow| {:e}", maxdiff_dup);
    let capped = pagerank_weighted_run(&Agg { n, arcs: raw, dup: false }, PageRankConfig { max_iterations: 3, ..cfg });
    println!("max_iterations=3: iterations {}, converged {}, diff_l1 {:e}", capped.iterations, capped.converged, capped.diff_l1);
    let empty = pagerank_weighted_run(&Agg { n: 0, arcs: vec![], dup: false }, cfg);
    println!("empty graph: iterations {}, converged {}, diff_l1 {}", empty.iterations, empty.converged, empty.diff_l1);
}


/// Copy of the repository's `canonical_order` (summary_schedule.rs:59-108 at 948b2a88), error type simplified.
fn canonical_order<N: Copy + Ord>(mut components: Vec<Vec<N>>, edges: impl Iterator<Item = (N, N)>) -> Option<Vec<Vec<N>>> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut membership = BTreeMap::new();
    for (index, component) in components.iter_mut().enumerate() {
        component.sort();
        for node in component.iter() { membership.insert(*node, index); }
    }
    let mut callees = vec![BTreeSet::new(); components.len()];
    let mut callers = vec![BTreeSet::new(); components.len()];
    for (source, target) in edges {
        let (s, t) = (membership[&source], membership[&target]);
        if s != t { callees[s].insert(t); callers[t].insert(s); }
    }
    let mut ready = callees.iter().enumerate().filter(|(_, c)| c.is_empty()).map(|(i, _)| (components[i][0], i)).collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some((_, index)) = ready.pop_first() {
        for &caller in &callers[index] {
            callees[caller].remove(&index);
            if callees[caller].is_empty() { ready.insert((components[caller][0], caller)); }
        }
        order.push(std::mem::take(&mut components[index]));
    }
    (order.len() == components.len()).then_some(order)
}

fn p6_schedule_equivalence() {
    use petgraph::algo::{condensation, kosaraju_scc};
    use petgraph::graph::DiGraph;
    use rustworkx_core::dag_algo::lexicographical_topological_sort;
    use std::convert::Infallible;
    println!("== P6 canonical callee-first SCC order vs petgraph condensation + rustworkx lexicographical_topological_sort(reverse=true)");
    let mut rng = Lcg(42);
    let (mut same, mut total, mut selfloop_truncated) = (0, 0, 0);
    for _ in 0..300 {
        let n = 2 + (rng.next() % 40) as usize;
        let m = (rng.next() % (3 * n as u64)) as usize;
        let mut g: DiGraph<u32, ()> = DiGraph::new();
        // node weights are a permutation so NodeIndex order != key order
        let mut keys: Vec<u32> = (0..n as u32).collect();
        for i in (1..n).rev() { let j = (rng.next() as usize) % (i + 1); keys.swap(i, j); }
        let ix: Vec<_> = keys.iter().map(|k| g.add_node(*k)).collect();
        let mut edges = vec![];
        for _ in 0..m {
            let (a, b) = ((rng.next() as usize) % n, (rng.next() as usize) % n);
            g.add_edge(ix[a], ix[b], ());
            edges.push((keys[a], keys[b]));
        }
        let comps: Vec<Vec<u32>> = kosaraju_scc(&g).into_iter().map(|c| c.into_iter().map(|i| g[i]).collect()).collect();
        let ours = canonical_order(comps, edges.into_iter()).unwrap();
        let cond = condensation(g.clone(), true);
        let lib = lexicographical_topological_sort(&cond, |c| Ok::<_, Infallible>(*cond[c].iter().min().unwrap()), true, None).unwrap();
        let mut lib: Vec<Vec<u32>> = lib.into_iter().map(|c| { let mut v = cond[c].clone(); v.sort(); v }).collect();
        if lib.len() != cond.node_count() { lib.clear(); }
        total += 1;
        if lib == ours { same += 1; }
        let keep = condensation(g, false);
        let r = lexicographical_topological_sort(&keep, |c| Ok::<_, Infallible>(*keep[c].iter().min().unwrap()), true, None).unwrap();
        if r.len() < keep.node_count() { selfloop_truncated += 1; }
    }
    println!("identical component order: {same}/{total}; with make_acyclic=false (self-loops kept) Ok-but-truncated results: {selfloop_truncated}/{total}");
}

fn main() {
    p6_schedule_equivalence();
    p1_biodivine();
    p2_rustworkx_lex_topo();
    p3_petgraph_condensation();
    p4::run();
    p5_graphops_pagerank();
}
