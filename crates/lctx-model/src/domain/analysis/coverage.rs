use crate::domain::normalized::coverage::EvidenceAvailability;
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
