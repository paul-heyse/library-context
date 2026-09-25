#![allow(dead_code)]
#[path = "../../../../crates/cpg-schema/src/codebook.rs"]
mod codebook;
pub use codebook::Codebook;
#[path = "../../../../crates/cpg-schema/src/id.rs"]
mod id;
#[path = "../../../../crates/cpg-schema/src/condition.rs"]
mod condition;
#[path = "../../../../crates/cpg-schema/src/condition_kernel.rs"]
mod condition_kernel;

use biodivine_lib_bdd::{Bdd, BddPointer, BddVariableSet, op_function};
use condition::Atom;
use condition_kernel::{ConditionRoot, Diagram, DiagramNode, KernelBoundary, hydrate_catalog};
use id::{Id, IdHasher};
use std::collections::{BTreeMap, HashMap};

fn atom(name: &str) -> Atom { Atom::Truthy { place: name.into() } }

// Export a raw upstream fixture through the real kernel's validated persisted-node boundary.
fn admit(bdd: &Bdd, names: &[String]) -> Diagram {
    fn visit(bdd: &Bdd, p: BddPointer, names: &[String], rows: &mut BTreeMap<Id, DiagramNode>, seen: &mut HashMap<BddPointer, Id>) -> Id {
        if p.is_zero() { return condition_kernel::false_terminal(); }
        if p.is_one() { return condition_kernel::true_terminal(); }
        if let Some(id) = seen.get(&p) { return *id; }
        let low = visit(bdd, bdd.low_link_of(p), names, rows, seen);
        let high = visit(bdd, bdd.high_link_of(p), names, rows, seen);
        let atom = names[bdd.var_of(p).to_index()].clone();
        let id = IdHasher::new("bdd-node").str(&atom).id(low).id(high).finish_id();
        rows.insert(id, DiagramNode { node_id: id, atom, low, high });
        seen.insert(p, id);
        id
    }
    let mut rows = BTreeMap::new();
    let root = visit(bdd, bdd.root_pointer(), names, &mut rows, &mut HashMap::new());
    Diagram::from_root_and_nodes(root, &rows.into_values().collect::<Vec<_>>()).unwrap()
}

fn main() {
    // F02: equal semantic IDs have different admission behavior until hydration.
    let mut accumulated = Diagram::never();
    for i in 0..128 {
        let a = Diagram::from_atom(&atom(&format!("a{i:03}"))).unwrap();
        accumulated = accumulated.or(&a.and(&a.not().unwrap()).unwrap()).unwrap();
    }
    let next = Diagram::from_atom(&atom("z")).unwrap();
    let (root, nodes) = accumulated.root_and_nodes();
    let hydrated = Diagram::from_root_and_nodes(root, &nodes).unwrap();
    assert_eq!(accumulated.id(), hydrated.id());
    assert!(matches!(accumulated.or(&next), Err(KernelBoundary::AtomLimit)));
    assert!(hydrated.or(&next).is_ok());
    println!("redundant_support live={} hydrated={} same_id=true live_next=AtomLimit hydrated_next=ok", accumulated.support().len(), hydrated.support().len());

    // F01: both operands meet the real kernel's limits, but their conjunction does not.
    let names: Vec<String> = (0..16).map(|i| atom(&format!("a{i:02}")).encode())
        .chain((0..16).map(|i| atom(&format!("z{i:02}")).encode())).collect();
    let variable_names: Vec<String> = (0..32).map(|i| format!("v{i}")).collect();
    let ctx = BddVariableSet::new(&variable_names.iter().map(String::as_str).collect::<Vec<_>>());
    let vars = ctx.variables();
    let pairs = |range: std::ops::Range<usize>| range.fold(ctx.mk_false(), |f, i| f.or(&ctx.mk_var(vars[i]).and(&ctx.mk_var(vars[i + 16]))));
    let left = pairs(0..8);
    let right = pairs(8..16);
    let dl = admit(&left, &names);
    let dr = admit(&right, &names);
    let full = left.and(&right);
    let decision = Bdd::check_binary_op(1_000_000, &left, &right, op_function::and).unwrap();
    println!("admitted_operands left={} right={} full_result={} kernel={:?} check={decision:?}", dl.node_count(), dr.node_count(), full.size(), dl.compatible(&dr));
    assert!(full.size() > 50_000);
    assert_eq!(dl.compatible(&dr), Err(KernelBoundary::NodeLimit));
    assert!(decision.0);

    // Product preflight refuses a small task traversal even for admitted inputs.
    let medium = pairs(0..12);
    let dm = admit(&medium, &names);
    let negated = dm.not().unwrap();
    let check = Bdd::check_binary_op(1_000_000, &medium, &medium.not(), op_function::and).unwrap();
    assert_eq!(dm.compatible(&negated), Err(KernelBoundary::WorkPreflight));
    assert!(!check.0);
    assert!(Bdd::check_binary_op(10, &medium, &medium.not(), op_function::and).is_none());
    println!("admitted_contradiction nodes={} kernel={:?} check={check:?} task_limit_control=None", dm.node_count(), dm.compatible(&negated));

    // New finding: distinct roots share one stored tail but hydrate into separate owned BDDs.
    let mut tail = Diagram::always();
    for i in 0..64 { tail = tail.and(&Diagram::from_atom(&atom(&format!("t{i:03}"))).unwrap()).unwrap(); }
    let mut roots = Vec::new();
    let mut catalog = BTreeMap::new();
    for i in 0..256 {
        let d = tail.and(&Diagram::from_atom(&atom(&format!("a{i:03}"))).unwrap()).unwrap();
        let (root_id, rows) = d.root_and_nodes();
        roots.push(ConditionRoot { condition_id: d.id(), root_id: Some(root_id), boundary_reason: None });
        for row in rows { catalog.insert(row.node_id, row); }
    }
    let rows: Vec<_> = catalog.into_values().collect();
    let loaded = hydrate_catalog(&roots, &rows).unwrap();
    let expanded: usize = loaded.values().map(Diagram::node_count).sum();
    assert_eq!(rows.len(), 320);
    assert_eq!(expanded, 17_152);
    println!("catalog roots={} unique_nonterminals={} hydrated_owned_nodes={expanded}", roots.len(), rows.len());
}
