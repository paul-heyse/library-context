//! Independent publication controls through the real disposable PostgreSQL 18 store.
use lctx_model::{
    Domain,
    domain::{input::Package, resources::ResourceBudget, stages::*, value::Literal, *},
};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::sync::Arc;
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="epoch_results", semantic_source=include_bytes!("vocabulary_epochs.rs"))]
struct ResultRow {
    #[model(key)]
    value: Id<Literal>,
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(64 << 20).unwrap()
}
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"real epoch test"),
        configuration: ContentHash::of(b"test"),
    }
}
fn schedule(model: &ValidatedModel) -> Schedule {
    Schedule::build_with_publications(
        model,
        vec![
            stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
            stage(
                "v1",
                vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
                vec![RelationUse::of::<Literal>()],
            ),
            stage("result", vec![], vec![RelationUse::of::<ResultRow>()]),
            stage(
                "reader",
                vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
                vec![RelationUse::of::<Package>()],
            ),
        ],
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
            PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["v1", "result"]),
        ],
    )
    .unwrap()
}
#[tokio::test]
async fn closed_epochs_isolate_private_deltas_deduplicate_and_publish_results_atomically() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let first = Literal::None;
    let second = Literal::Bool { value: true };
    let initial = Batch::new(&model, vec![first.clone()], &budget).unwrap();
    let mut v0 = execution.begin("v0").unwrap();
    v0.write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    v0.complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let old;
    {
        let mut v1 = execution.begin("v1").unwrap();
        old = v1.read::<Literal>().unwrap().source().unwrap().clone();
        let rows = Batch::new(&model, vec![first.clone(), second.clone()], &budget).unwrap();
        v1.write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
            .await
            .unwrap();
        // Equal ID/payload across batches is admitted; the candidate canonical prefix has one row.
        v1.write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
            .await
            .unwrap();
        v1.complete(&attempt, ProviderOutcome::Complete)
            .await
            .unwrap();
    }
    let view = format!(
        "{}.{}",
        g.schema(),
        VocabularyEpoch::Facts.view(Literal::NAME)
    );
    let count: i64 =
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {view}")))
            .fetch_one(&db.writer)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let canonical = format!("{}.{}", g.schema(), Literal::NAME);
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!("SELECT id FROM {canonical}")))
            .fetch_all(&db.writer)
            .await
            .is_err(),
        "importer cannot bypass the closed-prefix view"
    );
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {canonical} DEFAULT VALUES"
        )))
        .execute(&db.writer)
        .await
        .is_err()
    );
    for denied in [
        format!("UPDATE {canonical} SET id=id WHERE false"),
        format!("DELETE FROM {canonical} WHERE false"),
        format!(
            "SELECT id FROM {}.{}",
            g.schema(),
            VocabularyEpoch::Dispatch.view(Literal::NAME)
        ),
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(denied))
                .execute(&db.writer)
                .await
                .is_err()
        );
    }
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex') AND stage_name IN ('v1','result')").bind(g.hex()).fetch_one(&db.superuser).await.unwrap();
    assert_eq!(before, 0);
    let mut result = execution.begin("result").unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: second.id() }], &budget).unwrap();
    result
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    result
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        VocabularyEpoch::Dispatch.view(Literal::NAME)
    )))
    .fetch_one(&db.writer)
    .await
    .unwrap();
    assert_eq!(count, 2);
    let count: i64 =
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {view}")))
            .fetch_one(&db.writer)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let mut reader = execution.begin("reader").unwrap();
    assert_eq!(reader.read::<Literal>().unwrap().source(), Some(&old));
    // Reverification after a later close hashes epoch 0, never the larger canonical base.
    let contract = attempt.read_contract(&reader).await.unwrap();
    assert_eq!(contract.sources()[Literal::NAME].receipt().rows, 1);
    let empty = Batch::<Package>::new(&model, vec![], &budget).unwrap();
    reader
        .write::<Package, _>(async |p| attempt.copy(p, &empty).await)
        .await
        .unwrap();
    reader
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let sealed = attempt.seal(execution.finish().unwrap()).await.unwrap();
    let validated = sealed.validate().await.unwrap();
    validated.publish().await.unwrap();
    let mut lease = store.pin(&db.reader, g, budget.clone()).await.unwrap();
    assert_eq!(lease.read::<Literal>().await.unwrap().rows().len(), 2);
    assert_eq!(lease.read::<ResultRow>().await.unwrap().rows(), rows.rows());
    lease.release().await.unwrap();
    store.retire(g).await.unwrap();
}
#[tokio::test]
async fn missing_reference_rolls_back_every_group_output_and_poisoning_prevents_retry() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let first = Literal::None;
    let initial = Batch::new(&model, vec![first], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let second = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![second], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let missing = Literal::Bool { value: false };
    let rows = Batch::new(
        &model,
        vec![ResultRow {
            value: missing.id(),
        }],
        &budget,
    )
    .unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    assert!(
        stage
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        Literal::NAME
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(count, 1, "failed close cannot expose vocabulary");
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.epoch_results",
        g.schema()
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(count, 0, "failed close cannot expose ordinary results");
    assert!(execution.begin("reader").is_err());
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}

#[tokio::test]
async fn cancelled_group_closure_keeps_sealed_deltas_private_and_attempt_terminal() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let first = Literal::None;
    let initial = Batch::new(&model, vec![first], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let second = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![second.clone()], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let mut blocker = db.superuser.begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {}.{} IN ACCESS EXCLUSIVE MODE",
        g.schema(),
        Literal::NAME
    )))
    .execute(&mut *blocker)
    .await
    .unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: second.id() }], &budget).unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            stage.complete(&attempt, ProviderOutcome::Complete)
        )
        .await
        .is_err()
    );
    blocker.rollback().await.unwrap();
    assert!(execution.begin("reader").is_err());
    assert!(execution.finish().is_err());
    // Drain SQLx cancellation cleanup through the lifecycle connection before scanning a table
    // on which the abandoned transaction may still hold a lock.
    attempt
        .fail(&ModelError::Invalid("cancelled publication".into()))
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        Literal::NAME
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(count, 1);
    store.abort(g).await.unwrap();
}
#[tokio::test]
async fn physically_present_future_rows_do_not_satisfy_candidate_prefix_references() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let initial = Batch::new(&model, vec![Literal::None], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let second = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![second.clone()], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let relation = model.require::<Literal>().unwrap();
    let columns = relation
        .schema()
        .fields()
        .iter()
        .map(|f| format!("\"{}\"", f.name()))
        .collect::<Vec<_>>()
        .join(",");
    let delta: String=sqlx::query_scalar("SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relkind='r' AND left(c.relname,8)='__delta_' AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname=$2)").bind(g.schema()).bind(relation.sum().unwrap().tag).fetch_one(&db.superuser).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.{} ({columns},introduced_epoch) SELECT {columns},5 FROM {}.{}",
        g.schema(),
        Literal::NAME,
        g.schema(),
        delta
    )))
    .execute(&db.superuser)
    .await
    .unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: second.id() }], &budget).unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    assert!(
        stage
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        VocabularyEpoch::Facts.view(Literal::NAME)
    )))
    .fetch_one(&db.writer)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}
