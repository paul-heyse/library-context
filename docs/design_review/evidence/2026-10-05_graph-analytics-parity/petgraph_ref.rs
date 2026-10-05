//! In-process reference on the exported facts graph: petgraph 0.8.3 and leiden-rs 0.8.1 used
//! with the same conventions as the model-owned kernels (independent re-implementation of the
//! conventions, not the product code):
//!   * vertices sorted by semantic key; arcs kept with multiplicity (parallel arcs, self-loops);
//!   * SCC: kosaraju_scc; condensation node count; WCC via connected_components + union-find;
//!   * PageRank: uniform teleport, dangling mass spread uniformly, out-degree counts parallel
//!     arcs, accumulation in sorted-source order, damping 0.85, tol 1e-10 (L1), max 100
//!     iterations; residual reported (the definition our ranking kernel states);
//!   * Leiden: undirected pairs, self-loops dropped, parallel arcs summed (weight = count),
//!     RBER quality, resolution 1.0, seeds 0..10, epsilon 1e-10, 100 iterations, canonical labels.
//! Copied to build/review-probes/graph-analytics-parity/petgraph-ref/src/main.rs.
//! Usage: petgraph-ref <dir with vertices.csv arcs.csv>  -> writes ref_*.csv + ref_summary.json
use leiden_rs::{GraphDataBuilder, Leiden, LeidenConfig, QualityType};
use petgraph::algo::{condensation, connected_components, kosaraju_scc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::unionfind::UnionFind;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let vtext = fs::read_to_string(format!("{dir}/vertices.csv")).unwrap();
    let mut keys: Vec<&str> = vtext.lines().skip(1).collect();
    keys.sort_unstable();
    keys.dedup();
    let index: BTreeMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let atext = fs::read_to_string(format!("{dir}/arcs.csv")).unwrap();
    let mut arcs: Vec<(usize, usize)> = atext
        .lines()
        .skip(1)
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (index[f[1]], index[f[2]])
        })
        .collect();
    arcs.sort_unstable();
    let n = keys.len();
    let mut g = DiGraph::<usize, ()>::with_capacity(n, arcs.len());
    for i in 0..n {
        g.add_node(i);
    }
    for &(a, b) in &arcs {
        g.add_edge(NodeIndex::new(a), NodeIndex::new(b), ());
    }
    let mut out = String::new();
    // SCC
    let sccs = kosaraju_scc(&g);
    let mut scc_label = vec![0usize; n];
    for comp in &sccs {
        let min = comp.iter().map(|x| x.index()).min().unwrap();
        for x in comp {
            scc_label[x.index()] = min;
        }
    }
    let cond = condensation(g.clone(), true);
    // WCC
    let wcc_count = connected_components(&g);
    let mut uf = UnionFind::<usize>::new(n);
    for &(a, b) in &arcs {
        uf.union(a, b);
    }
    let wcc_label: Vec<usize> = (0..n).map(|i| uf.find(i)).collect();
    // PageRank (stated definition)
    let mut outdeg = vec![0usize; n];
    for &(a, _) in &arcs {
        outdeg[a] += 1;
    }
    let d = 0.85f64;
    let uniform = 1.0 / n as f64;
    let mut s = vec![uniform; n];
    let (mut iters, mut residual, mut converged) = (0usize, f64::NAN, false);
    for _ in 0..100 {
        iters += 1;
        let dangling: f64 = (0..n).filter(|&i| outdeg[i] == 0).map(|i| s[i]).sum();
        let base = (1.0 - d) * uniform + d * dangling * uniform;
        let mut next = vec![base; n];
        for &(a, b) in &arcs {
            next[b] += d * s[a] / outdeg[a] as f64;
        }
        residual = next.iter().zip(&s).map(|(x, y)| (x - y).abs()).sum();
        s = next;
        if residual < 1e-10 {
            converged = true;
            break;
        }
    }
    // Leiden (RBER, undirected, self-loops dropped, parallel summed)
    let mut pairs = BTreeMap::<(usize, usize), f64>::new();
    for &(a, b) in &arcs {
        if a != b {
            *pairs.entry((a.min(b), a.max(b))).or_default() += 1.0;
        }
    }
    let mut builder = GraphDataBuilder::new(n);
    for (&(a, b), &w) in &pairs {
        builder.add_edge(a, b, w).unwrap();
    }
    let gd = builder.build().unwrap();
    let mut leiden_runs = Vec::new();
    for seed in 0..10u64 {
        let r = Leiden::new(LeidenConfig {
            resolution: 1.0,
            seed: Some(seed),
            quality: QualityType::RBER,
            max_iterations: 100,
            epsilon: 1e-10,
            track_quality_history: true,
            ..Default::default()
        })
        .run(&gd)
        .unwrap();
        let mut canon = BTreeMap::new();
        let labels: Vec<usize> = r
            .partition
            .as_slice()
            .iter()
            .map(|v| {
                let next = canon.len();
                *canon.entry(*v).or_insert(next)
            })
            .collect();
        let modularity = leiden_rs::compute_modularity(&gd, &r.partition);
        leiden_runs.push((seed, labels, r.quality, modularity, r.quality_history.len()));
    }
    // write label files
    let mut lab = String::from("key,scc,wcc,pagerank");
    for (seed, ..) in &leiden_runs {
        let _ = write!(lab, ",leiden_s{seed}");
    }
    lab.push('\n');
    for i in 0..n {
        let _ = write!(lab, "{},{},{},{:.17e}", keys[i], scc_label[i], wcc_label[i], s[i]);
        for (_, labels, ..) in &leiden_runs {
            let _ = write!(lab, ",{}", labels[i]);
        }
        lab.push('\n');
    }
    fs::write(format!("{dir}/ref_labels.csv"), lab).unwrap();
    let _ = write!(
        out,
        "{{\"vertices\":{n},\"arcs\":{},\"self_loops\":{},\"undirected_pairs\":{},\"scc_count\":{},\"scc_nontrivial\":{},\"largest_scc\":{},\"condensation_nodes\":{},\"condensation_edges\":{},\"wcc_count\":{wcc_count},\"pagerank\":{{\"iterations\":{iters},\"residual\":{residual:e},\"converged\":{converged},\"sum\":{}}},\"leiden\":[",
        arcs.len(),
        arcs.iter().filter(|(a, b)| a == b).count(),
        pairs.len(),
        sccs.len(),
        sccs.iter().filter(|c| c.len() > 1).count(),
        sccs.iter().map(|c| c.len()).max().unwrap_or(0),
        cond.node_count(),
        cond.edge_count(),
        s.iter().sum::<f64>()
    );
    for (k, (seed, labels, q, m, it)) in leiden_runs.iter().enumerate() {
        let comms = labels.iter().max().map(|x| x + 1).unwrap_or(0);
        let _ = write!(
            out,
            "{}{{\"seed\":{seed},\"communities\":{comms},\"rber_quality\":{q},\"modularity\":{m},\"iterations\":{it}}}",
            if k > 0 { "," } else { "" }
        );
    }
    out.push_str("]}");
    fs::write(format!("{dir}/ref_summary.json"), &out).unwrap();
    println!("{out}");
}
