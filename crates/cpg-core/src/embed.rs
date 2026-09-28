//! Attempt-owned embedding admission and immutable exact-value replay (ADR-0068).

use std::future::Future;
use std::pin::Pin;

use cpg_schema::findings::BriefDocumentsRow;
use cpg_schema::id::{Digest, Id};
use sha2::{Digest as _, Sha256};

use crate::CoreError;

/// A future an embedder returns.
pub type EmbedFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, CoreError>> + Send + 'a>>;

pub use cpg_schema::embedding_spec::{Spec, check_vector};

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
                format: 2,
                source_dimensions: 1024,
                reduction: "none".to_owned(),
                admission: None,
                model: "lctx-fake-embedder".to_owned(),
                revision: "2".to_owned(),
                tokenizer_revision: "bytes/4".to_owned(),
                server: "in-process".to_owned(),
                served_dtype: "float32".to_owned(),
                pooling: "none".to_owned(),
                query_template: "Instruct: {task_description}\nQuery:{query}".to_owned(),
                query_task: "Given a coding task, retrieve capability briefs of a Python \
                             library that solve it"
                    .to_owned(),
                document_template: "{text}".to_owned(),
                dimensions: 1024,
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

/// Exact replay state owned by one compile attempt, never by cloneable Analysis or Embedder.
pub struct Session {
    snapshot_id: Id,
    cache: Option<crate::postgres::Store>,
    spec: Option<String>,
    values: std::collections::BTreeMap<Digest, crate::postgres::CacheValue>,
    uses: std::collections::BTreeMap<Digest, i64>,
    max_bytes: usize,
}

pub use cpg_schema::embedding::Usage;

pub struct Receipt {
    pub values: Vec<cpg_schema::embedding::UsedEmbeddingsRow>,
    pub uses: Vec<cpg_schema::embedding::EmbeddingUsesRow>,
    pub spec: Option<Spec>,
    pub digest: Option<Digest>,
}

impl Session {
    pub fn new(snapshot_id: Id, cache: Option<crate::postgres::Store>, max_bytes: usize) -> Self {
        Self {
            snapshot_id,
            cache,
            spec: None,
            values: Default::default(),
            uses: Default::default(),
            max_bytes,
        }
    }

    /// Explicitly uncached deterministic fixtures. Production selects a configured cache;
    /// failure of that cache is never an invitation to switch this mode.
    pub fn uncached(snapshot_id: Id) -> Self {
        Self::new(snapshot_id, None, 268435456)
    }

