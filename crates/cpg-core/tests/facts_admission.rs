//! The production frontier admits every exact coverage scope and never invents a ty run in catalog.
// The pinned compiler needs this compile-time depth for the finite native
// cursor future graph (rust-lang/rust#159228); keep Send checking intact.
#![recursion_limit = "256"]
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
        budget.clone(),
        crate::native_fixture::store_with_budget(budget),
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
async fn finish_workspace(workspace: &Workspace) -> Result<(), ModelError> {
    let mut completion = completion::Completion::default();
    completion.step(
        "facts control workspace execution drainage",
        workspace.drain().await,
    );
    // Drain owns admitted execution; abandonment separately acknowledges the
    // actual fixture attempt and its session before the test's owner disappears.
    completion.step(
        "facts control native attempt/session finalization",
        workspace.native().abandon().await,
    );
    completion::complete(Ok(()), completion)
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
    real_pyrefly_document_predecessor(Profile::ALL).await;
}

// Keep the smallest real completion/predecessor case independently selectable.
// Behavioral adds a different provider contract and is not needed to localize
// Catalog contribution completion.
#[tokio::test]
async fn real_catalog_pyrefly_document_predecessor_uses_exact_completed_native_views() {
    use tracing::instrument::WithSubscriber;
    real_pyrefly_document_predecessor([Profile::Catalog])
        .with_subscriber(cpg_extract::logging::dispatch())
        .await;
}

