//! Coherent topic controls; common drivers compile once in this target.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

#[path = "cases/catalog_core.rs"]
mod catalog_core;
#[path = "cases/catalog_evidence.rs"]
mod catalog_evidence;
#[path = "cases/catalog_selection.rs"]
mod catalog_selection;
#[path = "cases/normalized_generation.rs"]
mod normalized_generation;
#[path = "cases/normalized_relations.rs"]
mod normalized_relations;
