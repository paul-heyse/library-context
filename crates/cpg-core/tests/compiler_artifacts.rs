//! Coherent topic controls; common drivers compile once in this target.
// The pinned compiler needs this compile-time depth for the finite native
// cursor future graph (rust-lang/rust#159228); keep Send checking intact.
#![recursion_limit = "256"]
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
#[path = "fixtures/native.rs"]
mod native_fixture;
#[path = "fixtures/scoped_source_execution.rs"]
mod scoped_source_execution;
#[path = "fixtures/scoped_summary.rs"]
mod scoped_summary;

#[path = "cases/entry_value_publication.rs"]
mod entry_value_publication;
#[path = "cases/graph_artifact.rs"]
mod graph_artifact;
#[path = "cases/model_publication.rs"]
mod model_publication;
#[path = "cases/summary_publication.rs"]
mod summary_publication;
