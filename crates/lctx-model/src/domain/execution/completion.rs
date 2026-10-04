//! The base completion channel consumes independently admitted base evaluations. It does not
//! evaluate source calls, read enriched outcomes, or use its own body outcome as a premise.
use super::{
    ExactRuntimeException,
    evaluation::{
        CheckedEvaluation, EvaluationData, EvaluationError, Evaluator, ExpressionRequest,
        PreparedExecution, ReleaseSafety, boundary,
    },
    outcome::PendingOutcome,
};
use crate::domain::{
    analysis::native::NativeAssertionPremise,
    lexical::SyntaxField as F,
    normalized::entities::EntityRef,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind as S},
    syntax::SyntaxPlacement,
    *,
};

pub const COMPLETION_DEPTH_LIMIT: usize = 128;
pub const COMPLETION_WORK_LIMIT: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionRequest {
    pub input: Id<input::InputRevision>,
    pub context: Id<attribution::AnalysisContext>,
    pub owner: Id<EntityRef>,
    pub statement: Id<Occurrence>,
}
pub struct CheckedCompletion {
    request: CompletionRequest,
    outcome: PendingOutcome,
    native: Vec<Id<NativeAssertionPremise>>,
    statements: Vec<Id<Occurrence>>,
    expressions: Vec<Id<Occurrence>>,
    releases: Vec<(Id<Occurrence>, ReleaseSafety)>,
    headers: Vec<Id<super::source_call_records::SourceCallHeader>>,
    definitions: Vec<Id<super::definition::DefinitionEvaluation>>,
    contexts: Vec<Id<super::context_execution::ContextExecution>>,
    facts: Vec<super::records::EvaluationFacts>,
    qualification: Id<assertion::AssertionQualification>,
    status: analysis::policy::EvidenceStatus,
    _charge: charged::StateCharge,
    _results_charge: charged::StateCharge,
}
impl CheckedCompletion {
    pub fn request(&self) -> CompletionRequest {
        self.request
    }
    pub fn outcome(&self) -> PendingOutcome {
        self.outcome
    }
    pub fn native_premises(&self) -> &[Id<NativeAssertionPremise>] {
        &self.native
    }
    pub fn entered_statements(&self) -> &[Id<Occurrence>] {
        &self.statements
    }
    pub fn evaluated_expressions(&self) -> &[Id<Occurrence>] {
        &self.expressions
    }
    pub fn release_inputs(&self) -> &[(Id<Occurrence>, ReleaseSafety)] {
        &self.releases
    }
    pub fn qualification(&self) -> Id<assertion::AssertionQualification> {
        self.qualification
    }
    pub fn status(&self) -> analysis::policy::EvidenceStatus {
        self.status
    }
    pub(crate) fn context_premises(&self) -> &[Id<super::context_execution::ContextExecution>] {
        &self.contexts
    }
    pub(crate) fn definition_premises(&self) -> &[Id<super::definition::DefinitionEvaluation>] {
        &self.definitions
    }
    pub(crate) fn header_premises(&self) -> &[Id<super::source_call_records::SourceCallHeader>] {
        &self.headers
    }
    pub(crate) fn evaluation_facts(&self) -> &[super::records::EvaluationFacts] {
        &self.facts
    }
}
struct Kernel<'a, 'b, 'c> {
    available_headers: &'a [(
        &'a super::source_call::CheckedSourceBinding,
        &'a super::source_call_records::SourceCallHeader,
    )],
    headers: Vec<Id<super::source_call_records::SourceCallHeader>>,
    definitions: Vec<Id<super::definition::DefinitionEvaluation>>,
    contexts: Vec<Id<super::context_execution::ContextExecution>>,
    data: &'a EvaluationData,
    request: CompletionRequest,
    evaluations: &'a [&'a CheckedEvaluation],
    available_definitions: &'a [super::definition::CheckedDefinition],
    available_contexts: &'a [super::context_execution::CheckedContextExecution],
    syntax: &'b mut Evaluator<'c>,
    statements: Vec<Id<Occurrence>>,
    expressions: Vec<Id<Occurrence>>,
    releases: Vec<(Id<Occurrence>, ReleaseSafety)>,
    facts: Vec<super::records::EvaluationFacts>,
    status: analysis::policy::EvidenceStatus,
    charge: charged::StateCharge,
    active: Option<ExactRuntimeException>,
}
type Result<T> = std::result::Result<T, EvaluationError>;
fn one(nodes: &[SyntaxPlacement], field: F) -> Result<Id<Occurrence>> {
    let mut rows = nodes.iter().filter(|row| row.field == field);
    let row = rows
        .next()
        .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?;
    if rows.next().is_some() {
        return Err(boundary(ObligationKind::MissingEvidence));
    }
    Ok(row.occurrence)
}
impl Kernel<'_, '_, '_> {
    fn expression(&mut self, expression: Id<Occurrence>) -> Result<&CheckedEvaluation> {
        self.syntax.tick(self.evaluations.len()).map_err(boundary)?;
        let mut matching = self
            .evaluations
            .iter()
            .filter(|proof| proof.request().expression == expression);
        let proof = *matching
            .next()
            .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
        if matching.next().is_some()
            || proof.request().input != self.request.input
            || proof.request().context != self.request.context
            || proof.request().owner != self.request.owner
        {
            return Err(boundary(ObligationKind::IncompatibleContexts));
        }
        if self.expressions.len() == 64 {
            return Err(boundary(ObligationKind::SummaryProofLimit));
        }
        self.charge.grow(
            size_of::<Id<Occurrence>>() * 2
                + size_of::<(Id<Occurrence>, ReleaseSafety)>() * 2
                + size_of::<super::records::EvaluationFacts>() * 2,
        )?;
        self.expressions.push(expression);
        self.releases.push((expression, proof.release()));
        self.facts.push(super::records::EvaluationFacts::of(proof));
        self.status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [self.status, proof.status()],
        );
        Ok(proof)
    }
    fn raise_expression(
        &mut self,
        expression: Id<Occurrence>,
    ) -> Result<(ExactRuntimeException, Option<PendingOutcome>)> {
        if self
            .data
            .occurrences
            .get(expression)
            .is_some_and(|node| node.syntax_kind == S::ExprCall)
            && !self
                .evaluations
                .iter()
                .any(|proof| proof.request().expression == expression)
        {
            self.syntax.observe(expression)?;
            let children = self.syntax.children(expression)?;
            let class = self
                .expression(one(&children, F::Callee)?)?
                .class_value()
                .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?;
            let kind = self.exact_exception_class(class)?;
            let arguments = self.syntax.positional_arguments(&children)?;
            for argument in arguments {
                if let Some((site, exception)) = self.expression(argument.occurrence)?.exception() {
                    return Ok((kind, Some(PendingOutcome::Raise { site, exception })));
                }
            }
            return Ok((kind, None));
        }
        let proof = self.expression(expression)?;
        let abrupt = proof
            .exception()
            .map(|(site, exception)| PendingOutcome::Raise { site, exception });
        let instance = proof.raised_value();
        let class = proof.class_value();
        if abrupt.is_some() {
            return Ok((ExactRuntimeException::TypeError, abrupt));
        }
        let kind = if let Some(kind) = instance {
            kind
        } else if let Some(class) = class {
            self.exact_exception_class(class)?
        } else if self.data.occurrences.get(expression).is_some_and(|node| {
            matches!(
                node.syntax_kind,
                S::ExprNoneLiteral
                    | S::ExprBooleanLiteral
                    | S::ExprNumberLiteral
                    | S::ExprStringLiteral
                    | S::ExprBytesLiteral
                    | S::ExprEllipsisLiteral
            )
        }) {
            ExactRuntimeException::TypeError
        } else {
            return Err(boundary(ObligationKind::UnsupportedControlFlow));
        };
        Ok((kind, None))
    }
    fn exact_exception_class(
        &mut self,
        class: Id<calls::ProviderSymbol>,
    ) -> Result<ExactRuntimeException> {
        let symbol = self
            .data
            .symbols
            .get(class)
            .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
        for kind in ExactRuntimeException::ALL {
            if symbol.name == kind.class().1 && self.syntax.builtin_class(kind.class().1)? == class
            {
                return Ok(*kind);
            }
        }
        Err(boundary(ObligationKind::UnsupportedControlFlow))
    }
    fn handler_classes(
        &mut self,
        expression: Id<Occurrence>,
        depth: usize,
        classes: &mut Vec<Id<calls::ProviderSymbol>>,
    ) -> Result<Option<PendingOutcome>> {
        if depth > COMPLETION_DEPTH_LIMIT {
            return Err(boundary(ObligationKind::CompletionDepthLimit));
        }
        if self
            .data
            .occurrences
            .get(expression)
            .is_some_and(|node| node.syntax_kind == S::ExprTuple)
        {
            self.syntax.observe(expression)?;
            let children = self.syntax.children(expression)?;
            for (ordinal, child) in children.iter().enumerate() {
                if child.field != F::Element || child.ordinal != ordinal as i64 {
                    return Err(boundary(ObligationKind::MissingEvidence));
                }
                if self
                    .data
                    .occurrences
                    .get(child.occurrence)
                    .is_some_and(|node| node.syntax_kind == S::ExprTuple)
                {
                    return Err(boundary(ObligationKind::UnsupportedControlFlow));
                }
                if let Some(outcome) = self.handler_classes(child.occurrence, depth + 1, classes)? {
                    return Ok(Some(outcome));
                }
            }
        } else {
            let proof = self.expression(expression)?;
            if let Some((site, exception)) = proof.exception() {
                return Ok(Some(PendingOutcome::Raise { site, exception }));
            }
            let class = proof
                .class_value()
                .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?;
            if classes.len() >= 64 {
                return Err(boundary(ObligationKind::SummaryProofLimit));
            }
            self.charge
                .grow(size_of::<Id<calls::ProviderSymbol>>() * 2)?;
            classes.push(class);
        }
        Ok(None)
    }
    fn handler_cleanup(&mut self, handler: Id<Occurrence>, name: &str) -> Result<()> {
        let occurrence = self
            .data
            .occurrences
            .get(handler)
            .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
        self.syntax
            .tick(self.data.binding_events.len())
            .map_err(boundary)?;
        self.syntax
            .tick(self.data.bindings.len())
            .map_err(boundary)?;
        let mut matches = self.data.bindings.iter().filter(|binding| {
            binding.kind == lexical::BindingEventKind::ExceptHandler
                && self
                    .data
                    .binding_events
                    .get(binding.event)
                    .is_some_and(|event| event.site == handler && event.name == name)
        });
        let binding = matches
            .next()
            .ok_or_else(|| boundary(ObligationKind::HandlerNameCleanup))?;
        if matches.next().is_some() {
            return Err(boundary(ObligationKind::HandlerNameCleanup));
        }
        self.syntax
            .support(binding, binding.qualification, handler)?;
        // Rebinding an already occupied local can release an unknown object on handler entry.
        if self.data.bindings.iter().any(|other| {
            other.id() != binding.id()
                && other.scope == binding.scope
                && self
                    .data
                    .binding_events
                    .get(other.event)
                    .is_some_and(|event| event.name == name)
        }) {
            return Err(boundary(ObligationKind::HandlerNameCleanup));
        }
        // A builtin exception with a closed argument inventory has no user-defined disposal.
        // Writes/captures of the handler name or retained arguments keep the cleanup boundary open.
        self.syntax
            .tick(
                self.expressions
                    .len()
                    .saturating_mul(self.evaluations.len()),
            )
            .map_err(boundary)?;
        self.syntax
            .tick(self.data.references.len())
            .map_err(boundary)?;
        if self.expressions.iter().any(|expression| {
            self.evaluations.iter().any(|proof| {
                proof.request().expression == *expression
                    && proof.release() != ReleaseSafety::Closed
                    && proof.class_value().is_none()
            })
        }) || self.data.binding_events.iter().any(|event| {
            event.name == name
                && event.site != handler
                && self.data.occurrences.get(event.site).is_some_and(|node| {
                    node.source == occurrence.source
                        && node.start >= occurrence.start
                        && node.end <= occurrence.end
                })
        }) || self.data.references.iter().any(|reference| {
            reference.name == name
                && self
                    .data
                    .occurrences
                    .get(reference.read)
                    .is_some_and(|node| {
                        node.source == occurrence.source
                            && node.start >= occurrence.start
                            && node.end <= occurrence.end
                    })
        }) {
            return Err(boundary(ObligationKind::HandlerNameCleanup));
        }
        Ok(())
    }
    fn suite(
        &mut self,
        children: &[SyntaxPlacement],
        field: F,
        depth: usize,
    ) -> Result<PendingOutcome> {
        self.charge.grow(
            children
                .len()
                .checked_mul(size_of::<&SyntaxPlacement>() * 2)
                .ok_or_else(|| ModelError::Invalid("completion suite allowance overflow".into()))?,
        )?;
        let mut statements = children
            .iter()
            .filter(|row| row.field == field)
            .collect::<Vec<_>>();
        statements.sort_by_key(|row| (row.ordinal, row.occurrence));
        for (ordinal, row) in statements.iter().enumerate() {
            if row.ordinal != ordinal as i64 {
                return Err(boundary(ObligationKind::MissingEvidence));
            }
        }
        for row in statements {
            let result = self.statement(row.occurrence, depth + 1)?;
            if !result.is_normal() {
                return Ok(result);
            }
        }
        Ok(PendingOutcome::Normal)
    }
    fn statement(&mut self, site: Id<Occurrence>, depth: usize) -> Result<PendingOutcome> {
        if depth > COMPLETION_DEPTH_LIMIT {
            return Err(boundary(ObligationKind::CompletionDepthLimit));
        }
        self.syntax.observe(site)?;
        let kind = self
            .data
            .occurrences
            .get(site)
            .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?
            .syntax_kind;
        let children = self.syntax.children(site)?;
        let outcome = match kind {
            S::StmtPass if children.is_empty() => PendingOutcome::Normal,
            S::StmtAssign => {
                let mut admitted = self.available_headers.iter().filter(|(proof, _)| {
                    proof.caller() == self.request.owner
                        && proof.request().input == self.request.input
                        && proof.request().context == self.request.context
                        && proof.captures().iter().any(|origin| {
                            origin
                                .literal_prefix()
                                .is_some_and(|(statement, _)| statement == site)
                        })
                });
                let (proof, row) = admitted
                    .next()
                    .ok_or_else(|| boundary(ObligationKind::CapturedStateUnavailable))?;
                if admitted.next().is_some() {
                    return Err(boundary(ObligationKind::AmbiguousBinding));
                }
                let value = one(&children, F::Value)?;
                if !proof
                    .captures()
                    .iter()
                    .any(|origin| origin.literal_prefix() == Some((site, value)))
                {
                    return Err(boundary(ObligationKind::CapturedStateUnavailable));
                }
                self.expression(value)?;
                self.charge
                    .grow(size_of::<Id<super::source_call_records::SourceCallHeader>>() * 2)?;
                self.headers.push(row.id());
                self.status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [self.status, proof.status()],
                );
                PendingOutcome::Normal
            }
            S::StmtWith => {
                let mut proofs = self.available_contexts.iter().filter(|proof| {
                    proof.record().statement == site && proof.record().owner == self.request.owner
                });
                let proof = proofs
                    .next()
                    .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
                if proofs.next().is_some() {
                    return Err(boundary(ObligationKind::AmbiguousBinding));
                }
                self.charge.grow(
                    std::mem::size_of_val(proof.entered()) * 2
                        + std::mem::size_of_val(proof.releases()) * 2
                        + size_of::<Id<super::context_execution::ContextExecution>>() * 2,
                )?;
                self.contexts.push(proof.record().id());
                self.statements.extend_from_slice(proof.entered());
                self.releases.extend_from_slice(proof.releases());
                self.status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [self.status, proof.record().status],
                );
                proof.outcome()
            }
            S::StmtFunctionDef => {
                if let Some(proof) = self.available_definitions.iter().find(|proof| {
                    proof.record().statement == site && proof.record().owner == self.request.owner
                }) {
                    self.charge
                        .grow(size_of::<Id<super::definition::DefinitionEvaluation>>() * 2)?;
                    self.definitions.push(proof.record().id());
                    for default in proof.defaults() {
                        self.expression(*default)?;
                    }
                    self.status = analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        [self.status, proof.record().status],
                    );
                    PendingOutcome::Normal
                } else {
                    let mut admitted = self.available_headers.iter().filter(|(proof, _)| {
                        proof.declaration() == site
                            && proof.caller() == self.request.owner
                            && proof.request().input == self.request.input
                            && proof.request().context == self.request.context
                    });
                    let (proof, row) = admitted
                        .next()
                        .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
                    if admitted.next().is_some() {
                        return Err(boundary(ObligationKind::AmbiguousBinding));
                    }
                    self.charge
                        .grow(size_of::<Id<super::source_call_records::SourceCallHeader>>() * 2)?;
                    self.headers.push(row.id());
                    self.status = analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        [self.status, proof.status()],
                    );
                    PendingOutcome::Normal
                }
            }
            S::StmtExpr if children.len() == 1 => {
                match self.expression(one(&children, F::Value)?)?.exception() {
                    Some((site, exception)) => PendingOutcome::Raise { site, exception },
                    None => PendingOutcome::Normal,
                }
            }
            S::StmtReturn => {
                let exception = if !children.is_empty() {
                    if children.len() != 1 {
                        return Err(boundary(ObligationKind::UnsupportedControlFlow));
                    }
                    self.expression(one(&children, F::Value)?)?.exception()
                } else {
                    None
                };
                match exception {
                    Some((site, exception)) => PendingOutcome::Raise { site, exception },
                    None => PendingOutcome::Return { site },
                }
            }
            S::StmtRaise if children.is_empty() => PendingOutcome::Raise {
                site,
                exception: self
                    .active
                    .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?,
            },
            S::StmtRaise => {
                if children
                    .iter()
                    .any(|row| !matches!(row.field, F::Exc | F::Cause))
                {
                    return Err(boundary(ObligationKind::UnsupportedControlFlow));
                }
                let expression = one(&children, F::Exc)?;
                let (kind, abrupt) = self.raise_expression(expression)?;
                if let Some(outcome) = abrupt {
                    outcome
                } else {
                    let mut cause_failure = None;
                    if children.iter().any(|row| row.field == F::Cause) {
                        let cause = one(&children, F::Cause)?;
                        if self
                            .data
                            .occurrences
                            .get(cause)
                            .is_some_and(|node| node.syntax_kind == S::ExprNoneLiteral)
                        {
                            let proof = self.expression(cause)?;
                            cause_failure = proof
                                .exception()
                                .map(|(site, exception)| PendingOutcome::Raise { site, exception });
                        } else {
                            let (_, abrupt) = self.raise_expression(cause)?;
                            cause_failure = abrupt.or_else(|| {
                                self.data
                                    .occurrences
                                    .get(cause)
                                    .filter(|node| {
                                        matches!(
                                            node.syntax_kind,
                                            S::ExprBooleanLiteral
                                                | S::ExprNumberLiteral
                                                | S::ExprStringLiteral
                                                | S::ExprBytesLiteral
                                                | S::ExprEllipsisLiteral
                                        )
                                    })
                                    .map(|_| PendingOutcome::Raise {
                                        site: cause,
                                        exception: ExactRuntimeException::TypeError,
                                    })
                            });
                        }
                    }
                    cause_failure.unwrap_or(PendingOutcome::Raise {
                        site,
                        exception: kind,
                    })
                }
            }
            S::StmtBreak if children.is_empty() => PendingOutcome::Break { site },
            S::StmtContinue if children.is_empty() => PendingOutcome::Continue { site },
            S::StmtIf => {
                let truth = self
                    .expression(one(&children, F::Test)?)?
                    .truth()
                    .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?;
                if truth {
                    self.suite(&children, F::Body, depth)?
                } else {
                    let mut outcome = PendingOutcome::Normal;
                    for clause in children.iter().filter(|row| row.field == F::Orelse) {
                        self.syntax.observe(clause.occurrence)?;
                        if self
                            .data
                            .occurrences
                            .get(clause.occurrence)
                            .is_none_or(|node| node.syntax_kind != S::ElifElseClause)
                        {
                            return Err(boundary(ObligationKind::UnsupportedControlFlow));
                        }
                        let clause_children = self.syntax.children(clause.occurrence)?;
                        let selected = if clause_children.iter().any(|row| row.field == F::Test) {
                            self.expression(one(&clause_children, F::Test)?)?
                                .truth()
                                .ok_or_else(|| boundary(ObligationKind::UnsupportedControlFlow))?
                        } else {
                            true
                        };
                        if selected {
                            outcome = self.suite(&clause_children, F::Body, depth + 1)?;
                            break;
                        }
                    }
                    outcome
                }
            }
            S::StmtTry => {
                if self.syntax.detail(site)?
                    != Some(syntax::SyntaxDetail::TryMode { is_star: false })
                    || children.iter().any(|row| {
                        !matches!(row.field, F::Body | F::Handler | F::Orelse | F::Finalbody)
                    })
                {
                    return Err(boundary(ObligationKind::UnsupportedControlFlow));
                }
                let mut pending = self.suite(&children, F::Body, depth)?;
                if let Some(exception) = pending.exception() {
                    let mut handlers = children
                        .iter()
                        .filter(|row| row.field == F::Handler)
                        .collect::<Vec<_>>();
                    self.charge
                        .grow(handlers.len() * size_of::<&SyntaxPlacement>() * 2)?;
                    handlers.sort_by_key(|row| row.ordinal);
                    for (ordinal, handler) in handlers.iter().enumerate() {
                        if handler.ordinal != ordinal as i64 {
                            return Err(boundary(ObligationKind::MissingEvidence));
                        }
                        self.syntax.observe(handler.occurrence)?;
                        if self
                            .data
                            .occurrences
                            .get(handler.occurrence)
                            .is_none_or(|node| node.syntax_kind != S::ExceptHandlerExceptHandler)
                        {
                            return Err(boundary(ObligationKind::MissingEvidence));
                        }
                        let Some(syntax::SyntaxDetail::HandlerName { name }) =
                            self.syntax.detail(handler.occurrence)?
                        else {
                            return Err(boundary(ObligationKind::MissingEvidence));
                        };
                        let handler_children = self.syntax.children(handler.occurrence)?;
                        let mut identifiers =
                            handler_children.iter().filter(|row| row.field == F::Child);
                        match (&name, identifiers.next()) {
                            (Some(name), Some(identifier))
                                if identifier.ordinal == 0
                                    && self
                                        .data
                                        .occurrences
                                        .get(identifier.occurrence)
                                        .is_some_and(|node| node.syntax_kind == S::Identifier) =>
                            {
                                if identifiers.next().is_some() {
                                    return Err(boundary(ObligationKind::UnsupportedControlFlow));
                                }
                                self.syntax.observe(identifier.occurrence)?;
                                self.syntax
                                    .tick(self.data.spellings.len())
                                    .map_err(boundary)?;
                                let mut spellings = self.data.spellings.iter().filter(|row| {
                                    row.occurrence == identifier.occurrence
                                        && self
                                            .data
                                            .qualifications
                                            .get(row.qualification)
                                            .is_some_and(|q| q.context == self.request.context)
                                });
                                let spelling = spellings
                                    .next()
                                    .ok_or_else(|| boundary(ObligationKind::MissingEvidence))?;
                                if spellings.next().is_some() || spelling.spelling != *name {
                                    return Err(boundary(ObligationKind::MissingEvidence));
                                }
                                self.syntax.support(
                                    spelling,
                                    spelling.qualification,
                                    identifier.occurrence,
                                )?;
                            }
                            (None, None) => {}
                            _ => return Err(boundary(ObligationKind::MissingEvidence)),
                        }
                        if handler_children
                            .iter()
                            .any(|row| !matches!(row.field, F::Test | F::Body | F::Child))
                        {
                            return Err(boundary(ObligationKind::UnsupportedControlFlow));
                        }
                        let matched = if handler_children.iter().any(|row| row.field == F::Test) {
                            let mut classes = Vec::new();
                            if let Some(outcome) = self.handler_classes(
                                one(&handler_children, F::Test)?,
                                depth + 1,
                                &mut classes,
                            )? {
                                pending = outcome;
                                break;
                            }
                            let mut matched = false;
                            // Validate every tuple member, including members after a positive match.
                            for class in classes {
                                matched |= self.syntax.class_matches(exception, class)?;
                            }
                            matched
                        } else {
                            true
                        };
                        if !matched {
                            continue;
                        }
                        let active = self.active;
                        self.active = Some(exception);
                        let result = self.suite(&handler_children, F::Body, depth + 1);
                        self.active = active;
                        pending = result?;
                        if let Some(name) = name {
                            self.handler_cleanup(handler.occurrence, &name)?;
                        }
                        break;
                    }
                } else if pending.is_normal() {
                    pending = self.suite(&children, F::Orelse, depth)?;
                }
                let active = self.active;
                if let Some(exception) = pending.exception() {
                    self.active = Some(exception);
                }
                let finalizer = self.suite(&children, F::Finalbody, depth);
                self.active = active;
                pending.after_finalizer(finalizer?)
            }
            _ => return Err(boundary(ObligationKind::UnsupportedControlFlow)),
        };
        if self.statements.len() == 64 {
            return Err(boundary(ObligationKind::SummaryProofLimit));
        }
        self.charge.grow(size_of::<Id<Occurrence>>() * 2)?;
        self.statements.push(site);
        Ok(outcome)
    }
}

