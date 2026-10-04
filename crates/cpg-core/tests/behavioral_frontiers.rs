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
    admission::FrontierContract, local_semantics, normalized::entity_normalization, stages::*, *,
};
use lctx_postgres::{
    generations::GenerationStore,
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;

#[tokio::test]
async fn behavioral_terminal_frontiers_retain_scoped_basis_and_unknown_twins() {
    let profile=Profile::Behavioral;
    let fixture="behavioral_frontiers";
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
    declarations.sort_by_key(Relation::name);
    declarations.dedup_by_key(|r| r.name());
    let model = Arc::new(ValidatedModel::validate(declarations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python").join(fixture);
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(&root, &["cases.py".into()], budget).unwrap(),
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
    // This probe mutates the distinct symbolic constructor/reader pairs of its own fixture.
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
    let frontiers:Vec<(String,i64,bool,bool,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT spelling.spelling,basis.count,f.effects_unknown,f.exceptions_unknown,f.question FROM {0}.conditional_terminal_frontiers f JOIN {0}.assertion_qualifications q ON q.id=f.qualification JOIN {0}.assumption_sets basis ON basis.id=q.assumptions JOIN {0}.entity_refs e ON e.id=f.owner JOIN {0}.callable_entities c ON c.id=e.callable_callable JOIN {0}.declaration_observations d ON d.declaration=c.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=d.name ORDER BY 1",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    eprintln!("frontiers={frontiers:?}");
    type TargetDiagnostic = (String,i16,Option<i16>,i16,i16,Option<i16>,Option<i16>,Option<i16>);
    let diagnostic:Vec<TargetDiagnostic>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT spelling.spelling,t.basis,t.reason,a.outcome,a.authority_reason,eff.identity,eff.descriptor,eff.signatures FROM {0}.closed_target_assessments t JOIN {0}.call_binding_attempts a ON a.id=t.attempt JOIN {0}.normalized_call_events event ON event.id=t.event JOIN {0}.occurrence_ownership o ON o.id=event.owner JOIN {0}.entity_refs e ON e.id=o.entity JOIN {0}.callable_entities c ON c.id=e.callable_callable JOIN {0}.declaration_observations d ON d.declaration=c.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=d.name LEFT JOIN {0}.effective_callable_assessments eff ON eff.id=a.effective ORDER BY 1,2,3",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    eprintln!("target_diagnostic={diagnostic:?}");
    let exits:Vec<(bool,i16,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT e.exceptional,e.characterization,count(*) FROM {0}.native_exit_characterizations e GROUP BY 1,2 ORDER BY 1,2",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    eprintln!("exits={exits:?}");
    let expected=std::collections::BTreeSet::from([("declared_use".to_owned(),1,true,true,0),("final_use".to_owned(),3,true,true,0),("final_method_use".to_owned(),3,true,true,0)]);
    assert_eq!(frontiers.into_iter().collect::<std::collections::BTreeSet<_>>(),expected);
    let contradictory_mro:Vec<i16>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT ancestry.linearization FROM {0}.class_ancestry_observations ancestry JOIN {0}.provider_symbols symbol ON symbol.id=ancestry.class WHERE symbol.name='MroGap' AND ancestry.relation=1",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    assert_eq!(contradictory_mro,vec![symbols::Linearization::Prefix as i16],"the MRO refusal uses actual contradictory-C3 recovery evidence");
    let conditional_bindings:Vec<(String,i16,i16,i16,bool,bool)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT spelling.spelling,a.outcome,a.authority,a.authority_reason,dispatch.open,bindings.unique FROM {0}.conditional_terminal_frontiers f JOIN {0}.closed_target_assessments t ON t.id=f.target JOIN {0}.call_binding_attempts a ON a.id=t.attempt JOIN {0}.normalized_dispatch_assessments dispatch ON dispatch.target=t.target AND dispatch.event=t.event JOIN {0}.binding_set_members member ON member.attempt=a.id JOIN {0}.binding_variant_assessments variant ON variant.id=member.variant JOIN {0}.binding_set_assessments bindings ON bindings.id=variant.set JOIN {0}.entity_refs e ON e.id=f.owner JOIN {0}.callable_entities c ON c.id=e.callable_callable JOIN {0}.declaration_observations d ON d.declaration=c.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=d.name WHERE t.basis=1 ORDER BY 1",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    assert_eq!(conditional_bindings,vec![("final_method_use".to_owned(),0,0,10,true,false),("final_use".to_owned(),0,0,10,true,false)],"named source binding never promotes open runtime dispatch or binding-set completeness");
    let routing_and_question:Vec<(i16,i16,i16,i16,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT original.modality,original.approximation,receiver.modality,question.modality,count(*) FROM {0}.closed_target_assessments t JOIN {0}.conditional_terminal_frontiers frontier ON frontier.target=t.id JOIN {0}.call_target_observations call ON call.id=t.target JOIN {0}.assertion_qualifications original ON original.id=t.original_qualification AND original.id=call.qualification JOIN {0}.type_observations observation ON observation.id=t.receiver_observation AND observation.role=8 JOIN {0}.assertion_qualifications receiver ON receiver.id=t.receiver_qualification AND receiver.id=observation.qualification JOIN {0}.assertion_qualifications question ON question.id=t.qualification WHERE t.basis=1 GROUP BY 1,2,3,4",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    assert_eq!(routing_and_question,vec![(1,0,0,0,2)],"original Overrides candidates retain their qualification while exact receiver evidence owns the conditional question");
    let phases:Vec<(i16,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT DISTINCT action,phase FROM {0}.protocol_action_assessments WHERE action IN (3,4,9) ORDER BY 1,2",id.schema())))
        .fetch_all(db.owner.pool()).await.unwrap();
    assert_eq!(phases,vec![(3,0),(4,0),(9,0)],"native iterator creation, next and operator actions preserve their call phase");
    let witnesses:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {0}.summary_terminal_witnesses w JOIN {0}.summary_claims c ON c.id=w.claim WHERE c.kind=3 AND w.question=0",id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(witnesses,3,"first Summary consumer preserves the three scoped claims");
    let runtime_negative:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {0}.conditional_terminal_frontiers f JOIN {0}.assertion_qualifications q ON q.id=f.qualification JOIN {0}.assumption_sets s ON s.id=q.assumptions WHERE s.count=0 OR NOT f.exceptions_unknown OR NOT f.effects_unknown",id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(runtime_negative,0,"typing frontier never becomes unconditional completion");
    let universe:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {0}.closed_target_assessments t JOIN {0}.assumption_universe_supports s ON s.id=t.universe_support JOIN {0}.authored_models m ON m.id=s.model JOIN {0}.model_catalogs c ON c.id=s.catalog WHERE t.basis=1",id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert!(universe>0,"final premise cites the actual late pinned definition");
    assert_eq!(exits,vec![(false,0,1),(false,1,2),(false,2,2),(true,0,1),(true,1,2),(true,2,2)],"declared Literal[True], Literal[False] and None characterize typing only; Any and async remain unknown");
    let suppress:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {0}.native_exit_characterizations WHERE runtime_completion_admitted",id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();assert_eq!(suppress,0);
    validated.abort().await.unwrap();
}