    pub async fn texts(
        &mut self,
        embedder: &dyn Embedder,
        texts: &[String],
        usage: Usage,
    ) -> Result<Vec<Vec<f32>>, CoreError> {
        let spec = embedder.spec();
        spec.validate().map_err(CoreError::Embed)?;
        let canonical = spec.canonical_json();
        if self.spec.as_ref().is_some_and(|s| s != &canonical) {
            return Err(CoreError::Embed(
                "an attempt cannot mix embedding specs".to_owned(),
            ));
        }
        self.spec = Some(canonical);
        let requests: Vec<_> = texts
            .iter()
            .map(|t| {
                let text = spec.document_text(t);
                (input_hash(&text), text)
            })
            .collect();
        let unique: std::collections::BTreeMap<_, _> =
            requests.iter().map(|(k, t)| (*k, t)).collect();
        let missing: Vec<_> = unique
            .keys()
            .filter(|k| !self.values.contains_key(k))
            .copied()
            .collect();
        let count = self
            .values
            .len()
            .checked_add(missing.len())
            .ok_or_else(|| CoreError::Embed("receipt budget overflow".to_owned()))?;
        if count
            .checked_mul(spec.dimensions as usize)
            .and_then(|n| n.checked_mul(4))
            .is_none_or(|n| n > self.max_bytes)
        {
            return Err(CoreError::Embed(
                "attempt vector receipt exceeds configured memory budget".to_owned(),
            ));
        }
        if let Some(cache) = &self.cache
            && !missing.is_empty()
        {
            cache.ensure_spec(spec).await?;
            self.values.extend(cache.cached(spec, &missing).await?);
        }
        let missing: Vec<_> = missing
            .into_iter()
            .filter(|k| !self.values.contains_key(k))
            .collect();
        for chunk in missing.chunks(32) {
            let texts: Vec<String> = chunk.iter().map(|k| (*unique[k]).clone()).collect();
            let mut tokens = Vec::with_capacity(texts.len());
            for text in &texts {
                let count = embedder.count_tokens(text).await?;
                if count > spec.max_document_tokens as usize {
                    return Err(CoreError::Embed(format!(
                        "a document request is {count} tokens, over the {}-token cap",
                        spec.max_document_tokens
                    )));
                }
                tokens.push(count as u32);
            }
            let vectors = embedder.embed(&texts).await?;
            if vectors.len() != texts.len() {
                return Err(CoreError::Embed(
                    "embedding response count mismatch".to_owned(),
                ));
            }
            let mut candidates = Vec::with_capacity(vectors.len());
            for ((key, vector), admitted_tokens) in chunk.iter().zip(vectors).zip(tokens) {
                check_vector(&vector, spec.dimensions).map_err(CoreError::Embed)?;
                candidates.push(crate::postgres::CacheValue {
                    input_hash: *key,
                    vector,
                    admitted_tokens,
                });
            }
            let winners = match &self.cache {
                Some(cache) => cache.admit(spec, &candidates).await?,
                None => candidates.into_iter().map(|v| (v.input_hash, v)).collect(),
            };
            self.values.extend(winners);
        }
        // Record every consumer before any value escapes. The separate inventory survives
        // publication, so a removed E0-only receipt row fails shared validation too.
        for key in unique.keys() {
            *self.uses.entry(*key).or_default() |= usage as i64;
        }
        requests
            .iter()
            .map(|(key, _)| {
                self.values
                    .get(key)
                    .map(|v| v.vector.clone())
                    .ok_or_else(|| CoreError::Embed("missing committed vector".to_owned()))
            })
            .collect()
    }

    pub async fn documents(
        &mut self,
        embedder: &dyn Embedder,
        docs: &mut [BriefDocumentsRow],
    ) -> Result<(), CoreError> {
        self.texts(
            embedder,
            &docs.iter().map(|d| d.text.clone()).collect::<Vec<_>>(),
            Usage::Brief,
        )
        .await?;
        for doc in docs {
            doc.spec_hash = Some(embedder.spec().hash());
            doc.input_hash = Some(input_hash(&embedder.spec().document_text(&doc.text)));
        }
        Ok(())
    }

