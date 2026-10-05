//! The production frontier admits every exact coverage scope and never invents a ty run in catalog.
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{
    admission::*, attribution::*, resources::ResourceBudget, stages::Profile, *,
};
use std::sync::Arc;
use cpg_core::workspace::{Workspace, WorkspaceOptions};
async fn compile(captured: Arc<CapturedInputs>, budget: ResourceBudget, profile: Profile) -> Arc<Workspace> {
    let workspace = Workspace::with_budget(Arc::new(model().unwrap()), WorkspaceOptions { memory_bytes: budget.limit(), ..Default::default() }, budget).unwrap();
    cpg_core::facts::compile_facts(&workspace, &captured, profile, cpg_core::facts::providers(ContentHash::of(b"fixture")), Default::default()).await.unwrap();
    workspace.validate().await.unwrap();
    workspace
}
fn availability(admission: &ScopedAvailability, family: FactFamily) -> Vec<Availability> {
    let mut values = admission.evidence().iter().filter(|row| row.family == family).map(|row| row.availability).collect::<Vec<_>>();
    if let Some(empty) = admission.empty_universe(family) {values.push(empty);}
    values.dedup();
    values
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn captured(
    source: &[u8],
    document: bool,
    profile: Profile,
    resources: &ResourceBudget,
) -> Arc<CapturedInputs> {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("api.py"), source).unwrap();
    let mut paths = vec!["api.py".into()];
    if document {
        std::fs::write(root.path().join("README.md"), b"# API\n\nRead `api.f`.\n").unwrap();
        paths.push("README.md".into());
    }
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(root.path(), &paths, resources).unwrap(),
            "facts-admission",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, resources).unwrap(),
    ))
}
#[tokio::test]
async fn complete_frontier_is_admitted_with_profile_owned_availability() {
    for profile in Profile::ALL {
        let resources = budget();
        let workspace = compile(
            captured(b"def f(x):\n    return x\n", true, profile, &resources),
            resources.clone(),
            profile,
        )
        .await;
        let admission = workspace.facts_availability(profile).unwrap();
        assert_eq!(
            availability(&admission, FactFamily::Artifacts),
            vec![Availability::Complete]
        );
        assert_eq!(
            availability(&admission, FactFamily::Docs),
            vec![Availability::Complete]
        );
        assert_eq!(
            availability(&admission, FactFamily::Flow),
            if profile == Profile::Catalog {
                vec![Availability::NotRequested]
            } else {
                vec![Availability::Complete]
            }
        );
    }
    let resources = budget();
    let workspace = compile(captured(b"def f(x): return x\n", false, Profile::Catalog, &resources), resources.clone(), Profile::Catalog).await;
    assert!(
        !rows::<Provider>(&workspace).iter()
            .any(|p| p.tool == "ty")
    );
    assert!(
        !rows::<ProviderCoverage>(&workspace).iter()
            .filter(|c| c.family == FactFamily::Flow)
            .any(|c| c.provider.is_some() || c.run.is_some())
    );
}
#[tokio::test]
async fn syntax_failure_and_no_document_scope_have_distinct_availability() {
    let resources = budget();
    let workspace = compile(
        captured(b"def f(:\n", false, Profile::Catalog, &resources),
        resources.clone(),
        Profile::Catalog,
    )
    .await;
    let admission = workspace.facts_availability(Profile::Catalog).unwrap();
    assert_eq!(
        availability(&admission, FactFamily::Syntax),
        vec![Availability::Partial]
    );
    assert_eq!(
        availability(&admission, FactFamily::Docs),
        vec![Availability::NoScope]
    );
}

fn rows<R:Record>(workspace:&Workspace)->Vec<R>{workspace.completed::<R>().unwrap().batches().unwrap().flat_map(|batch|R::decode(&batch.unwrap()).unwrap()).collect()}
