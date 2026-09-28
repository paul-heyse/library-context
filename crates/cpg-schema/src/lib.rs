//! Authoritative Arrow table contracts for the code property graph (DESIGN §B2, §3).
//!
//! Dependencies are `arrow-*` and `blake3` only. Column-level contracts live here and are
//! snapshot-tested (`tests/`); DESIGN.md does not repeat them.

pub mod behavior;
pub mod bundle;
pub mod call_execution;
pub mod catalog;
pub mod codebook;
pub mod column;
pub mod communities;
pub mod concept_attributes;
pub mod concepts;
pub mod condition;
pub mod condition_kernel;
pub mod context_observations;
pub mod context_protocol;
pub mod context_value;
pub mod derived;
pub mod embedding;
pub mod embedding_spec;
pub mod evidence;
pub mod findings;
pub mod flows;
pub mod frame_exit;
pub mod graph;
pub mod hash;
pub mod id;
pub mod mdx;
pub mod metrics;
pub mod modeled_identity;
pub mod models;
pub mod neighbours;
pub mod parameter_identity;
pub mod primitive_theory;
pub mod projection;
pub mod public;
pub mod query;
pub mod rules;
pub mod summary_contract;
pub mod table;
pub mod tables;

// For `query_row!` in other crates.
#[doc(hidden)]
pub use arrow_array;
#[doc(hidden)]
pub use arrow_schema;
pub use codebook::Codebook;
pub use id::{Digest, Id, IdHasher};
pub use table::Table;

pub mod completion_proof;

pub mod action;

pub mod source_body;
pub mod source_call;

pub mod serving_projection;
pub mod serving_support;

pub mod postgres_report;

pub mod wire;
