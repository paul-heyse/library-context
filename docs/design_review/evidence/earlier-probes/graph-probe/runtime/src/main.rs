//! Behaviour probes for the graph-library review (2026-09-23). Scratch only.
use std::collections::BTreeSet;
use std::time::Instant;

use petgraph::Direction;
use petgraph::algo::dominators::simple_fast;
use petgraph::algo::{
    condensation, has_path_connecting, kosaraju_scc, page_rank, tarjan_scc, toposort,
};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, Dfs, DfsPostOrder, EdgeFiltered, EdgeRef, NodeFiltered, Reversed};

#[derive(Clone, Copy, Debug)]
struct ArcRow {
    call_site: u32,
    phase: u8,
}

fn arc(call_site: u32) -> ArcRow {
    ArcRow {
        call_site,
        phase: 0,
    }
}

fn p1_order() {
    println!("== P1 sibling order (Graph<(), ArcRow, Directed, u32>)");
    let mut g: DiGraph<(), ArcRow, u32> = DiGraph::default();
    let r = g.add_node(());
    let a = g.add_node(());
    let b = g.add_node(());
    let c = g.add_node(());
    let d = g.add_node(());
    g.add_edge(r, a, arc(1));
    g.add_edge(r, b, arc(2));
    g.add_edge(r, c, arc(3));
    g.add_edge(a, d, arc(4));
    let ix = |v: Vec<NodeIndex<u32>>| v.into_iter().map(|n| n.index()).collect::<Vec<_>>();
    println!("insertion order of r's arcs: r->1, r->2, r->3; 1->4");
    println!(
        "neighbors(r)            = {:?}",
        ix(g.neighbors(r).collect())
    );
    println!(
        "edges_directed(r,Out)   = {:?} (call_site)",
        g.edges_directed(r, Direction::Outgoing)
            .map(|e| e.weight().call_site)
            .collect::<Vec<_>>()
    );
    let mut v = vec![];
    let mut bfs = Bfs::new(&g, r);
    while let Some(n) = bfs.next(&g) {
        v.push(n)
    }
    println!("Bfs                     = {:?}", ix(v));
    let mut v = vec![];
    let mut dfs = Dfs::new(&g, r);
    while let Some(n) = dfs.next(&g) {
        v.push(n)
    }
    println!("Dfs                     = {:?}", ix(v));
    let mut v = vec![];
    let mut po = DfsPostOrder::new(&g, r);
    while let Some(n) = po.next(&g) {
        v.push(n)
    }
    println!("DfsPostOrder            = {:?}", ix(v));
}

/// Textbook PageRank with multiplicity-weighted transitions and uniform dangling redistribution.
fn textbook_pr(n: usize, edges: &[(usize, usize)], d: f64, iters: usize) -> (Vec<f64>, usize, f64) {
    let mut out = vec![0f64; n];
    for &(u, _) in edges {
        out[u] += 1.0;
    }
    let mut pr = vec![1.0 / n as f64; n];
    let mut last_delta = f64::NAN;
    for it in 0..iters {
        let dangling: f64 = (0..n).filter(|&i| out[i] == 0.0).map(|i| pr[i]).sum();
        let mut next = vec![(1.0 - d) / n as f64 + d * dangling / n as f64; n];
        for &(u, v) in edges {
            next[v] += d * pr[u] / out[u];
        }
        last_delta = next.iter().zip(&pr).map(|(a, b)| (a - b).abs()).sum();
        pr = next;
        if last_delta < 1e-12 {
            return (pr, it + 1, last_delta);
        }
    }
    (pr, iters, last_delta)
}

fn pg_pr(n: usize, edges: &[(usize, usize)], d: f64, iters: usize) -> Vec<f64> {
    let mut g: DiGraph<(), (), u32> = DiGraph::default();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in edges {
        g.add_edge(NodeIndex::new(u), NodeIndex::new(v), ());
    }
    page_rank(&g, d, iters)
}

fn maxdiff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

