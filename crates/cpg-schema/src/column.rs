//! Rust value types → Arrow columns (DESIGN §3.3 physical profiles). The mapping lives in
//! `lctx-model` so legacy contracts and new relation declarations share it.

pub use lctx_model::decl::column::{ArrowColumn, Blob, CODEBOOK_KEY};