async fn real_pyrefly_document_predecessor(profiles: impl IntoIterator<Item = Profile>) {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    for profile in profiles {
        let resources = budget();
        let capture = captured(
            b"def f(x):\n    \"\"\"Return x.\"\"\"\n    return x\n",
            true,
            profile,
            &resources,
        );
        let workspace = Workspace::with_budget(
            Arc::new(model().unwrap()),
            WorkspaceOptions {
                memory_bytes: resources.limit(),
                batch_rows: 17,
                ..Default::default()
            },
            resources.clone(),
            crate::native_fixture::store_with_budget(resources),
        )
        .unwrap();
        let providers = cpg_core::facts::providers(ContentHash::of(b"real-native-predecessor"))
            .into_iter()
            .filter(|provider| {
                [
                    cpg_extract::acquisition::ACQUIRE,
                    cpg_extract::pyrefly_stage::PYREFLY,
                    cpg_extract::document_parser::DOCUMENTS,
                    cpg_extract::assembly::ASSEMBLE,
                ]
                .contains(&provider.declaration(profile).name)
            })
            .collect();
        cpg_core::facts::compile_facts(
            &workspace,
            &capture,
            profile,
            providers,
            Default::default(),
        )
        .await
        .unwrap();
        assert!(!rows::<lctx_model::domain::documents::PassageObservation>(&workspace).is_empty());
        let completed = workspace
            .native()
            .contributions(&native_operation_budget)
            .await
            .unwrap();
        let predecessor = completed
            .iter()
            .find(|row| row.spec.producer == cpg_extract::pyrefly_stage::PYREFLY)
            .unwrap();
        let documents = completed
            .iter()
            .find(|row| row.spec.producer == cpg_extract::document_parser::DOCUMENTS)
            .unwrap();
        let relation = lctx_model::domain::syntax::DeclarationObservation::NAME;
        let source = documents
            .spec
            .inputs
            .iter()
            .find(|source| source.relation() == relation)
            .unwrap();
        let view = workspace
            .completed::<lctx_model::domain::syntax::DeclarationObservation>()
            .unwrap();
        assert_eq!(source.view(), view.view_identity());
        assert!(
            view.view()
                .contributions
                .contains(&predecessor.identity().unwrap())
        );
        let native = workspace
            .native()
            .completed_state(&native_operation_budget)
            .await
            .unwrap();
        assert_eq!(native.contributions, 4);
        finish_workspace(&workspace).await.unwrap();
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
    use futures::FutureExt;
    use tracing::instrument::WithSubscriber;
    async {
        use input::{ArtifactUse, InputRevision, SourceRole};
        use source::{CoverageScope, SourceArtifact};
        use uniform_outcome::UniformNotRequested;
        type Attempt = (
            Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
            Option<Arc<Workspace>>,
        );
        fn owned_workspace(resources: ResourceBudget, attempts: &mut Vec<Attempt>) -> Arc<Workspace> {
            let native = native_fixture::store_with_budget(resources.clone());
            attempts.push((native.clone(), None));
            let workspace = Workspace::with_budget(
                Arc::new(model().unwrap()),
                WorkspaceOptions {
                    memory_bytes: resources.limit(),
                    ..Default::default()
                },
                resources,
                native,
            )
            .unwrap();
            attempts.last_mut().unwrap().1 = Some(workspace.clone());
            workspace
        }
        async fn compile_empty(
            profile: Profile,
            resources: &ResourceBudget,
            attempts: &mut Vec<Attempt>,
        ) -> Arc<Workspace> {
            let workspace = owned_workspace(resources.clone(), attempts);
            let captures = Arc::new(CapturedInputs::new(
                vec![],
                cpg_extract::native_context::NativeContextConfig::committed(profile, resources)
                    .unwrap(),
            ));
            cpg_core::facts::compile_facts(
                &workspace,
                &captures,
                profile,
                cpg_core::facts::providers(ContentHash::of(b"fixture")),
                Default::default(),
            )
            .await
            .unwrap();
            workspace.validate().await.unwrap();
            workspace
        }
        let mut attempts = Vec::new();
        let mut expected_refusals = Vec::new();
        let result = std::panic::AssertUnwindSafe(async {
        let profile = Profile::Catalog;
        // This checks semantic authority across two retained full provider profiles
        // using the existing full-compilation budget. It does not establish a
        // low-memory capacity contract; independent resource-refusal controls retain
        // their own bounds under operation-owned contribution preparation.
        let resources = budget();
        let workspace = compile_empty(profile, &resources, &mut attempts).await;
        let admitted = workspace.facts_availability_async(profile).await.unwrap();
        assert_eq!(
            admitted.empty_universe(FactFamily::Flow),
            Some(Availability::NotRequested)
        );
        let synchronous = workspace.facts_availability(profile).unwrap();
        assert!(
            Arc::ptr_eq(&admitted, &synchronous),
            "sync and async consumers borrow the same exact admitted authority"
        );
        let repeated = workspace.facts_availability_async(profile).await.unwrap();
        assert!(Arc::ptr_eq(&admitted, &repeated));
        let other = compile_empty(Profile::Behavioral, &resources, &mut attempts).await;
        let behavioral = other
            .facts_availability_async(Profile::Behavioral)
            .await
            .unwrap();
        assert_eq!(
            behavioral.empty_universe(FactFamily::Flow),
            Some(Availability::NoScope)
        );
        assert!(
            !Arc::ptr_eq(&admitted, &behavioral),
            "another profile must be independently admitted"
        );
        let before = workspace.facts_availability_async(profile).await.unwrap();
        let synchronous_before = workspace.facts_availability(profile).unwrap();
        assert!(Arc::ptr_eq(&before, &synchronous_before));
        let scopes_before = workspace.completed::<CoverageScope>().unwrap();
        let nominal = serde_json::from_value(serde_json::to_value(vec![1u8; 16]).unwrap()).unwrap();
        let inputs = workspace
            .inputs("availability-added-scope", profile, [])
            .unwrap();
        let output = workspace.output(
            "availability-added-scope",
            profile,
            ContentHash::of(b"availability-control"),
            inputs,
            [CoverageScope::NAME],
        );
        output.declare_async::<CoverageScope>().await.unwrap();
        output
            .push(CoverageScope::Input { input: nominal })
            .await
            .unwrap();
        let error = output
            .finish(stages::ProviderOutcome::Complete)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("completed output coverage_scopes already has an owner"),
            "completed authority cannot be widened: {error}"
        );
        expected_refusals.push(error.to_string());
        assert!(workspace.cancellation().check().is_err(),
            "failed required membership completion must cancel further compilation");
        let error = workspace.facts_availability_async(profile).await.unwrap_err();
        assert!(
            error.to_string().contains("compilation cancelled"),
            "a cancelled attempt must refuse new admission lookups: {error}"
        );
        expected_refusals.push(error.to_string());
        let error = workspace.facts_availability(profile).unwrap_err();
        assert!(error.to_string().contains("compilation cancelled"),
            "synchronous lookup must also refuse the cancelled attempt: {error}");
        expected_refusals.push(error.to_string());
        assert!(Arc::ptr_eq(
            &scopes_before, &workspace.completed::<CoverageScope>().unwrap()
        ), "failed membership replacement must retain the exact completed scope view");
        assert!(Arc::ptr_eq(&before, &admitted));
        assert_eq!(before.as_ref(), synchronous_before.as_ref(),
            "already borrowed immutable admission remains unchanged after refusal");
        assert_eq!(before.empty_universe(FactFamily::Flow), Some(Availability::NotRequested));

        // Profile misuse can refuse a native read and cancel its attempt. Exercise it
        // independently, rather than performing more production on a refused owner.
        let wrong_profile = compile_empty(profile, &resources, &mut attempts).await;
        let exact_profile = wrong_profile.facts_availability_async(profile).await.unwrap();
        let error = wrong_profile.facts_availability_async(Profile::Behavioral).await.unwrap_err();
        expected_refusals.push(error.to_string());
        assert_eq!(exact_profile.empty_universe(FactFamily::Flow),
            Some(Availability::NotRequested),
            "captured producer profile cannot be relabelled by a cache lookup");
        // An invalid first completion is a separate attempt, never a replacement of an admitted view.
        let invalid = owned_workspace(ResourceBudget::fixed(8 << 20).unwrap(), &mut attempts);
        let inputs = invalid
            .inputs("availability-invalid-use", profile, [])
            .unwrap();
        let output = invalid.output(
            "availability-invalid-use",
            profile,
            ContentHash::of(b"availability-control"),
            inputs,
            [
                InputRevision::NAME,
                SourceArtifact::NAME,
                ArtifactUse::NAME,
                CoverageScope::NAME,
                ProviderCoverage::NAME,
                Provider::NAME,
                ProviderRun::NAME,
                RunFamily::NAME,
                AnalysisContext::NAME,
                UniformNotRequested::NAME,
            ],
        );
        output.declare_async::<InputRevision>().await.unwrap();
        output.declare_async::<SourceArtifact>().await.unwrap();
        output.declare_async::<ArtifactUse>().await.unwrap();
        output.declare_async::<CoverageScope>().await.unwrap();
        output.declare_async::<ProviderCoverage>().await.unwrap();
        output.declare_async::<Provider>().await.unwrap();
        output.declare_async::<ProviderRun>().await.unwrap();
        output.declare_async::<RunFamily>().await.unwrap();
        output.declare_async::<AnalysisContext>().await.unwrap();
        output.declare_async::<UniformNotRequested>().await.unwrap();
        let artifact =
            serde_json::from_value(serde_json::to_value(vec![2u8; 16]).unwrap()).unwrap();
        output
            .push(ArtifactUse {
                artifact,
                input: nominal,
                role: SourceRole::Release,
            })
            .await
            .unwrap();
        output
            .finish(stages::ProviderOutcome::Complete)
            .await
            .unwrap();
        for _ in 0..2 {
            let error = invalid.facts_availability_async(profile).await.unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("missing completed declared producer"),
                "an undeclared contribution cannot supply captured facts authority: {error}"
            );
            expected_refusals.push(error.to_string());
        }
        }).catch_unwind().await;
        let mut finality = completion::Completion::default();
        let mut diagnostics = completion::Completion::default();
        for (native, workspace) in attempts.into_iter().rev() {
            let mut owned = match &workspace {
                Some(workspace) => workspace.drain_owned_report().await,
                None => completion::Completion::default(),
            };
            if owned.local == completion::LocalState::Terminal
                && owned.remote == completion::RemoteState::Confirmed
                && owned.failures.iter().all(|failure| failure.error.permits_storage_cleanup())
            {
                owned.merge(native.abandon_finality_report().await);
            } else {
                owned.storage.push(completion::StorageState::Orphan(format!(
                    "native_attempt:{}", native.attempt().hex()
                )));
            }
            if let Some(workspace) = &workspace {
                let signal = workspace.complete_signal(&owned).await;
                owned.step("availability control workspace signal", signal);
                workspace.append_operation_failures(&mut diagnostics);
            } else {
                native.append_operation_failures(&mut diagnostics);
            }
            finality.merge(owned);
        }
        let cleaned = completion::complete(Ok(()), finality);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("availability control finalization after assertion panic: {error:#?}");
            }
            if !diagnostics.failures.is_empty() {
                eprintln!("availability control operation diagnostics after assertion panic: {diagnostics:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
        assert!(diagnostics.failures.iter().all(|failure| {
            failure.error.permits_storage_cleanup()
                && expected_refusals.contains(&failure.error.to_string())
        }), "only the exact acknowledged control refusals may remain as diagnostics: {diagnostics:#?}");
    }
    .with_subscriber(cpg_extract::logging::dispatch())
    .await;
}

#[path = "fixtures/native.rs"]
mod native_fixture;
