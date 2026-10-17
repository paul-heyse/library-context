//! One finite SCC summary owner. Raw branches, finite proof occurrences and publication
//! aggregates are separate; mutable iteration state never becomes semantic authority.
use super::{
    source_call_records::SourceCallHeader,
    summary_path::{self, PathData, SummaryPathContribution},
    summary_proof::{self, SummaryProofCost},
    summary_worklist::{self, ProofCost},
};
use crate::domain::{
    analysis::{
        self, summary as owner,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::*,
    calls::*,
    charged::{ChargedMap, StateCharge},
    composition::*,
    conditions::{entry::EntryData, stability::*, *},
    flow::*,
    normalized::{
        Rows,
        binding_normalization::{BindingData, BindingOutput, VerifiedBindings},
        bindings::*,
        entities::*,
    },
    projection::{normalization::ProjectionKey, snapshot::MaterializedGraph},
    resources::{Reservation, ResourceBudget},
    transfer::{summary::*, *},
    value::*,
    *,
};
use crate::{Domain, DomainSum};
#[path = "summary_enqueue.rs"]
mod enqueue_groups;
use enqueue_groups::{EndpointIndex, ExhaustedGroups};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid(format!("Summary predecessor absent: {}", R::NAME)))
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "summary_origins", rule = "summary_source_origin")]
pub enum SummaryOrigin {
    #[model(code = 0)]
    Local {
        #[model(premise)]
        contribution: Id<local_semantics::LocalContribution>,
    },
    #[model(code = 1)]
    Model {
        #[model(premise)]
        support: Id<transfer::model::TransferSupport>,
    },
    #[model(code = 2)]
    Captured {
        #[model(premise)]
        witness: Id<super::summary_capture::SummaryCaptureWitness>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="summary_runs",invariant_refs=super::summary_replay::invariants_refs,publication_refs=super::summary_replay::profile_checks_refs)]
