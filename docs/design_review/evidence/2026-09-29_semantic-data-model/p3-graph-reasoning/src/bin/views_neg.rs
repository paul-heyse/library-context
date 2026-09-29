//! Negative control: page_rank needs NodeCount, which NodeFiltered does not implement.
use fixedbitset::FixedBitSet;
use petgraph::algo::page_rank;
use petgraph::graph::DiGraph;
use petgraph::visit::NodeFiltered;
fn main() {
    let g: DiGraph<(), u32, u32> = DiGraph::default();
    let mask = FixedBitSet::with_capacity(0);
    let _ = page_rank(&NodeFiltered(&g, &mask), 0.85f64, 10);
}