/// Complete one entered statement using base evidence only. The caller must retain the actual
/// earlier evaluation tokens while this operation runs; persisted inputs are independently replayed.
pub fn complete(
    data: &EvaluationData,
    request: CompletionRequest,
    evaluations: &[&CheckedEvaluation],
    budget: &ResourceBudget,
) -> std::result::Result<std::result::Result<CheckedCompletion, ObligationKind>, ModelError> {
    complete_with_headers(data, request, evaluations, &[], budget)
}
pub(crate) fn complete_with_headers(
    data: &EvaluationData,
    request: CompletionRequest,
    evaluations: &[&CheckedEvaluation],
    headers: &[(
        &super::source_call::CheckedSourceBinding,
        &super::source_call_records::SourceCallHeader,
    )],
    budget: &ResourceBudget,
) -> std::result::Result<std::result::Result<CheckedCompletion, ObligationKind>, ModelError> {
    complete_with_definitions(data, request, evaluations, headers, &[], budget)
}
pub(crate) fn complete_with_definitions(
    data: &EvaluationData,
    request: CompletionRequest,
    evaluations: &[&CheckedEvaluation],
    headers: &[(
        &super::source_call::CheckedSourceBinding,
        &super::source_call_records::SourceCallHeader,
    )],
    definitions: &[super::definition::CheckedDefinition],
    budget: &ResourceBudget,
) -> std::result::Result<std::result::Result<CheckedCompletion, ObligationKind>, ModelError> {
    complete_with_contexts(
        data,
        request,
        evaluations,
        headers,
        definitions,
        &[],
        budget,
    )
}
pub(crate) fn complete_with_contexts(
    data: &EvaluationData,
    request: CompletionRequest,
    evaluations: &[&CheckedEvaluation],
    headers: &[(
        &super::source_call::CheckedSourceBinding,
        &super::source_call_records::SourceCallHeader,
    )],
    definitions: &[super::definition::CheckedDefinition],
    contexts: &[super::context_execution::CheckedContextExecution],
    budget: &ResourceBudget,
) -> std::result::Result<std::result::Result<CheckedCompletion, ObligationKind>, ModelError> {
    let prepared = PreparedExecution::new(data, request.input, request.context, budget)?;
    complete_prepared_with_contexts(
        &prepared,
        request,
        evaluations,
        headers,
        definitions,
        contexts,
    )
}
pub(crate) fn complete_prepared_with_contexts(
    prepared: &PreparedExecution<'_>,
    request: CompletionRequest,
    evaluations: &[&CheckedEvaluation],
    headers: &[(
        &super::source_call::CheckedSourceBinding,
        &super::source_call_records::SourceCallHeader,
    )],
    definitions: &[super::definition::CheckedDefinition],
    contexts: &[super::context_execution::CheckedContextExecution],
) -> std::result::Result<std::result::Result<CheckedCompletion, ObligationKind>, ModelError> {
    let data = prepared.data();
    let budget = prepared.budget();
    let expression_request = ExpressionRequest {
        input: request.input,
        context: request.context,
        owner: request.owner,
        expression: request.statement,
    };
    prepared.with_completion_syntax(expression_request, |syntax| {
        let mut kernel = Kernel {
            data,
            request,
            evaluations,
            available_headers: headers,
            headers: Vec::new(),
            available_definitions: definitions,
            definitions: Vec::new(),
            available_contexts: contexts,
            contexts: Vec::new(),
            syntax,
            statements: Vec::new(),
            expressions: Vec::new(),
            releases: Vec::new(),
            facts: Vec::new(),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            charge: charged::StateCharge::new(budget, "base-completion-results"),
            active: None,
        };
        let outcome = kernel.statement(request.statement, 0)?;
        let qualification = kernel
            .syntax
            .qualification(request.statement)
            .map_err(boundary)?;
        let status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [kernel.syntax.status(), kernel.status],
        );
        let (native, native_charge) = kernel.syntax.take_admission();
        Ok(CheckedCompletion {
            request,
            outcome,
            native,
            statements: kernel.statements,
            expressions: kernel.expressions,
            releases: kernel.releases,
            facts: kernel.facts,
            headers: kernel.headers,
            definitions: kernel.definitions,
            contexts: kernel.contexts,
            qualification,
            status,
            _charge: native_charge,
            _results_charge: kernel.charge,
        })
    })
}

