//! Real native facts -> frozen checkpoint -> completed-stage normalization and policy views.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::FrontierContract,
    normalized::{
        binding_normalization, callable_normalization, entity_normalization, event_normalization,
        events::*, receiver, relation_normalization,
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
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "policy_view_probes", semantic_source = include_bytes!("normalized_relations.rs"))]
struct PolicyProbe {
    #[model(key)]
    name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "graph_preparation_probes", semantic_source = include_bytes!("normalized_relations.rs"))]
struct GraphPrepared {
    #[model(key)]
    name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "graph_borrow_probes", semantic_source = include_bytes!("normalized_relations.rs"))]
struct GraphBorrowed {
    #[model(key)]
    name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "graph_denied_probes", semantic_source = include_bytes!("normalized_relations.rs"))]
struct GraphDenied {
    #[model(key)]
    name: String,
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
    // This control qualifies the normalized boundary; later analysis owners have not run.
    let mut relations = normalized_relations();
    relations.extend(embedding::text::relations());
    relations.extend([
        Relation::of::<PolicyProbe>(),
        Relation::of::<GraphPrepared>(),
        Relation::of::<GraphBorrowed>(),
        Relation::of::<GraphDenied>(),
    ]);
    let model = Arc::new(ValidatedModel::validate(relations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/normalized_relations");
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(
                &root,
                &[
                    "relations.py".into(),
                    "dual.py".into(),
                    "dual.pyi".into(),
                    "guide.md".into(),
                    "analytic_text.py".into(),
                ],
                budget,
            )
            .unwrap(),
            "entities",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let captured_bytes = budget.reserved();
    let mut providers = cpg_core::facts::providers(ContentHash::of(b"normalized entities fixture"));
    let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    declarations.push(entity_normalization::stage());
    declarations.push(relation_normalization::stage(profile));
    declarations.push(callable_normalization::stage(profile));
    declarations.push(receiver::stage(profile));
    declarations.push(event_normalization::stage(profile));
    declarations.push(binding_normalization::stage(profile));
    declarations.push(projection::normalization::stage(profile));
    let text_definition = embedding::text::TextDefinition {
        requested: true,
        ..embedding::text::TextDefinition::builtin()
    };
    declarations.push(embedding::text::stage(profile, &text_definition).unwrap());
    let mut probe_inputs = event_normalization::stage(profile).inputs;
    macro_rules! probe_input { ($($field:ident: $ty:ty,)*) => { $(probe_inputs.push(RelationUse::stored::<$ty>());)* }; }
    lctx_model::normalized_event_outputs!(probe_input);
    declarations.push(Stage {
        name: "probe_policy_views",
        inputs: probe_inputs,
        outputs: vec![RelationUse::of::<PolicyProbe>()],
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("normalized_relations.rs")),
        configuration: ContentHash::of(b"policy views"),
    });
    let mut graph_inputs = projection::normalization::stage(profile).inputs;
    graph_inputs.extend(
        projection::relations()
            .iter()
            .map(|r| RelationUse::of_relation(r).completed_store()),
    );
    graph_inputs.sort_by_key(|r| r.name());
    graph_inputs.dedup_by_key(|r| r.name());
    let mut graph_stage = Stage {
        name: "prepare_analysis_graphs",
        inputs: graph_inputs,
        outputs: vec![RelationUse::of::<GraphPrepared>()],
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("normalized_relations.rs")),
        configuration: ContentHash::of(b"graph preparation"),
    };
    declarations.push(graph_stage.clone());
    graph_stage.name = "reuse_analysis_graphs";
    graph_stage
        .inputs
        .push(RelationUse::stored::<GraphPrepared>());
    graph_stage.outputs = vec![RelationUse::of::<GraphBorrowed>()];
    declarations.push(graph_stage.clone());
    graph_stage.name = "refuse_unpermitted_graphs";
    graph_stage.inputs = vec![RelationUse::stored::<GraphPrepared>()];
    graph_stage.outputs = vec![RelationUse::of::<GraphDenied>()];
    declarations.push(graph_stage);
    let facts_group = declarations
        .iter()
        .filter(|s| s.outputs.iter().any(|r| is_vocabulary(r.name())))
        .map(|s| s.name)
        .collect();
    let schedule = Schedule::build_with_publications(
        &model,
        declarations,
        &[],
        profile,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts_group),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["normalize_events"]),
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
    let mut prepared: Option<cpg_core::analysis_graphs::PreparedGraphs> = None;
    let mut graph_addresses = std::collections::BTreeMap::new();
    let mut schedules = std::collections::BTreeMap::new();
    for declaration in schedule.stages() {
        if declaration.name == "normalize_entities" {
            attempt.checkpoint(&execution, &facts).await.unwrap();
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::entities(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "analytic_text" {
            cpg_core::analytic_text::publish(
                execution.begin(declaration.name).unwrap(),
                &attempt,
                &config,
                &runtime,
                &model,
                text_definition.clone(),
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_relations" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::relations(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_callables" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::callables(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_receivers" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::receivers(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_events" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::events(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_bindings" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::bindings(access, &attempt, &config, &runtime, &model).await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "normalize_projections" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::normalize::projections(access, &attempt, &config, &runtime, &model)
                        .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "probe_policy_views" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution, declaration, async |access| {
                use cpg_core::generation_read::{AttemptSession, ProviderOptions};
                let reader = AttemptSession::open(&config, &attempt, &access, model.clone(), ProviderOptions::default()).await.unwrap();
                let session = runtime.session(&access);
                assert!(session.register_call_policy_views().await.is_err(), "unregistered or unvalidated sources cannot supply policy views");
                macro_rules! register { ($($ty:ty),+) => { $(let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).unwrap())?;)+ }; }
                register!(NormalizedCallEvent, NormalizedCallAlternative, CallPolicyAssessment, CallPolicyAdmission);
                session.register_call_policy_views().await?;
                for policy in CallPolicy::ALL {
                    let query = session.query(&format!("SELECT COUNT(*) FROM {}", policy.view_name())).await.unwrap();
                    assert_eq!(query.scan_demand(), 3, "logical view retains all three source scans");
                    let batches = query.collect().await.unwrap();
                    let actual = batches[0].column(0).as_any().downcast_ref::<arrow_array::Int64Array>().unwrap().value(0);
                    let expected: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.{}", id.schema(), policy.view_name()))).fetch_one(db.owner.pool()).await.unwrap();
                    assert_eq!(actual, expected, "{} membership parity", policy.view_name());
                }
                drop(session); reader.close().await.unwrap();
                let mut output = StageOutput::new(access, &attempt, &model, budget.clone(), Default::default())?;
                output.declare::<PolicyProbe>()?; output.finish(ProviderOutcome::Complete).await
            }, &mut |_| {}).await.unwrap();
        } else if declaration.name == "refuse_unpermitted_graphs" {
            let access = execution.begin(declaration.name).unwrap();
            let key = *graph_addresses.keys().next().unwrap();
            assert!(
                prepared
                    .as_ref()
                    .unwrap()
                    .graph(&access, budget, key)
                    .is_err(),
                "retention does not grant a later consumer undeclared reads"
            );
            let mut output =
                StageOutput::new(access, &attempt, &model, budget.clone(), Default::default())
                    .unwrap();
            output.declare::<GraphDenied>().unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
        } else if declaration.name == "prepare_analysis_graphs"
            || declaration.name == "reuse_analysis_graphs"
        {
            let access = execution.begin(declaration.name).unwrap();
            if declaration.name == "prepare_analysis_graphs" {
                prepared = Some(
                    cpg_core::analysis_graphs::PreparedGraphs::load(
                        &access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &projection::ProjectionName::ALL.into_iter().collect(),
                    )
                    .await
                    .unwrap(),
                );
            }
            let graphs = prepared.as_ref().unwrap();
            let keys = graphs.keys(&access, budget).unwrap().collect::<Vec<_>>();
            assert_eq!(keys.len(), 4);
            for key in keys {
                let graph = graphs.graph(&access, budget, key).unwrap();
                let address = std::ptr::from_ref(graph) as usize;
                let assessment = graphs.assessment(&access, budget, key).unwrap();
                assert_eq!(assessment.vertices as usize, graph.vertex_count());
                assert_eq!(assessment.arcs as usize, graph.arc_count());
                assert!(
                    graphs
                        .graph(
                            &access,
                            &resources::ResourceBudget::fixed(1 << 20).unwrap(),
                            key
                        )
                        .is_err()
                );
                if declaration.name == "prepare_analysis_graphs" {
                    graph_addresses.insert(key, address);
                } else {
                    assert_eq!(
                        graph_addresses[&key], address,
                        "later consumer borrows the same hydrated graph"
                    );
                }
                if key.name == projection::ProjectionName::CallableInvocation {
                    let scc =
                        lctx_analytics::native_schedule::invocation_sccs(graph, budget).unwrap();
                    if declaration.name == "prepare_analysis_graphs" {
                        schedules.insert(key, scc.components().to_vec());
                    } else {
                        assert_eq!(&schedules[&key], scc.components());
                    }
                }
            }
            let mut output =
                StageOutput::new(access, &attempt, &model, budget.clone(), Default::default())
                    .unwrap();
            if declaration.name == "prepare_analysis_graphs" {
                output.declare::<GraphPrepared>().unwrap();
            } else {
                output.declare::<GraphBorrowed>().unwrap();
            }
            output.finish(ProviderOutcome::Complete).await.unwrap();
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
    let counts: (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.symbol_entity_resolutions), (SELECT count(*) FROM {}.provider_symbols)", id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert!(counts.0 > 0);
    assert_eq!(counts.0, counts.1);
    let leaves: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.flow_test_leaf_observations), (SELECT count(*) FROM {}.test_operand_type_assessments), (SELECT count(*) FROM {}.test_operand_type_links)", id.schema(), id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(leaves.0, leaves.1);
    match profile {
        Profile::Catalog => assert_eq!(leaves, (0, 0, 0)),
        Profile::Behavioral => assert!(leaves.0 > 0 && leaves.2 > 0),
    }
    let signatures: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.signature_observations), (SELECT count(*) FROM {}.signature_variants), (SELECT count(*) FROM {}.effective_callable_assessments)", id.schema(), id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert!(signatures.0 > 0 && signatures.2 > 0);
    assert_eq!(signatures.0, signatures.1);
    let snapshot_counts: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM {}.projection_source_assessments), (SELECT count(*) FROM {}.projection_snapshots), (SELECT count(*) FROM {}.projection_snapshot_chunks)", id.schema(), id.schema(), id.schema())))
        .fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(snapshot_counts.0, 4);
    assert_eq!(snapshot_counts.1, 4);
    assert!(snapshot_counts.2 >= 4);
    let text_bytes: Vec<Vec<u8>> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT text FROM {}.analytic_text_windows",
        id.schema()
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    let texts = text_bytes
        .into_iter()
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text|text=="analytic_text.Container.méthode(self, value: int, /, *items, option=None, **options)\nKeep the original parameter and documentation evidence."),"original source parameter spans and Unicode survive normalization: {texts:?}");
    assert!(texts.iter().any(
        |text| text == "analytic_text.factory.nested(value=\"αβ\")\nNested source declaration."
    ));
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Use `relations.Box`")),
        "original document passages are independent of catalog/retrieval"
    );
    validated.abort().await.unwrap();
    drop(prepared);
    assert_eq!(
        budget.reserved(),
        captured_bytes,
        "only the retained captured input remains charged"
    );
    drop(captured);
    assert_eq!(
        budget.reserved(),
        0,
        "all stage and captured input state released"
    );
}

#[tokio::test]
async fn catalog_normalizes_completed_facts_with_explicitly_unrequested_flow() {
    run(Profile::Catalog).await;
}
#[tokio::test]
async fn behavioral_normalizes_completed_facts_including_exact_test_operands() {
    run(Profile::Behavioral).await;
}
