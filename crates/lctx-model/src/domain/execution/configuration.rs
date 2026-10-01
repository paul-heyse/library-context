//! The bound base rule sets have fixed limits. Other authored limits/method meanings require
//! their own operation binding before activation; a producer cannot label this kernel arbitrarily.
use crate::domain::{
    analysis::{AnalysisDefinition, AnalysisMethod, Interpretation, MethodParameters},
    *,
};
/// These are Summary proof bounds; graph traversal has independent A0 limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryLimits {
    pub depth: u32,
    pub proof_steps: u32,
    pub work: u32,
    pub members: u32,
}
impl Default for SummaryLimits {
    fn default() -> Self {
        Self {
            depth: 8,
            proof_steps: 64,
            work: 1 << 20,
            members: 1 << 16,
        }
    }
}
pub fn summaries(
    catalog: Id<models::ModelCatalog>,
    limits: SummaryLimits,
) -> Result<(MethodParameters, AnalysisDefinition), ModelError> {
    if limits.proof_steps == 0 || limits.work == 0 || limits.members == 0 {
        return Err(ModelError::Invalid(
            "Summary proof/work/member bounds must be positive".into(),
        ));
    }
    let parameters = MethodParameters {
        depth: Some(i64::from(limits.depth)),
        proof_steps: Some(i64::from(limits.proof_steps)),
        work: Some(i64::from(limits.work)),
        members: Some(i64::from(limits.members)),
        model_catalog: Some(catalog),
        ..parameters()
    };
    let mut key = KeySink::new("finite-summary-rule-set");
    ContentHash::of(include_bytes!("configuration.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("../composition.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("../transfer/witness.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_production.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_replay.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_path.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_alias.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_control.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_proof.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_worklist.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_schedule.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_consequences.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("summary_symbolic.rs")).encode(&mut key);
    enriched_execution(catalog)
        .1
        .semantic_version
        .encode(&mut key);
    let definition = AnalysisDefinition {
        method: AnalysisMethod::Summaries,
        semantic_version: key.finish(),
        parameters: parameters.id(),
        interpretation: Interpretation::ExactUnderContext,
    };
    Ok((parameters, definition))
}
fn parameters() -> MethodParameters {
    MethodParameters {
        depth: None,
        proof_steps: None,
        work: None,
        members: None,
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    }
}
pub fn base_evaluation() -> (MethodParameters, AnalysisDefinition) {
    let parameters = parameters();
    let mut key = KeySink::new("base-evaluation-rule-set");
    ContentHash::of(include_bytes!("read_channels.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("read_fields.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("read_dynamic.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("../conditions/entry.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("evaluation.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("builtin_read.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("production.rs")).encode(&mut key);
    let definition = AnalysisDefinition {
        method: AnalysisMethod::Execution,
        semantic_version: key.finish(),
        parameters: parameters.id(),
        interpretation: Interpretation::Structural,
    };
    (parameters, definition)
}
pub fn base_completion() -> (MethodParameters, AnalysisDefinition) {
    let parameters = parameters();
    let mut key = KeySink::new("base-completion-rule-set");
    ContentHash::of(include_bytes!("completion.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("completion_production.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("outcome.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("body.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("body_records.rs")).encode(&mut key);
    let definition = AnalysisDefinition {
        method: AnalysisMethod::Completion,
        semantic_version: key.finish(),
        parameters: parameters.id(),
        interpretation: Interpretation::Structural,
    };
    (parameters, definition)
}

pub fn source_calls() -> (MethodParameters, AnalysisDefinition) {
    let parameters = parameters();
    let mut key = KeySink::new("fresh-source-call-rule-set");
    ContentHash::of(include_bytes!("source_call.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("source_call_records.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("source_invocation.rs")).encode(&mut key);
    let definition = AnalysisDefinition {
        method: AnalysisMethod::SourceCalls,
        semantic_version: key.finish(),
        parameters: parameters.id(),
        interpretation: Interpretation::Structural,
    };
    (parameters, definition)
}

fn enriched_version() -> ContentHash {
    let mut key = KeySink::new("enriched-execution-rule-set");
    ContentHash::of(include_bytes!("context_execution.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("context_binding.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_context.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_construction.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("definition.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("enriched.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("enriched_records.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("modeled_call.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("enriched_production.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_application.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("../normalized/binding_normalization.rs")).encode(&mut key);
    base_evaluation().1.semantic_version.encode(&mut key);
    base_completion().1.semantic_version.encode(&mut key);
    source_calls().1.semantic_version.encode(&mut key);
    key.finish()
}
pub fn enriched_execution(
    catalog: Id<models::ModelCatalog>,
) -> (MethodParameters, AnalysisDefinition) {
    let parameters = MethodParameters {
        model_catalog: Some(catalog),
        work: Some(super::enriched::ENRICHED_WORK_LIMIT as i64),
        ..parameters()
    };
    let definition = AnalysisDefinition {
        method: AnalysisMethod::EnrichedExecution,
        semantic_version: enriched_version(),
        parameters: parameters.id(),
        interpretation: Interpretation::Structural,
    };
    (parameters, definition)
}
/// Publication additionally verifies the exact selected nominal catalog and parameters.
pub fn enriched_kernel(definition: &AnalysisDefinition) -> bool {
    definition.method == AnalysisMethod::EnrichedExecution
        && definition.interpretation == Interpretation::Structural
        && definition.semantic_version == enriched_version()
}

pub fn models(
    catalog: Id<crate::domain::models::ModelCatalog>,
) -> (MethodParameters, AnalysisDefinition) {
    let mut parameters = parameters();
    parameters.model_catalog = Some(catalog);
    let mut key = KeySink::new("authored-model-rule-set");
    ContentHash::of(include_bytes!("model_application.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_context.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_construction.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_transfer.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_protocol.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_context_transfer.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_rules.rs")).encode(&mut key);
    ContentHash::of(include_bytes!("model_production.rs")).encode(&mut key);
    let definition = AnalysisDefinition {
        method: AnalysisMethod::Models,
        semantic_version: key.finish(),
        parameters: parameters.id(),
        interpretation: Interpretation::Structural,
    };
    (parameters, definition)
}
