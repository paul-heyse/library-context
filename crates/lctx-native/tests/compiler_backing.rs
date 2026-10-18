//! Actual native ownership for attempt-scoped model-declared projection backing.
#[path = "support/compiler_backing.rs"]
mod physical_backing;
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{
    ContentHash, EvidenceBytes, Id, ModelError, Record, Relation,
    admission::Frontier,
    analysis::sources::SourceSnapshot,
    completed::{CompletedBinding, ContributionSpec},
    projection::{
        ProjectionSnapshot, ProjectionSnapshotChunk, ProjectionSourceAssessment, snapshot,
    },
    stages::{Profile, ProviderOutcome},
};
use lctx_native::RuntimeConfig;
use std::collections::{BTreeMap, BTreeSet};

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
    assert_eq!(outcome.completion.local, LocalState::Terminal,
        "acknowledged write abandonment must finish local ownership: {failure:?}");
    assert_eq!(outcome.completion.remote, RemoteState::Confirmed,
        "acknowledged write abandonment must confirm native finality: {failure:?}");
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

#[tokio::test(flavor = "multi_thread")]
async fn projection_backing_preserves_typed_keys_payloads_and_cold_state() {
    projection_backing_transport(snapshot::CHUNK_BYTES, true).await;
}
#[tokio::test(flavor = "multi_thread")]
async fn small_projection_backing_crosses_scan_and_finalization_pools() {
    projection_backing_transport(3, false).await;
}
async fn projection_backing_transport(payload_bytes: usize, maximum_chunk: bool) {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        Frontier::Normalized,
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
    let assessment: Id<ProjectionSourceAssessment> =
        serde_json::from_value(serde_json::json!(&store.attempt().0[..16])).unwrap();
    let header = snapshot::header(assessment, payload_bytes).unwrap();
    let mut payload = vec![0xff; payload_bytes];
    payload[..3].copy_from_slice(&[0xff, 0, 0x80]);
    let chunk = ProjectionSnapshotChunk {
        snapshot: header.id(),
        ordinal: 0,
        payload: EvidenceBytes(payload),
    };
    let header_relation = Relation::of::<ProjectionSnapshot>();
    let chunk_relation = Relation::of::<ProjectionSnapshotChunk>();
    let spec = ContributionSpec {
        captured_binding: None,
        producer: "projection-backing".into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"projection-backing-model"),
        implementation: ContentHash::of(b"projection-backing-code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([
            ProjectionSnapshot::NAME.into(),
            ProjectionSnapshotChunk::NAME.into(),
        ]),
    };
    let contribution = store.begin_contribution(spec.clone()).await.unwrap();
    store
        .write_batch(
            &contribution,
            &header_relation,
            &ProjectionSnapshot::encode(std::slice::from_ref(&header)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(std::slice::from_ref(&chunk)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &[header_relation.clone(), chunk_relation.clone()],
            &BTreeMap::new(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    for relation in [&header_relation, &chunk_relation] {
        let view = views[relation.name()].clone();
        store
            .bind(CompletedBinding {
                boundary: None,
                source: SourceSnapshot::of_completed_view(relation, spec.model, &view).unwrap(),
                view,
                configuration: None,
            })
            .await
            .unwrap();
    }
    if maximum_chunk {
        // The warm maximum-sized scan borrows the already-earned producer pool.
        // It does not establish independent 8MiB receiving capacity.
        store
            .installed_view(
                &views[ProjectionSnapshotChunk::NAME],
                &native_operation_budget,
            )
            .await
            .unwrap();
    }
    let budget = if maximum_chunk { native_operation_budget.clone() } else {
        lctx_model::domain::resources::ResourceBudget::fixed(8 << 20).unwrap()
    };
    use futures::TryStreamExt;
    let batches = store
        .scan_batches(
            &views[ProjectionSnapshotChunk::NAME],
            &chunk_relation,
            None,
            None,
            &budget,
            32,
        )
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    let actual = batches
        .iter()
        .flat_map(|batch| ProjectionSnapshotChunk::decode(batch).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(actual, vec![chunk.clone()]);
    let file = tempfile::NamedTempFile::new().unwrap();
    store
        .freeze_content(&native_operation_budget)
        .await
        .unwrap();
    let state = store
        .export_state(file.path(), &native_operation_budget)
        .await
        .unwrap();
    assert_eq!(state.semantic.backing_rows, 2);
    if maximum_chunk {
        assert!(
            std::fs::read_to_string(file.path())
                .unwrap()
                .lines()
                .any(|line| line.len() > lctx_model::domain::resources::TRANSFER_BYTES),
            "maximum declared opaque chunk requires one oversized transport envelope"
        );
    }
    let restored = NativeCompilerStore::begin(
        &config,
        Frontier::Normalized,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            budget.clone(),
        )
        .expect("owned native control factory"),
    )
    .await
    .unwrap();
    restored
        .import_state(file.path(), &state, &budget)
        .await
        .unwrap();
    restored
        .freeze_content(&budget)
        .await
        .unwrap();
    assert_eq!(
        restored
            .completed_state(&budget)
            .await
            .unwrap(),
        state.semantic
    );
    let batches = restored
        .scan_batches(
            &views[ProjectionSnapshot::NAME],
            &header_relation,
            None,
            None,
            &budget,
            32,
        )
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    assert_eq!(
        batches
            .iter()
            .flat_map(|batch| ProjectionSnapshot::decode(batch).unwrap())
            .collect::<Vec<_>>(),
        vec![header]
    );
    restored.abandon().await.unwrap();
    use lctx_native::surrealdb::types::{Bytes, Number, Value};
    let backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    let mut rows = backing.into_iter().filter(|backing| backing.row.as_object().unwrap().get("semantic_type") == Some(&Value::String("projection_snapshot_chunks".into()))).collect::<Vec<_>>();
    let backing = rows.pop().unwrap(); assert!(rows.is_empty());
    let Value::Object(original) = backing.row.clone() else { panic!("native logical backing row"); };
    let mut altered = original.clone();
    let Some(Value::Object(body)) = altered.get_mut("body") else {
        panic!("native backing body");
    };
    body.insert("ordinal", Value::Number(Number::Int(1)));
    let canonical = serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    altered.insert("canonical", Bytes::from(canonical.clone()));
    altered.insert("content", ContentHash::of(&canonical).hex());
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    let changed = replacement.row.as_object().unwrap();
    assert_eq!(changed.get("body").unwrap().as_object().unwrap().get("ordinal"), Some(&Value::Number(Number::Int(1))));
    assert_eq!(changed.get("canonical"), Some(&Value::Bytes(Bytes::from(canonical.clone()))));
    assert_eq!(changed.get("content"), Some(&Value::String(ContentHash::of(&canonical).hex())));
    for field in ["id", "semantic_key", "scope_keys"] {
        assert_eq!(changed.get(field), original.get(field), "coherent body/content mutation preserves {field}");
    }
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.iter().filter(|row| row.locator == backing.locator).map(|row| &row.row).collect::<Vec<_>>(), vec![&backing.row],
        "restore exact physical locator and logical row before refusal assertions");
    assert_eq!(cached, state.semantic, "ordinary frozen identity reuses its earned owner before independent refusal");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary), Some(ModelError::Conflict("compiler backing typed identity"))),
        "coherent canonical/body/content changes cannot bypass full nominal-key validation: {verification:?}");
    let mut altered = original.clone(); altered.insert("semantic_key", "00".repeat(16));
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    assert_eq!(replacement.row.as_object().unwrap().get("semantic_key"), Some(&Value::String("00".repeat(16))));
    for field in ["id", "body", "canonical", "content", "scope_keys"] {
        assert_eq!(replacement.row.as_object().unwrap().get(field), original.get(field), "false-key mutation preserves {field}");
    }
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.iter().filter(|row| row.locator == backing.locator).map(|row| &row.row).collect::<Vec<_>>(), vec![&backing.row],
        "restore exact physical locator and nominal key before refusal assertions");
    assert_eq!(cached, state.semantic, "cached identity is not an independent corruption oracle");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary), Some(ModelError::Conflict("compiler backing typed identity"))),
        "stored nominal keys are independently recomputed: {verification:?}");
    store.verify_state(&native_operation_budget).await.unwrap();
    assert_eq!(
        store
            .completed_state(&native_operation_budget)
            .await
            .unwrap(),
        state.semantic
    );
    store.abandon().await.unwrap();
    // A distinct open contribution must reject a changed payload for its existing
    // nominal key, independently of the already-abandoned export source.
    let conflict_store = NativeCompilerStore::begin(
        &config,
        Frontier::Normalized,
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
    let mut conflicting = chunk.clone();
    conflicting.payload = EvidenceBytes(vec![1, 2, 3]);
    assert_eq!(conflicting.id(), chunk.id(), "different payload retains the same nominal chunk key");
    assert_ne!(conflicting.payload, chunk.payload, "the repeated key changes its opaque payload");
    let mut next = spec;
    next.producer = "conflicting-projection-backing".into();
    let contribution = conflict_store.begin_contribution(next).await.unwrap();
    conflict_store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(&[chunk]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let mut prior_backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    let prior = prior_backing.pop().expect("conflict owner has its exact logical member");
    assert!(prior_backing.is_empty(), "the conflict owner initially has one exact logical member");
    assert_eq!(prior.row, backing.row, "the distinct contributor initially shares the exact original logical payload");
    let prior_locator = prior.locator;
    // Reuse the original source row already retained by this control. Do not keep
    // another maximum-sized snapshot across independent physical observations.
    drop(prior.row);
    let result = conflict_store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(&[conflicting]).unwrap(),
            &native_operation_budget,
        )
        .await;
    let failure = result.expect_err("same nominal key with changed opaque payload must refuse");
    let mut after_backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    let actual = after_backing.pop().expect("refused write retains the original logical member");
    assert!(after_backing.is_empty(), "refused write adds no logical members");
    assert_eq!(actual.locator, prior_locator, "refused write preserves the exact selected placement");
    assert_eq!(actual.row, backing.row, "refused write preserves the original opaque payload");
    drop(actual);
    // The fused locator prelude refuses a changed logical member before installing
    // its content-addressed container; preserve that exact acknowledged cause.
    let native_collision = failure.primary().and_then(|primary| match primary {
        ModelError::Cause(cause) => cause.downcast_ref::<lctx_native::surrealdb::Error>(),
        _ => None,
    });
    assert!(
        native_collision.is_some_and(|error| {
            // SurrealDB 3.3.0 retains the server's THROW prefix in Error::message().
            error.is_thrown()
                && error.message() == "An error occurred: compiler logical member collision"
        }),
        "exact logical member conflict primary: {failure:?}"
    );
    assert!(
        failure.permits_storage_cleanup(),
        "collision effects are confirmed: {failure:?}"
    );
    assert!(
        !failure.has_committed_effect(),
        "refused transaction commits no conflicting effect"
    );
    abandon_acknowledged_write_failure(&conflict_store, &config, &failure).await;
    admin.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_backing_rejects_coherent_negative_zero_before_membership_checks() {
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::{FiniteF64, analytics::QualityStep};
    use lctx_native::surrealdb::types::{Bytes, Number, Value};
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        Frontier::Facts,
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
        value: FiniteF64::new(0.0).unwrap(),
    };
    let spec = ContributionSpec {
        captured_binding: None,
        producer: "quality-zero".into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"quality-zero-model"),
        implementation: ContentHash::of(b"quality-zero-code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([QualityStep::NAME.into()]),
    };
    let model = spec.model;
    let contribution = store.begin_contribution(spec).await.unwrap();
    store
        .write_batch(
            &contribution,
            &relation,
            &QualityStep::encode(&[row]).unwrap(),
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
            source: SourceSnapshot::of_completed_view(&relation, model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    store.freeze_content(&native_operation_budget).await.unwrap();
    let state = store
        .completed_state(&native_operation_budget)
        .await
        .unwrap();
    assert_eq!(state.contributions, 1);
    assert_eq!(state.memberships, 1);
    assert_eq!(state.backing_rows, 1);
    let mut backing = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    let backing = backing.pop().unwrap();
    let Value::Object(mut altered) = backing.row.clone() else { panic!("quality logical backing row"); };
    let saved = altered.clone();
    let Some(Value::Object(body)) = altered.get_mut("body") else {
        panic!("quality backing body");
    };
    body.insert("value", Value::Number(Number::Float(-0.0)));
    let canonical = serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    let content = ContentHash::of(&canonical);
    let nominal = hex::decode(altered.get("semantic_key").unwrap().as_string().unwrap()).unwrap();
    let negative_node =
        lctx_native::loader::payload_id("compiler_record", QualityStep::NAME, &nominal, content)
            .unwrap();
    altered.insert("id", negative_node.clone());
    altered.insert("canonical", Bytes::from(canonical));
    altered.insert("content", content.hex());
    assert_ne!(
        saved.get("id"),
        altered.get("id"),
        "content-addressed negative zero has its own address"
    );
    assert_ne!(
        saved.get("canonical"),
        altered.get("canonical"),
        "negative zero changes canonical bytes"
    );
    assert_eq!(
        saved.get("semantic_key"),
        altered.get("semantic_key"),
        "nominal identity is unchanged"
    );

    let saved_membership = backing.locator.clone();
    assert_eq!(saved_membership.as_object().unwrap().get("node"), saved.get("id"));
    let replacement = backing.replace(&admin, store.attempt(), contribution, Value::Object(altered), &native_operation_budget).await;
    let expected_block = replacement.block.clone();
    let expected_membership = replacement.locator.clone();
    assert_eq!(expected_membership.as_object().unwrap().get("node"), Some(&Value::RecordId(negative_node)));
    assert_eq!(expected_membership.as_object().unwrap().get("content"), Some(&Value::String(content.hex())));
    assert_ne!(saved_membership.as_object().unwrap().get("block"), Some(&Value::RecordId(expected_block)),
        "the coherently malformed logical row lives in a separately held actual block");
    // The ordinary earned identity is intentionally reused; the following fresh
    // physical/typed receiver independently checks the altered canonical bytes.
    let cached = store.completed_state(&native_operation_budget).await.unwrap();
    let verification = physical_backing::Backing::try_read(&admin, contribution, &native_operation_budget).await.map(|_| ());
    replacement.restore(&admin).await;
    let mut restored = physical_backing::Backing::read(&admin, contribution, &native_operation_budget).await;
    assert_eq!(restored.pop().unwrap().row, Value::Object(saved)); assert!(restored.is_empty());
    assert_eq!(cached, state, "ordinary frozen identity reuses its earned root; this is not a cold audit");
    assert!(matches!(verification.as_ref().err().and_then(ModelError::primary), Some(ModelError::Conflict("compiler backing declared body"))),
        "coherent negative-zero independent verification before membership checks: {verification:?}");

    store.verify_state(&native_operation_budget).await.unwrap();
    store.abandon().await.unwrap();
    admin.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_later_artifact_range_has_no_auxiliary_window_effects() {
    use lctx_model::domain::{
        artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunk},
        source::SourceArtifact,
    };
    use lctx_native::surrealdb::types::Value;
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(
        &config,
        Frontier::Facts,
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
    let artifact: Id<SourceArtifact> =
        serde_json::from_value(serde_json::to_value(&store.attempt().0[..16]).unwrap()).unwrap();
    let good = ArtifactChunk {
        artifact,
        ordinal: 0,
        body: EvidenceBytes(vec![0xff, 0, 0x80]),
    };
    let invalid_start = i64::MAX / (ARTIFACT_CHUNK_BYTES as i64) + 1;
    let bad = (invalid_start..invalid_start + 1024)
        .map(|ordinal| ArtifactChunk {
            artifact,
            ordinal,
            body: EvidenceBytes(vec![7]),
        })
        .find(|row| row.id() > good.id())
        .expect("invalid row follows valid row in canonical key order");
    let rows = [good.clone(), bad];
    let relation = Relation::of::<ArtifactChunk>();
    let batch = ArtifactChunk::encode(&rows).unwrap();
    assert_eq!(
        ArtifactChunk::decode(&relation.canonical(&batch).unwrap()).unwrap()[0],
        good,
        "control must reach invalid metadata after preparing the valid first row"
    );
    let spec = ContributionSpec {
        captured_binding: None,
        producer: "range-window".into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"range-window-model"),
        implementation: ContentHash::of(b"range-window-code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([ArtifactChunk::NAME.into()]),
    };
    let contribution = store.begin_contribution(spec).await.unwrap();
    let native_operation_budget =
        lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let result = store
        .write_batch(&contribution, &relation, &batch, &native_operation_budget)
        .await;
    let failure =
        result.expect_err("later unrepresentable original range must refuse the complete window");
    assert!(
        matches!(
            failure.primary(),
            Some(ModelError::Schema("original chunk range"))
        ),
        "exact acknowledged range primary: {failure:?}"
    );
    for (table, predicate) in [
        ("original", "id=$source"),
        ("original_chunk", "blob=$source.blob"),
        ("native_hold", "owner=$owner AND record::table(object)='compiler_block'"),
        ("compiler_membership", "contribution=$owner"),
    ] {
        let mut bindings = lctx_native::surrealdb::types::Variables::new();
        bindings.insert(
            "source",
            lctx_native::surrealdb::types::RecordId::new(
                "original",
                lctx_model::domain::graph::EntityId::of(artifact).0.hex(),
            ),
        );
        bindings.insert(
            "owner",
            lctx_native::surrealdb::types::RecordId::new(
                "compiler_contribution",
                contribution.hex(),
            ),
        );
        let mut response = admin
            .query(format!("SELECT * FROM {table} WHERE {predicate}"))
            .bind(bindings)
            .await
            .unwrap()
            .check()
            .unwrap();
        let rows: Vec<Value> = response.take(0).unwrap();
        assert!(
            rows.is_empty(),
            "later invalid range must precede every owned effect in {table}"
        );
    }
    abandon_acknowledged_write_failure(&store, &config, &failure).await;
    admin.invalidate().await.unwrap();
}
