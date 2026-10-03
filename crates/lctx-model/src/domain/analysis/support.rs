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
    /// Materialized when distinct bases are conjoined; the vocabulary owner publishes these rows.
    pub assumptions: Option<assumptions::ResolvedAssumptions>,
    _assumption_reservation: Option<Box<dyn resources::Reservation>>,
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
    qualify_with_basis(operation, premises, None, budget)
}
/// Resolve distinct conjunction bases; identical bases preserve their already validated identity.
/// Alternatives must be partitioned by the full governing qualification before entering here.
pub fn qualify_with_basis<S: Record>(
    operation: QualificationOperation,
    premises: &[QualifiedPremise<'_, S>],
    basis: Option<&dyn assumptions::AssumptionResolver>,
    budget: &resources::ResourceBudget,
) -> Result<QualifiedResult, ModelError> {
    let first = premises
        .first()
        .ok_or_else(|| invalid("qualified derivation needs evidence"))?;
    let mut reservation = budget.reserve(
        "qualified_analysis_diagram",
        first.condition.allocation_allowance(),
    )?;
    let _basis_lookup = if basis.is_some() { Some(budget.reserve("qualified_assumption_lookup", assumptions::MAX_ASSUMPTIONS * size_of::<assumptions::AssumptionSetMember>())?) } else { None };
    let mut condition = first.condition.clone();
    let mut assumption_ids = std::collections::BTreeSet::new();
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
        assumption_ids.insert(q.assumptions);
        if operation == QualificationOperation::AlternativeUnion
            && (q.assumptions, q.modality, q.approximation)
                != (first.qualification.assumptions, first.qualification.modality, first.qualification.approximation)
        { return Err(invalid("alternative union needs identical governing qualifications")); }
        if let Some(index) = basis { index.resolve(q.assumptions)?.check(q.assumptions)?; }
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
    let mut assumption_reservation = None;
    let assumptions = if assumption_ids.len() > 1 {
        let index = basis.ok_or_else(|| invalid("distinct assumption bases need resolved membership"))?;
        assumption_reservation = Some(budget.reserve("qualified_assumption_union", assumptions::MAX_ASSUMPTIONS.checked_mul(assumption_ids.len()).and_then(|n|n.checked_mul(size_of::<assumptions::AssumptionSetMember>())).ok_or_else(||invalid("assumption union allowance overflow"))?)?);
        let sets = assumption_ids.into_iter().map(|id| { let set=index.resolve(id)?;set.check(id)?;Ok(set) }).collect::<Result<Vec<_>, ModelError>>()?;
        Some(assumptions::ResolvedAssumptions::union(&sets)?)
    } else { None };
    let assumption_id = assumptions.as_ref().map_or(first.qualification.assumptions, |s| s.set.id());
    Ok(QualifiedResult {
        qualification: AssertionQualification {
        assumptions: assumption_id,
            context: first.qualification.context,
            scope: first.qualification.scope,
            condition: condition.id(),
            modality,
            approximation,
        },
        condition,
        assumptions,
        _assumption_reservation: assumption_reservation,
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

/// Complete governing qualifications of an alternative, excluding only its Boolean condition.
pub fn alternative_basis(q: &AssertionQualification) -> ContentHash {
    let mut sink = KeySink::new("qualified-alternative-basis");
    q.context.encode(&mut sink); q.scope.encode(&mut sink); q.assumptions.encode(&mut sink);
    q.modality.encode(&mut sink); q.approximation.encode(&mut sink); sink.finish()
}
