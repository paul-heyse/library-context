use crate::domain::normalized::coverage::EvidenceAvailability;
use crate::domain::{attribution::AnalysisContext, source::CoverageScope, *};
pub(crate) mod sealed {
    pub trait CoverageEvidence {}
}
/// Only concrete immutable owner coverage rows implement this adapter.
pub trait CoverageEvidence: Record + sealed::CoverageEvidence {
    fn coverage_frame(&self) -> (Id<CoverageScope>, Id<AnalysisContext>, EvidenceAvailability);
}
pub fn combine_availability(
    values: impl IntoIterator<Item = EvidenceAvailability>,
) -> EvidenceAvailability {
    let mut result = EvidenceAvailability::Complete;
    for value in values {
        result = match (result, value) {
            (EvidenceAvailability::Unavailable, _) | (_, EvidenceAvailability::Unavailable) => {
                EvidenceAvailability::Unavailable
            }
            (EvidenceAvailability::NotRequested, _) | (_, EvidenceAvailability::NotRequested) => {
                EvidenceAvailability::NotRequested
            }
            (EvidenceAvailability::Partial, _) | (_, EvidenceAvailability::Partial) => {
                EvidenceAvailability::Partial
            }
            _ => EvidenceAvailability::Complete,
        };
    }
    result
}
