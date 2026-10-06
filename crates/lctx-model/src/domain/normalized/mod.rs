//! Normalized semantic identities and total attributed relationships (ADR-0103).
pub(crate) mod binding_inventory;
pub mod binding_normalization;
pub mod bindings;
pub mod callable_aspects;
pub(crate) mod callable_inventory;
pub mod callable_normalization;
pub mod callables;
pub mod coverage;
pub mod decorator_identity;
pub mod dispatch;
pub mod entities;
pub mod entity_normalization;
pub(crate) mod event_inventory;
pub mod event_normalization;
pub mod events;
pub mod generic_specialization;
pub(crate) mod inventory;
pub mod links;
pub mod overload_association;
pub mod parameter_correspondence;
pub mod receiver;
pub(crate) mod receiver_inventory;
pub(crate) mod relation_inventory;
pub mod relation_normalization;
pub(crate) mod rows;
pub mod signature_applicability;
pub mod symbolic_fields;
use super::*;
pub use rows::Rows;

/// Normalized owners reconstruct the captured facts universe. Later vocabulary cannot enlarge it.
pub(crate) fn facts_inputs(inputs: Vec<ValidationInput>) -> Vec<ValidationInput> {
    let mut inputs = inputs
        .into_iter()
        .map(|input| {
            if stages::is_vocabulary(input.name()) {
                input.at_epoch(stages::PublicationBoundary::Facts)
            } else {
                input
            }
        })
        .collect::<Vec<_>>();
    inputs.sort_by_key(ValidationInput::name);
    inputs.dedup_by_key(|input| input.name());
    inputs
}

/// Revision of semantic normalization, included in every assessment key and stage declaration.
pub fn policy_revision() -> ContentHash {
    // Revise this declared policy when normalization meaning changes. Rust source and dependency
    // changes belong to producer implementation provenance, not every normalized logical key.
    ContentHash::of(b"lctx-normalization/graph-native/v1")
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

/// Every facts-derived normalization declaration pins vocabulary after its entire input assembly.
pub fn facts_stage_inputs(inputs: Vec<stages::RelationUse>) -> Vec<stages::RelationUse> {
    let mut inputs = inputs
        .into_iter()
        .map(|input| {
            if stages::is_vocabulary(input.name()) {
                input.at_epoch(stages::PublicationBoundary::Facts)
            } else {
                input
            }
        })
        .collect::<Vec<_>>();
    inputs.sort_by_key(|input| input.name());
    inputs.dedup_by_key(|input| input.name());
    inputs
}

pub(crate) mod native_lexical;

pub mod contract_comparison;

pub mod incoming_references;
