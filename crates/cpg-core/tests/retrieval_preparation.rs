//! Scoped E0 mandatory helper qualification with actual native owners and disposable PG18.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/catalog_schedule.rs"]
mod catalog_schedule;
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

fn mandatory_reader_inputs(profile: Profile, model: &ValidatedModel) -> Vec<RelationUse> {
    // The helper reads C1's outputs as well as its predecessors. Their nominal references
    // and shared invariants must be eligible inputs too, just as in the final E0 stage.
    // Facts references already belong to the validated checkpoint; every later row below
    // must instead have a completed producer in this fixture's actual schedule.
    let mut inputs = retrieval::build::mandatory_inputs(profile, model).unwrap();
    let facts = facts_relations()
        .iter()
        .map(Relation::name)
        .collect::<std::collections::BTreeSet<_>>();
    let relation = |name| {
        model
            .relations()
            .iter()
            .find(|r| r.name() == name)
            .unwrap_or_else(|| panic!("mandatory reader relation absent: {name}"))
    };
    let mut pending = inputs.iter().map(|r| r.name()).collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        let row = relation(name);
        for required in row
            .fields()
            .iter()
            .filter_map(|f| f.target().map(|(_, n)| n))
            .chain(
                row.invariants()
                    .iter()
                    .flat_map(|i| i.inputs.iter().map(ValidationInput::name)),
            )
        {
            if (!facts.contains(required) || is_vocabulary(required))
                && !inputs.iter().any(|r| r.name() == required)
            {
                inputs.push(RelationUse::of_relation(relation(required)).completed_store());
                pending.push(required);
            }
        }
    }
    for input in &mut inputs {
        if is_vocabulary(input.name()) {
            *input = (*input).at_epoch(PublicationBoundary::Local);
        }
    }
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    inputs
}

