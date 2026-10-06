//! Fixed-snapshot native querying and model-owned packet construction.
pub mod scope;
pub mod selection;
pub mod search;
pub mod library;
mod records;
mod claims;
pub mod core;
pub use lctx_surrealdb::materialization;
/// Complete executable operation policy installed and sealed by the publisher.
pub fn native_definitions()->String {
    let mut sql=materialization::native_definitions();
    sql.push_str(selection::native_definitions());
    sql.push_str(library::native_definitions());
    sql
}
pub mod source_evidence;
mod flow_inventory;
mod source_characterization;
mod source_usage;
pub mod behavior;
mod capture;

