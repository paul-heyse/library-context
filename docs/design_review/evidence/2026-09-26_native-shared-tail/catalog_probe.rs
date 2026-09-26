use std::collections::BTreeMap;

use cpg_schema::condition::Condition;
use cpg_schema::condition_kernel::Diagram;

fn main() {
    let mut nodes = BTreeMap::new();
    for source in ["truthy(a) & truthy(z)", "truthy(b) & truthy(z)"] {
        let diagram = Diagram::from_condition(&Condition::parse(source).unwrap()).unwrap();
        let (root, closure) = diagram.root_and_nodes();
        println!("C {} {}", diagram.id().hex(), root.hex());
        for node in closure {
            nodes.insert(node.node_id, node);
        }
    }
    for node in nodes.into_values() {
        println!("N {} {} {} {}", node.node_id.hex(), node.atom,
            node.low.hex(), node.high.hex());
    }
}
