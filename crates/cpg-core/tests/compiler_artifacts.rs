//! Coherent topic controls; common drivers compile once in this target.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;

#[path = "cases/entry_value_publication.rs"]
mod entry_value_publication;
#[path = "cases/graph_artifact.rs"]
mod graph_artifact;
#[path = "cases/model_publication.rs"]
mod model_publication;
#[path = "cases/summary_publication.rs"]
mod summary_publication;
