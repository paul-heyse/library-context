//! Validated compiler admission and immutable native completed inputs.
pub use lctx_native::{RuntimeConfig, ReuseConfig, AuthenticationScope, Loader, NativeReader, reader, surrealdb};
pub use lctx_native::{adapter, codec, schema, acknowledged_candidates, ordered_rows, prepared,
    selection, loader, phase, projections, control, batches, config, realization, recovery};
pub mod compiler;
pub mod compiler_provider;
pub mod product_cache;
mod product_codec;
pub use product_cache::{LeasedProduct, NativeProductCache};
pub use lctx_native::projected_arrow;
pub mod admission;
pub mod artifact;
pub mod consumed_rows;
pub mod normalize;
pub mod scope_compilation;
pub mod scoped_admission;
pub mod scoped_inventory;
pub mod scoped_retrieval;
pub mod scoped_execution;
pub mod scoped_aspects;
pub mod scoped_batch;
pub mod sql;
pub mod stage_runtime;
pub mod native_calls;
pub mod native_bridge;
pub mod facts;
mod artifact_manifest;
mod native_canonical;
#[cfg(test)]
mod test_native;

pub mod original_demands;
