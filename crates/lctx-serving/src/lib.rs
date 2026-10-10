//! Fixed-snapshot native querying and model-owned packet construction.
mod claims;
pub mod core;
pub mod library;
mod records;
pub mod scope;
pub mod search;
pub mod selection;
pub use lctx_surrealdb::materialization;
/// Complete executable operation policy installed and sealed by the publisher.
pub fn native_definitions() -> String {
    let mut sql = materialization::native_definitions();
    sql.push_str(library::native_definitions());
    sql.push_str(&format!(
        "DEFINE FUNCTION fn::lctx_operation_definition() {{ RETURN '{}'; }};\n",
        operation_definition().hex()
    ));
    sql
}
pub mod behavior;
mod capture;
mod flow_inventory;
mod source_characterization;
pub mod source_evidence;
mod source_usage;

mod candidates;
mod capability;
pub(crate) mod defaults;
pub(crate) mod delivery;
mod evidence;
mod inspection;
mod operation_packet;
mod operations;
mod originals;
mod pagination;
mod preparation;
mod ranked_results;
mod service;
mod vocabulary;
pub use service::{NativeService, QueryVector};

/// Exact executable Rust operation policy is part of the native realization, separate from graph meaning.
pub fn operation_definition() -> lctx_model::domain::ContentHash {
    use lctx_model::domain::{ContentHash, Key, KeySink};
    let mut sink = KeySink::new("native-operation-implementation/v2");
    ContentHash::of(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/serving-implementation.bin"
    )))
    .encode(&mut sink);
    lctx_model::domain::implementation_digest().encode(&mut sink);
    lctx_model::domain::native_requests::definition().encode(&mut sink);
    lctx_model::domain::serving::mappings::identity()
        .0
        .encode(&mut sink);
    lctx_model::domain::serving::wire_identity()
        .0
        .encode(&mut sink);
    sink.finish()
}

#[cfg(test)]
#[path = "../tests/fixtures/scoped.rs"]
pub(crate) mod scoped_fixture;
