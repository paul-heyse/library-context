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
    let model = Arc::new(local_model::model());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/local_semantics");
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
    stages.push(analysis::preparation::native_stage(profile, &model, &alignment_publication_order()).unwrap());
    let (parameters, definition) = lctx_model::domain::local_semantics::definition();
    let configuration = analysis::preparation::Configuration::new(
        captured.config().catalog(),
        [(parameters, definition.clone())],
        budget,
    )
    .unwrap();
    stages.push(configuration.declaration());
    stages.push(local_semantics::stage(profile, &definition, &model, &alignment_publication_order()).unwrap());
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
    let checked = attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await;
    let validated = checked.unwrap();
    let counts:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.local_flow_contributions),(SELECT count(*) FROM {}.local_transfer_alternatives),(SELECT count(*) FROM {}.local_flow_assessments WHERE reason IS NOT NULL)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
    if profile == Profile::Behavioral {
        assert!(counts.0 > 0);
        assert!(counts.1 >= counts.0);
        assert!(counts.2 > 0);
    } else {
        assert_eq!(counts, (0, 0, 0));
        let statuses: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT status FROM {}.local_analysis_outcomes",
            id.schema()
        )))
        .fetch_all(db.owner.pool())
        .await
        .unwrap();
        assert!(!statuses.is_empty());
        assert!(statuses.iter().all(|s| *s == 3));
        let availabilities: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT availability FROM {}.local_analysis_coverage",
            id.schema()
        )))
        .fetch_all(db.owner.pool())
        .await
        .unwrap();
        assert!(!availabilities.is_empty());
        assert!(
            availabilities
                .iter()
                .all(|s| *s == normalized::coverage::EvidenceAvailability::NotRequested.code())
        );
    }
    let theory:(i64,i64,i64)=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT (SELECT count(*) FROM {}.local_type_domains),(SELECT count(*) FROM {}.local_theory_witnesses),(SELECT count(*) FROM {}.local_type_class_members)",id.schema(),id.schema(),id.schema()))).fetch_one(db.owner.pool()).await.unwrap();
    if profile == Profile::Behavioral {
        assert!(theory.0 > 0 && theory.1 > 0 && theory.2 >= 4);
        let decisions: Vec<(i64, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT occurrence.start, decision.outcome FROM {0}.local_atom_decisions decision JOIN {0}.flow_test_leaf_observations leaf ON leaf.id=decision.leaf JOIN {0}.occurrences occurrence ON occurrence.id=leaf.test", id.schema()
        ))).fetch_all(db.owner.pool()).await.unwrap();
        let text = std::fs::read_to_string(root.join("cases.py")).unwrap();
        for (function, expected) in [
            ("finite_zero", 1_i16),
            ("finite_one", 0),
            ("uninhabited", 3),
        ] {
            let start = text.find(&format!("def {function}(")).unwrap();
            let test = start + text[start..].find("if value:").unwrap() + 3;
            assert!(
                decisions.contains(&(test as i64, expected)),
                "missing independent {function} decision at {test}: {decisions:?}"
            );
        }
        let refinements: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.local_atom_restrictions r JOIN {0}.local_transfer_alternatives original ON original.id=r.original JOIN {0}.assertion_qualifications oldq ON oldq.id=original.qualification JOIN {0}.assertion_qualifications newq ON newq.id=r.qualification JOIN {0}.assumption_sets oldbasis ON oldbasis.id=oldq.assumptions JOIN {0}.assumption_sets newbasis ON newbasis.id=newq.assumptions WHERE oldbasis.count=0 AND newbasis.count=1 AND oldq.condition<>newq.condition", id.schema()
        ))).fetch_one(db.owner.pool()).await.unwrap();
        assert!(
            refinements >= 2,
            "true and false refinements must change actual transfers while preserving originals"
        );
        // Independent branch expectations: both a predicate and its negation are restricted.
        // The leaf's provider formula is not an ambient guard that excludes the negative arm.
        let constants: Vec<(i64, i16)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT occurrence.start, node.kind FROM {0}.local_atom_restrictions r JOIN {0}.local_atom_decisions decision ON decision.id=r.decision JOIN {0}.flow_test_leaf_observations leaf ON leaf.id=decision.leaf JOIN {0}.occurrences occurrence ON occurrence.id=leaf.test JOIN {0}.assertion_qualifications q ON q.id=r.qualification JOIN {0}.conditions c ON c.id=q.condition JOIN {0}.condition_nodes node ON node.id=c.root", id.schema()
        ))).fetch_all(db.owner.pool()).await.unwrap();
        for function in ["finite_zero", "finite_one"] {
            let start = text.find(&format!("def {function}(")).unwrap();
            let test = (start + text[start..].find("if value:").unwrap() + 3) as i64;
            assert!(
                constants.contains(&(test, 0)) && constants.contains(&(test, 1)),
                "both polarities need independent conditional restrictions for {function}: {constants:?}"
            );
        }
        let empty_refinements: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {0}.local_atom_restrictions r JOIN {0}.local_atom_decisions d ON d.id=r.decision WHERE d.outcome=3", id.schema()
        ))).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(empty_refinements, 0, "Never must not certify either branch");
    } else {
        assert_eq!(theory, (0, 0, 0));
    }
    let complete: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.local_analysis_coverage WHERE availability=0",
        id.schema()
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(complete, 0);
    validated.abort().await.unwrap();
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test]
async fn actual_local_transfers_publish_from_native_completed_sources() {
    run(Profile::Behavioral).await;
}

