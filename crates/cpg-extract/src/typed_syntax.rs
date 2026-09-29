//! Typed syntax emission from Pyrefly's retained Ruff parse (ADR-0085).
//!
//! This is an identifier-observation subset, not complete syntax-family coverage. The caller
//! supplies the AST and text from the same pinned transaction, retains every captured byte,
//! and publishes only after the generation's shared invariants succeed. No legacy row or ID
//! enters this boundary. Emission is incremental; any callback/refusal aborts the attempt.
use lctx_model::domain::{ContentHash, ModelError, Record, assertion::*, attribution::*, source::*};
use ruff_python_ast::{Alias, AnyNodeRef, Arguments, BoolOp, BytesLiteral, CmpOp, Comprehension, Decorator, ElifElseClause, ExceptHandler, Expr,
    ExprContext, FString, Identifier, InterpolatedStringElement, Keyword, MatchCase, Mod, ModModule, NodeKind, Operator, Parameter,
    ParameterWithDefault, Parameters, Pattern, PatternArguments, PatternKeyword, Singleton, Stmt, StringLiteral, TString, TypeParam,
    TypeParams, UnaryOp, WithItem};
use ruff_python_ast::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
use ruff_text_size::Ranged;

/// One structural occurrence and, for name/identifier leaves, its qualified observation.
/// Parent traversal never copies the entire source slice into every enclosing node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxEvent {
    pub occurrence: Occurrence,
    pub observation: Option<(SyntaxObservation, Evidence, SyntaxSupport)>,
}

pub struct SyntaxInvocation<'a> {
    pub source: &'a SourceArtifact,
    pub qualification: &'a AssertionQualification,
    pub run: &'a ProviderRun,
    pub surface: &'a ProviderSurface,
}

/// Deterministic refusal bounds on one artifact's syntax traversal, independent of display limits.
/// The source is admitted before parsing, which bounds the tree and so also the residual
/// callbacks Ruff makes into a halted visitor: one immediate return per remaining child slot.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxLimits { pub nodes: usize, pub depth: usize, pub source_bytes: usize, pub callbacks: usize }
impl Default for SyntaxLimits {
    fn default() -> Self { Self { nodes: 1_000_000, depth: 256, source_bytes: 16 << 20, callbacks: 8_000_000 } }
}
/// The traversal work of one emission: nodes emitted, visitor callbacks before the traversal
/// halted, residual callbacks after it halted, and the deepest structural path entered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyntaxWork { pub emitted: usize, pub callbacks: usize, pub residual: usize, pub deepest: usize }
/// The bound that refused an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxLimit { SourceBytes, Nodes, Depth, Callbacks }
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    /// A declared bound refused the artifact; its coverage is `Unavailable(ResourceRefused)`.
    #[error("syntax {limit:?} limit refused the artifact ({work:?})")]
    Refused { limit: SyntaxLimit, work: SyntaxWork },
    #[error(transparent)]
    Model(#[from] ModelError),
}
impl SyntaxError {
    /// The coverage reason of a refusal; other errors abort the attempt.
    pub fn reason(&self) -> Option<ObligationKind> { matches!(self, Self::Refused { .. }).then_some(ObligationKind::ResourceRefused) }
}

/// Admit an artifact before it is handed to the provider: an oversized source is refused, never parsed.
pub fn admit(source: &SourceArtifact, limits: SyntaxLimits) -> Result<(), SyntaxError> {
    match usize::try_from(source.byte_len) {
        Ok(bytes) if bytes <= limits.source_bytes => Ok(()),
        _ => Err(SyntaxError::Refused { limit: SyntaxLimit::SourceBytes, work: SyntaxWork::default() }),
    }
}

pub fn emit(
    ast: &ModModule, text: &str, invocation: SyntaxInvocation<'_>, limits: SyntaxLimits,
    mut output: impl FnMut(SyntaxEvent) -> Result<(), ModelError>,
) -> Result<SyntaxWork, SyntaxError> {
    if limits.nodes == 0 || limits.depth == 0 || limits.depth > 256 || limits.callbacks == 0 {
        return Err(invalid("invalid syntax traversal limits").into());
    }
    admit(invocation.source, limits)?;
    if text.len() > limits.source_bytes { return Err(SyntaxError::Refused { limit: SyntaxLimit::SourceBytes, work: SyntaxWork::default() }); }
    if invocation.source.content != ContentHash::of(text.as_bytes())
        || usize::try_from(invocation.source.byte_len).ok() != Some(text.len()) {
        return Err(invalid("analyzer text differs from captured artifact").into());
    }
    if invocation.run.input != invocation.source.input
        || invocation.run.context != invocation.qualification.context
        || invocation.surface.provider != invocation.run.provider
        || invocation.surface.family != FactFamily::Syntax {
        return Err(invalid("syntax invocation attribution differs from its input/context/provider/family").into());
    }
    let mut visitor = Emitter { invocation, output: &mut output, limits, path: vec![],
        children: vec![0], work: SyntaxWork::default(), error: None, text };
    let root = AnyNodeRef::from(ast);
    if visitor.enter_node(root).is_traverse() { visitor.visit_body(&ast.body); }
    visitor.leave_node(root);
    match visitor.error {
        Some(Halt::Limit(limit)) => Err(SyntaxError::Refused { limit, work: visitor.work }),
        Some(Halt::Model(error)) => Err(error.into()),
        None => Ok(visitor.work),
    }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }

