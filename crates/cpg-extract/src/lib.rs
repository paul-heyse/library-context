//! Typed fact producers over pinned Pyrefly, Ruff, ty and captured evidence.

pub mod acquisition;
pub mod assembly;
pub mod bundle;
pub mod call_records;
pub mod capture;
pub mod capture_records;
pub mod deployment;
pub mod deployment_parser;
pub mod diagnostic_records;
mod docstrings;
pub mod document_parser;
mod lexical;
pub mod lexical_records;
pub mod library;
pub mod logging;
mod native_branches;
pub mod native_context;
pub mod natives;
pub mod parameter_definition_records;
pub mod protocol_records;
pub mod public_records;
pub mod pyrefly_stage;
pub mod ruff_context;
pub mod ruff_lexical;
mod runtime_scripts;
pub mod symbol_records;
pub mod syntax_records;
pub mod ty_flow;
pub mod type_records;
pub mod typed_syntax;

use std::path::PathBuf;
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("path must be absolute: {}",.0.display())]
    RelativePath(PathBuf),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("library: {0}")]
    Library(String),
}
