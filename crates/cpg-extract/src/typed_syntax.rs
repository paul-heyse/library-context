//! Typed syntax emission from Pyrefly's retained Ruff parse (ADR-0085).
//!
//! This is an identifier-observation subset, not complete syntax-family coverage. The caller
//! supplies the AST and text from the same pinned transaction, retains every captured byte,
//! and publishes only after the generation's shared invariants succeed. No legacy row or ID
//! enters this boundary. Emission is incremental; any callback/refusal aborts the attempt.
use lctx_model::domain::{ContentHash, ModelError, Record, assertion::*, attribution::*, source::*};
use ruff_python_ast::{AnyNodeRef, ExprContext, ModModule, NodeKind};
use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
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

/// Deterministic emission/depth refusal bounds, independent of display limits.
/// Ruff can still dispatch later siblings after refusal; this is not a total traversal-work bound.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxLimits { pub nodes: usize, pub depth: usize }
impl Default for SyntaxLimits {
    fn default() -> Self { Self { nodes: 1_000_000, depth: 256 } }
}

pub fn emit(
    ast: &ModModule, text: &str, invocation: SyntaxInvocation<'_>, limits: SyntaxLimits,
    mut output: impl FnMut(SyntaxEvent) -> Result<(), ModelError>,
) -> Result<usize, ModelError> {
    if invocation.source.content != ContentHash::of(text.as_bytes())
        || usize::try_from(invocation.source.byte_len).ok() != Some(text.len()) {
        return Err(invalid("analyzer text differs from captured artifact"));
    }
    if invocation.run.input != invocation.source.input
        || invocation.run.context != invocation.qualification.context
        || invocation.surface.provider != invocation.run.provider
        || invocation.surface.family != FactFamily::Syntax {
        return Err(invalid("syntax invocation attribution differs from its input/context/provider/family"));
    }
    if limits.nodes == 0 || limits.depth == 0 || limits.depth > 256 {
        return Err(invalid("invalid syntax traversal limits"));
    }
    let mut visitor = Emitter { invocation, output: &mut output, limits, path: vec![],
        children: vec![0], count: 0, error: None, text };
    let root = AnyNodeRef::from(ast);
    if visitor.enter_node(root).is_traverse() { visitor.visit_body(&ast.body); }
    visitor.leave_node(root);
    match visitor.error { Some(error) => Err(error), None => Ok(visitor.count) }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }

struct Emitter<'a, F> {
    invocation: SyntaxInvocation<'a>, output: &'a mut F, limits: SyntaxLimits,
    path: Vec<i32>, children: Vec<i32>, count: usize, error: Option<ModelError>, text: &'a str,
}
impl<F: FnMut(SyntaxEvent) -> Result<(), ModelError>> Emitter<'_, F> {
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
impl<'tree, F: FnMut(SyntaxEvent) -> Result<(), ModelError>> SourceOrderVisitor<'tree> for Emitter<'_, F> {
    fn enter_node(&mut self, node: AnyNodeRef<'tree>) -> TraversalSignal {
        // Ruff calls leave_node even after Skip, so every entry owns a balanced stack frame.
        let ordinal = *self.children.last().expect("root counter");
        self.path.push(ordinal); self.children.push(0);
        if self.error.is_some() { return TraversalSignal::Skip; }
        if self.count >= self.limits.nodes || self.path.len() > self.limits.depth {
            self.error = Some(invalid("syntax emission/depth limit exceeded"));
            return TraversalSignal::Skip;
        }
        match self.event(node).and_then(&mut self.output) {
            Ok(()) => { self.count += 1; TraversalSignal::Traverse }
            Err(error) => { self.error = Some(error); TraversalSignal::Skip }
        }
    }
    fn leave_node(&mut self, _: AnyNodeRef<'tree>) {
        self.children.pop(); self.path.pop();
        let next = self.children.last_mut().expect("parent counter");
        match next.checked_add(1) {
            Some(value) => *next = value,
            None if self.error.is_none() => self.error = Some(invalid("syntax child ordinal overflow")),
            None => {},
        }
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

