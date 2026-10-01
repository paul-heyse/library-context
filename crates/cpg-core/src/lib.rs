//! Typed facts orchestration and generation-bound DataFusion reads. Downstream analysis,
//! catalog and serving modules remain dormant until cutover phases 4–5 (DESIGN §15).

pub mod analysis_graphs;
pub mod analysis_prepare;
pub mod analytic_text;
pub mod analytic_embedding;
pub mod analyze;
pub mod arrow_types;
pub mod behavior;
pub mod bundle;
pub mod catalog;
pub mod catalog_core;
pub mod catalog_evidence;
pub mod catalog_selection;
pub mod structural;
pub mod embed;
pub mod embedding_realization;
pub mod embedding_service;
pub mod entry_links;
pub mod evidence;
pub mod facts;
pub mod flow_model;
pub mod generation_read;
pub mod model_runtime;
pub mod postgres;
pub mod producer;
pub mod session;
pub mod sql;
pub mod stage_runtime;
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

pub mod normalize;



pub mod local_semantics;
