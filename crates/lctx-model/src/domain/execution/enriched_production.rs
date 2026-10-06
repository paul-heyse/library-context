//! One finite enriched owner, with independently replayed SourceCall predecessors.
use super::{
    capture_bridge::{CapturedEntryBinding, CapturedValueSource, CheckedCapturedEntry},
    context_binding::{BindingMember, BindingSource, CheckedContextBinding, ContextEntryBinding},
    context_execution::*,
    definition::*,
    enriched_records::*,
    modeled_call::*,
    source_call_records::{self, SourceCallData},
};
use crate::Domain;
use crate::domain::{
    analysis::{self, enriched_execution as publication},
    input::ArtifactUse,
    normalized::Rows,
    resources::ResourceBudget,
    source::{Occurrence, SourceArtifact},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "execution_boundaries")]
pub struct ExecutionBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Option<Id<normalized::entities::EntityRef>>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "execution_body_boundaries")]
pub struct BodyBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub owner: Id<normalized::entities::EntityRef>,
    pub declaration: Id<Occurrence>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="execution_runs",invariant_refs=run_invariants_refs,publication_refs=profile_checks_refs)]
pub struct ExecutionRun {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    pub requested: bool,
    pub executed: i64,
    pub refused: i64,
    pub bodied: i64,
    pub body_refused: i64,
}
macro_rules! source_inputs{($apply:ident)=>{$apply!{
 parameters:analysis::MethodParameters,
 catalogs:models::ModelCatalog,
 source_invocations:analysis::source_call::AnalysisInvocation,
 source_runs:source_call_records::SourceCallRun,
 source_headers:source_call_records::SourceCallHeader,
 source_members:source_call_records::HeaderMember,
 source_boundaries:source_call_records::SourceCallBoundary,
 source_results:analysis::source_call::AnalysisOutcome,
 source_calls:source_call_records::SourceInvocation,
 source_releases:source_call_records::SourceFrameRelease,source_arguments:source_call_records::SourceFrameArgument,
 source_outcomes:source_call_records::SourceCallOutcome,
 source_invocation_boundaries:source_call_records::InvocationBoundary,
}};}
macro_rules! data{($($field:ident:$ty:ty,)*)=>{
 pub struct EnrichedData{pub source:SourceCallData,pub application:super::model_application::ModelApplicationData,$(pub $field:Rows<$ty>,)*}
 impl EnrichedData{
  pub fn new(budget:&ResourceBudget)->Self{Self{source:SourceCallData::new(budget),application:super::model_application::ModelApplicationData::new(budget),$($field:Rows::new(budget),)*}}
  pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.source.visit(name,batch)?;self.application.visit(name,batch)?;$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*Ok(())}
  pub fn visit_input(&mut self,input:&ValidationInput,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{super::require_facts_view(input)?;self.visit(input.name(),batch)}
  pub fn consumed_inputs(profile:stages::Profile)->Vec<ValidationInput>{if profile==stages::Profile::Behavioral{return Self::inputs();}vec![ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"]),ValidationInput::of::<models::ModelCatalog>(&["id"])]}
  pub fn inputs()->Vec<ValidationInput>{let mut inputs=SourceCallData::inputs();inputs.extend(super::model_application::ModelApplicationData::validation_inputs());inputs.extend(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]);inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
 }
};}
source_inputs!(data);
impl EnrichedData {
    fn check_source(
        &self,
        expected: &source_call_records::SourceCallRecords,
    ) -> Result<(), ModelError> {
        let invalid = || {
            ModelError::Invalid(
                "enriched source predecessor differs from shared SourceCall replay".into(),
            )
        };
        if self.source_runs.get(expected.run.id()) != Some(&expected.run)
            || self.source_results.get(expected.outcome.id()) != Some(&expected.outcome)
        {
            return Err(invalid());
        }
        macro_rules! compare{($($earlier:ident:$field:ident,)*)=>{$(for row in expected.$field.iter(){if self.$earlier.get(row.id())!=Some(row){return Err(invalid());}})*};}
        compare! {source_headers:headers,source_members:members,source_boundaries:boundaries,source_calls:invocations,source_releases:releases,source_arguments:arguments,source_outcomes:call_outcomes,source_invocation_boundaries:invocation_boundaries,}
        Ok(())
    }
}
macro_rules! outputs{($apply:ident)=>{$apply!{
 executions:StatementExecution,outcomes:ExecutionOutcome,sources:ExecutionSource,members:ExecutionMember,entered:EnteredStatement,boundaries:ExecutionBoundary,
 modeled_calls:ModeledCallEvaluation,modeled_arguments:ModeledCallArgument,modeled_native:ModeledCallNative,fresh_calls:SourceExecutionInvocation,fresh_arguments:SourceExecutionArgument,captured_entries:CapturedEntryBinding,captured_values:CapturedValueSource,definition_evaluations:DefinitionEvaluation,definition_sources:DefinitionSource,definition_members:DefinitionMember,contexts:ContextExecution,context_items:ContextItem,context_sources:ContextSource,context_members:ContextMember,context_bindings:ContextEntryBinding,context_binding_sources:BindingSource,context_binding_members:BindingMember,
 bodies:BodyExecution,body_sources:BodySource,body_members:BodyMember,releases:BodyReleaseInput,body_boundaries:BodyBoundary,
}};}
macro_rules! records{($($field:ident:$ty:ty,)*)=>{
 pub struct ExecutionRecords{pub run:ExecutionRun,pub outcome:publication::AnalysisOutcome,$(pub $field:Rows<$ty>,)*}
 impl ExecutionRecords{fn new(invocation:Id<publication::AnalysisInvocation>,budget:&ResourceBudget)->Self{Self{run:ExecutionRun{invocation,requested:false,executed:0,refused:0,bodied:0,body_refused:0},outcome:publication::AnalysisOutcome{invocation,status:analysis::AnalysisStatus::Completed,reason:None},$($field:Rows::new(budget),)*}}}
};}
outputs!(records);
/// One actual Enriched invocation shares its finite-work allowance across owner grains.
/// The frame and budget binding prevents a caller from lending another attempt's meter.
pub struct EnrichedWork {
    invocation: publication::AnalysisInvocation,
    work: usize,
    catalog: Option<PreparedEnrichedCatalog>,
    charge: charged::StateCharge,
}
struct PreparedEnrichedCatalog {
    catalog: std::sync::Arc<models::Catalog>,
    _reservation: Box<dyn resources::Reservation>,
}
impl EnrichedWork {
    pub fn new(
        invocation: &publication::AnalysisInvocation,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "enriched-work");
        charge.grow(size_of::<Self>())?;
        Ok(Self {
            invocation: invocation.clone(),
            work: 0,
            catalog: None,
            charge,
        })
    }
    fn require(
        &self,
        invocation: &publication::AnalysisInvocation,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.invocation != *invocation
            || !self
                .charge
                .budget()
                .expect("bound Enriched work")
                .shares_pool(budget)
        {
            return Err(ModelError::Conflict("enriched work frame/budget"));
        }
        Ok(())
    }
    fn step(&mut self) -> Result<(), ModelError> {
        self.work = self
            .work
            .checked_add(1)
            .ok_or(ModelError::Conflict("enriched work overflow"))?;
        if self.work > super::enriched::ENRICHED_WORK_LIMIT {
            return Err(ModelError::Resource {
                owner: "enriched_finite_work",
                requested: 1,
                used: self.work - 1,
                limit: super::enriched::ENRICHED_WORK_LIMIT,
            });
        }
        Ok(())
    }
    fn catalog(
        &mut self,
        row: &models::ModelCatalog,
    ) -> Result<std::sync::Arc<models::Catalog>, ModelError> {
        if let Some(prepared) = &self.catalog {
            if prepared.catalog.declaration() != row {
                return Err(ModelError::Conflict("Enriched selected catalog changed"));
            }
            return Ok(prepared.catalog.clone());
        }
        let bytes = row
            .source
            .len()
            .checked_mul(32)
            .and_then(|n| n.checked_add(65536))
            .ok_or(ModelError::Conflict("Enriched catalog allowance"))?;
        let reservation = self
            .charge
            .budget()
            .expect("bound Enriched work")
            .reserve("enriched_selected_catalog", bytes)?;
        let catalog = std::sync::Arc::new(
            models::Catalog::parse(&row.source_name, &row.source).map_err(ModelError::Invalid)?,
        );
        if catalog.declaration() != row {
            return Err(ModelError::Conflict(
                "Enriched selected catalog is not its canonical declaration",
            ));
        }
        self.catalog = Some(PreparedEnrichedCatalog {
            catalog: catalog.clone(),
            _reservation: reservation,
        });
        Ok(catalog)
    }
}
pub fn enrich_all(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<ExecutionRecords, ModelError> {
    let verified = if profile == stages::Profile::Behavioral {
        Some(normalized::binding_normalization::prepare(
            &data.source.bindings,
            &data.source.output,
            budget,
        )?)
    } else {
        None
    };
    enrich_with_application(
        data,
        invocation,
        definition,
        profile,
        budget,
        verified.as_ref(),
        None,
        None,
        None,
        None,
        None,
    )
}
/// Consume one normalized application authority across SourceCall and modeled-call consumers.
pub fn enrich_all_prepared(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    verified: Option<&normalized::binding_normalization::VerifiedBindings>,
) -> Result<ExecutionRecords, ModelError> {
    enrich_with_application(
        data, invocation, definition, profile, budget, verified, None, None, None, None, None,
    )
}
/// Actual production consumes predecessor values retained by their own owners.
#[allow(
    clippy::too_many_arguments,
    reason = "Frame configuration and actual binding, evaluation and SourceCall owners are independent semantic inputs."
)]
pub fn enrich_all_produced(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    evaluations: Option<&super::production::ProducedEvaluations>,
    source_calls: Option<&source_call_records::ProducedSourceCalls>,
) -> Result<ExecutionRecords, ModelError> {
    if profile == stages::Profile::Behavioral && (evaluations.is_none() || source_calls.is_none()) {
        return Err(ModelError::Conflict(
            "requested Enriched predecessor owner absent",
        ));
    }
    enrich_with_application(
        data,
        invocation,
        definition,
        profile,
        budget,
        verified,
        evaluations,
        source_calls,
        None,
        None,
        None,
    )
}
/// Publish one actual owner against its complete scoped call/context/capture dependencies.
/// Intermediate operand results retain their native owner; final statement/body roots select
/// this owner explicitly so referenced declarations cannot become new publication roots.
#[allow(
    clippy::too_many_arguments,
    reason = "Selected owner, immutable frame configuration, predecessor authorities and shared work must remain explicit."
)]
pub fn enrich_owner_produced(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    evaluations: Option<&super::production::ProducedEvaluations>,
    source_calls: Option<&source_call_records::HydratedSourceCalls>,
    owner: Id<normalized::entities::EntityRef>,
    work: &mut EnrichedWork,
) -> Result<ExecutionRecords, ModelError> {
    work.require(invocation, budget)?;
    if profile == stages::Profile::Behavioral && (evaluations.is_none() || source_calls.is_none()) {
        return Err(ModelError::Conflict(
            "requested Enriched selected predecessor absent",
        ));
    }
    if profile == stages::Profile::Behavioral && data.source.evaluation.refs.get(owner).is_none() {
        return Err(ModelError::Conflict("selected Enriched owner absent"));
    }
    enrich_with_application(
        data,
        invocation,
        definition,
        profile,
        budget,
        verified,
        evaluations,
        None,
        source_calls,
        Some(owner),
        Some(work),
    )
}
/// Complete an actual empty selected publication domain without invoking any predecessor
/// producer. The compiler owns the full empty-root stream; visible primary roots refuse.
pub fn enrich_empty_produced(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    source_calls: Option<&source_call_records::HydratedSourceCalls>,
    work: &mut EnrichedWork,
) -> Result<ExecutionRecords, ModelError> {
    work.require(invocation, budget)?;
    enriched_configuration(data, invocation, definition)?;
    let mut output = ExecutionRecords::new(invocation.id(), budget);
    if profile != stages::Profile::Behavioral {
        output.outcome.status = analysis::AnalysisStatus::NotRequested;
        output.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok(output);
    }
    let mut parents = data
        .source_invocations
        .iter()
        .filter(|row| (row.input, row.context) == (invocation.input, invocation.context));
    let parent = parents.next().ok_or(ModelError::Conflict(
        "empty Enriched SourceCall frame absent",
    ))?;
    if parents.next().is_some() || data.source.definitions.get(parent.definition).is_none() {
        return Err(ModelError::Conflict(
            "empty Enriched SourceCall frame/definition",
        ));
    }
    let source_calls = source_calls.ok_or(ModelError::Conflict(
        "empty Enriched actual SourceCall owner absent",
    ))?;
    source_calls.require(parent, budget)?;
    if !source_calls.headers.is_empty()
        || !source_calls.calls.is_empty()
        || data
            .source_headers
            .iter()
            .any(|row| row.invocation == parent.id())
        || data
            .source_calls
            .iter()
            .any(|row| row.invocation == parent.id())
    {
        return Err(ModelError::Conflict(
            "empty Enriched source scope contains values",
        ));
    }
    let facts = &data.source.evaluation;
    let root_bytes = facts
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<SourceArtifact>() + row.heap_bytes() + 128)
        })
        .and_then(|n| n.checked_add(facts.uses.len().checked_mul(size_of::<ArtifactUse>())?))
        .and_then(|n| n.checked_mul(2))
        .ok_or(ModelError::Conflict("empty Enriched root allowance"))?;
    let _roots = budget.reserve("enriched-empty-roots", root_bytes)?;
    let roots = admission::analysis_roots(
        &facts.artifacts.iter().cloned().collect::<Vec<_>>(),
        &facts.uses.iter().cloned().collect::<Vec<_>>(),
    )?;
    let selected = |occurrence: &Occurrence| -> Result<bool, ModelError> {
        let source = facts
            .artifacts
            .get(occurrence.source)
            .ok_or(ModelError::Conflict("empty Enriched source absent"))?;
        Ok(source.input == invocation.input
            && roots.contains(&source.id())
            && admission::ArtifactClass::of(&source.path)
                == Some(admission::ArtifactClass::PythonSource))
    };
    for row in facts
        .occurrences
        .iter()
        .filter(|row| super::completion_production::is_statement(row.syntax_kind))
    {
        if selected(row)? {
            return Err(ModelError::Conflict(
                "empty Enriched scope contains statement",
            ));
        }
    }
    for row in facts.occurrences.iter().filter(|row| {
        row.syntax_kind == source::SyntaxKind::ExprName
            && facts
                .owners
                .iter()
                .any(|owner| owner.occurrence == row.id())
    }) {
        if selected(row)? {
            return Err(ModelError::Conflict(
                "empty Enriched scope contains context binding candidate",
            ));
        }
    }
    for attempt in data.source.output.attempts.iter() {
        let event = data
            .source
            .bindings
            .event_events
            .get(attempt.event)
            .ok_or(ModelError::Conflict("empty Enriched call event absent"))?;
        if event.context == invocation.context {
            let site = facts
                .occurrences
                .get(event.site)
                .ok_or(ModelError::Conflict("empty Enriched call site absent"))?;
            if selected(site)? {
                return Err(ModelError::Conflict(
                    "empty Enriched scope contains modeled call candidate",
                ));
            }
        }
    }
    for callable in facts.callables.iter() {
        if let normalized::entities::CallableEntity::Source { declaration, .. } = callable {
            let declaration = facts
                .occurrences
                .get(*declaration)
                .ok_or(ModelError::Conflict(
                    "empty Enriched callable declaration absent",
                ))?;
            if selected(declaration)? {
                return Err(ModelError::Conflict(
                    "empty Enriched scope contains callable",
                ));
            }
        }
    }
    output.run.requested = true;
    Ok(output)
}
/// The ordinary final statement kernel refuses both absent and ambiguous owners with the
/// same explicit MissingEvidence boundary. Preserve that actual root even with no owner grain.
pub fn enrich_unowned_statement(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    statement: Id<Occurrence>,
    work: &mut EnrichedWork,
) -> Result<ExecutionRecords, ModelError> {
    work.require(invocation, budget)?;
    enriched_configuration(data, invocation, definition)?;
    let mut output = ExecutionRecords::new(invocation.id(), budget);
    if profile != stages::Profile::Behavioral {
        output.outcome.status = analysis::AnalysisStatus::NotRequested;
        output.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok(output);
    }
    let mut parents = data
        .source_invocations
        .iter()
        .filter(|row| (row.input, row.context) == (invocation.input, invocation.context));
    let parent = parents.next().ok_or(ModelError::Conflict(
        "unowned Enriched SourceCall frame absent",
    ))?;
    if parents.next().is_some() || data.source.definitions.get(parent.definition).is_none() {
        return Err(ModelError::Conflict(
            "unowned Enriched SourceCall frame/definition",
        ));
    }
    let facts = &data.source.evaluation;
    let row = facts
        .occurrences
        .get(statement)
        .ok_or(ModelError::Conflict(
            "selected unowned Enriched statement absent",
        ))?;
    let artifact = facts.artifacts.get(row.source).ok_or(ModelError::Conflict(
        "selected unowned Enriched artifact absent",
    ))?;
    if !super::completion_production::is_statement(row.syntax_kind)
        || artifact.input != invocation.input
        || admission::ArtifactClass::of(&artifact.path)
            != Some(admission::ArtifactClass::PythonSource)
        || facts
            .owners
            .iter()
            .filter(|owner| owner.occurrence == statement)
            .count()
            == 1
    {
        return Err(ModelError::Conflict(
            "selected unowned Enriched root is inapplicable",
        ));
    }
    let size = facts
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<SourceArtifact>() + row.heap_bytes() + 128)
        })
        .and_then(|n| n.checked_add(facts.uses.len().checked_mul(size_of::<ArtifactUse>())?))
        .and_then(|n| n.checked_mul(2))
        .ok_or(ModelError::Conflict("selected Enriched root allowance"))?;
    let _roots = budget.reserve("enriched-unowned-roots", size)?;
    if !admission::analysis_roots(
        &facts.artifacts.iter().cloned().collect::<Vec<_>>(),
        &facts.uses.iter().cloned().collect::<Vec<_>>(),
    )?
    .contains(&row.source)
    {
        return Err(ModelError::Conflict(
            "selected unowned Enriched source is not an analysis root",
        ));
    }
    work.step()?;
    output.run.requested = true;
    output.run.refused = 1;
    output.boundaries.insert(ExecutionBoundary {
        invocation: invocation.id(),
        statement,
        owner: None,
        reason: obligation::ObligationKind::MissingEvidence,
    })?;
    output.outcome.status = analysis::AnalysisStatus::Partial;
    output.outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
    Ok(output)
}
fn enriched_configuration<'a>(
    data: &'a EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
) -> Result<&'a models::ModelCatalog, ModelError> {
    let invalid = |message: &str| ModelError::Invalid(message.into());
    if !super::configuration::enriched_kernel(definition)
        || invocation.definition != definition.id()
        || invocation.subject.is_some()
    {
        return Err(invalid(
            "enriched execution requires its whole-frame definition",
        ));
    }
    let parameters = data
        .parameters
        .get(definition.parameters)
        .ok_or_else(|| invalid("enriched parameters absent"))?;
    let catalog = parameters
        .model_catalog
        .and_then(|id| data.catalogs.get(id))
        .ok_or_else(|| invalid("enriched selected catalog absent"))?;
    if super::configuration::enriched_execution(catalog.id())
        != (parameters.clone(), definition.clone())
    {
        return Err(invalid(
            "enriched selected configuration differs from canonical factory",
        ));
    }
    Ok(catalog)
}
#[allow(
    clippy::too_many_arguments,
    reason = "Diagnostic and selected kernels share explicit configuration, predecessor authorities, owner selection and work."
)]
fn enrich_with_application(
    data: &EnrichedData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    evaluations: Option<&super::production::ProducedEvaluations>,
    source_calls: Option<&source_call_records::ProducedSourceCalls>,
    hydrated: Option<&source_call_records::HydratedSourceCalls>,
    selected_owner: Option<Id<normalized::entities::EntityRef>>,
    shared_work: Option<&mut EnrichedWork>,
) -> Result<ExecutionRecords, ModelError> {
    let invalid = |message: &str| ModelError::Invalid(message.into());
    let catalog = enriched_configuration(data, invocation, definition)?;
    let mut output = ExecutionRecords::new(invocation.id(), budget);
    if profile != stages::Profile::Behavioral {
        output.outcome.status = analysis::AnalysisStatus::NotRequested;
        output.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok(output);
    }
    output.run.requested = true;
    let mut parents = data
        .source_invocations
        .iter()
        .filter(|row| (row.input, row.context) == (invocation.input, invocation.context));
    let source = parents
        .next()
        .ok_or_else(|| invalid("enriched SourceCall frame absent"))?;
    if parents.next().is_some() {
        return Err(invalid("enriched SourceCall frame ambiguous"));
    }
    let source_definition = data
        .source
        .definitions
        .get(source.definition)
        .ok_or_else(|| invalid("enriched SourceCall definition absent"))?;
    let facts = &data.source.evaluation;
    let bytes = facts
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<SourceArtifact>() + row.heap_bytes() + 128)
        })
        .and_then(|n| n.checked_add(facts.uses.len().checked_mul(size_of::<ArtifactUse>())?))
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| invalid("enriched root allowance overflow"))?;
    let _roots = budget.reserve("enriched_source_roots", bytes)?;
    let roots = admission::analysis_roots(
        &facts.artifacts.iter().cloned().collect::<Vec<_>>(),
        &facts.uses.iter().cloned().collect::<Vec<_>>(),
    )?;
    let selected = |source| {
        roots.contains(&source)
            && facts.artifacts.get(source).is_some_and(|artifact| {
                artifact.input == invocation.input
                    && admission::ArtifactClass::of(&artifact.path)
                        == Some(admission::ArtifactClass::PythonSource)
            })
    };
    let mut local_work = if shared_work.is_none() {
        Some(EnrichedWork::new(invocation, budget)?)
    } else {
        None
    };
    let work = shared_work.unwrap_or_else(|| local_work.as_mut().expect("ordinary frame work"));
    work.require(invocation, budget)?;
    let catalog = work.catalog(catalog)?;
    let construction = if evaluations.is_some() {
        Some(super::model_construction::PreparedConstructionInputs::new(
            &data.application,
            verified.ok_or_else(|| invalid("requested Enriched application authority absent"))?,
            budget,
        )?)
    } else {
        None
    };
    let visit = |frame: &mut super::enriched::EnrichedFrame<'_>| {
        let mut step = || -> Result<(), ModelError> { work.step() };
        let verified =
            verified.ok_or_else(|| invalid("requested Enriched application authority absent"))?;
        for attempt in data.source.output.attempts.iter() {
            step()?;
            let Some(shape) = verified.shape(attempt.id()) else {
                continue;
            };
            if (shape.input(), shape.context()) != (invocation.input, invocation.context) {
                continue;
            }
            let Some(bound) = verified.bound(attempt.id()) else {
                continue;
            };
            if !selected(
                facts
                    .occurrences
                    .get(bound.bound().site())
                    .ok_or_else(|| invalid("modeled call site absent"))?
                    .source,
            ) {
                continue;
            }
            let application =
                super::model_application::CheckedModelApplication::derive_with_inputs(
                    &catalog,
                    &data.application,
                    bound,
                    shape,
                    verified.effective_invocation(attempt.id()),
                    budget,
                    construction.as_ref(),
                )?;
            let Ok(application) = application else {
                continue;
            };
            if frame.has_call(shape.event()) {
                continue;
            }
            let checked = CheckedModeledEvaluation::derive_with_values(
                facts,
                &application,
                &data.source.completed,
                invocation,
                definition,
                budget,
                evaluations,
            )?;
            let Ok(checked) = checked else { continue };
            output.modeled_calls.insert(checked.record().clone())?;
            for row in checked.arguments().iter() {
                output.modeled_arguments.insert(row.clone())?;
            }
            for row in checked.native().iter() {
                output.modeled_native.insert(row.clone())?;
            }
            frame.push_modeled(checked)?;
        }
        for occurrence in facts.occurrences.iter().filter(|row| {
            row.syntax_kind == source::SyntaxKind::StmtFunctionDef && selected(row.source)
        }) {
            step()?;
            for owner in facts
                .owners
                .iter()
                .filter(|owner| owner.occurrence == occurrence.id())
            {
                let request = super::completion::CompletionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner: owner.entity,
                    statement: occurrence.id(),
                };
                if let Ok(proof) = CheckedDefinition::derive_with_values(
                    &data.source,
                    request,
                    invocation,
                    budget,
                    evaluations,
                )? {
                    output
                        .definition_evaluations
                        .insert(proof.record().clone())?;
                    for row in proof.sources().iter() {
                        output.definition_sources.insert(row.clone())?;
                    }
                    for row in proof.members().iter() {
                        output.definition_members.insert(row.clone())?;
                    }
                    frame.push_definition(proof)?;
                }
            }
        }
        for occurrence in facts
            .occurrences
            .iter()
            .filter(|row| row.syntax_kind == source::SyntaxKind::ExprName && selected(row.source))
        {
            step()?;
            for owner in facts
                .owners
                .iter()
                .filter(|owner| owner.occurrence == occurrence.id())
            {
                let request = super::evaluation::ExpressionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner: owner.entity,
                    expression: occurrence.id(),
                };
                if let Ok(proof) = CheckedContextBinding::derive_with_values(
                    &catalog,
                    &data.application,
                    &data.source,
                    invocation,
                    request,
                    budget,
                    evaluations,
                    construction.as_ref(),
                )? {
                    output.context_bindings.insert(proof.record().clone())?;
                    for row in proof.sources().iter() {
                        output.context_binding_sources.insert(row.clone())?;
                    }
                    for row in proof.members().iter() {
                        output.context_binding_members.insert(row.clone())?;
                    }
                    frame.push_binding(proof)?;
                }
            }
        }
        // Every progressing round adds a context for a previously unseen selected with
        // statement. Contexts are append-only and the next round skips those statements.
        // Thus N selected statements admit at most N progressing rounds, followed by one
        // stable round. The independent finite-work meter still refuses exhausted work.
        let mut context_round_bound = 1usize;
        for occurrence in facts.occurrences.iter() {
            step()?;
            if occurrence.syntax_kind == source::SyntaxKind::StmtWith && selected(occurrence.source)
            {
                context_round_bound = context_round_bound
                    .checked_add(1)
                    .ok_or_else(|| invalid("enriched context round bound overflow"))?;
            }
        }
        for _ in 0..context_round_bound {
            let mut progress = false;
            for occurrence in facts.occurrences.iter().filter(|row| {
                row.syntax_kind == source::SyntaxKind::StmtWith && selected(row.source)
            }) {
                step()?;
                if frame
                    .contexts()
                    .iter()
                    .any(|proof| proof.record().statement == occurrence.id())
                {
                    continue;
                }
                for owner in facts
                    .owners
                    .iter()
                    .filter(|owner| owner.occurrence == occurrence.id())
                {
                    let request = super::completion::CompletionRequest {
                        input: invocation.input,
                        context: invocation.context,
                        owner: owner.entity,
                        statement: occurrence.id(),
                    };
                    let mut proofs = Vec::new();
                    let mut charge = charged::StateCharge::new(budget, "context_body_tokens");
                    for placement in facts.placements.iter().filter(|p| {
                        p.parent == Some(occurrence.id()) && p.field == lexical::SyntaxField::Body
                    }) {
                        step()?;
                        if let Ok(proof) = frame.complete(super::completion::CompletionRequest {
                            statement: placement.occurrence,
                            ..request
                        })? {
                            charge.grow(size_of::<super::completion::CheckedCompletion>() * 2)?;
                            proofs.push(proof);
                        }
                    }
                    let mut body = Vec::new();
                    charge.grow(
                        proofs.len()
                            * size_of::<(
                                &super::completion::CheckedCompletion,
                                Id<StatementExecution>,
                            )>()
                            * 2,
                    )?;
                    for proof in &proofs {
                        let row = emit_statement(frame, proof, invocation, definition, budget)?;
                        body.push((proof, row.execution.id()));
                    }
                    if let Ok(proof) = CheckedContextExecution::derive_with_values(
                        &catalog,
                        &data.application,
                        &data.source,
                        invocation,
                        request,
                        &body,
                        budget,
                        evaluations,
                        construction.as_ref(),
                    )? {
                        for body in &proofs {
                            insert_statement(
                                &mut output,
                                emit_statement(frame, body, invocation, definition, budget)?,
                            )?;
                        }
                        output
                            .outcomes
                            .insert(ExecutionOutcome::from(proof.outcome()))?;
                        for outcome in proof.exit_outcomes() {
                            output.outcomes.insert(outcome)?;
                        }
                        output.contexts.insert(proof.record().clone())?;
                        for row in proof.items().iter() {
                            output.context_items.insert(row.clone())?;
                        }
                        for row in proof.sources().iter() {
                            output.context_sources.insert(row.clone())?;
                        }
                        for row in proof.members().iter() {
                            output.context_members.insert(row.clone())?;
                        }
                        frame.push_context(proof)?;
                        progress = true;
                    }
                }
            }
            if !progress {
                break;
            }
        }
        // Fresh source bodies form finite immutable proof occurrences. Only successful prior
        // occurrences may become call premises; recursive/self-captured bodies remain refused.
        for _ in 0..frame.headers().len() {
            let mut progress = false;
            for index in 0..frame.headers().len() {
                step()?;
                let (header, header_row) = frame.headers()[index];
                if frame.has_call(header_row.event) {
                    continue;
                }
                let mut captures = Vec::new();
                let _capture_allowance = budget.reserve(
                    "captured-active-frame",
                    header.captures().len() * size_of::<CheckedCapturedEntry>() * 2,
                )?;
                for origin in header.captures() {
                    let proof = CheckedCapturedEntry::activate(
                        origin, header, header_row, invocation, facts,
                    )?;
                    if output.captured_entries.get(proof.row.id()).is_none() {
                        frame.push_capture(&proof)?;
                    }
                    captures.push(proof);
                }
                let mut proofs = Vec::new();
                let mut charge =
                    charged::StateCharge::new(budget, "enriched_fresh_body_completions");
                for occurrence in facts.occurrences.iter().filter(|row| {
                    super::completion_production::is_statement(row.syntax_kind)
                        && facts.owners.iter().any(|owner| {
                            owner.occurrence == row.id() && owner.entity == header.callee()
                        })
                }) {
                    step()?;
                    if let Ok(proof) = frame.complete(super::completion::CompletionRequest {
                        input: invocation.input,
                        context: invocation.context,
                        owner: header.callee(),
                        statement: occurrence.id(),
                    })? {
                        charge.grow(size_of::<super::completion::CheckedCompletion>() * 2)?;
                        proofs.push(proof);
                    }
                }
                let _refs = budget.reserve(
                    "enriched_fresh_statement_refs",
                    proofs.len() * size_of::<&super::completion::CheckedCompletion>() * 2,
                )?;
                let refs = proofs.iter().collect::<Vec<_>>();
                let body = super::body::complete_body(
                    facts,
                    super::body::SourceBodyRequest {
                        input: invocation.input,
                        context: invocation.context,
                        callee: header.callee(),
                    },
                    &refs,
                    budget,
                )?;
                let Ok(body) = body else { continue };
                let call = super::source_invocation::CheckedSourceInvocation::derive_with_values(
                    facts,
                    header,
                    &body,
                    &data.source.completed,
                    &captures,
                    budget,
                    evaluations,
                )?;
                let Ok(call) = call else { continue };
                for capture in captures {
                    output.captured_values.insert(capture.source)?;
                    output.captured_entries.insert(capture.row)?;
                }
                for proof in &proofs {
                    insert_statement(
                        &mut output,
                        emit_statement(frame, proof, invocation, definition, budget)?,
                    )?;
                }
                let body_records = emit_body(&body, invocation, &output.executions, budget)?;
                let body_id = body_records.body.id();
                insert_body(&mut output, body_records)?;
                let outcome = match call.outcome() {
                    super::source_invocation::InvocationOutcome::Normal => ExecutionOutcome::Normal,
                    super::source_invocation::InvocationOutcome::Raised { site, exception } => {
                        ExecutionOutcome::Raise { site, exception }
                    }
                };
                output.outcomes.insert(outcome.clone())?;
                let mut digest = KeySink::new("source-frame-arguments");
                for argument in call.arguments() {
                    argument.formal.encode(&mut digest);
                    argument.actual.encode(&mut digest);
                    argument.evaluation.encode(&mut digest);
                }
                let row = SourceExecutionInvocation {
                    invocation: invocation.id(),
                    header: header_row.id(),
                    body: body_id,
                    event: call.event(),
                    qualification: call.qualification(),
                    outcome: outcome.id(),
                    status: call.status(),
                    arguments: digest.finish(),
                    release: call.release(),
                };
                for (ordinal, argument) in call.arguments().iter().enumerate() {
                    output.fresh_arguments.insert(SourceExecutionArgument {
                        call: row.id(),
                        ordinal: ordinal as i64,
                        formal: argument.formal,
                        actual: argument.actual,
                        evaluation: argument.evaluation,
                    })?;
                }
                frame.push_fresh(&call, &row)?;
                output.fresh_calls.insert(row)?;
                progress = true;
            }
            if !progress {
                break;
            }
        }
        let mut statements = Vec::new();
        let mut charge = charged::StateCharge::new(budget, "enriched_retained_completions");
        for row in facts.occurrences.iter().filter(|row| {
            super::completion_production::is_statement(row.syntax_kind)
                && selected(row.source)
                && selected_owner.is_none_or(|owner| {
                    facts.owners.iter().any(|membership| {
                        membership.occurrence == row.id() && membership.entity == owner
                    })
                })
        }) {
            step()?;
            let mut owners = facts
                .owners
                .iter()
                .filter(|owner| owner.occurrence == row.id());
            let first = owners.next();
            let owner = if owners.next().is_none() {
                first.map(|owner| owner.entity)
            } else {
                None
            };
            let result = if let Some(owner) = owner {
                frame.complete(super::completion::CompletionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner,
                    statement: row.id(),
                })?
            } else {
                Err(obligation::ObligationKind::MissingEvidence)
            };
            match result {
                Err(reason) => {
                    output.boundaries.insert(ExecutionBoundary {
                        invocation: invocation.id(),
                        statement: row.id(),
                        owner,
                        reason,
                    })?;
                }
                Ok(proof) => {
                    let records = emit_statement(frame, &proof, invocation, definition, budget)?;
                    output.executions.insert(records.execution)?;
                    output.outcomes.insert(records.outcome)?;
                    for row in records.sources {
                        output.sources.insert(row)?;
                    }
                    for row in records.members {
                        output.members.insert(row)?;
                    }
                    for row in records.entered {
                        output.entered.insert(row)?;
                    }
                    charge.grow(size_of::<super::completion::CheckedCompletion>() * 2)?;
                    statements.push(proof);
                }
            }
        }
        let _scratch = budget.reserve(
            "enriched_body_statement_refs",
            statements
                .len()
                .checked_mul(size_of::<&super::completion::CheckedCompletion>() * 2)
                .ok_or_else(|| invalid("enriched body allowance overflow"))?,
        )?;
        let refs = statements.iter().collect::<Vec<_>>();
        for callable in facts.callables.iter() {
            step()?;
            let normalized::entities::CallableEntity::Source { declaration, .. } = callable else {
                continue;
            };
            let occurrence = facts
                .occurrences
                .get(*declaration)
                .ok_or_else(|| invalid("enriched body declaration absent"))?;
            if !selected(occurrence.source) {
                continue;
            }
            let owner = normalized::entities::EntityRef::Callable {
                callable: callable.id(),
            }
            .id();
            if selected_owner.is_some_and(|selected| selected != owner) {
                continue;
            }
            match super::body::complete_body(
                facts,
                super::body::SourceBodyRequest {
                    input: invocation.input,
                    context: invocation.context,
                    callee: owner,
                },
                &refs,
                budget,
            )? {
                Err(reason) => {
                    output.body_boundaries.insert(BodyBoundary {
                        invocation: invocation.id(),
                        owner,
                        declaration: *declaration,
                        reason,
                    })?;
                }
                Ok(proof) => {
                    let records = emit_body(&proof, invocation, &output.executions, budget)?;
                    output.bodies.insert(records.body)?;
                    output.outcomes.insert(records.outcome)?;
                    for row in records.sources {
                        output.body_sources.insert(row)?;
                    }
                    for row in records.members {
                        output.body_members.insert(row)?;
                    }
                    for row in records.releases {
                        output.releases.insert(row)?;
                    }
                }
            }
        }
        Ok(())
    };
    if let (Some(evaluations), Some(hydrated)) = (evaluations, hydrated) {
        hydrated.require(source, budget)?;
        for (_, row) in &hydrated.headers {
            if data.source_headers.get(row.id()) != Some(row) {
                return Err(invalid("actual Enriched source header changed"));
            }
        }
        for (_, row) in &hydrated.calls {
            if data.source_calls.get(row.id()) != Some(row) {
                return Err(invalid("actual Enriched source invocation changed"));
            }
        }
        for row in data
            .source_headers
            .iter()
            .filter(|row| row.invocation == source.id())
        {
            if !hydrated.headers.iter().any(|(_, actual)| actual == row) {
                return Err(invalid("actual Enriched source header scope omitted"));
            }
        }
        for row in data
            .source_calls
            .iter()
            .filter(|row| row.invocation == source.id())
        {
            if !hydrated.calls.iter().any(|(_, actual)| actual == row) {
                return Err(invalid("actual Enriched source invocation scope omitted"));
            }
        }
        super::enriched::with_frame_hydrated(
            &data.source,
            source,
            budget,
            hydrated,
            evaluations,
            visit,
        )?;
    } else if let (Some(evaluations), Some(source_calls)) = (evaluations, source_calls) {
        super::enriched::with_frame_produced(
            &data.source,
            source,
            budget,
            source_calls,
            evaluations,
            visit,
        )?;
    } else {
        let (_, expected) = super::enriched::with_frame_prepared(
            &data.source,
            source,
            source_definition,
            budget,
            verified.ok_or_else(|| invalid("requested Enriched application authority absent"))?,
            visit,
        )?;
        data.check_source(&expected)?;
    }

    output.run.executed = output
        .executions
        .len()
        .try_into()
        .map_err(|_| invalid("enriched count overflow"))?;
    output.run.refused = output
        .boundaries
        .len()
        .try_into()
        .map_err(|_| invalid("enriched count overflow"))?;
    output.run.bodied = output
        .bodies
        .len()
        .try_into()
        .map_err(|_| invalid("enriched count overflow"))?;
    output.run.body_refused = output
        .body_boundaries
        .len()
        .try_into()
        .map_err(|_| invalid("enriched count overflow"))?;
    if !output.boundaries.is_empty() || !output.body_boundaries.is_empty() {
        output.outcome.status = analysis::AnalysisStatus::Partial;
        output.outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
    }
    Ok(output)
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ExecutionRun>(),
        Relation::of::<ExecutionBoundary>(),
        Relation::of::<BodyBoundary>(),
    ]
}
fn invariant_inputs() -> Vec<ValidationInput> {
    let mut inputs = EnrichedData::inputs();
    inputs.extend([
        ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<ExecutionRun>(&["id"]),
        ValidationInput::of::<publication::AnalysisOutcome>(&["id"]),
    ]);
    macro_rules! append{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    outputs!(append);
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    inputs
}
pub(crate) fn run_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "enriched_execution_inventory",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(ExecutionCheck::new(budget))),
    }]
}
pub(crate) fn binding_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "context_entry_binding_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|b| Box::new(ExecutionCheck::new(b))),
    }]
}
pub(crate) fn context_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "context_execution_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|b| Box::new(ExecutionCheck::new(b))),
    }]
}
pub(crate) fn definition_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "definition_evaluation_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|b| Box::new(ExecutionCheck::new(b))),
    }]
}
pub(crate) fn modeled_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "enriched_modeled_call_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(ExecutionCheck::new(budget))),
    }]
}
pub(crate) fn statement_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "enriched_statement_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(ExecutionCheck::new(budget))),
    }]
}
macro_rules! checker{($($field:ident:$ty:ty,)*)=>{
 struct ExecutionCheck{data:EnrichedData,invocations:Rows<publication::AnalysisInvocation>,runs:Rows<ExecutionRun>,results:Rows<publication::AnalysisOutcome>,$($field:Rows<$ty>,)*budget:ResourceBudget}
 impl ExecutionCheck{fn new(budget:&ResourceBudget)->Self{Self{data:EnrichedData::new(budget),invocations:Rows::new(budget),runs:Rows::new(budget),results:Rows::new(budget),$($field:Rows::new(budget),)*budget:budget.clone()}}}
 impl InvariantCheck for ExecutionCheck{
  fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.data.visit(name,batch)?;if name==publication::AnalysisInvocation::NAME{self.invocations.decode(batch)?;}if name==ExecutionRun::NAME{self.runs.decode(batch)?;}if name==publication::AnalysisOutcome::NAME{self.results.decode(batch)?;}$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*Ok(())}
  fn visit_input(&mut self,input:&ValidationInput,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{if stages::is_vocabulary(input.name()){self.data.visit_input(input,batch)}else{self.visit(input.name(),batch)}}
  fn finish(self:Box<Self>)->Result<(),ModelError>{
   let invalid=|message:&str|ModelError::Invalid(message.into());let mut frames=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(&self.budget,"enriched_frame_inventory");for row in self.data.source_invocations.iter(){frames.insert(&mut charge,(row.input,row.context))?;}if self.invocations.len()!=frames.len()||frames.iter().any(|frame|self.invocations.iter().filter(|row|(row.input,row.context)==*frame).count()!=1){return Err(invalid("enriched omitted/duplicated SourceCall frame"));}
   let mut runs=Rows::new(&self.budget);let mut results=Rows::new(&self.budget);$(let mut $field=Rows::new(&self.budget);)*
   for invocation in self.invocations.iter(){let mut parents=Rows::new(&self.budget);for source in self.data.source_invocations.iter().filter(|row|(row.input,row.context)==(invocation.input,invocation.context)){parents.insert(publication::InvocationSource::SourceCallAnalysis{invocation:source.id()})?;}let mut key=KeySink::new("analysis-invocation-inputs");for row in parents.iter(){row.id().encode(&mut key);}if invocation.inputs!=key.finish(){return Err(invalid("enriched changes exact SourceCall parents"));}
    let run=self.runs.iter().find(|row|row.invocation==invocation.id()).ok_or_else(||invalid("enriched run absent"))?;let definition=self.data.source.definitions.get(invocation.definition).ok_or_else(||invalid("enriched definition absent"))?;let expected=enrich_all(&self.data,invocation,definition,if run.requested{stages::Profile::Behavioral}else{stages::Profile::Catalog},&self.budget)?;runs.insert(expected.run)?;results.insert(expected.outcome)?;$(for row in expected.$field.iter(){$field.insert(row.clone())?;})*
   }
   if !self.runs.same(&runs)||!self.results.same(&results)$(||!self.$field.same(&$field))*{return Err(invalid("enriched inventory differs from exact shared ordered execution"));}Ok(())
  }
 }
};}
outputs!(checker);
pub(crate) fn profile_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "enriched_execution_profile",
        inputs: vec![
            ValidationInput::of::<ExecutionRun>(&["id"]),
            ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ProfileCheck {
                runs: Rows::new(budget),
                invocations: Rows::new(budget),
            })
        }),
    }]
}
struct ProfileCheck {
    runs: Rows<ExecutionRun>,
    invocations: Rows<publication::AnalysisInvocation>,
}
impl PublicationCheck for ProfileCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == ExecutionRun::NAME {
            self.runs.decode(batch)?;
        } else if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        } else {
            return Err(ModelError::Invalid(
                "undeclared enriched profile input".into(),
            ));
        }
        Ok(())
    }
    fn finish(
        self: Box<Self>,
        _sources: &[crate::domain::analysis::sources::SourceSnapshot],
        profile: stages::Profile,
    ) -> Result<(), ModelError> {
        if self.runs.len() != self.invocations.len()
            || self.runs.iter().any(|row| {
                self.invocations.get(row.invocation).is_none()
                    || row.requested != (profile == stages::Profile::Behavioral)
            })
        {
            return Err(ModelError::Invalid(
                "enriched request differs from actual publication profile".into(),
            ));
        }
        Ok(())
    }
}

pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    if !super::configuration::enriched_kernel(definition) {
        return Err(ModelError::Invalid(
            "enriched execution stage definition is unbound".into(),
        ));
    }
    let mut outputs = publication::publication_relations();
    outputs.extend(super::enriched_records::relations());
    outputs.extend(super::modeled_call::relations());
    outputs.extend(super::definition::relations());
    outputs.extend(super::context_execution::relations());
    outputs.extend(super::context_binding::relations());
    outputs.extend(relations());
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let mut initial = if profile == Profile::Behavioral {
        invariant_inputs()
    } else {
        vec![
            ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<models::ModelCatalog>(&["id"]),
        ]
    };
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
    let mut key = KeySink::new("enriched-execution-definition");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "enrich_execution",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("enriched_production.rs")),
        configuration: key.finish(),
    })
}

fn insert_statement(
    output: &mut ExecutionRecords,
    records: StatementRecords,
) -> Result<(), ModelError> {
    output.executions.insert(records.execution)?;
    output.outcomes.insert(records.outcome)?;
    for row in records.sources {
        output.sources.insert(row)?;
    }
    for row in records.members {
        output.members.insert(row)?;
    }
    for row in records.entered {
        output.entered.insert(row)?;
    }
    Ok(())
}
fn insert_body(output: &mut ExecutionRecords, records: BodyRecords) -> Result<(), ModelError> {
    output.bodies.insert(records.body)?;
    output.outcomes.insert(records.outcome)?;
    for row in records.sources {
        output.body_sources.insert(row)?;
    }
    for row in records.members {
        output.body_members.insert(row)?;
    }
    for row in records.releases {
        output.releases.insert(row)?;
    }
    Ok(())
}

