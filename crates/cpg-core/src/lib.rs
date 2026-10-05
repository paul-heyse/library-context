//! Store-free semantic graph compilation; publication and serving are separate consumers.

pub mod analysis_graphs;
pub mod analysis_prepare;
pub mod analytic_embedding;
pub mod analytic_text;
// src/bundle.rs remains uncompiled Phase 5 recovery source (P4 plan §11).
pub mod analytic;
pub mod catalog_core;
pub mod catalog_evidence;
pub mod catalog_selection;
pub mod consumed_rows;
pub mod embedding_realization;
pub mod embedding_service;
pub mod facts;
pub mod final_coverage;
pub mod retrieval_preparation;
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
    #[error("embedding: {0}")]
    Embed(String),
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

/// Store-free attempt workspace and immutable completed inputs.
pub mod workspace;
