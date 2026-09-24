//! The compile-time embedding client (DESIGN §11.1; ADR-0010): `cpg_core::embed::Embedder` over a
//! separate vLLM 0.30.0 service serving Qwen3-Embedding-8B.
//!
//! - The request body is built here with `serde_json`, so its bytes are ours and the Python query
//!   client can be held to the same bytes (`specs/embedding/request_bodies.json`, E2).
//! - Every response is checked: the vector count, the index mapping, the length, finiteness and
//!   unit norm, and the served model. `dimensions` is never sent (vLLM rejects it without a
//!   Matryoshka override, E1).
//! - Token counts come from the service's own tokenizer (`POST /tokenize`, attached for every
//!   runner in vLLM 0.30.0), so no Rust tokenizer is needed.
//! - An unreachable service is an error naming the URL: a compile asked for live vectors is
//!   `blocked` without the service, never silently fake.

use cpg_core::CoreError;
use cpg_core::embed::{EmbedFuture, Embedder, Spec, check_vector};
use serde::{Deserialize, Serialize};

/// The committed spec (canonical JSON; its SHA-256 is the spec hash).
pub const QWEN_SPEC: &str = include_str!("../../../specs/embedding/qwen3-embedding-8b.json");

/// The spec every live vector is made under.
pub fn qwen_spec() -> Spec {
    serde_json::from_str(QWEN_SPEC).expect("the committed spec parses")
}

/// An embeddings request (the holistic assessment's D3): a struct, so its key order is its field
/// order whatever `serde_json` features the build enables (DataFusion turns on `preserve_order`).
#[derive(Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    input: &'a [String],
    encoding_format: &'a str,
}

/// A tokenize request: the model and one prompt.
#[derive(Serialize)]
struct TokenizeRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

/// The embedding request body for request texts: the model, the texts, float encoding.
pub fn request_body(spec: &Spec, request_texts: &[String]) -> Vec<u8> {
    serde_json::to_vec(&EmbeddingsRequest {
        model: &spec.model,
        input: request_texts,
        encoding_format: "float",
    })
    .expect("a request serializes")
}

#[derive(Deserialize)]
struct Embeddings {
    model: String,
    data: Vec<Datum>,
}

#[derive(Deserialize)]
struct Datum {
    index: usize,
    embedding: Vec<f64>,
}

#[derive(Deserialize)]
struct Tokenized {
    count: usize,
}

/// Parse and check an embeddings response for `n` inputs (§11.1's rejections).
pub fn parse_embeddings(spec: &Spec, n: usize, body: &[u8]) -> Result<Vec<Vec<f32>>, String> {
    let parsed: Embeddings =
        serde_json::from_slice(body).map_err(|e| format!("unreadable response: {e}"))?;
    if parsed.model != spec.model {
        return Err(format!(
            "the service answered with model {}, not {}",
            parsed.model, spec.model
        ));
    }
    if parsed.data.len() != n {
        return Err(format!("{} vectors for {n} inputs", parsed.data.len()));
    }
    let mut out: Vec<Option<Vec<f32>>> = vec![None; n];
    for d in parsed.data {
        let slot = out
            .get_mut(d.index)
            .ok_or_else(|| format!("index {} out of range", d.index))?;
        if slot.is_some() {
            return Err(format!("index {} answered twice", d.index));
        }
        let v: Vec<f32> = d.embedding.iter().map(|x| *x as f32).collect();
        check_vector(&v, spec.dimensions)?;
        *slot = Some(v);
    }
    out.into_iter()
        .map(|v| v.ok_or_else(|| "an index is missing".to_owned()))
        .collect()
}

/// How long a connection to the service may take.
pub const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// How long one request may take: a batch of 32 documents at the 2,048-token cap is about 65,000
/// tokens, seconds of work for the served model.
pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// A vLLM service at `base` (e.g. `http://127.0.0.1:8000`) under the committed spec.
pub struct VllmEmbedder {
    base: String,
    spec: Spec,
    client: reqwest::Client,
}

impl VllmEmbedder {
    pub fn new(base: &str, spec: Spec) -> Self {
        Self {
            base: base.trim_end_matches('/').to_owned(),
            spec,
            // The holistic assessment's D3: a hung service fails the compile rather than hanging it.
            client: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("a client builds"),
        }
    }

    async fn post(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, CoreError> {
        let url = format!("{}{path}", self.base);
        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                CoreError::Embed(format!("blocked: no embedding service at {url}: {e}"))
            })?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| CoreError::Embed(format!("{url}: {e}")))?;
        if !status.is_success() {
            return Err(CoreError::Embed(format!(
                "{url} answered {status}: {}",
                String::from_utf8_lossy(&bytes)
            )));
        }
        Ok(bytes.to_vec())
    }
}

impl Embedder for VllmEmbedder {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn count_tokens<'a>(&'a self, request_text: &'a str) -> EmbedFuture<'a, usize> {
        Box::pin(async move {
            let body = serde_json::to_vec(&TokenizeRequest {
                model: &self.spec.model,
                prompt: request_text,
            })
            .expect("a request serializes");
            let bytes = self.post("/tokenize", body).await?;
            let parsed: Tokenized = serde_json::from_slice(&bytes)
                .map_err(|e| CoreError::Embed(format!("unreadable /tokenize response: {e}")))?;
            Ok(parsed.count)
        })
    }

    fn embed<'a>(&'a self, request_texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            let bytes = self
                .post("/v1/embeddings", request_body(&self.spec, request_texts))
                .await?;
            parse_embeddings(&self.spec, request_texts.len(), &bytes).map_err(CoreError::Embed)
        })
    }
}
