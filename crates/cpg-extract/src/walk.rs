//! One Ruff walk over Pyrefly's own parse (DESIGN §4.2.2): the `ruff-ast` raw tables.

use std::collections::HashMap;

use cpg_schema::codebook::ExtractionMode;
use cpg_schema::codebook::{
    ArgumentKind, DeclarationKind, ExportSyntaxKind, Fidelity, Modality, Origin, ParameterKind,
};
use cpg_schema::id::{Id, IdHasher, kind};
use cpg_schema::tables::{
    Arguments, ArgumentsRow, CallSyntax, CallSyntaxRow, Declarations, DeclarationsRow,
    ExportSyntax, ExportSyntaxRow, ParameterSyntax, ParameterSyntaxRow,
};
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, TraversalSignal, walk_annotation,
};
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, Expr, ExprCall, ModModule, Parameter, Stmt, StmtClassDef,
    StmtFunctionDef,
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::facts::{FactSink, Provenance, Surface, fact_row};

pub(crate) struct ModuleCtx<'s> {
    pub release_id: Id,
    /// Release-relative path.
    pub path: &'s str,
    pub module_name: &'s str,
    pub module_node_id: Id,
    pub text: &'s str,
}

#[derive(Default)]
pub(crate) struct WalkOut {
    pub declarations: Vec<DeclarationsRow>,
    pub export_syntax: Vec<ExportSyntaxRow>,
    pub parameter_syntax: Vec<ParameterSyntaxRow>,
    pub call_syntax: Vec<CallSyntaxRow>,
    pub arguments: Vec<ArgumentsRow>,
    /// Ranges of module-level `__all__` statements that are not literal (F3 detector input).
    pub nonliteral_dunder_all: Vec<TextRange>,
}

fn ruff() -> Provenance {
    Provenance {
        surface: Surface::RuffAst,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::SourceObservation,
        modality: Modality::Definite,
        fidelity: Fidelity::NativeStructural,
    }
}

pub(crate) fn span(r: TextRange) -> (i64, i64) {
    (i64::from(r.start().to_u32()), i64::from(r.end().to_u32()))
}

struct DeclFrame {
    node_id: Id,
    name: String,
    body_start: TextSize,
}

struct ImportCtx {
    kind: ExportSyntaxKind,
    module: Option<String>,
    level: i64,
}

struct Walker<'s, 'f> {
    ctx: &'f ModuleCtx<'s>,
    sink: &'f mut FactSink,
    /// Structural occurrence path of the current node: `Kind#ordinal` from the module body down.
    path: Vec<String>,
    counters: Vec<u32>,
    decls: Vec<DeclFrame>,
    occurrences: HashMap<String, i64>,
    annotation_depth: u32,
    import: Option<ImportCtx>,
    out: WalkOut,
}

pub(crate) fn walk_module(ctx: &ModuleCtx<'_>, ast: &ModModule, sink: &mut FactSink) -> WalkOut {
    let mut w = Walker {
        ctx,
        sink,
        path: Vec::new(),
        counters: vec![0],
        decls: Vec::new(),
        occurrences: HashMap::new(),
        annotation_depth: 0,
        import: None,
        out: WalkOut::default(),
    };
    for stmt in &ast.body {
        w.visit_stmt(stmt);
    }
    w.out
}

fn trailing_name(expr: &Expr) -> String {
    match expr {
        Expr::Name(n) => n.id.to_string(),
        Expr::Attribute(a) => a.attr.to_string(),
        Expr::Call(c) => trailing_name(&c.func),
        _ => "<expr>".to_owned(),
    }
}

fn docstring(body: &[Stmt]) -> Option<(String, TextRange)> {
    match body.first() {
        Some(Stmt::Expr(e)) => e
            .value
            .as_string_literal_expr()
            .map(|s| (s.value.to_str().to_owned(), e.range())),
        _ => None,
    }
}

fn is_str_seq(expr: &Expr) -> bool {
    let elts = match expr {
        Expr::List(l) => &l.elts,
        Expr::Tuple(t) => &t.elts,
        _ => return false,
    };
    elts.iter().all(|e| matches!(e, Expr::StringLiteral(_)))
}

