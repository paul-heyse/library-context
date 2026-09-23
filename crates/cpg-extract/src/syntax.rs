//! The `syntax` family's vocabulary (DESIGN §3.2, CPG slice C2): which Ruff nodes are placed in
//! `syntax_nodes`, each node's kind as the `syntax_kind` codebook (an exhaustive match on Ruff's
//! `NodeKind`, so a new Ruff variant fails the build), the fields a parent's children sit in,
//! and each kind's detail text.
//!
//! Placed: every statement; the clause nodes (`elif`/`else`, `except`, `case`, `with` items);
//! the expressions Pysa reports sites at (attribute, subscript, operators, comparisons, format
//! strings, `await`, `yield`, walrus, lambda); and, under every `test`, `exc`, `cause`, `guard` and
//! `msg`, the full expression subtree (the exhaustive-exporter contract, scoped to the fields the
//! passes read). A `def`, a `class` and a call are placed under their existing declaration and
//! call-site ids, so one syntax node never has two ids. Nothing inside an annotation is placed:
//! types are the `types` family's.

use cpg_schema::codebook::{SyntaxField, SyntaxKind};
use ruff_python_ast::{AnyNodeRef, NodeKind, Stmt};
use ruff_text_size::{Ranged, TextRange};

/// Ruff's node kind as the codebook value.
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn syntax_kind(k: NodeKind) -> SyntaxKind {
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

/// Whether the node is placed (every kind decided explicitly); `in_subtree` says it lies in a `test`/`exc`/`cause`/`guard`/`msg`
/// field of a placed ancestor.
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn placed(node: AnyNodeRef<'_>, in_subtree: bool) -> bool {
    match node.kind() {
        // Statements, and the clause nodes a branch's context is read from.
        NodeKind::StmtFunctionDef
        | NodeKind::StmtClassDef
        | NodeKind::StmtReturn
        | NodeKind::StmtDelete
        | NodeKind::StmtTypeAlias
        | NodeKind::StmtAssign
        | NodeKind::StmtAugAssign
        | NodeKind::StmtAnnAssign
        | NodeKind::StmtFor
        | NodeKind::StmtWhile
        | NodeKind::StmtIf
        | NodeKind::StmtWith
        | NodeKind::StmtMatch
        | NodeKind::StmtRaise
        | NodeKind::StmtTry
        | NodeKind::StmtAssert
        | NodeKind::StmtImport
        | NodeKind::StmtImportFrom
        | NodeKind::StmtGlobal
        | NodeKind::StmtNonlocal
        | NodeKind::StmtExpr
        | NodeKind::StmtPass
        | NodeKind::StmtBreak
        | NodeKind::StmtContinue
        | NodeKind::StmtIpyEscapeCommand
        | NodeKind::ElifElseClause
        | NodeKind::ExceptHandlerExceptHandler
        | NodeKind::MatchCase
        | NodeKind::WithItem => true,
        // Calls always (under their call-site id), and the expressions Pysa reports sites at.
        NodeKind::ExprCall
        | NodeKind::ExprAttribute
        | NodeKind::ExprSubscript
        | NodeKind::ExprBinOp
        | NodeKind::ExprUnaryOp
        | NodeKind::ExprBoolOp
        | NodeKind::ExprCompare
        | NodeKind::ExprFString
        | NodeKind::InterpolatedElement
        | NodeKind::ExprAwait
        | NodeKind::ExprYield
        | NodeKind::ExprYieldFrom
        | NodeKind::ExprNamed
        | NodeKind::ExprLambda => true,
        // Any other expression only inside a test, exception, cause, guard or message.
        NodeKind::ExprIf
        | NodeKind::ExprDict
        | NodeKind::ExprSet
        | NodeKind::ExprListComp
        | NodeKind::ExprSetComp
        | NodeKind::ExprDictComp
        | NodeKind::ExprGenerator
        | NodeKind::ExprTString
        | NodeKind::ExprStringLiteral
        | NodeKind::ExprBytesLiteral
        | NodeKind::ExprNumberLiteral
        | NodeKind::ExprBooleanLiteral
        | NodeKind::ExprNoneLiteral
        | NodeKind::ExprEllipsisLiteral
        | NodeKind::ExprStarred
        | NodeKind::ExprName
        | NodeKind::ExprList
        | NodeKind::ExprTuple
        | NodeKind::ExprSlice
        | NodeKind::ExprIpyEscapeCommand => in_subtree,
        // Containers, patterns, parameters, type parameters and string parts are not placed.
        NodeKind::ModModule
        | NodeKind::ModExpression
        | NodeKind::InterpolatedStringLiteralElement
        | NodeKind::PatternMatchValue
        | NodeKind::PatternMatchSingleton
        | NodeKind::PatternMatchSequence
        | NodeKind::PatternMatchMapping
        | NodeKind::PatternMatchClass
        | NodeKind::PatternMatchStar
        | NodeKind::PatternMatchAs
        | NodeKind::PatternMatchOr
        | NodeKind::TypeParamTypeVar
        | NodeKind::TypeParamTypeVarTuple
        | NodeKind::TypeParamParamSpec
        | NodeKind::InterpolatedStringFormatSpec
        | NodeKind::PatternArguments
        | NodeKind::PatternKeyword
        | NodeKind::Comprehension
        | NodeKind::Arguments
        | NodeKind::Parameters
        | NodeKind::Parameter
        | NodeKind::ParameterWithDefault
        | NodeKind::Keyword
        | NodeKind::Alias
        | NodeKind::Decorator
        | NodeKind::TypeParams
        | NodeKind::FString
        | NodeKind::TString
        | NodeKind::StringLiteral
        | NodeKind::BytesLiteral
        | NodeKind::Identifier => false,
    }
}

fn suite(body: &[Stmt]) -> Option<TextRange> {
    Some(TextRange::new(body.first()?.start(), body.last()?.end()))
}

/// The fields a placed node's children sit in, with their ranges. A child's field is the first
/// one whose range contains it; anything else is `child`.
pub(crate) fn fields(node: AnyNodeRef<'_>) -> Vec<(SyntaxField, TextRange)> {
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
            add(F::Left, Some(e.left.range()));
            for c in &e.comparators {
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
        AnyNodeRef::ExprLambda(e) => add(F::Value, Some(e.body.range())),
        AnyNodeRef::ExprList(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        AnyNodeRef::ExprTuple(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        AnyNodeRef::ExprSet(e) => e.elts.iter().for_each(|x| add(F::Element, Some(x.range()))),
        _ => {}
    }
    v
}

/// Fields whose whole expression subtree is placed.
pub(crate) fn subtree_field(f: SyntaxField) -> bool {
    matches!(
        f,
        SyntaxField::Test
            | SyntaxField::Exc
            | SyntaxField::Cause
            | SyntaxField::Guard
            | SyntaxField::Msg
    )
}

/// The kind's detail text: a name, an attribute, an operator, a literal as written, a handler's
/// bound name, the names of `global`/`nonlocal`.
pub(crate) fn detail(node: AnyNodeRef<'_>, text: &str) -> Option<String> {
    let raw = |r: TextRange| {
        text.get(r.start().to_usize()..r.end().to_usize())
            .map(str::to_owned)
    };
    match node {
        AnyNodeRef::ExprName(n) => Some(n.id.to_string()),
        AnyNodeRef::ExprAttribute(a) => Some(a.attr.to_string()),
        AnyNodeRef::ExprCompare(c) => Some(
            c.ops
                .iter()
                .map(|o| o.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        AnyNodeRef::ExprBinOp(b) => Some(b.op.as_str().to_owned()),
        AnyNodeRef::ExprBoolOp(b) => Some(b.op.as_str().to_owned()),
        AnyNodeRef::ExprUnaryOp(u) => Some(u.op.as_str().to_owned()),
        AnyNodeRef::StmtAugAssign(a) => Some(a.op.as_str().to_owned()),
        AnyNodeRef::ExprStringLiteral(e) => raw(e.range()),
        AnyNodeRef::ExprBytesLiteral(e) => raw(e.range()),
        AnyNodeRef::ExprNumberLiteral(e) => raw(e.range()),
        AnyNodeRef::ExprBooleanLiteral(e) => raw(e.range()),
        AnyNodeRef::ExprNoneLiteral(e) => raw(e.range()),
        AnyNodeRef::ExprEllipsisLiteral(e) => raw(e.range()),
        AnyNodeRef::ExceptHandlerExceptHandler(h) => h.name.as_ref().map(ToString::to_string),
        AnyNodeRef::StmtGlobal(g) => Some(
            g.names
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        AnyNodeRef::StmtNonlocal(g) => Some(
            g.names
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        _ => None,
    }
}
