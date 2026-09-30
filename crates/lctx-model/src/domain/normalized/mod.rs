//! Normalized semantic identities and total attributed relationships (ADR-0102).
mod inventory;
pub mod entities;
pub mod entity_normalization;
mod rows;
pub use rows::Rows;
use super::*;

/// Revision of semantic normalization, included in every assessment key and stage declaration.
pub fn policy_revision() -> ContentHash { ContentHash::of(b"lctx-normalization/phase3/v1") }
pub fn relations() -> Vec<Relation> { entities::relations() }
