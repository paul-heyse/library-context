//! Coherent topic controls; common drivers compile once in this target.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

#[path = "cases/base_execution.rs"]
mod base_execution;
#[path = "cases/behavioral_frontiers.rs"]
mod behavioral_frontiers;
#[path = "cases/local_semantics.rs"]
mod local_semantics;