fn is_dunder_all(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(n) if n.id.as_str() == "__all__")
}

/// `__all__.extend([...])`, `__all__.append("x")`, `__all__.remove("x")`: whether the argument is
/// literal; `None` when the expression is not an `__all__` mutation.
fn dunder_all_call_literal(expr: &Expr) -> Option<bool> {
    let Expr::Call(c) = expr else {
        return None;
    };
    let Expr::Attribute(a) = c.func.as_ref() else {
        return None;
    };
    if !is_dunder_all(&a.value) {
        return None;
    }
    let arg = c.arguments.args.first();
    Some(match a.attr.as_str() {
        "extend" => arg.is_some_and(is_str_seq),
        "append" | "remove" => arg.is_some_and(|x| matches!(x, Expr::StringLiteral(_))),
        _ => false,
    })
}

type ParamSpec<'p> = (ParameterKind, &'p Parameter, Option<&'p Expr>, TextRange);

impl Walker<'_, '_> {
    fn syntax_id(&self) -> Id {
        IdHasher::new(kind::SYNTAX)
            .id(self.ctx.release_id)
            .str(self.ctx.path)
            .strs(self.path.iter().map(String::as_str))
            .finish_id()
    }

    fn text(&self, r: TextRange) -> String {
        self.ctx.text[r].to_owned()
    }

