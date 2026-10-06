//! Semantic catalog contracts through the actual store-free cumulative compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
async fn run(profile: Profile) {
    let fixture = catalog_runtime::compile(
        "catalog_core",
        profile,
        Frontier::Catalog,
        catalog_runtime::settings("api"),
        None,
    )
    .await;
    let names: Vec<String> =
        catalog_runtime::query(&fixture, "SELECT name FROM catalog_members ORDER BY name").await;
    for name in [
        "choose",
        "alias",
        "wrapped",
        "Base",
        "Child",
        "Config",
        "ChildConfig",
        "Config.read_left",
        "Accessors.value",
    ] {
        assert!(
            names.iter().any(|n| n == name),
            "public slot {name} absent: {names:?}"
        );
    }
    let class_metadata: Vec<(String, bool, bool)> = catalog_runtime::query(&fixture, "SELECT m.name, o.abstract_absence_known, (o.record_options IS NOT NULL) FROM catalog_class_metadata x JOIN catalog_classes c ON c.id=x.class JOIN catalog_members m ON m.id=c.member JOIN class_metadata_observations o ON o.id=x.observation ORDER BY m.name").await;
    assert!(
        class_metadata
            .iter()
            .any(|(name, absent, options)| name == "Config" && !absent && *options)
    );
    assert!(
        class_metadata
            .iter()
            .any(|(name, absent, options)| name == "Base" && !absent && !options)
    );
    assert!(class_metadata.iter().all(|(_, absent, _)| !absent));
    let property_kinds: Vec<i16> = catalog_runtime::query(&fixture, "SELECT o.kind FROM catalog_class_members x JOIN catalog_classes c ON c.id=x.class JOIN catalog_members m ON m.id=c.member JOIN class_member_observations o ON o.id=x.observation WHERE m.name='Accessors' AND o.name='value'").await;
    assert!(!property_kinds.is_empty());
    assert!(property_kinds.iter().all(|kind| *kind == 0));

    let aliases:i64=catalog_runtime::one(&fixture, "SELECT count(DISTINCT c.member) FROM catalog_callables c JOIN catalog_members m ON m.id=c.member WHERE m.name IN ('choose','alias')").await;
    assert_eq!(aliases, 2);
    let alias_basis:Vec<i16>=catalog_runtime::query(&fixture, "SELECT c.basis FROM catalog_callables c JOIN catalog_members m ON m.id=c.member WHERE m.name='alias'").await;
    assert!(!alias_basis.is_empty());
    assert!(alias_basis.iter().all(|v| *v == 1));
    let none:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_options o JOIN catalog_members m ON m.id=o.member JOIN catalog_defaults d ON d.id=o.\"default\" JOIN literal_values l ON l.id=d.literal_literal WHERE m.name='wrapped' AND d.kind=3 AND l.kind=0").await;
    assert!(none > 0);
    let inherited:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_constructors k JOIN catalog_classes c ON c.id=k.class JOIN catalog_members m ON m.id=c.member WHERE m.name='Child' AND k.origin=1 AND k.ancestry IS NOT NULL").await;
    assert!(inherited > 0);
    let synthetic:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_constructors k JOIN catalog_classes c ON c.id=k.class JOIN catalog_members m ON m.id=c.member WHERE m.name='Config' AND k.origin=2").await;
    assert!(synthetic > 0);
    let copied_signatures:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_invocations c JOIN signature_variants v ON v.id=c.variant JOIN signature_observations r ON r.id=v.signature JOIN catalog_callables a ON a.id=c.callable JOIN catalog_members m ON m.id=a.member WHERE m.name='choose'").await;
    assert!(copied_signatures >= 2);
    let metadata:Vec<i16>=catalog_runtime::query(&fixture, "SELECT DISTINCT a.kind FROM catalog_callable_aspects c JOIN callable_aspects a ON a.id=c.aspect").await;
    for kind in [1, 2, 3, 5, 7] {
        assert!(
            metadata.contains(&kind),
            "normalized metadata kind {kind} absent: {metadata:?}"
        );
    }
    let factory:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_options o JOIN catalog_members m ON m.id=o.member JOIN catalog_defaults d ON d.id=o.\"default\" JOIN catalog_option_subjects u ON u.id=o.subject JOIN field_entities f ON f.id=u.field_field WHERE m.name='Config' AND f.name='cache' AND d.kind=5").await;
    assert!(factory > 0);
    let flow:Vec<i16>=catalog_runtime::query(&fixture, "SELECT c.availability FROM normalization_coverage c JOIN normalization_computations n ON n.id=c.computation WHERE n.capability=10").await;
    assert!(!flow.is_empty());
    assert!(flow.iter().all(|v| *v == 3));
}
#[tokio::test]
async fn mandatory_catalog_uses_completed_normalized_contracts() {
    run(Profile::Catalog).await;
}
