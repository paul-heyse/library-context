//! The stage-bound production subset (plan E1): the capture and syntax stages over any
//! [`StageSink`]. Their schedule is not admissible to the facts frontier. It drives the permanent
//! producers through the production path into conformance generations or memory.
use lctx_model::domain::{ContentHash, ModelError, Record, ValidatedModel, artifact::ArtifactChunk, assertion::*, attribution::*,
    batching::{BatchWriter, TransferLimits}, conditions::{Condition, ConditionNode}, input::*, resources::ResourceBudget, source::*, stages::*};
use crate::{capture::{CaptureError, CapturedInput}, typed_syntax::{SyntaxFacts, SyntaxLimits, syntax_provider}};

pub const CAPTURE: &str = "capture";
pub const SYNTAX: &str = "syntax";

/// The acquisition stage: the captured input's revision, origin, artifacts and their bytes.
pub fn capture_stage() -> Stage {
    Stage { name: CAPTURE, inputs: vec![], contributes: vec![], coverage: vec![], provider: None,
        outputs: vec![RelationUse::of::<InputRevision>(), RelationUse::of::<InputOrigin>(), RelationUse::of::<InputAcquisition>(),
            RelationUse::of::<SourceArtifact>(), RelationUse::of::<ArtifactChunk>()],
        profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Acquisition,
        code: ContentHash::of(include_str!("capture.rs").as_bytes()), configuration: ContentHash::of(b"capture") }
}
/// The syntax stage: typed occurrences and identifier observations with artifact-grain Syntax
/// coverage. Its traversal limits are configuration.
pub fn syntax_stage(limits: SyntaxLimits) -> Stage {
    let provider = syntax_provider();
    Stage { name: SYNTAX, inputs: vec![], contributes: vec![], coverage: vec![FactFamily::Syntax], provider: Some(provider.id()),
        outputs: vec![RelationUse::of::<Module>(), RelationUse::of::<Occurrence>(), RelationUse::of::<Provider>(), RelationUse::of::<AnalysisContext>(),
            RelationUse::of::<ProviderRun>(), RelationUse::of::<RunFamily>(), RelationUse::of::<ProviderSurface>(), RelationUse::of::<Condition>(),
            RelationUse::of::<ConditionNode>(), RelationUse::of::<CoverageScope>(), RelationUse::of::<AssertionQualification>(),
            RelationUse::of::<Evidence>(), RelationUse::of::<SyntaxObservation>(), RelationUse::of::<SyntaxSupport>(), RelationUse::of::<ProviderCoverage>()],
        profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Extraction,
        code: provider.build_digest, configuration: ContentHash::of(format!("{limits:?}").as_bytes()) }
}
fn capture_error(error: CaptureError) -> ModelError {
    match error { CaptureError::Model(error) => error, other => ModelError::codec(other) }
}

/// Write the captured input, streaming each artifact's chunks in reserved transfer batches, and
/// recheck the frozen bytes before the stage completes.
pub async fn run_capture<S: StageSink>(mut output: StageOutput<'_, '_, '_, S>, captured: &CapturedInput, origin: InputOrigin,
    model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
    output.declare::<InputRevision>()?; output.declare::<InputOrigin>()?; output.declare::<InputAcquisition>()?;
    output.declare::<SourceArtifact>()?; output.declare::<ArtifactChunk>()?;
    let revision = captured.revision().clone();
    output.push(InputAcquisition { input: revision.id(), origin: origin.id() }).await?;
    output.push(revision).await?; output.push(origin).await?;
    for artifact in captured.artifacts() { output.push(artifact.clone()).await?; }
    let mut chunks = BatchWriter::<ArtifactChunk>::new(budget, TransferLimits::default())?;
    for index in 0..captured.artifacts().len() {
        let mut full = Vec::new();
        captured.emit_chunks(index, |chunk| { if let Some(batch) = chunks.push(model, chunk.clone())? { full.push(batch); } Ok(()) }).map_err(capture_error)?;
        for batch in full { output.push_batch(batch).await?; }
    }
    if let Some(batch) = chunks.finish(model)? { output.push_batch(batch).await?; }
    captured.verify().map_err(capture_error)?;
    output.finish(ProviderOutcome::Complete).await
}

/// Write one input's syntax facts and their coverage; the stage outcome follows the coverage.
/// The frozen bytes are rechecked after the provider ran, before the stage completes.
pub async fn run_syntax<S: StageSink>(mut output: StageOutput<'_, '_, '_, S>, captured: &CapturedInput, facts: SyntaxFacts) -> Result<(), ModelError> {
    output.declare::<Module>()?; output.declare::<Occurrence>()?; output.declare::<Provider>()?; output.declare::<AnalysisContext>()?;
    output.declare::<ProviderRun>()?; output.declare::<RunFamily>()?; output.declare::<ProviderSurface>()?; output.declare::<Condition>()?;
    output.declare::<ConditionNode>()?; output.declare::<CoverageScope>()?; output.declare::<AssertionQualification>()?;
    output.declare::<Evidence>()?; output.declare::<SyntaxObservation>()?; output.declare::<SyntaxSupport>()?; output.declare::<ProviderCoverage>()?;
    let outcome = facts.outcome();
    let SyntaxFacts { provider, context, run, families, surface, condition, nodes, modules, scopes, qualifications, coverage,
        occurrences, observations, evidence, supports, .. } = facts;
    output.push(provider).await?; output.push(context).await?; output.push(run).await?; output.push(surface).await?; output.push(condition).await?;
    for row in families { output.push(row).await?; }
    for row in nodes { output.push(row).await?; }
    for row in modules { output.push(row).await?; }
    for row in scopes { output.push(row).await?; }
    for row in qualifications { output.push(row).await?; }
    for row in coverage { output.push(row).await?; }
    for batch in occurrences { output.push_batch(batch).await?; }
    for batch in observations { output.push_batch(batch).await?; }
    for batch in evidence { output.push_batch(batch).await?; }
    for batch in supports { output.push_batch(batch).await?; }
    captured.verify().map_err(capture_error)?;
    output.finish(outcome).await
}
