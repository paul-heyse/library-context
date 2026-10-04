//! Native original text and actual PostgreSQL publication with deterministic effect contracts.
//! These checks do not qualify a live embedding service or vector quality.
use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, run_stage},
    capture::CapturedInput,
};
use lctx_model::domain::{
    admission::FrontierContract,
    embedding::{self, analytic, configuration, text},
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
/// One `analysis_embedding_uses` row: availability, admitted tokens, codec and vector bytes.
type UseRow = (i16, Option<i64>, Option<i16>, Option<Vec<u8>>);
async fn run(mode: Mode) {
    let profile = Profile::Catalog;
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
    relations.extend(analysis::analytic_embedding::relations());
    relations.extend(embedding::relations());
    relations.sort_by_key(Relation::name);
    relations.dedup_by_key(|r| r.name());
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
                &["analytic_text.py".into(), "guide.md".into()],
                budget,
            )
            .unwrap(),
            "C0",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let configuration = analysis::preparation::Configuration::new(
        captured.config().catalog(),
        [analytic::definition()],
        budget,
    )
    .unwrap();
    let provider = ContractEmbedder {
        inner: cpg_core::embedding_service::FakeEmbedder::new(),
        mode,
    };
    let requested = !matches!(mode, Mode::Disabled);
    let selection = if requested {
        Some(
            configuration::Configuration::new(provider.spec(), provider.endpoint(), budget)
                .unwrap(),
        )
    } else {
        None
    };
    let text_definition = text::TextDefinition {
        requested,
        ..text::TextDefinition::builtin()
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
        analysis::preparation::native_stage(profile, &model, &alignment_publication_order())
            .unwrap(),
        normalized::callable_aspects::stage(profile),
        text::stage(
            profile,
            &text_definition,
            &model,
            &alignment_publication_order(),
        )
        .unwrap(),
        configuration::stage(selection.as_ref()),
        analytic::stage(profile, requested, &model, &alignment_publication_order()).unwrap(),
    ]);
    let facts_members = declarations
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
            PublicationGroup::new(PublicationBoundary::Facts, facts_members),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["normalize_events"]),
        ],
    )
    .unwrap();
    assert!(
        !schedule
            .stages()
            .iter()
            .any(|s| s.name == "flow" || s.name.contains("synth") || s.name == "catalog_core")
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
                selection.as_ref(),
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
                        if requested { Some(&provider) } else { None },
                        None,
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
    let uses: Vec<UseRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT availability, admitted_tokens, codec, bytes FROM {}.analysis_embedding_uses",
        id.schema()
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    let outcomes: Vec<i16> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT status FROM {}.analytic_embedding_analysis_outcomes",
        id.schema()
    )))
    .fetch_all(db.owner.pool())
    .await
    .unwrap();
    assert!(!outcomes.is_empty());
    match mode {
        Mode::Disabled => {
            assert!(uses.is_empty());
            assert!(outcomes.iter().all(|s| *s == 3));
        }
        Mode::Available => {
            assert!(!uses.is_empty());
            assert!(uses.iter().all(|(a, t, c, b)| *a == 0
                && t.is_some()
                && *c == Some(1)
                && b.as_ref().is_some_and(|b| b.len() == 4096)));
            assert!(outcomes.iter().all(|s| *s == 0));
        }
        Mode::Unavailable => {
            assert!(!uses.is_empty());
            assert!(
                uses.iter()
                    .all(|(a, t, c, b)| *a == 1 && t.is_none() && c.is_none() && b.is_none())
            );
            assert!(outcomes.iter().all(|s| *s == 1));
        }
        Mode::TokenLimit => {
            assert!(!uses.is_empty());
            assert!(uses.iter().all(|(a, t, c, b)| *a == 2
                && t.is_some_and(|n| n > 2048)
                && c.is_none()
                && b.is_none()));
            assert!(outcomes.iter().all(|s| *s == 1));
        }
    }
    let mismatch:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.analytic_embedding_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows",s=id.schema()))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(mismatch, 0);
    validated.abort().await.unwrap();
    drop(configuration);
    drop(selection);
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}
use cpg_core::embedding_service::{EmbedFuture, Embedder};
#[derive(Clone, Copy)]
enum Mode {
    Available,
    Disabled,
    Unavailable,
    TokenLimit,
}
struct ContractEmbedder {
    inner: cpg_core::embedding_service::FakeEmbedder,
    mode: Mode,
}
impl Embedder for ContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        self.inner.spec()
    }
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        match self.mode {
            Mode::TokenLimit => Box::pin(async { Ok(4096) }),
            Mode::Unavailable => Box::pin(async {
                Err(cpg_core::CoreError::EmbeddingService(
                    "contract service unavailable".into(),
                ))
            }),
            _ => self.inner.count_tokens(text),
        }
    }
    fn embed<'a>(&'a self, text: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        self.inner.embed(text)
    }
}
#[tokio::test]
async fn native_analytic_use_publishes_and_cold_replays_exact_vectors() {
    run(Mode::Available).await;
}
#[tokio::test]
async fn native_unrequested_analytic_use_has_no_service_effect() {
    run(Mode::Disabled).await;
}
#[tokio::test]
async fn native_optional_failures_preserve_explicit_per_window_availability() {
    run(Mode::Unavailable).await;
    run(Mode::TokenLimit).await;
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
    ])
    .unwrap()
}
