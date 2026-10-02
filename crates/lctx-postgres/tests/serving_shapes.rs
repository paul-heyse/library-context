//! Physical shape controls use a complete empty canonical conformance generation, not mock pins.
use std::sync::Arc;
use lctx_model::domain::{model,resources::ResourceBudget,stages::Profile};
use lctx_postgres::{generations::{GenerationStore,GenerationReader},testing::{DisposableDatabase,Harness}};
#[tokio::test]
async fn identity_views_and_declared_indexes_are_checked_under_the_original_lease() {
    let db=DisposableDatabase::start().await;
    db.migrate().await;
    let model=Arc::new(model().unwrap());
    let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let budget=ResourceBudget::fixed(128<<20).unwrap();
    let mut attempt=Harness::begin_empty_conformance(&store,db.writer.clone(),Profile::Catalog,budget.clone(),vec![]).await.unwrap();
    attempt.seal().await.unwrap();
    attempt.validate(&budget).await.unwrap();
    attempt.publish().await.unwrap();
    let dir=tempfile::tempdir().unwrap();
    db.write_configs(dir.path()).unwrap();
    let role=lctx_postgres::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
    let reader=GenerationReader::connect(model,&role).await.unwrap();
    let guard=reader.guard(attempt.generation(),budget.clone()).await.unwrap();
    guard.release().await.unwrap();
    let view=format!("{}.serving_members",attempt.generation().schema());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE VIEW {view} AS SELECT * FROM {}.catalog_members WHERE false",attempt.generation().schema())))
        .execute(&db.superuser).await.unwrap();
    assert!(reader.guard(attempt.generation(),budget).await.is_err());
    reader.close().await;
}
