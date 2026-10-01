//! Native facts and confirmed entity sources publish Local entry/stability through the real store.
#[path = "fixtures/local_model.rs"]
mod local_model;
#[path = "fixtures/summary_replay_probe.rs"]
mod replay_probe;
use cpg_core::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, RuntimeOptions},
};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    admission::FrontierContract,
    local_semantics,
    normalized::{entities::*, entity_normalization},
    source::*,
    stages::*,
    value::*,
    *,
};
use lctx_postgres::{
    generations::GenerationStore,
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;
async fn run(profile: Profile) {
    let (completion, source_calls, enriched) = (true, true, true);
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
    declarations.extend(transfer::model::relations());
    declarations.extend(analysis::model::relations());
    declarations.extend(execution::model_production::relations());
    declarations.extend(analysis::model::support::relations());
    declarations.extend(analysis::summary::relations());
    declarations.extend(transfer::summary::relations());
    declarations.extend(execution::summary_replay::relations());
    declarations.extend(execution::summary_replay::output_relations());
    declarations.extend(execution::summary_path::relations());
    declarations.extend(execution::summary_proof::relations());
    declarations.push(Relation::of::<replay_probe::ProbeReceipt>());
    declarations.sort_by_key(Relation::name);
    declarations.dedup_by_key(|r| r.name());
    let model = Arc::new(ValidatedModel::validate(declarations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/phase4_summaries");
    let bytes = std::fs::read(root.join("cases.py")).unwrap();
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(&root, &["cases.py".into()], budget).unwrap(),
            "entry",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let retained = budget.reserved();
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
    let (model_parameters, model_definition) =
        execution::configuration::models(captured.config().catalog().declaration().id());
    definitions.push((model_parameters, model_definition.clone()));
    let (summary_parameters, summary_definition) = execution::configuration::summaries(
        captured.config().catalog().declaration().id(),
        execution::configuration::SummaryLimits {
            depth: 2,
            ..Default::default()
        },
    )
    .unwrap();
    definitions.push((summary_parameters, summary_definition.clone()));
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

    stages.push(execution::model_production::stage(profile, &model_definition, &model).unwrap());
    stages.push(execution::summary_replay::stage(profile, &summary_definition, &model).unwrap());
    stages.push(replay_probe::stage(&model, profile));
    let facts_members = stages
        .iter()
        .filter(|s| {
            s.name != "analyze_local"
                && s.name != "apply_models"
                && s.name != "analyze_summaries"
                && s.outputs.iter().any(|r| is_vocabulary(r.name()))
        })
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
            PublicationGroup::new(PublicationBoundary::Model, vec!["apply_models"]),
            PublicationGroup::new(PublicationBoundary::Summary, vec!["analyze_summaries"]),
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
        } else if stage.name == "analyze_summaries" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
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
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "summary_replay_probe" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| replay_probe::run(access, &attempt, &config, &runtime, &model).await,
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if stage.name == "apply_models" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                stage,
                async |access| {
                    cpg_core::semantic_models::apply(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &model_definition,
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
    let runs: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.summary_runs",
        id.schema()
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert!(runs > 0);
    let proofs: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.summary_transfer_witnesses",
        id.schema()
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT status FROM {}.summary_analysis_outcomes",
        id.schema()
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    if profile == Profile::Behavioral {
        assert!(proofs > 0);
        assert!(statuses.iter().all(|s| *s == 1));
        let alternatives:Vec<(Vec<u8>,Vec<u8>,i64,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT constructor_qualification,reader_qualification,depth,reason FROM {}.summary_symbolic_field_alternatives",id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(alternatives.len(),4,"three readers retain the call argument and its separate returned-value qualification");
        assert!(alternatives.iter().all(|(_,_,d,reason)|*d==2&&*reason==obligation::ObligationKind::ScopeBoundary as i16));
        let original_qualifications:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.summary_symbolic_field_alternatives a JOIN {0}.flow_value_observations v ON v.id=a.value WHERE a.reader_qualification=v.qualification",id.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(original_qualifications,4,"each reader retains its exact native value qualification, including a legitimately unconditional argument");
        let matrix:Vec<(i16,i16,bool,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT kind,transfer,through_call,count(*) FROM {}.summary_symbolic_field_alternatives GROUP BY 1,2,3 ORDER BY 1,2,3",id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(matrix,vec![(1,0,false,1),(2,0,false,1),(2,1,false,1),(2,1,true,1)]);
        let readers:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(DISTINCT link) FROM {}.summary_symbolic_field_alternatives",id.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(readers,3);
        let conclusions:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.summary_behavioral_conclusions c JOIN {0}.summary_obligation_subjects s ON s.id=c.subject JOIN {0}.summary_claims q ON q.id=s.summaryclaim_transfer JOIN {0}.summary_symbolic_field_alternatives a ON a.id=q.symbolicfieldassociation_alternative WHERE c.verdict=3 AND c.proof IS NULL AND c.qualification=a.reader_qualification AND c.reason=a.reason",id.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(conclusions,4);
    } else {
        assert_eq!(proofs, 0);
        assert!(statuses.iter().all(|s| *s == 3));
    }
    validated.abort().await.unwrap();
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test]
async fn behavioral_summaries_publish_finite_proofs_and_validate() {
    run(Profile::Behavioral).await;
}
#[tokio::test]
async fn catalog_summaries_are_explicitly_not_requested() {
    run(Profile::Catalog).await;
}