#[tokio::test]
async fn catalog_local_publishes_not_requested_without_semantic_rows() {
    run(Profile::Catalog).await;
}

#[test]
fn catalog_local_keeps_expected_metadata_out_of_unrequested_domain_state() {
    use lctx_model::domain::{
        attribution::*, calls::*, local_semantics::LocalData, normalized::entities::ClassEntity,
        resources::ResourceBudget, source::*, value::Literal,
    };
    let input = input::InputRevision {
        manifest: ContentHash::of(b"profile retention control"),
    };
    let artifact = SourceArtifact::from_bytes(input.id(), "control.py".into(), b"pass\n").unwrap();
    let scope = CoverageScope::Artifact {
        artifact: artifact.id(),
    };
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "test".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let coverage = ProviderCoverage {
        scope: scope.id(),
        provider: None,
        context: context.id(),
        family: FactFamily::Flow,
        run: None,
        status: CoverageStatus::NotRequested,
        reason: None,
        diagnostic: None,
    };
    let batches = [
        (
            SourceArtifact::NAME,
            SourceArtifact::encode(&[artifact]).unwrap(),
        ),
        (
            CoverageScope::NAME,
            <CoverageScope as Record>::encode(&[scope]).unwrap(),
        ),
        (
            ProviderCoverage::NAME,
            ProviderCoverage::encode(&[coverage]).unwrap(),
        ),
    ];
    let small = ResourceBudget::fixed(1).unwrap();
    let mut catalog = LocalData::new(&small);
    for (name, batch) in &batches {
        assert!(
            !catalog
                .visit_consumed(Profile::Catalog, name, batch)
                .unwrap()
        );
    }
    assert!(
        catalog.entry.artifacts.is_empty()
            && catalog.entry.scopes.is_empty()
            && catalog.entry.coverage.is_empty()
    );
    assert_eq!(
        small.reserved(),
        0,
        "Catalog does not reserve a second copy of expected-only metadata"
    );
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let mut behavioral = LocalData::new(&budget);
    for (name, batch) in &batches {
        assert!(
            behavioral
                .visit_consumed(Profile::Behavioral, name, batch)
                .unwrap()
        );
    }
    assert_eq!(
        (
            behavioral.entry.artifacts.len(),
            behavioral.entry.scopes.len(),
            behavioral.entry.coverage.len()
        ),
        (1, 1, 1)
    );
    let provider = Provider {
        tool: "test".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let module = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "test".into(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: module.id(),
        native_key: "C".into(),
        name: "C".into(),
        kind: SymbolKind::Class,
    };
    let class = ClassEntity::Synthetic {
        symbol: symbol.id(),
    };
    assert!(
        behavioral
            .visit_consumed(
                Profile::Behavioral,
                Literal::NAME,
                &<Literal as Record>::encode(&[Literal::None]).unwrap()
            )
            .unwrap()
    );
    assert!(
        behavioral
            .visit_consumed(
                Profile::Behavioral,
                ClassEntity::NAME,
                &<ClassEntity as Record>::encode(&[class]).unwrap()
            )
            .unwrap()
    );
    assert_eq!(
        (
            behavioral.theory.literals.len(),
            behavioral.fields.class_entities.len()
        ),
        (1, 1),
        "Behavioral visitation reaches both independent inventories"
    );
    let mut catalog = LocalData::new(&budget);
    assert!(
        catalog
            .visit_consumed(
                Profile::Catalog,
                Provider::NAME,
                &Provider::encode(&[provider]).unwrap()
            )
            .unwrap()
    );
    assert_eq!(
        catalog.entry.providers.len(),
        1,
        "Catalog still retains its declared provider metadata"
    );
}

fn alignment_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    use lctx_model::domain::stages::*;
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
        PublicationGroup::new(PublicationBoundary::Model, vec!["model"]),
        PublicationGroup::new(PublicationBoundary::Summary, vec!["summary"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Analytic, vec!["analytic"]),
        PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
    ]).unwrap()
}
