//! Actual native ownership for attempt-scoped model-declared projection backing.
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
use lctx_surrealdb::{RuntimeConfig, compiler::NativeCompilerStore};
use std::collections::{BTreeMap, BTreeSet};

async fn fixture_client(
    store: &NativeCompilerStore,
    config: &RuntimeConfig,
) -> std::sync::Arc<
    lctx_surrealdb::surrealdb::Surreal<lctx_surrealdb::surrealdb::engine::remote::grpc::Client>,
> {
    lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        store.namespace().as_str(),
        store.database().as_str(),
    )
    .await
    .unwrap()
}

async fn compiler_backing_nodes(
    client: &std::sync::Arc<
        lctx_surrealdb::surrealdb::Surreal<lctx_surrealdb::surrealdb::engine::remote::grpc::Client>,
    >,
    contribution: ContentHash,
) -> Vec<lctx_surrealdb::surrealdb::types::RecordId> {
    let mut bindings = lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert(
        "owner",
        lctx_surrealdb::surrealdb::types::RecordId::new(
            "compiler_contribution",
            contribution.hex(),
        ),
    );
    lctx_surrealdb::NativeReader::private(client.clone())
        .query_native(
            "SELECT VALUE node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND record::table(node)='compiler_record'",
            bindings,
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
            lctx_surrealdb::surrealdb::types::RecordId::new(
                "native_attempt",
                store.attempt().hex(),
            ),
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
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Normalized)
        .await
        .unwrap();
    let admin = fixture_client(&store, &config).await;
    let assessment: Id<ProjectionSourceAssessment> =
        serde_json::from_value(serde_json::json!(&store.attempt().0[..16])).unwrap();
    let header = snapshot::header(assessment, snapshot::CHUNK_BYTES).unwrap();
    let mut payload = vec![0xff; snapshot::CHUNK_BYTES];
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
        profile: Profile::Catalog,
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
        )
        .await
        .unwrap();
    store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(std::slice::from_ref(&chunk)).unwrap(),
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &[header_relation.clone(), chunk_relation.clone()],
            &BTreeMap::new(),
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
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(8 << 20).unwrap();
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
    let state = store.export_state(file.path()).await.unwrap();
    assert_eq!(state.backing_rows, 2);
    assert!(
        std::fs::read_to_string(file.path())
            .unwrap()
            .lines()
            .any(|line| line.len() > lctx_model::domain::resources::TRANSFER_BYTES),
        "maximum declared opaque chunk requires one oversized transport envelope"
    );
    let restored = NativeCompilerStore::begin(&config, Frontier::Normalized)
        .await
        .unwrap();
    restored.import_state(file.path(), &state).await.unwrap();
    assert_eq!(restored.completed_state().await.unwrap(), state);
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
    use lctx_surrealdb::surrealdb::types::{Bytes, Number, Value, Variables};
    let backing_nodes = compiler_backing_nodes(&admin, contribution).await;
    let mut response = admin
        .query("SELECT * FROM $nodes WHERE record::table(id)='compiler_record' AND semantic_type='projection_snapshot_chunks'")
        .bind(("nodes", backing_nodes.clone()))
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut rows: Vec<Value> = response.take(0).unwrap();
    let original = rows.pop().unwrap();
    assert!(rows.is_empty());
    let Value::Object(original) = original else {
        panic!("native backing row");
    };
    let id = original.get("id").unwrap().clone();
    let mut altered = original.clone();
    let Some(Value::Object(body)) = altered.get_mut("body") else {
        panic!("native backing body");
    };
    body.insert("ordinal", Value::Number(Number::Int(1)));
    let canonical = serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    altered.insert("canonical", Bytes::from(canonical.clone()));
    altered.insert("content", ContentHash::of(&canonical).hex());
    let mut bindings = Variables::new();
    bindings.insert("id", id.clone());
    bindings.insert("row", Value::Object(altered));
    admin
        .query("UPDATE $id CONTENT $row RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        matches!(
            store.verify_state().await,
            Err(ModelError::Conflict("compiler backing typed identity"))
        ),
        "coherent canonical/body/content changes cannot bypass full nominal-key validation"
    );
    let mut bindings = Variables::new();
    bindings.insert("id", id.clone());
    bindings.insert("row", Value::Object(original.clone()));
    admin
        .query("UPDATE $id CONTENT $row RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut bindings = Variables::new();
    bindings.insert("id", id.clone());
    bindings.insert("key", "00".repeat(16));
    admin
        .query("UPDATE $id SET semantic_key=$key RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        matches!(
            store.verify_state().await,
            Err(ModelError::Conflict("compiler backing typed identity"))
        ),
        "stored nominal keys are independently recomputed"
    );
    let mut bindings = Variables::new();
    bindings.insert("id", id);
    bindings.insert("row", Value::Object(original));
    admin
        .query("UPDATE $id CONTENT $row RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert_eq!(store.completed_state().await.unwrap(), state);
    let mut conflicting = chunk.clone();
    conflicting.payload = EvidenceBytes(vec![1, 2, 3]);
    let mut next = spec;
    next.producer = "conflicting-projection-backing".into();
    let contribution = store.begin_contribution(next).await.unwrap();
    store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(&[chunk]).unwrap(),
        )
        .await
        .unwrap();
    let result = store
        .write_batch(
            &contribution,
            &chunk_relation,
            &ProjectionSnapshotChunk::encode(&[conflicting]).unwrap(),
        )
        .await;
    let failure = result.expect_err("same nominal key with changed opaque payload must refuse");
    // The shared immutable-address guard refuses the changed membership before the
    // former caller-specific conflict label; preserve its actual acknowledged cause.
    let native_collision = failure.primary().and_then(|primary| match primary {
        ModelError::Cause(cause) => cause.downcast_ref::<lctx_surrealdb::surrealdb::Error>(),
        _ => None,
    });
    assert!(
        native_collision.is_some_and(|error| {
            // SurrealDB 3.3.0 retains the server's THROW prefix in Error::message().
            error.is_thrown()
                && error.message() == "An error occurred: native immutable address collision"
        }),
        "exact physical conflict primary: {failure:?}"
    );
    assert!(
        failure.permits_storage_cleanup(),
        "collision effects are confirmed: {failure:?}"
    );
    assert!(
        !failure.has_committed_effect(),
        "refused transaction commits no conflicting effect"
    );
    abandon_acknowledged_write_failure(&store, &config, &failure).await;
    admin.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_backing_rejects_coherent_negative_zero_before_membership_checks() {
    use lctx_model::domain::{FiniteF64, analytics::QualityStep};
    use lctx_surrealdb::surrealdb::types::{Bytes, Number, ToSql, Value};
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts)
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
        profile: Profile::Catalog,
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
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
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
    let state = store.completed_state().await.unwrap();
    assert_eq!(state.contributions, 1);
    assert_eq!(state.memberships, 1);
    assert_eq!(state.backing_rows, 1);
    let backing_nodes = compiler_backing_nodes(&admin, contribution).await;
    let mut response = admin
        .query("SELECT * FROM $nodes WHERE record::table(id)='compiler_record'")
        .bind(("nodes", backing_nodes.clone()))
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut rows: Vec<Value> = response.take(0).unwrap();
    let Value::Object(mut altered) = rows.pop().unwrap() else {
        panic!("quality backing row");
    };
    assert!(rows.is_empty());
    let saved = altered.clone();
    let Some(Value::Object(body)) = altered.get_mut("body") else {
        panic!("quality backing body");
    };
    body.insert("value", Value::Number(Number::Float(-0.0)));
    let canonical = serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    let content = ContentHash::of(&canonical);
    let nominal = hex::decode(altered.get("semantic_key").unwrap().as_string().unwrap()).unwrap();
    let negative_node = lctx_surrealdb::loader::payload_id(
        "compiler_record", QualityStep::NAME, &nominal, content,
    ).unwrap();
    altered.insert("id", negative_node.clone());
    altered.insert("canonical", Bytes::from(canonical));
    altered.insert("content", content.hex());
    assert_ne!(saved.get("id"), altered.get("id"), "content-addressed negative zero has its own address");
    assert_ne!(saved.get("canonical"), altered.get("canonical"), "negative zero changes canonical bytes");
    assert_eq!(saved.get("semantic_key"), altered.get("semantic_key"), "nominal identity is unchanged");

    let mut membership = admin.query("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner")
        .bind(("owner", lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution", contribution.hex())))
        .await.unwrap().check().unwrap();
    let memberships: Vec<Value> = membership.take(0).unwrap();
    assert_eq!(memberships.len(), 1, "exact owned contributor membership");
    let saved_membership = memberships[0].as_object().unwrap().clone();
    assert_eq!(saved_membership.get("node"), saved.get("id"));
    let membership_id = saved_membership.get("id").unwrap().clone();
    // This deliberate corruption remains independently owned while both cold validators run.
    // The native state reader selects backing through this exact contributor membership;
    // view descriptors retain the same nominal membership/cardinality.
    let pin = lctx_surrealdb::control::ReaderPin::acquire(admin.clone(), &[]).await.unwrap();
    let observations = async {
        lctx_surrealdb::control::ensure_rows(&admin, Some(store.attempt()), vec![Value::Object(altered.clone())]).await?;
        pin.protect(negative_node.clone()).await?;
        let mut mutation = admin.query("UPDATE $id SET node=$node,content=$content RETURN AFTER")
            .bind(("id", membership_id.clone())).bind(("node", negative_node.clone()))
            .bind(("content", content.hex())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let changed: Vec<Value> = mutation.take(0).map_err(ModelError::codec)?;
        let verification = store.verify_state().await;
        let completed = store.completed_state().await;
        Ok::<_, ModelError>((changed, verification, completed))
    }.await;
    let mut finality = lctx_model::domain::completion::Completion::default();
    let restoration = async {
        admin.query("UPDATE $id CONTENT $row RETURN NONE")
            .bind(("id", membership_id.clone())).bind(("row", Value::Object(saved_membership.clone())))
            .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(())
    }.await;
    let restored = restoration.is_ok();
    finality.step("negative-zero exact membership restoration", restoration);
    // Never retire the replacement while a failed restoration may still refer to it.
    if !restored {
        finality.step("negative-zero failed restoration retention", lctx_surrealdb::control::hold(
            &admin, Some(store.attempt()),
            lctx_surrealdb::surrealdb::types::RecordId::new("native_attempt", store.attempt().hex()),
            vec![negative_node.clone()],
        ).await);
    }
    finality.step("negative-zero temporary pin release", pin.release().await);
    if restored {
        finality.step("negative-zero exact malformed payload retirement", async {
            let progress = lctx_surrealdb::control::retire_reachable(&admin, vec![negative_node.clone()], 1).await?;
            if !progress.remaining.is_empty() {
                return Err(ModelError::Conflict("negative-zero malformed payload retained"));
            }
            Ok(())
        }.await);
    } else {
        finality.storage.push(lctx_model::domain::completion::StorageState::Orphan(negative_node.to_sql()));
    }
    let (changed, verification, completed) = lctx_model::domain::completion::complete(observations, finality).unwrap();
    let mut readback = admin.query("SELECT * FROM $member; SELECT * FROM $original; SELECT * FROM $negative")
        .bind(("member", membership_id)).bind(("original", saved.get("id").unwrap().clone()))
        .bind(("negative", negative_node.clone())).await.unwrap().check().unwrap();
    let restored_memberships: Vec<Value> = readback.take(0).unwrap();
    let restored_backing: Vec<Value> = readback.take(1).unwrap();
    let retired_negative: Vec<Value> = readback.take(2).unwrap();
    assert_eq!(restored_memberships, vec![Value::Object(saved_membership.clone())]);
    assert_eq!(restored_backing, vec![Value::Object(saved)]);
    assert!(retired_negative.is_empty(), "temporary malformed payload retired before refusal assertions");
    let mut expected_membership = saved_membership;
    expected_membership.insert("node", negative_node);
    expected_membership.insert("content", content.hex());
    assert_eq!(changed, vec![Value::Object(expected_membership)], "one exact membership changes only physical payload and content");
    assert!(matches!(&verification, Err(ModelError::Conflict("compiler backing declared body"))),
        "coherent negative-zero independent verification before membership checks: {verification:?}");
    assert!(matches!(&completed, Err(ModelError::Conflict("compiler backing declared body"))),
        "coherent negative-zero completed state before membership checks: {completed:?}");
    store.verify_state().await.unwrap();
    store.abandon().await.unwrap();
    admin.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_later_artifact_range_has_no_auxiliary_window_effects() {
    use lctx_model::domain::{
        artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunk},
        source::SourceArtifact,
    };
    use lctx_surrealdb::surrealdb::types::Value;
    let path =
        std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts)
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
        profile: Profile::Catalog,
        model: ContentHash::of(b"range-window-model"),
        implementation: ContentHash::of(b"range-window-code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([ArtifactChunk::NAME.into()]),
    };
    let contribution = store.begin_contribution(spec).await.unwrap();
    let result = store.write_batch(&contribution, &relation, &batch).await;
    let failure =
        result.expect_err("later unrepresentable original range must refuse the complete window");
    assert!(
        matches!(
            failure.primary(),
            Some(ModelError::Schema("original chunk range"))
        ),
        "exact acknowledged range primary: {failure:?}"
    );
    let backing_nodes = compiler_backing_nodes(&admin, contribution).await;
    for (table, predicate) in [
        ("original", "id=$source"),
        ("original_chunk", "source=$source"),
        ("compiler_record", "record::table(id)='compiler_record'"),
        ("compiler_membership", "contribution=$owner"),
    ] {
        let mut bindings = lctx_surrealdb::surrealdb::types::Variables::new();
        bindings.insert(
            "source",
            lctx_surrealdb::surrealdb::types::RecordId::new(
                "original",
                lctx_model::domain::graph::EntityId::of(artifact).0.hex(),
            ),
        );
        bindings.insert(
            "owner",
            lctx_surrealdb::surrealdb::types::RecordId::new(
                "compiler_contribution",
                contribution.hex(),
            ),
        );
        let source = if table == "compiler_record" {
            bindings.insert("nodes", backing_nodes.clone());
            "$nodes"
        } else {
            table
        };
        let mut response = admin
            .query(format!("SELECT * FROM {source} WHERE {predicate}"))
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
