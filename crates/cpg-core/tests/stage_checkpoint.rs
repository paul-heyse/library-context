//! A private frozen facts checkpoint over the real native providers, still inside one attempt.
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::{Availability, Frontier, FrontierContract},
    attribution::FactFamily,
    resources::ResourceBudget,
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::sync::Arc;

#[tokio::test]
async fn facts_checkpoint_validates_immutable_content_without_publishing() {
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("api.py"), "def f(x):\n    return x\n").unwrap();
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(root.path(), &["api.py".into()], &budget).unwrap(),
        "checkpoint",
    )], cpg_extract::native_context::NativeContextConfig::committed(lctx_model::domain::stages::Profile::Catalog, &budget).unwrap()));
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"checkpoint fixture"));
    let profile = Profile::Catalog;
    let schedule = Schedule::build(
        &model,
        providers.iter().map(|p| p.declaration(profile)).collect(),
        &[],
        profile,
    )
    .unwrap();
    let contract = FrontierContract::facts(&model, profile).unwrap();
    let mut execution = schedule.execute();
    let attempt = store
        .begin(db.writer.clone(), &mut execution, &contract, budget.clone())
        .await
        .unwrap();
    let id = attempt.generation();
    for declaration in schedule.stages() {
        let position = providers
            .iter()
            .position(|p| p.declaration(profile).name == declaration.name)
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
    let checkpoint = attempt.checkpoint(&execution, &contract).await.unwrap();
    assert_eq!(checkpoint.generation(), id);
    assert_eq!(checkpoint.admission().frontier(), Frontier::Facts);
    assert_eq!(
        checkpoint.admission().availability()[&FactFamily::Flow],
        Availability::NotRequested
    );
    let state: String = sqlx::query_scalar(
        "SELECT state FROM lctx_model_store.generations WHERE id=decode($1,'hex')",
    )
    .bind(id.hex())
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(state, "staging");
    assert!(store.select(id).await.is_err());
    // Freeze even the explicitly unrequested families' empty tables at the checkpoint boundary.
    let writable: bool =
        sqlx::query_scalar("SELECT has_table_privilege('lctx_importer', $1, 'INSERT')")
            .bind(format!(
                "{}.{}",
                id.schema(),
                lctx_model::domain::flow::FlowUseObservation::NAME
            ))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
    assert!(!writable);
    let check = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(check.clean(), "{check:#?}");
    let validated = attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap();
    assert_eq!(validated.content(), checkpoint.admission().content());
    assert_eq!(validated.admission().unwrap(), checkpoint.admission());
    validated.abort().await.unwrap();
    let mut incomplete = schedule.execute();
    let attempt = store
        .begin(
            db.writer.clone(),
            &mut incomplete,
            &contract,
            budget.clone(),
        )
        .await
        .unwrap();
    assert!(
        attempt.checkpoint(&incomplete, &contract).await.is_err(),
        "an unproduced facts prefix cannot be admitted"
    );
    attempt.abort().await.unwrap();
}
