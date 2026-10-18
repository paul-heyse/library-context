// The pinned compiler needs this compile-time depth for the finite native
// cursor future graph (rust-lang/rust#159228); keep Send checking intact.
#![recursion_limit = "256"]
#[path = "support/compiler_backing.rs"]
mod physical_backing;
use futures::TryStreamExt;
use lctx_compiler_store::compiler::{NativeCompilerStore, NativePredicate};
use lctx_model::domain::{
    ContentHash, ModelError, Record, Relation,
    completed::{CompletedBinding, ContributionSpec},
    input::Package,
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_native::RuntimeConfig;
use std::collections::{BTreeMap, BTreeSet};

fn assert_atomic_nomination_plan(
    plan: &[lctx_native::surrealdb::types::Value],
    owner: ContentHash,
    relation: &str,
    field: &str,
    values: &[lctx_native::surrealdb::types::Value],
) {
    use lctx_native::surrealdb::types::{RecordId, ToSql, Value};
    fn outer_scans(node: &Value, prefixes: &mut Vec<String>, maximum_branches: usize) {
        match node {
            Value::Array(nodes) => {
                for node in nodes.iter() { outer_scans(node, prefixes, maximum_branches); }
            }
            Value::Object(node) => {
                if let Some(Value::String(operator)) = node.get("operator") {
                    assert!(!operator.ends_with("Scan")
                        || matches!(operator.as_str(), "IndexScan" | "UnionIndexScan"),
                        "unbounded or foreign nomination scan: {node:?}");
                    if operator == "IndexScan" {
                        let attributes = node.get("attributes").unwrap().as_object().unwrap();
                        assert_eq!(attributes.get("index"), Some(&Value::String("member_scope".into())),
                            "nomination must start at exact contributor-qualified fused locators: {node:?}");
                        assert!(!attributes.contains_key("limit") && !attributes.contains_key("offset"),
                            "nomination cannot truncate before exact membership and residual filtering: {node:?}");
                        let Some(Value::String(access)) = attributes.get("access") else {
                            panic!("bounded contributor/relation/token index prefix: {node:?}");
                        };
                        prefixes.push(access.clone());
                    } else if operator == "UnionIndexScan" {
                        let attributes = node.get("attributes").unwrap().as_object().unwrap();
                        assert_eq!(attributes.get("table"), Some(&Value::String("compiler_membership".into())));
                        let Some(Value::String(branches)) = attributes.get("branches") else {
                            panic!("finite locator nomination union: {node:?}");
                        };
                        assert!(branches.parse::<usize>().is_ok_and(|branches| branches > 0 && branches <= maximum_branches),
                            "each union remains within the requested atomic scope window: {node:?}");
                    }
                }
                // The single array-element index stores the complete owner/relation/
                // token tuple as one value. EXPLAIN3.3 renders its exact Equality seek.
                // Only outer children belong to this nomination, not nested expressions.
                if let Some(children) = node.get("children") { outer_scans(children, prefixes, maximum_branches); }
            }
            _ => panic!("native outer nomination plan node: {node:?}"),
        }
    }
    assert!(!values.is_empty() && values.len() <= 32, "production atomic nomination window");
    let mut actual = Vec::new();
    for node in plan { outer_scans(node, &mut actual, values.len()); }
    let owner = Value::RecordId(RecordId::new("compiler_contribution", owner.hex()));
    let mut expected = values.iter().map(|value| {
        format!("= {}", Value::Array(vec![owner.clone(), Value::String(relation.into()), Value::String(format!(
            "{relation}|{field}|{}", lctx_native::prepared::scope_string(value)))].into()).to_sql())
    }).collect::<Vec<_>>();
    actual.sort(); expected.sort();
    assert_eq!(actual, expected, "exact contributor/relation/one-token member_scope equality keys: {plan:?}");
}

async fn fixture_client(
    store: &NativeCompilerStore,
    config: &RuntimeConfig,
) -> std::sync::Arc<
    lctx_native::surrealdb::Surreal<lctx_native::surrealdb::engine::remote::grpc::Client>,
> {
    lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        store.namespace().as_str(),
        store.database().as_str(),
    )
    .await
    .unwrap()
}

fn spec(producer: &str, relation: &Relation) -> ContributionSpec {
    ContributionSpec {
        captured_binding: None,
        producer: producer.into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"view-fixture-model"),
        implementation: ContentHash::of(b"view-fixture-implementation"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([relation.name().into()]),
    }
}

