//! Coherent topic controls; common drivers compile once in this target.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

#[path = "cases/synthesis_documentary.rs"]
mod synthesis_documentary;
#[path = "cases/synthesis_refutation.rs"]
mod synthesis_refutation;
