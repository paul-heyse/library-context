#[path = "fixtures/flow.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,flow::*,transfer::TransferKind};

#[test]
fn raw_flow_preserves_nominal_places_attribution_and_unbound_alternatives() {
    let mut f = Fixture::new();
    assert_eq!(f.reaching.use_,f.use_.id());
    for name in ["flow_reaching_places","flow_source_structure",FlowUseSupport::NAME,FlowDefinitionSupport::NAME,FlowReachingSupport::NAME,FlowValueSupport::NAME,FlowRegionSupport::NAME] {
        f.base.check(f.base.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    let unbound = ReachingDefinition::Unbound;
    let alternative = FlowReachingObservation { target: unbound.id(),..f.reaching.clone() };
    assert_ne!(alternative.id(),f.reaching.id());
    let support = FlowReachingSupport { assertion: alternative.id(),..f.reaching_support.clone() };
    let mut targets = f.base.rows::<ReachingDefinition>(); targets.push(unbound); f.base.put(targets);
    f.base.put(vec![f.reaching.clone(),alternative]); f.base.put(vec![f.reaching_support.clone(),support]);
    f.base.check(&FlowUse::invariants()[0]).unwrap(); f.base.check(&FlowReachingSupport::invariants()[0]).unwrap();
    let mut value = f.base.rows::<FlowValueObservation>().pop().unwrap(); value.through_call = true;
    assert!(value.validate().is_err()); value.transfer = TransferKind::Derived; value.validate().unwrap();
}

#[test]
fn reaching_and_support_checks_include_the_definition_place_root() {
    let mut f = Fixture::new(); f.foreign_place();
    assert!(f.base.check(&FlowUse::invariants()[0]).is_err());
    assert!(f.base.check(&FlowDefinitionSupport::invariants()[0]).is_err());
    assert!(f.base.check(&FlowReachingSupport::invariants()[0]).is_err());
}

#[test]
fn broad_input_coverage_does_not_authorize_cross_file_flow_structure() {
    use lctx_model::domain::{assertion::*,lexical::*,source::*,input::InputRevision};
    for case in 0..6 {
        let mut f = Fixture::new();
        let scope = CoverageScope::Input { input: f.base.rows::<InputRevision>()[0].id() };
        let mut scopes = f.base.rows::<CoverageScope>(); scopes.push(scope.clone()); f.base.put(scopes);
        let q = AssertionQualification { scope: scope.id(),..f.base.rows::<AssertionQualification>()[0].clone() };
        let mut qs = f.base.rows::<AssertionQualification>(); qs.push(q.clone()); f.base.put(qs);
        let other = Occurrence { source: f.base.foreign.source,start: 0,end: 1,syntax_kind: SyntaxKind::ModModule,role: OccurrenceRole::Syntax,structural_path: vec![0] };
        let foreign_scope = LexicalScope { owner: other.id(),kind: LexicalScopeKind::Module };
        let mut occurrences = f.base.rows::<Occurrence>(); occurrences.push(other); f.base.put(occurrences);
        let mut scopes = f.base.rows::<LexicalScope>(); scopes.push(foreign_scope.clone()); f.base.put(scopes);
        macro_rules! change {
            ($row:ty,$support:ty,$field:ident,$value:expr) => {{
                let mut row = f.base.rows::<$row>().pop().unwrap(); row.qualification = q.id(); row.$field = $value;
                let mut support = f.base.rows::<$support>().pop().unwrap(); support.assertion = row.id();
                f.base.put(vec![row]); f.base.put(vec![support]);
                // Every source is acquired by this input: authorization alone cannot detect F01.
                f.base.check(&<$support>::invariants()[0]).unwrap();
            }};
        }
        match case {
            0 => change!(FlowUseObservation,FlowUseSupport,scope,foreign_scope.id()),
            1 => change!(FlowDefinitionObservation,FlowDefinitionSupport,scope,foreign_scope.id()),
            2 => change!(FlowDefinitionObservation,FlowDefinitionSupport,value,Some(f.base.foreign.id())),
            3 => change!(FlowRegionObservation,FlowRegionSupport,scope,foreign_scope.id()),
            4 => change!(FlowValueObservation,FlowValueSupport,sink,f.base.foreign.id()),
            _ => {
                // An acquired Place root in another file remains distinct from local source geometry.
                f.foreign_place();
                change!(FlowDefinitionObservation,FlowDefinitionSupport,scope,f.base.rows::<LexicalScope>().into_iter().find(|s| s.kind == LexicalScopeKind::Module && s.id() != foreign_scope.id()).unwrap().id());
            },
        }
        assert_eq!(f.base.check(&FlowUse::invariants()[1]).is_ok(),case == 5);
    }
}
