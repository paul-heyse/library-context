#[path = "fixtures/stability.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    attribution::*,
    conditions::{rebase::RootBinding, stability::*, *},
    flow::*,
    lexical::BindingEventKind,
    value::*,
    *,
};
use std::collections::BTreeMap;

fn refused(fixture: &Fixture, expected: &str) {
    let error = fixture.validate().unwrap_err().to_string();
    assert!(
        error.contains(expected),
        "expected `{expected}`, got `{error}`"
    );
}

#[test]
fn a_witnessed_formal_guard_is_restated_over_the_bound_actual() {
    let f = Fixture::new();
    f.validate().unwrap();
    let witnesses = BTreeMap::from([(f.guard.id(), f.witness.clone())]);
    let bound = BTreeMap::from([(f.formal.id(), RootBinding::Actual(f.arguments[0].clone()))]);
    // A compound callee condition keeps its shape: the formal guard becomes a BoundGuard, a
    // callee-local guard an opaque InvokedGuard, and nothing becomes unconditional.
    let local = EvaluationAtom {
        evaluation: f.evaluation.id(),
        context: f.context.id(),
        predicate: Predicate::Opaque {
            text: "local_flag".into(),
        }
        .id(),
        operand: None,
    };
    let mut g = Fixture::new();
    let mut atoms = g.rows::<EvaluationAtom>();
    atoms.push(local.clone());
    g.put(atoms);
    let mut predicates = g.rows::<Predicate>();
    predicates.push(Predicate::Opaque {
        text: "local_flag".into(),
    });
    g.put(predicates);
    let source = Diagram::from_atom(f.guard.id())
        .and(&Diagram::from_atom(local.id()).not().unwrap())
        .unwrap();
    let rebased = g.substitute(Some(&witnesses), &bound, &source).unwrap();
    let by = |bound: bool| {
        rebased
            .atoms
            .iter()
            .find(|a| {
                rebased.predicates.iter().any(|p| {
                    p.id() == a.predicate && matches!(p, Predicate::BoundGuard { .. }) == bound
                })
            })
            .unwrap()
    };
    let (bound_guard, invoked) = (by(true), by(false));
    assert_eq!(
        rebased.condition.id(),
        Diagram::from_atom(bound_guard.id())
            .and(&Diagram::from_atom(invoked.id()).not().unwrap())
            .unwrap()
            .id()
    );
    assert_eq!(rebased.substitutions.len(), 1);
    assert_eq!(
        bound_guard.operand,
        Some(
            Place {
                root: PlaceRoot::Occurrence {
                    occurrence: f.actual.id()
                }
                .id(),
                path: AccessPath::empty().id()
            }
            .id()
        )
    );
    let guard = Diagram::from_atom(f.guard.id());
    assert_eq!(
        f.substitute(Some(&BTreeMap::new()), &bound, &guard)
            .unwrap_err(),
        ObligationKind::ConditionTransferUnsupported,
        "no witness: refused, never true"
    );
    assert_eq!(
        f.substitute(None, &bound, &guard).unwrap_err(),
        ObligationKind::NotRequested
    );
    let defaulted = BTreeMap::from([(f.formal.id(), RootBinding::Default)]);
    assert_eq!(
        f.substitute(Some(&witnesses), &defaulted, &guard)
            .unwrap_err(),
        ObligationKind::DefaultStabilityUnknown
    );
    let mut truthy = Fixture::new();
    truthy.predicate = Predicate::Truthy;
    truthy.guard.predicate = truthy.predicate.id();
    let witnesses_t = BTreeMap::from([(truthy.guard.id(), truthy.witness.clone())]);
    assert_eq!(
        truthy
            .substitute(
                Some(&witnesses_t),
                &bound,
                &Diagram::from_atom(truthy.guard.id())
            )
            .unwrap_err(),
        ObligationKind::ConditionTransferUnsupported
    );
    let mut attribute = Fixture::new();
    let segment = PathSegment::Attribute {
        name: "value".into(),
    };
    attribute.place = Place {
        root: attribute.formal.id(),
        path: AccessPath {
            first: Some(segment.id()),
            second: None,
            unknown_suffix: false,
        }
        .id(),
    };
    attribute.guard.operand = Some(attribute.place.id());
    let witnesses_a = BTreeMap::from([(attribute.guard.id(), attribute.witness.clone())]);
    assert_eq!(
        attribute
            .substitute(
                Some(&witnesses_a),
                &bound,
                &Diagram::from_atom(attribute.guard.id())
            )
            .unwrap_err(),
        ObligationKind::ConditionTransferUnsupported
    );
}

