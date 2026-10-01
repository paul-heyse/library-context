use std::collections::BTreeMap;

use super::{Error, Store};
use lctx_model::domain::embedding::{Spec, check_vector};
use lctx_model::domain::{
    ContentHash,
    embedding::value::{VALUE_CODEC, decode_vector, encode_vector, value_digest},
};

#[derive(Clone, Debug)]
pub struct CacheValue {
    pub input_hash: ContentHash,
    pub vector: Vec<f32>,
    pub admitted_tokens: u32,
}

#[derive(sqlx::FromRow)]
struct StoredValue {
    input_hash: Vec<u8>,
    codec: i16,
    dimensions: i32,
    vector_bytes: Vec<u8>,
    value_digest: Vec<u8>,
    admitted_tokens: i32,
}

impl Store {
    pub async fn ensure_spec(&self, spec: &Spec) -> Result<(), Error> {
        spec.validate()
            .map_err(|_| Error::Integrity("embedding specification"))?;
        let dimensions =
            i32::try_from(spec.dimensions).map_err(|_| Error::Integrity("dimensions"))?;
        if !(1..=65536).contains(&dimensions) {
            return Err(Error::Integrity("dimensions"));
        }
        let hash = spec.hash();
        let canonical = spec.canonical_json();
        sqlx::query("INSERT INTO lctx_cache.specs(spec_hash, canonical_spec, dimensions) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(hash.0.as_slice()).bind(&canonical).bind(dimensions).execute(&self.pool).await?;
        // A separate READ COMMITTED statement sees the winner after a concurrent insert waits.
        let existing: (String, i32) = sqlx::query_as(
            "SELECT canonical_spec, dimensions FROM lctx_cache.specs WHERE spec_hash=$1",
        )
        .bind(hash.0.as_slice())
        .fetch_one(&self.pool)
        .await?;
        if existing != (canonical, dimensions) {
            return Err(Error::Integrity("canonical spec conflict"));
        }
        Ok(())
    }

    pub async fn cached(
        &self,
        spec: &Spec,
        keys: &[ContentHash],
    ) -> Result<BTreeMap<ContentHash, CacheValue>, Error> {
        let started = std::time::Instant::now();
        let mut values = BTreeMap::new();
        for chunk in keys.chunks(128) {
            let wanted: Vec<Vec<u8>> = chunk.iter().map(|k| k.0.to_vec()).collect();
            let hash = spec.hash();
            let rows: Vec<StoredValue> = sqlx::query_as("SELECT input_hash, codec, dimensions, vector_bytes, value_digest, admitted_tokens FROM lctx_cache.embedding_values WHERE spec_hash=$1 AND input_hash=ANY($2)")
                .bind(hash.0.as_slice()).bind(&wanted).fetch_all(&self.pool).await?;
            for row in rows {
                if row.codec != VALUE_CODEC
                    || row.dimensions != spec.dimensions as i32
                    || row.admitted_tokens < 0
                    || row.admitted_tokens as u32 > spec.max_document_tokens
                {
                    return Err(Error::Integrity("cached admission metadata"));
                }
                let vector = decode_vector(&row.vector_bytes, spec.dimensions)
                    .map_err(|_| Error::Integrity("cached vector encoding"))?;
                check_vector(&vector, spec.dimensions)
                    .map_err(|_| Error::Integrity("cached vector shape or norm"))?;
                if row.value_digest.as_slice() != value_digest(&vector).0 {
                    return Err(Error::Integrity("cached value digest"));
                }
                let input_hash = ContentHash(
                    row.input_hash
                        .try_into()
                        .map_err(|_| Error::Integrity("input hash length"))?,
                );
                values.insert(
                    input_hash,
                    CacheValue {
                        input_hash,
                        vector,
                        admitted_tokens: row.admitted_tokens as u32,
                    },
                );
            }
        }
        tracing::info!(target: "lctx::postgres", operation="cache_read", requested=keys.len(), hits=values.len(), vector_bytes=values.len()*spec.dimensions as usize*4, seconds=started.elapsed().as_secs_f64());
        Ok(values)
    }

    /// Idempotent insert plus a new-statement read. On connection/commit ambiguity inspect the
    /// committed keys before retrying. Never hold a pool lease across an embedder request.
    pub async fn admit(
        &self,
        spec: &Spec,
        values: &[CacheValue],
    ) -> Result<BTreeMap<ContentHash, CacheValue>, Error> {
        for value in values {
            check_vector(&value.vector, spec.dimensions)
                .map_err(|_| Error::Integrity("candidate vector"))?;
            if value.admitted_tokens > spec.max_document_tokens {
                return Err(Error::Integrity("candidate token cap"));
            }
        }
        let keys: Vec<_> = values
            .iter()
            .map(|v| v.input_hash)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut last = None;
        for attempt in 0..3 {
            if attempt > 0 {
                match self.cached(spec, &keys).await {
                    Ok(found) if found.len() == keys.len() => return Ok(found),
                    Ok(_) => {}
                    Err(e) if e.retryable() => {}
                    Err(e) => return Err(e),
                }
                tokio::time::sleep(std::time::Duration::from_millis(50 * attempt)).await;
            }
            match self.insert_values(spec, values).await {
                Ok(()) => {
                    let found = match self.cached(spec, &keys).await {
                        Ok(found) => found,
                        Err(e) if e.retryable() => {
                            last = Some(e);
                            continue;
                        }
                        Err(e) => return Err(e),
                    };
                    if found.len() != keys.len() {
                        return Err(Error::Integrity("committed cache omitted a requested key"));
                    }
                    return Ok(found);
                }
                Err(e) if e.retryable() => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or(Error::Integrity("cache retry exhausted")))
    }

    async fn insert_values(&self, spec: &Spec, values: &[CacheValue]) -> Result<(), Error> {
        let started = std::time::Instant::now();
        let mut inserted = 0;
        for chunk in values.chunks(32) {
            let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                "INSERT INTO lctx_cache.embedding_values (spec_hash,input_hash,codec,dimensions,vector_bytes,value_digest,admitted_tokens) ",
            );
            query.push_values(chunk, |mut row, value| {
                row.push_bind(spec.hash().0.to_vec())
                    .push_bind(value.input_hash.0.to_vec())
                    .push_bind(VALUE_CODEC)
                    .push_bind(spec.dimensions as i32)
                    .push_bind(encode_vector(&value.vector))
                    .push_bind(value_digest(&value.vector).0.to_vec())
                    .push_bind(value.admitted_tokens as i32);
            });
            inserted += query
                .push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&self.pool)
                .await?
                .rows_affected();
        }
        tracing::info!(target: "lctx::postgres", operation="cache_admit", candidates=values.len(), inserted, vector_bytes=values.len()*spec.dimensions as usize*4, seconds=started.elapsed().as_secs_f64());
        Ok(())
    }
}
