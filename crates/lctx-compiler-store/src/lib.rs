//! Validated compiler admission and immutable native completed inputs.
pub use lctx_native::{
    AuthenticationScope, Loader, NativeReader, ReuseConfig, RuntimeConfig, reader, surrealdb,
};
pub use lctx_native::{
    acknowledged_candidates, adapter, batches, codec, config, control, loader, ordered_rows, phase,
    prepared, projections, realization, recovery, schema, selection,
};
pub mod compiler;
pub mod compiler_provider;
pub mod product_cache;
mod product_codec;
pub use lctx_native::projected_arrow;
pub use product_cache::{LeasedProduct, NativeProductCache};
pub mod admission;
pub mod artifact;
mod artifact_manifest;
pub mod consumed_rows;
pub mod facts;
pub mod native_bridge;
pub mod native_calls;
mod native_canonical;
pub mod normalize;
pub mod scope_compilation;
pub mod scoped_admission;
pub mod scoped_aspects;
pub mod scoped_batch;
pub mod scoped_execution;
pub mod scoped_inventory;
mod scoped_positions;
pub mod scoped_retrieval;
pub mod sql;
pub mod stage_runtime;
#[cfg(test)]
mod test_native;

pub mod original_demands;
