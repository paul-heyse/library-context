//! Necessary portable execution frame, source and formal/actual correspondence.
//! These predicates do not evaluate expressions or execute statements.
use super::{
    body_records::{BodyMember, BodyReleaseInput, BodySource, SourceBodyCompletion},
    completion_records::{
        CompletionMember, CompletionOutcome, CompletionSource, EnteredStatement,
        StatementCompletion,
    },
    records::{
        EvaluationMember, EvaluationOperand, EvaluationSource, ExpressionEvaluation, ordered_digest,
    },
    source_call_records::*,
};
use crate::domain::{
    analysis,
    assertion::*,
    attribution::*,
    calls::*,
    normalized::{Rows, bindings::*, callables::*, entities::*, events::*},
    resources::ResourceBudget,
    source::*,
    *,
};
#[derive(Clone)]
pub struct ExecutionScope {
    pub root: ValidationInput,
    pub memberships: Vec<(ValidationInput, &'static str, ValidationInput)>,
    pub joins: Vec<ExecutionMembership>,
}
#[derive(Clone)]
pub struct ExecutionMembership {
    pub source: ValidationInput,
    pub source_key: &'static str,
    pub member: ValidationInput,
    pub member_key: &'static str,
    pub context: Option<ExecutionMembershipContext>,
}
/// A grammar member's qualification context must match the explicitly named owner route.
#[derive(Clone, Copy)]
pub enum ExecutionMembershipContext {
    EnrichedInvocation,
    EnrichedContextItem,
}
#[derive(Clone, Copy)]
enum Kind {
    Expression,
    Statement,
    Body,
    Header,
    Release,
    Invocation,
    EnrichedStatement,
    EnrichedBody,
    Fresh,
    Modeled,
    Definition,
    ContextBinding,
    Context,
    Capture,
    EnrichedFrames,
}
mod enriched;
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Expression => "execution_expression_fidelity",
            Self::Statement => "execution_statement_fidelity",
            Self::Body => "execution_body_fidelity",
            Self::Header => "execution_header_fidelity",
            Self::Release => "execution_release_fidelity",
            Self::Invocation => "execution_invocation_fidelity",
            kind => enriched::name(kind),
        }
    }
    fn root(self) -> ValidationInput {
        match self {
            Self::Expression => ValidationInput::of::<ExpressionEvaluation>(&["id"]),
            Self::Statement => ValidationInput::of::<StatementCompletion>(&["id"]),
            Self::Body => ValidationInput::of::<SourceBodyCompletion>(&["id"]),
            Self::Header => ValidationInput::of::<SourceCallHeader>(&["id"]),
            Self::Release => ValidationInput::of::<SourceFrameRelease>(&["id"]),
            Self::Invocation => ValidationInput::of::<SourceInvocation>(&["id"]),
            kind => enriched::root(kind),
        }
    }
    fn scope(self) -> ExecutionScope {
        if matches!(
            self,
            Self::EnrichedStatement
                | Self::EnrichedBody
                | Self::Fresh
                | Self::Modeled
                | Self::Definition
                | Self::ContextBinding
                | Self::Context
                | Self::Capture
        ) {
            return enriched::scope(self);
        }
        let mut memberships = if matches!(self, Self::Release) {
            vec![
                (
                    ValidationInput::of::<SourceFrameArgument>(&["id"]),
                    "release",
                    ValidationInput::of::<SourceFrameRelease>(&["id"]),
                ),
                (
                    ValidationInput::of::<CallBinding>(&["id"]),
                    "attempt",
                    ValidationInput::of::<CallBindingAttempt>(&["id"]),
                ),
            ]
        } else {
            Vec::new()
        };
        if matches!(self, Self::Header | Self::Release | Self::Invocation) {
            memberships.push((
                ValidationInput::of::<syntax::DeclarationObservation>(&["id"]),
                "declaration",
                ValidationInput::of::<Occurrence>(&["id"]),
            ));
        }
        if matches!(self, Self::Expression | Self::Statement) {
            memberships.push((
                ValidationInput::of::<OccurrenceOwnership>(&["id"]),
                "occurrence",
                ValidationInput::of::<Occurrence>(&["id"]),
            ));
        }
        memberships.push((
            ValidationInput::of::<input::ArtifactUse>(&["id"]),
            "artifact",
            ValidationInput::of::<SourceArtifact>(&["id"]),
        ));
        macro_rules! owned {
            ($member:ty, $field:literal, $owner:ty) => {{
                memberships.push((
                    ValidationInput::of::<$member>(&["id"]),
                    $field,
                    ValidationInput::of::<$owner>(&["id"]),
                ));
            }};
        }
        match self {
            Self::Expression => {
                owned!(EvaluationMember, "evaluation", ExpressionEvaluation);
                owned!(EvaluationOperand, "evaluation", ExpressionEvaluation);
            }
            Self::Statement => {
                owned!(CompletionMember, "completion", StatementCompletion);
                owned!(EnteredStatement, "completion", StatementCompletion);
            }
            Self::Body => {
                owned!(BodyMember, "body", SourceBodyCompletion);
                owned!(BodyReleaseInput, "body", SourceBodyCompletion);
            }
            Self::Header => owned!(HeaderMember, "header", SourceCallHeader),
            _ => {}
        }
        ExecutionScope {
            root: self.root(),
            memberships,
            joins: Vec::new(),
        }
    }
    fn inputs(self) -> Vec<ValidationInput> {
        if matches!(self, Self::EnrichedFrames) {
            return enriched::frame_inputs();
        }
        let mut inputs = vec![
            self.root(),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<CoverageScope>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<input::ArtifactUse>(&["id"]),
            ValidationInput::of::<Module>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ];
        macro_rules! add {($($ty:ty),* $(,)?) => {{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*}};}
        match self {
            Self::Expression => add!(
                analysis::base_evaluation::AnalysisInvocation,
                OccurrenceOwnership,
                EvaluationSource,
                EvaluationMember,
                EvaluationOperand,
                conditions::entry::EntryValueWitness
            ),
            Self::Statement => add!(
                analysis::base_completion::AnalysisInvocation,
                OccurrenceOwnership,
                CompletionSource,
                CompletionMember,
                EnteredStatement,
                ExpressionEvaluation,
                analysis::base_evaluation::AnalysisInvocation
            ),
            Self::Body => add!(
                analysis::base_completion::AnalysisInvocation,
                EntityRef,
                CallableEntity,
                BodySource,
                BodyMember,
                BodyReleaseInput,
                StatementCompletion
            ),
            Self::Header | Self::Release | Self::Invocation => {
                add!(
                    analysis::source_call::AnalysisInvocation,
                    SourceCallHeader,
                    NormalizedCallEvent,
                    OccurrenceOwnership,
                    CallBindingAttempt,
                    NormalizedCallAlternative,
                    CallSyntax,
                    EntityRef,
                    CallableEntity,
                    syntax::DeclarationObservation
                );
                if matches!(self, Self::Header) {
                    add!(HeaderMember);
                }
                if matches!(self, Self::Invocation) {
                    add!(SourceCallOutcome, CompletionOutcome);
                }
                if matches!(self, Self::Release | Self::Invocation) {
                    add!(
                        SourceFrameRelease,
                        SourceBodyCompletion,
                        analysis::base_completion::AnalysisInvocation
                    );
                }
                if matches!(self, Self::Release) {
                    add!(
                        SourceFrameArgument,
                        ExpressionEvaluation,
                        analysis::base_evaluation::AnalysisInvocation,
                        CallBinding,
                        SignatureSlot,
                        SignatureParameter,
                        BindingSource,
                        BindingProjection
                    );
                }
            }
            kind => inputs.extend(enriched::inputs(kind)),
        }
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        inputs
    }
}
macro_rules! fields {($apply:ident) => {$apply! {
    qualifications:AssertionQualification, scopes:CoverageScope, occurrences:Occurrence,
    artifacts:SourceArtifact, uses:input::ArtifactUse, modules:Module, definitions:analysis::AnalysisDefinition, owners:OccurrenceOwnership,
    refs:EntityRef, callables:CallableEntity, events:NormalizedCallEvent, attempts:CallBindingAttempt,
    alternatives:NormalizedCallAlternative, syntax:CallSyntax, declarations:syntax::DeclarationObservation, bindings:CallBinding,
    slots:SignatureSlot, parameters:SignatureParameter, binding_sources:BindingSource, projections:BindingProjection,
    base:analysis::base_evaluation::AnalysisInvocation, completion:analysis::base_completion::AnalysisInvocation,
    source:analysis::source_call::AnalysisInvocation, evaluations:ExpressionEvaluation,
    statements:StatementCompletion, bodies:SourceBodyCompletion, headers:SourceCallHeader,
    releases:SourceFrameRelease, arguments:SourceFrameArgument, invocations:SourceInvocation,
    evaluation_sources:EvaluationSource, evaluation_members:EvaluationMember, operands:EvaluationOperand, entries:conditions::entry::EntryValueWitness,
    completion_sources:CompletionSource, completion_members:CompletionMember, entered:EnteredStatement,
    body_sources:BodySource, body_members:BodyMember, body_releases:BodyReleaseInput, header_members:HeaderMember,
    source_outcomes:SourceCallOutcome, completion_outcomes:CompletionOutcome,
    enriched_frames:analysis::enriched_execution::AnalysisInvocation, method_parameters:analysis::MethodParameters,
    enriched_statements:super::enriched_records::StatementExecution, enriched_sources:super::enriched_records::ExecutionSource,
    enriched_members:super::enriched_records::ExecutionMember, enriched_entered:super::enriched_records::EnteredStatement,
    enriched_bodies:super::enriched_records::BodyExecution, enriched_body_sources:super::enriched_records::BodySource,
    enriched_body_members:super::enriched_records::BodyMember, enriched_releases:super::enriched_records::BodyReleaseInput,
    fresh:super::enriched_records::SourceExecutionInvocation, fresh_arguments:super::enriched_records::SourceExecutionArgument,
    modeled:super::modeled_call::ModeledCallEvaluation, modeled_arguments:super::modeled_call::ModeledCallArgument,
    modeled_native:super::modeled_call::ModeledCallNative, authored_models:models::AuthoredModel,
    definition_values:super::definition::DefinitionEvaluation, definition_sources:super::definition::DefinitionSource,
    definition_members:super::definition::DefinitionMember, parameter_syntax:syntax::ParameterSyntaxObservation,
    context_bindings:super::context_binding::ContextEntryBinding, context_binding_sources:super::context_binding::BindingSource,
    context_binding_members:super::context_binding::BindingMember, protocols:models::AuthoredContextProtocol,
    contexts:super::context_execution::ContextExecution, context_sources:super::context_execution::ContextSource,
    context_members:super::context_execution::ContextMember, context_items:super::context_execution::ContextItem,
    captured:super::capture_bridge::CapturedEntryBinding, captured_values:super::capture_bridge::CapturedValueSource,
    placements:syntax::SyntaxPlacement, call_arguments:CallArgument, provider_symbols:ProviderSymbol, parameter_entities:ParameterEntity, call_targets:CallTarget, call_destinations:CallDestination, signature_enumerations:SignatureEnumerationObservation, enriched_inputs:analysis::enriched_execution::AnalysisInput, enriched_parents:analysis::enriched_execution::InvocationSource,
}};}
macro_rules! data {($($field:ident:$ty:ty,)*) => {
    struct Check {kind:Kind, budget:ResourceBudget, $( $field:Rows<$ty>, )*}
    impl Check {
        fn new(kind:Kind, budget:&ResourceBudget) -> Self {Self {kind, budget:budget.clone(), $($field:Rows::new(budget),)*}}
        fn visit(&mut self, name:&str, batch:&arrow_array::RecordBatch) -> Result<(), ModelError> {
            $(if name == <$ty>::NAME {self.$field.decode(batch)?;})* Ok(())
        }
    }
};}
fields!(data);
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(format!("execution fidelity: {message}"))
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id).ok_or_else(|| invalid(R::NAME))
}
impl Check {
    fn frame(
        &self,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
        subject: Option<Id<EntityRef>>,
        qualification: Id<AssertionQualification>,
        occurrence: Id<Occurrence>,
    ) -> Result<(), ModelError> {
        if subject.is_some() {
            return Err(invalid("whole execution frame has a subject"));
        }
        let q = need(&self.qualifications, qualification)?;
        let occurrence = need(&self.occurrences, occurrence)?;
        let artifact = need(&self.artifacts, occurrence.source)?;
        if artifact.input != input
            || admission::ArtifactClass::of(&artifact.path)
                != Some(admission::ArtifactClass::PythonSource)
            || q.context != context
            || q.modality != Modality::Definite
            || q.approximation != Approximation::Exact
            || q.condition != conditions::Diagram::always().id()
        {
            return Err(invalid("frame/qualification/source correspondence"));
        }
        let _roots = self.budget.reserve(
            "execution-fidelity-source-selection",
            self.uses
                .len()
                .checked_mul(size_of::<input::ArtifactUse>() * 2)
                .and_then(|bytes| bytes.checked_add(artifact.heap_bytes() + 256))
                .ok_or_else(|| invalid("source selection allowance"))?,
        )?;
        let uses = self
            .uses
            .iter()
            .filter(|usage| usage.artifact == artifact.id())
            .cloned()
            .collect::<Vec<_>>();
        if !admission::analysis_roots(std::slice::from_ref(artifact), &uses)?
            .contains(&artifact.id())
        {
            return Err(invalid("source is not an analysis root"));
        }
        let included = match need(&self.scopes, q.scope)? {
            CoverageScope::Input { input: owner } => *owner == input,
            CoverageScope::Artifact { artifact } => *artifact == occurrence.source,
            CoverageScope::Module { module } => {
                need(&self.modules, *module)?.source == occurrence.source
            }
            _ => false,
        };
        if !included {
            return Err(invalid("qualification excludes source"));
        }
        Ok(())
    }
    fn definition(
        &self,
        actual: Id<analysis::AnalysisDefinition>,
        expected: analysis::AnalysisDefinition,
    ) -> Result<(), ModelError> {
        if need(&self.definitions, actual)? != &expected {
            return Err(invalid("selected owner definition"));
        }
        Ok(())
    }
    fn owner(&self, occurrence: Id<Occurrence>, owner: Id<EntityRef>) -> Result<(), ModelError> {
        let mut candidates = self
            .owners
            .iter()
            .filter(|candidate| candidate.occurrence == occurrence);
        let candidate = candidates
            .next()
            .ok_or_else(|| invalid("occurrence ownership absent"))?;
        if candidates.next().is_some() || candidate.entity != owner {
            return Err(invalid("occurrence owner differs"));
        }
        Ok(())
    }
    fn declaration(
        &self,
        owner: Id<EntityRef>,
        declaration: Id<Occurrence>,
    ) -> Result<(), ModelError> {
        let EntityRef::Callable { callable } = need(&self.refs, owner)? else {
            return Err(invalid("body owner is not callable"));
        };
        if need(&self.callables, *callable)?
            != &(CallableEntity::Source {
                declaration,
                kind: CallableKind::Function,
            })
            || need(&self.occurrences, declaration)?.syntax_kind != SyntaxKind::StmtFunctionDef
        {
            return Err(invalid("source callable declaration differs"));
        }
        Ok(())
    }
    fn header(&self, row: &SourceCallHeader) -> Result<(), ModelError> {
        let frame = need(&self.source, row.invocation)?;
        self.definition(frame.definition, super::configuration::source_calls().1)?;
        let event = need(&self.events, row.event)?;
        self.frame(
            frame.input,
            frame.context,
            frame.subject,
            row.qualification,
            event.site,
        )?;
        let owner = need(&self.owners, event.owner)?;
        let attempt = need(&self.attempts, row.attempt)?;
        if attempt.outcome != BindingOutcome::Bound {
            return Err(invalid("source header binding is not Bound"));
        }
        let alternative = need(&self.alternatives, attempt.alternative)?;
        let syntax = need(
            &self.syntax,
            attempt
                .syntax
                .ok_or_else(|| invalid("header syntax absent"))?,
        )?;
        if event.context != frame.context
            || owner.occurrence != event.site
            || owner.entity != row.owner
            || attempt.event != event.id()
            || alternative.event != event.id()
            || alternative.entity != Some(row.callee)
            || syntax.site != event.site
            || syntax.qualification != row.qualification
            || syntax.in_annotation
        {
            return Err(invalid("header event/binding/caller/callee correspondence"));
        }
        self.declaration(row.callee, row.declaration)?;
        let EntityRef::Callable { callable } = need(&self.refs, row.owner)? else {
            return Err(invalid("caller is not source callable"));
        };
        let CallableEntity::Source {
            declaration: caller,
            kind: CallableKind::Function,
        } = need(&self.callables, *callable)?
        else {
            return Err(invalid("caller declaration absent"));
        };
        if *caller == row.declaration
            || need(&self.occurrences, *caller)?.source
                != need(&self.occurrences, row.declaration)?.source
            || need(&self.occurrences, event.site)?.source
                != need(&self.occurrences, row.declaration)?.source
        {
            return Err(invalid(
                "fresh source declarations leave their source domain",
            ));
        }
        let mut declarations = self.declarations.iter().filter(|decl| {
            decl.declaration == row.declaration
                && self
                    .qualifications
                    .get(decl.qualification)
                    .is_some_and(|q| q.context == frame.context)
        });
        let declaration = declarations
            .next()
            .ok_or_else(|| invalid("native source declaration absent"))?;
        if declarations.next().is_some()
            || declaration.kind != syntax::DeclarationKind::Function
            || declaration.parent != Some(*caller)
        {
            return Err(invalid("native callee/caller declaration correspondence"));
        }
        Ok(())
    }
    fn body(&self, row: &SourceBodyCompletion) -> Result<(), ModelError> {
        let frame = need(&self.completion, row.invocation)?;
        self.definition(frame.definition, super::configuration::base_completion().1)?;
        self.frame(
            frame.input,
            frame.context,
            frame.subject,
            row.qualification,
            row.declaration,
        )?;
        self.declaration(row.owner, row.declaration)
    }
    fn release(&self, row: &SourceFrameRelease, check_arguments: bool) -> Result<(), ModelError> {
        let header = need(&self.headers, row.header)?;
        let body = need(&self.bodies, row.body)?;
        self.header(header)?;
        self.body(body)?;
        let caller = need(&self.source, header.invocation)?;
        let callee = need(&self.completion, body.invocation)?;
        if (
            caller.input,
            caller.context,
            header.callee,
            header.declaration,
            header.qualification,
        ) != (
            callee.input,
            callee.context,
            body.owner,
            body.declaration,
            body.qualification,
        ) {
            return Err(invalid("release header/body frame correspondence"));
        }
        if !check_arguments {
            return Ok(());
        }
        let attempt = need(&self.attempts, header.attempt)?;
        let _scratch = self.budget.reserve(
            "execution-fidelity-argument-domain",
            self.bindings
                .len()
                .checked_add(self.arguments.len())
                .and_then(|count| {
                    count.checked_mul(
                        4 * size_of::<(Id<SignatureParameter>, Id<Occurrence>)>() + 128,
                    )
                })
                .ok_or_else(|| invalid("argument domain allowance overflow"))?,
        )?;
        let mut expected = std::collections::BTreeSet::new();
        let mut expected_order = Vec::new();
        let mut bindings = self
            .bindings
            .iter()
            .filter(|binding| binding.attempt == attempt.id())
            .collect::<Vec<_>>();
        bindings.sort_by_key(|binding| binding.ordinal);
        for binding in bindings {
            let slot = need(&self.slots, binding.slot)?;
            let parameter = need(&self.parameters, slot.parameter)?;
            if Some(parameter.signature) != attempt.signature {
                return Err(invalid("formal belongs to another signature"));
            }
            match need(&self.binding_sources, binding.source)? {
                BindingSource::Actual { occurrence } => {
                    if need(&self.projections, binding.projection)? != &BindingProjection::Whole {
                        return Err(invalid("projected actual in fresh frame"));
                    }
                    if !expected.insert((parameter.id(), *occurrence)) {
                        return Err(invalid("duplicate formal/actual binding"));
                    }
                    expected_order.push((parameter.id(), *occurrence));
                }
                BindingSource::EmptyVarargs | BindingSource::EmptyKwargs => {}
                _ => return Err(invalid("fresh argument source is not actual")),
            }
        }
        let mut actual = std::collections::BTreeSet::new();
        let mut arguments = self
            .arguments
            .iter()
            .filter(|argument| argument.release == row.id())
            .collect::<Vec<_>>();
        arguments.sort_by_key(|argument| argument.ordinal);
        let mut digest = KeySink::new("source-frame-arguments");
        for (ordinal, argument) in arguments.iter().enumerate() {
            if argument.ordinal != ordinal as i64
                || !actual.insert((argument.formal, argument.actual))
            {
                return Err(invalid("argument ordinal/formal membership"));
            }
            let evaluation = need(&self.evaluations, argument.evaluation)?;
            let frame = need(&self.base, evaluation.invocation)?;
            if evaluation.expression != argument.actual
                || evaluation.owner != header.owner
                || (frame.input, frame.context) != (caller.input, caller.context)
            {
                return Err(invalid("actual evaluation frame/owner correspondence"));
            }
            argument.formal.encode(&mut digest);
            argument.actual.encode(&mut digest);
            argument.evaluation.encode(&mut digest);
        }
        if actual != expected
            || arguments
                .iter()
                .map(|argument| (argument.formal, argument.actual))
                .ne(expected_order)
            || digest.finish() != row.arguments
        {
            return Err(invalid("complete formal/actual argument domain"));
        }
        Ok(())
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Check::visit(self, name, batch)
    }
    fn execution_scope(&self) -> Option<ExecutionScope> {
        if matches!(self.kind, Kind::EnrichedFrames) {
            None
        } else {
            Some(self.kind.scope())
        }
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        // The workspace enforces the selected immutable epoch. Admission reads the
        // advertised final vocabulary, including qualifications authored after Facts.
        self.visit(input.name(), batch)
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        match self.kind {
            Kind::Expression => {
                for row in self.evaluations.iter() {
                    let frame = need(&self.base, row.invocation)?;
                    self.definition(frame.definition, super::configuration::base_evaluation().1)?;
                    self.frame(
                        frame.input,
                        frame.context,
                        frame.subject,
                        row.qualification,
                        row.expression,
                    )?;
                    self.owner(row.expression, row.owner)?;
                    if !super::production::is_expression(
                        need(&self.occurrences, row.expression)?.syntax_kind,
                    ) {
                        return Err(invalid("expression source kind"));
                    }
                    let _scratch = self.budget.reserve(
                        "evaluation-fidelity-members",
                        self.evaluation_members
                            .len()
                            .checked_add(self.operands.len())
                            .and_then(|n| n.checked_mul(64))
                            .ok_or_else(|| invalid("evaluation member allowance"))?,
                    )?;
                    let members = ordered(
                        self.evaluation_members
                            .iter()
                            .filter(|member| member.evaluation == row.id()),
                        |member| member.ordinal,
                    )?;
                    let mut sources = Vec::new();
                    for member in members {
                        let source = need(&self.evaluation_sources, member.source)?;
                        if let EvaluationSource::Entry { witness } = source {
                            let entry = need(&self.entries, *witness)?;
                            if entry.owner != row.owner
                                || entry.context != frame.context
                                || need(&self.occurrences, entry.access)?.source
                                    != need(&self.occurrences, row.expression)?.source
                            {
                                return Err(invalid("evaluation entry owner/context/source"));
                            }
                        }
                        sources.push(source.id());
                    }
                    let operands = ordered(
                        self.operands
                            .iter()
                            .filter(|operand| operand.evaluation == row.id()),
                        |operand| operand.ordinal,
                    )?;
                    for operand in &operands {
                        let occurrence = need(&self.occurrences, operand.expression)?;
                        if occurrence.source != need(&self.occurrences, row.expression)?.source {
                            return Err(invalid("evaluation operand source"));
                        }
                    }
                    if ordered_digest("base-evaluation-sources", sources) != row.sources
                        || ordered_digest(
                            "base-evaluation-operands",
                            operands.iter().map(|operand| operand.expression),
                        ) != row.operands
                    {
                        return Err(invalid("evaluation source/operand membership digest"));
                    }
                }
            }
            Kind::Statement => {
                for row in self.statements.iter() {
                    let frame = need(&self.completion, row.invocation)?;
                    self.definition(frame.definition, super::configuration::base_completion().1)?;
                    self.frame(
                        frame.input,
                        frame.context,
                        frame.subject,
                        row.qualification,
                        row.statement,
                    )?;
                    self.owner(row.statement, row.owner)?;
                    if !super::completion_production::is_statement(
                        need(&self.occurrences, row.statement)?.syntax_kind,
                    ) {
                        return Err(invalid("statement source kind"));
                    }
                    let _scratch = self.budget.reserve(
                        "statement-fidelity-members",
                        self.completion_members
                            .len()
                            .checked_add(self.entered.len())
                            .and_then(|n| n.checked_mul(64))
                            .ok_or_else(|| invalid("statement member allowance"))?,
                    )?;
                    let members = ordered(
                        self.completion_members
                            .iter()
                            .filter(|member| member.completion == row.id()),
                        |member| member.ordinal,
                    )?;
                    let mut sources = Vec::new();
                    for member in members {
                        let source = need(&self.completion_sources, member.source)?;
                        if let CompletionSource::Evaluation { evaluation } = source {
                            let evaluation = need(&self.evaluations, *evaluation)?;
                            let parent = need(&self.base, evaluation.invocation)?;
                            if evaluation.owner != row.owner
                                || (parent.input, parent.context) != (frame.input, frame.context)
                            {
                                return Err(invalid("statement evaluation frame/owner"));
                            }
                        }
                        sources.push(source.id());
                    }
                    let entered = ordered(
                        self.entered
                            .iter()
                            .filter(|entered| entered.completion == row.id()),
                        |entered| entered.ordinal,
                    )?;
                    for entered in &entered {
                        if need(&self.occurrences, entered.statement)?.source
                            != need(&self.occurrences, row.statement)?.source
                        {
                            return Err(invalid("entered statement source"));
                        }
                    }
                    if ordered_digest("base-completion-sources", sources) != row.sources
                        || ordered_digest(
                            "base-completion-entered",
                            entered.iter().map(|entered| entered.statement),
                        ) != row.entered
                    {
                        return Err(invalid("statement source/entered membership digest"));
                    }
                }
            }
            Kind::Body => {
                for row in self.bodies.iter() {
                    self.body(row)?;
                    let _scratch = self.budget.reserve(
                        "body-fidelity-members",
                        self.body_members
                            .len()
                            .checked_add(self.body_releases.len())
                            .and_then(|n| n.checked_mul(64))
                            .ok_or_else(|| invalid("body member allowance"))?,
                    )?;
                    let members = ordered(
                        self.body_members
                            .iter()
                            .filter(|member| member.body == row.id()),
                        |member| member.ordinal,
                    )?;
                    let mut sources = Vec::new();
                    for member in members {
                        let source = need(&self.body_sources, member.source)?;
                        if let BodySource::Statement { completion } = source {
                            let statement = need(&self.statements, *completion)?;
                            if statement.invocation != row.invocation
                                || statement.owner != row.owner
                            {
                                return Err(invalid("body statement frame/owner"));
                            }
                        }
                        sources.push(source.id());
                    }
                    let releases = ordered(
                        self.body_releases
                            .iter()
                            .filter(|release| release.body == row.id()),
                        |release| release.ordinal,
                    )?;
                    let mut digest = KeySink::new("base-source-body-releases");
                    for release in releases {
                        if need(&self.occurrences, release.expression)?.source
                            != need(&self.occurrences, row.declaration)?.source
                        {
                            return Err(invalid("body release source"));
                        }
                        release.expression.encode(&mut digest);
                        release.safety.encode(&mut digest);
                    }
                    if ordered_digest("base-source-body-sources", sources) != row.sources
                        || digest.finish() != row.releases
                    {
                        return Err(invalid("body source/release membership digest"));
                    }
                }
            }
            Kind::Header => {
                for row in self.headers.iter() {
                    self.header(row)?;
                    let _scratch = self.budget.reserve(
                        "header-fidelity-members",
                        self.header_members
                            .len()
                            .checked_mul(32)
                            .ok_or_else(|| invalid("header member allowance"))?,
                    )?;
                    let members = ordered(
                        self.header_members
                            .iter()
                            .filter(|member| member.header == row.id()),
                        |member| member.ordinal,
                    )?;
                    if ordered_digest(
                        "source-call-header-premises",
                        members.iter().map(|member| member.premise),
                    ) != row.premises
                    {
                        return Err(invalid("header premise membership digest"));
                    }
                }
            }
            Kind::Release => {
                for row in self.releases.iter() {
                    self.release(row, true)?;
                }
            }
            Kind::Invocation => {
                for row in self.invocations.iter() {
                    let release = need(&self.releases, row.release)?;
                    self.release(release, false)?;
                    let header = need(&self.headers, release.header)?;
                    if row.invocation != header.invocation
                        || row.qualification != header.qualification
                    {
                        return Err(invalid("invocation header/frame correspondence"));
                    }
                    let body = need(&self.bodies, release.body)?;
                    let agrees = match (
                        need(&self.source_outcomes, row.outcome)?,
                        need(&self.completion_outcomes, body.outcome)?,
                    ) {
                        (
                            SourceCallOutcome::Normal,
                            CompletionOutcome::Normal | CompletionOutcome::Return { .. },
                        ) => true,
                        (
                            SourceCallOutcome::Raised { site, exception },
                            CompletionOutcome::Raise {
                                site: expected,
                                exception: expected_exception,
                            },
                        ) => site == expected && exception == expected_exception,
                        _ => false,
                    };
                    if !agrees {
                        return Err(invalid("invocation/body nominal outcome correspondence"));
                    }
                }
            }
            _ => self.finish_enriched()?,
        }
        Ok(())
    }
}
pub fn invariants() -> Vec<Invariant> {
    [
        Kind::Expression,
        Kind::Statement,
        Kind::Body,
        Kind::Header,
        Kind::Release,
        Kind::Invocation,
        Kind::EnrichedStatement,
        Kind::EnrichedBody,
        Kind::Fresh,
        Kind::Modeled,
        Kind::Definition,
        Kind::ContextBinding,
        Kind::Context,
        Kind::Capture,
        Kind::EnrichedFrames,
    ]
    .into_iter()
    .map(|kind| Invariant {
        purpose: InvariantPurpose::Admission,
        name: kind.name(),
        revision: 1,
        inputs: kind.inputs(),
        create: std::sync::Arc::new(move |budget| Box::new(Check::new(kind, budget))),
    })
    .collect()
}

