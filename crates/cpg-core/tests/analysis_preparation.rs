//! Real native facts and explicit authored configuration feed one immutable premise projection.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::FrontierContract,
    analysis::{preparation::Configuration, *},
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
async fn both_profiles_store_selected_catalog_and_exact_native_inventory() {
    for profile in Profile::ALL {
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
        // Qualify the early production boundary before normalized/analysis owners run. Their
        // whole-model validators require their own producers and belong to assembled controls.
        let model = Arc::new(
            ValidatedModel::validate(
                facts_relations()
                    .into_iter()
                    .chain(preparation::configuration_relations())
                    .chain(native::relations())
                    .collect(),
            )
            .unwrap(),
        );
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
                    ],
                    budget,
                )
                .unwrap(),
                "analysis preparation",
            )],
            cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
        ));
        let parameters = MethodParameters {
            depth: None,
            proof_steps: None,
            work: None,
            members: None,
            seed: None,
            iterations: None,
            threshold: None,
            resolution: None,
            damping: None,
            model_catalog: Some(captured.config().catalog().declaration().id()),
        };
        let definition = AnalysisDefinition {
            method: AnalysisMethod::Models,
            semantic_version: ContentHash::of(b"selected model control"),
            parameters: parameters.id(),
            interpretation: Interpretation::Structural,
        };
        let analytics = settings::AnalyticsConfiguration {
            module_prefixes: vec!["relations".into()],
            public_roots: vec!["relations".into()],
            configured_seeds: vec!["relations.value".into()],
            depth: 2,
            vertices: 128,
            arcs: 512,
            witnesses: 3,
            brief_budget: 4,
            communities: false,
            pagerank: false,
            fca: true,
            rca: false,
            knn: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        };
        let configuration = Configuration::new(
            captured.config().catalog(),
            [(parameters, definition)],
            budget,
        )
        .unwrap()
        .with_analytics(analytics.clone())
        .unwrap();
        let mut providers = cpg_core::facts::providers(ContentHash::of(b"analysis preparation"));
        let mut declarations = providers
            .iter()
            .map(|p| p.declaration(profile))
            .collect::<Vec<_>>();
        declarations.extend([
            configuration.declaration(),
            preparation::native_stage(profile),
        ]);
        let schedule = Schedule::build(&model, declarations, &[], profile).unwrap();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap();
        cpg_core::analysis_prepare::configuration(
            execution.begin("analysis_configuration").unwrap(),
            &attempt,
            &model,
            &runtime,
            &configuration,
        )
        .await
        .unwrap();
        for stage in schedule.stages() {
            let Some(position) = providers
                .iter()
                .position(|p| p.declaration(profile).name == stage.name)
            else {
                continue;
            };
            run_stage(
                providers.swap_remove(position),
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
        attempt
            .checkpoint(
                &execution,
                &FrontierContract::facts(&model, profile).unwrap(),
            )
            .await
            .unwrap();
        cpg_core::analysis_prepare::native_inventory(
            execution.begin("analysis_native_inventory").unwrap(),
            &attempt,
            &config,
            &runtime,
            &model,
        )
        .await
        .unwrap();
        let generation = attempt.generation();
        let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.native_analysis_premises),(SELECT count(*) FROM {}.native_qualifications),(SELECT count(*) FROM {}.model_catalogs)",generation.schema(),generation.schema(),generation.schema())))
            .fetch_one(db.owner.pool()).await.unwrap();
        assert!(counts.0 > 0);
        assert_eq!(counts.0, counts.1);
        assert_eq!(counts.2, 1);
        let selected:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.analytics_configurations WHERE fca AND NOT rca AND NOT communities AND depth=2 AND brief_budget=4",generation.schema()))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(selected, 1);
        let readonly: bool =
            sqlx::query_scalar("SELECT has_table_privilege('lctx_importer',$1,'INSERT')")
                .bind(format!("{}.native_analysis_premises", generation.schema()))
                .fetch_one(db.owner.pool())
                .await
                .unwrap();
        assert!(!readonly);
        let validated = attempt
            .seal(execution.finish().unwrap())
            .await
            .unwrap()
            .validate()
            .await
            .unwrap();
        validated.abort().await.unwrap();
        drop(configuration);
        drop(captured);
        assert_eq!(budget.reserved(), 0);
    }
}
