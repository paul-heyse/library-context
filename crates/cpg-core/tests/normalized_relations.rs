//! Completed normalization contracts and original text from the actual cumulative compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
async fn run(profile: Profile) {
    let mut settings = catalog_runtime::settings("relations");
    settings.knn = true;
    let embedder = cpg_core::embedding_service::FakeEmbedder::new();
    let fixture = catalog_runtime::compile("normalized_relations", profile, Frontier::Catalog, settings, Some(&embedder)).await;
    let counts: (i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM symbol_entity_resolutions), (SELECT count(*) FROM provider_symbols)").await;
    assert!(counts.0 > 0);
    assert_eq!(counts.0, counts.1);
    let leaves: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM flow_test_leaf_observations), (SELECT count(*) FROM test_operand_type_assessments), (SELECT count(*) FROM test_operand_type_links)").await;
    assert_eq!(leaves.0, leaves.1);
    match profile {
        Profile::Catalog => assert_eq!(leaves, (0, 0, 0)),
        Profile::Behavioral => assert!(leaves.0 > 0 && leaves.2 > 0),
    }
    let signatures: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM signature_observations), (SELECT count(*) FROM signature_variants), (SELECT count(*) FROM effective_callable_assessments)").await;
    assert!(signatures.0 > 0 && signatures.2 > 0);
    assert_eq!(signatures.0, signatures.1);
    let snapshot_counts: (i64, i64, i64) = catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM projection_source_assessments), (SELECT count(*) FROM projection_snapshots), (SELECT count(*) FROM projection_snapshot_chunks)").await;
    assert_eq!(snapshot_counts.0, 4);
    assert_eq!(snapshot_counts.1, 4);
    assert!(snapshot_counts.2 >= 4);
    let text_bytes: Vec<Vec<u8>> = catalog_runtime::query(&fixture, "SELECT text FROM analytic_text_windows").await;
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
async fn catalog_normalizes_completed_facts_with_explicitly_unrequested_flow() {run(Profile::Catalog).await;}
#[tokio::test]
async fn behavioral_normalizes_completed_facts_including_exact_test_operands() {run(Profile::Behavioral).await;}
