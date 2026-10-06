//! Base completion publishes one exact outcome or explicit boundary for every selected statement.
//! It reconstructs earlier nominal evaluations; no source-call/enriched conclusions are inputs.
use super::{body_records::*, completion::*, completion_records::*, records::BaseCheck};
use crate::Domain;
use crate::domain::{
    analysis::{self, base_completion as publication},
    input::ArtifactUse,
    normalized::Rows,
    resources::ResourceBudget,
    source::{Occurrence, SourceArtifact, SyntaxKind},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_completion_boundaries")]
pub struct CompletionBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Option<Id<normalized::entities::EntityRef>>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="base_completion_runs",invariant_refs=run_invariants_refs,publication_refs=run_publication_checks_refs)]
pub struct CompletionRun {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    pub requested: bool,
    pub completed: i64,
    pub refused: i64,
    pub bodied: i64,
    pub body_refused: i64,
}
pub struct CompletionRecords {
    pub run: CompletionRun,
    pub completions: Rows<StatementCompletion>,
    pub outcomes: Rows<CompletionOutcome>,
    pub sources: Rows<CompletionSource>,
    pub members: Rows<CompletionMember>,
    pub entered: Rows<EnteredStatement>,
    pub boundaries: Rows<CompletionBoundary>,
    pub bodies: Rows<SourceBodyCompletion>,
    pub body_sources: Rows<BodySource>,
    pub body_members: Rows<BodyMember>,
    pub body_releases: Rows<BodyReleaseInput>,
    pub body_boundaries: Rows<BodyBoundary>,
    pub outcome: publication::AnalysisOutcome,
}
impl CompletionRecords {
    fn new(invocation: Id<publication::AnalysisInvocation>, budget: &ResourceBudget) -> Self {
        Self {
            run: CompletionRun {
                invocation,
                requested: false,
                completed: 0,
                refused: 0,
                bodied: 0,
                body_refused: 0,
            },
            completions: Rows::new(budget),
            outcomes: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            entered: Rows::new(budget),
            boundaries: Rows::new(budget),
            bodies: Rows::new(budget),
            body_sources: Rows::new(budget),
            body_members: Rows::new(budget),
            body_releases: Rows::new(budget),
            body_boundaries: Rows::new(budget),
            outcome: publication::AnalysisOutcome {
                invocation,
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            },
        }
    }
}
/// Raw stored rows cannot grant publication authority. This reader replays the existing shared
/// evaluator; the actual producer additionally captures their sealed completed source receipt.
pub struct CompletedEvaluations {
    base: BaseCheck,
    budget: ResourceBudget,
}
/// Actual body owner outputs retain compact private outcome, membership and release values.
pub struct ProducedBodies {
    frames: Rows<publication::AnalysisInvocation>,
    values: charged::ChargedMap<Id<SourceBodyCompletion>,ActualBody>,
    charge: charged::StateCharge,
}
#[derive(Clone)]
struct ActualBody {row:SourceBodyCompletion,value:super::body::ProducedBodyValue}
impl HeapSize for ActualBody {fn heap_bytes(&self)->usize {0}}
impl ProducedBodies {
    fn new(budget: &ResourceBudget) -> Self {Self {frames:Rows::new(budget),values:Default::default(),charge:charged::StateCharge::new(budget,"produced-body-values")}}
    pub fn append(&mut self,other:Self)->Result<(),ModelError> {
        if !self.charge.budget().expect("bound owner").shares_pool(other.charge.budget().expect("bound owner")) {return Err(ModelError::Conflict("produced bodies budget"));}
        for row in other.frames.iter() {self.frames.insert(row.clone())?;}
        for (id,value) in other.values.iter() {if self.values.contains_key(id) {return Err(ModelError::Conflict("produced body duplicated"));}self.values.insert(&mut self.charge,*id,value.clone())?;}
        Ok(())
    }
    pub(crate) fn visit_frame(&self,frame:&publication::AnalysisInvocation,data:&super::source_call_records::SourceCallData,mut visit:impl FnMut(&super::body::CheckedSourceBody,&SourceBodyCompletion)->Result<(),ModelError>)->Result<(),ModelError> {
        if self.frames.get(frame.id())!=Some(frame) {return Err(ModelError::Conflict("produced body frame"));}
        for value in self.values.values().filter(|value|value.row.invocation==frame.id()) {
            match data.bodies.get(value.row.id()) {None=>continue,Some(row) if row==&value.row=>{},Some(_)=>return Err(ModelError::Conflict("produced body changed"))}
            let proof=data.hydrate_body(&value.row,value.value,self.charge.budget().expect("bound owner"))?;
            visit(&proof,&value.row)?;
        }
        Ok(())
    }
}
impl CompletedEvaluations {
    pub(crate) fn earlier(&self) -> &BaseCheck {
        &self.base
    }
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            base: BaseCheck::new(budget),
            budget: budget.clone(),
        }
    }
    pub fn consumed_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
        if profile == stages::Profile::Behavioral {
            return super::records::base_invariants().remove(0).inputs;
        }
        vec![
            ValidationInput::of::<analysis::base_evaluation::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ]
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        super::require_facts_view(input)?;
        self.base.visit(input.name(), batch)
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.base.visit(name, batch)
    }
}
pub(crate) fn is_statement(kind: SyntaxKind) -> bool {
    use SyntaxKind::*;
    matches!(
        kind,
        StmtFunctionDef
            | StmtClassDef
            | StmtReturn
            | StmtDelete
            | StmtTypeAlias
            | StmtAssign
            | StmtAugAssign
            | StmtAnnAssign
            | StmtFor
            | StmtWhile
            | StmtIf
            | StmtWith
            | StmtMatch
            | StmtRaise
            | StmtTry
            | StmtAssert
            | StmtImport
            | StmtImportFrom
            | StmtGlobal
            | StmtNonlocal
            | StmtExpr
            | StmtPass
            | StmtBreak
            | StmtContinue
            | StmtIpyEscapeCommand
    )
}
pub fn complete_all(
    data: &CompletedEvaluations,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<CompletionRecords, ModelError> {
    complete_all_with_bodies(
        data,
        invocation,
        definition,
        profile,
        budget,
        &mut |_, _| Ok(()),
    )
}
pub(crate) fn complete_all_with_bodies(
    data: &CompletedEvaluations,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    on_body: &mut impl FnMut(
        &super::body::CheckedSourceBody,
        &SourceBodyCompletion,
    ) -> Result<(), ModelError>,
) -> Result<CompletionRecords, ModelError> {
    complete_with_values(data, invocation, definition, profile, budget, None, on_body,None,None,true).map(|(records, _)| records)
}
/// The compiler passes values minted by the actual base producer; this path does not replay it.
pub fn complete_all_produced(
    data: &CompletedEvaluations, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile, budget: &ResourceBudget,
    evaluations: Option<&super::production::ProducedEvaluations>,
) -> Result<(CompletionRecords, ProducedBodies), ModelError> {
    if profile == stages::Profile::Behavioral && evaluations.is_none() {
        return Err(ModelError::Conflict("base evaluation owner authority absent"));
    }
    complete_with_values(data, invocation, definition, profile, budget, evaluations, &mut |_, _| Ok(()),None,None,true)
}
/// One statement and its complete syntax/evaluation alternatives. The whole-frame run is
/// assembled by the compiler; this kernel does not claim completion of any callable body.
#[allow(clippy::too_many_arguments,reason="Actual owner, immutable frame, and selected semantic root remain explicit.")]
pub fn complete_statement_produced(data:&CompletedEvaluations,invocation:&publication::AnalysisInvocation,definition:&analysis::AnalysisDefinition,profile:stages::Profile,budget:&ResourceBudget,evaluations:&super::production::ProducedEvaluations,statement:Id<Occurrence>)->Result<(CompletionRecords,ProducedBodies),ModelError> {
    if data.base.data.occurrences.get(statement).is_none() {return Err(ModelError::Invalid("selected statement absent".into()));}
    complete_with_values(data,invocation,definition,profile,budget,Some(evaluations),&mut |_,_|Ok(()),Some(statement),None,false)
}
/// An ordered callable body is an indivisible completion kernel. Other callable metadata may
/// be dependency premises, but cannot become additional body roots in this operation.
#[allow(clippy::too_many_arguments,reason="Actual owner, immutable frame, and selected semantic root remain explicit.")]
pub fn complete_body_produced(data:&CompletedEvaluations,invocation:&publication::AnalysisInvocation,definition:&analysis::AnalysisDefinition,profile:stages::Profile,budget:&ResourceBudget,evaluations:&super::production::ProducedEvaluations,owner:Id<normalized::entities::EntityRef>)->Result<(CompletionRecords,ProducedBodies),ModelError> {
    complete_with_values(data,invocation,definition,profile,budget,Some(evaluations),&mut |_,_|Ok(()),None,Some(owner),true)
}
fn complete_with_values(
    data: &CompletedEvaluations, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile, budget: &ResourceBudget,
    evaluations: Option<&super::production::ProducedEvaluations>,
    on_body: &mut impl FnMut(&super::body::CheckedSourceBody, &SourceBodyCompletion) -> Result<(), ModelError>,
    selected_statement:Option<Id<Occurrence>>,selected_body:Option<Id<normalized::entities::EntityRef>>,emit_bodies:bool,
) -> Result<(CompletionRecords, ProducedBodies), ModelError> {
    let invalid = |message: &str| ModelError::Invalid(message.into());
    if *definition != super::configuration::base_completion().1
        || invocation.definition != definition.id()
        || invocation.subject.is_some()
    {
        return Err(invalid(
            "base completion requires its whole-frame definition",
        ));
    }
    if !data.budget.shares_pool(budget) {
        return Err(invalid(
            "completion inputs belong to a different attempt budget",
        ));
    }
    let mut output = CompletionRecords::new(invocation.id(), budget);
    let mut produced = ProducedBodies::new(budget);
    produced.frames.insert(invocation.clone())?;
    if profile != stages::Profile::Behavioral {
        output.outcome.status = analysis::AnalysisStatus::NotRequested;
        output.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok((output, produced));
    }
    output.run.requested = true;
    let earlier = &data.base;
    let facts = &earlier.data;
    let bytes = facts
        .artifacts
        .iter()
        .try_fold(0usize, |n, row| {
            n.checked_add(size_of::<SourceArtifact>())?
                .checked_add(row.heap_bytes())
        })
        .and_then(|n| n.checked_add(facts.uses.len().checked_mul(size_of::<ArtifactUse>())?))
        .and_then(|n| n.checked_mul(2))
        .and_then(|n| n.checked_add(facts.artifacts.len().checked_mul(128)?))
        .ok_or_else(|| invalid("completion source roots allowance overflow"))?;
    let _roots = budget.reserve("base_completion_roots", bytes)?;
    let artifacts = facts.artifacts.iter().cloned().collect::<Vec<_>>();
    let uses = facts.uses.iter().cloned().collect::<Vec<_>>();
    let roots = admission::analysis_roots(&artifacts, &uses)?;
    let mut charge = charged::StateCharge::new(budget, "base_completion_evaluation_inventory");
    let mut checked = Vec::new();
    let mut rows = Vec::new();
    let mut statements = Vec::new();
    for row in earlier.evaluations.iter() {
        let parent = earlier
            .invocations
            .get(row.invocation)
            .ok_or_else(|| invalid("completion earlier evaluation invocation absent"))?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        charge.grow(
            size_of::<super::evaluation::CheckedEvaluation>() * 2
                + size_of::<CompletionEvaluation<'_>>() * 2
                + size_of::<&super::evaluation::CheckedEvaluation>() * 2,
        )?;
        checked.push(match evaluations {
            Some(evaluations) => evaluations.get(row, parent, earlier)?,
            None => std::sync::Arc::new(earlier.replay(row)?),
        });
        rows.push(CompletionEvaluation {
            evaluation: row,
            invocation: parent,
        });
    }
    for row in facts.occurrences.iter().filter(|row| {
        selected_statement.is_none_or(|selected|row.id()==selected)
            && selected_body.is_none_or(|selected|facts.owners.iter().any(|owner|owner.occurrence==row.id() && owner.entity==selected))
            && is_statement(row.syntax_kind)
            && roots.contains(&row.source)
            && facts.artifacts.get(row.source).is_some_and(|source| {
                source.input == invocation.input
                    && admission::ArtifactClass::of(&source.path)
                        == Some(admission::ArtifactClass::PythonSource)
            })
    }) {
        let mut owners = facts
            .owners
            .iter()
            .filter(|owner| owner.occurrence == row.id());
        let first = owners.next();
        let owner = if owners.next().is_none() {
            first.map(|owner| owner.entity)
        } else {
            None
        };
        // Searching only the exact owner bounds each statement's evaluation work and prevents a
        // foreign callable's independently valid evaluation from entering this completion frame.
        let _scratch = budget.reserve(
            "base_completion_owner_operands",
            checked
                .len()
                .checked_mul(
                    size_of::<&super::evaluation::CheckedEvaluation>() * 2
                        + size_of::<CompletionEvaluation<'_>>() * 2,
                )
                .ok_or_else(|| invalid("completion owner scratch overflow"))?,
        )?;
        let refs = checked
            .iter()
            .filter(|proof| Some(proof.request().owner) == owner)
            .map(std::sync::Arc::as_ref)
            .collect::<Vec<_>>();
        let result = if let Some(owner) = owner {
            complete(
                facts,
                CompletionRequest {
                    input: invocation.input,
                    context: invocation.context,
                    owner,
                    statement: row.id(),
                },
                &refs,
                budget,
            )?
        } else {
            Err(obligation::ObligationKind::MissingEvidence)
        };
        match result {
            Err(reason) => {
                output.boundaries.insert(CompletionBoundary {
                    invocation: invocation.id(),
                    statement: row.id(),
                    owner,
                    reason,
                })?;
            }
            Ok(proof) => {
                let selected = proof
                    .evaluation_facts()
                    .iter()
                    .map(|facts| {
                        let mut matching = rows
                            .iter()
                            .filter(|row| facts.matches(row.evaluation, row.invocation));
                        let row = matching.next().ok_or_else(|| {
                            invalid("completion entered evaluation mapping absent")
                        })?;
                        if matching.next().is_some() {
                            return Err(invalid("completion entered evaluation mapping ambiguous"));
                        }
                        Ok(CompletionEvaluation {
                            evaluation: row.evaluation,
                            invocation: row.invocation,
                        })
                    })
                    .collect::<Result<Vec<_>, ModelError>>()?;
                let records = proof.emit_base(invocation, definition, &selected, budget)?;
                output.completions.insert(records.completion)?;
                output.outcomes.insert(records.outcome)?;
                for row in records.sources {
                    output.sources.insert(row)?;
                }
                for row in records.members {
                    output.members.insert(row)?;
                }
                for row in records.entered {
                    output.entered.insert(row)?;
                }
                charge.grow(size_of::<super::completion::CheckedCompletion>() * 2)?;
                statements.push(proof);
            }
        }
    }
    let _body_scratch = budget.reserve(
        "base_source_body_statement_refs",
        statements
            .len()
            .checked_mul(size_of::<&super::completion::CheckedCompletion>() * 2)
            .ok_or_else(|| invalid("source body statement scratch overflow"))?,
    )?;
    let statement_refs = statements.iter().collect::<Vec<_>>();
    for callable in facts.callables.iter().filter(|callable|emit_bodies && selected_body.is_none_or(|selected|normalized::entities::EntityRef::Callable {callable:callable.id()}.id()==selected)) {
        let normalized::entities::CallableEntity::Source { declaration, .. } = callable else {
            continue;
        };
        let source = facts
            .occurrences
            .get(*declaration)
            .ok_or_else(|| invalid("source body declaration absent"))?
            .source;
        if !roots.contains(&source)
            || facts.artifacts.get(source).is_none_or(|row| {
                row.input != invocation.input
                    || admission::ArtifactClass::of(&row.path)
                        != Some(admission::ArtifactClass::PythonSource)
            })
        {
            continue;
        }
        let owner = normalized::entities::EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        match super::body::complete_body(
            facts,
            super::body::SourceBodyRequest {
                input: invocation.input,
                context: invocation.context,
                callee: owner,
            },
            &statement_refs,
            budget,
        )? {
            Err(reason) => {
                output.body_boundaries.insert(BodyBoundary {
                    invocation: invocation.id(),
                    owner,
                    declaration: *declaration,
                    reason,
                })?;
            }
            Ok(proof) => {
                let rows =
                    super::body_records::emit(&proof, invocation, &output.completions, budget)?;
                on_body(&proof, &rows.body)?;
                produced.values.insert(&mut produced.charge,rows.body.id(),ActualBody {row:rows.body.clone(),value:super::body::ProducedBodyValue::of(&proof)})?;
                output.bodies.insert(rows.body)?;
                output.outcomes.insert(rows.outcome)?;
                for row in rows.sources {
                    output.body_sources.insert(row)?;
                }
                for row in rows.members {
                    output.body_members.insert(row)?;
                }
                for row in rows.releases {
                    output.body_releases.insert(row)?;
                }
            }
        }
    }
    output.run.bodied = output
        .bodies
        .len()
        .try_into()
        .map_err(|_| invalid("source body count exceeds signed domain"))?;
    output.run.body_refused = output
        .body_boundaries
        .len()
        .try_into()
        .map_err(|_| invalid("source body boundary count exceeds signed domain"))?;
    output.run.completed = output
        .completions
        .len()
        .try_into()
        .map_err(|_| invalid("completion count exceeds signed domain"))?;
    output.run.refused = output
        .boundaries
        .len()
        .try_into()
        .map_err(|_| invalid("completion boundary count exceeds signed domain"))?;
    if !output.boundaries.is_empty() || !output.body_boundaries.is_empty() {
        output.outcome.status = analysis::AnalysisStatus::Partial;
        output.outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
    }
    Ok((output, produced))
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<CompletionRun>(),
        Relation::of::<CompletionBoundary>(),
    ]
}

