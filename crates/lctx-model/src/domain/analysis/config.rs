use super::invalid;
use crate::domain::*;
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisMethod {
    LocalTransfers = 0,
    Execution = 1,
    Completion = 2,
    Models = 3,
    Summaries = 4,
    Delegation = 5,
    Controls = 6,
    Handoffs = 7,
    DirectUsage = 8,
    SeedSelection = 9,
    Communities = 10,
    PageRank = 11,
    Concepts = 12,
    RelationalConcepts = 13,
    Neighbours = 14,
    Catalog = 15,
    Synthesis = 16,
    Retrieval = 17,
    Dispatch = 18,
    AnalyticEmbedding = 19,
    CatalogEvidence = 20,
    CatalogSelection = 21,
    SourceCalls = 22,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Interpretation {
    Structural = 0,
    ExactUnderContext = 1,
    Heuristic = 2,
}
/// All parameters are typed, identity-bearing inputs. Diagnostics never feed this definition.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "method_parameters", validate = validate_parameters)]
pub struct MethodParameters {
    #[model(key)]
    pub depth: Option<i64>,
    #[model(key)]
    pub proof_steps: Option<i64>,
    #[model(key)]
    pub work: Option<i64>,
    #[model(key)]
    pub members: Option<i64>,
    #[model(key)]
    pub seed: Option<i64>,
    #[model(key)]
    pub iterations: Option<i64>,
    #[model(key)]
    pub threshold: Option<FiniteF64>,
    #[model(key)]
    pub resolution: Option<FiniteF64>,
    #[model(key)]
    pub damping: Option<FiniteF64>,
    #[model(key)]
    pub model_catalog: Option<Id<crate::domain::models::ModelCatalog>>,
}
fn validate_parameters(row: &MethodParameters) -> Result<(), ModelError> {
    if [
        row.depth,
        row.proof_steps,
        row.work,
        row.members,
        row.iterations,
    ]
    .into_iter()
    .flatten()
    .any(|v| v < 0)
    {
        return Err(invalid("analysis limits must be nonnegative"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_definitions", validate = validate_definition)]
pub struct AnalysisDefinition {
    #[model(key)]
    pub method: AnalysisMethod,
    #[model(key)]
    pub semantic_version: ContentHash,
    #[model(key)]
    pub parameters: Id<MethodParameters>,
    #[model(key)]
    pub interpretation: Interpretation,
}
fn validate_definition(row: &AnalysisDefinition) -> Result<(), ModelError> {
    if matches!(
        row.method,
        AnalysisMethod::Communities
            | AnalysisMethod::PageRank
            | AnalysisMethod::Neighbours
            | AnalysisMethod::AnalyticEmbedding
    ) && row.interpretation != Interpretation::Heuristic
    {
        return Err(invalid(
            "statistical method requires heuristic interpretation",
        ));
    }
    if matches!(
        row.method,
        AnalysisMethod::Concepts | AnalysisMethod::RelationalConcepts
    ) && row.interpretation != Interpretation::ExactUnderContext
    {
        return Err(invalid(
            "concept method requires its exact declared context",
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisStatus {
    Completed = 0,
    Partial = 1,
    Unavailable = 2,
    NotRequested = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisCapability {
    Transfers = 0,
    ControlInfluence = 1,
    Execution = 2,
    Completion = 3,
    Models = 4,
    Summaries = 5,
    Delegation = 6,
    Controls = 7,
    Handoffs = 8,
    DirectUsage = 9,
    SeedSelection = 10,
    Communities = 11,
    PageRank = 12,
    Concepts = 13,
    RelationalConcepts = 14,
    Neighbours = 15,
    Catalog = 16,
    Synthesis = 17,
    Retrieval = 18,
    AnalyticEmbedding = 19,
    CatalogEvidence = 20,
    CatalogSelection = 21,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisChannel {
    Value = 0,
    Effect = 1,
    Exception = 2,
    Role = 3,
    Execution = 4,
    Completion = 5,
    Catalog = 6,
}
/// Early immutable reference to the existing projection owner's finite meaning. This contains no
/// late assessment or snapshot target, so Dispatch can use it before the normalized checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="analysis_projection_definitions",validate=validate_projection_definition)]
pub struct ProjectionDefinition {
    #[model(key)]
    pub name: projection::ProjectionName,
    #[model(key)]
    pub version: i32,
    #[model(key)]
    pub policy: ContentHash,
}
impl ProjectionDefinition {
    pub fn builtin(name: projection::ProjectionName) -> Self {
        let spec = projection::ProjectionSpec::builtin(name);
        let mut sink = KeySink::new("analysis-projection-definition");
        name.encode(&mut sink);
        projection::ProjectionSpec::VERSION.encode(&mut sink);
        format!("{:?}", spec.universe()).encode(&mut sink);
        format!("{:?}", spec.multiplicity()).encode(&mut sink);
        format!("{:?}", spec.availability()).encode(&mut sink);
        format!("{:?}", spec.roles()).encode(&mut sink);
        format!("{:?}", spec.call_policy()).encode(&mut sink);
        Self {
            name,
            version: projection::ProjectionSpec::VERSION,
            policy: sink.finish(),
        }
    }
}
fn validate_projection_definition(row: &ProjectionDefinition) -> Result<(), ModelError> {
    if *row != ProjectionDefinition::builtin(row.name) {
        return Err(invalid(
            "analysis projection differs from its authoritative built-in meaning",
        ));
    }
    Ok(())
}
