//! Actual structural publication over native C0/Local and borrowed normalized graphs.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path="fixtures/structural_mutations.rs"]
mod structural_mutations;
use structural_mutations::{Case,MutatingSink,State};
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
#[tokio::test]
async fn structural_candidates_paths_and_usage_publish_in_both_profiles() {
    for (profile,case) in Profile::ALL.into_iter().map(|p|(p,Case::Truthful)).chain(Case::ALL.into_iter().filter(|c|*c!=Case::Truthful).map(|c|(Profile::Behavioral,c))) {
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
        let model = Arc::new(lctx_model::domain::model().unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/python/structural_usage");
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
            configured_seeds: vec!["api.Client".into(), "api.missing".into()],
            depth: 2,
            vertices: 512,
            arcs: 2048,
            witnesses: 3,
            brief_budget: 8,
            communities: false,
            pagerank: false,
            fca: false,
            rca: false,
            knn: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        };
        let configuration = analysis::preparation::Configuration::new(
            captured.config().catalog(),
            catalog_runtime::definitions().into_iter().chain([
                build::definition(),
                catalog::evidence::build::definition(),
                selection::build::definition(),
                structural::build::definition(&settings, analysis::AnalysisMethod::Delegation)
                    .unwrap(),
                structural::build::definition(&settings, analysis::AnalysisMethod::DirectUsage)
                    .unwrap(),
                structural::build::definition(&settings, analysis::AnalysisMethod::Handoffs)
                    .unwrap(),
                structural::build::definition(&settings, analysis::AnalysisMethod::Controls)
                    .unwrap(),
            ]),
            budget,
        )
        .unwrap()
        .with_analytics(settings.clone())
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
            analysis::preparation::native_stage(profile),
            normalized::callable_aspects::stage(profile),
            build::stage(profile),
            catalog::evidence::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
            selection::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
        ]);
        declarations.extend(catalog_runtime::stages(profile, &model));
        declarations.push(structural::build::stage(profile, &settings, &model).unwrap());
        let facts_members = declarations
            .iter()
            .filter(|s| {
                s.name != "analyze_local"
                    && s.name != "analyze_structural"
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
            ],
        )
        .unwrap();
        assert!(
            !schedule
                .stages()
                .iter()
                .any(|s| s.name.contains("synth") || s.name.contains("embed"))
        );
        let mutation_state=Arc::new(std::sync::Mutex::new(State::default()));
        let mut refused=false;
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
                            cpg_core::normalize::entities(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        1 => {
                            cpg_core::normalize::relations(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        2 => {
                            cpg_core::normalize::callables(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        3 => {
                            cpg_core::normalize::events(access, &attempt, &config, &runtime, &model)
                                .await
                        }
                        4 => {
                            cpg_core::normalize::bindings(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        5 => {
                            cpg_core::normalize::projections(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        7 => {
                            cpg_core::normalize::receivers(
                                access, &attempt, &config, &runtime, &model,
                            )
                            .await
                        }
                        _ => {
                            cpg_core::normalize::coverage(
                                access, &attempt, &config, &runtime, &model,
                            )
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
                .unwrap();
            } else if declaration.name == "analyze_structural" {
                let result=cpg_core::stage_runtime::run_declared_stage(
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
                        if case==Case::Truthful {
                            cpg_core::structural::produce(access,&attempt,&config,&runtime,&model,&graphs).await
                        } else {
                            let sources=structural_mutations::capture(&access,budget)?;
                            let mut admission=analysis::expected::CoverageAdmission::new(&sources,budget)?;
                            structural_mutations::load_admission(&access,&attempt,&config,&runtime,&model,&mut admission).await?;
                            let definitions=structural::build::methods().into_iter().map(|m|structural::build::definition(&settings,m).map(|(_,d)|d)).collect::<Result<Vec<_>,_>>()?;
                            let sink=MutatingSink {sink:&attempt,model:&model,budget,admission:&admission,definitions:&definitions,case,state:mutation_state.clone()};
                            cpg_core::structural::produce_with_sink(access,&attempt,&config,&runtime,&model,&graphs,&sink).await
                        }
                    },
                    &mut |_| {},
                )
                .await;
                if case==Case::Truthful { result.unwrap(); } else {
                    let error=result.expect_err("adversarial Structural output cannot close publication");
                    let detail=error.to_string();
                    let expected=match case {Case::Strengthen|Case::Missing=>"Structural outcomes differ from retained semantic inventory",Case::Extra=>"structural exact invocation/parent membership differs",Case::Paired=>"structural invocation domain incomplete",Case::Truthful=>unreachable!()};
                    assert!(detail.contains(expected),"{case:?}: {detail}");
                    let schema=id.schema();
                    let parents:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {schema}.catalog_core_analysis_invocations"))).fetch_one(db.owner.pool()).await.unwrap();assert!(parents>0,"independent C0 parent survives {case:?}");
                    let (changed,changed_coverage,stops)={let state=mutation_state.lock().unwrap();(state.changed,state.changed_coverage,state.stops.clone())};assert!(changed>0,"{case:?} performed its mutation");
                    if case==Case::Strengthen {assert!(changed_coverage>0);assert!(!stops.is_empty());for (relation,expected) in &stops {let actual=sqlx::query_scalar::<_,Vec<u8>>(sqlx::AssertSqlSafe(format!("SELECT id FROM {schema}.{relation} WHERE stop IS NOT NULL"))).fetch_all(db.owner.pool()).await.unwrap().into_iter().collect::<std::collections::BTreeSet<_>>();assert_eq!(&actual,expected,"stop inventory unchanged");}}
                    eprintln!("profile={profile:?} case={case:?} refused: {detail}");refused=true;break;
                }
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
                        cpg_core::catalog_evidence::produce(
                            access, &attempt, &config, &runtime, &model,
                        )
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
        if refused {
            attempt.fail(&ModelError::Invalid(format!("intentional {case:?} control"))).await.unwrap();
            store.abort(id).await.unwrap();drop(configuration);drop(captured);assert_eq!(budget.reserved(),0);continue;
        }
        let validated = attempt
            .seal(execution.finish().unwrap())
            .await
            .unwrap()
            .validate()
            .await
            .unwrap();
        let s = id.schema();
        let frames: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.structural_frames"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(frames > 0);
        let invocations: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.structural_analysis_invocations"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert_eq!(invocations, frames * 4);
        let public: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.structural_public_candidates"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(public > 0);
        let missing:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_configured_seeds WHERE path='api.missing' AND candidates=0"))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(missing, frames);
        let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows"))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(mismatches, 0);
        let handoffs: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.structural_handoff_occurrences"
        )))
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(
            handoffs > 0,
            "actual official nested result handoff must survive"
        );
        let named:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_handoff_occurrences h JOIN {s}.structural_handoff_values v ON v.id=h.value WHERE v.kind=1"))).fetch_one(db.owner.pool()).await.unwrap();
        if profile == Profile::Behavioral {
            assert!(
                named > 0,
                "actual reaching-definition result handoff must survive"
            );
        } else {
            assert_eq!(
                named, 0,
                "catalog must not claim named result identity without Flow"
            );
        }
        let false_spare:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_handoff_assessments a JOIN {s}.occurrences o ON o.id=a.argument WHERE a.value IS NOT NULL AND o.syntax_kind=48"))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(false_spare, 0, "same type is not producer-result identity");
        let controls:(i64,i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {s}.structural_argument_flows),(SELECT count(*) FROM {s}.structural_argument_flows WHERE alias IS NOT NULL),(SELECT count(*) FROM {s}.structural_conditional_raises),(SELECT count(*) FROM {s}.structural_unfollowed_arguments)"))).fetch_one(db.owner.pool()).await.unwrap();
        if profile == Profile::Behavioral {
            assert!(controls.0 > 0, "actual direct parameter forwarding");
            assert!(controls.1 > 0, "one native identity alias");
            assert!(controls.2 > 0, "qualified local conditional raise");
            assert!(controls.3 > 0, "computed/rebound argument boundaries");
            let caught:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_conditional_raises r JOIN {s}.structural_control_paths p ON p.id=r.path JOIN {s}.structural_control_traversals t ON t.id=p.traversal JOIN {s}.structural_public_candidates c ON c.entity=t.seed AND c.frame=t.frame WHERE c.path IN ('api.caught','api.swallowed','api.tested')"))).fetch_one(db.owner.pool()).await.unwrap();
            assert_eq!(
                caught, 0,
                "catching, swallowed and value-tested paths must suppress propagated raise claim"
            );
        } else {
            assert_eq!(controls, (0, 0, 0, 0));
        }
        if profile == Profile::Behavioral {
            let stopped: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {s}.structural_control_traversals WHERE stop IS NOT NULL"
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(stopped > 0, "bounded forwarding retains its depth stop");
            let literal: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {s}.structural_literal_arguments"
            )))
            .fetch_one(db.owner.pool())
            .await
            .unwrap();
            assert!(literal > 0, "exact native literal argument retained");
            let unpacked:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_unfollowed_arguments WHERE binding IS NULL AND reason=$1"))).bind(obligation::ObligationKind::UnsupportedUnpacking.code()).fetch_one(db.owner.pool()).await.unwrap();
            assert!(
                unpacked > 0,
                "unmapped parameter unpacking remains explicit"
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
        assert!(
            report
                .outcomes
                .iter()
                .any(|r| r.owner == "structural" && r.method == "Delegation")
        );
        assert!(
            report
                .capabilities
                .iter()
                .any(|r| r.capability == "Controls"
                    && r.availability
                        == if profile == Profile::Catalog {
                            "NotRequested"
                        } else {
                            "Partial"
                        })
        );
        assert!(!report.outcomes.iter().any(|r| r.owner == "analytic"));
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("serving_ready")
        );
        drop(report);
        store.retire(id).await.unwrap();
        drop(configuration);
        drop(captured);
        assert_eq!(budget.reserved(), 0);
    }
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
