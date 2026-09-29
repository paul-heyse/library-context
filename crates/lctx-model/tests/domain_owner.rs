//! The owner rule (C12) over the structure of fixtures/python/semantic_owner/owner.py, with the
//! batch sweep checked against the per-item oracle on shuffled input.
use lctx_model::domain::{*, input::*, occurrence_owner::*, resources::ResourceBudget, source::*};

struct Tree { nodes: Vec<Occurrence>, named: Vec<(&'static str, usize)> }
fn tree() -> Tree {
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "owner.py".into(), include_bytes!("../../../fixtures/python/semantic_owner/owner.py")).unwrap();
    let mut nodes = Vec::new(); let mut named = Vec::new();
    let mut node = |name: &'static str, kind: SyntaxKind, path: &[i32]| {
        let at = nodes.len() as i64;
        nodes.push(Occurrence { source: source.id(), start: at, end: at + 1, syntax_kind: kind, role: OccurrenceRole::Syntax, structural_path: path.to_vec() });
        if !name.is_empty() { named.push((name, nodes.len() - 1)); }
    };
    use SyntaxKind::*;
    node("module", ModModule, &[0]);
    node("f", StmtFunctionDef, &[0, 0]);
    node("", Parameters, &[0, 0, 0]);
    node("", ParameterWithDefault, &[0, 0, 0, 0]);
    node("", Parameter, &[0, 0, 0, 0, 0]);
    node("g()", ExprCall, &[0, 0, 0, 0, 1]);
    node("ann()", ExprCall, &[0, 0, 1]);
    node("", StmtExpr, &[0, 0, 2]);
    node("h()", ExprCall, &[0, 0, 2, 0]);
    node("", StmtReturn, &[0, 0, 3]);
    node("", ExprListComp, &[0, 0, 3, 0]);
    node("k()", ExprCall, &[0, 0, 3, 0, 0]);
    node("C", StmtClassDef, &[0, 1]);
    node("", Decorator, &[0, 1, 0]);
    node("deco()", ExprCall, &[0, 1, 0, 0]);
    node("", Arguments, &[0, 1, 1]);
    node("base()", ExprCall, &[0, 1, 1, 0]);
    node("", StmtAssign, &[0, 1, 2]);
    node("k2()", ExprCall, &[0, 1, 2, 0]);
    node("m", StmtFunctionDef, &[0, 1, 3]);
    node("", StmtReturn, &[0, 1, 3, 0]);
    node("lambda", ExprLambda, &[0, 1, 3, 0, 0]);
    node("m2()", ExprCall, &[0, 1, 3, 0, 0, 0]);
    Tree { nodes, named }
}
impl Tree {
    fn id(&self, name: &str) -> Id<Occurrence> { self.nodes[self.named.iter().find(|(n, _)| *n == name).unwrap().1].id() }
}
fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 20).unwrap() }

#[test]
fn every_occurrence_belongs_to_the_innermost_declaration_whose_body_holds_it() {
    let t = tree();
    let expected = [("g()", "module"), ("ann()", "module"), ("h()", "f"), ("k()", "f"), ("deco()", "module"), ("base()", "module"),
        ("k2()", "C"), ("m", "C"), ("lambda", "m"), ("m2()", "lambda"), ("f", "module"), ("C", "module"), ("module", "module")];
    let table = OwnerTable::build(&t.nodes, &budget()).unwrap();
    for (occurrence, owner) in expected {
        assert_eq!(owner_of(&t.nodes[t.named.iter().find(|(n, _)| *n == occurrence).unwrap().1], &t.nodes).unwrap(), t.id(owner), "oracle: {occurrence}");
        assert_eq!(table.owner(t.id(occurrence)), Some(t.id(owner)), "sweep: {occurrence}");
    }
    for node in &t.nodes { assert_eq!(table.owner(node.id()), Some(owner_of(node, &t.nodes).unwrap())); }
}

#[test]
fn the_sweep_is_order_independent_and_refuses_gaps_repeats_and_short_budgets() {
    let t = tree();
    let reference = OwnerTable::build(&t.nodes, &budget()).unwrap();
    let mut reversed = t.nodes.clone(); reversed.reverse();
    let mut shuffled = t.nodes.clone(); let len = shuffled.len();
    for i in 0..len { shuffled.swap(i, (i * 7 + 3) % len); }
    for nodes in [reversed, shuffled] {
        let table = OwnerTable::build(&nodes, &budget()).unwrap();
        for node in &t.nodes { assert_eq!(table.owner(node.id()), reference.owner(node.id())); }
    }
    let gap: Vec<_> = t.nodes.iter().filter(|n| n.structural_path != vec![0, 0, 2]).cloned().collect();
    assert!(OwnerTable::build(&gap, &budget()).is_err(), "a missing ancestor is a gap, not a guess");
    assert!(owner_of(&t.nodes[8], &gap).is_err());
    let mut repeated = t.nodes.clone(); repeated.push(Occurrence { start: 99, end: 100, ..t.nodes[8].clone() });
    assert!(OwnerTable::build(&repeated, &budget()).is_err(), "two occurrences at one structural position conflict");
    let tiny = ResourceBudget::fixed(64).unwrap();
    assert!(matches!(OwnerTable::build(&t.nodes, &tiny), Err(ModelError::Resource { .. })));
    assert_eq!(tiny.reserved(), 0);
    drop(reference);
}
