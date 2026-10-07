//! Static compiler inputs preserve each exact immutable semantic view.
use lctx_model::{
    Domain,
    domain::{dependency_closure::*, input::Package, stages::*, value::Literal, *},
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "closure_probe")]
struct Probe {
    #[model(key)]
    value: Id<Literal>,
    package: Id<Package>,
}
fn order() -> PublicationOrder {
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
    ])
    .unwrap()
}
#[test]
fn separate_views_and_orders_preserve_each_compiler_input() {
    let model = ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap();
    let closure = DependencyClosure::build(
        &model,
        vec![
            ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts),
            ValidationInput::of::<Literal>(&["kind", "id"])
                .at_epoch(PublicationBoundary::Structural),
            ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Local),
        ],
        vec![],
        &[],
        PublicationBoundary::Facts,
        LowerLayerPolicy::OmitInferredOrdinaryFacts,
        &order(),
    )
    .unwrap();
    assert_eq!(closure.requirements.len(), 3);
    assert!(closure.requirements.iter().any(|r| r.prefix()
        == Some(PublicationBoundary::Structural)
        && r.order() == ["kind", "id"]));
    assert!(
        closure
            .requirements
            .iter()
            .any(|r| r.prefix() == Some(PublicationBoundary::Facts))
    );
    assert_eq!(closure.grants.len(), 3);
    for view in [
        PublicationBoundary::Facts,
        PublicationBoundary::Structural,
        PublicationBoundary::Local,
    ] {
        assert!(
            closure
                .grants
                .iter()
                .any(|input| input.prefix() == Some(view))
        );
    }
}
#[test]
fn inferred_facts_omission_preserves_explicit_inputs_and_unfinished_output_refuses() {
    let model = ValidatedModel::declared(vec![
        Relation::of::<Probe>(),
        Relation::of::<Literal>(),
        Relation::of::<Package>(),
    ])
    .unwrap();
    let closure = DependencyClosure::build(
        &model,
        vec![ValidationInput::of::<Probe>(&["id"])],
        vec![RelationUse::completed::<Package>()],
        &[],
        PublicationBoundary::Facts,
        LowerLayerPolicy::OmitInferredOrdinaryFacts,
        &order(),
    )
    .unwrap();
    assert!(
        !closure
            .requirements
            .iter()
            .any(|r| r.name() == Package::NAME)
    );
    assert!(
        closure.grants.iter().any(|r| r.name() == Package::NAME),
        "direct facts remain declared"
    );
    assert!(
        closure
            .requirements
            .iter()
            .any(|r| r.name() == Literal::NAME && r.prefix() == Some(PublicationBoundary::Facts))
    );
    assert!(
        DependencyClosure::build(
            &model,
            vec![ValidationInput::of::<Probe>(&["id"])],
            vec![],
            &[RelationUse::of::<Probe>()],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order()
        )
        .is_err()
    );
    assert!(
        DependencyClosure::build(
            &model,
            vec![ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Analytic)],
            vec![],
            &[],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order()
        )
        .is_err()
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="closure_epoch_probe", invariant_refs=epoch_checks_refs)]
struct EpochProbe {
    #[model(key)]
    value: Id<Literal>,
}
fn epoch_checks() -> Vec<Invariant> {
    vec![Invariant {
        purpose: lctx_model::domain::InvariantPurpose::Admission,
        revision: 1,
        name: "closure_extra_epoch_and_fact_premise",
        inputs: vec![
            ValidationInput::of::<Package>(&["id"]),
            ValidationInput::of::<Literal>(&["kind", "id"]).at_epoch(PublicationBoundary::Facts),
        ],
        create: std::sync::Arc::new(|_| Box::new(EpochCheck)),
    }]
}
struct EpochCheck;
impl InvariantCheck for EpochCheck {
    fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
#[test]
fn extra_invariant_epoch_and_fact_premise_survive_grant_projection() {
    let model = ValidatedModel::validate(
        vec![
            Relation::of::<EpochProbe>(),
            Relation::of::<Package>(),
            Relation::of::<Literal>(),
        ],
        ValidationDefinitions {
            invariants: epoch_checks(),
            publication_checks: vec![],
        },
    )
    .unwrap();
    let build = |direct| {
        DependencyClosure::build(
            &model,
            vec![ValidationInput::of::<EpochProbe>(&["id"])],
            direct,
            &[],
            PublicationBoundary::Local,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order(),
        )
        .unwrap()
    };
    let closure = build(vec![]);
    assert!(
        closure
            .requirements
            .iter()
            .any(|i| i.name() == Package::NAME && i.order() == ["id"])
    );
    assert!(
        closure
            .requirements
            .iter()
            .any(|i| i.name() == Literal::NAME
                && i.prefix() == Some(PublicationBoundary::Facts)
                && i.order() == ["kind", "id"])
    );
    assert!(
        closure
            .requirements
            .iter()
            .any(|i| i.name() == Literal::NAME && i.prefix() == Some(PublicationBoundary::Local))
    );
    assert!(
        !closure.grants.iter().any(|g| g.name() == Package::NAME),
        "upstream validation premise is covered by the frozen Facts checkpoint"
    );
    assert_eq!(
        closure
            .grants
            .iter()
            .find(|g| g.name() == Literal::NAME && g.prefix() == Some(PublicationBoundary::Local))
            .unwrap()
            .prefix(),
        Some(PublicationBoundary::Local)
    );
    assert!(
        build(vec![RelationUse::completed::<Package>()])
            .grants
            .iter()
            .any(|g| g.name() == Package::NAME),
        "an actual consumed fact always needs its own grant"
    );
}

#[test]
fn convenient_grants_traverse_direct_uses_and_retain_explicit_ordinary_facts() {
    let model = ValidatedModel::declared(vec![
        Relation::of::<Probe>(),
        Relation::of::<Literal>(),
        Relation::of::<Package>(),
    ])
    .unwrap();
    let order = order();
    let build = |direct, owned: &[RelationUse], lower| {
        DependencyClosure::grants(
            &model,
            vec![],
            direct,
            owned,
            PublicationBoundary::Facts,
            lower,
            &order,
        )
    };
    let grants = build(
        vec![
            RelationUse::completed::<Probe>(),
            RelationUse::completed::<Package>(),
        ],
        &[],
        LowerLayerPolicy::OmitInferredOrdinaryFacts,
    )
    .unwrap();
    assert_eq!(
        grants.iter().map(|r| r.name()).collect::<Vec<_>>(),
        vec![Probe::NAME, Literal::NAME, Package::NAME]
    );
    assert_eq!(
        grants
            .iter()
            .find(|r| r.name() == Literal::NAME)
            .unwrap()
            .prefix(),
        Some(PublicationBoundary::Facts)
    );
    let all = build(
        vec![RelationUse::completed::<Probe>()],
        &[],
        LowerLayerPolicy::IncludeInferredOrdinaryFacts,
    )
    .unwrap();
    assert!(all.iter().any(|r| r.name() == Package::NAME));
    assert!(
        build(
            vec![RelationUse::completed::<Probe>()],
            &[RelationUse::of::<Probe>()],
            LowerLayerPolicy::OmitInferredOrdinaryFacts
        )
        .is_err()
    );
    assert!(
        build(
            vec![RelationUse::completed::<Probe>()],
            &[RelationUse::of::<Package>()],
            LowerLayerPolicy::OmitInferredOrdinaryFacts
        )
        .is_err()
    );
    let partial = ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap();
    assert!(
        DependencyClosure::grants(
            &partial,
            vec![],
            vec![RelationUse::completed::<Package>()],
            &[],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order
        )
        .is_err()
    );
}

#[test]
fn grant_composition_inherits_empty_validators_and_refuses_distinct_nonempty_policies() {
    use lctx_model::domain::attribution::FactFamily;
    let model = ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap();
    let use_ = RelationUse::completed::<Literal>().at_epoch(PublicationBoundary::Facts);
    let order = order();
    let build = |direct| {
        DependencyClosure::grants(
            &model,
            vec![],
            direct,
            &[],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order,
        )
    };
    for direct in [
        vec![use_, use_.validated_by(&["a"])],
        vec![use_.validated_by(&["a"]), use_],
        vec![use_.validated_by(&["a"]), use_.validated_by(&["a"])],
    ] {
        assert_eq!(build(direct).unwrap()[0].validators(), ["a"]);
    }
    assert!(build(vec![use_.validated_by(&["a"]), use_.validated_by(&["b"])]).is_err());
    assert!(
        build(vec![
            use_,
            RelationUse::of::<Literal>().at_epoch(PublicationBoundary::Facts)
        ])
        .is_err()
    );
    assert!(
        build(vec![
            use_.availability(FactFamily::Types, AvailabilityPolicy::RequireComplete),
            use_.availability(FactFamily::Types, AvailabilityPolicy::ObserveAvailability)
        ])
        .is_err()
    );
}

#[test]
fn sorted_name_lookup_and_grants_are_independent_of_source_order() {
    let relations = vec![
        Relation::of::<Probe>(),
        Relation::of::<Literal>(),
        Relation::of::<Package>(),
    ];
    let first = ValidatedModel::declared(relations.clone()).unwrap();
    let shuffled = ValidatedModel::declared(relations.into_iter().rev().collect()).unwrap();
    assert_eq!(first.relation(Probe::NAME).unwrap().name(), Probe::NAME);
    assert!(first.relation("missing").is_none());
    for model in [&first, &shuffled] {
        let roots = vec![ValidationInput::of::<Probe>(&["id"])];
        let grants = DependencyClosure::stage_grants(
            model,
            roots,
            &[],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order(),
        )
        .unwrap();
        assert_eq!(
            grants
                .iter()
                .map(|r| (r.name(), r.prefix()))
                .collect::<Vec<_>>(),
            vec![
                (Probe::NAME, None),
                (Literal::NAME, Some(PublicationBoundary::Facts))
            ]
        );
    }
}

#[test]
fn local_missing_predecessor_is_a_typed_rejection() {
    let incomplete = ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap();
    for profile in Profile::ALL {
        let result = local_semantics::stage(
            profile,
            &local_semantics::definition().1,
            &incomplete,
            &order(),
        );
        assert!(matches!(result, Err(ModelError::Invalid(_))));
    }
}

fn epoch_checks_refs() -> Vec<&'static str> {
    vec!["closure_extra_epoch_and_fact_premise"]
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "embedding_full_values")]
struct FullProbe {
    #[model(key)]
    input: ContentHash,
    payload: String,
}
#[test]
fn shared_value_closure_retains_an_earlier_view_while_the_stage_appends_values() {
    let model = ValidatedModel::validate(
        vec![Relation::of::<FullProbe>()],
        ValidationDefinitions {
            invariants: vec![Invariant {
                purpose: InvariantPurpose::Admission,
                revision: 1,
                name: "shared_value_epoch_probe",
                inputs: vec![
                    ValidationInput::of::<FullProbe>(&["id"])
                        .at_epoch(PublicationBoundary::AnalyticEmbedding),
                ],
                create: std::sync::Arc::new(|_| Box::new(EpochCheck)),
            }],
            publication_checks: vec![],
        },
    )
    .unwrap();
    let order = PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::AnalyticEmbedding, vec!["e1"]),
        PublicationGroup::new(PublicationBoundary::Retrieval, vec!["e0"]),
    ])
    .unwrap();
    let closure = DependencyClosure::build(
        &model,
        vec![
            ValidationInput::of::<FullProbe>(&["id"])
                .at_epoch(PublicationBoundary::AnalyticEmbedding),
        ],
        vec![
            RelationUse::completed::<FullProbe>().at_epoch(PublicationBoundary::AnalyticEmbedding),
        ],
        &[RelationUse::of::<FullProbe>()],
        PublicationBoundary::AnalyticEmbedding,
        LowerLayerPolicy::OmitInferredOrdinaryFacts,
        &order,
    )
    .unwrap();
    assert_eq!(closure.requirements.len(), 1);
    assert_eq!(
        closure.requirements[0].prefix(),
        Some(PublicationBoundary::AnalyticEmbedding)
    );
    assert_eq!(closure.grants.len(), 1);
    assert_eq!(
        closure.grants[0].prefix(),
        Some(PublicationBoundary::AnalyticEmbedding)
    );
}


