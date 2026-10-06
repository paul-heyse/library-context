//! Effect interface for immutable embedding winners; the native store owns effects.
use super::Spec;
use crate::domain::{ContentHash,ModelError};
use std::{future::Future,pin::Pin,collections::BTreeMap};
/// Cache winners are keyed by the complete embedding specification and request identity.
#[derive(Clone, Debug)]
pub struct CacheValue {
    pub input_hash: ContentHash,
    pub vector: Vec<f32>,
    pub admitted_tokens: u32,
}
pub type CacheFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ModelError>> + Send + 'a>>;
pub trait EmbeddingCache: Send + Sync {
    fn cached<'a>(
        &'a self,
        spec: &'a Spec,
        keys: &'a [ContentHash],
    ) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>>;
    /// Atomically choose existing or new immutable winners; return a winner for every candidate.
    fn admit<'a>(
        &'a self,
        spec: &'a Spec,
        candidates: &'a [CacheValue],
    ) -> CacheFuture<'a, BTreeMap<ContentHash, CacheValue>>;
}
