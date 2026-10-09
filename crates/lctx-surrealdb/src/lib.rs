//! Native operations over immutable graph snapshots on a managed remote SurrealDB server.
mod adapter;
pub mod cache;
pub mod product_cache;
pub use product_cache::{NativeProductCache, LeasedProduct};
pub mod codec;
pub use cache::NativeEmbeddingCache;
pub mod config;
pub mod prepared;
pub mod reader;
pub use config::{RuntimeConfig, ReuseConfig};
pub mod acknowledged_candidates;
pub mod batches;
pub mod loader;
pub mod materialization;
pub mod ordered_rows;
pub mod reconciliation;
pub use loader::Loader;
pub mod schema;
pub use reader::{Credentials, NativeReader, RecordSelection};
pub use surrealdb;
pub mod phase;
pub mod projections;

pub mod compiler;
pub mod compiler_provider;
mod projected_arrow;

pub mod derived_search;
pub mod realization;
pub mod scope;
