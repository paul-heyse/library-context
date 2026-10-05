//! Base evaluation's nominal conclusions and exact ordered proof inventory. The enriched owner
//! will have its own records and predecessor sum; base evidence never references a source call.
use crate::domain::{
    analysis::{
        self, AnalysisDefinition, base_evaluation::AnalysisInvocation,
        native::NativeAssertionPremise, policy::EvidenceStatus,
    },
    assertion::AssertionQualification,
    conditions::entry::{EntryAccessSource, EntryData, EntryValueWitness},
    execution::evaluation::*,
    normalized::{Rows, entities::EntityRef},
    resources::{Reservation, ResourceBudget},
    source::Occurrence,
    *,
};
use crate::{Domain, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "base_evaluation_sources", rule = "base_evaluation_source")]
pub enum EvaluationSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Entry {
        #[model(premise)]
        witness: Id<EntryValueWitness>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_expression_evaluations",rule="base_closed_expression",invariant_refs=base_invariants_refs)]
pub struct ExpressionEvaluation {
    #[model(key, premise)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub expression: Id<Occurrence>,
    pub owner: Id<EntityRef>,
    pub qualification: Id<AssertionQualification>,
    pub boolean_value: Option<bool>,
    pub release: ReleaseSafety,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
    pub operands: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_evaluation_members",rule="base_evaluation_premise",conclusion=evaluation)]
pub struct EvaluationMember {
    #[model(key)]
    pub evaluation: Id<ExpressionEvaluation>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<EvaluationSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_evaluation_operands")]
pub struct EvaluationOperand {
    #[model(key)]
    pub evaluation: Id<ExpressionEvaluation>,
    #[model(key)]
    pub ordinal: i64,
    pub expression: Id<Occurrence>,
}
impl analysis::support::sealed::DerivedEvidence for ExpressionEvaluation {}
impl analysis::support::DerivedEvidence for ExpressionEvaluation {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct BaseEvaluationRecords {
    pub evaluation: ExpressionEvaluation,
    pub sources: Vec<EvaluationSource>,
    pub members: Vec<EvaluationMember>,
    pub operands: Vec<EvaluationOperand>,
    _charge: Box<dyn Reservation>,
}
pub(crate) fn ordered_digest<R: Record>(
    name: &str,
    rows: impl IntoIterator<Item = Id<R>>,
) -> ContentHash {
    let mut sink = KeySink::new(name);
    for id in rows {
        id.encode(&mut sink);
    }
    sink.finish()
}
impl CheckedEvaluation {
    pub fn emit_base(
        &self,
        invocation: &AnalysisInvocation,
        definition: &AnalysisDefinition,
        budget: &ResourceBudget,
    ) -> Result<BaseEvaluationRecords, ModelError> {
        let request = self.request();
        if self.call_source().is_some() {
            return Err(ModelError::Invalid(
                "base emission refuses enriched call proof".into(),
            ));
        }
        if invocation.input != request.input
            || invocation.context != request.context
            || invocation
                .subject
                .is_some_and(|owner| owner != request.owner)
            || invocation.definition != definition.id()
            || *definition != super::configuration::base_evaluation().1
        {
            return Err(ModelError::Invalid(
                "base evaluation invocation changes admitted frame or method".into(),
            ));
        }
        let source_count = self
            .native_premises()
            .len()
            .checked_add(self.entry_premises().len())
            .ok_or_else(|| ModelError::Invalid("base evaluation source count overflow".into()))?;
        let allowance = source_count
            .checked_mul(size_of::<EvaluationSource>() + size_of::<EvaluationMember>())
            .and_then(|n| {
                n.checked_add(
                    self.evaluated_operands()
                        .len()
                        .checked_mul(size_of::<EvaluationOperand>())?,
                )
            })
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| n.checked_add(size_of::<BaseEvaluationRecords>()))
            .ok_or_else(|| {
                ModelError::Invalid("base evaluation output allowance overflow".into())
            })?;
        let charge = budget.reserve("base_evaluation_records", allowance)?;
        let mut sources = Vec::with_capacity(source_count);
        sources.extend(
            self.native_premises()
                .iter()
                .map(|id| EvaluationSource::Native { premise: *id }),
        );
        sources.extend(
            self.entry_premises()
                .iter()
                .map(|id| EvaluationSource::Entry { witness: *id }),
        );
        let evaluation = ExpressionEvaluation {
            invocation: invocation.id(),
            expression: request.expression,
            owner: request.owner,
            qualification: self.qualification(),
            boolean_value: self.truth(),
            release: self.release(),
            status: self.status(),
            sources: ordered_digest("base-evaluation-sources", sources.iter().map(Record::id)),
            operands: ordered_digest(
                "base-evaluation-operands",
                self.evaluated_operands().iter().copied(),
            ),
        };
        let members = sources
            .iter()
            .enumerate()
            .map(|(ordinal, row)| EvaluationMember {
                evaluation: evaluation.id(),
                ordinal: ordinal as i64,
                source: row.id(),
            })
            .collect();
        let operands = self
            .evaluated_operands()
            .iter()
            .enumerate()
            .map(|(ordinal, id)| EvaluationOperand {
                evaluation: evaluation.id(),
                ordinal: ordinal as i64,
                expression: *id,
            })
            .collect();
        Ok(BaseEvaluationRecords {
            evaluation,
            sources,
            members,
            operands,
            _charge: charge,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvaluationFacts {
    request: ExpressionRequest,
    qualification: Id<AssertionQualification>,
    boolean_value: Option<bool>,
    release: ReleaseSafety,
    status: EvidenceStatus,
    sources: ContentHash,
    operands: ContentHash,
    call: Option<super::evaluation::CallOrigin>,
}
impl EvaluationFacts {
    pub(crate) fn of(checked: &CheckedEvaluation) -> Self {
        let sources = checked
            .native_premises()
            .iter()
            .map(|id| EvaluationSource::Native { premise: *id }.id())
            .chain(
                checked
                    .entry_premises()
                    .iter()
                    .map(|id| EvaluationSource::Entry { witness: *id }.id()),
            );
        Self {
            call: checked.call_source(),
            request: checked.request(),
            qualification: checked.qualification(),
            boolean_value: checked.truth(),
            release: checked.release(),
            status: checked.status(),
            sources: ordered_digest("base-evaluation-sources", sources),
            operands: ordered_digest(
                "base-evaluation-operands",
                checked.evaluated_operands().iter().copied(),
            ),
        }
    }
    pub(crate) fn is_base(&self) -> bool {
        self.call.is_none()
    }
    pub(crate) fn matches(
        &self,
        row: &ExpressionEvaluation,
        invocation: &AnalysisInvocation,
    ) -> bool {
        self.call.is_none()
            && invocation.input == self.request.input
            && invocation.context == self.request.context
            && invocation
                .subject
                .is_none_or(|owner| owner == self.request.owner)
            && row.invocation == invocation.id()
            && row.expression == self.request.expression
            && row.owner == self.request.owner
            && row.qualification == self.qualification
            && row.boolean_value == self.boolean_value
            && row.release == self.release
            && row.status == self.status
            && row.sources == self.sources
            && row.operands == self.operands
    }
}

pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<EvaluationSource>(),
        Relation::of::<ExpressionEvaluation>(),
        Relation::of::<EvaluationMember>(),
        Relation::of::<EvaluationOperand>(),
    ]
}
pub fn base_invariants() -> Vec<Invariant> {
    let mut inputs = EvaluationData::validation_inputs();
    inputs.extend(EntryData::validation_inputs().into_iter().map(|input| {
        if stages::is_vocabulary(input.name()) {
            input
        } else {
            input
        }
    }));
    inputs.extend([
        ValidationInput::of::<EntryValueWitness>(&["id"]),
        ValidationInput::of::<EntryAccessSource>(&["id"]),
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<AnalysisDefinition>(&["id"]),
        ValidationInput::of::<ExpressionEvaluation>(&["id"]),
        ValidationInput::of::<EvaluationSource>(&["id"]),
        ValidationInput::of::<EvaluationMember>(&["id"]),
        ValidationInput::of::<EvaluationOperand>(&["id"]),
    ]);
    inputs.sort_by_key(|input| (input.name(), input.prefix()));
    inputs.dedup_by_key(|input| (input.name(), input.prefix()));
    vec![Invariant {
        revision: 1,
        name: "base_closed_expression_replay",
        inputs,
        create: std::sync::Arc::new(|budget| Box::new(BaseCheck::new(budget))),
    }]
}
pub(crate) struct BaseCheck {
    pub(crate) data: EvaluationData,
    entry: EntryData,
    pub(crate) invocations: Rows<AnalysisInvocation>,
    definitions: Rows<AnalysisDefinition>,
    pub(crate) evaluations: Rows<ExpressionEvaluation>,
    sources: Rows<EvaluationSource>,
    members: Rows<EvaluationMember>,
    operands: Rows<EvaluationOperand>,
    entries: Rows<EntryValueWitness>,
    entry_sources: Rows<EntryAccessSource>,
    budget: ResourceBudget,
}
impl BaseCheck {
    /// The exact current caller formal slot remains an outside holder during an entered call.
    /// This does not prove disposal at the caller's own frame exit or guard stability.
    pub(crate) fn held_formal(
        &self,
        row: &ExpressionEvaluation,
    ) -> Result<Option<Id<calls::SignatureParameter>>, ModelError> {
        let checked = self.replay(row)?;
        let request = checked.request();
        if checked.release() != ReleaseSafety::CallerRetained
            || self
                .data
                .occurrences
                .get(request.expression)
                .is_none_or(|o| o.syntax_kind != source::SyntaxKind::ExprName)
            || checked.entry_premises().len() != 1
        {
            return Ok(None);
        }
        let witness = self
            .entries
            .get(checked.entry_premises()[0])
            .ok_or_else(|| ModelError::Invalid("caller argument holder missing".into()))?;
        let access_source = self
            .entry_sources
            .get(witness.access_source)
            .ok_or_else(|| ModelError::Invalid("caller argument holder source missing".into()))?;
        if !matches!(access_source, EntryAccessSource::Use { .. })
            || witness.owner != request.owner
            || witness.context != request.context
            || witness.access != request.expression
            || self
                .entry
                .runs
                .get(witness.run)
                .is_none_or(|run| run.input != request.input)
        {
            return Ok(None);
        }
        let proof = EntryValueWitness::derive_for(
            &self.entry,
            witness.request(),
            access_source,
            &self.budget,
        )?
        .map_err(|_| ModelError::Invalid("caller argument holder replay refused".into()))?;
        Ok(
            (proof.witness() == witness && proof.qualification().id() == row.qualification)
                .then_some(proof.parameter()),
        )
    }

