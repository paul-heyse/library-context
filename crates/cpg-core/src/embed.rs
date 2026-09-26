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
/// cap, embed the keys the cache lacks, merge them, and return the cache version and the keys the
/// snapshot uses (the documents' and `prior`, the analytics' E0 keys).
pub async fn embed_documents(
    root: &std::path::Path,
    snapshot_id: Id,
    embedder: &dyn Embedder,
    docs: &mut [BriefDocumentsRow],
    prior: &[Digest],
) -> Result<Embedded, CoreError> {
    let spec = embedder.spec();
    let spec_hash = spec.hash();
    let mut requests: Vec<(cpg_schema::id::Digest, String)> = Vec::new();
    for doc in docs.iter_mut() {
        let request = spec.document_text(&doc.text);
        let key = input_hash(&request);
        doc.spec_hash = Some(spec_hash);
        doc.input_hash = Some(key);
        requests.push((key, request));
    }
    requests.sort();
    requests.dedup_by(|a, b| a.0 == b.0);
    let (_, version) = fill_cache(root, snapshot_id, embedder, &requests).await?;
    // The snapshot's keys: its documents' and the analytics' (E0, slice 3.1), sorted and distinct.
    let mut keys: Vec<Digest> = requests
        .iter()
        .map(|(k, _)| *k)
        .chain(prior.iter().copied())
        .collect();
    keys.sort();
    keys.dedup();
    let mut h = IdHasher::new("embedding-keys");
    h.digest_field(spec_hash);
    for key in &keys {
        h.digest_field(*key);
    }
    Ok(Embedded {
        spec_hash,
        version,
        used: keys.len() as i64,
        keys_digest: h.finish_digest(),
    })
}

/// Embed document texts through the cache for the analytics (E0; DESIGN §9.7, slice 3.1): each
/// text's vector in order, read from the cache or embedded and merged by the insert-only MERGE,
/// with the distinct keys used and the cache version after the merge. A text is embedded as a
/// document (the spec's document template); its caller keeps it under the document cap.
pub async fn embed_texts(
    root: &std::path::Path,
    snapshot_id: Id,
    embedder: &dyn Embedder,
    texts: &[String],
) -> Result<(Vec<Vec<f32>>, Vec<Digest>, u64), CoreError> {
    let spec = embedder.spec();
    let requests: Vec<(Digest, String)> = texts
        .iter()
        .map(|t| {
            let request = spec.document_text(t);
            (input_hash(&request), request)
        })
        .collect();
    let (vectors, version) = fill_cache(root, snapshot_id, embedder, &requests).await?;
    let out = requests
        .iter()
        .map(|(k, _)| vectors.get(k).cloned())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| CoreError::Embed("a text without a vector".to_owned()))?;
    Ok((out, vectors.into_keys().collect(), version))
}

/// The single cache-fill contract: admit missing requests with the real tokenizer, merge only
/// validated vectors, and return the winning rows at precisely the committed Delta version.
async fn fill_cache(
    root: &std::path::Path,
    snapshot_id: Id,
    embedder: &dyn Embedder,
    requests: &[(Digest, String)],
) -> Result<(std::collections::BTreeMap<Digest, Vec<f32>>, u64), CoreError> {
    let spec = embedder.spec();
    let spec_hash = spec.hash();
    let unique: std::collections::BTreeMap<Digest, &String> =
        requests.iter().map(|(key, text)| (*key, text)).collect();
    let have = cached_keys(root, spec_hash, unique.keys().copied()).await?;
    let missing: Vec<(Digest, &String)> = unique
        .iter()
        .filter(|(key, _)| !have.contains(*key))
        .map(|(key, text)| (*key, *text))
        .collect();
    // A cached key already passed this same spec's tokenizer at insertion. No service is needed
    // for a fully cached compile; every new path through the cache shares this check.
    for (_, request) in &missing {
        let tokens = embedder.count_tokens(request).await?;
        if tokens > spec.max_document_tokens as usize {
            return Err(CoreError::Embed(format!(
                "a document request is {tokens} tokens, over the {}-token cap",
                spec.max_document_tokens
            )));
        }
    }
    let (rows, failed) = embed_batches(embedder, &missing).await;
    // Completed batches survive a later failure; the insert-only merge makes retry idempotent.
    let batch = EmbeddingCache::to_sorted_batch(&rows)?;
    let version = crate::delta::merge_global::<EmbeddingCache>(root, batch, snapshot_id).await?;
    if let Some(error) = failed {
        return Err(error);
    }
    let vectors = cached_vectors(root, spec_hash, version, unique.keys().copied()).await?;
    if vectors.len() != unique.len() {
        return Err(CoreError::Embed(
            "committed embedding cache omitted a requested key".to_owned(),
        ));
    }
    Ok((vectors, version))
}

