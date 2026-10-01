//! Native facts and confirmed entity sources publish Local entry/stability through the real store.
#[path = "fixtures/local_model.rs"]
mod local_model;
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::FrontierContract,
    local_semantics,
    normalized::entity_normalization,
    stages::*,
    *,
};
use lctx_postgres::{
    generations::GenerationStore,
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;
async fn run(profile: Profile, completion: bool) {
    run_case(profile, completion, false, false).await;
}
async fn run_case(profile: Profile, completion: bool, source_calls: bool, enriched: bool) {
    run_fixture(
        profile,
        completion,
        source_calls,
        enriched,
        "execution_channels",
        "cases.py",
    )
    .await;
}
async fn run_fixture(
    profile: Profile,
    completion: bool,
    source_calls: bool,
    enriched: bool,
    fixture: &str,
    file: &str,
) {
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
    let mut declarations = local_model::model().relations().to_vec();
    declarations.extend(analysis::base_evaluation::relations());
    declarations.extend(execution::records::relations());
    declarations.extend(execution::production::relations());
    if completion {
        declarations.extend(analysis::base_completion::relations());
        declarations.extend(execution::completion_records::relations());
        declarations.extend(execution::completion_production::relations());
        declarations.extend(execution::body_records::relations());
    }
    if source_calls {
        declarations.extend(analysis::source_call::relations());
        declarations.extend(execution::source_call_records::relations());
    }
    if enriched {
        declarations.extend(analysis::enriched_execution::relations());
        declarations.extend(execution::enriched_records::relations());
        declarations.extend(execution::modeled_call::relations());
        declarations.extend(execution::definition::relations());
        declarations.extend(execution::context_execution::relations());
        declarations.extend(execution::context_binding::relations());
        declarations.extend(execution::enriched_production::relations());
    }
    declarations.sort_by_key(Relation::name);
    declarations.dedup_by_key(|r| r.name());
    let model = Arc::new(ValidatedModel::validate(declarations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python")
        .join(fixture);
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(&root, &[file.into()], budget).unwrap(),
            "entry",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"native entry publication"));
    let mut stages: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    stages.extend([
        entity_normalization::stage(),
        normalized::relation_normalization::stage(profile),
        normalized::callable_normalization::stage(profile),
        normalized::callable_aspects::stage(profile),
        normalized::receiver::stage(profile),
        normalized::event_normalization::stage(profile),
        normalized::binding_normalization::stage(profile),
        projection::normalization::stage(profile),
        normalized::coverage::stage(profile),
    ]);
    stages.push(analysis::preparation::native_stage(profile));
    let (parameters, definition) = lctx_model::domain::local_semantics::definition();
    let (base_parameters, base_definition) = execution::configuration::base_evaluation();
    let mut definitions = vec![
        (parameters, definition.clone()),
        (base_parameters, base_definition.clone()),
    ];
    let (completion_parameters, completion_definition) =
        execution::configuration::base_completion();
    if completion {
        definitions.push((completion_parameters, completion_definition.clone()));
    }
    let (source_parameters, source_definition) = execution::configuration::source_calls();
    if source_calls {
        definitions.push((source_parameters, source_definition.clone()));
    }
    let (enriched_parameters, enriched_definition) = execution::configuration::enriched_execution(
        captured.config().catalog().declaration().id(),
    );
    if enriched {
        definitions.push((enriched_parameters, enriched_definition.clone()));
    }
    let configuration =
        analysis::preparation::Configuration::new(captured.config().catalog(), definitions, budget)
            .unwrap();
    stages.push(configuration.declaration());
    stages.push(local_semantics::stage(profile, &definition, &model));
    stages.push(execution::production::stage(profile, &base_definition, &model).unwrap());
    if completion {
        stages.push(
            execution::completion_production::stage(profile, &completion_definition, &model)
                .unwrap(),
        );
    }
    if source_calls {
        stages.push(execution::source_call::stage(profile, &source_definition, &model).unwrap());
    }
    if enriched {
        stages.push(
            execution::enriched_production::stage(profile, &enriched_definition, &model).unwrap(),
        );
    }

    let facts_members = stages
        .iter()
        .filter(|s| s.name != "analyze_local" && s.outputs.iter().any(|r| is_vocabulary(r.name())))
        .map(|s| s.name)
        .collect();
    let schedule = Schedule::build_with_publications(
        &model,
        stages,
        &[],
        profile,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts_members),
            PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
        ],
    )
    .unwrap();
    let facts = FrontierContract::facts(&model, profile).unwrap();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let id = attempt.generation();
    for stage in schedule.stages() {
        if stage.name == "normalize_entities" {
            attempt.checkpoint(&execution, &facts).await.unwrap();
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::entities(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_relations" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::relations(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_callables" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::callables(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_callable_aspects" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::catalog_core::aspects(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_receivers" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::receivers(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_events" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::events(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_bindings" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::bindings(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_projections" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::projections(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "normalize_coverage" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::normalize::coverage(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "analysis_native_inventory" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::analysis_prepare::native_inventory(
                        access, &attempt, &config, &runtime, &model,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "analysis_configuration" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::analysis_prepare::configuration(
                        access,
                        &attempt,
                        &model,
                        &runtime,
                        &configuration,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "prepare_source_calls" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::semantic_execution::prepare_source_calls(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &source_definition,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "enrich_execution" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::semantic_execution::enrich(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &enriched_definition,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "complete_base" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::semantic_execution::complete_base(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &completion_definition,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "evaluate_base" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::semantic_execution::evaluate_base(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &base_definition,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "analyze_local" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::local_semantics::run(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &definition,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else {
            let index = providers
                .iter()
                .position(|p| p.declaration(profile).name == stage.name)
                .unwrap();
            run_stage(
                providers.swap_remove(index),
                execution.begin(stage.name).unwrap(),
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
    drop(configuration);
    let validated = attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap();
    let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.base_expression_evaluations),(SELECT count(*) FROM {}.base_evaluation_boundaries),(SELECT count(*) FROM {}.base_evaluation_runs)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
    assert!(counts.2 > 0);
    let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT status FROM {}.base_evaluation_analysis_outcomes",
        id.schema()
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    assert!(!statuses.is_empty());
    if profile == Profile::Behavioral {
        assert!(counts.0 > 0 && counts.1 > 0);
        assert!(statuses.iter().all(|s| *s == 1));
        let entries: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {}.base_evaluation_sources WHERE kind=1",
            id.schema()
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(entries > 0);
    } else {
        assert_eq!((counts.0, counts.1), (0, 0));
        assert!(statuses.iter().all(|s| *s == 3));
    }
    let read_counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.base_read_observations),(SELECT count(*) FROM {}.base_formal_read_assessments),(SELECT count(*) FROM {}.base_attribute_reads)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
    if profile == Profile::Behavioral {
        assert!(read_counts.0 > 0 && read_counts.1 > 0 && read_counts.2 > 0);
    } else {
        assert_eq!(read_counts, (0, 0, 0));
    }
    if fixture == "transfer_alternatives" && profile == Profile::Behavioral {
        let native_negative: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {}.base_formal_read_assessments WHERE status=1",
            id.schema()
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(native_negative > 0);
        let dynamic:(i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT count(*),count(*) FILTER (WHERE inspection=0) FROM {}.base_dynamic_access_observations",id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(dynamic.0 > 0 && dynamic.1 > 0);
    }
    if fixture == "field_read_screen" {
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.base_field_read_assessments WHERE status=1),(SELECT count(*) FROM {}.base_field_read_assessments WHERE status=0),(SELECT count(*) FROM {}.base_global_field_read_assessments WHERE status=1)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        if profile == Profile::Behavioral {
            assert!(counts.0 >= 2 && counts.1 > 0 && counts.2 >= 2);
        } else {
            assert_eq!(counts, (0, 0, 0));
        }
    }
    if completion {
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.base_statement_completions),(SELECT count(*) FROM {}.base_completion_boundaries),(SELECT count(*) FROM {}.base_completion_runs)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(counts.2 > 0);
        let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT status FROM {}.base_completion_analysis_outcomes",
            id.schema()
        )))
        .fetch_all(db.owner.pool())
        .await
        .unwrap();
        if profile == Profile::Behavioral {
            let body_counts:(i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.base_source_body_completions),(SELECT count(*) FROM {}.base_source_body_boundaries)",id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
            assert!(body_counts.0 > 0 && body_counts.1 > 0);
            assert!(counts.0 > 0 && counts.1 > 0);
            assert!(statuses.iter().all(|s| *s == 1));
            let sources: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.base_completion_sources WHERE kind=1",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(sources > 0);
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
            assert!(statuses.iter().all(|s| *s == 3));
        }
    }
    if source_calls {
        let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT status FROM {}.source_call_analysis_outcomes",
            id.schema()
        )))
        .fetch_all(db.owner.pool())
        .await
        .unwrap();
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .all(|s| *s == if profile == Profile::Behavioral { 1 } else { 3 })
        );
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.source_call_headers),(SELECT count(*) FROM {}.source_call_boundaries),(SELECT count(*) FROM {}.source_call_runs)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(counts.2 > 0);
        if profile == Profile::Behavioral {
            assert!(counts.0 > 0 && counts.1 > 0);
            let invoked: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.source_call_invocations",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(invoked > 0);
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
        }
    }
    if enriched {
        let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT status FROM {}.enriched_execution_analysis_outcomes",
            id.schema()
        )))
        .fetch_all(db.owner.pool())
        .await
        .unwrap();
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .all(|s| *s == if profile == Profile::Behavioral { 1 } else { 3 })
        );
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.statement_executions),(SELECT count(*) FROM {}.execution_sources WHERE kind=2),(SELECT count(*) FROM {}.execution_runs)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(counts.2 > 0);
        if profile == Profile::Behavioral {
            assert!(counts.0 > 0 && counts.1 > 0);
            let modeled: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.modeled_call_evaluations",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(modeled > 0);
            let fresh: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.source_execution_invocations",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(fresh > 0);
            let args: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.source_frame_arguments",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(args > 0);
            let defaults: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.definition_evaluations",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(defaults > 0);
            let contexts: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.context_executions",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(contexts > 0);
            let suppressed: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.context_execution_items WHERE suppressed",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(suppressed > 0);
            let bindings: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.context_entry_bindings",
                id.schema()
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(bindings > 0);
            let retained_bindings:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.context_entry_bindings WHERE release=1 AND entry_actual IS NOT NULL",id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
            assert!(retained_bindings > 0);
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
        }
    }
    validated.abort().await.unwrap();
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test]
async fn behavioral_base_evaluation_uses_closed_native_local_sources() {
    run(Profile::Behavioral, false).await;
}
#[tokio::test]
async fn catalog_base_evaluation_is_explicitly_not_requested() {
    run(Profile::Catalog, false).await;
}

