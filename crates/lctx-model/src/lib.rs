//! The declared semantic relation model (DESIGN §15, ADR-0082).
//!
//! `domain` is the typed semantic authority (ADR-0085): relations, identity, vocabulary, the named
//! policies, the transfer algebra and composition, the condition kernel, obligations and verdicts,
//! derivations, stages and generations. It performs no I/O and depends on neither `cpg-schema` nor
//! `cpg-core`; `lctx-postgres` stores its generations and `cpg-core` runs its stages.
//!
//! `decl`, `id` and `legacy` are the pre-reconstruction declaration and identity machinery that the
//! dormant `cpg-schema` still compiles against; they leave with `cpg-schema` in phases 3–5 (cutover
//! plan §4.1.1). No adapter connects them to `domain`.

pub mod decl;
pub mod id;
pub mod legacy;

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
pub use lctx_model_macros::{Domain, DomainCode, DomainSum, Assertion};
