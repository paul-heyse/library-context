use lctx_model::{
    Domain,
    domain::{stages::Profile, *},
};
use lctx_postgres::{
    generations::GenerationStore,
    testing::{DisposableDatabase, Harness},
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "test_metrics", semantic_source = include_bytes!("finite_metrics.rs"))]
struct Metric {
    #[model(key)]
    subject: String,
    value: FiniteF64,
    residual: Option<FiniteF64>,
}

#[tokio::test]
async fn checked_metrics_roundtrip_and_store_refuses_noncanonical_values() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Metric>()]).unwrap());
    let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut harness = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget.clone())
        .await
        .unwrap();
    let generation = harness.generation();
    let mut expected = [f64::MIN, -1.5, f64::from_bits(1), -0.0, f64::MAX]
        .into_iter()
        .enumerate()
        .map(|(index, value)| Metric {
            subject: index.to_string(),
            value: FiniteF64::new(value).unwrap(),
            residual: if index == 0 {
                None
            } else {
                Some(FiniteF64::new(0.125).unwrap())
            },
        })
        .collect::<Vec<_>>();
    let sql = format!(
        "INSERT INTO \"{}\".test_metrics (id,subject,value) VALUES ($1,$2,$3)",
        generation.schema()
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0] {
        let error = sqlx::query(sqlx::AssertSqlSafe(sql.clone()))
            .bind(vec![42u8; 16])
            .bind("invalid")
            .bind(value)
            .execute(db.owner.pool())
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23514")
        );
    }
    let batch = Batch::new(&model, expected.clone(), &budget).unwrap();
    harness.copy(&batch, &budget).await.unwrap();
    harness.seal().await.unwrap();
    harness.validate(&budget).await.unwrap();
    harness.publish().await.unwrap();
    let mut lease = store.pin(&db.reader, generation, budget).await.unwrap();
    expected.sort_by_key(Record::id);
    assert_eq!(lease.read::<Metric>().await.unwrap().rows(), expected);
    lease.release().await.unwrap();
    store.retire(generation).await.unwrap();
}
