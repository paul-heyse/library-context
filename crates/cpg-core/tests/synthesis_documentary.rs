//! Product compiler contracts over actual completed native and normalized streams.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
#[tokio::test]
async fn documentary_preparation_preserves_native_literal_spans_and_candidates() {
    let fixture = catalog_runtime::compile(
        "synthesis_sources",
        Profile::Catalog,
        Frontier::Catalog,
        catalog_runtime::settings("api"),
        None,
    )
    .await;
    let count: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM synthesis_documentary_conclusions",
    )
    .await;
    assert!(
        count >= 2,
        "original plus sourced alias remain independently qualified"
    );
    let missing: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM synthesis_documentary_boundaries WHERE reason IN(1,3)",
    )
    .await;
    assert!(
        missing >= 2,
        "no docstring and unsupported literal mapping stay explicit"
    );
    let forged:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM synthesis_documentary_conclusions c JOIN assertion_qualifications q ON q.id=c.qualification WHERE q.modality<>1 OR q.approximation<>1").await;
    assert_eq!(forged, 0);
    let passages: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM synthesis_documentary_sources WHERE kind=1",
    )
    .await;
    assert!(
        passages > 0,
        "actual C1 mention associations produce original documentary excerpts"
    );
    let mismatched:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM synthesis_documentary_sources d JOIN synthesis_documentary_conclusions c ON c.source=d.id JOIN assertion_qualifications native ON native.id=coalesce(d.literal_source_qualification,d.passage_source_qualification,d.component_source_qualification) JOIN assertion_qualifications derived ON derived.id=c.qualification WHERE native.scope=derived.scope OR native.context<>derived.context").await;
    assert_eq!(
        mismatched, 0,
        "native artifact evidence and static release association remain distinct"
    );
    let components: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM synthesis_documentary_sources WHERE kind=2",
    )
    .await;
    assert!(
        components >= 4,
        "actual native Warning and ParamField conclusions are published"
    );
    for reason in [1i16, 3, 6] {
        let count: i64 = catalog_runtime::one_with(
            &fixture,
            "SELECT count(*) FROM synthesis_documentary_component_boundaries WHERE reason=$1",
            vec![datafusion::common::ScalarValue::Int16(Some(reason))],
        )
        .await;
        assert!(
            count > 0,
            "unknown/nested/inline template boundary {reason} remains explicit"
        );
    }
    let wrong:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM synthesis_documentary_sources d JOIN document_component_observations c ON c.id=d.component_component JOIN document_nodes n ON n.id=c.component WHERE d.kind=2 AND c.form<>0").await;
    assert_eq!(
        wrong, 0,
        "inline templates never acquire authored assertion authority"
    );
}

#[path = "fixtures/native.rs"]
mod native_fixture;
