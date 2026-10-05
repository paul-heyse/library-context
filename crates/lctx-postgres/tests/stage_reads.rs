//! Completed outputs freeze atomically while the cumulative generation stays private and staging.
use lctx_model::domain::{input::*, resources::ResourceBudget, stages::*, *};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::{sync::Arc, time::Duration};

fn budget() -> ResourceBudget {
    ResourceBudget::fixed(64 << 20).unwrap()
}
fn schedule(model: &ValidatedModel) -> Schedule {
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Store,
        code: ContentHash::of(b"freeze control"),
        configuration: ContentHash::of(b"c"),
    };
    Schedule::build(
        model,
        vec![
            stage("packages", vec![], vec![RelationUse::of::<Package>()]),
            stage(
                "releases",
                vec![RelationUse::stored::<Package>()],
                vec![RelationUse::of::<Release>()],
            ),
        ],
        &[],
        Profile::Catalog,
    )
    .unwrap()
}

#[tokio::test]
async fn completion_waits_out_direct_writes_then_freezes_and_receipts_the_same_content() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
            .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget())
        .await
        .unwrap();
    let id = attempt.generation();
    let p = Package {
        name: "regular".into(),
    };
    let batch = Batch::new(&model, vec![p], &budget()).unwrap();
    let mut source = execution.begin("packages").unwrap();
    source
        .write::<Package, _>(async |permit| attempt.copy(permit, &batch).await)
        .await
        .unwrap();
    let mut direct = db.writer.begin().await.unwrap();
    let late = Package {
        name: "in-flight".into(),
    };
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.packages(id,name) VALUES($1,$2)",
        id.schema()
    )))
    .bind(late.id().bytes().to_vec())
    .bind(&late.name)
    .execute(&mut *direct)
    .await
    .unwrap();
    {
        let completion = source.complete(&attempt, ProviderOutcome::Complete);
        tokio::pin!(completion);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut completion)
                .await
                .is_err(),
            "freeze must wait for the direct transaction"
        );
        direct.commit().await.unwrap();
        completion.await.unwrap();
    }
    let held = execution.begin("releases").unwrap();
    assert_eq!(
        held.read::<Package>()
            .unwrap()
            .source()
            .unwrap()
            .receipt()
            .rows,
        2
    );
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.packages",
        id.schema()
    )))
    .fetch_one(&db.writer)
    .await
    .unwrap();
    assert_eq!(count, 2);
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {}.packages SELECT * FROM {}.packages",
            id.schema(),
            id.schema()
        )))
        .execute(&db.writer)
        .await
        .is_err(),
        "late INSERT revoked"
    );
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM {}.packages",
            id.schema()
        )))
        .execute(&db.reader)
        .await
        .is_err(),
        "staging is not a published-reader capability"
    );
    let state: String = sqlx::query_scalar(
        "SELECT state FROM lctx_model_store.generations WHERE id=decode($1, 'hex')",
    )
    .bind(id.hex())
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(state, "staging");
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(
        report
            .findings
            .iter()
            .all(|f| !f.subject.contains(&id.schema())),
        "{report:?}"
    );
    let mut output =
        StageOutput::new(held, &attempt, &model, budget(), Default::default()).unwrap();
    output.declare::<Release>().unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap()
        .publish()
        .await
        .unwrap();
    store.retire(id).await.unwrap();
}

#[tokio::test]
async fn cancelled_completion_never_advances_execution_or_grants_partial_outputs() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
            .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget())
        .await
        .unwrap();
    let id = attempt.generation();
    let mut source = execution.begin("packages").unwrap();
    let empty = Batch::<Package>::new(&model, vec![], &budget()).unwrap();
    source
        .write::<Package, _>(async |permit| attempt.copy(permit, &empty).await)
        .await
        .unwrap();
    let mut blocker = db.owner.pool().begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {}.packages IN ACCESS EXCLUSIVE MODE",
        id.schema()
    )))
    .execute(&mut *blocker)
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(100),
            source.complete(&attempt, ProviderOutcome::Complete)
        )
        .await
        .is_err()
    );
    blocker.rollback().await.unwrap();
    assert!(execution.begin("releases").is_err());
    assert!(execution.finish().is_err());
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1, 'hex')")
        .bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(rows, 0);
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM {}.packages",
            id.schema()
        )))
        .execute(&db.writer)
        .await
        .is_err()
    );
    attempt.abort().await.unwrap();
}

#[tokio::test]
async fn refused_outcome_rolls_back_receipts_and_read_grants_together() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
            .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget())
        .await
        .unwrap();
    let id = attempt.generation();
    let mut source = execution.begin("packages").unwrap();
    let empty = Batch::<Package>::new(&model, vec![], &budget()).unwrap();
    source
        .write::<Package, _>(async |permit| attempt.copy(permit, &empty).await)
        .await
        .unwrap();
    // Refuse the last write in completion, after output receipts and grants were issued.
    sqlx::query("ALTER TABLE lctx_model_store.stage_outcomes ADD CONSTRAINT injected_refusal CHECK(false) NOT VALID")
        .execute(db.owner.pool()).await.unwrap();
    assert!(
        source
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .is_err()
    );
    assert!(execution.begin("releases").is_err());
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex')",
    )
    .bind(id.hex())
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(rows, 0);
    let grants: (bool, bool) = sqlx::query_as("SELECT has_table_privilege('lctx_importer',$1,'INSERT'),has_table_privilege('lctx_importer',$1,'SELECT')")
        .bind(format!("{}.packages", id.schema())).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(
        grants,
        (true, false),
        "the entire completion transaction rolled back"
    );
    attempt.abort().await.unwrap();
}
