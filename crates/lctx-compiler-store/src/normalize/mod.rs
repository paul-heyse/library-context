//! Shared selected normalization kernels and necessary owner admission.
pub mod admission;
pub mod call_scope;
pub mod callable_scope;
pub mod projection_admission;
pub mod receiver_scope;
pub use admission::{
    prepare_bindings, prepare_events, prepare_receivers, validate_bindings, validate_events,
    validate_receivers,
};
pub use callable_scope::validate_callables;
pub use projection_admission::validate_projections;
