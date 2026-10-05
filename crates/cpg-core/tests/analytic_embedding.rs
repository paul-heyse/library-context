//! Explicit deterministic embedding effects exercise codec and refusal contracts, not live quality.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier,embedding,stages::Profile};
type UseRow=(i16,Option<i64>,Option<i16>,Option<Vec<u8>>);
async fn run(mode:Mode) {
    let provider=ContractEmbedder {inner:cpg_core::embedding_service::FakeEmbedder::new(),mode};
    let mut settings=catalog_runtime::settings("analytic_text");
    settings.knn=!matches!(mode,Mode::Disabled);
    let requested=settings.knn;
    let fixture=catalog_runtime::compile("normalized_relations",Profile::Catalog,Frontier::Analysis,settings,if requested{Some(&provider)}else{None}).await;
    let uses: Vec<UseRow> = catalog_runtime::query(&fixture, "SELECT availability, admitted_tokens, codec, bytes FROM analysis_embedding_uses").await;
    let outcomes: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM analytic_embedding_analysis_outcomes").await;
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
                && *c == Some(1)
                && b.as_ref().is_some_and(|b| b.len() == 4096)));
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