#[tokio::test]
async fn a_later_result_cannot_be_verified_using_an_earlier_vocabulary_target() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let baseline = schedule(&model);
    let mut declarations = baseline.stages().to_vec();
    declarations
        .iter_mut()
        .find(|s| s.name == "reader")
        .unwrap()
        .inputs
        .push(RelationUse::stored::<ResultRow>());
    let schedule = Schedule::build_with_publications(
        &model,
        declarations,
        &[],
        Profile::Catalog,
        baseline.publication_groups().to_vec(),
    )
    .unwrap();
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let initial = Batch::new(&model, vec![Literal::None], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let later = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![later.clone()], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: later.id() }], &budget).unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let reader = execution.begin("reader").unwrap();
    assert_eq!(
        reader
            .read::<ResultRow>()
            .unwrap()
            .source()
            .unwrap()
            .prefix(),
        Some(VocabularyEpoch::Dispatch)
    );
    assert!(
        attempt.read_contract(&reader).await.is_err(),
        "a base row visible at epoch1 cannot satisfy a declared epoch0 target"
    );
    drop(reader);
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}
struct LostAcknowledgement<'a>(&'a lctx_postgres::generations::GenerationAttempt);
impl StageSink for LostAcknowledgement<'_> {
    async fn copy<R: Record>(
        &self,
        p: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> Result<(), ModelError> {
        self.0.copy(p, batch).await
    }
    async fn complete(&self, c: StageCompletion) -> Result<CompletedStage, ModelError> {
        self.0.complete(c).await
    }
    async fn compute(&self, c: StageCompletion) -> Result<ComputedStage, ModelError> {
        self.0.compute(c).await
    }
    async fn close_group(&self, g: GroupCompletion) -> Result<ClosedGroup, ModelError> {
        self.0.close_group(g).await?;
        Err(ModelError::infrastructure(
            Infrastructure::Unconfirmed,
            "injected lost publication acknowledgement after real commit",
        ))
    }
}
#[tokio::test]
async fn lost_group_commit_acknowledgement_never_mints_read_sources_or_allows_retry() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let initial = Batch::new(&model, vec![Literal::None], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let later = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![later.clone()], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: later.id() }], &budget).unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    let error = stage
        .complete(&LostAcknowledgement(&attempt), ProviderOutcome::Complete)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ModelError::Infrastructure {
            class: Infrastructure::Unconfirmed,
            ..
        }
    ));
    let closed:bool=sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=decode($1,'hex') AND epoch=1").bind(g.hex()).fetch_one(&db.superuser).await.unwrap();
    assert!(
        closed,
        "the real close committed before its acknowledgement was lost"
    );
    assert!(execution.begin("reader").is_err());
    assert!(execution.finish().is_err());
    attempt.fail(&error).await.unwrap();
    store.abort(g).await.unwrap();
}
#[tokio::test]
async fn frozen_delta_content_is_reverified_before_any_group_merge() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = schedule(&model);
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let initial = Batch::new(&model, vec![Literal::None], &budget).unwrap();
    let mut stage = execution.begin("v0").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &initial).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let later = Literal::Bool { value: true };
    let rows = Batch::new(&model, vec![later.clone()], &budget).unwrap();
    let mut stage = execution.begin("v1").unwrap();
    stage
        .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    stage
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let tag = model.require::<Literal>().unwrap().sum().unwrap().tag;
    let delta:String=sqlx::query_scalar("SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relkind='r' AND left(c.relname,8)='__delta_' AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname=$2)").bind(g.schema()).bind(tag).fetch_one(&db.superuser).await.unwrap();
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {}.{} DEFAULT VALUES",
            g.schema(),
            delta
        )))
        .execute(&db.writer)
        .await
        .is_err(),
        "sealed delta INSERT grant is revoked"
    );
    // Owner corruption is a negative control for the frozen receipt, not an allowed producer write.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM {}.{}",
        g.schema(),
        delta
    )))
    .execute(&db.superuser)
    .await
    .unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: later.id() }], &budget).unwrap();
    let mut stage = execution.begin("result").unwrap();
    stage
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    assert!(
        stage
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        Literal::NAME
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}
#[tokio::test]
async fn inherited_prefix_is_verified_by_the_store_with_original_producer_receipt() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<ResultRow>(),
            Relation::of::<Package>(),
        ])
        .unwrap(),
    );
    let schedule = Schedule::build_with_publications(
        &model,
        vec![
            stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
            stage(
                "result",
                vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
                vec![RelationUse::of::<ResultRow>()],
            ),
            stage(
                "reader",
                vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Dispatch)],
                vec![RelationUse::of::<Package>()],
            ),
        ],
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
            PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["result"]),
        ],
    )
    .unwrap();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let budget = budget();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let literal = Literal::None;
    let rows = Batch::new(&model, vec![literal.clone()], &budget).unwrap();
    let mut v0 = execution.begin("v0").unwrap();
    v0.write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    v0.complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let mut result = execution.begin("result").unwrap();
    let old = result.read::<Literal>().unwrap().source().unwrap().clone();
    let rows = Batch::new(
        &model,
        vec![ResultRow {
            value: literal.id(),
        }],
        &budget,
    )
    .unwrap();
    result
        .write::<ResultRow, _>(async |p| attempt.copy(p, &rows).await)
        .await
        .unwrap();
    result
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let mut reader = execution.begin("reader").unwrap();
    let inherited = reader.read::<Literal>().unwrap().source().unwrap().clone();
    assert_eq!(inherited.producer(), "v0");
    assert_eq!(inherited.receipt(), old.receipt());
    assert_eq!(inherited.prefix(), Some(VocabularyEpoch::Dispatch));
    assert_eq!(old.prefix(), Some(VocabularyEpoch::Facts));
    let contract = attempt.read_contract(&reader).await.unwrap();
    assert_eq!(contract.sources()[Literal::NAME], inherited);
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        inherited.physical_relation()
    )))
    .fetch_one(&db.writer)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let empty = Batch::<Package>::new(&model, vec![], &budget).unwrap();
    reader
        .write::<Package, _>(async |p| attempt.copy(p, &empty).await)
        .await
        .unwrap();
    reader
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
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
    store.retire(g).await.unwrap();
}
#[tokio::test]
async fn a_closed_literal_set_cannot_gain_members_in_a_later_epoch() {
    use lctx_model::domain::value::{LiteralSet, LiteralSetMember};
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<LiteralSet>(),
            Relation::of::<LiteralSetMember>(),
        ])
        .unwrap(),
    );
    let outputs = vec![
        RelationUse::of::<Literal>(),
        RelationUse::of::<LiteralSet>(),
        RelationUse::of::<LiteralSetMember>(),
    ];
    let schedule = Schedule::build_with_publications(
        &model,
        vec![
            stage("v0", vec![], outputs.clone()),
            stage("v1", vec![], outputs),
        ],
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
            PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["v1"]),
        ],
    )
    .unwrap();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut execution = schedule.execute();
    let budget = budget();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    let original = Literal::None;
    let (set, members) = LiteralSet::of([original.id()]);
    let mut first = execution.begin("v0").unwrap();
    let literals = Batch::new(&model, vec![original], &budget).unwrap();
    let sets = Batch::new(&model, vec![set.clone()], &budget).unwrap();
    let membership = Batch::new(&model, members, &budget).unwrap();
    first
        .write::<Literal, _>(async |p| attempt.copy(p, &literals).await)
        .await
        .unwrap();
    first
        .write::<LiteralSet, _>(async |p| attempt.copy(p, &sets).await)
        .await
        .unwrap();
    first
        .write::<LiteralSetMember, _>(async |p| attempt.copy(p, &membership).await)
        .await
        .unwrap();
    first
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let extra = Literal::Bool { value: true };
    let mut later = execution.begin("v1").unwrap();
    let literals = Batch::new(&model, vec![extra.clone()], &budget).unwrap();
    let sets = Batch::<LiteralSet>::new(&model, vec![], &budget).unwrap();
    let membership = Batch::new(
        &model,
        vec![LiteralSetMember {
            set: set.id(),
            value: extra.id(),
        }],
        &budget,
    )
    .unwrap();
    later
        .write::<Literal, _>(async |p| attempt.copy(p, &literals).await)
        .await
        .unwrap();
    later
        .write::<LiteralSet, _>(async |p| attempt.copy(p, &sets).await)
        .await
        .unwrap();
    later
        .write::<LiteralSetMember, _>(async |p| attempt.copy(p, &membership).await)
        .await
        .unwrap();
    assert!(
        later
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .is_err()
    );
    for name in [Literal::NAME, LiteralSet::NAME, LiteralSetMember::NAME] {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {}.{}",
            g.schema(),
            name
        )))
        .fetch_one(&db.superuser)
        .await
        .unwrap();
        assert_eq!(count, 1, "failed merge changed {name}");
    }
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}
