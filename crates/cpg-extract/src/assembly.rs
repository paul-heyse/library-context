//! Attachment of provider events to occurrences (ADR-0089). Only an exact span match attaches;
//! every other outcome goes back to the provider with its candidates, to be disclosed as a subject
//! boundary rather than guessed.
use std::sync::Arc;
use lctx_model::domain::{Batch, ContentHash, Id, ModelError, attachment::{Attachment, AttachmentBudget, AttachmentQuery, AttachmentResult, OccurrenceIndex},
    resources::ResourceBudget, source::Occurrence, stages::{Effect, Profile, ProviderOutcome, Stage, StageSink}};
use crate::bundle::{Declared, ProviderStage, StageContext};

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

pub const ASSEMBLE: &str = "assemble";
/// The `assemble` stage: the single writer of the vocabulary providers contribute (ADR-0089). Its
/// output writers merge every contribution and emit each identity once.
pub struct Assemble;
impl Declared for Assemble {
    fn declaration(&self, _: Profile) -> Stage {
        Stage { name: ASSEMBLE, inputs: vec![], outputs: crate::pyrefly_stage::vocabulary(), contributes: vec![], coverage: vec![], provider: None,
            profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(include_str!("assembly.rs").as_bytes()),
            configuration: ContentHash::of(b"assemble") }
    }
}
impl<S: StageSink + 'static> ProviderStage<S> for Assemble {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        use lctx_model::domain::{assertion::*, attribution::*, conditions::*, source::CoverageScope, syntax::*, value::*};
        macro_rules! declare { ($($ty:ty),+) => { $( context.declare::<$ty>()?; )+ }; }
        declare!(Evidence, Literal, LiteralSet, LiteralSetMember, SyntaxDetail, AssertionQualification, Condition, ConditionNode, Provider, AnalysisContext,
            ProviderRun, RunFamily, ProviderSurface, CoverageScope, ProviderCoverage, SubjectBoundary);
        Ok(ProviderOutcome::Complete)
    }
}
