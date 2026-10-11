//! Native operations over immutable graph snapshots on a managed remote SurrealDB server.
pub mod adapter;
pub mod cache;
pub mod codec;
pub use cache::NativeEmbeddingCache;
pub mod config;
pub mod prepared;
pub mod reader;
pub mod selection;
pub use config::{AuthenticationScope, ReuseConfig, RuntimeConfig};
pub mod acknowledged_candidates;
pub mod batches;
pub mod loader;
pub mod materialization;
pub mod ordered_rows;
pub mod originals;
pub mod reconciliation;
pub use loader::Loader;
pub mod schema;
pub use reader::{Credentials, NativeReader, RecordSelection};
pub use surrealdb;
pub mod phase;
pub mod projections;

pub mod installation;
pub mod recovery;
pub mod control;
pub mod installed_view;
pub mod completed_state;
pub mod completed_rows;
pub mod completed_logical_rows;
pub mod binding_descriptor;
pub mod derivation;
pub mod upgrade;

pub mod derived_search;
pub mod lexical_stats;
pub mod realization;
pub mod scope;
pub mod signal;

#[doc(hidden)]
pub mod projected_arrow;
