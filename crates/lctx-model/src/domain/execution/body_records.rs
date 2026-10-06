//! Body outcomes under source-function entry. Fresh binding, invocation and frame release are
//! separate consumers; these records never establish that the callable was available at a call.
use super::{
    body::CheckedSourceBody,
    completion_records::{CompletionOutcome, StatementCompletion},
    evaluation::ReleaseSafety,
    records::ordered_digest,
};
use crate::domain::{
    analysis::{
        self, base_completion::AnalysisInvocation, native::NativeAssertionPremise,
        policy::EvidenceStatus,
    },
    normalized::{Rows, entities::EntityRef},
    resources::{Reservation, ResourceBudget},
    source::Occurrence,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "base_source_body_sources", rule = "base_source_body_source")]
pub enum BodySource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Statement {
        #[model(premise)]
        completion: Id<StatementCompletion>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="base_source_body_completions",rule="base_source_body_completion",invariant_refs=body_invariants_refs)]
pub struct SourceBodyCompletion {
    #[model(key, premise)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub owner: Id<EntityRef>,
    pub declaration: Id<Occurrence>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<CompletionOutcome>,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
    pub releases: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_source_body_members",rule="base_source_body_member",conclusion=body)]
pub struct BodyMember {
    #[model(key)]
    pub body: Id<SourceBodyCompletion>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<BodySource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_source_body_release_inputs")]
pub struct BodyReleaseInput {
    #[model(key)]
    pub body: Id<SourceBodyCompletion>,
    #[model(key)]
    pub ordinal: i64,
    pub expression: Id<Occurrence>,
    pub safety: ReleaseSafety,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_source_body_boundaries")]
pub struct BodyBoundary {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub owner: Id<EntityRef>,
    pub declaration: Id<Occurrence>,
    pub reason: obligation::ObligationKind,
}
impl analysis::support::sealed::DerivedEvidence for SourceBodyCompletion {}
impl analysis::support::DerivedEvidence for SourceBodyCompletion {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub(crate) struct BodyRecords {
    pub body: SourceBodyCompletion,
    pub outcome: CompletionOutcome,
    pub sources: Vec<BodySource>,
    pub members: Vec<BodyMember>,
    pub releases: Vec<BodyReleaseInput>,
    _charge: Box<dyn Reservation>,
}
/// Called only while the producer retains the private statement tokens that emitted `statements`.
/// Stored replay repeats that entire operation; raw rows are never a body admission route.
pub(crate) fn emit(
    proof: &CheckedSourceBody,
    invocation: &AnalysisInvocation,
    statements: &Rows<StatementCompletion>,
    budget: &ResourceBudget,
) -> Result<BodyRecords, ModelError> {
    let request = proof.request();
    let invalid = |message: &str| ModelError::Invalid(message.into());
    if invocation.input != request.input
        || invocation.context != request.context
        || invocation.subject.is_some()
    {
        return Err(invalid("source body changes its base completion frame"));
    }
    let count = proof
        .native_premises()
        .len()
        .checked_add(proof.entered_statements().len())
        .ok_or_else(|| invalid("source body premise count overflow"))?;
    let bytes = count
        .checked_mul((size_of::<BodySource>() + size_of::<BodyMember>()) * 2)
        .and_then(|n| {
            n.checked_add(
                proof
                    .release_inputs()
                    .len()
                    .checked_mul(size_of::<BodyReleaseInput>() * 2)?,
            )
        })
        .and_then(|n| n.checked_add(size_of::<BodyRecords>()))
        .ok_or_else(|| invalid("source body output allowance overflow"))?;
    let charge = budget.reserve("base_source_body_records", bytes)?;
    let mut sources = Vec::with_capacity(count);
    sources.extend(
        proof
            .native_premises()
            .iter()
            .map(|premise| BodySource::Native { premise: *premise }),
    );
    for statement in proof.entered_statements() {
        let mut rows = statements.iter().filter(|row| {
            row.invocation == invocation.id()
                && row.owner == request.callee
                && row.statement == *statement
        });
        let row = rows
            .next()
            .ok_or_else(|| invalid("source body statement mapping missing"))?;
        if rows.next().is_some() {
            return Err(invalid("source body statement mapping ambiguous"));
        }
        sources.push(BodySource::Statement {
            completion: row.id(),
        });
    }
    let outcome = CompletionOutcome::from(proof.outcome());
    let mut key = KeySink::new("base-source-body-releases");
    for (expression, safety) in proof.release_inputs() {
        expression.encode(&mut key);
        safety.encode(&mut key);
    }
    let body = SourceBodyCompletion {
        invocation: invocation.id(),
        owner: request.callee,
        declaration: proof.declaration(),
        qualification: proof.qualification(),
        outcome: outcome.id(),
        status: proof.status(),
        sources: ordered_digest("base-source-body-sources", sources.iter().map(Record::id)),
        releases: key.finish(),
    };
    let members = sources
        .iter()
        .enumerate()
        .map(|(ordinal, source)| BodyMember {
            body: body.id(),
            ordinal: ordinal as i64,
            source: source.id(),
        })
        .collect();
    let releases = proof
        .release_inputs()
        .iter()
        .enumerate()
        .map(|(ordinal, (expression, safety))| BodyReleaseInput {
            body: body.id(),
            ordinal: ordinal as i64,
            expression: *expression,
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
pub(crate) fn body_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "base_source_body_inventory",
        inputs: super::completion_production::inventory_inputs(),
        create: std::sync::Arc::new(|budget| super::completion_production::inventory_check(budget)),
    }]
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<BodySource>(),
        Relation::of::<SourceBodyCompletion>(),
        Relation::of::<BodyMember>(),
        Relation::of::<BodyReleaseInput>(),
        Relation::of::<BodyBoundary>(),
    ]
}

pub(crate) fn body_invariants_refs() -> Vec<&'static str> {
    vec!["base_source_body_inventory", "execution_body_fidelity"]
}
