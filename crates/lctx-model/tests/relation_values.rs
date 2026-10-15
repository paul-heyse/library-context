use lctx_model::domain::{
    analysis::{
        preparation::{Configuration, ConfigurationGroup},
        settings::AnalyticsConfiguration,
    },
    compilation_product::{RelationValue, RelationValueOutcome},
    *,
};
#[test]
fn relation_values_preserve_empty_outcomes_domains_and_consumed_nominal_values() {
    let domain = ContentHash::of(b"exact selected domain");
    let empty = RelationValue::of_rows::<analysis::MethodParameters>(
        domain,
        RelationValueOutcome::Complete,
        [],
    )
    .unwrap();
    assert_eq!(empty.rows, 0);
    let mut changed = empty.clone();
    changed.outcome = RelationValueOutcome::Partial;
    assert_ne!(empty.identity().unwrap(), changed.identity().unwrap());
    changed = empty.clone();
    changed.outcome = RelationValueOutcome::NotRequested;
    assert_ne!(empty.identity().unwrap(), changed.identity().unwrap());
    changed = empty.clone();
    changed.domain = ContentHash::of(b"different membership");
    assert_ne!(empty.identity().unwrap(), changed.identity().unwrap());
    changed = empty.clone();
    changed.contract = ContentHash::of(b"new local interpretation");
    assert_ne!(empty.identity().unwrap(), changed.identity().unwrap());
    let (parameters, _) = catalog::build::definition();
    let one =
        RelationValue::of_rows(domain, RelationValueOutcome::Complete, [&parameters]).unwrap();
    assert_ne!(empty.identity().unwrap(), one.identity().unwrap());
    let mut other = parameters.clone();
    other.depth = Some(7);
    assert_ne!(
        one.identity().unwrap(),
        RelationValue::of_rows(domain, RelationValueOutcome::Complete, [&other])
            .unwrap()
            .identity()
            .unwrap()
    );
}
fn analytics(depth: i64) -> AnalyticsConfiguration {
    AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec![],
        depth,
        vertices: 512,
        arcs: 2048,
        witnesses: 3,
        brief_budget: 0,
        communities: false,
        pagerank: false,
        fca: false,
        rca: false,
        knn: false,
        type_layer: false,
        mention_layer: false,
        knn_layer: false,
    }
}
#[test]
fn unrelated_analytics_and_sibling_method_changes_do_not_change_selected_value_contracts() {
    let budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
    let authored = models::Catalog::committed().unwrap();
    let selected = catalog::build::definition();
    let sibling = retrieval::build::definition();
    let configuration = |settings| {
        Configuration::new(&authored, [selected.clone(), sibling.clone()], &budget)
            .unwrap()
            .with_analytics(settings)
            .unwrap()
    };
    let left = configuration(analytics(2));
    let right = configuration(analytics(4));
    for group in [
        ConfigurationGroup::Authored,
        ConfigurationGroup::Methods,
        ConfigurationGroup::Projections,
        ConfigurationGroup::Retrieval,
    ] {
        assert_eq!(
            left.declaration(group).configuration,
            right.declaration(group).configuration
        );
    }
    assert_ne!(
        left.declaration(ConfigurationGroup::Analytics)
            .configuration,
        right
            .declaration(ConfigurationGroup::Analytics)
            .configuration
    );
    let domain = ContentHash::of(b"empty exact root domain");
    let core = catalog::build::CatalogData::new(&budget);
    let baseline = core.value_dependencies(domain, &budget).unwrap();
    assert_eq!(baseline, core.value_dependencies(domain, &budget).unwrap());
    let render = retrieval::build::Data::new(&budget);
    assert!(
        render
            .value_dependencies(domain, &budget)
            .unwrap()
            .iter()
            .all(|value| value.kind == compilation_product::DependencyKind::RelationValue)
    );
    let selected_value =
        RelationValue::of_rows(domain, RelationValueOutcome::Complete, [&selected.1]).unwrap();
    let mut sibling_changed = sibling.1.clone();
    sibling_changed.semantic_version = ContentHash::of(b"different sibling meaning");
    let changed = Configuration::new(
        &authored,
        [selected.clone(), (sibling.0, sibling_changed)],
        &budget,
    )
    .unwrap();
    assert_ne!(
        left.declaration(ConfigurationGroup::Methods).configuration,
        changed
            .declaration(ConfigurationGroup::Methods)
            .configuration
    );
    assert_eq!(
        selected_value,
        RelationValue::of_rows(
            domain,
            RelationValueOutcome::Complete,
            changed
                .definitions()
                .iter()
                .filter(|row| row.method == analysis::AnalysisMethod::Catalog)
        )
        .unwrap()
    );
}
fn id<R: Record>(value: u8) -> Id<R> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
#[test]
fn catalog_completeness_projection_cuts_reporting_lineage_and_preserves_status_scope_and_context() {
    use attribution::{CoverageStatus, FactFamily, ProviderCoverage};
    let budget = resources::ResourceBudget::fixed(32 << 20).unwrap();
    let domain = ContentHash::of(b"current complete selected scopes");
    let coverage = ProviderCoverage {
        scope: id(1),
        context: id(2),
        family: FactFamily::Types,
        provider: Some(id(3)),
        run: Some(id(4)),
        status: CoverageStatus::CompleteUnderStatedModel,
        reason: None,
        diagnostic: None,
    };
    let token = |row: ProviderCoverage| {
        let mut data = catalog::build::CatalogData::new(&budget);
        data.native_coverage.insert(row).unwrap();
        data.value_dependencies(domain, &budget)
            .unwrap()
            .into_iter()
            .find(|value| value.role == "native_coverage")
            .unwrap()
    };
    let baseline = token(coverage.clone());
    let mut lineage = coverage.clone();
    lineage.provider = Some(id(5));
    lineage.run = Some(id(6));
    lineage.diagnostic = Some("current reporting detail".into());
    assert_ne!(coverage.id(), lineage.id());
    assert_eq!(baseline, token(lineage));
    let mut partial = coverage.clone();
    partial.status = CoverageStatus::Partial;
    partial.reason = Some(obligation::ObligationKind::BudgetReached);
    assert_ne!(baseline, token(partial));
    let mut source = coverage.clone();
    source.scope = id(7);
    assert_ne!(baseline, token(source));
    let mut context = coverage;
    context.context = id(8);
    assert_ne!(baseline, token(context));
    let empty = catalog::build::CatalogData::new(&budget)
        .value_dependencies(domain, &budget)
        .unwrap()
        .into_iter()
        .find(|value| value.role == "native_coverage")
        .unwrap();
    assert_ne!(baseline, empty);
}
#[test]
fn retrieval_actual_definition_changes_the_value_request_without_sibling_method_receipts() {
    let budget = resources::ResourceBudget::fixed(32 << 20).unwrap();
    let domain = ContentHash::of(b"selected retrieval root");
    let selected = |embedding| {
        let mut data = retrieval::build::Data::new(&budget);
        data.facts
            .definitions
            .insert(retrieval::RetrievalDefinition::builtin(embedding))
            .unwrap();
        data.value_dependencies(domain, &budget).unwrap()
    };
    assert_ne!(selected(false), selected(true));
    let mut left = retrieval::build::Data::new(&budget);
    left.facts
        .definitions
        .insert(retrieval::RetrievalDefinition::builtin(false))
        .unwrap();
    let baseline = left.value_dependencies(domain, &budget).unwrap();
    let (parameters, mut sibling) = execution::configuration::base_evaluation();
    sibling.semantic_version = ContentHash::of(b"unconsumed sibling method");
    left.synthesis.parameters.insert(parameters).unwrap();
    left.synthesis.analysis_definitions.insert(sibling).unwrap();
    assert_eq!(baseline, left.value_dependencies(domain, &budget).unwrap());
}