#[tokio::test]
async fn behavioral_completion_consumes_actual_closed_base_evaluations() {
    run(Profile::Behavioral, true).await;
}
#[tokio::test]
async fn catalog_completion_is_explicitly_not_requested() {
    run(Profile::Catalog, true).await;
}

#[tokio::test]
async fn behavioral_source_calls_publish_only_replayed_fresh_bindings() {
    run_case(Profile::Behavioral, true, true, false).await;
}
#[tokio::test]
async fn catalog_source_calls_are_explicitly_not_requested() {
    run_case(Profile::Catalog, true, true, false).await;
}

#[tokio::test]
async fn behavioral_enriched_replays_source_outcomes_in_caller_frame() {
    run_case(Profile::Behavioral, true, true, true).await;
}
#[tokio::test]
async fn catalog_enriched_is_explicitly_not_requested() {
    run_case(Profile::Catalog, true, true, true).await;
}

#[tokio::test]
async fn behavioral_read_channels_preserve_native_observation_and_negative_inventory() {
    run_fixture(
        Profile::Behavioral,
        false,
        false,
        false,
        "transfer_alternatives",
        "transferpkg/__init__.py",
    )
    .await;
}
#[tokio::test]
async fn catalog_read_channels_are_not_requested() {
    run_fixture(
        Profile::Catalog,
        false,
        false,
        false,
        "transfer_alternatives",
        "transferpkg/__init__.py",
    )
    .await;
}

#[tokio::test]
async fn behavioral_field_read_screen_preserves_source_model_negatives() {
    run_fixture(
        Profile::Behavioral,
        false,
        false,
        false,
        "field_read_screen",
        "cases.py",
    )
    .await;
}
#[tokio::test]
async fn catalog_field_read_screen_is_not_requested() {
    run_fixture(
        Profile::Catalog,
        false,
        false,
        false,
        "field_read_screen",
        "cases.py",
    )
    .await;
}
