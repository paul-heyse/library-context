//! Derived analysis is a computation over typed evidence, never a native provider invocation.
pub mod coverage;
pub mod findings;
pub mod native;
pub mod policy;
mod invocation;
pub mod obligations;
pub mod support;
use crate::domain::*;
pub use invocation::*;

pub fn relations() -> Vec<Relation> {
    let mut rows = vec![
        Relation::of::<AnalysisDefinition>(),
        Relation::of::<MethodParameters>(),
        Relation::of::<AnalysisInvocation>(),
        Relation::of::<AnalysisInput>(),
        Relation::of::<AnalysisOutcome>(),
        Relation::of::<AnalysisDiagnostic>(),
    ];
    rows.extend(coverage::relations());
    rows.extend(findings::relations());
    rows.extend(obligations::relations());
    rows.extend(support::relations());
    rows
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
