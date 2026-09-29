use petgraph::graph::{Edge, Node, NodeIndex, EdgeIndex};
use petgraph::stable_graph::StableGraph;
fn main() {
    println!("Graph Node<(),u32>={} Edge<u32,u32>={} NodeIndex<u32>={} EdgeIndex<u32>={}",
        std::mem::size_of::<Node<(), u32>>(), std::mem::size_of::<Edge<u32, u32>>(),
        std::mem::size_of::<NodeIndex<u32>>(), std::mem::size_of::<EdgeIndex<u32>>());
    println!("Option<u32> (StableGraph edge weight slot)={} Option<()>={}", std::mem::size_of::<Option<u32>>(), std::mem::size_of::<Option<()>>());
    let _s: StableGraph<(), u32> = StableGraph::default();
    println!("biodivine BddNode={}", std::mem::size_of::<biodivine_lib_bdd::BddNode>());
}
