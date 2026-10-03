//! The retained lexical builtin-name operation proves normal lookup under its captured
//! structural context. It proves neither mutable state stability nor parameter entry identity.
use super::evaluation::{EvaluationData, ExpressionRequest};
use crate::domain::{
    analysis::{self, native::NativeAssertionPremise, policy::EvidenceStatus},
    lexical::*,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::*,
    *,
};
pub struct CheckedBuiltinRead {
    request: ExpressionRequest,
    name: String,
    qualification: Id<assertion::AssertionQualification>,
    status: EvidenceStatus,
    premises: [Id<NativeAssertionPremise>; 3],
    _charge: charged::StateCharge,
}
impl CheckedBuiltinRead {
    pub fn name(&self) -> &str { &self.name }
    pub fn request(&self) -> ExpressionRequest {
        self.request
    }
    pub fn qualification(&self) -> Id<assertion::AssertionQualification> {
        self.qualification
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn native_premises(&self) -> &[Id<NativeAssertionPremise>] {
        &self.premises
    }
    pub fn release(&self) -> super::evaluation::ReleaseSafety {
        super::evaluation::ReleaseSafety::CallerRetained
    }
    pub fn derive(
        data: &EvaluationData,
        request: ExpressionRequest,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        use super::evaluation::boundary;
        let mut charge = charged::StateCharge::new(budget, "lexical_builtin_read");
        charge.grow(size_of::<Self>() * 2)?;
        super::evaluation::with_completion_syntax(data, request, budget, |syntax| {
            syntax.observe(request.expression)?;
            let occurrence = data
                .occurrences
                .get(request.expression)
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if occurrence.syntax_kind != SyntaxKind::ExprName {
                return Err(boundary(ObligationKind::UnsupportedControlFlow));
            }
            let qualification = syntax.qualification(request.expression).map_err(boundary)?;
            let same = |read: Id<Occurrence>| {
                data.occurrences.get(read).is_some_and(|row| {
                    (
                        row.source,
                        row.start,
                        row.end,
                        row.syntax_kind,
                        &row.structural_path,
                    ) == (
                        occurrence.source,
                        occurrence.start,
                        occurrence.end,
                        occurrence.syntax_kind,
                        &occurrence.structural_path,
                    )
                })
            };
            let mut references = data.references.iter().filter(|row| {
                same(row.read)
                    && data
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let reference = references
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if references.next().is_some() {
                return Err(boundary(ObligationKind::AmbiguousBinding));
            }
            let mut resolutions = data.lexical_resolutions.iter().filter(|row| {
                row.read == reference.read
                    && data
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let resolution = resolutions
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if resolutions.next().is_some() || resolution.captured {
                return Err(boundary(ObligationKind::EntryValueUnknown));
            }
            let target = data
                .lexical_targets
                .get(resolution.target)
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if !matches!(target,LexicalTarget::Builtin{name,variable:false}if *name==reference.name)
            {
                return Err(boundary(ObligationKind::EntryValueUnknown));
            }
            let mut spellings = data.spellings.iter().filter(|row| {
                row.occurrence == request.expression
                    && data
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let spelling = spellings
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if spellings.next().is_some()
                || spelling.spelling != reference.name
                || spelling.qualification != qualification
            {
                return Err(boundary(ObligationKind::MissingEvidence));
            }
            let scope = data
                .lexical_scopes
                .get(reference.scope)
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            let normalized::entities::EntityRef::Callable { callable } = data
                .refs
                .get(request.owner)
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?
            else {
                return Err(boundary(ObligationKind::ScopeBoundary));
            };
            if !matches!(data.callables.get(*callable),Some(normalized::entities::CallableEntity::Source{declaration,..})if *declaration==scope.owner)
                || scope.kind != LexicalScopeKind::Function
            {
                return Err(boundary(ObligationKind::ScopeBoundary));
            }
            if data.qualifications.get(reference.qualification)
                != data.qualifications.get(qualification)
                || data.qualifications.get(resolution.qualification)
                    != data.qualifications.get(qualification)
            {
                return Err(boundary(ObligationKind::Approximation));
            }
            let mut reference_supports = data.reference_supports.iter().filter(|support| {
                support.assertion == reference.id()
                    && data.runs.get(support.run).is_some_and(|run| {
                        (run.input, run.context) == (request.input, request.context)
                    })
            });
            let reference_support = reference_supports
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if reference_supports.next().is_some() {
                return Err(boundary(ObligationKind::AmbiguousBinding));
            }
            let mut resolution_supports =
                data.lexical_resolution_supports.iter().filter(|support| {
                    support.assertion == resolution.id() && support.run == reference_support.run
                });
            let resolution_support = resolution_supports
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if resolution_supports.next().is_some() {
                return Err(boundary(ObligationKind::AmbiguousBinding));
            }
            let mut spelling_pairs=data.premises.iter().filter(|pair|matches!(pair,NativeAssertionPremise::SyntaxObservation{assertion,..}if *assertion==spelling.id()));
            let spelling_pair = spelling_pairs
                .next()
                .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
            if spelling_pairs.next().is_some() {
                return Err(boundary(ObligationKind::AmbiguousBinding));
            }
            let pairs = [
                spelling_pair.clone(),
                NativeAssertionPremise::ReferenceObservation {
                    assertion: reference.id(),
                    support: reference_support.id(),
                },
                NativeAssertionPremise::LexicalResolution {
                    assertion: resolution.id(),
                    support: resolution_support.id(),
                },
            ];
            let mut status = syntax.status();
            for pair in &pairs {
                if data.premises.get(pair.id()) != Some(pair) {
                    return Err(boundary(ObligationKind::MissingEvidence));
                }
                let mut native = data.native.iter().filter(|row| row.premise == pair.id());
                let row = native
                    .next()
                    .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
                if native.next().is_some()
                    || row.qualification != qualification
                    || analysis::policy::behavioral_support(row.status, false).is_err()
                {
                    return Err(boundary(ObligationKind::MissingEvidence));
                }
                status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [status, row.status],
                );
            }
            Ok(Self {
                request,
                name: { charge.grow(reference.name.len())?; reference.name.clone() },
                qualification,
                status,
                premises: [pairs[0].id(), pairs[1].id(), pairs[2].id()],
                _charge: charge,
            })
        })
    }
}
