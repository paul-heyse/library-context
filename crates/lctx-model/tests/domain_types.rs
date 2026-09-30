#[path = "fixtures/types.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,calls::ProviderModule,types::*,value::Literal};

#[test]
fn type_structure_keeps_large_literals_variable_identity_and_recursive_restrictions() {
    let f = Fixture::new(false);
    for name in ["type_sequence_membership","structural_type_shapes",TypeSupport::NAME,TypePresentationSupport::NAME,TypeRestrictionSupport::NAME] {
        f.base.check(f.base.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    let views = f.base.rows::<TypePresentation>(); assert_eq!(views.len(),2);
    assert!(views.iter().all(|v| v.term == f.term.id())); assert_ne!(views[0].id(),views[1].id());
    let mut renamed = f.variable.clone(); renamed.module = ProviderModule::Bundled { provider: f.variable.provider, bundle: lctx_model::domain::calls::ModuleBundle::Typeshed, name: "unrelated".into() }.id(); assert_ne!(renamed.id(),f.variable.id());
    let literal = f.base.rows::<Literal>().into_iter().find(|v| matches!(v,Literal::Integer { .. })).unwrap();
    assert_eq!(Literal::decode(Batch::new(&f.base.model,vec![literal.clone()], &budget()).unwrap().arrow()).unwrap(),vec![literal]);
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

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}

/// Supports over one shared wide term walk it once per provider key: 600 presentations of a
/// 2,000-member union were about 1.2M cumulative closure steps, which a global cap refused as an
/// invalid model (resource review F03).
#[test]
fn shared_type_closures_are_verified_once() {
    use lctx_model::domain::attribution::Fidelity;
    let mut f = Fixture::new(false);
    let literals: Vec<_> = (0..2000).map(|n| Literal::Integer { decimal: n.to_string() }).collect();
    let terms: Vec<_> = literals.iter().map(|l| TypeTerm::Literal { value: l.id() }).collect();
    let (union, members) = TypeSequence::new(&terms.iter().map(|t| (TypeChildRole::Member, t.id())).collect::<Vec<_>>()).unwrap();
    let wide = TypeTerm::Union { members: union.id() };
    let base = f.base.rows::<TypePresentation>()[0].clone();
    let support = f.base.rows::<TypePresentationSupport>()[0].clone();
    let presentations: Vec<_> = (0..600).map(|n| TypePresentation { term: wide.id(), display: format!("wide{n}"), ..base.clone() }).collect();
    let supports: Vec<_> = presentations.iter().map(|p| TypePresentationSupport { assertion: p.id(), ..support.clone() }).collect();
    macro_rules! append { ($ty:ty, $rows:expr) => { let mut rows = f.base.rows::<$ty>(); rows.extend($rows); f.base.put(rows); }; }
    append!(Literal, literals); append!(TypeTerm, terms.into_iter().chain([wide])); append!(TypeSequence, vec![union]);
    append!(TypePresentation, presentations); append!(TypePresentationSupport, supports);
    let mut all = f.base.rows::<TypeSequenceMember>(); all.extend(members); f.put_members(all);
    f.base.check(&TypePresentationSupport::invariants()[0]).unwrap();
    // The memo is keyed by fidelity: an opaque term verified for display-only support still
    // refuses a structural support of the same term.
    let mut g = Fixture::new(false); g.opaque(false, true, Fidelity::DisplayOnly);
    g.base.check(&TypeSupport::invariants()[0]).unwrap();
    let observation = TypeObservation { role: TypeRole::CallResult, ..g.base.rows::<TypeObservation>()[0].clone() };
    let structural = TypeSupport { assertion: observation.id(), fidelity: Fidelity::NativeStructural, ..g.base.rows::<TypeSupport>()[0].clone() };
    let mut observations = g.base.rows::<TypeObservation>(); observations.push(observation); g.base.put(observations);
    let mut supports = g.base.rows::<TypeSupport>(); supports.push(structural); g.base.put(supports);
    let error = g.base.check(&TypeSupport::invariants()[0]).unwrap_err();
    assert!(error.to_string().contains("display-only"), "{error}");
}
