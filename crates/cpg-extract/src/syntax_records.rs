//! The syntax records a module's parse states beyond occurrences (cutover plan A4): declarations
//! and their decorators, import aliases, `__all__`, parameters, class fields and call syntax. Each
//! names occurrences the traversal emitted, found by span and kind: a record never invents one, and
//! a span and kind emitted twice is refused rather than chosen.
use std::collections::HashMap;
use lctx_model::domain::{Id, ModelError, Record, assertion::AssertionQualification, calls::{Actual, ArgumentKind, CallArgument, CallSyntax, ParameterKind},
    source::{Occurrence, SyntaxKind}, syntax::*, value::{Literal, LiteralSet, LiteralSetMember}};
use pyrefly_python::{docstring::Docstring, module_name::ModuleName};
use ruff_python_ast::{AnyNodeRef, ArgOrKeyword, Expr, ModModule, Parameters, Stmt, name::Name};
use ruff_python_ast::visitor::source_order::{self, SourceOrderVisitor};
use ruff_text_size::{Ranged, TextRange};

fn invalid(message: String) -> ModelError { ModelError::Invalid(message) }

/// The occurrences one module's traversal emitted, by span and kind, with each one's parent and
/// the parent's field that holds it.
#[derive(Default)]
pub struct Spans { map: HashMap<(i64, i64, i16), Option<Id<Occurrence>>>, parents: HashMap<Id<Occurrence>, (Id<Occurrence>, lctx_model::domain::lexical::SyntaxField)> }
impl Spans {
    pub fn insert(&mut self, occurrence: &Occurrence) {
        self.map.entry((occurrence.start, occurrence.end, occurrence.syntax_kind as i16))
            .and_modify(|id| *id = None).or_insert(Some(occurrence.id()));
    }
    pub fn place(&mut self, occurrence: Id<Occurrence>, parent: Id<Occurrence>, field: lctx_model::domain::lexical::SyntaxField) {
        self.parents.insert(occurrence, (parent, field));
    }
    pub fn parent(&self, occurrence: Id<Occurrence>) -> Option<(Id<Occurrence>, lctx_model::domain::lexical::SyntaxField)> { self.parents.get(&occurrence).copied() }
    pub fn len(&self) -> usize { self.map.len() }
    pub fn is_empty(&self) -> bool { self.map.is_empty() }
    pub fn get(&self, range: TextRange, kind: SyntaxKind) -> Result<Id<Occurrence>, ModelError> {
        match self.map.get(&(i64::from(u32::from(range.start())), i64::from(u32::from(range.end())), kind as i16)) {
            Some(Some(id)) => Ok(*id),
            Some(None) => Err(invalid(format!("two {kind:?} occurrences share the span {range:?}"))),
            None => Err(invalid(format!("no {kind:?} occurrence was emitted at {range:?}"))),
        }
    }
}
fn kind_of(expr: &Expr) -> SyntaxKind { crate::typed_syntax::kind(AnyNodeRef::from(expr).kind()) }

/// The records of one module, qualified by its artifact's qualification.
#[derive(Debug, Default)]
pub struct Records {
    pub declarations: Vec<DeclarationObservation>, pub decorators: Vec<DeclarationDecorator>, pub imports: Vec<ImportAliasObservation>,
    pub dunder_all: Vec<DunderAllObservation>,
    /// `__all__` statements whose names the syntax does not state.
    pub computed_all: Vec<Id<Occurrence>>,
    pub parameters: Vec<ParameterSyntaxObservation>, pub fields: Vec<ClassFieldSyntaxObservation>,
    pub calls: Vec<(CallSyntax, Vec<CallArgument>)>,
    pub literals: Vec<Literal>, pub sets: Vec<(LiteralSet, Vec<LiteralSetMember>)>,
}

/// The imported module as an absolute name, relative levels resolved against `module`; `None` when
/// they climb past its top package.
pub fn absolute_module(module: &str, is_package: bool, level: i64, imported: Option<&str>) -> Option<String> {
    let suffix = imported.map(Name::new);
    let name = ModuleName::from_str(module).new_maybe_relative(is_package, u32::try_from(level).ok()?, suffix.as_ref())?;
    (!name.as_str().is_empty()).then(|| name.to_string())
}

