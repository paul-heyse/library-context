//! Relation declarations and their column vocabulary (DESIGN §15.2).
//!
//! `column` maps Rust value types to Arrow columns, `hash` feeds them to identity recipes and
//! `codebook` declares closed categories. They are shared by the legacy `table!` contracts until
//! phase 5.

pub mod codebook;
pub mod column;
pub mod hash;