/// Embed `(key, request text)` pairs in batches of [`BATCH`], checking each vector: the cache rows
/// of every batch that completed, and the first failure, after which nothing more is sent.
async fn embed_batches(
    embedder: &dyn Embedder,
    requests: &[(Digest, &String)],
) -> (Vec<EmbeddingCacheRow>, Option<CoreError>) {
    let spec = embedder.spec();
    let spec_hash = spec.hash();
    let mut rows = Vec::new();
    for chunk in requests.chunks(BATCH) {
        let texts: Vec<String> = chunk.iter().map(|(_, r)| (*r).clone()).collect();
        let vectors = match embedder.embed(&texts).await {
            Ok(v) if v.len() == texts.len() => v,
            Ok(v) => {
                let e = format!("{} vectors for {} texts", v.len(), texts.len());
                return (rows, Some(CoreError::Embed(e)));
            }
            Err(e) => return (rows, Some(e)),
        };
        let mut checked = Vec::with_capacity(vectors.len());
        for ((key, _), vector) in chunk.iter().zip(vectors) {
            if let Err(e) = check_vector(&vector, spec.dimensions) {
                return (rows, Some(CoreError::Embed(e)));
            }
            checked.push(EmbeddingCacheRow {
                spec_hash,
                input_hash: *key,
                vector,
                model: spec.model.clone(),
            });
        }
        rows.extend(checked);
    }
    (rows, None)
}

// The embedding cache's relations (the holistic assessment's A4): the spec and keys bound, never
// spliced as `X'…'` literals.
cpg_schema::relations! {
    inventory relations;
    /// Every cached vector of one spec (`$spec`).
    cache_relation = "embedding_cache_vectors",
        deps = ["embedding_cache"],
        sql = "SELECT input_hash, vector FROM embedding_cache \
               WHERE spec_hash = $spec AND array_has($keys, input_hash)".to_owned();
    /// The keys of one spec (`$spec`) the cache holds among `$keys`.
    keys_relation = "embedding_cache_keys",
        deps = ["embedding_cache"],
        sql = "SELECT input_hash FROM embedding_cache \
               WHERE spec_hash = $spec AND array_has($keys, input_hash)"
            .to_owned();
}

cpg_schema::query_row! {
    struct CachedRow {
        input_hash: Digest,
        vector: Vec<f32>,
    }
}

cpg_schema::query_row! {
    struct KeyRow {
        input_hash: Digest,
    }
}

/// Requested vectors of `spec_hash` at exactly the version returned by the merge.
async fn cached_vectors(
    root: &std::path::Path,
    spec_hash: Digest,
    version: u64,
    wanted: impl Iterator<Item = Digest>,
) -> Result<std::collections::BTreeMap<Digest, Vec<f32>>, CoreError> {
    let mut out = std::collections::BTreeMap::new();
    let wanted: Vec<Digest> = wanted.collect();
    if wanted.is_empty() {
        return Ok(out);
    }
    let table = crate::snapshot::load_at(root, EmbeddingCache::NAME, version).await?;
    crate::delta::verify::<EmbeddingCache>(&table)?;
    let ctx = crate::snapshot::empty_session();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(EmbeddingCache::NAME, table.table_provider().await?)?;
    for r in crate::sql::fetch::<CachedRow>(
        &ctx,
        &cache_relation(),
        crate::sql::Params::new()
            .digest("spec", spec_hash)
            .digests("keys", wanted),
    )
    .await?
    {
        out.insert(r.input_hash, r.vector);
    }
    Ok(out)
}

