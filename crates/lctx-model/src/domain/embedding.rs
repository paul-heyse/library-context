//! Shared embedding specification and exact value contracts. Consumption has distinct owners.
pub mod analytic;
pub mod cache;
pub mod configuration;
pub mod consumption;
pub(crate) mod spec;
pub mod text;
pub mod value;
pub mod projection;
use super::*;
use crate::{Domain, DomainCode};
pub use spec::{MatryoshkaAdmission, Spec, check_vector, QUERY_TEMPLATE, QUERY_TASK, QueryRecipe};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Reduction {
    None = 0,
    MatryoshkaPrefix = 1,
}

/// Typed persistence of the selected configuration. The protocol JSON is derived mechanically;
/// it is never a second mutable store or an unparsed semantic column.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
    pub dimensions: i64,
    pub output_dtype: String,
    pub normalization: String,
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
            dimensions: spec.dimensions.into(),
            output_dtype: spec.output_dtype.clone(),
            normalization: spec.normalization.clone(),
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
            query_template: QUERY_TEMPLATE.into(),
            query_task: QUERY_TASK.into(),
            document_template: "{text}".into(),
            dimensions: integer(self.dimensions)?,
            output_dtype: self.output_dtype.clone(),
            normalization: self.normalization.clone(),
            max_document_tokens: 2048,
            max_query_tokens:8192,
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

/// Document admission/rendering is independent of encoder and query identity.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="embedding_document_recipes",validate=validate_document_recipe)]
pub struct DocumentRecipe {
    #[model(key)] pub digest: ContentHash,
    pub template: String,
    pub max_tokens: i64,
}
impl DocumentRecipe {
    pub fn new(spec: &Spec) -> Result<Self, ModelError> {
        spec.validate().map_err(ModelError::Invalid)?;
        Ok(Self { digest: spec.document_hash(), template: spec.document_template.clone(), max_tokens: spec.max_document_tokens.into() })
    }
    pub fn configuration(&self, encoder: &EmbeddingSpec) -> Result<Spec, ModelError> {
        let mut spec = encoder.configuration()?;
        spec.document_template.clone_from(&self.template);
        spec.max_document_tokens = u32::try_from(self.max_tokens).map_err(ModelError::codec)?;
        spec.validate().map_err(ModelError::Invalid)?;
        if self.digest != spec.document_hash() { return Err(ModelError::Invalid("document recipe digest mismatch".into())); }
        Ok(spec)
    }
}
fn validate_document_recipe(row: &DocumentRecipe) -> Result<(),ModelError> {
    if row.max_tokens <= 0 || row.max_tokens > u32::MAX.into() || row.template.matches("{text}").count()!=1 {
        return Err(ModelError::Invalid("invalid document recipe".into()));
    }
    // Same typed recipe serialization as Spec, without encoder dependency.
    #[derive(serde::Serialize)] struct Recipe<'a> { template: &'a str, max_tokens: u32 }
    if row.digest != spec::recipe_hash("document/v3", &Recipe {template:&row.template,max_tokens:row.max_tokens as u32}) {
        return Err(ModelError::Invalid("document recipe digest mismatch".into()));
    }
    Ok(())
}

pub fn configuration_relations() -> Vec<Relation> {
    vec![
        Relation::of::<EmbeddingSpec>(),
        Relation::of::<DocumentRecipe>(),
        Relation::of::<projection::ProjectionDefinition>(),
        Relation::of::<configuration::ServiceConfiguration>(),
    ]
}
pub fn relations() -> Vec<Relation> {
    let mut rows = configuration_relations();
    rows.push(Relation::of::<value::FullValue>());
    rows.push(Relation::of::<projection::ProjectedValue>());
    rows.extend(text::relations());
    rows.extend(analytic::relations());
    rows
}
