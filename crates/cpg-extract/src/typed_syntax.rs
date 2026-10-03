//! Typed syntax emission from the canonical latest-Ruff parse (ADR-0085, plan A4).
//!
//! Every parse node is one occurrence, with its placement (the parent occurrence, the parent's
//! field that holds it and its ordinal there) and the detail its bytes do not state: operator
//! kinds and parsed literal values. The caller supplies the AST and text from the same pinned
//! transaction, retains every captured byte, and publishes only after the generation's shared
//! invariants succeed. Emission is incremental: a sink or model error aborts the attempt; a
//! declared bound refuses the artifact, and [`SyntaxError::coverage`] states what it publishes.
use lctx_model::domain::{
    ContentHash, EvidenceBytes, Id, ModelError, Record,
    assertion::*,
    attribution::*,
    lexical::SyntaxField,
    source::*,
    syntax::{OperatorKind, SyntaxDetail},
    value::Literal,
};
use ruff_python_ast_latest::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
use ruff_python_ast_latest::{
    Alias, AnyNodeRef, Arguments, BoolOp, BytesLiteral, CmpOp, Comprehension, Decorator,
    ElifElseClause, ExceptHandler, Expr, ExprContext, FString, Identifier,
    InterpolatedStringElement, Keyword, MatchCase, Mod, ModModule, NodeKind, Operator, Parameter,
    ParameterWithDefault, Parameters, Pattern, PatternArguments, PatternKeyword, Singleton, Stmt,
    StringLiteral, TString, TypeParam, TypeParams, UnaryOp, WithItem,
};
use ruff_text_size_latest::{Ranged, TextRange};

/// One structural occurrence and, for name/identifier leaves, its qualified observation, with its
/// placement and details. Parent traversal never copies the entire source slice into every
/// enclosing node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxEvent {
    pub occurrence: Occurrence,
    pub observation: Option<(SyntaxObservation, Evidence, SyntaxSupport)>,
    /// The parent occurrence, the parent's field holding this one and its ordinal in that field;
    /// none for the module.
    pub placement: Option<(Id<Occurrence>, SyntaxField, i64)>,
    /// The occurrence's details in order, each with the literal it names.
    pub details: Vec<(SyntaxDetail, Option<Literal>)>,
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
pub struct SyntaxLimits {
    pub nodes: usize,
    pub depth: usize,
    pub source_bytes: usize,
    pub callbacks: usize,
}
impl Default for SyntaxLimits {
    fn default() -> Self {
        Self {
            nodes: 1_000_000,
            depth: 256,
            source_bytes: 16 << 20,
            callbacks: 8_000_000,
        }
    }
}
/// The traversal work of one emission: nodes emitted, visitor callbacks before the traversal
/// halted, residual callbacks after it halted, and the deepest structural path entered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyntaxWork {
    pub emitted: usize,
    pub callbacks: usize,
    pub residual: usize,
    pub deepest: usize,
}
/// The bound that refused an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxLimit {
    SourceBytes,
    Nodes,
    Depth,
    Callbacks,
}
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    /// A declared bound refused the artifact after `work`.
    #[error("syntax {limit:?} limit refused the artifact ({work:?})")]
    Refused {
        limit: SyntaxLimit,
        work: SyntaxWork,
    },
    #[error(transparent)]
    Model(#[from] ModelError),
}
impl SyntaxError {
    /// The artifact's Syntax coverage after a refusal; `None` for errors that abort the attempt.
    /// A source-size refusal emitted nothing, so the artifact is `Unavailable`. A traversal bound
    /// halts after the emitted prefix, which is kept: every emitted node's ancestors were emitted
    /// before it, so the prefix is a well-formed tree and the artifact is `Partial`.
    pub fn coverage(&self) -> Option<(CoverageStatus, ObligationKind)> {
        match self {
            Self::Refused {
                limit: SyntaxLimit::SourceBytes,
                ..
            } => Some((CoverageStatus::Unavailable, ObligationKind::ResourceRefused)),
            Self::Refused { .. } => {
                Some((CoverageStatus::Partial, ObligationKind::ResourceRefused))
            }
            Self::Model(_) => None,
        }
    }
}

