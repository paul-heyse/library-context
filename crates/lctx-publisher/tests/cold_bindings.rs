//! Synthetic captured suppliers exercise cold semantic admission, not provider qualification.
use cpg_core::workspace::{Workspace, WorkspaceOptions};
use lctx_model::domain::{
    ContentHash, Record, Relation,
    admission::{Availability, Frontier},
    analysis::sources::SourceSnapshot,
    attribution::{
        AnalysisContext, CoverageStatus, FactFamily, Provider, ProviderCoverage, ProviderRun,
    },
    completed::{CompletedBinding, CompletedView, ContributionSpec},
    input::InputRevision,
    producer_contract::{CapturedSourceBinding, ConfigurationBinding},
    source::CoverageScope,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{RuntimeConfig, compiler::NativeCompilerStore};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

// The expected supplier roles/families are deliberately literal. No executable declaration,
// current-provider-ID replacement map or production expected-coverage expansion builds them.
fn suppliers() -> Vec<(&'static str, &'static str, Provider, Vec<FactFamily>)> {
    use FactFamily::*;
    [
        (
            "acquire",
            "acquisition",
            "lctx-acquire",
            "0.1.0",
            vec![Artifacts],
        ),
        (
            "pyrefly",
            "pyrefly",
            "pyrefly",
            "63cda076956013cd0bd1d0d05c785f747fe6adc0;ruff=0.0.14",
            vec![Lexical, Exports, Signatures, Calls, Types],
        ),
        (
            "pyrefly",
            "pyrefly-ruff",
            "ruff",
            "9080ee7a82a4ec7359ba2ade60839ef1108a2968",
            vec![Syntax, Exports, Lexical],
        ),
        ("documents", "documents", "markdown-rs", "1.0.0", vec![Docs]),
        (
            "deployment",
            "deployment",
            "lctx-deployment",
            "0.1.0",
            vec![Deployment],
        ),
    ]
    .into_iter()
    .map(|(producer, role, tool, revision, families)| {
        (
            producer,
            role,
            Provider {
                tool: tool.into(),
                revision: revision.into(),
                build_digest: ContentHash::of(
                    format!("foreign captured {role} executable").as_bytes(),
                ),
            },
            families,
        )
    })
    .collect()
}

async fn write<R: Record>(native: &Arc<NativeCompilerStore>, pending: &ContentHash, rows: &[R]) {
    native
        .write_batch(pending, &Relation::of::<R>(), &R::encode(rows).unwrap())
        .await
        .unwrap();
}

async fn workspace(config: &RuntimeConfig) -> Arc<Workspace> {
    let native = NativeCompilerStore::begin(config, Frontier::Facts)
        .await
        .unwrap();
    Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions::default(),
        native,
    )
    .unwrap()
}

