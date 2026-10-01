use super::invalid;
use crate::domain::{
    attribution::AnalysisContext, input::InputRevision, normalized::entities::EntityRef,
    obligation::ObligationKind, *,
};
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
    pub model_catalog: Option<ContentHash>,
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
fn validate_definition(row:&AnalysisDefinition)->Result<(),ModelError> {
    if matches!(row.method,AnalysisMethod::Communities|AnalysisMethod::PageRank|AnalysisMethod::Neighbours) && row.interpretation!=Interpretation::Heuristic {return Err(invalid("statistical method requires heuristic interpretation"));}
    if matches!(row.method,AnalysisMethod::Concepts|AnalysisMethod::RelationalConcepts) && row.interpretation!=Interpretation::ExactUnderContext {return Err(invalid("concept method requires its exact declared context"));}
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_invocations", invariants = invocation_invariants)]
pub struct AnalysisInvocation {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub definition: Id<AnalysisDefinition>,
    #[model(key)]
    pub subject: Option<Id<EntityRef>>,
    /// Exact typed parent membership is validated; its digest participates in invocation identity.
    #[model(key)]
    pub inputs: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_inputs", rule = "analysis_input", conclusion = invocation)]
pub struct AnalysisInput {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key, premise)]
    pub parent: Id<AnalysisInvocation>,
}
impl AnalysisInvocation {
    pub fn new(
        input: Id<InputRevision>,
        context: Id<AnalysisContext>,
        definition: Id<AnalysisDefinition>,
        subject: Option<Id<EntityRef>>,
        parents: impl IntoIterator<Item = Id<Self>>,
    ) -> (Self, Vec<AnalysisInput>) {
        let parents = parents
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let row = Self {
            input,
            context,
            definition,
            subject,
            inputs: parent_digest(&parents),
        };
        let inputs = parents
            .into_iter()
            .map(|parent| AnalysisInput {
                invocation: row.id(),
                parent,
            })
            .collect();
        (row, inputs)
    }
}
fn parent_digest(parents: &std::collections::BTreeSet<Id<AnalysisInvocation>>) -> ContentHash {
    let mut sink = KeySink::new("analysis-invocation-inputs");
    for parent in parents {
        parent.encode(&mut sink);
    }
    sink.finish()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisStatus {
    Completed = 0,
    Partial = 1,
    Unavailable = 2,
    NotRequested = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_outcomes", validate = validate_outcome)]
pub struct AnalysisOutcome {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    pub status: AnalysisStatus,
    pub reason: Option<ObligationKind>,
}
fn validate_outcome(row: &AnalysisOutcome) -> Result<(), ModelError> {
    if match row.status {
        AnalysisStatus::Completed => row.reason.is_some(),
        AnalysisStatus::Partial | AnalysisStatus::Unavailable => {
            row.reason.is_none() || row.reason == Some(ObligationKind::NotRequested)
        }
        AnalysisStatus::NotRequested => row.reason != Some(ObligationKind::NotRequested),
    } {
        return Err(invalid("analysis outcome disagrees with boundary"));
    }
    Ok(())
}
/// Measurement payloads never distinguish the semantic invocation or outcome.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_diagnostics", validate = validate_diagnostic)]
pub struct AnalysisDiagnostic {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    pub elapsed_micros: Option<i64>,
    pub iterations: Option<i64>,
    pub examined_members: Option<i64>,
    pub residual: Option<FiniteF64>,
    pub converged: Option<bool>,
}
fn validate_diagnostic(row: &AnalysisDiagnostic) -> Result<(), ModelError> {
    if [row.elapsed_micros, row.iterations, row.examined_members]
        .into_iter()
        .flatten()
        .any(|v| v < 0)
    {
        return Err(invalid("negative analysis diagnostic"));
    }
    Ok(())
}
fn invocation_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "analysis_invocation_inputs",
        inputs: vec![
            ValidationInput::of::<AnalysisInvocation>(&["id"]),
            ValidationInput::of::<AnalysisInput>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InvocationCheck {
                charge: charged::StateCharge::new(budget, "analysis_invocation_inputs"),
                rows: Default::default(),
                edges: Default::default(),
            })
        }),
    }]
}
struct InvocationCheck {
    charge: charged::StateCharge,
    rows: charged::ChargedMap<Id<AnalysisInvocation>, AnalysisInvocation>,
    edges: charged::ChargedMap<
        Id<AnalysisInvocation>,
        std::collections::BTreeSet<Id<AnalysisInvocation>>,
    >,
}
impl InvariantCheck for InvocationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == AnalysisInvocation::NAME {
            for row in AnalysisInvocation::decode(batch)? {
                if self.rows.insert(&mut self.charge, row.id(), row)?.is_some() {
                    return Err(ModelError::Conflict(AnalysisInvocation::NAME));
                }
            }
        } else if relation == AnalysisInput::NAME {
            for row in AnalysisInput::decode(batch)? {
                let invocation = self
                    .rows
                    .get(&row.invocation)
                    .ok_or_else(|| invalid("analysis input invocation absent"))?;
                let parent = self
                    .rows
                    .get(&row.parent)
                    .ok_or_else(|| invalid("analysis parent absent"))?;
                if row.invocation == row.parent
                    || (invocation.input, invocation.context) != (parent.input, parent.context)
                {
                    return Err(invalid("analysis parent crosses input/context or self"));
                }
                if !self
                    .edges
                    .update(&mut self.charge, row.invocation, |edges| {
                        edges.insert(row.parent)
                    })?
                {
                    return Err(invalid("duplicate analysis parent"));
                }
            }
        } else {
            return Err(invalid("undeclared analysis invocation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let empty = std::collections::BTreeSet::new();
        for (id, row) in self.rows.iter() {
            if parent_digest(self.edges.get(id).unwrap_or(&empty)) != row.inputs {
                return Err(invalid(
                    "analysis input membership differs from invocation identity",
                ));
            }
        }
        // Global declared derivation validation owns cross-relation cycle checking.
        Ok(())
    }
}
