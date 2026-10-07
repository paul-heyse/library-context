//! Immutable shared embedding winners, hosted outside private snapshot databases.
use lctx_model::domain::{
    ContentHash, Infrastructure, ModelError,
    embedding::{
        EmbeddingSpec, Spec,
        cache::{CacheFuture, CacheValue, EmbeddingCache},
        check_vector,
        value::{decode_vector, encode_vector, value_digest},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Bytes, ConnectionError, QueryError, RecordId, SurrealValue, Variables},
};
#[derive(Clone)]
pub struct NativeEmbeddingCache {
    client: Arc<Surreal<Client>>,
}
impl NativeEmbeddingCache {
    pub async fn install(client: Arc<Surreal<Client>>) -> Result<Self, ModelError> {
        client.query("DEFINE TABLE IF NOT EXISTS embedding_cache TYPE NORMAL SCHEMAFULL; DEFINE FIELD IF NOT EXISTS spec ON embedding_cache TYPE string; DEFINE FIELD IF NOT EXISTS input ON embedding_cache TYPE string; DEFINE FIELD IF NOT EXISTS definition ON embedding_cache TYPE bytes; DEFINE FIELD IF NOT EXISTS tokens ON embedding_cache TYPE int ASSERT $value >= 0; DEFINE FIELD IF NOT EXISTS bytes ON embedding_cache TYPE bytes; DEFINE FIELD IF NOT EXISTS digest ON embedding_cache TYPE string; DEFINE INDEX IF NOT EXISTS winner ON embedding_cache FIELDS spec,input UNIQUE;").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(Self { client })
    }
    async fn read(
        &self,
        spec: &Spec,
        keys: &[ContentHash],
    ) -> Result<BTreeMap<ContentHash, CacheValue>, ModelError> {
        validate_spec(spec)?;
        let spec_hash = spec.hash().hex();
        let definition = serde_json::to_vec(&EmbeddingSpec::new(spec)?).map_err(ModelError::codec)?;
        let keys: Vec<_> = keys
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut result = BTreeMap::new();
        for batch in keys.chunks(BATCH_ROWS) {
            let mut bindings = Variables::new();
            bindings.insert(
                "ids",
                batch
                    .iter()
                    .map(|key| winner_id(&spec_hash, *key))
                    .collect::<Vec<_>>(),
            );
            let mut response = self
                .client
                .query("SELECT * FROM $ids")
                .bind(bindings)
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let rows: Vec<Winner> = response.take(0).map_err(ModelError::codec)?;
            for row in rows {
                let hash = ContentHash(
                    hex::decode(&row.input)
                        .map_err(ModelError::codec)?
                        .try_into()
                        .map_err(|_| ModelError::Schema("embedding input hash"))?,
                );
                if row.spec != spec_hash
                    || row.definition.as_ref() != definition
                    || row.input != hash.hex()
                    || batch.binary_search(&hash).is_err()
                    || row.id != winner_id(&spec_hash, hash)
                {
                    return Err(ModelError::Conflict(
                        "embedding cache key/specification/token admission",
                    ));
                }
                let vector = decode_vector(row.bytes.as_ref(), spec.dimensions).map_err(ModelError::Invalid)?;
                check_vector(&vector, spec.dimensions).map_err(ModelError::Invalid)?;
                if encode_vector(&vector) != row.bytes.as_ref()
                    || value_digest(&vector).hex() != row.digest
                {
                    return Err(ModelError::Conflict("embedding cache winning bytes"));
                }
                if result
                    .insert(
                        hash,
                        CacheValue {
                            input_hash: hash,
                            vector,
                            admitted_tokens: row.tokens,
                        },
                    )
                    .is_some()
                {
                    return Err(ModelError::Conflict("duplicate embedding winner"));
                }
            }
        }
        Ok(result)
    }

