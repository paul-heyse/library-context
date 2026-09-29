//! Delta persistence of the fact-family tables, the one read-only SQL helper, and the compile
//! attempt: write, derive, validate, publish (DESIGN §4.3, §6, §8).

pub mod analyze;
pub mod arrow_types;
pub mod behavior;
pub mod bundle;
pub mod catalog;
pub mod embed;
pub mod entry_links;
pub mod evidence;
pub mod flow_model;
pub mod postgres;
pub mod producer;
pub mod session;
pub mod model_runtime;
pub mod generation_read;
pub mod sql;
pub mod summaries;
pub mod synth;
pub mod udf;
pub mod usage;
pub mod validate;

use validate::Violation;

fn summary(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|v| format!("{} ({} rows)", v.rule, v.rows))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error("datafusion: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),
    #[error("arrow: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("a validation rule's task failed: {0}")]
    Rule(String),
    #[error("not a raw table: {0}")]
    UnknownTable(String),
    #[error("analysis: {0}")]
    Analysis(String),
    #[error("embedding: {0}")]
    Embed(String),
    #[error("embedding service: {0}")]
    EmbeddingService(String),
    #[error("bundle: {0}")]
    Bundle(String),
    #[error("validation failed, nothing published: {}", summary(.0))]
    Invalid(Vec<Violation>),
}


pub mod surface;

pub mod catalog_domains;

pub mod retrieval;