#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "closure_value_use")]
struct ValueUseProbe {
    #[model(key)]
    value: Id<FullProbe>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "embedding_projected_values")]
struct ProjectionProbe {
    #[model(key)]
    value: Id<FullProbe>,
}
fn value_order() -> PublicationOrder {
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::AnalyticEmbedding, vec!["e1"]),
        PublicationGroup::new(PublicationBoundary::Retrieval, vec!["e0"]),
    ]).unwrap()
}
#[test]
fn canonical_parent_views_propagate_without_collapsing_views_or_orders() {
    let model = ValidatedModel::declared(vec![
        Relation::of::<FullProbe>(), Relation::of::<ProjectionProbe>(),
    ]).unwrap();
    let closure = DependencyClosure::build(
        &model,
        vec![
            ValidationInput::of::<ProjectionProbe>(&["id"])
                .at_epoch(PublicationBoundary::AnalyticEmbedding),
            ValidationInput::of::<ProjectionProbe>(&["value", "id"])
                .at_epoch(PublicationBoundary::Retrieval),
        ], vec![], &[], PublicationBoundary::Structural,
        LowerLayerPolicy::OmitInferredOrdinaryFacts, &value_order(),
    ).unwrap();
    for epoch in [PublicationBoundary::AnalyticEmbedding, PublicationBoundary::Retrieval] {
        for name in [FullProbe::NAME, ProjectionProbe::NAME] {
            assert!(closure.grants.iter().any(|g| g.name() == name && g.prefix() == Some(epoch)));
        }
    }
    assert_eq!(closure.grants.len(), 4);
    assert!(closure.requirements.iter().any(|r| r.name() == ProjectionProbe::NAME
        && r.prefix() == Some(PublicationBoundary::Retrieval) && r.order() == ["value", "id"]));
}
#[test]
fn ordinary_canonical_foreign_keys_require_an_unambiguous_declared_view() {
    let model = ValidatedModel::declared(vec![
        Relation::of::<FullProbe>(), Relation::of::<ValueUseProbe>(),
    ]).unwrap();
    for epochs in [vec![], vec![PublicationBoundary::AnalyticEmbedding],
        vec![PublicationBoundary::AnalyticEmbedding, PublicationBoundary::Retrieval]]
    {
        let mut roots = vec![ValidationInput::of::<ValueUseProbe>(&["id"])];
        roots.extend(epochs.iter().map(|epoch|
            ValidationInput::of::<FullProbe>(&["id"]).at_epoch(*epoch)));
        let result = DependencyClosure::build(&model, roots, vec![], &[],
            PublicationBoundary::Structural, LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &value_order());
        if epochs.len() == 1 {
            let closure = result.unwrap();
            assert!(closure.grants.iter().any(|g| g.name() == FullProbe::NAME
                && g.prefix() == Some(PublicationBoundary::AnalyticEmbedding)));
            assert!(!closure.grants.iter().any(|g| g.prefix() == Some(PublicationBoundary::Structural)));
        } else {
            assert!(matches!(result, Err(ModelError::Invalid(_))));
        }
    }
}
#[test]
fn actual_embedding_use_owner_selects_e1_despite_other_declared_value_views() {
    use embedding::{analytic::AnalysisEmbeddingUse, projection::ProjectedValue, value::FullValue};
    let model = model().unwrap();
    for extra_view in [false, true] {
    let mut direct = vec![];
    if extra_view {
        for epoch in [PublicationBoundary::AnalyticEmbedding, PublicationBoundary::Retrieval] {
            direct.push(RelationUse::completed::<FullValue>().at_epoch(epoch));
            direct.push(RelationUse::completed::<ProjectedValue>().at_epoch(epoch));
        }
    }
    let closure = DependencyClosure::build(
        &model,
        vec![ValidationInput::of::<AnalysisEmbeddingUse>(&["id"])],
        direct,
        &[], PublicationBoundary::Structural,
        LowerLayerPolicy::OmitInferredOrdinaryFacts, &value_order(),
    ).unwrap();
    for name in [FullValue::NAME, ProjectedValue::NAME] {
        assert!(closure.grants.iter().any(|g| g.name() == name
            && g.prefix() == Some(PublicationBoundary::AnalyticEmbedding)));
        assert_eq!(closure.grants.iter().any(|g| g.name() == name
            && g.prefix() == Some(PublicationBoundary::Retrieval)), extra_view);
        assert!(!closure.grants.iter().any(|g| g.name() == name
            && g.prefix() == Some(PublicationBoundary::Structural)));
    }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="closure_ambiguous_owner", invariant_refs=ambiguous_owner_refs)]
