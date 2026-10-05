//! Product compiler contracts over actual completed native and normalized streams.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
#[tokio::test]
async fn mandatory_four_family_preparation_uses_completed_native_catalog_sources() {
    let fixture = catalog_runtime::compile("catalog_context",Profile::Catalog,Frontier::Catalog,catalog_runtime::settings("api"),None).await;
    for family in 0i16..4 {
        let units: i64 = catalog_runtime::one_with(&fixture, "SELECT count(*) FROM retrieval_units WHERE family=$1", vec![datafusion::common::ScalarValue::Int16(Some(family))]).await;
        assert!(units > 0, "family {family} has no authoritative units");
    }
    let definitions: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_definitions").await;
    assert_eq!(definitions, 1);
    let forged:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_original_anchors a LEFT JOIN retrieval_anchor_sources x ON x.id=a.original WHERE x.id IS NULL").await;
    assert_eq!(forged, 0);
    let units: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_units").await;
    let roots: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_unit_roots").await;
    assert!(roots >= units);
    let unrun:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_corpus_texts WHERE CAST(text AS VARCHAR) LIKE '%execution=NotRun%'").await;
    assert!(unrun > 0);
}
