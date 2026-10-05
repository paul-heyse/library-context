//! A qualified direct-suite normal edge under the question GivenInvocationEntered.
//! This is neither a body receipt nor an exception/effect/cleanup completion certificate.
use super::{
    protocol_interpretation::{
        ConditionalTerminalFrontier, InvocationQuestion, NormalContinuationRestriction,
    },
    summary_consequences::SummaryClaim,
    summary_production::{SummaryData, SummaryRecords},
};
use crate::Domain;
use crate::domain::{
    analysis::{
        self, summary as owner,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::AssertionQualification,
    attribution::Modality,
    conditions::Diagram,
    resources::ResourceBudget,
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(
    name = "summary_terminal_witnesses",
    rule = "given_entry_terminal_summary"
)]
pub struct SummaryTerminalWitness {
    #[model(key, premise)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)]
    pub frontier: Id<ConditionalTerminalFrontier>,
    #[model(key, premise)]
    pub restriction: Id<NormalContinuationRestriction>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub claim: Id<SummaryClaim>,
    pub question: InvocationQuestion,
    pub status: analysis::policy::EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for SummaryTerminalWitness {}
impl DerivedEvidence for SummaryTerminalWitness {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub(super) fn produce(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    out: &mut SummaryRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let invalid = |s: &str| ModelError::Invalid(s.into());
    for edge in data.normal_restrictions.iter() {
        let f = data
            .terminal_frontiers
            .get(edge.frontier)
            .ok_or_else(|| invalid("terminal edge frontier absent"))?;
        let parent = data
            .model_invocations
            .get(f.invocation)
            .ok_or_else(|| invalid("terminal Model parent absent"))?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        let target = data
            .closed_targets
            .get(f.target)
            .ok_or_else(|| invalid("terminal target assessment absent"))?;
        if target.invocation != f.invocation
            || target.basis == super::closed_targets::TargetBasis::Open
            || target.reason.is_some()
            || f.question != InvocationQuestion::GivenInvocationEntered
            || !f.effects_unknown
            || !f.exceptions_unknown
            || (edge.owner, edge.from, edge.qualification)
                != (f.owner, f.statement, f.qualification)
        {
            return Err(invalid(
                "terminal frontier changes its admitted question or target",
            ));
        }
        let q = data
            .vocabulary
            .qualifications
            .get(&f.qualification)
            .ok_or_else(|| invalid("terminal qualification absent from Model vocabulary"))?;
        if q.context != invocation.context
            || q.assumptions == assumptions::AssumptionSet::empty_id()
            || q.modality != Modality::Definite
            || q.approximation != assertion::Approximation::Exact
        {
            return Err(invalid("terminal question lost its conditional basis"));
        }
        let basis = assumptions::AssumptionResolver::resolve(
            &assumptions::AssumptionCatalog {
                sets: &data.vocabulary.assumption_sets,
                members: &data.vocabulary.assumption_members,
                definitions: &data.vocabulary.assumptions,
            },
            q.assumptions,
        )?;
        if !basis.members.iter().any(|m| m.assumption == f.conformance) {
            return Err(invalid(
                "terminal question lost declared return conformance",
            ));
        }
        for id in [target.receiver_conformance, target.no_extra_overrides]
            .into_iter()
            .flatten()
        {
            if !basis.members.iter().any(|m| m.assumption == id) {
                return Err(invalid("terminal target closure basis lost"));
            }
        }
        let claim = SummaryClaim::NoNormalContinuation {
            owner: f.owner,
            frontier: f.id(),
            restriction: edge.id(),
            qualification: q.id(),
            question: f.question,
        };
        let witness = SummaryTerminalWitness {
            invocation: invocation.id(),
            frontier: f.id(),
            restriction: edge.id(),
            qualification: q.id(),
            claim: claim.id(),
            question: f.question,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
        };
        let source = owner::SupportSource::TerminalFrontier {
            witness: witness.id(),
        };
        let subject = owner::ObligationSubject::SummaryClaim {
            transfer: claim.id(),
        };
        let condition = Diagram::from_records(
            data.vocabulary
                .conditions
                .get(&q.condition)
                .ok_or_else(|| invalid("terminal condition absent"))?,
            &data.vocabulary.nodes.values().cloned().collect::<Vec<_>>(),
        )?;
        let evidence = owner::support::EvidencePremise::derived(&source, &witness, q, &condition)?;
        let (derivation, proposition, members, qualified) = owner::AnalysisDerivation::emit(
            invocation,
            definition,
            subject.id(),
            analysis::AnalysisChannel::Role,
            calls::CallPhase::Call,
            analysis::support::QualificationOperation::Conjunction,
            &[evidence],
            budget,
        )?;
        out.terminal_witnesses.insert(witness)?;
        out.claims.insert(claim)?;
        out.sources.insert(source)?;
        out.subjects.insert(subject)?;
        out.propositions.insert(proposition)?;
        out.derivations.insert(derivation)?;
        for m in members {
            out.derivation_premises.insert(m)?;
        }
        out.vocabulary
            .qualified(&qualified.qualification, &qualified.condition)?;
    }
    Ok(())
}