/// The keys of `spec_hash` the cache holds among `wanted`, read at its latest version over all
/// its files (the global read mode).
async fn cached_keys(
    root: &std::path::Path,
    spec_hash: Digest,
    wanted: impl Iterator<Item = Digest>,
) -> Result<std::collections::BTreeSet<Digest>, CoreError> {
    let wanted: Vec<Digest> = wanted.collect();
    let mut out = std::collections::BTreeSet::new();
    if wanted.is_empty() || !root.join(EmbeddingCache::NAME).join("_delta_log").exists() {
        return Ok(out);
    }
    let table = crate::delta::open_verified::<EmbeddingCache>(root).await?;
    let ctx = crate::snapshot::empty_session();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(EmbeddingCache::NAME, table.table_provider().await?)?;
    for r in crate::sql::fetch::<KeyRow>(
        &ctx,
        &keys_relation(),
        crate::sql::Params::new()
            .digest("spec", spec_hash)
            .digests("keys", wanted),
    )
    .await?
    {
        out.insert(r.input_hash);
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

    /// The fake embedder, counting what it is asked, and failing its `fail_on`-th batch.
    struct Probe {
        fake: FakeEmbedder,
        counted: std::sync::atomic::AtomicUsize,
        embedded: std::sync::atomic::AtomicUsize,
        batches: std::sync::atomic::AtomicUsize,
        fail_on: Option<usize>,
    }

    impl Probe {
        fn new(fail_on: Option<usize>) -> Self {
            Probe {
                fake: FakeEmbedder::new(),
                counted: Default::default(),
                embedded: Default::default(),
                batches: Default::default(),
                fail_on,
            }
        }
        fn get(n: &std::sync::atomic::AtomicUsize) -> usize {
            n.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl Embedder for Probe {
        fn spec(&self) -> &Spec {
            self.fake.spec()
        }
        fn count_tokens<'a>(&'a self, request_text: &'a str) -> EmbedFuture<'a, usize> {
            self.counted
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.fake.count_tokens(request_text)
        }
        fn embed<'a>(&'a self, request_texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
            let n = self
                .batches
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            if self.fail_on == Some(n) {
                return Box::pin(async {
                    Err(CoreError::Embed("the service went down".to_owned()))
                });
            }
            self.embedded
                .fetch_add(request_texts.len(), std::sync::atomic::Ordering::SeqCst);
            self.fake.embed(request_texts)
        }
    }

    fn document(text: &str) -> BriefDocumentsRow {
        BriefDocumentsRow {
            snapshot_id: Id([1; 16]),
            brief_id: Id([2; 16]),
            chunk: 0,
            text: text.to_owned(),
            spec_hash: None,
            input_hash: None,
        }
    }

    /// The holistic assessment's D3: a cached document is neither counted nor embedded again, so
    /// a fully cached compile needs no service.
    #[tokio::test]
    async fn only_documents_the_cache_lacks_are_counted_and_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let first = Probe::new(None);
        let mut docs = vec![document("alpha"), document("beta")];
        embed_documents(dir.path(), Id([1; 16]), &first, &mut docs, &[])
            .await
            .unwrap();
        assert_eq!(Probe::get(&first.counted), 2);
        let second = Probe::new(None);
        let mut again = vec![document("alpha"), document("beta"), document("gamma")];
        embed_documents(dir.path(), Id([3; 16]), &second, &mut again, &[])
            .await
            .unwrap();
        assert_eq!(Probe::get(&second.counted), 1);
        assert_eq!(Probe::get(&second.embedded), 1);
        let cached = Probe::new(None);
        let mut all = vec![document("gamma"), document("alpha")];
        embed_documents(dir.path(), Id([4; 16]), &cached, &mut all, &[])
            .await
            .unwrap();
        assert_eq!(Probe::get(&cached.counted) + Probe::get(&cached.batches), 0);
    }

    /// D3: when a later batch fails, the batches before it are merged, so a rerun embeds only
    /// what is still missing.
    #[tokio::test]
    async fn completed_batches_survive_a_later_failure() {
        let dir = tempfile::tempdir().unwrap();
        let texts: Vec<String> = (0..BATCH + 8).map(|i| format!("text {i}")).collect();
        let failing = Probe::new(Some(2));
        assert!(
            embed_texts(dir.path(), Id([1; 16]), &failing, &texts)
                .await
                .is_err()
        );
        let rerun = Probe::new(None);
        let (vectors, keys, _) = embed_texts(dir.path(), Id([2; 16]), &rerun, &texts)
            .await
            .unwrap();
        assert_eq!(Probe::get(&rerun.embedded), 8);
        assert_eq!((vectors.len(), keys.len()), (BATCH + 8, BATCH + 8));
    }

    struct OverCap {
        fake: FakeEmbedder,
    }

    impl Embedder for OverCap {
        fn spec(&self) -> &Spec {
            self.fake.spec()
        }
        fn count_tokens<'a>(&'a self, _: &'a str) -> EmbedFuture<'a, usize> {
            Box::pin(async { Ok(2049) })
        }
        fn embed<'a>(&'a self, _: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
            Box::pin(async { panic!("over-cap input reached the embedder") })
        }
    }

    #[tokio::test]
    async fn every_cache_fill_entry_admits_with_the_tokenizer() {
        let dir = tempfile::tempdir().unwrap();
        let embedder = OverCap {
            fake: FakeEmbedder::new(),
        };
        let mut docs = vec![document("one")];
        assert!(embed_documents(dir.path(), Id([1; 16]), &embedder, &mut docs, &[])
            .await
            .is_err());
        assert!(embed_texts(dir.path(), Id([2; 16]), &embedder, &["one".to_owned()])
            .await
            .is_err());
        assert!(!dir.path().join(EmbeddingCache::NAME).exists());
    }

    struct Racing {
        fake: FakeEmbedder,
        gate: std::sync::Arc<tokio::sync::Barrier>,
        negative: bool,
    }

    impl Embedder for Racing {
        fn spec(&self) -> &Spec {
            self.fake.spec()
        }
        fn count_tokens<'a>(&'a self, request: &'a str) -> EmbedFuture<'a, usize> {
            self.fake.count_tokens(request)
        }
        fn embed<'a>(&'a self, requests: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
            Box::pin(async move {
                self.gate.wait().await;
                Ok(requests
                    .iter()
                    .map(|request| {
                        let mut vector = self.fake.vector(request);
                        if self.negative {
                            vector.iter_mut().for_each(|value| *value = -*value);
                        }
                        vector
                    })
                    .collect())
            })
        }
    }

    #[tokio::test]
    async fn racing_fillers_return_the_committed_winner_at_their_version() {
        let dir = tempfile::tempdir().unwrap();
        let gate = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let a = Racing {
            fake: FakeEmbedder::new(),
            gate: gate.clone(),
            negative: false,
        };
        let b = Racing {
            fake: FakeEmbedder::new(),
            gate,
            negative: true,
        };
        let text = vec!["shared".to_owned()];
        let (left, right) = tokio::join!(
            embed_texts(dir.path(), Id([1; 16]), &a, &text),
            embed_texts(dir.path(), Id([2; 16]), &b, &text)
        );
        let (left_vectors, left_keys, left_version) = left.unwrap();
        let (right_vectors, right_keys, right_version) = right.unwrap();
        assert_eq!(left_keys, right_keys);
        assert_eq!(left_vectors, right_vectors);
        let spec_hash = a.spec().hash();
        for (vectors, version) in [(&left_vectors, left_version), (&right_vectors, right_version)] {
            let stored = cached_vectors(dir.path(), spec_hash, version, left_keys.iter().copied())
                .await
                .unwrap();
            assert_eq!(vectors[0], stored[&left_keys[0]]);
        }
    }
}