struct AmbiguousOwnerProbe {
    #[model(key)]
    value: Id<FullProbe>,
}
fn ambiguous_owner_refs() -> Vec<&'static str> {
    vec!["closure_ambiguous_owner_views"]
}
#[test]
fn ambiguous_owner_views_refuse_instead_of_falling_back_to_a_unique_caller_view() {
    let model = ValidatedModel::validate(
        vec![Relation::of::<AmbiguousOwnerProbe>(), Relation::of::<FullProbe>()],
        ValidationDefinitions {
            invariants: vec![Invariant {
                purpose: InvariantPurpose::Admission,
                revision: 1,
                name: "closure_ambiguous_owner_views",
                inputs: [PublicationBoundary::AnalyticEmbedding, PublicationBoundary::Retrieval]
                    .into_iter().map(|epoch| ValidationInput::of::<FullProbe>(&["id"])
                        .at_epoch(epoch)).collect(),
                create: std::sync::Arc::new(|_| Box::new(EpochCheck)),
            }],
            publication_checks: vec![],
        },
    ).unwrap();
    let result = DependencyClosure::build(&model,
        vec![ValidationInput::of::<AmbiguousOwnerProbe>(&["id"]),
            ValidationInput::of::<FullProbe>(&["id"])
                .at_epoch(PublicationBoundary::AnalyticEmbedding)],
        vec![], &[], PublicationBoundary::Structural,
        LowerLayerPolicy::OmitInferredOrdinaryFacts, &value_order());
    assert!(matches!(result, Err(ModelError::Invalid(message))
        if message == "ambiguous canonical dependency view: embedding_full_values"));
}