#[test]
fn stored_witnesses_and_substitutions_refuse_what_they_cannot_justify() {
    let mut f = Fixture::new();
    f.put::<GuardSubstitution>(vec![]);
    refused(&f, "a bound guard needs a witnessed substitution");
    let mut f = Fixture::new();
    let other = FlowReachingObservation {
        target: ReachingDefinition::Unbound.id(),
        ..f.reaching.clone()
    };
    let mut targets = f.rows::<ReachingDefinition>();
    targets.push(ReachingDefinition::Unbound);
    f.put(targets);
    let mut observed = f.rows::<FlowReachingObservation>();
    observed.push(other.clone());
    f.put(observed);
    let mut supports = f.rows::<FlowReachingSupport>();
    supports.push(FlowReachingSupport {
        assertion: other.id(),
        ..supports[0].clone()
    });
    f.put(supports);
    refused(&f, "more than one reaching definition");
    // `def reset(): nonlocal timeout; timeout = None` then `reset(); if timeout is None`: the nested
    // scope's binding reaches the read beside the parameter, or alone; neither is a witness.
    let mut f = Fixture::new();
    let nested = FlowReachingObservation {
        target: ReachingDefinition::Nested.id(),
        ..f.reaching.clone()
    };
    let mut targets = f.rows::<ReachingDefinition>();
    targets.push(ReachingDefinition::Nested);
    f.put(targets);
    let mut observed = f.rows::<FlowReachingObservation>();
    observed.push(nested.clone());
    f.put(observed);
    let mut supports = f.rows::<FlowReachingSupport>();
    supports.push(FlowReachingSupport {
        assertion: nested.id(),
        ..supports[0].clone()
    });
    f.put(supports);
    refused(&f, "more than one reaching definition");
    let mut f = Fixture::new();
    f.target = ReachingDefinition::Nested;
    f.reaching.target = f.target.id();
    f.store_flow();
    f.store_substitution();
    refused(&f, "unbound or from a nested scope");
    let mut f = Fixture::new();
    f.coverage = ProviderCoverage {
        status: CoverageStatus::Partial,
        reason: Some(ObligationKind::OutsideProviderModel),
        ..f.coverage.clone()
    };
    f.store_flow();
    f.store_substitution();
    refused(&f, "complete flow coverage");
    let mut f = Fixture::new();
    f.definition_observation.kind = BindingEventKind::Assignment;
    f.store_flow();
    f.store_substitution();
    refused(&f, "not the formal's parameter definition");
    let mut f = Fixture::new();
    let mut atoms = f.rows::<EvaluationAtom>();
    let bound = atoms
        .iter()
        .position(|a| a.evaluation == f.site.id())
        .unwrap();
    let wrong = Place {
        root: PlaceRoot::Occurrence {
            occurrence: f.callee_name.id(),
        }
        .id(),
        path: AccessPath::empty().id(),
    };
    atoms[bound].operand = Some(wrong.id());
    let mut substitutions = f.rows::<GuardSubstitution>();
    substitutions[0].atom = atoms[bound].id();
    f.put(atoms);
    f.put(substitutions);
    let mut roots = f.rows::<PlaceRoot>();
    roots.push(PlaceRoot::Occurrence {
        occurrence: f.callee_name.id(),
    });
    f.put(roots);
    let mut places = f.rows::<Place>();
    places.push(wrong);
    f.put(places);
    refused(&f, "must test the argument's value at its call");
}

#[test]
fn only_a_bound_guard_may_stand_between_an_instantiation_and_a_formal() {
    for through_bound in [true, false] {
        let mut f = Fixture::new();
        let mut atoms = f.rows::<EvaluationAtom>();
        let bound = atoms
            .iter()
            .find(|a| a.evaluation == f.site.id())
            .unwrap()
            .id();
        let predicate = Predicate::InvokedGuard {
            source: if through_bound { bound } else { f.guard.id() },
        };
        atoms.push(EvaluationAtom {
            evaluation: f.site.id(),
            context: f.context.id(),
            predicate: predicate.id(),
            operand: None,
        });
        let mut predicates = f.rows::<Predicate>();
        predicates.push(predicate);
        f.put(atoms);
        f.put(predicates);
        if through_bound {
            f.validate().unwrap();
        } else {
            refused(&f, "needs binding and stability evidence");
        }
    }
}
