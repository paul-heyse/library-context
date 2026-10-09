//! Both profiles use the selected model catalog and native input inventory in the actual compiler.
use crate::catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
#[tokio::test]
async fn both_profiles_keep_selected_catalog_and_exact_native_inventory() {
    for profile in Profile::ALL {
        let mut settings = catalog_runtime::settings("relations");
        settings.configured_seeds = vec!["relations.value".into()];
        settings.fca = true;
        settings.brief_budget = 4;
        let fixture = catalog_runtime::compile(
            "normalized_relations",
            profile,
            Frontier::Analysis,
            settings,
            None,
        )
        .await;
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM native_analysis_premises) AS fixture_column_0,(SELECT count(*) FROM native_qualifications) AS fixture_column_1,(SELECT count(*) FROM model_catalogs) AS fixture_column_2").await;
        assert!(counts.0 > 0);
        assert_eq!(counts.0, counts.1);
        assert_eq!(counts.2, 1);
        let selected:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM analytics_configurations WHERE fca AND NOT rca AND NOT communities AND depth=2 AND brief_budget=4").await;
        assert_eq!(selected, 1);
    }
}