#[tokio::test]
async fn foreign_captured_bindings_admit_before_and_after_independent_native_transport() {
        let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"),
    ))
    .unwrap();
    let source = workspace(&config).await;
    let input = InputRevision::from_entries(vec![]).unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"cold captured source configuration"),
        environment_digest: ContentHash::of(b"cold captured environment"),
        lock_digest: None,
    };
    let scope = CoverageScope::Input { input: input.id() };
    let inventory = suppliers();
    assert_ne!(
        inventory[0].2.id(),
        cpg_extract::acquisition::acquisition_provider().id()
    );
    assert_ne!(
        inventory[1].2.id(),
        cpg_extract::pyrefly_stage::pyrefly_provider().id()
    );
    assert_ne!(
        inventory[2].2.id(),
        cpg_extract::ruff_context::provider().id()
    );
    let providers = inventory
        .iter()
        .map(|(_, _, provider, _)| provider.clone())
        .collect::<Vec<_>>();
    let mut runs = Vec::new();
    let mut families = Vec::new();
    for (_, _, provider, requested) in &inventory {
        let (run, members) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            requested.clone(),
        )
        .unwrap();
        runs.push(run);
        families.extend(members);
    }
    // These are the only input-grained obligations for an independently empty source
    // universe. Syntax/docs/etc have no scope; Flow is explicitly NotRequested in Catalog.
    let coverage = [
        (0, FactFamily::Artifacts),
        (1, FactFamily::Signatures),
        (4, FactFamily::Deployment),
    ]
    .map(|(supplier, family)| ProviderCoverage {
        scope: scope.id(),
        provider: Some(providers[supplier].id()),
        context: context.id(),
        family,
        run: Some(runs[supplier].id()),
        status: CoverageStatus::CompleteUnderStatedModel,
        reason: None,
        diagnostic: None,
    });
    let mut views = BTreeMap::<String, CompletedView>::new();
    for contract in cpg_extract::contracts::facts()
        .into_iter()
        .filter(|contract| contract.profiles.contains(&Profile::Catalog))
    {
        let settings = match contract.configuration {
            ConfigurationBinding::Fixed(value) => value,
            ConfigurationBinding::ProducerSettings => {
                ContentHash::of(b"foreign captured producer settings")
            }
            ConfigurationBinding::CapturedAnalysisContext => panic!("producer settings contract"),
        };
        let stage = contract.bind(
            ContentHash::of(
                format!("foreign captured {} execution composition", contract.name).as_bytes(),
            ),
            settings,
            inventory
                .iter()
                .filter(|(producer, _, _, _)| *producer == contract.name)
                .map(|(_, role, provider, _)| (*role, provider.clone())),
        );
        let mut captured = stage.captured_binding.unwrap();
        captured.sources = vec![CapturedSourceBinding {
            input: input.id(),
            context: context.id(),
            configuration: context.config_digest,
        }];
        let outputs = contract
            .outputs
            .iter()
            .chain(&contract.contributes)
            .map(|relation| source.model().relation(relation.name()).unwrap().clone())
            .collect::<Vec<_>>();
        let spec = ContributionSpec {
            captured_binding: Some(captured),
            producer: contract.name.into(),
            profile: Profile::Catalog,
            model: source.model().digest(),
            implementation: stage.code,
            configuration: Some(settings),
            inputs: contract
                .inputs
                .iter()
                .map(|relation| {
                    SourceSnapshot::of_completed_view(
                        source.model().relation(relation.name()).unwrap(),
                        source.model().digest(),
                        &views[relation.name()],
                    )
                    .unwrap()
                })
                .collect(),
            outputs: outputs
                .iter()
                .map(|relation| relation.name().into())
                .collect(),
        };
        let pending = source.native().begin_contribution(spec).await.unwrap();
        if contract.name == "acquire" {
            write(source.native(), &pending, std::slice::from_ref(&input)).await;
        }
        if contract.name == "assemble" {
            write(source.native(), &pending, &providers).await;
            write(source.native(), &pending, std::slice::from_ref(&context)).await;
            write(source.native(), &pending, std::slice::from_ref(&scope)).await;
            write(source.native(), &pending, &runs).await;
            write(source.native(), &pending, &families).await;
            write(source.native(), &pending, &coverage).await;
        }
        // Declared contributions include attribution beside semantic vocabulary. Assembly
        // retains every supplier's contribution to those exact shared completed views.
        let previous = outputs
            .iter()
            .filter_map(|relation| {
                views
                    .get(relation.name())
                    .map(|view| (relation.name().into(), view.clone()))
            })
            .collect();
        let completed = source
            .native()
            .complete_contribution(pending, ProviderOutcome::Complete, &outputs, &previous, &native_operation_budget)
            .await
            .unwrap();
        for relation in &outputs {
            let view = completed[relation.name()].clone();
            source
                .native()
                .bind(CompletedBinding {
                    boundary: None,
                    source: SourceSnapshot::of_completed_view(
                        relation,
                        source.model().digest(),
                        &view,
                    )
                    .unwrap(),
                    view: view.clone(),
                    configuration: None,
                })
                .await
                .unwrap();
            views.insert(relation.name().into(), view);
        }
    }
    source.restore(Profile::Catalog).await.unwrap();
    let admitted = source
        .facts_availability_async(Profile::Catalog)
        .await
        .unwrap();
    assert_eq!(
        admitted
            .evidence()
            .iter()
            .map(|row| (row.scope, row.family, row.provider))
            .collect::<BTreeSet<_>>(),
        coverage
            .iter()
            .map(|row| (scope.id(), row.family, row.provider))
            .collect()
    );
    assert!(
        admitted
            .evidence()
            .iter()
            .all(|row| row.availability == Availability::Complete)
    );
    assert_eq!(
        admitted.empty_universe(FactFamily::Flow),
        Some(Availability::NotRequested)
    );
    assert_eq!(
        admitted.empty_universe(FactFamily::Syntax),
        Some(Availability::NoScope)
    );
    let mut expected_contributions = source.native().contributions(&native_operation_budget).await.unwrap();
    expected_contributions.sort_by_key(|row| row.identity().unwrap());
    let mut expected_bindings = source.native().bindings().await.unwrap();
    expected_bindings.sort_by_key(CompletedBinding::key);
    let export = tempfile::NamedTempFile::new().unwrap();
    let state = source.native().export_state(export.path(), &native_operation_budget).await.unwrap();
    assert_eq!(state.format_version, 3);
    let obsolete = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        obsolete.path(),
        b"{\"format_version\":1}\nnot a reconstructible descriptor\n",
    )
    .unwrap();
    let refused = workspace(&config).await;
    let error = refused
        .native()
        .import_state(obsolete.path(), &state, &native_operation_budget)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported completed-state transport format"),
        "old header must refuse before malformed descriptor decoding: {error}"
    );
    assert!(error.permits_storage_cleanup());
    let cleanup = refused.native().abandon().await.unwrap_err();
    let lctx_model::domain::ModelError::Completion(outcome) = cleanup else {
        panic!("refused import finalization")
    };
    assert_eq!(
        outcome.completion.local,
        lctx_model::domain::completion::LocalState::Terminal
    );
    assert_eq!(
        outcome.completion.remote,
        lctx_model::domain::completion::RemoteState::Confirmed
    );
    assert!(
        outcome.completion.storage.is_empty(),
        "abandonment must not remove shared storage"
    );
    let inspector = lctx_surrealdb::compiler::check_installation(&config)
        .await
        .unwrap();
    let attempt = lctx_surrealdb::surrealdb::types::RecordId::new(
        "native_attempt",
        refused.native().attempt().hex(),
    );
    let mut response=inspector.query("SELECT VALUE state FROM $attempt; SELECT VALUE id FROM native_hold WHERE owner=$attempt").bind(("attempt",attempt)).await.unwrap().check().unwrap();
    let states: Vec<String> = response.take(0).unwrap();
    assert_eq!(states, ["abandoned"]);
    let holds: Vec<lctx_surrealdb::surrealdb::types::RecordId> = response.take(1).unwrap();
    assert!(
        holds.is_empty(),
        "refused attempt must release only its outgoing holds"
    );
    inspector.invalidate().await.unwrap();
    assert!(outcome.completion.failures.iter().any(|failure| {
        failure.step == "import_state"
            && failure
                .error
                .to_string()
                .contains("unsupported completed-state transport format")
    }));

    let restored = workspace(&config).await;
    assert_eq!(source.native().database(), restored.native().database());
    assert_ne!(source.native().attempt(), restored.native().attempt());
    // Shared immutable canonical payload survives attempt closure. Import independently
    // validates the transported descriptors/membership against those retained actual bytes;
    // it does not replay providers or mutate a separate database.
    restored
        .native()
        .import_state(export.path(), &state, &native_operation_budget)
        .await
        .unwrap();
    restored.restore(Profile::Catalog).await.unwrap();
    assert_eq!(
        *restored
            .facts_availability_async(Profile::Catalog)
            .await
            .unwrap(),
        *admitted
    );
    // Attempt-specific physical record order is not portable authority. Compare every
    // descriptor byte in logical identity order without deduplicating repeated rows.
    let mut actual_contributions = restored.native().contributions(&native_operation_budget).await.unwrap();
    actual_contributions.sort_by_key(|row| row.identity().unwrap());
    assert_eq!(actual_contributions, expected_contributions);
    let mut actual_bindings = restored.native().bindings().await.unwrap();
    actual_bindings.sort_by_key(CompletedBinding::key);
    assert_eq!(actual_bindings, expected_bindings);
    assert_eq!(restored.native().completed_state(&native_operation_budget).await.unwrap(), state);
    for name in [
        "input_revisions",
        "source_artifacts",
        "artifact_uses",
        "coverage_scopes",
        "provider_coverage",
        "providers",
        "provider_runs",
        "run_families",
        "analysis_contexts",
    ] {
        assert_eq!(
            restored.relation(name).unwrap().view(),
            source.relation(name).unwrap().view(),
            "exact admission premise {name}"
        );
    }
    source.drain().await.unwrap();
    restored.drain().await.unwrap();
    source.native().abandon().await.unwrap();
    restored.native().abandon().await.unwrap();
}
