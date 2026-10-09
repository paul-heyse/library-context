//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/normalized_bindings.rs"]
mod normalized_bindings;
#[path = "cases/normalized_callables.rs"]
mod normalized_callables;
#[path = "cases/normalized_entities.rs"]
mod normalized_entities;
#[path = "cases/normalized_events.rs"]
mod normalized_events;
#[path = "cases/normalized_imports.rs"]
mod normalized_imports;
#[path = "cases/normalized_projections.rs"]
mod normalized_projections;
#[path = "cases/normalized_recovery.rs"]
mod normalized_recovery;
#[path = "cases/normalized_relations.rs"]
mod normalized_relations;