fn run_inputs() -> Vec<ValidationInput> {
    let mut inputs = super::completion_records::completion_invariants()
        .remove(0)
        .inputs;
    inputs.extend([
        ValidationInput::of::<CompletionRun>(&["id"]),
        ValidationInput::of::<CompletionBoundary>(&["id"]),
        ValidationInput::of::<publication::AnalysisOutcome>(&["id"]),
    ]);
    inputs.extend([
        ValidationInput::of::<SourceBodyCompletion>(&["id"]),
        ValidationInput::of::<BodySource>(&["id"]),
        ValidationInput::of::<BodyMember>(&["id"]),
        ValidationInput::of::<BodyReleaseInput>(&["id"]),
        ValidationInput::of::<BodyBoundary>(&["id"]),
    ]);
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    inputs
}
pub(crate) fn inventory_inputs() -> Vec<ValidationInput> {
    run_inputs()
}
pub(crate) fn inventory_check(budget: &ResourceBudget) -> Box<dyn InvariantCheck> {
    Box::new(CompletionRunCheck::new(budget))
}
pub(crate) fn run_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "base_completion_inventory",
        inputs: run_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(CompletionRunCheck::new(budget))),
    }]
}
struct CompletionRunCheck {
    data: CompletedEvaluations,
    invocations: Rows<publication::AnalysisInvocation>,
    definitions: Rows<analysis::AnalysisDefinition>,
    runs: Rows<CompletionRun>,
    completions: Rows<StatementCompletion>,
    outcomes: Rows<CompletionOutcome>,
    sources: Rows<CompletionSource>,
    members: Rows<CompletionMember>,
    entered: Rows<EnteredStatement>,
    boundaries: Rows<CompletionBoundary>,
    bodies: Rows<SourceBodyCompletion>,
    body_sources: Rows<BodySource>,
    body_members: Rows<BodyMember>,
    body_releases: Rows<BodyReleaseInput>,
    body_boundaries: Rows<BodyBoundary>,
    results: Rows<publication::AnalysisOutcome>,
    budget: ResourceBudget,
}
impl CompletionRunCheck {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            data: CompletedEvaluations::new(budget),
            invocations: Rows::new(budget),
            definitions: Rows::new(budget),
            runs: Rows::new(budget),
            completions: Rows::new(budget),
            outcomes: Rows::new(budget),
            sources: Rows::new(budget),
            members: Rows::new(budget),
            entered: Rows::new(budget),
            boundaries: Rows::new(budget),
            bodies: Rows::new(budget),
            body_sources: Rows::new(budget),
            body_members: Rows::new(budget),
            body_releases: Rows::new(budget),
            body_boundaries: Rows::new(budget),
            results: Rows::new(budget),
            budget: budget.clone(),
        }
    }
}
impl InvariantCheck for CompletionRunCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.data.visit(name, batch)?;
        macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        rows! {invocations:publication::AnalysisInvocation,definitions:analysis::AnalysisDefinition,runs:CompletionRun,completions:StatementCompletion,outcomes:CompletionOutcome,sources:CompletionSource,members:CompletionMember,entered:EnteredStatement,boundaries:CompletionBoundary,bodies:SourceBodyCompletion,body_sources:BodySource,body_members:BodyMember,body_releases:BodyReleaseInput,body_boundaries:BodyBoundary,results:publication::AnalysisOutcome,}
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let invalid = |message: &str| ModelError::Invalid(message.into());
        let mut charge = charged::StateCharge::new(&self.budget, "base_completion_frames");
        let mut frames = charged::ChargedSet::default();
        for row in self.data.base.invocations.iter() {
            frames.insert(&mut charge, (row.input, row.context))?;
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
                "base completion omitted or duplicated a base evaluation frame",
            ));
        }
        let mut runs = Rows::new(&self.budget);
        let mut completions = Rows::new(&self.budget);
        let mut outcomes = Rows::new(&self.budget);
        let mut sources = Rows::new(&self.budget);
        let mut members = Rows::new(&self.budget);
        let mut entered = Rows::new(&self.budget);
        let mut boundaries = Rows::new(&self.budget);
        let mut results = Rows::new(&self.budget);
        let mut bodies = Rows::new(&self.budget);
        let mut body_sources = Rows::new(&self.budget);
        let mut body_members = Rows::new(&self.budget);
        let mut body_releases = Rows::new(&self.budget);
        let mut body_boundaries = Rows::new(&self.budget);
        for invocation in self.invocations.iter() {
            let mut parents = Rows::new(&self.budget);
            for base in
                self.data.base.invocations.iter().filter(|row| {
                    (row.input, row.context) == (invocation.input, invocation.context)
                })
            {
                parents.insert(publication::InvocationSource::BaseEvaluation {
                    invocation: base.id(),
                })?;
            }
            let mut key = KeySink::new("analysis-invocation-inputs");
            for row in parents.iter() {
                row.id().encode(&mut key);
            }
            if invocation.inputs != key.finish() {
                return Err(invalid("base completion changes exact evaluation parents"));
            }
            let run = self
                .runs
                .iter()
                .find(|r| r.invocation == invocation.id())
                .ok_or_else(|| invalid("base completion run missing"))?;
            let definition = self
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("base completion definition missing"))?;
            let expected = complete_all(
                &self.data,
                invocation,
                definition,
                if run.requested {
                    stages::Profile::Behavioral
                } else {
                    stages::Profile::Catalog
                },
                &self.budget,
            )?;
            runs.insert(expected.run)?;
            results.insert(expected.outcome)?;
            macro_rules! append{($($field:ident,)*)=>{$(for row in expected.$field.iter(){$field.insert(row.clone())?;})*};}
            append! {completions,outcomes,sources,members,entered,boundaries,bodies,body_sources,body_members,body_releases,body_boundaries,}
        }
        if !self.runs.same(&runs)
            || !self.results.same(&results)
            || !self.completions.same(&completions)
            || !self.outcomes.same(&outcomes)
            || !self.sources.same(&sources)
            || !self.members.same(&members)
            || !self.entered.same(&entered)
            || !self.boundaries.same(&boundaries)
            || !self.bodies.same(&bodies)
            || !self.body_sources.same(&body_sources)
            || !self.body_members.same(&body_members)
            || !self.body_releases.same(&body_releases)
            || !self.body_boundaries.same(&body_boundaries)
        {
            return Err(invalid(
                "base completion inventory differs from captured statement universe",
            ));
        }
        Ok(())
    }
}
pub(crate) fn run_publication_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "base_completion_request_profile",
        inputs: vec![
            ValidationInput::of::<CompletionRun>(&["id"]),
            ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CompletionProfileCheck {
                runs: Rows::new(budget),
                invocations: Rows::new(budget),
            })
        }),
    }]
}
struct CompletionProfileCheck {
    runs: Rows<CompletionRun>,
    invocations: Rows<publication::AnalysisInvocation>,
}
impl PublicationCheck for CompletionProfileCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == CompletionRun::NAME {
            self.runs.decode(batch)?;
        } else if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        } else {
            return Err(ModelError::Invalid(
                "undeclared base completion profile input".into(),
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
                "base completion differs from actual publication profile".into(),
            ));
        }
        Ok(())
    }
}