pub(crate) fn run_invariants_refs() -> Vec<&'static str> {
    vec!["enriched_execution_inventory"]
}
pub(crate) fn binding_invariants_refs() -> Vec<&'static str> {
    vec!["context_entry_binding_replay", "enriched_binding_fidelity"]
}
pub(crate) fn context_invariants_refs() -> Vec<&'static str> {
    vec!["context_execution_replay", "enriched_context_fidelity"]
}
pub(crate) fn definition_invariants_refs() -> Vec<&'static str> {
    vec![
        "definition_evaluation_replay",
        "enriched_definition_fidelity",
    ]
}
pub(crate) fn modeled_invariants_refs() -> Vec<&'static str> {
    vec!["enriched_modeled_call_replay", "enriched_modeled_fidelity"]
}
pub(crate) fn statement_invariants_refs() -> Vec<&'static str> {
    vec!["enriched_statement_replay", "enriched_statement_fidelity"]
}
pub(crate) fn profile_checks_refs() -> Vec<&'static str> {
    vec!["enriched_execution_profile"]
}

#[cfg(test)]
mod selected_enriched_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn unowned_statement_preserves_boundary_without_replaying_predecessor_and_shares_work() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let catalog =
            models::Catalog::parse("empty.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n")
                .unwrap();
        let (parameters, definition) =
            super::super::configuration::enriched_execution(catalog.declaration().id());
        let (frame, _) =
            publication::AnalysisInvocation::new(id(1), id(2), definition.id(), None, []);
        let source_definition = super::super::configuration::source_calls().1;
        let (parent, _) = analysis::source_call::AnalysisInvocation::new(
            frame.input,
            frame.context,
            source_definition.id(),
            None,
            [],
        );
        let mut data = EnrichedData::new(&budget);
        data.parameters.insert(parameters).unwrap();
        data.catalogs.insert(catalog.declaration().clone()).unwrap();
        data.source_invocations.insert(parent).unwrap();
        data.source.definitions.insert(source_definition).unwrap();
        let artifact = SourceArtifact::from_bytes(frame.input, "unit.py".into(), b"pass").unwrap();
        let statement = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 4,
            syntax_kind: source::SyntaxKind::StmtPass,
            role: source::OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        data.source
            .evaluation
            .uses
            .insert(ArtifactUse {
                artifact: artifact.id(),
                input: frame.input,
                role: input::SourceRole::Release,
            })
            .unwrap();
        data.source.evaluation.artifacts.insert(artifact).unwrap();
        data.source
            .evaluation
            .occurrences
            .insert(statement.clone())
            .unwrap();
        // No Base or SourceCall values are available. An owner-absence boundary needs neither.
        let mut work = EnrichedWork::new(&frame, &budget).unwrap();
        work.work = super::super::enriched::ENRICHED_WORK_LIMIT - 1;
        let output = enrich_unowned_statement(
            &data,
            &frame,
            &definition,
            stages::Profile::Behavioral,
            &budget,
            statement.id(),
            &mut work,
        )
        .unwrap();
        assert_eq!(
            output.boundaries.iter().next().unwrap(),
            &ExecutionBoundary {
                invocation: frame.id(),
                statement: statement.id(),
                owner: None,
                reason: obligation::ObligationKind::MissingEvidence
            }
        );
        assert_eq!(output.outcome.status, analysis::AnalysisStatus::Partial);
        assert_eq!(output.run.refused, 1);
        assert!(matches!(
            enrich_unowned_statement(
                &data,
                &frame,
                &definition,
                stages::Profile::Behavioral,
                &budget,
                statement.id(),
                &mut work
            ),
            Err(ModelError::Resource {
                owner: "enriched_finite_work",
                ..
            })
        ));
        let foreign = ResourceBudget::fixed(8 << 20).unwrap();
        assert!(work.require(&frame, &foreign).is_err());
        let mut changed = frame.clone();
        changed.context = id(9);
        assert!(work.require(&changed, &budget).is_err());
        data.source
            .evaluation
            .owners
            .insert(normalized::entities::OccurrenceOwnership {
                occurrence: statement.id(),
                owner: statement.id(),
                entity: id(10),
            })
            .unwrap();
        let mut fresh = EnrichedWork::new(&frame, &budget).unwrap();
        assert!(
            enrich_unowned_statement(
                &data,
                &frame,
                &definition,
                stages::Profile::Behavioral,
                &budget,
                statement.id(),
                &mut fresh
            )
            .is_err()
        );
    }
    #[test]
    fn catalog_owner_scope_does_not_request_missing_value_or_owner_premises() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let catalog =
            models::Catalog::parse("empty.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n")
                .unwrap();
        let (parameters, definition) =
            super::super::configuration::enriched_execution(catalog.declaration().id());
        let (frame, _) =
            publication::AnalysisInvocation::new(id(1), id(2), definition.id(), None, []);
        let mut data = EnrichedData::new(&budget);
        data.parameters.insert(parameters).unwrap();
        data.catalogs.insert(catalog.declaration().clone()).unwrap();
        let mut work = EnrichedWork::new(&frame, &budget).unwrap();
        let output = enrich_owner_produced(
            &data,
            &frame,
            &definition,
            stages::Profile::Catalog,
            &budget,
            None,
            None,
            None,
            id(3),
            &mut work,
        )
        .unwrap();
        assert_eq!(
            output.outcome.status,
            analysis::AnalysisStatus::NotRequested
        );
        assert!(output.executions.is_empty());
        assert!(
            enrich_owner_produced(
                &data,
                &frame,
                &definition,
                stages::Profile::Behavioral,
                &budget,
                None,
                None,
                None,
                id(3),
                &mut work
            )
            .is_err()
        );
    }
    #[test]
    fn empty_actual_frame_uses_source_issuer_and_catalog_is_prepared_once() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let catalog =
            models::Catalog::parse("empty.toml", "version=7\nmodels=[]\ncontext_protocols=[]\n")
                .unwrap();
        let (parameters, definition) =
            super::super::configuration::enriched_execution(catalog.declaration().id());
        let (frame, _) =
            publication::AnalysisInvocation::new(id(1), id(2), definition.id(), None, []);
        let source_definition = super::super::configuration::source_calls().1;
        let (parent, _) = analysis::source_call::AnalysisInvocation::new(
            frame.input,
            frame.context,
            source_definition.id(),
            None,
            [],
        );
        let mut data = EnrichedData::new(&budget);
        data.parameters.insert(parameters).unwrap();
        data.catalogs.insert(catalog.declaration().clone()).unwrap();
        data.source_invocations.insert(parent.clone()).unwrap();
        data.source
            .definitions
            .insert(source_definition.clone())
            .unwrap();
        let (_, source) = source_call_records::prepare_empty_produced(
            &data.source,
            &parent,
            &source_definition,
            stages::Profile::Behavioral,
            &budget,
        )
        .unwrap();
        let hydrated = source.hydrate_empty(&parent, &budget).unwrap();
        let mut work = EnrichedWork::new(&frame, &budget).unwrap();
        let output = enrich_empty_produced(
            &data,
            &frame,
            &definition,
            stages::Profile::Behavioral,
            &budget,
            Some(&hydrated),
            &mut work,
        )
        .unwrap();
        assert!(output.run.requested && output.executions.is_empty());
        assert_eq!(output.outcome.status, analysis::AnalysisStatus::Completed);
        assert!(
            enrich_empty_produced(
                &data,
                &frame,
                &definition,
                stages::Profile::Behavioral,
                &budget,
                None,
                &mut work
            )
            .is_err()
        );
        let first = work.catalog(catalog.declaration()).unwrap();
        let second = work.catalog(catalog.declaration()).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let mut changed = catalog.declaration().clone();
        changed.source.push('\n');
        assert!(work.catalog(&changed).is_err());
        let artifact = SourceArtifact::from_bytes(frame.input, "unit.py".into(), b"pass").unwrap();
        data.source
            .evaluation
            .uses
            .insert(ArtifactUse {
                artifact: artifact.id(),
                input: frame.input,
                role: input::SourceRole::Release,
            })
            .unwrap();
        data.source
            .evaluation
            .occurrences
            .insert(Occurrence {
                source: artifact.id(),
                start: 0,
                end: 4,
                syntax_kind: source::SyntaxKind::StmtPass,
                role: source::OccurrenceRole::Syntax,
                structural_path: vec![0],
            })
            .unwrap();
        data.source.evaluation.artifacts.insert(artifact).unwrap();
        assert!(
            enrich_empty_produced(
                &data,
                &frame,
                &definition,
                stages::Profile::Behavioral,
                &budget,
                Some(&hydrated),
                &mut work
            )
            .is_err()
        );
    }
}
