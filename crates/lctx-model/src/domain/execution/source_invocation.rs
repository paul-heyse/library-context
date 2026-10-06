//! Caller continuation requires independent fresh binding, ordered actual evaluation,
//! callee outcome and exact outside holders during frame release.
use super::{
    body::CheckedSourceBody,
    completion_production::CompletedEvaluations,
    evaluation::{EvaluationData, ReleaseSafety},
    outcome::PendingOutcome,
    source_call::CheckedSourceBinding,
};
use crate::domain::{
    analysis::{Interpretation, policy::EvidenceStatus, support::inferred_status},
    obligation::ObligationKind,
    resources::ResourceBudget,
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationOutcome {
    Normal,
    Raised {
        site: Id<source::Occurrence>,
        exception: super::ExactRuntimeException,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvocationArgument {
    pub formal: Id<calls::SignatureParameter>,
    pub actual: Id<source::Occurrence>,
    pub evaluation: Id<super::records::ExpressionEvaluation>,
}
pub struct CheckedSourceInvocation {
    event: Id<normalized::events::NormalizedCallEvent>,
    callee: Id<normalized::entities::EntityRef>,
    qualification: Id<assertion::AssertionQualification>,
    outcome: InvocationOutcome,
    status: EvidenceStatus,
    arguments: Vec<InvocationArgument>,
    release: ReleaseSafety,
    _charge: charged::StateCharge,
}
impl CheckedSourceInvocation {
    pub fn event(&self) -> Id<normalized::events::NormalizedCallEvent> {
        self.event
    }
    pub fn callee(&self) -> Id<normalized::entities::EntityRef> {
        self.callee
    }
    pub fn qualification(&self) -> Id<assertion::AssertionQualification> {
        self.qualification
    }
    pub fn outcome(&self) -> InvocationOutcome {
        self.outcome
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn arguments(&self) -> &[InvocationArgument] {
        &self.arguments
    }
    pub fn release(&self) -> ReleaseSafety {
        self.release
    }
    pub fn derive(
        data: &EvaluationData,
        header: &CheckedSourceBinding,
        body: &CheckedSourceBody,
        earlier: &CompletedEvaluations,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_with_captures(data, header, body, earlier, &[], budget)
    }
    pub(super) fn derive_with_captures(
        data: &EvaluationData,
        header: &CheckedSourceBinding,
        body: &CheckedSourceBody,
        earlier: &CompletedEvaluations,
        captures: &[super::capture_bridge::CheckedCapturedEntry],
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_with_values(data, header, body, earlier, captures, budget, None)
    }
    pub fn derive_produced(
        data: &EvaluationData, header: &CheckedSourceBinding, body: &CheckedSourceBody,
        earlier: &CompletedEvaluations, budget: &ResourceBudget,
        evaluations: &super::production::ProducedEvaluations,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_with_values(data, header, body, earlier, &[], budget, Some(evaluations))
    }
    pub(super) fn derive_with_values(
        data: &EvaluationData, header: &CheckedSourceBinding, body: &CheckedSourceBody,
        earlier: &CompletedEvaluations, captures: &[super::capture_bridge::CheckedCapturedEntry],
        budget: &ResourceBudget, evaluations: Option<&super::production::ProducedEvaluations>,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let h = header.request();
        if captures.iter().any(|c| {
            c.row.caller != header.caller()
                || c.row.callee != header.callee()
                || data
                    .qualifications
                    .get(c.row.qualification)
                    .is_none_or(|q| q.context != h.context)
                || !header.captures().iter().any(|origin| {
                    origin.read == c.row.read && origin.source.id() == c.row.value_source
                })
        }) {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let mut charge = charged::StateCharge::new(budget, "source_call_frame_release");
        charge.grow(
            size_of::<Self>() + header.arguments().len() * size_of::<InvocationArgument>() * 2,
        )?;
        let h = header.request();
        let b = body.request();
        if (h.input, h.context, header.callee(), header.declaration())
            != (b.input, b.context, b.callee, body.declaration())
        {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let Some(caller) = data.qualifications.get(header.qualification()) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let Some(callee) = data.qualifications.get(body.qualification()) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        if caller != callee {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let base = earlier.earlier();
        let mut arguments = Vec::new();
        let mut status =
            inferred_status(Interpretation::Structural, [header.status(), body.status()]);
        for (formal, actual) in header.arguments() {
            let mut rows = base.evaluations.iter().filter(|row| {
                row.expression == *actual
                    && row.owner == header.caller()
                    && base
                        .invocations
                        .get(row.invocation)
                        .is_some_and(|frame| (frame.input, frame.context) == (h.input, h.context))
            });
            let Some(row) = rows.next() else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            if rows.next().is_some() {
                return Ok(Err(ObligationKind::AmbiguousBinding));
            }
            let checked = checked_value(base, row, evaluations)?;
            if checked.exception().is_some() {
                return Ok(Err(ObligationKind::CallTransfer));
            }
            if checked.release() != ReleaseSafety::Closed
                && held_formal(base, row, evaluations)?.is_none()
                && !super::builtin_read::CheckedBuiltinRead::derive(
                    data,
                    checked.request(),
                    budget,
                )?
                .is_ok()
            {
                return Ok(Err(ObligationKind::FrameExitCleanup));
            }
            if data.qualifications.get(checked.qualification()) != Some(caller) {
                return Ok(Err(ObligationKind::IncompatibleContexts));
            }
            status = inferred_status(Interpretation::Structural, [status, checked.status()]);
            arguments.push(InvocationArgument {
                formal: *formal,
                actual: *actual,
                evaluation: row.id(),
            });
        }
        for (site, safety) in body.release_inputs() {
            if *safety == ReleaseSafety::Closed {
                continue;
            }
            if *safety != ReleaseSafety::CallerRetained {
                return Ok(Err(ObligationKind::FrameExitCleanup));
            }
            if captures.iter().any(|capture| capture.row.read == *site) {
                continue;
            }
            let mut rows = base.evaluations.iter().filter(|row| {
                row.expression == *site
                    && row.owner == header.callee()
                    && base
                        .invocations
                        .get(row.invocation)
                        .is_some_and(|frame| (frame.input, frame.context) == (h.input, h.context))
            });
            let Some(row) = rows.next() else {
                return Ok(Err(ObligationKind::FrameExitCleanup));
            };
            if rows.next().is_some() {
                return Ok(Err(ObligationKind::AmbiguousBinding));
            }
            let Some(formal) = held_formal(base, row, evaluations)? else {
                return Ok(Err(ObligationKind::FrameExitCleanup));
            };
            if arguments
                .iter()
                .filter(|argument| argument.formal == formal)
                .count()
                != 1
            {
                return Ok(Err(ObligationKind::FrameExitCleanup));
            }
        }
        let outcome = match body.outcome() {
            PendingOutcome::Normal | PendingOutcome::Return { .. } => InvocationOutcome::Normal,
            PendingOutcome::Raise { site, exception } => {
                InvocationOutcome::Raised { site, exception }
            }
            PendingOutcome::Break { .. } | PendingOutcome::Continue { .. } => {
                return Ok(Err(ObligationKind::UnsupportedControlFlow));
            }
        };
        let mut release = ReleaseSafety::Closed;
        if let PendingOutcome::Return { site } = body.outcome() {
            let mut values = data
                .placements
                .iter()
                .filter(|p| p.parent == Some(site) && p.field == lexical::SyntaxField::Value);
            if let Some(value) = values.next() {
                if values.next().is_some() {
                    return Ok(Err(ObligationKind::MissingEvidence));
                }
                let Some((_, safety)) = body
                    .release_inputs()
                    .iter()
                    .find(|(expression, _)| *expression == value.occurrence)
                else {
                    return Ok(Err(ObligationKind::MissingEvidence));
                };
                release = *safety;
                if release == ReleaseSafety::CallerRetained
                    && !captures.iter().any(|c| c.row.read == value.occurrence)
                {
                    let row = base
                        .evaluations
                        .iter()
                        .find(|row| {
                            row.expression == value.occurrence
                                && row.owner == header.callee()
                                && base.invocations.get(row.invocation).is_some_and(|frame| {
                                    (frame.input, frame.context) == (h.input, h.context)
                                })
                        })
                        .ok_or_else(|| {
                            ModelError::Invalid("returned formal evaluation absent".into())
                        })?;
                    let formal = held_formal(base, row, evaluations)?.ok_or_else(|| {
                        ModelError::Invalid("returned formal holder absent".into())
                    })?;
                    let argument = arguments
                        .iter()
                        .find(|argument| argument.formal == formal)
                        .ok_or_else(|| {
                            ModelError::Invalid("returned formal actual absent".into())
                        })?;
                    let row = base.evaluations.get(argument.evaluation).ok_or_else(|| {
                        ModelError::Invalid("returned actual proof absent".into())
                    })?;
                    release = checked_value(base, row, evaluations)?.release();
                }
            }
        }
        Ok(Ok(Self {
            event: h.event,
            callee: b.callee,
            qualification: header.qualification(),
            outcome,
            status,
            arguments,
            release,
            _charge: charge,
        }))
    }
}
fn checked_value(base: &super::records::BaseCheck, row: &super::records::ExpressionEvaluation,
    values: Option<&super::production::ProducedEvaluations>,
) -> Result<std::sync::Arc<super::evaluation::CheckedEvaluation>, ModelError> {
    match values {
        Some(values) => values.get(row, base.invocations.get(row.invocation)
            .ok_or(ModelError::Conflict("produced argument frame absent"))?),
        None => base.replay(row).map(std::sync::Arc::new),
    }
}
fn held_formal(base: &super::records::BaseCheck, row: &super::records::ExpressionEvaluation,
    values: Option<&super::production::ProducedEvaluations>,
) -> Result<Option<Id<calls::SignatureParameter>>, ModelError> {
    match values {
        Some(values) => values.held_formal(row, base.invocations.get(row.invocation)
            .ok_or(ModelError::Conflict("produced holder frame absent"))?),
        None => base.held_formal(row),
    }
}
