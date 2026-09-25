//! Authoritative Arrow table contracts for the code property graph (DESIGN §B2, §3).
//!
//! Dependencies are `arrow-*` and `blake3` only. Column-level contracts live here and are
//! snapshot-tested (`tests/`); DESIGN.md does not repeat them.

pub mod behavior;
pub mod bundle;
pub mod codebook;
pub mod column;
pub mod communities;
pub mod concepts;
pub mod condition;
pub mod condition_kernel;
pub mod derived;
pub mod embedding;
pub mod findings;
pub mod flows;
pub mod graph;
pub mod hash;
pub mod id;
pub mod mdx;
pub mod metrics;
pub mod models;
pub mod neighbours;
pub mod primitive_theory;
pub mod projection;
pub mod public;
pub mod query;
pub mod rules;
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
