//! Immutable shared embedding winners, hosted outside private snapshot databases.
use lctx_model::domain::{
    ContentHash, ModelError,
    embedding::{
        Spec,
        cache::{CacheFuture, CacheValue, EmbeddingCache},
        check_vector,
        value::{encode_vector, value_digest},
    },
};
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Bytes, RecordId, SurrealValue, Variables},
};
#[derive(Clone)]
pub struct NativeEmbeddingCache {
    client: Arc<Surreal<Client>>,
}
impl NativeEmbeddingCache {
    pub async fn install(client: Arc<Surreal<Client>>) -> Result<Self, ModelError> {
        client.query("DEFINE TABLE IF NOT EXISTS embedding_cache TYPE NORMAL SCHEMAFULL; DEFINE FIELD IF NOT EXISTS spec ON embedding_cache TYPE string; DEFINE FIELD IF NOT EXISTS input ON embedding_cache TYPE string; DEFINE FIELD IF NOT EXISTS definition ON embedding_cache TYPE bytes; DEFINE FIELD IF NOT EXISTS tokens ON embedding_cache TYPE int ASSERT $value >= 0; DEFINE FIELD IF NOT EXISTS vector ON embedding_cache TYPE array<float,1024>; DEFINE FIELD IF NOT EXISTS bytes ON embedding_cache TYPE bytes; DEFINE FIELD IF NOT EXISTS digest ON embedding_cache TYPE string; DEFINE INDEX IF NOT EXISTS winner ON embedding_cache FIELDS spec,input UNIQUE;").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(Self { client })
    }
    async fn read(
        &self,
        spec: &Spec,
        keys: &[ContentHash],
    ) -> Result<BTreeMap<ContentHash, CacheValue>, ModelError> {
        spec.validate().map_err(ModelError::Invalid)?;
        if spec.dimensions != 1024 {
            return Err(ModelError::Invalid(
                "native embedding cache requires selected 1024-dimensional specification".into(),
            ));
        }
        if keys.is_empty() {
            return Ok(BTreeMap::new());
        }
        let mut bindings = Variables::new();
        bindings.insert("spec", spec.hash().hex());
        bindings.insert("inputs", keys.iter().map(|k| k.hex()).collect::<Vec<_>>());
        let mut response=self.client.query("SELECT * FROM embedding_cache WHERE spec=$spec AND input IN $inputs ORDER BY input").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows: Vec<Winner> = response.take(0).map_err(ModelError::codec)?;
        let definition = serde_json::to_vec(spec).map_err(ModelError::codec)?;
        let mut result = BTreeMap::new();
        for row in rows {
            if row.spec != spec.hash().hex()
                || row.definition.as_ref() != definition
                || row.tokens > spec.max_document_tokens
            {
                return Err(ModelError::Conflict(
                    "embedding cache specification/token admission",
                ));
            }
            let hash = ContentHash(
                hex::decode(&row.input)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("embedding input hash"))?,
            );
            check_vector(&row.vector, spec.dimensions).map_err(ModelError::Invalid)?;
            if encode_vector(&row.vector) != row.bytes.as_ref()
                || value_digest(&row.vector).hex() != row.digest
            {
                return Err(ModelError::Conflict("embedding cache winning bytes"));
            }
            if result
                .insert(
                    hash,
                    CacheValue {
                        input_hash: hash,
                        vector: row.vector,
                        admitted_tokens: row.tokens,
                    },
                )
                .is_some()
            {
                return Err(ModelError::Conflict("duplicate embedding winner"));
            }
        }
        Ok(result)
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
    vector: Vec<f32>,
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
            spec.validate().map_err(ModelError::Invalid)?;
            if spec.dimensions != 1024 {
                return Err(ModelError::Invalid(
                    "native embedding cache requires selected 1024-dimensional specification"
                        .into(),
                ));
            }
            let definition = serde_json::to_vec(spec).map_err(ModelError::codec)?;
            let mut keys = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                if candidate.admitted_tokens > spec.max_document_tokens {
                    return Err(ModelError::Invalid("embedding cache token limit".into()));
                }
                check_vector(&candidate.vector, spec.dimensions).map_err(ModelError::Invalid)?;
                keys.push(candidate.input_hash);
                let winner = Winner {
                    id: RecordId::new(
                        "embedding_cache",
                        format!("{}_{}", spec.hash().hex(), candidate.input_hash.hex()),
                    ),
                    spec: spec.hash().hex(),
                    input: candidate.input_hash.hex(),
                    definition: Bytes::from(definition.clone()),
                    tokens: candidate.admitted_tokens,
                    vector: candidate.vector.clone(),
                    bytes: Bytes::from(encode_vector(&candidate.vector)),
                    digest: value_digest(&candidate.vector).hex(),
                };
                // INSERT IGNORE never updates an existing immutable winner. Every submitted statement is checked.
                let mut bindings = Variables::new();
                bindings.insert("winner", winner);
                self.client
                    .query("INSERT IGNORE INTO embedding_cache $winner")
                    .bind(bindings)
                    .await
                    .map_err(ModelError::codec)?
                    .check()
                    .map_err(ModelError::codec)?;
            }
            let result = self.read(spec, &keys).await?;
            if keys.iter().any(|key| !result.contains_key(key)) {
                return Err(ModelError::Conflict("embedding winner admission"));
            }
            Ok(result)
        })
    }
}
