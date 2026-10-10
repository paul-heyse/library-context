use futures::TryStreamExt;
use lctx_model::domain::{
    ContentHash, Record, Relation,
    completed::ContributionSpec,
    input::Package,
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{
    RuntimeConfig,
    compiler::{NativeCompilerStore, NativePredicate},
};
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

fn spec(producer: &str, relation: &Relation) -> ContributionSpec {
    ContributionSpec {
        captured_binding: None,
        producer: producer.into(),
        profile: Profile::Catalog,
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
async fn cold_backing_rejects_valid_body_changes_and_false_typed_keys() {
    use lctx_model::domain::{FiniteF64, ModelError, analytics::QualityStep};
    use lctx_surrealdb::surrealdb::types::{Number, Value, Variables};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
        )
        .await
        .unwrap();
    store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &[relation],
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    store.verify_state().await.unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let state = store.export_state(file.path()).await.unwrap();
    assert_eq!(state.backing_rows, 1);
    let restored =
        NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
            .await
            .unwrap();
    restored.import_state(file.path(), &state).await.unwrap();
    restored.abandon().await.unwrap();

    admin
        .query("UPDATE compiler_record SET body.value=2.0f WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        store.verify_state().await,
        Err(ModelError::Conflict("compiler backing canonical body"))
    ));
    assert!(matches!(
        store.completed_state().await,
        Err(ModelError::Conflict("compiler backing canonical body"))
    ));
    admin
        .query("UPDATE compiler_record SET body.value=0.5f WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut bindings = Variables::new();
    bindings.insert("key", "00".repeat(16));
    admin
        .query("UPDATE compiler_record SET semantic_key=$key WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        store.verify_state().await,
        Err(ModelError::Conflict("compiler backing typed identity"))
    ));
    assert!(matches!(
        store.completed_state().await,
        Err(ModelError::Conflict("compiler backing typed identity"))
    ));
    let mut bindings = Variables::new();
    bindings.insert("key", row.id().hex());
    admin
        .query("UPDATE compiler_record SET semantic_key=$key WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert_eq!(store.completed_state().await.unwrap(), state);

    // External transport must reject the row before trusting even an outer state identity.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Envelope {
        table: String,
        row: Value,
    }
    let text = std::fs::read_to_string(file.path()).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    lctx_model::domain::completed::CompletedStateHeader::decode(header.as_bytes()).unwrap();
    let mut envelopes = lines
        .map(|line| serde_json::from_str::<Envelope>(line).unwrap())
        .collect::<Vec<_>>();
    for envelope in &mut envelopes {
        if envelope.table == "compiler_record" {
            let Value::Object(object) = &mut envelope.row else {
                panic!("backing object");
            };
            let Some(Value::Object(body)) = object.get_mut("body") else {
                panic!("backing body");
            };
            body.insert("value", Value::Number(Number::Float(2.0)));
        }
    }
    let text = header.to_string()
        + "\n"
        + &envelopes
            .iter()
            .map(|row| serde_json::to_string(row).unwrap() + "\n")
            .collect::<String>();
    std::fs::write(file.path(), text).unwrap();
    let altered =
        NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
            .await
            .unwrap();
    let error = altered.import_state(file.path(), &state).await.unwrap_err();
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
    use lctx_model::domain::ModelError;
    use lctx_surrealdb::surrealdb::types::{RecordId, Value, Variables};

    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
        .write_batch(&contribution, &relation, &Package::encode(&rows).unwrap())
        .await
        .unwrap();
    store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    store.verify_state().await.unwrap();

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
    assert!(matches!(
        store.verify_state().await,
        Err(ModelError::Conflict(
            "compiler membership physical identity"
        ))
    ));
    let mut restore = Variables::new();
    restore.insert("original", original);
    restore.insert("renamed", renamed);
    restore.insert("payload", Value::Object(payload));
    admin.query("BEGIN TRANSACTION; DELETE $renamed; CREATE $original CONTENT $payload; COMMIT TRANSACTION;").bind(restore).await.unwrap().check().unwrap();
    store.verify_state().await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn pending_overlap_frozen_selection_and_state_transport() {
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
        .await
        .unwrap();
    let admin = fixture_client(&store, &config).await;
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
        )
        .await
        .unwrap();
    store
        .write_batch(
            &a,
            &relation,
            &Package::encode(std::slice::from_ref(&first)).unwrap(),
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            a,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
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
            lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution", b.hex()),
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
    let state = store.completed_state().await.unwrap();
    assert_eq!(state.contributions, 2);
    assert_eq!(state.memberships, 3);
    let file = tempfile::NamedTempFile::new().unwrap();
    assert_eq!(store.export_state(file.path()).await.unwrap(), state);
    let restored =
        NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
            .await
            .unwrap();
    let restored_admin = fixture_client(&restored, &config).await;
    // Complete state references canonical families; detached import loads those separately.
    lctx_surrealdb::Loader::new(restored_admin.clone())
        .entities(&[
            lctx_model::domain::graph::Entity::from(first.clone()),
            lctx_model::domain::graph::Entity::from(Package {
                name: "second".into(),
            }),
        ])
        .await
        .unwrap();
    restored.import_state(file.path(), &state).await.unwrap();
    assert_eq!(restored.completed_state().await.unwrap(), state);
    // Replay compares complete immutable membership rows, rather than replacing corrupted
    // metadata or silently ignoring an existing key. Published graph payloads remain identical.
    let replay = restored
        .begin_contribution(spec("replay", &relation))
        .await
        .unwrap();
    let batch = Package::encode(std::slice::from_ref(&first)).unwrap();
    restored
        .write_batch(&replay, &relation, &batch)
        .await
        .unwrap();
    restored
        .write_batch(&replay, &relation, &batch)
        .await
        .unwrap();
    let mut bindings = lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert(
        "contribution",
        lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution", replay.hex()),
    );
    bindings.insert("content", ContentHash::of(b"corrupt-membership").hex());
    restored_admin
        .query("UPDATE compiler_membership SET content=$content WHERE contribution=$contribution")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let error = restored
        .write_batch(&replay, &relation, &batch)
        .await
        .unwrap_err();
    assert!(
        matches!(
            error.primary(),
            Some(lctx_model::domain::ModelError::Conflict(
                "native membership same-key payload"
            ))
        ),
        "{error:?}"
    );
    store.abandon().await.unwrap();
    abandon_known_failure(
        &restored,
        &config,
        "write_batch",
        "native membership same-key payload",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn opaque_original_chunks_use_one_physical_owner_and_detect_same_key_conflict() {
    use lctx_model::domain::{
        EvidenceBytes, artifact::ArtifactChunk, input::InputRevision, source::SourceArtifact,
    };
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &[relation.clone(), source_relation],
            &BTreeMap::new(),
        )
        .await
        .unwrap();
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
        .query("SELECT count() AS rows FROM original_chunk WHERE source=$source GROUP ALL")
        .bind((
            "source",
            lctx_surrealdb::surrealdb::types::RecordId::new(
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
    let mut response = admin
        .query("SELECT VALUE body FROM compiler_record WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    let metadata: Vec<lctx_surrealdb::surrealdb::types::Value> = response.take(0).unwrap();
    assert!(metadata.iter().all(|body|matches!(body,lctx_surrealdb::surrealdb::types::Value::Object(object) if !object.contains_key("body"))));
    store.verify_state().await.unwrap();
    let state = store.completed_state().await.unwrap();
    admin
        .query("UPDATE compiler_record SET body.ordinal=1 WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        store.verify_state().await,
        Err(lctx_model::domain::ModelError::Conflict(
            "compiler original metadata"
        ))
    ));
    assert!(matches!(
        store.completed_state().await,
        Err(lctx_model::domain::ModelError::Conflict(
            "compiler original metadata"
        ))
    ));
    admin
        .query("UPDATE compiler_record SET body.ordinal=0 WHERE id IN (SELECT VALUE node FROM compiler_membership WHERE contribution=$owner)")
        .bind(("owner",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",contribution.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    assert_eq!(store.completed_state().await.unwrap(), state);
    // Metadata is intact; the independently read physical byte owner must still agree.
    let mut response = admin
        .query("SELECT * FROM original_chunk WHERE source=$source ORDER BY start LIMIT 1")
        .bind((
            "source",
            lctx_surrealdb::surrealdb::types::RecordId::new(
                "original",
                lctx_model::domain::graph::EntityId::of(source.id()).0.hex(),
            ),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let physical: Vec<lctx_surrealdb::surrealdb::types::Value> = response.take(0).unwrap();
    let lctx_surrealdb::surrealdb::types::Value::Object(chunk) = &physical[0] else {
        panic!("physical chunk");
    };
    let Some(lctx_surrealdb::surrealdb::types::Value::Bytes(original)) = chunk.get("bytes") else {
        panic!("physical bytes");
    };
    let mut changed = original.to_vec();
    changed[0] ^= 1;
    let mut bindings = lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert("id", chunk.get("id").unwrap().clone());
    bindings.insert(
        "bytes",
        lctx_surrealdb::surrealdb::types::Bytes::from(changed.clone()),
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
        store.verify_state().await,
        Err(lctx_model::domain::ModelError::Conflict(
            "compiler original digest"
        ))
    ));
    let mut bindings = lctx_surrealdb::surrealdb::types::Variables::new();
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
    assert!(
        store
            .write_batch(
                &other,
                &relation,
                &ArtifactChunk::encode(&[altered]).unwrap()
            )
            .await
            .is_err()
    );
    assert!(store.check().is_err());
    abandon_known_failure(&store, &config, "write_batch", "original same-key payload").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn overlapping_membership_windows_count_distinct_keys_across_contributors() {
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
                .write_batch(&contribution, &relation, &Package::encode(&rows).unwrap())
                .await
                .unwrap();
        }
        views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
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
    store.verify_state().await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn large_exact_key_and_atomic_field_selections_preserve_frozen_membership() {
    use arrow_array::{FixedSizeBinaryArray, StringArray};
    use lctx_model::domain::{graph::Entity, input::Release};
    use lctx_surrealdb::compiler_provider::{select_field_table, select_table};
    use lctx_surrealdb::surrealdb::types::{Number, Value};
    use std::sync::Arc;

    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
        .await
        .unwrap();
    let admin = fixture_client(&store, &config).await;
    let relation = Relation::of::<Release>();
    let packages = (0..180)
        .map(|index| Package {
            name: format!("selection-{index:04}"),
        })
        .collect::<Vec<_>>();
    lctx_surrealdb::Loader::new(admin.clone())
        .entities(
            &packages
                .iter()
                .cloned()
                .map(Entity::from)
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    let releases = (0..720)
        .map(|index| Release {
            package: packages[index / 4].id(),
            version: (index % 4).to_string(),
        })
        .collect::<Vec<_>>();
    let mut views = BTreeMap::new();
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
        let rows = range
            .rev()
            .map(|index| releases[index].clone())
            .collect::<Vec<_>>();
        store
            .write_batch(&contribution, &relation, &Release::encode(&rows).unwrap())
            .await
            .unwrap();
        views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
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
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    for completed in [false, true] {
        if completed {
            views = store
                .complete_contribution(
                    newer,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &views,
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
    let provider = store
        .table_provider(&frozen, relation.clone(), budget.clone(), 37)
        .unwrap();
    let charge = Arc::new(lctx_model::domain::charged::StateCharge::new(
        &budget,
        "nested-window-fixture",
    ));
    let provider = select_table(&provider, Arc::new(wide_keys), charge.clone())
        .unwrap()
        .unwrap();
    let provider = select_field_table(&provider, "package", Arc::new(field_keys.clone()), charge)
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
        .filter(|row| row.version == "2" && field_keys.binary_search(row.package.bytes()).is_ok())
        .map(|row| (row.id().bytes().to_vec(), row.version.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        observed, expected,
        "every nominal window retains the same field and residual predicates"
    );
    // An unsupported residual stays above the source. Fetch must consume rejected
    // rows before counting output, and cannot stop on the first native candidate.
    let provider = store
        .table_provider(&frozen, relation.clone(), budget.clone(), 37)
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
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn canonical_graph_scans_select_completed_families_and_one_hop_aliases() {
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    use lctx_model::domain::{FiniteF64, analytics::QualityStep, graph::Entity};
    use lctx_surrealdb::surrealdb::types::{Bytes, RecordId, SurrealValue, Value, Variables};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store =
        NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Normalized)
            .await
            .unwrap();
    let admin = fixture_client(&store, &config).await;
    let relation = Relation::of::<Package>();
    let first = Package {
        name: "completed".into(),
    };
    let alias = Package {
        name: "one-hop-alias".into(),
    };
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
            &Package::encode(std::slice::from_ref(&first)).unwrap(),
        )
        .await
        .unwrap();
    store
        .write_batch(
            &complete,
            &metadata,
            &QualityStep::encode(&[quality]).unwrap(),
        )
        .await
        .unwrap();
    store
        .complete_contribution(
            complete,
            ProviderOutcome::Complete,
            &[relation.clone(), metadata],
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    let uncompleted = store
        .begin_contribution(spec("pending", &relation))
        .await
        .unwrap();
    store
        .write_batch(
            &uncompleted,
            &relation,
            &Package::encode(&[alias.clone(), pending]).unwrap(),
        )
        .await
        .unwrap();
    // Mechanical one-hop selection: only a completed source can admit its intrinsic target.
    let mut variables = Variables::new();
    variables.insert(
        "source",
        lctx_surrealdb::loader::entity_payload_id(&Entity::from(first.clone())).unwrap(),
    );
    variables.insert(
        "target",
        lctx_surrealdb::loader::entity_payload_id(&Entity::from(alias.clone())).unwrap(),
    );
    admin
        .query("CREATE $id SET source=$source,target=$target")
        .bind(("id", RecordId::new("compiler_alias", store.attempt().hex())))
        .bind(variables)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut rows = store.scan_canonical(true, &budget).await.unwrap();
    let mut names = BTreeSet::new();
    while let Some(row) = rows.next().await.unwrap() {
        let Value::Object(object) = row else {
            panic!("canonical graph record");
        };
        let bytes = Bytes::from_value(object.get("canonical").unwrap().clone()).unwrap();
        let entity: Entity = serde_json::from_slice(&bytes).unwrap();
        let Entity::Package(package) = entity else {
            panic!("only selected packages");
        };
        names.insert(package.name);
    }
    assert_eq!(names, BTreeSet::from([first.name, alias.name]));
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
async fn exact_empty_reads_retain_view_shape_and_lifecycle_checks() {
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    use lctx_model::domain::completed::CompletedView;
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
        )
        .await
        .unwrap();
    let frozen = views[relation.name()].clone();
    assert_eq!(frozen.rows, 0);
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
        )
        .await
        .unwrap();
    let current = store
        .complete_contribution(
            next,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &views,
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
        .unwrap();
    let charge = std::sync::Arc::new(lctx_model::domain::charged::StateCharge::new(
        &budget,
        "empty-key-owner",
    ));
    let provider = lctx_surrealdb::compiler_provider::select_table(
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
    use lctx_surrealdb::surrealdb::types::{ToSql, Value, Variables};
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
            )
            .await
            .unwrap();
        views = store
            .complete_contribution(
                id,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &views,
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
async fn local_membership_preparation_refusal_does_not_poison_later_consumer() {
    use lctx_model::domain::ModelError;
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
    let view = &views[relation.name()];

    // Setup and the cache entry fit, but the next reservation refuses before any
    // candidate query is launched. This is a local refusal, not a failed native effect.
    let refused = ResourceBudget::fixed(4096).unwrap();
    let error = match store.scan_rows(view, &relation, None, None, &refused).await {
        Err(error) => error,
        Ok(_) => panic!("the preparation's local budget must refuse"),
    };
    assert!(
        matches!(
            error.primary(),
            Some(ModelError::Resource {
                owner: "native-membership-preparation",
                ..
            })
        ),
        "{error:?}"
    );
    assert!(error.permits_storage_cleanup());
    assert_eq!(
        refused.reserved(),
        0,
        "failed entry and setup reservations are released"
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
async fn optional_singleton_capture_preserves_completed_state_and_refuses_unowned_reads() {
    use lctx_model::domain::{ModelError, completed::CompletedView, source::Occurrence};
    use lctx_surrealdb::surrealdb::types::{Bytes, RecordId, Variables};
    let config = RuntimeConfig::read(&std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    ))
    .unwrap();
    let off = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
        .await
        .unwrap();
    let cold = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
                )
                .await
                .unwrap();
            previous = store
                .complete_contribution(
                    id,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &previous,
                )
                .await
                .unwrap();
            if name == "second" {
                second = Some(id);
            }
        }
    }
    let second = second.unwrap();
    let expected = off.completed_state().await.unwrap();
    assert_eq!(cold.completed_state().await.unwrap(), expected);
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
        cold.completed_state().await.unwrap(),
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
    assert!(
        matches!(
            cold.scan_rows(&singleton, &relation, None, None, &budget)
                .await,
            Err(ModelError::Conflict("unregistered native completed view"))
        ),
        "the private contribution read must not authorize public dependency views"
    );
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
