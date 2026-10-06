//! Nominal Model publication. Applicability consumes completed native/catalog/Enriched inputs;
//! invocation actions and exit postconditions are independent. Model never reads Summary.
use super::{
    model_application::{CheckedModelApplication, ModelApplicationData},
    model_rules::{
        self, ActionPhase, AppliedRule, AppliedRules, ChannelAssessment, ModelValuePath,
        ModeledOperation, ResourceIdentity,
    },
};
use crate::domain::{
    analysis::{self, model as publication, policy::EvidenceStatus},
    models::{AuthoredModel, Catalog, ModelCatalog},
    normalized::{
        Rows,
        binding_normalization::{BindingOutput, VerifiedBindings, prepare},
    },
    resources::{Reservation, ResourceBudget},
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "model_applications", rule = "checked_authored_application")]
pub struct ModelApplication {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    #[model(premise)]
    pub model: Id<AuthoredModel>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub target: Id<calls::CallTarget>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub callee: Id<normalized::entities::EntityRef>,
    pub phase: calls::CallPhase,
    pub normal_formal: Option<Id<calls::SignatureParameter>>,
    pub normal_actual: Option<Id<source::Occurrence>>,
    pub call_defaults_available: bool,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
    pub event_members: ContentHash,
    pub set_members: ContentHash,
    pub bindings: ContentHash,
    pub signature_enumeration: Option<Id<calls::SignatureEnumerationObservation>>,
    pub signature_members: Option<ContentHash>,
    pub premises: ContentHash,
}
impl analysis::support::sealed::DerivedEvidence for ModelApplication {}
impl analysis::support::DerivedEvidence for ModelApplication {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="model_application_premises",rule="model_runtime_premise",conclusion=application)]
pub struct ApplicationPremise {
    #[model(key)]
    pub application: Id<ModelApplication>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub premise: Id<analysis::native::NativeAssertionPremise>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_application_boundaries")]
pub struct ApplicationBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_target_assessments", rule = "model_target_applicability")]
pub struct TargetAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub model: Id<AuthoredModel>,
    pub symbol: Option<Id<calls::ProviderSymbol>>,
    pub reason: Option<obligation::ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "modeled_action_assessments",
    rule = "model_action_phase_admission"
)]
pub struct ActionAssessment {
    #[model(key, premise)]
    pub rule: Id<AppliedRule>,
    pub admitted: bool,
    pub reason: Option<obligation::ObligationKind>,
}
/// An invocation receipt supports invocation-phase semantics only. An exit postcondition must
/// name an independently replayed execution result; no authored exit flag supplies that evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "modeled_action_sources", rule = "model_action_evidence")]
pub enum ActionSource {
    #[model(code = 0)]
    Invocation {
        #[model(premise)]
        application: Id<ModelApplication>,
    },
    #[model(code = 1)]
    NormalCall {
        #[model(premise)]
        call: Id<super::modeled_call::ModeledCallEvaluation>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "modeled_action_postconditions",
    rule = "modeled_action_postcondition"
)]
pub struct ActionPostcondition {
    #[model(key, premise)]
    pub assessment: Id<ActionAssessment>,
    #[model(premise)]
    pub source: Id<ActionSource>,
    pub phase: ActionPhase,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="model_runs",invariant_refs=run_invariants_refs,publication_refs=profile_checks_refs)]
pub struct ModelRun {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    pub requested: bool,
    pub applied: i64,
    pub refused: i64,
}

macro_rules! output_rows{($apply:ident)=>{$apply!{closed_targets:super::closed_targets::ClosedTargetAssessment,protocol_actions:super::protocol_interpretation::ProtocolActionAssessment,terminal_assessments:super::protocol_interpretation::TerminalFrontierAssessment,terminal_frontiers:super::protocol_interpretation::ConditionalTerminalFrontier,normal_restrictions:super::protocol_interpretation::NormalContinuationRestriction,exit_characterizations:super::protocol_interpretation::NativeExitCharacterization,assumption_sets:assumptions::AssumptionSet,assumption_members:assumptions::AssumptionSetMember,assumptions:assumptions::Assumption,assumption_universes:assumptions::AssumptionUniverse,universe_supports:assumptions_universe::AssumptionUniverseSupport,applications:ModelApplication,application_premises:ApplicationPremise,boundaries:ApplicationBoundary,targets:TargetAssessment,rules:AppliedRule,channels:ChannelAssessment,operations:ModeledOperation,resources:ResourceIdentity,paths:ModelValuePath,action_assessments:ActionAssessment,action_sources:ActionSource,postconditions:ActionPostcondition,
 context_transfers:super::model_context_transfer::ContextTransferWitness,context_resources:super::model_protocol::ContextResource,context_values:super::model_protocol::ContextEntryValue,context_postconditions:super::model_protocol::ContextPostcondition,transfer_witnesses:super::model_transfer::ModelTransferWitness,transfer_keys:transfer::model::TransferKey,transfer_alternatives:transfer::model::TransferAlternative,transfer_supports:transfer::model::TransferSupport,
 transfer_roots:value::PlaceRoot,transfer_places:value::Place,qualifications:assertion::AssertionQualification,conditions:conditions::Condition,condition_nodes:conditions::ConditionNode,
 subjects:publication::ObligationSubject,support_sources:publication::SupportSource,derivations:publication::AnalysisDerivation,propositions:publication::AnalysisProposition,derivation_premises:publication::AnalysisDerivationPremise,}};}
