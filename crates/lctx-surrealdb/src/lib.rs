//! Native operations over immutable graph snapshots on a managed remote SurrealDB server.
pub mod cache;
pub mod codec;
pub use cache::NativeEmbeddingCache;
pub mod config;
pub mod reader;
pub mod prepared;
pub use config::RuntimeConfig;
pub mod batches;
pub mod loader;
pub mod materialization;
pub mod ordered_rows;
pub mod acknowledged_candidates;
pub mod reconciliation;
pub use loader::Loader;
pub mod schema;
pub use reader::{Credentials, NativeReader, RecordSelection};
pub use surrealdb;
pub mod projections;

pub mod compiler;
pub mod compiler_provider;
mod projected_arrow;
