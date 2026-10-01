//! Real provider registration uses its receipt's literal epoch view, after later vocabulary closes.
use cpg_core::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, RuntimeOptions},
};
use lctx_model::domain::{input::Package, stages::*, value::Literal, *};
use lctx_postgres::{generations::GenerationStore, roles::RoleConfig, testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn source_bound_provider_reads_the_old_literal_prefix_after_later_publication() {
    let db = DisposableDatabase::start().await;
    let directory = tempfile::tempdir().unwrap();
    db.write_configs(directory.path()).unwrap();
    let importer = RoleConfig::load(&directory.path().join("postgres-importer.json")).unwrap();
    let model = Arc::new(
        ValidatedModel::validate(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
            .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"prefix provider control"),
        configuration: ContentHash::of(b"test"),
    };
    let schedule = Schedule::build_with_publications(
        &model,
        vec![
            stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
            stage("v1", vec![], vec![RelationUse::of::<Literal>()]),
            stage(
                "reader",
                vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Facts)],
                vec![RelationUse::of::<Package>()],
            ),
        ],
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, vec!["v0"]),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["v1"]),
        ],
    )
    .unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions::default()).unwrap();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, runtime.budget().clone())
        .await
        .unwrap();
    for (name, rows) in [
        ("v0", vec![Literal::None]),
        ("v1", vec![Literal::Bool { value: true }]),
    ] {
        let rows = Batch::new(&model, rows, runtime.budget()).unwrap();
        let mut stage = execution.begin(name).unwrap();
        stage
            .write::<Literal, _>(async |p| attempt.copy(p, &rows).await)
            .await
            .unwrap();
        stage
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .unwrap();
    }
    let reader = execution.begin("reader").unwrap();
    let permit = reader.read::<Literal>().unwrap();
    assert_eq!(
        permit.source().unwrap().prefix(),
        Some(PublicationBoundary::Facts)
    );
    let read = AttemptSession::open(
        &importer,
        &attempt,
        &reader,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .unwrap();
    let session = runtime.session(&reader);
    session
        .register(&permit, read.table(&permit).unwrap())
        .unwrap();
    let batches = session
        .query("SELECT * FROM literal_values")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let rows = batches
        .iter()
        .flat_map(|b| Literal::decode(b).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![Literal::None],
        "source-bound provider must not expose epoch1's boolean literal"
    );
    read.close().await.unwrap();
    drop(session);
    let mut output = StageOutput::new(
        reader,
        &attempt,
        &model,
        runtime.budget().clone(),
        Default::default(),
    )
    .unwrap();
    output.declare::<Package>().unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    let g = attempt
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
async fn native_facts_then_condition_groups_enforce_the_shared_canonical_catalog() {
    use cpg_extract::{
        acquisition::AcquiredInput,
        bundle::{CapturedInputs, run_stage},
        capture::CapturedInput,
    };
    use lctx_model::domain::{
        conditions::{Condition, ConditionNode},
        resources::ResourceBudget,
    };
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("api.py"), "def f(x):\n    return x\n").unwrap();
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(root.path(), &["api.py".into()], &budget).unwrap(),
        "condition epoch",
    )]));
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"condition epoch fixture"));
    let mut declarations: Vec<_> = providers
        .iter()
        .map(|p| p.declaration(Profile::Catalog))
        .collect();
    let facts = declarations
        .iter()
        .filter(|s| s.outputs.iter().any(|r| is_vocabulary(r.name())))
        .map(|s| s.name)
        .collect();
    for name in ["canonical", "orphan"] {
        declarations.push(Stage {
            name,
            inputs: vec![],
            outputs: vec![
                RelationUse::of::<ConditionNode>(),
                RelationUse::of::<Condition>(),
            ],
            contributes: vec![],
            coverage: vec![],
            provider: None,
            profiles: vec![Profile::Catalog],
            effect: Effect::Pure,
            code: ContentHash::of(b"condition group"),
            configuration: ContentHash::of(b"test"),
        });
    }
    let schedule = Schedule::build_with_publications(
        &model,
        declarations,
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["canonical"]),
            PublicationGroup::new(PublicationBoundary::BaseSemantic, vec!["orphan"]),
        ],
    )
    .unwrap();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let g = attempt.generation();
    for declaration in schedule
        .stages()
        .iter()
        .filter(|s| s.name != "canonical" && s.name != "orphan")
    {
        let position = providers
            .iter()
            .position(|p| p.declaration(Profile::Catalog).name == declaration.name)
            .unwrap();
        run_stage(
            providers.swap_remove(position),
            execution.begin(declaration.name).unwrap(),
            &attempt,
            &model,
            &captured,
            &budget,
            Default::default(),
        )
        .await
        .unwrap();
    }
    let before: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        ConditionNode::NAME
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(
        before, 1,
        "native facts retain the canonical true condition"
    );
    let mut canonical = execution.begin("canonical").unwrap();
    let nodes = Batch::new(&model, vec![ConditionNode::True], &budget).unwrap();
    let conditions = Batch::new(
        &model,
        vec![Condition {
            root: ConditionNode::True.id(),
        }],
        &budget,
    )
    .unwrap();
    canonical
        .write::<ConditionNode, _>(async |p| attempt.copy(p, &nodes).await)
        .await
        .unwrap();
    canonical
        .write::<Condition, _>(async |p| attempt.copy(p, &conditions).await)
        .await
        .unwrap();
    canonical
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap();
    let mut orphan = execution.begin("orphan").unwrap();
    let nodes = Batch::new(&model, vec![ConditionNode::False], &budget).unwrap();
    let conditions = Batch::<Condition>::new(&model, vec![], &budget).unwrap();
    orphan
        .write::<ConditionNode, _>(async |p| attempt.copy(p, &nodes).await)
        .await
        .unwrap();
    orphan
        .write::<Condition, _>(async |p| attempt.copy(p, &conditions).await)
        .await
        .unwrap();
    let error = orphan
        .complete(&attempt, ProviderOutcome::Complete)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unreferenced nodes"), "{error}");
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.{}",
        g.schema(),
        ConditionNode::NAME
    )))
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert!(execution.finish().is_err());
    attempt.abort().await.unwrap();
}
