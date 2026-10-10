//! Actual persistent gRPC cache boundaries; no tokenizer, provider or compiler input.
use lctx_model::domain::{
    ContentHash,
    embedding::{
        Spec,
        cache::{CacheValue, EmbeddingCache},
        value::encode_vector,
    },
};
use lctx_surrealdb::{NativeEmbeddingCache, RuntimeConfig, compiler::check_installation};
use std::{collections::BTreeSet, sync::Arc};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::Variables};

fn spec() -> Spec {
    serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap()
}
fn candidate(scope: &str, label: &str, axis: usize, tokens: u32) -> CacheValue {
    let mut vector = vec![-0.0; spec().dimensions as usize];
    vector[axis] = 1.0;
    CacheValue {
        input_hash: ContentHash::of(format!("{scope}:{label}").as_bytes()),
        vector,
        admitted_tokens: tokens,
    }
}
struct Fixture {
    client: Arc<Surreal<Client>>,
    cache: NativeEmbeddingCache,
    config: RuntimeConfig,
    scope: String,
}
impl Fixture {
    async fn new(label: &str) -> Self {
        let config = RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("installed validation runtime"))).unwrap();
        let client = check_installation(&config).await.unwrap();
        let cache = NativeEmbeddingCache::connect(client.clone()).await.unwrap();
        let nonce = tempfile::NamedTempFile::new().unwrap();
        let scope = format!("{label}:{}", nonce.path().display());
        Self {client, cache, config, scope}
    }
    async fn connect(config: &RuntimeConfig) -> Arc<Surreal<Client>> {
        check_installation(config).await.unwrap()
    }
    async fn close(self) {
        self.client.invalidate().await.unwrap();
    }
    fn winner(&self, spec: &Spec, input: ContentHash) -> surrealdb::types::RecordId {
        surrealdb::types::RecordId::new("embedding_cache", format!("{}_{}", spec.hash().hex(), input.hex()))
    }
}

#[tokio::test]
async fn empty_native_batches_leave_existing_winners_unchanged() {
    let f = Fixture::new("empty").await;
    let spec = spec();
    assert!(f.cache.admit(&spec, &[]).await.unwrap().is_empty());
    assert!(f.cache.cached(&spec, &[]).await.unwrap().is_empty());
    let first = candidate(&f.scope, "existing-before-empty", 9, 3);
    let before = f
        .cache
        .admit(&spec, std::slice::from_ref(&first))
        .await
        .unwrap();
    assert!(f.cache.admit(&spec, &[]).await.unwrap().is_empty());
    assert!(f.cache.cached(&spec, &[]).await.unwrap().is_empty());
    let after = f.cache.cached(&spec, &[first.input_hash]).await.unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        encode_vector(&after[&first.input_hash].vector),
        encode_vector(&before[&first.input_hash].vector)
    );
    assert_eq!(
        after[&first.input_hash].admitted_tokens,
        before[&first.input_hash].admitted_tokens
    );
    let mut response = f
        .client
        .query("SELECT VALUE total FROM (SELECT count() AS total FROM embedding_cache WHERE id=$id GROUP ALL)")
        .bind(("id", f.winner(&spec, first.input_hash)))
        .await
        .unwrap()
        .check()
        .unwrap();
    let counts: Vec<i64> = response.take(0).unwrap();
    assert_eq!(counts, vec![1]);
    f.close().await;
}