    fn declaration(
        &mut self,
        name: &str,
        kind_code: DeclarationKind,
        range: TextRange,
        name_range: TextRange,
        body: &[Stmt],
        decorators: Vec<String>,
    ) -> Id {
        let mut qual = vec![self.ctx.module_name.to_owned()];
        qual.extend(self.decls.iter().map(|d| d.name.clone()));
        qual.push(name.to_owned());
        let qualified_name = qual.join(".");
        let occurrence = {
            let n = self.occurrences.entry(qualified_name.clone()).or_insert(0);
            *n += 1;
            *n - 1
        };
        let node_id = IdHasher::new(kind::DECLARATION)
            .id(self.ctx.release_id)
            .str(self.ctx.path)
            .str(&qualified_name)
            .i64(occurrence)
            .finish_id();
        let (start, end) = span(range);
        let (name_start, name_end) = span(name_range);
        let doc = docstring(body);
        let is_overload = decorators.iter().any(|d| d == "overload");
        let row = fact_row!(
            self.sink,
            Declarations,
            ruff(),
            DeclarationsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id,
                module_node_id: self.ctx.module_node_id,
                parent_node_id: self.decls.last().map(|d| d.node_id),
                qualified_name,
                name: name.to_owned(),
                kind: kind_code,
                start_byte: start,
                end_byte: end,
                name_start_byte: name_start,
                name_end_byte: name_end,
                docstring: doc.as_ref().map(|d| d.0.clone()),
                docstring_start_byte: doc.as_ref().map(|d| span(d.1).0),
                docstring_end_byte: doc.as_ref().map(|d| span(d.1).1),
                is_overload,
                decorators,
            }
        );
        self.out.declarations.push(row);
        self.decls.push(DeclFrame {
            node_id,
            name: name.to_owned(),
            body_start: body.first().map_or(range.end(), Ranged::start),
        });
        node_id
    }

    fn function(&mut self, f: &StmtFunctionDef) {
        let decorators = f
            .decorator_list
            .iter()
            .map(|d| trailing_name(&d.expression))
            .collect();
        let kind_code = if f.is_async {
            DeclarationKind::AsyncFunction
        } else {
            DeclarationKind::Function
        };
        let function_node_id = self.declaration(
            &f.name,
            kind_code,
            f.range(),
            f.name.range(),
            &f.body,
            decorators,
        );
        let p = &f.parameters;
        let mut params: Vec<ParamSpec<'_>> = Vec::new();
        for pw in &p.posonlyargs {
            params.push((
                ParameterKind::PositionalOnly,
                &pw.parameter,
                pw.default.as_deref(),
                pw.range(),
            ));
        }
        for pw in &p.args {
            params.push((
                ParameterKind::PositionalOrKeyword,
                &pw.parameter,
                pw.default.as_deref(),
                pw.range(),
            ));
        }
        if let Some(v) = &p.vararg {
            params.push((ParameterKind::VarPositional, v, None, v.range()));
        }
        for pw in &p.kwonlyargs {
            params.push((
                ParameterKind::KeywordOnly,
                &pw.parameter,
                pw.default.as_deref(),
                pw.range(),
            ));
        }
        if let Some(k) = &p.kwarg {
            params.push((ParameterKind::VarKeyword, k, None, k.range()));
        }
        for (ordinal, (pkind, param, default, range)) in params.into_iter().enumerate() {
            let ordinal = ordinal as i64;
            let node_id = IdHasher::new(kind::SYNTAX)
                .id(self.ctx.release_id)
                .str(self.ctx.path)
                .str("Parameter")
                .id(function_node_id)
                .i64(ordinal)
                .finish_id();
            let (start, end) = span(range);
            let row = fact_row!(
                self.sink,
                ParameterSyntax,
                ruff(),
                ParameterSyntaxRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    node_id,
                    function_node_id,
                    ordinal,
                    name: param.name.to_string(),
                    kind: pkind,
                    default_text: default.map(|d| self.text(d.range())),
                    default_start_byte: default.map(|d| span(d.range()).0),
                    default_end_byte: default.map(|d| span(d.range()).1),
                    annotation_text: param.annotation.as_deref().map(|a| self.text(a.range())),
                    start_byte: start,
                    end_byte: end,
                }
            );
            self.out.parameter_syntax.push(row);
        }
    }

    fn class(&mut self, c: &StmtClassDef) {
        let decorators = c
            .decorator_list
            .iter()
            .map(|d| trailing_name(&d.expression))
            .collect();
        self.declaration(
            &c.name,
            DeclarationKind::Class,
            c.range(),
            c.name.range(),
            &c.body,
            decorators,
        );
    }

    fn call(&mut self, c: &ExprCall) {
        let node_id = self.syntax_id();
        let r = c.range();
        // The owner is the innermost declaration whose *body* holds the call: decorators, defaults
        // and annotations run in the enclosing scope.
        let owner = self
            .decls
            .iter()
            .rev()
            .find(|d| d.body_start <= r.start())
            .map(|d| d.node_id);
        let (start, end) = span(r);
        let (callee_start, callee_end) = span(c.func.range());
        let row = fact_row!(
            self.sink,
            CallSyntax,
            ruff(),
            CallSyntaxRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id,
                module_node_id: self.ctx.module_node_id,
                owner_node_id: owner,
                start_byte: start,
                end_byte: end,
                callee_start_byte: callee_start,
                callee_end_byte: callee_end,
                in_annotation: self.annotation_depth > 0,
                positional_count: c.arguments.args.len() as i64,
                keyword_count: c.arguments.keywords.len() as i64,
            }
        );
        self.out.call_syntax.push(row);
        for (ordinal, arg) in c.arguments.iter_source_order().enumerate() {
            let (akind, keyword, range) = match arg {
                ArgOrKeyword::Arg(Expr::Starred(s)) => (ArgumentKind::Starred, None, s.range()),
                ArgOrKeyword::Arg(e) => (ArgumentKind::Positional, None, e.range()),
                ArgOrKeyword::Keyword(k) => match &k.arg {
                    Some(name) => (ArgumentKind::Keyword, Some(name.to_string()), k.range()),
                    None => (ArgumentKind::DoubleStarred, None, k.range()),
                },
            };
            let (start, end) = span(range);
            let row = fact_row!(
                self.sink,
                Arguments,
                ruff(),
                ArgumentsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    // A role in the call (§3.4.1): never the expression's own id, which for
                    // `f(g(x))` would be the inner call site's.
                    node_id: cpg_schema::id::recipe::argument(node_id, ordinal as i64),
                    call_node_id: node_id,
                    ordinal: ordinal as i64,
                    kind: akind,
                    keyword,
                    start_byte: start,
                    end_byte: end,
                }
            );
            self.out.arguments.push(row);
        }
    }

    fn export_row(
        &mut self,
        range: TextRange,
        kind_code: ExportSyntaxKind,
        imported: (Option<String>, Option<String>, Option<String>, i64),
        literal: Option<bool>,
    ) {
        let node_id = self.syntax_id();
        let (start, end) = span(range);
        let (imported_module, imported_name, alias, level) = imported;
        let row = fact_row!(
            self.sink,
            ExportSyntax,
            ruff(),
            ExportSyntaxRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id,
                module_node_id: self.ctx.module_node_id,
                kind: kind_code,
                imported_module,
                imported_name,
                alias,
                level,
                start_byte: start,
                end_byte: end,
                dunder_all_literal: literal,
            }
        );
        self.out.export_syntax.push(row);
        if literal == Some(false) {
            self.out.nonliteral_dunder_all.push(range);
        }
    }

    /// A module-level `__all__` statement (assignment, `+=`, annotated, `.extend`/`.append`).
    fn dunder_all(&mut self, range: TextRange, literal: bool) {
        if self.decls.is_empty() {
            self.export_row(
                range,
                ExportSyntaxKind::DunderAll,
                (None, None, None, 0),
                Some(literal),
            );
        }
    }
}

