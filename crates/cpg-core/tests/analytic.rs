//! Actual structural publication over native C0/Local and borrowed normalized graphs.
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
async fn run(
    profile: Profile,
    selected: bool,
    vectors_available: bool,
    extra_layers: bool,
    layer_only: bool,
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
    let mut relations = normalized_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::catalog_core::relations());
    relations.extend(catalog::core_relations());

    relations.extend(analysis::local::relations());
    relations.extend(transfer::local::relations());
    relations.extend(local_semantics::relations());
    relations.extend(local_theory::relations());
    relations.extend(local_fields::relations());
    macro_rules! declared {($($field:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};}
    lctx_model::local_semantic_outputs!(declared);
    relations.extend(analysis::structural::relations());
    relations.extend(structural::relations());
    relations.extend(analysis::analytic::relations());
    relations.extend(analytics::relations());
    relations.extend(analysis::analytic_embedding::relations());
    relations.extend(embedding::relations());
    relations.sort_by_key(Relation::name);
    relations.dedup_by_key(|r| r.name());
    let model = Arc::new(ValidatedModel::validate(relations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/analytic_optional");
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
    let settings = analysis::settings::AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec!["api.alpha".into(), "api.missing".into()],
        depth: 2,
        vertices: 512,
        arcs: 2048,
        witnesses: 3,
        brief_budget: 8,
        communities: selected,
        pagerank: selected,
        fca: selected,
        rca: selected,
        knn: selected && extra_layers && !layer_only,
        type_layer: selected && extra_layers,
        mention_layer: selected && extra_layers,
        knn_layer: selected && extra_layers,
    };
    let (parameters, local) = local_semantics::definition();
    let mut definitions = vec![
        build::definition(),
        (parameters, local.clone()),
        structural::build::definition(&settings, analysis::AnalysisMethod::Delegation).unwrap(),
        structural::build::definition(&settings, analysis::AnalysisMethod::DirectUsage).unwrap(),
        embedding::analytic::definition(),
    ];
    for method in [
        analysis::AnalysisMethod::Handoffs,
        analysis::AnalysisMethod::Controls,
    ] {
        definitions.push(structural::build::definition(&settings, method).unwrap());
    }
    for method in analytics::build::METHODS {
        definitions.push(analytics::build::definition(&settings, method).unwrap());
    }
    let configuration =
        analysis::preparation::Configuration::new(captured.config().catalog(), definitions, budget)
            .unwrap()
            .with_analytics(settings.clone())
            .unwrap();
    let provider = ContractEmbedder::new(vectors_available);
    use cpg_core::embedding_service::Embedder;
    let vectors_requested = settings.knn || settings.knn_layer;
    let service = vectors_requested.then(|| {
        embedding::configuration::Configuration::new(provider.spec(), provider.endpoint(), budget)
            .unwrap()
    });
    let text_definition = embedding::text::TextDefinition {
        requested: vectors_requested,
        ..embedding::text::TextDefinition::builtin()
    };
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
        analysis::preparation::native_stage(profile),
        normalized::callable_aspects::stage(profile),
        build::stage(profile),
    ]);
    declarations.extend([
        local_semantics::stage(profile, &local, &model),
        structural::build::stage(profile, &settings, &model).unwrap(),
        embedding::text::stage(profile, &text_definition).unwrap(),
        embedding::configuration::stage(service.as_ref()),
        embedding::analytic::stage(profile, vectors_requested),
        analytics::build::stage(profile, &settings, &model, &fixture_publication_order()).unwrap(),
    ]);
    let facts_members = declarations
        .iter()
        .filter(|s| {
            s.name != "analyze_local"
                && s.name != "analyze_structural"
                && s.name != "analyze_analytic"
                && s.name != "normalize_events"
                && s.outputs.iter().any(|r| is_vocabulary(r.name()))
        })
        .map(|s| s.name)
        .collect();
    let schedule = Schedule::build_with_publications(
        &model,
        declarations,
        &[],
        profile,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts_members),
            PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
            PublicationGroup::new(PublicationBoundary::Structural, vec!["analyze_structural"]),
            PublicationGroup::new(PublicationBoundary::Analytic, vec!["analyze_analytic"]),
        ],
    )
    .unwrap();
    assert!(!schedule.stages().iter().any(|s| s.name.contains("synth")));
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
        } else if declaration.name == "analyze_local" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::local_semantics::run(
                        access, &attempt, &config, &runtime, &model, &local,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        } else if declaration.name == "analyze_structural" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    let graphs = cpg_core::analysis_graphs::PreparedGraphs::load(
                        &access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &[
                            projection::ProjectionName::CallableInvocation,
                            projection::ProjectionName::DefinitionContainment,
                        ]
                        .into_iter()
                        .collect(),
                    )
                    .await?;
                    cpg_core::structural::produce(
                        access, &attempt, &config, &runtime, &model, &graphs,
                    )
                    .await
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
        } else if declaration.name == "embedding_configuration" {
            cpg_core::analysis_prepare::embedding_configuration(
                execution.begin(declaration.name).unwrap(),
                &attempt,
                &model,
                &runtime,
                service.as_ref(),
            )
            .await
            .unwrap();
        } else if declaration.name == "analytic_embedding" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    cpg_core::analytic_embedding::produce(
                        access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        vectors_requested
                            .then_some(&provider as &dyn cpg_core::embedding_service::Embedder),
                        None,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("E1 failed: {e}"));
        } else if declaration.name == "analyze_analytic" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    let graphs = cpg_core::analysis_graphs::PreparedGraphs::load(
                        &access,
                        &attempt,
                        &config,
                        &runtime,
                        &model,
                        &[projection::ProjectionName::CallableInvocation]
                            .into_iter()
                            .collect(),
                    )
                    .await?;
                    if selected && vectors_available {
                        replay_controls(&access, &attempt, &config, &runtime, &model, &graphs)
                            .await;
                    }
                    cpg_core::analytic::produce(
                        access, &attempt, &config, &runtime, &model, &graphs,
                    )
                    .await
                },
                &mut |_| {},
            )
            .await
            .unwrap_or_else(|e| panic!("A1 failed: {e}"));
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
    let results: Vec<(i16, bool, i16, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT method,selected,status,stop FROM {s}.analytic_technique_results"
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    assert!(!results.is_empty());
    assert_eq!(results.len() % 5, 0);
    assert!(results.iter().all(|r| r.1
        == (selected
            && (extra_layers && !layer_only
                || r.0 != analysis::AnalysisMethod::Neighbours.code()))));
    if !selected {
        assert!(
            results
                .iter()
                .all(|r| r.2 == analysis::AnalysisStatus::NotRequested.code())
        );
        let ranks: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_rank_scores"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert_eq!(ranks, 0);
    } else {
        let ranks: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_rank_scores"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(ranks >= 7);
        let runs: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_community_runs"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(runs >= 40);
        let scopes: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_concept_scopes"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(scopes > 0);
        let parameter_inputs: Vec<(String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT convert_from(a.parameter_name, 'UTF8'), count(DISTINCT i.entity) FROM {s}.analytic_attributes a JOIN {s}.analytic_incidences i ON i.attribute=a.id WHERE a.kind=0 GROUP BY a.parameter_name"
        )))
        .fetch_all(db.owner.pool()).await.unwrap();
        assert!(parameter_inputs.iter().any(|(name, n)| name == "flag" && *n == 1), "defaulted formal must contribute its source parameter attribute");
        assert!(parameter_inputs.iter().any(|(name, n)| name == "value" && *n == 8), "all eight declared value parameters must contribute independently");
        let typed_parameters: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(DISTINCT i.entity) FROM {s}.analytic_incidences i JOIN {s}.analytic_attributes a ON a.id=i.attribute JOIN {s}.analytic_incidence_sources p ON p.id=i.source WHERE a.kind=1 AND p.kind=1"
        )))
        .fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(typed_parameters, 8, "parameter type subjects attach to formals rather than their containers");
        // Independent fixture oracle: only alpha/beta/gamma declare LeftValue. Its actual
        // native final/record traits enrich those parameter ports, with no RightValue negatives.
        let traits: Vec<(String, i16, i16, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT DISTINCT m.name, a.typeclasstrait_role, a.typeclasstrait_basis, a.typeclasstrait_trait_kind FROM {s}.analytic_attributes a JOIN {s}.analytic_incidences i ON i.attribute=a.id JOIN {s}.structural_public_candidates p ON p.entity=i.entity JOIN {s}.catalog_members m ON m.id=p.member WHERE a.kind=8 ORDER BY 1,2,3,4"
        ))).fetch_all(db.owner.pool()).await.unwrap();
        let expected: Vec<_> = ["alpha", "beta", "gamma"].into_iter().map(|name| (name.to_owned(), 0_i16, 1_i16, 0_i16)).collect();
        assert_eq!(traits, expected, "metadata incidence must match the independently enumerated declared-port universe");
        let records: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT DISTINCT m.name FROM {s}.analytic_attributes a JOIN {s}.analytic_incidences i ON i.attribute=a.id JOIN {s}.structural_public_candidates p ON p.entity=i.entity JOIN {s}.catalog_members m ON m.id=p.member JOIN {s}.record_options o ON o.id=a.typeclassrecord_options WHERE a.kind=9 AND a.typeclassrecord_role=0 AND o.frozen ORDER BY 1"
        ))).fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(records, ["alpha", "beta", "gamma"]);
        let uncertain: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_type_metadata_selections WHERE status=1 AND abstract_absence_known IS NULL"
        ))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(uncertain > 0, "builtin class metadata is unavailable rather than a negative trait");
        let known_absence: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_type_metadata_selections WHERE abstract_absence_known"
        ))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(known_absence, 0);
        let captures: Vec<(String, i16, Option<bool>, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT DISTINCT m.name, a.capturedependence_origin, a.capturedependence_mutable, a.capturedependence_timing FROM {s}.analytic_attributes a JOIN {s}.analytic_incidences i ON i.attribute=a.id JOIN {s}.structural_public_candidates p ON p.entity=i.entity JOIN {s}.catalog_members m ON m.id=p.member WHERE a.kind=15 AND a.capturedependence_name='GLOBAL' ORDER BY 1"
        ))).fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(captures, [("isolate".to_owned(), 1_i16, None, 0_i16)], "global capture dependence has unknown mutation and snapshot time");
        let decorators: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT DISTINCT m.name,r.name FROM {s}.analytic_attributes a JOIN {s}.analytic_incidences i ON i.attribute=a.id JOIN {s}.analytic_incidence_sources src ON src.id=i.source JOIN {s}.structural_public_candidates p ON p.entity=i.entity JOIN {s}.catalog_members m ON m.id=p.member JOIN {s}.reference_entity_assessments ra ON ra.id=src.resolveddecorator_assessment JOIN {s}.reference_observations r ON r.id=ra.reference WHERE a.kind=7 AND src.kind=7 ORDER BY 1,2"
        ))).fetch_all(db.owner.pool()).await.unwrap();
        assert_eq!(decorators.into_iter().collect::<std::collections::BTreeSet<_>>(),
            [("isolate".to_owned(), "_marker".to_owned()), ("isolate".to_owned(), "deprecated".to_owned())].into_iter().collect(),
            "only declared decorator heads supply their resolved identities");
        let selected_decorators: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_decorator_selections WHERE status=0"
        ))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(selected_decorators > 0);
        if settings.type_layer {
            let pairs: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {s}.analytic_layer_pairs WHERE layer=2 GROUP BY frame"
            )))
            .fetch_all(db.owner.pool()).await.unwrap();
            assert!(!pairs.is_empty());
            assert!(pairs.iter().all(|count| *count == 6), "two release-class triples supply exactly six pairs; builtin int cannot add a release-class edge");
        }
        let handoffs: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_incidence_sources WHERE kind=4"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        if handoffs == 0 {
            let attempts: Vec<(Option<i16>, i16, i16, i16, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT sig.role,b.authority,b.authority_reason,b.outcome,count(*) FROM {s}.call_binding_attempts b LEFT JOIN {s}.signature_observations sig ON sig.id=b.signature GROUP BY 1,2,3,4 ORDER BY 1,2,3,4"
            ))).fetch_all(db.owner.pool()).await.unwrap();
            let effective: Vec<(i16, i16, i16, i16, i16, i16, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT identity,identity_reason,signatures,signature_reason,descriptor,descriptor_reason,count(*) FROM {s}.effective_callable_assessments GROUP BY 1,2,3,4,5,6 ORDER BY 1,2,3,4,5,6"
            ))).fetch_all(db.owner.pool()).await.unwrap();
            let coverage: Vec<(i16, i16, Option<String>, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT family,status,diagnostic,count(*) FROM {s}.provider_coverage GROUP BY 1,2,3 ORDER BY 1,2,3"
            ))).fetch_all(db.owner.pool()).await.unwrap();
            let a0: Vec<(Option<i16>, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT reason,count(*) FROM {s}.structural_handoff_assessments GROUP BY 1 ORDER BY 1"
            ))).fetch_all(db.owner.pool()).await.unwrap();
            eprintln!("handoff attempts {attempts:?}; effective {effective:?}; coverage {coverage:?}; A0 {a0:?}");
        }
        assert!(handoffs > 0, "RCA must retain exact A0 handoff occurrences");
        if settings.knn && !vectors_available {
            assert!(
                results
                    .iter()
                    .filter(|r| r.0 == analysis::AnalysisMethod::Neighbours.code())
                    .all(|r| matches!(r.2, 1 | 2))
            );
        }
    }
    let selectors: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.analytic_public_selectors"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert!(
        selectors > 2,
        "configured two seed paths cannot shrink all public candidates"
    );
    let public: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.structural_public_candidates WHERE in_subsystem"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(selectors, public);
    let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.analytic_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows"))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(mismatches, 0);
    if selected && !extra_layers {
        let isolated: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_universe_members WHERE excluded_isolate"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(isolated >= 1);
        let co_use: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_layer_pairs WHERE layer=1 AND count=1"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(
            co_use >= 1,
            "distinct call sites in one official scope must co-occur"
        );
    }
    if layer_only {
        let ordinary:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {s}.analytic_neighbours)+(SELECT count(*) FROM {s}.analytic_document_neighbours)+(SELECT count(*) FROM {s}.analytic_community_labels)"))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(ordinary, 0);
        let layer: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.analytic_layer_neighbours"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(layer > 0);
        assert!(
            results
                .iter()
                .filter(|r| r.0 == analysis::AnalysisMethod::Neighbours.code())
                .all(|r| !r.1 && r.2 == analysis::AnalysisStatus::NotRequested.code())
        );
    }
    validated.publish().await.unwrap();
    let serving = RoleConfig {
        role: Role::Serving,
        url: db.url("lctx_serving"),
        max_connections: 3,
        provider_connections: 2,
        ..config.clone()
    };
    let session = cpg_core::generation_read::GenerationSession::open(
        &serving,
        model.clone(),
        id,
        Default::default(),
    )
    .await
    .unwrap();
    let report = cpg_core::analysis_report::read(session, &model)
        .await
        .unwrap();
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|r| r.owner == "analytic")
            .count(),
        results.iter().filter(|r| r.1).count()
    );
    for diagnostic in report.diagnostics.iter().filter(|r| r.owner == "analytic") {
        assert!(diagnostic.iterations.is_some() && diagnostic.examined_members.is_some());
        assert!(
            diagnostic.elapsed_micros.is_none(),
            "readback must not invent elapsed measurements"
        );
    }
    drop(report);
    store.retire(id).await.unwrap();
    drop(configuration);
    drop(service);
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}

