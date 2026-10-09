//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/context_constructors.rs"]
mod context_constructors;
#[path = "cases/direct_usage.rs"]
mod direct_usage;
#[path = "cases/entry_value_witnesses.rs"]
mod entry_value_witnesses;
#[path = "cases/execution_channels.rs"]
mod execution_channels;
#[path = "cases/local_semantics.rs"]
mod local_semantics;
#[path = "cases/native_callable_deprecation.rs"]
mod native_callable_deprecation;
#[path = "cases/native_callable_variants.rs"]
mod native_callable_variants;
#[path = "cases/native_overload_origins.rs"]
mod native_overload_origins;
#[path = "cases/native_usage.rs"]
mod native_usage;
#[path = "cases/typed_calls.rs"]
mod typed_calls;
