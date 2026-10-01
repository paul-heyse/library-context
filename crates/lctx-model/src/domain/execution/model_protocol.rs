//! Model-owned resource and lifecycle conclusions cite actual ordered source execution.
//! Authored declarations select semantics; only independently replayed source records admit
//! entry and exit phases. Per-item exit transitions preserve suppression and reverse order.
use super::{
    context_execution::{ContextExecution, ContextItem},
    model_application::ModelApplicationData,
    model_construction::CheckedContextConstruction,
    model_context::{CheckedContextProtocol, ContextValue},
    model_rules::ActionPhase,
};
use crate::domain::{
    analysis::{self, model as publication, policy::EvidenceStatus},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_context_resources", rule = "authored_context_resource")]
pub struct ContextResource {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub execution: Id<ContextExecution>,
    #[model(key, premise)]
    pub item: Id<ContextItem>,
    #[model(premise)]
    pub protocol: Id<models::AuthoredContextProtocol>,
    pub site: Id<source::Occurrence>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub context: Id<attribution::AnalysisContext>,
    pub class: Id<calls::ProviderSymbol>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for ContextResource {}
impl analysis::support::DerivedEvidence for ContextResource {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ContextLifecycle {
    Allocation = 0,
    Initialization = 1,
    Entry = 2,
    Exit = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "model_context_values", rule = "authored_context_entry_value")]
pub enum ContextEntryValue {
    #[model(code = 0)]
    None {
        #[model(premise)]
        resource: Id<ContextResource>,
    },
    #[model(code = 1)]
    Actual {
        #[model(premise)]
        resource: Id<ContextResource>,
        actual: Id<source::Occurrence>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "model_context_postconditions",
    rule = "authored_context_lifecycle"
)]
pub struct ContextPostcondition {
    #[model(key, premise)]
    pub resource: Id<ContextResource>,
    #[model(key)]
    pub event: ContextLifecycle,
    #[model(key)]
    pub phase: ActionPhase,
    pub native_target: Option<Id<calls::CallTarget>>,
    pub entry_value: Option<Id<ContextEntryValue>>,
    pub input: Option<Id<super::enriched_records::ExecutionOutcome>>,
    pub output: Option<Id<super::enriched_records::ExecutionOutcome>>,
    pub suppressed: bool,
}
pub(super) struct ProtocolInputs<'a> {
    pub(super) catalog: &'a models::Catalog,
    pub(super) data: &'a ModelApplicationData,
    pub(super) execution: &'a ContextExecution,
}

pub(super) fn emit(
    protocol_inputs: ProtocolInputs<'_>,
    items: &Rows<ContextItem>,
    outcomes: &Rows<super::enriched_records::ExecutionOutcome>,
    invocation: &publication::AnalysisInvocation,
    records: &mut super::model_production::ModelRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let ProtocolInputs { catalog, data, execution } = protocol_inputs;
    for item in items.iter().filter(|i| i.execution == execution.id()) {
        let protocol = CheckedContextProtocol::derive(
            catalog,
            data,
            item.class,
            invocation.input,
            invocation.context,
            budget,
        )?
        .map_err(|reason| {
            ModelError::Invalid(format!("replayed context protocol unavailable: {reason:?}"))
        })?;
        let construction = CheckedContextConstruction::derive(
            &protocol,
            data,
            item.site,
            execution.owner,
            invocation.input,
            invocation.context,
            budget,
        )?
        .map_err(|reason| {
            ModelError::Invalid(format!(
                "replayed context construction unavailable: {reason:?}"
            ))
        })?;
        if protocol.compiled().declaration().id() != item.protocol
            || construction.allocation() != item.allocation
            || construction.initialization() != item.initialization
            || construction.enumeration() != item.enumeration
            || construction.entry_value()
                != item
                    .entry_actual
                    .map_or(ContextValue::None, ContextValue::Actual)
        {
            return Err(ModelError::Invalid(
                "source context item differs from early checked protocol".into(),
            ));
        }
        let status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [execution.status, construction.status()],
        );
        let resource = ContextResource {
            invocation: invocation.id(),
            execution: execution.id(),
            item: item.id(),
            protocol: item.protocol,
            site: item.site,
            owner: execution.owner,
            context: invocation.context,
            class: item.class,
            qualification: execution.qualification,
            status,
        };
        let value = match construction.entry_value() {
            ContextValue::None => ContextEntryValue::None {
                resource: resource.id(),
            },
            ContextValue::Actual(actual) => ContextEntryValue::Actual {
                resource: resource.id(),
                actual,
            },
        };
        for (event, target) in [
            (ContextLifecycle::Allocation, Some(item.allocation)),
            (ContextLifecycle::Initialization, Some(item.initialization)),
            (ContextLifecycle::Entry, None),
        ] {
            records
                .context_postconditions
                .insert(ContextPostcondition {
                    resource: resource.id(),
                    event,
                    phase: if event == ContextLifecycle::Entry {
                        ActionPhase::Normal
                    } else {
                        ActionPhase::Invocation
                    },
                    native_target: target,
                    entry_value: if event == ContextLifecycle::Entry {
                        Some(value.id())
                    } else {
                        None
                    },
                    input: None,
                    output: None,
                    suppressed: false,
                })?;
        }
        let phase = if matches!(
            outcomes
                .get(item.exit_input)
                .ok_or_else(|| ModelError::Invalid("context exit input outcome absent".into()))?,
            super::enriched_records::ExecutionOutcome::Raise { .. }
        ) {
            ActionPhase::Exceptional
        } else {
            ActionPhase::Normal
        };
        for phase in [phase, ActionPhase::Finally] {
            records
                .context_postconditions
                .insert(ContextPostcondition {
                    resource: resource.id(),
                    event: ContextLifecycle::Exit,
                    phase,
                    native_target: None,
                    entry_value: None,
                    input: Some(item.exit_input),
                    output: Some(item.exit_output),
                    suppressed: item.suppressed,
                })?;
        }
        records.context_values.insert(value)?;
        records.context_resources.insert(resource)?;
    }
    Ok(())
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ContextResource>(),
        Relation::of::<ContextEntryValue>(),
        Relation::of::<ContextPostcondition>(),
    ]
}
