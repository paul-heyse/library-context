//! The production frontier admits every exact coverage scope and never invents a ty run in catalog.
use cpg_core::workspace::{Workspace, WorkspaceOptions};
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{
    admission::*, attribution::*, resources::ResourceBudget, stages::Profile, *,
};
use std::sync::Arc;
async fn compile(
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    profile: Profile,
) -> Arc<Workspace> {
    let workspace = Workspace::with_budget(
        Arc::new(model().unwrap()),
        WorkspaceOptions {
            memory_bytes: budget.limit(),
            ..Default::default()
        },
        budget, crate::native_fixture::store()
)
    .unwrap();
    cpg_core::facts::compile_facts(
        &workspace,
        &captured,
        profile,
        cpg_core::facts::providers(ContentHash::of(b"fixture")),
        Default::default(),
    )
    .await
    .unwrap();
    workspace.validate().await.unwrap();
    workspace
}
fn availability(admission: &ScopedAvailability, family: FactFamily) -> Vec<Availability> {
    let mut values = admission
        .evidence()
        .iter()
        .filter(|row| row.family == family)
        .map(|row| row.availability)
        .collect::<Vec<_>>();
    if let Some(empty) = admission.empty_universe(family) {
        values.push(empty);
    }
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
    let workspace = compile(
        captured(b"def f(x): return x\n", false, Profile::Catalog, &resources),
        resources.clone(),
        Profile::Catalog,
    )
    .await;
    assert!(!rows::<Provider>(&workspace).iter().any(|p| p.tool == "ty"));
    assert!(
        !rows::<ProviderCoverage>(&workspace)
            .iter()
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

#[tokio::test]
async fn real_pyrefly_document_predecessor_uses_exact_completed_native_views() {
    for profile in Profile::ALL {
        let resources=budget();
        let capture=captured(b"def f(x):\n    \"\"\"Return x.\"\"\"\n    return x\n",true,profile,&resources);
        let workspace=Workspace::with_budget(Arc::new(model().unwrap()),WorkspaceOptions{memory_bytes:resources.limit(),batch_rows:17,..Default::default()},resources,crate::native_fixture::store()).unwrap();
        let providers=cpg_core::facts::providers(ContentHash::of(b"real-native-predecessor"))
            .into_iter().filter(|provider|[cpg_extract::acquisition::ACQUIRE,cpg_extract::pyrefly_stage::PYREFLY,cpg_extract::document_parser::DOCUMENTS,cpg_extract::assembly::ASSEMBLE].contains(&provider.declaration(profile).name)).collect();
        cpg_core::facts::compile_facts(&workspace,&capture,profile,providers,Default::default()).await.unwrap();
        assert!(!rows::<lctx_model::domain::documents::PassageObservation>(&workspace).is_empty());
        let completed=workspace.native().contributions().await.unwrap();
        let predecessor=completed.iter().find(|row|row.spec.producer==cpg_extract::pyrefly_stage::PYREFLY).unwrap();
        let documents=completed.iter().find(|row|row.spec.producer==cpg_extract::document_parser::DOCUMENTS).unwrap();
        let relation=lctx_model::domain::syntax::DeclarationObservation::NAME;
        let source=documents.spec.inputs.iter().find(|source|source.relation()==relation).unwrap();
        let view=workspace.completed::<lctx_model::domain::syntax::DeclarationObservation>().unwrap();
        assert_eq!(source.view(),view.view_identity());
        assert!(view.view().contributions.contains(&predecessor.identity().unwrap()));
        let native=workspace.native().completed_state().await.unwrap();
        assert_eq!(native.contributions,4);
        workspace.drain().await.unwrap();
    }
}

fn rows<R: Record>(workspace: &Workspace) -> Vec<R> {
    workspace
        .completed::<R>()
        .unwrap()
        .batches()
        .unwrap()
        .flat_map(|batch| R::decode(&batch.unwrap()).unwrap())
        .collect()
}

#[tokio::test]
async fn availability_reuses_exact_admission_and_refuses_changed_invalid_membership() {
    use input::{InputRevision,ArtifactUse,SourceRole};
    use source::{SourceArtifact,CoverageScope};
    let workspace=Workspace::new(Arc::new(model().unwrap()),WorkspaceOptions{memory_bytes:8<<20,..Default::default()},native_fixture::store()).unwrap();
    let profile=Profile::Catalog;
    let inputs=workspace.inputs("availability-empty",profile,[]).unwrap();
    let output=workspace.output("availability-empty",profile,ContentHash::of(b"availability-control"),inputs,[InputRevision::NAME,SourceArtifact::NAME,ArtifactUse::NAME,CoverageScope::NAME,ProviderCoverage::NAME,Provider::NAME,ProviderRun::NAME,RunFamily::NAME,AnalysisContext::NAME]);
    output.declare_async::<InputRevision>().await.unwrap();
    output.declare_async::<SourceArtifact>().await.unwrap();
    output.declare_async::<ArtifactUse>().await.unwrap();
    output.declare_async::<CoverageScope>().await.unwrap();
    output.declare_async::<ProviderCoverage>().await.unwrap();
    output.declare_async::<Provider>().await.unwrap();
    output.declare_async::<ProviderRun>().await.unwrap();
    output.declare_async::<RunFamily>().await.unwrap();
    output.declare_async::<AnalysisContext>().await.unwrap();
    output.finish(stages::ProviderOutcome::Complete).await.unwrap();
    let admitted=workspace.facts_availability_async(profile).await.unwrap();
    assert_eq!(admitted.empty_universe(FactFamily::Flow),Some(Availability::NotRequested));
    let synchronous=workspace.facts_availability(profile).unwrap();
    assert!(Arc::ptr_eq(&admitted,&synchronous),"sync and async consumers borrow the same exact admitted authority");
    let repeated=workspace.facts_availability_async(profile).await.unwrap();
    assert!(Arc::ptr_eq(&admitted,&repeated));
    let behavioral=workspace.facts_availability_async(Profile::Behavioral).await.unwrap();
    assert_eq!(behavioral.empty_universe(FactFamily::Flow),Some(Availability::NoScope));
    assert!(!Arc::ptr_eq(&admitted,&behavioral),"another profile must be independently admitted");
    let before=workspace.facts_availability_async(profile).await.unwrap();
    let nominal=serde_json::from_value(serde_json::to_value(vec![1u8;16]).unwrap()).unwrap();
    let inputs=workspace.inputs("availability-added-scope",profile,[]).unwrap();
    let output=workspace.output("availability-added-scope",profile,ContentHash::of(b"availability-control"),inputs,[CoverageScope::NAME]);
    output.declare_async::<CoverageScope>().await.unwrap();
    output.push(CoverageScope::Input{input:nominal}).await.unwrap();
    let error=output.finish(stages::ProviderOutcome::Complete).await.unwrap_err();
    assert!(error.to_string().contains("completed output coverage_scopes already has an owner"),"completed authority cannot be widened: {error}");
    let unchanged=workspace.facts_availability_async(profile).await.unwrap();
    assert!(Arc::ptr_eq(&before,&unchanged),"failed membership replacement must preserve the admitted authority");
    assert!(Arc::ptr_eq(&unchanged,&workspace.facts_availability(profile).unwrap()));
    // An invalid first completion is a separate attempt, never a replacement of an admitted view.
    let invalid=Workspace::new(Arc::new(model().unwrap()),WorkspaceOptions{memory_bytes:8<<20,..Default::default()},native_fixture::store()).unwrap();
    let inputs=invalid.inputs("availability-invalid-use",profile,[]).unwrap();
    let output=invalid.output("availability-invalid-use",profile,ContentHash::of(b"availability-control"),inputs,[InputRevision::NAME,SourceArtifact::NAME,ArtifactUse::NAME,CoverageScope::NAME,ProviderCoverage::NAME,Provider::NAME,ProviderRun::NAME,RunFamily::NAME,AnalysisContext::NAME]);
    output.declare_async::<InputRevision>().await.unwrap();
    output.declare_async::<SourceArtifact>().await.unwrap();
    output.declare_async::<ArtifactUse>().await.unwrap();
    output.declare_async::<CoverageScope>().await.unwrap();
    output.declare_async::<ProviderCoverage>().await.unwrap();
    output.declare_async::<Provider>().await.unwrap();
    output.declare_async::<ProviderRun>().await.unwrap();
    output.declare_async::<RunFamily>().await.unwrap();
    output.declare_async::<AnalysisContext>().await.unwrap();
    let artifact=serde_json::from_value(serde_json::to_value(vec![2u8;16]).unwrap()).unwrap();
    output.push(ArtifactUse{artifact,input:nominal,role:SourceRole::Release}).await.unwrap();
    output.finish(stages::ProviderOutcome::Complete).await.unwrap();
    for _ in 0..2 {
        let error=invalid.facts_availability_async(profile).await.unwrap_err();
        assert!(error.to_string().contains("artifact use names an absent artifact"),"invalid membership must be independently refused: {error}");
    }
    invalid.drain().await.unwrap();
    workspace.drain().await.unwrap();
}

#[path = "fixtures/native.rs"]
mod native_fixture;
