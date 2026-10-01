//! Dormant pre-cutover receipt writer; retirement waits for typed E1/E0 consumption owners.
use crate::{
    CoreError,
    embedding_service::{Embedder, Spec, check_vector, input_hash},
};
use cpg_schema::{
    findings::BriefDocumentsRow,
    id::{Digest, Id},
};
use lctx_model::domain::ContentHash;

/// Exact replay state owned by one compile attempt, never by cloneable Analysis or Embedder.
pub struct Session {
    snapshot_id: Id,
    cache: Option<crate::postgres::Store>,
    spec: Option<String>,
    values: std::collections::BTreeMap<ContentHash, crate::postgres::CacheValue>,
    uses: std::collections::BTreeMap<ContentHash, i64>,
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
                let count = embedder
                    .count_tokens(text)
                    .await
                    .map_err(|e| CoreError::EmbeddingService(e.to_string()))?;
                if count > spec.max_document_tokens as usize {
                    return Err(CoreError::Embed(format!(
                        "a document request is {count} tokens, over the {}-token cap",
                        spec.max_document_tokens
                    )));
                }
                tokens.push(count as u32);
            }
            let vectors = embedder
                .embed(&texts)
                .await
                .map_err(|e| CoreError::EmbeddingService(e.to_string()))?;
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
            doc.spec_hash = Some(Digest(embedder.spec().hash().0));
            doc.input_hash = Some(Digest(
                input_hash(&embedder.spec().document_text(&doc.text)).0,
            ));
        }
        Ok(())
    }

    /// Consumes retained bytes; it cannot perform database/network I/O.
    pub fn finish(self) -> Result<Receipt, CoreError> {
        use cpg_schema::embedding::{EmbeddingUsesRow, UsedEmbeddingsRow, receipt_digest};
        use lctx_model::domain::embedding::value::value_digest;
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
        let spec_hash = spec.as_ref().map(|s| Digest(s.hash().0));
        let values = self
            .values
            .into_iter()
            .map(|(input_hash, v)| UsedEmbeddingsRow {
                snapshot_id: self.snapshot_id,
                spec_hash: spec_hash.expect("values have spec"),
                input_hash: Digest(input_hash.0),
                value_digest: Digest(value_digest(&v.vector).0),
                vector: v.vector,
            })
            .collect::<Vec<_>>();
        let uses = self
            .uses
            .into_iter()
            .map(|(input_hash, usage_mask)| EmbeddingUsesRow {
                snapshot_id: self.snapshot_id,
                spec_hash: spec_hash.expect("uses have spec"),
                input_hash: Digest(input_hash.0),
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
    use crate::embedding_service::FakeEmbedder;
    use sha2::{Digest as _, Sha256};

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
                .find(|u| u.input_hash.0 == input_hash("shared").0)
                .unwrap()
                .usage_mask,
            7
        );
        assert_eq!(
            receipt
                .uses
                .iter()
                .find(|u| u.input_hash.0 == input_hash("E0 only").0)
                .unwrap()
                .usage_mask,
            2
        );
        let mut changed = receipt.values.clone();
        changed[0].vector[0] = -changed[0].vector[0];
        changed[0].value_digest =
            Digest(lctx_model::domain::embedding::value::value_digest(&changed[0].vector).0);
        assert_ne!(
            receipt.digest.unwrap(),
            cpg_schema::embedding::receipt_digest(Digest(fake.spec().hash().0), &changed)
        );
        changed.reverse();
        assert_eq!(
            cpg_schema::embedding::receipt_digest(Digest(fake.spec().hash().0), &changed),
            {
                changed.reverse();
                cpg_schema::embedding::receipt_digest(Digest(fake.spec().hash().0), &changed)
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
        use lctx_model::domain::embedding::value::{decode_vector, encode_vector, value_digest};
        assert_eq!(encode_vector(&[1.0, -0.0]), [0, 0, 128, 63, 0, 0, 0, 128]);
        assert_eq!(
            decode_vector(&[0, 0, 0, 128], 1).unwrap()[0].to_bits(),
            (-0.0f32).to_bits()
        );
        assert!(decode_vector(&[0, 0, 0], 1).is_err());
        assert_ne!(value_digest(&[1.0, 0.0]), value_digest(&[1.0, -0.0]));
    }
}