/// Walk one module's statements for its records.
pub fn records(ast: &ModModule, module: &str, is_package: bool, spans: &Spans, qualification: Id<AssertionQualification>) -> Result<Records, ModelError> {
    let mut walker = Walker { spans, qualification, module, is_package, enclosing: vec![], annotation: 0, records: Records::default(), error: None };
    walker.visit_body(&ast.body);
    match walker.error { Some(error) => Err(error), None => Ok(walker.records) }
}

struct Walker<'a> {
    spans: &'a Spans, qualification: Id<AssertionQualification>, module: &'a str, is_package: bool,
    enclosing: Vec<Id<Occurrence>>, annotation: usize, records: Records, error: Option<ModelError>,
}
fn trailing_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Name(name) => Some(name.id.as_str()),
        Expr::Attribute(attribute) => Some(attribute.attr.as_str()),
        Expr::Call(call) => trailing_name(&call.func),
        _ => None,
    }
}
fn is_dunder_all(expr: &Expr) -> bool { matches!(expr, Expr::Name(n) if n.id.as_str() == "__all__") }
/// A literal list or tuple of strings: its strings.
fn string_sequence(expr: &Expr) -> Option<Vec<String>> {
    let elements = match expr { Expr::List(list) => &list.elts, Expr::Tuple(tuple) => &tuple.elts, _ => return None };
    elements.iter().map(|e| e.as_string_literal_expr().map(|s| s.value.to_str().to_owned())).collect()
}
impl Walker<'_> {
    fn occ(&self, range: TextRange, kind: SyntaxKind) -> Result<Id<Occurrence>, ModelError> { self.spans.get(range, kind) }
    fn expr(&self, expr: &Expr) -> Result<Id<Occurrence>, ModelError> { self.occ(expr.range(), kind_of(expr)) }
    fn run(&mut self, step: impl FnOnce(&mut Self) -> Result<(), ModelError>) {
        if self.error.is_none() && let Err(error) = step(self) { self.error = Some(error); }
    }
    fn docstring(&self, body: &[Stmt]) -> Result<Option<Id<Occurrence>>, ModelError> {
        match (Docstring::range_from_stmts(body), body.first()) {
            (Some(_), Some(statement @ Stmt::Expr(_))) => Ok(Some(self.occ(statement.range(), SyntaxKind::StmtExpr)?)),
            _ => Ok(None),
        }
    }
    fn declaration(&mut self, declaration: Id<Occurrence>, name: TextRange, kind: DeclarationKind, decorators: &[ruff_python_ast::Decorator], body: &[Stmt])
        -> Result<(), ModelError> {
        let overload = decorators.iter().any(|d| trailing_name(&d.expression) == Some("overload"));
        self.records.declarations.push(DeclarationObservation { qualification: self.qualification, declaration, name: self.occ(name, SyntaxKind::Identifier)?,
            kind, parent: self.enclosing.last().copied(), overload, docstring: self.docstring(body)? });
        for (ordinal, decorator) in decorators.iter().enumerate() {
            self.records.decorators.push(DeclarationDecorator { qualification: self.qualification, declaration,
                decorator: self.occ(decorator.range(), SyntaxKind::Decorator)?, ordinal: ordinal as i64 });
        }
        Ok(())
    }
    fn parameters(&mut self, function: Id<Occurrence>, parameters: &Parameters) -> Result<(), ModelError> {
        let mut ordinal = 0i64;
        let mut push = |walker: &mut Self, parameter: Id<Occurrence>, kind: ParameterKind, default: Option<&Expr>, annotation: Option<&Expr>| -> Result<(), ModelError> {
            let default_literal = default.and_then(crate::typed_syntax::literal);
            let row = ParameterSyntaxObservation { qualification: walker.qualification, function, parameter, ordinal, kind,
                default: default.map(|d| walker.expr(d)).transpose()?, default_literal: default_literal.as_ref().map(Record::id),
                annotation: annotation.map(|a| walker.expr(a)).transpose()? };
            walker.records.literals.extend(default_literal);
            walker.records.parameters.push(row);
            ordinal += 1;
            Ok(())
        };
        for (group, kind) in [(&parameters.posonlyargs, ParameterKind::PositionalOnly), (&parameters.args, ParameterKind::PositionalOrKeyword)] {
            for p in group {
                let id = self.occ(p.range(), SyntaxKind::ParameterWithDefault)?;
                push(self, id, kind, p.default.as_deref(), p.parameter.annotation.as_deref())?;
            }
        }
        if let Some(p) = &parameters.vararg { let id = self.occ(p.range(), SyntaxKind::Parameter)?; push(self, id, ParameterKind::VarPositional, None, p.annotation.as_deref())?; }
        for p in &parameters.kwonlyargs {
            let id = self.occ(p.range(), SyntaxKind::ParameterWithDefault)?;
            push(self, id, ParameterKind::KeywordOnly, p.default.as_deref(), p.parameter.annotation.as_deref())?;
        }
        if let Some(p) = &parameters.kwarg { let id = self.occ(p.range(), SyntaxKind::Parameter)?; push(self, id, ParameterKind::VarKeyword, None, p.annotation.as_deref())?; }
        Ok(())
    }
    fn dunder_all(&mut self, statement: TextRange, kind: SyntaxKind, names: Option<Vec<String>>) -> Result<(), ModelError> {
        let statement = self.occ(statement, kind)?;
        match names {
            Some(names) => {
                let literals: Vec<Literal> = names.into_iter().map(|value| Literal::String { value }).collect();
                let (set, members) = LiteralSet::of(literals.iter().map(Record::id));
                self.records.dunder_all.push(DunderAllObservation { qualification: self.qualification, statement, literal: true, names: Some(set.id()) });
                self.records.literals.extend(literals);
                self.records.sets.push((set, members));
            }
            None => {
                self.records.dunder_all.push(DunderAllObservation { qualification: self.qualification, statement, literal: false, names: None });
                self.records.computed_all.push(statement);
            }
        }
        Ok(())
    }
    fn statement(&mut self, stmt: &Stmt) -> Result<(), ModelError> {
        let module_level = self.enclosing.is_empty();
        match stmt {
            Stmt::Import(import) => for alias in &import.names {
                self.records.imports.push(ImportAliasObservation { qualification: self.qualification, statement: self.occ(stmt.range(), SyntaxKind::StmtImport)?,
                    alias: self.occ(alias.range(), SyntaxKind::Alias)?, level: 0, resolved_module: Some(alias.name.to_string()) });
            },
            Stmt::ImportFrom(from) => for alias in &from.names {
                let level = i64::from(from.level);
                self.records.imports.push(ImportAliasObservation { qualification: self.qualification, statement: self.occ(stmt.range(), SyntaxKind::StmtImportFrom)?,
                    alias: self.occ(alias.range(), SyntaxKind::Alias)?, level,
                    resolved_module: absolute_module(self.module, self.is_package, level, from.module.as_deref()) });
            },
            Stmt::Assign(assign) if module_level && assign.targets.iter().any(is_dunder_all) =>
                self.dunder_all(stmt.range(), SyntaxKind::StmtAssign, string_sequence(&assign.value))?,
            Stmt::AugAssign(assign) if module_level && is_dunder_all(&assign.target) =>
                self.dunder_all(stmt.range(), SyntaxKind::StmtAugAssign, string_sequence(&assign.value))?,
            Stmt::AnnAssign(assign) if module_level && is_dunder_all(&assign.target) && assign.value.is_some() =>
                self.dunder_all(stmt.range(), SyntaxKind::StmtAnnAssign, assign.value.as_deref().and_then(string_sequence))?,
            Stmt::Expr(expr) if module_level => {
                // `__all__.extend([...])` and `__all__.append("x")` add literal names; any other
                // mutation (`remove`, a computed argument) is not stated by the syntax.
                if let Expr::Call(call) = expr.value.as_ref() && let Expr::Attribute(attribute) = call.func.as_ref() && is_dunder_all(&attribute.value) {
                    let argument = call.arguments.args.first().filter(|_| call.arguments.len() == 1);
                    let names = match (attribute.attr.as_str(), argument) {
                        ("extend", Some(argument)) => string_sequence(argument),
                        ("append", Some(Expr::StringLiteral(s))) => Some(vec![s.value.to_str().to_owned()]),
                        _ => None,
                    };
                    self.dunder_all(stmt.range(), SyntaxKind::StmtExpr, names)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl<'t> SourceOrderVisitor<'t> for Walker<'_> {
    fn visit_stmt(&mut self, stmt: &'t Stmt) {
        if self.error.is_some() { return; }
        match stmt {
            Stmt::FunctionDef(def) => {
                let declaration = match self.occ(stmt.range(), SyntaxKind::StmtFunctionDef) { Ok(id) => id, Err(e) => { self.error = Some(e); return; } };
                let kind = if def.is_async { DeclarationKind::AsyncFunction } else { DeclarationKind::Function };
                self.run(|w| w.declaration(declaration, def.name.range(), kind, &def.decorator_list, &def.body));
                self.run(|w| w.parameters(declaration, &def.parameters));
                self.enclosing.push(declaration);
                source_order::walk_stmt(self, stmt);
                self.enclosing.pop();
            }
            Stmt::ClassDef(class) => {
                let declaration = match self.occ(stmt.range(), SyntaxKind::StmtClassDef) { Ok(id) => id, Err(e) => { self.error = Some(e); return; } };
                self.run(|w| w.declaration(declaration, class.name.range(), DeclarationKind::Class, &class.decorator_list, &class.body));
                self.run(|w| {
                    for member in &class.body {
                        let (target, annotation, value) = match member {
                            Stmt::AnnAssign(a) if matches!(*a.target, Expr::Name(_)) => (&*a.target, Some(&*a.annotation), a.value.as_deref()),
                            Stmt::Assign(a) if a.targets.len() == 1 && matches!(a.targets[0], Expr::Name(_)) => (&a.targets[0], None, Some(&*a.value)),
                            _ => continue,
                        };
                        w.records.fields.push(ClassFieldSyntaxObservation { qualification: w.qualification, class: declaration, target: w.expr(target)?,
                            annotation: annotation.map(|a| w.expr(a)).transpose()?, value: value.map(|v| w.expr(v)).transpose()? });
                    }
                    Ok(())
                });
                self.enclosing.push(declaration);
                source_order::walk_stmt(self, stmt);
                self.enclosing.pop();
            }
            _ => {
                self.run(|w| w.statement(stmt));
                source_order::walk_stmt(self, stmt);
            }
        }
    }
    fn visit_annotation(&mut self, expr: &'t Expr) {
        self.annotation += 1;
        source_order::walk_annotation(self, expr);
        self.annotation -= 1;
    }
    fn visit_expr(&mut self, expr: &'t Expr) {
        if self.error.is_some() { return; }
        match expr {
            Expr::Call(call) => self.run(|w| {
                let mut actuals = Vec::new();
                for argument in call.arguments.iter_source_order() {
                    actuals.push(match argument {
                        ArgOrKeyword::Arg(value @ Expr::Starred(_)) => Actual { occurrence: w.expr(value)?, kind: ArgumentKind::Starred, keyword: None },
                        ArgOrKeyword::Arg(value) => Actual { occurrence: w.expr(value)?, kind: ArgumentKind::Positional, keyword: None },
                        ArgOrKeyword::Keyword(keyword) => match &keyword.arg {
                            Some(name) => Actual { occurrence: w.expr(&keyword.value)?, kind: ArgumentKind::Keyword, keyword: Some(name.to_string()) },
                            None => Actual { occurrence: w.expr(&keyword.value)?, kind: ArgumentKind::DoubleStarred, keyword: None },
                        },
                    });
                }
                let site = w.occ(expr.range(), SyntaxKind::ExprCall)?;
                w.records.calls.push(CallSyntax::new(w.qualification, site, w.expr(&call.func)?, w.annotation > 0, &actuals)?);
                Ok(())
            }),
            Expr::Lambda(lambda) => self.run(|w| match &lambda.parameters {
                Some(parameters) => { let id = w.occ(expr.range(), SyntaxKind::ExprLambda)?; w.parameters(id, parameters) },
                None => Ok(()),
            }),
            _ => {}
        }
        source_order::walk_expr(self, expr);
    }
}
