#[path = "../../lctx-model/tests/fixtures/validation_views.rs"]
mod fixture;
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use fixture::*;
use lctx_model::domain::{stages::*, value::Literal, *};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="validation_future_probes",publication_checks=future_check,semantic_source=include_bytes!("validation_views.rs"))]
struct FutureProbe {
    #[model(key)]
    marker: bool,
}
fn future_check() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        name: "future_view_refusal_control",
        inputs: vec![
            ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Dispatch),
        ],
        create: Arc::new(|_| Box::new(Noop)),
    }]
}
struct Noop;
impl PublicationCheck for Noop {
    fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Ok(())
    }
    fn finish(self: Box<Self>, _: &[CompletedRelation], _: Profile) -> Result<(), ModelError> {
        Ok(())
    }
}

#[tokio::test]
async fn validation_cannot_widen_a_facts_grant_to_later_vocabulary_even_after_it_closes() {
    for close_later in [false, true] {
        let model = Arc::new(
            ValidatedModel::validate(vec![
                Relation::of::<Literal>(),
                Relation::of::<FutureProbe>(),
            ])
            .unwrap(),
        );
        let schedule = Schedule::build_with_publications(
            &model,
            vec![
                stage("facts", vec![], vec![RelationUse::of::<Literal>()]),
                stage("later", vec![], vec![RelationUse::of::<Literal>()]),
                stage(
                    "future",
                    vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Facts)],
                    vec![RelationUse::of::<FutureProbe>()],
                ),
            ],
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
                PublicationGroup::new(PublicationBoundary::Dispatch, vec!["later"]),
            ],
        )
        .unwrap();
        let db = DisposableDatabase::start().await;
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let runtime = AttemptRuntime::new(RuntimeOptions {
            memory_bytes: 1 << 24,
            partitions: 1,
        })
        .unwrap();
        let budget = runtime.budget();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap();
        for (name, row) in [
            ("facts", Literal::None),
            ("later", Literal::Bool { value: true }),
        ] {
            if name == "later" && !close_later {
                continue;
            }
            let mut output = StageOutput::new(
                execution.begin(name).unwrap(),
                &attempt,
                &model,
                budget.clone(),
                Default::default(),
            )
            .unwrap();
            output.declare::<Literal>().unwrap();
            output.push(row).await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
        }
        let access = execution.begin("future").unwrap();
        assert!(
            access
                .read_at_epoch::<Literal>(PublicationBoundary::Dispatch)
                .is_err()
        );
        let mut output =
            StageOutput::new(access, &attempt, &model, budget.clone(), Default::default()).unwrap();
        output.declare::<FutureProbe>().unwrap();
        output.push(FutureProbe { marker: true }).await.unwrap();
        assert!(
            output.finish(ProviderOutcome::Complete).await.is_err(),
            "later close cannot widen this relation's Facts grant"
        );
        attempt.abort().await.unwrap();
        assert_eq!(budget.reserved(), 0);
    }
}

