//! Ordered source context lifecycle consumes only early pinned protocol operations and
//! independently entered body proofs. No later Model publication is an input.
use super::{
    completion::{CheckedCompletion, CompletionRequest},
    evaluation::{self, ExpressionRequest, ReleaseSafety, boundary},
    model_application::ModelApplicationData,
    model_construction::{CheckedContextConstruction, checked_handlers::CheckedContextHandler},
    model_context::{CheckedContextProtocol, CheckedExactClass, ContextValue},
    outcome::PendingOutcome,
    source_call_records::SourceCallData,
};
use crate::domain::{
    analysis::{self, enriched_execution as publication, policy::EvidenceStatus},
    lexical::SyntaxField,
    normalized::Rows,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="context_executions",rule="context_execution",invariant_refs=super::enriched_production::context_invariants_refs)]
pub struct ContextExecution {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
    pub outcome: Id<super::enriched_records::ExecutionOutcome>,
    pub items: ContentHash,
    pub sources: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="context_execution_items",rule="context_execution_item",conclusion=execution)]
pub struct ContextItem {
    #[model(key)]
    pub execution: Id<ContextExecution>,
    #[model(key)]
    pub ordinal: i64,
    pub item: Id<Occurrence>,
    pub site: Id<Occurrence>,
    pub class: Id<calls::ProviderSymbol>,
    #[model(premise)]
    pub protocol: Id<models::AuthoredContextProtocol>,
    pub allocation: Id<calls::CallTarget>,
    pub initialization: Id<calls::CallTarget>,
    pub enumeration: Id<calls::SignatureEnumerationObservation>,
    pub entry_actual: Option<Id<Occurrence>>,
    pub exit_input: Id<super::enriched_records::ExecutionOutcome>,
    pub exit_output: Id<super::enriched_records::ExecutionOutcome>,
    pub suppressed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "context_execution_sources", rule = "context_execution_source")]
pub enum ContextSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Actual {
        #[model(premise)]
        evaluation: Id<super::records::ExpressionEvaluation>,
    },
    #[model(code = 2)]
    Body {
        #[model(premise)]
        statement: Id<super::enriched_records::StatementExecution>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="context_execution_members",rule="context_execution_member",conclusion=execution)]
