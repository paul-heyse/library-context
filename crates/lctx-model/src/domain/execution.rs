//! Exact outcomes of the retained, bounded Python execution rules.
use crate::DomainCode;

/// Adding an exact runtime exception also declares its required native hierarchy evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum ExactRuntimeException {
    TypeError = 0,
}
impl ExactRuntimeException {
    pub const ALL: &[Self] = &[Self::TypeError];
    pub const fn class(self) -> (&'static str, &'static str) {
        match self { Self::TypeError => ("builtins", "TypeError") }
    }
}