fn p3_page_rank() {
    println!("== P3 page_rank");
    let simple = [(0, 1), (0, 2), (1, 2), (2, 0)];
    let (t, it, delta) = textbook_pr(3, &simple, 0.85, 1000);
    let p = pg_pr(3, &simple, 0.85, 200);
    println!("simple graph textbook = {t:.6?} (iters {it}, L1 delta {delta:.1e})");
    println!(
        "simple graph petgraph = {p:.6?}  max|diff| = {:.4}",
        maxdiff(&t, &p)
    );
    let dangling = [(0, 1), (1, 2)];
    let (t, _, _) = textbook_pr(3, &dangling, 0.85, 1000);
    let p = pg_pr(3, &dangling, 0.85, 200);
    println!(
        "chain w/ dangling  textbook = {t:.6?}  petgraph = {p:.6?}  max|diff| = {:.4}",
        maxdiff(&t, &p)
    );
    let parallel = [(0, 1), (0, 1), (0, 2), (1, 0), (2, 0)];
    let collapsed = [(0, 1), (0, 2), (1, 0), (2, 0)];
    let (t, _, _) = textbook_pr(3, &parallel, 0.85, 1000);
    let p = pg_pr(3, &parallel, 0.85, 200);
    let pc = pg_pr(3, &collapsed, 0.85, 200);
    println!("parallel 0->1 x2: textbook(multiplicity) = {t:.6?}");
    println!("parallel 0->1 x2: petgraph               = {p:.6?}");
    println!("collapsed        : petgraph               = {pc:.6?}");
    // Cost: O(iters * V * E) per the doc; time a modest graph.
    for &(n, m) in &[(500usize, 1500usize), (1000, 3000), (2000, 6000)] {
        let mut state = 12345u64;
        let mut rnd = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let edges: Vec<(usize, usize)> = (0..m)
            .map(|_| ((rnd() % n as u64) as usize, (rnd() % n as u64) as usize))
            .collect();
        let t0 = Instant::now();
        let _ = pg_pr(n, &edges, 0.85, 20);
        let pg = t0.elapsed();
        let t0 = Instant::now();
        let _ = textbook_pr(n, &edges, 0.85, 20);
        let tb = t0.elapsed();
        println!("V={n} E={m} 20 iters: petgraph page_rank {pg:?}  sparse power iteration {tb:?}");
    }
}

fn p4_simplifying_containers() {
    println!("== P4 Csr / GraphMap and parallel edges");
    let dup = petgraph::csr::Csr::<(), ()>::from_sorted_edges(&[(0u32, 1u32), (0, 1)]);
    println!(
        "Csr::from_sorted_edges([(0,1),(0,1)]) is_err = {}",
        dup.is_err()
    );
    let mut csr = petgraph::csr::Csr::<(), u32>::with_nodes(2);
    let first = csr.add_edge(0, 1, 7);
    let second = csr.add_edge(0, 1, 8);
    println!(
        "Csr::add_edge twice -> {first}, {second}; edge_count = {}",
        csr.edge_count()
    );
    let mut gm = petgraph::graphmap::DiGraphMap::<u32, u32>::new();
    gm.add_edge(0, 1, 7);
    let prev = gm.add_edge(0, 1, 8);
    println!(
        "DiGraphMap::add_edge twice -> second returned {prev:?}; edge_count = {}",
        gm.edge_count()
    );
}

fn p5_views() {
    println!("== P5 filtered / reversed views (compile = trait bounds hold)");
    // r -> a (phase 0), r -> b (phase 1 = constructor), a -> c, b -> c, c -> a (cycle a<->c)
    let mut g: DiGraph<(), ArcRow, u32> = DiGraph::default();
    let r = g.add_node(());
    let a = g.add_node(());
    let b = g.add_node(());
    let c = g.add_node(());
    g.add_edge(r, a, arc(1));
    g.add_edge(
        r,
        b,
        ArcRow {
            call_site: 2,
            phase: 1,
        },
    );
    g.add_edge(a, c, arc(3));
    g.add_edge(b, c, arc(4));
    g.add_edge(c, a, arc(5));
    let ef = EdgeFiltered::from_fn(&g, |e| e.weight().phase == 0);
    let mut seen = vec![];
    let mut bfs = Bfs::new(&ef, r);
    while let Some(n) = bfs.next(&ef) {
        seen.push(n.index());
    }
    println!("Bfs over EdgeFiltered(phase==0) from r = {seen:?}");
    let sccs: BTreeSet<BTreeSet<usize>> = tarjan_scc(&ef)
        .into_iter()
        .map(|c| c.into_iter().map(|n| n.index()).collect())
        .collect();
    println!("tarjan_scc(&EdgeFiltered) = {sccs:?}");
    let nf = NodeFiltered::from_fn(&g, |n: NodeIndex<u32>| n != a);
    let sccs: BTreeSet<BTreeSet<usize>> = tarjan_scc(&nf)
        .into_iter()
        .map(|c| c.into_iter().map(|n| n.index()).collect())
        .collect();
    println!("tarjan_scc(&NodeFiltered without a) = {sccs:?}");
    let rev = Reversed(&g);
    let mut seen = vec![];
    let mut bfs = Bfs::new(rev, c);
    while let Some(n) = bfs.next(rev) {
        seen.push(n.index());
    }
    println!("Bfs over Reversed(&g) from c (callers of c) = {seen:?}");
    println!(
        "kosaraju_scc(Reversed(&g)) count = {}",
        kosaraju_scc(rev).len()
    );
    let dom = simple_fast(&ef, r);
    println!(
        "dominators over EdgeFiltered: idom(c) = {:?}",
        dom.immediate_dominator(c).map(|n| n.index())
    );
    println!(
        "has_path_connecting(&ef, r, b) = {}",
        has_path_connecting(&ef, r, b, None)
    );
    println!(
        "toposort(&g) is_err (cycle) = {}",
        toposort(&g, None).is_err()
    );
    let cond = condensation(g.clone(), true);
    println!(
        "condensation(g, true): {} nodes, {} edges; condensed edge weights (call_site) = {:?}",
        cond.node_count(),
        cond.edge_count(),
        cond.edge_references()
            .map(|e| e.weight().call_site)
            .collect::<Vec<_>>()
    );
}