macro_rules! output {($($field:ident:$ty:ty,)*)=>{
pub struct ModelRecords{pub run:ModelRun,pub outcome:publication::AnalysisOutcome,$(pub $field:Rows<$ty>,)*pub sources:Rows<calls::BindingSource>,pub projections:Rows<calls::BindingProjection>}
impl ModelRecords{fn new(invocation:Id<publication::AnalysisInvocation>,budget:&ResourceBudget)->Self{Self{run:ModelRun{invocation,requested:false,applied:0,refused:0},outcome:publication::AnalysisOutcome{invocation,status:analysis::AnalysisStatus::Completed,reason:None},$($field:Rows::new(budget),)*sources:Rows::new(budget),projections:Rows::new(budget)}}}
};}
output_rows!(output);
pub struct ModelData {
    pub protocol: super::protocol_interpretation::ProtocolData,
    pub context_bindings: Rows<super::context_binding::ContextEntryBinding>,
    pub context_binding_sources: Rows<super::context_binding::BindingSource>,
    pub context_binding_members: Rows<super::context_binding::BindingMember>,
    pub contexts: Rows<super::context_execution::ContextExecution>,
    pub context_items: Rows<super::context_execution::ContextItem>,
    pub context_sources: Rows<super::context_execution::ContextSource>,
    pub context_members: Rows<super::context_execution::ContextMember>,
    pub context_outcomes: Rows<super::enriched_records::ExecutionOutcome>,
    pub completed: super::completion_production::CompletedEvaluations,
    pub flow: conditions::entry::EntryData,
    pub entries: Rows<conditions::entry::EntryValueWitness>,
    pub entry_sources: Rows<conditions::entry::EntryAccessSource>,
    pub modeled_calls: Rows<super::modeled_call::ModeledCallEvaluation>,
    pub modeled_arguments: Rows<super::modeled_call::ModeledCallArgument>,
    pub modeled_native: Rows<super::modeled_call::ModeledCallNative>,
    pub early: ModelApplicationData,
    pub bindings: BindingOutput,
    pub catalogs: Rows<ModelCatalog>,
    pub parameters: Rows<analysis::MethodParameters>,
    pub definitions: Rows<analysis::AnalysisDefinition>,
    pub enriched: Rows<analysis::enriched_execution::AnalysisInvocation>,
    pub source_calls: Rows<analysis::source_call::AnalysisInvocation>,
    pub local: Rows<analysis::local::AnalysisInvocation>,
    pub native: Rows<analysis::native::NativeQualification>,
    pub premises: Rows<analysis::native::NativeAssertionPremise>,
}
impl ModelData {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            protocol: super::protocol_interpretation::ProtocolData::new(budget),
            context_bindings: Rows::new(budget),
            context_binding_sources: Rows::new(budget),
            context_binding_members: Rows::new(budget),
            contexts: Rows::new(budget),
            context_items: Rows::new(budget),
            context_sources: Rows::new(budget),
            context_members: Rows::new(budget),
            context_outcomes: Rows::new(budget),
            completed: super::completion_production::CompletedEvaluations::new(budget),
            flow: conditions::entry::EntryData::new(budget),
            entries: Rows::new(budget),
            entry_sources: Rows::new(budget),
            modeled_calls: Rows::new(budget),
            modeled_arguments: Rows::new(budget),
            modeled_native: Rows::new(budget),
            early: ModelApplicationData::new(budget),
            bindings: BindingOutput::new(budget),
            catalogs: Rows::new(budget),
            parameters: Rows::new(budget),
            definitions: Rows::new(budget),
            enriched: Rows::new(budget),
            source_calls: Rows::new(budget),
            local: Rows::new(budget),
            native: Rows::new(budget),
            premises: Rows::new(budget),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.protocol.visit(name, batch)?;
        self.completed.visit(name, batch)?;
        self.flow.visit(name, batch)?;
        self.early.visit(name, batch)?;
        self.bindings.visit(name, batch)?;
        macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        rows! {context_bindings:super::context_binding::ContextEntryBinding,context_binding_sources:super::context_binding::BindingSource,context_binding_members:super::context_binding::BindingMember,contexts:super::context_execution::ContextExecution,context_items:super::context_execution::ContextItem,context_sources:super::context_execution::ContextSource,context_members:super::context_execution::ContextMember,context_outcomes:super::enriched_records::ExecutionOutcome,entries:conditions::entry::EntryValueWitness,entry_sources:conditions::entry::EntryAccessSource,modeled_calls:super::modeled_call::ModeledCallEvaluation,modeled_arguments:super::modeled_call::ModeledCallArgument,modeled_native:super::modeled_call::ModeledCallNative,catalogs:ModelCatalog,parameters:analysis::MethodParameters,definitions:analysis::AnalysisDefinition,enriched:analysis::enriched_execution::AnalysisInvocation,source_calls:analysis::source_call::AnalysisInvocation,local:analysis::local::AnalysisInvocation,native:analysis::native::NativeQualification,premises:analysis::native::NativeAssertionPremise,}
        Ok(())
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        super::require_facts_view(input)?;
        self.visit(input.name(), batch)
    }
    pub fn consumed_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
        if profile == stages::Profile::Behavioral {
            return Self::inputs();
        }
        vec![
            ValidationInput::of::<ModelCatalog>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ]
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut rows = ModelApplicationData::validation_inputs();
        rows.extend(
            super::protocol_interpretation::ProtocolData::inputs()
                .into_iter()
                .map(|i| {
                    if stages::is_vocabulary(i.name()) {
                        i.at_epoch(stages::PublicationBoundary::Facts)
                    } else {
                        i
                    }
                }),
        );
        rows.extend(BindingOutput::validation_inputs());
        rows.extend(
            super::completion_production::CompletedEvaluations::consumed_inputs(
                stages::Profile::Behavioral,
            ),
        );
        rows.extend(normalized::facts_inputs(
            conditions::entry::EntryData::validation_inputs(),
        ));
        rows.extend([
            ValidationInput::of::<super::context_binding::ContextEntryBinding>(&["id"]),
            ValidationInput::of::<super::context_binding::BindingSource>(&["id"]),
            ValidationInput::of::<super::context_binding::BindingMember>(&["id"]),
        ]);
        rows.extend([
            ValidationInput::of::<super::context_execution::ContextExecution>(&["id"]),
            ValidationInput::of::<super::context_execution::ContextItem>(&["id"]),
            ValidationInput::of::<super::context_execution::ContextSource>(&["id"]),
            ValidationInput::of::<super::context_execution::ContextMember>(&["id"]),
            ValidationInput::of::<super::enriched_records::ExecutionOutcome>(&["id"]),
        ]);
        rows.extend([
            ValidationInput::of::<super::modeled_call::ModeledCallEvaluation>(&["id"]),
            ValidationInput::of::<super::modeled_call::ModeledCallArgument>(&["id"]),
            ValidationInput::of::<super::modeled_call::ModeledCallNative>(&["id"]),
            ValidationInput::of::<conditions::entry::EntryValueWitness>(&["id"]),
            ValidationInput::of::<conditions::entry::EntryAccessSource>(&["id"]),
        ]);
        rows.extend([
            ValidationInput::of::<ModelCatalog>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
        ]);
        rows.sort_by_key(|i| (i.name(), i.prefix()));
        rows.dedup_by_key(|i| (i.name(), i.prefix()));
        rows
    }
}
pub struct SelectedCatalog {
    catalog: Catalog,
    _parse: Box<dyn Reservation>,
}
impl SelectedCatalog {
    pub fn read(row: &ModelCatalog, budget: &ResourceBudget) -> Result<Self, ModelError> {
        row.validate()?;
        let parse = budget.reserve(
            "model-selected-catalog",
            row.source.len().saturating_mul(32).saturating_add(65536),
        )?;
        let catalog = Catalog::parse(&row.source_name, &row.source).map_err(ModelError::Invalid)?;
        if catalog.declaration() != row {
            return Err(invalid("model selected source identity differs"));
        }
        Ok(Self {
            catalog,
            _parse: parse,
        })
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn catalog_id(
    data: &ModelData,
    definition: &analysis::AnalysisDefinition,
) -> Result<Id<ModelCatalog>, ModelError> {
    let catalog = data
        .parameters
        .get(definition.parameters)
        .and_then(|p| p.model_catalog)
        .ok_or_else(|| invalid("Models definition has no selected catalog"))?;
    if *definition != super::configuration::models(catalog).1 {
        return Err(invalid("Models definition is not bound"));
    }
    Ok(catalog)
}
pub fn apply_all(
    data: &ModelData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<ModelRecords, ModelError> {
    let selected = catalog_id(data, definition)?;
    let row = data
        .catalogs
        .get(selected)
        .ok_or_else(|| invalid("selected catalog is absent"))?;
    let parsed = SelectedCatalog::read(row, budget)?;
    let verified = if profile == stages::Profile::Behavioral {
        Some(prepare(&data.early.bindings, &data.bindings, budget)?)
    } else {
        None
    };
    apply_selected_inner(
        data,
        invocation,
        definition,
        profile,
        &parsed,
        verified.as_ref(),
        ProductionScope::All,
        None,
        budget,
    )
}
/// The selected root is a publication domain. Referenced dependencies never become extra roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductionScope {
    All,
    Frame,
    Target(Id<AuthoredModel>),
    Context(Id<super::context_execution::ContextExecution>),
    Event(Id<normalized::events::NormalizedCallEvent>),
    Terminal(Id<protocols::NativeTerminalObservation>),
    Exit(Id<protocols::NativeExitObservation>),
}
impl ProductionScope {
    pub(super) fn attempt(
        self,
        data: &ModelData,
        attempt: Id<normalized::bindings::CallBindingAttempt>,
    ) -> bool {
        self == Self::All
            || matches!(self,Self::Event(event) if data.bindings.attempts.get(attempt).is_some_and(|row|row.event==event))
    }
    pub(super) fn target(self, data: &ModelData, target: &calls::CallTarget) -> bool {
        self == Self::All
            || matches!(self,Self::Event(event) if data.early.bindings.event_events.get(event).is_some_and(|event|
            event.site==target.site && event.origin==target.origin && data.early.bindings.qualifications.get(target.qualification).is_some_and(|q|q.context==event.context)))
    }
    pub(super) fn terminal(
        self,
        data: &ModelData,
        row: &protocols::NativeTerminalObservation,
    ) -> bool {
        self == Self::All
            || self == Self::Terminal(row.id())
            || matches!(self,Self::Event(event) if data.early.bindings.event_events.get(event).is_some_and(|event|
            event.site==row.subject && event.origin==calls::CallOrigin::explicit() && data.early.bindings.qualifications.get(row.qualification).is_some_and(|q|q.context==event.context)))
    }
}
pub struct ActualInputs<'a> {
    pub evaluations: &'a super::production::ProducedEvaluations,
    pub local: &'a local_semantics::ProducedLocal,
}
/// Consume actual binding-owner authority; the ordinary `apply_all` remains an explicit diagnostic.
#[allow(clippy::too_many_arguments, reason = "Selected catalog, binding authority, actual execution values, semantic root and frame configuration are independent inputs.")]
pub fn apply_selected(
    data: &ModelData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    parsed: &SelectedCatalog,
    verified: Option<&VerifiedBindings>,
    scope: ProductionScope,
    actual: Option<&ActualInputs<'_>>,
    budget: &ResourceBudget,
) -> Result<ModelRecords, ModelError> {
    if profile == stages::Profile::Behavioral && actual.is_none() {
        return Err(invalid("Model actual execution/Local owner values absent"));
    }
    if scope == ProductionScope::All {
        return Err(invalid("Model production requires an explicit root"));
    }
    apply_selected_inner(
        data, invocation, definition, profile, parsed, verified, scope, actual, budget,
    )
}
#[allow(clippy::too_many_arguments, reason = "Diagnostic and actual application share explicit catalog, authorities, selected root and immutable frame configuration.")]
fn apply_selected_inner(
    data: &ModelData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    parsed: &SelectedCatalog,
    verified: Option<&VerifiedBindings>,
    scope: ProductionScope,
    actual: Option<&ActualInputs<'_>>,
    budget: &ResourceBudget,
) -> Result<ModelRecords, ModelError> {
    let selected = catalog_id(data, definition)?;
    if invocation.definition != definition.id() || invocation.subject.is_some() {
        return Err(invalid(
            "Model invocation changes bound whole-frame definition",
        ));
    }
    if parsed.catalog().declaration().id() != selected
        || data.catalogs.get(selected) != Some(parsed.catalog().declaration())
    {
        return Err(invalid("Model selected catalog differs from actual source"));
    }
    if let ProductionScope::Target(model) = scope
        && !parsed
            .catalog()
            .models()
            .iter()
            .any(|compiled| compiled.declaration().id() == model)
    {
        return Err(invalid("Model target root absent from selected catalog"));
    }
    let catalog = parsed.catalog();
    let mut records = ModelRecords::new(invocation.id(), budget);
    if profile != stages::Profile::Behavioral {
        records.outcome.status = analysis::AnalysisStatus::NotRequested;
        records.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok(records);
    }
    records.run.requested = true;
    let mut parents = data
        .enriched
        .iter()
        .filter(|p| (p.input, p.context) == (invocation.input, invocation.context));
    let parent = parents
        .next()
        .ok_or_else(|| invalid("Model Enriched frame absent"))?;
    if parents.next().is_some() {
        return Err(invalid("Model Enriched frame ambiguous"));
    }
    let parent_definition = data
        .definitions
        .get(parent.definition)
        .ok_or_else(|| invalid("Model Enriched definition absent"))?;
    let (enriched_parameters, enriched_definition) =
        super::configuration::enriched_execution(selected);
    if parent.subject.is_some()
        || *parent_definition != enriched_definition
        || data.parameters.get(enriched_parameters.id()) != Some(&enriched_parameters)
    {
        return Err(invalid(
            "Model completed Enriched configuration differs from selected catalog",
        ));
    }
    // Actual selected regions borrow enumeration admission once; diagnostic application
    // retains ordinary construction checks through the same core with no prepared borrow.
    let construction = if actual.is_some() {
        Some(super::model_construction::PreparedConstructionInputs::new(
            &data.early,
            verified.ok_or_else(|| invalid("Model binding owner authority absent"))?,
            budget,
        )?)
    } else {
        None
    };
    let same_frame = |id| id == parent.id();
    // Enriched is a completed, checked predecessor. Model-specific applicability uses its
    // exact frame directly; recreating execution would replace predecessor authority with a
    // second producer run and retain another complete set of rich context/call records.
    for context in data.contexts.iter().filter(|context| {
        same_frame(context.invocation)
            && (scope == ProductionScope::All || scope == ProductionScope::Context(context.id()))
    }) {
        let inputs = super::model_protocol::ProtocolInputs {
            catalog,
            data: &data.early,
            execution: context,
        };
        match construction.as_ref() {
            Some(prepared) => super::model_protocol::emit_with_inputs(
                inputs,
                &data.context_items,
                &data.context_outcomes,
                invocation,
                &mut records,
                budget,
                Some(prepared),
            )?,
            None => super::model_protocol::emit(
                inputs,
                &data.context_items,
                &data.context_outcomes,
                invocation,
                &mut records,
                budget,
            )?,
        }
    }
    for binding in data.context_bindings.iter().filter(|binding| {
        same_frame(binding.invocation)
            && matches!(scope, ProductionScope::All | ProductionScope::Context(_))
    }) {
        let resource = {
            let mut resources = records.context_resources.iter().filter(|r| {
                data.context_items
                    .get(r.item)
                    .is_some_and(|i| i.item == binding.item)
                    && r.site == binding.site
            });
            let first = resources.next().cloned();
            if resources.next().is_some() {
                return Err(invalid("Model context binding resource ambiguous"));
            }
            first
        };
        if let Some(resource) = resource
            && let Ok(proof) = super::model_context_transfer::CheckedContextTransfer::derive(
                &resource,
                binding,
                &data.completed,
                &data.flow,
                &data.entries,
                &data.entry_sources,
                actual,
                budget,
            )?
        {
            super::model_context_transfer::emit(
                proof,
                invocation,
                definition,
                &mut records,
                budget,
            )?;
        }
    }
    let verified = verified.ok_or_else(|| invalid("Model binding owner authority absent"))?;
    // Catalog targets describe Python operations. A document-only frame has no such
    // domain; missing catalog targets there are not missing Python evidence. Retain the
    // predecessor/binding checks above and derive emptiness from captured uses, never outputs.
    if matches!(scope, ProductionScope::All | ProductionScope::Target(_))
        && !has_python_domain(&data.early, invocation.input, budget)?
    {
        return Ok(records);
    }
    for compiled in catalog.models().iter().filter(|compiled| {
        scope == ProductionScope::All
            || scope == ProductionScope::Target(compiled.declaration().id())
    }) {
        let mut symbols = data.early.bindings.symbols.iter().filter(|s| {
            s.context == invocation.context
                && super::model_application::matches_target(
                    &data.early,
                    &compiled.model().target,
                    s,
                    invocation.input,
                    invocation.context,
                )
                .is_ok()
        });
        let first = symbols.next();
        let unique = symbols.next().is_none();
        let symbol = if unique { first.map(Record::id) } else { None };
        records.targets.insert(TargetAssessment {
            invocation: invocation.id(),
            model: compiled.declaration().id(),
            symbol,
            reason: if symbol.is_none() {
                Some(if unique {
                    obligation::ObligationKind::MissingEvidence
                } else {
                    obligation::ObligationKind::AmbiguousBinding
                })
            } else {
                None
            },
        })?;
    }
    for attempt in data.bindings.attempts.iter().filter(|a| {
        scope.attempt(data, a.id())
            && data
                .early
                .bindings
                .event_events
                .get(a.event)
                .is_some_and(|e| {
                    e.context == invocation.context
                        && data
                            .early
                            .bindings
                            .occurrences
                            .get(e.site)
                            .and_then(|o| data.early.bindings.artifacts.get(o.source))
                            .is_some_and(|a| a.input == invocation.input)
                })
    }) {
        let result = match (verified.bound(attempt.id()), verified.shape(attempt.id())) {
            (Some(bound), Some(shape)) => CheckedModelApplication::derive_with_inputs(
                catalog,
                &data.early,
                bound,
                shape,
                verified.effective_invocation(attempt.id()),
                budget,
                construction.as_ref(),
            )?,
            _ => Err(attempt
                .refusal
                .unwrap_or(obligation::ObligationKind::MissingEvidence)),
        };
        match result {
            Err(reason) => {
                records.boundaries.insert(ApplicationBoundary {
                    invocation: invocation.id(),
                    attempt: attempt.id(),
                    event: attempt.event,
                    reason,
                })?;
            }
            Ok(checked) => {
                let target = data
                    .early
                    .bindings
                    .targets
                    .get(checked.shape().target())
                    .ok_or_else(|| invalid("model target absent"))?;
                let status = checked.status();
                let row = ModelApplication {
                    invocation: invocation.id(),
                    attempt: attempt.id(),
                    model: checked.model(),
                    event: checked.shape().event(),
                    target: checked.shape().target(),
                    owner: checked.shape().owner_entity(),
                    callee: checked.shape().callee(),
                    phase: checked.shape().phase(),
                    normal_formal: checked.normal_parameter().map(|p| p.0),
                    normal_actual: checked.normal_parameter().map(|p| p.1),
                    call_defaults_available: checked.call_defaults_available(),
                    qualification: target.qualification,
                    status,
                    event_members: checked.shape().event_members(),
                    set_members: checked.shape().set_members(),
                    bindings: checked.shape().bindings(),
                    signature_enumeration: checked.shape().enumeration(),
                    signature_members: checked.shape().signature_members(),
                    premises: super::records::ordered_digest(
                        "model-runtime-premises",
                        checked.premises().iter().map(Record::id),
                    ),
                };
                for (ordinal, premise) in checked.premises().iter().enumerate() {
                    records.application_premises.insert(ApplicationPremise {
                        application: row.id(),
                        ordinal: ordinal as i64,
                        premise: premise.id(),
                    })?;
                }
                let applied = AppliedRules::derive(&checked, row.id(), &data.early, budget)?;
                let normal_call = data
                    .modeled_calls
                    .iter()
                    .filter(|c| same_frame(c.invocation))
                    .find(|c| {
                        c.attempt == row.attempt
                            && c.model == row.model
                            && c.event == row.event
                            && c.owner == row.owner
                    });
                for rule in applied.rules.iter() {
                    let admitted = rule.applicable
                        && (rule.phase == ActionPhase::Invocation
                            || normal_call.is_some()
                                && matches!(
                                    rule.phase,
                                    ActionPhase::Normal | ActionPhase::Finally
                                ));
                    let assessment = ActionAssessment {
                        rule: rule.id(),
                        admitted,
                        reason: if admitted {
                            None
                        } else {
                            Some(
                                rule.reason
                                    .unwrap_or(obligation::ObligationKind::MissingEvidence),
                            )
                        },
                    };
                    if admitted {
                        let source = if rule.phase == ActionPhase::Invocation {
                            ActionSource::Invocation {
                                application: row.id(),
                            }
                        } else {
                            ActionSource::NormalCall {
                                call: normal_call.unwrap().id(),
                            }
                        };
                        records.postconditions.insert(ActionPostcondition {
                            assessment: assessment.id(),
                            source: source.id(),
                            phase: rule.phase,
                        })?;
                        records.action_sources.insert(source)?;
                        if let Some(call) = normal_call {
                            let operation = applied
                                .operations
                                .get(rule.operation)
                                .ok_or_else(|| invalid("Model operation absent"))?;
                            if matches!(operation, ModeledOperation::Transfer { .. })
                                && let Ok(proof) =
                                    super::model_transfer::CheckedModelTransfer::derive(
                                        super::model_transfer::TransferRuleInputs {
                                            application: &row,
                                            rule,
                                            operation,
                                            paths: &applied.paths,
                                        },
                                        call,
                                        &data.modeled_arguments,
                                        &data.completed,
                                        super::model_transfer::TransferEntryInputs {
                                            entry_data: &data.flow,
                                            entries: &data.entries,
                                            sources: &data.entry_sources,
                                        },
                                        actual,
                                        budget,
                                    )?
                            {
                                super::model_transfer::emit(
                                    proof,
                                    invocation,
                                    definition,
                                    &mut records,
                                    budget,
                                )?;
                            }
                        }
                    }
                    records.action_assessments.insert(assessment)?;
                    records.rules.insert(rule.clone())?;
                }
                for channel in applied.channels.iter() {
                    records.channels.insert(channel.clone())?;
                }
                for operation in applied.operations.iter() {
                    records.operations.insert(operation.clone())?;
                }
                for resource in applied.resources.iter() {
                    records.resources.insert(resource.clone())?;
                }
                for path in applied.paths.iter() {
                    records.paths.insert(path.clone())?;
                }
                for source in applied.sources.iter() {
                    records.sources.insert(source.clone())?;
                }
                for projection in applied.projections.iter() {
                    records.projections.insert(projection.clone())?;
                }
                records.applications.insert(row)?;
            }
        }
    }
    match construction.as_ref() {
        Some(prepared) => super::protocol_interpretation::emit_with_inputs(
            data,
            catalog,
            verified,
            invocation,
            scope,
            &mut records,
            budget,
            Some(prepared),
        )?,
        None => super::protocol_interpretation::emit(
            data,
            catalog,
            verified,
            invocation,
            scope,
            &mut records,
            budget,
        )?,
    }
    records.run.applied = records.applications.len() as i64;
    records.run.refused = records.boundaries.len() as i64;
    if !records.boundaries.is_empty()
        || records.targets.iter().any(|t| t.reason.is_some())
        || records.channels.iter().any(|c| !c.complete)
        || records.action_assessments.iter().any(|a| !a.admitted)
    {
        records.outcome.status = analysis::AnalysisStatus::Partial;
        records.outcome.reason = Some(obligation::ObligationKind::IncompleteCoverage)
    }
    Ok(records)
}
fn has_python_domain(
    data: &ModelApplicationData,
    input: Id<input::InputRevision>,
    budget: &ResourceBudget,
) -> Result<bool, ModelError> {
    let bytes = data
        .bindings
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<source::SourceArtifact>() + row.heap_bytes() + 128)
        })
        .and_then(|n| {
            n.checked_add(
                data.uses
                    .len()
                    .checked_mul(size_of::<input::ArtifactUse>())?,
            )
        })
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| invalid("Model root allowance overflow"))?;
    let _roots = budget.reserve("model_source_roots", bytes)?;
    let roots = admission::analysis_roots(
        &data.bindings.artifacts.iter().cloned().collect::<Vec<_>>(),
        &data.uses.iter().cloned().collect::<Vec<_>>(),
    )?;
    Ok(data.bindings.artifacts.iter().any(|artifact| {
        artifact.input == input
            && roots.contains(&artifact.id())
            && admission::ArtifactClass::of(&artifact.path)
                == Some(admission::ArtifactClass::PythonSource)
    }))
}
fn run_inputs() -> Vec<ValidationInput> {
    let mut inputs = ModelData::inputs();
    inputs.extend([
        ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<publication::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<ModelRun>(&["id"]),
    ]);
    macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    output_rows!(rows);
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    inputs
}
pub(crate) fn run_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "model_inventory_replay",
        inputs: run_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(ModelCheck::new(budget))),
    }]
}
macro_rules! checker{($($field:ident:$ty:ty,)*)=>{
struct ModelCheck{data:ModelData,invocations:Rows<publication::AnalysisInvocation>,outcomes:Rows<publication::AnalysisOutcome>,runs:Rows<ModelRun>,$($field:Rows<$ty>,)*budget:ResourceBudget}
impl ModelCheck{fn new(budget:&ResourceBudget)->Self{Self{data:ModelData::new(budget),invocations:Rows::new(budget),outcomes:Rows::new(budget),runs:Rows::new(budget),$($field:Rows::new(budget),)*budget:budget.clone()}}}
impl InvariantCheck for ModelCheck{
 fn visit_input(&mut self,input:&ValidationInput,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{
  if stages::is_vocabulary(input.name()) {if input.prefix()==Some(stages::PublicationBoundary::Facts){self.data.visit_input(input,batch)?;}else{if input.prefix().is_some(){return Err(invalid("Model output vocabulary requires the current completed view"));}$(if input.name()==<$ty>::NAME{self.$field.decode(batch)?;})*}Ok(())}else{self.visit(input.name(),batch)}
 }
 fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.data.visit(name,batch)?;$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*if name==publication::AnalysisInvocation::NAME{self.invocations.decode(batch)?;}if name==publication::AnalysisOutcome::NAME{self.outcomes.decode(batch)?;}if name==ModelRun::NAME{self.runs.decode(batch)?;}Ok(())}
 fn finish(self:Box<Self>)->Result<(),ModelError>{let mut runs=Rows::new(&self.budget);let mut outcomes=Rows::new(&self.budget);$(let mut $field=Rows::<$ty>::new(&self.budget);)*let mut frames=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(&self.budget,"model-frame-inventory");for run in self.data.early.bindings.runs.iter(){frames.insert(&mut charge,(run.input,run.context))?;}
 if frames.len()!=self.data.enriched.len()||frames.iter().any(|f|self.data.enriched.iter().filter(|i|(i.input,i.context)==*f&&i.subject.is_none()).count()!=1){return Err(invalid("Model predecessor omits or duplicates an independently captured native frame"));}
 if frames.len()!=self.invocations.len()||frames.iter().any(|f|self.invocations.iter().filter(|i|(i.input,i.context)==*f).count()!=1){return Err(invalid("Model omitted or duplicated an Enriched frame"));}
 for invocation in self.invocations.iter(){let mut parents=Rows::new(&self.budget);for p in self.data.enriched.iter().filter(|p|(p.input,p.context)==(invocation.input,invocation.context)){parents.insert(publication::InvocationSource::EnrichedExecution{invocation:p.id()})?;}for p in self.data.source_calls.iter().filter(|p|(p.input,p.context)==(invocation.input,invocation.context)){parents.insert(publication::InvocationSource::SourceCallAnalysis{invocation:p.id()})?;}for p in self.data.local.iter().filter(|p|(p.input,p.context)==(invocation.input,invocation.context)){parents.insert(publication::InvocationSource::Local{invocation:p.id()})?;}let mut key=KeySink::new("analysis-invocation-inputs");for p in parents.iter(){p.id().encode(&mut key);}if invocation.inputs!=key.finish(){return Err(invalid("Model changes its exact predecessor inventory"));}
 let definition=self.data.definitions.get(invocation.definition).ok_or_else(||invalid("Model definition absent"))?;let run=self.runs.iter().find(|r|r.invocation==invocation.id()).ok_or_else(||invalid("Model run absent"))?;let expected=apply_all(&self.data,invocation,definition,if run.requested{stages::Profile::Behavioral}else{stages::Profile::Catalog},&self.budget)?;runs.insert(expected.run)?;outcomes.insert(expected.outcome)?;$(for row in expected.$field.iter(){$field.insert(row.clone())?;})*}
 if !self.runs.same(&runs)||!self.outcomes.same(&outcomes)$(||if stages::is_vocabulary(<$ty>::NAME){$field.iter().any(|r|self.$field.get(r.id())!=Some(r))}else{!self.$field.same(&$field)})*{return Err(invalid("Model output differs from independently replayed native/authored universe"));}Ok(())}
}
};}
output_rows!(checker);
pub(crate) fn profile_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "model_profile",
        inputs: vec![
            ValidationInput::of::<ModelRun>(&["id"]),
            ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(ProfileCheck {
                runs: Rows::new(b),
                invocations: Rows::new(b),
            })
        }),
    }]
}
struct ProfileCheck {
    runs: Rows<ModelRun>,
    invocations: Rows<publication::AnalysisInvocation>,
}
impl PublicationCheck for ProfileCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == ModelRun::NAME {
            self.runs.decode(batch)?;
        }
        if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        }
        Ok(())
    }
    fn finish(
        self: Box<Self>,
        _sources: &[crate::domain::analysis::sources::SourceSnapshot],
        profile: stages::Profile,
    ) -> Result<(), ModelError> {
        if self.runs.len() != self.invocations.len()
            || self.runs.iter().any(|r| {
                self.invocations.get(r.invocation).is_none()
                    || r.requested != (profile == stages::Profile::Behavioral)
            })
        {
            return Err(invalid("Model changes actual requested profile"));
        }
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    let mut rows = vec![
        Relation::of::<assumptions_universe::AssumptionUniverseSupport>(),
        Relation::of::<ModelApplication>(),
        Relation::of::<ApplicationPremise>(),
        Relation::of::<ApplicationBoundary>(),
        Relation::of::<TargetAssessment>(),
        Relation::of::<ActionAssessment>(),
        Relation::of::<ActionSource>(),
        Relation::of::<ActionPostcondition>(),
        Relation::of::<ModelRun>(),
    ];
    rows.extend(super::closed_targets::relations());
    rows.extend(super::protocol_interpretation::relations());
    rows.extend(model_rules::relations());
    rows.extend(super::model_transfer::relations());
    rows.extend(super::model_protocol::relations());
    rows.extend(super::model_context_transfer::relations());
    rows
}
pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    let parameters = model.relation(analysis::MethodParameters::NAME);
    let _ = parameters;
    if definition.method != analysis::AnalysisMethod::Models {
        return Err(invalid("Model stage requires Models method"));
    }
    let mut outputs = publication::publication_relations();
    outputs.push(Relation::of::<publication::ObligationSubject>());
    outputs.extend(publication::support::relations());
    outputs.extend(transfer::model::relations());
    outputs.extend(relations());
    outputs.extend([
        Relation::of::<value::PlaceRoot>(),
        Relation::of::<value::Place>(),
        Relation::of::<assertion::AssertionQualification>(),
        Relation::of::<assumptions::AssumptionSet>(),
        Relation::of::<assumptions::AssumptionSetMember>(),
        Relation::of::<assumptions::Assumption>(),
        Relation::of::<assumptions::AssumptionUniverse>(),
        Relation::of::<conditions::Condition>(),
        Relation::of::<conditions::ConditionNode>(),
    ]);
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let mut initial = if profile == Profile::Behavioral {
        run_inputs()
    } else {
        vec![
            ValidationInput::of::<ModelCatalog>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ]
    };
    initial.extend([
        ValidationInput::of::<input::InputRevision>(&["id"]),
        ValidationInput::of::<source::SourceArtifact>(&["id"]),
        ValidationInput::of::<input::ArtifactUse>(&["id"]),
        ValidationInput::of::<source::CoverageScope>(&["id"]),
        ValidationInput::of::<normalized::coverage::NormalizationComputation>(&["id"]),
        ValidationInput::of::<normalized::coverage::NormalizationCoverage>(&["id"]),
        ValidationInput::of::<attribution::ProviderCoverage>(&["id"]),
    ]);
    for relation in &outputs {
        for id in relation.publication_refs() {
            let check = model.publication_check(id)?;
            initial.extend(check.inputs.iter().cloned());
        }
    }
    let owned = outputs
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    // A publication check can name the records being written; only its predecessors are roots.
    initial.retain(|input| {
        is_vocabulary(input.name()) || !owned.iter().any(|row| row.name() == input.name())
    });
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        initial,
        &owned,
        PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    let mut key = KeySink::new("model-definition");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "apply_models",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("model_production.rs")),
        configuration: key.finish(),
    })
}

