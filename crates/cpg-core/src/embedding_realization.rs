//! One effect owner for token admission and immutable cache winners, shared by E1 and E0.
use crate::{
    CoreError,
    embedding_service::Embedder,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
#[cfg(test)]
use lctx_model::domain::embedding::cache::CacheFuture;
use lctx_model::domain::embedding::cache::{CacheValue, EmbeddingCache};
use lctx_model::domain::{
    embedding::{
        DocumentRecipe, EmbeddingSpec,
        projection::{ProjectedValue, ProjectionDefinition},
        configuration::{Configuration, ServiceConfiguration},
        value::{AdmittedValue, FullValue, input_hash},
        consumption::PublishedValue,
    },
    resources::ResourceBudget,
    *,
};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom, Write},
    sync::Arc,
};

/// Resource and corruption failures remain distinct from optional service availability.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Service(#[from] CoreError),
    #[error("embedding request is {tokens} tokens, over the {limit}-token cap")]
    TokenLimit { tokens: usize, limit: u32 },
}
impl Error {
    pub fn unavailable(&self) -> bool {
        matches!(self, Self::Service(CoreError::EmbeddingService(_)))
    }
}

pub struct Session<'a> {
    embedder: &'a dyn Embedder,
    configuration: Configuration,
    cache: Option<Arc<dyn EmbeddingCache>>,
    budget: ResourceBudget,
    values: BTreeMap<ContentHash, WinnerSlot>,
    full_inputs: BTreeMap<Id<FullValue>, ContentHash>,
    charge: charged::StateCharge,
    refusals: BTreeMap<ContentHash, RefusalSlot>,
    // Attempt-private spill state has no persistence or semantic authority. Every load goes
    // through the existing value admission; only compact immutable offsets survive a request.
    spool: Option<std::fs::File>,
    active: Option<(ContentHash, AdmittedValue)>,
}
#[derive(Clone)]
enum Refusal {
    TokenLimit { tokens: usize, limit: u32 },
    ServiceUnavailable(String),
}
#[derive(Clone, Copy)]
struct PayloadSlot {
    offset: u64,
    length: usize,
}
#[derive(Clone, Copy)]
struct WinnerSlot {
    payload: PayloadSlot,
    digest: ContentHash,
    tokens: u32,
    full_published: bool,
    projection_published: bool,
}
#[derive(Clone, Copy)]
enum RefusalSlot {
    TokenLimit { tokens: usize, limit: u32 },
    ServiceUnavailable(PayloadSlot),
}
impl<'a> Session<'a> {
    /// The selected configuration comes from the caller's completed R0 inputs.
    /// A pure producer cannot acquire this effect.
    pub async fn open(
        access: &CompletedInputs,
        runtime: &Workspace,
        _model: &Arc<ValidatedModel>,
        embedder: &'a dyn Embedder,
        cache: Option<Arc<dyn EmbeddingCache>>,
    ) -> Result<Self, ModelError> {
        let session = access.session(runtime).await?;
        let mut specs = normalized::Rows::<EmbeddingSpec>::new(runtime.budget());
        let mut services = normalized::Rows::<ServiceConfiguration>::new(runtime.budget());
        let mut documents = normalized::Rows::<DocumentRecipe>::new(runtime.budget());
        let mut projections = normalized::Rows::<ProjectionDefinition>::new(runtime.budget());
        macro_rules! read {
            ($ty:ty,$rows:ident) => {{
                let _permit = access.read::<$ty>()?;
                let query =
                    crate::sql::query(&session, &format!("SELECT * FROM \"{}\"", <$ty>::NAME))
                        .await
                        .map_err(ModelError::codec)?;
                let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    $rows.decode(&batch)?;
                }
            }};
        }
        read!(EmbeddingSpec, specs);
        read!(ServiceConfiguration, services);
        read!(DocumentRecipe, documents);
        read!(ProjectionDefinition, projections);
        drop(session);
        if specs.len() != 1 || services.len() != 1 || documents.len() != 1 || projections.len() != 1 {
            return Err(ModelError::Invalid(
                "embedding effect needs one selected encoder, document, projection and service".into(),
            ));
        }
        let configuration = Configuration::from_selected(
            specs.iter().next().expect("one encoder"),
            documents.iter().next().expect("one document recipe"),
            projections.iter().next().expect("one projection policy"),
            services.iter().next().expect("one service"),
            runtime.budget(),
        )?;
        Self::selected(configuration, embedder, cache, runtime.budget())
    }
    fn selected(
        configuration: Configuration,
        embedder: &'a dyn Embedder,
        cache: Option<Arc<dyn EmbeddingCache>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        configuration.check_budget(budget)?;
        if configuration.row().id() != EmbeddingSpec::new(embedder.spec())?.id()
            || configuration.document().id() != DocumentRecipe::new(embedder.spec())?.id()
            || configuration.service().endpoint != embedder.endpoint()
        {
            return Err(ModelError::Invalid(
                "actual embedding service differs from selected configuration".into(),
            ));
        }
        Ok(Self {
            configuration,
            embedder,
            cache,
            budget: budget.clone(),
            values: BTreeMap::new(),
            full_inputs: BTreeMap::new(),
            charge: charged::StateCharge::new(budget, "embedding-winner-index"),
            refusals: BTreeMap::new(),
            spool: None,
            active: None,
        })
    }
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
    /// Replay the canonical E1 stream before E0 effects. Per-window receipts are not winners.
    pub fn seed_full_value(&mut self, full: &FullValue) -> Result<(), ModelError> {
        full.verify_encoder(self.configuration.row())?;
        let tokens = u32::try_from(full.tokens).map_err(ModelError::codec)?;
        let decoded = embedding::value::decode(
            self.configuration.specification(), &full.bytes.0, full.digest, tokens, &self.budget,
        )?;
        drop(decoded);
        self.insert_payload(full.input, full.digest, tokens, &full.bytes.0)?;
        self.bind_full(full.id(), full.input)?;
        self.values.get_mut(&full.input).expect("seeded winner").full_published = true;
        Ok(())
    }
    /// Verify the selected derivation against its already seeded full winner, then retain only
    /// the publication fact. A missing or foreign companion cannot fall back to fresh inference.
    pub fn seed_projection(&mut self, projection: &ProjectedValue) -> Result<(), ModelError> {
        if projection.definition != self.configuration.projection().id() {
            return Err(ModelError::Invalid("seeded projection has another selected policy".into()));
        }
        let input = *self.full_inputs.get(&projection.value)
            .ok_or_else(|| ModelError::Invalid("seeded projection lacks canonical full winner".into()))?;
        let _derive = self.budget.reserve("embedding-seed-projection", self.projection_bytes())?;
        let full = self.full_value(input)?;
        projection.verify(&full, self.configuration.projection())?;
        self.values.get_mut(&input).expect("seeded full winner").projection_published = true;
        Ok(())
    }
    /// Local producer binding to an already admitted and published winner; no payload copy.
    pub fn verify_published(&self,value:&PublishedValue)->Result<(),ModelError>{
        let slot=self.values.get(&value.input).ok_or_else(||ModelError::Invalid("published winner absent".into()))?;
        let full=Id::of(&embedding::value::FullValueKey{encoder:self.configuration.row().id(),input:value.input});
        let projection=Id::of(&embedding::projection::ProjectedValueKey{value:full,definition:self.configuration.projection().id()});
        if value.value!=full||value.projection!=projection||value.tokens!=slot.tokens||!slot.full_published||!slot.projection_published||self.full_inputs.get(&full)!=Some(&value.input){return Err(ModelError::Invalid("published winner identity/token/closure mismatch".into()));}Ok(())
    }
    fn bind_full(&mut self, full: Id<FullValue>, input: ContentHash) -> Result<(), ModelError> {
        if let Some(previous) = self.full_inputs.get(&full) {
            if *previous != input { return Err(ModelError::Conflict("canonical full input identity")); }
        } else {
            self.charge.grow(size_of::<Id<FullValue>>() + size_of::<ContentHash>() + 64)?;
            self.full_inputs.insert(full, input);
        }
        Ok(())
    }
    fn projection_bytes(&self) -> usize {
        (self.configuration.specification().dimensions as usize).saturating_mul(12)
            .saturating_add((self.configuration.projection().dimensions as usize).saturating_mul(12))
            .saturating_add(4096)
    }
    fn full_value(&mut self, input: ContentHash) -> Result<FullValue, ModelError> {
        let slot = *self.values.get(&input)
            .ok_or_else(|| ModelError::Invalid("canonical full winner absent".into()))?;
        let (bytes, _read) = self.read_payload(slot.payload)?;
        Ok(FullValue {
            encoder: self.configuration.row().id(), input,
            dimensions: i64::from(self.configuration.specification().dimensions),
            tokens: i64::from(slot.tokens), codec: embedding::value::VALUE_CODEC,
            digest: slot.digest, bytes: EvidenceBytes(bytes),
        })
    }
    /// Emit one canonical full/projection pair per winner, independently of window use count.
    /// Both outputs must be declared by the producer, even when every winner was seeded.
    pub async fn publish(
        &mut self, document: &str, output: &ProducerOutput,
    ) -> Result<PublishedValue, Error> {
        let spec = self.configuration.specification();
        let bound = document.len()
            .checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len()))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| ModelError::Invalid("embedding publication request overflow".into()))?;
        let _request = self.budget.reserve("embedding-publication-request", bound)?;
        let input = input_hash(&spec.document_text(document));
        if !self.values.contains_key(&input) && !self.refusals.contains_key(&input) {
            self.prepare([document]).await?;
        }
        if let Some(refusal) = self.refusals.get(&input).copied() {
            return Err(self.refusal_error(refusal)?);
        }
        let slot = *self.values.get(&input)
            .ok_or_else(|| ModelError::Invalid("canonical publication winner absent".into()))?;
        let full_id = Id::of(&embedding::value::FullValueKey {encoder:self.configuration.row().id(),input});
        let projection_id = Id::of(&embedding::projection::ProjectedValueKey {value:full_id,definition:self.configuration.projection().id()});
        if !slot.full_published || !slot.projection_published {
            // Construct payloads only for a new publication; another window receives just IDs.
            let _derive = self.budget.reserve("embedding-canonical-publication", self.projection_bytes())?;
            let full = self.full_value(input)?;
            let projection = ProjectedValue::new(&full, self.configuration.projection())?;
            if !slot.full_published {
                output.push(full).await?;
                self.values.get_mut(&input).expect("winner slot").full_published = true;
            }
            if !slot.projection_published {
                output.push(projection).await?;
                self.values.get_mut(&input).expect("winner slot").projection_published = true;
            }
        }
        self.bind_full(full_id, input)?;
        Ok(PublishedValue { value:full_id, projection:projection_id, input, tokens:slot.tokens })
    }
    fn append(&mut self, bytes: &[u8]) -> Result<PayloadSlot, ModelError> {
        if self.spool.is_none() {
            self.spool = Some(tempfile::tempfile().map_err(ModelError::codec)?);
        }
        let file = self.spool.as_mut().expect("opened private receipt spool");
        let offset = file.seek(SeekFrom::End(0)).map_err(ModelError::codec)?;
        offset
            .checked_add(u64::try_from(bytes.len()).map_err(ModelError::codec)?)
            .ok_or_else(|| ModelError::Invalid("embedding receipt offset overflow".into()))?;
        file.write_all(bytes).map_err(ModelError::codec)?;
        Ok(PayloadSlot {
            offset,
            length: bytes.len(),
        })
    }
    fn read_payload(
        &mut self,
        slot: PayloadSlot,
    ) -> Result<(Vec<u8>, Box<dyn resources::Reservation>), ModelError> {
        let reservation = self
            .budget
            .reserve("embedding-private-receipt-read", slot.length)?;
        let mut bytes = vec![0; slot.length];
        let file = self
            .spool
            .as_mut()
            .ok_or_else(|| ModelError::Invalid("embedding private receipt spool absent".into()))?;
        file.seek(SeekFrom::Start(slot.offset))
            .map_err(ModelError::codec)?;
        file.read_exact(&mut bytes).map_err(ModelError::codec)?;
        Ok((bytes, reservation))
    }
    fn insert(&mut self, admitted: AdmittedValue) -> Result<(), ModelError> {
        self.insert_payload(admitted.input(), admitted.digest(), admitted.tokens(), admitted.bytes())
    }
    fn insert_payload(&mut self, key: ContentHash, digest: ContentHash, tokens: u32, bytes: &[u8]) -> Result<(), ModelError> {
        if let Some(previous) = self.values.get(&key).copied() {
            let (previous_bytes, _read) = self.read_payload(previous.payload)?;
            if previous_bytes != bytes || previous.tokens != tokens || previous.digest != digest {
                return Err(ModelError::Invalid(
                    "embedding effects disagree on exact winning value".into(),
                ));
            }
        } else {
            self.charge.grow(size_of::<ContentHash>() + size_of::<WinnerSlot>() + 64)?;
            let payload = self.append(bytes)?;
            self.values.insert(key, WinnerSlot {
                payload, digest, tokens, full_published: false, projection_published: false,
            });
        }
        Ok(())
    }
    fn refuse(&mut self, key: ContentHash, refusal: Refusal) -> Result<(), ModelError> {
        if !self.refusals.contains_key(&key) {
            self.charge
                .grow(size_of::<ContentHash>() + size_of::<RefusalSlot>() + 64)?;
            let slot = match refusal {
                Refusal::TokenLimit { tokens, limit } => RefusalSlot::TokenLimit { tokens, limit },
                Refusal::ServiceUnavailable(message) => {
                    RefusalSlot::ServiceUnavailable(self.append(message.as_bytes())?)
                }
            };
            self.refusals.insert(key, slot);
        }
        Ok(())
    }
    fn refusal_error(&mut self, refusal: RefusalSlot) -> Result<Error, ModelError> {
        Ok(match refusal {
            RefusalSlot::TokenLimit { tokens, limit } => Error::TokenLimit { tokens, limit },
            RefusalSlot::ServiceUnavailable(slot) => {
                let (bytes, _read) = self.read_payload(slot)?;
                Error::Service(CoreError::EmbeddingService(
                    String::from_utf8(bytes).map_err(ModelError::codec)?,
                ))
            }
        })
    }
    /// Deduplicate before effects and use the existing service/cache batch interfaces. Chunks
    /// bound transient strings/vectors; refusals retain per-text admission and availability.
    pub async fn prepare<'d>(
        &mut self,
        documents: impl IntoIterator<Item = &'d str>,
    ) -> Result<(), Error> {
        const CHUNK: usize = 64;
        let mut requests = BTreeMap::new();
        let mut request_charge = charged::StateCharge::new(&self.budget, "embedding-request-batch");
        for document in documents {
            let spec = self.configuration.specification();
            let bound = document
                .len()
                .checked_mul(spec.document_template.matches("{text}").count())
                .and_then(|n| n.checked_add(spec.document_template.len()))
                .and_then(|n| n.checked_mul(2))
                .ok_or_else(|| {
                    ModelError::Invalid("embedding request allocation overflow".into())
                })?;
            let _copy = self.budget.reserve("embedding-request", bound)?;
            let request = spec.document_text(document);
            let key = input_hash(&request);
            if self.values.contains_key(&key)
                || self.refusals.contains_key(&key)
                || requests.contains_key(&key)
            {
                continue;
            }
            request_charge
                .grow(request.len() + size_of::<String>() + size_of::<ContentHash>() + 64)?;
            requests.insert(key, request);
            if requests.len() == CHUNK {
                self.prepare_chunk(std::mem::take(&mut requests)).await?;
                request_charge = charged::StateCharge::new(&self.budget, "embedding-request-batch");
            }
        }
        if !requests.is_empty() {
            self.prepare_chunk(requests).await?;
        }
        Ok(())
    }
    async fn prepare_chunk(
        &mut self,
        requests: BTreeMap<ContentHash, String>,
    ) -> Result<(), Error> {
        let spec = self.configuration.specification().clone();
        let _effect = self.budget.reserve(
            "embedding-effect-buffers",
            requests.len() * (spec.dimensions as usize * 128 + 8192)
                + requests.values().map(|s| s.len() * 4).sum::<usize>(),
        )?;
        let keys = requests.keys().copied().collect::<Vec<_>>();
        let mut cached = if let Some(cache) = &self.cache {
            cache.cached(&spec, &keys).await?
        } else {
            BTreeMap::new()
        };
        let mut uncached = Vec::new();
        let mut tokens = Vec::new();
        for (key, request) in &requests {
            if let Some(winner) = cached.remove(key) {
                if winner.input_hash != *key {
                    return Err(ModelError::Invalid(
                        "embedding winner has another input hash".into(),
                    )
                    .into());
                }
                if winner.admitted_tokens>spec.max_document_tokens {self.refuse(*key,Refusal::TokenLimit {tokens:winner.admitted_tokens as usize,limit:spec.max_document_tokens})?;continue;}
                self.insert(AdmittedValue::new(
                    &spec,
                    request,
                    winner.admitted_tokens,
                    &winner.vector,
                    &self.budget,
                )?)?;
                continue;
            }
            let counted=if let Some(tokenizer)=self.embedder.document_tokenizer() {
                // Assets compose the complete input; never compose the recipe twice.
                let (prefix,suffix)=spec.document_template.split_once("{text}").ok_or_else(||ModelError::Invalid("invalid document template".into()))?;
                let body=request.strip_prefix(prefix).and_then(|s|s.strip_suffix(suffix)).ok_or_else(||ModelError::Invalid("complete encoder input recipe mismatch".into()))?;
                Ok(tokenizer.encode(body)?.offsets.len())
            } else {self.embedder.count_tokens(request).await};
            match counted {
                Ok(count) if count <= spec.max_document_tokens as usize => {
                    uncached.push(request.clone());
                    tokens.push(count as u32);
                }
                Ok(count) => self.refuse(
                    *key,
                    Refusal::TokenLimit {
                        tokens: count,
                        limit: spec.max_document_tokens,
                    },
                )?,
                Err(CoreError::EmbeddingService(message)) => {
                    self.refuse(*key, Refusal::ServiceUnavailable(message))?
                }
                Err(error) => return Err(error.into()),
            }
        }
        if uncached.is_empty() {
            return Ok(());
        }
        let vectors = match self.embedder.embed(&uncached).await {
            Ok(vectors) => vectors,
            Err(CoreError::EmbeddingService(message)) => {
                for request in &uncached {
                    self.refuse(
                        input_hash(request),
                        Refusal::ServiceUnavailable(message.clone()),
                    )?;
                }
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        if vectors.len() != uncached.len() {
            return Err(ModelError::Invalid(
                "embedding response count differs from request".into(),
            )
            .into());
        }
        let candidates = vectors
            .into_iter()
            .zip(tokens)
            .zip(&uncached)
            .map(|((vector, admitted_tokens), request)| {
                lctx_model::domain::embedding::check_vector(&vector, spec.dimensions)
                    .map_err(ModelError::Invalid)?;
                Ok(CacheValue {
                    input_hash: input_hash(request),
                    vector,
                    admitted_tokens,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let mut winners = if let Some(cache) = &self.cache {
            cache.admit(&spec, &candidates).await?
        } else {
            candidates
                .into_iter()
                .map(|candidate| (candidate.input_hash, candidate))
                .collect()
        };
        for request in &uncached {
            let key = input_hash(request);
            let winner = winners
                .remove(&key)
                .ok_or_else(|| ModelError::Invalid("cache omitted its immutable winner".into()))?;
            if winner.input_hash != key {
                return Err(
                    ModelError::Invalid("embedding winner has another input hash".into()).into(),
                );
            }
            self.insert(AdmittedValue::new(
                &spec,
                request,
                winner.admitted_tokens,
                &winner.vector,
                &self.budget,
            )?)?;
        }
        Ok(())
    }
    /// Returns an exact immutable winner. Nominal consumer uses remain independently admitted.
    pub async fn realize(&mut self, document: &str) -> Result<&AdmittedValue, Error> {
        let spec = self.configuration.specification();
        let bound = document
            .len()
            .checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len()))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| ModelError::Invalid("embedding request allocation overflow".into()))?;
        let _copy = self.budget.reserve("embedding-request", bound)?;
        let key = input_hash(&spec.document_text(document));
        if !self.values.contains_key(&key) && !self.refusals.contains_key(&key) {
            self.prepare([document]).await?;
        }
        if let Some(refusal) = self.refusals.get(&key).copied() {
            return Err(self.refusal_error(refusal)?);
        }
        if self
            .active
            .as_ref()
            .is_none_or(|(active, _)| *active != key)
        {
            // Release the preceding hydrated vector before admitting the next one. A returned
            // borrow prevents replacement until its consumer is finished with those bytes.
            self.active = None;
            let slot =
                self.values.get(&key).copied().ok_or_else(|| {
                    ModelError::Invalid("prepared embedding winner absent".into())
                })?;
            let (bytes, _read) = self.read_payload(slot.payload)?;
            let spec = self.configuration.specification();
            let decoded =
                embedding::value::decode(spec, &bytes, slot.digest, slot.tokens, &self.budget)?;
            let request = spec.document_text(document);
            let admitted =
                AdmittedValue::new(spec, &request, slot.tokens, decoded.values(), &self.budget)?;
            if admitted.input() != key
                || admitted.bytes() != bytes
                || admitted.digest() != slot.digest
            {
                return Err(
                    ModelError::Invalid("embedding private winner receipt differs".into()).into(),
                );
            }
            self.active = Some((key, admitted));
        }
        Ok(&self.active.as_ref().expect("hydrated winner").1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding_service::{EmbedFuture, FakeEmbedder};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Counter {
        fake: FakeEmbedder,
        tokens: AtomicUsize,
        embeddings: AtomicUsize,
        over: bool,
    }
    impl Embedder for Counter {
        fn spec(&self) -> &embedding::Spec {
            self.fake.spec()
        }
        fn endpoint(&self) -> &str {
            "fixture://counter"
        }
        fn count_tokens<'b>(&'b self, text: &'b str) -> EmbedFuture<'b, usize> {
            self.tokens.fetch_add(1, Ordering::SeqCst);
            if self.over {
                Box::pin(async { Ok(usize::MAX) })
            } else {
                self.fake.count_tokens(text)
            }
        }
        fn embed<'b>(&'b self, texts: &'b [String]) -> EmbedFuture<'b, Vec<Vec<f32>>> {
            self.embeddings.fetch_add(1, Ordering::SeqCst);
            self.fake.embed(texts)
        }
    }
    struct Changing {
        specification: embedding::Spec,
        batches: std::sync::Mutex<Vec<Vec<String>>>,
        unavailable: bool,
    }
    impl Embedder for Changing {
        fn spec(&self) -> &embedding::Spec {
            &self.specification
        }
        fn endpoint(&self) -> &str {
            "fixture://changing"
        }
        fn count_tokens<'b>(&'b self, text: &'b str) -> EmbedFuture<'b, usize> {
            Box::pin(async move {
                Ok(if text == "over" {
                    self.spec().max_document_tokens as usize + 1
                } else {
                    1
                })
            })
        }
        fn embed<'b>(&'b self, texts: &'b [String]) -> EmbedFuture<'b, Vec<Vec<f32>>> {
            Box::pin(async move {
                let mut batches = self.batches.lock().unwrap();
                let sign = if batches.len().is_multiple_of(2) {
                    1.0
                } else {
                    -1.0
                };
                batches.push(texts.to_vec());
                if self.unavailable {
                    return Err(CoreError::EmbeddingService("fixture unavailable".into()));
                }
                Ok(texts
                    .iter()
                    .map(|_| {
                        let mut vector = vec![0.0; self.spec().dimensions as usize];
                        vector[0] = sign;
                        vector
                    })
                    .collect())
            })
        }
    }
    #[tokio::test]
    async fn private_spill_keeps_nonadjacent_first_winners_with_bounded_vectors_and_batches() {
        let provider = Changing {
            specification: FakeEmbedder::new().spec().clone(),
            batches: Default::default(),
            unavailable: false,
        };
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let configuration =
            Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(configuration, &provider, None, &budget).unwrap();
        let documents: Vec<_> = (0..5000).map(|index| format!("document-{index}")).collect();
        session
            .prepare(documents.iter().map(String::as_str))
            .await
            .unwrap();
        assert!(
            budget.reserved() < 2 << 20,
            "completed requests retain compact slots, not every vector"
        );
        assert_eq!(
            session.spool.as_ref().unwrap().metadata().unwrap().len(),
            5000 * u64::from(provider.spec().dimensions) * 4
        );
        let first = session
            .realize(&documents[0])
            .await
            .unwrap()
            .bytes()
            .to_vec();
        let other = session
            .realize(&documents[64])
            .await
            .unwrap()
            .bytes()
            .to_vec();
        assert_ne!(
            first, other,
            "the source-written provider actually changes later answers"
        );
        assert_eq!(session.realize(&documents[0]).await.unwrap().bytes(), first);
        session.prepare([documents[0].as_str()]).await.unwrap();
        let batches = provider.batches.lock().unwrap();
        assert_eq!(batches.len(), 5000usize.div_ceil(64));
        assert!(batches.iter().all(|batch| batch.len() <= 64));
        drop(batches);
        drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn private_spill_corruption_refuses_without_requesting_a_replacement_winner() {
        let provider = Changing {
            specification: FakeEmbedder::new().spec().clone(),
            batches: Default::default(),
            unavailable: false,
        };
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let configuration =
            Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(configuration, &provider, None, &budget).unwrap();
        session.prepare(["first"]).await.unwrap();
        session.spool.as_mut().unwrap().set_len(0).unwrap();
        assert!(session.realize("first").await.is_err());
        assert_eq!(provider.batches.lock().unwrap().len(), 1);
        drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn canonical_full_and_projection_seed_retrieval_without_a_second_service_winner() {
        let provider = Changing {
            specification: FakeEmbedder::new().spec().clone(),
            batches: Default::default(), unavailable: false,
        };
        let budget = ResourceBudget::fixed(1 << 26).unwrap();
        let configuration = || Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut analytic = Session::selected(configuration(), &provider, None, &budget).unwrap();
        analytic.realize("same").await.unwrap();
        let (full, projection) = embedding::projection::admit(
            analytic.configuration.row(), &analytic.active.as_ref().unwrap().1,
            analytic.configuration.projection(),
        ).unwrap();
        drop(analytic);
        let mut retrieval = Session::selected(configuration(), &provider, None, &budget).unwrap();
        assert!(retrieval.seed_projection(&projection).is_err(), "missing full companion refuses");
        retrieval.seed_full_value(&full).unwrap();
        retrieval.seed_projection(&projection).unwrap();
        assert_eq!(retrieval.realize("same").await.unwrap().bytes(), full.bytes.0);
        assert_eq!(provider.batches.lock().unwrap().len(), 1);
        assert_ne!(retrieval.realize("different").await.unwrap().digest(), full.digest);
        let mut conflict = full.clone();
        let mut vector = vec![0.0; provider.spec().dimensions as usize];
        vector[0] = -1.0;
        conflict.bytes = EvidenceBytes(embedding::value::encode_vector(&vector));
        conflict.digest = embedding::value::value_digest(&vector);
        assert!(retrieval.seed_full_value(&conflict).is_err(), "another valid payload cannot replace the winner");
        let mut foreign = projection.clone();
        foreign.definition = ProjectionDefinition { dimensions: 1, algorithm: embedding::projection::Algorithm::PrefixL2F64F32 }.id();
        assert!(retrieval.seed_projection(&foreign).is_err());
        let mut changed_projection = projection.clone();
        changed_projection.bytes.0[0] ^= 1;
        assert!(retrieval.seed_projection(&changed_projection).is_err());
        drop(retrieval);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn canonical_publication_happens_once_and_seeded_values_emit_only_new_winners() {
        let provider = Changing {
            specification: FakeEmbedder::new().spec().clone(),
            batches: Default::default(), unavailable: false,
        };
        let relations = embedding::configuration_relations().into_iter()
            .chain([Relation::of::<FullValue>(), Relation::of::<ProjectedValue>()]).collect();
        let model = Arc::new(ValidatedModel::declared(relations).unwrap());
        let workspace = Workspace::new(model, Default::default()).unwrap();
        let budget = workspace.budget();
        let configuration = || Configuration::new(provider.spec(), provider.endpoint(), budget).unwrap();
        let mut analytic = Session::selected(configuration(), &provider, None, budget).unwrap();
        let output = workspace.output("e1", stages::Profile::Catalog, ContentHash::of(b"e1"), workspace.inputs("e1", stages::Profile::Catalog, []).unwrap());
        output.declare::<FullValue>().unwrap();
        output.declare::<ProjectedValue>().unwrap();
        let first = analytic.publish("same", &output).await.unwrap();
        let again = analytic.publish("same", &output).await.unwrap();
        assert_eq!(first.value, again.value);
        assert_eq!(first.projection, again.projection);
        output.finish(stages::ProviderOutcome::Complete).await.unwrap();
        let full = workspace.relation(FullValue::NAME).unwrap().batches().unwrap().flat_map(|batch| FullValue::decode(&batch.unwrap()).unwrap()).next().unwrap();
        let projection = workspace.relation(ProjectedValue::NAME).unwrap().batches().unwrap().flat_map(|batch| ProjectedValue::decode(&batch.unwrap()).unwrap()).next().unwrap();
        assert_eq!(full.dimensions, i64::from(provider.spec().dimensions));
        assert_eq!(projection.dimensions, i64::from(provider.spec().dimensions.min(1024)));
        projection.verify(&full, analytic.configuration.projection()).unwrap();
        drop(analytic);
        let mut retrieval = Session::selected(configuration(), &provider, None, budget).unwrap();
        retrieval.seed_full_value(&full).unwrap();
        retrieval.seed_projection(&projection).unwrap();
        // No declarations means an attempted duplicate push fails instead of being deduplicated
        // by workspace ordering, so this proves the Session skips both seeded publications.
        let no_outputs = workspace.output("no_duplicate", stages::Profile::Catalog, ContentHash::of(b"no_duplicate"), workspace.inputs("no_duplicate", stages::Profile::Catalog, []).unwrap());
        let replay = retrieval.publish("same", &no_outputs).await.unwrap();
        assert_eq!(replay.value, first.value);
        assert_eq!(replay.projection, first.projection);
        no_outputs.finish(stages::ProviderOutcome::Complete).await.unwrap();
        let output = workspace.output("e0", stages::Profile::Catalog, ContentHash::of(b"e0"), workspace.inputs("e0", stages::Profile::Catalog, []).unwrap());
        output.declare::<FullValue>().unwrap();
        output.declare::<ProjectedValue>().unwrap();
        retrieval.publish("new", &output).await.unwrap();
        output.finish(stages::ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.relation(FullValue::NAME).unwrap().rows(), 2);
        assert_eq!(workspace.relation(ProjectedValue::NAME).unwrap().rows(), 2);
        assert_eq!(provider.batches.lock().unwrap().len(), 2);
    }
    #[test]
    fn selected_effect_binds_encoder_and_document_but_query_policy_is_independent() {
        let fake = FakeEmbedder::new();
        let budget = ResourceBudget::fixed(1 << 24).unwrap();
        let mut query_changed = fake.spec().clone();
        query_changed.query_template = "Independent instruction: {task_description}\nQuery: {query}".into();
        query_changed.query_task = "different ranking task".into();
        query_changed.max_query_tokens -= 1;
        let configuration = Configuration::new(&query_changed, fake.endpoint(), &budget).unwrap();
        assert!(Session::selected(configuration, &fake, None, &budget).is_ok());
        let mut document_changed = fake.spec().clone();
        document_changed.document_template = "Document: {text}".into();
        let configuration = Configuration::new(&document_changed, fake.endpoint(), &budget).unwrap();
        assert!(Session::selected(configuration, &fake, None, &budget).is_err());
        let mut encoder_changed = fake.spec().clone();
        encoder_changed.model = "another-encoder".into();
        let configuration = Configuration::new(&encoder_changed, fake.endpoint(), &budget).unwrap();
        assert!(Session::selected(configuration, &fake, None, &budget).is_err());
        assert_eq!(budget.reserved(), 0);
    }
    #[derive(Default)]
    struct Cache {
        values: std::sync::Mutex<BTreeMap<(ContentHash, ContentHash), CacheValue>>,
        lookups: std::sync::Mutex<Vec<Vec<ContentHash>>>,
        admissions: std::sync::Mutex<Vec<Vec<ContentHash>>>,
    }
    impl EmbeddingCache for Cache {
        fn cached<'b>(
            &'b self,
            spec: &'b embedding::Spec,
            keys: &'b [ContentHash],
        ) -> CacheFuture<'b, BTreeMap<ContentHash, CacheValue>> {
            Box::pin(async move {
                self.lookups.lock().unwrap().push(keys.to_vec());
                let values = self.values.lock().unwrap();
                Ok(keys
                    .iter()
                    .filter_map(|key| values.get(&(spec.hash(), *key)).cloned().map(|v| (*key, v)))
                    .collect())
            })
        }
        fn admit<'b>(
            &'b self,
            spec: &'b embedding::Spec,
            candidates: &'b [CacheValue],
        ) -> CacheFuture<'b, BTreeMap<ContentHash, CacheValue>> {
            Box::pin(async move {
                self.admissions
                    .lock()
                    .unwrap()
                    .push(candidates.iter().map(|v| v.input_hash).collect());
                let mut values = self.values.lock().unwrap();
                Ok(candidates
                    .iter()
                    .map(|candidate| {
                        let winner = values
                            .entry((spec.hash(), candidate.input_hash))
                            .or_insert_with(|| candidate.clone())
                            .clone();
                        (candidate.input_hash, winner)
                    })
                    .collect())
            })
        }
    }
    #[tokio::test]
    async fn cache_lookups_and_admissions_share_the_unique_request_batch() {
        let provider = Changing {
            specification: FakeEmbedder::new().spec().clone(),
            batches: Default::default(),
            unavailable: false,
        };
        let budget = ResourceBudget::fixed(1 << 27).unwrap();
        let cache = Arc::new(Cache::default());
        let mut vector = vec![0.0; provider.spec().dimensions as usize];
        vector[0] = -1.0;
        cache.values.lock().unwrap().insert(
            (provider.spec().hash(), input_hash("cached")),
            CacheValue {
                input_hash: input_hash("cached"),
                vector: vector.clone(),
                admitted_tokens: 1,
            },
        );
        let configuration =
            Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session =
            Session::selected(configuration, &provider, Some(cache.clone()), &budget).unwrap();
        session
            .prepare(["cached", "second", "third", "second"])
            .await
            .unwrap();
        assert_eq!(
            session.realize("cached").await.unwrap().bytes(),
            embedding::value::encode_vector(&vector)
        );
        assert!(session.realize("second").await.is_ok() && session.realize("third").await.is_ok());
        assert_eq!(
            cache
                .lookups
                .lock()
                .unwrap()
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [3]
        );
        assert_eq!(
            cache
                .admissions
                .lock()
                .unwrap()
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [2]
        );
        assert_eq!(
            provider
                .batches
                .lock()
                .unwrap()
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [2]
        );
        drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn contract_only_freezes_one_value_admits_tokens_first_and_releases_budget() {
        let provider = Counter {
            fake: FakeEmbedder::new(),
            tokens: AtomicUsize::new(0),
            embeddings: AtomicUsize::new(0),
            over: false,
        };
        let budget = ResourceBudget::fixed(1 << 22).unwrap();
        let selection = Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(selection, &provider, None, &budget).unwrap();
        let first = session.realize("same").await.unwrap().digest();
        assert_eq!(first, session.realize("same").await.unwrap().digest());
        assert_eq!(provider.tokens.load(Ordering::SeqCst), 1);
        assert_eq!(provider.embeddings.load(Ordering::SeqCst), 1);
        drop(session);
        assert_eq!(budget.reserved(), 0);
        let over = Counter {
            over: true,
            ..provider
        };
        let selection = Configuration::new(over.spec(), over.endpoint(), &budget).unwrap();
        let before = over.embeddings.load(Ordering::SeqCst);
        let mut session = Session::selected(selection, &over, None, &budget).unwrap();
        assert!(session.realize("over cap").await.is_err());
        assert_eq!(over.embeddings.load(Ordering::SeqCst), before);
        drop(session);
        assert_eq!(budget.reserved(), 0);
        let wrong = Configuration::new(over.spec(), "fixture://different", &budget).unwrap();
        assert!(Session::selected(wrong, &over, None, &budget).is_err());
        assert_eq!(budget.reserved(), 0);
    }
}
