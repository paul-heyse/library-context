//! Acyclic preparation for the first fresh source-call domain. Base body contracts use only
//! pinned expressions; source calls enrich caller expressions once, never their own premises.
pub struct Outcome {
    pub expressions: crate::evaluation::EvaluationOutcome,
    pub completions: crate::completion::Outcome,
}
pub fn prepare(
    expressions: crate::evaluation::EvaluationInputs<'_>,
    completion: crate::completion::Inputs<'_>,
    targets: crate::source_call::Targets<'_>,
) -> Outcome {
    let base = crate::evaluation::evaluate(crate::evaluation::EvaluationInputs {
        source_bindings: &[],
        source_invocations: &[],
        source_calls: &[],
        source_refusals: &[],
        ..expressions
    });
    let base_input = crate::completion::Inputs {
        expressions: &base.evaluations,
        expression_steps: &base.steps,
        invocations: &base.invocations,
        model_frames: &base.frames,
        source_bindings: &[],
        source_calls: &[],
        ..completion
    };
    let base_completion = crate::completion::complete(base_input);
    let source = crate::source_call::prepare(targets, &base_input, &base_completion);
    let enriched = crate::evaluation::evaluate(crate::evaluation::EvaluationInputs {
        source_bindings: &source.bindings,
        source_invocations: &source.invocations,
        source_calls: &source.normals,
        source_refusals: &source.refusals,
        ..expressions
    });
    let mut completions = crate::completion::complete(crate::completion::Inputs {
        expressions: &enriched.evaluations,
        expression_steps: &enriched.steps,
        invocations: &enriched.invocations,
        model_frames: &enriched.frames,
        source_bindings: &source.bindings,
        source_calls: &source.normals,
        ..completion
    });
    // This is an explicit base-body stage, not an accidental two-iteration fixed point.
    completions.bodies = base_completion.bodies;
    completions.body_steps = base_completion.body_steps;
    completions.body_releases = base_completion.body_releases;
    completions.source_bindings = source.bindings;
    completions.source_calls = source.normals;
    completions.source_call_headers = source.headers;
    Outcome {
        expressions: enriched,
        completions,
    }
}
