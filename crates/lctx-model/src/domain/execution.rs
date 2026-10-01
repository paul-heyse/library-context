//! Exact outcomes of the retained, bounded Python execution rules.
use crate::DomainCode;
pub mod outcome;
pub mod evaluation;
pub mod completion;
pub mod records;
pub mod completion_records;

pub fn relations()->Vec<crate::domain::Relation> {let mut relations=records::relations();relations.extend(completion_records::relations());relations.extend(production::relations());relations.extend(completion_production::relations());relations.extend(body_records::relations());relations.extend(source_call_records::relations());relations}

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




pub mod body;
pub mod body_records;
pub mod source_call;
pub mod source_call_records;
pub mod source_invocation;
pub mod configuration;
pub mod production;
pub mod completion_production;
