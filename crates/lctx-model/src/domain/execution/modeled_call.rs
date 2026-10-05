//! Authored normal-return metadata is consumed only after the exact bound actuals have
//! independently replayed normal evaluation and safe temporary disposal.
use super::{
    completion_production::CompletedEvaluations,
    evaluation::{CheckedEvaluation, ReleaseSafety},
    model_application::CheckedModelApplication,
};
use crate::Domain;
use crate::domain::{
    analysis::{self, enriched_execution as publication, policy::EvidenceStatus},
    calls::{BindingProjection, BindingSource, SignatureParameter},
    normalized::{Rows, entities::EntityRef},
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::Occurrence,
    *,
};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="modeled_call_evaluations",rule="modeled_call_evaluation",invariant_refs=super::enriched_production::modeled_invariants_refs)]
pub struct ModeledCallEvaluation {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub expression: Id<Occurrence>,
    pub owner: Id<EntityRef>,
    pub qualification: Id<assertion::AssertionQualification>,
    #[model(premise)]
    pub model: Id<models::AuthoredModel>,
    pub catalog: Id<models::ModelCatalog>,
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub returned_formal: Id<SignatureParameter>,
    pub returned_actual: Id<Occurrence>,
    pub arguments: ContentHash,
    pub release: ReleaseSafety,
    pub status: EvidenceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="modeled_call_arguments",rule="modeled_call_argument",conclusion=call)]
pub struct ModeledCallArgument {
    #[model(key)]
    pub call: Id<ModeledCallEvaluation>,
    #[model(key)]
    pub ordinal: i64,
    pub formal: Id<SignatureParameter>,
    pub actual: Id<Occurrence>,
    #[model(premise)]
    pub evaluation: Id<super::records::ExpressionEvaluation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="modeled_call_native_premises",rule="modeled_call_native_premise",conclusion=call)]
