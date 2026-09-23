//! Authoritative Arrow table contracts for the code property graph (DESIGN §B2, §3).
//!
//! Dependencies are `arrow-*` and `blake3` only. Column-level contracts live here and are
//! snapshot-tested (`tests/`); DESIGN.md does not repeat them.

pub mod codebook;
pub mod column;
pub mod derived;
pub mod hash;
pub mod id;
pub mod rules;
pub mod table;
pub mod tables;

pub use codebook::Codebook;
pub use id::{Digest, Id, IdHasher};
pub use table::Table;