fn p6_rustworkx() {
    use petgraph::stable_graph::StableDiGraph;
    use rustworkx_core::centrality::{
        betweenness_centrality, closeness_centrality, eigenvector_centrality,
    };
    use rustworkx_core::connectivity::connected_components;
    println!("== P6 rustworkx-core 0.18.1 on petgraph 0.8.3 containers");
    let mut g: DiGraph<(), ArcRow, u32> = DiGraph::default();
    let n: Vec<_> = (0..5).map(|_| g.add_node(())).collect();
    for (i, (u, v)) in [(0, 1), (1, 2), (2, 3), (1, 3), (3, 4), (0, 1)]
        .iter()
        .enumerate()
    {
        g.add_edge(n[*u], n[*v], arc(i as u32));
    }
    let bc = betweenness_centrality(&g, false, true, usize::MAX);
    println!("betweenness (sequential; parallel edge 0->1 x2) = {bc:.4?}");
    let cc = closeness_centrality(&g, true, usize::MAX);
    println!("closeness = {cc:.4?}");
    let ev: Result<Option<Vec<f64>>, std::convert::Infallible> =
        eigenvector_centrality(&g, |_| Ok(1.0), None, None);
    println!("eigenvector on a DAG (no convergence expected) = {ev:?}");
    let mut s: StableDiGraph<(), (), u32> = StableDiGraph::default();
    let sn: Vec<_> = (0..4).map(|_| s.add_node(())).collect();
    s.add_edge(sn[0], sn[1], ());
    s.add_edge(sn[1], sn[3], ());
    s.add_edge(sn[0], sn[3], ());
    s.remove_node(sn[2]);
    let bc = betweenness_centrality(&s, false, false, usize::MAX);
    println!("betweenness on StableGraph with node 2 removed (indexed by to_index) = {bc:?}");
    let mut u: petgraph::graph::UnGraph<(), (), u32> = petgraph::graph::UnGraph::default();
    let un: Vec<_> = (0..12).map(|_| u.add_node(())).collect();
    for i in 0..11 {
        u.add_edge(un[i], un[i + 1], ());
    }
    let comps = connected_components(&u);
    println!(
        "connected_components HashSet iteration order (varies per process?) = {:?}",
        comps[0].iter().map(|x| x.index()).collect::<Vec<_>>()
    );
}

fn membership_canonical(m: &[usize]) -> Vec<usize> {
    // relabel communities by first occurrence so partitions compare by membership
    let mut map = std::collections::HashMap::new();
    m.iter()
        .map(|c| {
            let l = map.len();
            *map.entry(*c).or_insert(l)
        })
        .collect()
}

fn p7_leiden() { p7_leiden_mu(0.2); p7_leiden_mu(0.5); }

