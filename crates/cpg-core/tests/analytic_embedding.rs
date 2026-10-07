//! Explicit deterministic embedding effects exercise codec and refusal contracts, not live quality.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, embedding, stages::Profile};
type UseRow = (i16, Option<i64>, Option<Vec<u8>>, Option<Vec<u8>>);
async fn run(mode: Mode) {
    let provider = ContractEmbedder {
        inner: cpg_core::embedding_service::FakeEmbedder::new(),
        mode,
    };
    let mut settings = catalog_runtime::settings("analytic_text");
    settings.knn = !matches!(mode, Mode::Disabled);
    let requested = settings.knn;
    let fixture = catalog_runtime::compile(
        "normalized_relations",
        Profile::Catalog,
        Frontier::Analysis,
        settings,
        if requested { Some(&provider) } else { None },
    )
    .await;
    let uses: Vec<UseRow> = catalog_runtime::query(
        &fixture,
        "SELECT availability, admitted_tokens, value, projection FROM analysis_embedding_uses",
    )
    .await;
    let outcomes: Vec<i16> = catalog_runtime::query(
        &fixture,
        "SELECT status FROM analytic_embedding_analysis_outcomes",
    )
    .await;
    assert!(!outcomes.is_empty());
    match mode {
        Mode::Disabled => {
            assert!(uses.is_empty());
            assert!(outcomes.iter().all(|s| *s == 3));
        }
        Mode::Available => {
            assert!(!uses.is_empty());
            assert!(uses.iter().all(|(a, t, c, b)| *a == 0
                && t.is_some()
                && c.as_ref().is_some_and(|id| id.len() == 16)
                && b.as_ref().is_some_and(|id| id.len() == 16)));
            let full: Vec<(i64, Vec<u8>)> = catalog_runtime::query(
                &fixture,
                "SELECT dimensions, bytes FROM embedding_full_values",
            )
            .await;
            let projected: Vec<(i64, Vec<u8>)> = catalog_runtime::query(
                &fixture,
                "SELECT dimensions, bytes FROM embedding_projected_values",
            )
            .await;
            assert!(!full.is_empty() && !projected.is_empty());
            assert!(
                full.iter()
                    .all(|(dimensions, bytes)| *dimensions == 4096 && bytes.len() == 16384)
            );
            assert!(
                projected
                    .iter()
                    .all(|(dimensions, bytes)| *dimensions == 1024 && bytes.len() == 4096)
            );
            assert!(outcomes.iter().all(|s| *s == 0));
        }
        Mode::Unavailable => {
            assert!(!uses.is_empty());
            assert!(
                uses.iter()
                    .all(|(a, t, c, b)| *a == 1 && t.is_none() && c.is_none() && b.is_none())
            );
            assert!(outcomes.iter().all(|s| *s == 1));
        }
        Mode::TokenLimit => {
            assert!(!uses.is_empty());
            assert!(uses.iter().all(|(a, t, c, b)| *a == 2
                && t.is_some_and(|n| n > 2048)
                && c.is_none()
                && b.is_none()));
            assert!(outcomes.iter().all(|s| *s == 1));
        }
    }
}
use cpg_core::embedding_service::{EmbedFuture, Embedder};
#[derive(Clone, Copy)]
enum Mode {
    Available,
    Disabled,
    Unavailable,
    TokenLimit,
}
struct ContractEmbedder {
    inner: cpg_core::embedding_service::FakeEmbedder,
    mode: Mode,
}
impl Embedder for ContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        self.inner.spec()
    }
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }
    fn document_tokenizer(
        &self,
    ) -> Option<std::sync::Arc<dyn lctx_model::domain::retrieval::partition::Tokenizer>> {
        if matches!(self.mode, Mode::TokenLimit) {
            None
        } else {
            self.inner.document_tokenizer()
        }
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        match self.mode {
            Mode::TokenLimit => Box::pin(async { Ok(4096) }),
            Mode::Unavailable => Box::pin(async {
                Err(cpg_core::CoreError::EmbeddingService(
                    "contract service unavailable".into(),
                ))
            }),
            _ => self.inner.count_tokens(text),
        }
    }
    fn embed<'a>(&'a self, text: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        if matches!(self.mode, Mode::Unavailable) {
            return Box::pin(async {
                Err(cpg_core::CoreError::EmbeddingService(
                    "contract service unavailable".into(),
                ))
            });
        }
        self.inner.embed(text)
    }
}
#[tokio::test]
async fn native_analytic_use_publishes_and_cold_replays_exact_vectors() {
    run(Mode::Available).await;
}
#[tokio::test]
async fn native_unrequested_analytic_use_has_no_service_effect() {
    run(Mode::Disabled).await;
}
#[tokio::test]
async fn native_optional_failures_preserve_explicit_per_window_availability() {
    run(Mode::Unavailable).await;
    run(Mode::TokenLimit).await;
}

