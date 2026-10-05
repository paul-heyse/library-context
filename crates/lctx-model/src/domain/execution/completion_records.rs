//! Base statement outcomes cite earlier base evaluations through a nominal predecessor edge.
//! Expression/value admission and ordered statement execution remain distinct proof owners.
use super::{
    ExactRuntimeException,
    completion::*,
    outcome::PendingOutcome,
    records::{self, BaseCheck, ExpressionEvaluation, ordered_digest},
};
use crate::domain::{
    analysis::{
        self, AnalysisDefinition, base_completion::AnalysisInvocation,
        native::NativeAssertionPremise, policy::EvidenceStatus,
    },
    assertion::AssertionQualification,
    normalized::{Rows, entities::EntityRef},
    resources::{Reservation, ResourceBudget},
    source::Occurrence,
    *,
};
use crate::{Domain, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(name = "base_completion_outcomes")]
pub enum CompletionOutcome {
    #[model(code = 0)]
    Normal,
    #[model(code = 1)]
    Return { site: Id<Occurrence> },
    #[model(code = 2)]
    Raise {
        site: Id<Occurrence>,
        exception: ExactRuntimeException,
    },
    #[model(code = 3)]
    Break { site: Id<Occurrence> },
    #[model(code = 4)]
    Continue { site: Id<Occurrence> },
}
impl From<PendingOutcome> for CompletionOutcome {
    fn from(outcome: PendingOutcome) -> Self {
        match outcome {
            PendingOutcome::Normal => Self::Normal,
            PendingOutcome::Return { site } => Self::Return { site },
            PendingOutcome::Raise { site, exception } => Self::Raise { site, exception },
            PendingOutcome::Break { site } => Self::Break { site },
            PendingOutcome::Continue { site } => Self::Continue { site },
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "base_completion_sources", rule = "base_completion_source")]
pub enum CompletionSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Evaluation {
        #[model(premise)]
        evaluation: Id<ExpressionEvaluation>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name="base_statement_completions",rule="base_statement_completion",invariant_refs=completion_invariants_refs)]