    pub(crate) fn caller_holds_argument(
        &self,
        row: &ExpressionEvaluation,
    ) -> Result<bool, ModelError> {
        Ok(self.held_formal(row)?.is_some())
    }
    pub(crate) fn new(budget: &ResourceBudget) -> Self {
        Self {
            data: EvaluationData::new(budget),
            entry: EntryData::new(budget),
            invocations: Rows::new(budget),
            definitions: Rows::new(budget),
            evaluations: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            operands: Rows::new(budget),
            entries: Rows::new(budget),
            entry_sources: Rows::new(budget),
            budget: budget.clone(),
        }
    }
}
impl InvariantCheck for BaseCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.data.visit(name, batch)?;
        self.entry.visit(name, batch)?;
        macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME {self.$field.decode(batch)?;})*};}
        rows! {invocations:AnalysisInvocation,definitions:AnalysisDefinition,evaluations:ExpressionEvaluation,sources:EvaluationSource,members:EvaluationMember,operands:EvaluationOperand,entries:EntryValueWitness,entry_sources:EntryAccessSource,}
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for row in self.evaluations.iter() {
            let _ = self.replay(row)?;
        }
        Ok(())
    }
}
impl BaseCheck {
    pub(crate) fn replay(
        &self,
        row: &ExpressionEvaluation,
    ) -> Result<CheckedEvaluation, ModelError> {
        let invalid = |message: &str| ModelError::Invalid(message.into());
        let invocation = self
            .invocations
            .get(row.invocation)
            .ok_or_else(|| invalid("base evaluation invocation missing"))?;
        let definition = self
            .definitions
            .get(invocation.definition)
            .ok_or_else(|| invalid("base evaluation definition missing"))?;
        let members_count = self
            .members
            .iter()
            .filter(|member| member.evaluation == row.id())
            .count();
        let operands_count = self
            .operands
            .iter()
            .filter(|operand| operand.evaluation == row.id())
            .count();
        if members_count > 128 || operands_count > 64 {
            return Err(invalid(
                "base evaluation proof inventory exceeds finite limit",
            ));
        }
        let _scratch = self.budget.reserve(
            "base_evaluation_replay_scratch",
            members_count
                .checked_mul(
                    size_of::<EvaluationMember>()
                        + size_of::<conditions::entry::DerivedEntryValue>() * 2
                        + size_of::<&conditions::entry::DerivedEntryValue>() * 2,
                )
                .and_then(|n| n.checked_add(operands_count * size_of::<EvaluationOperand>()))
                .ok_or_else(|| invalid("base evaluation replay allowance overflow"))?,
        )?;
        let mut entries = Vec::with_capacity(members_count);
        for member in self
            .members
            .iter()
            .filter(|member| member.evaluation == row.id())
        {
            let source = self
                .sources
                .get(member.source)
                .ok_or_else(|| invalid("base evaluation source missing"))?;
            if let EvaluationSource::Entry { witness } = source {
                let witness = self
                    .entries
                    .get(*witness)
                    .ok_or_else(|| invalid("base evaluation entry premise missing"))?;
                let source = self
                    .entry_sources
                    .get(witness.access_source)
                    .ok_or_else(|| invalid("base evaluation entry source missing"))?;
                if !matches!(source, EntryAccessSource::Use { .. }) {
                    return Err(invalid(
                        "base evaluation name read requires Use entry source",
                    ));
                }
                let proof = EntryValueWitness::derive_for(
                    &self.entry,
                    witness.request(),
                    source,
                    &self.budget,
                )?
                .map_err(|_| invalid("base evaluation entry premise refused"))?;
                if proof.witness() != witness {
                    return Err(invalid("base evaluation entry premise changed"));
                }
                entries.push(proof);
            }
        }
        let refs = entries.iter().collect::<Vec<_>>();
        let checked = evaluate_with_entries(
            &self.data,
            ExpressionRequest {
                input: invocation.input,
                context: invocation.context,
                owner: row.owner,
                expression: row.expression,
            },
            &refs,
            &self.budget,
        )?
        .map_err(|_| invalid("base evaluation replay refused"))?;
        let expected = checked.emit_base(invocation, definition, &self.budget)?;
        if *row != expected.evaluation {
            return Err(invalid(
                "base evaluation changes qualification, value, disposal or evidence",
            ));
        }
        let mut members = self
            .members
            .iter()
            .filter(|member| member.evaluation == row.id())
            .collect::<Vec<_>>();
        members.sort_by_key(|member| member.ordinal);
        if members.len() != expected.members.len()
            || members
                .iter()
                .zip(&expected.members)
                .any(|(actual, expected)| *actual != expected)
        {
            return Err(invalid("base evaluation omitted or reordered a source"));
        }
        let mut operands = self
            .operands
            .iter()
            .filter(|operand| operand.evaluation == row.id())
            .collect::<Vec<_>>();
        operands.sort_by_key(|operand| operand.ordinal);
        if operands.len() != expected.operands.len()
            || operands
                .iter()
                .zip(&expected.operands)
                .any(|(actual, expected)| *actual != expected)
        {
            return Err(invalid(
                "base evaluation omitted or reordered an entered operand",
            ));
        }
        Ok(checked)
    }
}

pub(crate) fn base_invariants_refs() -> Vec<&'static str> {
    vec!["base_closed_expression_replay"]
}
