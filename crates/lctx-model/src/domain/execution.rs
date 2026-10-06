//! Exact outcomes of the retained, bounded Python execution rules.
use crate::DomainCode;
pub mod completion;
pub mod completion_records;
pub mod evaluation;
pub mod outcome;
pub mod records;
pub mod fidelity;

pub fn relations() -> Vec<crate::domain::Relation> {
    let mut relations = records::relations();
    relations.extend(completion_records::relations());
    relations.extend(production::relations());
    relations.extend(completion_production::relations());
    relations.extend(body_records::relations());
    relations.extend(source_call_records::relations());
    relations.extend(enriched_records::relations());
    relations.extend(enriched_production::relations());
    relations.extend(modeled_call::relations());
    relations.extend(model_production::relations());
    relations.extend(definition::relations());
    relations.extend(context_execution::relations());
    relations.extend(context_binding::relations());
    relations.extend(summary_path::relations());
    relations.extend(summary_proof::relations());
    relations.extend(summary_replay::relations());
    relations.extend(summary_consequences::relations());
    relations
}

/// Adding an exact runtime exception also declares its required native hierarchy evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
#[model(inventory = "slice")]
pub enum ExactRuntimeException {
    #[model(wire = "TypeError")]
    TypeError = 0,
    #[model(wire = "ValueError")]
    ValueError = 1,
    #[model(wire = "RuntimeError")]
    RuntimeError = 2,
    #[model(wire = "Exception")]
    Exception = 3,
}
impl ExactRuntimeException {
    pub const fn class(self) -> (&'static str, &'static str) {
        match self {
            Self::TypeError => ("builtins", "TypeError"),
            Self::ValueError => ("builtins", "ValueError"),
            Self::RuntimeError => ("builtins", "RuntimeError"),
            Self::Exception => ("builtins", "Exception"),
        }
    }
}

pub mod body;
pub mod body_records;
pub mod capture_bridge;
pub mod completion_production;
pub mod configuration;
pub mod enriched;
pub mod enriched_production;
pub mod enriched_records;
pub mod production;
pub mod read_channels;
pub mod read_dynamic;
pub mod source_call;
pub mod source_call_records;
pub mod source_invocation;
pub mod summary_capture;

pub mod model_application;
pub mod model_context;
pub mod model_production;
pub mod model_rules;

pub mod builtin_read;
pub mod model_protocol;
pub mod modeled_call;

pub mod model_construction;

pub mod summary_worklist;

pub mod summary_path;
pub mod summary_proof;

pub mod summary_production;
pub mod summary_replay;
pub mod summary_schedule;

pub mod model_transfer;

pub mod definition;

pub mod summary_control;

pub mod summary_alias;

pub mod context_execution;

pub mod context_binding;

pub mod model_context_transfer;

pub mod summary_consequences;
pub mod summary_symbolic;

pub mod read_fields;

pub mod summary_exceptions;

pub mod closed_targets;
pub mod protocol_interpretation;
pub mod summary_terminal;

/// Dispatch a native collector only from its declared immutable semantic predecessor.
/// This selector is compiler metadata, not a persisted runtime epoch or read grant.
pub(crate) fn require_facts_view(
    input: &crate::domain::ValidationInput,
) -> Result<(), crate::domain::ModelError> {
    if crate::domain::stages::is_vocabulary(input.name())
        && input.prefix() != Some(crate::domain::stages::PublicationBoundary::Facts)
    {
        return Err(crate::domain::ModelError::Invalid(format!(
            "native input {} requires the Facts view",
            input.name()
        )));
    }
    Ok(())
}