    async fn insert(
        &self,
        spec: &Spec,
        candidates: &[&CacheValue],
        definition: &[u8],
    ) -> Result<(), surrealdb::Error> {
        let spec_hash = spec.hash().hex();
        let rows: Vec<_> = candidates
            .iter()
            .map(|candidate| Winner {
                id: winner_id(&spec_hash, candidate.input_hash),
                spec: spec_hash.clone(),
                input: candidate.input_hash.hex(),
                definition: Bytes::from(definition.to_vec()),
                tokens: candidate.admitted_tokens,
                bytes: Bytes::from(encode_vector(&candidate.vector)),
                digest: value_digest(&candidate.vector).hex(),
            })
            .collect();
        let mut bindings = Variables::new();
        bindings.insert("rows", rows);
        // IGNORE can swallow nonduplicate row failures too. RETURN NONE is not a receipt;
        // only the following exact-key read establishes committed immutable winners.
        self.client
            .query("INSERT IGNORE INTO embedding_cache $rows RETURN NONE")
            .bind(bindings)
            .await?
            .check()
            .map(|_| ())
    }

    async fn reconcile_attempt<'a>(
        &self,
        spec: &Spec,
        candidates: &[&'a CacheValue],
        written: Result<(), surrealdb::Error>,
    ) -> Result<Attempt<'a>, ModelError> {
        // Never conceal a schema/permission/unknown failure just because an old winner exists.
        if let Err(error) = &written
            && !retryable_conflict(error)
            && !uncertain_transport(error)
        {
            return Err(ModelError::codec(error));
        }
        let keys: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.input_hash)
            .collect();
        let winners = self.read(spec, &keys).await?;
        let mut missing = Vec::new();
        for candidate in candidates {
            match winners.get(&candidate.input_hash) {
                Some(winner) if winner.admitted_tokens != candidate.admitted_tokens => {
                    return Err(ModelError::Conflict(
                        "embedding cache admitted token receipt",
                    ));
                }
                Some(_) => {}
                None => missing.push(*candidate),
            }
        }
        if !missing.is_empty() {
            match written {
                Ok(()) => return Err(ModelError::Conflict("embedding winner admission")),
                Err(error) if !retryable_conflict(&error) => {
                    // A transport acknowledgement is uncertain. Present winners are reusable;
                    // missing keys do not authorize replay of an unclassified failed write.
                    return Err(ModelError::infrastructure(Infrastructure::Transport, error));
                }
                Err(_) => {}
            }
        }
        Ok(Attempt { winners, missing })
    }

    async fn admit_batch(
        &self,
        spec: &Spec,
        candidates: &[&CacheValue],
        definition: &[u8],
    ) -> Result<BTreeMap<ContentHash, CacheValue>, ModelError> {
        let mut pending = candidates.to_vec();
        // Each statement is its own short transaction. Retry only known transient conflicts,
        // without provider work; the CacheFuture's caller still owns cancellation/deadlines.
        for _ in 0..CONFLICT_ATTEMPTS {
            let written = self.insert(spec, &pending, definition).await;
            let attempt = self.reconcile_attempt(spec, candidates, written).await?;
            if attempt.missing.is_empty() {
                return Ok(attempt.winners);
            }
            pending = attempt.missing;
        }
        Err(ModelError::infrastructure(
            Infrastructure::Contention,
            "embedding cache conflict retry limit exhausted",
        ))
    }
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Winner {
    id: RecordId,
    spec: String,
    input: String,
    definition: Bytes,
    tokens: u32,
    bytes: Bytes,
    digest: String,
}
impl EmbeddingCache for NativeEmbeddingCache {
    fn cached<'a>(
        &'a self,
        spec: &'a Spec,
        keys: &'a [ContentHash],
    ) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>> {
        Box::pin(self.read(spec, keys))
    }
    fn admit<'a>(
        &'a self,
        spec: &'a Spec,
        candidates: &'a [CacheValue],
    ) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>> {
        Box::pin(async move {
            validate_spec(spec)?;
            let definition = serde_json::to_vec(&EmbeddingSpec::new(spec)?).map_err(ModelError::codec)?;
            // Validate every proposal, including later duplicates, before any effects. The
            // compiler owns text/hash agreement; this effect owner never reconstructs text.
            let mut unique = BTreeMap::<ContentHash, &CacheValue>::new();
            for candidate in candidates {
                if candidate.admitted_tokens > spec.max_document_tokens {
                    return Err(ModelError::Invalid("embedding cache token limit".into()));
                }
                check_vector(&candidate.vector, spec.dimensions).map_err(ModelError::Invalid)?;
                if let Some(first) = unique.get(&candidate.input_hash) {
                    if first.admitted_tokens != candidate.admitted_tokens {
                        return Err(ModelError::Conflict(
                            "embedding cache duplicate token receipt",
                        ));
                    }
                } else {
                    // Differing valid vectors at the same admitted key use the first proposal.
                    unique.insert(candidate.input_hash, candidate);
                }
            }
            let candidates: Vec<_> = unique.into_values().collect();
            let mut result = BTreeMap::new();
            for batch in candidates.chunks(BATCH_ROWS) {
                result.extend(self.admit_batch(spec, batch, &definition).await?);
            }
            Ok(result)
        })
    }
}

