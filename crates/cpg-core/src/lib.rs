//! Persisted semantic graph compilation and exact completed-view admission.

pub mod analysis_bindings;
pub mod analysis_graphs;
pub mod analysis_prepare;
pub mod analytic;
pub mod analytic_embedding;
pub mod analytic_text;
pub mod catalog_core;
mod catalog_core_scope;
pub mod catalog_evidence;
mod catalog_evidence_scope;
pub mod catalog_selection;
pub mod consumed_rows;
pub mod embedding_realization;
pub mod embedding_service;
pub mod facts;
pub mod final_coverage;
mod producer_operations;
pub mod retrieval_preparation;
mod scope_compilation;
mod scoped_admission;
mod scoped_aspects;
mod scoped_batch;
pub mod sql;
pub mod stage_runtime;
pub mod structural;
pub mod synthesis;
pub mod synthesis_preparation;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
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
    #[error("embedding service: {0}")]
    EmbeddingService(String),
}

pub mod retrieval;

pub mod compilation;
pub mod normalize;

pub mod local_semantics;

pub mod semantic_execution;

pub mod semantic_models;

pub mod semantic_summaries;

/// Private native attempt workspace and immutable completed memberships.
pub mod workspace;

mod documentary_spool;
mod native_bridge;
mod native_calls;
mod original_demands;

pub mod artifact;

mod analytical_scopes;

#[cfg(test)]
#[path = "../tests/fixtures/native.rs"]
mod test_native;

mod catalog_products;

mod retrieval_products;
