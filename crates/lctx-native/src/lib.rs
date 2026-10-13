//! Native operations over immutable graph snapshots on a managed remote SurrealDB server.
pub mod adapter;
pub(crate) mod alias_topology;
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

pub mod binding_descriptor;
pub mod completed_logical_rows;
pub mod completed_rows;
pub mod completed_state;
pub mod control;
pub mod derivation;
pub mod installation;
pub mod installed_view;
pub mod maintenance;
pub mod recovery;

pub mod derived_search;
pub mod diagnostic;
pub mod lexical_stats;
pub mod realization;
pub mod scope;
pub mod signal;

#[doc(hidden)]
pub mod projected_arrow;
