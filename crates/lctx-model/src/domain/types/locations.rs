//! Shared validation of the selected source locations. Expected and inferred observations
//! use separate roles, and both retain their actual source expression and provider context.
use crate::domain::{assertion::AssertionQualification, lexical::SyntaxField, normalized::Rows, source::{Occurrence, SyntaxKind}, syntax::SyntaxPlacement, types::{TypeObservation, TypeRole}, *};
pub fn invariants() -> Vec<Invariant> {
    vec![Invariant { name: "selected_type_location_shapes", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]), ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<SyntaxPlacement>(&["id"]), ValidationInput::of::<TypeObservation>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(Check { qualifications: Rows::new(budget), occurrences: Rows::new(budget), placements: Rows::new(budget), observations: Rows::new(budget) })) }]
}
struct Check { qualifications: Rows<AssertionQualification>, occurrences: Rows<Occurrence>, placements: Rows<SyntaxPlacement>, observations: Rows<TypeObservation> }
impl InvariantCheck for Check {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        match relation {
            AssertionQualification::NAME => self.qualifications.decode(batch),
            Occurrence::NAME => self.occurrences.decode(batch),
            SyntaxPlacement::NAME => self.placements.decode(batch),
            TypeObservation::NAME => self.observations.decode(batch),
            _ => Err(ModelError::Invalid("undeclared selected type location input".into())),
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for observation in self.observations.iter() {
            let expected_parent = match observation.role {
                TypeRole::AttributeBase => Some(&[SyntaxKind::ExprAttribute][..]),
                TypeRole::AssignmentValue => Some(&[SyntaxKind::StmtAssign, SyntaxKind::StmtAnnAssign][..]),
                TypeRole::ReturnExpression => Some(&[SyntaxKind::StmtReturn][..]),
                TypeRole::Expected => Some(&[SyntaxKind::ExprAttribute, SyntaxKind::StmtAssign, SyntaxKind::StmtAnnAssign, SyntaxKind::StmtReturn][..]),
                _ => None,
            };
            let Some(expected_parent) = expected_parent else { continue; };
            let qualification = self.qualifications.get(observation.qualification).ok_or_else(|| ModelError::Invalid("located type qualification absent".into()))?;
            let source = self.occurrences.get(observation.subject).ok_or_else(|| ModelError::Invalid("located type source absent".into()))?;
            if observation.declared || !self.placements.iter().any(|p| p.occurrence == observation.subject && p.field == SyntaxField::Value
                && self.qualifications.get(p.qualification).is_some_and(|q| q.context == qualification.context)
                && p.parent.and_then(|id| self.occurrences.get(id)).is_some_and(|parent| parent.source == source.source && expected_parent.contains(&parent.syntax_kind))) {
                return Err(ModelError::Invalid("selected located type has wrong source role or context".into()));
            }
        }
        Ok(())
    }
}
