use std::collections::BTreeMap;

use cpg_schema::condition::{Atom, EvaluationIdentity, Value};
use cpg_schema::condition_kernel::Diagram;
use cpg_schema::id::IdHasher;

fn main() {
    let atoms = [
        Atom::member_of("transport", [Value::Str("sse".into()), Value::Str("http".into())]),
        Atom::Equals { place: "transport".into(), value: Value::Int(2) },
    ];
    let mut nodes = BTreeMap::new();
    for atom in atoms {
        let atom = atom.evaluated(EvaluationIdentity::Site {
            module: "02".repeat(16), start: 10, end: 20,
        });
        let encoded = atom.encode();
        let atom_id = IdHasher::new("bdd-atom").str(&encoded).finish_id();
        let diagram = Diagram::from_atom(&atom).unwrap();
        let (root, closure) = diagram.root_and_nodes();
        println!("C {} {} {} {}", diagram.id().hex(), root.hex(), atom_id.hex(), encoded);
        for node in closure {
            nodes.insert(node.node_id, node);
        }
    }
    for node in nodes.into_values() {
        println!("N {} {} {} {}", node.node_id.hex(), node.atom, node.low.hex(), node.high.hex());
    }
}
