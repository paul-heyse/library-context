#[path = "fixtures/types.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,types::*,value::Literal};

#[test]
fn type_structure_keeps_large_literals_variable_identity_and_recursive_restrictions() {
    let f = Fixture::new(false);
    for name in ["type_sequence_membership","structural_type_shapes",TypeSupport::NAME,TypePresentationSupport::NAME,TypeRestrictionSupport::NAME] {
        f.base.check(f.base.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    let views = f.base.rows::<TypePresentation>(); assert_eq!(views.len(),2);
    assert!(views.iter().all(|v| v.term == f.term.id())); assert_ne!(views[0].id(),views[1].id());
    let mut renamed = f.variable.clone(); renamed.module = "unrelated".into(); assert_ne!(renamed.id(),f.variable.id());
    let literal = f.base.rows::<Literal>().into_iter().find(|v| matches!(v,Literal::Integer { .. })).unwrap();
    assert_eq!(Literal::decode(Batch::new(&f.base.model,vec![literal.clone()]).unwrap().arrow()).unwrap(),vec![literal]);
    let restriction = f.base.rows::<TypeVariableRestriction>()[0].clone();
    let instance = f.base.rows::<TypeTerm>().into_iter().find(|t| t.id() == restriction.term).unwrap();
    assert!(matches!(instance,TypeTerm::ClassInstance { .. }),"recursive bound is a relationship, not key expansion");
}
#[test]
fn a_nested_type_variable_cannot_switch_provider_namespace() {
    let f = Fixture::new(true);
    let error = f.base.check(&TypeSupport::invariants()[0]).unwrap_err();
    assert!(error.to_string().contains("type variable belongs"));
    assert!(f.base.check(&TypeRestrictionSupport::invariants()[0]).is_err());
}
#[test]
fn type_sequences_require_complete_ordered_membership_and_kind_correct_children() {
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>(); members.pop(); f.put_members(members);
    assert!(f.base.check(&TypeSequence::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>(); members[0].ordinal += 1; f.put_members(members);
    assert!(f.base.check(&TypeSequence::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let wrong = TypeTerm::ParamSpec { variable: f.variable.id() }; let mut terms = f.base.rows::<TypeTerm>(); terms.push(wrong); f.base.put(terms);
    assert!(f.base.check(&TypeTerm::invariants()[0]).is_err());
}

#[test]
fn opaque_type_leaves_cannot_be_promoted_to_structural_fidelity() {
    use lctx_model::domain::attribution::Fidelity;
    for truncated in [false,true] { for nested in [false,true] {
        for fidelity in [Fidelity::Raw,Fidelity::NativeStructural,Fidelity::NormalizedStructural,Fidelity::ReportProjection,Fidelity::DisplayOnly] {
            let mut f = Fixture::new(false); f.opaque(truncated,nested,fidelity);
            let result = f.base.check(&TypeSupport::invariants()[0]);
            assert_eq!(result.is_ok(),fidelity == Fidelity::DisplayOnly,"{truncated}/{nested}/{fidelity:?}: {result:?}");
        }
    } }
}
