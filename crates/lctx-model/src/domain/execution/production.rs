//! Finite base producers. Each selected source expression has either its exact evaluation or
//! an explicit shared boundary reason. Missing observations never become successful evaluation.
use super::{evaluation::*, records::*};
use crate::Domain;
use crate::domain::{
    analysis::{self, base_evaluation as publication},
    conditions::entry::{EntryAccessSource, EntryData, EntryValueWitness},
    input::ArtifactUse,
    normalized::Rows,
    resources::ResourceBudget,
    source::{Occurrence, SourceArtifact, SyntaxKind},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_evaluation_boundaries")]
pub struct EvaluationBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub expression: Id<Occurrence>,
    pub owner: Option<Id<normalized::entities::EntityRef>>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_evaluation_runs",invariant_refs=run_invariants_refs,publication_refs=run_publication_checks_refs)]
pub struct EvaluationRun {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    pub requested: bool,
    pub evaluated: i64,
    pub refused: i64,
}
pub struct EvaluationRecords {
    pub run: EvaluationRun,
    pub reads: super::read_channels::ReadRecords,
    pub evaluations: Rows<ExpressionEvaluation>,
    pub sources: Rows<EvaluationSource>,
    pub members: Rows<EvaluationMember>,
    pub operands: Rows<EvaluationOperand>,
    pub boundaries: Rows<EvaluationBoundary>,
    pub outcome: publication::AnalysisOutcome,
}
impl EvaluationRecords {
    fn new(invocation: Id<publication::AnalysisInvocation>, budget: &ResourceBudget) -> Self {
        Self {
            reads: super::read_channels::ReadRecords::new(budget),
            run: EvaluationRun {
                invocation,
                requested: false,
                evaluated: 0,
                refused: 0,
            },
            evaluations: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            operands: Rows::new(budget),
            boundaries: Rows::new(budget),
            outcome: publication::AnalysisOutcome {
                invocation,
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            },
        }
    }
}
/// Compact private values produced by this owner, associated with their exact emitted rows.
/// No stored stream or source snapshot can construct an evaluation in this index.
pub struct ProducedEvaluations {
    values: charged::ChargedMap<Id<ExpressionEvaluation>, ActualEvaluation>,
    frames: Rows<publication::AnalysisInvocation>,
    charge: charged::StateCharge,
}
#[derive(Clone)]
struct ActualEvaluation {
    row: ExpressionEvaluation,
    value: ProducedValue,
    held: Option<Id<calls::SignatureParameter>>,
}
impl HeapSize for ActualEvaluation {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl ProducedEvaluations {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            values: Default::default(),
            frames: Rows::new(budget),
            charge: charged::StateCharge::new(budget, "produced-base-values"),
        }
    }
    pub fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if !self
            .charge
            .budget()
            .expect("bound owner")
            .shares_pool(other.charge.budget().expect("bound owner"))
        {
            return Err(ModelError::Conflict("produced evaluations budget"));
        }
        for row in other.frames.iter() {
            self.frames.insert(row.clone())?;
        }
        for (id, value) in other.values.iter() {
            if self.values.contains_key(id) {
                return Err(ModelError::Conflict("produced evaluation duplicated"));
            }
            self.values.insert(&mut self.charge, *id, value.clone())?;
        }
        Ok(())
    }
    fn actual(
        &self,
        row: &ExpressionEvaluation,
        frame: &publication::AnalysisInvocation,
    ) -> Result<&ActualEvaluation, ModelError> {
        if row.invocation != frame.id() || self.frames.get(frame.id()) != Some(frame) {
            return Err(ModelError::Conflict("produced evaluation frame"));
        }
        let value = self
            .values
            .get(&row.id())
            .ok_or(ModelError::Conflict("produced evaluation absent"))?;
        if value.row != *row {
            return Err(ModelError::Conflict("produced evaluation changed"));
        }
        Ok(value)
    }
    pub(crate) fn get(
        &self,
        row: &ExpressionEvaluation,
        frame: &publication::AnalysisInvocation,
        earlier: &super::records::BaseCheck,
    ) -> Result<std::sync::Arc<CheckedEvaluation>, ModelError> {
        let actual = self.actual(row, frame)?;
        Ok(std::sync::Arc::new(
            earlier.hydrate_actual(row, actual.value)?,
        ))
    }
    pub(crate) fn held_formal(
        &self,
        row: &ExpressionEvaluation,
        frame: &publication::AnalysisInvocation,
    ) -> Result<Option<Id<calls::SignatureParameter>>, ModelError> {
        Ok(self.actual(row, frame)?.held)
    }
}
pub(crate) fn is_expression(kind: SyntaxKind) -> bool {
    use SyntaxKind::*;
    matches!(
        kind,
        ExprBoolOp
            | ExprNamed
            | ExprBinOp
            | ExprUnaryOp
            | ExprLambda
            | ExprIf
            | ExprDict
            | ExprSet
            | ExprListComp
            | ExprSetComp
            | ExprDictComp
            | ExprGenerator
            | ExprAwait
            | ExprYield
            | ExprYieldFrom
            | ExprCompare
            | ExprCall
            | ExprFString
            | ExprTString
            | ExprStringLiteral
            | ExprBytesLiteral
            | ExprNumberLiteral
            | ExprBooleanLiteral
            | ExprNoneLiteral
            | ExprEllipsisLiteral
            | ExprAttribute
            | ExprSubscript
            | ExprStarred
            | ExprName
            | ExprList
            | ExprTuple
            | ExprSlice
            | ExprIpyEscapeCommand
    )
}
/// The caller's captured profile owns request selection. Publication independently verifies that
/// profile through the analysis family's declared frontier callback.
#[allow(
    clippy::too_many_arguments,
    reason = "Public publication boundary keeps independently admitted evaluation, entry, binding and event owners explicit."
)]
pub fn evaluate_all(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<EvaluationRecords, ModelError> {
    evaluate_all_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
    )
    .map(|(records, _)| records)
}
pub fn evaluate_all_produced(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    evaluate_selected_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
        None,
        true,
        None,
        None,
    )
}
/// Actual compiler production borrows the Local issuer's entry values. Raw stored entry
/// witnesses cannot replace that authority or trigger another Local producer operation.
#[allow(
    clippy::too_many_arguments,
    reason = "Actual Local owner and immutable evaluation frame remain explicit."
)]
pub fn evaluate_all_with_local(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    actual_local: &local_semantics::ProducedLocal,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    evaluate_selected_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
        None,
        true,
        Some(actual_local),
        None,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Actual Local owner and immutable expression kernel remain explicit."
)]
pub fn evaluate_expression_with_local(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    expression: Id<Occurrence>,
    actual_local: &local_semantics::ProducedLocal,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    if data.occurrences.get(expression).is_none() {
        return Err(ModelError::Invalid("selected expression is absent".into()));
    }
    evaluate_selected_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
        Some(expression),
        false,
        Some(actual_local),
        None,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Actual Local values and complete projected owner uncertainty remain explicit."
)]
pub fn evaluate_expression_scoped(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    expression: Id<Occurrence>,
    actual_local: &local_semantics::ProducedLocal,
    metadata: &super::evaluation::PreparedEvaluationMetadata,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    if data.occurrences.get(expression).is_none() {
        return Err(ModelError::Conflict("selected expression is absent"));
    }
    evaluate_selected_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
        Some(expression),
        false,
        Some(actual_local),
        Some(metadata),
    )
}
/// One selected expression and its complete dependency closure. Read observations and complete
/// negatives belong to independent read/universe kernels; this expression kernel emits neither.
#[allow(
    clippy::too_many_arguments,
    reason = "Actual predecessor/definition/frame owners remain explicit."
)]
pub fn evaluate_expression_produced(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    expression: Id<Occurrence>,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    if data.occurrences.get(expression).is_none() {
        return Err(ModelError::Invalid("selected expression is absent".into()));
    }
    evaluate_selected_produced(
        data,
        entry,
        stored_entries,
        entry_sources,
        invocation,
        definition,
        profile,
        budget,
        Some(expression),
        false,
        None,
        None,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Selected actual owner operation shares its ordinary whole-operation oracle."
)]
fn evaluate_selected_produced(
    data: &EvaluationData,
    entry: &EntryData,
    stored_entries: &Rows<EntryValueWitness>,
    entry_sources: &Rows<EntryAccessSource>,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    selected: Option<Id<Occurrence>>,
    emit_reads: bool,
    actual_local: Option<&local_semantics::ProducedLocal>,
    metadata: Option<&super::evaluation::PreparedEvaluationMetadata>,
) -> Result<(EvaluationRecords, ProducedEvaluations), ModelError> {
    if *definition != super::configuration::base_evaluation().1
        || invocation.definition != definition.id()
        || invocation.subject.is_some()
    {
        return Err(ModelError::Invalid(
            "base evaluation producer requires its whole-frame definition".into(),
        ));
    }
    let mut records = EvaluationRecords::new(invocation.id(), budget);
    let mut produced = ProducedEvaluations::new(budget);
    produced.frames.insert(invocation.clone())?;
    if profile != stages::Profile::Behavioral {
        records.outcome.status = analysis::AnalysisStatus::NotRequested;
        records.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok((records, produced));
    }
    records.run.requested = true;
    let bytes = data
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<SourceArtifact>() + row.heap_bytes())
        })
        .and_then(|n| n.checked_add(data.uses.len().checked_mul(size_of::<ArtifactUse>())?))
        .ok_or_else(|| ModelError::Invalid("base root selection allowance overflow".into()))?;
    let allowance = bytes
        .checked_mul(2)
        .and_then(|n| n.checked_add(data.artifacts.len().checked_mul(128)?))
        .ok_or_else(|| ModelError::Invalid("base root selection allowance overflow".into()))?;
    let _roots_charge = budget.reserve("base_evaluation_roots", allowance)?;
    let artifacts = data.artifacts.iter().cloned().collect::<Vec<_>>();
    let uses = data.uses.iter().cloned().collect::<Vec<_>>();
    let roots = admission::analysis_roots(&artifacts, &uses)?;
    if emit_reads {
        records.reads = if let Some(actual) = actual_local {
            super::read_channels::produce_with_local(
                data,
                entry,
                invocation,
                &roots,
                budget,
                super::read_channels::ReadEntries {
                    witnesses: stored_entries,
                    sources: entry_sources,
                    actual,
                },
            )?
        } else {
            super::read_channels::produce(data, entry, invocation, &roots, budget)?
        };
    }
    let prepared = match metadata {
        Some(metadata) => PreparedExecution::with_metadata(
            data,
            invocation.input,
            invocation.context,
            budget,
            metadata,
        )?,
        None => PreparedExecution::new(data, invocation.input, invocation.context, budget)?,
    };
    let mut entry_proofs = Vec::new();
    let mut charge = charged::StateCharge::new(budget, "base_entry_inventory");
    for witness in stored_entries
        .iter()
        .filter(|w| w.context == invocation.context)
    {
        let source = entry_sources
            .get(witness.access_source)
            .ok_or_else(|| ModelError::Invalid("base stored entry source missing".into()))?;
        if !matches!(source, EntryAccessSource::Use { .. }) {
            continue;
        }
        let run = entry
            .runs
            .get(witness.run)
            .ok_or_else(|| ModelError::Invalid("base stored entry run missing".into()))?;
        if run.input != invocation.input {
            continue;
        }
        let checked = match actual_local {
            Some(actual) => actual.entry(witness, source, budget)?,
            None => EntryValueWitness::derive_for(entry, witness.request(), source, budget)?
                .map_err(|_| ModelError::Invalid("base stored Use entry replay refused".into()))?,
        };
        if checked.witness() != witness {
            return Err(ModelError::Invalid("base stored Use entry changed".into()));
        }
        charge.grow(
            size_of::<conditions::entry::DerivedEntryValue>() * 2
                + size_of::<&conditions::entry::DerivedEntryValue>() * 2,
        )?;
        entry_proofs.push(checked);
    }
    let refs = entry_proofs.iter().collect::<Vec<_>>();
    for row in data.occurrences.iter().filter(|row| {
        selected.is_none_or(|selected| row.id() == selected)
            && is_expression(row.syntax_kind)
            && roots.contains(&row.source)
            && data.artifacts.get(row.source).is_some_and(|source| {
                source.input == invocation.input
                    && admission::ArtifactClass::of(&source.path)
                        == Some(admission::ArtifactClass::PythonSource)
            })
    }) {
        let mut owners = data
            .owners
            .iter()
            .filter(|owner| owner.occurrence == row.id());
        let first = owners.next();
        let owner = if owners.next().is_none() {
            first.map(|owner| owner.entity)
        } else {
            None
        };
        let result = if let Some(owner) = owner {
            prepared.evaluate(
                ExpressionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner,
                    expression: row.id(),
                },
                &refs,
            )?
        } else {
            Err(obligation::ObligationKind::MissingEvidence)
        };
        match result {
            Ok(proof) => {
                let output = proof.emit_base(invocation, definition, budget)?;
                let held = if proof.release() == ReleaseSafety::CallerRetained
                    && row.syntax_kind == SyntaxKind::ExprName
                    && proof.entry_premises().len() == 1
                {
                    entry_proofs
                        .iter()
                        .find(|entry| entry.witness().id() == proof.entry_premises()[0])
                        .filter(|entry| {
                            entry.witness().owner == proof.request().owner
                                && entry.witness().context == proof.request().context
                                && entry.witness().access == proof.request().expression
                                && entry.qualification().id() == output.evaluation.qualification
                        })
                        .map(|entry| entry.parameter())
                } else {
                    None
                };
                produced.values.insert(
                    &mut produced.charge,
                    output.evaluation.id(),
                    ActualEvaluation {
                        row: output.evaluation.clone(),
                        value: ProducedValue::of(&proof),
                        held,
                    },
                )?;
                records.evaluations.insert(output.evaluation)?;
                for row in output.sources {
                    records.sources.insert(row)?;
                }
                for row in output.members {
                    records.members.insert(row)?;
                }
                for row in output.operands {
                    records.operands.insert(row)?;
                }
            }
            Err(reason) => {
                records.boundaries.insert(EvaluationBoundary {
                    invocation: invocation.id(),
                    expression: row.id(),
                    owner,
                    reason,
                })?;
            }
        }
    }
    records.run.evaluated = records
        .evaluations
        .len()
        .try_into()
        .map_err(|_| ModelError::Invalid("evaluation count exceeds signed domain".into()))?;
    records.run.refused = records
        .boundaries
        .len()
        .try_into()
        .map_err(|_| ModelError::Invalid("boundary count exceeds signed domain".into()))?;
    if !records.boundaries.is_empty() {
        records.outcome.status = analysis::AnalysisStatus::Partial;
        records.outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
    }
    Ok((records, produced))
}