    /// Consumes retained bytes; it cannot perform database/network I/O.
    pub fn finish(self) -> Result<Receipt, CoreError> {
        use cpg_schema::embedding::{
            EmbeddingUsesRow, UsedEmbeddingsRow, receipt_digest, value_digest,
        };
        let spec: Option<Spec> = self
            .spec
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| CoreError::Embed(e.to_string()))?;
        if self.uses.len() != self.values.len()
            || self.uses.keys().any(|k| !self.values.contains_key(k))
        {
            return Err(CoreError::Embed(
                "consumed vector receipt is incomplete".to_owned(),
            ));
        }
        let spec_hash = spec.as_ref().map(Spec::hash);
        let values = self
            .values
            .into_iter()
            .map(|(input_hash, v)| UsedEmbeddingsRow {
                snapshot_id: self.snapshot_id,
                spec_hash: spec_hash.expect("values have spec"),
                input_hash,
                value_digest: value_digest(&v.vector),
                vector: v.vector,
            })
            .collect::<Vec<_>>();
        let uses = self
            .uses
            .into_iter()
            .map(|(input_hash, usage_mask)| EmbeddingUsesRow {
                snapshot_id: self.snapshot_id,
                spec_hash: spec_hash.expect("uses have spec"),
                input_hash,
                usage_mask,
            })
            .collect();
        let digest = spec_hash.map(|s| receipt_digest(s, &values));
        tracing::info!(target: "lctx::postgres", operation="receipt_freeze", retained_keys=values.len(), vector_bytes=values.iter().map(|v| v.vector.len()*4).sum::<usize>(), budget_bytes=self.max_bytes);
        Ok(Receipt {
            values,
            uses,
            spec,
            digest,
        })
    }
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
        check_vector(&a, 1024).unwrap();
        assert!(check_vector(&a[..10], 1024).is_err());
        let mut nan = a.clone();
        nan[0] = f32::NAN;
        assert!(check_vector(&nan, 1024).is_err());
        assert!(check_vector(&vec![0.5; 1024], 1024).is_err());
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

    #[tokio::test]
    async fn every_consumer_is_recorded_before_use_and_exact_values_affect_identity() {
        let fake = FakeEmbedder::new();
        let mut session = Session::uncached(Id([1; 16]));
        session
            .texts(&fake, &["shared".to_owned()], Usage::Operation)
            .await
            .unwrap();
        session
            .texts(
                &fake,
                &["shared".to_owned(), "E0 only".to_owned()],
                Usage::Analytics,
            )
            .await
            .unwrap();
        session
            .texts(
                &fake,
                &["shared".to_owned(), "brief only".to_owned()],
                Usage::Brief,
            )
            .await
            .unwrap();
        let receipt = session.finish().unwrap();
        assert_eq!(receipt.values.len(), 3);
        assert_eq!(
            receipt
                .uses
                .iter()
                .find(|u| u.input_hash == input_hash("shared"))
                .unwrap()
                .usage_mask,
            7
        );
        assert_eq!(
            receipt
                .uses
                .iter()
                .find(|u| u.input_hash == input_hash("E0 only"))
                .unwrap()
                .usage_mask,
            2
        );
        let mut changed = receipt.values.clone();
        changed[0].vector[0] = -changed[0].vector[0];
        changed[0].value_digest = cpg_schema::embedding::value_digest(&changed[0].vector);
        assert_ne!(
            receipt.digest.unwrap(),
            cpg_schema::embedding::receipt_digest(fake.spec().hash(), &changed)
        );
        changed.reverse();
        assert_eq!(
            cpg_schema::embedding::receipt_digest(fake.spec().hash(), &changed),
            {
                changed.reverse();
                cpg_schema::embedding::receipt_digest(fake.spec().hash(), &changed)
            }
        );
    }

    #[tokio::test]
    async fn omitted_e0_receipt_and_oversized_requests_fail_closed() {
        let fake = FakeEmbedder::new();
        let mut session = Session::uncached(Id([1; 16]));
        session
            .texts(&fake, &["E0 only".to_owned()], Usage::Analytics)
            .await
            .unwrap();
        session.values.clear();
        assert!(session.finish().is_err());
        let mut session = Session::uncached(Id([1; 16]));
        assert!(
            session
                .texts(&fake, &["x".repeat(8193)], Usage::Operation)
                .await
                .is_err()
        );
        let mut tiny = Session::new(Id([1; 16]), None, 1);
        assert!(
            tiny.texts(&fake, &["x".to_owned()], Usage::Brief)
                .await
                .is_err()
        );
    }

    #[test]
    fn vector_codec_preserves_signed_zero_and_rejects_wrong_width() {
        use cpg_schema::embedding::{decode_vector, encode_vector, value_digest};
        assert_eq!(encode_vector(&[1.0, -0.0]), [0, 0, 128, 63, 0, 0, 0, 128]);
        assert_eq!(
            decode_vector(&[0, 0, 0, 128], 1).unwrap()[0].to_bits(),
            (-0.0f32).to_bits()
        );
        assert!(decode_vector(&[0, 0, 0], 1).is_err());
        assert_ne!(value_digest(&[1.0, 0.0]), value_digest(&[1.0, -0.0]));
    }
}
