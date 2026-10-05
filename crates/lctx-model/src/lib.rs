//! Store-free typed semantic authority for compiler graph artifacts.
//!
//! `domain` owns semantic entities, role-labelled attributed assertions, identity recipes,
//! named policies, the transfer algebra, condition kernel, obligations and derivations.
//! Completed relation streams are compiler inputs; finite semantic graph mappings and admission
//! requirements define the published contract. Static stage declarations describe producers and
//! semantic predecessor selection. `cpg-core` executes producers and emits immutable artifacts;
//! database publication and physical realization are separate owners.
//!
extern crate self as lctx_model;
pub mod domain;
pub use lctx_model_macros::{Assertion, Domain, DomainCode, DomainSum};