pub struct SummaryRun {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    pub requested: bool,
    pub components: i64,
    pub work: i64,
    pub proofs: i64,
    pub residuals: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_components", rule = "finite_summary_component")]
pub struct SummaryComponent {
    #[model(key, premise)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub members: ContentHash,
    pub ordinal: i64,
    pub work: i64,
    pub closed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_component_members", rule = "summary_component_member")]
pub struct ComponentMember {
    #[model(key, premise)]
    pub component: Id<SummaryComponent>,
    #[model(key)]
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_proof_origins", rule = "finite_summary_origin")]
pub struct ProofOrigin {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)]
    pub proof: Id<SummaryPremise>,
    #[model(key, premise)]
    pub origin: Id<SummaryOrigin>,
    pub semantic: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_residuals")]
pub struct SummaryResidual {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub component: ContentHash,
    #[model(key)]
    pub origin: Id<SummaryOrigin>,
    #[model(key)]
    pub event: Id<normalized::events::NormalizedCallEvent>,
    #[model(key)]
    pub channel: analysis::AnalysisChannel,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub input: Id<Place>,
    #[model(key)]
    pub output: Id<Place>,
    #[model(key)]
    pub reason: obligation::ObligationKind,
    pub qualification: Id<AssertionQualification>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_call_members")]
pub struct CallMember {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
    pub header: Option<Id<SourceCallHeader>>,
    pub reason: Option<obligation::ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_origin_boundaries")]
/// A publication boundary on an admitted finite member, not an exhausted recursive suffix.
pub struct OriginBoundary {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub component: ContentHash,
    #[model(key)]
    pub origin: Id<SummaryOrigin>,
    #[model(key)]
    pub transfer: Id<TransferKey>,
    #[model(key)]
    pub reason: obligation::ObligationKind,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
}
/// Invocation-local questions discovered from processed live operands. This is preparation,
/// never a persisted claim or a second pair trace.
pub(super) type Question = (
    Id<normalized::events::NormalizedCallEvent>, Id<Place>, Id<Place>,
    Id<AssertionQualification>,
);
struct QuestionDomain {
    questions: crate::domain::charged::ChargedSet<Question>,
    charge: StateCharge,
}
impl QuestionDomain {
    fn new(budget: &ResourceBudget) -> Self {
        Self { questions: Default::default(), charge: StateCharge::new(budget, "summary-question-domain") }
    }
    fn discover(&mut self, question: Question) -> Result<(), ModelError> {
        self.questions.insert(&mut self.charge, question)?;
        Ok(())
    }
    fn observe(
        &mut self, event: Id<normalized::events::NormalizedCallEvent>,
        caller: &dyn CompositionOperand, callee: &dyn CompositionOperand,
        results: &[CallComposition],
    ) -> Result<(), ModelError> {
        if !results.is_empty() {
            self.discover((event, caller.descriptor().input, callee.descriptor().output,
                caller.qualification().id()))?;
        }
        Ok(())
    }

}
#[macro_export]
macro_rules! summary_owned_inputs{($apply:ident)=>{$apply!{
 atom_restrictions:$crate::domain::atom_decision::AtomRestriction,
 terminal_frontiers:$crate::domain::execution::protocol_interpretation::ConditionalTerminalFrontier,normal_restrictions:$crate::domain::execution::protocol_interpretation::NormalContinuationRestriction,closed_targets:$crate::domain::execution::closed_targets::ClosedTargetAssessment,
 captured_entries:$crate::domain::execution::capture_bridge::CapturedEntryBinding,captured_values:$crate::domain::execution::capture_bridge::CapturedValueSource,fresh_calls:$crate::domain::execution::enriched_records::SourceExecutionInvocation,
 exception_bodies:$crate::domain::execution::enriched_records::BodyExecution,exception_values:$crate::domain::execution::enriched_records::ExecutionOutcome,
 symbolic_classes:$crate::domain::normalized::symbolic_fields::SourceFieldClass,symbolic_stores:$crate::domain::normalized::symbolic_fields::SourceFieldStore,symbolic_associations:$crate::domain::normalized::symbolic_fields::SourceFieldAssociation,symbolic_readers:$crate::domain::normalized::symbolic_fields::SourceFieldReader,symbolic_links:$crate::domain::normalized::symbolic_fields::SourceFieldReaderLink,symbolic_local_stores:$crate::domain::local_symbolic::SymbolicFieldStore,
 local_invocations:$crate::domain::analysis::local::AnalysisInvocation,model_invocations:$crate::domain::analysis::model::AnalysisInvocation,enriched_invocations:$crate::domain::analysis::enriched_execution::AnalysisInvocation,source_invocations:$crate::domain::analysis::source_call::AnalysisInvocation,
 local_outcomes:$crate::domain::analysis::local::AnalysisOutcome,model_outcomes:$crate::domain::analysis::model::AnalysisOutcome,enriched_outcomes:$crate::domain::analysis::enriched_execution::AnalysisOutcome,source_outcomes:$crate::domain::analysis::source_call::AnalysisOutcome,model_derivations:$crate::domain::analysis::model::AnalysisDerivation,
 local_contributions:$crate::domain::local_semantics::LocalContribution,local_guards:$crate::domain::local_semantics::LocalGuardContribution,local_keys:$crate::domain::transfer::local::TransferKey,local_alternatives:$crate::domain::transfer::local::TransferAlternative,local_supports:$crate::domain::transfer::local::TransferSupport,
 model_keys:$crate::domain::transfer::model::TransferKey,model_alternatives:$crate::domain::transfer::model::TransferAlternative,model_supports:$crate::domain::transfer::model::TransferSupport,
 native:$crate::domain::analysis::native::NativeQualification,headers:$crate::domain::execution::source_call_records::SourceCallHeader,parameters:$crate::domain::analysis::MethodParameters,definitions:$crate::domain::analysis::AnalysisDefinition,
 entries:$crate::domain::conditions::entry::EntryValueWitness,entry_sources:$crate::domain::conditions::entry::EntryAccessSource,stability:$crate::domain::conditions::stability::StabilityWitness,
}};}
/// Typed dispatch inventory for the finite predecessor evidence consumed by Summary.
#[macro_export]
macro_rules! summary_evidence_inputs {($apply:ident)=>{$apply!{
 local_sources:$crate::domain::analysis::local::SupportSource,
 local_derivations:$crate::domain::analysis::local::AnalysisDerivation,
 model_sources:$crate::domain::analysis::model::SupportSource,
 model_derivations:$crate::domain::analysis::model::AnalysisDerivation,
 theory_witnesses:$crate::domain::local_theory::TheoryWitness,
 atom_decisions:$crate::domain::atom_decision::AtomDecision,
 atom_restrictions:$crate::domain::atom_decision::AtomRestriction,
 model_applications:$crate::domain::execution::model_production::ModelApplication,
 model_transfer_witnesses:$crate::domain::execution::model_transfer::ModelTransferWitness,
 context_transfer_witnesses:$crate::domain::execution::model_context_transfer::ContextTransferWitness,
}};}
#[macro_export]
macro_rules! summary_projection_inputs {
    ($apply:ident) => {
        $apply! {
         graph_assessments:$crate::domain::projection::ProjectionSourceAssessment,
         graph_snapshots:$crate::domain::projection::ProjectionSnapshot,
         graph_chunks:$crate::domain::projection::ProjectionSnapshotChunk,
        }
    };
}
macro_rules! projection_inventory {($($f:ident:$t:ty,)*)=>{fn projection_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}};}
crate::summary_projection_inputs!(projection_inventory);
#[macro_export]
macro_rules! summary_vocabulary{($apply:ident)=>{$apply!{
 assumption_sets:$crate::domain::assumptions::AssumptionSet,assumption_members:$crate::domain::assumptions::AssumptionSetMember,assumptions:$crate::domain::assumptions::Assumption,assumption_universes:$crate::domain::assumptions::AssumptionUniverse,
 roots:$crate::domain::value::PlaceRoot,places:$crate::domain::value::Place,paths:$crate::domain::value::AccessPath,segments:$crate::domain::value::PathSegment,literals:$crate::domain::value::Literal,predicates:$crate::domain::value::Predicate,atoms:$crate::domain::conditions::EvaluationAtom,qualifications:$crate::domain::assertion::AssertionQualification,conditions:$crate::domain::conditions::Condition,nodes:$crate::domain::conditions::ConditionNode,
}};}
macro_rules! vocabulary{($($field:ident:$ty:ty,)*)=>{pub struct Vocabulary{$(pub $field:ChargedMap<Id<$ty>,$ty>,)*charge:StateCharge}impl Vocabulary{pub(super) fn new(b:&ResourceBudget)->Self{Self{$($field:Default::default(),)*charge:StateCharge::new(b,"summary-vocabulary")}}$(pub fn $field(&mut self,max_rows:usize,max_bytes:usize)->Result<Option<crate::domain::normalized::CanonicalBatch<$ty>>,ModelError>{crate::domain::normalized::rows::take_checked_batch(&mut self.$field,&mut self.charge,max_rows,max_bytes)})*pub(super) fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{$(if n==<$ty>::NAME{for row in <$ty>::decode(b)?{self.$field.insert(&mut self.charge,row.id(),row)?;}})*Ok(())}fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]).at_epoch(stages::PublicationBoundary::Model),)*]}}};}
crate::summary_vocabulary!(vocabulary);
// Predecessor evidence and entry bindings use the explicit completed vocabulary view.
fn evidence_inputs() -> Vec<ValidationInput> {
    let mut inputs = analysis::local::support::EvidenceIndex::inputs();
    inputs.extend(analysis::model::support::EvidenceIndex::inputs());
    for input in &mut inputs {
        if stages::is_vocabulary(input.name()) {
            *input = input.clone().at_epoch(stages::PublicationBoundary::Model);
        }
    }
    inputs
}
macro_rules! data{($($field:ident:$ty:ty,)*)=>{pub struct SummaryData{pub occurrences:source::properties::OccurrenceIndex,pub symbols:source::properties::SymbolIndex,pub artifacts:source::properties::ArtifactIndex,pub graphs:projection::normalization::ProjectionOutput,pub entry:EntryData,pub bindings:BindingData,pub binding_output:BindingOutput,pub path:PathData,pub vocabulary:Vocabulary,pub local_evidence:analysis::local::support::EvidenceIndex,pub model_evidence:analysis::model::support::EvidenceIndex,$(pub $field:Rows<$ty>,)*}
impl SummaryData{pub fn new(b:&ResourceBudget)->Self{Self{occurrences:source::properties::OccurrenceIndex::new(b),symbols:source::properties::SymbolIndex::new(b),artifacts:source::properties::ArtifactIndex::new(b),graphs:projection::normalization::ProjectionOutput::new(b),entry:EntryData::new(b),bindings:BindingData::new(b),binding_output:BindingOutput::new(b),path:PathData::new(b),vocabulary:Vocabulary::new(b),local_evidence:analysis::local::support::EvidenceIndex::new(b),model_evidence:analysis::model::support::EvidenceIndex::new(b),$($field:Rows::new(b),)*}}
 pub fn visit_input(&mut self,input:&ValidationInput,b:&arrow_array::RecordBatch)->Result<(),ModelError>{let n=input.name();if stages::is_vocabulary(n){return match input.prefix(){Some(stages::PublicationBoundary::Facts)=>{self.entry.visit_input(input,b)?;self.bindings.visit(n,b)?;Ok(())},Some(stages::PublicationBoundary::Model)=>{self.vocabulary.visit(n,b)?;self.local_evidence.visit(n,b)?;self.model_evidence.visit(n,b)?;Ok(())},_=>Err(invalid("Summary input changes completed vocabulary view"))}}self.visit(n,b)}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{self.graphs.visit(n,b)?;self.entry.visit(n,b)?;self.bindings.visit(n,b)?;self.binding_output.visit(n,b)?;self.path.visit(n,b)?;self.local_evidence.visit(n,b)?;self.model_evidence.visit(n,b)?;$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}
 fn visit_owned(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}
 pub fn consumed_inputs(profile:stages::Profile)->Vec<ValidationInput>{let mut inputs=super::summary_replay::production_inputs(profile);inputs.extend(analysis::expected::inputs(analysis::AnalysisMethod::Summaries));inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
 pub fn inputs()->Vec<ValidationInput>{let mut inputs=BindingData::validation_inputs();inputs.extend(BindingOutput::validation_inputs());inputs.extend(EntryData::facts_inputs());inputs.extend(PathData::inputs());inputs.extend(projection_inputs());inputs.extend(Vocabulary::inputs());inputs.extend(evidence_inputs());inputs.push(ValidationInput::of::<analysis::native::NativeAssertionPremise>(&["id"]));inputs.extend([$(ValidationInput::of::<$ty>(&["id"]),)*]);inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
}};}
crate::summary_owned_inputs!(data);
impl SummaryData {
    /// Actual predecessor owners already admitted entry/binding proofs. Retain only the
    /// properties and canonical semantic operands this Summary kernel consumes.
    pub fn actual_inputs() -> Vec<ValidationInput> {
        let mut inputs = Vec::new();
        macro_rules! collect{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
        crate::summary_owned_inputs!(collect);
        crate::normalized_binding_outputs!(collect);
        crate::summary_path_inputs!(collect);
        crate::summary_vocabulary!(collect);
        crate::summary_evidence_inputs!(collect);
        inputs.extend(evidence_inputs());
        macro_rules! retained{($($ty:ty),*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
        retained!(
            source::Occurrence,
            source::SourceArtifact,
            normalized::events::NormalizedCallEvent,
            normalized::events::NormalizedCallAlternative,
            normalized::events::EventAssessment,
            normalized::events::CallAlternativeSource,
            calls::CallTarget,
            calls::Signature,
            calls::SignatureParameter,
            calls::ProviderSymbol,
            calls::CallArgument,
            calls::CallDestination,
            calls::Receiver,
            normalized::entities::OccurrenceOwnership,
            normalized::callables::SignatureSlot,
            declarations::SymbolDeclaration,
            declarations::ParameterDeclaration,
            normalized::entities::EntityRef,
            normalized::entities::CallableEntity,
            flow::FlowValueObservation,
            flow::FlowValueSupport,
            flow::FlowUse,
            flow::FlowUseObservation,
            flow::FlowUseSupport,
            flow::FlowDefinition,
            flow::FlowDefinitionObservation,
            flow::FlowDefinitionSupport,
            flow::ReachingDefinition,
            flow::FlowReachingObservation,
            flow::FlowReachingSupport,
            lexical::LexicalScope,
            attribution::ProviderRun,
            assertion::ProviderSurface,
            attribution::ProviderCoverage
        );
        inputs
    }
    pub fn consumes_actual(input: &ValidationInput) -> bool {
        Self::actual_inputs()
            .iter()
            .any(|candidate| candidate.type_id() == input.type_id())
    }
    pub fn visit_actual(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let n = input.name();
        if [
            source::Occurrence::NAME,
            calls::ProviderSymbol::NAME,
            source::SourceArtifact::NAME,
        ]
        .contains(&n)
        {
            return Err(invalid(
                "Summary actual source identities require completed property projection",
            ));
        }
        if !Self::consumes_actual(input) {
            return Ok(());
        }
        if stages::is_vocabulary(n) {
            return self.visit_input(input, b);
        }
        macro_rules! binding{($($ty:ty),*)=>{if [$(<$ty>::NAME,)*].contains(&n){self.bindings.visit(n,b)?;}};}
        binding!(
            source::SourceArtifact,
            normalized::events::NormalizedCallEvent,
            normalized::events::NormalizedCallAlternative,
            normalized::events::EventAssessment,
            normalized::events::CallAlternativeSource,
            calls::CallTarget,
            calls::Signature,
            calls::SignatureParameter,
            calls::ProviderSymbol,
            calls::CallArgument,
            calls::CallDestination,
            calls::Receiver,
            normalized::entities::OccurrenceOwnership,
            normalized::callables::SignatureSlot
        );
        macro_rules! entry{($($ty:ty),*)=>{if [$(<$ty>::NAME,)*].contains(&n){self.entry.visit(n,b)?;}};}
        entry!(
            source::SourceArtifact,
            declarations::SymbolDeclaration,
            declarations::ParameterDeclaration,
            calls::Signature,
            calls::SignatureParameter,
            normalized::entities::EntityRef,
            normalized::entities::CallableEntity,
            normalized::entities::OccurrenceOwnership,
            flow::FlowValueObservation,
            flow::FlowValueSupport,
            flow::FlowUse,
            flow::FlowUseObservation,
            flow::FlowUseSupport,
            flow::FlowDefinition,
            flow::FlowDefinitionObservation,
            flow::FlowDefinitionSupport,
            flow::ReachingDefinition,
            flow::FlowReachingObservation,
            flow::FlowReachingSupport,
            lexical::LexicalScope,
            attribution::ProviderRun,
            assertion::ProviderSurface,
            attribution::ProviderCoverage
        );
        self.binding_output.visit(n, b)?;
        self.path.visit(n, b)?;
        self.local_evidence.visit(n, b)?;
        self.model_evidence.visit(n, b)?;
        self.visit_owned(n, b)
    }
    pub fn symbol(
        &self,
        id: Id<calls::ProviderSymbol>,
    ) -> Option<source::properties::SymbolProperties> {
        self.symbols.get(id).copied().or_else(|| {
            self.bindings
                .symbols
                .get(id)
                .map(source::properties::SymbolProperties::from_row)
        })
    }
    pub fn artifact_input(
        &self,
        id: Id<source::SourceArtifact>,
    ) -> Option<Id<input::InputRevision>> {
        self.artifacts
            .get(id)
            .or_else(|| self.bindings.artifacts.get(id).map(|row| row.input))
    }
    pub fn occurrence(
        &self,
        id: Id<source::Occurrence>,
    ) -> Option<source::properties::OccurrenceProperties> {
        self.occurrences.get(id).copied().or_else(|| {
            self.entry
                .occurrences
                .get(id)
                .or_else(|| self.bindings.occurrences.get(id))
                .map(source::properties::OccurrenceProperties::from_row)
        })
    }
    pub fn occurrence_values(
        &self,
    ) -> impl Iterator<Item = source::properties::OccurrenceProperties> + '_ {
        self.occurrences.iter().copied().chain(
            self.entry
                .occurrences
                .iter()
                .map(source::properties::OccurrenceProperties::from_row),
        )
    }
    pub fn same_occurrence(&self, a: Id<source::Occurrence>, b: Id<source::Occurrence>) -> bool {
        self.occurrence(a)
            .zip(self.occurrence(b))
            .is_some_and(|(a, b)| {
                (a.source, a.start, a.end, a.syntax_kind)
                    == (b.source, b.start, b.end, b.syntax_kind)
            })
    }
}

#[macro_export]
macro_rules! summary_outputs{($apply:ident)=>{$apply!{
 terminal_witnesses:$crate::domain::execution::summary_terminal::SummaryTerminalWitness,
 capture_witnesses:$crate::domain::execution::summary_capture::SummaryCaptureWitness,capture_contributions:$crate::domain::execution::summary_capture::SummaryCaptureContribution,
 exception_outcomes:$crate::domain::execution::summary_exceptions::SummaryExceptionOutcome,
 symbolic_alternatives:$crate::domain::execution::summary_symbolic::SymbolicFieldAlternative,
 origin_boundaries:$crate::domain::execution::summary_production::OriginBoundary,
 runs:$crate::domain::execution::summary_production::SummaryRun,components:$crate::domain::execution::summary_production::SummaryComponent,component_members:$crate::domain::execution::summary_production::ComponentMember,origins:$crate::domain::execution::summary_production::SummaryOrigin,proof_origins:$crate::domain::execution::summary_production::ProofOrigin,residuals:$crate::domain::execution::summary_production::SummaryResidual,call_members:$crate::domain::execution::summary_production::CallMember,
 control_witnesses:$crate::domain::execution::summary_control::SummaryControlWitness,
 costs:$crate::domain::execution::summary_proof::SummaryProofCost,path_routes:$crate::domain::execution::summary_path::SummaryPathRoute,path_witnesses:$crate::domain::execution::summary_path::SummaryPathWitness,path_contributions:$crate::domain::execution::summary_path::SummaryPathContribution,
 premises:$crate::domain::transfer::summary::SummaryPremise,witnesses:$crate::domain::transfer::summary::SummaryWitness,contributions:$crate::domain::transfer::summary::SummaryContribution,keys:$crate::domain::transfer::summary::TransferKey,alternatives:$crate::domain::transfer::summary::TransferAlternative,supports:$crate::domain::transfer::summary::TransferSupport,
 influences:$crate::domain::transfer::summary::ControlInfluence,control_supports:$crate::domain::transfer::summary::ControlSupport,selections:$crate::domain::transfer::summary::Selection,substitutions:$crate::domain::conditions::stability::GuardSubstitution,
 claims:$crate::domain::execution::summary_consequences::SummaryClaim,claim_members:$crate::domain::execution::summary_consequences::ClaimMember,claim_standings:$crate::domain::execution::summary_consequences::ClaimStanding,claim_proofs:$crate::domain::execution::summary_consequences::ClaimProof,claim_proof_members:$crate::domain::execution::summary_consequences::ClaimProofMember,refutation_coverage:$crate::domain::execution::summary_consequences::ClaimRefutationCoverage,conclusions:$crate::domain::execution::summary_consequences::ClaimConclusion,obligation_sources:$crate::domain::analysis::summary::ObligationSource,
 sources:$crate::domain::analysis::summary::SupportSource,subjects:$crate::domain::analysis::summary::ObligationSubject,propositions:$crate::domain::analysis::summary::AnalysisProposition,derivations:$crate::domain::analysis::summary::AnalysisDerivation,derivation_premises:$crate::domain::analysis::summary::AnalysisDerivationPremise,obligations:$crate::domain::analysis::summary::AnalysisObligation,discharges:$crate::domain::analysis::summary::DischargeEvidence,
}};}
macro_rules! output{($($field:ident:$ty:ty,)*)=>{pub struct SummaryRecords{$(pub $field:Rows<$ty>,)*pub outcome:owner::AnalysisOutcome,pub vocabulary:Vocabulary,questions:QuestionDomain,pending:Option<(Vec<super::summary_consequences::PendingDischarge>,StateCharge)>}impl SummaryRecords{pub(super) fn new(i:Id<owner::AnalysisInvocation>,b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*outcome:owner::AnalysisOutcome{invocation:i,status:analysis::AnalysisStatus::Completed,reason:None},vocabulary:Vocabulary::new(b),questions:QuestionDomain::new(b),pending:None}}}};}
crate::summary_outputs!(output);
impl SummaryRecords {
    pub(super) fn questions(&self) -> impl Iterator<Item = &Question> { self.questions.questions.iter() }
}

impl SummaryRecords {
    fn consequences(
        &mut self,
        data: &SummaryData,
        invocation: &owner::AnalysisInvocation,
        definition: &analysis::AnalysisDefinition,
        profile: stages::Profile,
        verified_bindings: Option<&VerifiedBindings>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let rows = super::summary_consequences::derive_with_verified_bindings(
            data,
            self,
            invocation,
            definition,
            profile,
            verified_bindings,
            budget,
        )?;
        macro_rules! append{($($dst:ident:$src:ident),* $(,)?)=>{$(for row in rows.$src.iter(){self.$dst.insert(row.clone())?;})*};}
        append!(claims:claims,claim_members:members,claim_standings:standings,claim_proofs:proofs,claim_proof_members:proof_members,refutation_coverage:refutation_coverage,conclusions:conclusions,keys:keys,premises:premises,subjects:subjects,sources:sources,derivations:derivations,propositions:propositions,derivation_premises:derivation_premises,obligations:obligations);
        self.pending = Some(rows.into_pending_parts());
        Ok(())
    }
    /// Finalize checked claims only against the canonical coverage publication for this frame.
    pub fn discharge(
        &mut self,
        coverage: &Rows<owner::AnalysisCoverage>,
    ) -> Result<(), ModelError> {
        let Some((pending, _charge)) = self.pending.take() else {
            return Err(invalid("Summary consequences already discharged"));
        };
        for token in pending {
            let mut matching = coverage.iter().filter(|row| token.matches(row));
            let row = matching
                .next()
                .ok_or_else(|| invalid("Summary claim coverage absent"))?;
            if matching.next().is_some() {
                return Err(invalid("Summary claim coverage ambiguous"));
            }
            let (source, evidence) = token.admit(row)?;
            self.obligation_sources.insert(source)?;
            self.discharges.insert(evidence)?;
        }
        Ok(())
    }
}
#[derive(Clone)]
struct WorkBranch {
    branch: TransferBranch<TransferKey>,
    premise: SummaryPremise,
    origin: Id<SummaryOrigin>,
    facts: SourceFacts,
    cost: ProofCost,
}
impl HeapSize for WorkBranch {}
impl CompositionOperand for WorkBranch {
    fn descriptor(&self) -> TransferDescriptor {
        self.branch.descriptor()
    }
    fn qualification(&self) -> &AssertionQualification {
        self.branch.qualification()
    }
    fn condition(&self) -> &Diagram {
        self.branch.condition()
    }
    fn premise(&self) -> SummaryPremise {
        self.premise.clone()
    }
}
type ProofId = (Id<SummaryOrigin>, Id<SummaryPremise>);
impl WorkBranch {
    fn id(&self) -> ProofId {
        (self.origin, self.premise.id())
    }
    fn semantic(&self) -> ContentHash {
        let d = self.descriptor();
        let mut k = KeySink::new("finite-summary-semantic-member");
        self.origin.encode(&mut k);
        analysis::AnalysisChannel::Value.encode(&mut k);
        CallPhase::Call.encode(&mut k);
        d.owner.encode(&mut k);
        d.input.encode(&mut k);
        d.output.encode(&mut k);
        d.context.encode(&mut k);
        d.scope.encode(&mut k);
        d.modality.encode(&mut k);
        d.approximation.encode(&mut k);
        d.kind.encode(&mut k);
        self.qualification().condition.encode(&mut k);
        self.qualification().assumptions.encode(&mut k);
        k.finish()
    }
}
impl Vocabulary {
    pub(super) fn copy(&self, b: &ResourceBudget) -> Result<Self, ModelError> {
        let mut result = Self::new(b);
        macro_rules! copy{($($f:ident:$t:ty,)*)=>{$(for row in self.$f.values(){result.$f.insert(&mut result.charge,row.id(),row.clone())?;})*};}
        crate::summary_vocabulary!(copy);
        Ok(result)
    }
    fn catalog(&self) -> CompositionCatalog<'_> {
        CompositionCatalog {
            assumptions: assumptions::AssumptionCatalog {
                sets: &self.assumption_sets,
                members: &self.assumption_members,
                definitions: &self.assumptions,
            },
            guards: self.guards(),
            paths: &self.paths,
            segments: place_composition::PathCatalog {
                segments: &*self.segments,
                literals: &*self.literals,
            },
        }
    }
    fn guards(&self) -> conditions::rebase::GuardCatalog<'_> {
        conditions::rebase::GuardCatalog {
            atoms: &self.atoms,
            predicates: &self.predicates,
            places: &self.places,
            roots: &self.roots,
        }
    }
    fn diagram(&self, id: Id<Condition>, budget: &ResourceBudget) -> Result<Diagram, ModelError> {
        let _buffer = budget.reserve(
            "summary-condition-decode",
            self.nodes.len().saturating_mul(2048).saturating_add(4096),
        )?;
        if self.nodes.len() > 100_000 {
            return Err(invalid("condition node admission exceeded"));
        }
        Diagram::from_catalog(
            self.conditions
                .get(&id)
                .ok_or_else(|| invalid("Summary condition absent"))?,
            &self.nodes,
        )
    }

    fn composition(&mut self, rows: &CompositionRecords) -> Result<(), ModelError> {
        macro_rules! rows{($($f:ident),*)=>{$(for row in &rows.$f{self.$f.insert(&mut self.charge,row.id(),row.clone())?;})*};}
        rows!(
            roots,
            places,
            paths,
            segments,
            literals,
            predicates,
            atoms,
            conditions,
            nodes,
            assumption_sets,
            assumption_members
        );
        if let Some(q) = &rows.qualification {
            self.qualifications
                .insert(&mut self.charge, q.id(), q.clone())?;
        }
        Ok(())
    }
    fn path(&mut self, e: &summary_path::PathEmission) -> Result<(), ModelError> {
        self.roots
            .insert(&mut self.charge, e.root.id(), e.root.clone())?;
        self.places
            .insert(&mut self.charge, e.place.id(), e.place.clone())?;
        self.qualified(e.branch.qualification(), e.branch.condition())
    }
    pub(super) fn qualified(
        &mut self,
        q: &AssertionQualification,
        diagram: &Diagram,
    ) -> Result<(), ModelError> {
        self.qualifications
            .insert(&mut self.charge, q.id(), q.clone())?;
        let (condition, nodes) = diagram.records();
        self.conditions
            .insert(&mut self.charge, condition.id(), condition)?;
        for node in nodes {
            self.nodes.insert(&mut self.charge, node.id(), node)?;
        }
        Ok(())
    }
}
type GuardInventory = (
    BTreeMap<Id<EvaluationAtom>, CheckedStability>,
    Box<dyn Reservation>,
);

impl SummaryData {
    fn evidence(
        &self,
        out: &SummaryRecords,
        premise: &SummaryPremise,
        discovery: &Discovery<'_>,
        budget: &ResourceBudget,
    ) -> Result<TransferEvidence, ModelError> {
        match premise {
            SummaryPremise::Local { alternative } => {
                let a = need(&self.local_alternatives, *alternative)?;
                let selected = discovery
                    .local_supports
                    .get(alternative)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let _charge = budget.reserve(
                    "summary-local-supports",
                    selected
                        .len()
                        .saturating_mul(size_of::<transfer::local::TransferSupport>()),
                )?;
                let supports = selected.iter().map(|s| (**s).clone()).collect::<Vec<_>>();
                TransferEvidence::local(a, &supports, &self.local_evidence, budget)
            }
            SummaryPremise::Model { alternative } => {
                let a = need(&self.model_alternatives, *alternative)?;
                let selected = discovery
                    .model_supports
                    .get(alternative)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let _charge = budget.reserve(
                    "summary-model-supports",
                    selected
                        .len()
                        .saturating_mul(size_of::<transfer::model::TransferSupport>()),
                )?;
                let supports = selected.iter().map(|s| (**s).clone()).collect::<Vec<_>>();
                TransferEvidence::model(a, &supports, &self.model_evidence, budget)
            }
            SummaryPremise::Witness { witness } => {
                TransferEvidence::witness(need(&out.witnesses, *witness)?, budget)
            }
            SummaryPremise::Captured { witness } => {
                TransferEvidence::captured(need(&out.capture_witnesses, *witness)?, budget)
            }
            SummaryPremise::Path { witness } => {
                TransferEvidence::path(need(&out.path_witnesses, *witness)?, budget)
            }
        }
    }
    fn seeds(
        &self,
        invocation: &owner::AnalysisInvocation,
        out: &mut SummaryRecords,
        discovery: &Discovery<'_>,
        budget: &ResourceBudget,
    ) -> Result<(Vec<WorkBranch>, Box<dyn Reservation>), ModelError> {
        let charge = budget.reserve(
            "summary-raw-seeds",
            self.local_contributions
                .len()
                .saturating_mul(self.atom_restrictions.len().saturating_add(1))
                .saturating_add(self.model_supports.len())
                .saturating_add(self.captured_entries.len())
                .saturating_mul(size_of::<WorkBranch>() + 512),
        )?;
        let diagram = |id| -> Result<std::sync::Arc<AdmittedDiagram>, ModelError> {
            let record = self
                .vocabulary
                .conditions
                .get(&id)
                .ok_or_else(|| invalid("Summary condition absent"))?;
            discovery.vocabulary_conditions.diagram(record)
        };
        let mut seeds = Vec::new();
        for contribution in self.local_contributions.iter() {
            let parent = need(&self.local_invocations, contribution.invocation)?;
            if (parent.input, parent.context) != (invocation.input, invocation.context) {
                continue;
            }
            let key = need(&self.local_keys, contribution.transfer)?;
            let q = self
                .vocabulary
                .qualifications
                .get(&contribution.qualification)
                .ok_or_else(|| invalid("Local summary seed qualification absent"))?;
            let alternative = key.alternative(q);
            if self.local_alternatives.get(alternative.id()) != Some(&alternative) {
                return Err(invalid("Local summary seed alternative absent"));
            }
            for selected in crate::domain::atom_decision::selected_local_alternatives(
                &alternative,
                &self.local_alternatives,
                &self.atom_restrictions,
            ) {
                let q = self
                    .vocabulary
                    .qualifications
                    .get(&selected.qualification)
                    .ok_or_else(|| invalid("restricted Local seed qualification absent"))?;
                let origin = SummaryOrigin::Local {
                    contribution: contribution.id(),
                };
                out.origins.insert(origin.clone())?;
                let premise = SummaryPremise::Local {
                    alternative: selected.id(),
                };
                let evidence = self.evidence(out, &premise, discovery, budget)?;
                let facts = SourceFacts {
                    qualification: q.id(),
                    status: analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        evidence.facts.iter().map(|f| f.status),
                    ),
                    heuristic: evidence.facts.iter().any(|f| f.heuristic),
                };
                seeds.push(WorkBranch {
                    branch: TransferBranch::from_prepared(
                        TransferKey::from_descriptor(key.descriptor()),
                        q.clone(),
                        &**diagram(q.condition)?,
                        budget,
                    )?,
                    premise,
                    origin: origin.id(),
                    facts,
                    cost: ProofCost::SOURCE,
                });
            }
        }
        for support in self.model_supports.iter() {
            let (source, _) = self.model_evidence.get(support.source)?;
            let analysis::model::SupportSource::AnalysisDerivation { derivation } = source else {
                return Err(invalid("Model summary support lacks owning derivation"));
            };
            let producer = need(&self.model_derivations, *derivation)?;
            let parent = need(&self.model_invocations, producer.invocation)?;
            if (parent.input, parent.context) != (invocation.input, invocation.context) {
                continue;
            }
            let alternative = need(&self.model_alternatives, support.assertion)?;
            let key = need(&self.model_keys, alternative.transfer)?;
            if key.context != invocation.context {
                return Err(invalid("Model summary seed changes producer context"));
            }
            let q = self
                .vocabulary
                .qualifications
                .get(&alternative.qualification)
                .ok_or_else(|| invalid("Model summary seed qualification absent"))?;
            let (_, facts) = self.model_evidence.get(support.source)?;
            if facts.qualification != q.id() {
                return Err(invalid("Model summary support qualification mismatch"));
            }
            let origin = SummaryOrigin::Model {
                support: support.id(),
            };
            out.origins.insert(origin.clone())?;
            seeds.push(WorkBranch {
                branch: TransferBranch::from_prepared(
                    TransferKey::from_descriptor(key.descriptor()),
                    q.clone(),
                    &**diagram(q.condition)?,
                    budget,
                )?,
                premise: SummaryPremise::Model {
                    alternative: alternative.id(),
                },
                origin: origin.id(),
                facts,
                cost: ProofCost::SOURCE,
            });
        }
        let (captures, _captures) = super::summary_capture::seeds(self, invocation, budget)?;
        for seed in captures {
            for root in seed.roots {
                out.vocabulary
                    .roots
                    .insert(&mut out.vocabulary.charge, root.id(), root)?;
            }
            for place in seed.places {
                out.vocabulary
                    .places
                    .insert(&mut out.vocabulary.charge, place.id(), place)?;
            }
            let path = AccessPath::empty();
            out.vocabulary
                .paths
                .insert(&mut out.vocabulary.charge, path.id(), path)?;
            let origin = SummaryOrigin::Captured {
                witness: seed.witness.id(),
            };
            out.origins.insert(origin.clone())?;
            let premise = SummaryPremise::Captured {
                witness: seed.witness.id(),
            };
            out.keys.insert(seed.branch.key().clone())?;
            out.premises.insert(premise.clone())?;
            let facts = seed.witness.source_facts();
            out.capture_witnesses.insert(seed.witness)?;
            seeds.push(WorkBranch {
                branch: seed.branch,
                premise,
                origin: origin.id(),
                facts,
                cost: ProofCost::SOURCE,
            });
        }
        seeds.sort_by_key(WorkBranch::id);
        Ok((seeds, charge))
    }
    fn guards(
        &self,
        budget: &ResourceBudget,
        actual: Option<&local_semantics::ProducedLocal>,
    ) -> Result<GuardInventory, ModelError> {
        let charge = budget.reserve(
            "summary-private-stability",
            self.local_guards
                .len()
                .saturating_mul(size_of::<CheckedStability>() + 128),
        )?;
        let mut results = BTreeMap::new();
        for guard in self.local_guards.iter() {
            let stored = need(&self.entries, guard.entry)?;
            let source = need(&self.entry_sources, stored.access_source)?;
            let witness = need(&self.stability, guard.stability)?;
            let checked = if let Some(actual) = actual {
                actual.require_entry(stored, source, budget)?;
                if witness.entry != stored.id() {
                    return Err(invalid("Local guard changes actual entry owner"));
                }
                actual.guard(witness, budget)?
            } else {
                let request = conditions::entry::EntryRequest {
                    owner: stored.owner,
                    formal: stored.formal,
                    access: stored.access,
                    context: stored.context,
                    run: stored.run,
                };
                let checked = conditions::entry::EntryValueWitness::derive_for(
                    &self.entry,
                    request,
                    source,
                    budget,
                )?
                .map_err(|r| invalid(format!("stored Local guard entry refuses replay: {r:?}")))?;
                if checked.witness() != stored {
                    return Err(invalid("Local guard changes private entry witness"));
                }
                let checked = StabilityWitness::derive(&self.entry, witness.atom, &checked)
                    .map_err(|r| invalid(format!("stored stability refuses replay: {r:?}")))?;
                if checked.witness() != witness {
                    return Err(invalid("Local guard stability changes"));
                }
                checked
            };
            if let Some(old) = results.insert(witness.atom, checked)
                && old.witness() != witness
            {
                return Err(invalid("multiple stability witnesses for one atom"));
            }
        }
        Ok((results, charge))
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct CallInfo {
    attempt: Id<CallBindingAttempt>,
    // Retain the existing private inventory footprint used by resource reservations.
    _event: Id<normalized::events::NormalizedCallEvent>,
    _header: Option<Id<SourceCallHeader>>,
    owner: Id<EntityRef>,
    callee: Id<EntityRef>,
    target: Id<CallTarget>,
}
impl HeapSize for CallInfo {}
fn call_infos(
    data: &SummaryData,
    discovery: &Discovery<'_>,
    verified: &VerifiedBindings,
    invocation: &owner::AnalysisInvocation,
    out: &mut SummaryRecords,
    budget: &ResourceBudget,
) -> Result<(Vec<CallInfo>, Box<dyn Reservation>), ModelError> {
    let charge = budget.reserve(
        "summary-call-inventory",
        data.binding_output
            .attempts
            .len()
            .saturating_mul(size_of::<CallInfo>() + 128),
    )?;
    let mut calls = Vec::new();
    for attempt in data.binding_output.attempts.iter() {
        let event = need(&data.bindings.event_events, attempt.event)?;
        let occurrence = data
            .occurrence(event.site)
            .ok_or_else(|| invalid("Summary call occurrence absent"))?;
        if event.context != invocation.context
            || data
                .artifact_input(occurrence.source)
                .ok_or_else(|| invalid("Summary source artifact absent"))?
                != invocation.input
        {
            continue;
        }
        let mut headers = discovery
            .headers
            .get(&attempt.id())
            .into_iter()
            .flatten()
            .copied();
        let header = headers.next();
        if headers.next().is_some() {
            return Err(invalid("Summary source header ambiguous"));
        }
        let result = (|| {
            let frame = verified.composition(attempt.id()).ok_or(
                attempt
                    .refusal
                    .unwrap_or(obligation::ObligationKind::NonDefiniteAlternative),
            )?;
            if header.is_some_and(|h| {
                h.owner != frame.owner_entity()
                    || h.callee != frame.callee()
                    || h.event != event.id()
            }) {
                return Err(obligation::ObligationKind::IncompatibleContexts);
            }
            Ok(CallInfo {
                attempt: attempt.id(),
                _event: event.id(),
                _header: header.map(Record::id),
                owner: frame.owner_entity(),
                callee: frame.callee(),
                target: frame.target(),
            })
        })();
        let reason = match result {
            Ok(call) => {
                calls.push(call);
                None
            }
            Err(r) => Some(r),
        };
        out.call_members.insert(CallMember {
            invocation: invocation.id(),
            attempt: attempt.id(),
            header: header.map(Record::id),
            reason,
        })?;
    }
    Ok((calls, charge))
}
// One invocation borrows the complete immutable source and private admission owners. Entries are
// created only for a pair actually consumed by the existing worklist, never for a queued suffix.
struct CallPreparation<'a, 's> {
    data: &'a SummaryData,
    verified: &'a VerifiedBindings,
    discovery: &'a Discovery<'a>,
    signatures: &'s mut SignaturePreparation<'a>,
    calls: BTreeMap<Id<CallBindingAttempt>, PreparedAttempt<'a>>,
    charge: StateCharge,
}
impl<'a, 's> CallPreparation<'a, 's> {
    fn new(
        data: &'a SummaryData,
        verified: &'a VerifiedBindings,
        discovery: &'a Discovery<'a>,
        signatures: &'s mut SignaturePreparation<'a>,
        budget: &ResourceBudget,
    ) -> Self {
        Self {
            data,
            verified,
            discovery,
            signatures,
            calls: BTreeMap::new(),
            charge: StateCharge::new(budget, "summary-prepared-call-slots"),
        }
    }
    fn get(
        &mut self,
        call: &CallInfo,
        budget: &ResourceBudget,
    ) -> Result<&mut PreparedAttempt<'a>, ModelError> {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.calls.entry(call.attempt) {
            self.charge.grow(
                size_of::<Id<CallBindingAttempt>>() + size_of::<PreparedAttempt<'a>>() + 128,
            )?;
            let prepared = PreparedAttempt::new(
                self.data,
                self.verified,
                self.discovery,
                call,
                self.signatures,
                budget,
            )?;
            entry.insert(prepared);
        }
        let prepared = self.calls.get_mut(&call.attempt).expect("prepared above");
        if prepared.info != *call {
            return Err(invalid(
                "Summary prepared call changes its exact admission frame",
            ));
        }
        Ok(prepared)
    }
}
// Signature ingredients are immutable and shared, while every PreparedAttempt still owns its
// exact caller admission, ports, guards and arguments. An unused signature is never realized.
struct SignaturePreparation<'a> {
    data: &'a SummaryData,
    discovery: &'a Discovery<'a>,
    signatures: BTreeMap<Id<Signature>, Arc<PreparedSignature<'a>>>,
    charge: StateCharge,
}
impl<'a> SignaturePreparation<'a> {
    fn new(data: &'a SummaryData, discovery: &'a Discovery<'a>, budget: &ResourceBudget) -> Self {
        Self {
            data,
            discovery,
            signatures: BTreeMap::new(),
            charge: StateCharge::new(budget, "summary-shared-signatures"),
        }
    }
    fn require_header(&self, id: Id<Signature>) -> Result<(), ModelError> {
        if !self.signatures.contains_key(&id) {
            signature_header(self.data, self.discovery, id)?;
        }
        Ok(())
    }
    fn get(
        &mut self,
        id: Id<Signature>,
        budget: &ResourceBudget,
    ) -> Result<Arc<PreparedSignature<'a>>, ModelError> {
        if let Some(signature) = self.signatures.get(&id) {
            return Ok(Arc::clone(signature));
        }
        self.charge
            .grow(size_of::<Id<Signature>>() + size_of::<Arc<PreparedSignature<'a>>>() + 128)?;
        let signature = Arc::new(prepare_signature(self.data, self.discovery, id, budget)?);
        self.signatures.insert(id, Arc::clone(&signature));
        Ok(signature)
    }
}
fn signature_header<'a>(
    data: &'a SummaryData,
    discovery: &Discovery<'a>,
    signature_id: Id<Signature>,
) -> Result<
    (
        &'a Signature,
        source::properties::SymbolProperties,
        &'a declarations::SymbolDeclaration,
    ),
    ModelError,
