//! Normalized semantic identities and total attributed relationships (ADR-0103).
mod binding_inventory;
pub mod binding_normalization;
pub mod bindings;
mod callable_inventory;
pub mod callable_normalization;
pub mod callables;
pub mod coverage;
pub mod entities;
pub mod entity_normalization;
mod event_inventory;
pub mod event_normalization;
pub mod events;
mod inventory;
pub mod links;
mod relation_inventory;
pub mod relation_normalization;
mod receiver_inventory;
pub mod receiver;
mod rows;
pub mod signature_applicability;
use super::*;
pub use rows::Rows;

/// Revision of semantic normalization, included in every assessment key and stage declaration.
pub fn policy_revision() -> ContentHash {
    ContentHash::of(b"lctx-normalization/phase4/class-of/v2")
}
pub fn relations() -> Vec<Relation> {
    let mut relations = entities::relations();
    relations.extend(links::relations());
    relations.extend(callables::relations());
    relations.extend(events::relations());
    relations.extend(receiver::relations());
    relations.extend(bindings::relations());
    relations.extend(super::projection::relations());
    relations.extend(coverage::relations());
    relations
}
