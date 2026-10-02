//! The declared semantic relation model (DESIGN §15, ADR-0082).
//!
//! `domain` is the typed semantic authority (ADR-0085): relations, identity, vocabulary, the named
//! policies, the transfer algebra and composition, the condition kernel, obligations and verdicts,
//! derivations, stages and generations. It performs no I/O and depends on neither `cpg-schema` nor
//! `cpg-core`; `lctx-postgres` stores its generations and `cpg-core` runs its stages.
//!
extern crate self as lctx_model;
pub mod domain;
pub use lctx_model_macros::{Assertion, Domain, DomainCode, DomainSum};