async fn abandon_known_failure(
    store: &NativeCompilerStore,
    config: &RuntimeConfig,
    operation: &str,
    cause: &str,
) {
    use lctx_model::domain::{ModelError, completion::*};
    let failure = store.abandon().await.unwrap_err();
    let ModelError::Completion(outcome) = failure else {
        panic!("failed operation must survive abandonment: {failure:?}");
    };
    assert!(outcome.primary.is_none());
    assert_eq!(outcome.completion.local, LocalState::Terminal);
    assert_eq!(outcome.completion.remote, RemoteState::Confirmed);
    assert_eq!(outcome.completion.storage, vec![]);
    assert_eq!(outcome.completion.failures.len(), 1);
    let retained = &outcome.completion.failures[0];
    assert_eq!(retained.step, operation);
    assert!(
        matches!(retained.error.primary(), Some(ModelError::Conflict(actual)) if *actual == cause)
    );

    // Abandonment closes only this attempt; the installed database and content remain.
    let observer = fixture_client(store, config).await;
    let mut response = observer
        .query("SELECT VALUE state FROM $attempt")
        .bind((
            "attempt",
            lctx_native::surrealdb::types::RecordId::new("native_attempt", store.attempt().hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let states: Vec<String> = response.take(0).unwrap();
    assert_eq!(states, vec!["abandoned"]);
    observer.invalidate().await.unwrap();
}

async fn abandon_acknowledged_write_failure(
    store: &NativeCompilerStore,
    config: &RuntimeConfig,
    original: &ModelError,
) {
    use lctx_model::domain::completion::{LocalState, RemoteState};
    let result = store.abandon().await;
    let failure = result.expect_err("acknowledged write failure remains in the cleanup receipt");
    let ModelError::Completion(outcome) = &failure else {
        panic!("structured cleanup receipt: {failure:?}");
    };
    assert!(
        outcome.primary.is_none(),
        "abandonment does not invent a new primary: {failure:?}"
    );
    assert_eq!(outcome.completion.local, LocalState::Terminal);
    assert_eq!(outcome.completion.remote, RemoteState::Confirmed);
    assert_eq!(outcome.completion.storage, vec![]);
    assert!(outcome.completion.committed.is_empty());
    assert_eq!(
        outcome.completion.failures.len(),
        1,
        "cleanup adds no unrelated failure: {failure:?}"
    );
    let retained = &outcome.completion.failures[0];
    assert_eq!(retained.step, "write_batch");
    match (retained.error.primary(), original.primary()) {
        (Some(ModelError::Conflict(actual)), Some(ModelError::Conflict(expected))) => {
            assert_eq!(actual, expected)
        }
        (Some(ModelError::Schema(actual)), Some(ModelError::Schema(expected))) => {
            assert_eq!(actual, expected)
        }
        (Some(ModelError::Cause(actual)), Some(ModelError::Cause(expected))) => {
            assert!(
                std::ptr::eq(actual.as_ref(), expected.as_ref()),
                "cleanup retains the same acknowledged native cause"
            );
        }
        _ => panic!(
            "cleanup must retain the exact acknowledged primary class and cause: original={original:?}; cleanup={failure:?}"
        ),
    }
    if let (ModelError::SharedCause(actual), ModelError::SharedCause(expected)) =
        (&retained.error, original)
    {
        assert!(
            std::sync::Arc::ptr_eq(actual, expected),
            "cleanup retains the same owned cause"
        );
    }
    // Abandonment closes only this attempt; the installed database and content remain.
    let observer = fixture_client(store, config).await;
    let mut response = observer
        .query("SELECT VALUE state FROM $attempt")
        .bind((
            "attempt",
            lctx_native::surrealdb::types::RecordId::new("native_attempt", store.attempt().hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let states: Vec<String> = response.take(0).unwrap();
    assert_eq!(states, vec!["abandoned"]);
    observer.invalidate().await.unwrap();
}

fn assert_acknowledged_native_guard(failure: &ModelError, guard: &str) {
    use lctx_native::surrealdb::types::{Number, Value};
    let native = failure
        .primary()
        .and_then(|primary| match primary {
            ModelError::Cause(cause) => cause.downcast_ref::<lctx_native::surrealdb::Error>(),
            _ => None,
        })
        .expect("exact native transaction guard cause");
    assert!(native.is_thrown());
    assert_eq!(native.message(), format!("An error occurred: {guard}"));
    let Value::Object(wire) = Value::from_t(native.clone()) else {
        panic!("native error wire object");
    };
    assert_eq!(wire.get("code"), Some(&Value::Number(Number::Int(-32006))));
    let mut envelope = failure;
    loop {
        match envelope {
            ModelError::SharedCause(inner) => envelope = inner.as_ref(),
            ModelError::Cause(cause) => {
                let request = cause
                    .downcast_ref::<lctx_native::control::NativeRequestError>()
                    .expect("retained native request context");
                envelope = &request.source;
            }
            _ => break,
        }
    }
    let ModelError::Completion(outcome) = envelope else {
        panic!("checked native transaction failure: {failure:?}");
    };
    assert_eq!(
        outcome.completion.remote,
        lctx_model::domain::completion::RemoteState::Confirmed
    );
    assert!(failure.permits_storage_cleanup());
    assert!(!failure.has_committed_effect());
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_backing_rejects_valid_body_changes_and_false_typed_keys() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::{FiniteF64, ModelError, analytics::QualityStep};
    use lctx_native::surrealdb::types::{Number, Value};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&store, &config).await;
    let relation = Relation::of::<QualityStep>();
    let row = QualityStep {
        run: serde_json::from_value(serde_json::json!(&store.attempt().0[..16])).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(0.5).unwrap(),
    };
    let contribution = store
        .begin_contribution(spec("quality", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &contribution,
            &relation,
            &QualityStep::encode(std::slice::from_ref(&row)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let view = views[relation.name()].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                &relation,
                spec("quality", &relation).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    store.verify_state(&native_operation_budget).await.unwrap();
    store
        .freeze_content(&native_operation_budget)
        .await
        .unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let state = store
        .export_state(file.path(), &native_operation_budget)
        .await
        .unwrap();
    assert_eq!(state.semantic.backing_rows, 1);
    let restored = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    restored
        .import_state(file.path(), &state, &native_operation_budget)
        .await
        .unwrap();
    restored.abandon().await.unwrap();
    let mut backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(backing.len(), 1);
    let backing = backing.pop().unwrap();
    let saved = vec![backing.row.clone()];
    let mut altered = backing.row.as_object().unwrap().clone();
    let Some(Value::Object(body)) = altered.get_mut("body") else { panic!("quality body"); };
    body.insert("value", Value::Number(Number::Float(2.0)));
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    assert_ne!(replacement.row, saved[0], "body mutation is substantive");
    let changed = replacement.row.as_object().unwrap();
    assert_eq!(changed.get("body").unwrap().as_object().unwrap().get("value"), Some(&Value::Number(Number::Float(2.0))));
    for field in ["id", "canonical", "content", "semantic_key", "scope_keys"] {
        assert_eq!(changed.get(field), saved[0].as_object().unwrap().get(field), "body-only mutation preserves {field}");
    }
    // Ordinary identity reuse precedes the deliberately refused independent read.
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.into_iter().map(|row| row.row).collect::<Vec<_>>(), saved,
        "restore exact logical row and actual locator before refusal assertions");
    assert_eq!(cached, state.semantic, "ordinary frozen identity deliberately reuses its earned owner");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary),
        Some(ModelError::Conflict("compiler backing typed identity" | "compiler backing canonical body"))),
        "body-only corruption independent verification: {verification:?}");

    // Keep body and logical id intact; encode a different valid body in canonical.
    let mut mismatched_body = saved[0].as_object().unwrap().get("body").unwrap().as_object().unwrap().clone();
    mismatched_body.insert("value", Value::Number(Number::Float(2.0)));
    let mismatched_canonical = lctx_native::surrealdb::types::Bytes::from(
        serde_json::to_vec(&Value::Object(mismatched_body)).unwrap());
    let mut altered = backing.row.as_object().unwrap().clone();
    altered.insert("canonical", mismatched_canonical.clone());
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    assert_ne!(replacement.row, saved[0], "canonical mutation is substantive");
    let changed = replacement.row.as_object().unwrap();
    assert_eq!(changed.get("body"), saved[0].as_object().unwrap().get("body"));
    assert_eq!(changed.get("id"), saved[0].as_object().unwrap().get("id"));
    assert_eq!(changed.get("canonical"), Some(&Value::Bytes(mismatched_canonical)));
    // Ordinary identity reuse precedes the deliberately refused independent read.
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.into_iter().map(|row| row.row).collect::<Vec<_>>(), saved,
        "restore canonical bytes before refusal assertions");
    assert_eq!(cached, state.semantic, "cached frozen identity is not independent corruption assurance");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary), Some(ModelError::Conflict("compiler backing canonical body"))),
        "canonical-only corruption independent verification: {verification:?}");

    let mut altered = backing.row.as_object().unwrap().clone();
    altered.insert("semantic_key", "00".repeat(16));
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    assert_eq!(replacement.row.as_object().unwrap().get("semantic_key"), Some(&Value::String("00".repeat(16))));
    assert_ne!(replacement.row, saved[0], "false typed key mutation is substantive");
    for field in ["id", "canonical", "content", "body", "scope_keys"] {
        assert_eq!(replacement.row.as_object().unwrap().get(field), saved[0].as_object().unwrap().get(field),
            "false-key mutation preserves {field}");
    }
    // Ordinary identity reuse precedes the deliberately refused independent read.
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.into_iter().map(|row| row.row).collect::<Vec<_>>(), saved,
        "restore exact physical locator and logical key before refusal assertions");
    assert_eq!(cached, state.semantic, "false-key corruption does not relabel an already-earned root");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary), Some(ModelError::Conflict("compiler backing typed identity"))),
        "false typed key independent verification: {verification:?}");

    store.verify_state(&native_operation_budget).await.unwrap();
    assert_eq!(store.completed_state(&native_operation_budget).await.unwrap(), state.semantic);

    // External transport must reject the row before trusting even an outer state identity.
    let text = std::fs::read_to_string(file.path()).unwrap();
    let mut frames = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let mut changed_backing_rows = 0;
    for frame in &mut frames {
        if frame
            .get("row")
            .and_then(|row| row.get("table"))
            .and_then(|value| value.as_str())
            == Some("compiler_record")
        {
            let mut row: Value = serde_json::from_value(frame["row"]["row"].clone()).unwrap();
            let Value::Object(object) = &mut row else {
                panic!("backing object");
            };
            let Some(Value::Object(body)) = object.get_mut("body") else {
                panic!("backing body");
            };
            body.insert("value", Value::Number(Number::Float(2.0)));
            assert_ne!(object.get("body"), saved[0].as_object().unwrap().get("body"),
                "external body mutation is substantive");
            for field in ["id", "canonical", "content", "semantic_key", "scope_keys"] {
                assert_eq!(object.get(field), saved[0].as_object().unwrap().get(field),
                    "external body-only mutation preserves {field}");
            }
            frame["row"]["row"] = serde_json::to_value(row).unwrap();
            changed_backing_rows += 1;
        }
    }
    assert_eq!(
        changed_backing_rows, 1,
        "external mutation changes one selected backing envelope"
    );
    std::fs::write(
        file.path(),
        frames
            .iter()
            .map(|frame| serde_json::to_string(frame).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    let altered = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let error = altered
        .import_state(file.path(), &state, &native_operation_budget)
        .await
        .unwrap_err();
    // Generic receiving checks exact body/canonical correspondence before the
    // typed nominal callback; this transport mutation changes only the body.
    assert!(
        matches!(
            error.primary(),
            Some(ModelError::Conflict("compiler backing canonical body"))
        ),
        "{error:?}"
    );
    abandon_known_failure(
        &altered,
        &config,
        "import_state",
        "compiler backing canonical body",
    )
    .await;
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_backing_rejects_coherently_renamed_membership_identity() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::ModelError;
    use lctx_native::surrealdb::types::{RecordId, Value, Variables};

    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&store, &config).await;
    let relation = Relation::of::<Package>();
    let rows = vec![
        Package {
            name: "membership-first".into(),
        },
        Package {
            name: "membership-second".into(),
        },
    ];
    let contribution = store
        .begin_contribution(spec("membership-identity", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &contribution,
            &relation,
            &Package::encode(&rows).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let view = views[relation.name()].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                &relation,
                spec("membership-identity", &relation).model,
                &view,
            )
            .unwrap(),
            view: view.clone(),
            configuration: None,
        })
        .await
        .unwrap();
    assert_eq!(view.rows, rows.len() as u64);
    assert_eq!(store.views().await.unwrap(), vec![view]);
    store.verify_state(&native_operation_budget).await.unwrap();

    let mut response = admin
        .query("SELECT * FROM compiler_membership WHERE contribution=$owner ORDER BY semantic_key")
        .bind((
            "owner",
            RecordId::new("compiler_contribution", contribution.hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let memberships: Vec<Value> = response.take(0).unwrap();
    assert_eq!(memberships.len(), 2);
    let Value::Object(mut payload) = memberships[0].clone() else {
        panic!("native membership object");
    };
    let original = payload.remove("id").unwrap();
    let renamed = RecordId::new(
        "compiler_membership",
        ContentHash::of(&store.attempt().0).hex(),
    );
    assert_ne!(original, Value::RecordId(renamed.clone()));
    let mut bindings = Variables::new();
    bindings.insert("original", original.clone());
    bindings.insert("renamed", renamed.clone());
    bindings.insert("payload", Value::Object(payload.clone()));
    // Delete before create preserves the unique owner/relation/key index. The immutable
    // payload and contribution claims remain intact; only the physical lookup ID changes.
    admin.query("BEGIN TRANSACTION; DELETE $original; CREATE $renamed CONTENT $payload; COMMIT TRANSACTION;").bind(bindings).await.unwrap().check().unwrap();
    let mut response = admin
        .query("SELECT * FROM compiler_membership WHERE contribution=$owner ORDER BY semantic_key")
        .bind((
            "owner",
            RecordId::new("compiler_contribution", contribution.hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let memberships: Vec<Value> = response.take(0).unwrap();
    assert_eq!(memberships.len(), 2);
    let Value::Object(mut observed) = memberships[0].clone() else {
        panic!("native membership object");
    };
    assert_eq!(
        observed.remove("id"),
        Some(Value::RecordId(renamed.clone()))
    );
    assert_eq!(
        observed, payload,
        "renaming preserves every membership claim and backing pointer"
    );
    let refusal = store.verify_state(&native_operation_budget).await;
    let mut restore = Variables::new();
    restore.insert("original", original);
    restore.insert("renamed", renamed);
    restore.insert("payload", Value::Object(payload));
    admin.query("BEGIN TRANSACTION; DELETE $renamed; CREATE $original CONTENT $payload; COMMIT TRANSACTION;").bind(restore).await.unwrap().check().unwrap();
    assert!(
        matches!(
            refusal.as_ref().err().and_then(ModelError::primary),
            Some(ModelError::Conflict(
                "compiler membership physical identity"
            ))
        ),
        "{refusal:?}"
    );
    store.verify_state(&native_operation_budget).await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn pending_overlap_frozen_selection_and_state_transport() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use futures::FutureExt;
    use lctx_model::domain::{ModelError, completion};
    use std::panic::AssertUnwindSafe;
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let mut admin_owner = None;
    let mut restored_owner = None;
    let mut restored_admin_owner = None;
    let mut replay_owner = None;
    let mut replay_admin_owner = None;
    let mut store_finalized = false;
    let mut restored_finalized = false;
    let mut replay_finalized = false;
    let outcome = AssertUnwindSafe(async {
        admin_owner = Some(fixture_client(&store, &config).await);
        let admin = admin_owner.as_ref().unwrap();
        let relation = Relation::of::<Package>();
        let first = Package {
            name: "first".into(),
        };
        let second = Package {
            name: "second".into(),
        };
        let a = store
            .begin_contribution(spec("a", &relation))
            .await
            .unwrap();
        store
            .write_batch(
                &a,
                &relation,
                &Package::encode(std::slice::from_ref(&first)).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        store
            .write_batch(
                &a,
                &relation,
                &Package::encode(std::slice::from_ref(&first)).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                a,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        let frozen = views[relation.name()].clone();
        let mut next = spec("b", &relation);
        next.inputs.push(
            lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                &relation, next.model, &frozen,
            )
            .unwrap(),
        );
        let b = store.begin_contribution(next).await.unwrap();
        store
            .write_batch(
                &b,
                &relation,
                &Package::encode(&[first.clone(), second.clone()]).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let mut rows = store
            .scan_batches(&frozen, &relation, None, None, &budget, 8)
            .await
            .unwrap();
        let mut observed = vec![];
        while let Some(batch) = rows.try_next().await.unwrap() {
            observed.extend(Package::decode(&batch).unwrap());
        }
        assert_eq!(
            observed,
            vec![first.clone()],
            "pending values cannot widen a frozen view"
        );
        let current = store
            .complete_contribution(
                b,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
                &native_operation_budget,
            )
            .await
            .unwrap();
        assert_eq!(
            current[relation.name()].rows,
            2,
            "overlapping membership must deduplicate"
        );
        assert_ne!(current[relation.name()].identity, frozen.identity);
        let mut response = admin
            .query("SELECT producer,inputs.relation AS predecessors FROM $owner")
            .bind((
                "owner",
                lctx_native::surrealdb::types::RecordId::new("compiler_contribution", b.hex()),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let projected: Vec<serde_json::Value> = response.take(0).unwrap();
        assert_eq!(
            projected[0]["predecessors"],
            serde_json::json!([relation.name()]),
            "dependency views are natively queryable"
        );
        let mut rows = store
            .scan_batches(
                &current[relation.name()],
                &relation,
                None,
                Some(NativePredicate::Keys(vec![*second.id().bytes()])),
                &budget,
                8,
            )
            .await
            .unwrap();
        let selected = rows.try_next().await.unwrap().unwrap();
        assert_eq!(Package::decode(&selected).unwrap(), vec![second]);
        assert!(rows.try_next().await.unwrap().is_none());
        assert!(
            store.views().await.unwrap().is_empty(),
            "unbound intermediate views are not completed-state roots"
        );
        assert!(matches!(
            store.completed_state(&native_operation_budget).await,
            Err(ModelError::Conflict("native content is not frozen"))
        ));
        let current_view = current[relation.name()].clone();
        store
            .bind(lctx_model::domain::completed::CompletedBinding {
                boundary: None,
                source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation,
                    spec("b", &relation).model,
                    &current_view,
                )
                .unwrap(),
                view: current_view,
                configuration: None,
            })
            .await
            .unwrap();
        store
            .freeze_content(&native_operation_budget)
            .await
            .unwrap();
        let state = store
            .completed_state(&native_operation_budget)
            .await
            .unwrap();
        assert_eq!(state.contributions, 2);
        assert_eq!(state.memberships, 3);
        let file = tempfile::NamedTempFile::new().unwrap();
        let receipt = store
            .export_state(file.path(), &native_operation_budget)
            .await
            .unwrap();
        assert_eq!(receipt.semantic, state);
        restored_owner = Some(
            NativeCompilerStore::begin(
                &config,
                lctx_model::domain::admission::Frontier::Facts,
                &lctx_compiler_store::compiler::recovery_factory(
                    &config,
                    (lctx_model::domain::resources::ResourceBudget::fixed(
                        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                    )
                    .expect("owned native control recovery budget"))
                    .clone(),
                )
                .expect("owned native control factory"),
            )
            .await
            .unwrap(),
        );
        let restored = restored_owner.as_ref().unwrap();
        restored_admin_owner = Some(fixture_client(restored, &config).await);
        let restored_admin = restored_admin_owner.as_ref().unwrap();
        // Complete state references canonical families; detached import loads those separately.
        lctx_native::Loader::new(restored_admin.clone())
            .entities(&[
                lctx_model::domain::graph::Entity::from(first.clone()),
                lctx_model::domain::graph::Entity::from(Package {
                    name: "second".into(),
                }),
            ])
            .await
            .unwrap();
        restored
            .import_state(file.path(), &receipt, &native_operation_budget)
            .await
            .unwrap();
        restored
            .freeze_content(&native_operation_budget)
            .await
            .unwrap();
        assert_eq!(
            restored
                .completed_state(&native_operation_budget)
                .await
                .unwrap(),
            state
        );
        // Replay compares complete immutable membership rows, rather than replacing corrupted
        // metadata or silently ignoring an existing key. Published graph payloads remain identical.
        // A separate mutable attempt owns this mutation challenge; the transported owner is frozen.
        replay_owner = Some(
            NativeCompilerStore::begin(
                &config,
                lctx_model::domain::admission::Frontier::Facts,
                &lctx_compiler_store::compiler::recovery_factory(
                    &config,
                    (lctx_model::domain::resources::ResourceBudget::fixed(
                        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                    )
                    .expect("owned native control recovery budget"))
                    .clone(),
                )
                .expect("owned native control factory"),
            )
            .await
            .unwrap(),
        );
        let replay_store = replay_owner.as_ref().unwrap();
        replay_admin_owner = Some(fixture_client(replay_store, &config).await);
        let replay_admin = replay_admin_owner.as_ref().unwrap();
        let replay = replay_store
            .begin_contribution(spec("replay", &relation))
            .await
            .unwrap();
        let batch = Package::encode(std::slice::from_ref(&first)).unwrap();
        replay_store
            .write_batch(&replay, &relation, &batch, &native_operation_budget)
            .await
            .unwrap();
        replay_store
            .write_batch(&replay, &relation, &batch, &native_operation_budget)
            .await
            .unwrap();
        let mut bindings = lctx_native::surrealdb::types::Variables::new();
        bindings.insert(
            "contribution",
            lctx_native::surrealdb::types::RecordId::new("compiler_contribution", replay.hex()),
        );
        bindings.insert("content", ContentHash::of(b"corrupt-membership").hex());
        replay_admin
            .query(
                "UPDATE compiler_membership SET content=$content WHERE contribution=$contribution",
            )
            .bind(bindings)
            .await
            .unwrap()
            .check()
            .unwrap();
        let error = replay_store
            .write_batch(&replay, &relation, &batch, &native_operation_budget)
            .await
            .unwrap_err();
        assert_acknowledged_native_guard(&error, "native immutable address collision");
        store.abandon().await.unwrap();
        store_finalized = true;
        restored.abandon().await.unwrap();
        restored_finalized = true;
        abandon_acknowledged_write_failure(replay_store, &config, &error).await;
        replay_finalized = true;
    })
    .catch_unwind()
    .await;
    let mut completion = completion::Completion::default();
    if !store_finalized {
        completion.step("pending overlap source finalization", store.abandon().await);
    }
    if let Some(restored) = restored_owner.as_ref()
        && !restored_finalized
    {
        completion.step(
            "pending overlap restored finalization",
            restored.abandon().await,
        );
    }
    if let Some(replay) = replay_owner.as_ref()
        && !replay_finalized
    {
        completion.step(
            "pending overlap replay finalization",
            replay.abandon().await,
        );
    }
    for client in [
        admin_owner.as_ref(),
        restored_admin_owner.as_ref(),
        replay_admin_owner.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        completion.step(
            "pending overlap observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
    }
    let cleanup = completion::complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(secondary) = cleanup {
            eprintln!("pending overlap control failed; owned cleanup also failed: {secondary:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn opaque_original_chunks_use_one_physical_owner_and_detect_same_key_conflict() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::{
        EvidenceBytes, artifact::ArtifactChunk, input::InputRevision, source::SourceArtifact,
    };
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&store, &config).await;
    let mut bytes = (0..(1024 * 1024))
        .map(|index| (index % 256) as u8)
        .collect::<Vec<_>>();
    bytes[..32].copy_from_slice(&store.attempt().0);
    let source = SourceArtifact::from_bytes(
        InputRevision::from_entries(vec![]).unwrap().id(),
        "opaque.bin".into(),
        &bytes,
    )
    .unwrap();
    let relation = Relation::of::<ArtifactChunk>();
    let source_relation = Relation::of::<SourceArtifact>();
    let mut specification = spec("bytes", &relation);
    specification.outputs.insert(source_relation.name().into());
    let contribution = store.begin_contribution(specification).await.unwrap();
    store
        .write_batch(
            &contribution,
            &source_relation,
            &SourceArtifact::encode(std::slice::from_ref(&source)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let row = ArtifactChunk {
        artifact: source.id(),
        ordinal: 0,
        body: EvidenceBytes(bytes.clone()),
    };
    store
        .write_batch(
            &contribution,
            &relation,
            &ArtifactChunk::encode(std::slice::from_ref(&row)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &[relation.clone(), source_relation.clone()],
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    for output in [&relation, &source_relation] {
        let view = views[output.name()].clone();
        store
            .bind(CompletedBinding {
                boundary: None,
                source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    output,
                    spec("bytes", &relation).model,
                    &view,
                )
                .unwrap(),
                view,
                configuration: None,
            })
            .await
            .unwrap();
    }
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let mut rows = store
        .scan_batches(&views[relation.name()], &relation, None, None, &budget, 8)
        .await
        .unwrap();
    assert_eq!(
        ArtifactChunk::decode(&rows.try_next().await.unwrap().unwrap()).unwrap()[0]
            .body
            .0,
        bytes
    );
    assert!(rows.try_next().await.unwrap().is_none());
    let mut response = admin
        .query("SELECT count() AS rows FROM original_chunk WHERE blob=$source.blob GROUP ALL")
        .bind((
            "source",
            lctx_native::surrealdb::types::RecordId::new(
                "original",
                lctx_model::domain::graph::EntityId::of(source.id()).0.hex(),
            ),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let counts: Vec<serde_json::Value> = response.take(0).unwrap();
    assert_eq!(counts, vec![serde_json::json!({"rows":16})]);
    let mut backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(backing.len(), 1);
    let backing = backing.pop().unwrap();
    let metadata = backing.row.as_object().unwrap().get("body").unwrap();
    assert!(matches!(metadata, lctx_native::surrealdb::types::Value::Object(object) if !object.contains_key("body")));
    store.verify_state(&native_operation_budget).await.unwrap();
    let state = store
        .completed_state(&native_operation_budget)
        .await
        .unwrap();
    assert_eq!(state.contributions, 1);
    assert_eq!(state.memberships, 2);
    assert_eq!(state.backing_rows, 1);
    let mut altered = backing.row.as_object().unwrap().clone();
    let Some(lctx_native::surrealdb::types::Value::Object(body)) = altered.get_mut("body") else { panic!("original metadata body"); };
    body.insert("ordinal", 1i64);
    let replacement = backing.replace(&admin, store.attempt(), contribution, lctx_native::surrealdb::types::Value::Object(altered), &native_operation_budget).await;
    let verification = store.verify_state(&native_operation_budget).await;
    let completed = store.completed_state(&native_operation_budget).await;
    replacement.restore(&admin).await;
    assert!(matches!(verification, Err(ModelError::Conflict("compiler original metadata"))), "{verification:?}");
    assert!(matches!(completed, Err(ModelError::Conflict("compiler original metadata"))), "{completed:?}");
    assert_eq!(store.completed_state(&native_operation_budget).await.unwrap(), state);
    // Metadata is intact; the independently read physical byte owner must still agree.
    let mut response = admin
        .query("SELECT * FROM original_chunk WHERE blob=$source.blob ORDER BY start LIMIT 1")
        .bind((
            "source",
            lctx_native::surrealdb::types::RecordId::new(
                "original",
                lctx_model::domain::graph::EntityId::of(source.id()).0.hex(),
            ),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let physical: Vec<lctx_native::surrealdb::types::Value> = response.take(0).unwrap();
    let lctx_native::surrealdb::types::Value::Object(chunk) = &physical[0] else {
        panic!("physical chunk");
    };
    let Some(lctx_native::surrealdb::types::Value::Bytes(original)) = chunk.get("bytes") else {
        panic!("physical bytes");
    };
    let mut changed = original.to_vec();
    changed[0] ^= 1;
    let mut bindings = lctx_native::surrealdb::types::Variables::new();
    bindings.insert("id", chunk.get("id").unwrap().clone());
    bindings.insert(
        "bytes",
        lctx_native::surrealdb::types::Bytes::from(changed.clone()),
    );
    bindings.insert("content", ContentHash::of(&changed).hex());
    admin
        .query("UPDATE $id SET bytes=$bytes,content=$content")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        store.verify_state(&native_operation_budget).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "compiler original digest"
        ))
    ));
    let mut bindings = lctx_native::surrealdb::types::Variables::new();
    bindings.insert("id", chunk.get("id").unwrap().clone());
    bindings.insert("bytes", original.clone());
    bindings.insert("content", ContentHash::of(original).hex());
    admin
        .query("UPDATE $id SET bytes=$bytes,content=$content")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let other = store
        .begin_contribution(spec("conflict", &relation))
        .await
        .unwrap();
    let mut altered = row;
    altered.body.0[0] ^= 1;
    let error = store
        .write_batch(
            &other,
            &relation,
            &ArtifactChunk::encode(&[altered]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap_err();
    assert_acknowledged_native_guard(
        &error,
        "completed original blob refuses missing or changed piece",
    );
    assert!(store.check().is_err());
    abandon_acknowledged_write_failure(&store, &config, &error).await;
    admin.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn overlapping_membership_windows_count_distinct_keys_across_contributors() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<Package>();
    let mut views = BTreeMap::new();
    let mut frozen = Vec::new();
    for (producer, range, expected) in [
        ("a", 0..200, 200),
        ("b", 120..400, 400),
        ("c", 150..450, 450),
        ("empty", 0..0, 450),
    ] {
        let mut specification = spec(producer, &relation);
        if let Some(view) = views.get(relation.name()) {
            specification.inputs.push(
                lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation,
                    specification.model,
                    view,
                )
                .unwrap(),
            );
        }
        let contribution = store.begin_contribution(specification).await.unwrap();
        let rows = range
            .rev()
            .map(|index| Package {
                name: format!("window-{index:04}"),
            })
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            store
                .write_batch(
                    &contribution,
                    &relation,
                    &Package::encode(&rows).unwrap(),
                    &native_operation_budget,
                )
                .await
                .unwrap();
        }
        views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
                &native_operation_budget,
            )
            .await
            .unwrap();
        let view = views[relation.name()].clone();
        assert_eq!(view.rows, expected, "union cardinality for {producer}");
        frozen.push(view);
    }
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    for (view, expected) in frozen.iter().zip([200, 400, 450, 450]) {
        let mut stream = store
            .scan_batches(view, &relation, None, None, &budget, 37)
            .await
            .unwrap();
        let mut observed = BTreeSet::new();
        while let Some(batch) = stream.try_next().await.unwrap() {
            for row in Package::decode(&batch).unwrap() {
                assert!(observed.insert(row.name));
            }
        }
        let expected = (0..expected)
            .map(|index| format!("window-{index:04}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            observed, expected,
            "frozen membership retains its own exact union"
        );
    }
    store.verify_state(&native_operation_budget).await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn large_exact_key_and_atomic_field_selections_preserve_frozen_membership() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use arrow_array::{FixedSizeBinaryArray, StringArray};
    use lctx_compiler_store::compiler_provider::{select_field_table, select_table};
    use lctx_model::domain::{graph::Entity, input::Release};
    use lctx_native::surrealdb::types::{Number, Value};
    use std::sync::Arc;

    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(&config, native_operation_budget.clone())
            .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let scope = store.recovery_scope().unwrap();
    let admin = scope.client().clone();
    let connection = admin.connection_completion().unwrap();
    let relation = Relation::of::<Release>();
    let packages = (0..180)
        .map(|index| Package {
            name: format!("selection-{index:04}"),
        })
        .collect::<Vec<_>>();
    lctx_native::control::with_recovery_scope(scope.clone(), async {
        lctx_native::Loader::for_attempt_views(admin.clone(), store.attempt(), vec![])
            .with_budget(&native_operation_budget)
            .entities(
                &packages
                    .iter()
                    .cloned()
                    .map(Entity::from)
                    .collect::<Vec<_>>(),
            )
            .await
            .unwrap();
    })
    .await;
    let releases = (0..720)
        .map(|index| Release {
            package: packages[index / 4].id(),
            version: (index % 4).to_string(),
        })
        .collect::<Vec<_>>();
    let mut views = BTreeMap::new();
    let mut frozen_owners = Vec::new();
    for (producer, range) in [("selection-a", 0..360), ("selection-b", 240..600)] {
        let mut specification = spec(producer, &relation);
        if let Some(view) = views.get(relation.name()) {
            specification.inputs.push(
                lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation,
                    specification.model,
                    view,
                )
                .unwrap(),
            );
        }
        let contribution = store.begin_contribution(specification).await.unwrap();
        frozen_owners.push(contribution);
        let rows = range
            .rev()
            .map(|index| releases[index].clone())
            .collect::<Vec<_>>();
        store
            .write_batch(
                &contribution,
                &relation,
                &Release::encode(&rows).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
                &native_operation_budget,
            )
            .await
            .unwrap();
    }
    let frozen = views[relation.name()].clone();
    assert_eq!(
        frozen.rows, 600,
        "identical releases from both completed owners count once"
    );
    let mut specification = spec("selection-newer", &relation);
    specification.inputs.push(
        lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
            &relation,
            specification.model,
            &frozen,
        )
        .unwrap(),
    );
    let newer = store.begin_contribution(specification).await.unwrap();
    store
        .write_batch(
            &newer,
            &relation,
            &Release::encode(&releases[480..]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();

    let mut nominal_keys = (30..680)
        .rev()
        .filter(|index| index % 5 != 0)
        .map(|index| *releases[index].id().bytes())
        .collect::<Vec<_>>();
    nominal_keys.extend(
        (100..160)
            .filter(|index| index % 5 != 0)
            .map(|index| *releases[index].id().bytes()),
    );
    nominal_keys.push(
        *Release {
            package: packages[0].id(),
            version: "missing".into(),
        }
        .id()
        .bytes(),
    );
    let missing_package = Package {
        name: "selection-missing".into(),
    };
    let mut field_keys = packages[10..175]
        .iter()
        .map(|package| *package.id().bytes())
        .collect::<Vec<_>>();
    field_keys.push(*packages[179].id().bytes());
    field_keys.push(*missing_package.id().bytes());
    field_keys.sort_unstable();
    let mut field_values = field_keys
        .iter()
        .rev()
        .chain(field_keys[..40].iter())
        .map(|key| {
            Value::Array(
                key.iter()
                    .map(|byte| Value::Number(Number::Int(i64::from(*byte))))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    // The repeated values are separated in the demand, and several real values only belong
    // to the newer owner. Neither repetition nor physical presence changes frozen membership.
    field_values.reverse();
    let mut expected_keys = (30..600)
        .filter(|index| index % 5 != 0)
        .map(|index| releases[index].clone())
        .collect::<Vec<_>>();
    expected_keys.sort_by_key(|row| *row.id().bytes());
    let mut expected_fields = releases[40..600].to_vec();
    expected_fields.sort_by_key(|row| *row.id().bytes());
    let budget = native_operation_budget.clone();
    for completed in [false, true] {
        if completed {
            views = store
                .complete_contribution(
                    newer,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &views,
                    &native_operation_budget,
                )
                .await
                .unwrap();
            assert_eq!(views[relation.name()].rows, 720);
        }
        for (predicate, expected) in [
            (NativePredicate::Keys(nominal_keys.clone()), &expected_keys),
            (
                NativePredicate::Field {
                    field: "package".into(),
                    values: field_values.clone(),
                },
                &expected_fields,
            ),
        ] {
            let selection = match &predicate {
                NativePredicate::Keys(_) => "nominal keys",
                NativePredicate::Field { .. } => "atomic field",
                _ => unreachable!(),
            };
            let mut stream = store
                .scan_batches(&frozen, &relation, None, Some(predicate), &budget, 37)
                .await
                .unwrap();
            let mut observed = Vec::new();
            while let Some(batch) = stream.try_next().await.unwrap_or_else(|error| {
                panic!("{selection}, newer owner completed={completed}: {error:?}")
            }) {
                observed.extend(Release::decode(&batch).unwrap());
            }
            assert_eq!(
                &observed, expected,
                "selected records remain sorted and exact with newer owner completed={completed}"
            );
        }
        for (package_end, release_end, expected_count) in [(91, 364, 124), (123, 492, 252)] {
            let mut keys = packages[60..package_end]
                .iter()
                .chain([&packages[179], &missing_package])
                .map(|package| *package.id().bytes())
                .collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), package_end - 60 + 2);
            let values = keys
                .iter()
                .map(|key| {
                    Value::Array(
                        key.iter()
                            .map(|byte| Value::Number(Number::Int(i64::from(*byte))))
                            .collect::<Vec<_>>()
                            .into(),
                    )
                })
                .collect::<Vec<_>>();
            // Independent fixture answers: four releases per in-view package. The
            // overlap between physical owners must not duplicate them; package179
            // belongs only to the newer owner and the missing package has no rows.
            let mut expected = releases[240..release_end].to_vec();
            expected.sort_by_key(Record::id);
            assert_eq!(expected.len(), expected_count);
            let mut stream = store
                .scan_batches(
                    &frozen,
                    &relation,
                    None,
                    Some(NativePredicate::Field {
                        field: "package".into(),
                        values: values.clone(),
                    }),
                    &budget,
                    37,
                )
                .await
                .unwrap();
            let mut observed = Vec::new();
            while let Some(batch) = stream.try_next().await.unwrap() {
                observed.extend(Release::decode(&batch).unwrap());
            }
            assert_eq!(
                observed,
                expected,
                "{}-key direct field window, newer completed={completed}",
                keys.len()
            );
            drop(stream);

            let provider = store
                .table_provider(&frozen, relation.clone(), budget.clone(), 37)
                .await
                .unwrap();
            let mut charge = lctx_model::domain::charged::StateCharge::new(
                &budget,
                "atomic-boundary-control-keys",
            );
            charge.grow(keys.len() * 160 + 4096).unwrap();
            let provider =
                select_field_table(&provider, "package", Arc::new(keys), Arc::new(charge))
                    .unwrap()
                    .unwrap();
            let session = datafusion::prelude::SessionContext::new();
            session.register_table("atomic_boundary", provider).unwrap();
            let batches = session
                .table("atomic_boundary")
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            let observed = batches
                .iter()
                .flat_map(|batch| Release::decode(batch).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                observed, expected,
                "selected-provider field windows, newer completed={completed}"
            );
            drop(batches);
            drop(session);

            if !completed {
                // EXPLAIN the actual fused-locator builder. Its32/1 and32/32/1
                // windows retain exact owner/relation/token prefixes. The direct and
                // selected-provider scans above independently check final atomic field
                // filtering on complete canonical rows, after nomination and hydration.
                for owner in &frozen_owners {
                    for window in values.chunks(32) {
                        let query = lctx_compiler_store::compiler::atomic_field_nomination_query(
                            &relation, "package", *owner, window, true,
                        )
                        .unwrap();
                        assert_eq!(query.expected_terminals(), 1);
                        assert_eq!(query.result_positions(), &[0]);
                        let (sql, bindings) = query.into_request();
                        assert_eq!(bindings.get("owner"), Some(&Value::RecordId(
                            lctx_native::surrealdb::types::RecordId::new("compiler_contribution", owner.hex()))));
                        assert_eq!(bindings.get("relation"), Some(&Value::String(relation.name().into())));
                        let qualified = window.iter().map(|value| Value::Array(vec![
                            Value::RecordId(lctx_native::surrealdb::types::RecordId::new("compiler_contribution", owner.hex())),
                            Value::String(relation.name().into()), Value::String(format!("{}|package|{}",
                                relation.name(), lctx_native::prepared::scope_string(value)))].into())).collect::<Vec<_>>();
                        let (name, expected, containment) = if window.len() == 1 {
                            ("__compiler_scope_key", qualified[0].clone(), "scope_lookup CONTAINS $__compiler_scope_key")
                        } else { ("__compiler_scope_lookup", Value::Array(qualified.into()), "scope_lookup CONTAINSANY $__compiler_scope_lookup") };
                        assert_eq!(bindings.get(name), Some(&expected),
                            "the planner receives literal full owner/relation/token keys, including singleton input");
                        assert!(sql.contains(&format!("SELECT relation,semantic_key,node FROM compiler_membership WITH INDEX member_scope WHERE contribution=$owner AND relation=$relation AND {containment} EXPLAIN;")),
                            "nomination preserves exact owner, relation and atomic tuple predicate: {sql}");
                        let mut response =
                            lctx_native::control::with_recovery_scope(scope.clone(), async {
                                admin
                                    .query(sql)
                                    .bind(bindings)
                                    .await
                                    .unwrap()
                                    .check()
                                    .unwrap()
                            })
                            .await;
                        let plan: Vec<Value> = response.take(0).unwrap();
                        assert_atomic_nomination_plan(&plan, *owner, relation.name(), "package", window);
                    }
                }
            }
        }
    }

    // Cancel an exact membership-window wait and resume the same returned owner.
    // No accepted key may be skipped or re-emitted when the SDK request is pending.
    let columns = vec!["id".to_string()];
    let mut resumed = store
        .scan_rows(
            &frozen,
            &relation,
            Some(&columns),
            Some(NativePredicate::Keys(nominal_keys.clone())),
            &budget,
        )
        .await
        .unwrap();
    let mut interrupted = Box::pin(resumed.next());
    assert!(futures::poll!(&mut interrupted).is_pending());
    drop(interrupted);
    let mut resumed_keys = Vec::new();
    while let Some(row) = resumed.next().await.unwrap() {
        let Value::Object(row) = row else {
            panic!("resumed selected row");
        };
        let Some(Value::String(key)) = row.get("id") else {
            panic!("resumed selected nominal identity");
        };
        resumed_keys.push(key.clone());
    }
    assert_eq!(
        resumed_keys,
        expected_keys
            .iter()
            .map(|row| row.id().hex())
            .collect::<Vec<_>>()
    );
    drop(resumed);

    nominal_keys.sort_unstable();
    nominal_keys.dedup();
    let mut expected_all = releases[..600].to_vec();
    expected_all.sort_by_key(Record::id);
    let package_list = packages[10..175]
        .iter()
        .chain(std::iter::once(&missing_package))
        .map(|package| format!("X'{}'", package.id().hex()))
        .collect::<Vec<_>>()
        .join(",");
    for (table_name, field_selection) in [
        ("nominal_selected", Some(false)),
        ("field_selected", Some(true)),
        ("all_frozen", None),
    ] {
        let provider = store
            .table_provider(&frozen, relation.clone(), budget.clone(), 37)
            .await
            .unwrap();
        let charge = Arc::new(lctx_model::domain::charged::StateCharge::new(
            &budget,
            "selected-release-fixture",
        ));
        let provider = match field_selection {
            Some(true) => {
                select_field_table(&provider, "package", Arc::new(field_keys.clone()), charge)
                    .unwrap()
                    .unwrap()
            }
            Some(false) => select_table(&provider, Arc::new(nominal_keys.clone()), charge)
                .unwrap()
                .unwrap(),
            None => provider,
        };
        let session = datafusion::prelude::SessionContext::new();
        session.register_table(table_name, provider).unwrap();
        let source = match field_selection {
            Some(true) => &expected_fields,
            Some(false) => &expected_keys,
            None => &expected_all,
        };
        let cases = [
            (
                "version = '2'".to_string(),
                source
                    .iter()
                    .filter(|row| row.version == "2")
                    .collect::<Vec<_>>(),
            ),
            (
                format!("package IN ({package_list}) AND version = '2'"),
                source
                    .iter()
                    .filter(|row| {
                        row.version == "2"
                            && packages[10..175]
                                .iter()
                                .any(|package| package.id() == row.package)
                    })
                    .collect(),
            ),
            (
                format!(
                    "(package = X'{}' AND version = '2') OR (package = X'{}' AND version = '3')",
                    packages[11].id().hex(),
                    packages[14].id().hex()
                ),
                source
                    .iter()
                    .filter(|row| {
                        (row.package == packages[11].id() && row.version == "2")
                            || (row.package == packages[14].id() && row.version == "3")
                    })
                    .collect(),
            ),
        ];
        for (predicate, expected) in cases {
            let batches = session
                .sql(&format!(
                    "SELECT id, version FROM {table_name} WHERE {predicate}"
                ))
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            let mut observed = Vec::new();
            for batch in batches {
                assert_eq!(
                    batch
                        .schema()
                        .fields()
                        .iter()
                        .map(|field| field.name().as_str())
                        .collect::<Vec<_>>(),
                    vec!["id", "version"]
                );
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .unwrap();
                let versions = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                for index in 0..batch.num_rows() {
                    observed.push((ids.value(index).to_vec(), versions.value(index).to_owned()));
                }
            }
            let expected = expected
                .into_iter()
                .map(|row| (row.id().bytes().to_vec(), row.version.clone()))
                .collect::<Vec<_>>();
            assert_eq!(
                observed, expected,
                "projected {table_name} retains exact membership, static filtering and unique sorted output"
            );
        }
    }
    // Nested field demand stays invariant across multiple nominal transfer windows.
    // Absent keys enlarge demand, never the exact frozen universe. Cluster them around
    // a real middle identity so both transfer windows contain real candidate identities.
    let mut wide_keys = nominal_keys.clone();
    let pivot = *expected_keys[expected_keys.len() / 2].id().bytes();
    wide_keys.extend((0u32..5000).map(|index| {
        let mut key = pivot;
        key[12..].copy_from_slice(&index.to_be_bytes());
        key
    }));
    wide_keys.sort_unstable();
    wide_keys.dedup();
    assert!(wide_keys.len() > lctx_model::domain::resources::TRANSFER_ROWS);
    for window in wide_keys.chunks(lctx_model::domain::resources::TRANSFER_ROWS) {
        assert!(
            expected_keys.iter().any(|row| row.version == "2"
                && field_keys.binary_search(row.package.bytes()).is_ok()
                && window.binary_search(row.id().bytes()).is_ok()),
            "both windows must exercise field and residual predicates on real identities"
        );
    }
    {
        let provider = store
            .table_provider(&frozen, relation.clone(), budget.clone(), 37)
            .await
            .unwrap();
        let charge = Arc::new(lctx_model::domain::charged::StateCharge::new(
            &budget,
            "nested-window-fixture",
        ));
        let provider = select_table(&provider, Arc::new(wide_keys), charge.clone())
            .unwrap()
            .unwrap();
        let provider =
            select_field_table(&provider, "package", Arc::new(field_keys.clone()), charge)
                .unwrap()
                .unwrap();
        let session = datafusion::prelude::SessionContext::new();
        session.register_table("nested_windows", provider).unwrap();
        let batches = session
            .sql("SELECT id,version FROM nested_windows WHERE version = '2'")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let mut observed = Vec::new();
        for batch in batches {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap();
            let versions = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            for index in 0..batch.num_rows() {
                observed.push((ids.value(index).to_vec(), versions.value(index).to_owned()));
            }
        }
        let expected = expected_keys
            .iter()
            .filter(|row| {
                row.version == "2" && field_keys.binary_search(row.package.bytes()).is_ok()
            })
            .map(|row| (row.id().bytes().to_vec(), row.version.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            observed, expected,
            "every nominal window retains the same field and residual predicates"
        );
        drop(session);
    }
    // An unsupported residual stays above the source. Fetch must consume rejected
    // rows before counting output, and cannot stop on the first native candidate.
    let provider = store
        .table_provider(&frozen, relation.clone(), budget.clone(), 37)
        .await
        .unwrap();
    let session = datafusion::prelude::SessionContext::new();
    session.register_table("fetch_frozen", provider).unwrap();
    for predicate in ["version = '2'", "version LIKE '2'"] {
        let batches = session
            .sql(&format!(
                "SELECT id,version FROM fetch_frozen WHERE {predicate} LIMIT 1"
            ))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let expected = expected_all.iter().find(|row| row.version == "2").unwrap();
        assert_eq!(
            batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
            1
        );
        let batch = batches.iter().find(|batch| batch.num_rows() > 0).unwrap();
        assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
                .value(0),
            expected.id().bytes()
        );
        assert_eq!(
            batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "2"
        );
    }
    drop(session);
    drop(admin);
    drop(scope);
    store.abandon().await.unwrap();
    drop(store);
    connection.wait().await.unwrap();
    drop(connection);
    assert_eq!(native_operation_budget.reserved(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn canonical_graph_scans_select_completed_families_and_one_hop_aliases() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    use lctx_model::domain::{FiniteF64, analytics::QualityStep, graph::Entity};
    use lctx_native::surrealdb::types::{Bytes, SurrealValue, Value};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Normalized,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<lctx_model::domain::value::Place>();
    let first = lctx_model::domain::value::Place {
        root: serde_json::from_value(serde_json::to_value([21u8; 16]).unwrap()).unwrap(),
        path: serde_json::from_value(serde_json::to_value([22u8; 16]).unwrap()).unwrap(),
    };
    let alias = lctx_model::domain::normalized::entities::EntityRef::Place { place: first.id() };
    let pending = Package {
        name: "pending-unselected".into(),
    };
    let quality = QualityStep {
        run: serde_json::from_value(serde_json::json!(vec![8u8; 16])).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(0.25).unwrap(),
    };
    let metadata = Relation::of::<QualityStep>();
    let mut specification = spec("complete", &relation);
    specification.outputs.insert(metadata.name().into());
    let complete = store.begin_contribution(specification).await.unwrap();
    store
        .write_batch(
            &complete,
            &relation,
            &lctx_model::domain::value::Place::encode(std::slice::from_ref(&first)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    store
        .write_batch(
            &complete,
            &metadata,
            &QualityStep::encode(&[quality]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            complete,
            ProviderOutcome::Complete,
            &[relation.clone(), metadata],
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    for selected_relation in [&relation, &Relation::of::<QualityStep>()] {
        let view = views[selected_relation.name()].clone();
        store
            .bind(CompletedBinding {
                boundary: None,
                source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    selected_relation,
                    spec("complete", &relation).model,
                    &view,
                )
                .unwrap(),
                view,
                configuration: None,
            })
            .await
            .unwrap();
    }
    let pending_relation = Relation::of::<Package>();
    let uncompleted = store
        .begin_contribution(spec("pending", &pending_relation))
        .await
        .unwrap();
    store
        .write_batch(
            &uncompleted,
            &pending_relation,
            &Package::encode(&[pending]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let mut rows = store.scan_canonical(true, &budget).await.unwrap();
    let mut selected = BTreeSet::new();
    while let Some(row) = rows.next().await.unwrap() {
        let Value::Object(object) = row else {
            panic!("canonical graph record");
        };
        let bytes = Bytes::from_value(object.get("canonical").unwrap().clone()).unwrap();
        let entity: Entity = serde_json::from_slice(&bytes).unwrap();
        selected.insert(entity.id());
    }
    assert_eq!(
        selected,
        BTreeSet::from([Entity::from(first).id(), Entity::from(alias).id()])
    );
    let mut headers = store.scan_graph_headers(true, &budget).await.unwrap();
    let mut count = 0;
    while headers.next().await.unwrap().is_some() {
        count += 1;
    }
    assert_eq!(
        count, 2,
        "nongraph backing and unrelated pending entities stay excluded"
    );
    let mut assertions = store.scan_canonical(false, &budget).await.unwrap();
    assert!(assertions.next().await.unwrap().is_none());
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn installed_reader_pins_exact_closure_through_source_abandon_and_releases_after_drain() {
    use lctx_native::surrealdb::types::{RecordId, Variables};
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    ))
    .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<Package>();
    let nonce = lctx_native::control::fresh_identity("installed-reader-lifetime").unwrap();
    let row = Package {
        name: format!("installed-reader-{}", nonce.hex()),
    };
    let physical = store
        .begin_contribution(spec(&format!("reader-{}", nonce.hex()), &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &physical,
            &relation,
            &Package::encode(std::slice::from_ref(&row)).unwrap(),
            &budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            physical,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
            &budget,
        )
        .await
        .unwrap();
    let view = &views[relation.name()];
    let held = store.installed_view(view, &budget).await.unwrap();
    let completion = RecordId::new("compiler_contribution", physical.hex());
    let source_pin = held.owner().clone();
    let client = fixture_client(&store, &config).await;
    // Receiving authority rechecks actual native incarnation and full bytes even
    // when the opaque source already earned and retained its exact inventory.
    let guard = RecordId::new(
        "native_guard",
        ContentHash::of(
            &serde_json::to_vec(&lctx_native::surrealdb::types::Value::RecordId(
                completion.clone(),
            ))
            .unwrap(),
        )
        .hex(),
    );
    for (kind, id, mutation) in [
        (
            "physical incarnation",
            guard.clone(),
            "UPDATE $id SET incarnation+=1 RETURN NONE",
        ),
        (
            "inactive physical guard",
            guard,
            "UPDATE $id SET phase='retired' RETURN NONE",
        ),
        (
            "full contributor bytes",
            completion.clone(),
            "UPDATE $id SET producer='receiving-corruption' RETURN NONE",
        ),
    ] {
        let mut variables = Variables::new();
        variables.insert("id", id);
        let mut response = client
            .query("SELECT * FROM $id")
            .bind(variables.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        let original: Vec<lctx_native::surrealdb::types::Value> = response.take(0).unwrap();
        assert_eq!(original.len(), 1);
        client
            .query(mutation)
            .bind(variables.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        let received = lctx_native::NativeReader::for_installed_views(
            client.clone(),
            std::slice::from_ref(&held),
            &budget,
        )
        .await;
        // Restore only the exact owned row before reporting refusal. An accidental
        // successful receiver still owns a real pin, which must be drained first.
        variables.insert("row", original[0].clone());
        client
            .query("UPSERT $id CONTENT $row RETURN NONE")
            .bind(variables)
            .await
            .unwrap()
            .check()
            .unwrap();
        if let Ok(unexpected) = received {
            unexpected.close().await.unwrap();
            panic!("fresh receiving pin accepted changed {kind}");
        }
    }
    let reader = lctx_native::NativeReader::for_installed_views(
        client.clone(),
        std::slice::from_ref(&held),
        &budget,
    )
    .await
    .unwrap();
    let from_store = store
        .reader_for_views(std::slice::from_ref(view), &budget)
        .await
        .unwrap();
    let observer = lctx_native::NativeReader::private(client.clone()).with_budget(&budget);
    held.ensure_borrowed_current(&observer, &budget)
        .await
        .unwrap();
    let closed_borrow =
        lctx_native::NativeReader::borrow_installed_views(std::slice::from_ref(&held), &budget)
            .await
            .unwrap();
    assert_eq!(
        closed_borrow
            .records::<Package>(lctx_native::reader::RecordSelection::Keys(vec![
                *row.id().bytes()
            ]))
            .await
            .unwrap(),
        vec![row.clone()]
    );
    closed_borrow.close().await.unwrap();
    drop(closed_borrow);
    held.ensure_borrowed_current(&observer, &budget)
        .await
        .unwrap();
    assert_eq!(
        store
            .scan_batches(view, &relation, None, None, &budget, 32)
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
            .iter()
            .flat_map(|batch| Package::decode(batch).unwrap())
            .collect::<Vec<_>>(),
        vec![row.clone()],
        "closing an operation-local reader preserves its source pin and SDK session"
    );
    let borrowed =
        lctx_native::NativeReader::borrow_installed_views(std::slice::from_ref(&held), &budget)
            .await
            .unwrap();
    assert_eq!(
        borrowed
            .records::<Package>(lctx_native::reader::RecordSelection::Keys(vec![
                *row.id().bytes()
            ]))
            .await
            .unwrap(),
        vec![row.clone()]
    );
    let pins = async {
        let mut bindings = Variables::new();
        bindings.insert("completion", completion.clone());
        observer.query_native::<Vec<RecordId>>("SELECT VALUE owner FROM native_hold WHERE object=$completion AND record::table(owner)='native_pin'",bindings).await
    };
    let before = pins.await.unwrap();
    assert_eq!(
        before.iter().filter(|owner| **owner != source_pin).count(),
        2,
        "both readers own exact contributor closure independently of the source audit pin"
    );
    store.abandon().await.unwrap();
    assert!(
        held.ensure_borrowed_current(&observer, &budget)
            .await
            .is_err(),
        "an internal source borrow ends with its actual source pin; independently received readers retain their own authority"
    );
    assert!(
        borrowed.prepare_selection().await.is_err(),
        "a cached internal selection cannot outlive its actual source pin"
    );
    assert!(
        borrowed
            .query_native::<i64>("RETURN 1", Variables::new())
            .await
            .is_err(),
        "source close refuses new borrowed queries before using its invalidated SDK session"
    );
    borrowed.close().await.unwrap();
    drop(borrowed);
    for selected in [&reader, &from_store] {
        assert_eq!(
            selected
                .records::<Package>(lctx_native::reader::RecordSelection::Keys(vec![
                    *row.id().bytes()
                ]))
                .await
                .unwrap(),
            vec![row.clone()],
            "source invalidation cannot end independently owned reader sessions"
        );
    }
    reader.close().await.unwrap();
    drop(reader);
    from_store.close().await.unwrap();
    drop(from_store);
    let mut bindings = Variables::new();
    bindings.insert("completion", completion);
    let after:Vec<RecordId>=observer.query_native("SELECT VALUE owner FROM native_hold WHERE object=$completion AND record::table(owner)='native_pin'",bindings).await.unwrap();
    assert!(
        after.is_empty(),
        "confirmed reader drainage releases its exact closure pin"
    );
    for pin in before {
        assert!(
            lctx_native::control::retirement_eligibility(&client, pin)
                .await
                .unwrap()
                .eligible,
            "released pin may enter ordinary retirement"
        );
    }
    observer.close().await.unwrap();
    client.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_empty_reads_retain_view_shape_and_lifecycle_checks() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    use lctx_model::domain::completed::CompletedView;
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<Package>();
    let empty = store
        .begin_contribution(spec("empty-read", &relation))
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            empty,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let frozen = views[relation.name()].clone();
    assert_eq!(frozen.rows, 0);
    let native = store
        .reader_for_views(std::slice::from_ref(&frozen), &budget)
        .await
        .unwrap();
    assert!(
        native
            .records::<Package>(lctx_native::reader::RecordSelection::Keys(vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    let mut empty = native.relation_bodies(Package::NAME, 0).unwrap();
    assert!(empty.next().await.unwrap().is_none());
    empty.drain_transport().await.unwrap();
    drop(empty);
    let mut empty = native.relation_bodies(Package::NAME, 1).unwrap();
    assert!(empty.next().await.unwrap().is_none());
    empty.drain_transport().await.unwrap();
    drop(empty);
    native.close().await.unwrap();
    drop(native);

    assert!(
        store
            .scan_rows(&frozen, &relation, None, None, &budget)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .is_none()
    );
    let invalid = vec!["missing_field".to_owned()];
    assert!(
        store
            .scan_rows(&frozen, &relation, Some(&invalid), None, &budget)
            .await
            .is_err()
    );
    let invalid = NativePredicate::Field {
        field: "missing_field".into(),
        values: vec![],
    };
    assert!(
        store
            .scan_rows(&frozen, &relation, None, Some(invalid), &budget)
            .await
            .is_err()
    );
    let forged =
        CompletedView::new(relation.name().into(), frozen.contributions.clone(), 1).unwrap();
    assert!(
        store
            .scan_rows(
                &forged,
                &relation,
                None,
                Some(NativePredicate::Keys(vec![])),
                &budget
            )
            .await
            .is_err()
    );
    let next = store
        .begin_contribution(spec("populated-read", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &next,
            &relation,
            &Package::encode(&[Package {
                name: "present".into(),
            }])
            .unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let current = store
        .complete_contribution(
            next,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &views,
            &native_operation_budget,
        )
        .await
        .unwrap();
    let view = &current[relation.name()];
    assert_eq!(view.rows, 1);
    for predicate in [
        NativePredicate::Keys(vec![]),
        NativePredicate::Field {
            field: "name".into(),
            values: vec![],
        },
    ] {
        assert!(
            store
                .scan_rows(view, &relation, None, Some(predicate), &budget)
                .await
                .unwrap()
                .next()
                .await
                .unwrap()
                .is_none()
        );
    }
    // An empty physical demand is also preserved through the shared selected provider.
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let provider = store
        .table_provider(view, relation.clone(), budget.clone(), 8)
        .await
        .unwrap();
    let charge = std::sync::Arc::new(lctx_model::domain::charged::StateCharge::new(
        &budget,
        "empty-key-owner",
    ));
    let provider = lctx_compiler_store::compiler_provider::select_table(
        &provider,
        std::sync::Arc::new(vec![]),
        charge,
    )
    .unwrap()
    .unwrap();
    let session = datafusion::prelude::SessionContext::new();
    session.register_table("selected", provider).unwrap();
    let batches = session
        .sql("SELECT * FROM selected")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(
        batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
        0
    );
    // Newer members do not widen the exact completed empty view.
    assert!(
        store
            .scan_rows(&frozen, &relation, None, None, &budget)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .is_none()
    );
    store.end_writes().await.unwrap();
    assert!(
        store
            .scan_rows(&frozen, &relation, None, None, &budget)
            .await
            .is_err()
    );
    assert!(
        store
            .scan_rows(
                view,
                &relation,
                None,
                Some(NativePredicate::Keys(vec![])),
                &budget
            )
            .await
            .is_err()
    );
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn membership_prefix_selection_uses_scalar_index_and_exact_owners() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_native::surrealdb::types::{ToSql, Value, Variables};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&store, &config).await;
    let relation = Relation::of::<Package>();
    let shared = Package {
        name: "prefix-shared".into(),
    };
    let absent = Package {
        name: "prefix-absent".into(),
    };
    let foreign = Package {
        name: "prefix-foreign".into(),
    };
    let mut views = BTreeMap::new();
    let mut expected = vec![shared.clone()];
    for index in 0..16 {
        let own = Package {
            name: format!("prefix-owner-{index}"),
        };
        expected.push(own.clone());
        let mut specification = spec(&format!("prefix-{index}"), &relation);
        if let Some(view) = views.get(relation.name()) {
            specification.inputs.push(
                lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation,
                    specification.model,
                    view,
                )
                .unwrap(),
            );
        }
        let id = store.begin_contribution(specification).await.unwrap();
        store
            .write_batch(
                &id,
                &relation,
                &Package::encode(&[shared.clone(), own]).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        views = store
            .complete_contribution(
                id,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
                &native_operation_budget,
            )
            .await
            .unwrap();
    }
    let frozen = views[relation.name()].clone();
    let id = store
        .begin_contribution(spec("prefix-unrelated", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &id,
            &relation,
            &Package::encode(&[shared.clone(), foreign.clone()]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();

    // Qualify the actual 3.3 native plan: a two-column unary prefix on the
    // three-column index, rather than a table walk or a multi-value union.
    let mut bindings = Variables::new();
    bindings.insert("relation", relation.name().to_string());
    bindings.insert("key", shared.id().hex());
    let mut response = admin.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX member_keys WHERE relation=$relation AND semantic_key=$key EXPLAIN").bind(bindings).await.unwrap().check().unwrap();
    let plan: Vec<Value> = response.take(0).unwrap();
    // SurrealDB 3.3's pipeline planner emits a plan tree, not the retired flat
    // "Iterate Index" ledger. Admit exactly one scalar-prefix IndexScan leaf;
    // table scans, native union/dedup/order operators cannot pass this shape.
    let [Value::Object(project)] = plan.as_slice() else {
        panic!("single pipeline projection: {plan:?}")
    };
    assert_eq!(
        project.get("operator"),
        Some(&Value::String("SelectProject".into())),
        "{plan:?}"
    );
    let Some(Value::Array(children)) = project.get("children") else {
        panic!("projection scan: {plan:?}")
    };
    let [Value::Object(scan)] = children.as_slice() else {
        panic!("one native branch: {plan:?}")
    };
    assert_eq!(
        scan.get("operator"),
        Some(&Value::String("IndexScan".into())),
        "{plan:?}"
    );
    assert!(
        scan.get("children").is_none(),
        "scalar index leaf: {plan:?}"
    );
    let Some(Value::Object(attributes)) = scan.get("attributes") else {
        panic!("index attributes: {plan:?}")
    };
    assert_eq!(
        attributes.get("index"),
        Some(&Value::String("member_keys".into())),
        "{plan:?}"
    );
    let prefix = Value::Array(
        vec![
            Value::String(relation.name().to_string()),
            Value::String(shared.id().hex()),
        ]
        .into(),
    );
    assert_eq!(
        attributes.get("access"),
        Some(&Value::String(prefix.to_sql())),
        "scalar relation/key prefix: {plan:?}"
    );
    assert_eq!(
        attributes.get("direction"),
        Some(&Value::String("Forward".into())),
        "{plan:?}"
    );

    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let mut keys = vec![
        *shared.id().bytes(),
        *expected[7].id().bytes(),
        *absent.id().bytes(),
        *foreign.id().bytes(),
    ];
    keys.sort_unstable();
    let sparse = store
        .row_tokens(&frozen, &relation, &keys, &budget)
        .await
        .unwrap();
    assert_eq!(
        sparse.iter().map(|(key, _)| *key).collect::<BTreeSet<_>>(),
        BTreeSet::from([*shared.id().bytes(), *expected[7].id().bytes()])
    );
    let mut selected = store
        .scan_batches(
            &frozen,
            &relation,
            None,
            Some(NativePredicate::Keys(keys.clone())),
            &budget,
            13,
        )
        .await
        .unwrap();
    let mut observed = Vec::new();
    while let Some(batch) = selected.try_next().await.unwrap() {
        observed.extend(Package::decode(&batch).unwrap());
    }
    let mut selected_expected = vec![shared, expected[7].clone()];
    selected_expected.sort_by_key(Record::id);
    assert_eq!(observed, selected_expected);

    // A broad consumer prepares actual memberships once. Independent concurrent cursors
    // keep complete sorted output, and subsequent tokens retain exactly the sparse domain.
    let mut first = store
        .scan_batches(&frozen, &relation, None, None, &budget, 3)
        .await
        .unwrap();
    let mut second = store
        .scan_batches(&frozen, &relation, None, None, &budget, 5)
        .await
        .unwrap();
    let mut a = Vec::new();
    let mut b = Vec::new();
    loop {
        let (left, right) = tokio::join!(first.try_next(), second.try_next());
        match (left.unwrap(), right.unwrap()) {
            (None, None) => break,
            (left, right) => {
                if let Some(batch) = left {
                    a.extend(Package::decode(&batch).unwrap());
                }
                if let Some(batch) = right {
                    b.extend(Package::decode(&batch).unwrap());
                }
            }
        }
    }
    expected.sort_by_key(Record::id);
    assert_eq!(a, expected);
    assert_eq!(b, expected);
    let dense_keys = expected
        .iter()
        .map(|row| *row.id().bytes())
        .collect::<Vec<_>>();
    assert_eq!(
        store
            .row_tokens(&frozen, &relation, &dense_keys, &budget)
            .await
            .unwrap()
            .len(),
        expected.len()
    );
    assert_eq!(
        store
            .row_tokens(&frozen, &relation, &keys, &budget)
            .await
            .unwrap(),
        sparse
    );
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_current_observation_refusal_does_not_poison_later_consumer() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::ModelError;
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<Package>();
    let expected = Package {
        name: "healthy-after-local-preparation-refusal".into(),
    };
    let contribution = store
        .begin_contribution(spec("preparation-refusal", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &contribution,
            &relation,
            &Package::encode(std::slice::from_ref(&expected)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let view = &views[relation.name()];

    // Earn the exact installed view under its source owner's budget before testing
    // a consumer-local current-observation reservation. First physical earning is a separate
    // prerequisite and must not consume the tiny budget intended for this refusal.
    store
        .installed_view(view, &native_operation_budget)
        .await
        .unwrap();

    // Consumer setup fits, but the required fresh current-installation observation
    // refuses before its query or membership preparation. No native effect has failed.
    let refused = ResourceBudget::fixed(4096).unwrap();
    let error = match store.scan_rows(view, &relation, None, None, &refused).await {
        Err(error) => error,
        Ok(_) => panic!("the current observation's local budget must refuse"),
    };
    assert!(
        matches!(
            error.primary(),
            Some(ModelError::Resource {
                owner: "earned borrowed current installation observation",
                ..
            })
        ),
        "{error:?}"
    );
    assert!(error.permits_storage_cleanup());
    assert_eq!(
        refused.reserved(),
        0,
        "failed observation and setup reservations are released"
    );
    store
        .check()
        .expect("local refusal does not poison native authority");

    // A separate healthy consumer may prepare this same exact view. It does not
    // silently retry the refused call, and its immutable result remains reusable.
    let healthy = ResourceBudget::fixed(32 << 20).unwrap();
    for _ in 0..2 {
        let mut stream = store
            .scan_batches(view, &relation, None, None, &healthy, 1)
            .await
            .unwrap();
        let mut observed = Vec::new();
        while let Some(batch) = stream.try_next().await.unwrap() {
            observed.extend(Package::decode(&batch).unwrap());
        }
        assert_eq!(observed, vec![expected.clone()]);
    }
    store.abandon().await.unwrap();
    drop(store);
    assert_eq!(healthy.reserved(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn singleton_output_namespaces_preserve_logical_state_and_refuse_unowned_reads() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::{ModelError, completed::CompletedView, source::Occurrence};
    use lctx_native::surrealdb::types::{Bytes, RecordId, Variables};
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    ))
    .unwrap();
    let off = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let cold = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let relation = Relation::of::<Package>();
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let mut second = None;
    for store in [&off, &cold] {
        let mut previous = BTreeMap::new();
        for name in ["first", "second"] {
            let id = store
                .begin_contribution(spec(name, &relation))
                .await
                .unwrap();
            store
                .write_batch(
                    &id,
                    &relation,
                    &Package::encode(&[Package { name: name.into() }]).unwrap(),
                    &native_operation_budget,
                )
                .await
                .unwrap();
            previous = store
                .complete_contribution(
                    id,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &previous,
                    &native_operation_budget,
                )
                .await
                .unwrap();
            if name == "second" {
                second = Some(id);
            }
        }
        let view = previous[relation.name()].clone();
        store
            .bind(CompletedBinding {
                boundary: None,
                source: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation,
                    spec("second", &relation).model,
                    &view,
                )
                .unwrap(),
                view,
                configuration: None,
            })
            .await
            .unwrap();
    }
    let second = second.unwrap();
    let expected = off.completed_state(&native_operation_budget).await.unwrap();
    assert_eq!(expected.contributions, 2);
    assert_eq!(expected.memberships, 2);
    assert_eq!(
        cold.completed_state(&native_operation_budget)
            .await
            .unwrap(),
        expected
    );
    let mut stream = cold
        .scan_contribution_batches(second, &relation, &budget, 32)
        .await
        .unwrap();
    let mut actual = Vec::new();
    while let Some(batch) = stream.try_next().await.unwrap() {
        actual.extend(Package::decode(&batch).unwrap());
    }
    drop(stream);
    assert_eq!(
        actual,
        vec![Package {
            name: "second".into()
        }],
        "capture excludes prior cumulative output"
    );
    assert_eq!(
        cold.completed_state(&native_operation_budget)
            .await
            .unwrap(),
        expected,
        "optional cold capture changes no retained state"
    );

    let descriptor = cold.completed_contribution(second).await.unwrap();
    let singleton = CompletedView::new(
        relation.name().into(),
        BTreeSet::from([descriptor.identity().unwrap()]),
        descriptor.outputs[relation.name()].rows,
    )
    .unwrap();
    let mut selected = cold
        .scan_rows(&singleton, &relation, None, None, &budget)
        .await
        .unwrap();
    assert!(
        selected.next().await.unwrap().is_some(),
        "production prepares exact singleton authority before an optional capture"
    );
    assert!(selected.next().await.unwrap().is_none());
    assert!(matches!(
        cold.scan_contribution_batches(
            ContentHash::of(b"unknown-contributor"),
            &relation,
            &budget,
            32
        )
        .await,
        Err(ModelError::Conflict("missing completed contribution"))
    ));
    assert!(matches!(
        cold.scan_contribution_batches(second, &Relation::of::<Occurrence>(), &budget, 32)
            .await,
        Err(ModelError::Conflict("undeclared contribution output"))
    ));

    let admin = fixture_client(&cold, &config).await;
    let saved_descriptor = descriptor.clone();
    let mut unsupported = descriptor;
    unsupported.outputs.get_mut(relation.name()).unwrap().rows += 1;
    let mut bindings = Variables::new();
    bindings.insert("id", RecordId::new("compiler_contribution", second.hex()));
    bindings.insert(
        "descriptor",
        Bytes::from(serde_json::to_vec(&unsupported).unwrap()),
    );
    admin
        .query("UPDATE $id SET descriptor=$descriptor RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        matches!(
            cold.scan_contribution_batches(second, &relation, &budget, 32)
                .await,
            Err(ModelError::Conflict("completed contribution identity"))
        ),
        "a changed output count cannot manufacture a contribution read authority"
    );
    let mut restore = Variables::new();
    restore.insert("id", RecordId::new("compiler_contribution", second.hex()));
    restore.insert(
        "descriptor",
        Bytes::from(serde_json::to_vec(&saved_descriptor).unwrap()),
    );
    admin
        .query("UPDATE $id SET descriptor=$descriptor RETURN NONE")
        .bind(restore)
        .await
        .unwrap()
        .check()
        .unwrap();
    admin.invalidate().await.unwrap();
    off.abandon().await.unwrap();
    cold.abandon().await.unwrap();
}

/// Arbitrary raw relation order is a public ingress contract. Replay lives in native
/// attempt ownership, while path/revision layouts share only their complete byte blob.
#[tokio::test(flavor = "multi_thread")]
async fn missing_source_headers_replay_native_staging_into_shared_and_empty_blobs() {
    use futures::FutureExt;
    use lctx_model::domain::{
        EvidenceBytes, artifact::ArtifactChunk, completion, input::InputRevision,
        source::SourceArtifact,
    };
    use lctx_native::surrealdb::types::RecordId;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"),
    ))
    .unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let scope = store.recovery_scope().unwrap();
    let admin = scope.client().clone();
    let reader = lctx_native::NativeReader::private(admin.clone()).with_budget(&budget);
    let result=std::panic::AssertUnwindSafe(async{
        let mut bytes=vec![0xff;1024*1024];bytes[..32].copy_from_slice(&store.attempt().0);
        let input=InputRevision::from_entries(vec![]).unwrap().id();
        let first=SourceArtifact::from_bytes(input,"first.bin".into(),&bytes).unwrap();
        let second=SourceArtifact::from_bytes(input,"second.bin".into(),&bytes).unwrap();
        let empty=SourceArtifact::from_bytes(input,"empty.bin".into(),b"").unwrap();
        let chunk_relation=Relation::of::<ArtifactChunk>();let source_relation=Relation::of::<SourceArtifact>();
        let mut specification=spec("native-staging-order",&chunk_relation);specification.outputs.insert(source_relation.name().into());
        let contribution=store.begin_contribution(specification).await.unwrap();
        let first_chunk=ArtifactChunk{artifact:first.id(),ordinal:0,body:EvidenceBytes(bytes.clone())};
        store.write_batch(&contribution,&chunk_relation,&ArtifactChunk::encode(std::slice::from_ref(&first_chunk)).unwrap(),&budget).await.unwrap();
        // Exact raw replay earns no second ordinal or byte copy in private staging.
        store.write_batch(&contribution,&chunk_relation,&ArtifactChunk::encode(std::slice::from_ref(&first_chunk)).unwrap(),&budget).await.unwrap();
        let mut response=admin.query("SELECT VALUE id FROM native_pending_original WITH INDEX pending_attempt WHERE attempt=$attempt").bind(("attempt",RecordId::new("native_attempt",store.attempt().hex()))).await.unwrap().check().unwrap();
        let roots:Vec<RecordId>=response.take(0).unwrap();assert_eq!(roots.len(),1,"one attempt/source replay root before any header");
        store.write_batch(&contribution,&source_relation,&SourceArtifact::encode(&[first.clone(),second.clone(),empty.clone()]).unwrap(),&budget).await.unwrap();
        let second_chunk=ArtifactChunk{artifact:second.id(),ordinal:0,body:EvidenceBytes(bytes.clone())};
        store.write_batch(&contribution,&chunk_relation,&ArtifactChunk::encode(&[second_chunk]).unwrap(),&budget).await.unwrap();
        // A new local preparation object can replay the durable staging without a host index.
        lctx_native::control::with_recovery_scope(scope.clone(),
            lctx_native::originals::complete_pending(&reader,store.attempt(),&budget)).await.unwrap();
        store.complete_contribution(contribution,ProviderOutcome::Complete,&[chunk_relation,source_relation],&BTreeMap::new(),&budget).await.unwrap();
        let a=lctx_native::originals::Layout::source(&first).unwrap();let b=lctx_native::originals::Layout::source(&second).unwrap();
        assert_ne!(a.id,b.id,"logical path identity is retained");assert_eq!(a.blob.id(),b.blob.id(),"only full exact byte blobs are shared");
        let page=reader.original_bytes(lctx_model::domain::graph::EntityId::of(first.id()),65534,4).await.unwrap();assert_eq!(page.as_ref(),&bytes[65534..65538]);
        assert!(reader.original_bytes(lctx_model::domain::graph::EntityId::of(empty.id()),0,0).await.unwrap().is_empty(),"empty content earns explicit completion");
        let mut response=admin.query("SELECT count() AS n FROM original_chunk WITH INDEX position WHERE blob=$blob GROUP ALL; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND object IN $roots").bind(("blob",a.blob.id())).bind(("attempt",RecordId::new("native_attempt",store.attempt().hex()))).bind(("roots",roots.clone())).await.unwrap().check().unwrap();
        let counts:Vec<serde_json::Value>=response.take(0).unwrap();let held:Vec<RecordId>=response.take(1).unwrap();
        assert_eq!(counts,vec![serde_json::json!({"n":16})],"two logical artifacts retain one shared physical body");assert_eq!(held.len(),1,"staging remains owned until terminal cleanup acknowledges nomination");
    }).catch_unwind().await;
    let mut cleanup = completion::Completion::default();
    cleanup.step("staging reader close", reader.close().await);
    cleanup.step("staging attempt cleanup", store.abandon().await);
    // admin is an Arc borrow of the store's actual recovery client, not an
    // independently connected observer. store.abandon owns its session closure.
    let cleanup = completion::complete(Ok(()), cleanup);
    if let Err(primary) = result {
        if let Err(secondary) = cleanup {
            eprintln!("staging control cleanup failed: {secondary:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}

/// Shared dependency metadata never stands in for each physical producer's edge.
/// The later logical contributor is deliberate: deleting the first edge would not
/// reveal a cache shortcut that validates only the first incoming dependency.
#[tokio::test(flavor = "multi_thread")]
async fn shared_dependency_view_requires_each_selected_contributors_exact_hold() {
    use futures::FutureExt;
    use lctx_model::domain::{
        analysis::sources::SourceSnapshot,
        completion::{Completion, complete},
    };
    use lctx_native::surrealdb::types::{Object, RecordId, Value};
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned format-7 native fixture"),
    ))
    .unwrap();
    let source = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&source, &config).await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let nonce = lctx_native::control::fresh_identity("shared-dependency-edge-control").unwrap();
    let mut restore: Option<(Object, Object, Object)> = None;
    let audit = lctx_native::installed_view::InstallationAudit::new(admin.clone(), &budget)
        .await
        .unwrap();
    let mut independent_audit = None;
    let outcome=std::panic::AssertUnwindSafe(async {
        let relation=Relation::of::<Package>();
        let leaf=source.begin_contribution(spec(&format!("leaf-{}",nonce.hex()),&relation)).await.unwrap();
        source.write_batch(&leaf,&relation,&Package::encode(&[Package{name:format!("leaf-{}",nonce.hex())}]).unwrap(),&budget).await.unwrap();
        let leaf_views=source.complete_contribution(leaf,ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new(),&budget).await.unwrap();
        let dependency=leaf_views[relation.name()].clone();
        let input=SourceSnapshot::of_completed_view(&relation,spec("unused",&relation).model,&dependency).unwrap();
        let mut previous=BTreeMap::new();
        for name in ["left","right"] {
            let mut declared=spec(&format!("{name}-{}",nonce.hex()),&relation);declared.inputs.push(input.clone());
            let physical=source.begin_contribution(declared).await.unwrap();
            previous=source.complete_contribution(physical,ProviderOutcome::Complete,std::slice::from_ref(&relation),&previous,&budget).await.unwrap();
        }
        let view=previous[relation.name()].clone();assert_eq!(view.contributions.len(),2);assert_eq!(view.rows,0);
        let held=source.installed_view(&view,&budget).await.unwrap();let sealed=held.completion().clone();
        let original=lctx_native::installed_view::select_live_view_scoped(admin.clone(),source.attempt(),&view,&sealed.mappings,&budget,&audit).await.unwrap();
        assert_eq!(original.completion(),&sealed,"reselection keeps the exact validated logical/physical selector");
        let last=*view.contributions.iter().next_back().unwrap();
        let selected=sealed.mappings.iter().find(|mapping|mapping.view==view.identity && mapping.logical==last).unwrap();
        let contributor=RecordId::new("compiler_contribution",selected.physical.hex());let dependency=RecordId::new("compiler_view",dependency.identity.hex());
        let mut response=admin.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object")
            .bind(("owner",contributor.clone())).bind(("object",dependency.clone())).await.unwrap().check().unwrap();
        let edges:Vec<Object>=response.take(0).unwrap();let [edge]=edges.as_slice()else{panic!("one exact shared dependency edge")};
        let guarded=|object:&RecordId|RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex());
        let mut response=admin.query("SELECT * FROM $owner_guard; SELECT * FROM $object_guard;")
            .bind(("owner_guard",guarded(&contributor))).bind(("object_guard",guarded(&dependency))).await.unwrap().check().unwrap();
        let owner_guards:Vec<Object>=response.take(0).unwrap();let object_guards:Vec<Object>=response.take(1).unwrap();
        assert_eq!(owner_guards.len(),1);assert_eq!(object_guards.len(),1);
        restore=Some((edge.clone(),owner_guards[0].clone(),object_guards[0].clone()));
        let (edge,owner_guard,object_guard)=restore.as_ref().unwrap();
        admin.query("BEGIN; IF (SELECT * FROM ONLY $edge.id)!=$edge OR (SELECT * FROM ONLY $owner_guard.id)!=$owner_guard OR (SELECT * FROM ONLY $object_guard.id)!=$object_guard { THROW 'owned dependency control observation changed'; }; fn::native_incidence($edge); DELETE $edge.id RETURN NONE; COMMIT;")
            .bind(("edge",edge.clone())).bind(("owner_guard",owner_guard.clone())).bind(("object_guard",object_guard.clone())).await.unwrap().check().unwrap();
        // Privileged corruption bypasses the pinned immutable-owner protocol. A
        // fresh independent audit must inspect every selected owner's exact edge.
        independent_audit=Some(lctx_native::installed_view::InstallationAudit::new(admin.clone(),&budget).await.unwrap());
        let fresh=independent_audit.as_ref().unwrap();
        let rejected=lctx_native::installed_view::select_live_view_scoped(admin.clone(),source.attempt(),&view,&sealed.mappings,&budget,fresh).await;
        // Restore the exact owned incidence before observing the expected refusal.
        restore_dependency_edge(&admin,restore.as_ref().unwrap()).await.unwrap();restore=None;
        assert!(matches!(rejected.as_ref().err().and_then(ModelError::primary),Some(ModelError::Conflict("native required hold missing"))),"the second selected physical contributor must independently retain its input: {:?}",rejected.as_ref().err());
        let restored=lctx_native::installed_view::select_live_view_scoped(admin.clone(),source.attempt(),&view,&sealed.mappings,&budget,fresh).await.unwrap();
        assert_eq!(restored.completion(),&sealed,"dependency repair restores the same exact selector without a per-view seal");
    }).catch_unwind().await;
    let mut completion = Completion::default();
    if let Some(edge) = restore.as_ref() {
        completion.step(
            "owned dependency edge restoration",
            restore_dependency_edge(&admin, edge).await,
        );
    }
    if let Some(fresh) = independent_audit.as_ref() {
        completion.step(
            "independent corrupted dependency audit close",
            fresh.close().await,
        );
    }
    completion.step("shared dependency audit close", audit.close().await);
    completion.step(
        "shared dependency control source close",
        source.abandon().await,
    );
    completion.step(
        "shared dependency control observer close",
        admin.invalidate().await.map_err(ModelError::codec),
    );
    let cleanup = complete(Ok(()), completion);
    if let Err(primary) = outcome {
        if let Err(error) = cleanup {
            eprintln!("dependency control cleanup: {error:#?}");
        }
        std::panic::resume_unwind(primary);
    }
    cleanup.unwrap();
}
async fn restore_dependency_edge(
    client: &std::sync::Arc<
        lctx_native::surrealdb::Surreal<lctx_native::surrealdb::engine::remote::grpc::Client>,
    >,
    original: &(
        lctx_native::surrealdb::types::Object,
        lctx_native::surrealdb::types::Object,
        lctx_native::surrealdb::types::Object,
    ),
) -> Result<(), ModelError> {
    let (edge, owner_guard, object_guard) = original;
    // Fresh audit pins legitimately touch guard revisions. Restoration preserves
    // exact incidence and endpoint incarnation/retirement state, without treating
    // another incoming protection as endpoint replacement.
    client.query("BEGIN; LET $owner_now=SELECT * FROM ONLY $owner_guard.id; LET $object_now=SELECT * FROM ONLY $object_guard.id; IF $owner_now.phase!='active' OR $object_now.phase!='active' OR $owner_now.incarnation!=$owner_guard.incarnation OR $object_now.incarnation!=$object_guard.incarnation OR $owner_now.retired!=$owner_guard.retired OR $object_now.retired!=$object_guard.retired OR $owner_now.retiring_item!=$owner_guard.retiring_item OR $object_now.retiring_item!=$object_guard.retiring_item OR $owner_now.tombstone_revision!=$owner_guard.tombstone_revision OR $object_now.tombstone_revision!=$object_guard.tombstone_revision { THROW 'owned dependency restoration endpoints changed'; }; LET $actual=SELECT * FROM ONLY $edge.id; IF $actual!=NONE AND $actual!=$edge { THROW 'owned dependency restoration incidence changed'; }; IF $actual=NONE { fn::native_incidence($edge); CREATE $edge.id CONTENT $edge RETURN NONE; }; COMMIT;")
        .bind(("edge",edge.clone())).bind(("owner_guard",owner_guard.clone())).bind(("object_guard",object_guard.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    Ok(())
}

/// Overlapping roots reuse a completed contributor only while a real native pin
/// retains its immutable namespace. Distinct logical ranges still earn validation.
#[tokio::test(flavor = "multi_thread")]
async fn held_installation_scope_reuses_whole_blob_and_contributor_across_roots() {
    use futures::FutureExt;
    use lctx_model::domain::{
        EvidenceBytes,
        artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunk},
        completion,
        input::{InputRevision, Release},
        source::SourceArtifact,
    };
    use lctx_native::installed_view::{InstallationAudit, select_live_view_scoped};
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
    ))
    .unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        lctx_model::domain::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&store, &config).await;
    let budget = ResourceBudget::fixed(768 << 20).unwrap();
    let audit = InstallationAudit::new(admin.clone(), &budget)
        .await
        .unwrap();
    let result=std::panic::AssertUnwindSafe(async{
        let packages=Relation::of::<Package>();let releases=Relation::of::<Release>();let sources=Relation::of::<SourceArtifact>();let chunks=Relation::of::<ArtifactChunk>();
        let relations=[packages.clone(),releases.clone(),sources.clone(),chunks.clone()];let mut specification=spec("held-installation-scope",&packages);
        specification.outputs.extend(relations.iter().map(|relation|relation.name().to_owned()));
        let contribution=store.begin_contribution(specification).await.unwrap();
        let mut package_rows=(0..257).map(|index|Package{name:format!("scope-package-{index}")}).collect::<Vec<_>>();
        package_rows.push(Package{name:"a".repeat(64<<10)});
        store.write_batch(&contribution,&packages,&Package::encode(&package_rows).unwrap(),&budget).await.unwrap();
        let release_rows=package_rows[..257].iter().map(|package|Release{package:package.id(),version:"1".into()}).collect::<Vec<_>>();
        store.write_batch(&contribution,&releases,&Release::encode(&release_rows).unwrap(),&budget).await.unwrap();
        let mut bytes=vec![0xff;ARTIFACT_CHUNK_BYTES*2+7];bytes[..32].copy_from_slice(&store.attempt().0);
        let input=InputRevision::from_entries(vec![]).unwrap().id();
        let source_rows=[SourceArtifact::from_bytes(input,"one.bin".into(),&bytes).unwrap(),SourceArtifact::from_bytes(input,"two.bin".into(),&bytes).unwrap()];
        store.write_batch(&contribution,&sources,&SourceArtifact::encode(&source_rows).unwrap(),&budget).await.unwrap();
        let chunk_rows=source_rows.iter().flat_map(|source|bytes.chunks(ARTIFACT_CHUNK_BYTES).enumerate().map(move |(ordinal,body)|ArtifactChunk{artifact:source.id(),ordinal:ordinal as i64,body:EvidenceBytes(body.to_vec())})).collect::<Vec<_>>();
        store.write_batch(&contribution,&chunks,&ArtifactChunk::encode(&chunk_rows).unwrap(),&budget).await.unwrap();
        let views=store.complete_contribution(contribution,ProviderOutcome::Complete,&relations,&BTreeMap::new(),&budget).await.unwrap();
        for relation in &relations{
            let view=&views[relation.name()];let baseline=store.installed_view(view,&budget).await.unwrap();
            let mappings=baseline.completion().mappings.clone();
            let checked=select_live_view_scoped(admin.clone(),store.attempt(),view,&mappings,&budget,&audit).await.unwrap();
            assert_eq!(checked.completion(),baseline.completion());
            assert_eq!(audit.observations(),(1,1),"all six logical ranges share one full blob observation, and later roots reuse exactly the retained contributor");
            drop(checked);
        }
        audit.close().await.unwrap();
        let view=&views[packages.name()];let baseline=store.installed_view(view,&budget).await.unwrap();
        assert!(select_live_view_scoped(admin.clone(),store.attempt(),view,&baseline.completion().mappings,&budget,&audit).await.is_err(),"a released pin never retains reusable authority");
    }).catch_unwind().await;
    let mut cleanup = completion::Completion::default();
    cleanup.step("installation observation pin release", audit.close().await);
    cleanup.step(
        "installation observation attempt cleanup",
        store.abandon().await,
    );
    cleanup.step(
        "installation observer invalidation",
        admin.invalidate().await.map_err(ModelError::codec),
    );
    let cleaned = completion::complete(Ok(()), cleanup);
    if let Err(panic) = result {
        if let Err(error) = cleaned {
            eprintln!("installation observation cleanup: {error:?}");
        }
        std::panic::resume_unwind(panic);
    }
    cleaned.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_transport_distinct_empty_copies_keep_root_qualified_dependency_choices() {
    use lctx_model::domain::{
        self as d, analytics::QualityStep, completed_transport::CompletedTransportEnvelope,
    };
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("owned schema8 validation runtime"),
    ))
    .unwrap();
    let source = NativeCompilerStore::begin(
        &config,
        d::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let target = NativeCompilerStore::begin(
        &config,
        d::admission::Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let admin = fixture_client(&target, &config).await;
    let result=async {
        let package=Relation::of::<Package>();let release=Relation::of::<d::input::Release>();let quality=Relation::of::<QualityStep>();
        let mut base=spec(&format!("copy-base-{}",source.attempt().hex()),&package);base.model=d::model()?.digest();
        let physical=source.begin_contribution(base.clone()).await?;
        let base_views=source.complete_contribution(physical,ProviderOutcome::Complete,std::slice::from_ref(&package),&BTreeMap::new(),&budget).await?;
        let base_view=base_views[package.name()].clone();
        let input=d::analysis::sources::SourceSnapshot::of_completed_view(&package,base.model,&base_view)?;
        let mut roots=vec![base_view.clone()];
        for relation in [&release,&quality] {
            let mut dependent=spec(&format!("copy-dependent-{}-{}",source.attempt().hex(),relation.name()),relation);dependent.model=base.model;dependent.inputs=vec![input.clone()];
            let owner=source.begin_contribution(dependent).await?;
            let views=source.complete_contribution(owner,ProviderOutcome::Complete,std::slice::from_ref(relation),&BTreeMap::new(),&budget).await?;
            roots.push(views[relation.name()].clone());
        }
        for view in &roots {
            let relation=if view.relation==package.name(){&package}else if view.relation==release.name(){&release}else{&quality};
            source.bind(CompletedBinding{boundary:None,source:d::analysis::sources::SourceSnapshot::of_completed_view(relation,base.model,view)?,view:view.clone(),configuration:None}).await?;
        }
        source.freeze_content(&budget).await?;
        let file=tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        let original=source.export_state(file.path(),&budget).await?;
        let mut frames=std::fs::read_to_string(file.path()).map_err(ModelError::codec)?.lines().map(serde_json::from_str::<serde_json::Value>).collect::<Result<Vec<_>,_>>().map_err(ModelError::codec)?;
        let mut envelope:CompletedTransportEnvelope=serde_json::from_value(frames[0]["header"]["envelope"].clone()).map_err(ModelError::codec)?;
        let base_logical=*base_view.contributions.iter().next().ok_or(ModelError::Schema("base logical"))?;
        let base_label=envelope.copies.iter().find(|copy|copy.descriptor.identity().ok()==Some(base_logical)).ok_or(ModelError::Schema("base transport copy"))?.label;
        let mut duplicate=envelope.copies[base_label as usize].clone();duplicate.label=envelope.copies.len() as u64;let duplicate_label=duplicate.label;envelope.copies.push(duplicate);
        let root=envelope.roots.iter_mut().find(|root|root.view.relation==quality.name()).ok_or(ModelError::Schema("qualified dependency root"))?;
        root.slots.iter_mut().find(|slot|slot.logical==base_logical).ok_or(ModelError::Schema("qualified dependency slot"))?.copy=duplicate_label;
        // Deterministic labels reflect first use; adding a physical copy never changes v4.
        let old=envelope.copies;let mut labels=BTreeMap::new();let mut copies=vec![];
        for root in &mut envelope.roots {for slot in &mut root.slots {
            let next=labels.len() as u64;let label=*labels.entry(slot.copy).or_insert(next);
            if label==copies.len() as u64 {let mut copy=old[slot.copy as usize].clone();copy.label=label;copies.push(copy);}
            slot.copy=label;
        }}
        envelope.copies=copies;let expected=envelope.receipt()?;
        assert_eq!(original.semantic,expected.semantic);assert_eq!(original.eligibility,expected.eligibility);assert_ne!(original.envelope,expected.envelope);
        assert!(envelope.copies.iter().all(|copy|copy.membership.rows==0&&copy.aliases.rows==0));
        frames[0]["header"]["envelope"]=serde_json::to_value(&envelope).map_err(ModelError::codec)?;
        let last=frames.len()-1;frames[last]["end"]["receipt"]=serde_json::to_value(&expected).map_err(ModelError::codec)?;
        let encoded=frames.iter().map(|frame|serde_json::to_string(frame).map(|line|line+"\n")).collect::<Result<String,_>>().map_err(ModelError::codec)?;
        std::fs::write(file.path(),encoded).map_err(ModelError::codec)?;
        target.import_state(file.path(),&expected,&budget).await?;
        let release_view=roots.iter().find(|view|view.relation==release.name()).unwrap();let quality_view=roots.iter().find(|view|view.relation==quality.name()).unwrap();
        let one=target.installed_view(release_view,&budget).await?;let two=target.installed_view(quality_view,&budget).await?;
        let selected=|held:&lctx_native::installed_view::HeldInstalledView|held.completion().mappings.iter().find(|mapping|mapping.view==base_view.identity&&mapping.logical==base_logical).map(|mapping|mapping.physical).unwrap();
        assert_ne!(selected(&one),selected(&two),"root-qualified dependency copies must remain distinct");
        assert_ne!(selected(&one),physical);assert_ne!(selected(&two),physical);
        let mut response=admin.query("SELECT id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt ORDER BY id").bind(("attempt",lctx_native::surrealdb::types::RecordId::new("native_attempt",target.attempt().hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let actual:Vec<lctx_native::surrealdb::types::Value>=response.take(0).map_err(ModelError::codec)?;assert_eq!(actual.len(),4,"one fresh owner per declared copy label");
        target.freeze_content(&budget).await?;let roundtrip=tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        assert_eq!(target.export_state(roundtrip.path(),&budget).await?,expected,"fresh physical owners reproduce the exact portable correspondence");
        Ok::<_,ModelError>(())
    }.await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("distinct-copy target close", target.abandon().await);
    terminal.step("distinct-copy source close", source.abandon().await);
    terminal.step(
        "distinct-copy observer close",
        admin.invalidate().await.map_err(ModelError::codec),
    );
    d::completion::complete(result, terminal).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_transport_permuted_copy_alias_chain_keeps_local_absence() {
    use lctx_model::domain::{
        self as d, Key, KeySink, graph::Entity, normalized::entities::EntityRef, value::Place,
    };
    use lctx_native::surrealdb::types::{Object, RecordId, Value};
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("owned schema8 validation runtime"),
    ))
    .unwrap();
    let source = NativeCompilerStore::begin(
        &config,
        d::admission::Frontier::Normalized,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let target = NativeCompilerStore::begin(
        &config,
        d::admission::Frontier::Normalized,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    let source_admin = fixture_client(&source, &config).await;
    let target_admin = fixture_client(&target, &config).await;
    let result = async {
        let package_relation = Relation::of::<Package>();
        let place_relation = Relation::of::<Place>();
        let endpoint_relation = Relation::of::<EntityRef>();
        let nonce = source.attempt();
        let package = Package {
            name: format!("alias-chain-{}", nonce.hex()),
        };
        let mut package_spec = spec(
            &format!("alias-chain-base-{}", nonce.hex()),
            &package_relation,
        );
        package_spec.model = d::model()?.digest();
        package_spec.outputs.insert(endpoint_relation.name().into());
        let package_owner = source.begin_contribution(package_spec.clone()).await?;
        source
            .write_batch(
                &package_owner,
                &package_relation,
                &Package::encode(std::slice::from_ref(&package))?,
                &budget,
            )
            .await?;
        let place = Place {
            root: serde_json::from_value(
                serde_json::to_value(
                    <[u8; 16]>::try_from(&nonce.0[..16]).map_err(ModelError::codec)?,
                )
                .map_err(ModelError::codec)?,
            )
            .map_err(ModelError::codec)?,
            path: serde_json::from_value(
                serde_json::to_value(
                    <[u8; 16]>::try_from(&nonce.0[16..]).map_err(ModelError::codec)?,
                )
                .map_err(ModelError::codec)?,
            )
            .map_err(ModelError::codec)?,
        };
        let endpoint = EntityRef::Place { place: place.id() };
        let mut probe_spec = spec(
            &format!("alias-chain-probe-{}", nonce.hex()),
            &place_relation,
        );
        probe_spec.model = package_spec.model;
        let probe_owner = source.begin_contribution(probe_spec).await?;
        source
            .write_batch(
                &probe_owner,
                &place_relation,
                &Place::encode(std::slice::from_ref(&place))?,
                &budget,
            )
            .await?;
        source
            .complete_contribution(
                probe_owner,
                ProviderOutcome::Complete,
                std::slice::from_ref(&place_relation),
                &BTreeMap::new(),
                &budget,
            )
            .await?;
        let a = lctx_native::loader::entity_payload_id(&Entity::from(place.clone()))?;
        let b = lctx_native::loader::entity_payload_id(&Entity::from(endpoint.clone()))?;
        let c = lctx_native::loader::entity_payload_id(&Entity::from(package.clone()))?;
        let alias = |source: &RecordId, target: &RecordId| -> Result<Value, ModelError> {
            let mut sink = KeySink::new("native-place-alias/v1");
            sink.part(
                b"source",
                &serde_json::to_vec(source).map_err(ModelError::codec)?,
            );
            sink.part(
                b"target",
                &serde_json::to_vec(target).map_err(ModelError::codec)?,
            );
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
            row.insert("source", source.clone());
            row.insert("target", target.clone());
            Ok(Value::Object(row))
        };
        // Completion independently earns dependency selectors as well as the
        // composed root. Give this dependency copy B as an explicit membership,
        // so its owned B→C edge is reachable from its own complete output seeds.
        // The Place copy independently owns only A→B; neither copy gets the
        // other's alias merely because the composed root can reach both.
        source
            .write_batch(
                &package_owner,
                &endpoint_relation,
                &<EntityRef as d::Record>::encode(std::slice::from_ref(&endpoint))?,
                &budget,
            )
            .await?;
        let first = alias(&a, &b)?;
        let later = alias(&b, &c)?;
        let Some(Value::RecordId(first_id)) = first.as_object().and_then(|row| row.get("id"))
        else {
            return Err(ModelError::Schema("first chain alias id"));
        };
        let Some(Value::RecordId(later_id)) = later.as_object().and_then(|row| row.get("id"))
        else {
            return Err(ModelError::Schema("later chain alias id"));
        };
        lctx_native::control::ensure_rows(
            &source_admin,
            Some(source.attempt()),
            vec![later.clone()],
        )
        .await?;
        lctx_native::control::hold_many(
            &source_admin,
            Some(source.attempt()),
            vec![
                (
                    RecordId::new("compiler_contribution", package_owner.hex()),
                    later_id.clone(),
                ),
                (
                    RecordId::new("compiler_contribution", package_owner.hex()),
                    c.clone(),
                ),
                (later_id.clone(), b.clone()),
            ],
        )
        .await?;
        // Establish the package copy's entire alias incidence before completion.
        // Its target and the chain's source already have canonical backing from
        // ordinary producing writes; no completed contributor is mutated.
        let package_views = source
            .complete_contribution(
                package_owner,
                ProviderOutcome::Complete,
                &[package_relation.clone(), endpoint_relation.clone()],
                &BTreeMap::new(),
                &budget,
            )
            .await?;
        let package_view = package_views[package_relation.name()].clone();
        let mut place_spec = spec(
            &format!("alias-chain-root-{}", nonce.hex()),
            &place_relation,
        );
        place_spec.model = package_spec.model;
        place_spec.inputs = vec![d::analysis::sources::SourceSnapshot::of_completed_view(
            &package_relation,
            package_spec.model,
            &package_view,
        )?];
        let mut descriptor = source.completed_contribution(probe_owner).await?;
        let mut iteration = 0u64;
        let place_view = loop {
            let mut sink = KeySink::new("alias-chain-order-control/v1");
            nonce.encode(&mut sink);
            sink.part(b"iteration", &iteration.to_le_bytes());
            place_spec.implementation = sink.finish();
            descriptor.spec = place_spec.clone();
            let candidate = d::completed::CompletedView::new(
                place_relation.name().into(),
                [descriptor.identity()?].into(),
                1,
            )?;
            if candidate.identity > package_view.identity {
                break candidate;
            }
            iteration += 1;
        };
        let place_owner = source.begin_contribution(place_spec.clone()).await?;
        source
            .write_batch(
                &place_owner,
                &place_relation,
                &Place::encode(std::slice::from_ref(&place))?,
                &budget,
            )
            .await?;
        let actual = source
            .complete_contribution(
                place_owner,
                ProviderOutcome::Complete,
                std::slice::from_ref(&place_relation),
                &BTreeMap::new(),
                &budget,
            )
            .await?;
        assert_eq!(actual[place_relation.name()], place_view);
        source
            .bind(CompletedBinding {
                boundary: None,
                source: d::analysis::sources::SourceSnapshot::of_completed_view(
                    &place_relation,
                    place_spec.model,
                    &place_view,
                )?,
                view: place_view.clone(),
                configuration: None,
            })
            .await?;
        source.freeze_content(&budget).await?;
        let file = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        let expected = source.export_state(file.path(), &budget).await?;
        let decoded =
            lctx_native::completed_transport::read_transport(file.path(), &expected, &budget)?;
        assert_eq!(decoded.copies().len(), 2);
        assert_eq!(decoded.copies()[0].physical, ContentHash([0; 32]));
        assert_eq!(
            decoded.envelope().copies[0].descriptor.outputs[endpoint_relation.name()].rows,
            1,
            "dependency reachability is earned from explicit B membership"
        );
        assert_eq!(
            decoded.envelope().copies[0].membership.rows,
            2,
            "dependency copy retains B and C, including the independent-selector premise"
        );
        assert_eq!(
            decoded.envelope().copies[1].membership.rows,
            1,
            "Place copy retains only its A membership"
        );
        let mut earlier = decoded.copies()[0].aliases.cursor_with_budget(&budget)?;
        assert_eq!(earlier.next_row()?.as_ref(), Some(&later));
        assert!(earlier.next_row()?.is_none());
        let mut following = decoded.copies()[1].aliases.cursor_with_budget(&budget)?;
        assert_eq!(following.next_row()?.as_ref(), Some(&first));
        assert!(following.next_row()?.is_none());
        target
            .import_entities(&[
                Entity::from(place),
                Entity::from(endpoint),
                Entity::from(package),
            ])
            .await?;
        target.import_state(file.path(), &expected, &budget).await?;
        let selected = target.installed_view(&place_view, &budget).await?;
        let logical_package = *package_view.contributions.iter().next().unwrap();
        let logical_place = *place_view.contributions.iter().next().unwrap();
        let owner = |logical| {
            selected
                .completion()
                .mappings
                .iter()
                .find(|mapping| mapping.logical == logical)
                .map(|mapping| RecordId::new("compiler_contribution", mapping.physical.hex()))
                .unwrap()
        };
        let package_copy = owner(logical_package);
        let place_copy = owner(logical_place);
        let reader = lctx_native::NativeReader::private(target_admin.clone()).with_budget(&budget);
        let exact = lctx_native::control::inspect_hold_window_read(
            &reader,
            &[
                (package_copy.clone(), later_id.clone()),
                (package_copy, first_id.clone()),
                (place_copy.clone(), first_id.clone()),
                (place_copy, later_id.clone()),
            ],
            &budget,
        )
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "alias chain exact absence reader close",
            reader.close().await,
        );
        let exact = d::completion::complete(exact, terminal)?;
        let package_copy = owner(logical_package);
        let place_copy = owner(logical_place);
        assert!(exact.contains(&package_copy, later_id)?);
        assert!(!exact.contains(&package_copy, first_id)?);
        assert!(exact.contains(&place_copy, first_id)?);
        assert!(!exact.contains(&place_copy, later_id)?);
        target.freeze_content(&budget).await?;
        let roundtrip = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        assert_eq!(
            target.export_state(roundtrip.path(), &budget).await?,
            expected
        );
        Ok::<_, ModelError>(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("alias chain target close", target.abandon().await);
    terminal.step("alias chain source close", source.abandon().await);
    terminal.step(
        "alias chain target observer close",
        target_admin.invalidate().await.map_err(ModelError::codec),
    );
    terminal.step(
        "alias chain source observer close",
        source_admin.invalidate().await.map_err(ModelError::codec),
    );
    d::completion::complete(result, terminal).unwrap();
}

/// Family3 transports nongraph envelopes only. Graph members remain actual shared
/// prerequisites and must earn independent receiving byte/incarnation protection.
#[tokio::test(flavor = "multi_thread")]
async fn cold_transport_mixed_graph_and_nongraph_retains_exact_imported_sources() {
    use lctx_model::domain::{self as d, analytics::QualityStep, input::{InputRevision, CorpusLibrary}};
    assert_eq!(lctx_native::adapter::select(InputRevision::NAME).unwrap().table,
        lctx_native::schema::ScopeTable::Entity, "control must include an actual graph member");
    assert_eq!(lctx_native::adapter::select(QualityStep::NAME).unwrap().table,
        lctx_native::schema::ScopeTable::CompilerRecord, "control must also include nongraph block content");
    assert_eq!(lctx_native::adapter::select(CorpusLibrary::NAME).unwrap().table,
        lctx_native::schema::ScopeTable::Assertion, "multi-token member must be an actual graph assertion");
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"))).unwrap();
    let factory = lctx_compiler_store::compiler::recovery_factory(&config, budget.clone()).unwrap();
    let source = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts, &factory).await.unwrap();
    let target = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts, &factory).await.unwrap();
    let mut source_abandoned = false;
    let result = async {
        let graph = InputRevision::from_entries(vec![])?;
        let multiple = CorpusLibrary { corpus: graph.id(), library: graph.id() };
        let nongraph = QualityStep {
            run: serde_json::from_value(serde_json::to_value(&source.attempt().0[..16]).map_err(ModelError::codec)?)
                .map_err(ModelError::codec)?, ordinal: 0, value: d::FiniteF64::new(0.25)?,
        };
        let graph_relation = Relation::of::<InputRevision>();
        let multiple_relation = Relation::of::<CorpusLibrary>();
        let nongraph_relation = Relation::of::<QualityStep>();
        let relations = [graph_relation.clone(), nongraph_relation.clone(), multiple_relation.clone()];
        let mut declared = spec(&format!("mixed-cold-{}", source.attempt().hex()), &graph_relation);
        declared.model = d::model()?.digest();
        declared.outputs.insert(nongraph_relation.name().into());
        declared.outputs.insert(multiple_relation.name().into());
        let model = declared.model;
        let owner = source.begin_contribution(declared).await?;
        source.write_batch(&owner, &graph_relation, &InputRevision::encode(std::slice::from_ref(&graph))?, &budget).await?;
        source.write_batch(&owner, &nongraph_relation, &QualityStep::encode(std::slice::from_ref(&nongraph))?, &budget).await?;
        source.write_batch(&owner, &multiple_relation, &CorpusLibrary::encode(std::slice::from_ref(&multiple))?, &budget).await?;
        // Independently inspect every source membership before trusting producer
        // descriptors. Empty scopes and multi-token fanout share this one contributor.
        let client = source.recovery_scope()?.client().clone();
        let mut response = client.query("SELECT relation,scope_keys FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner ORDER BY relation,semantic_key")
            .bind(("owner", lctx_native::surrealdb::types::RecordId::new("compiler_contribution", owner.hex())))
            .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let members: Vec<lctx_native::surrealdb::types::Value> = response.take(0).map_err(ModelError::codec)?;
        let mut source_counts = BTreeMap::new();
        for row in members {
            let object = row.as_object().ok_or(ModelError::Schema("mixed source membership"))?;
            let Some(lctx_native::surrealdb::types::Value::String(relation)) = object.get("relation") else { return Err(ModelError::Schema("mixed source relation")); };
            let Some(lctx_native::surrealdb::types::Value::Array(scopes)) = object.get("scope_keys") else { return Err(ModelError::Schema("mixed source scope tokens")); };
            if source_counts.insert(relation.clone(), scopes.len()).is_some() {
                return Err(ModelError::Conflict("mixed source duplicate membership"));
            }
        }
        let expected_counts = BTreeMap::from([(InputRevision::NAME.to_owned(), 0usize),
            (CorpusLibrary::NAME.to_owned(), 2usize), (QualityStep::NAME.to_owned(), 1usize)]);
        if source_counts != expected_counts { return Err(ModelError::Conflict("mixed source complete membership scope cardinality")); }
        let views = source.complete_contribution(owner, ProviderOutcome::Complete, &relations, &BTreeMap::new(), &budget).await?;
        for relation in &relations {
            let view = views[relation.name()].clone();
            source.bind(CompletedBinding { boundary: None,
                source: d::analysis::sources::SourceSnapshot::of_completed_view(relation, model, &view)?,
                view, configuration: None }).await?;
        }
        source.freeze_content(&budget).await?;
        let file = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        let expected = source.export_state(file.path(), &budget).await?;
        let mut nongraph_frames = 0usize;
        for line in std::fs::read_to_string(file.path()).map_err(ModelError::codec)?.lines() {
            let frame: serde_json::Value = serde_json::from_str(line).map_err(ModelError::codec)?;
            if frame.get("row").and_then(|row| row.get("table")).and_then(|table| table.as_str()) == Some("compiler_record") {
                let row: lctx_native::surrealdb::types::Value = serde_json::from_value(frame["row"]["row"].clone()).map_err(ModelError::codec)?;
                if row.as_object().and_then(|row| row.get("semantic_type")) != Some(&lctx_native::surrealdb::types::Value::String(QualityStep::NAME.into())) {
                    return Err(ModelError::Conflict("mixed transport family3 must be nongraph only"));
                }
                nongraph_frames += 1;
            }
        }
        target.import_state(file.path(), &expected, &budget).await?;
        // Receiving protection must survive release of the original producer's owner.
        source.abandon().await?; source_abandoned = true;
        let graph_batches = target.scan_batches(&views[graph_relation.name()], &graph_relation, None, None, &budget, 8)
            .await?.try_collect::<Vec<_>>().await.map_err(ModelError::codec)?;
        let graph_rows = graph_batches.iter().map(InputRevision::decode).collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        let nongraph_batches = target.scan_batches(&views[nongraph_relation.name()], &nongraph_relation, None, None, &budget, 8)
            .await?.try_collect::<Vec<_>>().await.map_err(ModelError::codec)?;
        let nongraph_rows = nongraph_batches.iter().map(QualityStep::decode).collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        let multiple_batches = target.scan_batches(&views[multiple_relation.name()], &multiple_relation, None, None, &budget, 8)
            .await?.try_collect::<Vec<_>>().await.map_err(ModelError::codec)?;
        let multiple_rows = multiple_batches.iter().map(CorpusLibrary::decode).collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        target.verify_state(&budget).await?;
        target.freeze_content(&budget).await?;
        let roundtrip = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
        let actual = target.export_state(roundtrip.path(), &budget).await?;
        Ok::<_, ModelError>((graph, nongraph, graph_rows, nongraph_rows, expected, actual, nongraph_frames, multiple, multiple_rows))
    }.await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step("mixed cold transport target cleanup", target.abandon().await);
    if !source_abandoned { terminal.step("mixed cold transport source cleanup", source.abandon().await); }
    let (graph, nongraph, graph_rows, nongraph_rows, expected, actual, nongraph_frames, multiple, multiple_rows) =
        lctx_model::domain::completion::complete(result, terminal).unwrap();
    assert_eq!(expected.semantic.memberships, 3, "independent fixture supplies exactly three logical memberships");
    assert_eq!(expected.semantic.backing_rows, 1, "only the nongraph row belongs to family3");
    assert_eq!(multiple_rows, vec![multiple], "multi-token graph assertion survives source cleanup with exact endpoints");
    assert_eq!(nongraph_frames, 1, "the sole nongraph row is transported independently of the graph prerequisite");
    assert_eq!(graph_rows, vec![graph], "actual graph source survives original attempt cleanup");
    assert_eq!(nongraph_rows, vec![nongraph], "independent nongraph block decodes the exact source");
    assert_eq!(actual, expected, "fresh physical receiving owners preserve exact logical roots and source content");
}