#[tokio::test]
async fn whole_batch_prevalidation_and_first_duplicate_proposal() {
    let f = Fixture::new("validation").await;
    let spec = spec();
    let first = candidate(&f.scope, "first", 0, 3);
    let mut bad = candidate(&f.scope, "bad", 1, 3);
    bad.vector[0] = f32::NAN;
    assert!(f.cache.admit(&spec, &[first.clone(), bad]).await.is_err());
    assert!(
        f.cache
            .cached(&spec, &[first.input_hash])
            .await
            .unwrap()
            .is_empty()
    );
    let token_conflict = CacheValue {
        admitted_tokens: 4,
        ..first.clone()
    };
    assert!(
        f.cache
            .admit(&spec, &[first.clone(), token_conflict])
            .await
            .is_err()
    );
    assert!(
        f.cache
            .cached(&spec, &[first.input_hash])
            .await
            .unwrap()
            .is_empty()
    );
    let alternative = candidate(&f.scope, "first", 1, 3);
    let mut candidates: Vec<_> = (0..257)
        .map(|i| candidate(&f.scope, &format!("batch-{i}"), i % spec.dimensions as usize, 3))
        .collect();
    candidates.extend([first.clone(), first.clone(), alternative]);
    let winners = f.cache.admit(&spec, &candidates).await.unwrap();
    assert_eq!(winners.len(), 258);
    assert_eq!(
        encode_vector(&winners[&first.input_hash].vector),
        encode_vector(&first.vector)
    );
    let reread = f
        .cache
        .cached(
            &spec,
            &candidates.iter().map(|c| c.input_hash).collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    assert_eq!(reread.len(), winners.len());
    for (key, winner) in winners {
        assert_eq!(
            encode_vector(&reread[&key].vector),
            encode_vector(&winner.vector)
        );
        assert_eq!(reread[&key].admitted_tokens, winner.admitted_tokens);
    }
    f.close().await;
}

#[tokio::test]
async fn overlapping_concurrent_batches_return_exact_existing_and_new_winners() {
    let f = Fixture::new("concurrent").await;
    let spec = spec();
    let preexisting: Vec<_> = (0..4)
        .map(|i| candidate(&f.scope, &format!("shared-{i}"), 19, 3))
        .collect();
    f.cache.admit(&spec, &preexisting).await.unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(8));
    let mut tasks = Vec::new();
    for caller in 0..8 {
        let client = Fixture::connect(&f.config).await;
        let cache = NativeEmbeddingCache::connect(client.clone()).await.unwrap();
        let spec = spec.clone();
        let barrier = barrier.clone();
        let scope = f.scope.clone();
        tasks.push(tokio::spawn(async move {
            let mut proposals: Vec<_> = (0..32)
                .map(|i| candidate(&scope, &format!("shared-{i}"), caller, 3))
                .collect();
            proposals.extend((0..8).map(|i| candidate(&scope, &format!("caller-{caller}-{i}"), caller, 3)));
            barrier.wait().await;
            let winners = cache.admit(&spec, &proposals).await.unwrap();
            assert_eq!(winners.len(), proposals.len());
            client.invalidate().await.unwrap();
            winners
        }));
    }
    let mut outcomes = Vec::new();
    for task in tasks {
        outcomes.push(task.await.unwrap());
    }
    let keys: Vec<_> = outcomes
        .iter()
        .flat_map(|map| map.keys().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(keys.len(), 96);
    let stored = f.cache.cached(&spec, &keys).await.unwrap();
    assert_eq!(stored.len(), 96);
    for outcome in &outcomes {
        for (key, winner) in outcome {
            assert_eq!(
                encode_vector(&winner.vector),
                encode_vector(&stored[key].vector)
            );
            assert_eq!(winner.admitted_tokens, stored[key].admitted_tokens);
        }
    }
    for first in preexisting {
        assert_eq!(
            encode_vector(&stored[&first.input_hash].vector),
            encode_vector(&first.vector)
        );
    }
    f.close().await;
}

#[tokio::test]
async fn supplied_token_mismatch_refuses_and_offline_lookup_retains_stored_receipt() {
    let f = Fixture::new("receipt").await;
    let spec = spec();
    let first = candidate(&f.scope, "receipt", 0, 3);
    f.cache
        .admit(&spec, std::slice::from_ref(&first))
        .await
        .unwrap();
    let later = candidate(&f.scope, "receipt", 1, 4);
    assert!(f.cache.admit(&spec, &[later]).await.is_err());
    let read = f.cache.cached(&spec, &[first.input_hash]).await.unwrap();
    assert_eq!(read[&first.input_hash].admitted_tokens, 3);
    assert_eq!(
        encode_vector(&read[&first.input_hash].vector),
        encode_vector(&first.vector)
    );
    f.close().await;
}

#[tokio::test]
async fn ignored_row_failure_without_a_winner_refuses_admission() {
    let f = Fixture::new("ignored").await;
    let spec = spec();
    let good = candidate(&f.scope, "good-row", 0, 3);
    let rejected = candidate(&f.scope, "rejected-row", 1, 11);
    // A unique-key conflict at an owned noncanonical physical ID is swallowed by
    // INSERT IGNORE, but cannot establish the exact canonical winner receipt.
    let mut bindings = Variables::new();
    bindings.insert("id", surrealdb::types::RecordId::new("embedding_cache", format!("rejected_{}", rejected.input_hash.hex())));
    bindings.insert("spec", spec.hash().hex());
    bindings.insert("input", rejected.input_hash.hex());
    bindings.insert("definition", surrealdb::types::Bytes::from(serde_json::to_vec(&lctx_model::domain::embedding::EmbeddingSpec::new(&spec).unwrap()).unwrap()));
    bindings.insert("bytes", surrealdb::types::Bytes::from(encode_vector(&rejected.vector)));
    bindings.insert("digest", lctx_model::domain::embedding::value::value_digest(&rejected.vector).hex());
    f.client.query("CREATE $id CONTENT {spec:$spec,input:$input,definition:$definition,tokens:11,bytes:$bytes,digest:$digest}").bind(bindings).await.unwrap().check().unwrap();
    assert!(
        f.cache
            .admit(&spec, &[good.clone(), rejected.clone()])
            .await
            .is_err()
    );
    let stored = f
        .cache
        .cached(&spec, &[good.input_hash, rejected.input_hash])
        .await
        .unwrap();
    assert!(stored.contains_key(&good.input_hash));
    assert!(!stored.contains_key(&rejected.input_hash));
    f.close().await;
}

#[tokio::test]
async fn exact_key_read_refuses_altered_identity_and_winning_bytes() {
    let f = Fixture::new("corruption").await;
    let spec = spec();
    let first = candidate(&f.scope, "corrupt", 0, 3);
    f.cache
        .admit(&spec, std::slice::from_ref(&first))
        .await
        .unwrap();
    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        surrealdb::types::RecordId::new(
            "embedding_cache",
            format!("{}_{}", spec.hash().hex(), first.input_hash.hex()),
        ),
    );
    bindings.insert("wrong", ContentHash::of(b"wrong-input").hex());
    f.client
        .query("UPDATE $id SET input=$wrong")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(f.cache.cached(&spec, &[first.input_hash]).await.is_err());
    bindings.insert("input", first.input_hash.hex());
    f.client
        .query("UPDATE $id SET input=$input, digest='wrong-digest'")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(f.cache.cached(&spec, &[first.input_hash]).await.is_err());
    f.close().await;
}

