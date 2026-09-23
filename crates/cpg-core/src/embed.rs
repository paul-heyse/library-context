//! Embeddings through the cache (DESIGN §11.1, §B14; ADR-0010, ADR-0017 amendment).
//!
//! One hashed spec governs every vector. Vectors are cached in the global `embedding_cache` by
//! `(spec_hash, input_hash)`, where `input_hash` is the SHA-256 of the exact request text. An
//! attempt embeds only the keys the cache lacks, inserts them by an insert-only MERGE, and records
//! the cache version it read. The embedder is a trait here so that `reqwest` stays out of
//! `cpg-core`: `lctx-embed` implements it over vLLM, and the deterministic fake here keeps tests
//! and GPU-free runs working.
//!
//! Serving-side digests are SHA-256 (Python recomputes them); store-side ids stay BLAKE3.

use std::future::Future;
use std::pin::Pin;

use arrow_array::cast::AsArray;
use cpg_schema::embedding::{EmbeddingCache, EmbeddingCacheRow};
use cpg_schema::findings::BriefDocumentsRow;
use cpg_schema::id::{Digest, Id, IdHasher};
use cpg_schema::table::Table;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::CoreError;

/// A future an embedder returns.
pub type EmbedFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, CoreError>> + Send + 'a>>;

/// The embedding spec (DESIGN §11.1): everything that can change a vector. Its SHA-256 over the
/// canonical JSON (fields in this order, no whitespace) is the spec hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub model: String,
    pub revision: String,
    pub tokenizer_revision: String,
    /// The serving engine and version.
    pub server: String,
    pub served_dtype: String,
    pub pooling: String,
    pub query_template: String,
    pub query_task: String,
    pub document_template: String,
    pub dimensions: u32,
    pub output_dtype: String,
    pub normalization: String,
    /// The §11.1 cap on a document, in the model's tokens.
    pub max_document_tokens: u32,
}

impl Spec {
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("a spec always serializes")
    }

    /// SHA-256 of the canonical JSON.
    pub fn hash(&self) -> Digest {
        Digest(Sha256::digest(self.canonical_json().as_bytes()).into())
    }

    /// The request text of a document (§11.1: documents take no prefix).
    pub fn document_text(&self, text: &str) -> String {
        self.document_template.replace("{text}", text)
    }

    /// The request text of a query: `Instruct: {task}\nQuery:{query}`, with no space after
    /// `Query:` (E1).
    pub fn query_text(&self, query: &str) -> String {
        self.query_template
            .replace("{task_description}", &self.query_task)
            .replace("{query}", query)
    }
}

/// The cache key of a request text: its SHA-256.
pub fn input_hash(request_text: &str) -> Digest {
    Digest(Sha256::digest(request_text.as_bytes()).into())
}

/// An embedding service under one spec.
pub trait Embedder: Send + Sync {
    fn spec(&self) -> &Spec;
    /// The number of the served model's tokens in a request text.
    fn count_tokens<'a>(&'a self, request_text: &'a str) -> EmbedFuture<'a, usize>;
    /// One vector per request text, in order, each checked ([`check_vector`]).
    fn embed<'a>(&'a self, request_texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>>;
}

/// §11.1's rejections of a returned vector: its length, finiteness and unit norm.
pub fn check_vector(v: &[f32], dimensions: u32) -> Result<(), String> {
    if v.len() != dimensions as usize {
        return Err(format!("{} dimensions, not {dimensions}", v.len()));
    }
    if v.iter().any(|x| !x.is_finite()) {
        return Err("a non-finite component".to_owned());
    }
    let norm: f64 = v
        .iter()
        .map(|x| f64::from(*x) * f64::from(*x))
        .sum::<f64>()
        .sqrt();
    if (norm - 1.0).abs() > 1e-3 {
        return Err(format!("norm {norm}, not 1"));
    }
    Ok(())
}

/// A deterministic fake embedder with its own spec (DESIGN §11.1): each vector is a unit vector
/// drawn by splitmix64 from the first 8 bytes (little-endian) of the request text's SHA-256, so
/// the Python server's fake twin reproduces it bit for bit with its standard library. Tests and
/// `just check` use it; a fake vector never stands in for a live one in a `passed` claim.
pub struct FakeEmbedder {
    spec: Spec,
}

impl FakeEmbedder {
    pub fn new() -> Self {
        Self {
            spec: Spec {
                model: "lctx-fake-embedder".to_owned(),
                revision: "1".to_owned(),
                tokenizer_revision: "bytes/4".to_owned(),
                server: "in-process".to_owned(),
                served_dtype: "float32".to_owned(),
                pooling: "none".to_owned(),
                query_template: "Instruct: {task_description}\nQuery:{query}".to_owned(),
                query_task: "Given a coding task, retrieve capability briefs of a Python \
                             library that solve it"
                    .to_owned(),
                document_template: "{text}".to_owned(),
                dimensions: 4096,
                output_dtype: "float32".to_owned(),
                normalization: "l2".to_owned(),
                max_document_tokens: 2048,
            },
        }
    }

    /// The fake vector of a request text.
    pub fn vector(&self, request_text: &str) -> Vec<f32> {
        let seed = Sha256::digest(request_text.as_bytes());
        let mut state = u64::from_le_bytes(seed[..8].try_into().expect("8 bytes"));
        let mut raw: Vec<f64> = (0..self.spec.dimensions)
            .map(|_| {
                // splitmix64
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
            })
            .collect();
        let norm = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
        raw.iter_mut().for_each(|x| *x /= norm);
        raw.into_iter().map(|x| x as f32).collect()
    }
}

