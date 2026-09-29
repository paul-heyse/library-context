use std::sync::Arc;
use lctx_model::domain::{*, input::Package, stages::*};
use lctx_model::domain::resources::ResourceBudget;
use lctx_postgres::generations::{Error, GenerationStore};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{runners::AsyncRunner, ImageExt}};

#[tokio::test]
async fn generation_sink_requires_its_execution_and_cannot_bypass_sealing_receipt() {
    let (image, tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PG18 image required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Package>()]).unwrap());
    let store = GenerationStore::install(owner.clone(), model.clone()).await.unwrap();
    let schedule = Schedule::build(&model, vec![Stage {
        name: "packages", inputs: vec![], outputs: vec![RelationUse::of::<Package>()],
        profiles: vec![Profile::Catalog], effect: Effect::Extraction,
        code: ContentHash::of(b"package producer"), configuration: ContentHash::of(b"configuration"),
    }], &[RelationUse::of::<Package>()], Profile::Catalog).unwrap();
    let rows = Batch::new(&model, vec![Package { name: "example".into() }], &budget()).unwrap();
    let mut execution = schedule.execute();
    let attempt_budget = ResourceBudget::fixed(1 << 20).unwrap();
    let attempt = store.begin_conformance(writer.clone(), &mut execution, attempt_budget.clone()).await.unwrap();
    let generation = attempt.generation();
    assert!(store.begin_conformance(writer.clone(), &mut execution, attempt_budget.clone()).await.is_err(), "a sibling generation cannot share an execution");
    assert!(matches!(store.copy(&writer, generation, &rows, &budget()).await, Err(Error::Contract)));
    assert!(matches!(store.seal(generation).await, Err(Error::Contract)));

    let mut foreign = schedule.execute();
    let mut foreign_stage = foreign.begin("packages").unwrap();
    assert!(foreign_stage.write::<Package,_>(async |permit| attempt.copy(permit, &rows).await).await.is_err());
    drop(foreign_stage);
    assert!(foreign.finish().is_err());

    let mut stage = execution.begin("packages").unwrap();
    stage.write::<Package,_>(async |permit| attempt.copy(permit, &rows).await).await.unwrap();
    assert_eq!(attempt_budget.reserved(), 0);
    stage.finish(ProviderOutcome::Complete).unwrap();
    assert_eq!(attempt.seal(execution.finish().unwrap()).await.unwrap(), generation);
    store.validate(generation).await.unwrap();
    store.publish(generation).await.unwrap();
    assert!(matches!(store.select(generation).await, Err(Error::Frontier)));
    let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
    assert_eq!(lease.read::<Package>().await.unwrap().rows(), rows.rows());
    lease.release().await.unwrap();
    store.retire(generation).await.unwrap();

    // An identical schedule's receipt still cannot seal another execution's generation.
    let mut untouched = schedule.execute();
    let abandoned = store.begin_conformance(writer.clone(), &mut untouched, attempt_budget.clone()).await.unwrap();
    let abandoned_id = abandoned.generation();
    let mut other = schedule.execute();
    let mut stage = other.begin("packages").unwrap();
    stage.write::<Package,_>(async |_| Ok(())).await.unwrap();
    stage.finish(ProviderOutcome::Complete).unwrap();
    assert!(matches!(abandoned.seal(other.finish().unwrap()).await, Err(Error::Contract)));
    store.abort(abandoned_id).await.unwrap();

    // No wire buffer may escape the shared attempt_budget; resource refusal poisons execution.
    let mut execution = schedule.execute();
    let attempt = store.begin_conformance(writer.clone(), &mut execution, attempt_budget.clone()).await.unwrap();
    let id = attempt.generation();
    let blocker = attempt_budget.reserve("other stage", attempt_budget.limit()).unwrap();
    let mut stage = execution.begin("packages").unwrap();
    assert!(matches!(stage.write::<Package,_>(async |permit| attempt.copy(permit, &rows).await).await, Err(ModelError::Resource { .. })));
    drop(stage);
    assert!(execution.finish().is_err());
    drop(blocker);
    assert_eq!(attempt_budget.reserved(), 0);
    store.abort(id).await.unwrap();

    // Admit a small first row, then refuse a larger wire row during active COPY. The server
    // must acknowledge COPY abort and transaction rollback before the failure is returned.
    let large = Package { name: "a".repeat(4096) };
    let small = (0..1000).map(|n| Package { name: format!("small-{n}") }).find(|p| p.id() < large.id()).unwrap();
    let wire_rows = Batch::new(&model,vec![small.clone(),large], &budget()).unwrap();
    assert_eq!(wire_rows.rows()[0],small);
    for admitted in [false,true] {
        let wire_budget = ResourceBudget::fixed(wire_rows.arrow().schema().fields().len()*4096 + if admitted { 32768 } else { 128 }).unwrap();
        let mut execution = schedule.execute();
        let attempt = store.begin_conformance(writer.clone(),&mut execution,wire_budget.clone()).await.unwrap();
        let id = attempt.generation(); let mut stage = execution.begin("packages").unwrap();
        let result = stage.write::<Package,_>(async |permit| attempt.copy(permit,&wire_rows).await).await;
        if admitted { result.unwrap(); stage.finish(ProviderOutcome::Complete).unwrap(); }
        else { assert!(matches!(result,Err(ModelError::Resource { .. }))); drop(stage); assert!(execution.finish().is_err()); }
        assert_eq!(wire_budget.reserved(),0);
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.packages",id.schema()))).fetch_one(&owner).await.unwrap();
        assert_eq!(count,if admitted { 2 } else { 0 });
        assert!(store.publish(id).await.is_err());
        store.abort(id).await.unwrap();
    }

    for empty_write in [false, true] {
        let mut execution = schedule.execute();
        let attempt = store.begin_conformance(writer.clone(), &mut execution, attempt_budget.clone()).await.unwrap();
        let id = attempt.generation();
        let mut stage = execution.begin("packages").unwrap();
        let empty = Batch::<Package>::new(&model, vec![], &budget()).unwrap();
        stage.write::<Package,_>(async |permit| {
            if empty_write { attempt.copy(permit, &empty).await?; }
            Ok(())
        }).await.unwrap();
        stage.finish(ProviderOutcome::Complete).unwrap();
        let sealed = attempt.seal(execution.finish().unwrap()).await;
        if empty_write {
            assert_eq!(sealed.unwrap(), id);
            store.validate(id).await.unwrap();
        } else { assert!(matches!(sealed, Err(Error::State)), "a successful no-op did not write an output"); }
        store.abort(id).await.unwrap();
    }

    // The permanent writer path: transfer-bounded batches through the stage-bound sink, all
    // charged to the attempt budget and released once the stage completes.
    let mut execution = schedule.execute();
    let attempt = store.begin_conformance(writer.clone(), &mut execution, attempt_budget.clone()).await.unwrap();
    let id = attempt.generation();
    let mut output = StageOutput::new(execution.begin("packages").unwrap(), &attempt, &model, attempt_budget.clone(),
        lctx_model::domain::batching::TransferLimits::default()).unwrap();
    output.declare::<Package>().unwrap();
    for n in 0..5000 { output.push(Package { name: format!("p{n}") }).await.unwrap(); }
    output.push(Package { name: "p0".into() }).await.unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    assert_eq!(attempt_budget.reserved(), 0);
    assert_eq!(attempt.seal(execution.finish().unwrap()).await.unwrap(), id);
    store.validate(id).await.unwrap();
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.packages", id.schema()))).fetch_one(&owner).await.unwrap();
    assert_eq!(count, 5000, "a repeated row is emitted once across transfer batches");
    store.abort(id).await.unwrap();
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
