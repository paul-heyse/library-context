use lctx_model::domain::{
    analysis::{preparation::*, *},
    models::Catalog,
    stages::*,
    *,
};

fn parameters(catalog: &Catalog) -> MethodParameters {
    MethodParameters {
        depth: None,
        proof_steps: None,
        work: None,
        members: None,
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: Some(catalog.declaration().id()),
    }
}
#[test]
fn configuration_has_nominal_catalog_closure_and_refuses_foreign_selection() {
    let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let catalog = Catalog::committed().unwrap();
    let p = parameters(&catalog);
    let d = AnalysisDefinition {
        method: AnalysisMethod::Models,
        semantic_version: ContentHash::of(b"model operation"),
        parameters: p.id(),
        interpretation: Interpretation::Structural,
    };
    let rows = Configuration::new(&catalog, [(p.clone(), d.clone())], &budget).unwrap();
    assert_eq!(
        rows.catalogs().catalogs.iter().next().unwrap(),
        catalog.declaration()
    );
    assert_eq!(rows.projections().len(), 4);
    rows.check_budget(&budget).unwrap();
    assert!(
        rows.check_budget(&resources::ResourceBudget::fixed(16 << 20).unwrap())
            .is_err()
    );
    let changed =
        Catalog::parse("other.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n").unwrap();
    assert!(Configuration::new(&changed, [(p, d)], &budget).is_err());
    let model = ValidatedModel::validate(configuration_relations()).unwrap();
    for profile in Profile::ALL {
        Schedule::build(&model, vec![rows.declaration()], &[], profile).unwrap();
    }
    let relation = Relation::of::<MethodParameters>();
    assert!(relation.fields().iter().any(|field| {
        field
            .target()
            .is_some_and(|(_, name)| name == models::ModelCatalog::NAME)
    }));
}
#[test]
fn native_stage_uses_only_facts_with_frozen_vocabulary_and_exact_pair_inventory() {
    let model = lctx_model::domain::model().unwrap();
    let stage = native_stage(Profile::Behavioral, &model, &alignment_publication_order()).unwrap();
    let facts_relations = facts_relations();
    let facts = facts_relations
        .iter()
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    for input in &stage.inputs {
        assert!(facts.contains(input.name()));
        assert_eq!(input.transport(), InputTransport::CompletedStore);
        if is_vocabulary(input.name()) {
            assert_eq!(input.prefix(), Some(PublicationBoundary::Facts));
        }
    }
    let mut expected = std::collections::BTreeSet::from([assertion::AssertionQualification::NAME]);
    macro_rules! count_pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        assert!(stage.reads::<$assertion>() && stage.reads::<$support>());let _=$code;
        assert!(expected.insert(<$assertion>::NAME));assert!(expected.insert(<$support>::NAME));
    )*};}
    lctx_model::native_analysis_pairs!(count_pairs);
    // The finite pair registry is the seed, not the entire native validator contract:
    // canonical references and each declaration's invariants require auxiliary facts too.
    let mut pending = expected.iter().copied().collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        let relation = facts_relations.iter().find(|r| r.name() == name).unwrap();
        for dependency in relation
            .fields()
            .iter()
            .filter_map(|field| field.target().map(|(_, name)| name))
            .chain(
                relation
                    .invariants()
                    .iter()
                    .flat_map(|check| check.inputs.iter().map(ValidationInput::name)),
            )
        {
            if expected.insert(dependency) {
                pending.push(dependency);
            }
        }
    }
    assert_eq!(
        stage.inputs.len(),
        expected.len(),
        "exact native registry and declared dependency closure without duplicate reads"
    );
    assert_eq!(
        stage
            .inputs
            .iter()
            .map(|i| i.name())
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    assert!(stage.reads::<calls::SignatureEnumerationObservation>());
    assert!(stage.reads::<calls::SignatureEnumerationSupport>());
    assert_eq!(stage.outputs.len(), 2);
    assert!(
        stage.writes::<native::NativeAssertionPremise>()
            && stage.writes::<native::NativeQualification>()
    );
    let catalog = native_stage(Profile::Catalog, &model, &alignment_publication_order()).unwrap();
    macro_rules! profile_pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        let requested = <$assertion as assertion::Assertion>::FAMILY != attribution::FactFamily::Flow;
        assert_eq!(catalog.reads::<$assertion>(), requested);
        assert_eq!(catalog.reads::<$support>(), requested);
    )*};}
    lctx_model::native_analysis_pairs!(profile_pairs);
    assert!(
        !catalog.reads::<flow::FlowUseObservation>()
            && !catalog.reads::<flow::FlowAttributeLoadObservation>()
    );
}

fn alignment_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    use lctx_model::domain::stages::*;
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
        PublicationGroup::new(PublicationBoundary::Model, vec!["model"]),
        PublicationGroup::new(PublicationBoundary::Summary, vec!["summary"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Analytic, vec!["analytic"]),
        PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
    ])
    .unwrap()
}
