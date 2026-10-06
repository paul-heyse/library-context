//! Enriched owns its caller conclusions; no output is appended to Base or SourceCall tables.
use super::{
    body::CheckedSourceBody,
    completion::CheckedCompletion,
    enriched::{EnrichedFrame, EvaluationPremise},
    records::ordered_digest,
};
use crate::domain::{
    analysis::{self, enriched_execution as publication, policy::EvidenceStatus},
    normalized::{Rows, entities::EntityRef},
    resources::ResourceBudget,
    source::Occurrence,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "execution_outcomes")]
pub enum ExecutionOutcome {
    #[model(code = 0)]
    Normal,
    #[model(code = 1)]
    Return { site: Id<Occurrence> },
    #[model(code = 2)]
    Raise {
        site: Id<Occurrence>,
        exception: super::ExactRuntimeException,
    },
    #[model(code = 3)]
    Break { site: Id<Occurrence> },
    #[model(code = 4)]
    Continue { site: Id<Occurrence> },
}
impl From<super::outcome::PendingOutcome> for ExecutionOutcome {
    fn from(outcome: super::outcome::PendingOutcome) -> Self {
        use super::outcome::PendingOutcome as P;
        match outcome {
            P::Normal => Self::Normal,
            P::Return { site } => Self::Return { site },
            P::Raise { site, exception } => Self::Raise { site, exception },
            P::Break { site } => Self::Break { site },
            P::Continue { site } => Self::Continue { site },
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "execution_sources", rule = "execution_source")]
pub enum ExecutionSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    BaseEvaluation {
        #[model(premise)]
        evaluation: Id<super::records::ExpressionEvaluation>,
    },
    #[model(code = 2)]
    SourceInvocation {
        #[model(premise)]
        invocation: Id<super::source_call_records::SourceInvocation>,
    },
    #[model(code = 3)]
    FreshHeader {
        #[model(premise)]
        header: Id<super::source_call_records::SourceCallHeader>,
    },
    #[model(code = 4)]
    ModeledCall {
        #[model(premise)]
        call: Id<super::modeled_call::ModeledCallEvaluation>,
    },
    #[model(code = 5)]
    FreshSource {
        #[model(premise)]
        call: Id<SourceExecutionInvocation>,
    },
    #[model(code = 6)]
    Definition {
        #[model(premise)]
        definition: Id<super::definition::DefinitionEvaluation>,
    },
    #[model(code = 8)]
    ContextBinding {
        #[model(premise)]
        binding: Id<super::context_binding::ContextEntryBinding>,
    },
    #[model(code = 9)]
    CapturedEntry {
        #[model(premise)]
        binding: Id<super::capture_bridge::CapturedEntryBinding>,
    },
    #[model(code = 7)]
    Context {
        #[model(premise)]
        context: Id<super::context_execution::ContextExecution>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="statement_executions",rule="ordered_statement_execution",invariant_refs=super::enriched_production::statement_invariants_refs)]
pub struct StatementExecution {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Id<EntityRef>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<ExecutionOutcome>,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
    pub entered: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="execution_members",rule="execution_member",conclusion=execution)]
pub struct ExecutionMember {
    #[model(key)]
    pub execution: Id<StatementExecution>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<ExecutionSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "execution_entered_statements")]
