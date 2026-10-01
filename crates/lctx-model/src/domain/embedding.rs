//! Shared embedding specification and exact value contracts. Consumption has distinct owners.
pub mod configuration;
mod spec;
pub mod text;
pub mod value;
use super::*;
use crate::{Domain, DomainCode};
pub use spec::{MatryoshkaAdmission, Spec, check_vector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Reduction {
    None = 0,
    MatryoshkaPrefix = 1,
}

/// Typed persistence of the selected configuration. The protocol JSON is derived mechanically;
/// it is never a second mutable store or an unparsed semantic column.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="embedding_specifications",validate=validate_spec)]
pub struct EmbeddingSpec {
    #[model(key)]
    pub service_hash: ContentHash,
    pub format: i64,
    pub source_dimensions: i64,
    pub reduction: Reduction,
    pub is_matryoshka: Option<bool>,
    pub matryoshka_dimensions: Vec<i64>,
    pub use_activation: Option<bool>,
    pub model: String,
    pub revision: String,
    pub tokenizer_revision: String,
    pub server: String,
    pub served_dtype: String,
    pub pooling: String,
    pub query_template: String,
    pub query_task: String,
    pub document_template: String,
    pub dimensions: i64,
    pub output_dtype: String,
    pub normalization: String,
    pub max_document_tokens: i64,
}
impl EmbeddingSpec {
    pub fn new(spec: &Spec) -> Result<Self, ModelError> {
        spec.validate().map_err(ModelError::Invalid)?;
        Ok(Self {
            service_hash: spec.hash(),
            format: spec.format.into(),
            source_dimensions: spec.source_dimensions.into(),
            reduction: match spec.reduction.as_str() {
                "none" => Reduction::None,
                "mrl-prefix" => Reduction::MatryoshkaPrefix,
                _ => unreachable!("validated reduction"),
            },
            is_matryoshka: spec.admission.as_ref().map(|a| a.is_matryoshka),
            matryoshka_dimensions: spec
                .admission
                .as_ref()
                .map(|a| {
                    a.matryoshka_dimensions
                        .iter()
                        .map(|n| i64::from(*n))
                        .collect()
                })
                .unwrap_or_default(),
            use_activation: spec.admission.as_ref().map(|a| a.use_activation),
            model: spec.model.clone(),
            revision: spec.revision.clone(),
            tokenizer_revision: spec.tokenizer_revision.clone(),
            server: spec.server.clone(),
            served_dtype: spec.served_dtype.clone(),
            pooling: spec.pooling.clone(),
            query_template: spec.query_template.clone(),
            query_task: spec.query_task.clone(),
            document_template: spec.document_template.clone(),
            dimensions: spec.dimensions.into(),
            output_dtype: spec.output_dtype.clone(),
            normalization: spec.normalization.clone(),
            max_document_tokens: spec.max_document_tokens.into(),
        })
    }
    pub fn configuration(&self) -> Result<Spec, ModelError> {
        let integer = |n: i64| {
            u32::try_from(n).map_err(|_| {
                ModelError::Invalid("embedding configuration integer out of range".into())
            })
        };
        let admission = match (self.is_matryoshka, self.use_activation) {
            (None, None) if self.matryoshka_dimensions.is_empty() => None,
            (Some(is_matryoshka), Some(use_activation)) => Some(MatryoshkaAdmission {
                is_matryoshka,
                use_activation,
                matryoshka_dimensions: self
                    .matryoshka_dimensions
                    .iter()
                    .map(|n| integer(*n))
                    .collect::<Result<_, _>>()?,
            }),
            _ => {
                return Err(ModelError::Invalid(
                    "incomplete embedding reduction admission".into(),
                ));
            }
        };
        let spec = Spec {
            format: integer(self.format)?,
            source_dimensions: integer(self.source_dimensions)?,
            reduction: match self.reduction {
                Reduction::None => "none",
                Reduction::MatryoshkaPrefix => "mrl-prefix",
            }
            .into(),
            admission,
            model: self.model.clone(),
            revision: self.revision.clone(),
            tokenizer_revision: self.tokenizer_revision.clone(),
            server: self.server.clone(),
            served_dtype: self.served_dtype.clone(),
            pooling: self.pooling.clone(),
            query_template: self.query_template.clone(),
            query_task: self.query_task.clone(),
            document_template: self.document_template.clone(),
            dimensions: integer(self.dimensions)?,
            output_dtype: self.output_dtype.clone(),
            normalization: self.normalization.clone(),
            max_document_tokens: integer(self.max_document_tokens)?,
        };
        spec.validate().map_err(ModelError::Invalid)?;
        if spec.hash() != self.service_hash {
            return Err(ModelError::Invalid(
                "embedding specification differs from its service digest".into(),
            ));
        }
        Ok(spec)
    }
}
fn validate_spec(row: &EmbeddingSpec) -> Result<(), ModelError> {
    row.configuration().map(|_| ())
}

pub fn configuration_relations() -> Vec<Relation> {
    vec![
        Relation::of::<EmbeddingSpec>(),
        Relation::of::<configuration::ServiceConfiguration>(),
    ]
}
pub fn relations() -> Vec<Relation> {
    let mut rows = configuration_relations();
    rows.extend(text::relations());
    rows
}
