//! Semantic compiler owners consume completed native and normalized streams.
use crate::catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
async fn run(profile: Profile) {
    let fixture = catalog_runtime::compile(
        "phase4_models",
        profile,
        Frontier::Analysis,
        catalog_runtime::settings("cases"),
        None,
    )
    .await;
    let count: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM model_runs").await;
    assert!(count > 0);
    let statuses: Vec<i16> =
        catalog_runtime::query(&fixture, "SELECT status FROM model_analysis_outcomes").await;
    let applications: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM model_applications").await;
    let postconditions: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM modeled_action_postconditions WHERE phase=1",
    )
    .await;
    let context_counts:(i64,i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM model_context_resources) AS fixture_column_0,(SELECT count(*) FROM model_context_postconditions WHERE phase=1) AS fixture_column_1,(SELECT count(*) FROM model_context_postconditions WHERE phase=2) AS fixture_column_2,(SELECT count(*) FROM model_context_postconditions WHERE phase=3) AS fixture_column_3").await;
    let transfers: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM model_transfer_alternatives").await;
    let context_transfers: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM model_context_transfer_witnesses",
    )
    .await;
    // Each selected root publishes its evidence before contributing to the one frame run.
    // Independent stored cardinalities reveal either omitted evidence or omitted merge_run.
    let mismatched_runs: i64 = catalog_runtime::one(&fixture,
        "SELECT count(*) FROM model_runs r WHERE r.applied<>(SELECT count(*) FROM model_applications a WHERE a.invocation=r.invocation) OR r.refused<>(SELECT count(*) FROM model_application_boundaries b WHERE b.invocation=r.invocation)").await;
    assert_eq!(
        mismatched_runs, 0,
        "merged frame counts must match independently published selected-root evidence"
    );
    if profile == Profile::Behavioral {
        assert!(context_transfers > 0, "actual checked WithTarget transfer");
        assert!(
            context_counts.0 > 0
                && context_counts.1 > 0
                && context_counts.2 > 0
                && context_counts.3 > 0,
            "actual normal/exceptional/finally context lifecycle: {context_counts:?}"
        );
        assert!(
            transfers > 0,
            "actual parameter entry to call value transfer must publish"
        );
        assert!(applications > 0);
        assert!(postconditions > 0);
        assert!(statuses.iter().all(|s| *s == 1));
    } else {
        assert_eq!(context_transfers, 0);
        assert_eq!(context_counts, (0, 0, 0, 0));
        assert_eq!(transfers, 0);
        assert_eq!(applications, 0);
        assert_eq!(postconditions, 0);
        assert!(statuses.iter().all(|s| *s == 3));
    }
}
#[tokio::test]
async fn behavioral_models_consume_actual_enriched_calls() {
    run(Profile::Behavioral).await;
}
#[tokio::test]
async fn catalog_models_are_explicitly_not_requested() {
    run(Profile::Catalog).await;
}
