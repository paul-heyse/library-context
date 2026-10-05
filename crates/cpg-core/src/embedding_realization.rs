//! One effect owner for token admission and immutable cache winners, shared by E1 and E0.
use crate::{
    CoreError,
    embedding_service::Embedder,
    workspace::{CompletedInputs, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    embedding::{
        EmbeddingSpec,
        configuration::{Configuration, ServiceConfiguration},
        value::{AdmittedValue, input_hash},
        consumption::{ValueReceipt, Winners},
    },
    resources::ResourceBudget,
    *,
};
use std::{future::Future, pin::Pin};
/// Cache winners are keyed by the complete embedding specification and request identity.
#[derive(Clone, Debug)]
pub struct CacheValue { pub input_hash: ContentHash, pub vector: Vec<f32>, pub admitted_tokens: u32 }
pub type CacheFuture<'a, T> = Pin<Box<dyn Future<Output=Result<T, ModelError>> + Send + 'a>>;
pub trait EmbeddingCache: Send + Sync {
    fn cached<'a>(&'a self, spec: &'a embedding::Spec, keys: &'a [ContentHash]) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>>;
    /// Atomically choose existing or new immutable winners; return a winner for every candidate.
    fn admit<'a>(&'a self, spec: &'a embedding::Spec, candidates: &'a [CacheValue]) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>>;
}
use std::{collections::BTreeMap, sync::Arc};

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
    values: BTreeMap<ContentHash, AdmittedValue>,
    charge: charged::StateCharge,
    refusals: BTreeMap<ContentHash, Refusal>,
}
#[derive(Clone)]
enum Refusal { TokenLimit { tokens: usize, limit: u32 }, ServiceUnavailable(String) }
impl Refusal {
    fn error(&self) -> Error {
        match self {
            Self::TokenLimit { tokens, limit } => Error::TokenLimit { tokens: *tokens, limit: *limit },
            Self::ServiceUnavailable(message) => Error::Service(CoreError::EmbeddingService(message.clone())),
        }
    }
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
        macro_rules! read {
            ($ty:ty,$rows:ident) => {{
                let _permit = access.read::<$ty>()?;
                let query = crate::sql::query(&session,&format!("SELECT * FROM \"{}\"", <$ty>::NAME))
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
        drop(session);
        if specs.len() != 1 || services.len() != 1 {
            return Err(ModelError::Invalid(
                "embedding effect needs one selected service and spec".into(),
            ));
        }
        let spec = specs.iter().next().expect("one specification");
        let service = services.iter().next().expect("one service");
        if service.specification != spec.id() {
            return Err(ModelError::Invalid(
                "embedding service refers to another specification".into(),
            ));
        }
        let configuration =
            Configuration::new(&spec.configuration()?, &service.endpoint, runtime.budget())?;
        Self::selected(configuration, embedder, cache, runtime.budget())
    }
    fn selected(
        configuration: Configuration,
        embedder: &'a dyn Embedder,
        cache: Option<Arc<dyn EmbeddingCache>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        configuration.check_budget(budget)?;
        if configuration.specification() != embedder.spec()
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
            charge: charged::StateCharge::new(budget, "embedding-winner-index"),
            refusals: BTreeMap::new(),
        })
    }
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
    /// Replay a completed nominal receipt before any effects. This hands the exact E1 winner to
    /// E0 without requiring a persistent cache or another service request.
    pub fn seed_receipt(
        &mut self,
        specification: &EmbeddingSpec,
        document: &str,
        receipt: ValueReceipt<'_>,
    ) -> Result<(), ModelError> {
        let spec = self.configuration.specification();
        if specification.configuration()? != *spec {
            return Err(ModelError::Invalid("seeded embedding has another selected specification".into()));
        }
        let mut replay = Winners::new(&self.budget);
        let tokens = receipt.admitted_tokens;
        let decoded = replay.replay(spec, document, receipt)?;
        let bound = document.len().checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len())).and_then(|n| n.checked_mul(2))
            .ok_or_else(|| ModelError::Invalid("embedding seed request allocation overflow".into()))?;
        let _request = self.budget.reserve("embedding-seed-request", bound)?;
        let request = spec.document_text(document);
        let admitted = AdmittedValue::new(spec, &request, tokens, decoded.values(), &self.budget)?;
        self.insert(admitted)
    }
    fn insert(&mut self, admitted: AdmittedValue) -> Result<(), ModelError> {
        let key = admitted.input();
        if let Some(previous) = self.values.get(&key) {
            if previous.bytes() != admitted.bytes() || previous.tokens() != admitted.tokens() {
                return Err(ModelError::Invalid("embedding effects disagree on exact winning value".into()));
            }
        } else {
            self.charge.grow(size_of::<ContentHash>() + size_of::<AdmittedValue>() + 64)?;
            self.values.insert(key, admitted);
        }
        Ok(())
    }
    fn refuse(&mut self, key: ContentHash, refusal: Refusal) -> Result<(), ModelError> {
        if !self.refusals.contains_key(&key) {
            let message = match &refusal { Refusal::ServiceUnavailable(message) => message.len(), _ => 0 };
            self.charge.grow(size_of::<ContentHash>() + size_of::<Refusal>() + message + 64)?;
            self.refusals.insert(key, refusal);
        }
        Ok(())
    }
    /// Deduplicate before effects and use the existing service/cache batch interfaces. Chunks
    /// bound transient strings/vectors; refusals retain per-text admission and availability.
    pub async fn prepare<'d>(&mut self, documents: impl IntoIterator<Item = &'d str>) -> Result<(), Error> {
        const CHUNK: usize = 64;
        let mut requests = BTreeMap::new();
        let mut request_charge = charged::StateCharge::new(&self.budget, "embedding-request-batch");
        for document in documents {
            let spec = self.configuration.specification();
            let bound = document.len()
                .checked_mul(spec.document_template.matches("{text}").count())
                .and_then(|n| n.checked_add(spec.document_template.len()))
                .and_then(|n| n.checked_mul(2))
                .ok_or_else(|| ModelError::Invalid("embedding request allocation overflow".into()))?;
            let _copy = self.budget.reserve("embedding-request", bound)?;
            let request = spec.document_text(document);
            let key = input_hash(&request);
            if self.values.contains_key(&key) || self.refusals.contains_key(&key) || requests.contains_key(&key) {
                continue;
            }
            request_charge.grow(request.len() + size_of::<String>() + size_of::<ContentHash>() + 64)?;
            requests.insert(key, request);
            if requests.len() == CHUNK {
                self.prepare_chunk(std::mem::take(&mut requests)).await?;
                request_charge = charged::StateCharge::new(&self.budget, "embedding-request-batch");
            }
        }
        if !requests.is_empty() { self.prepare_chunk(requests).await?; }
        Ok(())
    }
    async fn prepare_chunk(&mut self, requests: BTreeMap<ContentHash, String>) -> Result<(), Error> {
        let spec = self.configuration.specification().clone();
        let _effect = self.budget.reserve("embedding-effect-buffers",
            requests.len() * (spec.dimensions as usize * 128 + 8192)
                + requests.values().map(|s| s.len() * 4).sum::<usize>())?;
        let keys = requests.keys().copied().collect::<Vec<_>>();
        let mut cached = if let Some(cache) = &self.cache { cache.cached(&spec, &keys).await? } else { BTreeMap::new() };
        let mut uncached = Vec::new();
        let mut tokens = Vec::new();
        for (key, request) in &requests {
            if let Some(winner) = cached.remove(key) {
                if winner.input_hash != *key { return Err(ModelError::Invalid("embedding winner has another input hash".into()).into()); }
                self.insert(AdmittedValue::new(&spec, request, winner.admitted_tokens, &winner.vector, &self.budget)?)?;
                continue;
            }
            match self.embedder.count_tokens(request).await {
                Ok(count) if count <= spec.max_document_tokens as usize => { uncached.push(request.clone()); tokens.push(count as u32); }
                Ok(count) => self.refuse(*key, Refusal::TokenLimit { tokens: count, limit: spec.max_document_tokens })?,
                Err(CoreError::EmbeddingService(message)) => self.refuse(*key, Refusal::ServiceUnavailable(message))?,
                Err(error) => return Err(error.into()),
            }
        }
        if uncached.is_empty() { return Ok(()); }
        let vectors = match self.embedder.embed(&uncached).await {
            Ok(vectors) => vectors,
            Err(CoreError::EmbeddingService(message)) => {
                for request in &uncached { self.refuse(input_hash(request), Refusal::ServiceUnavailable(message.clone()))?; }
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        if vectors.len() != uncached.len() { return Err(ModelError::Invalid("embedding response count differs from request".into()).into()); }
        let candidates = vectors.into_iter().zip(tokens).zip(&uncached).map(|((vector, admitted_tokens), request)| {
            lctx_model::domain::embedding::check_vector(&vector, spec.dimensions).map_err(ModelError::Invalid)?;
            Ok(CacheValue { input_hash: input_hash(request), vector, admitted_tokens })
        }).collect::<Result<Vec<_>, ModelError>>()?;
        let mut winners = if let Some(cache) = &self.cache { cache.admit(&spec, &candidates).await? } else {
            candidates.into_iter().map(|candidate| (candidate.input_hash, candidate)).collect()
        };
        for request in &uncached {
            let key = input_hash(request);
            let winner = winners.remove(&key).ok_or_else(|| ModelError::Invalid("cache omitted its immutable winner".into()))?;
            if winner.input_hash != key { return Err(ModelError::Invalid("embedding winner has another input hash".into()).into()); }
            self.insert(AdmittedValue::new(&spec, request, winner.admitted_tokens, &winner.vector, &self.budget)?)?;
        }
        Ok(())
    }
    /// Returns an exact immutable winner. Nominal consumer uses remain independently admitted.
    pub async fn realize(&mut self, document: &str) -> Result<&AdmittedValue, Error> {
        let spec = self.configuration.specification();
        let bound = document.len().checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len())).and_then(|n| n.checked_mul(2))
            .ok_or_else(|| ModelError::Invalid("embedding request allocation overflow".into()))?;
        let _copy = self.budget.reserve("embedding-request", bound)?;
        let key = input_hash(&spec.document_text(document));
        if !self.values.contains_key(&key) && !self.refusals.contains_key(&key) {
            self.prepare([document]).await?;
        }
        if let Some(refusal) = self.refusals.get(&key) { return Err(refusal.error()); }
        self.values.get(&key).ok_or_else(|| ModelError::Invalid("prepared embedding winner absent".into()).into())
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
        fn spec(&self) -> &embedding::Spec { &self.specification }
        fn endpoint(&self) -> &str { "fixture://changing" }
        fn count_tokens<'b>(&'b self, text: &'b str) -> EmbedFuture<'b, usize> {
            Box::pin(async move { Ok(if text == "over" { self.spec().max_document_tokens as usize + 1 } else { 1 }) })
        }
        fn embed<'b>(&'b self, texts: &'b [String]) -> EmbedFuture<'b, Vec<Vec<f32>>> {
            Box::pin(async move {
                let mut batches = self.batches.lock().unwrap();
                let sign = if batches.len() % 2 == 0 { 1.0 } else { -1.0 };
                batches.push(texts.to_vec());
                if self.unavailable { return Err(CoreError::EmbeddingService("fixture unavailable".into())); }
                Ok(texts.iter().map(|_| { let mut vector = vec![0.0; self.spec().dimensions as usize]; vector[0] = sign; vector }).collect())
            })
        }
    }
    #[tokio::test]
    async fn completed_analytic_receipt_seeds_retrieval_without_a_second_service_winner() {
        let provider = Changing { specification: FakeEmbedder::new().spec().clone(), batches: Default::default(), unavailable: false };
        let budget = ResourceBudget::fixed(1 << 26).unwrap();
        let specification = EmbeddingSpec::new(provider.spec()).unwrap();
        let configuration = || Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut analytic = Session::selected(configuration(), &provider, None, &budget).unwrap();
        let first = analytic.realize("same").await.unwrap();
        let (bytes, digest, tokens, input) = (first.bytes().to_vec(), first.digest(), first.tokens(), first.input());
        drop(analytic);
        let mut retrieval = Session::selected(configuration(), &provider, None, &budget).unwrap();
        retrieval.seed_receipt(&specification, "same", ValueReceipt { input, codec: embedding::value::VALUE_CODEC, digest, admitted_tokens: tokens, bytes: &bytes }).unwrap();
        assert_eq!(retrieval.realize("same").await.unwrap().bytes(), bytes);
        assert_eq!(provider.batches.lock().unwrap().len(), 1);
        // The next valid service answer really differs: seed reuse must prevent it winning "same".
        assert_ne!(retrieval.realize("different").await.unwrap().digest(), digest);
        let mut other = provider.spec().clone(); other.document_template = "Document: {text}".into();
        let other_specification = EmbeddingSpec::new(&other).unwrap();
        assert!(retrieval.seed_receipt(&other_specification, "same", ValueReceipt { input, codec: embedding::value::VALUE_CODEC, digest, admitted_tokens: tokens, bytes: &bytes }).is_err());
        drop(retrieval);
        let other_provider = Changing { specification: other, batches: Default::default(), unavailable: false };
        let configuration = Configuration::new(other_provider.spec(), other_provider.endpoint(), &budget).unwrap();
        let mut other_session = Session::selected(configuration, &other_provider, None, &budget).unwrap();
        assert!(other_session.seed_receipt(&specification, "same", ValueReceipt { input, codec: embedding::value::VALUE_CODEC, digest, admitted_tokens: tokens, bytes: &bytes }).is_err());
        assert_ne!(other_session.realize("same").await.unwrap().input(), input);
        assert_eq!(other_provider.batches.lock().unwrap()[0], ["Document: same"]);
        drop(other_session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn distinct_uncached_requests_are_batched_and_token_refusals_remain_per_text() {
        let provider = Changing { specification: FakeEmbedder::new().spec().clone(), batches: Default::default(), unavailable: false };
        let budget = ResourceBudget::fixed(1 << 27).unwrap();
        let selection = Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(selection, &provider, None, &budget).unwrap();
        session.prepare(["first", "second", "first", "over"]).await.unwrap();
        assert_eq!(session.realize("first").await.unwrap().tokens(), 1);
        assert_eq!(session.realize("second").await.unwrap().tokens(), 1);
        assert!(matches!(session.realize("over").await, Err(Error::TokenLimit { .. })));
        let batches = provider.batches.lock().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
        assert!(batches[0].contains(&"first".into()) && batches[0].contains(&"second".into()));
        drop(batches); drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn unavailable_batch_keeps_each_refusal_and_never_invents_empty_vectors() {
        let provider = Changing { specification: FakeEmbedder::new().spec().clone(), batches: Default::default(), unavailable: true };
        let budget = ResourceBudget::fixed(1 << 27).unwrap();
        let selection = Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(selection, &provider, None, &budget).unwrap();
        session.prepare(["first", "second", "over"]).await.unwrap();
        assert!(session.realize("first").await.err().unwrap().unavailable());
        assert!(session.realize("second").await.err().unwrap().unavailable());
        assert!(matches!(session.realize("over").await, Err(Error::TokenLimit { .. })));
        assert_eq!(provider.batches.lock().unwrap().len(), 1);
        drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[derive(Default)]
    struct Cache {
        values: std::sync::Mutex<BTreeMap<(ContentHash, ContentHash), CacheValue>>,
        lookups: std::sync::Mutex<Vec<Vec<ContentHash>>>,
        admissions: std::sync::Mutex<Vec<Vec<ContentHash>>>,
    }
    impl EmbeddingCache for Cache {
        fn cached<'b>(&'b self, spec: &'b embedding::Spec, keys: &'b [ContentHash]) -> CacheFuture<'b, BTreeMap<ContentHash, CacheValue>> {
            Box::pin(async move {
                self.lookups.lock().unwrap().push(keys.to_vec());
                let values = self.values.lock().unwrap();
                Ok(keys.iter().filter_map(|key| values.get(&(spec.hash(), *key)).cloned().map(|v| (*key, v))).collect())
            })
        }
        fn admit<'b>(&'b self, spec: &'b embedding::Spec, candidates: &'b [CacheValue]) -> CacheFuture<'b, BTreeMap<ContentHash, CacheValue>> {
            Box::pin(async move {
                self.admissions.lock().unwrap().push(candidates.iter().map(|v| v.input_hash).collect());
                let mut values = self.values.lock().unwrap();
                Ok(candidates.iter().map(|candidate| {
                    let winner = values.entry((spec.hash(), candidate.input_hash)).or_insert_with(|| candidate.clone()).clone();
                    (candidate.input_hash, winner)
                }).collect())
            })
        }
    }
    #[tokio::test]
    async fn cache_lookups_and_admissions_share_the_unique_request_batch() {
        let provider = Changing { specification: FakeEmbedder::new().spec().clone(), batches: Default::default(), unavailable: false };
        let budget = ResourceBudget::fixed(1 << 27).unwrap();
        let cache = Arc::new(Cache::default());
        let mut vector = vec![0.0; provider.spec().dimensions as usize]; vector[0] = -1.0;
        cache.values.lock().unwrap().insert((provider.spec().hash(), input_hash("cached")), CacheValue { input_hash: input_hash("cached"), vector: vector.clone(), admitted_tokens: 1 });
        let configuration = Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap();
        let mut session = Session::selected(configuration, &provider, Some(cache.clone()), &budget).unwrap();
        session.prepare(["cached", "second", "third", "second"]).await.unwrap();
        assert_eq!(session.realize("cached").await.unwrap().bytes(), embedding::value::encode_vector(&vector));
        assert!(session.realize("second").await.is_ok() && session.realize("third").await.is_ok());
        assert_eq!(cache.lookups.lock().unwrap().iter().map(Vec::len).collect::<Vec<_>>(), [3]);
        assert_eq!(cache.admissions.lock().unwrap().iter().map(Vec::len).collect::<Vec<_>>(), [2]);
        assert_eq!(provider.batches.lock().unwrap().iter().map(Vec::len).collect::<Vec<_>>(), [2]);
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
