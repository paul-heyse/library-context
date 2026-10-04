//! Actual C0/C1/C2 declaration producer qualification through disposable PG18.
#[path = "fixtures/catalog_runtime.rs"]
#[allow(
    dead_code,
    reason = "The shared module also supplies partial-model fixtures with relation inventory"
)]
mod catalog_runtime;
#[path = "fixtures/catalog_schedule.rs"]
mod catalog_schedule;
#[path = "fixtures/catalog_selection_controls.rs"]
mod catalog_selection_controls;
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::FrontierContract,
    catalog::build,
    normalized::{
        binding_normalization, callable_normalization, entity_normalization, event_normalization,
        relation_normalization,
    },
    stages::*,
    *,
};
use lctx_postgres::{
    generations::GenerationStore,
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;
#[test]
fn selection_fixture_model_preserves_completed_owner_dependencies() {
    let model = catalog_selection_controls::fixture_model();
    model.require::<selection::SelectionDomain>().unwrap();
    model
        .require::<execution::summary_exceptions::SummaryExceptionOutcome>()
        .unwrap();
    model
        .require::<execution::summary_capture::SummaryCaptureWitness>()
        .unwrap();
    assert!(model.require::<embedding::text::TextDefinition>().is_err());
    assert!(
        model
            .require::<embedding::analytic::AnalysisEmbeddingUse>()
            .is_err()
    );
}

#[tokio::test]
async fn finite_selection_domains_follow_actual_completed_catalog_owners() {
    run(Profile::Catalog).await;
}
#[tokio::test]
async fn behavioral_selection_keeps_runtime_field_witnesses_without_exact_state() {
    run(Profile::Behavioral).await;
}
async fn run(profile: Profile) {
    let runtime = AttemptRuntime::new(RuntimeOptions {
        memory_bytes: 1 << 30,
        partitions: 2,
    })
    .unwrap();
    let budget = runtime.budget();
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let config = RoleConfig {
        format: 1,
        role: Role::Importer,
        url: db.url("lctx_importer"),
        max_connections: 6,
        provider_connections: 4,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 60,
        lock_timeout_seconds: 10,
    };
    let model = Arc::new(catalog_selection_controls::fixture_model());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/catalog_context");
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture_derived(
                &root,
                &["api.py".into(), "guide.mdx".into()],
                budget,
                &["guide.mdx".into()],
                cpg_extract::acquisition::derive_blocks,
            )
            .unwrap(),
            "C0",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let catalog = captured.config().catalog().declaration().id();
    let (enriched_parameters, enriched_definition) =
        execution::configuration::enriched_execution(catalog);
    let (model_parameters, model_definition) = execution::configuration::models(catalog);
    let (summary_parameters, summary_definition) = execution::configuration::summaries(
        catalog,
        execution::configuration::SummaryLimits::default(),
    )
    .unwrap();
    let configuration = analysis::preparation::Configuration::new(
        captured.config().catalog(),
        catalog_runtime::definitions().into_iter().chain([
            (enriched_parameters, enriched_definition.clone()),
            (model_parameters, model_definition.clone()),
            (summary_parameters, summary_definition.clone()),
            build::definition(),
            catalog::evidence::build::definition(),
            selection::build::definition(),
        ]),
        budget,
    )
    .unwrap();
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"C0-native-fixture"));
    let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    declarations.extend([
        entity_normalization::stage(),
        relation_normalization::stage(profile),
        callable_normalization::stage(profile),
        normalized::receiver::stage(profile),
        event_normalization::stage(profile),
        binding_normalization::stage(profile),
        projection::normalization::stage(profile),
        normalized::coverage::stage(profile),
        configuration.declaration(),
        analysis::preparation::native_stage(profile, &model, &fixture_publication_order()).unwrap(),
        normalized::callable_aspects::stage(profile),
        build::stage(profile, &model, &fixture_publication_order()).unwrap(),
        catalog::evidence::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
        selection::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
    ]);
    declarations.extend(catalog_runtime::stages(profile, &model));
    declarations.extend([
        execution::enriched_production::stage(profile, &enriched_definition, &model, &fixture_publication_order()).unwrap(),
        execution::model_production::stage(profile, &model_definition, &model, &fixture_publication_order()).unwrap(),
        execution::summary_replay::stage(profile, &summary_definition, &model, &fixture_publication_order()).unwrap(),
    ]);
    let schedule = catalog_schedule::schedule(&model, declarations, profile);
    assert!(
        !schedule
            .stages()
            .iter()
            .any(|s| s.name.contains("synth") || s.name.contains("embed"))
    );
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let id = attempt.generation();
    for declaration in schedule.stages() {
        let installation:bool=sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton").bind(model.digest().0.to_vec()).bind(store.physical_digest().0.to_vec()).fetch_one(db.owner.pool()).await.unwrap();
        assert!(
            installation,
            "installation digest changed before {}",
            declaration.name
        );
        let normalization = match declaration.name {
            "normalize_entities" => Some(0),
            "normalize_relations" => Some(1),
            "normalize_callables" => Some(2),
            "normalize_events" => Some(3),
            "normalize_bindings" => Some(4),
            "normalize_projections" => Some(5),
            "normalize_coverage" => Some(6),
            "normalize_receivers" => Some(7),
            _ => None,
        };
        if let Some(which) = normalization {
            if which == 0 {
                attempt
                    .checkpoint(
                        &execution,
                        &FrontierContract::facts(&model, profile).unwrap(),
                    )
                    .await
                    .unwrap();
            }
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| match which {
                    0 => {
                        cpg_core::normalize::entities(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    1 => {
                        cpg_core::normalize::relations(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    2 => {
                        cpg_core::normalize::callables(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    3 => {
                        cpg_core::normalize::events(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    4 => {
                        cpg_core::normalize::bindings(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    5 => {
                        cpg_core::normalize::projections(
                            access, &attempt, &config, &runtime, &model,
                        )
                        .await
                    }
                    7 => {
                        cpg_core::normalize::receivers(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                    _ => {
                        cpg_core::normalize::coverage(access, &attempt, &config, &runtime, &model)
                            .await
                    }
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("stage {} failed: {e}", declaration.name));
        } else if declaration.name == "analysis_configuration" {
            cpg_core::analysis_prepare::configuration(
                execution.begin(declaration.name).unwrap(),
                &attempt,
                &model,
                &runtime,
                &configuration,
            )
            .await
            .unwrap();
        } else if declaration.name == "analysis_native_inventory" {
            cpg_core::analysis_prepare::native_inventory(
                execution.begin(declaration.name).unwrap(),
                &attempt,
                &config,
                &runtime,
                &model,
            )
            .await
            .unwrap_or_else(|e| panic!("stage {} failed: {e}", declaration.name));
        } else if declaration.name == "normalize_callable_aspects" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::catalog_core::aspects(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "catalog_core" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::catalog_core::produce(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("stage {} failed: {e}", declaration.name));
        } else if matches!(
            declaration.name,
            "analyze_local" | "evaluate_base" | "complete_base" | "prepare_source_calls"
        ) {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    catalog_runtime::run(
                        declaration.name,
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("parent {} failed: {e}", declaration.name));
        } else if matches!(
            declaration.name,
            "enrich_execution" | "apply_models" | "analyze_summaries"
        ) {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| match declaration.name {
                    "enrich_execution" => {
                        cpg_core::semantic_execution::enrich(
                            access,
                            &attempt,
                            &config,
                            &runtime,
                            &model,
                            &enriched_definition,
                        )
                        .await
                    }
                    "apply_models" => {
                        cpg_core::semantic_models::apply(
                            access,
                            &attempt,
                            &config,
                            &runtime,
                            &model,
                            &model_definition,
                        )
                        .await
                    }
                    _ => {
                        let graphs = cpg_core::analysis_graphs::PreparedGraphs::load(
                            &access,
                            &attempt,
                            &config,
                            &runtime,
                            &model,
                            &std::collections::BTreeSet::from([
                                projection::ProjectionName::CallableInvocation,
                            ]),
                        )
                        .await?;
                        cpg_core::semantic_summaries::produce(
                            access,
                            &attempt,
                            &config,
                            &runtime,
                            &model,
                            &summary_definition,
                            &graphs,
                        )
                        .await
                    }
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("parent {} failed: {e}", declaration.name));
        } else if declaration.name == "catalog_evidence" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::catalog_evidence::produce(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("stage {} failed: {e}", declaration.name));
        } else if declaration.name == "catalog_selection" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    catalog_selection_controls::replay_controls(
                        &access, &attempt, &config, &runtime, &model, profile,
                    )
                    .await?;
                    cpg_core::catalog_selection::produce(
                        access, &attempt, &config, &runtime, &model,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("stage {} failed: {e}", declaration.name));
        } else {
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
                budget,
                Default::default(),
            )
            .await
            .unwrap();
        }
    }
    let validated = attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap();
    let s = id.schema();
    let members: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.catalog_members"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    let domains: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.catalog_selection_domains"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(domains, members * 7);
    let links: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.catalog_selection_invocations"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(links, domains);
    let signatures: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.selection_contexts WHERE kind=2"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert!(signatures > 0);
    let missing_original:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_selection_domains d WHERE d.kind=5 AND NOT EXISTS (SELECT 1 FROM {s}.catalog_selection_domain_contexts c WHERE c.domain=d.id)"))).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(missing_original, 0);
    let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.selection_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows"))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(mismatches, 0);
    validated.abort().await.unwrap();
    drop(configuration);
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}

fn fixture_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    lctx_model::domain::stages::PublicationOrder::registered(
        lctx_model::domain::ContentHash::of(b"fixture publication order"),
        &[
            (0, lctx_model::domain::stages::PublicationBoundary::Facts),
            (1, lctx_model::domain::stages::PublicationBoundary::Local),
            (2, lctx_model::domain::stages::PublicationBoundary::Model),
            (3, lctx_model::domain::stages::PublicationBoundary::Summary),
            (
                4,
                lctx_model::domain::stages::PublicationBoundary::Structural,
            ),
            (5, lctx_model::domain::stages::PublicationBoundary::Analytic),
            (
                6,
                lctx_model::domain::stages::PublicationBoundary::Synthesis,
            ),
        ],
    )
    .unwrap()
}
