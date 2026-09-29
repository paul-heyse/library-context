//! Recursion-depth control: tarjan_scc (recursive) vs kosaraju_scc (iterative) on a long path.
use petgraph::algo::{kosaraju_scc, tarjan_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
fn main() {
    let which = std::env::args().nth(1).unwrap();
    let n: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let mut g: DiGraph<(), (), u32> = DiGraph::with_capacity(n, n);
    for _ in 0..n { g.add_node(()); }
    for i in 0..n - 1 { g.add_edge(NodeIndex::new(i), NodeIndex::new(i + 1), ()); }
    let k = match which.as_str() {
        "tarjan" => tarjan_scc(&g).len(),
        "kosaraju" => kosaraju_scc(&g).len(),
        "toposort" => toposort(&g, None).unwrap().len(),
        _ => unreachable!(),
    };
    println!("{which} n={n} components_or_len={k}");
}