pub struct EnteredStatement {
    #[model(key)]
    pub execution: Id<StatementExecution>,
    #[model(key)]
    pub ordinal: i64,
    pub statement: Id<Occurrence>,
}
impl analysis::support::sealed::DerivedEvidence for StatementExecution {}
impl analysis::support::DerivedEvidence for StatementExecution {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct StatementRecords {
    pub execution: StatementExecution,
    pub outcome: ExecutionOutcome,
    pub sources: Vec<ExecutionSource>,
    pub members: Vec<ExecutionMember>,
    pub entered: Vec<EnteredStatement>,
    _charge: Box<dyn resources::Reservation>,
}
pub(crate) fn emit_statement(
    frame: &EnrichedFrame<'_>,
    proof: &CheckedCompletion,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    budget: &ResourceBudget,
) -> Result<StatementRecords, ModelError> {
    let request = proof.request();
    if invocation.definition != definition.id()
        || !super::configuration::enriched_kernel(definition)
        || invocation.subject.is_some()
        || (invocation.input, invocation.context) != (request.input, request.context)
    {
        return Err(ModelError::Invalid(
            "enriched statement changes admitted method/frame".into(),
        ));
    }
    let count = proof
        .native_premises()
        .len()
        .checked_add(proof.evaluation_facts().len())
        .and_then(|n| {
            n.checked_add(
                proof.header_premises().len()
                    + proof.definition_premises().len()
                    + proof.context_premises().len(),
            )
        })
        .ok_or_else(|| ModelError::Invalid("enriched proof count overflow".into()))?;
    let bytes = count
        .checked_mul(
            (size_of::<ExecutionSource>()
                + size_of::<ExecutionMember>()
                + size_of::<EvaluationPremise>())
                * 2,
        )
        .and_then(|n| {
            n.checked_add(proof.entered_statements().len() * size_of::<EnteredStatement>() * 2)
        })
        .ok_or_else(|| ModelError::Invalid("enriched output allowance overflow".into()))?;
    let charge = budget.reserve("enriched_statement_records", bytes)?;
    let evaluations = frame.evaluation_premises(proof)?;
    let mut sources = Vec::with_capacity(count);
    sources.extend(
        proof
            .native_premises()
            .iter()
            .map(|id| ExecutionSource::Native { premise: *id }),
    );
    sources.extend(evaluations.iter().map(|source| match source {
        EvaluationPremise::Base(id) => ExecutionSource::BaseEvaluation { evaluation: *id },
        EvaluationPremise::Source(id) => ExecutionSource::SourceInvocation { invocation: *id },
        EvaluationPremise::Modeled(id) => ExecutionSource::ModeledCall { call: *id },
        EvaluationPremise::Fresh(id) => ExecutionSource::FreshSource { call: *id },
        EvaluationPremise::ContextBinding(id) => ExecutionSource::ContextBinding { binding: *id },
        EvaluationPremise::CapturedEntry(id) => ExecutionSource::CapturedEntry { binding: *id },
    }));
    sources.extend(
        proof
            .header_premises()
            .iter()
            .map(|id| ExecutionSource::FreshHeader { header: *id }),
    );
    sources.extend(proof.definition_premises().iter().map(|definition| {
        ExecutionSource::Definition {
            definition: *definition,
        }
    }));
    sources.extend(
        proof
            .context_premises()
            .iter()
            .map(|context| ExecutionSource::Context { context: *context }),
    );
    let outcome = ExecutionOutcome::from(proof.outcome());
    let execution = StatementExecution {
        invocation: invocation.id(),
        statement: request.statement,
        owner: request.owner,
        qualification: proof.qualification(),
        outcome: outcome.id(),
        status: proof.status(),
        sources: ordered_digest("execution-sources", sources.iter().map(Record::id)),
        entered: ordered_digest(
            "execution-entered",
            proof.entered_statements().iter().copied(),
        ),
    };
    let members = sources
        .iter()
        .enumerate()
        .map(|(ordinal, row)| ExecutionMember {
            execution: execution.id(),
            ordinal: ordinal as i64,
            source: row.id(),
        })
        .collect();
    let entered = proof
        .entered_statements()
        .iter()
        .enumerate()
        .map(|(ordinal, id)| EnteredStatement {
            execution: execution.id(),
            ordinal: ordinal as i64,
            statement: *id,
        })
        .collect();
    Ok(StatementRecords {
        execution,
        outcome,
        sources,
        members,
        entered,
        _charge: charge,
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "execution_body_sources", rule = "execution_body_source")]
pub enum BodySource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Statement {
        #[model(premise)]
        execution: Id<StatementExecution>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "body_executions", rule = "ordered_body_execution", invariant_refs=body_fidelity_refs)]
pub struct BodyExecution {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub owner: Id<EntityRef>,
    pub declaration: Id<Occurrence>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<ExecutionOutcome>,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
    pub releases: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="execution_body_members",rule="execution_body_member",conclusion=body)]
pub struct BodyMember {
    #[model(key)]
    pub body: Id<BodyExecution>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<BodySource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "execution_body_release_inputs")]
