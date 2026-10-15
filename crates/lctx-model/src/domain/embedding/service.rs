//! Embedding service interface, independent of compiler orchestration and transport clients.
use super::Spec;
use crate::domain::ModelError;
use std::{future::Future, pin::Pin};

/// A future an embedder returns.
pub type EmbedFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ModelError>> + Send + 'a>>;

/// An embedding service under one spec.
pub trait Embedder: Send + Sync {
    fn spec(&self) -> &Spec;
    fn endpoint(&self) -> &str;
    /// Exact acquired local complete-document tokenizer, shared by every construction grain.
    fn document_tokenizer(
        &self,
    ) -> Option<std::sync::Arc<dyn crate::domain::retrieval::partition::Tokenizer>> {
        None
    }
    /// The number of the served model's tokens in a request text.
    fn count_tokens<'a>(&'a self, request_text: &'a str) -> EmbedFuture<'a, usize>;
    /// One vector per request text, in order, each checked ([`super::check_vector`]).
    fn embed<'a>(&'a self, request_texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>>;
}

/// Optional service availability is distinct from invalid responses and vectors.
#[derive(Debug, thiserror::Error)]
#[error("embedding service: {0}")]
pub struct ServiceUnavailable(pub String);
impl ServiceUnavailable {
    pub fn cause(detail: impl Into<String>) -> ModelError {
        ModelError::Cause(Box::new(Self(detail.into())))
    }
    pub fn detail<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a str> {
        if let Some(unavailable) = error.downcast_ref::<Self>() {
            return Some(&unavailable.0);
        }
        if let Some(model) = error.downcast_ref::<ModelError>() {
            match model {
                ModelError::Cause(cause) => return Self::detail(cause.as_ref()),
                ModelError::SharedCause(cause) => return Self::detail(cause.as_ref()),
                // Finalization has independent obligations; a primary availability cause
                // cannot make the enclosing terminal/cleanup result optional.
                ModelError::Completion(_) => return None,
                _ => {}
            }
        }
        error.source().and_then(Self::detail)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[derive(Debug, thiserror::Error)]
    #[error("consumer context")]
    struct Context(#[source] ModelError);
    #[test]
    fn availability_survives_typed_wrapping_but_invalid_data_never_becomes_optional() {
        let direct = ServiceUnavailable::cause("offline");
        let shared = ModelError::SharedCause(std::sync::Arc::new(direct));
        let foreign = ModelError::Cause(Box::new(Context(shared)));
        assert_eq!(ServiceUnavailable::detail(&foreign), Some("offline"));
        let completed =
            ModelError::Completion(Box::new(crate::domain::completion::OperationFailure {
                primary: Some(Box::new(foreign)),
                completion: crate::domain::completion::Completion::default(),
            }));
        assert_eq!(ServiceUnavailable::detail(&completed), None);
        for error in [
            ModelError::Invalid("invalid vector".into()),
            ModelError::Codec("bad response".into()),
        ] {
            let wrapped = ModelError::Cause(Box::new(Context(error)));
            assert_eq!(ServiceUnavailable::detail(&wrapped), None);
        }
    }
}
