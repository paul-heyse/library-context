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
    sql.push_str(&format!("DEFINE FUNCTION fn::lctx_operation_definition() {{ RETURN '{}'; }};\n",operation_definition().hex()));
    sql
}
pub mod source_evidence;
mod flow_inventory;
mod source_characterization;
mod source_usage;
pub mod behavior;
mod capture;


mod pagination;
mod candidates;
mod originals;
mod capability;
mod evidence;
mod inspection;
mod operations;
mod vocabulary;
mod operation_packet;
mod service;
pub use service::{NativeService,QueryVector};

/// Exact executable Rust operation policy is part of the native realization, separate from graph meaning.
pub fn operation_definition()->lctx_model::domain::ContentHash{
 use lctx_model::domain::{Key,KeySink,ContentHash};
 let mut sink=KeySink::new("native-operation-implementation/v1");
 for source in [include_bytes!("service.rs").as_slice(),include_bytes!("operations.rs"),include_bytes!("records.rs"),include_bytes!("scope.rs"),include_bytes!("search.rs"),include_bytes!("selection.rs"),include_bytes!("library.rs"),include_bytes!("pagination.rs"),include_bytes!("candidates.rs"),include_bytes!("core.rs"),include_bytes!("claims.rs"),include_bytes!("originals.rs"),include_bytes!("evidence.rs"),include_bytes!("capability.rs"),include_bytes!("inspection.rs"),include_bytes!("operation_packet.rs"),include_bytes!("vocabulary.rs"),include_bytes!("source_evidence.rs"),include_bytes!("flow_inventory.rs"),include_bytes!("source_characterization.rs"),include_bytes!("source_usage.rs"),include_bytes!("behavior.rs"),include_bytes!("capture.rs"),include_bytes!("../../lctx-model/src/domain/serving/ranking.rs"),include_bytes!("../../lctx-model/src/domain/selection/evaluate.rs"),include_bytes!("../../lctx-model/src/domain/selection/algebra.rs"),include_bytes!("../../lctx-model/src/domain/selection/preparation.rs"),include_bytes!("../../lctx-model/src/domain/selection/classification.rs"),include_bytes!("../../lctx-model/src/domain/selection/build.rs"),include_bytes!("../../lctx-model/src/domain/selection/structural_facets.rs"),include_bytes!("../../lctx-model/src/domain/selection/source_fields.rs"),include_bytes!("../../lctx-model/src/domain/selection/facets.rs"),include_bytes!("../../lctx-model/src/domain/selection/frames.rs"),include_bytes!("../../lctx-model/src/domain/selection/vocabulary.rs"),include_bytes!("../../lctx-model/src/domain/selection/admission.rs"),include_bytes!("../../lctx-model/src/domain/selection/specialization.rs"),include_bytes!("../../lctx-model/src/domain/catalog/access_routes.rs"),include_bytes!("../../lctx-model/src/domain/normalized/contract_comparison.rs"),include_bytes!("../../lctx-model/src/domain/normalized/binding_normalization.rs"),include_bytes!("../../lctx-model/src/domain/normalized/incoming_references.rs"),include_bytes!("../../lctx-model/src/domain/types/contextual.rs"),include_bytes!("../../lctx-model/src/domain/synthesis/terminal.rs")]{ContentHash::of(source).encode(&mut sink);}
 lctx_model::domain::native_requests::definition().encode(&mut sink);
 lctx_model::domain::serving::mappings::identity().0.encode(&mut sink);
 lctx_model::domain::serving::wire_identity().0.encode(&mut sink);
 sink.finish()
}
