//! Model-owned value identity requires an actual independently replayed normal call and
//! parameter entry evidence. The output denotes the call expression; enclosing return
//! semantics remain owned by execution/Summary.
use super::{
    completion_production::CompletedEvaluations,
    model_production::ModelApplication,
    model_rules::{AppliedRule, ModelValuePath, ModeledOperation},
    modeled_call::{ModeledCallArgument, ModeledCallEvaluation},
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
#[model(name = "model_transfer_witnesses", rule = "modeled_value_identity")]
pub struct ModelTransferWitness {
    #[model(key, premise)]
    pub application: Id<ModelApplication>,
    #[model(key, premise)]
    pub rule: Id<AppliedRule>,
    #[model(premise)]
    pub call: Id<ModeledCallEvaluation>,
    #[model(premise)]
    pub entry: Id<EntryValueWitness>,
    pub input: Id<Place>,
    pub output: Id<Place>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for ModelTransferWitness {}
impl analysis::support::DerivedEvidence for ModelTransferWitness {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct CheckedModelTransfer {
    pub(super) witness: ModelTransferWitness,
    pub(super) entry: std::sync::Arc<DerivedEntryValue>,
    pub(super) root: PlaceRoot,
    pub(super) place: Place,
    pub(super) branch: TransferBranch<transfer::model::TransferKey>,
}
pub(super) struct TransferRuleInputs<'a> {
    pub(super) application: &'a ModelApplication,
    pub(super) rule: &'a AppliedRule,
    pub(super) operation: &'a ModeledOperation,
    pub(super) paths: &'a Rows<ModelValuePath>,
}

pub(super) struct TransferEntryInputs<'a> {
    pub(super) entry_data: &'a EntryData,
    pub(super) entries: &'a Rows<EntryValueWitness>,
    pub(super) sources: &'a Rows<EntryAccessSource>,
}

impl CheckedModelTransfer {
    pub(super) fn derive(
        transfer_rule_inputs: TransferRuleInputs<'_>,
        call: &ModeledCallEvaluation,
        arguments: &Rows<ModeledCallArgument>,
        earlier: &CompletedEvaluations,
        transfer_entry_inputs: TransferEntryInputs<'_>,
        produced: Option<&super::model_production::ActualInputs<'_>>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let TransferRuleInputs {
            application,
            rule,
            operation,
            paths,
        } = transfer_rule_inputs;
        let TransferEntryInputs {
            entry_data,
            entries,
            sources,
        } = transfer_entry_inputs;
        let result = (|| -> Result<_, ObligationKind> {
            if !rule.applicable
                || rule.application != application.id()
                || call.attempt != application.attempt
                || call.model != application.model
                || call.owner != application.owner
            {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let ModeledOperation::Transfer { kind } = operation else {
                return Err(ObligationKind::OutsideProviderModel);
            };
            let ModelValuePath::Argument {
                formal,
                source,
                projection,
            } = paths
                .get(rule.source.ok_or(ObligationKind::MissingEvidence)?)
                .ok_or(ObligationKind::MissingEvidence)?
            else {
                return Err(ObligationKind::OutsideProviderModel);
            };
            if *projection != calls::BindingProjection::Whole.id() {
                return Err(ObligationKind::UnsupportedUnpacking);
            }
            if !matches!(paths.get(rule.destination.ok_or(ObligationKind::MissingEvidence)?),Some(ModelValuePath::Returned{site}) if *site==call.expression)
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
            let mut members = arguments.iter().filter(|m| {
                m.call == call.id()
                    && m.formal == *formal
                    && calls::BindingSource::Actual {
                        occurrence: m.actual,
                    }
                    .id()
                        == *source
            });
            let member = members.next().ok_or(ObligationKind::MissingEvidence)?;
            if members.next().is_some() {
                return Err(ObligationKind::AmbiguousBinding);
            }
            let actual = earlier
                .earlier()
                .evaluations
                .get(member.evaluation)
                .ok_or(ObligationKind::MissingEvidence)?;
            if actual.expression != member.actual || actual.owner != application.owner {
                return Err(ObligationKind::IncompatibleContexts);
            }
            Ok((*kind, actual))
        })();
        let (kind, actual) = match result {
            Ok(v) => v,
            Err(reason) => return Ok(Err(reason)),
        };
        let checked = match produced {
            Some(produced) => {
                let frame = earlier
                    .earlier()
                    .invocations
                    .get(actual.invocation)
                    .ok_or_else(|| ModelError::Invalid("Model actual Base frame absent".into()))?;
                produced.evaluations.get(actual, frame, earlier.earlier())?
            }
            None => std::sync::Arc::new(earlier.earlier().replay(actual)?),
        };
        let mut matches = checked
            .entry_premises()
            .iter()
            .filter_map(|id| entries.get(*id))
            .filter(|e| e.access == actual.expression && e.owner == application.owner);
        let Some(witness) = matches.next() else {
            return Ok(Err(ObligationKind::EntryValueUnknown));
        };
        if matches.next().is_some() {
            return Ok(Err(ObligationKind::AmbiguousBinding));
        }
        let Some(source) = sources.get(witness.access_source) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let entry = match produced {
            Some(produced) => produced.local.entry(witness, source, budget)?,
            None => {
                match EntryValueWitness::derive_for(entry_data, witness.request(), source, budget)?
                {
                    Ok(value) => std::sync::Arc::new(value),
                    Err(reason) => return Ok(Err(reason)),
                }
            }
        };
        if entry.witness() != witness {
            return Err(ModelError::Invalid(
                "Model entry witness differs from shared replay".into(),
            ));
        }
        let mut qualification = entry.qualification().clone();
        qualification.modality = rule.modality;
        let condition = entry.condition().clone();
        let root = PlaceRoot::Occurrence {
            occurrence: call.expression,
        };
        let place = Place {
            root: root.id(),
            path: value::AccessPath::empty().id(),
        };
        let status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [
                application.status,
                call.status,
                checked.status(),
                entry.evidence_status(),
            ],
        );
        let witness = ModelTransferWitness {
            application: application.id(),
            rule: rule.id(),
            call: call.id(),
            entry: entry.witness().id(),
            input: entry.place().id(),
            output: place.id(),
            qualification: qualification.id(),
            status,
        };
        let descriptor = TransferDescriptor {
            owner: application.owner,
            input: witness.input,
            output: witness.output,
            context: qualification.context,
            scope: qualification.scope,
            modality: qualification.modality,
            approximation: qualification.approximation,
            kind,
            call_site: Some(call.expression),
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
    proof: CheckedModelTransfer,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    rows: &mut super::model_production::ModelRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let source = publication::SupportSource::ModelTransferWitness {
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
    rows.transfer_witnesses.insert(proof.witness)?;
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
    vec![Relation::of::<ModelTransferWitness>()]
}
