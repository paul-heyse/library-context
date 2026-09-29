//! The declared semantic relation model (DESIGN §15, ADR-0082).
//!
//! This crate owns the pure contracts of the model: relation declarations and the one registry,
//! identity, vocabulary, the named semantic policies, the transfer algebra, the condition kernel,
//! obligations and verdicts, the derivation-source registry, the stage table and generated DDL.
//! It performs no I/O and depends on neither `cpg-schema` nor `cpg-core`; `lctx-postgres` applies
//! its DDL and `cpg-core` runs its stages.
//!
//! During the cutover (cutover plan §3.1) the shared primitives the legacy contracts also use live
//! here, so legacy and new code share one `Id` type; `legacy` holds the temporary migration
//! machinery and is deleted in phase 5.

pub mod ddl;
pub mod decl;
pub mod id;
pub mod legacy;
pub mod relations;

pub use decl::codebook::Codebook;
pub use id::{Digest, Id, IdHasher};

/// Crates the declaration macros expand to, so a declaring crate needs no direct dependency.
#[doc(hidden)]
pub mod __private {
    pub use arrow_array;
    pub use arrow_schema;
}