/// Exact inventory is independently recomputed from admitted source roots, including refusals.
/// The profile flag is audit data; the publication check compares it with the actual stage profile.
pub fn relations() -> Vec<Relation> {
    {
        let mut rows = vec![
            Relation::of::<EvaluationRun>(),
            Relation::of::<EvaluationBoundary>(),
        ];
        rows.extend(super::read_channels::relations());
        rows
    }
}
fn run_inputs() -> Vec<ValidationInput> {
    let mut inputs = super::records::base_invariants().remove(0).inputs;
    inputs.extend([
        ValidationInput::of::<EvaluationRun>(&["id"]),
        ValidationInput::of::<EvaluationBoundary>(&["id"]),
        ValidationInput::of::<publication::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
    ]);
    inputs.extend(super::read_channels::validation_inputs());
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    inputs
}
pub(crate) fn run_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "base_evaluation_inventory",
        inputs: run_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(EvaluationRunCheck::new(budget))),
    }]
}
struct EvaluationRunCheck {
    reads: super::read_channels::ReadRecords,
    local: Rows<analysis::local::AnalysisInvocation>,
    data: EvaluationData,
    entry: EntryData,
    entries: Rows<EntryValueWitness>,
    entry_sources: Rows<EntryAccessSource>,
    invocations: Rows<publication::AnalysisInvocation>,
    definitions: Rows<analysis::AnalysisDefinition>,
    runs: Rows<EvaluationRun>,
    boundaries: Rows<EvaluationBoundary>,
    evaluations: Rows<ExpressionEvaluation>,
    sources: Rows<EvaluationSource>,
    members: Rows<EvaluationMember>,
    operands: Rows<EvaluationOperand>,
    outcomes: Rows<publication::AnalysisOutcome>,
    budget: ResourceBudget,
}
impl EvaluationRunCheck {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            reads: super::read_channels::ReadRecords::new(budget),
            local: Rows::new(budget),
            data: EvaluationData::new(budget),
            entry: EntryData::new(budget),
            entries: Rows::new(budget),
            entry_sources: Rows::new(budget),
            invocations: Rows::new(budget),
            definitions: Rows::new(budget),
            runs: Rows::new(budget),
            boundaries: Rows::new(budget),
            evaluations: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            operands: Rows::new(budget),
            outcomes: Rows::new(budget),
            budget: budget.clone(),
        }
    }
}
impl InvariantCheck for EvaluationRunCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.data.visit(name, batch)?;
        self.entry.visit(name, batch)?;
        self.reads.visit(name, batch)?;
        macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        rows! {local:analysis::local::AnalysisInvocation,entries:EntryValueWitness,entry_sources:EntryAccessSource,invocations:publication::AnalysisInvocation,definitions:analysis::AnalysisDefinition,runs:EvaluationRun,boundaries:EvaluationBoundary,evaluations:ExpressionEvaluation,sources:EvaluationSource,members:EvaluationMember,operands:EvaluationOperand,outcomes:publication::AnalysisOutcome,}
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let invalid = |message: &str| ModelError::Invalid(message.into());
        let mut expected_reads = super::read_channels::ReadRecords::new(&self.budget);
        let mut runs = Rows::new(&self.budget);
        let mut evaluations = Rows::new(&self.budget);
        let mut sources = Rows::new(&self.budget);
        let mut members = Rows::new(&self.budget);
        let mut operands = Rows::new(&self.budget);
        let mut boundaries = Rows::new(&self.budget);
        let mut outcomes = Rows::new(&self.budget);
        let mut frames = charged::ChargedSet::default();
        let mut frame_charge =
            charged::StateCharge::new(&self.budget, "base_evaluation_frame_inventory");
        for local in self.local.iter() {
            frames.insert(&mut frame_charge, (local.input, local.context))?;
        }
        if self.invocations.len() != frames.len()
            || frames.iter().any(|frame| {
                self.invocations
                    .iter()
                    .filter(|row| (row.input, row.context) == *frame)
                    .count()
                    != 1
            })
        {
            return Err(invalid(
                "base evaluation omitted or duplicated a Local input frame",
            ));
        }
        for invocation in self.invocations.iter() {
            let mut parents = Rows::new(&self.budget);
            for local in self
                .local
                .iter()
                .filter(|row| (row.input, row.context) == (invocation.input, invocation.context))
            {
                parents.insert(publication::InvocationSource::Local {
                    invocation: local.id(),
                })?;
            }
            let mut key = KeySink::new("analysis-invocation-inputs");
            for row in parents.iter() {
                row.id().encode(&mut key);
            }
            if invocation.inputs != key.finish() {
                return Err(invalid(
                    "base evaluation changes its exact Local invocation parents",
                ));
            }
            let run = self
                .runs
                .iter()
                .find(|r| r.invocation == invocation.id())
                .ok_or_else(|| invalid("base evaluation run missing"))?;
            let definition = self
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("base evaluation run definition missing"))?;
            let expected = evaluate_all(
                &self.data,
                &self.entry,
                &self.entries,
                &self.entry_sources,
                invocation,
                definition,
                if run.requested {
                    stages::Profile::Behavioral
                } else {
                    stages::Profile::Catalog
                },
                &self.budget,
            )?;
            expected_reads.append(&expected.reads)?;
            runs.insert(expected.run)?;
            outcomes.insert(expected.outcome)?;
            macro_rules! append{($($field:ident,)*)=>{$(for row in expected.$field.iter(){ $field.insert(row.clone())?;})*};}
            append! {evaluations,sources,members,operands,boundaries,}
        }
        if !self.reads.same(&expected_reads)
            || !self.runs.same(&runs)
            || !self.outcomes.same(&outcomes)
            || !self.evaluations.same(&evaluations)
            || !self.sources.same(&sources)
            || !self.members.same(&members)
            || !self.operands.same(&operands)
            || !self.boundaries.same(&boundaries)
        {
            return Err(invalid(
                "base evaluation inventory differs from admitted source universe",
            ));
        }
        Ok(())
    }
}
pub(crate) fn run_publication_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "base_evaluation_request_profile",
        inputs: vec![
            ValidationInput::of::<EvaluationRun>(&["id"]),
            ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(EvaluationProfileCheck {
                runs: Rows::new(budget),
                invocations: Rows::new(budget),
            })
        }),
    }]
}
struct EvaluationProfileCheck {
    runs: Rows<EvaluationRun>,
    invocations: Rows<publication::AnalysisInvocation>,
}
impl PublicationCheck for EvaluationProfileCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == EvaluationRun::NAME {
            self.runs.decode(batch)?;
        } else if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        } else {
            return Err(ModelError::Invalid(
                "undeclared base evaluation profile input".into(),
            ));
        }
        Ok(())
    }
    fn finish(
        self: Box<Self>,
        _sources: &[crate::domain::analysis::sources::SourceSnapshot],
        profile: stages::Profile,
    ) -> Result<(), ModelError> {
        if self.runs.len() != self.invocations.len()
            || self.runs.iter().any(|r| {
                self.invocations.get(r.invocation).is_none()
                    || r.requested != (profile == stages::Profile::Behavioral)
            })
        {
            return Err(ModelError::Invalid(
                "base evaluation run disagrees with actual publication profile".into(),
            ));
        }
        Ok(())
    }
}

