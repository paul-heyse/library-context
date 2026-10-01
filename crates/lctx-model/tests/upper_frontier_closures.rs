use lctx_model::domain::{*, analysis, normalized, stages::Profile};
use std::collections::BTreeSet;

#[test]
fn declared_upper_closures_preserve_lower_owners_and_exclude_future_catalog_results() {
    let normalized = normalized_relations().iter().map(Relation::name).collect::<BTreeSet<_>>();
    let analysis = ValidatedModel::validate(analysis_frontier_relations()).unwrap();
    let catalog = ValidatedModel::validate(catalog_frontier_relations()).unwrap();
    let names = |model: &ValidatedModel| model.relations().iter().map(Relation::name).collect::<BTreeSet<_>>();
    let analysis_names = names(&analysis);
    let catalog_names = names(&catalog);
    assert!(normalized.is_subset(&analysis_names));
    assert!(analysis_names.is_subset(&catalog_names));
    assert!(analysis_names.contains(analysis::model::AnalysisInvocation::NAME));
    assert!(analysis_names.contains(analysis::summary::AnalysisInvocation::NAME));
    assert!(analysis_names.contains(catalog::CatalogMember::NAME));
    assert!(!analysis_names.contains(analysis::synthesis::AnalysisInvocation::NAME));
    assert!(!analysis_names.contains(analysis::findings::Finding::NAME));
    assert!(!analysis_names.contains(retrieval::Unit::NAME));
    assert!(catalog_names.contains(analysis::synthesis::AnalysisInvocation::NAME));
    assert_eq!(catalog.digest(), model().unwrap().digest());
    // These remain declarations: no additional product frontier is activated by this split.
    for profile in Profile::ALL {
        admission::FrontierContract::for_frontier(&catalog, profile, admission::Frontier::Normalized).unwrap();
        assert!(analysis_names.contains(normalized::coverage::NormalizationCoverage::NAME));
    }
}