pub(crate) fn run_invariants_refs() -> Vec<&'static str> {
    vec!["model_inventory_replay"]
}
pub(crate) fn profile_checks_refs() -> Vec<&'static str> {
    vec!["model_profile"]
}

#[cfg(test)]
mod domain_controls {
    use super::*;

    fn input(path: &str) -> input::InputRevision {
        input::InputRevision::from_entries(vec![input::ManifestEntry {
            path: path.into(),
            content: ContentHash::of(b"captured"),
            byte_len: 8,
        }])
        .unwrap()
    }

    #[test]
    fn model_domain_uses_owned_requested_python_artifacts_not_result_inventory() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let own = input("guide.md").id();
        let other = input("foreign.py").id();
        for role in [
            input::SourceRole::Release,
            input::SourceRole::Example,
            input::SourceRole::Test,
            input::SourceRole::DocBlock,
            input::SourceRole::Document,
            input::SourceRole::Configuration,
            input::SourceRole::Dependency,
        ] {
            let mut data = ModelApplicationData::new(&budget);
            let doc = source::SourceArtifact::from_bytes(own, "guide.md".into(), b"guide").unwrap();
            let foreign =
                source::SourceArtifact::from_bytes(other, "foreign.py".into(), b"x=1").unwrap();
            data.bindings.artifacts.insert(doc.clone()).unwrap();
            data.bindings.artifacts.insert(foreign.clone()).unwrap();
            data.uses
                .insert(input::ArtifactUse {
                    input: own,
                    artifact: doc.id(),
                    role: input::SourceRole::Document,
                })
                .unwrap();
            data.uses
                .insert(input::ArtifactUse {
                    input: other,
                    artifact: foreign.id(),
                    role: input::SourceRole::Release,
                })
                .unwrap();
            assert!(!has_python_domain(&data, own, &budget).unwrap());
            assert!(has_python_domain(&data, other, &budget).unwrap());
            let python =
                source::SourceArtifact::from_bytes(own, "example.pyi".into(), b"x: int").unwrap();
            data.bindings.artifacts.insert(python.clone()).unwrap();
            assert!(
                !has_python_domain(&data, own, &budget).unwrap(),
                "a filename without a use is supporting context"
            );
            data.uses
                .insert(input::ArtifactUse {
                    input: own,
                    artifact: python.id(),
                    role,
                })
                .unwrap();
            assert_eq!(
                has_python_domain(&data, own, &budget).unwrap(),
                matches!(
                    role,
                    input::SourceRole::Release
                        | input::SourceRole::Example
                        | input::SourceRole::Test
                        | input::SourceRole::DocBlock
                )
            );
            assert!(
                data.bindings.symbols.is_empty(),
                "the declared domain does not require result symbols"
            );
        }
    }

    #[test]
    fn model_domain_refuses_missing_artifacts_and_retains_resource_failure() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let own = input("example.py").id();
        let python = source::SourceArtifact::from_bytes(own, "example.py".into(), b"x=1").unwrap();
        let mut data = ModelApplicationData::new(&budget);
        data.uses
            .insert(input::ArtifactUse {
                input: own,
                artifact: python.id(),
                role: input::SourceRole::Release,
            })
            .unwrap();
        assert!(matches!(
            has_python_domain(&data, own, &budget),
            Err(ModelError::Frontier(_))
        ));
        data.bindings.artifacts.insert(python).unwrap();
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            has_python_domain(&data, own, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
        let before = budget.reserved();
        assert!(has_python_domain(&data, own, &budget).unwrap());
        assert_eq!(budget.reserved(), before);
    }
}

