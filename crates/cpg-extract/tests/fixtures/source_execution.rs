//! Actual base operation outputs for retained source controls.
#[path = "source_data.rs"]
mod source_data;
pub use source_data::data;
use lctx_model::domain::{analysis, execution, normalized::Rows, *};
pub fn base_rows_with_entries(
    data: &mut execution::source_call_records::SourceCallData,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
    budget: &resources::ResourceBudget,
    entries: &Rows<conditions::entry::EntryValueWitness>,
    sources: &Rows<conditions::entry::EntryAccessSource>,
) -> (
    analysis::base_completion::AnalysisInvocation,
    Vec<(&'static str, arrow_array::RecordBatch)>,
) {
    let mut batches = Vec::new();
    let (_, definition) = execution::configuration::base_evaluation();
    let (invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        None,
        [],
    );
    let output = execution::production::evaluate_all(
        &data.evaluation,
        &data.flow,
        entries,
        sources,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        budget,
    )
    .unwrap();
    macro_rules! put {
        ($ty:ty,$rows:expr) => {{
            let batch = <$ty as Record>::encode(($rows).as_ref()).unwrap();
            data.visit(<$ty>::NAME, &batch).unwrap();
            batches.push((<$ty>::NAME, batch));
        }};
    }
    put!(
        conditions::entry::EntryValueWitness,
        entries.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        conditions::entry::EntryAccessSource,
        sources.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        analysis::base_evaluation::AnalysisInvocation,
        [invocation]
    );
    put!(analysis::AnalysisDefinition, [definition]);
    put!(
        execution::records::ExpressionEvaluation,
        output.evaluations.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::records::EvaluationSource,
        output.sources.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::records::EvaluationMember,
        output.members.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::records::EvaluationOperand,
        output.operands.iter().cloned().collect::<Vec<_>>()
    );
    let (_, definition) = execution::configuration::base_completion();
    let (invocation, _) = analysis::base_completion::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        None,
        [],
    );
    let output = execution::completion_production::complete_all(
        &data.completed,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        budget,
    )
    .unwrap();
    put!(
        analysis::base_completion::AnalysisInvocation,
        std::slice::from_ref(&invocation)
    );
    put!(analysis::AnalysisDefinition, [definition]);
    put!(
        execution::body_records::SourceBodyCompletion,
        output.bodies.iter().cloned().collect::<Vec<_>>()
    );
    (invocation, batches)
}
