//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/native_class_traits.rs"]
mod native_class_traits;
#[path = "cases/native_generics.rs"]
mod native_generics;
#[path = "cases/native_model_context.rs"]
mod native_model_context;
#[path = "cases/typed_flow.rs"]
mod typed_flow;
#[path = "cases/typed_protocols.rs"]
mod typed_protocols;
#[path = "cases/typed_types.rs"]
mod typed_types;