> {
    let signature = need(&data.bindings.signatures, signature_id)?;
    let symbol = data
        .symbol(signature.symbol)
        .ok_or_else(|| invalid("Summary callee symbol absent"))?;
    let mut declarations = discovery
        .symbol_declarations
        .get(&symbol.id())
        .into_iter()
        .flatten()
        .copied();
    let declaration = declarations
        .next()
        .ok_or_else(|| invalid("Summary callee source declaration absent"))?;
    if declarations.next().is_some() {
        return Err(invalid("Summary callee declaration ambiguous"));
    }
    Ok((signature, symbol, declaration))
}
fn prepare_signature<'a>(
    data: &'a SummaryData,
    discovery: &Discovery<'a>,
    signature_id: Id<Signature>,
    budget: &ResourceBudget,
) -> Result<PreparedSignature<'a>, ModelError> {
    let (signature, symbol, declaration) = signature_header(data, discovery, signature_id)?;
    // Reserve the selected frame before cloning its rows. Unrelated signatures and calls do not
    // inflate each cache entry; all vectors and the temporary membership set stay charged.
    let parameter_count = discovery
        .parameters
        .get(&signature.id())
        .map_or(0, Vec::len);
    let parameter_bytes = parameter_count
        .checked_mul(size_of::<SignatureParameter>() + 128)
        .ok_or_else(|| invalid("Summary prepared signature allowance overflow"))?;
    let parameter_charge = budget.reserve("summary-prepared-signature", parameter_bytes)?;
    let mut parameters = Vec::with_capacity(parameter_count);
    parameters.extend(
        discovery
            .parameters
            .get(&signature.id())
            .into_iter()
            .flatten()
            .map(|p| (*p).clone()),
    );
    parameters.sort_by_key(|p| p.ordinal);
    let parameter_ids = parameters.iter().map(Record::id).collect::<BTreeSet<_>>();
    let link_count = parameter_ids
        .iter()
        .map(|id| discovery.links.get(id).map_or(0, Vec::len))
        .sum::<usize>();
    let mut signature_charge = parameter_charge;
    signature_charge.try_resize(
        parameter_bytes
            .checked_add(
                link_count
                    .checked_mul(
                        size_of::<declarations::ParameterDeclaration>()
                            + size_of::<(usize, &declarations::ParameterDeclaration)>(),
                    )
                    .ok_or_else(|| invalid("Summary prepared links allowance overflow"))?,
            )
            .ok_or_else(|| invalid("Summary prepared links allowance overflow"))?,
    )?;
    let mut links = Vec::with_capacity(link_count);
    let mut selected_links = Vec::with_capacity(link_count);
    selected_links.extend(
        parameter_ids
            .iter()
            .flat_map(|id| discovery.links.get(id).into_iter().flatten())
            .copied(),
    );
    // Source ordinals are unique. In-place sorting preserves their exact order without
    // admitting another stable-sort buffer or geometric collection capacity.
    selected_links.sort_unstable_by_key(|(ordinal, _)| *ordinal);
    links.extend(selected_links.into_iter().map(|(_, row)| row.clone()));
    drop(parameter_ids);
    // The membership set is gone; only the selected row vectors remain retained.
    signature_charge.try_resize(
        parameters
            .capacity()
            .checked_mul(size_of::<SignatureParameter>())
            .and_then(|n| {
                n.checked_add(
                    links
                        .capacity()
                        .checked_mul(size_of::<declarations::ParameterDeclaration>())?,
                )
            })
            .and_then(|n| {
                n.checked_add(size_of::<PreparedSignature<'a>>() + 2 * size_of::<usize>())
            })
            .ok_or_else(|| invalid("Summary retained signature allowance overflow"))?,
    )?;
    Ok(PreparedSignature {
        symbol,
        declaration,
        parameters,
        links,
        _reservation: signature_charge,
    })
}
struct PreparedAttempt<'a> {
    info: CallInfo,
    caller: CallerFrame<'a>,
    site: source::properties::OccurrenceProperties,
    target: &'a CallTarget,
    qualification: &'a AssertionQualification,
    destination: &'a CallDestination,
    receiver: &'a Receiver,
    binding: CallBindingFrame<'a>,
    arguments: Vec<CallArgument>,
    callee: PreparedCallee<'a>,
    _arguments_charge: Box<dyn Reservation>,
}
impl<'a> PreparedAttempt<'a> {
    fn new(
        data: &'a SummaryData,
        verified: &'a VerifiedBindings,
        discovery: &'a Discovery<'a>,
        call: &CallInfo,
        signatures: &mut SignaturePreparation<'a>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let attempt = need(&data.binding_output.attempts, call.attempt)?;
        let target = need(&data.bindings.targets, call.target)?;
        let bound = verified
            .bound(call.attempt)
            .ok_or_else(|| invalid("Summary bound call absent"))?;
        let signature_id = bound.bound().signature();
        signatures.require_header(signature_id)?;
        let admission = verified
            .composition(call.attempt)
            .ok_or_else(|| invalid("Summary composition admission absent"))?;
        let mut callers = discovery
            .declaration_symbols
            .get(&admission.owner_declaration())
            .into_iter()
            .flatten()
            .copied();
        let caller_declaration = callers
            .next()
            .ok_or_else(|| invalid("Summary caller declaration absent"))?;
        if callers.next().is_some() {
            return Err(invalid("Summary caller declaration ambiguous"));
        }
        // Preserve first-use refusal order: resolve both declarations before allocating the
        // selected signature. Later attempts reuse only these immutable ingredients.
        let signature = signatures.get(signature_id, budget)?;
        let syntax = attempt
            .syntax
            .ok_or_else(|| invalid("Summary call syntax absent"))?;
        let argument_count = discovery.arguments.get(&syntax).map_or(0, Vec::len);
        let argument_bytes = discovery
            .arguments
            .get(&syntax)
            .into_iter()
            .flatten()
            .try_fold(
                argument_count
                    .checked_mul(size_of::<CallArgument>())
                    .ok_or_else(|| invalid("Summary prepared arguments allowance overflow"))?,
                |bytes, argument| {
                    bytes
                        .checked_add(argument.heap_bytes())
                        .ok_or_else(|| invalid("Summary prepared arguments allowance overflow"))
                },
            )?;
        let sort_bytes = argument_count
            .checked_mul(size_of::<CallArgument>())
            .ok_or_else(|| invalid("Summary prepared arguments sort allowance overflow"))?;
        let mut arguments_charge = budget.reserve(
            "summary-prepared-arguments",
            argument_bytes
                .checked_add(sort_bytes)
                .ok_or_else(|| invalid("Summary prepared arguments allowance overflow"))?,
        )?;
        let mut arguments = Vec::with_capacity(argument_count);
        arguments.extend(
            discovery
                .arguments
                .get(&syntax)
                .into_iter()
                .flatten()
                .map(|a| (*a).clone()),
        );
        arguments.sort_by_key(|a| a.ordinal);
        arguments_charge.try_resize(argument_bytes)?;
        let site = data
            .occurrence(target.site)
            .ok_or_else(|| invalid("Summary call occurrence absent"))?;
        let qualification = need(&data.bindings.qualifications, target.qualification)?;
        let destination = need(&data.bindings.destinations, target.destination)?;
        let receiver = need(&data.bindings.receivers, target.receiver)?;
        let binding =
            CallBindingFrame::new(verified, call.attempt, &data.bindings, &data.binding_output)
                .map_err(|r| invalid(format!("Summary checked call refused: {r:?}")))?;
        Ok(Self {
            info: *call,
            caller: CallerFrame {
                declaration: caller_declaration,
                owner: call.owner,
            },
            site,
            target,
            qualification,
            destination,
            receiver,
            binding,
            arguments,
            callee: PreparedCallee::new(
                binding,
                signature,
                discovery
                    .guard_bindings
                    .get(&call.attempt)
                    .map_or(&[], Vec::as_slice),
            ),
            _arguments_charge: arguments_charge,
        })
    }
    fn compose(
        &mut self,
        caller: &WorkBranch,
        callee: &WorkBranch,
        vocabulary: &Vocabulary,
        guards: &BTreeMap<Id<EvaluationAtom>, CheckedStability>,
        budget: &ResourceBudget,
    ) -> Result<CompositionResults, ModelError> {
        self.callee.compose(
            caller,
            callee,
            &CallFrame {
                site: &self.site,
                target: self.target,
                qualification: self.qualification,
                destination: self.destination,
                receiver: self.receiver,
                binding: Some(self.binding),
                arguments: &self.arguments,
            },
            &self.caller,
            (guards, &vocabulary.catalog()),
            budget,
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WorkItem {
    Alias(
        ProofId,
        Id<FlowValueSupport>,
        Id<FlowReachingSupport>,
        Id<FlowDefinitionSupport>,
        Id<FlowUseSupport>,
    ),
    Call(Id<CallBindingAttempt>, ProofId, ProofId),
    Path(
        ProofId,
        Id<FlowValueSupport>,
        Id<FlowValuePathSupport>,
        Id<FlowCallStep>,
    ),
}
impl HeapSize for WorkItem {}
struct Progress {
    branches: ChargedMap<ProofId, WorkBranch>,
    by_owner: ChargedMap<Id<EntityRef>, BTreeSet<ProofId>>,
    frontier: summary_worklist::Frontier<ContentHash, ProofId>,
    endpoints: EndpointIndex,
    charge: StateCharge,
}
impl Progress {
    fn new(limits: super::configuration::SummaryLimits, b: &ResourceBudget) -> Self {
        Self {
            branches: Default::default(),
            by_owner: Default::default(),
            frontier: summary_worklist::Frontier::new(limits, b),
            endpoints: EndpointIndex::default(),
            charge: StateCharge::new(b, "finite-summary-progress"),
        }
    }
    fn admit(
        &mut self,
        branch: &WorkBranch,
        vocabulary: &Vocabulary,
    ) -> Result<summary_worklist::Admission, ModelError> {
        let result = self.frontier.insert(
            branch.semantic(),
            summary_worklist::Representative {
                witness: branch.id(),
                cost: branch.cost,
            },
        )?;
        if !matches!(result, summary_worklist::Admission::Refused(_))
            && !self.branches.contains_key(&branch.id())
        {
            self.by_owner
                .insert_set(&mut self.charge, branch.descriptor().owner, branch.id())?;
            self.branches
                .insert(&mut self.charge, branch.id(), branch.clone())?;
        }
        if result == summary_worklist::Admission::Advanced {
            self.index_current(branch, vocabulary)?;
        }
        Ok(result)
    }
    fn index_current(
        &mut self,
        branch: &WorkBranch,
        vocabulary: &Vocabulary,
    ) -> Result<(), ModelError> {
        self.endpoints.track(
            branch,
            vocabulary,
            &self.branches,
            &self.frontier,
            &mut self.charge,
        )
    }
    fn current(&self, id: ProofId) -> bool {
        self.branches
            .get(&id)
            .is_some_and(|b| self.frontier.current(&b.semantic(), id))
    }
}
// Immutable invocation-local correspondence. Every bucket retains canonical source-row order
// and all alternatives; only the worklist decides semantic/Pareto redundancy.
struct Discovery<'a> {
    seeds: BTreeMap<Id<SummaryOrigin>, usize>,
    headers: BTreeMap<Id<CallBindingAttempt>, Vec<&'a SourceCallHeader>>,
    symbol_declarations: BTreeMap<Id<ProviderSymbol>, Vec<&'a declarations::SymbolDeclaration>>,
    declaration_symbols: BTreeMap<Id<source::Occurrence>, Vec<&'a declarations::SymbolDeclaration>>,
    parameters: BTreeMap<Id<Signature>, Vec<&'a SignatureParameter>>,
    links: BTreeMap<Id<SignatureParameter>, Vec<(usize, &'a declarations::ParameterDeclaration)>>,
    arguments: BTreeMap<Id<CallSyntax>, Vec<&'a CallArgument>>,
    guard_bindings: BTreeMap<Id<CallBindingAttempt>, Vec<&'a CallBinding>>,
    events: BTreeMap<
        (Id<source::Occurrence>, Id<attribution::AnalysisContext>),
        Vec<&'a normalized::events::NormalizedCallEvent>,
    >,
    entry_conditions: PreparedConditions<'a>,
    vocabulary_conditions: PreparedConditions<'a>,
    local_supports: BTreeMap<
        Id<transfer::local::TransferAlternative>,
        Vec<&'a transfer::local::TransferSupport>,
    >,
    model_supports: BTreeMap<
        Id<transfer::model::TransferAlternative>,
        Vec<&'a transfer::model::TransferSupport>,
    >,
    definitions: BTreeMap<Id<source::Occurrence>, Vec<&'a FlowDefinitionObservation>>,
    reaching: BTreeMap<Id<ReachingDefinition>, Vec<&'a FlowReachingObservation>>,
    values: BTreeMap<Id<FlowUse>, Vec<&'a FlowValueObservation>>,
    uses: BTreeMap<Id<FlowUse>, Vec<&'a FlowUseObservation>>,
    value_supports: BTreeMap<Id<FlowValueObservation>, Vec<&'a FlowValueSupport>>,
    reaching_supports: BTreeMap<Id<FlowReachingObservation>, Vec<&'a FlowReachingSupport>>,
    definition_supports: BTreeMap<Id<FlowDefinitionObservation>, Vec<&'a FlowDefinitionSupport>>,
    use_supports: BTreeMap<Id<FlowUseObservation>, Vec<&'a FlowUseSupport>>,
    call_steps: BTreeMap<Id<source::Occurrence>, Vec<&'a FlowCallStep>>,
    value_paths: BTreeMap<Id<FlowCallPath>, Vec<&'a FlowValuePathObservation>>,
    path_supports: BTreeMap<Id<FlowValuePathObservation>, Vec<&'a FlowValuePathSupport>>,
    owners: BTreeSet<(Id<source::Occurrence>, Id<EntityRef>)>,
    callers: BTreeMap<Id<EntityRef>, Vec<usize>>,
    callees: BTreeMap<Id<EntityRef>, Vec<usize>>,
    attempts: BTreeMap<Id<CallBindingAttempt>, usize>,
    _reservation: Box<dyn Reservation>,
}
impl<'a> Discovery<'a> {
    // Terminal closure still needs condition lowering; pair correspondence has no later consumer.
    fn into_vocabulary_conditions(self) -> PreparedConditions<'a> {
        self.vocabulary_conditions
    }
    fn new(
        data: &'a SummaryData,
        calls: &[CallInfo],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut count = data
            .binding_output
            .attempts
            .len()
            .checked_mul(3)
            .ok_or_else(|| invalid("Summary discovery overflow"))?;
        macro_rules! count {($($rows:expr),*) => {$(count = count.checked_add($rows.len()).ok_or_else(|| invalid("Summary discovery overflow"))?;)*};}
        count!(
            data.headers,
            data.entry.symbol_declarations,
            data.entry.symbol_declarations,
            data.bindings.parameters,
            data.entry.declarations,
            data.bindings.arguments,
            data.binding_output.bindings,
            data.bindings.event_events,
            data.entry.definition_observations,
            data.entry.reaching,
            data.entry.values,
            data.entry.use_observations,
            data.entry.value_supports,
            data.entry.reaching_supports,
            data.entry.definition_supports,
            data.entry.use_supports,
            data.path.call_steps,
            data.path.value_paths,
            data.path.value_path_supports,
            data.entry.owners,
            data.local_supports,
            data.model_supports
        );
        let reservation = budget.reserve(
            "summary-discovery",
            count
                .checked_mul(256)
                .ok_or_else(|| invalid("Summary discovery allowance overflow"))?,
        )?;
        macro_rules! index {
            ($rows:expr, $key:expr) => {{
                let mut index = BTreeMap::<_, Vec<_>>::new();
                for row in $rows.iter() {
                    index.entry(($key)(row)).or_default().push(row);
                }
                index
            }};
        }
        let mut definitions = BTreeMap::<_, Vec<_>>::new();
        for row in data.entry.definition_observations.iter() {
            if let Some(site) = row.value {
                definitions.entry(site).or_default().push(row);
            }
        }
        let mut callers = BTreeMap::<_, Vec<_>>::new();
        let mut callees = BTreeMap::<_, Vec<_>>::new();
        let mut attempts = BTreeMap::new();
        for (index, call) in calls.iter().enumerate() {
            callers.entry(call.owner).or_default().push(index);
            callees.entry(call.callee).or_default().push(index);
            attempts.insert(call.attempt, index);
        }
        let mut links = BTreeMap::<_, Vec<_>>::new();
        for (ordinal, row) in data.entry.declarations.iter().enumerate() {
            links.entry(row.parameter).or_default().push((ordinal, row));
        }
        Ok(Self {
            seeds: BTreeMap::new(),
            headers: index!(data.headers, |r: &SourceCallHeader| r.attempt),
            symbol_declarations: index!(
                data.entry.symbol_declarations,
                |r: &declarations::SymbolDeclaration| r.symbol
            ),
            declaration_symbols: index!(
                data.entry.symbol_declarations,
                |r: &declarations::SymbolDeclaration| r.declaration
            ),
            parameters: index!(data.bindings.parameters, |r: &SignatureParameter| r
                .signature),
            links,
            arguments: index!(data.bindings.arguments, |r: &CallArgument| r.call),
            guard_bindings: index!(data.binding_output.bindings, |r: &CallBinding| r.attempt),
            events: index!(
                data.bindings.event_events,
                |r: &normalized::events::NormalizedCallEvent| (r.site, r.context)
            ),
            entry_conditions: PreparedConditions::new(data.entry.condition_nodes.iter(), budget)?,
            vocabulary_conditions: PreparedConditions::new(data.vocabulary.nodes.values(), budget)?,
            local_supports: index!(
                data.local_supports,
                |r: &transfer::local::TransferSupport| r.assertion
            ),
            model_supports: index!(
                data.model_supports,
                |r: &transfer::model::TransferSupport| r.assertion
            ),
            definitions,
            reaching: index!(data.entry.reaching, |r: &FlowReachingObservation| r.target),
            values: index!(data.entry.values, |r: &FlowValueObservation| r.use_),
            uses: index!(data.entry.use_observations, |r: &FlowUseObservation| r.use_),
            value_supports: index!(data.entry.value_supports, |r: &FlowValueSupport| r
                .assertion),
            reaching_supports: index!(data.entry.reaching_supports, |r: &FlowReachingSupport| r
                .assertion),
            definition_supports: index!(
                data.entry.definition_supports,
                |r: &FlowDefinitionSupport| r.assertion
            ),
            use_supports: index!(data.entry.use_supports, |r: &FlowUseSupport| r.assertion),
            call_steps: index!(data.path.call_steps, |r: &FlowCallStep| r.call),
            value_paths: index!(data.path.value_paths, |r: &FlowValuePathObservation| r.path),
            path_supports: index!(data.path.value_path_supports, |r: &FlowValuePathSupport| r
                .assertion),
            owners: data
                .entry
                .owners
                .iter()
                .map(|r| (r.occurrence, r.entity))
                .collect(),
            callers,
            callees,
            attempts,
            _reservation: reservation,
        })
    }
}
fn path_candidates(
    data: &SummaryData,
    discovery: &Discovery<'_>,
    vocabulary: &Vocabulary,
    branch: &WorkBranch,
    mut consume: impl FnMut(WorkItem) -> Result<bool, ModelError>,
) -> Result<(), ModelError> {
    let descriptor = branch.descriptor();
    let Some(place) = vocabulary.places.get(&descriptor.output) else {
        return Err(invalid("summary branch output absent"));
    };
    if place.path != AccessPath::empty().id() {
        return Ok(());
    }
    let Some(PlaceRoot::Occurrence { occurrence: site }) = vocabulary.roots.get(&place.root) else {
        return Ok(());
    };
    let Some(input) = vocabulary.places.get(&descriptor.input) else {
        return Err(invalid("summary branch input absent"));
    };
    let Some(PlaceRoot::Entry { declaration }) = vocabulary.roots.get(&input.root) else {
        return Ok(());
    };
    'aliases: for definition in discovery
        .definitions
        .get(site)
        .into_iter()
        .flatten()
        .copied()
    {
        let target = ReachingDefinition::Bound {
            definition: definition.definition,
        };
        for reaching in discovery
            .reaching
            .get(&target.id())
            .into_iter()
            .flatten()
            .copied()
        {
            for value in discovery
                .values
                .get(&reaching.use_)
                .into_iter()
                .flatten()
                .copied()
                .filter(|v| !v.through_call)
            {
                for observed in discovery.uses.get(&value.use_).into_iter().flatten() {
                    for vs in discovery
                        .value_supports
                        .get(&value.id())
                        .into_iter()
                        .flatten()
                    {
                        for rs in discovery
                            .reaching_supports
                            .get(&reaching.id())
                            .into_iter()
                            .flatten()
                        {
                            for ds in discovery
                                .definition_supports
                                .get(&definition.id())
                                .into_iter()
                                .flatten()
                            {
                                for us in discovery
                                    .use_supports
                                    .get(&observed.id())
                                    .into_iter()
                                    .flatten()
                                {
                                    if consume(WorkItem::Alias(
                                        branch.id(),
                                        vs.id(),
                                        rs.id(),
                                        ds.id(),
                                        us.id(),
                                    ))? {
                                        break 'aliases;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    'paths: for step in discovery
        .call_steps
        .get(site)
        .into_iter()
        .flatten()
        .copied()
    {
        for path in discovery
            .value_paths
            .get(&step.path)
            .into_iter()
            .flatten()
            .copied()
        {
            let value = need(&data.entry.values, path.value)?;
            let use_ = need(&data.entry.uses, value.use_)?;
            let original = need(&data.entry.places, use_.place)?;
            if data.entry.roots.get(original.root)
                != Some(&PlaceRoot::Formal {
                    declaration: *declaration,
                })
                || !discovery
                    .owners
                    .contains(&(use_.occurrence, descriptor.owner))
            {
                continue;
            }
            for vs in discovery
                .value_supports
                .get(&value.id())
                .into_iter()
                .flatten()
            {
                for ps in discovery
                    .path_supports
                    .get(&path.id())
                    .into_iter()
                    .flatten()
                {
                    if consume(WorkItem::Path(branch.id(), vs.id(), ps.id(), step.id()))? {
                        break 'paths;
                    }
                }
            }
        }
    }
    Ok(())
}
impl EnqueueContext<'_> {
    fn call_candidates(
        &self,
        new: ProofId,
        mut consume: impl FnMut(WorkItem) -> Result<bool, ModelError>,
    ) -> Result<(), ModelError> {
        let Self {
            discovery,
            vocabulary,
            calls,
            component,
            progress,
            ..
        } = *self;
        let branch = progress
            .branches
            .get(&new)
            .ok_or_else(|| invalid("queued summary proof absent"))?;
        if !progress.current(new) {
            return Ok(());
        }
        let mut caller = discovery
            .callers
            .get(&branch.descriptor().owner)
            .into_iter()
            .flatten()
            .peekable();
        let mut callee = discovery
            .callees
            .get(&branch.descriptor().owner)
            .into_iter()
            .flatten()
            .peekable();
        // Merge endpoint buckets in the former global call order, including recursive self-calls once.
        while caller.peek().is_some() || callee.peek().is_some() {
            let index = match (caller.peek(), callee.peek()) {
                (Some(a), Some(b)) => match a.cmp(b) {
                    std::cmp::Ordering::Less => *caller.next().unwrap(),
                    std::cmp::Ordering::Greater => *callee.next().unwrap(),
                    std::cmp::Ordering::Equal => {
                        callee.next();
                        *caller.next().unwrap()
                    }
                },
                (Some(_), None) => *caller.next().unwrap(),
                (None, Some(_)) => *callee.next().unwrap(),
                (None, None) => unreachable!(),
            };
            let call = &calls[index];
            if component.binary_search(&call.owner).is_err() {
                continue;
            }
            if call.owner == branch.descriptor().owner
                && enqueue_groups::caller_eligible(vocabulary, branch)
                && let Some(rows) = progress.by_owner.get(&call.callee)
            {
                for other in rows {
                    if progress.current(*other)
                        && !consume(WorkItem::Call(call.attempt, new, *other))?
                    {
                        return Ok(());
                    }
                }
            }
            if call.callee == branch.descriptor().owner
                && let Some(rows) = progress.by_owner.get(&call.owner)
            {
                for other in rows {
                    if progress.current(*other)
                        && enqueue_groups::caller_eligible(vocabulary, &progress.branches[other])
                        && !consume(WorkItem::Call(call.attempt, *other, new))?
                    {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}

struct ResidualContext<'a> {
    discovery: &'a Discovery<'a>,
    data: &'a SummaryData,
    invocation: &'a owner::AnalysisInvocation,
    component: ContentHash,
    progress: &'a Progress,
    seeds: &'a [WorkBranch],
}

fn residual(
    residual_context: ResidualContext<'_>,
    item: WorkItem,
    reason: obligation::ObligationKind,
    out: &mut SummaryRecords,
) -> Result<(), ModelError> {
    materialize_residual(residual_context, item, reason, out).map(|_| ())
}

/// The published residual tuple is the exact post-refusal equivalence class. Proof identity
/// and original queue admissions remain separate. Exhausted grouping supplies an actual
/// refused witness while preserving pending/processed exceptions; this helper also deduplicates
/// rich products from other explicit refusal routes.
fn materialize_residual(
    residual_context: ResidualContext<'_>,
    item: WorkItem,
    reason: obligation::ObligationKind,
    out: &mut SummaryRecords,
) -> Result<bool, ModelError> {
    let ResidualContext {
        discovery,
        data,
        invocation,
        component,
        progress,
        seeds,
    } = residual_context;
    let (origin, event, phase, input, output) = match item {
        WorkItem::Call(attempt, caller, callee) => {
            let caller = &progress.branches[&caller];
            let callee = &progress.branches[&callee];
            let attempt = need(&data.binding_output.attempts, attempt)?;
            let event = need(&data.bindings.event_events, attempt.event)?;
            let alternative = need(&data.bindings.event_alternatives, attempt.alternative)?;
            let target = need(
                &data.bindings.targets,
                need(&data.bindings.event_alternative_sources, alternative.source)?.target(),
            )?;
            (
                caller.origin,
                event.id(),
                target.phase,
                caller.descriptor().input,
                callee.descriptor().output,
            )
        }
        WorkItem::Alias(proof, _, _, _, _) => {
            let branch = &progress.branches[&proof];
            let site = branch
                .descriptor()
                .call_site
                .ok_or_else(|| invalid("alias boundary original call absent"))?;
            let event = discovery
                .events
                .get(&(site, invocation.context))
                .and_then(|rows| rows.first())
                .copied()
                .ok_or_else(|| invalid("alias boundary event absent"))?;
            (
                branch.origin,
                event.id(),
                CallPhase::Call,
                branch.descriptor().input,
                branch.descriptor().output,
            )
        }
        WorkItem::Path(proof, _, _, step) => {
            let branch = &progress.branches[&proof];
            let step = need(&data.path.call_steps, step)?;
            let mut events = discovery
                .events
                .get(&(step.call, invocation.context))
                .into_iter()
                .flatten()
                .copied();
            let event = events
                .next()
                .ok_or_else(|| invalid("summary path boundary event absent"))?;
            if events.next().is_some() {
                return Err(invalid("summary path boundary event ambiguous"));
            }
            (
                branch.origin,
                event.id(),
                CallPhase::Call,
                branch.descriptor().input,
                branch.descriptor().output,
            )
        }
    };
    let seed = discovery
        .seeds
        .get(&origin)
        .and_then(|index| seeds.get(*index))
        .ok_or_else(|| invalid("summary origin seed absent"))?;
    let q = AssertionQualification {
        approximation: Approximation::Over,
        ..seed.qualification().clone()
    };
    let row = SummaryResidual {
        invocation: invocation.id(),
        component,
        origin,
        event,
        channel: analysis::AnalysisChannel::Value,
        phase,
        input,
        output,
        reason,
        qualification: q.id(),
    };
    if let Some(existing) = out.residuals.get(row.id()) {
        if existing != &row {
            return Err(ModelError::Conflict(SummaryResidual::NAME));
        }
        return Ok(false);
    }
    out.vocabulary.qualified(&q, seed.condition())?;
    out.residuals.insert(row)?;
    let subject = owner::ObligationSubject::SourceCall {
        occurrence: need(&data.bindings.event_events, event)?.site,
    };
    out.obligations.insert(owner::AnalysisObligation {
        invocation: invocation.id(),
        subject: subject.id(),
        channel: analysis::AnalysisChannel::Value,
        phase,
        qualification: q.id(),
        reason,
        responsible: analysis::AnalysisMethod::Summaries,
    })?;
    out.subjects.insert(subject)?;
    Ok(true)
}
struct EnqueueContext<'a> {
    discovery: &'a Discovery<'a>,
    data: &'a SummaryData,
    vocabulary: &'a Vocabulary,
    calls: &'a [CallInfo],
    component: &'a [Id<EntityRef>],
    component_id: ContentHash,
    invocation: &'a owner::AnalysisInvocation,
    progress: &'a Progress,
    seeds: &'a [WorkBranch],
}

fn enqueue(
    enqueue_context: EnqueueContext<'_>,
    new: ProofId,
    queue: &mut summary_worklist::WorkQueue<WorkItem>,
    processed: &charged::ChargedSet<WorkItem>,
    exhausted: &mut ExhaustedGroups,
    out: &mut SummaryRecords,
) -> Result<usize, ModelError> {
    let EnqueueContext {
        discovery,
        data,
        vocabulary,
        component_id,
        invocation,
        progress,
        seeds,
        ..
    } = enqueue_context;
    if !progress.current(new) {
        return Ok(0);
    }
    let mut materialized = 0;
    // Preserve the original canonical admissions exactly until the permanent queue limit.
    if !queue.exhausted() {
        enqueue_context.call_candidates(new, |item| {
            exhausted.examined += 1;
            if !processed.contains(&item) {
                let _ = queue.insert(item)?;
            }
            Ok(!queue.exhausted())
        })?;
    }
    if queue.exhausted() {
        materialized += exhausted.calls(&enqueue_context, new, queue, processed, out)?;
    }
    path_candidates(
        data,
        discovery,
        vocabulary,
        &progress.branches[&new],
        |item| {
            exhausted.examined += 1;
            if processed.contains(&item) || queue.contains(&item) {
                return Ok(false);
            }
            if queue.exhausted() {
                materialized += usize::from(materialize_residual(
                    ResidualContext {
                        discovery,
                        data,
                        invocation,
                        component: component_id,
                        progress,
                        seeds,
                    },
                    item,
                    obligation::ObligationKind::SummaryPairWorkLimit,
                    out,
                )?);
                // Alias ignores all support IDs; Path ignores support/step IDs, and every
                // step in this bucket has the same call site. Each is one exact residual
                // group, but their two events may differ. Only an actual refusal ends it.
                return Ok(true);
            }
            let _ = queue.insert(item)?;
            Ok(false)
        },
    )?;
    Ok(materialized)
}

fn retain_proof(
    out: &mut SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    branch: &WorkBranch,
) -> Result<(), ModelError> {
    out.premises.insert(branch.premise.clone())?;
    out.costs
        .insert(SummaryProofCost::new(branch.premise.id(), branch.cost))?;
    out.proof_origins.insert(ProofOrigin {
        invocation: invocation.id(),
        proof: branch.premise.id(),
        origin: branch.origin,
        semantic: branch.semantic(),
    })?;
    Ok(())
}
fn origin_boundary(
    out: &mut SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    component: ContentHash,
    branch: &WorkBranch,
    reason: obligation::ObligationKind,
) -> Result<(), ModelError> {
    let q = AssertionQualification {
        approximation: Approximation::Over,
        ..branch.qualification().clone()
    };
    out.vocabulary.qualified(&q, branch.condition())?;
    out.origin_boundaries.insert(OriginBoundary {
        invocation: invocation.id(),
        component,
        origin: branch.origin,
        transfer: branch.branch.key().id(),
        reason,
        qualification: q.id(),
    })?;
    let key = branch.branch.key();
    out.keys.insert(key.clone())?;
    let subject = owner::ObligationSubject::SummaryTransfer { transfer: key.id() };
    out.obligations.insert(owner::AnalysisObligation {
        invocation: invocation.id(),
        subject: subject.id(),
        channel: analysis::AnalysisChannel::Value,
        phase: CallPhase::Call,
        qualification: q.id(),
        reason,
        responsible: analysis::AnalysisMethod::Summaries,
    })?;
    out.subjects.insert(subject)?;
    Ok(())
}
fn publish(
    out: &mut SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    progress: &Progress,
    component: &[Id<EntityRef>],
    component_id: ContentHash,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut members =
        charged::ChargedMap::<(Id<TransferKey>, ContentHash), BTreeSet<ProofId>>::default();
    let mut charge = StateCharge::new(budget, "summary-publication-members");
    for branch in component.iter()
        .filter_map(|owner| progress.by_owner.get(owner))
        .flatten()
        .map(|id| &progress.branches[id])
        .filter(|b| matches!(b.premise,
            SummaryPremise::Witness { .. } | SummaryPremise::Path { .. } | SummaryPremise::Captured { .. }
        ))
    {
        members.insert_set(
            &mut charge,
            (
                branch.branch.key().id(),
                analysis::support::alternative_basis(branch.qualification()),
            ),
            branch.id(),
        )?;
    }
    for ids in members.values() {
        match transfer::merge(
            ids.iter().map(|id| progress.branches[id].branch.clone()),
            budget,
        ) {
            Ok(merged) => {
                publish_members(out, invocation, definition, progress, ids, &merged, budget)?
            }
            Err(TransferError::Resource(error)) => return Err(error),
            Err(TransferError::Obligation(reason)) => {
                if reason == obligation::ObligationKind::ConflictingProof {
                    return Err(invalid("Summary aggregate descriptor conflict"));
                }
                // Bounded OR cannot erase already proven alternatives. Retain each exact finite member
                // and a separate over-approximate boundary; no widened result becomes behavioral proof.
                for id in ids {
                    let branch = &progress.branches[id];
                    origin_boundary(out, invocation, component_id, branch, reason)?;
                    let singleton = BTreeSet::from([*id]);
                    let merged = transfer::merge([branch.branch.clone()], budget).map_err(|e| {
                        invalid(format!("Summary singleton publication refused: {e:?}"))
                    })?;
                    publish_members(
                        out, invocation, definition, progress, &singleton, &merged, budget,
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn publish_members(
    out: &mut SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    progress: &Progress,
    ids: &BTreeSet<ProofId>,
    merged: &transfer::TransferAggregation<TransferKey>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if merged.len() != 1 {
        return Err(invalid("Summary aggregate key changed"));
    }
    let merged = &merged[0];
    let subject = owner::ObligationSubject::SummaryTransfer {
        transfer: merged.key.id(),
    };
    let _evidence = budget.reserve("summary-aggregate-evidence", ids.len().saturating_mul(1024))?;
    let mut proof_sources = Rows::new(budget);
    let mut source_branches = ChargedMap::default();
    let mut source_charge = StateCharge::new(budget, "summary-publication-proof-correspondence");
    for id in ids {
        let branch = &progress.branches[id];
        let source = match branch.premise {
            SummaryPremise::Witness { witness } => {
                owner::SupportSource::TransferWitness { witness }
            }
            SummaryPremise::Path { witness } => owner::SupportSource::PathWitness { witness },
            SummaryPremise::Captured { witness } => {
                owner::SupportSource::CaptureWitness { witness }
            }
            _ => unreachable!(),
        };
        if !source_branches.contains_key(&source.id()) {
            source_branches.insert(&mut source_charge, source.id(), *id)?;
        }
        proof_sources.insert(source)?;
    }
    let mut evidence = Vec::new();
    for source in proof_sources.iter() {
        let branch = &progress.branches[&source_branches[&source.id()]];
        evidence.push(match branch.premise {
            SummaryPremise::Witness { witness } => owner::support::EvidencePremise::derived(
                source,
                need(&out.witnesses, witness)?,
                branch.qualification(),
                branch.condition(),
            )?,
            SummaryPremise::Path { witness } => owner::support::EvidencePremise::derived(
                source,
                need(&out.path_witnesses, witness)?,
                branch.qualification(),
                branch.condition(),
            )?,
            SummaryPremise::Captured { witness } => owner::support::EvidencePremise::derived(
                source,
                need(&out.capture_witnesses, witness)?,
                branch.qualification(),
                branch.condition(),
            )?,
            _ => unreachable!(),
        });
    }
    let (derivation, proposition, premises, qualified) = owner::AnalysisDerivation::emit(
        invocation,
        definition,
        subject.id(),
        analysis::AnalysisChannel::Value,
        CallPhase::Call,
        analysis::support::QualificationOperation::AlternativeUnion,
        &evidence,
        budget,
    )?;
    if qualified.condition.id() != merged.condition.id() {
        return Err(invalid(
            "Summary final qualification disagrees with owned transfer union",
        ));
    }
    let generated = owner::SupportSource::AnalysisDerivation {
        derivation: derivation.id(),
    };
    let alternative = merged.key.alternative(&qualified.qualification);
    out.keys.insert(merged.key.clone())?;
    out.alternatives.insert(alternative.clone())?;
    out.supports.insert(TransferSupport {
        assertion: alternative.id(),
        source: generated.id(),
    })?;
    out.subjects.insert(subject)?;
    for source in proof_sources.iter() {
        out.sources.insert(source.clone())?;
    }
    out.sources.insert(generated)?;
    out.propositions.insert(proposition)?;
    out.derivations.insert(derivation)?;
    for member in premises {
        out.derivation_premises.insert(member)?;
    }
    out.vocabulary
        .qualified(&qualified.qualification, &qualified.condition)?;
    for id in ids {
        match progress.branches[id].premise {
            SummaryPremise::Witness { witness } => {
                out.contributions.insert(SummaryContribution {
                    alternative: alternative.id(),
                    witness,
                })?;
            }
            SummaryPremise::Path { witness } => {
                out.path_contributions.insert(SummaryPathContribution {
                    alternative: alternative.id(),
                    witness,
                })?;
            }
            SummaryPremise::Captured { witness } => {
                out.capture_contributions.insert(
                    super::summary_capture::SummaryCaptureContribution {
                        alternative: alternative.id(),
                        witness,
                    },
                )?;
            }
            _ => unreachable!(),
        }
    }
    for influence in out.influences.iter() {
        if qualified.condition.support().contains(&influence.atom) {
            let iq = out
                .vocabulary
                .qualifications
                .get(&influence.qualification)
                .ok_or_else(|| invalid("Summary influence qualification absent"))?;
            if (iq.scope, iq.context, iq.assumptions)
                == (
                    qualified.qualification.scope,
                    qualified.qualification.context,
                    qualified.qualification.assumptions,
                )
            {
                out.selections.insert(Selection {
                    influence: influence.id(),
                    atom: influence.atom,
                    alternative: alternative.id(),
                    transfer: merged.key.id(),
                })?;
            }
        }
    }
    Ok(())
}

pub fn produce(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    graph: Option<&MaterializedGraph>,
    budget: &ResourceBudget,
) -> Result<SummaryRecords, ModelError> {
    let verified = if profile == stages::Profile::Behavioral {
        Some(normalized::binding_normalization::prepare(
            &data.bindings,
            &data.binding_output,
            budget,
        )?)
    } else {
        None
    };
    produce_with_application(
        data,
        invocation,
        definition,
        profile,
        graph,
        budget,
        verified.as_ref(),
        None,
        None,
    )
}
/// Borrow the binding owner's admitted application index across Summary invocations.
#[allow(
    clippy::too_many_arguments,
    reason = "Summary configuration, shared frontier, binding admission and actual Local owner are independent semantic inputs."
)]
pub fn produce_prepared(
    data: &SummaryData,
    invocation: &analysis::summary::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    graph: Option<&MaterializedGraph>,
    budget: &ResourceBudget,
    verified: Option<&VerifiedBindings>,
    actual: Option<&local_semantics::ProducedLocal>,
) -> Result<SummaryRecords, ModelError> {
    if profile == stages::Profile::Behavioral && actual.is_none() {
        return Err(invalid("Summary actual Local owner values absent"));
    }
    produce_with_application(
        data, invocation, definition, profile, graph, budget, verified, actual, None,
    )
}
/// The SCC value observes complete topology only; invocation/source/projection metadata are
/// still regenerated from this call's current inputs and admitted application owners.
#[allow(
    clippy::too_many_arguments,
    reason = "Prepared computational schedule and semantic owners have separate contracts."
)]
pub fn produce_prepared_scheduled(
    data: &SummaryData,
    invocation: &analysis::summary::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    graph: Option<&MaterializedGraph>,
    budget: &ResourceBudget,
    verified: Option<&VerifiedBindings>,
    actual: Option<&local_semantics::ProducedLocal>,
    schedule: Option<&super::summary_schedule::SccSchedule>,
) -> Result<SummaryRecords, ModelError> {
    if profile == stages::Profile::Behavioral && (actual.is_none() || schedule.is_none()) {
        return Err(invalid(
            "Summary prepared Local owner or SCC schedule absent",
        ));
    }
    produce_with_application(
        data, invocation, definition, profile, graph, budget, verified, actual, schedule,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Shared summary kernel keeps configuration, frontier, binding authority and actual Local owner explicit."
)]
fn produce_with_application(
    data: &SummaryData,
    invocation: &analysis::summary::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    graph: Option<&MaterializedGraph>,
    budget: &ResourceBudget,
    verified: Option<&VerifiedBindings>,
    actual: Option<&local_semantics::ProducedLocal>,
    schedule: Option<&super::summary_schedule::SccSchedule>,
) -> Result<SummaryRecords, ModelError> {
    let limits = summary_proof::limits(need(&data.parameters, definition.parameters)?, definition)?;
    if invocation.definition != definition.id() || invocation.subject.is_some() {
        return Err(invalid("Summary invocation changes its exact frame"));
    }
    let mut out = SummaryRecords::new(invocation.id(), budget);
    let mut run = SummaryRun {
        invocation: invocation.id(),
        requested: profile == stages::Profile::Behavioral,
        components: 0,
        work: 0,
        proofs: 0,
        residuals: 0,
    };
    if profile == stages::Profile::Catalog {
        out.outcome.status = analysis::AnalysisStatus::NotRequested;
        out.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        out.runs.insert(run)?;
        out.consequences(data, invocation, definition, profile, None, budget)?;
        return Ok(out);
    }
    let graph = graph.ok_or_else(|| invalid("requested Summary graph absent"))?;
    if graph.key()
        != (ProjectionKey {
            input: invocation.input,
            context: invocation.context,
            name: projection::ProjectionName::CallableInvocation,
        })
    {
        return Err(invalid("Summary graph changes its exact frame"));
    }
    run.work +=
        super::summary_exceptions::derive(data, invocation, &mut out, limits.work, budget)? as i64;
    let verified =
        verified.ok_or_else(|| invalid("requested Summary application authority absent"))?;
    // All solving-only state ends before symbolic and terminal postprocessing. The returned
    // condition index borrows immutable input rows, not the solving workspace.
    let vocabulary_conditions = {
        let computed_schedule;
        let schedule = if let Some(schedule) = schedule {
            schedule.require_graph(graph)?;
            schedule
        } else {
            computed_schedule = super::summary_schedule::invocation_sccs(graph, budget)?;
            &computed_schedule
        };
        let mut discovery = Discovery::new(data, &[], budget)?;
        let (calls, _calls) = call_infos(data, &discovery, verified, invocation, &mut out, budget)?;
        for (index, call) in calls.iter().enumerate() {
            discovery.callers.entry(call.owner).or_default().push(index);
            discovery
                .callees
                .entry(call.callee)
                .or_default()
                .push(index);
            discovery.attempts.insert(call.attempt, index);
        }
        let (seeds, _seeds) = data.seeds(invocation, &mut out, &discovery, budget)?;
        let _seed_index = budget.reserve("summary-seed-index", seeds.len().saturating_mul(256))?;
        let mut seeds_by_owner = BTreeMap::<_, Vec<_>>::new();
        for (index, seed) in seeds.iter().enumerate() {
            discovery.seeds.entry(seed.origin).or_insert(index);
            seeds_by_owner
                .entry(seed.descriptor().owner)
                .or_default()
                .push(index);
        }
        let (guards, _guards) = data.guards(budget, actual)?;
        let mut signatures = SignaturePreparation::new(data, &discovery, budget);
        let mut vocabulary = data.vocabulary.copy(budget)?;
        for root in out.vocabulary.roots.values() {
            vocabulary
                .roots
                .insert(&mut vocabulary.charge, root.id(), root.clone())?;
        }
        for place in out.vocabulary.places.values() {
            vocabulary
                .places
                .insert(&mut vocabulary.charge, place.id(), place.clone())?;
        }
        for path in out.vocabulary.paths.values() {
            vocabulary
                .paths
                .insert(&mut vocabulary.charge, path.id(), path.clone())?;
        }
        let mut progress = Progress::new(limits, budget);
        let _component_index = budget.reserve(
            "summary-component-membership",
            schedule
                .components()
                .iter()
                .map(Vec::len)
                .sum::<usize>()
                .saturating_mul(128),
        )?;
        let scheduled = schedule
            .components()
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        // Projection omissions remain explicit even when no component can own a seed.
        for seed in &seeds {
            if !scheduled.contains(&seed.descriptor().owner) {
                let mut digest = KeySink::new("summary-missing-component");
                seed.descriptor().owner.encode(&mut digest);
                origin_boundary(
                    &mut out,
                    invocation,
                    digest.finish(),
                    seed,
                    obligation::ObligationKind::IncompleteDomain,
                )?;
            }
        }
        for (ordinal, component) in schedule.components().iter().enumerate() {
            // Attempts and mutable argument/port preparation belong to this caller SCC. The
            // immutable callee signature cache remains available to subsequent caller SCCs.
            let mut call_preparation =
                CallPreparation::new(data, verified, &discovery, &mut signatures, budget);
            let mut digest = KeySink::new("summary-scc-members");
            for entity in component {
                entity.encode(&mut digest);
            }
            let component_id = digest.finish();
            let mut queue = summary_worklist::WorkQueue::new(limits.work, budget);
            let mut exhausted = ExhaustedGroups::new(budget);
            let mut processed = charged::ChargedSet::default();
            let mut processed_charge = StateCharge::new(budget, "summary-processed-pairs");
            let _component_seeds = budget.reserve(
                "summary-component-seeds",
                component
                    .iter()
                    .filter_map(|owner| seeds_by_owner.get(owner))
                    .map(Vec::len)
                    .sum::<usize>()
                    .saturating_mul(size_of::<usize>()),
            )?;
            let mut component_seeds = component
                .iter()
                .filter_map(|owner| seeds_by_owner.get(owner))
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            component_seeds.sort_unstable();
            for index in component_seeds {
                let seed = &seeds[index];
                if let summary_worklist::Admission::Refused(reason) =
                    progress.admit(seed, &vocabulary)?
                {
                    origin_boundary(&mut out, invocation, component_id, seed, reason)?;
                }
            }
            let _initial = budget.reserve(
                "summary-component-initial",
                component
                    .iter()
                    .filter_map(|owner| progress.by_owner.get(owner))
                    .map(BTreeSet::len)
                    .sum::<usize>()
                    .saturating_mul(size_of::<ProofId>()),
            )?;
            let mut initial = component
                .iter()
                .filter_map(|owner| progress.by_owner.get(owner))
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            initial.sort_unstable();
            for id in initial {
                enqueue(
                    EnqueueContext {
                        discovery: &discovery,
                        data,
                        vocabulary: &vocabulary,
                        calls: &calls,
                        component,
                        component_id,
                        invocation,
                        progress: &progress,
                        seeds: &seeds,
                    },
                    id,
                    &mut queue,
                    &processed,
                    &mut exhausted,
                    &mut out,
                )?;
            }
            let mut closed = !out
                .origin_boundaries
                .iter()
                .any(|r| r.component == component_id);
            loop {
                let item = match queue.pop() {
                    Ok(Some(i)) => i,
                    Ok(None) => break,
                    Err(reason) => {
                        closed = false;
                        for item in queue.pending() {
                            residual(
                                ResidualContext {
                                    discovery: &discovery,
                                    data,
                                    invocation,
                                    component: component_id,
                                    progress: &progress,
                                    seeds: &seeds,
                                },
                                *item,
                                reason,
                                &mut out,
                            )?;
                        }
                        break;
                    }
                };
                processed.insert(&mut processed_charge, item)?;
                match item {
                    WorkItem::Call(attempt, caller, callee) => {
                        if !progress.current(caller) || !progress.current(callee) {
                            continue;
                        }
                        let caller = progress.branches[&caller].clone();
                        let callee = progress.branches[&callee].clone();
                        let call = discovery
                            .attempts
                            .get(&attempt)
                            .map(|index| &calls[*index])
                            .ok_or_else(|| invalid("summary queued call absent"))?;
                        let results = call_preparation.get(call, budget)?.compose(
                            &caller, &callee, &vocabulary, &guards, budget,
                        )?;
                        out.questions.observe(call._event, &caller, &callee, results.as_slice())?;
                        for result in results {
                            match result {
                                CallComposition::Disjoint | CallComposition::Subsumed => {}
                                CallComposition::Obligation(reason) => {
                                    residual(
                                        ResidualContext {
                                            discovery: &discovery,
                                            data,
                                            invocation,
                                            component: component_id,
                                            progress: &progress,
                                            seeds: &seeds,
                                        },
                                        item,
                                        reason,
                                        &mut out,
                                    )?;
                                }
                                CallComposition::Transfer(composed) => {
                                    let cost =
                                        match ProofCost::through_call(caller.cost, callee.cost) {
                                            Ok(c) => c,
                                            Err(reason) => {
                                                residual(
                                                    ResidualContext {
                                                        discovery: &discovery,
                                                        data,
                                                        invocation,
                                                        component: component_id,
                                                        progress: &progress,
                                                        seeds: &seeds,
                                                    },
                                                    item,
                                                    reason,
                                                    &mut out,
                                                )?;
                                                continue;
                                            }
                                        };
                                    let mut evidence = vec![data.evidence(
                                        &out,
                                        &caller.premise,
                                        &discovery,
                                        budget,
                                    )?];
                                    if caller.premise.id() != callee.premise.id() {
                                        evidence.push(data.evidence(
                                            &out,
                                            &callee.premise,
                                            &discovery,
                                            budget,
                                        )?);
                                    }
                                    let emission = composed.emit(invocation, &evidence, budget)?;
                                    let branch = WorkBranch {
                                        branch: TransferBranch::new(
                                            emission.key.clone(),
                                            composed.qualification.clone(),
                                            composed.condition.clone(),
                                            budget,
                                        )?,
                                        premise: SummaryPremise::Witness {
                                            witness: emission.witness.id(),
                                        },
                                        origin: caller.origin,
                                        facts: emission.witness.source_facts(),
                                        cost,
                                    };
                                    let admission = progress.frontier.insert(
                                        branch.semantic(),
                                        summary_worklist::Representative {
                                            witness: branch.id(),
                                            cost,
                                        },
                                    )?;
                                    if let summary_worklist::Admission::Refused(reason) = admission
                                    {
                                        residual(
                                            ResidualContext {
                                                discovery: &discovery,
                                                data,
                                                invocation,
                                                component: component_id,
                                                progress: &progress,
                                                seeds: &seeds,
                                            },
                                            item,
                                            reason,
                                            &mut out,
                                        )?;
                                        continue;
                                    }
                                    if cost.check(limits).is_err() {
                                        continue;
                                    }
                                    // Store every admissible proof, including equal-cost evidence; reopen only semantic progress.
                                    progress.by_owner.insert_set(
                                        &mut progress.charge,
                                        branch.descriptor().owner,
                                        branch.id(),
                                    )?;
                                    progress.branches.insert(
                                        &mut progress.charge,
                                        branch.id(),
                                        branch.clone(),
                                    )?;
                                    vocabulary.composition(&composed.records)?;
                                    out.vocabulary.composition(&composed.records)?;
                                    for premise in emission.premises {
                                        out.premises.insert(premise)?;
                                    }
                                    out.witnesses.insert(emission.witness.clone())?;
                                    out.keys.insert(emission.key)?;
                                    for substitution in &composed.records.substitutions {
                                        out.substitutions.insert(substitution.clone())?;
                                    }
                                    for influence in &composed.records.influences {
                                        let q = vocabulary
                                            .qualifications
                                            .get(&influence.qualification)
                                            .ok_or_else(|| {
                                                invalid("rebased influence qualification absent")
                                            })?;
                                        let condition = vocabulary.diagram(q.condition, budget)?;
                                        super::summary_control::publish(
                                            super::summary_control::SummaryControlInputs {
                                                invocation,
                                                definition,
                                                witness: &emission.witness,
                                            },
                                            influence,
                                            q,
                                            &condition,
                                            &mut out,
                                            budget,
                                        )?;
                                    }
                                    retain_proof(&mut out, invocation, &branch)?;
                                    if admission == summary_worklist::Admission::Advanced {
                                        progress.index_current(&branch, &vocabulary)?;
                                        enqueue(
                                            EnqueueContext {
                                                discovery: &discovery,
                                                data,
                                                vocabulary: &vocabulary,
                                                calls: &calls,
                                                component,
                                                component_id,
                                                invocation,
                                                progress: &progress,
                                                seeds: &seeds,
                                            },
                                            branch.id(),
                                            &mut queue,
                                            &processed,
                                            &mut exhausted,
                                            &mut out,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    WorkItem::Path(proof, _, _, _) | WorkItem::Alias(proof, _, _, _, _) => {
                        if !progress.current(proof) {
                            continue;
                        }
                        let source = progress.branches[&proof].clone();
                        let result = match item {
                            WorkItem::Path(_, vs, ps, step) => summary_path::derive(
                                &data.entry,
                                &data.path,
                                &data.native,
                                &vocabulary.guards(),
                                invocation,
                                &source,
                                source.facts,
                                vs,
                                ps,
                                step,
                                &discovery.entry_conditions,
                                budget,
                            )?,
                            WorkItem::Alias(_, vs, rs, ds, us) => super::summary_alias::derive(
                                &data.entry,
                                &data.native,
                                &vocabulary.guards(),
                                invocation,
                                &source,
                                source.facts,
                                vs,
                                rs,
                                ds,
                                us,
                                &discovery.entry_conditions,
                                budget,
                            )?,
                            _ => unreachable!(),
                        };
                        let result = match result {
                            Ok(mut emission) => match super::summary_exceptions::filter_raise(
                                data,
                                &out.exception_outcomes,
                                source.descriptor().owner,
                                &emission,
                                invocation.input,
                                budget,
                            )? {
                                Err(reason) => Err(reason),
                                Ok(None) => Ok(emission),
                                Ok(Some(outcome)) => {
                                    out.path_routes.insert(emission.route.clone())?;
                                    emission.route =
                                        summary_path::SummaryPathRoute::EscapingRaise {
                                            route: emission.route.id(),
                                            outcome,
                                        };
                                    emission.witness.route = emission.route.id();
                                    Ok(emission)
                                }
                            },
                            Err(reason) => Err(reason),
                        };
                        match result {
                            Err(reason) => residual(
                                ResidualContext {
                                    discovery: &discovery,
                                    data,
                                    invocation,
                                    component: component_id,
                                    progress: &progress,
                                    seeds: &seeds,
                                },
                                item,
                                reason,
                                &mut out,
                            )?,
                            Ok(emission) => {
                                let cost = match ProofCost::through_path(source.cost) {
                                    Ok(c) => c,
                                    Err(reason) => {
                                        residual(
                                            ResidualContext {
                                                discovery: &discovery,
                                                data,
                                                invocation,
                                                component: component_id,
                                                progress: &progress,
                                                seeds: &seeds,
                                            },
                                            item,
                                            reason,
                                            &mut out,
                                        )?;
                                        continue;
                                    }
                                };
                                let branch = WorkBranch {
                                    branch: emission.branch.clone(),
                                    premise: SummaryPremise::Path {
                                        witness: emission.witness.id(),
                                    },
                                    origin: source.origin,
                                    facts: emission.witness.source_facts(),
                                    cost,
                                };
                                let admission = progress.frontier.insert(
                                    branch.semantic(),
                                    summary_worklist::Representative {
                                        witness: branch.id(),
                                        cost,
                                    },
                                )?;
                                if let summary_worklist::Admission::Refused(reason) = admission {
                                    residual(
                                        ResidualContext {
                                            discovery: &discovery,
                                            data,
                                            invocation,
                                            component: component_id,
                                            progress: &progress,
                                            seeds: &seeds,
                                        },
                                        item,
                                        reason,
                                        &mut out,
                                    )?;
                                    continue;
                                }
                                if cost.check(limits).is_err() {
                                    continue;
                                }
                                progress.by_owner.insert_set(
                                    &mut progress.charge,
                                    branch.descriptor().owner,
                                    branch.id(),
                                )?;
                                progress.branches.insert(
                                    &mut progress.charge,
                                    branch.id(),
                                    branch.clone(),
                                )?;
                                vocabulary.path(&emission)?;
                                out.vocabulary.path(&emission)?;
                                out.premises.insert(emission.source)?;
                                out.path_routes.insert(emission.route)?;
                                out.path_witnesses.insert(emission.witness)?;
                                out.keys.insert(branch.branch.key().clone())?;
                                retain_proof(&mut out, invocation, &branch)?;
                                if admission == summary_worklist::Admission::Advanced {
                                    progress.index_current(&branch, &vocabulary)?;
                                    enqueue(
                                        EnqueueContext {
                                            discovery: &discovery,
                                            data,
                                            vocabulary: &vocabulary,
                                            calls: &calls,
                                            component,
                                            component_id,
                                            invocation,
                                            progress: &progress,
                                            seeds: &seeds,
                                        },
                                        branch.id(),
                                        &mut queue,
                                        &processed,
                                        &mut exhausted,
                                        &mut out,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            publish(
                &mut out,
                invocation,
                definition,
                &progress,
                component,
                component_id,
                budget,
            )?;
            closed &= !out.residuals.iter().any(|r| r.component == component_id)
                && !out
                    .origin_boundaries
                    .iter()
                    .any(|r| r.component == component_id);
            let row = SummaryComponent {
                invocation: invocation.id(),
                members: component_id,
                ordinal: ordinal as i64,
                work: queue.work().into(),
                closed,
            };
            for entity in component {
                out.component_members.insert(ComponentMember {
                    component: row.id(),
                    entity: *entity,
                })?;
            }
            run.work += i64::from(queue.work());
            out.components.insert(row)?;
            // Publication and its component receipt are complete; no later SCC consumes this
            // caller's prepared arguments, mutable ports or attempt-map reservation.
            drop(call_preparation);
        }
        drop(signatures);
        discovery.into_vocabulary_conditions()
    };
    run.components = out.components.len() as i64;
    run.proofs =
        (out.witnesses.len() + out.path_witnesses.len() + out.capture_witnesses.len()) as i64;
    run.residuals = out.residuals.len() as i64;
    out.runs.insert(run)?;
    if inherited_partial(data, invocation)?
        || !out.origin_boundaries.is_empty()
        || !out.residuals.is_empty()
        || out.call_members.iter().any(|m| m.reason.is_some())
    {
        out.outcome.status = analysis::AnalysisStatus::Partial;
        out.outcome.reason = Some(obligation::ObligationKind::IncompleteCoverage);
    }
    super::summary_symbolic::produce(data, invocation, &mut out, budget)?;
    super::summary_terminal::produce(
        data,
        invocation,
        definition,
        &mut out,
        &vocabulary_conditions,
        budget,
    )?;
    out.consequences(
        data,
        invocation,
        definition,
        profile,
        Some(verified),
        budget,
    )?;
    Ok(out)
}

/// A bounded positive never upgrades an unfinished lower channel to complete coverage.
fn inherited_partial(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
) -> Result<bool, ModelError> {
    let mut incomplete = false;
    macro_rules! lower {
        ($invocations:ident,$outcomes:ident) => {
            for parent in data.$invocations.iter().filter(|p| {
                (p.input, p.context) == (invocation.input, invocation.context)
                    && p.subject.is_none()
            }) {
                let mut rows = data
                    .$outcomes
                    .iter()
                    .filter(|o| o.invocation == parent.id());
                let row = rows
                    .next()
                    .ok_or_else(|| invalid("Summary lower outcome absent"))?;
                if rows.next().is_some() {
                    return Err(invalid("Summary lower outcome ambiguous"));
                }
                row.validate()?;
                incomplete |= row.status != analysis::AnalysisStatus::Completed;
            }
        };
    }
    lower!(local_invocations, local_outcomes);
    lower!(model_invocations, model_outcomes);
    lower!(enriched_invocations, enriched_outcomes);
    lower!(source_invocations, source_outcomes);
    Ok(incomplete)
}

#[cfg(test)]
mod assumption_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn actual_summary_publication_preserves_separate_conditional_and_unconditional_alternatives() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let (_, definition) =
            super::super::configuration::summaries(id(1), Default::default()).unwrap();
        let (invocation, _) =
            owner::AnalysisInvocation::new(id(2), id(3), definition.id(), None, []);
        let mut out = SummaryRecords::new(invocation.id(), &budget);
        let mut progress = Progress::new(Default::default(), &budget);
        let condition = Diagram::always();
        let a = assumptions::Assumption::TypeConformance {
            observation: id(4),
            support: id(5),
        };
        let basis = assumptions::AssumptionSet::new([a.id()]).unwrap();
        let q = AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: invocation.context,
            scope: id(6),
            condition: condition.id(),
            modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        let key = TransferKey::from_descriptor(TransferDescriptor {
            owner: id(7),
            input: id(8),
            output: id(9),
            context: q.context,
            scope: q.scope,
            modality: q.modality,
            approximation: q.approximation,
            kind: TransferKind::Derived,
            call_site: None,
            provenance: ProvenanceClass::Composed,
        });
        for q in [
            q.clone(),
            AssertionQualification {
                assumptions: basis.set.id(),
                ..q
            },
        ] {
            let witness = SummaryWitness {
                invocation: invocation.id(),
                transfer: key.id(),
                qualification: q.id(),
                caller: id(10),
                callee: id(11),
                target: id(12),
                signature: id(13),
                callee_declaration: id(14),
                attempt: id(15),
                selected_signature_enumeration: None,
                selected_signature_enumeration_support: None,
                bindings: ContentHash::of(b"bindings"),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                heuristic: false,
            };
            let premise = SummaryPremise::Witness {
                witness: witness.id(),
            };
            let branch = WorkBranch {
                branch: TransferBranch::new(key.clone(), q, condition.clone(), &budget).unwrap(),
                premise,
                origin: id(16),
                facts: witness.source_facts(),
                cost: ProofCost::SOURCE,
            };
            out.witnesses.insert(witness).unwrap();
            progress.admit(&branch, &out.vocabulary).unwrap();
        }
        // Both enter and leave the actual finite Summary publication, including derivations and contributions.
        publish(
            &mut out,
            &invocation,
            &definition,
            &progress,
            &[key.owner],
            ContentHash::of(b"component"),
            &budget,
        )
        .unwrap();
        assert_eq!(out.alternatives.len(), 2);
        assert_eq!(out.derivations.len(), 2);
        assert_eq!(out.contributions.len(), 2);
        let bases = out
            .vocabulary
            .qualifications
            .values()
            .map(|q| q.assumptions)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            bases,
            BTreeSet::from([assumptions::AssumptionSet::empty_id(), basis.set.id()])
        );
        assert_eq!(out.keys.len(), 1);
        assert_eq!(progress.branches.len(), 2);
    }
}

#[cfg(test)]
mod optional_graph_controls {
    use super::*;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn catalog_summary_needs_no_graph_and_behavioral_refuses_missing_graph() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let (parameters, definition) =
            super::super::configuration::summaries(nominal(3), Default::default()).unwrap();
        let mut data = SummaryData::new(&budget);
        data.parameters.insert(parameters).unwrap();
        let (invocation, _) =
            owner::AnalysisInvocation::new(nominal(1), nominal(2), definition.id(), None, []);
        let out = produce(
            &data,
            &invocation,
            &definition,
            stages::Profile::Catalog,
            None,
            &budget,
        )
        .unwrap();
        assert_eq!(out.outcome.status, analysis::AnalysisStatus::NotRequested);
        assert!(!out.runs.iter().next().unwrap().requested);
        assert!(
            produce(
                &data,
                &invocation,
                &definition,
                stages::Profile::Behavioral,
                None,
                &budget
            )
            .is_err()
        );
        let consumed = SummaryData::consumed_inputs(stages::Profile::Catalog);
        assert!(
            !consumed
                .iter()
                .any(|input| input.name() == projection::ProjectionSnapshotChunk::NAME)
        );
    }
}

#[cfg(test)]
mod actual_input_property_controls {
    use super::*;
    fn nominal<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn indexed_alias_discovery_preserves_every_support_combination_in_source_order() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let mut data = SummaryData::new(&budget);
            let path = AccessPath::empty();
            let input_root = PlaceRoot::Entry {
                declaration: nominal(1),
            };
            let output_root = PlaceRoot::Occurrence {
                occurrence: nominal(2),
            };
            let input = Place {
                root: input_root.id(),
                path: path.id(),
            };
            let output = Place {
                root: output_root.id(),
                path: path.id(),
            };
            for root in [input_root, output_root] {
                data.vocabulary
                    .roots
                    .insert(&mut data.vocabulary.charge, root.id(), root)
                    .unwrap();
            }
            for place in [&input, &output] {
                data.vocabulary
                    .places
                    .insert(&mut data.vocabulary.charge, place.id(), place.clone())
                    .unwrap();
            }
            let condition = Diagram::always();
            let q = AssertionQualification {
                assumptions: assumptions::AssumptionSet::empty_id(),
                context: nominal(3),
                scope: nominal(4),
                condition: condition.id(),
                modality: attribution::Modality::Definite,
                approximation: Approximation::Exact,
            };
            let key = TransferKey::from_descriptor(TransferDescriptor {
                owner: nominal(5),
                input: input.id(),
                output: output.id(),
                context: q.context,
                scope: q.scope,
                modality: q.modality,
                approximation: q.approximation,
                kind: TransferKind::Derived,
                call_site: Some(nominal(55)),
                provenance: ProvenanceClass::Composed,
            });
            let branch = WorkBranch {
                branch: TransferBranch::new(key, q.clone(), condition, &budget).unwrap(),
                premise: SummaryPremise::Local {
                    alternative: nominal(6),
                },
                origin: nominal(7),
                facts: SourceFacts {
                    qualification: q.id(),
                    status: analysis::policy::EvidenceStatus::StructurallyObserved,
                    heuristic: false,
                },
                cost: ProofCost {
                    depth: 2,
                    steps: 5,
                    rank: 2,
                },
            };
            let definition = FlowDefinitionObservation {
                qualification: q.id(),
                definition: nominal(8),
                scope: nominal(9),
                kind: lexical::BindingEventKind::Assignment,
                value: Some(nominal(2)),
            };
            let reaching = FlowReachingObservation {
                qualification: q.id(),
                use_: nominal(10),
                target: ReachingDefinition::Bound {
                    definition: definition.definition,
                }
                .id(),
                loop_carried: false,
            };
            let value = FlowValueObservation {
                qualification: q.id(),
                use_: nominal(10),
                sink: nominal(11),
                kind: FlowSinkKind::Return,
                transfer: TransferKind::Derived,
                through_call: false,
            };
            macro_rules! support {
                ($ty:ident, $row:expr, $n:expr) => {
                    $ty {
                        assertion: $row.id(),
                        run: nominal($n),
                        surface: nominal(12),
                        evidence: nominal(13),
                        origin: attribution::Origin::AnalyzerAssertion,
                        mode: attribution::ExtractionMode::NativeTraversal,
                        fidelity: attribution::Fidelity::NativeStructural,
                    }
                };
            }
            data.entry
                .definition_observations
                .insert(definition.clone())
                .unwrap();
            data.entry.reaching.insert(reaching.clone()).unwrap();
            data.entry.values.insert(value.clone()).unwrap();
            for n in [20, 21] {
                data.entry
                    .definition_supports
                    .insert(support!(FlowDefinitionSupport, definition, n))
                    .unwrap();
                data.entry
                    .reaching_supports
                    .insert(support!(FlowReachingSupport, reaching, n))
                    .unwrap();
                data.entry
                    .value_supports
                    .insert(support!(FlowValueSupport, value, n))
                    .unwrap();
            }
            for annotation in [false, true] {
                let observed = FlowUseObservation {
                    qualification: q.id(),
                    use_: nominal(10),
                    scope: nominal(9),
                    annotation,
                };
                for n in [20, 21] {
                    data.entry
                        .use_supports
                        .insert(support!(FlowUseSupport, observed, n))
                        .unwrap();
                }
                data.entry.use_observations.insert(observed).unwrap();
            }
            // A complete independent relation join supplies the expected proof population and order.
            let mut expected = Vec::new();
            for d in data
                .entry
                .definition_observations
                .iter()
                .filter(|d| d.value == Some(nominal(2)))
            {
                let target = ReachingDefinition::Bound {
                    definition: d.definition,
                }
                .id();
                for r in data.entry.reaching.iter().filter(|r| r.target == target) {
                    for v in data
                        .entry
                        .values
                        .iter()
                        .filter(|v| v.use_ == r.use_ && !v.through_call)
                    {
                        for u in data
                            .entry
                            .use_observations
                            .iter()
                            .filter(|u| u.use_ == v.use_)
                        {
                            for vs in data
                                .entry
                                .value_supports
                                .iter()
                                .filter(|s| s.assertion == v.id())
                            {
                                for rs in data
                                    .entry
                                    .reaching_supports
                                    .iter()
                                    .filter(|s| s.assertion == r.id())
                                {
                                    for ds in data
                                        .entry
                                        .definition_supports
                                        .iter()
                                        .filter(|s| s.assertion == d.id())
                                    {
                                        for us in data
                                            .entry
                                            .use_supports
                                            .iter()
                                            .filter(|s| s.assertion == u.id())
                                        {
                                            expected.push(WorkItem::Alias(
                                                branch.id(),
                                                vs.id(),
                                                rs.id(),
                                                ds.id(),
                                                us.id(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let retained = budget.reserved();
            let discovery = Discovery::new(&data, &[], &budget).unwrap();
            let mut actual = Vec::new();
            path_candidates(&data, &discovery, &data.vocabulary, &branch, |item| {
                actual.push(item);
                Ok(false)
            })
            .unwrap();
            assert_eq!(actual.len(), 32);
            assert_eq!(actual, expected);
            drop(discovery);
            assert_eq!(budget.reserved(), retained);
            // Two legitimate support groups have different events: Alias uses the original
            // call; Path uses this output call site. Repeated steps still share the latter.
            let formal = PlaceRoot::Formal {
                declaration: nominal(1),
            };
            let formal_place = Place {
                root: formal.id(),
                path: path.id(),
            };
            data.entry.roots.insert(formal).unwrap();
            data.entry.places.insert(formal_place.clone()).unwrap();
            let use_ = FlowUse {
                occurrence: nominal(40),
                place: formal_place.id(),
            };
            data.entry.uses.insert(use_.clone()).unwrap();
            data.entry
                .owners
                .insert(OccurrenceOwnership {
                    occurrence: use_.occurrence,
                    owner: nominal(1),
                    entity: branch.descriptor().owner,
                })
                .unwrap();
            let path_value = FlowValueObservation {
                qualification: q.id(),
                use_: use_.id(),
                sink: nominal(41),
                kind: FlowSinkKind::Return,
                transfer: TransferKind::Derived,
                through_call: true,
            };
            data.entry.values.insert(path_value.clone()).unwrap();
            let (call_path, steps) = FlowCallPath::new(&[
                (nominal(2), nominal(40), FlowCallOperandRole::Argument),
                (nominal(2), nominal(41), FlowCallOperandRole::Callee),
            ])
            .unwrap();
            let path_observed = FlowValuePathObservation {
                qualification: q.id(),
                value: path_value.id(),
                path: call_path.id(),
            };
            data.path.call_paths.insert(call_path).unwrap();
            for step in steps {
                data.path.call_steps.insert(step).unwrap();
            }
            data.path.value_paths.insert(path_observed.clone()).unwrap();
            for n in [20, 21] {
                data.entry
                    .value_supports
                    .insert(support!(FlowValueSupport, path_value, n))
                    .unwrap();
                data.path
                    .value_path_supports
                    .insert(support!(FlowValuePathSupport, path_observed, n))
                    .unwrap();
            }
            let events = [55, 2].map(|n| {
                let ownership = OccurrenceOwnership {
                    occurrence: nominal(n),
                    owner: nominal(1),
                    entity: branch.descriptor().owner,
                };
                data.entry.owners.insert(ownership.clone()).unwrap();
                normalized::events::NormalizedCallEvent {
                    site: ownership.occurrence,
                    origin: nominal(56),
                    context: q.context,
                    owner: ownership.id(),
                }
            });
            for event in &events {
                data.bindings.event_events.insert(event.clone()).unwrap();
            }
            let (_, definition) =
                super::super::configuration::summaries(nominal(1), Default::default()).unwrap();
            let (invocation, _) =
                owner::AnalysisInvocation::new(nominal(2), q.context, definition.id(), None, []);
            let seeds = [branch.clone()];
            let mut discovery = Discovery::new(&data, &[], &budget).unwrap();
            discovery.seeds.insert(branch.origin, 0);
            let mut all = Vec::new();
            path_candidates(&data, &discovery, &data.vocabulary, &branch, |item| {
                all.push(item);
                Ok(false)
            })
            .unwrap();
            assert_eq!(&all[..32], expected);
            let mut expected_paths = Vec::new();
            for step in data.path.call_steps.iter() {
                for vs in data
                    .entry
                    .value_supports
                    .iter()
                    .filter(|support| support.assertion == path_value.id())
                {
                    for ps in data
                        .path
                        .value_path_supports
                        .iter()
                        .filter(|support| support.assertion == path_observed.id())
                    {
                        expected_paths.push(WorkItem::Path(
                            branch.id(),
                            vs.id(),
                            ps.id(),
                            step.id(),
                        ));
                    }
                }
            }
            assert_eq!(&all[32..], expected_paths);
            assert_eq!(all.len(), 40);
            let component = [branch.descriptor().owner];
            let component_id = ContentHash::of(b"alias-path-groups");
            let mut progress = Progress::new(Default::default(), &budget);
            progress.admit(&branch, &data.vocabulary).unwrap();
            let mut queue = summary_worklist::WorkQueue::new(3, &budget);
            assert_eq!(queue.insert(all[0]).unwrap(), Ok(true));
            assert_eq!(queue.pop().unwrap(), Some(all[0]));
            assert_eq!(queue.insert(all[1]).unwrap(), Ok(true));
            let mut processed = charged::ChargedSet::default();
            let mut charge = StateCharge::new(&budget, "path-control-processed");
            processed.insert(&mut charge, all[0]).unwrap();
            let mut exhausted = ExhaustedGroups::new(&budget);
            let mut out = SummaryRecords::new(invocation.id(), &budget);
            retain_proof(&mut out, &invocation, &branch).unwrap();
            assert_eq!(
                enqueue(
                    EnqueueContext {
                        discovery: &discovery,
                        data: &data,
                        vocabulary: &data.vocabulary,
                        calls: &[],
                        component: &component,
                        component_id,
                        invocation: &invocation,
                        progress: &progress,
                        seeds: &seeds
                    },
                    branch.id(),
                    &mut queue,
                    &processed,
                    &mut exhausted,
                    &mut out
                )
                .unwrap(),
                2
            );
            assert_eq!(exhausted.examined, 4);
            assert_eq!(queue.work(), 3);
            assert_eq!(queue.pending().copied().collect::<Vec<_>>(), [all[1]]);
            let over = AssertionQualification {
                approximation: Approximation::Over,
                ..q.clone()
            };
            let mut expected_out = SummaryRecords::new(invocation.id(), &budget);
            expected_out
                .vocabulary
                .qualified(&over, &Diagram::always())
                .unwrap();
            for event in &events {
                expected_out
                    .residuals
                    .insert(SummaryResidual {
                        invocation: invocation.id(),
                        component: component_id,
                        origin: branch.origin,
                        event: event.id(),
                        channel: analysis::AnalysisChannel::Value,
                        phase: CallPhase::Call,
                        input: input.id(),
                        output: output.id(),
                        reason: obligation::ObligationKind::SummaryPairWorkLimit,
                        qualification: over.id(),
                    })
                    .unwrap();
                let subject = owner::ObligationSubject::SourceCall {
                    occurrence: event.site,
                };
                expected_out
                    .obligations
                    .insert(owner::AnalysisObligation {
                        invocation: invocation.id(),
                        subject: subject.id(),
                        channel: analysis::AnalysisChannel::Value,
                        phase: CallPhase::Call,
                        qualification: over.id(),
                        reason: obligation::ObligationKind::SummaryPairWorkLimit,
                        responsible: analysis::AnalysisMethod::Summaries,
                    })
                    .unwrap();
                expected_out.subjects.insert(subject).unwrap();
            }
            assert!(out.residuals.same(&expected_out.residuals));
            assert!(out.obligations.same(&expected_out.obligations));
            assert!(out.subjects.same(&expected_out.subjects));
            assert!(
                out.vocabulary
                    .qualifications
                    .values()
                    .eq(expected_out.vocabulary.qualifications.values())
            );
            assert_eq!(out.premises.len(), 1);
            assert_eq!(out.proof_origins.len(), 1);
            // Every tuple can legitimately already be processed or pending. That produces no
            // refusal; a new improving witness must still expose its unattempted equivalents.
            let mut excepted = summary_worklist::WorkQueue::new(79, &budget);
            let mut seen = charged::ChargedSet::default();
            for item in &all {
                assert_eq!(excepted.insert(*item).unwrap(), Ok(true));
            }
            for _ in 0..39 {
                let item = excepted.pop().unwrap().unwrap();
                seen.insert(&mut charge, item).unwrap();
            }
            let pending_last = all.iter().copied().max().unwrap();
            assert_eq!(
                excepted.pending().copied().collect::<Vec<_>>(),
                [pending_last]
            );
            let mut groups = ExhaustedGroups::new(&budget);
            let mut later = SummaryRecords::new(invocation.id(), &budget);
            retain_proof(&mut later, &invocation, &branch).unwrap();
            assert_eq!(
                enqueue(
                    EnqueueContext {
                        discovery: &discovery,
                        data: &data,
                        vocabulary: &data.vocabulary,
                        calls: &[],
                        component: &component,
                        component_id,
                        invocation: &invocation,
                        progress: &progress,
                        seeds: &seeds
                    },
                    branch.id(),
                    &mut excepted,
                    &seen,
                    &mut groups,
                    &mut later
                )
                .unwrap(),
                0
            );
            assert!(later.residuals.is_empty());
            assert_eq!(groups.examined, 40);
            let replacement = WorkBranch {
                premise: SummaryPremise::Local {
                    alternative: nominal(57),
                },
                cost: ProofCost::SOURCE,
                ..branch.clone()
            };
            assert_eq!(
                progress.admit(&replacement, &data.vocabulary).unwrap(),
                summary_worklist::Admission::Advanced
            );
            assert!(!progress.current(branch.id()));
            retain_proof(&mut later, &invocation, &replacement).unwrap();
            assert_eq!(
                enqueue(
                    EnqueueContext {
                        discovery: &discovery,
                        data: &data,
                        vocabulary: &data.vocabulary,
                        calls: &[],
                        component: &component,
                        component_id,
                        invocation: &invocation,
                        progress: &progress,
                        seeds: &seeds
                    },
                    replacement.id(),
                    &mut excepted,
                    &seen,
                    &mut groups,
                    &mut later
                )
                .unwrap(),
                2
            );
            assert_eq!(groups.examined, 42);
            assert!(later.residuals.same(&expected_out.residuals));
            assert!(later.obligations.same(&expected_out.obligations));
            assert!(later.subjects.same(&expected_out.subjects));
            assert_eq!(later.proof_origins.len(), 2);
            assert_eq!(later.premises.len(), 2);
            assert_eq!(excepted.work(), 79);
            assert_eq!(
                excepted.pending().copied().collect::<Vec<_>>(),
                [pending_last]
            );
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn actual_summary_does_not_decode_unused_provider_defaults_or_decorator_normalizer_inputs() {
        let budget = ResourceBudget::fixed(256 << 10).unwrap();
        let mut actual = SummaryData::new(&budget);
        let provider = attribution::Provider {
            tool: "provider".repeat(1 << 20),
            revision: "unused".into(),
            build_digest: ContentHash::of(b"provider"),
        };
        let provider_batch = <attribution::Provider as Record>::encode(&[provider]).unwrap();
        let shape = calls::ParameterShape {
            name: Some("default".repeat(1 << 20).into()),
            kind: calls::ParameterKind::PositionalOrKeyword,
            required: false,
        };
        let shape_batch = <calls::ParameterShape as Record>::encode(&[shape]).unwrap();
        let decorator = syntax::DeclarationDecorator {
            qualification: nominal(1),
            declaration: nominal(2),
            decorator: nominal(3),
            ordinal: 0,
        };
        let decorator_batch =
            <syntax::DeclarationDecorator as Record>::encode(&[decorator]).unwrap();
        actual
            .visit_actual(
                &ValidationInput::of::<attribution::Provider>(&["id"]),
                &provider_batch,
            )
            .unwrap();
        actual
            .visit_actual(
                &ValidationInput::of::<calls::ParameterShape>(&["id"]),
                &shape_batch,
            )
            .unwrap();
        actual
            .visit_actual(
                &ValidationInput::of::<syntax::DeclarationDecorator>(&["id"]),
                &decorator_batch,
            )
            .unwrap();
        assert!(
            actual.bindings.native_providers.is_empty()
                && actual.bindings.shapes.is_empty()
                && actual.bindings.decorators.is_empty()
        );
        assert_eq!(budget.reserved(), 0);
        let mut ordinary = SummaryData::new(&budget);
        assert!(
            ordinary
                .visit(attribution::Provider::NAME, &provider_batch)
                .is_err()
        );
        assert!(
            ordinary
                .visit(calls::ParameterShape::NAME, &shape_batch)
                .is_err()
        );
        drop(ordinary);
        drop(actual);
        assert_eq!(budget.reserved(), 0);
    }
}

#[cfg(test)]
mod correspondence_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn signature(data: &mut SummaryData, n: u8) -> (Signature, declarations::SymbolDeclaration) {
        let symbol = ProviderSymbol {
            provider: id(1),
            context: id(2),
            module: id(3),
            native_key: format!("callee-{n}"),
            name: format!("callee-{n}"),
            kind: SymbolKind::Function,
        };
        data.bindings.symbols.insert(symbol.clone()).unwrap();
        let signature = Signature {
            role: SignatureRole::Source,
            native: None,
            qualification: id(4),
            scope: id(5),
            symbol: symbol.id(),
            variant: 0,
            form: SignatureForm::List,
            parameters: ContentHash::of(&[n]),
        };
        data.bindings.signatures.insert(signature.clone()).unwrap();
        let declaration = declarations::SymbolDeclaration {
            qualification: id(4),
            symbol: symbol.id(),
            declaration: id(n),
        };
        data.entry
            .symbol_declarations
            .insert(declaration.clone())
            .unwrap();
        (signature, declaration)
    }
    #[test]
    fn signature_preparation_is_lazy_shared_ordered_and_borrow_scoped() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = SummaryData::new(&budget);
        let (selected, declaration) = signature(&mut data, 10);
        let unused = Signature {
            symbol: id(99),
            parameters: ContentHash::of(b"unused"),
            ..selected.clone()
        };
        data.bindings.signatures.insert(unused.clone()).unwrap();
        // Source inventory order is nominal, not parameter ordinal order.
        let parameters = [1, 0].map(|ordinal| SignatureParameter {
            signature: selected.id(),
            ordinal,
            shape: id(20 + ordinal as u8),
        });
        for row in &parameters {
            data.bindings.parameters.insert(row.clone()).unwrap();
        }
        for (n, parameter) in parameters.iter().enumerate() {
            data.entry
                .declarations
                .insert(declarations::ParameterDeclaration {
                    qualification: id(4),
                    parameter: parameter.id(),
                    declaration: id(30 + n as u8),
                })
                .unwrap();
        }
        let retained = budget.reserved();
        {
            let discovery = Discovery::new(&data, &[], &budget).unwrap();
            let expected_links = data.entry.declarations.iter().cloned().collect::<Vec<_>>();
            let mut preparation = SignaturePreparation::new(&data, &discovery, &budget);
            let first = preparation.get(selected.id(), &budget).unwrap();
            let once = budget.reserved();
            let second = preparation.get(selected.id(), &budget).unwrap();
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(budget.reserved(), once);
            assert_eq!(
                first
                    .parameters
                    .iter()
                    .map(|p| p.ordinal)
                    .collect::<Vec<_>>(),
                [0, 1]
            );
            assert_eq!(first.links, expected_links);
            assert_eq!(first.declaration, &declaration);
            // Missing rich data is exposed only if that signature is consumed.
            assert!(
                preparation
                    .get(unused.id(), &budget)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("callee symbol absent")
            );
        }
        assert_eq!(budget.reserved(), retained);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn component_call_slots_release_while_shared_signatures_remain() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = SummaryData::new(&budget);
        let verified = normalized::binding_normalization::prepare(
            &data.bindings,
            &data.binding_output,
            &budget,
        )
        .unwrap();
        let (selected, _) = signature(&mut data, 10);
        let retained = budget.reserved();
        {
            let discovery = Discovery::new(&data, &[], &budget).unwrap();
            let mut signatures = SignaturePreparation::new(&data, &discovery, &budget);
            let first = signatures.get(selected.id(), &budget).unwrap();
            let shared = budget.reserved();
            {
                let mut component =
                    CallPreparation::new(&data, &verified, &discovery, &mut signatures, &budget);
                // A selected missing attempt charges its map slot before refusing. Even
                // this interrupted component must release its private reservation.
                let missing = CallInfo {
                    attempt: id(99),
                    _event: id(98),
                    _header: None,
                    owner: id(97),
                    callee: id(96),
                    target: id(95),
                };
                assert!(component.get(&missing, &budget).is_err());
                assert!(budget.reserved() > shared);
            }
            assert_eq!(budget.reserved(), shared);
            // A later caller SCC retains exactly the same immutable callee preparation.
            {
                let component =
                    CallPreparation::new(&data, &verified, &discovery, &mut signatures, &budget);
                let second = component.signatures.get(selected.id(), &budget).unwrap();
                assert!(Arc::ptr_eq(&first, &second));
                assert_eq!(budget.reserved(), shared);
            }
            drop(first);
            drop(signatures);
            let before_terminal = budget.reserved();
            let conditions = discovery.into_vocabulary_conditions();
            assert!(budget.reserved() < before_terminal);
            drop(conditions);
        }
        assert_eq!(budget.reserved(), retained);
        drop(verified);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn correspondence_keeps_competing_headers_declarations_and_contextual_events() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = SummaryData::new(&budget);
        let (selected, declaration) = signature(&mut data, 10);
        data.entry
            .symbol_declarations
            .insert(declarations::SymbolDeclaration {
                qualification: id(6),
                ..declaration
            })
            .unwrap();
        for n in [2, 1] {
            data.headers
                .insert(SourceCallHeader {
                    invocation: id(n),
                    event: id(8),
                    attempt: id(9),
                    owner: id(10),
                    callee: id(11),
                    declaration: id(12),
                    qualification: id(4),
                    status: analysis::policy::EvidenceStatus::StructurallyObserved,
                    premises: ContentHash::of(&[n]),
                })
                .unwrap();
            data.bindings
                .event_events
                .insert(normalized::events::NormalizedCallEvent {
                    site: id(20),
                    origin: id(n),
                    context: id(21),
                    owner: id(22),
                })
                .unwrap();
        }
        data.bindings
            .event_events
            .insert(normalized::events::NormalizedCallEvent {
                site: id(20),
                origin: id(3),
                context: id(23),
                owner: id(22),
            })
            .unwrap();
        let retained = budget.reserved();
        {
            let discovery = Discovery::new(&data, &[], &budget).unwrap();
            assert_eq!(
                discovery.headers[&id(9)]
                    .iter()
                    .map(|r| r.id())
                    .collect::<Vec<_>>(),
                data.headers.iter().map(Record::id).collect::<Vec<_>>()
            );
            assert_eq!(discovery.symbol_declarations[&selected.symbol].len(), 2);
            let events = &discovery.events[&(id(20), id(21))];
            let expected = data
                .bindings
                .event_events
                .iter()
                .filter(|r| r.context == id(21))
                .map(Record::id)
                .collect::<Vec<_>>();
            assert_eq!(events.iter().map(|r| r.id()).collect::<Vec<_>>(), expected);
            assert_eq!(discovery.events[&(id(20), id(23))].len(), 1);
            assert!(
                prepare_signature(&data, &discovery, selected.id(), &budget)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("declaration ambiguous")
            );
        }
        assert_eq!(budget.reserved(), retained);
    }
}

#[cfg(test)]
mod exhausted_candidate_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn place(data: &mut SummaryData, n: u8, occurrence: bool) -> Id<Place> {
        let root = if occurrence {
            PlaceRoot::Occurrence { occurrence: id(n) }
        } else {
            PlaceRoot::Entry { declaration: id(n) }
        };
        let value = Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        };
        data.vocabulary
            .roots
            .insert(&mut data.vocabulary.charge, root.id(), root)
            .unwrap();
        data.vocabulary
            .places
            .insert(&mut data.vocabulary.charge, value.id(), value.clone())
            .unwrap();
        value.id()
    }
    fn branch(
        endpoints: (Id<EntityRef>, Id<Place>, Id<Place>),
        origin: Id<SummaryOrigin>,
        proof: u8,
        q: &AssertionQualification,
        cost: ProofCost,
        budget: &ResourceBudget,
    ) -> WorkBranch {
        let (owner, input, output) = endpoints;
        let key = TransferKey::from_descriptor(TransferDescriptor {
            owner,
            input,
            output,
            context: q.context,
            scope: q.scope,
            modality: q.modality,
            approximation: q.approximation,
            kind: TransferKind::Derived,
            call_site: None,
            provenance: ProvenanceClass::Composed,
        });
        WorkBranch {
            branch: TransferBranch::new(key, q.clone(), Diagram::always(), budget).unwrap(),
            premise: SummaryPremise::Local {
                alternative: id(proof),
            },
            origin,
            facts: SourceFacts {
                qualification: q.id(),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                heuristic: false,
            },
            cost,
        }
    }
    #[test]
    fn published_components_keep_frame_global_member_limit_and_shared_callee_progress() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let (_, definition) =
            super::super::configuration::summaries(id(1), Default::default()).unwrap();
        let (invocation, _) =
            owner::AnalysisInvocation::new(id(2), id(3), definition.id(), None, []);
        let q = AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: invocation.context,
            scope: id(4),
            condition: Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        let mut data = SummaryData::new(&budget);
        let input = place(&mut data, 5, false);
        let output = place(&mut data, 6, false);
        let callee = branch(
            (id(7), input, output),
            id(8),
            9,
            &q,
            ProofCost::SOURCE,
            &budget,
        );
        let later_caller = branch(
            (id(10), input, output),
            id(11),
            12,
            &q,
            ProofCost::SOURCE,
            &budget,
        );
        let mut progress = Progress::new(
            super::super::configuration::SummaryLimits {
                members: 1,
                ..Default::default()
            },
            &budget,
        );
        assert_eq!(
            progress.admit(&callee, &data.vocabulary).unwrap(),
            summary_worklist::Admission::Advanced
        );
        let mut out = SummaryRecords::new(invocation.id(), &budget);
        publish(
            &mut out,
            &invocation,
            &definition,
            &progress,
            &[id(7)],
            ContentHash::of(b"callee"),
            &budget,
        )
        .unwrap();
        assert!(progress.current(callee.id()));
        assert_eq!(progress.by_owner[&id(7)].len(), 1);
        // Publishing one SCC must not reset the frame-wide bound for a later SCC.
        assert_eq!(
            progress.admit(&later_caller, &data.vocabulary).unwrap(),
            summary_worklist::Admission::Refused(obligation::ObligationKind::SummaryProofLimit)
        );
        assert!(progress.current(callee.id()));
        assert_eq!(progress.branches.len(), 1);
        drop(out);
        drop(progress);
        drop(later_caller);
        drop(callee);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
    /// Actual private production context; no graph service or replacement queue model.
    fn exercise(
        callers: u8,
        callees: u8,
        distinct_outputs: bool,
        recursive: bool,
        competing: bool,
        exceptions: u8,
    ) {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        {
            let (_, definition) =
                super::super::configuration::summaries(id(1), Default::default()).unwrap();
            let (invocation, _) =
                owner::AnalysisInvocation::new(id(2), id(3), definition.id(), None, []);
            let q = AssertionQualification {
                assumptions: assumptions::AssumptionSet::empty_id(),
                context: invocation.context,
                scope: id(4),
                condition: Diagram::always().id(),
                modality: attribution::Modality::Definite,
                approximation: Approximation::Exact,
            };
            let mut data = SummaryData::new(&budget);
            let event = normalized::events::NormalizedCallEvent {
                site: id(5),
                origin: id(6),
                context: invocation.context,
                owner: id(7),
            };
            let target = CallTarget {
                qualification: q.id(),
                site: event.site,
                origin: event.origin,
                destination: id(8),
                channel: id(9),
                phase: CallPhase::Call,
                receiver: id(10),
                implicit: false,
                receiver_class: None,
                passing: None,
                class_method: None,
                static_method: None,
            };
            let source = normalized::events::CallAlternativeSource::Native {
                target: target.id(),
            };
            let alternative = normalized::events::NormalizedCallAlternative {
                event: event.id(),
                source: source.id(),
                resolution: None,
                correspondence: None,
                entity: Some(id(12)),
                status: ResolutionStatus::Resolved,
                reason: normalized::links::LinkReason::ExplicitIdentity,
            };
            let attempt = CallBindingAttempt {
                alternative: alternative.id(),
                variant: None,
                syntax: None,
                policy: ContentHash::of(b"candidate-control"),
                event: event.id(),
                signature: None,
                arguments: None,
                receiver: target.receiver,
                receiver_assessment: None,
                dispatch_member: None,
                effective: None,
                adjustment: normalized::callables::SignatureAdjustment::None,
                authority:
                    normalized::signature_applicability::BindingAuthority::EffectiveInvocation,
                authority_reason: normalized::signature_applicability::AuthorityReason::Established,
                outcome: BindingOutcome::Bound,
                reason: BindingReason::Bound,
                refusal: None,
                bindings: ContentHash::of(b"bindings"),
            };
            data.bindings.event_events.insert(event.clone()).unwrap();
            data.bindings.targets.insert(target.clone()).unwrap();
            data.bindings
                .event_alternative_sources
                .insert(source)
                .unwrap();
            data.bindings
                .event_alternatives
                .insert(alternative)
                .unwrap();
            data.binding_output
                .attempts
                .insert(attempt.clone())
                .unwrap();
            let caller_owner = id(11);
            let callee_owner = if recursive { caller_owner } else { id(12) };
            let calls = [CallInfo {
                attempt: attempt.id(),
                _event: event.id(),
                _header: None,
                owner: caller_owner,
                callee: callee_owner,
                target: target.id(),
            }];
            let caller_output = place(&mut data, 13, true);
            let mut progress = Progress::new(Default::default(), &budget);
            let mut seeds = Vec::new();
            for n in 0..callers {
                let input = place(&mut data, 30 + n, false);
                let value = branch(
                    (caller_owner, input, caller_output),
                    id(14),
                    50 + n,
                    &q,
                    if competing && n == 0 {
                        ProofCost {
                            depth: 1,
                            steps: 3,
                            rank: 1,
                        }
                    } else {
                        ProofCost::SOURCE
                    },
                    &budget,
                );
                progress.admit(&value, &data.vocabulary).unwrap();
                if n == 0 {
                    seeds.push(value.clone());
                }
                if competing && n == 0 {
                    let other = branch(
                        (caller_owner, input, caller_output),
                        value.origin,
                        80,
                        &q,
                        ProofCost {
                            depth: 2,
                            steps: 2,
                            rank: 2,
                        },
                        &budget,
                    );
                    assert_eq!(
                        progress.admit(&other, &data.vocabulary).unwrap(),
                        summary_worklist::Admission::Advanced
                    );
                }
            }
            for n in 0..callees {
                let input = place(&mut data, 100 + n, false);
                let output = place(
                    &mut data,
                    if distinct_outputs { 130 + n } else { 130 },
                    true,
                );
                let value = branch(
                    (callee_owner, input, output),
                    id(15),
                    160 + n,
                    &q,
                    if exceptions > 0 {
                        ProofCost {
                            depth: 2,
                            steps: 5,
                            rank: 2,
                        }
                    } else {
                        ProofCost::SOURCE
                    },
                    &budget,
                );
                progress.admit(&value, &data.vocabulary).unwrap();
                if n == 0 {
                    seeds.push(value);
                }
            }
            let first_caller = progress
                .branches
                .values()
                .find(|b| b.descriptor().owner == caller_owner)
                .unwrap()
                .id();
            let callee_ids = progress
                .branches
                .values()
                .filter(|b| b.descriptor().owner == callee_owner)
                .map(WorkBranch::id)
                .collect::<Vec<_>>();
            let replacement = if exceptions > 0 {
                let old = &progress.branches[&callee_ids[usize::from(exceptions - 1)]];
                Some((
                    old.id(),
                    branch(
                        (
                            callee_owner,
                            old.descriptor().input,
                            old.descriptor().output,
                        ),
                        old.origin,
                        250,
                        &q,
                        ProofCost::SOURCE,
                        &budget,
                    ),
                ))
            } else {
                None
            };
            let mut discovery = Discovery::new(&data, &calls, &budget).unwrap();
            for (index, seed) in seeds.iter().enumerate() {
                discovery.seeds.insert(seed.origin, index);
            }
            let mut component = vec![caller_owner, callee_owner];
            component.sort();
            component.dedup();
            let component_id = ContentHash::of(b"component");
            let queue_limit = if exceptions > 0 { 3 } else { 1 };
            let mut queue = summary_worklist::WorkQueue::new(queue_limit, &budget);
            let mut exhausted = ExhaustedGroups::new(&budget);
            let mut processed = charged::ChargedSet::default();
            let mut processed_charge = StateCharge::new(&budget, "exhausted-control-processed");
            if exceptions > 0 {
                let item = WorkItem::Call(attempt.id(), first_caller, callee_ids[0]);
                assert_eq!(queue.insert(item).unwrap(), Ok(true));
                assert_eq!(queue.pop().unwrap(), Some(item));
                processed.insert(&mut processed_charge, item).unwrap();
            }
            let mut out = SummaryRecords::new(invocation.id(), &budget);
            for value in progress.branches.values() {
                retain_proof(&mut out, &invocation, value).unwrap();
            }
            let proof_ids = out.premises.iter().map(Record::id).collect::<BTreeSet<_>>();
            let mut constructed = 0;
            for value in progress
                .branches
                .values()
                .filter(|b| b.descriptor().owner == caller_owner)
            {
                constructed += enqueue(
                    EnqueueContext {
                        discovery: &discovery,
                        data: &data,
                        vocabulary: &data.vocabulary,
                        calls: &calls,
                        component: &component,
                        component_id,
                        invocation: &invocation,
                        progress: &progress,
                        seeds: &seeds,
                    },
                    value.id(),
                    &mut queue,
                    &processed,
                    &mut exhausted,
                    &mut out,
                )
                .unwrap();
            }
            assert_eq!(queue.work(), queue_limit);
            assert_eq!(queue.pending().count(), 1);
            // Independent finite Cartesian expectation: every refused pair maps to the complete
            // model residual tuple; input proof identity is deliberately absent from that tuple.
            let pending = queue.pending().copied().collect::<BTreeSet<_>>();
            let mut expected = SummaryRecords::new(invocation.id(), &budget);
            let over = AssertionQualification {
                approximation: Approximation::Over,
                ..q.clone()
            };
            for caller in progress
                .branches
                .values()
                .filter(|b| b.descriptor().owner == caller_owner)
            {
                for callee in progress
                    .branches
                    .values()
                    .filter(|b| b.descriptor().owner == callee_owner)
                {
                    let item = WorkItem::Call(attempt.id(), caller.id(), callee.id());
                    if pending.contains(&item) || processed.contains(&item) {
                        continue;
                    }
                    expected
                        .vocabulary
                        .qualified(&over, &Diagram::always())
                        .unwrap();
                    expected
                        .residuals
                        .insert(SummaryResidual {
                            invocation: invocation.id(),
                            component: component_id,
                            origin: caller.origin,
                            event: event.id(),
                            channel: analysis::AnalysisChannel::Value,
                            phase: CallPhase::Call,
                            input: caller.descriptor().input,
                            output: callee.descriptor().output,
                            reason: obligation::ObligationKind::SummaryPairWorkLimit,
                            qualification: over.id(),
                        })
                        .unwrap();
                    let subject = owner::ObligationSubject::SourceCall {
                        occurrence: event.site,
                    };
                    expected
                        .obligations
                        .insert(owner::AnalysisObligation {
                            invocation: invocation.id(),
                            subject: subject.id(),
                            channel: analysis::AnalysisChannel::Value,
                            phase: CallPhase::Call,
                            qualification: over.id(),
                            reason: obligation::ObligationKind::SummaryPairWorkLimit,
                            responsible: analysis::AnalysisMethod::Summaries,
                        })
                        .unwrap();
                    expected.subjects.insert(subject).unwrap();
                }
            }
            assert!(out.residuals.same(&expected.residuals));
            assert!(out.obligations.same(&expected.obligations));
            assert!(out.subjects.same(&expected.subjects));
            assert!(
                out.vocabulary
                    .qualifications
                    .values()
                    .eq(expected.vocabulary.qualifications.values())
            );
            assert_eq!(constructed, out.residuals.len());
            assert_eq!(
                out.premises.iter().map(Record::id).collect::<BTreeSet<_>>(),
                proof_ids
            );
            assert_eq!(out.proof_origins.len(), progress.branches.len());
            assert_eq!(out.costs.len(), progress.branches.len());
            if !recursive && !competing && exceptions == 0 {
                assert_eq!(
                    constructed,
                    if distinct_outputs {
                        usize::from(callers * callees) - 1
                    } else {
                        usize::from(callers)
                    }
                );
            }
            if !recursive && !competing && exceptions == 0 {
                assert!(
                    exhausted.examined <= constructed + 3,
                    "examined {} pairs for {} residuals",
                    exhausted.examined,
                    constructed
                );
            }
            let examined = exhausted.examined;
            // Repeating discovery constructs no new rich products and consumes no queue work.
            for value in progress
                .branches
                .values()
                .filter(|b| b.descriptor().owner == caller_owner)
            {
                assert_eq!(
                    enqueue(
                        EnqueueContext {
                            discovery: &discovery,
                            data: &data,
                            vocabulary: &data.vocabulary,
                            calls: &calls,
                            component: &component,
                            component_id,
                            invocation: &invocation,
                            progress: &progress,
                            seeds: &seeds
                        },
                        value.id(),
                        &mut queue,
                        &processed,
                        &mut exhausted,
                        &mut out
                    )
                    .unwrap(),
                    0
                );
            }
            assert_eq!(queue.work(), queue_limit);
            assert_eq!(exhausted.examined, examined);
            if let Some((retired, replacement)) = replacement {
                assert!(
                    out.residuals.is_empty(),
                    "pending and processed are not refused work"
                );
                assert_eq!(
                    progress.admit(&replacement, &data.vocabulary).unwrap(),
                    summary_worklist::Admission::Advanced
                );
                assert!(!progress.current(retired));
                assert!(progress.current(replacement.id()));
                retain_proof(&mut out, &invocation, &replacement).unwrap();
                assert_eq!(
                    enqueue(
                        EnqueueContext {
                            discovery: &discovery,
                            data: &data,
                            vocabulary: &data.vocabulary,
                            calls: &calls,
                            component: &component,
                            component_id,
                            invocation: &invocation,
                            progress: &progress,
                            seeds: &seeds
                        },
                        replacement.id(),
                        &mut queue,
                        &processed,
                        &mut exhausted,
                        &mut out
                    )
                    .unwrap(),
                    1
                );
                let caller = &progress.branches[&first_caller];
                expected
                    .vocabulary
                    .qualified(&over, &Diagram::always())
                    .unwrap();
                expected
                    .residuals
                    .insert(SummaryResidual {
                        invocation: invocation.id(),
                        component: component_id,
                        origin: caller.origin,
                        event: event.id(),
                        channel: analysis::AnalysisChannel::Value,
                        phase: CallPhase::Call,
                        input: caller.descriptor().input,
                        output: replacement.descriptor().output,
                        reason: obligation::ObligationKind::SummaryPairWorkLimit,
                        qualification: over.id(),
                    })
                    .unwrap();
                let subject = owner::ObligationSubject::SourceCall {
                    occurrence: event.site,
                };
                expected
                    .obligations
                    .insert(owner::AnalysisObligation {
                        invocation: invocation.id(),
                        subject: subject.id(),
                        channel: analysis::AnalysisChannel::Value,
                        phase: CallPhase::Call,
                        qualification: over.id(),
                        reason: obligation::ObligationKind::SummaryPairWorkLimit,
                        responsible: analysis::AnalysisMethod::Summaries,
                    })
                    .unwrap();
                expected.subjects.insert(subject).unwrap();
                assert!(out.residuals.same(&expected.residuals));
                assert!(out.obligations.same(&expected.obligations));
                assert!(out.subjects.same(&expected.subjects));
                assert!(
                    out.vocabulary
                        .qualifications
                        .values()
                        .eq(expected.vocabulary.qualifications.values())
                );
                assert!(
                    out.premises.get(retired.1).is_some(),
                    "retired representative proof remains published"
                );
                assert_eq!(out.proof_origins.len(), progress.branches.len());
                assert_eq!(out.costs.len(), progress.branches.len());
                assert_eq!(queue.work(), queue_limit);
                assert_eq!(queue.pending().copied().collect::<BTreeSet<_>>(), pending);
            }
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn equivalent_exhausted_candidates_construct_one_complete_residual() {
        exercise(1, 5, false, false, false, 0);
    }
    #[test]
    fn equivalent_callee_inputs_and_distinct_caller_inputs_examine_only_residual_groups() {
        exercise(12, 20, false, false, false, 0);
    }
    #[test]
    fn distinct_exhausted_candidates_preserve_every_output() {
        exercise(1, 5, true, false, false, 0);
    }
    #[test]
    fn distinct_caller_inputs_and_callee_outputs_retain_necessary_cross_product() {
        exercise(4, 5, true, false, false, 0);
    }
    #[test]
    fn processed_only_exception_does_not_hide_new_current_generation() {
        exercise(1, 2, false, false, false, 1);
    }
    #[test]
    fn pending_only_exception_does_not_hide_new_current_generation() {
        exercise(1, 2, false, false, false, 2);
    }
    #[test]
    fn recursive_endpoints_and_incomparable_proofs_preserve_full_inventory() {
        exercise(2, 4, true, true, true, 0);
    }
}

#[cfg(test)]
mod question_domain_controls {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn operand(input: u8, output: u8, scope: u8, budget: &ResourceBudget) -> TransferBranch<transfer::local::TransferKey> {
        let q = AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(), context: id(20), scope: id(scope),
            condition: Diagram::always().id(), modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        TransferBranch::new(transfer::local::TransferKey::from_descriptor(TransferDescriptor {
            owner: id(21), input: id(input), output: id(output), context: q.context, scope: q.scope,
            modality: q.modality, approximation: q.approximation, kind: TransferKind::Identity,
            call_site: None, provenance: ProvenanceClass::Composed,
        }), q, Diagram::always(), budget).unwrap()
    }
    #[test]
    fn disjoint_and_subsumed_questions_use_original_operands_and_caller_qualification() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let caller = operand(2, 3, 4, &budget);
        let callee = operand(5, 6, 7, &budget);
        for result in [CallComposition::Disjoint, CallComposition::Subsumed] {
            let mut domain = QuestionDomain::new(&budget);
            domain.observe(id(1), &caller, &callee, &[result]).unwrap();
            assert_eq!(domain.questions.iter().copied().collect::<Vec<_>>(),
                vec![(id(1), id(2), id(6), caller.qualification().id())]);
        }
        let mut domain = QuestionDomain::new(&budget);
        domain.observe(id(1), &caller, &callee, &[]).unwrap();
        assert!(domain.questions.is_empty(), "no result or unprocessed candidate has no question");
    }
    #[test]
    fn question_domain_coalesces_discoveries_without_erasing_caller_qualification() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut domain = QuestionDomain::new(&budget);
        let question = (id(1), id(2), id(3), id(4));
        domain.discover(question).unwrap();
        let once = budget.reserved();
        for _ in 0..100 {
            domain.discover(question).unwrap();
        }
        assert_eq!(domain.questions.len(), 1);
        assert_eq!(budget.reserved(), once, "duplicate pairs retain no trace backing");
        domain.discover((id(1), id(2), id(3), id(5))).unwrap();
        domain.discover((id(6), id(2), id(3), id(4))).unwrap();
        assert_eq!(domain.questions.len(), 3);
        drop(domain);
        assert_eq!(budget.reserved(), 0);
    }
}
