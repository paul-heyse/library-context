//! Shared qualification policy for all immutable analysis owners.
use super::invalid;
use crate::DomainCode;
use crate::domain::{
    assertion::{Approximation, AssertionQualification},
    attribution::Modality,
    conditions::{BooleanOperation, Diagram, DiagramAdmissionError},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum QualificationOperation {
    Conjunction = 0,
    AlternativeUnion = 1,
}
/// Canonical input to the shared qualification operation. Construction checks the BDD identity;
/// nominal source linkage is checked by the stored invariant, rather than accepted as a boolean.
#[derive(Debug, Clone, Copy)]
pub struct QualifiedPremise<'a, S: Record> {
    pub source: &'a S,
    pub qualification: &'a AssertionQualification,
    pub condition: &'a Diagram,
}
pub struct QualifiedResult {
    pub qualification: AssertionQualification,
    pub condition: Diagram,
    _reservation: Box<dyn resources::Reservation>,
}
/// Both native and derived premises use this operation. Conditions use the authoritative BDD;
/// modalities/approximations use their shared weakest/join operations. No desired result status
/// is accepted. The source DAG and exact membership are validated separately.
pub fn qualify<S: Record>(
    operation: QualificationOperation,
    premises: &[QualifiedPremise<'_, S>],
    budget: &resources::ResourceBudget,
) -> Result<QualifiedResult, ModelError> {
    let first = premises
        .first()
        .ok_or_else(|| invalid("qualified derivation needs evidence"))?;
    let mut reservation = budget.reserve(
        "qualified_analysis_diagram",
        first.condition.allocation_allowance(),
    )?;
    let mut condition = first.condition.clone();
    let mut modality = Modality::Definite;
    let mut approximation = Approximation::Exact;
    for (index, premise) in premises.iter().enumerate() {
        let q = premise.qualification;
        if (q.context, q.scope) != (first.qualification.context, first.qualification.scope)
            || q.condition != premise.condition.id()
        {
            return Err(invalid(
                "qualification premise crosses frame or changes condition",
            ));
        }
        modality = modality.weakest(q.modality);
        approximation = approximation.join(q.approximation);
        if index != 0 {
            let admitted = condition
                .admitted_binary(
                    premise.condition,
                    match operation {
                        QualificationOperation::Conjunction => BooleanOperation::Conjunction,
                        QualificationOperation::AlternativeUnion => BooleanOperation::Disjunction,
                    },
                    budget,
                )
                .map_err(|e| match e {
                    DiagramAdmissionError::Resource(error) => error,
                    DiagramAdmissionError::Boundary(boundary) => invalid(&format!(
                        "qualification boundary: {:?}",
                        obligation::from_kernel(boundary)
                    )),
                })?;
            (condition, reservation) = admitted.into_parts();
        }
    }
    Ok(QualifiedResult {
        qualification: AssertionQualification {
            context: first.qualification.context,
            scope: first.qualification.scope,
            condition: condition.id(),
            modality,
            approximation,
        },
        condition,
        _reservation: reservation,
    })
}

pub(crate) fn membership_digest<R: Record>(
    owner: &'static str,
    values: &std::collections::BTreeSet<Id<R>>,
) -> ContentHash {
    let mut sink = KeySink::new(owner);
    for value in values {
        value.encode(&mut sink);
    }
    sink.finish()
}
#[derive(Debug, Clone, Copy)]
pub struct SourceFacts {
    pub qualification: Id<AssertionQualification>,
    pub status: super::policy::EvidenceStatus,
    pub heuristic: bool,
}
impl HeapSize for SourceFacts {}
/// The common status operation is conservative even when heuristic evidence has stronger siblings.
pub fn inferred_status(
    interpretation: super::Interpretation,
    lower: impl IntoIterator<Item = super::policy::EvidenceStatus>,
) -> super::policy::EvidenceStatus {
    use super::policy::{EvidenceStatus, SupportRole, derive_status};
    let mut status = EvidenceStatus::StructurallyObserved;
    let mut any = false;
    for lower in lower {
        any = true;
        status = derive_status(&[
            (SupportRole::Support, status),
            (SupportRole::Support, lower),
        ]);
    }
    if !any {
        EvidenceStatus::Unresolved
    } else if status != EvidenceStatus::Unresolved
        && interpretation == super::Interpretation::Heuristic
    {
        EvidenceStatus::StatisticallyDerived
    } else {
        status
    }
}
pub(crate) mod sealed {
    pub trait DerivedEvidence {}
}
pub trait DerivedEvidence: Record + sealed::DerivedEvidence {
    fn source_facts(&self) -> SourceFacts;
}
