//! The cumulative frontier uses a private facts checkpoint and one atomic publication.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{
    admission::{Frontier, FrontierContract},
    stages::Profile,
    *,
};
use lctx_postgres::{
    generations::{GenerationCatalog, GenerationStore, ListFilter},
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;
fn captured(budget: &resources::ResourceBudget) -> Arc<CapturedInputs> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/normalized_projections");
    Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(&root, &["graph.py".into(), "helper.py".into()], budget).unwrap(),
        "normalized-generation",
    )]))
}
fn config(db: &DisposableDatabase) -> RoleConfig {
    RoleConfig {
        format: 1,
        role: Role::Importer,
        url: db.url("lctx_importer"),
        max_connections: 6,
        provider_connections: 4,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 60,
        lock_timeout_seconds: 10,
    }
}
#[tokio::test]
async fn normalized_is_self_contained_repeatable_and_never_selects() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let runtime = AttemptRuntime::new(RuntimeOptions {
        memory_bytes: 1 << 30,
        partitions: 2,
    })
    .unwrap();
    let config = config(&db);
    let mut previous = None;
    for profile in [Profile::Catalog, Profile::Behavioral, Profile::Behavioral] {
        let published = cpg_core::normalize::publish(
            &store,
            &config,
            db.writer.clone(),
            captured(runtime.budget()),
            &runtime,
            profile,
            ContentHash::of(b"normalized-generation"),
        )
        .await
        .unwrap();
        assert_eq!(
            published.measurements.len(),
            if profile == Profile::Catalog { 12 } else { 13 }
        );
        let detail = catalog.show(published.generation).await.unwrap().unwrap();
        assert!(!detail.summary.selected);
        assert_eq!(detail.summary.frontier, Frontier::Normalized);
        if profile == Profile::Behavioral {
            if let Some(content) = previous {
                assert_eq!(content, published.content);
            }
            previous = Some(published.content);
        }
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.projection_snapshots),(SELECT count(*) FROM {}.call_binding_attempts),(SELECT count(*) FROM {}.source_artifacts)",published.generation.schema(),published.generation.schema(),published.generation.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        let availability: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT c.availability FROM {}.normalization_coverage c JOIN {}.normalization_computations n ON c.computation=n.id WHERE n.capability=10",published.generation.schema(),published.generation.schema()))).fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(
            availability.len(),
            2,
            "one flow scope outcome per captured Python artifact, even without observations"
        );
        assert!(
            availability
                .iter()
                .all(|a| (*a == 3) == (profile == Profile::Catalog))
        );
        let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.normalization_output_receipts n JOIN {}.normalization_computations c ON n.computation=c.id LEFT JOIN lctx_model_store.stage_receipts s ON s.generation_id=decode($1,'hex') AND s.relation_name=n.relation AND s.stage_name=c.producer WHERE s.content_digest IS DISTINCT FROM n.content OR s.row_count IS DISTINCT FROM n.rows",published.generation.schema(),published.generation.schema()))).bind(published.generation.hex()).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(mismatches, 0);
        assert_eq!(counts.0, 4);
        assert!(counts.1 > 0);
        assert_eq!(counts.2, 2);
        store.retire(published.generation).await.unwrap();
        assert_eq!(runtime.budget().reserved(), 0);
    }
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn failed_normalization_removes_private_facts_and_preserves_selected_generation() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let runtime = AttemptRuntime::new(RuntimeOptions {
        memory_bytes: 1 << 30,
        partitions: 2,
    })
    .unwrap();
    let prior = cpg_core::facts::publish(
        &store,
        db.writer.clone(),
        captured(runtime.budget()),
        runtime.budget().clone(),
        Profile::Catalog,
        ContentHash::of(b"prior"),
    )
    .await
    .unwrap();
    store.select(prior.generation).await.unwrap();
    let mut invalid = config(&db);
    invalid.url = "postgres://lctx_importer:invalid@127.0.0.1:1/unreachable".into();
    let failed = cpg_core::normalize::publish(
        &store,
        &invalid,
        db.writer.clone(),
        captured(runtime.budget()),
        &runtime,
        Profile::Catalog,
        ContentHash::of(b"failure"),
    )
    .await;
    assert!(failed.is_err());
    assert_eq!(runtime.budget().reserved(), 0);
    let listed = catalog.list(&ListFilter::default()).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, prior.generation);
    assert!(listed[0].selected);
    store.clear_selection().await.unwrap();
    store.retire(prior.generation).await.unwrap();
    assert!(
        GenerationStore::check(&db.owner, &model)
            .await
            .unwrap()
            .clean()
    );
}
#[test]
fn normalized_preflight_requires_all_snapshot_producers() {
    let model = model().unwrap();
    let profile = Profile::Catalog;
    let providers = cpg_core::facts::providers::<lctx_postgres::generations::GenerationAttempt>(
        ContentHash::of(b"preflight"),
    );
    let schedule = cpg_core::normalize::schedule(&model, &providers, profile).unwrap();
    let contract = FrontierContract::for_frontier(&model, profile, Frontier::Normalized).unwrap();
    assert!(contract.preflight(&schedule).is_ok());
    let missing = stages::Schedule::build(
        &model,
        schedule
            .stages()
            .iter()
            .filter(|s| !matches!(s.name, "normalize_projections" | "normalize_coverage"))
            .cloned()
            .collect(),
        &[],
        profile,
    )
    .unwrap();
    assert!(
        contract
            .preflight(&missing)
            .unwrap_err()
            .to_string()
            .contains("no scheduled stage writes normalized relation")
    );
    assert!(
        FrontierContract::facts(&model, profile)
            .unwrap()
            .preflight(&schedule)
            .is_err()
    );
}

#[tokio::test]
async fn empty_captured_scope_publishes_explicit_no_scope_and_empty_snapshots() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model)
        .await
        .unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions {
        memory_bytes: 1 << 30,
        partitions: 2,
    })
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/normalized_projections");
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(&root, &[], runtime.budget()).unwrap(),
        "empty-normalized",
    )]));
    let published = cpg_core::normalize::publish(
        &store,
        &config(&db),
        db.writer.clone(),
        captured,
        &runtime,
        Profile::Catalog,
        ContentHash::of(b"empty-normalized"),
    )
    .await
    .unwrap();
    let outcomes: Vec<(i16, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT capability,availability FROM {}.normalization_computations WHERE capability IN (0,8,10,16) ORDER BY capability", published.generation.schema()))).fetch_all(db.owner.pool()).await.unwrap();
    assert_eq!(outcomes, vec![(0, 4), (8, 4), (10, 3), (16, 3)]);
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.projection_source_assessments WHERE vertices=0 AND arcs=0",
        published.generation.schema()
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(count, 4);
    store.retire(published.generation).await.unwrap();
    assert_eq!(runtime.budget().reserved(), 0);
}
