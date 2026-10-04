//! Native facts and confirmed entity sources publish Local entry/stability through the real store.
#[path = "fixtures/local_model.rs"]
mod local_model;
#[path = "fixtures/summary_replay_probe.rs"]
mod replay_probe;
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
async fn run(profile: Profile) {
    run_fixture(profile, "phase4_summaries").await;
}
async fn run_fixture(profile: Profile, fixture: &str) {
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
    if fixture == "phase4_summaries" {
        declarations.push(Relation::of::<replay_probe::ProbeReceipt>());
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
    if fixture == "phase4_summaries" {
        stages.push(replay_probe::stage(&model, profile));
    }
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
        assert!(statuses.iter().all(|s| *s == 1));
    }
    if profile == Profile::Behavioral && fixture == "phase4_summaries" {
        assert_conditional_atom_summary(db.owner.pool(), &id.schema(), &root).await;
        assert!(proofs > 0);
        assert!(statuses.iter().all(|s| *s == 1));
        let alternatives:Vec<(Vec<u8>,Vec<u8>,i64,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT constructor_qualification,reader_qualification,depth,reason FROM {}.summary_symbolic_field_alternatives",id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(
            alternatives.len(),
            4,
            "three readers retain the call argument and its separate returned-value qualification"
        );
        assert!(alternatives.iter().all(|(_, _, d, reason)| *d == 2
            && *reason == obligation::ObligationKind::ScopeBoundary as i16));
        let original_qualifications:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.summary_symbolic_field_alternatives a JOIN {0}.flow_value_observations v ON v.id=a.value WHERE a.reader_qualification=v.qualification",id.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(
            original_qualifications, 4,
            "each reader retains its exact native value qualification, including a legitimately unconditional argument"
        );
        let matrix:Vec<(i16,i16,bool,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT kind,transfer,through_call,count(*) FROM {}.summary_symbolic_field_alternatives GROUP BY 1,2,3 ORDER BY 1,2,3",id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(
            matrix,
            vec![
                (1, 0, false, 1),
                (2, 0, false, 1),
                (2, 1, false, 1),
                (2, 1, true, 1)
            ]
        );
        let readers: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(DISTINCT link) FROM {}.summary_symbolic_field_alternatives",
            id.schema()
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert_eq!(readers, 3);
        let conclusions:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.summary_behavioral_conclusions c JOIN {0}.summary_obligation_subjects s ON s.id=c.subject JOIN {0}.summary_claims q ON q.id=s.summaryclaim_transfer JOIN {0}.summary_symbolic_field_alternatives a ON a.id=q.symbolicfieldassociation_alternative WHERE c.verdict=3 AND c.proof IS NULL AND c.qualification=a.reader_qualification AND c.reason=a.reason",id.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(conclusions, 4);
    } else if profile == Profile::Catalog {
        assert_eq!(proofs, 0);
        assert!(statuses.iter().all(|s| *s == 3));
    }
    if fixture == "exact_exception_shapes" {
        let results: Vec<(String, Option<i16>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT spelling.spelling,result.exception FROM {0}.summary_exception_outcomes result JOIN {0}.entity_refs e ON e.id=result.owner JOIN {0}.callable_entities c ON c.id=e.callable_callable JOIN {0}.declaration_observations d ON d.declaration=c.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=d.name ORDER BY 1", id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        eprintln!("B4_SUMMARY {results:?}");
        for (name, exception) in [
            ("bare_class", Some(1)),
            ("broad_first", None),
            ("tuple_match", None),
            ("unmatched", Some(1)),
            ("final_return", None),
            ("final_raise", Some(2)),
            ("reraised", Some(1)),
            ("named_disposal", None),
            ("argument_order", Some(1)),
        ] {
            assert!(
                results
                    .iter()
                    .any(|row| row == &(name.to_owned(), exception)),
                "missing actual finite Summary {name}: {results:?}"
            );
        }
        type StoredException = (
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Option<i16>,
            i16,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            i16,
            i16,
        );
        let stored: Vec<StoredException> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT s.id,s.invocation,s.body,s.input,s.context,s.owner,s.qualification,s.outcome,s.exception,s.status,q.assumptions,q.scope,q.condition,q.modality,q.approximation FROM {0}.summary_exception_outcomes s JOIN {0}.assertion_qualifications q ON q.id=s.qualification LIMIT 32", id.schema())))
            .fetch_all(db.owner.pool()).await.unwrap();
        assert!(!stored.is_empty() && stored.len() < 32);
        fn nominal<T>(bytes: Vec<u8>) -> Id<T> {
            serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new(bytes.into_iter()))
            .unwrap()
        }
        for (
            record,
            invocation,
            body,
            input,
            context,
            owner,
            qualification,
            outcome,
            exception,
            status,
            assumptions,
            scope,
            condition,
            modality,
            approximation,
        ) in stored
        {
            let result = execution::summary_exceptions::SummaryExceptionOutcome {
                invocation: nominal(invocation),
                body: nominal(body),
                input: nominal(input),
                context: nominal(context),
                owner: nominal(owner),
                qualification: nominal(qualification),
                outcome: nominal(outcome),
                exception: exception
                    .map(|value| serde_json::from_value(serde_json::json!(value)).unwrap()),
                status: serde_json::from_value(serde_json::json!(status)).unwrap(),
            };
            let q = assertion::AssertionQualification {
                context: result.context,
                assumptions: nominal(assumptions),
                scope: nominal(scope),
                condition: nominal(condition),
                modality: serde_json::from_value(serde_json::json!(modality)).unwrap(),
                approximation: serde_json::from_value(serde_json::json!(approximation)).unwrap(),
            };
            assert_eq!(result.id().bytes().as_slice(), record);
            assert_eq!(result.qualification, q.id());
            let packet = serving::BehavioralExceptionPacket::from_canonical(&result, &q).unwrap();
            assert_eq!(packet.exception.0, result.exception);
            assert!(packet.under_body_entry && packet.claim_basis.definitions.is_empty());
            assert_eq!(
                packet.claim_basis.set,
                assumptions::AssumptionSet::empty_id()
            );
            assert_eq!(packet.proof.len(), 2);
            let mut missing = serde_json::to_value(&packet).unwrap();
            missing.as_object_mut().unwrap().remove("claim_basis");
            assert!(serde_json::from_value::<serving::BehavioralExceptionPacket>(missing).is_err());
        }
        for name in [
            "dynamic_constructor",
            "argument_opaque",
            "unsupported_group",
            "shadowed_constructor",
            "invalid_handler",
            "named_retained",
            "starred_constructor",
            "handler_lookup_failure",
        ] {
            assert!(
                !results.iter().any(|row| row.0 == name),
                "unsupported body became Summary absence"
            );
        }
    }
    if fixture == "stable_capture_shapes" && profile == Profile::Behavioral {
        let rows:Vec<(String,i16,i16,i16,bool)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT spelling.spelling, source.kind, q.modality, q.approximation, capture.under_caller_entry FROM {0}.captured_entry_bindings capture JOIN {0}.captured_value_sources source ON source.id=capture.value_source JOIN {0}.entity_refs er ON er.id=capture.caller JOIN {0}.callable_entities ce ON ce.id=er.callable_callable JOIN {0}.declaration_observations decl ON decl.declaration=ce.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=decl.name JOIN {0}.assertion_qualifications q ON q.id=capture.qualification ORDER BY 1",id.schema()))).fetch_all(db.owner.pool()).await.unwrap();
        eprintln!("B2_CAPTURE {rows:?}");
        assert!(rows.iter().any(|r| r.0 == "captured_entry" && r.1 == 0));
        assert!(rows.iter().any(|r| r.0 == "captured_literal" && r.1 == 1));
        assert!(rows.iter().all(|r| r.2 == 0 && r.3 == 0 && r.4));
        let summaries:Vec<(String,i16,i16,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT spelling.spelling, k.kind, q.modality, q.approximation FROM {0}.summary_capture_witnesses w JOIN {0}.summary_transfer_keys k ON k.id=w.transfer JOIN {0}.entity_refs er ON er.id=k.owner JOIN {0}.callable_entities ce ON ce.id=er.callable_callable JOIN {0}.declaration_observations decl ON decl.declaration=ce.source_declaration JOIN {0}.syntax_observations spelling ON spelling.occurrence=decl.name JOIN {0}.assertion_qualifications q ON q.id=w.qualification JOIN {0}.summary_capture_contributions c ON c.witness=w.id JOIN {0}.summary_transfer_alternatives a ON a.id=c.alternative JOIN {0}.summary_transfer_supports s ON s.assertion=a.id ORDER BY 1",id.schema()))).fetch_all(db.owner.pool()).await.unwrap();
        eprintln!("B2_SUMMARY {summaries:?}");
        assert!(
            summaries
                .iter()
                .any(|r| r == &("captured_entry".into(), 0, 0, 0)),
            "actual supported identity transfer absent"
        );
        assert!(
            summaries
                .iter()
                .any(|r| r == &("captured_literal".into(), 0, 0, 0)),
            "literal capture transfer absent"
        );
        for name in [
            "mutation",
            "call_before_assignment",
            "escaped",
            "delayed",
            "nonlocal_write",
            "global_read",
            "loop_capture",
            "nested_scope",
        ] {
            assert!(
                !rows.iter().any(|r| r.0 == name),
                "unsupported capture hydrated: {name}"
            );
            assert!(
                !summaries.iter().any(|r| r.0 == name),
                "unsupported capture became Summary: {name}"
            );
        }
        let wrong_roots:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.summary_capture_witnesses w JOIN {0}.captured_entry_bindings b ON b.id=w.binding JOIN {0}.captured_value_sources source ON source.id=b.value_source JOIN {0}.summary_transfer_keys k ON k.id=w.transfer JOIN {0}.places p ON p.id=k.input JOIN {0}.place_roots r ON r.id=p.root WHERE (source.kind=0 AND (r.kind<>9 OR r.entry_declaration IS DISTINCT FROM source.entry_declaration)) OR (source.kind=1 AND (r.kind<>7 OR r.occurrence_occurrence IS DISTINCT FROM source.literal_value))",id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(
            wrong_roots, 0,
            "formal and literal sources retain their actual distinct input roots"
        );
        let formal_origin_edges:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.captured_entry_bindings b JOIN {0}.captured_value_sources source ON source.id=b.value_source JOIN {0}.flow_definition_observations d ON d.id=b.origin JOIN {0}.flow_definitions f ON f.id=d.definition JOIN {0}.occurrences native ON native.id=f.occurrence JOIN {0}.occurrences formal ON formal.id=source.entry_declaration JOIN {0}.syntax_placements p ON p.occurrence=native.id AND p.parent=formal.id WHERE source.kind=0 AND formal.syntax_kind=80 AND formal.role=2 AND native.syntax_kind=93 AND native.id<>formal.id AND native.source=formal.source AND native.start=formal.start AND native.\"end\"=formal.\"end\" AND p.field=24 AND p.ordinal=0",id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(
            formal_origin_edges, 1,
            "same-range Identifier origin must use the actual Parameter child edge while retaining its distinct formal root"
        );
        let forged:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {0}.entry_value_witnesses w JOIN {0}.occurrence_ownership owner ON owner.occurrence=w.access WHERE owner.entity<>w.owner",id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(forged, 0, "capture never forges a native caller read");
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

#[tokio::test]
async fn exact_exception_summary_projects_only_actual_completed_bodies() {
    run_fixture(Profile::Behavioral, "exact_exception_shapes").await;
}

// Independent source locations distinguish repeated evaluations and premises through real storage.
async fn assert_conditional_atom_summary(
    pool: &sqlx::PgPool,
    schema: &str,
    root: &std::path::Path,
) {
    let text = std::fs::read_to_string(root.join("cases.py")).unwrap();
    let decisions:Vec<(i64,Vec<u8>,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT o.start,d.leaf,d.outcome FROM {schema}.local_atom_decisions d JOIN {schema}.flow_test_leaf_observations l ON l.id=d.leaf JOIN {schema}.occurrences o ON o.id=l.test"
    ))).fetch_all(pool).await.unwrap();
    for (function, expected) in [
        ("finite_zero", 0_i16),
        ("finite_one", 1),
        ("uninhabited", 3),
    ] {
        let start = text.find(&format!("def {function}(")).unwrap();
        let test = (start + text[start..].find("if value == 0:").unwrap() + 3) as i64;
        assert!(
            decisions
                .iter()
                .any(|(at, _, outcome)| *at == test && *outcome == expected),
            "{function} exact native decision: {decisions:?}"
        );
    }
    let start = text.find("def finite_repeated(").unwrap();
    let end = text[start..].find("def effectful_repeated(").unwrap() + start;
    let repeated = decisions
        .iter()
        .filter(|(at, _, _)| (*at as usize) >= start && (*at as usize) < end)
        .map(|(_, leaf, _)| leaf)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        repeated.len(),
        2,
        "repeated equal source predicates remain two evaluations"
    );
    let forbidden:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {schema}.local_atom_restrictions r JOIN {schema}.local_atom_decisions d ON d.id=r.decision WHERE d.outcome IN (2,3,4)"
    ))).fetch_one(pool).await.unwrap();
    assert_eq!(
        forbidden, 0,
        "mixed, uninhabited and refused answers never prune"
    );
    let alternatives:Vec<(i64,i64,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT o.start,b.count,n.kind FROM {schema}.summary_transfer_alternatives a JOIN {schema}.summary_transfer_keys k ON k.id=a.transfer JOIN {schema}.entity_refs e ON e.id=k.owner JOIN {schema}.callable_entities callable ON callable.id=e.callable_callable JOIN {schema}.occurrences o ON o.id=callable.source_declaration JOIN {schema}.assertion_qualifications q ON q.id=a.qualification JOIN {schema}.assumption_sets b ON b.id=q.assumptions JOIN {schema}.conditions c ON c.id=q.condition JOIN {schema}.condition_nodes n ON n.id=c.root"
    ))).fetch_all(pool).await.unwrap();
    let start = text.find("def finite_zero(").unwrap() as i64;
    let end = text.find("def finite_one(").unwrap() as i64;
    let zero = alternatives
        .iter()
        .filter(|(at, _, _)| *at >= start && *at < end)
        .collect::<Vec<_>>();
    assert!(
        zero.iter()
            .any(|(_, basis, condition)| *basis == 1 && *condition == 1),
        "typing premise must change a real Summary guard to true: {zero:?}"
    );
    assert!(
        zero.iter().any(|(_, basis, _)| *basis == 0),
        "the unconditional runtime alternative survives: {zero:?}"
    );
    let start = text.find("def effectful_repeated(").unwrap() as i64;
    let end = text.find("def nonconforming_runtime(").unwrap() as i64;
    assert!(
        !alternatives
            .iter()
            .any(|(at, basis, _)| *at >= start && *at < end && *basis > 0),
        "effectful predicate calls cannot inherit a parameter truth premise"
    );
}

#[tokio::test]
async fn capture_timing_stable_entries_reach_actual_finite_summaries() {
    run_fixture(Profile::Behavioral, "stable_capture_shapes").await;
}