pub struct BodyReleaseInput {
    #[model(key)]
    pub body: Id<BodyExecution>,
    #[model(key)]
    pub ordinal: i64,
    pub expression: Id<Occurrence>,
    pub safety: super::evaluation::ReleaseSafety,
}
impl analysis::support::sealed::DerivedEvidence for BodyExecution {}
impl analysis::support::DerivedEvidence for BodyExecution {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct BodyRecords {
    pub body: BodyExecution,
    pub outcome: ExecutionOutcome,
    pub sources: Vec<BodySource>,
    pub members: Vec<BodyMember>,
    pub releases: Vec<BodyReleaseInput>,
    _charge: Box<dyn resources::Reservation>,
}
pub(crate) fn emit_body(
    proof: &CheckedSourceBody,
    invocation: &publication::AnalysisInvocation,
    statements: &Rows<StatementExecution>,
    budget: &ResourceBudget,
) -> Result<BodyRecords, ModelError> {
    let request = proof.request();
    if (invocation.input, invocation.context) != (request.input, request.context)
        || invocation.subject.is_some()
    {
        return Err(ModelError::Invalid(
            "enriched body changes admitted frame".into(),
        ));
    }
    let count = proof
        .native_premises()
        .len()
        .checked_add(proof.entered_statements().len())
        .ok_or_else(|| ModelError::Invalid("enriched body proof count overflow".into()))?;
    let bytes = count
        .checked_mul((size_of::<BodySource>() + size_of::<BodyMember>()) * 2)
        .and_then(|n| {
            n.checked_add(proof.release_inputs().len() * size_of::<BodyReleaseInput>() * 2)
        })
        .ok_or_else(|| ModelError::Invalid("enriched body allowance overflow".into()))?;
    let charge = budget.reserve("enriched_body_records", bytes)?;
    let mut sources = Vec::with_capacity(count);
    sources.extend(
        proof
            .native_premises()
            .iter()
            .map(|id| BodySource::Native { premise: *id }),
    );
    for site in proof.entered_statements() {
        let mut rows = statements.iter().filter(|row| {
            row.invocation == invocation.id()
                && row.owner == request.callee
                && row.statement == *site
        });
        let row = rows
            .next()
            .ok_or_else(|| ModelError::Invalid("enriched body statement mapping absent".into()))?;
        if rows.next().is_some() {
            return Err(ModelError::Invalid(
                "enriched body statement mapping ambiguous".into(),
            ));
        }
        sources.push(BodySource::Statement {
            execution: row.id(),
        });
    }
    let mut digest = KeySink::new("execution-body-releases");
    for (site, safety) in proof.release_inputs() {
        site.encode(&mut digest);
        safety.encode(&mut digest);
    }
    let outcome = ExecutionOutcome::from(proof.outcome());
    let body = BodyExecution {
        invocation: invocation.id(),
        owner: request.callee,
        declaration: proof.declaration(),
        qualification: proof.qualification(),
        outcome: outcome.id(),
        status: proof.status(),
        sources: ordered_digest("execution-body-sources", sources.iter().map(Record::id)),
        releases: digest.finish(),
    };
    let members = sources
        .iter()
        .enumerate()
        .map(|(ordinal, row)| BodyMember {
            body: body.id(),
            ordinal: ordinal as i64,
            source: row.id(),
        })
        .collect();
    let releases = proof
        .release_inputs()
        .iter()
        .enumerate()
        .map(|(ordinal, (site, safety))| BodyReleaseInput {
            body: body.id(),
            ordinal: ordinal as i64,
            expression: *site,
            safety: *safety,
        })
        .collect();
    Ok(BodyRecords {
        body,
        outcome,
        sources,
        members,
        releases,
        _charge: charge,
    })
}
pub fn relations() -> Vec<Relation> {
    let mut relations = vec![
        Relation::of::<ExecutionOutcome>(),
        Relation::of::<ExecutionSource>(),
        Relation::of::<StatementExecution>(),
        Relation::of::<ExecutionMember>(),
        Relation::of::<EnteredStatement>(),
        Relation::of::<BodySource>(),
        Relation::of::<BodyExecution>(),
        Relation::of::<BodyMember>(),
        Relation::of::<BodyReleaseInput>(),
        Relation::of::<SourceExecutionInvocation>(),
        Relation::of::<SourceExecutionArgument>(),
    ];
    relations.extend(super::capture_bridge::relations());
    relations
}

/// A fresh-source call enters only after an independently completed Enriched body and
/// the shared frame release operation; this record never appends SourceCall output tables.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "source_execution_invocations",
    rule = "source_execution_invocation",
    invariant_refs=fresh_fidelity_refs
)]
pub struct SourceExecutionInvocation {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub header: Id<super::source_call_records::SourceCallHeader>,
    #[model(premise)]
    pub body: Id<BodyExecution>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<ExecutionOutcome>,
    pub status: EvidenceStatus,
    pub arguments: ContentHash,
    pub release: super::evaluation::ReleaseSafety,
}
impl analysis::support::sealed::DerivedEvidence for SourceExecutionInvocation {}
impl analysis::support::DerivedEvidence for SourceExecutionInvocation {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="source_execution_arguments",rule="source_execution_argument",conclusion=call)]
pub struct SourceExecutionArgument {
    #[model(key)]
    pub call: Id<SourceExecutionInvocation>,
    #[model(key)]
    pub ordinal: i64,
    pub formal: Id<calls::SignatureParameter>,
    pub actual: Id<Occurrence>,
    #[model(premise)]
    pub evaluation: Id<super::records::ExpressionEvaluation>,
}

pub(crate) fn body_fidelity_refs()->Vec<&'static str> {vec!["enriched_body_fidelity"]}
pub(crate) fn fresh_fidelity_refs()->Vec<&'static str> {vec!["enriched_fresh_fidelity"]}
