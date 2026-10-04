//! One structural decorator-head and normalized binding identity selection for all consumers.
use super::{Rows, binding_normalization::BindingData, entities::*, links::*};
use crate::domain::{
    charged::{ChargedVec, StateCharge},
    resources::ResourceBudget,
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, crate::DomainCode)]
#[repr(i16)]
pub enum DecoratorSelectionStatus {
    Resolved = 0,
    MissingCorrespondence = 1,
    QualifiedUncertainty = 2,
    Ambiguous = 3,
    Unresolved = 4,
    UnsupportedTarget = 5,
}
pub struct Inputs<'a> {
    pub qualifications: &'a Rows<assertion::AssertionQualification>,
    pub occurrences: &'a Rows<source::Occurrence>,
    pub placements: &'a Rows<syntax::SyntaxPlacement>,
    pub references: &'a Rows<lexical::ReferenceObservation>,
    pub assessments: &'a Rows<ReferenceEntityAssessment>,
    pub candidates: &'a Rows<ReferenceEntityCandidate>,
    pub targets: &'a Rows<ReferenceEntityTarget>,
    pub resolutions: &'a Rows<lexical::LexicalResolution>,
}
impl<'a> From<&'a BindingData> for Inputs<'a> {
    fn from(d: &'a BindingData) -> Self {
        Self {
            qualifications: &d.qualifications,
            occurrences: &d.occurrences,
            placements: &d.placements,
            references: &d.references,
            assessments: &d.reference_assessments,
            candidates: &d.reference_candidates,
            targets: &d.reference_targets,
            resolutions: &d.lexical_resolutions,
        }
    }
}
pub struct Selection {
    pub assessment: Option<Id<ReferenceEntityAssessment>>,
    pub status: DecoratorSelectionStatus,
    pub candidate: Option<Id<ReferenceEntityCandidate>>,
    pub entity: Option<Id<EntityRef>>,
}
impl HeapSize for Selection {}
pub struct Output {
    pub selections: ChargedVec<Selection>,
    charge: StateCharge,
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| ModelError::Invalid(format!("decorator identity input absent: {}", R::NAME)))
}
impl Inputs<'_> {
    fn exact(
        &self,
        q: Id<assertion::AssertionQualification>,
        ctx: Id<attribution::AnalysisContext>,
    ) -> Result<bool, ModelError> {
        let q = need(self.qualifications, q)?;
        Ok(q.context == ctx
            && q.modality == attribution::Modality::Definite
            && q.approximation == assertion::Approximation::Exact
            && q.condition == conditions::Diagram::always().id()
            && q.assumptions == assumptions::AssumptionSet::empty().id())
    }
    pub fn select(
        &self,
        decorator: &syntax::DeclarationDecorator,
        ctx: Id<attribution::AnalysisContext>,
        budget: &ResourceBudget,
    ) -> Result<Output, ModelError> {
        use DecoratorSelectionStatus::*;
        let mut out = Output {
            selections: ChargedVec::default(),
            charge: StateCharge::new(budget, "decorator-identity"),
        };
        let retain = |out: &mut Output, assessment, status, candidate, entity| {
            out.selections.push(
                &mut out.charge,
                Selection {
                    assessment,
                    status,
                    candidate,
                    entity,
                },
            )
        };
        if !self.exact(decorator.qualification, ctx)? {
            retain(&mut out, None, QualifiedUncertainty, None, None)?;
            return Ok(out);
        }
        let mut head = decorator.decorator;
        if need(self.occurrences, head)?.syntax_kind == source::SyntaxKind::Decorator {
            let mut matches = self.placements.iter().filter(|p| {
                p.parent == Some(head)
                    && p.field == lexical::SyntaxField::Child
                    && self
                        .qualifications
                        .get(p.qualification)
                        .is_some_and(|q| q.context == ctx)
            });
            let Some(expression) = matches.next() else {
                retain(&mut out, None, MissingCorrespondence, None, None)?;
                return Ok(out);
            };
            if matches.next().is_some() {
                retain(&mut out, None, Ambiguous, None, None)?;
                return Ok(out);
            }
            if !self.exact(expression.qualification, ctx)? {
                retain(&mut out, None, QualifiedUncertainty, None, None)?;
                return Ok(out);
            }
            head = expression.occurrence;
        }
        let mut heads = self.placements.iter().filter(|p| {
            p.parent == Some(head)
                && p.field == lexical::SyntaxField::Callee
                && self
                    .qualifications
                    .get(p.qualification)
                    .is_some_and(|q| q.context == ctx)
        });
        let placement = heads.next();
        if heads.next().is_some() {
            retain(&mut out, None, Ambiguous, None, None)?;
            return Ok(out);
        }
        if let Some(p) = placement {
            if !self.exact(p.qualification, ctx)? {
                retain(&mut out, None, QualifiedUncertainty, None, None)?;
                return Ok(out);
            }
            head = p.occurrence;
        }
        let mut found = false;
        let mut qualified = false;
        for reference in self.references.iter().filter(|r| {
            r.read == head
                && self
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == ctx)
        }) {
            if !self.exact(reference.qualification, ctx)? {
                qualified = true;
                continue;
            }
            for assessment in self
                .assessments
                .iter()
                .filter(|a| a.reference == reference.id())
            {
                found = true;
                let mut candidate_exact = true;
                let mut entity = None;
                let mut ambiguous = false;
                for candidate in self
                    .candidates
                    .iter()
                    .filter(|c| c.assessment == assessment.id())
                {
                    candidate_exact &= self.exact(
                        need(self.resolutions, candidate.resolution)?.qualification,
                        ctx,
                    )?;
                    if let ReferenceEntityTarget::Binding { entity: target, .. } =
                        need(self.targets, candidate.target)?
                    {
                        if entity.is_some_and(|e| e != *target) {
                            ambiguous = true;
                        }
                        entity = Some(*target);
                    }
                }
                let status = if !candidate_exact {
                    QualifiedUncertainty
                } else {
                    match assessment.status {
                        ResolutionStatus::Ambiguous => Ambiguous,
                        ResolutionStatus::Unresolved => Unresolved,
                        ResolutionStatus::Resolved if ambiguous => Ambiguous,
                        ResolutionStatus::Resolved if entity.is_none() => UnsupportedTarget,
                        ResolutionStatus::Resolved => Resolved,
                    }
                };
                if status != Resolved {
                    retain(&mut out, Some(assessment.id()), status, None, None)?;
                    continue;
                }
                for candidate in self
                    .candidates
                    .iter()
                    .filter(|c| c.assessment == assessment.id())
                {
                    if let ReferenceEntityTarget::Binding { entity, .. } =
                        need(self.targets, candidate.target)?
                    {
                        retain(
                            &mut out,
                            Some(assessment.id()),
                            status,
                            Some(candidate.id()),
                            Some(*entity),
                        )?;
                    }
                }
            }
        }
        if qualified {
            retain(&mut out, None, QualifiedUncertainty, None, None)?;
        }
        if !found && !qualified {
            retain(&mut out, None, MissingCorrespondence, None, None)?;
        }
        Ok(out)
    }
}
