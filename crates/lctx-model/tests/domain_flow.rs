#[path = "fixtures/flow.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{flow::*, transfer::TransferKind, *};

#[test]
fn raw_flow_preserves_nominal_places_attribution_and_unbound_alternatives() {
    let mut f = Fixture::new();
    assert_eq!(f.reaching.use_, f.use_.id());
    for name in [
        "flow_reaching_places",
        "flow_source_structure",
        FlowUseSupport::NAME,
        FlowDefinitionSupport::NAME,
        FlowReachingSupport::NAME,
        FlowValueSupport::NAME,
        FlowRegionSupport::NAME,
    ] {
        f.base
            .check(
                f.base
                    .model
                    .invariants()
                    .iter()
                    .find(|i| i.name == name)
                    .unwrap(),
            )
            .unwrap();
    }
    let unbound = ReachingDefinition::Unbound;
    let alternative = FlowReachingObservation {
        target: unbound.id(),
        ..f.reaching.clone()
    };
    assert_ne!(alternative.id(), f.reaching.id());
    let support = FlowReachingSupport {
        assertion: alternative.id(),
        ..f.reaching_support.clone()
    };
    let mut targets = f.base.rows::<ReachingDefinition>();
    targets.push(unbound);
    f.base.put(targets);
    f.base.put(vec![f.reaching.clone(), alternative]);
    f.base.put(vec![f.reaching_support.clone(), support]);
    f.base.check(&lctx_model::domain::validation::invariants_for::<FlowUse>()[0]).unwrap();
    f.base.check(&lctx_model::domain::validation::invariants_for::<FlowReachingSupport>()[0]).unwrap();
    let mut value = f.base.rows::<FlowValueObservation>().pop().unwrap();
    value.through_call = true;
    assert!(value.validate().is_err());
    value.transfer = TransferKind::Derived;
    value.validate().unwrap();
}

#[test]
fn reaching_and_support_checks_include_the_definition_place_root() {
    let mut f = Fixture::new();
    f.foreign_place();
    assert!(f.base.check(&lctx_model::domain::validation::invariants_for::<FlowUse>()[0]).is_err());
    assert!(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<FlowDefinitionSupport>()[0])
            .is_err()
    );
    assert!(f.base.check(&lctx_model::domain::validation::invariants_for::<FlowReachingSupport>()[0]).is_err());
}