/// Finite owner publication: predecessor closure is read-only; this stage never writes Local.
pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    if *definition != super::configuration::base_evaluation().1 {
        return Err(ModelError::Invalid(
            "base evaluation stage definition is unbound".into(),
        ));
    }
    let mut outputs = publication::publication_relations();
    outputs.extend(super::records::relations());
    outputs.extend(relations());
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let mut initial = if profile == Profile::Behavioral {
        run_inputs()
    } else {
        vec![
            ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ]
    };
    for relation in &outputs {
        for id in relation.publication_refs() {
            let check = model.publication_check(id)?;
            initial.extend(check.inputs.iter().cloned());
        }
    }
    let owned = outputs
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    // A publication check can name the records being written; only its predecessors are roots.
    initial.retain(|input| {
        is_vocabulary(input.name()) || !owned.iter().any(|row| row.name() == input.name())
    });
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        initial,
        &owned,
        PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    let mut key = KeySink::new("base-evaluation-definition");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "evaluate_base",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("production.rs")),
        configuration: key.finish(),
    })
}

pub(crate) fn run_invariants_refs() -> Vec<&'static str> {
    vec!["base_evaluation_inventory"]
}
pub(crate) fn run_publication_checks_refs() -> Vec<&'static str> {
    vec!["base_evaluation_request_profile"]
}

