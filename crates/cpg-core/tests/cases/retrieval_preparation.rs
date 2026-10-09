//! Product compiler contracts over actual completed native and normalized streams.
use crate::catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
#[tokio::test]
async fn mandatory_four_family_preparation_uses_completed_native_catalog_sources() {
    let fixture = catalog_runtime::compile(
        "catalog_context",
        Profile::Catalog,
        Frontier::Catalog,
        catalog_runtime::settings("api"),
        None,
    )
    .await;
    for family in 0i16..4 {
        let units: i64 = catalog_runtime::one_with(
            &fixture,
            "SELECT count(*) FROM retrieval_units WHERE family=$1",
            vec![datafusion::common::ScalarValue::Int16(Some(family))],
        )
        .await;
        assert!(units > 0, "family {family} has no authoritative units");
    }
    let definitions: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_definitions").await;
    assert_eq!(definitions, 1);
    let forged:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_original_anchors a LEFT JOIN retrieval_anchor_sources x ON x.id=a.original WHERE x.id IS NULL").await;
    assert_eq!(forged, 0);
    let units: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_units").await;
    let roots: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_unit_roots").await;
    assert!(roots >= units);
    let parts: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_content_parts").await;
    let windows: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_search_windows").await;
    assert!(parts > 0 && windows > 0);
    let context_bindings:i64=catalog_runtime::one(&fixture,"SELECT count(*) FROM retrieval_window_bindings b JOIN retrieval_content_parts p ON b.part=p.id WHERE p.purpose=1").await;
    assert_eq!(
        context_bindings, 0,
        "context parts cannot nominate applicability"
    );
    let unrun:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM retrieval_corpus_texts WHERE CAST(text AS VARCHAR) LIKE '%execution=NotRun%'").await;
    assert!(unrun > 0);
}