fn ordered<'a, R>(
    rows: impl Iterator<Item = &'a R>,
    ordinal: impl Fn(&R) -> i64,
) -> Result<Vec<&'a R>, ModelError> {
    let mut rows = rows.collect::<Vec<_>>();
    rows.sort_by_key(|row| ordinal(row));
    if rows
        .iter()
        .enumerate()
        .any(|(index, row)| ordinal(row) != index as i64)
    {
        return Err(invalid("owner member ordinal domain"));
    }
    Ok(rows)
}

#[cfg(test)]
mod fidelity_controls {
    use super::*;
    fn id<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![byte; 16])).unwrap()
    }
    fn expression() -> Check {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut check = Check::new(Kind::Expression, &budget);
        let artifact = SourceArtifact::from_bytes(id(1), "source.py".into(), b"None").unwrap();
        let occurrence = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 4,
            syntax_kind: SyntaxKind::ExprNoneLiteral,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let qualification = AssertionQualification {
            context: id(2),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let definition = super::super::configuration::base_evaluation().1;
        let frame = analysis::base_evaluation::AnalysisInvocation::new(
            id(1),
            id(2),
            definition.id(),
            None,
            [],
        )
        .0;
        let row = ExpressionEvaluation {
            invocation: frame.id(),
            expression: occurrence.id(),
            owner: id(3),
            qualification: qualification.id(),
            boolean_value: Some(false),
            release: super::super::evaluation::ReleaseSafety::Closed,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            sources: ordered_digest(
                "base-evaluation-sources",
                std::iter::empty::<Id<EvaluationSource>>(),
            ),
            operands: ordered_digest(
                "base-evaluation-operands",
                std::iter::empty::<Id<Occurrence>>(),
            ),
        };
        check
            .uses
            .insert(input::ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: input::SourceRole::Release,
            })
            .unwrap();
        check.artifacts.insert(artifact).unwrap();
        check.scopes.insert(scope).unwrap();
        check.qualifications.insert(qualification).unwrap();
        check
            .owners
            .insert(OccurrenceOwnership {
                occurrence: occurrence.id(),
                owner: occurrence.id(),
                entity: row.owner,
            })
            .unwrap();
        check.occurrences.insert(occurrence).unwrap();
        check.definitions.insert(definition).unwrap();
        check.base.insert(frame).unwrap();
        check.evaluations.insert(row).unwrap();
        check
    }
    #[test]
    fn expression_fidelity_checks_frame_source_and_recorded_member_domain() {
        assert!(Box::new(expression()).finish().is_ok());
        let mut wrong = expression();
        let mut frame = wrong.base.iter().next().unwrap().clone();
        frame.input = id(9);
        let mut row = wrong.evaluations.iter().next().unwrap().clone();
        row.invocation = frame.id();
        wrong.base = Rows::new(&wrong.budget);
        wrong.base.insert(frame).unwrap();
        wrong.evaluations = Rows::new(&wrong.budget);
        wrong.evaluations.insert(row).unwrap();
        assert!(Box::new(wrong).finish().is_err());
        let mut wrong = expression();
        let mut row = wrong.evaluations.iter().next().unwrap().clone();
        row.owner = id(10);
        wrong.evaluations = Rows::new(&wrong.budget);
        wrong.evaluations.insert(row).unwrap();
        assert!(Box::new(wrong).finish().is_err());
        let mut wrong = expression();
        let mut row = wrong.evaluations.iter().next().unwrap().clone();
        row.operands = ContentHash::of(b"omitted operand");
        wrong.evaluations = Rows::new(&wrong.budget);
        wrong.evaluations.insert(row).unwrap();
        assert!(Box::new(wrong).finish().is_err());
    }
    #[test]
    fn advertised_body_owner_is_checked_even_when_no_callable_has_that_owner() {
        let mut check = expression();
        check.kind = Kind::Body;
        let mut declaration = check.occurrences.iter().next().unwrap().clone();
        declaration.syntax_kind = SyntaxKind::StmtFunctionDef;
        declaration.structural_path = vec![1];
        declaration.end = 3;
        check.occurrences.insert(declaration.clone()).unwrap();
        let callable = CallableEntity::Source {
            declaration: declaration.id(),
            kind: CallableKind::Function,
        };
        let entity = EntityRef::Callable {
            callable: callable.id(),
        };
        let non_callable = EntityRef::Occurrence {
            occurrence: declaration.id(),
        };
        check.callables.insert(callable).unwrap();
        check.refs.insert(entity.clone()).unwrap();
        check.refs.insert(non_callable.clone()).unwrap();
        let definition = super::super::configuration::base_completion().1;
        let frame = analysis::base_completion::AnalysisInvocation::new(
            id(1),
            id(2),
            definition.id(),
            None,
            [],
        )
        .0;
        let row = SourceBodyCompletion {
            invocation: frame.id(),
            owner: non_callable.id(),
            declaration: declaration.id(),
            qualification: check.qualifications.iter().next().unwrap().id(),
            outcome: CompletionOutcome::Normal.id(),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            sources: ordered_digest(
                "base-source-body-sources",
                std::iter::empty::<Id<BodySource>>(),
            ),
            releases: KeySink::new("base-source-body-releases").finish(),
        };
        check.completion.insert(frame).unwrap();
        check.definitions.insert(definition).unwrap();
        check.bodies.insert(row).unwrap();
        assert!(Box::new(check).finish().is_err());
    }

    fn dishonest_header(kind: Kind) -> Check {
        use crate::domain::normalized::{
            links::LinkReason,
            signature_applicability::{AuthorityReason, BindingAuthority},
        };
        let mut check = expression();
        check.kind = kind;
        let source = check.artifacts.iter().next().unwrap().id();
        let occurrence = |offset, syntax_kind| Occurrence {
            source,
            start: offset,
            end: offset + 1,
            syntax_kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![offset as i32],
        };
        let site = occurrence(1, SyntaxKind::ExprCall);
        let declaration = occurrence(2, SyntaxKind::StmtFunctionDef);
        let callable = CallableEntity::Source {
            declaration: declaration.id(),
            kind: CallableKind::Function,
        };
        let callee = EntityRef::Callable {
            callable: callable.id(),
        };
        let non_caller = EntityRef::Occurrence {
            occurrence: site.id(),
        };
        let owner = OccurrenceOwnership {
            occurrence: site.id(),
            owner: site.id(),
            entity: non_caller.id(),
        };
        let event = NormalizedCallEvent {
            site: site.id(),
            origin: CallOrigin::explicit(),
            context: id(2),
            owner: owner.id(),
        };
        let native = CallAlternativeSource::Native { target: id(11) };
        let alternative = NormalizedCallAlternative {
            event: event.id(),
            source: native.id(),
            resolution: None,
            correspondence: None,
            entity: Some(callee.id()),
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        };
        let qualification = check.qualifications.iter().next().unwrap().id();
        let syntax = CallSyntax::new(qualification, site.id(), site.id(), false, &[])
            .unwrap()
            .0;
        let attempt = CallBindingAttempt {
            alternative: alternative.id(),
            variant: None,
            syntax: Some(syntax.id()),
            policy: ContentHash::of(b"fidelity-fixture"),
            event: event.id(),
            signature: None,
            arguments: None,
            receiver: Receiver::None.id(),
            receiver_assessment: None,
            dispatch_member: None,
            effective: None,
            adjustment: SignatureAdjustment::None,
            authority: BindingAuthority::EffectiveInvocation,
            authority_reason: AuthorityReason::Established,
            outcome: BindingOutcome::Bound,
            reason: BindingReason::Bound,
            refusal: None,
            bindings: ContentHash::of(b"empty"),
        };
        let definition = super::super::configuration::source_calls().1;
        let frame =
            analysis::source_call::AnalysisInvocation::new(id(1), id(2), definition.id(), None, [])
                .0;
        let header = SourceCallHeader {
            invocation: frame.id(),
            event: event.id(),
            attempt: attempt.id(),
            owner: non_caller.id(),
            callee: callee.id(),
            declaration: declaration.id(),
            qualification,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            premises: ordered_digest(
                "source-call-header-premises",
                std::iter::empty::<Id<analysis::native::NativeAssertionPremise>>(),
            ),
        };
        let body_definition = super::super::configuration::base_completion().1;
        let body_frame = analysis::base_completion::AnalysisInvocation::new(
            id(1),
            id(2),
            body_definition.id(),
            None,
            [],
        )
        .0;
        let body = SourceBodyCompletion {
            invocation: body_frame.id(),
            owner: callee.id(),
            declaration: declaration.id(),
            qualification,
            outcome: CompletionOutcome::Normal.id(),
            status: header.status,
            sources: ordered_digest(
                "base-source-body-sources",
                std::iter::empty::<Id<BodySource>>(),
            ),
            releases: KeySink::new("base-source-body-releases").finish(),
        };
        let release = SourceFrameRelease {
            header: header.id(),
            body: body.id(),
            arguments: KeySink::new("source-frame-arguments").finish(),
            release: super::super::evaluation::ReleaseSafety::Closed,
        };
        let invocation = SourceInvocation {
            invocation: frame.id(),
            release: release.id(),
            qualification,
            outcome: SourceCallOutcome::Normal.id(),
            status: header.status,
        };
        check.occurrences.insert(site).unwrap();
        check.occurrences.insert(declaration).unwrap();
        check.callables.insert(callable).unwrap();
        check.refs.insert(callee).unwrap();
        check.refs.insert(non_caller).unwrap();
        check.owners.insert(owner).unwrap();
        check.events.insert(event).unwrap();
        check.alternatives.insert(alternative).unwrap();
        check.syntax.insert(syntax).unwrap();
        check.attempts.insert(attempt).unwrap();
        check.definitions.insert(definition).unwrap();
        check.source.insert(frame).unwrap();
        check.headers.insert(header).unwrap();
        check.definitions.insert(body_definition).unwrap();
        check.completion.insert(body_frame).unwrap();
        check.bodies.insert(body).unwrap();
        check.releases.insert(release).unwrap();
        check
            .source_outcomes
            .insert(SourceCallOutcome::Normal)
            .unwrap();
        check
            .completion_outcomes
            .insert(CompletionOutcome::Normal)
            .unwrap();
        check.invocations.insert(invocation).unwrap();
        check
    }
    #[test]
    fn advertised_header_release_and_invocation_reject_existing_non_source_caller() {
        for kind in [Kind::Header, Kind::Release, Kind::Invocation] {
            let error = Box::new(dishonest_header(kind)).finish().unwrap_err();
            assert!(
                error.to_string().contains("caller is not source callable"),
                "{error}"
            );
        }
    }

    #[test]
    fn advertised_statement_is_checked_against_its_actual_owner() {
        let mut check = expression();
        check.kind = Kind::Statement;
        let mut occurrence = check.occurrences.iter().next().unwrap().clone();
        occurrence.syntax_kind = SyntaxKind::StmtPass;
        occurrence.structural_path = vec![9];
        occurrence.end = 3;
        let owner = EntityRef::Occurrence {
            occurrence: occurrence.id(),
        };
        check.refs.insert(owner.clone()).unwrap();
        check
            .owners
            .insert(OccurrenceOwnership {
                occurrence: occurrence.id(),
                owner: occurrence.id(),
                entity: id(3),
            })
            .unwrap();
        check.occurrences.insert(occurrence.clone()).unwrap();
        let definition = super::super::configuration::base_completion().1;
        let frame = analysis::base_completion::AnalysisInvocation::new(
            id(1),
            id(2),
            definition.id(),
            None,
            [],
        )
        .0;
        let row = StatementCompletion {
            invocation: frame.id(),
            statement: occurrence.id(),
            owner: owner.id(),
            qualification: check.qualifications.iter().next().unwrap().id(),
            outcome: CompletionOutcome::Normal.id(),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            sources: ordered_digest(
                "base-completion-sources",
                std::iter::empty::<Id<CompletionSource>>(),
            ),
            entered: ordered_digest(
                "base-completion-entered",
                std::iter::empty::<Id<Occurrence>>(),
            ),
        };
        check.definitions.insert(definition).unwrap();
        check.completion.insert(frame).unwrap();
        check.statements.insert(row).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("occurrence owner differs"),
            "{error}"
        );
    }

    fn enriched_statement() -> Check {
        use super::super::enriched_records as e;
        let mut check = expression();
        check.kind = Kind::EnrichedStatement;
        let mut occurrence = check.occurrences.iter().next().unwrap().clone();
        occurrence.syntax_kind = SyntaxKind::StmtPass;
        occurrence.structural_path = vec![8];
        let owner = EntityRef::Occurrence {
            occurrence: occurrence.id(),
        };
        check.refs.insert(owner.clone()).unwrap();
        check.occurrences.insert(occurrence.clone()).unwrap();
        check
            .owners
            .insert(OccurrenceOwnership {
                occurrence: occurrence.id(),
                owner: occurrence.id(),
                entity: owner.id(),
            })
            .unwrap();
        let (parameters, definition) = super::super::configuration::enriched_execution(id(44));
        let frame = analysis::enriched_execution::AnalysisInvocation::new(
            id(1),
            id(2),
            definition.id(),
            None,
            [],
        )
        .0;
        let row = e::StatementExecution {
            invocation: frame.id(),
            statement: occurrence.id(),
            owner: owner.id(),
            qualification: check.qualifications.iter().next().unwrap().id(),
            outcome: e::ExecutionOutcome::Normal.id(),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            sources: ordered_digest(
                "execution-sources",
                std::iter::empty::<Id<e::ExecutionSource>>(),
            ),
            entered: ordered_digest("execution-entered", std::iter::empty::<Id<Occurrence>>()),
        };
        check.method_parameters.insert(parameters).unwrap();
        check.definitions.insert(definition).unwrap();
        check.enriched_frames.insert(frame).unwrap();
        check.enriched_statements.insert(row).unwrap();
        check
    }
    #[test]
    fn portable_enriched_admission_refuses_existing_foreign_frame_and_redirected_owner() {
        assert!(Box::new(enriched_statement()).finish().is_ok());
        let mut check = enriched_statement();
        let mut foreign = check.enriched_frames.iter().next().unwrap().clone();
        foreign.input = id(9);
        let mut row = check.enriched_statements.iter().next().unwrap().clone();
        row.invocation = foreign.id();
        check.enriched_frames.insert(foreign).unwrap();
        check.enriched_statements = Rows::new(&check.budget);
        check.enriched_statements.insert(row).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("frame/qualification/source"),
            "{error}"
        );
        let mut check = enriched_statement();
        let other = EntityRef::Occurrence {
            occurrence: check
                .occurrences
                .iter()
                .find(|row| row.syntax_kind == SyntaxKind::ExprNoneLiteral)
                .unwrap()
                .id(),
        };
        check.refs.insert(other.clone()).unwrap();
        let mut row = check.enriched_statements.iter().next().unwrap().clone();
        row.owner = other.id();
        check.enriched_statements = Rows::new(&check.budget);
        check.enriched_statements.insert(row).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("occurrence owner differs"),
            "{error}"
        );
    }
    #[test]
    fn portable_enriched_admission_checks_complete_ordered_existing_member_domain() {
        use super::super::enriched_records as e;
        let mut check = enriched_statement();
        let source = e::ExecutionSource::BaseEvaluation {
            evaluation: check.evaluations.iter().next().unwrap().id(),
        };
        let row = check.enriched_statements.iter().next().unwrap();
        check
            .enriched_members
            .insert(e::ExecutionMember {
                execution: row.id(),
                ordinal: 1,
                source: source.id(),
            })
            .unwrap();
        check.enriched_sources.insert(source).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("member ordinal domain"),
            "{error}"
        );
        let mut check = enriched_statement();
        let mut row = check.enriched_statements.iter().next().unwrap().clone();
        row.sources = ContentHash::of(b"omitted selected existing source");
        check.enriched_statements = Rows::new(&check.budget);
        check.enriched_statements.insert(row).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("ordered member digest"),
            "{error}"
        );
    }
    #[test]
    fn portable_enriched_modeled_admission_refuses_existing_nonmember_returned_parameter() {
        use super::super::modeled_call as m;
        use crate::domain::normalized::{
            bindings::*,
            signature_applicability::{AuthorityReason, BindingAuthority},
        };
        let mut check = enriched_statement();
        check.kind = Kind::Modeled;
        let mut site = check.occurrences.iter().next().unwrap().clone();
        site.syntax_kind = SyntaxKind::ExprCall;
        site.structural_path = vec![10];
        check.occurrences.insert(site.clone()).unwrap();
        let owner = EntityRef::Occurrence {
            occurrence: site.id(),
        };
        let ownership = OccurrenceOwnership {
            occurrence: site.id(),
            owner: site.id(),
            entity: owner.id(),
        };
        check.refs.insert(owner.clone()).unwrap();
        check.owners.insert(ownership.clone()).unwrap();
        let qualification = check.qualifications.iter().next().unwrap().id();
        let event = NormalizedCallEvent {
            site: site.id(),
            origin: CallOrigin::explicit(),
            context: id(2),
            owner: ownership.id(),
        };
        let syntax = CallSyntax::new(qualification, site.id(), site.id(), false, &[])
            .unwrap()
            .0;
        let attempt = CallBindingAttempt {
            alternative: id(5),
            variant: None,
            syntax: Some(syntax.id()),
            policy: ContentHash::of(b"fidelity-fixture"),
            event: event.id(),
            signature: None,
            arguments: None,
            receiver: Receiver::None.id(),
            receiver_assessment: None,
            dispatch_member: None,
            effective: None,
            adjustment: SignatureAdjustment::None,
            authority: BindingAuthority::EffectiveInvocation,
            authority_reason: AuthorityReason::Established,
            outcome: BindingOutcome::Bound,
            reason: BindingReason::Bound,
            refusal: None,
            bindings: ContentHash::of(b"empty"),
        };
        let parameter = SignatureParameter {
            signature: id(11),
            ordinal: 0,
            shape: id(12),
        };
        let model = models::AuthoredModel {
            catalog: id(44),
            target: id(13),
            revision: 1,
            phase: CallPhase::Call,
        };
        let row = m::ModeledCallEvaluation {
            invocation: check.enriched_frames.iter().next().unwrap().id(),
            expression: site.id(),
            owner: owner.id(),
            qualification,
            model: model.id(),
            catalog: id(44),
            attempt: attempt.id(),
            event: event.id(),
            returned_formal: parameter.id(),
            returned_actual: site.id(),
            arguments: KeySink::new("modeled-call-arguments").finish(),
            release: super::super::evaluation::ReleaseSafety::Closed,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
        };
        check.events.insert(event).unwrap();
        check.syntax.insert(syntax).unwrap();
        check.attempts.insert(attempt).unwrap();
        check.parameters.insert(parameter).unwrap();
        check.authored_models.insert(model).unwrap();
        check.modeled.insert(row).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("returned parameter is not an actual member"),
            "{error}"
        );
    }

    #[test]
    fn portable_source_header_refuses_existing_non_bound_attempt() {
        let mut check = dishonest_header(Kind::Header);
        let mut attempt = check.attempts.iter().next().unwrap().clone();
        attempt.outcome = BindingOutcome::Undetermined;
        attempt.reason = BindingReason::MissingSignature;
        let mut header = check.headers.iter().next().unwrap().clone();
        header.attempt = attempt.id();
        check.attempts = Rows::new(&check.budget);
        check.attempts.insert(attempt).unwrap();
        check.headers = Rows::new(&check.budget);
        check.headers.insert(header).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("binding is not Bound"),
            "{error}"
        );
    }

    fn enriched_frames() -> Check {
        let mut check = enriched_statement();
        check.kind = Kind::EnrichedFrames;
        let source_definition = super::super::configuration::source_calls().1;
        let source = analysis::source_call::AnalysisInvocation::new(
            id(1),
            id(2),
            source_definition.id(),
            None,
            [],
        )
        .0;
        let parent = analysis::enriched_execution::InvocationSource::SourceCallAnalysis {
            invocation: source.id(),
        };
        let definition = check.enriched_frames.iter().next().unwrap().definition;
        let (frame, inputs) = analysis::enriched_execution::AnalysisInvocation::new(
            id(1),
            id(2),
            definition,
            None,
            [parent.id()],
        );
        check.definitions.insert(source_definition).unwrap();
        check.source.insert(source).unwrap();
        check.enriched_parents.insert(parent).unwrap();
        check.enriched_frames = Rows::new(&check.budget);
        check.enriched_frames.insert(frame).unwrap();
        for input in inputs {
            check.enriched_inputs.insert(input).unwrap();
        }
        check
    }
    #[test]
    fn portable_enriched_frame_inventory_checks_omitted_parent_and_absent_whole_frame() {
        assert!(Box::new(enriched_frames()).finish().is_ok());
        let mut check = enriched_frames();
        let frame = check.enriched_frames.iter().next().unwrap();
        let omitted = analysis::enriched_execution::AnalysisInvocation::new(
            frame.input,
            frame.context,
            frame.definition,
            None,
            [],
        )
        .0;
        check.enriched_frames = Rows::new(&check.budget);
        check.enriched_frames.insert(omitted).unwrap();
        check.enriched_inputs = Rows::new(&check.budget);
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("complete SourceCall parent domain"),
            "{error}"
        );
        let mut check = enriched_frames();
        check.enriched_frames = Rows::new(&check.budget);
        check.enriched_inputs = Rows::new(&check.budget);
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error.to_string().contains("complete actual frame domain"),
            "{error}"
        );
        let mut check = enriched_frames();
        let mut extra = check.source.iter().next().unwrap().clone();
        extra.sources = ContentHash::of(b"another actual same-context source capture");
        check.source.insert(extra).unwrap();
        let error = Box::new(check).finish().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("complete SourceCall parent domain"),
            "{error}"
        );
    }
}