impl<'a> SourceOrderVisitor<'a> for Walker<'_, '_> {
    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let ordinal = self.counters.last_mut().map_or(0, |c| {
            *c += 1;
            *c - 1
        });
        self.path.push(format!("{:?}#{ordinal}", node.kind()));
        self.counters.push(0);
        match node {
            AnyNodeRef::StmtFunctionDef(f) => self.function(f),
            AnyNodeRef::StmtClassDef(c) => self.class(c),
            AnyNodeRef::ExprCall(c) => self.call(c),
            AnyNodeRef::StmtImport(_) => {
                self.import = Some(ImportCtx {
                    kind: ExportSyntaxKind::Import,
                    module: None,
                    level: 0,
                });
            }
            AnyNodeRef::StmtImportFrom(i) => {
                self.import = Some(ImportCtx {
                    kind: ExportSyntaxKind::ImportFrom,
                    module: i.module.as_ref().map(ToString::to_string),
                    level: i64::from(i.level),
                });
            }
            AnyNodeRef::Alias(a) => {
                if let Some(ctx) = &self.import {
                    let kind_code = ctx.kind;
                    let asname = a.asname.as_ref().map(ToString::to_string);
                    let imported = match kind_code {
                        ExportSyntaxKind::ImportFrom => (
                            ctx.module.clone(),
                            Some(a.name.to_string()),
                            asname,
                            ctx.level,
                        ),
                        _ => (Some(a.name.to_string()), None, asname, ctx.level),
                    };
                    self.export_row(a.range(), kind_code, imported, None);
                }
            }
            AnyNodeRef::StmtAssign(x) if x.targets.iter().any(is_dunder_all) => {
                self.dunder_all(x.range(), is_str_seq(&x.value));
            }
            AnyNodeRef::StmtAugAssign(x) if is_dunder_all(&x.target) => {
                self.dunder_all(x.range(), is_str_seq(&x.value));
            }
            AnyNodeRef::StmtAnnAssign(x) if is_dunder_all(&x.target) => {
                self.dunder_all(x.range(), x.value.as_deref().is_some_and(is_str_seq));
            }
            AnyNodeRef::StmtExpr(x) => {
                if let Some(literal) = dunder_all_call_literal(&x.value) {
                    self.dunder_all(x.range(), literal);
                }
            }
            _ => {}
        }
        TraversalSignal::Traverse
    }

    fn leave_node(&mut self, node: AnyNodeRef<'a>) {
        match node {
            AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_) => {
                self.decls.pop();
            }
            AnyNodeRef::StmtImport(_) | AnyNodeRef::StmtImportFrom(_) => self.import = None,
            _ => {}
        }
        self.counters.pop();
        self.path.pop();
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.annotation_depth += 1;
        walk_annotation(self, expr);
        self.annotation_depth -= 1;
    }
}