/// Why the traversal halted.
enum Halt { Limit(SyntaxLimit), Model(ModelError) }
struct Emitter<'a, F> {
    invocation: SyntaxInvocation<'a>, output: &'a mut F, limits: SyntaxLimits,
    path: Vec<i32>, children: Vec<i32>, work: SyntaxWork, error: Option<Halt>, text: &'a str,
}
impl<F: FnMut(SyntaxEvent) -> Result<(), ModelError>> Emitter<'_, F> {
    /// Count one callback; `true` when the traversal has halted and the callback must return.
    fn halted(&mut self) -> bool {
        if self.error.is_some() { self.work.residual += 1; return true; }
        self.work.callbacks += 1;
        if self.work.callbacks > self.limits.callbacks { self.error = Some(Halt::Limit(SyntaxLimit::Callbacks)); return true; }
        false
    }
    fn event(&self, node: AnyNodeRef<'_>) -> Result<SyntaxEvent, ModelError> {
        let range = node.range();
        let start = usize::from(range.start()); let end = usize::from(range.end());
        let text = self.text.get(start..end).ok_or_else(|| invalid("AST span outside captured UTF-8 source"))?;
        let role = match node {
            AnyNodeRef::ExprName(name) => match name.ctx {
                ExprContext::Load => OccurrenceRole::Read,
                ExprContext::Store => OccurrenceRole::Binding,
                ExprContext::Del | ExprContext::Invalid => OccurrenceRole::Syntax,
            },
            AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_) => OccurrenceRole::Declaration,
            AnyNodeRef::Parameter(_) => OccurrenceRole::Parameter,
            AnyNodeRef::ExprCall(_) => OccurrenceRole::Call,
            AnyNodeRef::WithItem(_) => OccurrenceRole::WithItem,
            AnyNodeRef::Decorator(_) => OccurrenceRole::Decorator,
            AnyNodeRef::StmtReturn(_) => OccurrenceRole::Return,
            AnyNodeRef::ExprYield(_) | AnyNodeRef::ExprYieldFrom(_) => OccurrenceRole::Yield,
            AnyNodeRef::StmtRaise(_) => OccurrenceRole::Raise,
            _ => OccurrenceRole::Syntax,
        };
        let occurrence = Occurrence { source: self.invocation.source.id(), start: start as i64,
            end: end as i64, syntax_kind: syntax_kind(node.kind()), role, structural_path: self.path.clone() };
        occurrence.validate()?;
        let observation = if matches!(node, AnyNodeRef::ExprName(_) | AnyNodeRef::Identifier(_)) {
            let assertion = SyntaxObservation { qualification: self.invocation.qualification.id(),
                occurrence: occurrence.id(), spelling: text.into() };
            let evidence = Evidence::Occurrence { occurrence: occurrence.id() };
            let support = SyntaxSupport { assertion: assertion.id(), run: self.invocation.run.id(),
                surface: self.invocation.surface.id(), evidence: evidence.id(),
                origin: Origin::SourceObservation, mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural };
            Some((assertion, evidence, support))
        } else { None };
        Ok(SyntaxEvent { occurrence, observation })
    }
}
/// Every dispatch point counts as one callback and returns at once after a halt, so a refused
/// artifact costs at most one call per remaining child slot, never a deeper descent.
macro_rules! bounded {
    ($($method:ident: $node:ty => $walk:ident;)+) => { $(
        fn $method(&mut self, node: &'tree $node) {
            if self.halted() { return; }
            source_order::$walk(self, node);
        }
    )+ };
}
impl<'tree, F: FnMut(SyntaxEvent) -> Result<(), ModelError>> SourceOrderVisitor<'tree> for Emitter<'_, F> {
    fn enter_node(&mut self, node: AnyNodeRef<'tree>) -> TraversalSignal {
        // Ruff calls leave_node even after Skip, so every entry owns a balanced stack frame.
        let ordinal = *self.children.last().expect("root counter");
        self.path.push(ordinal); self.children.push(0);
        if self.error.is_some() { return TraversalSignal::Skip; }
        self.work.deepest = self.work.deepest.max(self.path.len());
        if self.path.len() > self.limits.depth { self.error = Some(Halt::Limit(SyntaxLimit::Depth)); return TraversalSignal::Skip; }
        if self.work.emitted >= self.limits.nodes { self.error = Some(Halt::Limit(SyntaxLimit::Nodes)); return TraversalSignal::Skip; }
        match self.event(node).and_then(&mut self.output) {
            Ok(()) => { self.work.emitted += 1; TraversalSignal::Traverse }
            Err(error) => { self.error = Some(Halt::Model(error)); TraversalSignal::Skip }
        }
    }
    fn leave_node(&mut self, _: AnyNodeRef<'tree>) {
        self.children.pop(); self.path.pop();
        let next = self.children.last_mut().expect("parent counter");
        match next.checked_add(1) {
            Some(value) => *next = value,
            None if self.error.is_none() => self.error = Some(Halt::Model(invalid("syntax child ordinal overflow"))),
            None => {},
        }
    }
    /// The one sibling loop the visitor owns stops outright after a halt.
    fn visit_body(&mut self, body: &'tree [Stmt]) {
        for stmt in body {
            if self.halted() { return; }
            source_order::walk_stmt(self, stmt);
        }
    }
    fn visit_singleton(&mut self, _: &'tree Singleton) { self.halted(); }
    bounded! {
        visit_mod: Mod => walk_module;
        visit_stmt: Stmt => walk_stmt;
        visit_annotation: Expr => walk_annotation;
        visit_expr: Expr => walk_expr;
        visit_decorator: Decorator => walk_decorator;
        visit_bool_op: BoolOp => walk_bool_op;
        visit_operator: Operator => walk_operator;
        visit_unary_op: UnaryOp => walk_unary_op;
        visit_cmp_op: CmpOp => walk_cmp_op;
        visit_comprehension: Comprehension => walk_comprehension;
        visit_except_handler: ExceptHandler => walk_except_handler;
        visit_arguments: Arguments => walk_arguments;
        visit_parameters: Parameters => walk_parameters;
        visit_parameter: Parameter => walk_parameter;
        visit_parameter_with_default: ParameterWithDefault => walk_parameter_with_default;
        visit_keyword: Keyword => walk_keyword;
        visit_alias: Alias => walk_alias;
        visit_with_item: WithItem => walk_with_item;
        visit_type_params: TypeParams => walk_type_params;
        visit_type_param: TypeParam => walk_type_param;
        visit_match_case: MatchCase => walk_match_case;
        visit_pattern: Pattern => walk_pattern;
        visit_pattern_arguments: PatternArguments => walk_pattern_arguments;
        visit_pattern_keyword: PatternKeyword => walk_pattern_keyword;
        visit_elif_else_clause: ElifElseClause => walk_elif_else_clause;
        visit_f_string: FString => walk_f_string;
        visit_interpolated_string_element: InterpolatedStringElement => walk_interpolated_string_element;
        visit_t_string: TString => walk_t_string;
        visit_string_literal: StringLiteral => walk_string_literal;
        visit_bytes_literal: BytesLiteral => walk_bytes_literal;
        visit_identifier: Identifier => walk_identifier;
    }
}

