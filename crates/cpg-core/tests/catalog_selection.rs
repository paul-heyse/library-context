//! Semantic catalog contracts through the actual store-free cumulative compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile};
async fn run(profile: Profile) {
    let fixture = catalog_runtime::compile("catalog_context",profile,Frontier::Catalog,catalog_runtime::settings("api"),None).await;
    let members: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_members").await;
    let domains: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_selection_domains").await;
    assert_eq!(domains, members * 7);
    let links: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_selection_invocations").await;
    assert_eq!(links, domains);
    let signatures: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM selection_contexts WHERE kind=2").await;
    assert!(signatures > 0);
    let missing_original:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM catalog_selection_domains d WHERE d.kind=5 AND NOT EXISTS (SELECT 1 FROM catalog_selection_domain_contexts c WHERE c.domain=d.id)").await;
    assert_eq!(missing_original, 0);
}
#[tokio::test]
async fn finite_selection_domains_follow_actual_completed_catalog_owners() {run(Profile::Catalog).await;}
#[tokio::test]
async fn behavioral_selection_keeps_runtime_field_witnesses_without_exact_state() {run(Profile::Behavioral).await;}
