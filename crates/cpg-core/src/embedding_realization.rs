//! One effect owner for token admission and immutable cache winners, shared by E1 and E0.
use crate::{
    CoreError,
    embedding_service::Embedder,
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::AttemptRuntime,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    embedding::{
        EmbeddingSpec,
        configuration::{Configuration, ServiceConfiguration},
        value::{AdmittedValue, input_hash},
    },
    resources::ResourceBudget,
    stages::*,
    *,
};
use lctx_postgres::{CacheValue, Store, generations::GenerationAttempt, roles::RoleConfig};
use std::{collections::BTreeMap, sync::Arc};

/// Resource and corruption failures remain distinct from optional service availability.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Cache(#[from] lctx_postgres::Error),
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
    cache: Option<Store>,
    budget: ResourceBudget,
    values: BTreeMap<ContentHash, AdmittedValue>,
    charge: charged::StateCharge,
}
impl<'a> Session<'a> {
    /// The selected configuration is read through the caller's actual completed R0 grants.
    /// A pure stage cannot acquire this effect. No pool lease survives a service request.
    pub async fn open(
        access: &StageAccess<'_, '_>,
        attempt: &GenerationAttempt,
        roles: &RoleConfig,
        runtime: &AttemptRuntime,
        model: &Arc<ValidatedModel>,
        embedder: &'a dyn Embedder,
        cache: Option<Store>,
    ) -> Result<Self, ModelError> {
        if access.stage().effect != Effect::Embedding {
            return Err(ModelError::Invalid(
                "embedding effect was not declared".into(),
            ));
        }
        let reader = AttemptSession::open(
            roles,
            attempt,
            access,
            model.clone(),
            ProviderOptions::default(),
        )
        .await
        .map_err(ModelError::codec)?;
        let session = runtime.session(access);
        let mut specs = normalized::Rows::<EmbeddingSpec>::new(runtime.budget());
        let mut services = normalized::Rows::<ServiceConfiguration>::new(runtime.budget());
        macro_rules! read {
            ($ty:ty,$rows:ident) => {{
                let permit = access.read::<$ty>()?;
                session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
                let query = session
                    .query(&format!("SELECT * FROM \"{}\"", <$ty>::NAME))
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
        reader.close().await.map_err(ModelError::codec)?;
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
        cache: Option<Store>,
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
        })
    }
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
    /// Returns an exact immutable winner. A consumer stores its nominal use before decoding this
    /// value for a kernel; the shared model validator replays bytes without service or cache I/O.
    pub async fn realize(&mut self, document: &str) -> Result<&AdmittedValue, Error> {
        let spec = self.configuration.specification();
        let request_bound = document
            .len()
            .checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len()))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| ModelError::Invalid("embedding request allocation overflow".into()))?;
        let _request = self.budget.reserve("embedding-request", request_bound)?;
        let request = spec.document_text(document);
        let key = input_hash(&request);
        if self.values.contains_key(&key) {
            return Ok(&self.values[&key]);
        }
        // Bound the transient cache/service vector batch and request clones before crossing effects.
        let _effect = self.budget.reserve(
            "embedding-effect-buffers",
            spec.dimensions as usize * 128 + request.len() * 4 + 8192,
        )?;
        let cached = if let Some(cache) = &self.cache {
            cache.cached(spec, &[key]).await?.remove(&key)
        } else {
            None
        };
        let winner = match cached {
            Some(winner) => winner,
            None => {
                let tokens = self.embedder.count_tokens(&request).await?;
                if tokens > spec.max_document_tokens as usize {
                    return Err(Error::TokenLimit {
                        tokens,
                        limit: spec.max_document_tokens,
                    });
                }
                let mut vectors = self.embedder.embed(std::slice::from_ref(&request)).await?;
                if vectors.len() != 1 {
                    return Err(ModelError::Invalid(
                        "embedding response count differs from request".into(),
                    )
                    .into());
                }
                let candidate = CacheValue {
                    input_hash: key,
                    vector: vectors.pop().expect("one response"),
                    admitted_tokens: tokens as u32,
                };
                lctx_model::domain::embedding::check_vector(&candidate.vector, spec.dimensions)
                    .map_err(ModelError::Invalid)?;
                if let Some(cache) = &self.cache {
                    cache.ensure_spec(spec).await?;
                    cache
                        .admit(spec, &[candidate])
                        .await?
                        .remove(&key)
                        .ok_or_else(|| {
                            ModelError::Invalid("cache omitted its immutable winner".into())
                        })?
                } else {
                    candidate
                }
            }
        };
        if winner.input_hash != key {
            return Err(
                ModelError::Invalid("embedding winner has another input hash".into()).into(),
            );
        }
        let admitted = AdmittedValue::new(
            spec,
            &request,
            winner.admitted_tokens,
            &winner.vector,
            &self.budget,
        )?;
        self.charge
            .grow(size_of::<ContentHash>() + size_of::<AdmittedValue>() + 64)?;
        self.values.insert(key, admitted);
        Ok(&self.values[&key])
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