#[tokio::test]
async fn full_winners_reuse_encoder_identity_across_query_recipe_changes() {
    let f = Fixture::new("query_recipe").await;
    let spec = spec();
    let first = candidate(&f.scope, "query-independent-document", 0, 3);
    f.cache
        .admit(&spec, std::slice::from_ref(&first))
        .await
        .unwrap();
    let mut changed = spec.clone();
    changed.query_template = "New query instruction: {task_description}\nQuery:{query}".into();
    changed.query_task = "independent query ranking policy".into();
    let reused = f.cache.cached(&changed, &[first.input_hash]).await.unwrap();
    assert_eq!(
        encode_vector(&reused[&first.input_hash].vector),
        encode_vector(&first.vector)
    );
    assert_eq!(reused[&first.input_hash].vector.len(), 4096);
    let mut response = f
        .client
        .query("SELECT bytes FROM $id")
        .bind(("id", f.winner(&spec, first.input_hash)))
        .await
        .unwrap()
        .check()
        .unwrap();
    let rows: Vec<surrealdb::types::Object> = response.take(0).unwrap();
    assert_eq!(rows.len(), 1);
    let surrealdb::types::Value::Bytes(bytes) = rows[0].get("bytes").unwrap() else {
        panic!("canonical bytes")
    };
    assert_eq!(bytes.len(), 16384);
    let mut lower_document_cap = changed.clone();
    lower_document_cap.max_document_tokens = 1;
    assert_eq!(
        f.cache
            .cached(&lower_document_cap, &[first.input_hash])
            .await
            .unwrap()[&first.input_hash]
            .admitted_tokens,
        3
    );
    let mut another_encoder = changed;
    another_encoder.model = "another-encoder".into();
    assert!(
        f.cache
            .cached(&another_encoder, &[first.input_hash])
            .await
            .unwrap()
            .is_empty()
    );
    f.close().await;
}
