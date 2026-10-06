//! Finite alternatives and complete call-member claims are different semantic questions.
//! Only the shared obligation owner decides priority, discharge and behavioral verdicts.
use super::summary_production::{CallMember, PairDisposition, PairOutcome, SummaryData, SummaryRecords, SummaryResidual};
use crate::domain::{
    analysis::{
        self,
        policy::{EvidenceStatus, behavioral_support},
        summary as owner,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::*,
    attribution::*,
    calls::CallPhase,
    conditions::{ConditionNode, Diagram},
    normalized::{Rows, binding_normalization::VerifiedBindings, bindings::{BindingOutcome, CallBindingAttempt}, events::NormalizedCallEvent},
    obligation::{self, ObligationKind, Standing, Verdict},
    resources::ResourceBudget,
    transfer::{
        TransferBranch, TransferDescriptor, TransferKeyRecord,
        summary::{SummaryPremise, TransferKey},
    },
    value::Place,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
use std::collections::BTreeSet;
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("Summary consequence premise absent"))
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "summary_claims")]
pub enum SummaryClaim {
    #[model(code = 0)]
    FiniteAlternative {
        transfer: Id<TransferKey>,
        qualification: Id<AssertionQualification>,
        channel: analysis::AnalysisChannel,
        phase: CallPhase,
    },
    #[model(code = 1)]
    CallClosure {
        event: Id<NormalizedCallEvent>,
        input: Id<Place>,
        output: Id<Place>,
        qualification: Id<AssertionQualification>,
        channel: analysis::AnalysisChannel,
        phase: CallPhase,
        members: ContentHash,
    },
    #[model(code = 2)]
    SymbolicFieldAssociation {
        alternative: Id<super::summary_symbolic::SymbolicFieldAlternative>,
        qualification: Id<AssertionQualification>,
    },
    #[model(code = 3)]
    NoNormalContinuation {
        owner: Id<normalized::entities::EntityRef>,
        frontier: Id<super::protocol_interpretation::ConditionalTerminalFrontier>,
        restriction: Id<super::protocol_interpretation::NormalContinuationRestriction>,
        qualification: Id<AssertionQualification>,
        question: super::protocol_interpretation::InvocationQuestion,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_claim_members",rule="summary_claim_member",conclusion=claim)]
pub struct ClaimMember {
    #[model(key)]
    pub claim: Id<SummaryClaim>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub member: Id<CallMember>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MemberStanding {
    Proved = 0,
    Open = 1,
    Excluded = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "summary_claim_member_standings",
    rule = "summary_member_standing"
)]
pub struct ClaimStanding {
    #[model(key, premise)]
    pub member: Id<ClaimMember>,
    pub standing: MemberStanding,
    #[model(premise)]
    pub proof: Option<Id<ClaimProof>>,
    pub reason: Option<ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(name="summary_claim_proofs",rule="checked_summary_claim",invariant_refs=refutation_invariants_refs)]
pub enum ClaimProof {
    #[model(code = 0)]
    Finite {
        #[model(premise)]
        claim: Id<SummaryClaim>,
        #[model(premise)]
        source: Id<SummaryPremise>,
        qualification: Id<AssertionQualification>,
        status: EvidenceStatus,
        heuristic: bool,
    },
    #[model(code = 1)]
    Closure {
        #[model(premise)]
        claim: Id<SummaryClaim>,
        qualification: Id<AssertionQualification>,
        status: EvidenceStatus,
        members: ContentHash,
    },
    #[model(code = 2)]
    Refutation {
        #[model(premise)]
        invocation: Id<owner::AnalysisInvocation>,
        #[model(premise)]
        claim: Id<SummaryClaim>,
        #[model(premise)]
        source: Id<SummaryPremise>,
        qualification: Id<AssertionQualification>,
        status: EvidenceStatus,
        heuristic: bool,
        coverage: ContentHash,
    },
}
impl ClaimProof {
    pub fn claim(&self) -> Id<SummaryClaim> {
        match self {
            Self::Finite { claim, .. }
            | Self::Closure { claim, .. }
            | Self::Refutation { claim, .. } => *claim,
        }
    }
}
impl analysis::support::sealed::DerivedEvidence for ClaimProof {}
impl DerivedEvidence for ClaimProof {
    fn source_facts(&self) -> SourceFacts {
        match self {
            Self::Finite {
                qualification,
                status,
                heuristic,
                ..
            }
            | Self::Refutation {
                qualification,
                status,
                heuristic,
                ..
            } => SourceFacts {
                qualification: *qualification,
                status: *status,
                heuristic: *heuristic,
            },
            Self::Closure {
                qualification,
                status,
                ..
            } => SourceFacts {
                qualification: *qualification,
                status: *status,
                heuristic: false,
            },
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_claim_proof_members",rule="summary_claim_proof_member",conclusion=proof)]
pub struct ClaimProofMember {
    #[model(key)]
    pub proof: Id<ClaimProof>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub member: Id<ClaimStanding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_claim_refutation_coverage",rule="summary_refutation_coverage",conclusion=proof)]
pub struct ClaimRefutationCoverage {
    #[model(key)]
    pub proof: Id<ClaimProof>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub coverage: Id<ProviderCoverage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(
    name = "summary_behavioral_conclusions",
    rule = "summary_behavioral_conclusion"
)]
pub struct ClaimConclusion {
    #[model(key, premise)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub subject: Id<owner::ObligationSubject>,
    pub qualification: Option<Id<AssertionQualification>>,
    #[model(premise)]
    pub proof: Option<Id<ClaimProof>>,
    pub coverage: CoverageStatus,
    pub verdict: Verdict,
    pub reason: Option<ObligationKind>,
}
macro_rules! rows{($apply:ident)=>{$apply!{claims:SummaryClaim,members:ClaimMember,standings:ClaimStanding,proofs:ClaimProof,proof_members:ClaimProofMember,refutation_coverage:ClaimRefutationCoverage,conclusions:ClaimConclusion,keys:TransferKey,premises:SummaryPremise,subjects:owner::ObligationSubject,sources:owner::SupportSource,derivations:owner::AnalysisDerivation,propositions:owner::AnalysisProposition,derivation_premises:owner::AnalysisDerivationPremise,obligations:owner::AnalysisObligation,}};}
macro_rules! records{($($field:ident:$ty:ty,)*)=>{pub struct ConsequenceRecords{$(pub $field:Rows<$ty>,)*pub pending:Vec<PendingDischarge>,_pending_charge:charged::StateCharge}impl ConsequenceRecords{pub fn into_pending_parts(self)->(Vec<PendingDischarge>,charged::StateCharge){(self.pending,self._pending_charge)}fn new(b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*pending:Vec::new(),_pending_charge:charged::StateCharge::new(b,"summary-pending-discharge")}}}};}
rows!(records);
/// This token can be minted only from a checked complete claim, never from a desired verdict.
struct QualifiedClaim {
    qualification: AssertionQualification,
    condition: Diagram,
    _charge: Box<dyn resources::Reservation>,
}
pub struct PendingDischarge {
    obligation: owner::AnalysisObligation,
    derivation: owner::AnalysisDerivation,
    proposition: owner::AnalysisProposition,
    qualified: std::sync::Arc<QualifiedClaim>,
}
impl PendingDischarge {
    pub fn matches(&self, coverage: &owner::AnalysisCoverage) -> bool {
        coverage.invocation == self.obligation.invocation
            && (coverage.scope, coverage.context)
                == (
                    self.qualified.qualification.scope,
                    self.qualified.qualification.context,
                )
    }
    pub fn admit(
        &self,
        coverage: &owner::AnalysisCoverage,
    ) -> Result<(owner::ObligationSource, owner::DischargeEvidence), ModelError> {
        if !self.matches(coverage) || self.derivation.invocation != coverage.invocation {
            return Err(invalid("Summary discharge changes exact invocation"));
        }
        behavioral_support(self.derivation.status, self.derivation.heuristic)?;
        owner::obligations::admissible_discharge(
            &self.obligation,
            &self.proposition,
            &self.qualified.qualification,
            &self.qualified.condition,
            coverage,
        )?;
        let source = owner::ObligationSource::Current {
            obligation: self.obligation.id(),
        };
        let evidence = owner::DischargeEvidence {
            obligation: source.id(),
            derivation: self.derivation.id(),
            coverage: coverage.id(),
        };
        Ok((source, evidence))
    }
}
#[derive(Clone)]
struct Finite {
    branch: TransferBranch<TransferKey>,
    source: SummaryPremise,
    facts: SourceFacts,
}
impl HeapSize for Finite {}
/// Immutable Model/Summary nodes are merged once; every condition uses the shared kernel's
/// exact reachable closure and canonical atom ordering. Distinct qualifications may reuse a BDD.
struct ConditionCatalog {
    nodes: charged::ChargedMap<Id<ConditionNode>, ConditionNode>,
    diagrams: charged::ChargedMap<Id<conditions::Condition>, CachedCondition>,
    validated: bool,
    charge: charged::StateCharge,
}
struct CachedCondition(Diagram);
impl HeapSize for CachedCondition {
    fn heap_bytes(&self) -> usize { self.0.allocation_allowance() }
}
impl ConditionCatalog {
    fn new(data: &SummaryData, out: &SummaryRecords, b: &ResourceBudget) -> Result<Self, ModelError> {
        let mut catalog=Self {nodes:Default::default(),diagrams:Default::default(),validated:false,charge:charged::StateCharge::new(b,"summary-consequence-condition-catalog")};
        // Preserve the existing Summary-over-Model precedence for duplicate node identities.
        for node in data.vocabulary.nodes.values().chain(out.vocabulary.nodes.values()) {
            catalog.nodes.insert(&mut catalog.charge,node.id(),node.clone())?;
        }
        Ok(catalog)
    }
    fn diagram(&mut self, data:&SummaryData, out:&SummaryRecords, q:&AssertionQualification, b:&ResourceBudget) -> Result<&Diagram,ModelError> {
        if !self.diagrams.contains_key(&q.condition) {
            if !self.validated {
                // from_records previously checked the entire merged catalog on every request.
                if self.nodes.len()>100_000 {return Err(invalid("condition node admission exceeded"));}
                for node in self.nodes.values() {node.validate()?;}
                self.validated=true;
            }
            let record=out.vocabulary.conditions.get(&q.condition).or_else(||data.vocabulary.conditions.get(&q.condition)).ok_or_else(||invalid("Summary consequence condition absent"))?;
            let mut scratch=b.reserve("summary-consequence-condition",self.nodes.len().saturating_mul(128).saturating_add(4096))?;
            let selected=conditions::kernel::closure(record.root,&self.nodes)?;
            scratch.try_resize(self.nodes.len().saturating_mul(128).saturating_add(selected.len().saturating_mul(4096)).saturating_add(4096))?;
            let nodes=selected.iter().map(|id|self.nodes[id].clone()).collect::<Vec<_>>();
            let diagram=Diagram::from_records(record,&nodes)?;
            self.diagrams.insert(&mut self.charge,q.condition,CachedCondition(diagram))?;
        }
        Ok(&self.diagrams.get(&q.condition).expect("condition inserted above").0)
    }
}
fn qualification<'a>(
    data: &'a SummaryData,
    out: &'a SummaryRecords,
    id: Id<AssertionQualification>,
) -> Result<&'a AssertionQualification, ModelError> {
    out.vocabulary
        .qualifications
        .get(&id)
        .or_else(|| data.vocabulary.qualifications.get(&id))
        .ok_or_else(|| invalid("Summary consequence qualification absent"))
}
fn coverage(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    q: &AssertionQualification,
) -> CoverageStatus {
    let mut selected = None;
    for row in data
        .entry
        .coverage
        .iter()
        .filter(|r| r.context == q.context && r.family == FactFamily::Flow)
    {
        let Some(run) = row.run.and_then(|id| data.entry.runs.get(id)) else {
            continue;
        };
        if run.input != invocation.input || run.context != invocation.context {
            continue;
        }
        if row.scope != q.scope {
            continue;
        }
        selected = Some(match selected {
            None => row.status,
            Some(CoverageStatus::CompleteUnderStatedModel) => row.status,
            Some(other) => other,
        });
    }
    selected.unwrap_or(CoverageStatus::Partial)
}
fn facts(
    data: &SummaryData,
    out: &SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    conditions: &mut ConditionCatalog,
    b: &ResourceBudget,
) -> Result<
    (
        charged::ChargedMap<Id<SummaryPremise>, Finite>,
        charged::StateCharge,
    ),
    ModelError,
> {
    let mut rows = charged::ChargedMap::default();
    let mut charge = charged::StateCharge::new(b, "summary-consequence-finite-inventory");
    // Preserve each assertion's canonical support order while sharing the immutable lookup.
    let mut local_supports = charged::ChargedMap::<
        Id<transfer::local::TransferAlternative>,
        Vec<&transfer::local::TransferSupport>,
    >::default();
    let mut support_charge = charged::StateCharge::new(b, "summary-consequence-local-support-index");
    for support in data.local_supports.iter() {
        local_supports.update(&mut support_charge, support.assertion, |members| members.push(support))?;
    }
    let mut add = |descriptor: TransferDescriptor,
                   qid,
                   source: SummaryPremise,
                   status,
                   heuristic|
     -> Result<(), ModelError> {
        let q = qualification(data, out, qid)?;
        if q.context != invocation.context {
            return Err(invalid("Summary finite consequence changes context"));
        }
        let condition = conditions.diagram(data, out, q, b)?;
        let _copy=b.reserve("summary-consequence-condition-copy",condition.allocation_allowance())?;
        let branch = TransferBranch::new(
            TransferKey::from_descriptor(descriptor),
            q.clone(),
            condition.clone(),
            b,
        )?;
        rows.insert(
            &mut charge,
            source.id(),
            Finite {
                branch,
                source,
                facts: SourceFacts {
                    qualification: qid,
                    status,
                    heuristic,
                },
            },
        )?;
        Ok(())
    };
    for row in data.local_contributions.iter() {
        let parent = need(&data.local_invocations, row.invocation)?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        let key = need(&data.local_keys, row.transfer)?;
        let q = qualification(data, out, row.qualification)?;
        let alternative = key.alternative(q);
        if data.local_alternatives.get(alternative.id()) != Some(&alternative) {
            return Err(invalid("Summary Local finite alternative absent"));
        }
        for selected in crate::domain::atom_decision::selected_local_alternatives(
            &alternative,
            &data.local_alternatives,
            &data.atom_restrictions,
        ) {
            let matching = local_supports.get(&selected.id()).map(Vec::as_slice).unwrap_or(&[]);
            let _supports = b.reserve(
                "summary-consequence-local-supports",
                matching.len().saturating_mul(256),
            )?;
            let supports = matching.iter().map(|support| (**support).clone()).collect::<Vec<_>>();
            let evidence = transfer::summary::TransferEvidence::local(
                selected,
                &supports,
                &data.local_evidence,
                b,
            )?;
            let status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                evidence.facts.iter().map(|f| f.status),
            );
            add(
                key.descriptor(),
                selected.qualification,
                SummaryPremise::Local {
                    alternative: selected.id(),
                },
                status,
                evidence.facts.iter().any(|f| f.heuristic),
            )?;
        }
    }
    for row in data.model_supports.iter() {
        let (source, facts) = data.model_evidence.get(row.source)?;
        let analysis::model::SupportSource::AnalysisDerivation { derivation } = source else {
            return Err(invalid("Model finite origin lacks owning derivation"));
        };
        let producer = need(&data.model_derivations, *derivation)?;
        let parent = need(&data.model_invocations, producer.invocation)?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        let alternative = need(&data.model_alternatives, row.assertion)?;
        let key = need(&data.model_keys, alternative.transfer)?;
        if facts.qualification != alternative.qualification {
            return Err(invalid("Model finite support changes qualification"));
        }
        add(
            key.descriptor(),
            alternative.qualification,
            SummaryPremise::Model {
                alternative: alternative.id(),
            },
            facts.status,
            facts.heuristic,
        )?;
    }
    for row in out.witnesses.iter() {
        if row.invocation != invocation.id() {
            return Err(invalid("Summary consequence foreign witness"));
        }
        add(
            need(&out.keys, row.transfer)?.descriptor(),
            row.qualification,
            SummaryPremise::Witness { witness: row.id() },
            row.status,
            row.heuristic,
        )?;
    }
    for row in out.capture_witnesses.iter() {
        if row.invocation != invocation.id() {
            return Err(invalid("capture consequence foreign witness"));
        }
        add(
            need(&out.keys, row.transfer)?.descriptor(),
            row.qualification,
            SummaryPremise::Captured { witness: row.id() },
            row.status,
            false,
        )?;
    }
    for row in out.path_witnesses.iter() {
        if row.invocation != invocation.id() {
            return Err(invalid("Summary consequence foreign path"));
        }
        add(
            need(&out.keys, row.transfer)?.descriptor(),
            row.qualification,
            SummaryPremise::Path { witness: row.id() },
            row.status,
            row.heuristic,
        )?;
    }
    Ok((rows, charge))
}
fn conclusion(
    condition: &Diagram,
    q: &AssertionQualification,
    c: CoverageStatus,
    open: &[ObligationKind],
    facts: SourceFacts,
) -> obligation::Conclusion {
    let refusal = behavioral_support(facts.status, facts.heuristic)
        .err()
        .map(|_| ObligationKind::MissingEvidence);
    let mut reasons = open.to_vec();
    if let Some(r) = refusal {
        reasons.push(r);
    }
    obligation::verdict(obligation::VerdictInput {
        condition: Some(condition),
        open: &reasons,
        coverage: c,
        approximation: q.approximation,
        modality: q.modality,
    })
}
type RefutationCoverage = (Vec<Id<ProviderCoverage>>, Box<dyn resources::Reservation>);

fn complete_refutation(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    q: &AssertionQualification,
    condition: &Diagram,
    facts: SourceFacts,
    b: &ResourceBudget,
) -> Result<RefutationCoverage, ModelError> {
    if !condition.is_false()
        || condition.id() != q.condition
        || facts.qualification != q.id()
        || q.approximation != Approximation::Exact
        || q.modality != Modality::Definite
    {
        return Err(invalid(
            "refutation needs an exact checked false finite condition",
        ));
    }
    behavioral_support(facts.status, facts.heuristic)?;
    let held = b.reserve(
        "summary-refutation-coverage",
        data.entry
            .coverage
            .len()
            .saturating_mul(size_of::<Id<ProviderCoverage>>() + 128),
    )?;
    let members = data
        .entry
        .coverage
        .iter()
        .filter(|r| {
            r.context == q.context
                && r.scope == q.scope
                && r.family == FactFamily::Flow
                && r.run
                    .and_then(|id| data.entry.runs.get(id))
                    .is_some_and(|run| {
                        run.input == invocation.input && run.context == invocation.context
                    })
        })
        .collect::<Vec<_>>();
    if members.is_empty()
        || members
            .iter()
            .any(|r| r.status != CoverageStatus::CompleteUnderStatedModel)
    {
        return Err(invalid(
            "finite refutation requires exact complete native Flow membership",
        ));
    }
    for member in &members {
        member.validate()?;
        let run = need(
            &data.entry.runs,
            member
                .run
                .ok_or_else(|| invalid("refutation coverage run absent"))?,
        )?;
        if member.provider != Some(run.provider) {
            return Err(invalid("refutation coverage changes provider run"));
        }
    }
    Ok((members.iter().map(|r| r.id()).collect(), held))
}
fn emit_proof(
    proof: ClaimProof,
    q: &AssertionQualification,
    condition: &Diagram,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    rows: &mut ConsequenceRecords,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let claim = match &proof {
        ClaimProof::Finite { claim, .. }
        | ClaimProof::Closure { claim, .. }
        | ClaimProof::Refutation { claim, .. } => *claim,
    };
    let subject = owner::ObligationSubject::SummaryClaim { transfer: claim };
    let source = owner::SupportSource::ClaimProof {
        witness: proof.id(),
    };
    let evidence = owner::support::EvidencePremise::derived(&source, &proof, q, condition)?;
    let (derivation, proposition, premises, _) = owner::AnalysisDerivation::emit(
        invocation,
        definition,
        subject.id(),
        analysis::AnalysisChannel::Value,
        CallPhase::Call,
        analysis::support::QualificationOperation::Conjunction,
        &[evidence],
        b,
    )?;
    let obligation = owner::AnalysisObligation {
        invocation: invocation.id(),
        subject: subject.id(),
        channel: analysis::AnalysisChannel::Value,
        phase: CallPhase::Call,
        qualification: q.id(),
        reason: ObligationKind::CallTransfer,
        responsible: analysis::AnalysisMethod::Summaries,
    };
    if !matches!(&proof, ClaimProof::Refutation { .. }) {
        let held = b.reserve(
            "summary-discharge-condition",
            condition.allocation_allowance() + size_of::<QualifiedClaim>(),
        )?;
        let qualified = std::sync::Arc::new(QualifiedClaim {
            qualification: q.clone(),
            condition: condition.clone(),
            _charge: held,
        });
        rows.obligations.insert(obligation.clone())?;
        rows._pending_charge
            .grow(size_of::<PendingDischarge>() + 128)?;
        rows.pending.push(PendingDischarge {
            obligation,
            derivation: derivation.clone(),
            proposition: proposition.clone(),
            qualified,
        });
    }
    rows.subjects.insert(subject)?;
    rows.sources.insert(source)?;
    rows.derivations.insert(derivation)?;
    rows.propositions.insert(proposition)?;
    for row in premises {
        rows.derivation_premises.insert(row)?;
    }
    rows.proofs.insert(proof)?;
    Ok(())
}
/// Replays the exact finite inventory and every actual call attempt. Residuals affect only the
/// separate set question; a capped later attempt cannot erase an already established finite proof.
pub fn derive(
    data: &SummaryData,
    out: &SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    b: &ResourceBudget,
) -> Result<ConsequenceRecords, ModelError> {
    let verified = if profile == stages::Profile::Behavioral {
        Some(normalized::binding_normalization::verify(&data.bindings, &data.binding_output, b)?)
    } else {
        None
    };
    derive_with_verified_bindings(data, out, invocation, definition, profile, verified.as_ref(), b)
}
/// Production borrows its verification of the same immutable inputs; standalone derivation and
/// independent replay still obtain fresh verification through their respective public entries.
pub(super) fn derive_with_verified_bindings(
    data: &SummaryData,
    out: &SummaryRecords,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    verified_bindings: Option<&VerifiedBindings>,
    b: &ResourceBudget,
) -> Result<ConsequenceRecords, ModelError> {
    if invocation.definition != definition.id() || invocation.subject.is_some() {
        return Err(invalid(
            "Summary consequence changes exact whole-frame definition",
        ));
    }
    let mut runs = out.runs.iter().filter(|r| r.invocation == invocation.id());
    let run = runs
        .next()
        .ok_or_else(|| invalid("Summary consequence run absent"))?;
    if runs.next().is_some() || run.requested != (profile == stages::Profile::Behavioral) {
        return Err(invalid(
            "Summary consequence changes actual requested profile",
        ));
    }
    let mut rows = ConsequenceRecords::new(b);
    if profile == stages::Profile::Catalog {
        let subject = owner::ObligationSubject::Computation {
            invocation: invocation.id(),
        };
        let result = obligation::verdict(obligation::VerdictInput {
            condition: None,
            open: &[],
            coverage: CoverageStatus::NotRequested,
            approximation: Approximation::Exact,
            modality: Modality::Definite,
        });
        rows.conclusions.insert(ClaimConclusion {
            invocation: invocation.id(),
            subject: subject.id(),
            qualification: None,
            proof: None,
            coverage: CoverageStatus::NotRequested,
            verdict: result.verdict,
            reason: result.reason,
        })?;
        rows.subjects.insert(subject)?;
        return Ok(rows);
    }
    // Both entry paths provide verification of these same immutable inputs.
    // Replay invokes production afresh, obtaining its own verification rather than trusting output.
    let _verified = verified_bindings
        .ok_or_else(|| invalid("Summary behavioral consequences require verified bindings"))?;
    let mut conditions=ConditionCatalog::new(data,out,b)?;
    let (finite, _inventory) = facts(data, out, invocation, &mut conditions, b)?;
    let mut conclusions = charged::ChargedMap::<Id<SummaryClaim>, ClaimConclusion>::default();
    let mut proven = charged::ChargedMap::<Id<SummaryPremise>, Id<ClaimProof>>::default();
    let mut charge = charged::StateCharge::new(b, "summary-consequence-proof-index");
    for fact in finite.values() {
        let q = fact.branch.qualification();
        let claim = SummaryClaim::FiniteAlternative {
            transfer: fact.branch.key().id(),
            qualification: q.id(),
            channel: analysis::AnalysisChannel::Value,
            phase: CallPhase::Call,
        };
        let subject = owner::ObligationSubject::SummaryClaim {
            transfer: claim.id(),
        };
        let c = coverage(data, invocation, q);
        let result = conclusion(fact.branch.condition(), q, c, &[], fact.facts);
        let negative = result.verdict == Verdict::RefutedUnderModel;
        let (refutation_members, _refutation_charge) = if negative {
            let (m, c) =
                complete_refutation(data, invocation, q, fact.branch.condition(), fact.facts, b)?;
            (m, Some(c))
        } else {
            (Vec::new(), None)
        };
        let proof = if negative {
            ClaimProof::Refutation {
                invocation: invocation.id(),
                claim: claim.id(),
                source: fact.source.id(),
                qualification: q.id(),
                status: fact.facts.status,
                heuristic: fact.facts.heuristic,
                coverage: super::records::ordered_digest(
                    "summary-refutation-coverage",
                    refutation_members.iter().copied(),
                ),
            }
        } else {
            ClaimProof::Finite {
                claim: claim.id(),
                source: fact.source.id(),
                qualification: q.id(),
                status: fact.facts.status,
                heuristic: fact.facts.heuristic,
            }
        };
        let admissible = matches!(
            result.verdict,
            Verdict::Established | Verdict::Conditional | Verdict::RefutedUnderModel
        );
        rows.keys.insert(fact.branch.key().clone())?;
        rows.premises.insert(fact.source.clone())?;
        let claim_id = claim.id();
        rows.claims.insert(claim)?;
        rows.subjects.insert(subject.clone())?;
        let conclusion = ClaimConclusion {
            invocation: invocation.id(),
            subject: subject.id(),
            qualification: Some(q.id()),
            proof: admissible.then_some(proof.id()),
            coverage: c,
            verdict: result.verdict,
            reason: result.reason,
        };
        let replace =
            conclusions
                .get(&claim_id)
                .is_none_or(|old| match (old.proof, conclusion.proof) {
                    (None, Some(_)) => true,
                    (Some(a), Some(b)) => b < a,
                    (Some(_), None) => false,
                    (None, None) => {
                        obligation::first([old.reason, conclusion.reason].into_iter().flatten())
                            == conclusion.reason
                    }
                });
        if replace {
            conclusions.insert(&mut charge, claim_id, conclusion)?;
        }
        if admissible {
            if !negative {
                proven.insert(&mut charge, fact.source.id(), proof.id())?;
            }
            for (ordinal, coverage) in refutation_members.iter().enumerate() {
                rows.refutation_coverage.insert(ClaimRefutationCoverage {
                    proof: proof.id(),
                    ordinal: ordinal as i64,
                    coverage: *coverage,
                })?;
            }
            emit_proof(
                proof,
                fact.branch.qualification(),
                fact.branch.condition(),
                invocation,
                definition,
                &mut rows,
                b,
            )?;
        }
    }
    for row in conclusions.values() {
        rows.conclusions.insert(row.clone())?;
    }
    // A query comes from an actual attempted pair or residual, never from an empty flow table.
    let mut queries = charged::ChargedSet::default();
    // Qualification remains a proof comparison below; grouping must not discard other qids.
    let mut pairs=charged::ChargedMap::<(Id<CallBindingAttempt>,Id<Place>,Id<Place>),Vec<&PairOutcome>>::default();
    for row in out.pair_outcomes.iter() {
        let attempt = need(&data.binding_output.attempts, row.attempt)?;
        if let (Some(caller), Some(callee)) = (finite.get(&row.caller), finite.get(&row.callee)) {
            pairs.update(&mut charge,(row.attempt,caller.branch.descriptor().input,callee.branch.descriptor().output),|members:&mut Vec<_>|members.push(row))?;
            queries.insert(
                &mut charge,
                (
                    attempt.event,
                    caller.branch.descriptor().input,
                    callee.branch.descriptor().output,
                    caller.branch.qualification().id(),
                ),
            )?;
        }
    }
    // The original residual question is event/input/output, independent of qualification.
    let mut residuals=charged::ChargedMap::<(Id<NormalizedCallEvent>,Id<Place>,Id<Place>),Vec<&SummaryResidual>>::default();
    for row in out.residuals.iter() {
        residuals.update(&mut charge,(row.event,row.input,row.output),|members:&mut Vec<_>|members.push(row))?;
        queries.insert(
            &mut charge,
            (row.event, row.input, row.output, row.qualification),
        )?;
    }
    let mut members_by_event=charged::ChargedMap::<Id<NormalizedCallEvent>,Vec<&CallMember>>::default();
    for member in out.call_members.iter() {
        if let Some(attempt)=data.binding_output.attempts.get(member.attempt) {
            members_by_event.update(&mut charge,attempt.event,|members:&mut Vec<_>|members.push(member))?;
        }
    }
    let mut actual_by_event=charged::ChargedMap::<Id<NormalizedCallEvent>,usize>::default();
    for attempt in data.binding_output.attempts.iter() {
        actual_by_event.update(&mut charge,attempt.event,|count:&mut usize|*count+=1)?;
    }
    let mut assessed=charged::ChargedSet::<Id<NormalizedCallEvent>>::default();
    for assessment in data.bindings.event_assessments.iter().filter(|a|a.complete) {assessed.insert(&mut charge,assessment.event)?;}
    let mut complete_sets=charged::ChargedMap::<Id<NormalizedCallEvent>,bool>::default();
    for set in data.binding_output.sets.iter() {
        let complete=complete_sets.get(&set.event).copied().unwrap_or(true) && set.coverage_complete;
        complete_sets.insert(&mut charge,set.event,complete)?;
    }
    for &(event, input, output, qid) in queries.iter() {
        let q = qualification(data, out, qid)?;
        // Group insertion follows Rows' canonical order, so claim digests and ordinals are stable.
        let members=members_by_event.get(&event).map(Vec::as_slice).unwrap_or(&[]);
        let actual=actual_by_event.get(&event).copied().unwrap_or(0);
        let _member_buffer=b.reserve("summary-consequence-member-universe",members.len().saturating_mul(1024))?;
        if members.len() != actual || members.iter().any(|m| m.invocation != invocation.id()) {
            return Err(invalid("Summary claim omits actual call member"));
        }
        let digest = super::records::ordered_digest(
            "summary-claim-call-members",
            members.iter().map(|m| m.id()),
        );
        let claim = SummaryClaim::CallClosure {
            event,
            input,
            output,
            qualification: qid,
            channel: analysis::AnalysisChannel::Value,
            phase: CallPhase::Call,
            members: digest,
        };
        let subject = owner::ObligationSubject::SummaryClaim {
            transfer: claim.id(),
        };
        let mut decisions = obligation::Decisions::default();
        let mut applicable = BTreeSet::new();
        let mut statuses = Vec::new();
        let mut open = Vec::new();
        let mut standing_rows = Vec::new();
        for (ordinal, member) in members.iter().enumerate() {
            let m = ClaimMember {
                claim: claim.id(),
                ordinal: ordinal as i64,
                member: member.id(),
            };
            let attempt = need(&data.binding_output.attempts, member.attempt)?;
            let excluded = attempt.outcome == BindingOutcome::ProvenIncompatible
                && attempt.authority
                    == normalized::signature_applicability::BindingAuthority::EffectiveInvocation;
            if !excluded {
                applicable.insert(member.attempt);
                if let Some(reason) = member.reason {
                    decisions.open(member.attempt, reason);
                }
                for pair in pairs.get(&(member.attempt,input,output)).into_iter().flatten() {
                    if pair.disposition == PairDisposition::Refused {
                        decisions.open(
                            member.attempt,
                            pair.reason.unwrap_or(ObligationKind::MissingEvidence),
                        );
                    }
                    if pair.disposition == PairDisposition::Proven
                        && let Some(witness) = pair.witness
                    {
                        let source = SummaryPremise::Witness { witness };
                        if let (Some(fact), Some(proof)) =
                            (finite.get(&source.id()), proven.get(&source.id()))
                            && fact.branch.qualification() == q
                        {
                            decisions.proof(member.attempt, *proof, Verdict::Established);
                            statuses.push(fact.facts.status);
                        }
                    }
                }
                for residual in residuals.get(&(event,input,output)).into_iter().flatten() {
                    decisions.open(member.attempt, residual.reason);
                }
            }
            let (standing, proof, reason) = if excluded {
                (MemberStanding::Excluded, None, None)
            } else {
                match decisions.decide(member.attempt) {
                    Standing::Proved { proof } => (MemberStanding::Proved, Some(proof), None),
                    Standing::Open { obligation } => (MemberStanding::Open, None, Some(obligation)),
                }
            };
            if let Some(r) = reason {
                open.push(r);
            }
            let standing = ClaimStanding {
                member: m.id(),
                standing,
                proof,
                reason,
            };
            rows.members.insert(m)?;
            standing_rows.push(standing.id());
            rows.standings.insert(standing)?;
        }
        for boundary in out.origin_boundaries.iter() {
            need(&out.origins, boundary.origin)?;
            let descriptor = need(&out.keys, boundary.transfer)?.descriptor();
            if descriptor.context == q.context
                && descriptor.scope == q.scope
                && (descriptor.input == input || descriptor.output == output)
            {
                open.push(boundary.reason);
            }
        }
        let complete=assessed.contains(&event) && complete_sets.get(&event).copied().unwrap_or(false);
        let (closed, _) = obligation::discharge(&applicable, &decisions);
        if !complete {
            open.push(ObligationKind::IncompleteCoverage);
        }
        if !closed && open.is_empty() {
            open.push(ObligationKind::MissingEvidence);
        }
        let condition = conditions.diagram(data, out, q, b)?;
        let c = coverage(data, invocation, q);
        let status =
            analysis::support::inferred_status(analysis::Interpretation::Structural, statuses);
        let result = conclusion(
            &condition,
            q,
            c,
            &open,
            SourceFacts {
                qualification: qid,
                status,
                heuristic: false,
            },
        );
        let proof = ClaimProof::Closure {
            claim: claim.id(),
            qualification: qid,
            status,
            members: super::records::ordered_digest(
                "summary-claim-standings",
                standing_rows.iter().copied(),
            ),
        };
        let admissible = closed
            && complete
            && matches!(result.verdict, Verdict::Established | Verdict::Conditional);
        rows.claims.insert(claim)?;
        rows.subjects.insert(subject.clone())?;
        rows.conclusions.insert(ClaimConclusion {
            invocation: invocation.id(),
            subject: subject.id(),
            qualification: Some(qid),
            proof: admissible.then_some(proof.id()),
            coverage: c,
            verdict: result.verdict,
            reason: result.reason,
        })?;
        if admissible {
            for (ordinal, member) in standing_rows.iter().enumerate() {
                rows.proof_members.insert(ClaimProofMember {
                    proof: proof.id(),
                    ordinal: ordinal as i64,
                    member: *member,
                })?;
            }
            emit_proof(proof, q, &condition, invocation, definition, &mut rows, b)?;
        } else {
            let reason = result.reason.unwrap_or(ObligationKind::MissingEvidence);
            rows.obligations.insert(owner::AnalysisObligation {
                invocation: invocation.id(),
                subject: subject.id(),
                channel: analysis::AnalysisChannel::Value,
                phase: CallPhase::Call,
                qualification: qid,
                reason,
                responsible: analysis::AnalysisMethod::Summaries,
            })?;
        }
    }
    for alternative in out.symbolic_alternatives.iter() {
        let claim = SummaryClaim::SymbolicFieldAssociation {
            alternative: alternative.id(),
            qualification: alternative.reader_qualification,
        };
        let subject = owner::ObligationSubject::SummaryClaim {
            transfer: claim.id(),
        };
        let result = obligation::verdict(obligation::VerdictInput {
            condition: None,
            open: &[alternative.reason],
            coverage: CoverageStatus::Partial,
            approximation: Approximation::Exact,
            modality: Modality::Definite,
        });
        rows.conclusions.insert(ClaimConclusion {
            invocation: invocation.id(),
            subject: subject.id(),
            qualification: Some(alternative.reader_qualification),
            proof: None,
            coverage: CoverageStatus::Partial,
            verdict: result.verdict,
            reason: result.reason,
        })?;
        rows.obligations.insert(owner::AnalysisObligation {
            invocation: invocation.id(),
            subject: subject.id(),
            channel: analysis::AnalysisChannel::Value,
            phase: CallPhase::Init,
            qualification: alternative.reader_qualification,
            reason: alternative.reason,
            responsible: analysis::AnalysisMethod::Summaries,
        })?;
        rows.claims.insert(claim)?;
        rows.subjects.insert(subject)?;
    }
    Ok(rows)
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<SummaryClaim>(),
        Relation::of::<ClaimMember>(),
        Relation::of::<ClaimStanding>(),
        Relation::of::<ClaimProof>(),
        Relation::of::<ClaimProofMember>(),
        Relation::of::<ClaimRefutationCoverage>(),
        Relation::of::<ClaimConclusion>(),
    ]
}

pub(crate) fn refutation_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "checked_finite_refutation_membership",
        inputs: vec![
            ValidationInput::of::<ClaimProof>(&["id"]),
            ValidationInput::of::<ClaimRefutationCoverage>(&["ordinal", "id"]),
            ValidationInput::of::<SummaryClaim>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<conditions::Condition>(&["id"]),
            ValidationInput::of::<ConditionNode>(&["id"]),
            ValidationInput::of::<ProviderCoverage>(&["id"]),
            ValidationInput::of::<ProviderRun>(&["id"]),
            ValidationInput::of::<owner::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(RefutationCheck {
                proofs: Rows::new(b),
                members: Rows::new(b),
                claims: Rows::new(b),
                qualifications: Rows::new(b),
                conditions: Rows::new(b),
                nodes: Rows::new(b),
                invocations: Rows::new(b),
                data: SummaryData::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct RefutationCheck {
    proofs: Rows<ClaimProof>,
    members: Rows<ClaimRefutationCoverage>,
    claims: Rows<SummaryClaim>,
    qualifications: Rows<AssertionQualification>,
    conditions: Rows<conditions::Condition>,
    nodes: Rows<ConditionNode>,
    invocations: Rows<owner::AnalysisInvocation>,
    data: SummaryData,
    budget: ResourceBudget,
}
impl InvariantCheck for RefutationCheck {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! row {
            ($t:ty,$f:ident) => {
                if n == <$t>::NAME {
                    self.$f.decode(b)?;
                    return Ok(());
                }
            };
        }
        row!(ClaimProof, proofs);
        row!(ClaimRefutationCoverage, members);
        row!(SummaryClaim, claims);
        row!(AssertionQualification, qualifications);
        row!(conditions::Condition, conditions);
        row!(ConditionNode, nodes);
        row!(owner::AnalysisInvocation, invocations);
        if n == ProviderCoverage::NAME {
            self.data.entry.coverage.decode(b)?;
            return Ok(());
        }
        if n == ProviderRun::NAME {
            self.data.entry.runs.decode(b)?;
            return Ok(());
        }
        Err(invalid("undeclared finite refutation input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let _scratch = self.budget.reserve(
            "summary-refutation-replay",
            (self.nodes.len() + self.members.len())
                .saturating_mul(4096)
                .saturating_add(4096),
        )?;
        for row in self.members.iter() {
            if !matches!(
                self.proofs.get(row.proof),
                Some(ClaimProof::Refutation { .. })
            ) {
                return Err(invalid("coverage member lacks finite refutation"));
            }
        }
        for proof in self.proofs.iter() {
            let ClaimProof::Refutation {
                invocation,
                claim,
                qualification,
                status,
                heuristic,
                coverage,
                ..
            } = proof
            else {
                continue;
            };
            let q = need(&self.qualifications, *qualification)?;
            if !matches!(need(&self.claims,*claim)?,SummaryClaim::FiniteAlternative{qualification,..}if *qualification==q.id())
            {
                return Err(invalid("refutation names another finite question"));
            }
            let condition = Diagram::from_records(
                need(&self.conditions, q.condition)?,
                &self.nodes.iter().cloned().collect::<Vec<_>>(),
            )?;
            let invocation = need(&self.invocations, *invocation)?;
            let (expected, _charge) = complete_refutation(
                &self.data,
                invocation,
                q,
                &condition,
                SourceFacts {
                    qualification: q.id(),
                    status: *status,
                    heuristic: *heuristic,
                },
                &self.budget,
            )?;
            let mut members = self
                .members
                .iter()
                .filter(|m| m.proof == proof.id())
                .collect::<Vec<_>>();
            members.sort_by_key(|m| m.ordinal);
            if members.len() != expected.len()
                || members
                    .iter()
                    .enumerate()
                    .any(|(i, m)| m.ordinal != i as i64 || m.coverage != expected[i])
                || *coverage
                    != super::records::ordered_digest("summary-refutation-coverage", expected)
            {
                return Err(invalid(
                    "refutation changes complete native coverage membership",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn refutation_invariants_refs() -> Vec<&'static str> {
    vec!["checked_finite_refutation_membership"]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn nominal<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn prepared_conditions_reuse_roots_without_widening_atoms_or_qualifications() {
        let b=ResourceBudget::fixed(8<<20).unwrap();
        let mut data=SummaryData::new(&b);
        let out=SummaryRecords::new(nominal(1),&b);
        let mut charge=charged::StateCharge::new(&b,"summary-condition-control");
        let expected=Diagram::from_atom(nominal(2));
        let unrelated=Diagram::from_atom(nominal(3));
        for diagram in [&expected,&unrelated] {
            let (condition,nodes)=diagram.records();
            data.vocabulary.conditions.insert(&mut charge,condition.id(),condition).unwrap();
            for node in nodes {data.vocabulary.nodes.insert(&mut charge,node.id(),node).unwrap();}
        }
        let q=AssertionQualification {assumptions:assumptions::AssumptionSet::empty_id(),context:nominal(4),scope:nominal(5),condition:expected.id(),modality:Modality::Definite,approximation:Approximation::Exact};
        let mut other=q.clone();other.scope=nominal(6);
        let mut prepared=ConditionCatalog::new(&data,&out,&b).unwrap();
        assert_eq!(prepared.diagram(&data,&out,&q,&b).unwrap().records(),expected.records());
        assert_eq!(prepared.diagram(&data,&out,&other,&b).unwrap().records(),expected.records());
        assert_eq!(prepared.diagrams.len(),1);
        assert_ne!(q.id(),other.id());
        // Every merged node still receives the prior shape validation, even when unreachable.
        let invalid=ConditionNode::Branch {atom:nominal(7),low:ConditionNode::False.id(),high:ConditionNode::False.id()};
        data.vocabulary.nodes.insert(&mut charge,invalid.id(),invalid).unwrap();
        assert!(ConditionCatalog::new(&data,&out,&b).unwrap().diagram(&data,&out,&q,&b).is_err());
    }
    #[test]
    fn prepared_conditions_retain_missing_child_refusal() {
        let b=ResourceBudget::fixed(8<<20).unwrap();
        let mut data=SummaryData::new(&b);
        let out=SummaryRecords::new(nominal(1),&b);
        let mut charge=charged::StateCharge::new(&b,"summary-condition-control");
        let node=ConditionNode::Branch {atom:nominal(2),low:ConditionNode::False.id(),high:ConditionNode::True.id()};
        let condition=conditions::Condition {root:node.id()};
        data.vocabulary.conditions.insert(&mut charge,condition.id(),condition.clone()).unwrap();
        for node in [node,ConditionNode::False] {data.vocabulary.nodes.insert(&mut charge,node.id(),node).unwrap();}
        let q=AssertionQualification {assumptions:assumptions::AssumptionSet::empty_id(),context:nominal(3),scope:nominal(4),condition:condition.id(),modality:Modality::Definite,approximation:Approximation::Exact};
        assert!(ConditionCatalog::new(&data,&out,&b).unwrap().diagram(&data,&out,&q,&b).is_err());
    }
    #[test]
    fn checked_zero_condition_refutation_requires_all_complete_native_members() {
        let b = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = SummaryData::new(&b);
        let (_, definition) =
            super::super::configuration::summaries(nominal(8), Default::default()).unwrap();
        let (invocation, _) =
            owner::AnalysisInvocation::new(nominal(1), nominal(2), definition.id(), None, []);
        let condition = Diagram::never();
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: invocation.context,
            scope: nominal(3),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let facts = SourceFacts {
            qualification: q.id(),
            status: EvidenceStatus::StructurallyObserved,
            heuristic: false,
        };
        let run = ProviderRun {
            provider: nominal(4),
            context: invocation.context,
            input: invocation.input,
            configuration: ContentHash::of(b"pure run"),
            requested_families: ContentHash::of(b"Flow"),
        };
        data.entry.runs.insert(run.clone()).unwrap();
        let native = ProviderCoverage {
            scope: q.scope,
            provider: Some(run.provider),
            context: q.context,
            family: FactFamily::Flow,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        data.entry.coverage.insert(native.clone()).unwrap();
        let (members, _) =
            complete_refutation(&data, &invocation, &q, &condition, facts, &b).unwrap();
        assert_eq!(members, vec![native.id()]);
        assert!(
            complete_refutation(&data, &invocation, &q, &Diagram::always(), facts, &b).is_err()
        );
        assert!(
            complete_refutation(
                &data,
                &invocation,
                &q,
                &condition,
                SourceFacts {
                    heuristic: true,
                    ..facts
                },
                &b
            )
            .is_err()
        );
        let partial = ProviderCoverage {
            status: CoverageStatus::Partial,
            reason: Some(ObligationKind::IncompleteCoverage),
            ..native.clone()
        };
        data.entry.coverage = Rows::new(&b);
        data.entry.coverage.insert(partial).unwrap();
        assert!(complete_refutation(&data, &invocation, &q, &condition, facts, &b).is_err());
        data.entry.coverage = Rows::new(&b);
        assert!(complete_refutation(&data, &invocation, &q, &condition, facts, &b).is_err());
        data.entry.coverage.insert(native.clone()).unwrap();
        let claim = SummaryClaim::FiniteAlternative {
            transfer: nominal(5),
            qualification: q.id(),
            channel: analysis::AnalysisChannel::Value,
            phase: CallPhase::Call,
        };
        let proof = ClaimProof::Refutation {
            invocation: invocation.id(),
            claim: claim.id(),
            source: nominal(6),
            qualification: q.id(),
            status: facts.status,
            heuristic: false,
            coverage: super::super::records::ordered_digest(
                "summary-refutation-coverage",
                members.iter().copied(),
            ),
        };
        let mut rows = ConsequenceRecords::new(&b);
        emit_proof(
            proof.clone(),
            &q,
            &condition,
            &invocation,
            &definition,
            &mut rows,
            &b,
        )
        .unwrap();
        assert_eq!(rows.derivations.len(), 1);
        assert!(
            rows.pending.is_empty() && rows.obligations.is_empty(),
            "negative evidence cannot mint a positive discharge"
        );
        let replay = |erase: bool| {
            let mut check = RefutationCheck {
                proofs: Rows::new(&b),
                members: Rows::new(&b),
                claims: Rows::new(&b),
                qualifications: Rows::new(&b),
                conditions: Rows::new(&b),
                nodes: Rows::new(&b),
                invocations: Rows::new(&b),
                data: SummaryData::new(&b),
                budget: b.clone(),
            };
            check.proofs.insert(proof.clone()).unwrap();
            check.claims.insert(claim.clone()).unwrap();
            check.qualifications.insert(q.clone()).unwrap();
            let (c, nodes) = condition.records();
            check.conditions.insert(c).unwrap();
            for node in nodes {
                check.nodes.insert(node).unwrap();
            }
            check.invocations.insert(invocation.clone()).unwrap();
            check.data.entry.runs.insert(run.clone()).unwrap();
            check.data.entry.coverage.insert(native.clone()).unwrap();
            if !erase {
                check
                    .members
                    .insert(ClaimRefutationCoverage {
                        proof: proof.id(),
                        ordinal: 0,
                        coverage: native.id(),
                    })
                    .unwrap();
            }
            Box::new(check).finish()
        };
        replay(false).unwrap();
        assert!(
            replay(true).is_err(),
            "erased membership cannot establish a negative"
        );
    }
    #[test]
    fn shared_verdicts_and_exact_discharge_keep_negative_boundaries() {
        let yes = Diagram::always();
        let no = Diagram::never();
        let conditional = Diagram::from_atom(nominal(3));
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: nominal(1),
            scope: nominal(2),
            condition: yes.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let facts = SourceFacts {
            qualification: q.id(),
            status: EvidenceStatus::StructurallyObserved,
            heuristic: false,
        };
        assert_eq!(
            conclusion(&yes, &q, CoverageStatus::Partial, &[], facts).verdict,
            Verdict::Established
        );
        assert_eq!(
            conclusion(&conditional, &q, CoverageStatus::Partial, &[], facts).verdict,
            Verdict::Conditional
        );
        assert_eq!(
            conclusion(
                &no,
                &q,
                CoverageStatus::CompleteUnderStatedModel,
                &[],
                facts
            )
            .verdict,
            Verdict::RefutedUnderModel
        );
        assert_eq!(
            conclusion(&no, &q, CoverageStatus::Partial, &[], facts).verdict,
            Verdict::Unknown
        );
        assert_eq!(
            conclusion(&yes, &q, CoverageStatus::NotRequested, &[], facts).verdict,
            Verdict::NotAnalyzed
        );
        assert_eq!(
            conclusion(
                &yes,
                &q,
                CoverageStatus::CompleteUnderStatedModel,
                &[ObligationKind::SummaryProofLimit],
                facts
            )
            .verdict,
            Verdict::Unknown
        );
        assert_eq!(
            conclusion(
                &yes,
                &q,
                CoverageStatus::Partial,
                &[],
                SourceFacts {
                    heuristic: true,
                    ..facts
                }
            )
            .verdict,
            Verdict::Unknown
        );
        let mut decisions = obligation::Decisions::default();
        decisions.proof(1u8, 7u8, Verdict::Established);
        decisions.open(2u8, ObligationKind::SummaryProofLimit);
        assert!(!obligation::discharge(&BTreeSet::from([1u8, 2u8]), &decisions).0);
        assert!(matches!(decisions.decide(1), Standing::Proved { proof: 7 }));
    }
}
