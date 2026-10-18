//! The ordinary private chain must work without any installed native attempt.
use cpg_core::{
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use futures::FutureExt;
use lctx_model::domain::{ContentHash, admission::Frontier, stages::Profile};
use std::sync::Arc;

#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

async fn run(profile: Profile, frontier: Frontier, materialize: bool) {
    let workspace = Workspace::private(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions::default(),
    )
    .unwrap();
    let mut native_owner = None;
    let outcome = std::panic::AssertUnwindSafe(async {
        let captured = catalog_runtime::capture("native_signature", profile, workspace.budget());
        let settings = ContentHash::of(b"private-compilation-control");
        let prepared = matches!(frontier, Frontier::Analysis | Frontier::Catalog).then(|| {
            PreparedCompilation::new(
                frontier,
                catalog_runtime::settings("native_signature"),
                captured.config().catalog(),
                None,
                workspace.budget(),
            )
            .map(Arc::new)
            .unwrap()
        });
        compilation::compile(
            &workspace,
            captured.clone(),
            profile,
            settings,
            frontier,
            prepared,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(workspace.is_private());
        assert!(
            workspace.native().is_err(),
            "private compilation creates no native target"
        );
        assert!(!workspace.completed_relations().unwrap().is_empty());
        let originals = workspace
            .completed::<lctx_model::domain::source::SourceArtifact>()
            .unwrap();
        assert_eq!(
            originals.rows(),
            1,
            "genuine captured source remains part of the compile"
        );
        workspace.admit_semantics(profile).await.unwrap();
        if materialize {
            let native = native_fixture::store_with_budget(workspace.budget().clone());
            native_owner = Some(native.clone());
            native.set_frontier(frontier).unwrap();
            let artifact = cpg_core::artifact::materialize_and_admit(
                &workspace,
                &captured,
                frontier,
                profile,
                settings,
                native.clone(),
            )
            .await
            .unwrap();
            assert_eq!(artifact.manifest().frontier, frontier);
            assert_eq!(artifact.manifest().profile, profile);
            assert!(Arc::ptr_eq(artifact.native(), &native));
            assert!(
                workspace.native().is_err(),
                "materialization does not relabel private authority"
            );
            let mut finality = artifact.drain_owned_report().await;
            artifact.append_operation_failures(&mut finality);
            lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
            drop(artifact);
        }
    })
    .catch_unwind()
    .await;
    if let Some(native) = native_owner {
        catalog_runtime::complete_workspace(Some(workspace), native, outcome).await;
    } else {
        let mut finality = workspace.drain_owned_report().await;
        let signal = workspace.complete_signal(&finality).await;
        finality.step("private compilation control signal", signal);
        workspace.append_operation_failures(&mut finality);
        if let Err(panic) = outcome {
            if let Err(error) = lctx_model::domain::completion::complete(Ok(()), finality) {
                eprintln!("private control cleanup: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
    }
}

macro_rules! controls { ($($name:ident: $profile:ident, $frontier:ident, $native:literal;)*) => {$ (
    #[tokio::test(flavor = "multi_thread")]
    async fn $name() { run(Profile::$profile, Frontier::$frontier, $native).await; }
)*}; }
controls! {
    private_catalog_facts: Catalog, Facts, false;
    private_catalog_normalized: Catalog, Normalized, false;
    private_catalog_analysis: Catalog, Analysis, false;
    private_catalog_catalog: Catalog, Catalog, false;
    private_behavioral_facts: Behavioral, Facts, false;
    private_behavioral_normalized: Behavioral, Normalized, false;
    private_behavioral_analysis: Behavioral, Analysis, false;
    private_behavioral_catalog: Behavioral, Catalog, false;
    native_catalog_facts: Catalog, Facts, true;
    native_catalog_normalized: Catalog, Normalized, true;
    native_catalog_analysis: Catalog, Analysis, true;
    native_catalog_catalog: Catalog, Catalog, true;
    native_behavioral_facts: Behavioral, Facts, true;
    native_behavioral_normalized: Behavioral, Normalized, true;
    native_behavioral_analysis: Behavioral, Analysis, true;
    native_behavioral_catalog: Behavioral, Catalog, true;
}
