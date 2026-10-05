//! Real provider registration uses its receipt's literal epoch view, after later vocabulary closes.
use cpg_core::{
    consumed_rows::{ConsumedInputs, stream},
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, RuntimeOptions},
};
use datafusion_table_providers_postgres::{bounded::ChunkLimits, pool::PoolHealth};
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
        ValidatedModel::declared(vec![
            Relation::of::<Literal>(),
            Relation::of::<Package>(),
            Relation::of::<input::Release>(),
        ])
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
            stage(
                "both_reader",
                vec![
                    RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Dispatch),
                    RelationUse::stored::<Package>(),
                ],
                vec![RelationUse::of::<input::Release>()],
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
    for index in 0..512 {
        output
            .push(Package {
                name: format!("cancel-{index}"),
            })
            .await
            .unwrap();
    }
    output.finish(ProviderOutcome::Complete).await.unwrap();
    let both = execution.begin("both_reader").unwrap();
    let read = AttemptSession::open(
        &importer,
        &attempt,
        &both,
        model.clone(),
        ProviderOptions {
            chunks: ChunkLimits {
                rows: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut consumed = ConsumedInputs::new(
        vec![
            ValidationInput::of::<Literal>(&["id"]),
            ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts),
            ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Dispatch),
        ],
        runtime.budget(),
    )
    .unwrap();
    let mut values = std::collections::BTreeMap::new();
    let mut identities = std::collections::BTreeMap::new();
    while let Some((input, permit)) = consumed.next::<Literal>(&both).unwrap() {
        let epoch = input.prefix().unwrap();
        let session = runtime.session(&both);
        stream(&permit, &read, &session, |_, batch| {
            let rows = Literal::decode(batch)?;
            values
                .entry(epoch)
                .or_insert_with(Vec::new)
                .extend(rows.clone());
            identities
                .entry(epoch)
                .or_insert_with(Vec::new)
                .extend(rows.into_iter().map(|row| row.id()));
            Ok(())
        })
        .await
        .unwrap();
    }
    consumed.finish(both.stage().name).unwrap();
    assert_eq!(
        values.len(),
        2,
        "default and explicit latest alias must share one source"
    );
    assert_eq!(values[&PublicationBoundary::Facts], vec![Literal::None]);
    let mut latest = values[&PublicationBoundary::Dispatch].clone();
    latest.sort_by_key(Record::id);
    let mut expected = vec![Literal::None, Literal::Bool { value: true }];
    expected.sort_by_key(Record::id);
    assert_eq!(latest, expected);
    for (epoch, rows) in &values {
        assert_eq!(
            identities[epoch],
            rows.iter().map(Record::id).collect::<Vec<_>>(),
            "each resolved batch broadcasts once to both consumers"
        );
    }
    let permit = both.read::<Literal>().unwrap();
    let session = runtime.session(&both);
    let error = stream(&permit, &read, &session, |_, _| {
        Err(ModelError::Invalid("callback sentinel".into()))
    })
    .await
    .unwrap_err();
    assert!(matches!(error, ModelError::Invalid(ref message) if message == "callback sentinel"));
    assert!(
        stream(&permit, &read, &session, |_, _| Ok(()))
            .await
            .is_err(),
        "same session refuses duplicate registration"
    );
    drop(session);
    let mut recovered = 0;
    stream(&permit, &read, &runtime.session(&both), |_, batch| {
        recovered += batch.num_rows();
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        recovered, 2,
        "callback failure drops the query and drains without poisoning the reader"
    );
    let mut undeclared = ConsumedInputs::new(
        vec![ValidationInput::of::<input::Release>(&["id"])],
        runtime.budget(),
    )
    .unwrap();
    assert!(
        undeclared.next::<input::Release>(&both).is_err(),
        "owned consumption cannot bypass its stage grant"
    );
    drop(undeclared);
    let permit = both.read::<Package>().unwrap();
    let session = runtime.session(&both);
    let cancel = tokio::sync::Notify::new();
    let mut observed = 0;
    tokio::select! {
        biased;
        _ = cancel.notified() => {},
        result = stream(&permit, &read, &session, |_, batch| {observed += batch.num_rows(); cancel.notify_one(); Ok(())}) => panic!("helper completed instead of cancelling mid-stream: {result:?}"),
    }
    assert!(
        observed > 0 && observed < 512,
        "cancellation occurs after consumption starts and before it completes"
    );
    drop(session);
    for _ in 0..50 {
        if matches!(read.health(), PoolHealth::Ready {connections,idle} if connections == importer.provider_connections && idle == connections)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        matches!(read.health(), PoolHealth::Ready {connections,idle} if connections == importer.provider_connections && idle == connections),
        "cancelled helper returns all configured provider connections after draining: {:?}",
        read.health()
    );
    let mut recovered = 0;
    stream(&permit, &read, &runtime.session(&both), |_, batch| {
        recovered += batch.num_rows();
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(recovered, 512);
    read.close().await.unwrap();
    let mut output = StageOutput::new(
        both,
        &attempt,
        &model,
        runtime.budget().clone(),
        Default::default(),
    )
    .unwrap();
    output.declare::<input::Release>().unwrap();
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
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(root.path(), &["api.py".into()], &budget).unwrap(),
            "condition epoch",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog, &budget)
            .unwrap(),
    ));
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

#[test]
fn consumed_inventory_refuses_unloaded_rows_and_releases_its_reservation() {
    use lctx_model::domain::resources::ResourceBudget;
    let small = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        ConsumedInputs::new(vec![ValidationInput::of::<Literal>(&["id"])], &small),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(small.reserved(), 0);
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let inputs =
        ConsumedInputs::new(vec![ValidationInput::of::<Literal>(&["id"])], &budget).unwrap();
    assert!(budget.reserved() > 0);
    assert!(
        inputs.finish("test_reader").is_err(),
        "a missing typed loader must refuse instead of silently omitting consumption"
    );
    assert_eq!(budget.reserved(), 0);
}