#[tokio::test]
async fn mandatory_four_family_preparation_uses_completed_native_catalog_sources() {
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
    // Shared retrieval sums declare canonical prose and Brief targets, including their
    // transitive references. Use the authoritative model for this schema closure; the
    // scoped schedule below still produces only mandatory units, never synthesis or briefs.
    let mut relations = catalog_frontier_relations();
    relations.extend(catalog_runtime::relations());
    relations.sort_by_key(Relation::name);
    relations.dedup_by_key(|r| r.name());
    let model = Arc::new(ValidatedModel::validate(relations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/catalog_context");
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
    let configuration = analysis::preparation::Configuration::new(
        captured.config().catalog(),
        catalog_runtime::definitions()
            .into_iter()
            .chain([build::definition(), catalog::evidence::build::definition()]),
        budget,
    )
    .unwrap()
    .with_retrieval(retrieval::Definition::builtin(false))
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
        catalog::evidence::build::stage(profile, &model).unwrap(),
        Stage {
            name: "retrieval_mandatory_control",
            inputs: mandatory_reader_inputs(profile, &model),
            outputs: retrieval::rendering_relations()
                .iter()
                .map(RelationUse::of_relation)
                .collect(),
            contributes: vec![],
            coverage: vec![],
            provider: None,
            profiles: vec![profile],
            effect: Effect::Pure,
            code: ContentHash::of(b"native-mandatory-retrieval-control"),
            configuration: ContentHash::of(b"disabled-vectors-mandatory-renderer"),
        },
    ]);
    declarations.extend(catalog_runtime::stages(profile, &model));
    let mandatory = declarations
        .iter()
        .find(|s| s.name == "retrieval_mandatory_control")
        .unwrap();
    assert!(mandatory.reads::<analysis::source_call::AnalysisCoverage>());
    for input in &mandatory.inputs {
        assert!(
            declarations
                .iter()
                .filter(|s| s.name != mandatory.name)
                .any(|s| s.outputs.iter().any(|r| r.name() == input.name())),
            "mandatory input has no actual scheduled producer: {}",
            input.name()
        );
    }
    let schedule = catalog_schedule::schedule(&model, declarations, profile);
    for invariant in schedule
        .stages()
        .iter()
        .find(|s| s.name == "retrieval_mandatory_control")
        .unwrap()
        .read_invariants(&model)
        .unwrap()
    {
        for boundary in invariant.inputs.iter().filter_map(ValidationInput::prefix) {
            schedule.prefix_for(boundary).unwrap_or_else(|e| {
                panic!(
                    "mandatory invariant {} has no immutable publication: {e}",
                    invariant.name
                )
            });
        }
    }
    assert!(
        !schedule
            .stages()
            .iter()
            .any(|s| s.name == "flow" || s.name.contains("synth") || s.name.contains("embed"))
    );
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
        .await
        .unwrap();
    let id = attempt.generation();
    let mut expected_receipts = Vec::new();
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
            .unwrap_or_else(|e| panic!("parent {} failed: {e}", declaration.name));
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
        } else if declaration.name == "retrieval_mandatory_control" {
            cpg_core::stage_runtime::run_declared_stage(
                &mut execution,
                declaration,
                async |access| {
                    let (data, rows) = cpg_core::retrieval_preparation::mandatory(
                        &access, &attempt, &config, &runtime, &model,
                    )
                    .await?;
                    // Shared canonical hashing independently describes every expected output;
                    // the completion receipts below are computed from actual PostgreSQL rows.
                    macro_rules! receipts {
                        ($($field:ident:$ty:ty,)*) => { $(
                            let batch = Batch::new(&model, rows.$field.iter().cloned().collect(), budget)?;
                            let relation = Relation::of::<$ty>();
                            let mut content = relation.content();
                            relation.hash_rows(batch.arrow(), &mut content)?;
                            let (count, content) = content.finish();
                            expected_receipts.push((<$ty>::NAME, RelationReceipt { rows: count, content }));
                        )* };
                    }
                    lctx_model::retrieval_outputs!(receipts);
                    let mut output = StageOutput::new(
                        access,
                        &attempt,
                        &model,
                        budget.clone(),
                        Default::default(),
                    )?;
                    cpg_core::retrieval_preparation::publish_mandatory(&mut output, &rows).await?;
                    drop(rows);
                    drop(data);
                    output.finish(ProviderOutcome::Complete).await
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
    let receipt = execution.finish().unwrap();
    assert_eq!(expected_receipts.len(), 9);
    assert_eq!(
        receipt
            .sources()
            .iter()
            .filter(|source| source.producer() == "retrieval_mandatory_control")
            .count(),
        9
    );
    for (relation, expected) in expected_receipts {
        let actual = receipt
            .sources()
            .iter()
            .find(|source| {
                source.producer() == "retrieval_mandatory_control" && source.relation() == relation
            })
            .unwrap_or_else(|| panic!("mandatory output receipt missing: {relation}"));
        assert_eq!(
            actual.receipt(),
            expected,
            "stored mandatory output differs: {relation}"
        );
    }
    let sealed = attempt.seal(receipt).await.unwrap();
    let s = id.schema();
    for family in 0i16..4 {
        let units: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {s}.retrieval_units WHERE family=$1"
        )))
        .bind(family)
        .fetch_one(db.owner.pool())
        .await
        .unwrap();
        assert!(units > 0, "family {family} has no authoritative units");
    }
    let definitions: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.retrieval_definitions"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert_eq!(definitions, 1);
    let forged:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.retrieval_original_anchors a LEFT JOIN {s}.retrieval_anchor_sources x ON x.id=a.original WHERE x.id IS NULL"))).fetch_one(db.owner.pool()).await.unwrap();
    assert_eq!(forged, 0);
    let units: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.retrieval_units"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    let roots: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {s}.retrieval_unit_roots"
    )))
    .fetch_one(db.owner.pool())
    .await
    .unwrap();
    assert!(roots >= units);
    let unrun:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.retrieval_corpus_texts WHERE convert_from(text,'UTF8') LIKE '%execution=NotRun%'"))).fetch_one(db.owner.pool()).await.unwrap();
    assert!(unrun > 0);
    // Mandatory preparation has no selected upper owners. It must not qualify the complete
    // model or obtain a publication capability by leaving those producers unexecuted.
    let refusal = match sealed.validate().await {
        Ok(_) => panic!("partial assembly unexpectedly validated"),
        Err(error) => error,
    };
    assert!(
        matches!(refusal,
        lctx_postgres::generations::Error::Model(ModelError::Invalid(ref message))
        if message == "analytic embedding needs one immutable text definition"),
        "{refusal:?}"
    );
    store.abort(id).await.unwrap();
    drop(configuration);
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}
