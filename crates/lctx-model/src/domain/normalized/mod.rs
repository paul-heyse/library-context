//! Normalized semantic identities and total attributed relationships (ADR-0103).
mod binding_inventory;
pub mod binding_normalization;
pub mod bindings;
mod callable_inventory;
pub mod callable_normalization;
pub mod callables;
pub mod callable_aspects;
pub mod coverage;
pub mod dispatch;
pub mod entities;
pub mod entity_normalization;
mod event_inventory;
pub mod event_normalization;
pub mod events;
mod inventory;
pub mod links;
pub mod receiver;
mod receiver_inventory;
mod relation_inventory;
pub mod relation_normalization;
mod rows;
pub mod signature_applicability;
use super::*;
pub use rows::Rows;

/// Normalized owners reconstruct the captured facts universe. Later vocabulary cannot enlarge it.
pub(crate) fn facts_inputs(inputs:Vec<ValidationInput>)->Vec<ValidationInput> {
    let mut inputs=inputs.into_iter().map(|input|if stages::is_vocabulary(input.name()) {input.at_epoch(stages::PublicationBoundary::Facts)}else {input}).collect::<Vec<_>>();
    inputs.sort_by_key(|input|(input.name(),input.prefix()));inputs.dedup_by_key(|input|(input.name(),input.prefix()));inputs
}

/// Revision of semantic normalization, included in every assessment key and stage declaration.
pub fn policy_revision() -> ContentHash {
    ContentHash::of(b"lctx-normalization/phase4/dispatch-class-of/v3")
}
pub fn relations() -> Vec<Relation> {
    let mut relations = entities::relations();
    relations.extend(links::relations());
    relations.extend(callables::relations());
    relations.extend(callable_aspects::relations());
    relations.extend(events::relations());
    relations.extend(receiver::relations());
    relations.extend(bindings::relations());
    relations.extend(super::projection::relations());
    relations.extend(coverage::relations());
    relations
}
