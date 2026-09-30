//! Normalized semantic identities and total attributed relationships (ADR-0102).
mod inventory;
mod relation_inventory;
mod callable_inventory;
mod event_inventory;
mod binding_inventory;
pub mod bindings;
pub mod binding_normalization;
pub mod events;
pub mod event_normalization;
pub mod signature_applicability;
pub mod callables;
pub mod callable_normalization;
pub mod links;
pub mod relation_normalization;
pub mod entities;
pub mod entity_normalization;
mod rows;
pub use rows::Rows;
use super::*;

/// Revision of semantic normalization, included in every assessment key and stage declaration.
pub fn policy_revision() -> ContentHash { ContentHash::of(b"lctx-normalization/phase3/v1") }
pub fn relations() -> Vec<Relation> { let mut relations = entities::relations(); relations.extend(links::relations()); relations.extend(callables::relations()); relations.extend(events::relations()); relations.extend(bindings::relations()); relations }
