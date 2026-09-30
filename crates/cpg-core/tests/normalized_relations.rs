//! Real native facts -> frozen checkpoint -> completed-stage N1 through N3 reads and outputs.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput, bundle::{CapturedInputs, run_stage}, capture::CapturedInput};
use lctx_model::domain::{admission::FrontierContract, normalized::{entity_normalization, relation_normalization, callable_normalization}, stages::*, *};
use lctx_postgres::{generations::GenerationStore, roles::{Role, RoleConfig}, testing::DisposableDatabase};
use std::sync::Arc;
async fn run(profile: Profile) {
    let runtime = AttemptRuntime::new(RuntimeOptions { memory_bytes: 1 << 30, partitions: 2 }).unwrap();
    let budget = runtime.budget();
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let config = RoleConfig { format: 1, role: Role::Importer, url: db.url("lctx_importer"), max_connections: 4,
        provider_connections: 2, acquire_timeout_seconds: 5, statement_timeout_seconds: 60, lock_timeout_seconds: 10 };
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/normalized_relations");
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(&root, &["relations.py".into(), "dual.py".into(), "dual.pyi".into(), "guide.md".into()], budget).unwrap(), "entities")]));
    let captured_bytes = budget.reserved();
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"normalized entities fixture"));
    let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    declarations.push(entity_normalization::stage());
    declarations.push(relation_normalization::stage(profile));
    declarations.push(callable_normalization::stage(profile));
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
        } else if declaration.name == "normalize_relations" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution, declaration, async |access| {
                cpg_core::normalize::relations(access, &attempt, &config, &runtime, &model).await
            }, &mut |_| {}).await.unwrap();
        } else if declaration.name == "normalize_callables" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution, declaration, async |access| {
                cpg_core::normalize::callables(access, &attempt, &config, &runtime, &model).await
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
    let leaves: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.flow_test_leaf_observations), (SELECT count(*) FROM {}.test_operand_type_assessments), (SELECT count(*) FROM {}.test_operand_type_links)", id.schema(), id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(leaves.0, leaves.1);
    match profile { Profile::Catalog => assert_eq!(leaves, (0, 0, 0)), Profile::Behavioral => assert!(leaves.0 > 0 && leaves.2 > 0) }
    let signatures: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.signature_observations), (SELECT count(*) FROM {}.signature_variants), (SELECT count(*) FROM {}.effective_callable_assessments)", id.schema(), id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert!(signatures.0 > 0 && signatures.2 > 0); assert_eq!(signatures.0, signatures.1);
    validated.abort().await.unwrap();
    assert_eq!(budget.reserved(), captured_bytes, "only the retained captured input remains charged");
    drop(captured);
    assert_eq!(budget.reserved(), 0, "all stage and captured input state released");
}

#[tokio::test]
async fn catalog_normalizes_completed_facts_with_explicitly_unrequested_flow() { run(Profile::Catalog).await; }
#[tokio::test]
async fn behavioral_normalizes_completed_facts_including_exact_test_operands() { run(Profile::Behavioral).await; }