/// Finite owner publication: predecessor closure is read-only; this stage never writes earlier evaluation.
pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    if *definition != super::configuration::base_completion().1 {
        return Err(ModelError::Invalid(
            "base completion stage definition is unbound".into(),
        ));
    }
    let mut outputs = publication::publication_relations();
    outputs.extend(super::completion_records::relations());
    outputs.extend(super::body_records::relations());
    outputs.extend(relations());
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let mut initial = if profile == Profile::Behavioral {
        run_inputs()
    } else {
        vec![
            ValidationInput::of::<analysis::base_evaluation::AnalysisInvocation>(&["id"]),
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
    let mut key = KeySink::new("base-completion-definition");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "complete_base",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("completion_production.rs")),
        configuration: key.finish(),
    })
}

pub(crate) fn run_invariants_refs() -> Vec<&'static str> {
    vec!["base_completion_inventory"]
}
pub(crate) fn run_publication_checks_refs() -> Vec<&'static str> {
    vec!["base_completion_request_profile"]
}

#[cfg(test)]
mod actual_body_controls {
    use super::*;
    use crate::domain::{analysis::native::NativeInventory,assertion::*,attribution::*,input::SourceRole,normalized::entities::*,syntax::*};
    fn id<R>(n:u8)->Id<R> {serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([n;16].into_iter())).unwrap()}
    #[test]
    fn selected_statement_and_body_match_finite_whole_owner_and_hydrate_only_exact_published_members() {
        let budget=ResourceBudget::fixed(4<<20).unwrap();
        let artifact=SourceArtifact::from_bytes(id(1),"body.py".into(),b"def f():\n pass\n").unwrap();let scope=source::CoverageScope::Artifact {artifact:artifact.id()};
        let q=AssertionQualification {context:id(2),scope:scope.id(),assumptions:assumptions::AssumptionSet::empty_id(),condition:conditions::Diagram::always().id(),modality:Modality::Definite,approximation:Approximation::Exact};
        let declaration=Occurrence {source:artifact.id(),start:0,end:14,syntax_kind:SyntaxKind::StmtFunctionDef,role:source::OccurrenceRole::Declaration,structural_path:vec![0]};
        let name=Occurrence {start:4,end:5,syntax_kind:SyntaxKind::ExprName,role:source::OccurrenceRole::Syntax,structural_path:vec![0,0],..declaration.clone()};
        let statement=Occurrence {start:10,end:13,syntax_kind:SyntaxKind::StmtPass,role:source::OccurrenceRole::Syntax,structural_path:vec![0,1],..declaration.clone()};
        let callable=CallableEntity::Source {declaration:declaration.id(),kind:CallableKind::Function};let owner=EntityRef::Callable {callable:callable.id()};
        let placements=[SyntaxPlacement {qualification:q.id(),occurrence:declaration.id(),parent:None,field:lexical::SyntaxField::Body,ordinal:0},SyntaxPlacement {qualification:q.id(),occurrence:statement.id(),parent:Some(declaration.id()),field:lexical::SyntaxField::Body,ordinal:0}];
        let declaration_syntax=DeclarationObservation {qualification:q.id(),declaration:declaration.id(),name:name.id(),kind:DeclarationKind::Function,parent:None,overload:false,docstring:None};
        let supports=placements.iter().map(|row|SyntaxPlacementSupport {assertion:row.id(),run:id(3),surface:id(4),evidence:id(5),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural}).collect::<Vec<_>>();
        let declaration_support=DeclarationSupport {assertion:declaration_syntax.id(),run:id(3),surface:id(4),evidence:id(5),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural};
        let mut native=NativeInventory::new(&budget);
        macro_rules! native {($ty:ty,$rows:expr)=>{native.visit(<$ty>::NAME,&<$ty>::encode($rows).unwrap()).unwrap()};}
        native!(AssertionQualification,std::slice::from_ref(&q));native!(SyntaxPlacement,&placements);native!(SyntaxPlacementSupport,&supports);native!(DeclarationObservation,std::slice::from_ref(&declaration_syntax));native!(DeclarationSupport,&[declaration_support]);let native=native.collect().unwrap();
        let mut facts=super::super::evaluation::EvaluationData::new(&budget);facts.native=native.qualifications;facts.premises=native.premises;
        facts.artifacts.insert(artifact.clone()).unwrap();facts.uses.insert(ArtifactUse {artifact:artifact.id(),input:artifact.input,role:SourceRole::Release}).unwrap();facts.scopes.insert(scope).unwrap();facts.qualifications.insert(q.clone()).unwrap();facts.callables.insert(callable).unwrap();facts.refs.insert(owner.clone()).unwrap();facts.declarations.insert(declaration_syntax).unwrap();
        for row in [declaration.clone(),statement.clone()] {facts.occurrences.insert(row.clone()).unwrap();facts.owners.insert(OccurrenceOwnership {occurrence:row.id(),owner:declaration.id(),entity:owner.id()}).unwrap();}facts.occurrences.insert(name).unwrap();for row in placements {facts.placements.insert(row).unwrap();}
        let base_definition=super::super::configuration::base_evaluation().1;let base=analysis::base_evaluation::AnalysisInvocation::new(artifact.input,q.context,base_definition.id(),None,[]).0;
        let (_,values)=super::super::production::evaluate_all_produced(&facts,&conditions::entry::EntryData::new(&budget),&Rows::new(&budget),&Rows::new(&budget),&base,&base_definition,stages::Profile::Behavioral,&budget).unwrap();
        let mut data=CompletedEvaluations::new(&budget);
        macro_rules! visit {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty>::encode(&facts.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}crate::execution_evaluation_inputs!(visit);
        data.visit(analysis::base_evaluation::AnalysisInvocation::NAME,&analysis::base_evaluation::AnalysisInvocation::encode(&[base]).unwrap()).unwrap();
        let definition=super::super::configuration::base_completion().1;let frame=publication::AnalysisInvocation::new(artifact.input,q.context,definition.id(),None,[]).0;
        let (whole,bodies)=complete_all_produced(&data,&frame,&definition,stages::Profile::Behavioral,&budget,Some(&values)).unwrap();assert_eq!(whole.run.bodied,1);assert_eq!(whole.run.body_refused,0);
        let (selected,_)=complete_statement_produced(&data,&frame,&definition,stages::Profile::Behavioral,&budget,&values,statement.id()).unwrap();
        let mut expected=Rows::new(&budget);for row in whole.completions.iter().filter(|row|row.statement==statement.id()) {expected.insert(row.clone()).unwrap();}assert!(selected.completions.same(&expected));assert!(selected.bodies.is_empty());drop(selected);
        let (selected,_)=complete_body_produced(&data,&frame,&definition,stages::Profile::Behavioral,&budget,&values,owner.id()).unwrap();assert!(selected.bodies.same(&whole.bodies));assert!(selected.body_members.same(&whole.body_members));assert!(selected.body_sources.same(&whole.body_sources));assert!(selected.body_releases.same(&whole.body_releases));
        let mut stored=super::super::source_call_records::SourceCallData::new(&budget);stored.visit(publication::AnalysisInvocation::NAME,&publication::AnalysisInvocation::encode(std::slice::from_ref(&frame)).unwrap()).unwrap();
        macro_rules! stored {($field:ident:$ty:ty)=>{stored.visit(<$ty>::NAME,&<$ty>::encode(&whole.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap()};}
        stored!(bodies:SourceBodyCompletion);stored!(body_sources:BodySource);stored!(body_members:BodyMember);stored!(body_releases:BodyReleaseInput);stored!(completions:StatementCompletion);
        let mut seen=0;bodies.visit_frame(&frame,&stored,|proof,row|{seen+=1;assert_eq!(proof.request().callee,owner.id());assert_eq!(proof.entered_statements(),&[statement.id()]);assert_eq!(proof.declaration(),row.declaration);Ok(())}).unwrap();assert_eq!(seen,1);
        let body=whole.bodies.iter().next().unwrap();let mut changed=body.clone();changed.status=analysis::policy::EvidenceStatus::Unresolved;stored.bodies=Rows::new(&budget);stored.bodies.insert(changed).unwrap();assert!(bodies.visit_frame(&frame,&stored,|_,_|Ok(())).is_err());
        stored.bodies=Rows::new(&budget);stored.bodies.insert(body.clone()).unwrap();stored.body_members=Rows::new(&budget);assert!(bodies.visit_frame(&frame,&stored,|_,_|Ok(())).is_err());
        drop(stored);drop(expected);drop(selected);drop(whole);drop(bodies);drop(data);drop(facts);drop(values);assert_eq!(budget.reserved(),0);
    }
}