pub struct StatementCompletion {
    #[model(key, premise)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Id<EntityRef>,
    pub qualification: Id<AssertionQualification>,
    pub outcome: Id<CompletionOutcome>,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
    pub entered: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_completion_members",rule="base_completion_premise",conclusion=completion)]
pub struct CompletionMember {
    #[model(key)]
    pub completion: Id<StatementCompletion>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<CompletionSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_entered_statements")]
pub struct EnteredStatement {
    #[model(key)]
    pub completion: Id<StatementCompletion>,
    #[model(key)]
    pub ordinal: i64,
    pub statement: Id<Occurrence>,
}
impl analysis::support::sealed::DerivedEvidence for StatementCompletion {}
impl analysis::support::DerivedEvidence for StatementCompletion {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
/// Mapping a private evaluation token onto its earlier nominal conclusion never changes the
/// conclusion's identity. The emitter verifies all operation facts against this earlier row.
pub struct CompletionEvaluation<'a> {
    pub evaluation: &'a ExpressionEvaluation,
    pub invocation: &'a analysis::base_evaluation::AnalysisInvocation,
}
pub struct BaseCompletionRecords {
    pub completion: StatementCompletion,
    pub outcome: CompletionOutcome,
    pub sources: Vec<CompletionSource>,
    pub members: Vec<CompletionMember>,
    pub entered: Vec<EnteredStatement>,
    _charge: Box<dyn Reservation>,
}
impl CheckedCompletion {
    pub fn emit_base(
        &self,
        invocation: &AnalysisInvocation,
        definition: &AnalysisDefinition,
        evaluations: &[CompletionEvaluation<'_>],
        budget: &ResourceBudget,
    ) -> Result<BaseCompletionRecords, ModelError> {
        let request = self.request();
        let invalid = |message: &str| ModelError::Invalid(message.into());
        if invocation.input != request.input
            || invocation.context != request.context
            || invocation
                .subject
                .is_some_and(|owner| owner != request.owner)
            || invocation.definition != definition.id()
            || *definition != super::configuration::base_completion().1
        {
            return Err(invalid(
                "base completion invocation changes frame or method",
            ));
        }
        if !self.context_premises().is_empty()
            || !self.definition_premises().is_empty()
            || !self.header_premises().is_empty()
            || self.evaluation_facts().iter().any(|facts| !facts.is_base())
        {
            return Err(invalid("base completion emission refuses enriched proof"));
        }
        if evaluations.len() != self.evaluation_facts().len()
            || evaluations
                .iter()
                .zip(self.evaluation_facts())
                .any(|(input, facts)| !facts.matches(input.evaluation, input.invocation))
        {
            return Err(invalid(
                "base completion changes an entered earlier evaluation",
            ));
        }
        let count = self
            .native_premises()
            .len()
            .checked_add(evaluations.len())
            .ok_or_else(|| invalid("completion source count overflow"))?;
        let allowance = count
            .checked_mul((size_of::<CompletionSource>() + size_of::<CompletionMember>()) * 2)
            .and_then(|n| {
                n.checked_add(self.entered_statements().len() * size_of::<EnteredStatement>() * 2)
            })
            .and_then(|n| n.checked_add(size_of::<BaseCompletionRecords>()))
            .ok_or_else(|| invalid("completion output allowance overflow"))?;
        let charge = budget.reserve("base_completion_records", allowance)?;
        let mut sources = Vec::with_capacity(count);
        sources.extend(
            self.native_premises()
                .iter()
                .map(|id| CompletionSource::Native { premise: *id }),
        );
        sources.extend(
            evaluations
                .iter()
                .map(|input| CompletionSource::Evaluation {
                    evaluation: input.evaluation.id(),
                }),
        );
        let outcome = CompletionOutcome::from(self.outcome());
        let completion = StatementCompletion {
            invocation: invocation.id(),
            statement: request.statement,
            owner: request.owner,
            qualification: self.qualification(),
            outcome: outcome.id(),
            status: self.status(),
            sources: ordered_digest("base-completion-sources", sources.iter().map(Record::id)),
            entered: ordered_digest(
                "base-completion-entered",
                self.entered_statements().iter().copied(),
            ),
        };
        let members = sources
            .iter()
            .enumerate()
            .map(|(ordinal, row)| CompletionMember {
                completion: completion.id(),
                ordinal: ordinal as i64,
                source: row.id(),
            })
            .collect();
        let entered = self
            .entered_statements()
            .iter()
            .enumerate()
            .map(|(ordinal, id)| EnteredStatement {
                completion: completion.id(),
                ordinal: ordinal as i64,
                statement: *id,
            })
            .collect();
        Ok(BaseCompletionRecords {
            completion,
            outcome,
            sources,
            members,
            entered,
            _charge: charge,
        })
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<CompletionOutcome>(),
        Relation::of::<CompletionSource>(),
        Relation::of::<StatementCompletion>(),
        Relation::of::<CompletionMember>(),
        Relation::of::<EnteredStatement>(),
    ]
}
pub fn completion_invariants() -> Vec<Invariant> {
    let mut inputs = records::base_invariants().remove(0).inputs;
    inputs.extend([
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<CompletionOutcome>(&["id"]),
        ValidationInput::of::<StatementCompletion>(&["id"]),
        ValidationInput::of::<CompletionSource>(&["id"]),
        ValidationInput::of::<CompletionMember>(&["id"]),
        ValidationInput::of::<EnteredStatement>(&["id"]),
    ]);
    inputs.sort_by_key(|input| (input.name(), input.prefix()));
    inputs.dedup_by_key(|input| (input.name(), input.prefix()));
    vec![Invariant {
        revision: 1,
        name: "base_statement_completion_replay",
        inputs,
        create: std::sync::Arc::new(|budget| Box::new(CompletionCheck::new(budget))),
    }]
}
struct CompletionCheck {
    base: BaseCheck,
    invocations: Rows<AnalysisInvocation>,
    definitions: Rows<AnalysisDefinition>,
    completions: Rows<StatementCompletion>,
    outcomes: Rows<CompletionOutcome>,
    sources: Rows<CompletionSource>,
    members: Rows<CompletionMember>,
    entered: Rows<EnteredStatement>,
    base_invocations: Rows<analysis::base_evaluation::AnalysisInvocation>,
    budget: ResourceBudget,
}
impl CompletionCheck {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            base: BaseCheck::new(budget),
            invocations: Rows::new(budget),
            definitions: Rows::new(budget),
            completions: Rows::new(budget),
            outcomes: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            entered: Rows::new(budget),
            base_invocations: Rows::new(budget),
            budget: budget.clone(),
        }
    }
}
impl InvariantCheck for CompletionCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.base.visit(name, batch)?;
        macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME {self.$field.decode(batch)?;})*};}
        rows! {invocations:AnalysisInvocation,definitions:AnalysisDefinition,completions:StatementCompletion,outcomes:CompletionOutcome,sources:CompletionSource,members:CompletionMember,entered:EnteredStatement,base_invocations:analysis::base_evaluation::AnalysisInvocation,}
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let invalid = |message: &str| ModelError::Invalid(message.into());
        for row in self.completions.iter() {
            let invocation = self
                .invocations
                .get(row.invocation)
                .ok_or_else(|| invalid("base completion invocation missing"))?;
            let definition = self
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("base completion definition missing"))?;
            let count = self
                .members
                .iter()
                .filter(|m| m.completion == row.id())
                .count();
            let entered_count = self
                .entered
                .iter()
                .filter(|m| m.completion == row.id())
                .count();
            if count > 128 || entered_count > 64 {
                return Err(invalid("base completion proof exceeds finite limit"));
            }
            let _scratch = self.budget.reserve(
                "base_completion_replay_scratch",
                count
                    * (size_of::<super::evaluation::CheckedEvaluation>() * 2
                        + size_of::<CompletionEvaluation<'_>>() * 2
                        + size_of::<&super::evaluation::CheckedEvaluation>() * 2
                        + size_of::<&CompletionMember>() * 2)
                    + entered_count * size_of::<&EnteredStatement>() * 2,
            )?;
            let mut members = self
                .members
                .iter()
                .filter(|m| m.completion == row.id())
                .collect::<Vec<_>>();
            members.sort_by_key(|m| m.ordinal);
            let mut earlier = Vec::new();
            let mut checked = Vec::new();
            for member in &members {
                match self
                    .sources
                    .get(member.source)
                    .ok_or_else(|| invalid("base completion source missing"))?
                {
                    CompletionSource::Native { .. } => {}
                    CompletionSource::Evaluation { evaluation } => {
                        let row =
                            self.base.evaluations.get(*evaluation).ok_or_else(|| {
                                invalid("base completion earlier evaluation missing")
                            })?;
                        let invocation = self
                            .base_invocations
                            .get(row.invocation)
                            .ok_or_else(|| invalid("base completion earlier invocation missing"))?;
                        checked.push(self.base.replay(row)?);
                        earlier.push(CompletionEvaluation {
                            evaluation: row,
                            invocation,
                        });
                    }
                }
            }
            let refs = checked.iter().collect::<Vec<_>>();
            let result = complete(
                &self.base.data,
                CompletionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner: row.owner,
                    statement: row.statement,
                },
                &refs,
                &self.budget,
            )?
            .map_err(|_| invalid("base completion replay refused"))?;
            let expected = result.emit_base(invocation, definition, &earlier, &self.budget)?;
            if *row != expected.completion
                || self.outcomes.get(row.outcome) != Some(&expected.outcome)
            {
                return Err(invalid(
                    "base completion changes pending outcome, frame, qualification or evidence",
                ));
            }
            if members.len() != expected.members.len()
                || members.iter().zip(&expected.members).any(|(a, b)| *a != b)
            {
                return Err(invalid("base completion omitted or reordered evidence"));
            }
            let mut entered = self
                .entered
                .iter()
                .filter(|m| m.completion == row.id())
                .collect::<Vec<_>>();
            entered.sort_by_key(|m| m.ordinal);
            if entered.len() != expected.entered.len()
                || entered.iter().zip(&expected.entered).any(|(a, b)| *a != b)
            {
                return Err(invalid(
                    "base completion omitted or reordered entered statements",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn completion_invariants_refs() -> Vec<&'static str> {
    vec!["base_statement_completion_replay"]
}
