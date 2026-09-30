//! Attachment of provider events to occurrences (ADR-0089). Only an exact span match attaches;
//! every other outcome goes back to the provider with its candidates, to be disclosed as a subject
//! boundary rather than guessed.
use std::sync::Arc;
use lctx_model::domain::{Batch, Id, ModelError, attachment::{Attachment, AttachmentBudget, AttachmentQuery, AttachmentResult, OccurrenceIndex},
    resources::ResourceBudget, source::Occurrence};

/// An attachment index over a stage's handed-off occurrences.
#[derive(Debug)]
pub struct Attacher { index: OccurrenceIndex, budget: AttachmentBudget }
/// An exact attachment, or the unattached outcome (innermost, ambiguous, unmatched or over budget)
/// with its reserved candidates.
#[derive(Debug)]
pub enum Attached { Exact(Id<Occurrence>), Unattached(AttachmentResult) }
impl Attacher {
    pub fn new(occurrences: &[Arc<Batch<Occurrence>>], resources: ResourceBudget) -> Result<Self, ModelError> {
        let parts: Vec<&[Occurrence]> = occurrences.iter().map(|batch| batch.rows()).collect();
        Ok(Self { index: OccurrenceIndex::from_parts(&parts, resources)?, budget: AttachmentBudget::default() })
    }
    pub fn with_budget(self, budget: AttachmentBudget) -> Self { Self { budget, ..self } }
    pub fn attach(&self, query: &AttachmentQuery) -> Result<Attached, ModelError> {
        let result = self.index.attach(query, self.budget)?;
        Ok(match result.value() { Attachment::Exact(id) => Attached::Exact(*id), _ => Attached::Unattached(result) })
    }
}