#[deny(clippy::wildcard_enum_match_arm)]
fn syntax_kind(k: NodeKind) -> SyntaxKind {
    macro_rules! map {
        ($($v:ident),+ $(,)?) => {
            match k { $(NodeKind::$v => SyntaxKind::$v,)+ }
        };
    }
    map!(
        ModModule,
        ModExpression,
        StmtFunctionDef,
        StmtClassDef,
        StmtReturn,
        StmtDelete,
        StmtTypeAlias,
        StmtAssign,
        StmtAugAssign,
        StmtAnnAssign,
        StmtFor,
        StmtWhile,
        StmtIf,
        StmtWith,
        StmtMatch,
        StmtRaise,
        StmtTry,
        StmtAssert,
        StmtImport,
        StmtImportFrom,
        StmtGlobal,
        StmtNonlocal,
        StmtExpr,
        StmtPass,
        StmtBreak,
        StmtContinue,
        StmtIpyEscapeCommand,
        ExprBoolOp,
        ExprNamed,
        ExprBinOp,
        ExprUnaryOp,
        ExprLambda,
        ExprIf,
        ExprDict,
        ExprSet,
        ExprListComp,
        ExprSetComp,
        ExprDictComp,
        ExprGenerator,
        ExprAwait,
        ExprYield,
        ExprYieldFrom,
        ExprCompare,
        ExprCall,
        ExprFString,
        ExprTString,
        ExprStringLiteral,
        ExprBytesLiteral,
        ExprNumberLiteral,
        ExprBooleanLiteral,
        ExprNoneLiteral,
        ExprEllipsisLiteral,
        ExprAttribute,
        ExprSubscript,
        ExprStarred,
        ExprName,
        ExprList,
        ExprTuple,
        ExprSlice,
        ExprIpyEscapeCommand,
        ExceptHandlerExceptHandler,
        InterpolatedElement,
        InterpolatedStringLiteralElement,
        PatternMatchValue,
        PatternMatchSingleton,
        PatternMatchSequence,
        PatternMatchMapping,
        PatternMatchClass,
        PatternMatchStar,
        PatternMatchAs,
        PatternMatchOr,
        TypeParamTypeVar,
        TypeParamTypeVarTuple,
        TypeParamParamSpec,
        InterpolatedStringFormatSpec,
        PatternArguments,
        PatternKeyword,
        Comprehension,
        Arguments,
        Parameters,
        Parameter,
        ParameterWithDefault,
        Keyword,
        Alias,
        WithItem,
        MatchCase,
        Decorator,
        ElifElseClause,
        TypeParams,
        FString,
        TString,
        StringLiteral,
        BytesLiteral,
        Identifier,
    )
}