#[tokio::test]
async fn real_publication_and_final_replay_route_facts_and_current_views_independently() {
    let model = Arc::new(
        ValidatedModel::validate(vec![
            Relation::of::<Literal>(),
            Relation::of::<Probe>(),
            Relation::of::<ReadProbe>(),
        ])
        .unwrap(),
    );
    let schedule = schedule(&model);
    let db = DisposableDatabase::start().await;
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions {
        memory_bytes: 1 << 24,
        partitions: 1,
    })
    .unwrap();
    let budget = runtime.budget();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    for (name, row) in [
        ("facts", Literal::None),
        ("later", Literal::Bool { value: true }),
    ] {
        let mut output = StageOutput::new(
            execution.begin(name).unwrap(),
            &attempt,
            &model,
            budget.clone(),
            Default::default(),
        )
        .unwrap();
        output.declare::<Literal>().unwrap();
        output.push(row).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
    }
    let mut output = StageOutput::new(
        execution.begin("probe").unwrap(),
        &attempt,
        &model,
        budget.clone(),
        Default::default(),
    )
    .unwrap();
    output.declare::<Probe>().unwrap();
    output.push(Probe { marker: true }).await.unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    db.write_configs(directory.path()).unwrap();
    let roles =
        lctx_postgres::roles::RoleConfig::load(&directory.path().join("postgres-importer.json"))
            .unwrap();
    let access = execution.begin("reader").unwrap();
    let reader = cpg_core::generation_read::AttemptSession::open(
        &roles,
        &attempt,
        &access,
        model.clone(),
        Default::default(),
    )
    .await
    .unwrap();
    for (epoch, expected) in [
        (PublicationBoundary::Facts, 1),
        (PublicationBoundary::Dispatch, 2),
    ] {
        let permit = access.read_at_epoch::<Literal>(epoch).unwrap();
        let session = runtime.session(&access);
        session
            .register(&permit, reader.table(&permit).unwrap())
            .unwrap();
        let query = session.query("SELECT * FROM literal_values").await.unwrap();
        let mut stream = query.execute_stream().await.unwrap();
        let mut rows = 0;
        while let Some(batch) = futures::TryStreamExt::try_next(&mut stream).await.unwrap() {
            rows += batch.num_rows();
        }
        assert_eq!(
            rows, expected,
            "runtime narrowing must retain the exact acknowledged prefix"
        );
    }
    reader.close().await.unwrap();
    let mut output =
        StageOutput::new(access, &attempt, &model, budget.clone(), Default::default()).unwrap();
    output.declare::<ReadProbe>().unwrap();
    output.push(ReadProbe { marker: true }).await.unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    let generation = attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap();
    generation.abort().await.unwrap();
    assert_eq!(budget.reserved(), 0);
}

#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="validation_optional_probes",semantic_source=include_bytes!("validation_views.rs"))]
struct OptionalProbe {
    #[model(key)]
    marker: bool,
    target: Option<Id<ReadProbe>>,
}
#[tokio::test]
async fn inactive_nullable_target_is_allowed_but_a_physical_future_target_is_not_authority() {
    for actual_reference in [false, true] {
        let model = Arc::new(
            ValidatedModel::validate(vec![
                Relation::of::<Literal>(),
                Relation::of::<OptionalProbe>(),
                Relation::of::<ReadProbe>(),
            ])
            .unwrap(),
        );
        let schedule = Schedule::build_with_publications(
            &model,
            vec![
                stage(
                    "first",
                    vec![],
                    vec![
                        RelationUse::of::<Literal>(),
                        RelationUse::of::<OptionalProbe>(),
                    ],
                ),
                stage("future", vec![], vec![RelationUse::of::<ReadProbe>()]),
            ],
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["first"]),
                PublicationGroup::new(PublicationBoundary::Dispatch, vec!["future"]),
            ],
        )
        .unwrap();
        let db = DisposableDatabase::start().await;
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let runtime = AttemptRuntime::new(RuntimeOptions {
            memory_bytes: 1 << 24,
            partitions: 1,
        })
        .unwrap();
        let budget = runtime.budget();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap();
        let g = attempt.generation();
        let target = ReadProbe { marker: true };
        // An operator-injected future row satisfies the physical FK, but has no completed receipt.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {}.{}(generation_id,id,marker) VALUES(decode($1,'hex'),$2,true)",
            g.schema(),
            ReadProbe::NAME
        )))
        .bind(g.hex())
        .bind(target.id().bytes().to_vec())
        .execute(&db.superuser)
        .await
        .unwrap();
        let mut output = StageOutput::new(
            execution.begin("first").unwrap(),
            &attempt,
            &model,
            budget.clone(),
            Default::default(),
        )
        .unwrap();
        output.declare::<Literal>().unwrap();
        output.declare::<OptionalProbe>().unwrap();
        output.push(Literal::None).await.unwrap();
        output
            .push(OptionalProbe {
                marker: true,
                target: actual_reference.then(|| target.id()),
            })
            .await
            .unwrap();
        let result = output.finish(ProviderOutcome::Complete).await;
        if actual_reference {
            assert!(
                result.is_err(),
                "a future ordinary row cannot satisfy an acknowledged group reference"
            );
        } else {
            result.unwrap();
        }
        attempt.abort().await.unwrap();
        assert_eq!(budget.reserved(), 0);
    }
}
