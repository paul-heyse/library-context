//! The complete real producer graph, admission and PostgreSQL generation lifecycle.
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{resources::ResourceBudget, stages::Profile, *};
use lctx_postgres::{
    generations::{GenerationCatalog, GenerationState, GenerationStore, ListFilter},
    testing::DisposableDatabase,
};
use std::sync::Arc;
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn captured(profile: Profile, resources: &ResourceBudget) -> Arc<CapturedInputs> {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("api.py"),
        b"def f(x):\n    return g(h(x))\n",
    )
    .unwrap();
    Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(root.path(), &["api.py".into()], &budget()).unwrap(),
        "facts-generation",
    )], cpg_extract::native_context::NativeContextConfig::committed(profile, resources).unwrap()))
}
#[tokio::test]
async fn real_facts_publication_equals_memory_and_never_selects() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model)
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    for profile in Profile::ALL {
        let configuration = ContentHash::of(b"fixture");
        let resources = budget();
        let memory = cpg_core::facts::memory(captured(profile, &resources), resources.clone(), profile, configuration)
            .await
            .unwrap();
        let published = cpg_core::facts::publish(
            &store,
            db.writer.clone(),
            captured(profile, &resources),
            resources.clone(),
            profile,
            configuration,
        )
        .await
        .unwrap();
        assert_eq!(published.content, memory.content());
        assert_eq!(published.availability, *memory.availability());
        let detail = catalog.show(published.generation).await.unwrap().unwrap();
        assert_eq!(detail.summary.state, GenerationState::Published);
        assert!(!detail.summary.selected);
        store.retire(published.generation).await.unwrap();
    }
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
}

struct FailingPyrefly(cpg_extract::pyrefly_stage::Pyrefly);
impl cpg_extract::bundle::Declared for FailingPyrefly {
    fn declaration(&self, profile: Profile) -> lctx_model::domain::stages::Stage {
        cpg_extract::bundle::Declared::declaration(&self.0, profile)
    }
}
impl cpg_extract::bundle::ProviderStage<lctx_postgres::generations::GenerationAttempt>
    for FailingPyrefly
{
    fn run(
        &mut self,
        _: &mut cpg_extract::bundle::StageContext<lctx_postgres::generations::GenerationAttempt>,
    ) -> Result<lctx_model::domain::stages::ProviderOutcome, ModelError> {
        Err(ModelError::Invalid(
            "injected required provider failure".into(),
        ))
    }
}
#[tokio::test]
async fn required_failure_aborts_and_incomplete_schedule_has_no_registry_side_effect() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let store = GenerationStore::install(db.owner.clone(), Arc::new(model().unwrap()))
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let mut offered = cpg_core::facts::providers(ContentHash::of(b"fault"));
    offered[1] = Box::new(FailingPyrefly(cpg_extract::pyrefly_stage::Pyrefly::new(
        cpg_extract::typed_syntax::SyntaxLimits::default(),
    )));
    let resources = budget();
    let result = cpg_core::facts::publish_declared(
        &store,
        db.writer.clone(),
        captured(Profile::Catalog, &resources),
        resources.clone(),
        Profile::Catalog,
        offered,
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("injected required provider failure")
    );
    assert_eq!(resources.reserved(), 0);
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    let mut offered = cpg_core::facts::providers(ContentHash::of(b"missing"));
    offered.remove(1);
    assert!(
        cpg_core::facts::publish_declared(
            &store,
            db.writer.clone(),
            captured(Profile::Catalog, &resources),
            resources.clone(),
            Profile::Catalog,
            offered
        )
        .await
        .is_err()
    );
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    let check = GenerationStore::check(&db.owner, store.model())
        .await
        .unwrap();
    assert!(check.clean(), "{check:#?}");
}
