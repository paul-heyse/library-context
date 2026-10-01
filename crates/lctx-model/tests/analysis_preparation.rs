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
    let stage = native_stage(Profile::Behavioral);
    let facts = facts_relations()
        .into_iter()
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    for input in &stage.inputs {
        assert!(facts.contains(input.name()));
        assert_eq!(input.transport(), InputTransport::CompletedStore);
        if is_vocabulary(input.name()) {
            assert_eq!(input.prefix(), Some(PublicationBoundary::Facts));
        }
    }
    let mut pairs = 0;
    macro_rules! count_pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        assert!(stage.reads::<$assertion>() && stage.reads::<$support>());let _=$code;pairs+=1;
    )*};}
    lctx_model::native_analysis_pairs!(count_pairs);
    assert_eq!(pairs, 51);
    assert_eq!(stage.outputs.len(), 2);
    assert!(
        stage.writes::<native::NativeAssertionPremise>()
            && stage.writes::<native::NativeQualification>()
    );
    let catalog = native_stage(Profile::Catalog);
    assert!(
        !catalog.reads::<flow::FlowUseObservation>()
            && !catalog.reads::<flow::FlowAttributeLoadObservation>()
    );
}