use cpg_core::embedding_service::{EmbedFuture, Embedder};
struct ContractEmbedder {
    inner: cpg_core::embedding_service::FakeEmbedder,
    specification: embedding::Spec,
    available: bool,
}
impl ContractEmbedder {
    fn new(available: bool) -> Self {
        let inner = cpg_core::embedding_service::FakeEmbedder::new();
        let mut specification = inner.spec().clone();
        specification.model = "lctx-analytic-contract-unit-vectors".into();
        specification.revision = "unit-e0-v1".into();
        Self {
            inner,
            specification,
            available,
        }
    }
}
impl Embedder for ContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        &self.specification
    }
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        if self.available {
            self.inner.count_tokens(text)
        } else {
            Box::pin(async {
                Err(cpg_core::CoreError::EmbeddingService(
                    "contract unavailable".into(),
                ))
            })
        }
    }
    fn embed<'a>(&'a self, text: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            if !self.available {
                return Err(cpg_core::CoreError::EmbeddingService(
                    "contract unavailable".into(),
                ));
            }
            Ok(text
                .iter()
                .map(|_| {
                    let mut vector = vec![0.0; self.specification.dimensions as usize];
                    vector[0] = 1.0;
                    vector
                })
                .collect())
        })
    }
}
#[tokio::test]
async fn default_off_is_explicit_in_both_profiles() {
    for p in Profile::ALL {
        run(p, false, false, false, false).await;
    }
}
#[tokio::test]
async fn selected_algorithms_publish_actual_nominal_results() {
    run(Profile::Catalog, true, true, true, false).await;
}
#[tokio::test]
async fn selected_vector_failures_remain_visible() {
    run(Profile::Behavioral, true, false, true, false).await;
}

