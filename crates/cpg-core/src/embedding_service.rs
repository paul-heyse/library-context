//! Compile-time embedding service interface; semantic configuration belongs to lctx-model.
use std::{future::Future,pin::Pin};
use sha2::{Digest as _,Sha256};
use crate::CoreError;

/// A future an embedder returns.
pub type EmbedFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, CoreError>> + Send + 'a>>;

pub use lctx_model::domain::embedding::{Spec, check_vector};

/// The cache key of a request text: its SHA-256.
pub use lctx_model::domain::embedding::value::input_hash;


/// An embedding service under one spec.
pub trait Embedder: Send + Sync {
    fn spec(&self) -> &Spec;
    fn endpoint(&self) -> &str;
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
                query_task: "Given a coding task, retrieve relevant Python library APIs, capability briefs, configuration options, source code, usage examples, and documentation.".to_owned(),
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
    fn endpoint(&self) -> &str {"fixture://deterministic"}
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