fn p7_leiden_mu(mu: f64) {
    use leiden_rs::{
        GraphDataBuilder, Leiden, LeidenConfig, LfrConfig, QualityType, generate_lfr_graph, nmi,
    };
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    println!("== P7 leiden-rs 0.8.1 (default-features = false, features = [petgraph]) mu={mu}");
    let lfr = generate_lfr_graph(LfrConfig {
        n: 400,
        tau1: 2.5,
        tau2: 1.5,
        mu,
        average_degree: Some(8.0),
        min_degree: None,
        max_degree: Some(15),
        min_community: Some(20),
        max_community: Some(80),
        seed: Some(7),
    })
    .expect("lfr");
    let build = |n: usize, edges: &[(usize, usize, f64)]| {
        let mut b = GraphDataBuilder::new(n);
        for &(u, v, w) in edges {
            b.add_edge(u, v, w).unwrap();
        }
        b.build().unwrap()
    };
    let run = |data: &leiden_rs::GraphData, q: QualityType, res: f64, seed: u64| {
        let cfg = LeidenConfig::builder()
            .quality(q)
            .resolution(res)
            .seed(seed)
            .build();
        Leiden::new(cfg).run(data).unwrap()
    };
    let data = build(lfr.node_count, &lfr.edges);
    println!(
        "LFR n={} edges={} planted communities={}",
        lfr.node_count,
        lfr.edges.len(),
        lfr.ground_truth.iter().collect::<BTreeSet<_>>().len()
    );
    for (q, res) in [
        (QualityType::Modularity, 1.0),
        (QualityType::CPM, 0.05),
        (QualityType::CPM, 0.1),
    ] {
        let o1 = run(&data, q, res, 42);
        let o2 = run(&data, q, res, 42);
        let same = o1.partition.as_slice() == o2.partition.as_slice();
        println!(
            "{q:?} res={res}: communities={} quality={:.4} NMI(planted)={:.4} rerun identical={same}",
            o1.partition.num_communities(),
            o1.quality,
            nmi(o1.partition.as_slice(), &lfr.ground_truth)
        );
    }
    // Shuffled edge order, same node ids.
    let base = run(&data, QualityType::CPM, 0.05, 42);
    let mut shuffled = lfr.edges.clone();
    let mut rng = rand::rngs::StdRng::seed_from_u64(99);
    shuffled.shuffle(&mut rng);
    // Also flip orientation of half the undirected edges.
    for (i, e) in shuffled.iter_mut().enumerate() {
        if i % 2 == 0 {
            *e = (e.1, e.0, e.2);
        }
    }
    let s = run(
        &build(lfr.node_count, &shuffled),
        QualityType::CPM,
        0.05,
        42,
    );
    println!(
        "shuffled+flipped edge input, same seed: identical membership = {}",
        base.partition.as_slice() == s.partition.as_slice()
    );
    for (label, do_shuffle, do_flip) in [("shuffle only", true, false), ("flip only", false, true)] {
        let mut e2 = lfr.edges.clone();
        if do_shuffle {
            let mut r2 = rand::rngs::StdRng::seed_from_u64(1234);
            e2.shuffle(&mut r2);
        }
        if do_flip {
            for (i, e) in e2.iter_mut().enumerate() {
                if i % 2 == 0 {
                    *e = (e.1, e.0, e.2);
                }
            }
        }
        let o = run(&build(lfr.node_count, &e2), QualityType::CPM, 0.05, 42);
        println!("{label}: identical membership = {}", base.partition.as_slice() == o.partition.as_slice());
        // canonical orientation (min,max) then sort: the adapter's normal form
        let mut canon: Vec<_> = e2.iter().map(|&(u, v, w)| (u.min(v), u.max(v), w)).collect();
        canon.sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)));
        let mut base_canon: Vec<_> = lfr.edges.iter().map(|&(u, v, w)| (u.min(v), u.max(v), w)).collect();
        base_canon.sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)));
        let oc = run(&build(lfr.node_count, &canon), QualityType::CPM, 0.05, 42);
        let bc = run(&build(lfr.node_count, &base_canon), QualityType::CPM, 0.05, 42);
        println!("{label} after (min,max)+sort normal form: identical = {}", oc.partition.as_slice() == bc.partition.as_slice());
    }
    // Permuted node ids, mapped back.
    let mut perm: Vec<usize> = (0..lfr.node_count).collect();
    perm.shuffle(&mut rng);
    let permuted: Vec<_> = lfr
        .edges
        .iter()
        .map(|&(u, v, w)| (perm[u], perm[v], w))
        .collect();
    let p = run(
        &build(lfr.node_count, &permuted),
        QualityType::CPM,
        0.05,
        42,
    );
    let back: Vec<usize> = (0..lfr.node_count)
        .map(|old| p.partition.as_slice()[perm[old]])
        .collect();
    println!(
        "permuted node ids, same seed: same partition = {}  NMI(vs base) = {:.4}",
        membership_canonical(&back) == membership_canonical(base.partition.as_slice()),
        nmi(&back, base.partition.as_slice())
    );
    // Seed sensitivity.
    let parts: Vec<Vec<usize>> = (0..10u64)
        .map(|sd| {
            run(&data, QualityType::CPM, 0.05, sd)
                .partition
                .as_slice()
                .to_vec()
        })
        .collect();
    let mut min_nmi: f64 = 1.0;
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            min_nmi = min_nmi.min(nmi(&parts[i], &parts[j]));
        }
    }
    println!("10 seeds CPM 0.05: min pairwise NMI = {min_nmi:.4}");
    // Undirected orientation double-count.
    let tw = |edges: &[(usize, usize, f64)]| build(2, edges).total_weight();
    println!(
        "undirected total_weight: [(0,1,1)] = {}, [(0,1,1),(1,0,1)] = {}, [(0,1,1),(0,1,1)] = {}",
        tw(&[(0, 1, 1.0)]),
        tw(&[(0, 1, 1.0), (1, 0, 1.0)]),
        tw(&[(0, 1, 1.0), (0, 1, 1.0)])
    );
    // petgraph adapter needs E: Into<f64>.
    let mut pg: petgraph::graph::UnGraph<(), f64, u32> = petgraph::graph::UnGraph::default();
    let a = pg.add_node(());
    let b = pg.add_node(());
    pg.add_edge(a, b, 1.0);
    let gd = leiden_rs::from_petgraph(&pg).unwrap();
    println!(
        "from_petgraph(UnGraph<(), f64>) node_count = {}",
        gd.node_count()
    );
}

