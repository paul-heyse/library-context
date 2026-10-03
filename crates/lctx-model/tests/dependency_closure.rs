//! Exact epoch requirements are independently stronger than one sufficient grant.
use lctx_model::{
    Domain,
    domain::{dependency_closure::*, input::Package, stages::*, value::Literal, *},
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="closure_probe", semantic_source=include_bytes!("dependency_closure.rs"))]
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
fn separate_epochs_and_orders_lower_by_publication_ordinal_without_erasing_requirements() {
    let model = ValidatedModel::validate(vec![Relation::of::<Literal>()]).unwrap();
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
    assert_eq!(closure.grants.len(), 1);
    assert_eq!(
        closure.grants[0].prefix(),
        Some(PublicationBoundary::Local),
        "declared order wins even though boundary codes have another order"
    );
}
#[test]
fn inferred_facts_omission_preserves_explicit_inputs_and_unfinished_output_refuses() {
    let model = ValidatedModel::validate(vec![
        Relation::of::<Probe>(),
        Relation::of::<Literal>(),
        Relation::of::<Package>(),
    ])
    .unwrap();
    let closure = DependencyClosure::build(
        &model,
        vec![ValidationInput::of::<Probe>(&["id"])],
        vec![RelationUse::stored::<Package>()],
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
#[model(name="closure_epoch_probe", invariants=epoch_checks, semantic_source=include_bytes!("dependency_closure.rs"))]
struct EpochProbe {
    #[model(key)]
    value: Id<Literal>,
}
fn epoch_checks() -> Vec<Invariant> {
    vec![Invariant {
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
    let model = ValidatedModel::validate(vec![
        Relation::of::<EpochProbe>(),
        Relation::of::<Package>(),
        Relation::of::<Literal>(),
    ])
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
            .find(|g| g.name() == Literal::NAME)
            .unwrap()
            .prefix(),
        Some(PublicationBoundary::Local)
    );
    assert!(
        build(vec![RelationUse::stored::<Package>()])
            .grants
            .iter()
            .any(|g| g.name() == Package::NAME),
        "an actual consumed fact always needs its own grant"
    );
}
