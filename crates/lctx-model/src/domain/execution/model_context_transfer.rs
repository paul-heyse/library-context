//! Reusable context value identity requires an actual checked WithTarget read and the
//! existing Local parameter entry origin. The manager constructor is not this output.
use super::{
    completion_production::CompletedEvaluations, context_binding::ContextEntryBinding,
    model_protocol::ContextResource,
};
use crate::Domain;
use crate::domain::{
    analysis::{self, model as publication, policy::EvidenceStatus},
    conditions::entry::{DerivedEntryValue, EntryAccessSource, EntryData, EntryValueWitness},
    normalized::Rows,
    obligation::ObligationKind,
    resources::ResourceBudget,
    transfer::{self, TransferBranch, TransferDescriptor, TransferKeyRecord},
    value::{Place, PlaceRoot},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "model_context_transfer_witnesses",
    rule = "modeled_context_value_identity"
)]
pub struct ContextTransferWitness {
    #[model(key, premise)]
    pub resource: Id<ContextResource>,
    #[model(key, premise)]
    pub binding: Id<ContextEntryBinding>,
    #[model(premise)]
    pub entry: Id<EntryValueWitness>,
    pub input: Id<Place>,
    pub output: Id<Place>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for ContextTransferWitness {}
impl analysis::support::DerivedEvidence for ContextTransferWitness {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub(super) struct CheckedContextTransfer {
    witness: ContextTransferWitness,
    entry: std::sync::Arc<DerivedEntryValue>,
    root: PlaceRoot,
    place: Place,
    branch: TransferBranch<transfer::model::TransferKey>,
}
impl CheckedContextTransfer {
    #[allow(
        clippy::too_many_arguments,
        reason = "Context resource, stored binding, predecessor evaluation, Entry facts and actual values are distinct transfer premises."
    )]
    pub(super) fn derive(
        resource: &ContextResource,
        binding: &ContextEntryBinding,
        earlier: &CompletedEvaluations,
        data: &EntryData,
        entries: &Rows<EntryValueWitness>,
        sources: &Rows<EntryAccessSource>,
        produced: Option<&super::model_production::ActualInputs<'_>>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let Some(actual) = binding.entry_actual else {
            return Ok(Err(ObligationKind::EntryValueUnknown));
        };
        if (
            resource.site,
            resource.owner,
            resource.class,
            resource.protocol,
        ) != (binding.site, binding.owner, binding.class, binding.protocol)
        {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let mut evaluations = earlier.earlier().evaluations.iter().filter(|e| {
            e.expression == actual
                && e.owner == resource.owner
                && earlier
                    .earlier()
                    .invocations
                    .get(e.invocation)
                    .is_some_and(|i| i.context == resource.context)
        });
        let Some(row) = evaluations.next() else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        if evaluations.next().is_some() {
            return Ok(Err(ObligationKind::AmbiguousBinding));
        }
        let checked = match produced {
            Some(produced) => {
                let frame = earlier
                    .earlier()
                    .invocations
                    .get(row.invocation)
                    .ok_or_else(|| ModelError::Invalid("Model actual Base frame absent".into()))?;
                produced.evaluations.get(row, frame, earlier.earlier())?
            }
            None => std::sync::Arc::new(earlier.earlier().replay(row)?),
        };
        let mut witnesses = checked
            .entry_premises()
            .iter()
            .filter_map(|id| entries.get(*id))
            .filter(|e| {
                e.access == actual && e.owner == resource.owner && e.context == resource.context
            });
        let Some(witness) = witnesses.next() else {
            return Ok(Err(ObligationKind::EntryValueUnknown));
        };
        if witnesses.next().is_some() {
            return Ok(Err(ObligationKind::AmbiguousBinding));
        }
        let Some(source) = sources.get(witness.access_source) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let entry = match produced {
            Some(produced) => produced.local.entry(witness, source, budget)?,
            None => match EntryValueWitness::derive_for(data, witness.request(), source, budget)? {
                Ok(value) => std::sync::Arc::new(value),
                Err(reason) => return Ok(Err(reason)),
            },
        };
        if entry.witness() != witness {
            return Err(ModelError::Invalid(
                "context transfer Local origin differs from replay".into(),
            ));
        }
        let _nodes = budget.reserve(
            "context-transfer-condition-input",
            data.condition_nodes
                .len()
                .saturating_mul(2048)
                .saturating_add(entry.condition().allocation_allowance())
                .saturating_add(128),
        )?;
        let mut qualification = entry.qualification().clone();
        let mut condition = entry.condition().clone();
        let nodes = data.condition_nodes.iter().cloned().collect::<Vec<_>>();
        let mut reservations = Vec::with_capacity(2);
        for q in [resource.qualification, binding.qualification] {
            let Some(q) = data.qualifications.get(q) else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            if q.context != qualification.context
                || q.scope != qualification.scope
                || q.approximation != assertion::Approximation::Exact
                || q.modality != attribution::Modality::Definite
            {
                return Ok(Err(ObligationKind::IncompatibleContexts));
            }
            let Some(record) = data.conditions.get(q.condition) else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            let other = conditions::Diagram::from_records(record, &nodes)?;
            match condition.admitted_binary(
                &other,
                conditions::BooleanOperation::Conjunction,
                budget,
            ) {
                Ok(admitted) => {
                    let (result, reservation) = admitted.into_parts();
                    condition = result;
                    reservations.push(reservation);
                }
                Err(conditions::DiagramAdmissionError::Resource(error)) => return Err(error),
                Err(conditions::DiagramAdmissionError::Boundary(error)) => {
                    return Ok(Err(obligation::from_kernel(error)));
                }
            }
        }
        qualification.condition = condition.id();
        let root = PlaceRoot::Occurrence {
            occurrence: binding.target,
        };
        let place = Place {
            root: root.id(),
            path: value::AccessPath::empty().id(),
        };
        let status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [
                resource.status,
                binding.status,
                checked.status(),
                entry.evidence_status(),
            ],
        );
        let witness = ContextTransferWitness {
            resource: resource.id(),
            binding: binding.id(),
            entry: entry.witness().id(),
            input: entry.place().id(),
            output: place.id(),
            qualification: qualification.id(),
            status,
        };
        let descriptor = TransferDescriptor {
            owner: resource.owner,
            input: witness.input,
            output: witness.output,
            context: qualification.context,
            scope: qualification.scope,
            modality: qualification.modality,
            approximation: qualification.approximation,
            kind: transfer::TransferKind::Identity,
            call_site: Some(resource.site),
            provenance: transfer::ProvenanceClass::AuthoredModel,
        };
        let branch = TransferBranch::new(
            transfer::model::TransferKey::from_descriptor(descriptor),
            qualification,
            condition,
            budget,
        )?;
        Ok(Ok(Self {
            witness,
            entry,
            root,
            place,
            branch,
        }))
    }
}
pub(super) fn emit(
    proof: CheckedContextTransfer,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    rows: &mut super::model_production::ModelRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let source = publication::SupportSource::ContextTransferWitness {
        witness: proof.witness.id(),
    };
    let subject = publication::ObligationSubject::ModelTransfer {
        transfer: proof.branch.key().id(),
    };
    let evidence = publication::support::EvidencePremise::derived(
        &source,
        &proof.witness,
        proof.branch.qualification(),
        proof.branch.condition(),
    )?;
    let (derivation, proposition, premises, result) = publication::AnalysisDerivation::emit(
        invocation,
        definition,
        subject.id(),
        analysis::AnalysisChannel::Value,
        calls::CallPhase::Call,
        analysis::support::QualificationOperation::Conjunction,
        &[evidence],
        budget,
    )?;
    let generated = publication::SupportSource::AnalysisDerivation {
        derivation: derivation.id(),
    };
    let alternative = proof.branch.alternative();
    rows.transfer_keys.insert(proof.branch.key().clone())?;
    rows.transfer_alternatives.insert(alternative.clone())?;
    rows.transfer_supports
        .insert(transfer::model::TransferSupport {
            assertion: alternative.id(),
            source: generated.id(),
        })?;
    rows.context_transfers.insert(proof.witness)?;
    rows.transfer_roots.insert(proof.entry.root().clone())?;
    rows.transfer_places.insert(proof.entry.place().clone())?;
    rows.transfer_roots.insert(proof.root)?;
    rows.transfer_places.insert(proof.place)?;
    rows.support_sources.insert(source)?;
    rows.support_sources.insert(generated)?;
    rows.subjects.insert(subject)?;
    rows.derivations.insert(derivation)?;
    rows.propositions.insert(proposition)?;
    for row in premises {
        rows.derivation_premises.insert(row)?;
    }
    let (condition, nodes) = result.condition.records();
    rows.conditions.insert(condition)?;
    for node in nodes {
        rows.condition_nodes.insert(node)?;
    }
    rows.qualifications.insert(result.qualification)?;
    Ok(())
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<ContextTransferWitness>()]
}
