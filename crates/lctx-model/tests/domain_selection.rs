//! Retained independent finite requirement controls, with current nominal identities.
use lctx_model::domain::{
    conditions::{Diagram, EvaluationAtom},
    normalized::Rows,
    resources::ResourceBudget,
    selection::{algebra::*, *},
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([n; 16].into_iter()))
    .unwrap()
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(16 << 20).unwrap()
}
fn ctx(n: u8) -> ClaimContext {
    ClaimContext::Declaration(Context::Signature {
        member: id(1),
        candidate: id(9),
        invocation: id(n),
        analysis: id(2),
    })
}
fn obs(n: u8, value: Option<bool>) -> Observation {
    Observation {
        context: ctx(n),
        basis: EvidenceBasis::SourceDeclaration,
        value,
        evidence: vec![Witness::Invocation { invocation: id(n) }],
        admissible: vec![ctx(n)],
    }
}
fn req(q: Quantifier) -> Requirement {
    Requirement {
        predicate: Predicate::DeclaresParameter {
            name: "timeout".into(),
        },
        quantifier: q,
    }
}
fn domain(observations: Vec<Observation>, closed: bool) -> Domain {
    Domain {
        corpus_complete: closed,
        analyzer_complete: closed,
        closure: vec![Witness::Domain { domain: id(99) }],
        observations,
    }
}
#[test]
fn overloads_are_separate_claims_and_quantifiers_differ() {
    let b = budget();
    let d = domain(vec![obs(3, Some(true)), obs(4, Some(false))], true);
    assert_eq!(
        classify(&req(Quantifier::AnyApplicable), &d, &b)
            .unwrap()
            .outcome,
        Outcome::Supported
    );
    assert_eq!(
        classify(&req(Quantifier::AllApplicable), &d, &b)
            .unwrap()
            .outcome,
        Outcome::Contradicted
    );
    assert_eq!(b.reserved(), 0);
}
#[test]
fn complete_empty_domain_is_never_vacuously_supported_and_absence_needs_closure() {
    let b = budget();
    for quantifier in [Quantifier::AnyApplicable, Quantifier::AllApplicable] {
        assert_eq!(
            classify(&req(quantifier), &domain(vec![], true), &b)
                .unwrap()
                .reason,
            Reason::NoApplicableDomain
        );
        assert_eq!(
            classify(&req(quantifier), &domain(vec![obs(3, None)], true), &b)
                .unwrap()
                .outcome,
            Outcome::Unresolved
        );
        let mut d = domain(vec![], true);
        d.closure.clear();
        assert_eq!(
            classify(&req(quantifier), &d, &b).unwrap().outcome,
            Outcome::Unresolved
        );
    }
    assert_eq!(
        classify(
            &req(Quantifier::AnyApplicable),
            &domain(vec![obs(3, Some(false))], false),
            &b
        )
        .unwrap()
        .outcome,
        Outcome::Unresolved
    );
}
#[test]
fn comparable_conflict_precedes_existential_witness_but_clean_counterexample_wins_universal() {
    let b = budget();
    let d = domain(
        vec![
            obs(3, Some(true)),
            obs(3, Some(false)),
            obs(4, Some(true)),
            obs(5, Some(false)),
        ],
        true,
    );
    assert_eq!(
        classify(&req(Quantifier::AnyApplicable), &d, &b)
            .unwrap()
            .outcome,
        Outcome::Conflicting
    );
    assert_eq!(
        classify(&req(Quantifier::AllApplicable), &d, &b)
            .unwrap()
            .outcome,
        Outcome::Contradicted
    );
}
#[test]
fn entire_conjunction_intersection_is_required_and_missing_applicability_remains_unknown() {
    let b = budget();
    let r = req(Quantifier::AnyApplicable);
    let mut results = Vec::new();
    for ns in [[3, 4], [4, 5], [3, 5]] {
        results.push(
            classify(
                &r,
                &domain(ns.into_iter().map(|n| obs(n, Some(true))).collect(), true),
                &b,
            )
            .unwrap(),
        );
    }
    assert_eq!(
        joint(
            &results.iter().collect::<Vec<_>>(),
            JointPolicy::RequireCompatible,
            &b
        )
        .unwrap(),
        JointApplicability::ContradictoryModeledContext
    );
    assert_eq!(
        joint(
            &[&results[0], &results[1]],
            JointPolicy::RequireCompatible,
            &b
        )
        .unwrap(),
        JointApplicability::CompatibleModeledContext
    );
    results[1].clear_admissible();
    assert_eq!(
        joint(
            &[&results[0], &results[1]],
            JointPolicy::RequireCompatible,
            &b
        )
        .unwrap(),
        JointApplicability::NotEstablished
    );
    assert_eq!(
        joint(&[], JointPolicy::IndependentRecords, &b).unwrap(),
        JointApplicability::IndependentRecords
    );
}
#[test]
fn different_analysis_contexts_cannot_conflict_or_establish_joint_applicability() {
    let b = budget();
    let a = classify(
        &req(Quantifier::AnyApplicable),
        &domain(vec![obs(3, Some(true))], true),
        &b,
    )
    .unwrap();
    let other = ClaimContext::Declaration(Context::Signature {
        member: id(1),
        candidate: id(9),
        invocation: id(3),
        analysis: id(7),
    });
    let d = domain(
        vec![Observation {
            context: other.clone(),
            basis: EvidenceBasis::SourceDeclaration,
            value: Some(true),
            evidence: vec![Witness::Invocation { invocation: id(3) }],
            admissible: vec![other],
        }],
        true,
    );
    let other = classify(&req(Quantifier::AnyApplicable), &d, &b).unwrap();
    assert_eq!(
        joint(&[&a, &other], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::NotEstablished
    );
}
fn runtime(evaluation: u8, instance: u8) -> RuntimeContext {
    RuntimeContext {
        declaration: match ctx(3) {
            ClaimContext::Declaration(c) => c,
            _ => unreachable!(),
        },
        owner: id(4),
        instance: id(instance),
        evaluation: id(evaluation),
        model: id(8),
    }
}
fn runtime_claim(context: &RuntimeContext, b: &ResourceBudget) -> Classified {
    let context = ClaimContext::Runtime(context.clone());
    classify(
        &req(Quantifier::AnyApplicable),
        &domain(
            vec![Observation {
                context: context.clone(),
                basis: EvidenceBasis::BoundedModel,
                value: Some(true),
                evidence: vec![Witness::Qualification {
                    qualification: id(9),
                }],
                admissible: vec![context],
            }],
            true,
        ),
        b,
    )
    .unwrap()
}
#[test]
fn runtime_conditions_require_actual_atom_evaluation_and_exact_instance_model_identity() {
    let b = budget();
    let context = runtime(6, 5);
    let mut a = runtime_claim(&context, &b);
    let mut other = runtime_claim(&context, &b);
    assert_eq!(
        joint(&[&a, &other], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::NotEstablished
    );
    let mut atoms = Rows::new(&b);
    let atom = atoms
        .insert(EvaluationAtom {
            evaluation: id(6),
            context: id(2),
            predicate: id(10),
            operand: None,
        })
        .unwrap();
    let positive = Diagram::from_atom(atom);
    let negative = positive.not().unwrap();
    a.condition(RuntimeCondition::admit(context.clone(), positive.clone(), &atoms, &b).unwrap())
        .unwrap();
    other
        .condition(RuntimeCondition::admit(context.clone(), negative, &atoms, &b).unwrap())
        .unwrap();
    assert_eq!(
        joint(&[&a, &other], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::ContradictoryModeledContext
    );
    let wrong = runtime(11, 5);
    assert!(RuntimeCondition::admit(wrong, positive.clone(), &atoms, &b).is_err());
    let other_instance = runtime(6, 12);
    let mut separate = runtime_claim(&other_instance, &b);
    separate
        .condition(RuntimeCondition::admit(other_instance, positive, &atoms, &b).unwrap())
        .unwrap();
    assert_eq!(
        joint(&[&a, &separate], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::NotEstablished
    );
}
#[test]
fn canonical_request_digest_ignores_order_and_duplicates_but_retains_quantifier_and_policy() {
    let a = req(Quantifier::AnyApplicable);
    let other = req(Quantifier::AllApplicable);
    let x = Selection {
        requirements: vec![a.clone(), other.clone()],
        mode: Mode::Discovery,
        joint: JointPolicy::IndependentRecords,
    };
    let y = Selection {
        requirements: vec![other, a.clone(), a],
        ..x.clone()
    };
    assert_eq!(selection_digest(&x).unwrap(), selection_digest(&y).unwrap());
    let strict = Selection {
        mode: Mode::Strict,
        ..x.clone()
    };
    assert_ne!(
        selection_digest(&x).unwrap(),
        selection_digest(&strict).unwrap()
    );
    assert!(
        selection_digest(&Selection {
            requirements: vec![req(Quantifier::AnyApplicable); 17],
            ..x
        })
        .is_err()
    );
}
#[test]
fn unsupported_bare_booleans_and_budget_refusals_cannot_become_witnesses() {
    let b = budget();
    let mut observation = obs(3, Some(true));
    observation.evidence.clear();
    assert!(
        classify(
            &req(Quantifier::AnyApplicable),
            &domain(vec![observation], true),
            &b
        )
        .is_err()
    );
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        classify(
            &req(Quantifier::AnyApplicable),
            &domain(vec![obs(3, Some(true))], true),
            &tiny
        ),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn binding_signature_join_uses_candidate_identity_and_query_text_is_bounded() {
    let b = budget();
    let a = classify(
        &req(Quantifier::AnyApplicable),
        &domain(vec![obs(3, Some(true))], true),
        &b,
    )
    .unwrap();
    let binding = ClaimContext::Declaration(Context::Binding {
        member: id(1),
        candidate: id(9),
        analysis: id(2),
    });
    let result = classify(
        &req(Quantifier::AnyApplicable),
        &domain(
            vec![Observation {
                context: binding.clone(),
                basis: EvidenceBasis::SourceDeclaration,
                value: Some(true),
                evidence: vec![Witness::Candidate { candidate: id(9) }],
                admissible: vec![binding],
            }],
            true,
        ),
        &b,
    )
    .unwrap();
    assert_eq!(
        joint(&[&a, &result], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::CompatibleModeledContext
    );
    let foreign = ClaimContext::Declaration(Context::Binding {
        member: id(1),
        candidate: id(10),
        analysis: id(2),
    });
    let result = classify(
        &req(Quantifier::AnyApplicable),
        &domain(
            vec![Observation {
                context: foreign.clone(),
                basis: EvidenceBasis::SourceDeclaration,
                value: Some(true),
                evidence: vec![Witness::Candidate { candidate: id(10) }],
                admissible: vec![foreign],
            }],
            true,
        ),
        &b,
    )
    .unwrap();
    assert_eq!(
        joint(&[&a, &result], JointPolicy::RequireCompatible, &b).unwrap(),
        JointApplicability::ContradictoryModeledContext
    );
    assert!(
        Predicate::PublicPath {
            path: vec!["has.separator".into()]
        }
        .validate()
        .is_err()
    );
    assert!(
        Predicate::DeclaresParameter {
            name: "x".repeat(1025)
        }
        .validate()
        .is_err()
    );
    assert!(
        Predicate::ParameterType {
            name: "x".into(),
            r#type: StructuralType::Category { kind: 35 }
        }
        .validate()
        .is_err()
    );
}

#[test]
fn unsupported_facets_are_rejected_before_classification() {
    let b = budget();
    let requirement = Requirement {
        predicate: Predicate::FacetMembership {
            facet: Facet::DelegatesTo,
            value: FacetValue::ParameterName {
                name: "timeout".into(),
            },
        },
        quantifier: Quantifier::AnyApplicable,
    };
    assert!(requirement.predicate.validate().is_err());
    assert!(classify(&requirement, &domain(vec![], false), &b).is_err());
    assert_eq!(b.reserved(), 0);
}
