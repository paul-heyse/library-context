use lctx_model::domain::{*, conditions::*, value::*, source::*, attribution::*, input::*};

fn atom(index: i64) -> EvaluationAtom {
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "x.py".into(), b"x and y").unwrap();
    let occurrence = Occurrence { source: source.id(), start: index, end: index + 1,
        syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Predicate, structural_path: vec![] };
    let context = AnalysisContext { python_version: "3.14".into(), python_platform: "linux".into(),
        search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"c"),
        environment_digest: ContentHash::of(b"e"), lock_digest: None };
    EvaluationAtom { evaluation: occurrence.id(), context: context.id(), predicate: Predicate::Truthy.id(), operand: None }
}
fn evaluate(diagram: &Diagram, values: &[(Id<EvaluationAtom>, bool)]) -> bool {
    let live: Vec<_> = values.iter().copied().filter(|(id, _)| diagram.support().contains(id)).collect();
    let fixed = diagram.restrict_atoms(&live).unwrap();
    assert!(fixed.is_true() || fixed.is_false()); fixed.is_true()
}

#[test]
fn typed_bdd_matches_truth_tables_and_survives_reordered_physical_records() {
    let a = atom(0).id(); let b = atom(1).id(); let c = atom(2).id();
    let diagram = Diagram::from_expr(&CondExpr::Or(vec![
        CondExpr::And(vec![CondExpr::Atom(a), CondExpr::Not(Box::new(CondExpr::Atom(b)))]),
        CondExpr::Atom(c),
    ])).unwrap();
    let model = model().unwrap();
    let (condition, mut nodes) = diagram.records(); nodes.reverse();
    let batch = Batch::new(&model, nodes, &budget()).unwrap();
    let decoded = ConditionNode::decode(batch.arrow()).unwrap();
    let rebuilt = Diagram::from_records(&condition, &decoded).unwrap();
    assert_eq!(diagram.id(), rebuilt.id());
    for bits in 0..8 {
        let (av,bv,cv) = (bits&1 != 0, bits&2 != 0, bits&4 != 0);
        assert_eq!(evaluate(&rebuilt, &[(a,av),(b,bv),(c,cv)]), (av && !bv) || cv);
    }
    let reordered = Diagram::from_atom(c).or(&Diagram::from_atom(a).and(&Diagram::from_atom(b).not().unwrap()).unwrap()).unwrap();
    assert_eq!(diagram.id(), reordered.id());
    let invariant = Condition::invariants().remove(0);
    let mut check = (invariant.create)(&budget());
    check.visit(ConditionNode::NAME, batch.arrow()).unwrap();
    check.visit(Condition::NAME, Batch::new(&model, vec![condition.clone()], &budget()).unwrap().arrow()).unwrap();
    check.finish().unwrap();
    assert!(Diagram::from_records(&condition, &[]).is_err());
    assert!(diagram.render_terms(0).unwrap().truncated);
    assert_eq!(diagram.id(), rebuilt.id()); // display never becomes the truth representation
}

#[test]
fn substitution_is_capture_safe_and_unmapped_opaque_guards_survive() {
    let a = atom(0).id(); let b = atom(1).id(); let opaque = atom(2).id();
    let da = Diagram::from_atom(a); let db = Diagram::from_atom(b);
    let source = da.and(&db.not().unwrap()).unwrap().and(&Diagram::from_atom(opaque)).unwrap();
    let substituted = source.substitute_atoms(&[(a,&db),(b,&da)]).unwrap();
    for bits in 0..8 {
        let (av,bv,ov) = (bits&1 != 0, bits&2 != 0, bits&4 != 0);
        assert_eq!(evaluate(&substituted, &[(a,av),(b,bv),(opaque,ov)]), bv && !av && ov);
    }
    assert!(substituted.support().contains(&opaque));
    assert!(source.substitute_atoms(&[(a,&da),(a,&db)]).is_err());
    assert!(source.substitute_atoms(&[(atom(3).id(),&da)]).is_err());
    assert!(source.restrict_atoms(&[(a,true),(a,false)]).is_err());
    assert_eq!(da.and(&da.not().unwrap()).unwrap().id(), Diagram::never().id());
    assert!(da.compatible(&db).unwrap());
    assert!(!da.compatible(&da.not().unwrap()).unwrap());
    assert!(!da.implies(&db).unwrap());
    assert!(source.implies(&da).unwrap());
    assert!(source.given(&da).factored);
    assert!(!source.verify_quotient(&da, &Diagram::always()).factored);
}

