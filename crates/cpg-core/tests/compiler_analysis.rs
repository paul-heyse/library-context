//! Coherent topic controls; common drivers compile once in this target.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

#[path = "cases/analysis_preparation.rs"]
mod analysis_preparation;
#[path = "cases/analytic.rs"]
mod analytic;
#[path = "cases/analytic_embedding.rs"]
mod analytic_embedding;
#[path = "cases/retrieval_preparation.rs"]
mod retrieval_preparation;
#[path = "cases/structural.rs"]
mod structural;
