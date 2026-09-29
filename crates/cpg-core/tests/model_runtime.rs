use std::sync::Arc;
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use datafusion::datasource::MemTable;
use lctx_model::domain::{*,input::*,stages::*};
#[tokio::test]
async fn declared_stage_catalogs_are_fresh_and_reject_foreign_capabilities() {
    let model = model().unwrap();
    let schedule = Schedule::build(&model,vec![
        Stage { name: "source",inputs: vec![],outputs: vec![RelationUse::of::<Package>()], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") },
        Stage { name: "consumer",inputs: vec![RelationUse::of::<Package>()],outputs: vec![RelationUse::of::<Release>()], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") },
    ],&[], Profile::Catalog).unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions::default()).unwrap();
    let mut execution = schedule.execute(); let mut source = execution.begin("source").unwrap();
    source.write::<Package,_>(async |_| Ok(())).await.unwrap(); source.finish(ProviderOutcome::Complete).unwrap();
    let consumer = execution.begin("consumer").unwrap(); let permit = consumer.read::<Package>().unwrap();
    let first = runtime.session(&consumer); let second = runtime.session(&consumer);
    let rows = Batch::new(&model,vec![Package { name: "demo".into() }], &budget()).unwrap();
    let table = Arc::new(MemTable::try_new(Package::schema(),vec![vec![rows.arrow().clone()]]).unwrap());
    first.register(&permit,table.clone()).unwrap();
    assert_eq!(first.sql("SELECT * FROM packages").await.unwrap().collect().await.unwrap()[0].num_rows(),1);
    assert!(second.sql("SELECT * FROM packages").await.is_err());
    assert!(first.register(&permit,table.clone()).is_err());
    for sql in ["CREATE TABLE leak AS SELECT * FROM packages","DROP TABLE packages","SET datafusion.execution.target_partitions = 1"] {
        assert!(first.sql(sql).await.is_err(),"{sql}");
    }
    let mut another = schedule.execute(); let mut source = another.begin("source").unwrap();
    source.write::<Package,_>(async |_| Ok(())).await.unwrap(); source.finish(ProviderOutcome::Complete).unwrap();
    let different = another.begin("consumer").unwrap();
    assert!(second.register(&different.read::<Package>().unwrap(),table.clone()).is_err());
    second.register(&permit,table).unwrap();
    assert!(second.sql("SELECT * FROM packages").await.is_ok());
}
#[tokio::test]
async fn compute_and_external_reservations_share_one_attempt_pool() {
    let model = model().unwrap(); let schedule = Schedule::build(&model,vec![Stage { name: "s",inputs: vec![],outputs: vec![RelationUse::of::<Package>()], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") }],&[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute(); let stage = execution.begin("s").unwrap();
    let limit = 32*1024*1024; // Includes DataFusion's 10 MiB sort-spill reservation.
    let runtime = AttemptRuntime::new(RuntimeOptions { memory_bytes: limit,partitions: 1 }).unwrap();
    let first = runtime.session(&stage); let second = runtime.session(&stage);
    let allocation = runtime.budget().reserve("producer",limit).unwrap();
    assert!(runtime.budget().reserve("validator",1).is_err());
    let query = "SELECT v FROM generate_series(1, 20000) AS t(v) ORDER BY v DESC";
    assert!(first.sql(query).await.unwrap().collect().await.is_err());
    drop(allocation);
    let rows = second.sql(query).await.unwrap().collect().await.unwrap();
    assert_eq!(rows.iter().map(|b| b.num_rows()).sum::<usize>(),20000);
    assert_eq!(runtime.budget().reserved(),0);
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
