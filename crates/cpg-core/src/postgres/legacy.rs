//! Explicit, read-only import of a pinned legacy Delta cache version. Never a runtime fallback.
use super::{CacheValue, Store};
use crate::{
    CoreError,
    embed::{Embedder, input_hash},
    sql,
};
use cpg_schema::{
    embedding::{EmbeddingCache, EmbeddingCacheRow, encode_vector, value_digest},
    id::IdHasher,
    table::Table,
};
use serde::Serialize;
use std::path::Path;

cpg_schema::relations! {
    inventory relations;
    legacy_vectors = "legacy_cache_vectors", deps=["embedding_cache"],
        sql="SELECT * FROM embedding_cache WHERE spec_hash=$spec AND array_has($keys,input_hash) ORDER BY input_hash".to_owned();
    legacy_counts = "legacy_cache_counts", deps=["embedding_cache"],
        sql="SELECT count(*) AS total, count(DISTINCT input_hash) AS unique_keys FROM embedding_cache WHERE spec_hash=$spec".to_owned();
}

cpg_schema::query_row! {
    struct Counts { total: i64, unique_keys: i64 }
}

#[derive(Debug, Serialize)]
pub struct ImportReceipt {
    pub source_version: u64,
    pub spec_hash: String,
    pub imported: usize,
    pub inactive_without_request: usize,
    pub import_digest: String,
}

pub async fn import(
    db: &Store,
    root: &Path,
    version: u64,
    embedder: &dyn Embedder,
    requests: &[String],
) -> Result<ImportReceipt, CoreError> {
    let spec = embedder.spec();
    let table = crate::snapshot::load_at(root, EmbeddingCache::NAME, version).await?;
    crate::delta::verify::<EmbeddingCache>(&table)?;
    let ctx = crate::snapshot::empty_session();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(EmbeddingCache::NAME, table.table_provider().await?)?;
    let counts: Vec<Counts> = sql::fetch(
        &ctx,
        &legacy_counts(),
        sql::Params::new().digest("spec", spec.hash()),
    )
    .await?;
    let count = &counts[0];
    if count.total != count.unique_keys {
        return Err(CoreError::Embed("duplicate legacy cache key".to_owned()));
    }
    let requested: std::collections::BTreeMap<_, _> = requests
        .iter()
        .map(|text| (input_hash(text), text))
        .collect();
    db.ensure_spec(spec).await?;
    let mut h = IdHasher::new("legacy-embedding-import/v1");
    h.digest_field(spec.hash()).i64(version as i64);
    let mut imported = 0;
    // Transfer at most 32 vectors at once; inactive rows are counted without reading payloads.
    let keys: Vec<_> = requested.keys().copied().collect();
    for chunk in keys.chunks(32) {
        let rows: Vec<EmbeddingCacheRow> = sql::fetch(
            &ctx,
            &legacy_vectors(),
            sql::Params::new()
                .digest("spec", spec.hash())
                .digests("keys", chunk.iter().copied()),
        )
        .await?;
        for row in rows {
            let text = requested[&row.input_hash];
            if row.model != spec.model {
                return Err(CoreError::Embed(
                    "legacy cache model disagrees with spec".to_owned(),
                ));
            }
            let tokens = embedder.count_tokens(text).await?;
            if tokens > spec.max_document_tokens as usize {
                return Err(CoreError::Embed(
                    "legacy request exceeds tokenizer cap".to_owned(),
                ));
            }
            let candidate = CacheValue {
                input_hash: row.input_hash,
                vector: row.vector,
                admitted_tokens: tokens as u32,
            };
            let winners = db.admit(spec, std::slice::from_ref(&candidate)).await?;
            if encode_vector(&winners[&candidate.input_hash].vector)
                != encode_vector(&candidate.vector)
            {
                return Err(CoreError::Embed(
                    "legacy import conflicts with a committed PostgreSQL value".to_owned(),
                ));
            }
            h.digest_field(candidate.input_hash)
                .digest_field(value_digest(&candidate.vector));
            imported += 1;
        }
    }
    Ok(ImportReceipt {
        source_version: version,
        spec_hash: spec.hash().hex(),
        imported,
        inactive_without_request: count.total as usize - imported,
        import_digest: h.finish_digest().hex(),
    })
}