async fn replay_controls(
    access: &StageAccess<'_, '_>,
    attempt: &lctx_postgres::generations::GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    graphs: &cpg_core::analysis_graphs::PreparedGraphs,
) {
    use futures::TryStreamExt;
    async fn read<R: Record>(
        access: &StageAccess<'_, '_>,
        reader: &cpg_core::generation_read::AttemptSession,
        session: &cpg_core::model_runtime::StageSession,
        data: &mut analytics::build::Data,
        seen: &mut std::collections::BTreeSet<&'static str>,
    ) {
        if !access.stage().reads::<R>() || !seen.insert(R::NAME) {
            return;
        }
        let permit = access.read::<R>().unwrap();
        session
            .register(&permit, reader.table(&permit).unwrap())
            .unwrap();
        let q = session
            .query(&format!("SELECT * FROM \"{}\"", R::NAME))
            .await
            .unwrap();
        let mut stream = q.execute_stream().await.unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            data.visit(R::NAME, &batch).unwrap();
        }
    }
    let b = runtime.budget();
    let mut d = analytics::build::Data::new(b);
    let mut seen = std::collections::BTreeSet::new();
    let reader = cpg_core::generation_read::AttemptSession::open(
        config,
        attempt,
        access,
        model.clone(),
        Default::default(),
    )
    .await
    .unwrap();
    let session = runtime.session(access);
    macro_rules! collect{($($f:ident:$t:ty,)*)=>{$(read::<$t>(access,&reader,&session,&mut d,&mut seen).await;)*};}
    lctx_model::normalized_binding_inputs!(collect);
    lctx_model::structural_outputs!(collect);
    lctx_model::analytic_extra_inputs!(collect);
    lctx_model::analytic_consumption_inputs!(collect);
    read::<projection::ProjectionSourceAssessment>(access, &reader, &session, &mut d, &mut seen)
        .await;
    read::<projection::ProjectionSnapshot>(access, &reader, &session, &mut d, &mut seen).await;
    read::<projection::ProjectionSnapshotChunk>(access, &reader, &session, &mut d, &mut seen).await;
    drop(session);
    reader.close().await.unwrap();
    let mut c = analytics::frames::Context::new(b);
    let sources = analysis::sources::CapturedSources::capture(access, b).unwrap();
    let mut out = analytics::Output::new(b);
    for sf in d.structural.frames.iter() {
        let parent = d.structural_invocations.get(sf.invocation).unwrap();
        for method in analytics::build::METHODS {
            let def = analytics::build::definition(d.configuration().unwrap(), method)
                .unwrap()
                .1;
            let parents = analytics::frames::parents(&d, sf, method).unwrap();
            let (inv, inputs, _, _) = analysis::analytic::Invocation::admitted(
                parent.input,
                parent.context,
                def.id(),
                None,
                parents.iter().map(Record::id),
                &sources,
                [analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()],
                b,
            )
            .unwrap();
            for p in parents {
                c.sources.insert(p).unwrap();
            }
            for i in inputs {
                c.inputs.insert(i).unwrap();
            }
            c.invocations.insert(inv).unwrap();
        }
        let f = analytics::AnalyticFrame {
            structural: sf.id(),
            configuration: d.configuration().unwrap().id(),
        };
        let g = graphs
            .graph(
                access,
                b,
                projection::normalization::ProjectionKey {
                    input: parent.input,
                    context: parent.context,
                    name: projection::ProjectionName::CallableInvocation,
                },
            )
            .unwrap();
        out.extend(analytics::build::produce(&d, &f, &c.invocations, g, b).unwrap())
            .unwrap();
    }
    for r in out.results.iter() {
        c.outcomes.insert(analytics::frames::outcome(r)).unwrap();
    }
    analytics::frames::verify(&d, &c, &out, b).unwrap();
    assert!(out.attributes.iter().all(|a| !matches!(a, analytics::Attribute::NativeSignature { .. } | analytics::Attribute::NativeParameterType { .. } | analytics::Attribute::NativeReturnType { .. } | analytics::Attribute::NativeDeprecation { .. })), "default source/declared policy must not silently include native variants");
    // Exercise the public optional projection against actual PostgreSQL/native predecessors.
    // The eight explicit fixture callables provide the independent entity oracle below.
    let policy = analytics::policy::AttributePolicy {
        signature_roles: analytics::policy::SignatureRoles::AllAvailable,
        receiver: analytics::policy::ReceiverPolicy::ExcludeSourceBoundPreserveNative,
    };
    let mut native_context = analytics::frames::Context::new(b);
    let mut native_output = analytics::Output::new(b);
    for sf in d.structural.frames.iter() {
        let parent = d.structural_invocations.get(sf.invocation).unwrap();
        for method in analytics::build::METHODS {
            let (parameters, definition) = analytics::build::definition_with_attribute_policy(d.configuration().unwrap(), method, policy).unwrap();
            assert_ne!(definition.id(), analytics::build::definition(d.configuration().unwrap(), method).unwrap().1.id());
            let (_, include_receiver) = analytics::build::definition_with_attribute_policy(d.configuration().unwrap(), method,
                analytics::policy::AttributePolicy { receiver: analytics::policy::ReceiverPolicy::IncludeSourceBoundPreserveNative, ..policy }).unwrap();
            assert_ne!(definition.id(), include_receiver.id(), "receiver policy participates in persisted definition identity");
            d.parameters.insert(parameters).unwrap();
            d.definitions.insert(definition.clone()).unwrap();
            let parents = analytics::frames::parents(&d, sf, method).unwrap();
            let (invocation, inputs, _, _) = analysis::analytic::Invocation::admitted(parent.input, parent.context, definition.id(), None,
                parents.iter().map(Record::id), &sources,
                [analysis::ProjectionDefinition::builtin(projection::ProjectionName::CallableInvocation).id()], b).unwrap();
            for row in parents { native_context.sources.insert(row).unwrap(); }
            for row in inputs { native_context.inputs.insert(row).unwrap(); }
            native_context.invocations.insert(invocation).unwrap();
        }
        let frame = analytics::AnalyticFrame { structural: sf.id(), configuration: d.configuration().unwrap().id() };
        let graph = graphs.graph(access, b, projection::normalization::ProjectionKey { input: parent.input, context: parent.context,
            name: projection::ProjectionName::CallableInvocation }).unwrap();
        native_output.extend(analytics::build::produce_with_policy(&d, &frame, &native_context.invocations, graph, b, policy).unwrap()).unwrap();
    }
    for result in native_output.results.iter() { native_context.outcomes.insert(analytics::frames::outcome(result)).unwrap(); }
    analytics::frames::verify_with_policy(&d, &native_context, &native_output, b, policy).unwrap();
    assert!(analytics::frames::verify(&d, &native_context, &native_output, b).is_err(), "optional output cannot replay under the default policy");
    let native_names: std::collections::BTreeSet<_> = native_output.incidences.iter().filter(|i| matches!(native_output.attributes.get(i.attribute),
        Some(analytics::Attribute::NativeReturnType { role: calls::SignatureRole::EffectiveTyped, .. }))).flat_map(|i|
            d.structural.public.iter().filter(move |p| p.entity == i.entity).map(|p| d.members.get(p.member).unwrap().name.clone())).collect();
    assert_eq!(native_names, ["alpha", "beta", "delta", "epsilon", "gamma", "identity", "isolate", "zeta"].into_iter().map(str::to_owned).collect(),
        "actual effective native return ports must cover exactly the hand-enumerated public callable set");
    let native_members = &d.members;
    let deprecated: std::collections::BTreeSet<_> = native_output.incidences.iter().filter_map(|i| match native_output.attributes.get(i.attribute) {
        Some(analytics::Attribute::NativeDeprecation { role: calls::SignatureRole::EffectiveTyped, basis: class_metadata::MetadataBasis::NativeEffective,
            availability: types::CallableDeprecation::Deprecated, message }) => Some((i.entity, message.clone())), _ => None,
    }).flat_map(|(entity, message)| d.structural.public.iter().filter(move |p| p.entity == entity)
        .map(move |p| (native_members.get(p.member).unwrap().name.clone(), message.clone()))).collect();
    assert_eq!(deprecated, [("isolate".to_owned(), Some("  use the next isolate  ".to_owned()))].into_iter().collect(),
        "deprecation must use the live native payload without trimming or spelling inference");
    // The same production replay operates on native predecessors plus serialized/hydrated graph.
    let target = out.ranks.iter().next().unwrap().target;
    for corruption in 0..if out.neighbours.is_empty() && out.layer_neighbours.is_empty() {
        6
    } else {
        7
    } {
        let mut forged = analytics::Output::new(b);
        macro_rules! copy{($($f:ident:$t:ty,)*)=>{$(for r in out.$f.iter(){forged.$f.insert(r.clone()).unwrap();})*};}
        lctx_model::analytic_outputs!(copy);
        analytics::frames::verify(&d, &c, &forged, b).unwrap();
        match corruption {
            0 => {
                forged.ranks = normalized::Rows::new(b);
                for r in out.ranks.iter() {
                    let mut r = r.clone();
                    if r.target == target {
                        r.score = FiniteF64::new(r.score.get() + 0.125).unwrap();
                    }
                    forged.ranks.insert(r).unwrap();
                }
            }
            1 => {
                forged.universe = normalized::Rows::new(b);
                for r in out.universe.iter() {
                    let mut r = r.clone();
                    if r.entity == target {
                        r.graph = !r.graph;
                    }
                    forged.universe.insert(r).unwrap();
                }
            }
            2 => {
                forged.selectors = normalized::Rows::new(b);
                let removed = out.selectors.iter().next().unwrap().id();
                for r in out.selectors.iter().filter(|r| r.id() != removed) {
                    forged.selectors.insert(r.clone()).unwrap();
                }
                forged.ranks = normalized::Rows::new(b);
                for r in out.ranks.iter().filter(|r| r.target != target) {
                    forged.ranks.insert(r.clone()).unwrap();
                }
            }
            3 => {
                forged.results = normalized::Rows::new(b);
                for r in out.results.iter() {
                    let mut r = r.clone();
                    if r.method == analysis::AnalysisMethod::PageRank {
                        r.selected = false;
                        r.status = analysis::AnalysisStatus::NotRequested;
                        r.stop = analytics::Stop::NotRequested;
                    }
                    forged.results.insert(r).unwrap();
                }
            }
            4 => {
                forged.partitions = normalized::Rows::new(b);
                let removed = out.partitions.iter().next().unwrap().id();
                for r in out.partitions.iter().filter(|r| r.id() != removed) {
                    forged.partitions.insert(r.clone()).unwrap();
                }
            }
            5 => {
                forged.objects = normalized::Rows::new(b);
                let removed = out.objects.iter().next().unwrap().id();
                for r in out.objects.iter().filter(|r| r.id() != removed) {
                    forged.objects.insert(r.clone()).unwrap();
                }
            }
            _ if !out.neighbours.is_empty() => {
                forged.neighbours = normalized::Rows::new(b);
                let replaced = out.neighbours.iter().next().unwrap().id();
                for r in out.neighbours.iter() {
                    let mut r = r.clone();
                    if r.id() == replaced {
                        assert_ne!(r.query_use, r.target_use);
                        r.query_use = r.target_use;
                    }
                    forged.neighbours.insert(r).unwrap();
                }
            }
            _ => {
                forged.layer_neighbours = normalized::Rows::new(b);
                let replaced = out.layer_neighbours.iter().next().unwrap().id();
                for r in out.layer_neighbours.iter() {
                    let mut r = r.clone();
                    if r.id() == replaced {
                        assert_ne!(r.query_use, r.target_use);
                        r.query_use = r.target_use;
                    }
                    forged.layer_neighbours.insert(r).unwrap();
                }
            }
        }
        assert!(
            analytics::frames::verify(&d, &c, &forged, b).is_err(),
            "forgery {corruption} accepted"
        );
    }
}

#[tokio::test]
async fn baseline_community_scope_preserves_excluded_isolates_and_official_scope_pairs() {
    run(Profile::Catalog, true, true, false, false).await;
}

#[tokio::test]
async fn nearest_community_layer_preserves_unrequested_public_neighbours() {
    run(Profile::Catalog, true, true, true, true).await;
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