fn p8_datafrog() {
    use datafrog::Iteration;
    println!("== P8 datafrog transitive closure");
    let mut it = Iteration::new();
    let edges = it.variable::<(u32, u32)>("edges");
    let reach = it.variable::<(u32, u32)>("reach");
    let reach_by_mid = it.variable::<(u32, u32)>("reach_by_mid");
    edges.extend(vec![(0, 1), (1, 2), (2, 0), (2, 3)]);
    reach.extend(vec![(0, 1), (1, 2), (2, 0), (2, 3)]);
    while it.changed() {
        reach_by_mid.from_map(&reach, |&(a, b)| (b, a));
        reach.from_join(&reach_by_mid, &edges, |_b, &a, &c| (a, c));
    }
    let r = reach.complete();
    println!(
        "closure size = {} (expected 12: {{0,1,2}}x{{0,1,2,3}})",
        r.elements.len()
    );
}

fn p9_fcars() {
    println!("== P9 fcars 0.2.2 (PCbO)");
    let cxt = "B\n\n3\n3\n\no1\no2\no3\na1\na2\na3\nX..\nXX.\n.XX\n";
    let ctx: fcars::FormalContext = fcars::FormalContext::from_cxt(cxt.as_bytes());
    let concepts = ctx.all_concepts();
    let mut intents: Vec<Vec<String>> = concepts
        .iter()
        .map(|c| c.intent_names_iter().cloned().collect())
        .collect();
    println!("concept count = {} (hand count: 6)", concepts.len());
    intents.sort();
    println!("intents (sorted) = {intents:?}");
}


fn p5b_condensation_collapse() {
    println!("== P5b condensation(make_acyclic=true) and parallel inter-SCC arcs");
    let mut g: DiGraph<(), ArcRow, u32> = DiGraph::default();
    let r = g.add_node(());
    let a = g.add_node(());
    let c = g.add_node(());
    g.add_edge(r, a, arc(1));
    g.add_edge(r, c, arc(2));
    g.add_edge(r, a, arc(3));
    g.add_edge(a, c, arc(4));
    g.add_edge(c, a, arc(5));
    let cond = condensation(g.clone(), true);
    println!(
        "3 arcs r->{{a,c}} (call sites 1,2,3) become {} condensed edge(s) with call_site(s) {:?}",
        cond.edge_count(),
        cond.edge_references().map(|e| e.weight().call_site).collect::<Vec<_>>()
    );
    let cond = condensation(g, false);
    println!("make_acyclic=false keeps {} edges (incl. intra-SCC self-loops)", cond.edge_count());
}

fn p5c_page_rank_on_view() {
    let mut g: DiGraph<(), ArcRow, u32> = DiGraph::default();
    let r = g.add_node(());
    let a = g.add_node(());
    g.add_edge(r, a, arc(1));
    let ef = EdgeFiltered::from_fn(&g, |e| e.weight().phase == 0);
    let pr = page_rank(&ef, 0.85f64, 10);
    println!("page_rank(&EdgeFiltered) compiles: {pr:.4?}");
}

fn main() {
    p1_order();
    p3_page_rank();
    p4_simplifying_containers();
    p5_views();
    p5b_condensation_collapse();
    p5c_page_rank_on_view();
    p6_rustworkx();
    p7_leiden();
    p8_datafrog();
    p9_fcars();
}
