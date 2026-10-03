//! Attachment of provider events to occurrences (ADR-0089). Only an exact span match attaches;
//! every other outcome goes back to the provider with its candidates, to be disclosed as a subject
//! boundary rather than guessed.
use crate::bundle::{Declared, ProviderStage, StageContext};
use lctx_model::domain::{
    Batch, ContentHash, Id, ModelError, Record,
    attachment::{
        Attachment, AttachmentBudget, AttachmentQuery, AttachmentResult, OccurrenceIndex,
    },
    resources::ResourceBudget,
    source::Occurrence,
    stages::{Effect, Profile, ProviderOutcome, Stage, StageSink},
};
use std::sync::Arc;

/// An attachment index over a stage's handed-off occurrences.
#[derive(Debug)]
pub struct Attacher {
    index: OccurrenceIndex,
    budget: AttachmentBudget,
}
/// An exact attachment, or the unattached outcome (innermost, ambiguous, unmatched or over budget)
/// with its reserved candidates.
#[derive(Debug)]
pub enum Attached {
    Exact(Id<Occurrence>),
    Unattached(AttachmentResult),
}
impl Attacher {
    pub fn new(
        occurrences: &[Arc<Batch<Occurrence>>],
        resources: ResourceBudget,
    ) -> Result<Self, ModelError> {
        let parts: Vec<&[Occurrence]> = occurrences.iter().map(|batch| batch.rows()).collect();
        Ok(Self {
            index: OccurrenceIndex::from_parts(&parts, resources)?,
            budget: AttachmentBudget::default(),
        })
    }
    pub fn with_budget(self, budget: AttachmentBudget) -> Self {
        Self { budget, ..self }
    }
    pub fn attach(&self, query: &AttachmentQuery) -> Result<Attached, ModelError> {
        let result = self.index.attach(query, self.budget)?;
        Ok(match result.value() {
            Attachment::Exact(id) => Attached::Exact(*id),
            _ => Attached::Unattached(result),
        })
    }
}

pub const ASSEMBLE: &str = "assemble";
/// The `assemble` stage: the single writer of the vocabulary providers contribute (ADR-0089). Its
/// output writers merge every contribution and emit each identity once.
pub struct Assemble;
impl Declared for Assemble {
    fn declaration(&self, _: Profile) -> Stage {
        Stage {
            name: ASSEMBLE,
            inputs: vec![],
            outputs: vocabulary(),
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Pure,
            code: ContentHash::of(include_str!("assembly.rs").as_bytes()),
            configuration: ContentHash::of(b"assemble"),
        }
    }
}
impl<S: StageSink + 'static> ProviderStage<S> for Assemble {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        use lctx_model::domain::{
            assertion::*, assumptions::*, attribution::*, conditions::*, source::CoverageScope, syntax::*, value::*,
        };
        macro_rules! declare { ($($ty:ty),+) => { $( context.declare::<$ty>()?; )+ }; }
        declare!(
            Evidence,
            Literal,
            LiteralSet,
            LiteralSetMember,
            SyntaxDetail,
            Assumption,
            AssumptionUniverse,
            AssumptionSet,
            AssumptionSetMember,
            AssertionQualification,
            Condition,
            ConditionNode,
            Provider,
            AnalysisContext,
            ProviderRun,
            RunFamily,
            ProviderSurface,
            CoverageScope,
            ProviderCoverage,
            SubjectBoundary,
            AttachmentOutcome,
            AttachmentCandidate,
            PlaceRoot,
            PathSegment,
            AccessPath,
            Place,
            Predicate,
            EvaluationAtom
        );
        context.emit(AssumptionSet::empty())?;
        let captured = context.captured();
        let acquisition = crate::acquisition::acquisition_provider();
        context.emit(acquisition.clone())?;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                crate::acquisition::Acquisition::Corpus { library, .. } => {
                    captured.inputs().get(*library)
                }
                _ => None,
            };
            let analysis =
                crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
            let scope = CoverageScope::Input {
                input: input.captured().revision().id(),
            };
            let (run, families) = ProviderRun::new(
                acquisition.id(),
                analysis.id(),
                input.captured().revision().id(),
                analysis.config_digest,
                [FactFamily::Artifacts],
            )?;
            context.emit(analysis.clone())?;
            context.emit(scope.clone())?;
            context.emit(run.clone())?;
            for family in families {
                context.emit(family)?;
            }
            context.emit(ProviderCoverage {
                scope: scope.id(),
                provider: Some(acquisition.id()),
                context: analysis.id(),
                family: FactFamily::Artifacts,
                run: Some(run.id()),
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            })?;
            if context.profile() == Profile::Catalog {
                for artifact in crate::pyrefly_stage::roots(input)? {
                    let scope = CoverageScope::Artifact {
                        artifact: artifact.id(),
                    };
                    context.emit(scope.clone())?;
                    context.emit(ProviderCoverage {
                        scope: scope.id(),
                        provider: None,
                        context: analysis.id(),
                        family: FactFamily::Flow,
                        run: None,
                        status: CoverageStatus::NotRequested,
                        reason: None,
                        diagnostic: Some("catalog profile".into()),
                    })?;
                }
            }
        }
        Ok(ProviderOutcome::Complete)
    }
}

/// Shared vocabulary has one owner; providers declare the subset they contribute.
pub fn vocabulary() -> Vec<lctx_model::domain::stages::RelationUse> {
    use lctx_model::domain::stages::RelationUse;
    use lctx_model::domain::{
        assertion::*, assumptions::*, attribution::*, conditions::*, source::CoverageScope, syntax::*, value::*,
    };
    macro_rules! uses { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    uses!(
        Evidence,
        Literal,
        LiteralSet,
        LiteralSetMember,
        SyntaxDetail,
        Assumption,
        AssumptionUniverse,
        AssumptionSet,
        AssumptionSetMember,
        AssertionQualification,
        Condition,
        ConditionNode,
        Provider,
        AnalysisContext,
        ProviderRun,
        RunFamily,
        ProviderSurface,
        CoverageScope,
        ProviderCoverage,
        SubjectBoundary,
        AttachmentOutcome,
        AttachmentCandidate,
        PlaceRoot,
        PathSegment,
        AccessPath,
        Place,
        Predicate,
        EvaluationAtom
    )
}
