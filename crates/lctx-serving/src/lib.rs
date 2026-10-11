//! Fixed-snapshot native querying and model-owned packet construction.
mod claims;
pub mod core;
pub mod library;
mod records;
pub mod scope;
pub mod search;
pub mod selection;
pub use lctx_native::materialization;
/// Complete executable operation policy installed and sealed by the publisher.
pub fn native_definitions() -> String {
    let mut sql = materialization::native_definitions();
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
pub use service::{NativeService, QueryVector, TransportRequest};

/// Exact executable Rust operation policy is part of the native realization, separate from graph meaning.
pub fn operation_definition() -> lctx_model::domain::ContentHash {
    executable_constants().operation
}

/// Only compiled, immutable inputs live for the executable epoch. Installed native definitions,
/// selected snapshots, channel availability and request/vector bytes retain their own validation.
struct ExecutableConstants {
    operation: lctx_model::domain::ContentHash,
    wire: lctx_model::domain::serving::identity::WireIdentity,
    policy: Result<lctx_model::domain::serving::identity::PolicyIdentity, String>,
    default_ordering: lctx_model::domain::ContentHash,
    expanded_ordering: lctx_model::domain::ContentHash,
}
fn executable_constants() -> &'static ExecutableConstants {
    static CONSTANTS: std::sync::OnceLock<ExecutableConstants> = std::sync::OnceLock::new();
    CONSTANTS.get_or_init(|| {
        use lctx_model::domain::{ContentHash, serving};
        let wire = serving::wire_identity();
        let operation = operation_identity(
            ContentHash::of(include_bytes!(concat!(env!("OUT_DIR"), "/serving-implementation.bin"))),
            lctx_model::domain::implementation_digest(),
            lctx_model::domain::native_requests::definition(),
            serving::mappings::identity().0,
            wire.0,
        );
        ExecutableConstants {
            operation,
            wire,
            policy: serving::ranking::RankingPolicy::default().identity().map_err(|error| error.to_string()),
            default_ordering: ContentHash::of(b"serving-key-order/v2:default"),
            expanded_ordering: ContentHash::of(b"serving-key-order/v2:expanded"),
        }
    })
}
fn operation_identity(
    implementation: lctx_model::domain::ContentHash,
    model: lctx_model::domain::ContentHash,
    native_requests: lctx_model::domain::ContentHash,
    mapping: lctx_model::domain::ContentHash,
    wire: lctx_model::domain::ContentHash,
) -> lctx_model::domain::ContentHash {
    use lctx_model::domain::{Key, KeySink};
    let mut sink = KeySink::new("native-operation-implementation/v2");
    for input in [implementation, model, native_requests, mapping, wire] {
        input.encode(&mut sink);
    }
    sink.finish()
}

#[cfg(test)]
mod executable_constant_controls {
    use super::*;
    use lctx_model::domain::{ContentHash, serving};
    #[test]
    fn repeated_preparation_reuses_constants_and_preserves_semantic_inputs() {
        let prepared = executable_constants();
        assert!(std::ptr::eq(prepared, executable_constants()));
        assert_eq!(prepared.wire, serving::wire_identity());
        assert_eq!(prepared.policy.as_ref().unwrap(), &serving::ranking::RankingPolicy::default().identity().unwrap());
        let inputs = [
            ContentHash::of(include_bytes!(concat!(env!("OUT_DIR"), "/serving-implementation.bin"))),
            lctx_model::domain::implementation_digest(),
            lctx_model::domain::native_requests::definition(),
            serving::mappings::identity().0,
            serving::wire_identity().0,
        ];
        let [implementation, model, native_requests, mapping, wire] = inputs;
        assert_eq!(operation_definition(), operation_identity(implementation, model, native_requests, mapping, wire));
        for index in 0..inputs.len() {
            let mut changed = inputs;
            changed[index] = ContentHash::of(b"changed executable definition");
            let [implementation, model, native_requests, mapping, wire] = changed;
            assert_ne!(operation_definition(), operation_identity(implementation, model, native_requests, mapping, wire), "semantic input {index} remains bound");
        }
    }
}

#[cfg(test)]
#[path = "../tests/fixtures/scoped.rs"]
pub(crate) mod scoped_fixture;