#[cfg(test)]
mod completed_enriched_controls {
    use super::*;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn selected_protocol_borrow_refuses_another_immutable_region_before_empty_domain() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let catalog =
            Catalog::parse("empty.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n").unwrap();
        let data = ModelData::new(&budget);
        let verified = prepare(&data.early.bindings, &data.bindings, &budget).unwrap();
        let prepared = super::super::model_construction::PreparedConstructionInputs::new(
            &data.early,
            &verified,
            &budget,
        )
        .unwrap();
        let (_, definition) = super::super::configuration::models(catalog.declaration().id());
        let (invocation, _) =
            publication::AnalysisInvocation::new(nominal(1), nominal(2), definition.id(), None, []);
        let mut ordinary = ModelRecords::new(invocation.id(), &budget);
        let mut selected = ModelRecords::new(invocation.id(), &budget);
        super::super::protocol_interpretation::emit(
            &data,
            &catalog,
            &verified,
            &invocation,
            ProductionScope::All,
            &mut ordinary,
            &budget,
        )
        .unwrap();
        super::super::protocol_interpretation::emit_with_inputs(
            &data,
            &catalog,
            &verified,
            &invocation,
            ProductionScope::All,
            &mut selected,
            &budget,
            Some(&prepared),
        )
        .unwrap();
        // A legitimate empty action domain stays empty under either construction entry.
        assert!(ordinary.closed_targets.is_empty() && selected.closed_targets.is_empty());
        assert!(ordinary.protocol_actions.is_empty() && selected.protocol_actions.is_empty());
        assert!(
            ordinary.terminal_assessments.is_empty() && selected.terminal_assessments.is_empty()
        );
        let other = ModelData::new(&budget);
        assert!(matches!(
            super::super::protocol_interpretation::emit_with_inputs(
                &other,
                &catalog,
                &verified,
                &invocation,
                ProductionScope::All,
                &mut selected,
                &budget,
                Some(&prepared)
            ),
            Err(ModelError::Conflict(_))
        ));
        assert!(selected.closed_targets.is_empty() && selected.protocol_actions.is_empty());
    }
    #[test]
    fn model_consumes_completed_empty_enriched_frame_without_source_call_producer() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let catalog =
            Catalog::parse("empty.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n").unwrap();
        let selected = catalog.declaration().id();
        let mut data = ModelData::new(&budget);
        data.catalogs.insert(catalog.declaration().clone()).unwrap();
        let (parameters, definition) = super::super::configuration::models(selected);
        let (enriched_parameters, enriched_definition) =
            super::super::configuration::enriched_execution(selected);
        data.parameters.insert(parameters).unwrap();
        data.parameters.insert(enriched_parameters).unwrap();
        data.definitions.insert(definition.clone()).unwrap();
        data.definitions
            .insert(enriched_definition.clone())
            .unwrap();
        let (parent, _) = analysis::enriched_execution::AnalysisInvocation::new(
            nominal(1),
            nominal(2),
            enriched_definition.id(),
            None,
            [],
        );
        data.enriched.insert(parent.clone()).unwrap();
        let (invocation, _) = publication::AnalysisInvocation::new(
            parent.input,
            parent.context,
            definition.id(),
            None,
            [],
        );
        // No SourceCalls or execution input exists: this legitimate empty scope cannot be
        // serviced by replaying Enriched, which would demand those predecessor producers.
        assert!(data.source_calls.is_empty());
        let output = apply_all(
            &data,
            &invocation,
            &definition,
            stages::Profile::Behavioral,
            &budget,
        )
        .unwrap();
        assert!(output.run.requested);
        assert_eq!(output.outcome.status, analysis::AnalysisStatus::Completed);
        assert!(output.applications.is_empty());
    }
}
