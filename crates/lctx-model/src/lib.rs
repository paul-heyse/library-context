//! The declared semantic relation model (DESIGN §15, ADR-0082).
//!
//! This crate owns the pure contracts of the model: relation declarations and the one registry,
//! identity, vocabulary, the named semantic policies, the transfer algebra, the condition kernel,
//! obligations and verdicts, the derivation-source registry, the stage table and generated DDL.
//! It performs no I/O and depends on neither `cpg-schema` nor `cpg-core`; `lctx-postgres` applies
//! its DDL and `cpg-core` runs its stages.
//!
//! `domain` is the reconstructed typed authority under ADR-0085. Its production subset has focused
//! Arrow/PostgreSQL evidence. The pre-reconstruction modules below still serve the old pipeline;
//! their removal and semantic replacement remain open in cutover plan §4.2. No adapter connects the
//! two execution paths, and the existence of this foundation does not establish phase completion.

pub mod calls;
pub mod condition;
pub mod ddl;
pub mod decl;
pub mod derivation;
pub mod id;
pub mod legacy;
pub mod obligation;
pub mod projection;
pub mod relations;
pub mod stage;
pub mod transfer;
pub mod vocab;

pub use decl::codebook::Codebook;
pub use id::{Digest, Id, IdHasher};

/// Crates the declaration macros expand to, so a declaring crate needs no direct dependency.
#[doc(hidden)]
pub mod __private {
    pub use arrow_array;
    pub use arrow_schema;
}

extern crate self as lctx_model;
pub mod domain;
pub use lctx_model_macros::{Domain, DomainCode, DomainSum};
