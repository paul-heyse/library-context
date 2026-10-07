//! Completed normalization contracts and original text from the actual cumulative compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{
    admission::{Availability, Frontier},
    attribution::FactFamily,
    flow::FlowTestLeafObservation,
    stages::{AvailabilityPolicy, InputRequirement, Profile},
};
async fn run(profile: Profile) {
    let mut settings = catalog_runtime::settings("relations");
    settings.knn = true;
    let embedder = cpg_core::embedding_service::FakeEmbedder::new();
    let fixture = catalog_runtime::compile(
        "normalized_relations",
        profile,
        Frontier::Catalog,
        settings,
        Some(&embedder),
    )
    .await;
    let counts: (i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM symbol_entity_resolutions) AS fixture_column_0, (SELECT count(*) FROM provider_symbols) AS fixture_column_1").await;
    assert!(counts.0 > 0);
    assert_eq!(counts.0, counts.1);
    match profile {
        Profile::Catalog => {
            // Unrequested providers publish availability, not fabricated empty assertion streams.
            assert!(
                fixture
                    .workspace
                    .completed::<FlowTestLeafObservation>()
                    .is_err()
            );
            let operands: (i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM test_operand_type_assessments) AS fixture_column_0, (SELECT count(*) FROM test_operand_type_links) AS fixture_column_1").await;
            assert_eq!(operands, (0, 0));
            let availability = fixture.workspace.facts_availability(profile).unwrap();
            let flow = availability
                .evidence()
                .iter()
                .filter(|row| row.family == FactFamily::Flow)
                .collect::<Vec<_>>();
            assert!(
                !flow.is_empty(),
                "unrequested flow must be explicitly disclosed"
            );
            assert!(flow.iter().all(
                |row| row.availability == Availability::NotRequested && row.provider.is_none()
            ));
            assert!(
                availability
                    .admit(InputRequirement {
                        group: FactFamily::Flow,
                        policy: AvailabilityPolicy::RequireComplete,
                    })
                    .is_err(),
                "unrequested flow cannot authorize complete flow reads"
            );
        }
        Profile::Behavioral => {
            let leaves: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM flow_test_leaf_observations) AS fixture_column_0, (SELECT count(*) FROM test_operand_type_assessments) AS fixture_column_1, (SELECT count(*) FROM test_operand_type_links) AS fixture_column_2").await;
            assert_eq!(leaves.0, leaves.1);
            assert!(leaves.0 > 0 && leaves.2 > 0);
        }
    }
    let signatures: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM signature_observations) AS fixture_column_0, (SELECT count(*) FROM signature_variants) AS fixture_column_1, (SELECT count(*) FROM effective_callable_assessments) AS fixture_column_2").await;
    assert!(signatures.0 > 0 && signatures.2 > 0);
    assert_eq!(signatures.0, signatures.1);
    let snapshot_counts: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM projection_source_assessments) AS fixture_column_0, (SELECT count(*) FROM projection_snapshots) AS fixture_column_1, (SELECT count(*) FROM projection_snapshot_chunks) AS fixture_column_2").await;
    assert_eq!(snapshot_counts.0, 4);
    assert_eq!(snapshot_counts.1, 4);
    assert!(snapshot_counts.2 >= 4);
    let text_bytes: Vec<Vec<u8>> =
        catalog_runtime::query(&fixture, "SELECT text FROM analytic_text_windows").await;
    let texts = text_bytes
        .into_iter()
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text|text=="analytic_text.Container.méthode(self, value: int, /, *items, option=None, **options)\nKeep the original parameter and documentation evidence."),"original source parameter spans and Unicode survive normalization: {texts:?}");
    assert!(texts.iter().any(
        |text| text == "analytic_text.factory.nested(value=\"αβ\")\nNested source declaration."
    ));
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Use `relations.Box`")),
        "original document passages are independent of catalog/retrieval"
    );
}
#[tokio::test]
async fn catalog_normalizes_completed_facts_with_explicitly_unrequested_flow() {
    run(Profile::Catalog).await;
}
#[tokio::test]
async fn behavioral_normalizes_completed_facts_including_exact_test_operands() {
    run(Profile::Behavioral).await;
}

#[path = "fixtures/native.rs"]
mod native_fixture;