#[cfg(test)]
mod actual_value_controls {
    use super::*;
    use crate::domain::{
        analysis::native::NativeInventory, assertion::*, attribution::*, input::SourceRole,
        normalized::entities::*, syntax::*,
    };
    fn id<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn produced(
        budget: &ResourceBudget,
    ) -> (
        ProducedEvaluations,
        super::super::records::BaseCheck,
        publication::AnalysisInvocation,
        ExpressionEvaluation,
    ) {
        let artifact = SourceArtifact::from_bytes(id(1), "value.py".into(), b"None").unwrap();
        let scope = source::CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: id(2),
            scope: scope.id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let expression = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 4,
            syntax_kind: SyntaxKind::ExprNoneLiteral,
            role: source::OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let placement = SyntaxPlacement {
            qualification: q.id(),
            occurrence: expression.id(),
            parent: None,
            field: lexical::SyntaxField::Body,
            ordinal: 0,
        };
        let support = SyntaxPlacementSupport {
            assertion: placement.id(),
            run: id(3),
            surface: id(4),
            evidence: id(5),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        let mut native = NativeInventory::new(budget);
        native
            .visit(
                AssertionQualification::NAME,
                &AssertionQualification::encode(std::slice::from_ref(&q)).unwrap(),
            )
            .unwrap();
        native
            .visit(
                SyntaxPlacement::NAME,
                &SyntaxPlacement::encode(std::slice::from_ref(&placement)).unwrap(),
            )
            .unwrap();
        native
            .visit(
                SyntaxPlacementSupport::NAME,
                &SyntaxPlacementSupport::encode(&[support]).unwrap(),
            )
            .unwrap();
        let native = native.collect().unwrap();
        let mut data = EvaluationData::new(budget);
        data.premises = native.premises;
        data.native = native.qualifications;
        data.uses
            .insert(ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: SourceRole::Release,
            })
            .unwrap();
        data.artifacts.insert(artifact.clone()).unwrap();
        data.scopes.insert(scope).unwrap();
        data.qualifications.insert(q.clone()).unwrap();
        data.occurrences.insert(expression.clone()).unwrap();
        data.placements.insert(placement).unwrap();
        data.owners
            .insert(OccurrenceOwnership {
                occurrence: expression.id(),
                owner: expression.id(),
                entity: id(6),
            })
            .unwrap();
        let definition = super::super::configuration::base_evaluation().1;
        let invocation = publication::AnalysisInvocation::new(
            artifact.input,
            q.context,
            definition.id(),
            None,
            [],
        )
        .0;
        let (records, values) = evaluate_all_produced(
            &data,
            &EntryData::new(budget),
            &Rows::new(budget),
            &Rows::new(budget),
            &invocation,
            &definition,
            stages::Profile::Behavioral,
            budget,
        )
        .unwrap();
        assert_eq!(records.run.evaluated, 1);
        assert_eq!(records.run.refused, 0);
        let row = records.evaluations.iter().next().unwrap().clone();
        let (selected, _) = evaluate_expression_produced(
            &data,
            &EntryData::new(budget),
            &Rows::new(budget),
            &Rows::new(budget),
            &invocation,
            &definition,
            stages::Profile::Behavioral,
            budget,
            expression.id(),
        )
        .unwrap();
        assert!(selected.evaluations.same(&records.evaluations));
        assert!(selected.sources.same(&records.sources));
        assert!(selected.members.same(&records.members));
        assert!(selected.operands.same(&records.operands));
        assert!(selected.boundaries.same(&records.boundaries));
        let mut earlier = super::super::records::BaseCheck::new(budget);
        earlier.invocations.insert(invocation.clone()).unwrap();
        macro_rules! visit {
            ($field:ident:$ty:ty) => {
                earlier
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(
                            &records.$field.iter().cloned().collect::<Vec<_>>(),
                        )
                        .unwrap(),
                    )
                    .unwrap()
            };
        }
        visit!(evaluations:ExpressionEvaluation);
        visit!(sources:EvaluationSource);
        visit!(members:EvaluationMember);
        visit!(operands:EvaluationOperand);
        (values, earlier, invocation, row)
    }
    #[test]
    fn primitive_actual_receipt_hydrates_exact_members_and_refuses_changed_or_missing_evidence() {
        let budget = ResourceBudget::fixed(2 << 20).unwrap();
        let (values, mut earlier, frame, row) = produced(&budget);
        let actual = values.get(&row, &frame, &earlier).unwrap();
        assert_eq!(actual.truth(), Some(false));
        assert_eq!(actual.native_premises().len(), 1);
        drop(actual);
        let mut changed = row.clone();
        changed.boolean_value = Some(true);
        assert!(values.get(&changed, &frame, &earlier).is_err());
        let mut changed_frame = frame.clone();
        changed_frame.context = id(90);
        assert!(values.get(&row, &changed_frame, &earlier).is_err());
        // Replacing the selected published inventory cannot manufacture the sealed result.
        let member = EvaluationMember {
            evaluation: row.id(),
            ordinal: 0,
            source: EvaluationSource::Native { premise: id(91) }.id(),
        };
        earlier
            .visit(
                EvaluationMember::NAME,
                &EvaluationMember::encode(&[member]).unwrap(),
            )
            .unwrap_err();
        // A separate immutable view missing original evidence refuses hydration.
        let empty = super::super::records::BaseCheck::new(&budget);
        assert!(values.get(&row, &frame, &empty).is_err());
        drop(empty);
        drop(earlier);
        drop(values);
        assert_eq!(budget.reserved(), 0);
    }
}
