//! Shared question comparison for immutable, nominal obligation owners.
use super::{AnalysisChannel, invalid};
use crate::domain::{
    *, assertion::AssertionQualification, attribution::{AnalysisContext, CoverageStatus},
    calls::CallPhase, derivation::RowRef, normalized::coverage::EvidenceAvailability,
    source::CoverageScope,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Question {
    pub invocation: RowRef,
    pub subject: RowRef,
    pub channel: AnalysisChannel,
    pub phase: CallPhase,
    pub qualification: Id<AssertionQualification>,
}
impl HeapSize for Question {}
pub(crate) trait ObligationQuestion: Record {
    fn question(&self) -> Question;
}

pub(crate) fn admissible(
    question: &Question,
    proof: &Question,
    proposition: RowRef,
    qualification: &AssertionQualification,
    condition: &conditions::Diagram,
    coverage: (Id<CoverageScope>, Id<AnalysisContext>, EvidenceAvailability),
) -> Result<obligation::Conclusion, ModelError> {
    if (question.subject, question.channel, question.phase, question.qualification)
        != (proof.subject, proof.channel, proof.phase, proof.qualification)
        || proof.qualification != qualification.id()
        || condition.id() != qualification.condition
        || (coverage.0, coverage.1) != (qualification.scope, qualification.context)
    {
        return Err(invalid("discharge proof names another question or frame"));
    }
    let status = match coverage.2 {
        EvidenceAvailability::Complete => CoverageStatus::CompleteUnderStatedModel,
        EvidenceAvailability::Partial => CoverageStatus::Partial,
        EvidenceAvailability::Unavailable => CoverageStatus::Unavailable,
        EvidenceAvailability::NotRequested => CoverageStatus::NotRequested,
        EvidenceAvailability::NoScope => return Err(invalid("empty domain cannot discharge an obligation")),
    };
    let result = obligation::verdict(obligation::VerdictInput {
        condition: Some(condition), open: &[], coverage: status,
        approximation: qualification.approximation, modality: qualification.modality,
    });
    let mut decisions = obligation::Decisions::default();
    decisions.proof(question.subject, proposition, result.verdict);
    if !obligation::discharge(&std::collections::BTreeSet::from([question.subject]), &decisions).0 {
        return Err(invalid("proof does not discharge the exact obligation"));
    }
    Ok(result)
}