#[test]
fn broad_input_coverage_does_not_authorize_cross_file_flow_structure() {
    use lctx_model::domain::{assertion::*, input::InputRevision, lexical::*, source::*};
    for case in 0..6 {
        let mut f = Fixture::new();
        let scope = CoverageScope::Input {
            input: f.base.rows::<InputRevision>()[0].id(),
        };
        let mut scopes = f.base.rows::<CoverageScope>();
        scopes.push(scope.clone());
        f.base.put(scopes);
        let q = AssertionQualification {
            scope: scope.id(),
            ..f.base.rows::<AssertionQualification>()[0].clone()
        };
        let mut qs = f.base.rows::<AssertionQualification>();
        qs.push(q.clone());
        f.base.put(qs);
        let other = Occurrence {
            source: f.base.foreign.source,
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ModModule,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let foreign_scope = LexicalScope {
            owner: other.id(),
            kind: LexicalScopeKind::Module,
        };
        let mut occurrences = f.base.rows::<Occurrence>();
        occurrences.push(other);
        f.base.put(occurrences);
        let mut scopes = f.base.rows::<LexicalScope>();
        scopes.push(foreign_scope.clone());
        f.base.put(scopes);
        macro_rules! change {
            ($row:ty,$support:ty,$field:ident,$value:expr) => {{
                let mut row = f.base.rows::<$row>().pop().unwrap();
                row.qualification = q.id();
                row.$field = $value;
                let mut support = f.base.rows::<$support>().pop().unwrap();
                support.assertion = row.id();
                f.base.put(vec![row]);
                f.base.put(vec![support]);
                // Every source is acquired by this input: authorization alone cannot detect F01.
                f.base.check(&lctx_model::domain::validation::invariants_for::<$support>()[0]).unwrap();
            }};
        }
        match case {
            0 => change!(
                FlowUseObservation,
                FlowUseSupport,
                scope,
                foreign_scope.id()
            ),
            1 => change!(
                FlowDefinitionObservation,
                FlowDefinitionSupport,
                scope,
                foreign_scope.id()
            ),
            2 => change!(
                FlowDefinitionObservation,
                FlowDefinitionSupport,
                value,
                Some(f.base.foreign.id())
            ),
            3 => change!(
                FlowRegionObservation,
                FlowRegionSupport,
                scope,
                foreign_scope.id()
            ),
            4 => change!(
                FlowValueObservation,
                FlowValueSupport,
                sink,
                f.base.foreign.id()
            ),
            _ => {
                // An acquired Place root in another file remains distinct from local source geometry.
                f.foreign_place();
                change!(FlowDefinitionObservation,FlowDefinitionSupport,scope,f.base.rows::<LexicalScope>().into_iter().find(|s| s.kind == LexicalScopeKind::Module && s.id() != foreign_scope.id()).unwrap().id());
            }
        }
        assert_eq!(f.base.check(&lctx_model::domain::validation::invariants_for::<FlowUse>()[1]).is_ok(), case == 5);
    }
}

#[test]
fn raw_tests_leaves_and_call_paths_validate_without_provider_indices() {
    use lctx_model::domain::{assertion::*, calls::*, conditions::*};
    let mut f = Fixture::new();
    f.extended();
    for name in [
        "flow_source_structure",
        "flow_call_path_structure",
        FlowTestSupport::NAME,
        FlowTestLeafSupport::NAME,
        FlowAttributeLoadSupport::NAME,
        FlowValuePathSupport::NAME,
    ] {
        f.base
            .check(
                f.base
                    .model
                    .invariants()
                    .iter()
                    .find(|i| i.name == name)
                    .unwrap(),
            )
            .unwrap();
    }
    for case in 0..8 {
        let mut f = Fixture::new();
        f.extended();
        match case {
            0 => {
                let mut step = f.base.rows::<FlowCallStep>().pop().unwrap();
                step.ordinal = 1;
                f.base.put(vec![step]);
            }
            1 => {
                let mut step = f.base.rows::<FlowCallStep>().pop().unwrap();
                step.role = FlowCallOperandRole::Callee;
                f.base.put(vec![step]);
            }
            2 => {
                let mut step = f.base.rows::<FlowCallStep>().pop().unwrap();
                step.operand = f.base.foreign.id();
                f.base.put(vec![step]);
            }
            3 => f.base.put::<FlowValuePathObservation>(vec![]),
            4 => {
                let mut path = f.base.rows::<FlowCallPath>().pop().unwrap();
                path.steps = ContentHash::of(b"wrong order");
                f.base.put(vec![path]);
            }
            5 => {
                let mut leaf = f.base.rows::<FlowTestLeafObservation>().pop().unwrap();
                leaf.operand = Some(f.base.foreign.id());
                f.base.put(vec![leaf]);
            }
            6 => {
                let mut atom = f.base.rows::<EvaluationAtom>().pop().unwrap();
                atom.context = f.base.rows::<AssertionQualification>()[0].context;
                atom.evaluation = f.base.foreign.id();
                let mut leaf = f.base.rows::<FlowTestLeafObservation>().pop().unwrap();
                leaf.atom = atom.id();
                f.base.put(vec![atom]);
                f.base.put(vec![leaf]);
            }
            _ => {
                let mut link = f.base.rows::<FlowValuePathObservation>().pop().unwrap();
                link.qualification = AssertionQualification {
                    modality: lctx_model::domain::attribution::Modality::Candidate,
                    ..f.base.rows::<AssertionQualification>()[0].clone()
                }
                .id();
                f.base.put(vec![link]);
            }
        }
        let name = if case == 5 || case == 6 {
            "flow_source_structure"
        } else {
            "flow_call_path_structure"
        };
        assert!(
            f.base
                .check(
                    f.base
                        .model
                        .invariants()
                        .iter()
                        .find(|i| i.name == name)
                        .unwrap()
                )
                .is_err(),
            "case {case}"
        );
    }
    assert!(FlowCallPath::new(&[]).is_err());
    let call = f.base.rows::<CallSyntax>()[0].site;
    assert!(FlowCallPath::new(&vec![(call, call, FlowCallOperandRole::Callee); 257]).is_err());
}

#[test]
fn call_paths_preserve_every_crossing_from_the_originating_use() {
    use lctx_model::domain::{calls::*, source::*};
    let mut f = Fixture::new();
    f.nested_path();
    let invariant = f
        .base
        .model
        .invariants()
        .iter()
        .find(|i| i.name == "flow_call_path_structure")
        .unwrap()
        .clone();
    f.base.check(&invariant).unwrap();
    for case in 0..6 {
        let mut f = Fixture::new();
        f.nested_path();
        let mut steps = f.base.rows::<FlowCallStep>();
        steps.sort_by_key(|s| s.ordinal);
        let original: Vec<_> = steps.iter().map(|s| (s.call, s.operand, s.role)).collect();
        let tuples = match case {
            0 => vec![original[0]],                           // omitted inner call
            1 => vec![original[1]],                           // omitted outer call
            2 => vec![original[0], original[0], original[1]], // repeated crossing
            3 => vec![original[1], original[0]],              // reversed order
            4 => {
                // f(g(x),h(y)): both operands are real syntax arguments, but the selected
                // outer operand is the sibling rather than the ancestor of g(x).
                let inner = f
                    .base
                    .rows::<Occurrence>()
                    .into_iter()
                    .find(|o| o.id() == original[1].0)
                    .unwrap();
                let sibling = Occurrence {
                    structural_path: vec![0, 1, 2],
                    ..inner
                };
                let outer = f
                    .base
                    .rows::<CallSyntax>()
                    .into_iter()
                    .find(|c| c.site == original[0].0)
                    .unwrap();
                let mut arguments = f.base.rows::<CallArgument>();
                arguments.push(CallArgument {
                    call: outer.id(),
                    ordinal: 1,
                    kind: ArgumentKind::Positional,
                    keyword: None,
                    value: sibling.id(),
                });
                f.base.put(arguments);
                let mut occurrences = f.base.rows::<Occurrence>();
                occurrences.push(sibling.clone());
                f.base.put(occurrences);
                vec![
                    (original[0].0, sibling.id(), FlowCallOperandRole::Argument),
                    original[1],
                ]
            }
            _ => {
                // A different use cannot borrow this otherwise valid path.
                let mut uses = f.base.rows::<FlowUse>();
                let row = uses.iter_mut().find(|u| u.id() == f.use_.id()).unwrap();
                row.occurrence = f.base.foreign.id();
                let replacement = row.id();
                f.base.put(uses);
                let mut values = f.base.rows::<FlowValueObservation>();
                let value = values.iter_mut().find(|v| v.through_call).unwrap();
                value.use_ = replacement;
                let mut link = f.base.rows::<FlowValuePathObservation>()[0].clone();
                link.value = value.id();
                f.base.put(values);
                f.base.put(vec![link]);
                original.clone()
            }
        };
        let (path, new_steps) = FlowCallPath::new(&tuples).unwrap();
        steps = new_steps;
        f.base.put(vec![path.clone()]);
        f.base.put(steps);
        let mut link = f.base.rows::<FlowValuePathObservation>()[0].clone();
        link.path = path.id();
        f.base.put(vec![link]);
        assert!(f.base.check(&invariant).is_err(), "case {case}");
    }
}