#[derive(Default)]
struct EffectTrace {
    tokens: std::collections::BTreeMap<lctx_model::domain::ContentHash, usize>,
    embeddings: std::collections::BTreeMap<lctx_model::domain::ContentHash, usize>,
    batches: Vec<Vec<String>>,
}
struct ChangingContractEmbedder {
    inner: cpg_core::embedding_service::FakeEmbedder,
    trace: std::sync::Mutex<EffectTrace>,
}
impl Embedder for ChangingContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        self.inner.spec()
    }
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }
    fn document_tokenizer(
        &self,
    ) -> Option<std::sync::Arc<dyn lctx_model::domain::retrieval::partition::Tokenizer>> {
        self.inner.document_tokenizer()
    }
    fn count_tokens<'a>(&'a self, request: &'a str) -> EmbedFuture<'a, usize> {
        let key = embedding::value::input_hash(request);
        *self.trace.lock().unwrap().tokens.entry(key).or_default() += 1;
        self.inner.count_tokens(request)
    }
    fn embed<'a>(&'a self, requests: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            let mut trace = self.trace.lock().unwrap();
            trace.batches.push(requests.to_vec());
            Ok(requests
                .iter()
                .map(|request| {
                    let count = trace
                        .embeddings
                        .entry(embedding::value::input_hash(request))
                        .or_default();
                    let coordinate = *count % self.spec().dimensions as usize;
                    *count += 1;
                    // Every answer is a valid unit vector, but repeating a request changes its bytes.
                    let mut vector = vec![0.0; self.spec().dimensions as usize];
                    vector[coordinate] = 1.0;
                    vector
                })
                .collect())
        })
    }
}

type ExactReceipt = (i64, i16, Vec<u8>, Vec<u8>);
type ReceiptRow = (Vec<u8>, Vec<u8>, i64, i16, Vec<u8>, Vec<u8>);
fn receipts(rows: Vec<ReceiptRow>) -> std::collections::BTreeMap<(Vec<u8>, Vec<u8>), ExactReceipt> {
    let mut receipts = std::collections::BTreeMap::new();
    for (specification, input, tokens, codec, digest, bytes) in rows {
        let receipt = (tokens, codec, digest, bytes);
        if let Some(previous) = receipts.insert((specification, input), receipt.clone()) {
            assert_eq!(
                previous, receipt,
                "nominal uses changed their exact shared winner"
            );
        }
    }
    receipts
}

#[tokio::test]
async fn native_catalog_reuses_analytic_winners_and_batches_unique_requests_without_a_cache() {
    use lctx_model::domain::{ContentHash, Record};
    let provider = ChangingContractEmbedder {
        inner: cpg_core::embedding_service::FakeEmbedder::new(),
        trace: Default::default(),
    };
    let mut settings = catalog_runtime::settings("analytic_text");
    settings.knn = true;
    // The fixture helper invokes the actual native compiler with cache=None.
    let fixture = catalog_runtime::compile(
        "normalized_relations",
        Profile::Catalog,
        Frontier::Catalog,
        settings,
        Some(&provider),
    )
    .await;
    let analytic = receipts(catalog_runtime::query(&fixture,
        "SELECT winner.encoder, winner.input, winner.tokens, winner.codec, winner.digest, winner.bytes FROM analysis_embedding_uses consumption JOIN embedding_full_values winner ON winner.id=consumption.value WHERE consumption.availability=0").await);
    let retrieval = receipts(catalog_runtime::query(&fixture,
        "SELECT winner.encoder, winner.input, winner.tokens, winner.codec, winner.digest, winner.bytes FROM retrieval_embedding_uses consumption JOIN embedding_full_values winner ON winner.id=consumption.value WHERE consumption.availability=0").await);
    assert!(!analytic.is_empty() && !retrieval.is_empty());
    let overlap = analytic
        .keys()
        .filter(|key| retrieval.contains_key(*key))
        .collect::<Vec<_>>();
    // Contextual windows may have distinct actual encoder inputs. Every actual overlap must
    // share the canonical winner; deliberate seeded-input reuse is covered by Session controls.
    for key in &overlap {
        assert_eq!(analytic.get(*key), retrieval.get(*key));
    }
    let specification = embedding::EmbeddingSpec::new(provider.spec())
        .unwrap()
        .id()
        .bytes()
        .to_vec();
    let expected = analytic
        .keys()
        .chain(retrieval.keys())
        .map(|(spec, input)| {
            assert_eq!(spec, &specification);
            ContentHash(input.as_slice().try_into().unwrap())
        })
        .collect::<std::collections::BTreeSet<_>>();
    let trace = provider.trace.lock().unwrap();
    assert_eq!(
        trace
            .embeddings
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    assert!(
        trace.tokens.is_empty(),
        "complete local tokenization needs no token endpoint calls"
    );
    assert!(
        trace.embeddings.values().all(|count| *count == 1),
        "E1/E0 repeated a service request despite its completed winner"
    );
    assert!(
        trace.batches.iter().any(|batch| batch.len() > 1),
        "distinct requests never reached the bulk service interface"
    );
    assert!(trace.batches.iter().all(|batch| batch.len() <= 64));
}