pub struct ModeledCallNative {
    #[model(key)]
    pub call: Id<ModeledCallEvaluation>,
    #[model(key, premise)]
    pub premise: Id<analysis::native::NativeAssertionPremise>,
}
impl analysis::support::sealed::DerivedEvidence for ModeledCallEvaluation {}
impl analysis::support::DerivedEvidence for ModeledCallEvaluation {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct CheckedModeledEvaluation {
    evaluation: CheckedEvaluation,
    record: ModeledCallEvaluation,
    arguments: Rows<ModeledCallArgument>,
    native: Rows<ModeledCallNative>,
    _charge: charged::StateCharge,
}
impl CheckedModeledEvaluation {
    pub fn evaluation(&self) -> &CheckedEvaluation {
        &self.evaluation
    }
    pub fn record(&self) -> &ModeledCallEvaluation {
        &self.record
    }
    pub fn arguments(&self) -> &Rows<ModeledCallArgument> {
        &self.arguments
    }
    pub fn native(&self) -> &Rows<ModeledCallNative> {
        &self.native
    }
    pub(crate) fn into_evaluation(self) -> CheckedEvaluation {
        self.evaluation
    }
    pub fn derive(
        data: &super::evaluation::EvaluationData,
        application: &CheckedModelApplication<'_>,
        earlier: &CompletedEvaluations,
        invocation: &publication::AnalysisInvocation,
        definition: &analysis::AnalysisDefinition,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        if *definition != super::configuration::enriched_execution(application.catalog()).1
            || invocation.definition != definition.id()
            || invocation.subject.is_some()
            || (invocation.input, invocation.context)
                != (application.shape().input(), application.shape().context())
        {
            return Err(ModelError::Invalid(
                "modeled call changes selected catalog/frame".into(),
            ));
        }
        let Some((formal, actual)) = application.normal_parameter() else {
            return Ok(Err(ObligationKind::UnsupportedControlFlow));
        };
        let request = super::evaluation::ExpressionRequest {
            input: invocation.input,
            context: invocation.context,
            owner: application.shape().owner_entity(),
            expression: application.bound().bound().site(),
        };
        let mut charge = charged::StateCharge::new(budget, "modeled_call_argument_replay");
        let count = application.bound().bound().bindings().len();
        charge.grow(
            count
                .checked_mul(
                    (size_of::<CheckedEvaluation>() + size_of::<ModeledCallArgument>()) * 2,
                )
                .ok_or_else(|| ModelError::Invalid("modeled argument allowance overflow".into()))?,
        )?;
        let mut arguments = Vec::new();
        let mut returned = None;
        let mut status = application.status();
        let mut actuals = application.arguments().collect::<Vec<_>>();
        actuals.sort_by_key(|argument| argument.ordinal);
        if actuals.iter().enumerate().any(|(ordinal, argument)| {
            argument.ordinal != ordinal as i64
                || !matches!(
                    argument.kind,
                    calls::ArgumentKind::Positional | calls::ArgumentKind::Keyword
                )
        }) {
            return Ok(Err(ObligationKind::CallTransfer));
        }
        let mut bound_actuals = 0usize;
        for binding in application.bound().bound().bindings() {
            match (&binding.source, &binding.projection) {
                (BindingSource::Actual { occurrence }, BindingProjection::Whole) => {
                    if !actuals.iter().any(|argument| argument.value == *occurrence) {
                        return Ok(Err(ObligationKind::MissingEvidence));
                    }
                    bound_actuals += 1;
                }
                (BindingSource::EmptyVarargs | BindingSource::EmptyKwargs, _) => {}
                _ => return Ok(Err(ObligationKind::DefaultUnavailable)),
            }
        }
        if bound_actuals != actuals.len() {
            return Ok(Err(ObligationKind::MissingEvidence));
        }
        for actual_argument in actuals {
            let mut matching=application.bound().bound().bindings().iter().filter(|binding|matches!(binding.source,BindingSource::Actual{occurrence}if occurrence==actual_argument.value));
            let Some(binding) = matching.next() else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            if matching.next().is_some() {
                return Ok(Err(ObligationKind::AmbiguousBinding));
            }
            let occurrence = match (&binding.source, &binding.projection) {
                (BindingSource::Actual { occurrence }, BindingProjection::Whole) => *occurrence,
                (BindingSource::EmptyVarargs | BindingSource::EmptyKwargs, _) => continue,
                _ => return Ok(Err(ObligationKind::DefaultUnavailable)),
            };
            let mut rows = earlier.earlier().evaluations.iter().filter(|row| {
                row.expression == occurrence
                    && row.owner == request.owner
                    && earlier
                        .earlier()
                        .invocations
                        .get(row.invocation)
                        .is_some_and(|parent| {
                            (parent.input, parent.context) == (request.input, request.context)
                        })
            });
            let Some(row) = rows.next() else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            if rows.next().is_some() {
                return Ok(Err(ObligationKind::AmbiguousBinding));
            }
            let checked = earlier.earlier().replay(row)?;
            if checked.release() != ReleaseSafety::Closed {
                // Builtin lookup separately proves that the exact returned object remains externally held;
                // a parameter-read witness cannot substitute for this disposal premise.
                if !super::builtin_read::CheckedBuiltinRead::derive(
                    data,
                    checked.request(),
                    budget,
                )?
                .is_ok()
                    && !earlier.earlier().caller_holds_argument(row)?
                {
                    return Ok(Err(ObligationKind::FrameExitCleanup));
                }
            }
            status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                [status, checked.status()],
            );
            if binding.formal == formal && occurrence == actual {
                returned = Some(arguments.len());
            }
            arguments.push((binding.formal, occurrence, row.id(), checked));
        }
        let Some(returned) = returned else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let mut digest = KeySink::new("modeled-call-arguments");
        for (formal, actual, id, _) in &arguments {
            formal.encode(&mut digest);
            actual.encode(&mut digest);
            id.encode(&mut digest);
        }
        let checked = super::evaluation::modeled_call_evaluation(
            data,
            request,
            &arguments[returned].3,
            status,
            budget,
        )?;
        let mut checked = match checked {
            Ok(proof) => proof,
            Err(reason) => return Ok(Err(reason)),
        };
        if arguments.iter().any(|(_, _, _, proof)| {
            data.qualifications.get(proof.qualification())
                != data.qualifications.get(checked.qualification())
        }) {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let record = ModeledCallEvaluation {
            invocation: invocation.id(),
            expression: request.expression,
            owner: request.owner,
            qualification: checked.qualification(),
            model: application.model(),
            catalog: application.catalog(),
            attempt: application.shape().attempt(),
            event: application.shape().event(),
            returned_formal: formal,
            returned_actual: actual,
            arguments: digest.finish(),
            release: checked.release(),
            status: checked.status(),
        };
        super::evaluation::mark_modeled(&mut checked, record.id());
        let mut rows = Rows::new(budget);
        for (ordinal, (formal, actual, evaluation, _)) in arguments.into_iter().enumerate() {
            rows.insert(ModeledCallArgument {
                call: record.id(),
                ordinal: ordinal as i64,
                formal,
                actual,
                evaluation,
            })?;
        }
        let mut native = Rows::new(budget);
        for premise in application.premises().iter() {
            native.insert(ModeledCallNative {
                call: record.id(),
                premise: premise.id(),
            })?;
        }
        for premise in checked.native_premises() {
            native.insert(ModeledCallNative {
                call: record.id(),
                premise: *premise,
            })?;
        }
        Ok(Ok(Self {
            evaluation: checked,
            record,
            arguments: rows,
            native,
            _charge: charge,
        }))
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ModeledCallEvaluation>(),
        Relation::of::<ModeledCallArgument>(),
        Relation::of::<ModeledCallNative>(),
    ]
}
