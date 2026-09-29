#[path = "fixtures/guards.rs"] mod fixture;
use fixture::Fixture;
use std::collections::BTreeMap;
use lctx_model::domain::{*,conditions::{*,rebase::*},source::*,value::*,attribution::*};
#[test]
fn call_site_rebasing_preserves_truth_tables_and_nested_guard_origins() {
    let f = Fixture::new(false);
    f.check(&EvaluationAtom::invariants()[0]).unwrap(); f.check(&SyntaxSupport::invariants()[0]).unwrap();
    let mut atoms: BTreeMap<_,_> = f.rows::<EvaluationAtom>().into_iter().map(|r| (r.id(),r)).collect();
    let mut predicates: BTreeMap<_,_> = f.rows::<Predicate>().into_iter().map(|r| (r.id(),r)).collect();
    let places = BTreeMap::new(); let roots = BTreeMap::new();
    let source = Diagram::from_atom(f.origin.id()).not().unwrap();
    let second = rebase_local_guards(&source,&f.calls[1],f.origin.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }).unwrap();
    assert_ne!(second.atoms[0].id(),f.diagram.support()[0]);
    for value in [false,true] { assert_eq!(second.condition.restrict_atoms(&[(second.atoms[0].id(),value)]).unwrap().is_true(),!value); }
    assert!(!second.condition.is_true());
    for row in &second.atoms { atoms.insert(row.id(),row.clone()); } for row in &second.predicates { predicates.insert(row.id(),row.clone()); }
    let nested = rebase_local_guards(&second.condition,&f.calls[0],f.origin.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }).unwrap();
    assert_ne!(nested.atoms[0].id(),f.diagram.support()[0],"same outer site cannot erase distinct nested invocation history");
    assert_eq!(nested.predicates[0],Predicate::InvokedGuard { source: second.atoms[0].id() });
}
#[test]
fn local_rebasing_refuses_formal_operands_forged_records_and_missing_origins() {
    let f = Fixture::new(false);
    let root = PlaceRoot::Formal { declaration: f.origin.evaluation }; let path = AccessPath::empty(); let place = Place { root: root.id(),path: path.id() };
    let formal = EvaluationAtom { operand: Some(place.id()),..f.origin.clone() };
    let mut atoms = BTreeMap::from([(formal.id(),formal.clone())]);
    let predicates = f.rows::<Predicate>().into_iter().map(|r| (r.id(),r)).collect();
    let places = BTreeMap::from([(place.id(),place)]); let roots = BTreeMap::from([(root.id(),root)]);
    let result = rebase_local_guards(&Diagram::from_atom(formal.id()),&f.calls[0],formal.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots });
    assert!(matches!(result,Err(ObligationKind::ConditionTransferUnsupported)));
    atoms.insert(formal.id(),f.origin.clone());
    assert!(rebase_local_guards(&Diagram::from_atom(formal.id()),&f.calls[0],formal.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }).is_err());
    let mut missing = Fixture::new(false); missing.put(missing.rows::<EvaluationAtom>().into_iter().filter(|a| a.id() != missing.origin.id()).collect());
    assert!(missing.check(&EvaluationAtom::invariants()[0]).is_err());
}
#[test]
fn invoked_guards_preserve_source_authorization_and_reject_context_or_site_substitution() {
    let foreign = Fixture::new(true);
    foreign.check(&EvaluationAtom::invariants()[0]).unwrap();
    assert!(foreign.check(&SyntaxSupport::invariants()[0]).is_err(),"a local call site cannot authorize a foreign origin");
    for case in ["context","site","operand"] {
        let mut f = Fixture::new(false); let mut atoms = f.rows::<EvaluationAtom>();
        let index = atoms.iter().position(|a| a.id() != f.origin.id()).unwrap();
        match case {
            "context" => atoms[index].context = AnalysisContext { python_version: "foreign".into(),..f.rows::<AnalysisContext>()[0].clone() }.id(),
            "site" => atoms[index].evaluation = f.origin.evaluation,
            _ => atoms[index].operand = Some(Place { root: PlaceRoot::Occurrence { occurrence: f.origin.evaluation }.id(),path: AccessPath::empty().id() }.id()),
        }
        f.put(atoms); assert!(f.check(&EvaluationAtom::invariants()[0]).is_err(),"{case}");
    }
}

#[test]
fn bounded_nested_guards_refuse_without_erasing_a_condition() {
    let f = Fixture::new(false);
    let mut atoms: BTreeMap<_,_> = f.rows::<EvaluationAtom>().into_iter().map(|r| (r.id(),r)).collect();
    let mut predicates: BTreeMap<_,_> = f.rows::<Predicate>().into_iter().map(|r| (r.id(),r)).collect();
    let places = BTreeMap::new(); let roots = BTreeMap::new();
    let mut condition = Diagram::from_atom(f.origin.id());
    for _ in 1..MAX_GUARD_DEPTH {
        let rebased = rebase_local_guards(&condition,&f.calls[0],f.origin.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }).unwrap();
        assert!(!rebased.condition.is_true()); condition = rebased.condition;
        for row in rebased.atoms { atoms.insert(row.id(),row); } for row in rebased.predicates { predicates.insert(row.id(),row); }
    }
    assert!(matches!(rebase_local_guards(&condition,&f.calls[0],f.origin.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }),Err(ObligationKind::SummaryDepthLimit)));
    let implicit = Occurrence { syntax_kind: SyntaxKind::ExprBinOp,role: OccurrenceRole::Call,..f.calls[0].clone() };
    let original = Diagram::from_atom(f.origin.id());
    assert!(rebase_local_guards(&original,&implicit,f.origin.context,&GuardCatalog { atoms: &atoms,predicates: &predicates,places: &places,roots: &roots }).is_ok());
}

#[test]
fn stored_invoked_guards_enforce_the_same_local_operand_boundary() {
    for kind in ["formal","receiver","local"] { for nested in [false,true] {
        let mut f = Fixture::new(false); f.operand_origin(kind,nested);
        assert_eq!(f.check(&EvaluationAtom::invariants()[0]).is_ok(),kind == "local","{kind}/{nested}");
        // Ordinary, unrebased formal/receiver predicates remain legitimate native facts.
        f.put(vec![f.origin.clone()]); f.check(&EvaluationAtom::invariants()[0]).unwrap();
    } }
}