/// Admit an artifact's source size before its syntax is traversed. [`extract`] calls it for every
/// captured Python source before building Pyrefly handles, so a refused source is never handed to
/// the analyzer as a root. Pyrefly may still parse it when an admitted module imports it: such a
/// transitive load stays provider-internal and outside the budget until the Pyrefly stage decides
/// its import policy (plan A4; the pinned fork's `replace-imports-with-any` is a candidate).
pub fn admit(source: &SourceArtifact, limits: SyntaxLimits) -> Result<(), SyntaxError> {
    match usize::try_from(source.byte_len) {
        Ok(bytes) if bytes <= limits.source_bytes => Ok(()),
        _ => Err(SyntaxError::Refused {
            limit: SyntaxLimit::SourceBytes,
            work: SyntaxWork::default(),
        }),
    }
}

pub fn emit(
    ast: &ModModule,
    text: &str,
    invocation: SyntaxInvocation<'_>,
    limits: SyntaxLimits,
    mut output: impl FnMut(SyntaxEvent) -> Result<(), ModelError>,
) -> Result<SyntaxWork, SyntaxError> {
    if limits.nodes == 0 || limits.depth == 0 || limits.depth > 256 || limits.callbacks == 0 {
        return Err(invalid("invalid syntax traversal limits").into());
    }
    admit(invocation.source, limits)?;
    if text.len() > limits.source_bytes {
        return Err(SyntaxError::Refused {
            limit: SyntaxLimit::SourceBytes,
            work: SyntaxWork::default(),
        });
    }
    if invocation.source.content != ContentHash::of(text.as_bytes())
        || usize::try_from(invocation.source.byte_len).ok() != Some(text.len())
    {
        return Err(invalid("analyzer text differs from captured artifact").into());
    }
    if invocation.run.input != invocation.source.input
        || invocation.run.context != invocation.qualification.context
        || invocation.surface.provider != invocation.run.provider
        || invocation.surface.family != FactFamily::Syntax
    {
        return Err(invalid(
            "syntax invocation attribution differs from its input/context/provider/family",
        )
        .into());
    }
    let mut visitor = Emitter {
        invocation,
        output: &mut output,
        limits,
        path: vec![],
        children: vec![0],
        frames: vec![],
        work: SyntaxWork::default(),
        error: None,
        text,
    };
    let root = AnyNodeRef::from(ast);
    if visitor.enter_node(root).is_traverse() {
        visitor.visit_body(&ast.body);
    }
    visitor.leave_node(root);
    match visitor.error {
        Some(Halt::Limit(limit)) => Err(SyntaxError::Refused {
            limit,
            work: visitor.work,
        }),
        Some(Halt::Model(error)) => Err(error.into()),
        None => Ok(visitor.work),
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// Why the traversal halted.
enum Halt {
    Limit(SyntaxLimit),
    Model(ModelError),
}
/// An entered node: its occurrence, its fields' ranges and each field's next ordinal. A node
/// entered after a halt has none.
struct Frame {
    id: Id<Occurrence>,
    fields: Vec<(SyntaxField, TextRange)>,
    next: Vec<(SyntaxField, i64)>,
}
impl Frame {
    /// The field of a child with `range` (the first field containing it, else `Child`) and its
    /// ordinal there.
    fn place(&mut self, range: TextRange) -> (SyntaxField, i64) {
        let field = self
            .fields
            .iter()
            .find(|(_, r)| r.contains_range(range))
            .map_or(SyntaxField::Child, |(f, _)| *f);
        let ordinal = match self.next.iter_mut().find(|(f, _)| *f == field) {
            Some((_, next)) => {
                *next += 1;
                *next - 1
            }
            None => {
                self.next.push((field, 1));
                0
            }
        };
        (field, ordinal)
    }
}
struct Emitter<'a, F> {
    invocation: SyntaxInvocation<'a>,
    output: &'a mut F,
    limits: SyntaxLimits,
    path: Vec<i32>,
    children: Vec<i32>,
    frames: Vec<Option<Frame>>,
    work: SyntaxWork,
    error: Option<Halt>,
    text: &'a str,
}
impl<F: FnMut(SyntaxEvent) -> Result<(), ModelError>> Emitter<'_, F> {
    /// Count one callback; `true` when the traversal has halted and the callback must return.
    fn halted(&mut self) -> bool {
        if self.error.is_some() {
            self.work.residual += 1;
            return true;
        }
        self.work.callbacks += 1;
        if self.work.callbacks > self.limits.callbacks {
            self.error = Some(Halt::Limit(SyntaxLimit::Callbacks));
            return true;
        }
        false
    }
    fn event(&self, node: AnyNodeRef<'_>) -> Result<SyntaxEvent, ModelError> {
        let range = node.range();
        let start = usize::from(range.start());
        let end = usize::from(range.end());
        let text = self
            .text
            .get(start..end)
            .ok_or_else(|| invalid("AST span outside captured UTF-8 source"))?;
        let role = match node {
            AnyNodeRef::ExprName(name) => match name.ctx {
                ExprContext::Load => OccurrenceRole::Read,
                ExprContext::Store => OccurrenceRole::Binding,
                ExprContext::Del | ExprContext::Invalid => OccurrenceRole::Syntax,
            },
            AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_) => {
                OccurrenceRole::Declaration
            }
            AnyNodeRef::Parameter(_) => OccurrenceRole::Parameter,
            AnyNodeRef::ExprCall(_) => OccurrenceRole::Call,
            AnyNodeRef::WithItem(_) => OccurrenceRole::WithItem,
            AnyNodeRef::Decorator(_) => OccurrenceRole::Decorator,
            AnyNodeRef::StmtReturn(_) => OccurrenceRole::Return,
            AnyNodeRef::ExprYield(_) | AnyNodeRef::ExprYieldFrom(_) => OccurrenceRole::Yield,
            AnyNodeRef::StmtRaise(_) => OccurrenceRole::Raise,
            _ => OccurrenceRole::Syntax,
        };
        let occurrence = Occurrence {
            source: self.invocation.source.id(),
            start: start as i64,
            end: end as i64,
            syntax_kind: syntax_kind(node.kind()),
            role,
            structural_path: self.path.clone(),
        };
        occurrence.validate()?;
        let placement = match self.frames.last() {
            Some(Some(frame)) => Some(frame.id),
            Some(None) => return Err(invalid("a node was entered under a halted parent")),
            None => None,
        };
        let observation = if matches!(node, AnyNodeRef::ExprName(_) | AnyNodeRef::Identifier(_)) {
            let assertion = SyntaxObservation {
                qualification: self.invocation.qualification.id(),
                occurrence: occurrence.id(),
                spelling: text.into(),
            };
            let evidence = Evidence::Occurrence {
                occurrence: occurrence.id(),
            };
            let support = SyntaxSupport {
                assertion: assertion.id(),
                run: self.invocation.run.id(),
                surface: self.invocation.surface.id(),
                evidence: evidence.id(),
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            };
            Some((assertion, evidence, support))
        } else {
            None
        };
        Ok(SyntaxEvent {
            occurrence,
            observation,
            placement: placement.map(|id| (id, SyntaxField::Child, 0)),
            details: details(node),
        })
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
impl<'tree, F: FnMut(SyntaxEvent) -> Result<(), ModelError>> SourceOrderVisitor<'tree>
    for Emitter<'_, F>
{
    fn enter_node(&mut self, node: AnyNodeRef<'tree>) -> TraversalSignal {
        // Ruff calls leave_node even after Skip, so every entry owns a balanced stack frame.
        let ordinal = *self.children.last().expect("root counter");
        self.path.push(ordinal);
        self.children.push(0);
        if self.error.is_some() {
            self.frames.push(None);
            return TraversalSignal::Skip;
        }
        self.work.deepest = self.work.deepest.max(self.path.len());
        if self.path.len() > self.limits.depth {
            self.error = Some(Halt::Limit(SyntaxLimit::Depth));
            self.frames.push(None);
            return TraversalSignal::Skip;
        }
        if self.work.emitted >= self.limits.nodes {
            self.error = Some(Halt::Limit(SyntaxLimit::Nodes));
            self.frames.push(None);
            return TraversalSignal::Skip;
        }
        let event = self.event(node).map(|mut event| {
            if let (Some(placement), Some(Some(parent))) =
                (event.placement.as_mut(), self.frames.last_mut())
            {
                let (field, ordinal) = parent.place(node.range());
                *placement = (parent.id, field, ordinal);
            }
            event
        });
        let id = event.as_ref().ok().map(|event| event.occurrence.id());
        match event.and_then(&mut self.output) {
            Ok(()) => {
                self.work.emitted += 1;
                self.frames.push(Some(Frame {
                    id: id.expect("emitted event"),
                    fields: fields(node),
                    next: vec![],
                }));
                TraversalSignal::Traverse
            }
            Err(error) => {
                self.error = Some(Halt::Model(error));
                self.frames.push(None);
                TraversalSignal::Skip
            }
        }
    }
    fn leave_node(&mut self, _: AnyNodeRef<'tree>) {
        self.frames.pop();
        self.children.pop();
        self.path.pop();
        let next = self.children.last_mut().expect("parent counter");
        match next.checked_add(1) {
            Some(value) => *next = value,
            None if self.error.is_none() => {
                self.error = Some(Halt::Model(invalid("syntax child ordinal overflow")))
            }
            None => {}
        }
    }
    /// The one sibling loop the visitor owns stops outright after a halt.
    fn visit_body(&mut self, body: &'tree [Stmt]) {
        for stmt in body {
            if self.halted() {
                return;
            }
            source_order::walk_stmt(self, stmt);
        }
    }
    fn visit_singleton(&mut self, _: &'tree Singleton) {
        self.halted();
    }
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

fn suite(body: &[Stmt]) -> Option<TextRange> {
    Some(TextRange::new(body.first()?.start(), body.last()?.end()))
}
/// The fields a node's children sit in, with their ranges. A child's field is the first one whose
/// range contains it; anything else is `Child`.
fn fields(node: AnyNodeRef<'_>) -> Vec<(SyntaxField, TextRange)> {
    use SyntaxField as F;
    let mut v: Vec<(SyntaxField, TextRange)> = Vec::new();
    let mut add = |f: SyntaxField, r: Option<TextRange>| {
        if let Some(r) = r {
            v.push((f, r));
        }
    };
    match node {
        AnyNodeRef::ModModule(m) => add(F::Body, suite(&m.body)),
        AnyNodeRef::StmtFunctionDef(f) => {
            for d in &f.decorator_list {
                add(F::Decorator, Some(d.range()));
            }
            for p in f.parameters.iter_non_variadic_params() {
                add(F::Default, p.default.as_ref().map(|d| d.range()));
            }
            add(F::Annotation, f.returns.as_ref().map(|r| r.range()));
            add(F::Body, suite(&f.body));
        }
        AnyNodeRef::StmtClassDef(c) => {
            for d in &c.decorator_list {
                add(F::Decorator, Some(d.range()));
            }
            if let Some(a) = &c.arguments {
                add(F::Argument, Some(a.range()));
            }
            add(F::Body, suite(&c.body));
        }
        AnyNodeRef::StmtIf(s) => {
            add(F::Test, Some(s.test.range()));
            add(F::Body, suite(&s.body));
            for c in &s.elif_else_clauses {
                add(F::Orelse, Some(c.range()));
            }
        }
        AnyNodeRef::ElifElseClause(c) => {
            add(F::Test, c.test.as_ref().map(Ranged::range));
            add(F::Body, suite(&c.body));
        }
        AnyNodeRef::StmtWhile(s) => {
            add(F::Test, Some(s.test.range()));
            add(F::Body, suite(&s.body));
            add(F::Orelse, suite(&s.orelse));
        }
        AnyNodeRef::StmtFor(s) => {
            add(F::Target, Some(s.target.range()));
            add(F::Iter, Some(s.iter.range()));
            add(F::Body, suite(&s.body));
            add(F::Orelse, suite(&s.orelse));
        }
        AnyNodeRef::StmtTry(s) => {
            add(F::Body, suite(&s.body));
            for h in &s.handlers {
                add(F::Handler, Some(h.range()));
            }
            add(F::Orelse, suite(&s.orelse));
            add(F::Finalbody, suite(&s.finalbody));
        }
        AnyNodeRef::ExceptHandlerExceptHandler(h) => {
            add(F::Test, h.type_.as_ref().map(|t| t.range()));
            add(F::Body, suite(&h.body));
        }
        AnyNodeRef::StmtRaise(s) => {
            add(F::Exc, s.exc.as_ref().map(|e| e.range()));
            add(F::Cause, s.cause.as_ref().map(|e| e.range()));
        }
        AnyNodeRef::StmtAssert(s) => {
            add(F::Test, Some(s.test.range()));
            add(F::Msg, s.msg.as_ref().map(|e| e.range()));
        }
        AnyNodeRef::StmtReturn(s) => add(F::Value, s.value.as_ref().map(|e| e.range())),
        AnyNodeRef::StmtAssign(s) => {
            for t in &s.targets {
                add(F::Target, Some(t.range()));
            }
            add(F::Value, Some(s.value.range()));
        }
        AnyNodeRef::StmtAnnAssign(s) => {
            add(F::Target, Some(s.target.range()));
            add(F::Annotation, Some(s.annotation.range()));
            add(F::Value, s.value.as_ref().map(|e| e.range()));
        }
        AnyNodeRef::StmtAugAssign(s) => {
            add(F::Target, Some(s.target.range()));
            add(F::Value, Some(s.value.range()));
        }
        AnyNodeRef::StmtWith(s) => {
            for i in &s.items {
                add(F::Item, Some(i.range()));
            }
            add(F::Body, suite(&s.body));
        }
        AnyNodeRef::WithItem(i) => {
            add(F::Value, Some(i.context_expr.range()));
            add(F::Target, i.optional_vars.as_ref().map(|e| e.range()));
        }
        AnyNodeRef::StmtMatch(s) => {
            add(F::Subject, Some(s.subject.range()));
            for c in &s.cases {
                add(F::Case, Some(c.range()));
            }
        }
        AnyNodeRef::MatchCase(c) => {
            add(F::Guard, c.guard.as_ref().map(|e| e.range()));
            add(F::Body, suite(&c.body));
        }
        AnyNodeRef::StmtExpr(s) => add(F::Value, Some(s.value.range())),
        AnyNodeRef::StmtDelete(s) => {
            for t in &s.targets {
                add(F::Target, Some(t.range()));
            }
        }
        AnyNodeRef::ExprCompare(e) => {
            add(F::Left, Some(e.first_operand().range()));
            for c in e.comparators() {
                add(F::Right, Some(c.range()));
            }
        }
        AnyNodeRef::ExprBinOp(e) => {
            add(F::Left, Some(e.left.range()));
            add(F::Right, Some(e.right.range()));
        }
        AnyNodeRef::ExprBoolOp(e) => {
            for x in &e.values {
                add(F::Operand, Some(x.range()));
            }
        }
        AnyNodeRef::ExprUnaryOp(e) => add(F::Operand, Some(e.operand.range())),
        AnyNodeRef::ExprAttribute(e) => add(F::Value, Some(e.value.range())),
        AnyNodeRef::ExprSubscript(e) => {
            add(F::Value, Some(e.value.range()));
            add(F::Slice, Some(e.slice.range()));
        }
        AnyNodeRef::ExprCall(e) => {
            add(F::Callee, Some(e.func.range()));
            for a in e.arguments.iter_source_order() {
                add(F::Argument, Some(a.range()));
            }
        }
        AnyNodeRef::ExprNamed(e) => {
            add(F::Target, Some(e.target.range()));
            add(F::Value, Some(e.value.range()));
        }
        AnyNodeRef::ExprIf(e) => {
            add(F::Test, Some(e.test.range()));
            add(F::Value, Some(e.body.range()));
            add(F::Orelse, Some(e.orelse.range()));
        }
        AnyNodeRef::ExprAwait(e) => add(F::Value, Some(e.value.range())),
        AnyNodeRef::ExprYield(e) => add(F::Value, e.value.as_ref().map(|x| x.range())),
        AnyNodeRef::ExprYieldFrom(e) => add(F::Value, Some(e.value.range())),
        AnyNodeRef::ExprStarred(e) => add(F::Value, Some(e.value.range())),
        AnyNodeRef::Comprehension(c) => {
            add(F::Target, Some(c.target.range()));
            add(F::Iter, Some(c.iter.range()));
            for test in &c.ifs {
                add(F::Test, Some(test.range()));
            }
        }
        AnyNodeRef::ExprGenerator(e) => {
            add(F::Element, Some(e.elt.range()));
            for clause in &e.generators {
                add(F::Item, Some(clause.range()));
            }
        }
        AnyNodeRef::ExprListComp(e) => {
            add(F::Element, Some(e.elt.range()));
            for clause in &e.generators {
                add(F::Item, Some(clause.range()));
            }
        }
        AnyNodeRef::ExprSetComp(e) => {
            add(F::Element, Some(e.elt.range()));
            for clause in &e.generators {
                add(F::Item, Some(clause.range()));
            }
        }
        AnyNodeRef::ExprDictComp(e) => {
            add(F::Left, e.key.as_ref().map(|key| key.range()));
            add(F::Value, Some(e.value.range()));
            for clause in &e.generators {
                add(F::Item, Some(clause.range()));
            }
        }
        AnyNodeRef::ExprLambda(e) => {
            if let Some(ps) = &e.parameters {
                for p in ps.iter_non_variadic_params() {
                    add(F::Default, p.default.as_ref().map(|d| d.range()));
                }
            }
            add(F::Value, Some(e.body.range()));
        }
        AnyNodeRef::ExprList(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        AnyNodeRef::ExprTuple(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        AnyNodeRef::ExprSet(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        AnyNodeRef::ParameterWithDefault(p) => {
            add(F::Default, p.default.as_ref().map(|d| d.range()));
        }
        AnyNodeRef::Parameter(p) => add(F::Annotation, p.annotation.as_ref().map(|a| a.range())),
        _ => {}
    }
    v
}
fn operator(op: Operator) -> OperatorKind {
    use OperatorKind as K;
    match op {
        Operator::Add => K::Add,
        Operator::Sub => K::Sub,
        Operator::Mult => K::Mult,
        Operator::MatMult => K::MatMult,
        Operator::Div => K::Div,
        Operator::Mod => K::Mod,
        Operator::Pow => K::Pow,
        Operator::LShift => K::LShift,
        Operator::RShift => K::RShift,
        Operator::BitOr => K::BitOr,
        Operator::BitXor => K::BitXor,
        Operator::BitAnd => K::BitAnd,
        Operator::FloorDiv => K::FloorDiv,
    }
}
fn comparison(op: CmpOp) -> OperatorKind {
    use OperatorKind as K;
    match op {
        CmpOp::Eq => K::Eq,
        CmpOp::NotEq => K::NotEq,
        CmpOp::Lt => K::Lt,
        CmpOp::LtE => K::LtE,
        CmpOp::Gt => K::Gt,
        CmpOp::GtE => K::GtE,
        CmpOp::Is => K::Is,
        CmpOp::IsNot => K::IsNot,
        CmpOp::In => K::In,
        CmpOp::NotIn => K::NotIn,
    }
}
/// A literal expression's value, when the model can state it exactly: integers in canonical
/// decimal, floats by their bits, strings joined across implicit concatenation, bytes, booleans
/// and `None`. A complex number or an integer the parser keeps as text is not stated.
pub fn literal(expr: &Expr) -> Option<Literal> {
    literal_of(expr.into())
}
fn literal_of(expr: ruff_python_ast_latest::ExprRef<'_>) -> Option<Literal> {
    use ruff_python_ast_latest::ExprRef as Expr;
    let literal = match expr {
        Expr::NumberLiteral(number) => match &number.value {
            ruff_python_ast_latest::Number::Int(int) => Literal::Integer {
                decimal: int
                    .as_i64()
                    .map_or_else(|| int.to_string(), |v| v.to_string()),
            },
            ruff_python_ast_latest::Number::Float(value) => Literal::Float {
                bits: value.to_bits() as i64,
            },
            ruff_python_ast_latest::Number::Complex { .. } => return None,
        },
        Expr::StringLiteral(string) => Literal::String {
            value: string.value.to_str().into(),
        },
        Expr::BytesLiteral(bytes) => Literal::Bytes {
            value: EvidenceBytes(bytes.value.bytes().collect()),
        },
        Expr::BooleanLiteral(boolean) => Literal::Bool {
            value: boolean.value,
        },
        Expr::NoneLiteral(_) => Literal::None,
        _ => return None,
    };
    literal.validate().ok().map(|()| literal)
}
/// What a node's bytes do not state: its operators in order, or its literal value.
fn details(node: AnyNodeRef<'_>) -> Vec<(SyntaxDetail, Option<Literal>)> {
    let op = |kind: OperatorKind| (SyntaxDetail::Operator { operator: kind }, None);
    match node {
        AnyNodeRef::ExprBinOp(e) => vec![op(operator(e.op))],
        AnyNodeRef::StmtWith(s) => vec![(
            SyntaxDetail::WithMode {
                is_async: s.is_async,
            },
            None,
        )],
        AnyNodeRef::StmtTry(s) => vec![(SyntaxDetail::TryMode { is_star: s.is_star }, None)],
        AnyNodeRef::ExceptHandlerExceptHandler(h) => vec![(SyntaxDetail::HandlerName { name: h.name.as_ref().map(|name| name.to_string()) }, None)],
        AnyNodeRef::StmtAugAssign(s) => vec![op(operator(s.op))],
        AnyNodeRef::ExprBoolOp(e) => vec![op(match e.op {
            BoolOp::And => OperatorKind::And,
            BoolOp::Or => OperatorKind::Or,
        })],
        AnyNodeRef::ExprUnaryOp(e) => vec![op(match e.op {
            UnaryOp::Not => OperatorKind::Not,
            UnaryOp::Invert => OperatorKind::Invert,
            UnaryOp::UAdd => OperatorKind::UAdd,
            UnaryOp::USub => OperatorKind::USub,
        })],
        AnyNodeRef::ExprCompare(e) => e.ops.iter().map(|o| op(comparison(*o))).collect(),
        AnyNodeRef::ExprNumberLiteral(_)
        | AnyNodeRef::ExprStringLiteral(_)
        | AnyNodeRef::ExprBytesLiteral(_)
        | AnyNodeRef::ExprBooleanLiteral(_)
        | AnyNodeRef::ExprNoneLiteral(_) => node
            .as_expr_ref()
            .and_then(literal_of)
            .map(|value| {
                vec![(
                    SyntaxDetail::Literal {
                        literal: value.id(),
                    },
                    Some(value),
                )]
            })
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// The model's kind for a Ruff node kind.
pub(crate) fn kind(k: NodeKind) -> SyntaxKind {
    syntax_kind(k)
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

/// The pinned Pyrefly fork commit and the Ruff line its retained AST comes from.
pub const PYREFLY_REVISION: &str = "72bb34d6d67c2bc14720c77e2ad7eff6b89d360f;ruff=0.0.14";

/// The exact native source patch participates in the declared producer build fingerprint.
pub const PYREFLY_PATCH_SHA256: &str =
    "8edb905a630440b82baec9df453b26ee93db86f9ad346f73d86ad6fe80c1a077";
