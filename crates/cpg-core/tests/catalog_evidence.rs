//! Semantic catalog contracts through the actual persisted cumulative compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(profile: Profile) {
    let fixture = catalog_runtime::compile(
        "catalog_context",
        profile,
        Frontier::Catalog,
        catalog_runtime::settings("api"),
        None,
    )
    .await;
    let root = catalog_runtime::root("catalog_context");
    let scenarios: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_scenarios").await;
    assert!(scenarios >= 5);
    let executed: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM catalog_scenarios WHERE execution<>2",
    )
    .await;
    assert_eq!(executed, 0);
    let originals: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM catalog_original_sources WHERE kind=2",
    )
    .await;
    assert!(originals >= 2);
    let extracted: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM catalog_scenario_spans WHERE role=3",
    )
    .await;
    assert_eq!(extracted, 1);
    let associations: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM catalog_scenario_associations",
    )
    .await;
    assert!(associations > 0);
    let fields:Vec<(String,i16,i16)>=catalog_runtime::query(&fixture, "SELECT CAST(f.name AS VARCHAR),a.basis,a.applicability FROM catalog_field_access_assessments a JOIN field_entities f ON f.id=a.field").await;
    assert!(fields.iter().any(|r| r.0 == "left"));
    assert!(fields.iter().any(|r| r.0 == "right"));
    assert!(fields.iter().all(|r| r.1 == 4 && r.2 == 1));
    let access_spans:Vec<(String,i64,i64)>=catalog_runtime::query(&fixture, "SELECT CAST(f.name AS VARCHAR),o.start,o.\"end\" FROM catalog_field_access_assessments a JOIN field_entities f ON f.id=a.field JOIN occurrences o ON o.id=a.occurrence").await;
    let api = std::fs::read_to_string(root.join("api.py")).unwrap();
    for (name, start, end) in access_spans {
        assert_eq!(&api[start as usize..end as usize], format!("self.{name}"));
    }
    let source_links:Vec<(String,String,i16,i16)>=catalog_runtime::query(&fixture, "SELECT CAST(f.name AS VARCHAR),r.name,l.source_association,l.runtime_value FROM catalog_source_field_links l JOIN source_field_reader_links k ON k.id=l.reader JOIN source_field_readers r ON r.id=k.reader JOIN source_field_associations a ON a.id=l.association JOIN record_field_observations f ON f.id=a.field").await;
    assert_eq!(
        source_links.len(),
        12,
        "six exact field readers each retain declared and native field options"
    );
    assert!(
        source_links
            .iter()
            .all(|(field, reader, source, runtime)| field == reader
                && *source == normalized::callables::Knowledge::Known as i16
                && *runtime == normalized::callables::Knowledge::Unknown as i16)
    );
    let option_kinds:Vec<(i16,i16,i16,i16,i64)>=catalog_runtime::query(&fixture, "SELECT fs.kind,fe.kind,ps.kind,pe.kind,count(*) FROM catalog_source_field_links l JOIN catalog_options fo ON fo.id=l.field_option JOIN catalog_option_subjects fs ON fs.id=fo.subject JOIN catalog_option_evidence fe ON fe.id=fo.evidence JOIN catalog_options po ON po.id=l.parameter_option JOIN catalog_option_subjects ps ON ps.id=po.subject JOIN catalog_option_evidence pe ON pe.id=po.evidence GROUP BY 1,2,3,4 ORDER BY 1,2,3,4").await;
    assert_eq!(
        option_kinds,
        vec![(1, 2, 0, 1, 6), (1, 3, 0, 1, 6)],
        "both field-option forms bind only the generated initializer's native parameter"
    );
    let reader_count: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(DISTINCT reader) FROM catalog_source_field_links",
    )
    .await;
    assert_eq!(
        reader_count, 6,
        "PlainConfig supplies no supported-record source link"
    );
    let roots: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_evidence_roots").await;
    assert!(roots > scenarios);
    let links: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM catalog_evidence_invocations",
    )
    .await;
    assert_eq!(links, roots);
}
#[tokio::test]
async fn contextual_catalog_preserves_original_roots() {
    run(Profile::Catalog).await;
}
#[tokio::test]
async fn behavioral_contextual_catalog_retains_earlier_locations_without_heap_state() {
    run(Profile::Behavioral).await;
}

#[path = "fixtures/native.rs"]
mod native_fixture;
