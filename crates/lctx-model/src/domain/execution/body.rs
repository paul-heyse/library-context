//! A source body outcome under entry consumes independent base statement outcomes. It never
//! establishes callee object availability, default availability or caller continuation.
use super::{
    completion::CheckedCompletion,
    evaluation::{self, EvaluationData, ExpressionRequest, boundary},
    outcome::PendingOutcome,
};
use crate::domain::{
    analysis::{self, native::NativeAssertionPremise, policy::EvidenceStatus},
    assertion::AssertionQualification,
    lexical::SyntaxField,
    normalized::entities::{CallableEntity, CallableKind, EntityRef},
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceBodyRequest {
    pub input: Id<input::InputRevision>,
    pub context: Id<attribution::AnalysisContext>,
    pub callee: Id<EntityRef>,
}
pub struct CheckedSourceBody {
    request: SourceBodyRequest,
    declaration: Id<Occurrence>,
    outcome: PendingOutcome,
    native: Vec<Id<NativeAssertionPremise>>,
    statements: Vec<Id<Occurrence>>,
    qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
    releases: Vec<(Id<Occurrence>, evaluation::ReleaseSafety)>,
    _native_charge: charged::StateCharge,
    _charge: charged::StateCharge,
}
/// The actual body's scalar result. Ordered proof payloads live in its acknowledged records.
#[derive(Clone, Copy)]
pub(crate) struct ProducedBodyValue {
    request: SourceBodyRequest,
    declaration: Id<Occurrence>,
    outcome: PendingOutcome,
    qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
}
impl ProducedBodyValue {
    pub(crate) fn of(proof: &CheckedSourceBody) -> Self {
        Self {
            request: proof.request,
            declaration: proof.declaration,
            outcome: proof.outcome,
            qualification: proof.qualification,
            status: proof.status,
        }
    }
    pub(crate) fn hydrate(
        self,
        native: Vec<Id<NativeAssertionPremise>>,
        statements: Vec<Id<Occurrence>>,
        releases: Vec<(Id<Occurrence>, evaluation::ReleaseSafety)>,
        budget: &ResourceBudget,
    ) -> Result<CheckedSourceBody, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "actual-body-hydration");
        charge.grow(
            native.capacity() * size_of::<Id<NativeAssertionPremise>>()
                + statements.capacity() * size_of::<Id<Occurrence>>()
                + releases.capacity() * size_of::<(Id<Occurrence>, evaluation::ReleaseSafety)>(),
        )?;
        Ok(CheckedSourceBody {
            request: self.request,
            declaration: self.declaration,
            outcome: self.outcome,
            native,
            statements,
            qualification: self.qualification,
            status: self.status,
            releases,
            _native_charge: charged::StateCharge::new(budget, "actual-body-native"),
            _charge: charge,
        })
    }
}
impl CheckedSourceBody {
    pub fn request(&self) -> SourceBodyRequest {
        self.request
    }
    pub fn declaration(&self) -> Id<Occurrence> {
        self.declaration
    }
    pub fn outcome(&self) -> PendingOutcome {
        self.outcome
    }
    pub fn native_premises(&self) -> &[Id<NativeAssertionPremise>] {
        &self.native
    }
    pub fn entered_statements(&self) -> &[Id<Occurrence>] {
        &self.statements
    }
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn release_inputs(&self) -> &[(Id<Occurrence>, evaluation::ReleaseSafety)] {
        &self.releases
    }
    /// Mandatory even when all evaluated values are harmless. A caller must prove the exact
    /// callable and formal argument objects remain externally held during frame teardown.
    pub fn frame_obligation(&self) -> ObligationKind {
        ObligationKind::FrameExitCleanup
    }
}
pub fn complete_body(
    data: &EvaluationData,
    request: SourceBodyRequest,
    statements: &[&CheckedCompletion],
    budget: &ResourceBudget,
) -> Result<Result<CheckedSourceBody, ObligationKind>, ModelError> {
    let missing = || Ok(Err(ObligationKind::MissingEvidence));
    let Some(EntityRef::Callable { callable }) = data.refs.get(request.callee) else {
        return missing();
    };
    let Some(CallableEntity::Source {
        declaration,
        kind: CallableKind::Function,
    }) = data.callables.get(*callable)
    else {
        return Ok(Err(ObligationKind::ScopeBoundary));
    };
    let declaration = *declaration;
    if data
        .occurrences
        .get(declaration)
        .is_none_or(|row| row.syntax_kind != SyntaxKind::StmtFunctionDef)
    {
        return missing();
    }
    evaluation::with_completion_syntax(
        data,
        ExpressionRequest {
            input: request.input,
            context: request.context,
            owner: request.callee,
            expression: declaration,
        },
        budget,
        |syntax| {
            syntax.observe_callable(declaration)?;
            let children = syntax.children(declaration)?;
            let mut charge = charged::StateCharge::new(budget, "source_body_results");
            charge.grow(
                children
                    .len()
                    .checked_mul(size_of::<&crate::domain::syntax::SyntaxPlacement>() * 2)
                    .ok_or_else(|| {
                        ModelError::Invalid("source body suite allowance overflow".into())
                    })?,
            )?;
            let mut suite = children
                .iter()
                .filter(|row| row.field == SyntaxField::Body)
                .collect::<Vec<_>>();
            suite.sort_by_key(|row| row.ordinal);
            if suite.is_empty()
                || suite
                    .iter()
                    .enumerate()
                    .any(|(ordinal, row)| row.ordinal != ordinal as i64)
            {
                return Err(boundary(ObligationKind::MissingEvidence));
            }
            let mut entered = Vec::new();
            let mut releases = Vec::new();
            let mut status = syntax.status();
            let mut outcome = PendingOutcome::Normal;
            for row in suite {
                syntax.tick(statements.len()).map_err(boundary)?;
                let mut candidates = statements
                    .iter()
                    .filter(|proof| proof.request().statement == row.occurrence);
                let proof = *candidates
                    .next()
                    .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
                let frame = proof.request();
                if candidates.next().is_some()
                    || frame.owner != request.callee
                    || frame.input != request.input
                    || frame.context != request.context
                {
                    return Err(boundary(ObligationKind::IncompatibleContexts));
                }
                if entered.len() == 64
                    || releases
                        .len()
                        .checked_add(proof.release_inputs().len())
                        .is_none_or(|n| n > 64)
                {
                    return Err(boundary(ObligationKind::SummaryProofLimit));
                }
                charge.grow(
                    size_of::<Id<Occurrence>>() * 2
                        + std::mem::size_of_val(proof.release_inputs()) * 2,
                )?;
                entered.push(row.occurrence);
                releases.extend_from_slice(proof.release_inputs());
                status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [status, proof.status()],
                );
                outcome = proof.outcome();
                if !outcome.is_normal() {
                    break;
                }
            }
            if matches!(
                outcome,
                PendingOutcome::Break { .. } | PendingOutcome::Continue { .. }
            ) {
                return Err(boundary(ObligationKind::UnsupportedControlFlow));
            }
            let qualification = syntax.qualification(declaration).map_err(boundary)?;
            let (native, native_charge) = syntax.take_admission();
            Ok(CheckedSourceBody {
                request,
                declaration,
                outcome,
                native,
                statements: entered,
                qualification,
                status,
                releases,
                _native_charge: native_charge,
                _charge: charge,
            })
        },
    )
}