#[cfg(test)]
mod prepared_completion_tests {
    use super::*;

    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }

    #[test]
    fn prepared_completion_preserves_frame_and_refusal_authority() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = EvaluationData::new(&budget);
        let request = CompletionRequest {
            input: id(1),
            context: id(2),
            owner: id(3),
            statement: id(4),
        };
        data.owners
            .insert(normalized::entities::OccurrenceOwnership {
                occurrence: request.statement,
                owner: id(5),
                entity: request.owner,
            })
            .unwrap();
        let loaded = budget.reserved();
        let prepared =
            PreparedExecution::new(&data, request.input, request.context, &budget).unwrap();
        let retained = budget.reserved();
        assert!(retained > loaded);
        for _ in 0..3 {
            let one_off = complete(&data, request, &[], &budget)
                .unwrap()
                .err()
                .unwrap();
            let reused = complete_prepared_with_contexts(&prepared, request, &[], &[], &[], &[])
                .unwrap()
                .err()
                .unwrap();
            assert_eq!(one_off, ObligationKind::MissingEvidence);
            assert_eq!(reused, one_off);
            assert_eq!(budget.reserved(), retained);
        }
        let outside_owner = CompletionRequest {
            owner: id(6),
            ..request
        };
        assert_eq!(
            complete_prepared_with_contexts(&prepared, outside_owner, &[], &[], &[], &[])
                .unwrap()
                .err(),
            complete(&data, outside_owner, &[], &budget).unwrap().err(),
        );
        for foreign in [
            CompletionRequest {
                input: id(7),
                ..request
            },
            CompletionRequest {
                context: id(8),
                ..request
            },
        ] {
            assert_eq!(
                complete_prepared_with_contexts(&prepared, foreign, &[], &[], &[], &[])
                    .unwrap()
                    .err(),
                Some(ObligationKind::IncompatibleContexts),
            );
        }
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), loaded);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