pub struct ContextMember {
    #[model(key)]
    pub execution: Id<ContextExecution>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<ContextSource>,
}
impl analysis::support::sealed::DerivedEvidence for ContextExecution {}
impl analysis::support::DerivedEvidence for ContextExecution {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct CheckedContextExecution {
    record: ContextExecution,
    outcome: PendingOutcome,
    items: Rows<ContextItem>,
    sources: Rows<ContextSource>,
    members: Rows<ContextMember>,
    transitions: Vec<PendingOutcome>,
    entered: Vec<Id<Occurrence>>,
    releases: Vec<(Id<Occurrence>, ReleaseSafety)>,
    _charge: charged::StateCharge,
    _native_charge: charged::StateCharge,
}
impl CheckedContextExecution {
    pub fn record(&self) -> &ContextExecution {
        &self.record
    }
    pub fn outcome(&self) -> PendingOutcome {
        self.outcome
    }
    pub fn items(&self) -> &Rows<ContextItem> {
        &self.items
    }
    pub fn sources(&self) -> &Rows<ContextSource> {
        &self.sources
    }
    pub fn members(&self) -> &Rows<ContextMember> {
        &self.members
    }
    pub fn exit_outcomes(
        &self,
    ) -> impl Iterator<Item = super::enriched_records::ExecutionOutcome> + '_ {
        self.transitions
            .iter()
            .copied()
            .map(super::enriched_records::ExecutionOutcome::from)
    }
    pub(crate) fn entered(&self) -> &[Id<Occurrence>] {
        &self.entered
    }
    pub(crate) fn releases(&self) -> &[(Id<Occurrence>, ReleaseSafety)] {
        &self.releases
    }
    pub fn derive(
        catalog: &models::Catalog,
        application: &ModelApplicationData,
        data: &SourceCallData,
        invocation: &publication::AnalysisInvocation,
        request: CompletionRequest,
        body: &[(
            &CheckedCompletion,
            Id<super::enriched_records::StatementExecution>,
        )],
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        use ObligationKind as K;
        let facts = &data.evaluation;
        if invocation.subject.is_some()
            || (invocation.input, invocation.context) != (request.input, request.context)
        {
            return Ok(Err(K::IncompatibleContexts));
        }
        evaluation::with_completion_syntax(
            facts,
            ExpressionRequest {
                input: request.input,
                context: request.context,
                owner: request.owner,
                expression: request.statement,
            },
            budget,
            |syntax| {
                syntax.observe(request.statement)?;
                if facts
                    .occurrences
                    .get(request.statement)
                    .is_none_or(|o| o.syntax_kind != SyntaxKind::StmtWith)
                {
                    return Err(boundary(K::UnsupportedControlFlow));
                }
                let mode = syntax.detail(request.statement)?;
                if mode != Some(syntax::SyntaxDetail::WithMode { is_async: false }) {
                    return Err(boundary(K::UnsupportedControlFlow));
                }
                let children = syntax.children(request.statement)?;
                if children
                    .iter()
                    .any(|p| !matches!(p.field, SyntaxField::Item | SyntaxField::Body))
                {
                    return Err(boundary(K::MissingEvidence));
                }
                let mut charge = charged::StateCharge::new(budget, "ordered_context_execution");
                charge.grow(children.len() * 2048 + 4096)?;
                let mut items = children
                    .iter()
                    .filter(|p| p.field == SyntaxField::Item)
                    .collect::<Vec<_>>();
                items.sort_by_key(|p| p.ordinal);
                if items.is_empty()
                    || items.len() > 64
                    || items.iter().enumerate().any(|(i, p)| p.ordinal != i as i64)
                {
                    return Err(boundary(K::SummaryProofLimit));
                }
                let mut sources = Rows::new(budget);
                let mut retained = Vec::new();
                let mut status = syntax.status();
                let base = data.completed.earlier();
                for item in items {
                    syntax.observe(item.occurrence)?;
                    if facts
                        .occurrences
                        .get(item.occurrence)
                        .is_none_or(|o| o.syntax_kind != SyntaxKind::WithItem)
                    {
                        return Err(boundary(K::MissingEvidence));
                    }
                    let nodes = syntax.children(item.occurrence)?;
                    if nodes
                        .iter()
                        .any(|p| !matches!(p.field, SyntaxField::Target | SyntaxField::Value))
                        || nodes
                            .iter()
                            .filter(|p| p.field == SyntaxField::Value)
                            .count()
                            != 1
                        || nodes
                            .iter()
                            .filter(|p| p.field == SyntaxField::Target)
                            .count()
                            > 1
                    {
                        return Err(boundary(K::MissingEvidence));
                    }
                    let site = nodes
                        .iter()
                        .find(|p| p.field == SyntaxField::Value)
                        .unwrap()
                        .occurrence;
                    if let Some(target) = nodes.iter().find(|p| p.field == SyntaxField::Target) {
                        syntax.observe(target.occurrence)?;
                        if facts
                            .occurrences
                            .get(target.occurrence)
                            .is_none_or(|o| o.syntax_kind != SyntaxKind::ExprName)
                        {
                            return Err(boundary(K::UnsupportedControlFlow));
                        }
                        let target_row = facts.occurrences.get(target.occurrence).unwrap();
                        let mut definitions =
                            data.flow.definition_observations.iter().filter(|row| {
                                row.kind == lexical::BindingEventKind::WithTarget
                                    && row.value == Some(site)
                                    && data
                                        .flow
                                        .definitions
                                        .get(row.definition)
                                        .and_then(|d| data.flow.occurrences.get(d.occurrence))
                                        .is_some_and(|o| {
                                            (o.source, o.start, o.end, &o.structural_path)
                                                == (
                                                    target_row.source,
                                                    target_row.start,
                                                    target_row.end,
                                                    &target_row.structural_path,
                                                )
                                        })
                            });
                        let definition = definitions
                            .next()
                            .ok_or_else(|| boundary(K::MissingEvidence))?;
                        if definitions.next().is_some() {
                            return Err(boundary(K::AmbiguousBinding));
                        }
                        syntax.support(definition, definition.qualification, target.occurrence)?;
                        let place = data
                            .flow
                            .definitions
                            .get(definition.definition)
                            .unwrap()
                            .place;
                        if data.flow.definition_observations.iter().any(|row| {
                            row.id() != definition.id()
                                && row.scope == definition.scope
                                && data
                                    .flow
                                    .definitions
                                    .get(row.definition)
                                    .is_some_and(|d| d.place == place)
                        }) {
                            return Err(boundary(K::FrameExitCleanup));
                        }
                    }
                    let b = &application.bindings;
                    let mut initializers = b.targets.iter().filter(|target| {
                        target.site == site
                            && target.phase == calls::CallPhase::Init
                            && b.qualifications
                                .get(target.qualification)
                                .is_some_and(|q| q.context == request.context)
                    });
                    let init = initializers
                        .next()
                        .ok_or_else(|| boundary(K::UnresolvedTarget))?;
                    if initializers.next().is_some() {
                        return Err(boundary(K::AmbiguousBinding));
                    }
                    let class = init
                        .receiver_class
                        .ok_or_else(|| boundary(K::MissingEvidence))?;
                    let protocol = CheckedContextProtocol::derive(
                        catalog,
                        application,
                        class,
                        request.input,
                        request.context,
                        budget,
                    )?
                    .map_err(boundary)?;
                    let construction = CheckedContextConstruction::derive(
                        &protocol,
                        application,
                        site,
                        request.owner,
                        request.input,
                        request.context,
                        budget,
                    )?
                    .map_err(boundary)?;
                    status = analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        [status, construction.status()],
                    );
                    for premise in construction.premises().iter() {
                        sources.insert(ContextSource::Native {
                            premise: premise.id(),
                        })?;
                    }
                    for (ordinal, argument) in construction.arguments().iter().enumerate() {
                        if argument.ordinal != ordinal as i64
                            || !matches!(
                                argument.kind,
                                calls::ArgumentKind::Positional | calls::ArgumentKind::Keyword
                            )
                        {
                            return Err(boundary(K::UnsupportedUnpacking));
                        }
                        let mut evaluations = base.evaluations.iter().filter(|row| {
                            row.expression == argument.value
                                && row.owner == request.owner
                                && base.invocations.get(row.invocation).is_some_and(|parent| {
                                    (parent.input, parent.context)
                                        == (request.input, request.context)
                                })
                        });
                        let row = evaluations
                            .next()
                            .ok_or_else(|| boundary(K::MissingEvidence))?;
                        if evaluations.next().is_some() {
                            return Err(boundary(K::AmbiguousBinding));
                        }
                        let checked = base.replay(row)?;
                        if checked.exception().is_some() {
                            return Err(boundary(K::UnsupportedControlFlow));
                        }
                        if checked.release() != ReleaseSafety::Closed
                            && !base.caller_holds_argument(row)?
                            && !super::builtin_read::CheckedBuiltinRead::derive(
                                facts,
                                checked.request(),
                                budget,
                            )?
                            .is_ok()
                        {
                            return Err(boundary(K::FrameExitCleanup));
                        }
                        sources.insert(ContextSource::Actual {
                            evaluation: row.id(),
                        })?;
                        status = analysis::support::inferred_status(
                            analysis::Interpretation::Structural,
                            [status, checked.status()],
                        );
                    }
                    retained.push((
                        item.occurrence,
                        site,
                        class,
                        protocol.compiled().declaration().id(),
                        construction.allocation(),
                        construction.initialization(),
                        construction.enumeration(),
                        match construction.entry_value() {
                            ContextValue::None => None,
                            ContextValue::Actual(actual) => Some(actual),
                        },
                        PendingOutcome::Normal,
                        PendingOutcome::Normal,
                        false,
                    ));
                }
                let mut suite = children
                    .iter()
                    .filter(|p| p.field == SyntaxField::Body)
                    .collect::<Vec<_>>();
                suite.sort_by_key(|p| p.ordinal);
                if suite.is_empty() || suite.iter().enumerate().any(|(i, p)| p.ordinal != i as i64)
                {
                    return Err(boundary(K::MissingEvidence));
                }
                let mut outcome = PendingOutcome::Normal;
                let mut entered = Vec::new();
                let mut releases = Vec::new();
                for statement in suite {
                    let mut proofs = body
                        .iter()
                        .filter(|(proof, _)| proof.request().statement == statement.occurrence);
                    let (proof, id) = proofs.next().ok_or_else(|| boundary(K::MissingEvidence))?;
                    if proofs.next().is_some()
                        || (
                            proof.request().input,
                            proof.request().context,
                            proof.request().owner,
                        ) != (request.input, request.context, request.owner)
                    {
                        return Err(boundary(K::IncompatibleContexts));
                    }
                    if entered.len() + proof.entered_statements().len() > 64
                        || releases.len() + proof.release_inputs().len() > 64
                    {
                        return Err(boundary(K::SummaryProofLimit));
                    }
                    charge.grow(
                        (proof.entered_statements().len() + proof.release_inputs().len()) * 128,
                    )?;
                    entered.extend_from_slice(proof.entered_statements());
                    releases.extend_from_slice(proof.release_inputs());
                    sources.insert(ContextSource::Body { statement: *id })?;
                    status = analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        [status, proof.status()],
                    );
                    outcome = proof.outcome();
                    if !outcome.is_normal() {
                        break;
                    }
                }
                for (_, site, class, _, _, _, _, _, exit_input, exit_output, suppressed) in
                    retained.iter_mut().rev()
                {
                    *exit_input = outcome;
                    let protocol = CheckedContextProtocol::derive(
                        catalog,
                        application,
                        *class,
                        request.input,
                        request.context,
                        budget,
                    )?
                    .map_err(boundary)?;
                    let construction = CheckedContextConstruction::derive(
                        &protocol,
                        application,
                        *site,
                        request.owner,
                        request.input,
                        request.context,
                        budget,
                    )?
                    .map_err(boundary)?;
                    if let Some(exception) = outcome.exception()
                        && !protocol.preserves()
                    {
                        let (module, name) = exception.class();
                        let mut classes=application.bindings.symbols.iter().filter(|s|s.context==request.context&&s.name==name&&s.kind==calls::SymbolKind::Class&&matches!(application.bindings.provider_modules.get(s.module),Some(calls::ProviderModule::Bundled{name,bundle:calls::ModuleBundle::Typeshed,..})if name==module));
                        let symbol = classes.next().ok_or_else(|| boundary(K::MissingEvidence))?;
                        if classes.next().is_some() {
                            return Err(boundary(K::AmbiguousBinding));
                        }
                        let raised = CheckedExactClass::derive(
                            application,
                            symbol.id(),
                            request.context,
                            budget,
                        )?
                        .map_err(boundary)?;
                        for premise in raised.premises().iter() {
                            sources.insert(ContextSource::Native {
                                premise: premise.id(),
                            })?;
                        }
                        status = analysis::support::inferred_status(
                            analysis::Interpretation::Structural,
                            [status, raised.status()],
                        );
                        let occurrences = construction
                            .handler_occurrences(application, budget)?
                            .map_err(boundary)?;
                        charge.grow(occurrences.occurrences().len() * 2048)?;
                        let mut handlers = Vec::new();
                        for actual in occurrences.occurrences() {
                            let handler = CheckedContextHandler::derive(
                                &construction,
                                application,
                                facts,
                                request.input,
                                *actual,
                                budget,
                            )?
                            .map_err(boundary)?;
                            for premise in handler.class().premises().iter() {
                                sources.insert(ContextSource::Native {
                                    premise: premise.id(),
                                })?;
                            }
                            for premise in handler.lookup().native_premises() {
                                sources.insert(ContextSource::Native { premise: *premise })?;
                            }
                            status = analysis::support::inferred_status(
                                analysis::Interpretation::Structural,
                                [status, handler.class().status(), handler.lookup().status()],
                            );
                            handlers.push(handler);
                        }
                        let refs = handlers.iter().collect::<Vec<_>>();
                        if construction
                            .suppresses(application, &raised, &refs, budget)?
                            .map_err(boundary)?
                        {
                            outcome = PendingOutcome::Normal;
                            *suppressed = true;
                        }
                    }
                    *exit_output = outcome;
                }
                let qualification = syntax.qualification(request.statement).map_err(boundary)?;
                status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [status, syntax.status()],
                );
                let (native, native_charge) = syntax.take_admission();
                for premise in native {
                    sources.insert(ContextSource::Native { premise })?;
                }
                let mut digest = KeySink::new("context-items");
                for (
                    item,
                    site,
                    class,
                    protocol,
                    allocation,
                    initialization,
                    enumeration,
                    entry,
                    exit_input,
                    exit_output,
                    suppressed,
                ) in &retained
                {
                    item.encode(&mut digest);
                    site.encode(&mut digest);
                    class.encode(&mut digest);
                    protocol.encode(&mut digest);
                    allocation.encode(&mut digest);
                    initialization.encode(&mut digest);
                    enumeration.encode(&mut digest);
                    entry.encode(&mut digest);
                    super::enriched_records::ExecutionOutcome::from(*exit_input)
                        .id()
                        .encode(&mut digest);
                    super::enriched_records::ExecutionOutcome::from(*exit_output)
                        .id()
                        .encode(&mut digest);
                    suppressed.encode(&mut digest);
                }
                let record = ContextExecution {
                    invocation: invocation.id(),
                    statement: request.statement,
                    owner: request.owner,
                    qualification,
                    status,
                    outcome: super::enriched_records::ExecutionOutcome::from(outcome).id(),
                    items: digest.finish(),
                    sources: super::records::ordered_digest(
                        "context-sources",
                        sources.iter().map(Record::id),
                    ),
                };
                let transitions = retained.iter().flat_map(|item| [item.8, item.9]).collect();
                let mut items = Rows::new(budget);
                for (
                    ordinal,
                    (
                        item,
                        site,
                        class,
                        protocol,
                        allocation,
                        initialization,
                        enumeration,
                        entry_actual,
                        exit_input,
                        exit_output,
                        suppressed,
                    ),
                ) in retained.into_iter().enumerate()
                {
                    items.insert(ContextItem {
                        execution: record.id(),
                        ordinal: ordinal as i64,
                        item,
                        site,
                        class,
                        protocol,
                        allocation,
                        initialization,
                        enumeration,
                        entry_actual,
                        exit_input: super::enriched_records::ExecutionOutcome::from(exit_input)
                            .id(),
                        exit_output: super::enriched_records::ExecutionOutcome::from(exit_output)
                            .id(),
                        suppressed,
                    })?;
                }
                let mut members = Rows::new(budget);
                for (ordinal, source) in sources.iter().enumerate() {
                    members.insert(ContextMember {
                        execution: record.id(),
                        ordinal: ordinal as i64,
                        source: source.id(),
                    })?;
                }
                Ok(Self {
                    record,
                    outcome,
                    items,
                    sources,
                    members,
                    transitions,
                    entered,
                    releases,
                    _charge: charge,
                    _native_charge: native_charge,
                })
            },
        )
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ContextExecution>(),
        Relation::of::<ContextItem>(),
        Relation::of::<ContextSource>(),
        Relation::of::<ContextMember>(),
    ]
}
