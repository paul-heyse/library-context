use lctx_model::domain::{stages::Profile, value::Literal, *};
use lctx_postgres::{
    generations::GenerationStore,
    testing::{DisposableDatabase, Harness},
};
use std::sync::Arc;

#[tokio::test]
async fn unicode_string_values_roundtrip_through_real_postgres_without_escaping() {
    let db = DisposableDatabase::start().await;
    // This control owns literal encoding, not upper-frontier producer configuration.
    let model = Arc::new(ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut harness = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget())
        .await
        .unwrap();
    let generation = harness.generation();
    let mut expected: Vec<_> = ["", "ordinary", "\0", "é\0終"]
        .into_iter()
        .map(|text| Literal::String { value: text.into() })
        .collect();
    let batch = Batch::new(&model, expected.clone(), &budget()).unwrap();
    harness.copy(&batch, &budget()).await.unwrap();
    harness.seal().await.unwrap();
    harness.validate(&budget()).await.unwrap();
    harness.publish().await.unwrap();
    let mut lease = store.pin(&db.reader, generation, budget()).await.unwrap();
    expected.sort_by_key(Record::id);
    assert_eq!(lease.read::<Literal>().await.unwrap().rows(), expected);
    lease.release().await.unwrap();
    store.retire(generation).await.unwrap();
}

fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(1 << 30).unwrap()
}
