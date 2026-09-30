//! Real native facts -> frozen checkpoint -> completed-stage reads -> typed N1 outputs.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput, bundle::{CapturedInputs, run_stage}, capture::CapturedInput};
use lctx_model::domain::{admission::FrontierContract, normalized::entity_normalization, stages::*, *};
use lctx_postgres::{generations::GenerationStore, roles::{Role, RoleConfig}, testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn normalized_entities_run_from_frozen_facts_and_validate_in_the_store() {
    let runtime = AttemptRuntime::new(RuntimeOptions { memory_bytes: 1 << 30, partitions: 2 }).unwrap();
    let budget = runtime.budget();
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let config = RoleConfig { format: 1, role: Role::Importer, url: db.url("lctx_importer"), max_connections: 4,
        provider_connections: 2, acquire_timeout_seconds: 5, statement_timeout_seconds: 60, lock_timeout_seconds: 10 };
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/normalized_entities");
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(&root, &["entities.py".into(), "dual.py".into(), "dual.pyi".into()], budget).unwrap(), "entities")]));
    let captured_bytes = budget.reserved();
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"normalized entities fixture"));
    let profile = Profile::Catalog;
    let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    declarations.push(entity_normalization::stage());
    let schedule = Schedule::build(&model, declarations, &[], profile).unwrap();
    let facts = FrontierContract::facts(&model, profile).unwrap();
    let mut execution = schedule.execute();
    let attempt = store.begin_conformance(db.writer.clone(), &mut execution, budget.clone()).await.unwrap();
    let id = attempt.generation();
    for declaration in schedule.stages() {
        if declaration.name == "normalize_entities" {
            attempt.checkpoint(&execution, &facts).await.unwrap();
            cpg_core::stage_runtime::run_declared_stage(&mut execution, declaration, async |access| {
                cpg_core::normalize::entities(access, &attempt, &config, &runtime, &model).await
            }, &mut |_| {}).await.unwrap();
        } else {
            let position = providers.iter().position(|p| p.declaration(profile).name == declaration.name).unwrap();
            run_stage(providers.swap_remove(position), execution.begin(declaration.name).unwrap(), &attempt,
                &model, &captured, budget, Default::default()).await.unwrap();
        }
    }
    let validated = attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();
    let counts: (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.symbol_entity_resolutions), (SELECT count(*) FROM {}.provider_symbols)", id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert!(counts.0 > 0); assert_eq!(counts.0, counts.1);
    validated.abort().await.unwrap();
    assert_eq!(budget.reserved(), captured_bytes, "only the retained captured input remains charged");
    drop(captured);
    assert_eq!(budget.reserved(), 0, "all stage and captured input state released");
}
