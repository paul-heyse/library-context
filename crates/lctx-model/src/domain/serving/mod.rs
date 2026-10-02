//! Generation-bound serving declarations. Canonical relations retain semantic ownership.
pub mod identity;
pub use identity::GenerationKey;
pub mod mappings;
pub mod resources;
pub use resources::ResourceLimits;
mod schema;
pub mod values;
pub use values::*;
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    #[error("invalid request: {0}")] Invalid(String),
    #[error("resource_refused: {0}")] ResourceRefused(String),
    #[error("unknown tool: {0}")] UnknownTool(String),
    #[error("continuation mismatch: {0}")] Continuation(String),
}
impl From<serde_json::Error> for WireError {fn from(e:serde_json::Error)->Self{Self::Invalid(e.to_string())}}
pub mod requests;
pub use requests::*;
pub mod packets;
pub use packets::*;
pub mod cursor;
pub use cursor::*;
pub mod responses;
pub use responses::*;
pub mod dispatch;
pub use dispatch::*;
pub mod ranking;
