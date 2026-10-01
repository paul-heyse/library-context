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
        self,
        summary as owner,
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
use crate::{Domain, DomainCode, DomainSum};
use std::collections::{BTreeMap, BTreeSet};
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
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_runs",invariants=super::summary_replay::invariants,publication_checks=super::summary_replay::profile_checks)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PairDisposition {
    Disjoint = 0,
    Subsumed = 1,
    Proven = 2,
    Refused = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "summary_pair_outcomes")]
pub struct PairOutcome {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
    #[model(key)]
    pub caller_origin: Id<SummaryOrigin>,
    #[model(key)]
    pub callee_origin: Id<SummaryOrigin>,
    #[model(key)]
    pub caller: Id<SummaryPremise>,
    #[model(key)]
    pub callee: Id<SummaryPremise>,
    #[model(key)]
    pub disposition: PairDisposition,
    #[model(key)]
    pub witness: Option<Id<SummaryWitness>>,
    #[model(key)]
    pub reason: Option<obligation::ObligationKind>,
}
#[macro_export]
macro_rules! summary_owned_inputs{($apply:ident)=>{$apply!{
 symbolic_classes:$crate::domain::normalized::symbolic_fields::SourceFieldClass,symbolic_stores:$crate::domain::normalized::symbolic_fields::SourceFieldStore,symbolic_associations:$crate::domain::normalized::symbolic_fields::SourceFieldAssociation,symbolic_readers:$crate::domain::normalized::symbolic_fields::SourceFieldReader,symbolic_links:$crate::domain::normalized::symbolic_fields::SourceFieldReaderLink,symbolic_local_stores:$crate::domain::local_symbolic::SymbolicFieldStore,
 local_invocations:$crate::domain::analysis::local::AnalysisInvocation,model_invocations:$crate::domain::analysis::model::AnalysisInvocation,enriched_invocations:$crate::domain::analysis::enriched_execution::AnalysisInvocation,source_invocations:$crate::domain::analysis::source_call::AnalysisInvocation,
 local_outcomes:$crate::domain::analysis::local::AnalysisOutcome,model_outcomes:$crate::domain::analysis::model::AnalysisOutcome,enriched_outcomes:$crate::domain::analysis::enriched_execution::AnalysisOutcome,source_outcomes:$crate::domain::analysis::source_call::AnalysisOutcome,model_derivations:$crate::domain::analysis::model::AnalysisDerivation,
 local_contributions:$crate::domain::local_semantics::LocalContribution,local_guards:$crate::domain::local_semantics::LocalGuardContribution,local_keys:$crate::domain::transfer::local::TransferKey,local_alternatives:$crate::domain::transfer::local::TransferAlternative,local_supports:$crate::domain::transfer::local::TransferSupport,
 model_keys:$crate::domain::transfer::model::TransferKey,model_alternatives:$crate::domain::transfer::model::TransferAlternative,model_supports:$crate::domain::transfer::model::TransferSupport,
 native:$crate::domain::analysis::native::NativeQualification,headers:$crate::domain::execution::source_call_records::SourceCallHeader,parameters:$crate::domain::analysis::MethodParameters,definitions:$crate::domain::analysis::AnalysisDefinition,
 entries:$crate::domain::conditions::entry::EntryValueWitness,entry_sources:$crate::domain::conditions::entry::EntryAccessSource,stability:$crate::domain::conditions::stability::StabilityWitness,
}};}
#[macro_export]
macro_rules! summary_vocabulary{($apply:ident)=>{$apply!{
 roots:$crate::domain::value::PlaceRoot,places:$crate::domain::value::Place,paths:$crate::domain::value::AccessPath,segments:$crate::domain::value::PathSegment,literals:$crate::domain::value::Literal,predicates:$crate::domain::value::Predicate,atoms:$crate::domain::conditions::EvaluationAtom,qualifications:$crate::domain::assertion::AssertionQualification,conditions:$crate::domain::conditions::Condition,nodes:$crate::domain::conditions::ConditionNode,
}};}
macro_rules! vocabulary{($($field:ident:$ty:ty,)*)=>{pub struct Vocabulary{$(pub $field:ChargedMap<Id<$ty>,$ty>,)*charge:StateCharge}impl Vocabulary{pub(super) fn new(b:&ResourceBudget)->Self{Self{$($field:Default::default(),)*charge:StateCharge::new(b,"summary-vocabulary")}}pub(super) fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{$(if n==<$ty>::NAME{for row in <$ty>::decode(b)?{self.$field.insert(&mut self.charge,row.id(),row)?;}})*Ok(())}fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]).at_epoch(stages::PublicationBoundary::Model),)*]}}};}
crate::summary_vocabulary!(vocabulary);
macro_rules! data{($($field:ident:$ty:ty,)*)=>{pub struct SummaryData{pub graphs:projection::normalization::ProjectionOutput,pub entry:EntryData,pub bindings:BindingData,pub binding_output:BindingOutput,pub path:PathData,pub vocabulary:Vocabulary,pub local_evidence:analysis::local::support::EvidenceIndex,pub model_evidence:analysis::model::support::EvidenceIndex,$(pub $field:Rows<$ty>,)*}
impl SummaryData{pub fn new(b:&ResourceBudget)->Self{Self{graphs:projection::normalization::ProjectionOutput::new(b),entry:EntryData::new(b),bindings:BindingData::new(b),binding_output:BindingOutput::new(b),path:PathData::new(b),vocabulary:Vocabulary::new(b),local_evidence:analysis::local::support::EvidenceIndex::new(b),model_evidence:analysis::model::support::EvidenceIndex::new(b),$($field:Rows::new(b),)*}}
 pub fn visit_input(&mut self,input:&ValidationInput,b:&arrow_array::RecordBatch)->Result<(),ModelError>{let n=input.name();if stages::is_vocabulary(n){return match input.prefix(){Some(stages::PublicationBoundary::Facts)=>{self.entry.visit(n,b)?;self.bindings.visit(n,b)?;Ok(())},Some(stages::PublicationBoundary::Model)=>self.vocabulary.visit(n,b),_=>Err(invalid("Summary input changes vocabulary prefix"))}}self.visit(n,b)}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{self.graphs.visit(n,b)?;self.entry.visit(n,b)?;self.bindings.visit(n,b)?;self.binding_output.visit(n,b)?;self.path.visit(n,b)?;self.local_evidence.visit(n,b)?;self.model_evidence.visit(n,b)?;$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}
 pub fn inputs()->Vec<ValidationInput>{let mut inputs=BindingData::validation_inputs();inputs.extend(BindingOutput::validation_inputs());inputs.extend(EntryData::validation_inputs().into_iter().map(|i|if stages::is_vocabulary(i.name()){i.at_epoch(stages::PublicationBoundary::Facts)}else{i}));inputs.extend(PathData::inputs());inputs.extend([ValidationInput::of::<projection::ProjectionSourceAssessment>(&["id"]),ValidationInput::of::<projection::ProjectionSnapshot>(&["id"]),ValidationInput::of::<projection::ProjectionSnapshotChunk>(&["id"])]);inputs.extend(Vocabulary::inputs());inputs.extend(analysis::local::support::EvidenceIndex::inputs());inputs.extend(analysis::model::support::EvidenceIndex::inputs());inputs.extend([$(ValidationInput::of::<$ty>(&["id"]),)*]);inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
}};}
crate::summary_owned_inputs!(data);
#[macro_export]
macro_rules! summary_outputs{($apply:ident)=>{$apply!{
 symbolic_alternatives:$crate::domain::execution::summary_symbolic::SymbolicFieldAlternative,
 origin_boundaries:$crate::domain::execution::summary_production::OriginBoundary,pair_outcomes:$crate::domain::execution::summary_production::PairOutcome,
 runs:$crate::domain::execution::summary_production::SummaryRun,components:$crate::domain::execution::summary_production::SummaryComponent,component_members:$crate::domain::execution::summary_production::ComponentMember,origins:$crate::domain::execution::summary_production::SummaryOrigin,proof_origins:$crate::domain::execution::summary_production::ProofOrigin,residuals:$crate::domain::execution::summary_production::SummaryResidual,call_members:$crate::domain::execution::summary_production::CallMember,
 control_witnesses:$crate::domain::execution::summary_control::SummaryControlWitness,
 costs:$crate::domain::execution::summary_proof::SummaryProofCost,path_routes:$crate::domain::execution::summary_path::SummaryPathRoute,path_witnesses:$crate::domain::execution::summary_path::SummaryPathWitness,path_contributions:$crate::domain::execution::summary_path::SummaryPathContribution,
 premises:$crate::domain::transfer::summary::SummaryPremise,witnesses:$crate::domain::transfer::summary::SummaryWitness,contributions:$crate::domain::transfer::summary::SummaryContribution,keys:$crate::domain::transfer::summary::TransferKey,alternatives:$crate::domain::transfer::summary::TransferAlternative,supports:$crate::domain::transfer::summary::TransferSupport,
 influences:$crate::domain::transfer::summary::ControlInfluence,control_supports:$crate::domain::transfer::summary::ControlSupport,selections:$crate::domain::transfer::summary::Selection,substitutions:$crate::domain::conditions::stability::GuardSubstitution,
 claims:$crate::domain::execution::summary_consequences::SummaryClaim,claim_members:$crate::domain::execution::summary_consequences::ClaimMember,claim_standings:$crate::domain::execution::summary_consequences::ClaimStanding,claim_proofs:$crate::domain::execution::summary_consequences::ClaimProof,claim_proof_members:$crate::domain::execution::summary_consequences::ClaimProofMember,refutation_coverage:$crate::domain::execution::summary_consequences::ClaimRefutationCoverage,conclusions:$crate::domain::execution::summary_consequences::ClaimConclusion,obligation_sources:$crate::domain::analysis::summary::ObligationSource,
 sources:$crate::domain::analysis::summary::SupportSource,subjects:$crate::domain::analysis::summary::ObligationSubject,propositions:$crate::domain::analysis::summary::AnalysisProposition,derivations:$crate::domain::analysis::summary::AnalysisDerivation,derivation_premises:$crate::domain::analysis::summary::AnalysisDerivationPremise,obligations:$crate::domain::analysis::summary::AnalysisObligation,discharges:$crate::domain::analysis::summary::DischargeEvidence,
}};}
macro_rules! output{($($field:ident:$ty:ty,)*)=>{pub struct SummaryRecords{$(pub $field:Rows<$ty>,)*pub outcome:owner::AnalysisOutcome,pub vocabulary:Vocabulary,pending:Option<(Vec<super::summary_consequences::PendingDischarge>,StateCharge)>}impl SummaryRecords{pub(super) fn new(i:Id<owner::AnalysisInvocation>,b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*outcome:owner::AnalysisOutcome{invocation:i,status:analysis::AnalysisStatus::Completed,reason:None},vocabulary:Vocabulary::new(b),pending:None}}}};}
crate::summary_outputs!(output);
impl SummaryRecords {
    fn consequences(
        &mut self,
        data: &SummaryData,
        invocation: &owner::AnalysisInvocation,
        definition: &analysis::AnalysisDefinition,
        profile: stages::Profile,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let rows = super::summary_consequences::derive(
            data, self, invocation, definition, profile, budget,
        )?;
        macro_rules! append{($($dst:ident:$src:ident),* $(,)?)=>{$(for row in rows.$src.iter(){self.$dst.insert(row.clone())?;})*};}
        append!(claims,claim_members:members,claim_standings:standings,claim_proofs:proofs,claim_proof_members:proof_members,refutation_coverage,conclusions,keys,premises,subjects,sources,derivations,propositions,derivation_premises,obligations:obligations);
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
            guards: self.guards(),
            paths: &self.paths,
            segments: place_composition::PathCatalog {
                segments: &self.segments,
                literals: &self.literals,
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
        Diagram::from_records(
            self.conditions
                .get(&id)
                .ok_or_else(|| invalid("Summary condition absent"))?,
            &self.nodes.values().cloned().collect::<Vec<_>>(),
        )
    }
    fn composition(&mut self, rows: &CompositionRecords) -> Result<(), ModelError> {
        macro_rules! rows{($($f:ident),*)=>{$(for row in &rows.$f{self.$f.insert(&mut self.charge,row.id(),row.clone())?;})*};}
        rows!(
            roots, places, paths, segments, literals, predicates, atoms, conditions, nodes
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
type GuardInventory = (BTreeMap<Id<EvaluationAtom>, CheckedStability>, Box<dyn Reservation>);

impl SummaryData {
    fn evidence(
        &self,
        out: &SummaryRecords,
        premise: &SummaryPremise,
        budget: &ResourceBudget,
    ) -> Result<TransferEvidence, ModelError> {
        match premise {
            SummaryPremise::Local { alternative } => {
                let a = need(&self.local_alternatives, *alternative)?;
                let _charge = budget.reserve(
                    "summary-local-supports",
                    self.local_supports
                        .len()
                        .saturating_mul(size_of::<transfer::local::TransferSupport>()),
                )?;
                let supports = self
                    .local_supports
                    .iter()
                    .filter(|s| s.assertion == *alternative)
                    .cloned()
                    .collect::<Vec<_>>();
                TransferEvidence::local(a, &supports, &self.local_evidence, budget)
            }
            SummaryPremise::Model { alternative } => {
                let a = need(&self.model_alternatives, *alternative)?;
                let _charge = budget.reserve(
                    "summary-model-supports",
                    self.model_supports
                        .len()
                        .saturating_mul(size_of::<transfer::model::TransferSupport>()),
                )?;
                let supports = self
                    .model_supports
                    .iter()
                    .filter(|s| s.assertion == *alternative)
                    .cloned()
                    .collect::<Vec<_>>();
                TransferEvidence::model(a, &supports, &self.model_evidence, budget)
            }
            SummaryPremise::Witness { witness } => {
                TransferEvidence::witness(need(&out.witnesses, *witness)?, budget)
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
        budget: &ResourceBudget,
    ) -> Result<(Vec<WorkBranch>, Box<dyn Reservation>), ModelError> {
        let charge = budget.reserve(
            "summary-raw-seeds",
            (self.local_contributions.len() + self.model_supports.len())
                .saturating_mul(size_of::<WorkBranch>() + 512),
        )?;
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
            let origin = SummaryOrigin::Local {
                contribution: contribution.id(),
            };
            out.origins.insert(origin.clone())?;
            let premise = SummaryPremise::Local {
                alternative: alternative.id(),
            };
            let evidence = self.evidence(out, &premise, budget)?;
            let facts = SourceFacts {
                qualification: q.id(),
                status: analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    evidence.facts.iter().map(|f| f.status),
                ),
                heuristic: evidence.facts.iter().any(|f| f.heuristic),
            };
            seeds.push(WorkBranch {
                branch: TransferBranch::new(
                    TransferKey::from_descriptor(key.descriptor()),
                    q.clone(),
                    self.vocabulary.diagram(q.condition, budget)?,
                    budget,
                )?,
                premise,
                origin: origin.id(),
                facts,
                cost: ProofCost::SOURCE,
            });
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
                branch: TransferBranch::new(
                    TransferKey::from_descriptor(key.descriptor()),
                    q.clone(),
                    self.vocabulary.diagram(q.condition, budget)?,
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
        seeds.sort_by_key(WorkBranch::id);
        Ok((seeds, charge))
    }
    fn guards(
        &self,
        budget: &ResourceBudget,
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
            let witness = need(&self.stability, guard.stability)?;
            let checked = StabilityWitness::derive(&self.entry, witness.atom, &checked)
                .map_err(|r| invalid(format!("stored stability refuses replay: {r:?}")))?;
            if checked.witness() != witness {
                return Err(invalid("Local guard stability changes"));
            }
            if let Some(old) = results.insert(witness.atom, checked)
                && old.witness() != witness {
                return Err(invalid("multiple stability witnesses for one atom"));
                }
        }
        Ok((results, charge))
    }
}
#[derive(Clone, Copy)]
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
        let occurrence = need(&data.bindings.occurrences, event.site)?;
        if event.context != invocation.context
            || need(&data.bindings.artifacts, occurrence.source)?.input != invocation.input
        {
            continue;
        }
        let mut headers = data.headers.iter().filter(|h| h.attempt == attempt.id());
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
struct CompositionInputs<'a> {
    data: &'a SummaryData,
    verified: &'a VerifiedBindings,
    vocabulary: &'a Vocabulary,
    guards: &'a BTreeMap<Id<EvaluationAtom>, CheckedStability>,
}

fn compose(
    composition_inputs: CompositionInputs<'_>,
    call: &CallInfo,
    caller: &WorkBranch,
    callee: &WorkBranch,
    budget: &ResourceBudget,
) -> Result<CompositionResults, ModelError> {
    let CompositionInputs { data, verified, vocabulary, guards } = composition_inputs;
    let attempt = need(&data.binding_output.attempts, call.attempt)?;
    let target = need(&data.bindings.targets, call.target)?;
    let bound = verified
        .bound(call.attempt)
        .ok_or_else(|| invalid("Summary bound call absent"))?;
    let signature = need(&data.bindings.signatures, bound.bound().signature())?;
    let symbol = need(&data.bindings.symbols, signature.symbol)?;
    let mut declarations = data
        .entry
        .symbol_declarations
        .iter()
        .filter(|d| d.symbol == symbol.id());
    let declaration = declarations
        .next()
        .ok_or_else(|| invalid("Summary callee source declaration absent"))?;
    if declarations.next().is_some() {
        return Err(invalid("Summary callee declaration ambiguous"));
    }
    let admission = verified
        .composition(call.attempt)
        .ok_or_else(|| invalid("Summary composition admission absent"))?;
    let mut callers = data
        .entry
        .symbol_declarations
        .iter()
        .filter(|d| d.declaration == admission.owner_declaration());
    let caller_declaration = callers
        .next()
        .ok_or_else(|| invalid("Summary caller declaration absent"))?;
    if callers.next().is_some() {
        return Err(invalid("Summary caller declaration ambiguous"));
    }
    let bytes = data
        .bindings
        .parameters
        .len()
        .saturating_mul(size_of::<SignatureParameter>())
        .saturating_add(
            data.entry
                .declarations
                .len()
                .saturating_mul(size_of::<declarations::ParameterDeclaration>()),
        )
        .saturating_add(
            data.bindings
                .arguments
                .len()
                .saturating_mul(size_of::<CallArgument>() + 128),
        );
    let _frame = budget.reserve("summary-call-frame", bytes)?;
    let mut parameters = data
        .bindings
        .parameters
        .iter()
        .filter(|p| p.signature == signature.id())
        .cloned()
        .collect::<Vec<_>>();
    parameters.sort_by_key(|p| p.ordinal);
    let links = data
        .entry
        .declarations
        .iter()
        .filter(|p| parameters.iter().any(|v| v.id() == p.parameter))
        .cloned()
        .collect::<Vec<_>>();
    let syntax = attempt
        .syntax
        .ok_or_else(|| invalid("Summary call syntax absent"))?;
    let mut arguments = data
        .bindings
        .arguments
        .iter()
        .filter(|a| a.call == syntax)
        .cloned()
        .collect::<Vec<_>>();
    arguments.sort_by_key(|a| a.ordinal);
    let frame = CallFrame {
        site: need(&data.bindings.occurrences, target.site)?,
        target,
        qualification: need(&data.bindings.qualifications, target.qualification)?,
        destination: need(&data.bindings.destinations, target.destination)?,
        receiver: need(&data.bindings.receivers, target.receiver)?,
        binding: Some(
            CallBindingFrame::new(verified, call.attempt, &data.bindings, &data.binding_output)
                .map_err(|r| invalid(format!("Summary checked call refused: {r:?}")))?,
        ),
        arguments: &arguments,
    };
    compose_call(
        caller,
        callee,
        &frame,
        &CallerFrame {
            declaration: caller_declaration,
            owner: call.owner,
        },
        &CalleeFrame {
            symbol,
            declaration,
            parameters: &parameters,
            links: &links,
            witnesses: Some(guards),
        },
        &vocabulary.catalog(),
        budget,
    )
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
    charge: StateCharge,
}
impl Progress {
    fn new(limits: super::configuration::SummaryLimits, b: &ResourceBudget) -> Self {
        Self {
            branches: Default::default(),
            by_owner: Default::default(),
            frontier: summary_worklist::Frontier::new(limits, b),
            charge: StateCharge::new(b, "finite-summary-progress"),
        }
    }
    fn admit(&mut self, branch: &WorkBranch) -> Result<summary_worklist::Admission, ModelError> {
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
            self.charge
                .grow(size_of::<ProofId>().saturating_mul(2).saturating_add(128))?;
            self.by_owner
                .update(&mut self.charge, branch.descriptor().owner, |rows| {
                    rows.insert(branch.id())
                })?;
            self.branches
                .insert(&mut self.charge, branch.id(), branch.clone())?;
        }
        Ok(result)
    }
    fn current(&self, id: ProofId) -> bool {
        self.branches
            .get(&id)
            .is_some_and(|b| self.frontier.current(&b.semantic(), id))
    }
}
fn path_candidates(
    data: &SummaryData,
    vocabulary: &Vocabulary,
    branch: &WorkBranch,
    mut consume: impl FnMut(WorkItem) -> Result<(), ModelError>,
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
    for definition in data
        .entry
        .definition_observations
        .iter()
        .filter(|d| d.value == Some(*site))
    {
        let target = ReachingDefinition::Bound {
            definition: definition.definition,
        };
        for reaching in data
            .entry
            .reaching
            .iter()
            .filter(|r| r.target == target.id())
        {
            for value in data
                .entry
                .values
                .iter()
                .filter(|v| v.use_ == reaching.use_ && !v.through_call)
            {
                for observed in data
                    .entry
                    .use_observations
                    .iter()
                    .filter(|u| u.use_ == value.use_)
                {
                    for vs in data
                        .entry
                        .value_supports
                        .iter()
                        .filter(|s| s.assertion == value.id())
                    {
                        for rs in data
                            .entry
                            .reaching_supports
                            .iter()
                            .filter(|s| s.assertion == reaching.id())
                        {
                            for ds in data
                                .entry
                                .definition_supports
                                .iter()
                                .filter(|s| s.assertion == definition.id())
                            {
                                for us in data
                                    .entry
                                    .use_supports
                                    .iter()
                                    .filter(|s| s.assertion == observed.id())
                                {
                                    consume(WorkItem::Alias(
                                        branch.id(),
                                        vs.id(),
                                        rs.id(),
                                        ds.id(),
                                        us.id(),
                                    ))?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for step in data.path.call_steps.iter().filter(|s| s.call == *site) {
        for path in data.path.value_paths.iter().filter(|p| p.path == step.path) {
            let value = need(&data.entry.values, path.value)?;
            let use_ = need(&data.entry.uses, value.use_)?;
            let original = need(&data.entry.places, use_.place)?;
            if data.entry.roots.get(original.root)
                != Some(&PlaceRoot::Formal {
                    declaration: *declaration,
                })
                || !data
                    .entry
                    .owners
                    .iter()
                    .any(|o| o.occurrence == use_.occurrence && o.entity == descriptor.owner)
            {
                continue;
            }
            for vs in data
                .entry
                .value_supports
                .iter()
                .filter(|s| s.assertion == value.id())
            {
                for ps in data
                    .path
                    .value_path_supports
                    .iter()
                    .filter(|s| s.assertion == path.id())
                {
                    consume(WorkItem::Path(branch.id(), vs.id(), ps.id(), step.id()))?;
                }
            }
        }
    }
    Ok(())
}
fn candidates(
    data: &SummaryData,
    vocabulary: &Vocabulary,
    calls: &[CallInfo],
    component: &[Id<EntityRef>],
    progress: &Progress,
    new: ProofId,
    mut consume: impl FnMut(WorkItem) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let branch = progress
        .branches
        .get(&new)
        .ok_or_else(|| invalid("queued summary proof absent"))?;
    if !progress.current(new) {
        return Ok(());
    }
    let caller_eligible = |branch: &WorkBranch| {
        vocabulary
            .places
            .get(&branch.descriptor().output)
            .and_then(|p| vocabulary.roots.get(&p.root))
            .is_some_and(|r| matches!(r, PlaceRoot::Occurrence { .. }))
    };
    for call in calls
        .iter()
        .filter(|c| component.binary_search(&c.owner).is_ok())
    {
        if call.owner == branch.descriptor().owner && caller_eligible(branch)
            && let Some(rows) = progress.by_owner.get(&call.callee) {
            for other in rows {
                if progress.current(*other) {
                    consume(WorkItem::Call(call.attempt, new, *other))?;
                }
            }
            }
        if call.callee == branch.descriptor().owner
            && let Some(rows) = progress.by_owner.get(&call.owner) {
            for other in rows {
                if progress.current(*other) && caller_eligible(&progress.branches[other]) {
                    consume(WorkItem::Call(call.attempt, *other, new))?;
                }
            }
            }
    }
    path_candidates(data, vocabulary, branch, consume)
}
struct ResidualContext<'a> {
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
    let ResidualContext { data, invocation, component, progress, seeds } = residual_context;
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
            let event = data
                .bindings
                .event_events
                .iter()
                .find(|e| e.site == site && e.context == invocation.context)
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
            let mut events = data
                .bindings
                .event_events
                .iter()
                .filter(|e| e.site == step.call && e.context == invocation.context);
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
    let seed = seeds
        .iter()
        .find(|b| b.origin == origin)
        .ok_or_else(|| invalid("summary origin seed absent"))?;
    let q = AssertionQualification {
        approximation: Approximation::Over,
        ..seed.qualification().clone()
    };
    out.vocabulary.qualified(&q, seed.condition())?;
    out.residuals.insert(SummaryResidual {
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
    })?;
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
    Ok(())
}
struct EnqueueContext<'a> {
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
    out: &mut SummaryRecords,
) -> Result<(), ModelError> {
    let EnqueueContext { data, vocabulary, calls, component, component_id, invocation, progress, seeds } = enqueue_context;
    candidates(data, vocabulary, calls, component, progress, new, |item| {
        if processed.contains(&item) {
            return Ok(());
        }
        if let Err(reason) = queue.insert(item)? {
            residual(
                ResidualContext { data, invocation, component: component_id, progress, seeds },
                item,
                reason,
                out,
            )?;
        }
        Ok(())
    })
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
    let mut members = charged::ChargedMap::<Id<TransferKey>, BTreeSet<ProofId>>::default();
    let mut charge = StateCharge::new(budget, "summary-publication-members");
    for branch in progress.branches.values().filter(|b| {
        component.binary_search(&b.descriptor().owner).is_ok()
            && matches!(
                b.premise,
                SummaryPremise::Witness { .. } | SummaryPremise::Path { .. }
            )
    }) {
        charge.grow(size_of::<ProofId>() + 64)?;
        members.update(&mut charge, branch.branch.key().id(), |rows| {
            rows.insert(branch.id())
        })?;
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
    for id in ids {
        let branch = &progress.branches[id];
        let source = match branch.premise {
            SummaryPremise::Witness { witness } => {
                owner::SupportSource::TransferWitness { witness }
            }
            SummaryPremise::Path { witness } => owner::SupportSource::PathWitness { witness },
            _ => unreachable!(),
        };
        proof_sources.insert(source)?;
    }
    let mut evidence = Vec::new();
    for source in proof_sources.iter() {
        let branch = ids
            .iter()
            .map(|id| &progress.branches[id])
            .find(|b| match (source, &b.premise) {
                (
                    owner::SupportSource::TransferWitness { witness: a },
                    SummaryPremise::Witness { witness: b },
                ) => a == b,
                (
                    owner::SupportSource::PathWitness { witness: a },
                    SummaryPremise::Path { witness: b },
                ) => a == b,
                _ => false,
            })
            .ok_or_else(|| invalid("aggregate witness branch absent"))?;
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
            if (iq.scope, iq.context)
                == (
                    qualified.qualification.scope,
                    qualified.qualification.context,
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
    graph: &MaterializedGraph,
    budget: &ResourceBudget,
) -> Result<SummaryRecords, ModelError> {
    let limits = summary_proof::limits(need(&data.parameters, definition.parameters)?, definition)?;
    if invocation.definition != definition.id()
        || invocation.subject.is_some()
        || graph.key()
            != (ProjectionKey {
                input: invocation.input,
                context: invocation.context,
                name: projection::ProjectionName::CallableInvocation,
            })
    {
        return Err(invalid(
            "Summary invocation or graph changes its exact frame",
        ));
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
        out.consequences(data, invocation, definition, profile, budget)?;
        return Ok(out);
    }
    let verified =
        normalized::binding_normalization::verify(&data.bindings, &data.binding_output, budget)?;
    let schedule = super::summary_schedule::invocation_sccs(graph, budget)?;
    let (calls, _calls) = call_infos(data, &verified, invocation, &mut out, budget)?;
    let (seeds, _seeds) = data.seeds(invocation, &mut out, budget)?;
    let (guards, _guards) = data.guards(budget)?;
    let mut vocabulary = data.vocabulary.copy(budget)?;
    let mut progress = Progress::new(limits, budget);
    // Projection omissions remain explicit even when no component can own a seed.
    for seed in &seeds {
        if !schedule
            .components()
            .iter()
            .any(|members| members.binary_search(&seed.descriptor().owner).is_ok())
        {
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
        let mut digest = KeySink::new("summary-scc-members");
        for entity in component {
            entity.encode(&mut digest);
        }
        let component_id = digest.finish();
        let mut queue = summary_worklist::WorkQueue::new(limits.work, budget);
        let mut processed = charged::ChargedSet::default();
        let mut processed_charge = StateCharge::new(budget, "summary-processed-pairs");
        for seed in seeds
            .iter()
            .filter(|b| component.binary_search(&b.descriptor().owner).is_ok())
        {
            if let summary_worklist::Admission::Refused(reason) = progress.admit(seed)? {
                origin_boundary(&mut out, invocation, component_id, seed, reason)?;
            }
        }
        let _initial = budget.reserve(
            "summary-component-initial",
            progress.branches.len().saturating_mul(size_of::<ProofId>()),
        )?;
        let initial = progress
            .branches
            .iter()
            .filter(|(_, b)| component.binary_search(&b.descriptor().owner).is_ok())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in initial {
            enqueue(
                EnqueueContext { data, vocabulary: &vocabulary, calls: &calls, component, component_id, invocation, progress: &progress, seeds: &seeds },
                id,
                &mut queue,
                &processed,
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
                            ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
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
                    out.premises.insert(caller.premise.clone())?;
                    out.premises.insert(callee.premise.clone())?;
                    let call = calls
                        .iter()
                        .find(|c| c.attempt == attempt)
                        .ok_or_else(|| invalid("summary queued call absent"))?;
                    let pair = |disposition, witness, reason| PairOutcome {
                        invocation: invocation.id(),
                        attempt,
                        caller_origin: caller.origin,
                        callee_origin: callee.origin,
                        caller: caller.premise.id(),
                        callee: callee.premise.id(),
                        disposition,
                        witness,
                        reason,
                    };
                    for result in compose(
                        CompositionInputs { data, verified: &verified, vocabulary: &vocabulary, guards: &guards },
                        call,
                        &caller,
                        &callee,
                        budget,
                    )? {
                        match result {
                            CallComposition::Disjoint => {
                                out.pair_outcomes.insert(pair(
                                    PairDisposition::Disjoint,
                                    None,
                                    None,
                                ))?;
                            }
                            CallComposition::Subsumed => {
                                out.pair_outcomes.insert(pair(
                                    PairDisposition::Subsumed,
                                    None,
                                    None,
                                ))?;
                            }
                            CallComposition::Obligation(reason) => {
                                out.pair_outcomes.insert(pair(
                                    PairDisposition::Refused,
                                    None,
                                    Some(reason),
                                ))?;
                                residual(
                                    ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
                                    item,
                                    reason,
                                    &mut out,
                                )?;
                            }
                            CallComposition::Transfer(composed) => {
                                let cost = match ProofCost::through_call(caller.cost, callee.cost) {
                                    Ok(c) => c,
                                    Err(reason) => {
                                        out.pair_outcomes.insert(pair(
                                            PairDisposition::Refused,
                                            None,
                                            Some(reason),
                                        ))?;
                                        residual(
                                            ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
                                            item,
                                            reason,
                                            &mut out,
                                        )?;
                                        continue;
                                    }
                                };
                                let mut evidence =
                                    vec![data.evidence(&out, &caller.premise, budget)?];
                                if caller.premise.id() != callee.premise.id() {
                                    evidence.push(data.evidence(&out, &callee.premise, budget)?);
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
                                if let summary_worklist::Admission::Refused(reason) = admission {
                                    out.pair_outcomes.insert(pair(
                                        PairDisposition::Refused,
                                        None,
                                        Some(reason),
                                    ))?;
                                    residual(
                                        ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
                                        item,
                                        reason,
                                        &mut out,
                                    )?;
                                    continue;
                                }
                                if cost.check(limits).is_err() {
                                    out.pair_outcomes.insert(pair(
                                        PairDisposition::Subsumed,
                                        None,
                                        None,
                                    ))?;
                                    continue;
                                }
                                // Store every admissible proof, including equal-cost evidence; reopen only semantic progress.
                                progress
                                    .charge
                                    .grow(size_of::<ProofId>().saturating_mul(2) + 128)?;
                                progress.by_owner.update(
                                    &mut progress.charge,
                                    branch.descriptor().owner,
                                    |rows| rows.insert(branch.id()),
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
                                out.pair_outcomes.insert(pair(
                                    PairDisposition::Proven,
                                    Some(emission.witness.id()),
                                    None,
                                ))?;
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
                                        super::summary_control::SummaryControlInputs { invocation, definition, witness: &emission.witness },
                                        influence,
                                        q,
                                        &condition,
                                        &mut out,
                                        budget,
                                    )?;
                                }
                                retain_proof(&mut out, invocation, &branch)?;
                                if admission == summary_worklist::Admission::Advanced {
                                    enqueue(
                                        EnqueueContext { data, vocabulary: &vocabulary, calls: &calls, component, component_id, invocation, progress: &progress, seeds: &seeds },
                                        branch.id(),
                                        &mut queue,
                                        &processed,
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
                            budget,
                        )?,
                        _ => unreachable!(),
                    };
                    match result {
                        Err(reason) => residual(
                            ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
                            item,
                            reason,
                            &mut out,
                        )?,
                        Ok(emission) => {
                            let cost = match ProofCost::through_path(source.cost) {
                                Ok(c) => c,
                                Err(reason) => {
                                    residual(
                                        ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
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
                                    ResidualContext { data, invocation, component: component_id, progress: &progress, seeds: &seeds },
                                    item,
                                    reason,
                                    &mut out,
                                )?;
                                continue;
                            }
                            if cost.check(limits).is_err() {
                                continue;
                            }
                            progress
                                .charge
                                .grow(size_of::<ProofId>().saturating_mul(2) + 128)?;
                            progress.by_owner.update(
                                &mut progress.charge,
                                branch.descriptor().owner,
                                |rows| rows.insert(branch.id()),
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
                                enqueue(
                                    EnqueueContext { data, vocabulary: &vocabulary, calls: &calls, component, component_id, invocation, progress: &progress, seeds: &seeds },
                                    branch.id(),
                                    &mut queue,
                                    &processed,
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
    }
    run.components = out.components.len() as i64;
    run.proofs = (out.witnesses.len() + out.path_witnesses.len()) as i64;
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
    super::summary_symbolic::produce(data,invocation,&mut out,budget)?;
    out.consequences(data, invocation, definition, profile, budget)?;
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