#[test]
fn bdd_refusals_are_unknown_and_stored_nodes_must_be_reduced_and_ordered() {
    let model = model().unwrap();
    let mut atoms: Vec<_> = (0..129).map(|i| atom(i).id()).collect(); atoms.sort();
    assert_eq!(Diagram::from_expr(&CondExpr::And(atoms.iter().copied().map(CondExpr::Atom).collect())).unwrap_err(), KernelBoundary::AtomLimit);
    let terminal = ConditionNode::True;
    let invalid = ConditionNode::Branch { atom: atoms[0], low: terminal.id(), high: terminal.id() };
    assert!(Batch::new(&model, vec![invalid], &budget()).is_err());
    let child = ConditionNode::Branch { atom: atoms[0], low: ConditionNode::False.id(), high: terminal.id() };
    let parent = ConditionNode::Branch { atom: atoms[1], low: child.id(), high: terminal.id() };
    assert!(Diagram::from_records(&Condition { root: parent.id() }, &[parent,child,terminal,ConditionNode::False]).is_err());
    let mut check = (Condition::invariants()[0].create)(&budget());
    check.visit(ConditionNode::NAME, Batch::new(&model, vec![ConditionNode::True], &budget()).unwrap().arrow()).unwrap();
    assert!(check.finish().is_err());
    let first = atom(0); let other = atom(1); assert_ne!(first.id(), other.id());
    let mut context = first.clone(); context.context = AnalysisContext { python_version: "3.13".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"c"), environment_digest: ContentHash::of(b"e"), lock_digest: None }.id();
    assert_ne!(first.id(), context.id());
}

#[test]
fn literal_sets_and_structural_paths_preserve_semantic_distinctions() {
    let model = model().unwrap();
    let integer = Literal::Integer { decimal: "184467440737095516160000".into() };
    let bytes = Literal::Bytes { value: EvidenceBytes(vec![0,255]) };
    let batch = Batch::new(&model, vec![integer.clone(), bytes.clone(), Literal::Float { bits: i64::MIN }], &budget()).unwrap();
    assert_eq!(Literal::decode(batch.arrow()).unwrap(), batch.rows());
    for spelling in ["+1","01","-0","","--1"] {
        assert!(Batch::new(&model, vec![Literal::Integer { decimal: spelling.into() }], &budget()).is_err());
    }
    let (set, members) = LiteralSet::of([integer.id(), bytes.id(), integer.id()]);
    assert_eq!(set, LiteralSet::of([bytes.id(), integer.id()]).0);
    assert_eq!(members.len(), 2);
    for omit in [false,true] {
        let mut check = (LiteralSet::invariants()[0].create)(&budget());
        check.visit(LiteralSet::NAME, Batch::new(&model, vec![set.clone()], &budget()).unwrap().arrow()).unwrap();
        let mut ordered = members.clone(); ordered.sort_by_key(|m| m.value);
        if omit { ordered.pop(); }
        for row in ordered { check.visit(LiteralSetMember::NAME, Batch::new(&model, vec![row], &budget()).unwrap().arrow()).unwrap(); }
        assert_eq!(check.finish().is_err(), omit);
    }
    let a = PathSegment::Attribute { name: "a.b".into() }.id();
    let b = PathSegment::Attribute { name: "a".into() }.id();
    let c = PathSegment::Attribute { name: "b".into() }.id();
    assert_ne!(AccessPath::empty().extend(a).id(), AccessPath::empty().extend(b).extend(c).id());
    let saturated = AccessPath::empty().extend(a).extend(b).extend(c);
    assert!(saturated.unknown_suffix); assert_eq!(saturated, saturated.extend(a));
    assert!(Batch::new(&model, vec![AccessPath { first: None, second: Some(a), unknown_suffix: false }], &budget()).is_err());
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
