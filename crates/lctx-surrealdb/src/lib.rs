//! Native operations over immutable graph snapshots on a managed remote SurrealDB server.
pub mod codec;
pub mod cache;
pub use cache::NativeEmbeddingCache;
pub mod reader;
pub mod schema;
pub use reader::{Credentials, NativeReader, RecordSelection};
pub use surrealdb;