impl Default for FakeEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl Embedder for FakeEmbedder {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn count_tokens<'a>(&'a self, request_text: &'a str) -> EmbedFuture<'a, usize> {
        Box::pin(async move { Ok(request_text.len().div_ceil(4)) })
    }

    fn embed<'a>(&'a self, request_texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move { Ok(request_texts.iter().map(|t| self.vector(t)).collect()) })
    }
}

/// What an attempt's embedding step recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Embedded {
    pub spec_hash: Digest,
    /// The cache version read after the attempt's merge.
    pub version: u64,
    /// Distinct keys the snapshot uses.
    pub used: i64,
    /// A digest of the sorted keys used (`content_digest`'s input, never the shared version).
    pub keys_digest: Digest,
}

const BATCH: usize = 32;

/// Embed the brief documents through the cache: fill each document's `input_hash`, check the token
/// cap, embed the keys the cache lacks, merge them, and return the cache version and keys used.
pub async fn embed_documents(
    root: &std::path::Path,
    snapshot_id: Id,
    embedder: &dyn Embedder,
    docs: &mut [BriefDocumentsRow],
) -> Result<Embedded, CoreError> {
    let spec = embedder.spec();
    let spec_hash = spec.hash();
    let mut requests: Vec<(cpg_schema::id::Digest, String)> = Vec::new();
    for doc in docs.iter_mut() {
        let request = spec.document_text(&doc.text);
        let tokens = embedder.count_tokens(&request).await?;
        if tokens > spec.max_document_tokens as usize {
            return Err(CoreError::Embed(format!(
                "a brief document is {tokens} tokens, over the {}-token cap; splitting by \
                 applicable case arrives in increment 2",
                spec.max_document_tokens
            )));
        }
        let key = input_hash(&request);
        doc.input_hash = Some(key);
        requests.push((key, request));
    }
    requests.sort();
    requests.dedup_by(|a, b| a.0 == b.0);

    let have = cached_keys(root, spec_hash, requests.iter().map(|r| r.0)).await?;
    let missing: Vec<&(Digest, String)> =
        requests.iter().filter(|r| !have.contains(&r.0)).collect();
    let mut rows = Vec::new();
    for chunk in missing.chunks(BATCH) {
        let texts: Vec<String> = chunk.iter().map(|r| r.1.clone()).collect();
        let vectors = embedder.embed(&texts).await?;
        if vectors.len() != texts.len() {
            return Err(CoreError::Embed(format!(
                "{} vectors for {} texts",
                vectors.len(),
                texts.len()
            )));
        }
        for ((key, _), vector) in chunk.iter().copied().zip(vectors) {
            check_vector(&vector, spec.dimensions).map_err(CoreError::Embed)?;
            rows.push(EmbeddingCacheRow {
                spec_hash,
                input_hash: *key,
                vector,
                model: spec.model.clone(),
            });
        }
    }
    let batch = EmbeddingCache::to_sorted_batch(&rows)?;
    let version = crate::delta::merge_global::<EmbeddingCache>(root, batch, snapshot_id).await?;
    let mut h = IdHasher::new("embedding-keys");
    h.digest_field(spec_hash);
    for (key, _) in &requests {
        h.digest_field(*key);
    }
    Ok(Embedded {
        spec_hash,
        version,
        used: requests.len() as i64,
        keys_digest: h.finish_digest(),
    })
}

/// The keys of `spec_hash` the cache holds among `wanted`, read at its latest version over all
/// its files (the global read mode).
async fn cached_keys(
    root: &std::path::Path,
    spec_hash: Digest,
    wanted: impl Iterator<Item = Digest>,
) -> Result<std::collections::BTreeSet<Digest>, CoreError> {
    let wanted: Vec<String> = wanted.map(|k| format!("X'{}'", k.hex())).collect();
    let mut out = std::collections::BTreeSet::new();
    if wanted.is_empty() || !root.join(EmbeddingCache::NAME).join("_delta_log").exists() {
        return Ok(out);
    }
    let table = crate::delta::open_verified::<EmbeddingCache>(root).await?;
    let ctx = crate::snapshot::empty_session();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(EmbeddingCache::NAME, table.table_provider().await?)?;
    let query = format!(
        "SELECT input_hash FROM embedding_cache WHERE spec_hash = X'{}' AND input_hash IN ({})",
        spec_hash.hex(),
        wanted.join(", ")
    );
    for b in crate::sql::query(&ctx, &query).await?.collect().await? {
        let col = arrow_cast::cast(b.column(0), &arrow_schema::DataType::Binary)?;
        for v in col.as_binary::<i32>().iter().flatten() {
            if let Ok(bytes) = <[u8; 32]>::try_from(v) {
                out.insert(Digest(bytes));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_embedder_is_deterministic_unit_and_distinct() {
        let f = FakeEmbedder::new();
        let a = f.vector("alpha");
        assert_eq!(a, f.vector("alpha"));
        assert_ne!(a, f.vector("beta"));
        check_vector(&a, 4096).unwrap();
        assert!(check_vector(&a[..10], 4096).is_err());
        let mut nan = a.clone();
        nan[0] = f32::NAN;
        assert!(check_vector(&nan, 4096).is_err());
        assert!(check_vector(&vec![0.5; 4096], 4096).is_err());
    }

    #[test]
    fn the_spec_hash_is_the_canonical_json_and_templates_apply() {
        let f = FakeEmbedder::new();
        let s = f.spec();
        assert_eq!(
            s.hash().0,
            <[u8; 32]>::from(Sha256::digest(s.canonical_json().as_bytes()))
        );
        assert_eq!(
            s.query_text("add a tool"),
            format!("Instruct: {}\nQuery:add a tool", s.query_task)
        );
        assert_eq!(s.document_text("x"), "x");
    }
}