// Vectors have the selected fixed dimension; only one bounded set of native row bodies is
// constructed at a time. The caller-supplied proposals and final trait result remain caller-sized.
const BATCH_ROWS: usize = 128;
const CONFLICT_ATTEMPTS: usize = 8;
struct Attempt<'a> {
    winners: BTreeMap<ContentHash, CacheValue>,
    missing: Vec<&'a CacheValue>,
}
fn winner_id(spec: &str, input: ContentHash) -> RecordId {
    RecordId::new("embedding_cache", format!("{spec}_{}", input.hex()))
}
fn validate_spec(spec: &Spec) -> Result<(), ModelError> {
    spec.validate().map_err(ModelError::Invalid)?;
    if spec.dimensions != 4096 {
        return Err(ModelError::Invalid(
            "native embedding cache requires selected full4096-dimensional encoder".into(),
        ));
    }
    Ok(())
}
fn retryable_conflict(error: &surrealdb::Error) -> bool {
    // Query frames preserve the typed reason. Unary gRPC commit maps Aborted through
    // status_to_error to Query(None), losing it. Recognize only the selected RocksDB Busy
    // message observed on 3.3, composed from core Error::Kvs + KVS TransactionConflict;
    // never infer retryability from generic Internal errors or a substring/retry hint.
    error.query_details() == Some(&QueryError::TransactionConflict)
        || (error.is_query()
            && error.query_details().is_none()
            && error.message() == ROCKSDB_BUSY_CONFLICT)
}
const ROCKSDB_BUSY_CONFLICT: &str = "There was a problem with the key-value store: Transaction conflict: Resource busy. This transaction can be retried";
fn uncertain_transport(error: &surrealdb::Error) -> bool {
    error.is_connection()
        && matches!(
            error.connection_details(),
            None | Some(ConnectionError::ConnectionFailed)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Credentials, reader};

    async fn fixture(label: &str) -> (NativeEmbeddingCache, String) {
        let path =
            std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned persistent fixture required");
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let database = format!("cache_internal_{label}_{}", std::process::id());
        let client = reader::connect(
            config["grpc_endpoint"].as_str().unwrap(),
            &Credentials::Root {
                username: config["admin_user"].as_str().unwrap().into(),
                password: config["admin_password"].as_str().unwrap().into(),
            },
            "cache_controls",
            &database,
        )
        .await
        .unwrap();
        (
            NativeEmbeddingCache::install(client).await.unwrap(),
            database,
        )
    }
    fn spec() -> Spec {
        serde_json::from_slice(include_bytes!(
            "../../../specs/embedding/qwen3-embedding-8b.json"
        ))
        .unwrap()
    }
    fn candidate(label: &str) -> CacheValue {
        let mut vector = vec![0.0; spec().dimensions as usize];
        vector[0] = 1.0;
        CacheValue {
            input_hash: ContentHash::of(label.as_bytes()),
            vector,
            admitted_tokens: 3,
        }
    }
    async fn close(cache: NativeEmbeddingCache, database: String) {
        cache
            .client
            .query(format!("REMOVE DATABASE {database}"))
            .await
            .unwrap()
            .check()
            .unwrap();
        cache.client.invalidate().await.unwrap();
    }
    #[test]
    fn only_source_confirmed_transaction_conflicts_allow_replay() {
        assert!(retryable_conflict(&surrealdb::Error::query(
            "conflict".into(),
            QueryError::TransactionConflict
        )));
        assert!(retryable_conflict(&surrealdb::Error::query(
            ROCKSDB_BUSY_CONFLICT.into(),
            None
        )));
        assert!(!retryable_conflict(&surrealdb::Error::query(
            "unclassified transaction conflict".into(),
            None
        )));
        assert!(!retryable_conflict(&surrealdb::Error::internal(
            ROCKSDB_BUSY_CONFLICT.into()
        )));
        assert!(!retryable_conflict(&surrealdb::Error::internal(
            "Transaction conflict: Resource busy; can be retried".into()
        )));
        assert!(!retryable_conflict(&surrealdb::Error::validation(
            "bad row".into(),
            None
        )));
        assert!(!uncertain_transport(&surrealdb::Error::connection(
            "uninitialized".into(),
            ConnectionError::Uninitialised
        )));
    }
    #[tokio::test]
    async fn lost_acknowledgement_reconciles_actual_committed_winner_without_replay() {
        let (cache, database) = fixture("ack").await;
        let spec = spec();
        let first = candidate("committed");
        cache
            .admit(&spec, std::slice::from_ref(&first))
            .await
            .unwrap();
        let mut losing = first.clone();
        losing.vector.swap(0, 1);
        // Inject the write-result failure after an actual native commit. This controls the
        // wrapper's uncertain-acknowledgement branch, not a newly claimed wire-loss probe.
        let attempt = cache
            .reconcile_attempt(
                &spec,
                &[&losing],
                Err(surrealdb::Error::connection(
                    "injected lost acknowledgement".into(),
                    None,
                )),
            )
            .await
            .unwrap();
        assert!(attempt.missing.is_empty());
        assert_eq!(
            encode_vector(&attempt.winners[&first.input_hash].vector),
            encode_vector(&first.vector)
        );
        let missing = candidate("absent");
        assert!(
            cache
                .reconcile_attempt(
                    &spec,
                    &[&missing],
                    Err(surrealdb::Error::connection(
                        "injected lost acknowledgement".into(),
                        None
                    ))
                )
                .await
                .is_err()
        );
        assert!(
            cache
                .cached(&spec, &[missing.input_hash])
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            cache
                .reconcile_attempt(
                    &spec,
                    &[&first],
                    Err(surrealdb::Error::validation(
                        "injected row refusal".into(),
                        None
                    ))
                )
                .await
                .is_err()
        );
        close(cache, database).await;
    }
    #[tokio::test]
    async fn rocksdb_grpc_commit_conflict_has_the_source_confirmed_retryable_reason() {
        let (cache, database) = fixture("conflict").await;
        cache
            .client
            .query("CREATE cache_conflict:one SET value=0")
            .await
            .unwrap()
            .check()
            .unwrap();
        let first = cache.client.as_ref().clone().begin().await.unwrap();
        let second = cache.client.as_ref().clone().begin().await.unwrap();
        first
            .query("UPDATE cache_conflict:one SET value=1")
            .await
            .unwrap()
            .check()
            .unwrap();
        second
            .query("UPDATE cache_conflict:one SET value=2")
            .await
            .unwrap()
            .check()
            .unwrap();
        first.commit().await.unwrap();
        let error = second.commit().await.unwrap_err();
        assert!(error.is_query(), "actual gRPC commit error: {error:?}");
        assert_eq!(error.query_details(), None);
        assert_eq!(error.message(), ROCKSDB_BUSY_CONFLICT);
        assert!(retryable_conflict(&error));
        close(cache, database).await;
    }
}
