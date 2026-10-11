//! Shared selected normalization kernels and necessary owner admission.
pub mod admission;
pub mod projection_admission;
pub mod callable_scope;
pub mod receiver_scope;
pub mod call_scope;
pub use admission::{validate_bindings, validate_events, validate_receivers, prepare_bindings, prepare_events, prepare_receivers};
pub use projection_admission::validate_projections;
pub use callable_scope::validate_callables;
