//! The syntax records a module's parse states beyond occurrences (cutover plan A4): declarations
//! and their decorators, import aliases, `__all__`, parameters, class fields and call syntax. Each
//! names occurrences the traversal emitted, found by span and kind: a record never invents one, and
//! a span and kind emitted twice is refused rather than chosen.
use lctx_model::domain::{
    Id, ModelError, Record,
    assertion::AssertionQualification,
    calls::{Actual, ArgumentKind, CallArgument, CallSyntax, ParameterKind},
    charged::StateCharge,
    resources::ResourceBudget,
    source::{Occurrence, OccurrenceRole, SyntaxKind},
    syntax::*,
    value::{Literal, LiteralSet, LiteralSetMember},
};
use pyrefly_python::module_name::ModuleName;
use ruff_python_ast_latest::visitor::source_order::{self, SourceOrderVisitor};
use ruff_python_ast_latest::{AnyNodeRef, ArgOrKeyword, Expr, ModModule, Parameters, Stmt};
use ruff_text_size_latest::{Ranged, TextRange, TextSize};
use std::collections::HashMap;

fn invalid(message: String) -> ModelError {
    ModelError::Invalid(message)
}

/// Owned offsets at the nominal provider boundary. This carries no foreign AST node.
#[derive(Debug, Clone, Copy)]
pub struct ByteRange {
    start: u32,
    end: u32,
}
impl From<ruff_text_size::TextRange> for ByteRange {
    fn from(range: ruff_text_size::TextRange) -> Self {
        Self {
            start: range.start().to_u32(),
            end: range.end().to_u32(),
        }
    }
}
impl From<TextRange> for ByteRange {
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start().to_u32(),
            end: range.end().to_u32(),
        }
    }
}

/// The occurrences one module's traversal emitted, by span and kind, with each one's parent and
/// the parent's field that holds it.
pub struct Spans {
    charge: StateCharge,
    source: Option<Id<lctx_model::domain::source::SourceArtifact>>,
    roles: HashMap<Id<Occurrence>, OccurrenceRole>,
    ranges: HashMap<Id<Occurrence>, (i64, i64, SyntaxKind)>,
    map: HashMap<(i64, i64, i16), Option<Id<Occurrence>>>,
    parents: HashMap<Id<Occurrence>, (Id<Occurrence>, lctx_model::domain::lexical::SyntaxField)>,
    events: SpanEvents,
    children:
        HashMap<(Id<Occurrence>, lctx_model::domain::lexical::SyntaxField), Vec<Id<Occurrence>>>,
}
impl Spans {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "syntax_spans"),
            source: None,
            roles: HashMap::new(),
            ranges: HashMap::new(),
            map: HashMap::new(),
            parents: HashMap::new(),
            events: HashMap::new(),
            children: HashMap::new(),
        }
    }
    pub fn insert(&mut self, occurrence: &Occurrence) -> Result<(), ModelError> {
        if self
            .source
            .is_some_and(|source| source != occurrence.source)
        {
            return Err(invalid("canonical span index mixes sources".into()));
        }
        self.source = Some(occurrence.source);
        // Covers hash-table spare capacity, keys, values and each range's candidate buffer.
        self.charge.grow(768)?;
        self.roles.insert(occurrence.id(), occurrence.role);
        self.ranges.insert(
            occurrence.id(),
            (occurrence.start, occurrence.end, occurrence.syntax_kind),
        );
        self.map
            .entry((
                occurrence.start,
                occurrence.end,
                occurrence.syntax_kind as i16,
            ))
            .and_modify(|id| *id = None)
            .or_insert(Some(occurrence.id()));
        self.events
            .entry((occurrence.start, occurrence.end))
            .or_default()
            .push((occurrence.syntax_kind, occurrence.role, occurrence.id()));
        Ok(())
    }
    pub fn source(&self) -> Option<Id<lctx_model::domain::source::SourceArtifact>> {
        self.source
    }
    pub fn role_of(&self, subject: Id<Occurrence>) -> Option<OccurrenceRole> {
        self.roles.get(&subject).copied()
    }
    /// A finalized semantic reference may name a load, an augmented target, a deletion or a
    /// nonlocal declaration. Admit only the corresponding exact canonical structure.
    pub fn reference(&self, range: impl Into<ByteRange>, is_load: bool) -> Option<Id<Occurrence>> {
        use lctx_model::domain::lexical::SyntaxField;
        let range = range.into();
        let mut candidates = self.event_candidates(range, Some(SyntaxKind::ExprName));
        candidates.extend(self.event_candidates(range, Some(SyntaxKind::Identifier)));
        candidates.retain(|id| {
            let Some((_, _, kind)) = self.ranges.get(id) else {
                return false;
            };
            let parent = self.parent(*id).and_then(|(parent, field)| {
                self.ranges.get(&parent).map(|(_, _, kind)| (*kind, field))
            });
            matches!(
                (*kind, self.role_of(*id), is_load, parent),
                (SyntaxKind::ExprName, Some(OccurrenceRole::Read), true, _)
                    | (
                        SyntaxKind::ExprName,
                        Some(OccurrenceRole::Binding),
                        true,
                        Some((SyntaxKind::StmtAugAssign, SyntaxField::Target))
                    )
                    | (
                        SyntaxKind::ExprName,
                        Some(OccurrenceRole::Syntax),
                        false,
                        Some((SyntaxKind::StmtDelete, _))
                    )
                    | (
                        SyntaxKind::Identifier,
                        Some(OccurrenceRole::Syntax),
                        true,
                        Some((SyntaxKind::StmtNonlocal | SyntaxKind::StmtGlobal, _))
                    )
            )
        });
        match candidates.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }
    pub fn place(
        &mut self,
        occurrence: Id<Occurrence>,
        parent: Id<Occurrence>,
        field: lctx_model::domain::lexical::SyntaxField,
    ) -> Result<(), ModelError> {
        self.charge.grow(512)?;
        self.parents.insert(occurrence, (parent, field));
        self.children
            .entry((parent, field))
            .or_default()
            .push(occurrence);
        Ok(())
    }
    pub fn child(
        &self,
        parent: Id<Occurrence>,
        field: lctx_model::domain::lexical::SyntaxField,
    ) -> Option<Id<Occurrence>> {
        match self.children.get(&(parent, field)).map(Vec::as_slice) {
            Some([child]) => Some(*child),
            _ => None,
        }
    }
    /// Exact range plus native kind; an implicit event lacking a native syntax kind accepts only
    /// one semantic expression/statement candidate. Same-span alternatives remain unattached.
    pub fn event(
        &self,
        range: impl Into<ByteRange>,
        kind: Option<SyntaxKind>,
    ) -> Option<Id<Occurrence>> {
        match self.event_candidates(range, kind).as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }
    /// Keep every exact candidate for diagnostics; candidate order does not depend on traversal.
    pub fn event_candidates(
        &self,
        range: impl Into<ByteRange>,
        kind: Option<SyntaxKind>,
    ) -> Vec<Id<Occurrence>> {
        let range = range.into();
        let at = (i64::from(range.start), i64::from(range.end));
        let mut found: Vec<_> = self
            .events
            .get(&at)
            .into_iter()
            .flatten()
            .filter(|(k, _, _)| match kind {
                Some(expected) => *k == expected,
                None => {
                    (27..=59).contains(&(*k as i16))
                        || matches!(
                            k,
                            SyntaxKind::StmtAssign
                                | SyntaxKind::StmtAugAssign
                                | SyntaxKind::StmtFunctionDef
                                | SyntaxKind::StmtClassDef
                                | SyntaxKind::WithItem
                        )
                }
            })
            .map(|(_, _, id)| *id)
            .collect();
        found.sort();
        found
    }
    pub fn range_of(&self, id: Id<Occurrence>) -> Option<ruff_text_size::TextRange> {
        self.ranges.get(&id).map(|(s, e, _)| {
            ruff_text_size::TextRange::new(
                ruff_text_size::TextSize::new(*s as u32),
                ruff_text_size::TextSize::new(*e as u32),
            )
        })
    }
    pub fn kind(&self, id: Id<Occurrence>) -> Option<SyntaxKind> {
        self.ranges.get(&id).map(|(_, _, kind)| *kind)
    }
    pub fn nodes(&self) -> impl Iterator<Item = (Id<Occurrence>, SyntaxKind)> + '_ {
        self.ranges.iter().map(|(id, (_, _, kind))| (*id, *kind))
    }
    pub fn parent(
        &self,
        occurrence: Id<Occurrence>,
    ) -> Option<(Id<Occurrence>, lctx_model::domain::lexical::SyntaxField)> {
        self.parents.get(&occurrence).copied()
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn get(
        &self,
        range: impl Into<ByteRange>,
        kind: SyntaxKind,
    ) -> Result<Id<Occurrence>, ModelError> {
        let range = range.into();
        match self
            .map
            .get(&(i64::from(range.start), i64::from(range.end), kind as i16))
        {
            Some(Some(id)) => Ok(*id),
            Some(None) => Err(invalid(format!(
                "two {kind:?} occurrences share the span {range:?}"
            ))),
            None => Err(invalid(format!(
                "no {kind:?} occurrence was emitted at {range:?}"
            ))),
        }
    }
}
fn kind_of(expr: &Expr) -> SyntaxKind {
    crate::typed_syntax::kind(AnyNodeRef::from(expr).kind())
}

/// Source inputs for the provider's per-alias lookup, attached by the retained parse.
#[derive(Debug, Clone)]
pub struct ImportLookup {
    pub qualification: Id<AssertionQualification>,
    pub alias: Id<Occurrence>,
    pub base: Option<String>,
    pub member: Option<String>,
    pub spelling: String,
    pub binding_position: TextSize,
}

/// The records of one module, qualified by its artifact's qualification.
#[derive(Debug, Default, Clone)]
pub struct Records {
    pub declarations: Vec<DeclarationObservation>,
    pub decorators: Vec<DeclarationDecorator>,
    pub imports: Vec<ImportAliasObservation>,
    pub import_lookups: Vec<ImportLookup>,
    pub dunder_all: Vec<DunderAllObservation>,
    /// `__all__` statements whose names the syntax does not state.
    pub computed_all: Vec<Id<Occurrence>>,
    pub parameters: Vec<ParameterSyntaxObservation>,
    /// Each parameter occurrence's formal: the `Parameter` node naming it (itself for `*args` and
    /// `**kwargs`, the node inside `x: int = 1` otherwise), the occurrence a declaration link names.
    pub formals: Vec<(Id<Occurrence>, Id<Occurrence>)>,
    pub fields: Vec<ClassFieldSyntaxObservation>,
    pub calls: Vec<(CallSyntax, Vec<CallArgument>)>,
    pub literals: Vec<Literal>,
    pub sets: Vec<(LiteralSet, Vec<LiteralSetMember>)>,
}

/// The imported module as an absolute name, relative levels resolved against `module`; `None` when
/// they climb past its top package.
pub fn absolute_module(
    module: &str,
    is_package: bool,
    level: i64,
    imported: Option<&str>,
) -> Option<String> {
    let suffix = imported.map(ruff_python_ast::name::Name::new);
    let name = ModuleName::from_str(module).new_maybe_relative(
        is_package,
        u32::try_from(level).ok()?,
        suffix.as_ref(),
    )?;
    (!name.as_str().is_empty()).then(|| name.to_string())
}

/// Walk one module's statements for its records.
pub fn records(
    ast: &ModModule,
    module: &str,
    is_package: bool,
    spans: &Spans,
    qualification: Id<AssertionQualification>,
    contextual: &[lctx_model::domain::ruff::RuffContextObservation],
) -> Result<Records, ModelError> {
    let mut walker = Walker {
        spans,
        contextual,
        qualification,
        module,
        is_package,
        enclosing: vec![],
        annotation: 0,
        records: Records::default(),
        error: None,
    };
    walker.visit_body(&ast.body);
    match walker.error {
        Some(error) => Err(error),
        None => Ok(walker.records),
    }
}

struct Walker<'a> {
    contextual: &'a [lctx_model::domain::ruff::RuffContextObservation],
    spans: &'a Spans,
    qualification: Id<AssertionQualification>,
    module: &'a str,
    is_package: bool,
    enclosing: Vec<Id<Occurrence>>,
    annotation: usize,
    records: Records,
    error: Option<ModelError>,
}
fn is_dunder_all(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(n) if n.id.as_str() == "__all__")
}
/// Known literal string subset, without guessing values from names or arbitrary calls.
struct StringSubset {
    literal: bool,
    names: Vec<String>,
}
fn string_sequence(expr: &Expr) -> StringSubset {
    let elements = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        Expr::BinOp(binary) if binary.op == ruff_python_ast_latest::Operator::Add => {
            let mut left = string_sequence(&binary.left);
            let right = string_sequence(&binary.right);
            left.names.extend(right.names);
            left.literal = false;
            return left;
        }
        _ => {
            return StringSubset {
                literal: false,
                names: vec![],
            };
        }
    };
    let mut out = StringSubset {
        literal: true,
        names: vec![],
    };
    for element in elements {
        if let Some(string) = element.as_string_literal_expr() {
            out.names.push(string.value.to_str().to_owned());
        } else if let Expr::Starred(starred) = element {
            out.literal = false;
            out.names.extend(string_sequence(&starred.value).names);
        } else {
            out.literal = false;
        }
    }
    out
}
impl Walker<'_> {
    fn occ(&self, range: TextRange, kind: SyntaxKind) -> Result<Id<Occurrence>, ModelError> {
        self.spans.get(range, kind)
    }
    fn expr(&self, expr: &Expr) -> Result<Id<Occurrence>, ModelError> {
        self.occ(expr.range(), kind_of(expr))
    }
    fn run(&mut self, step: impl FnOnce(&mut Self) -> Result<(), ModelError>) {
        if self.error.is_none()
            && let Err(error) = step(self)
        {
            self.error = Some(error);
        }
    }
    fn docstring(&self, body: &[Stmt]) -> Result<Option<Id<Occurrence>>, ModelError> {
        match (crate::docstrings::docstring(body), body.first()) {
            (Some(_), Some(statement @ Stmt::Expr(_))) => {
                Ok(Some(self.occ(statement.range(), SyntaxKind::StmtExpr)?))
            }
            _ => Ok(None),
        }
    }
    fn declaration(
        &mut self,
        declaration: Id<Occurrence>,
        name: TextRange,
        kind: DeclarationKind,
        decorators: &[ruff_python_ast_latest::Decorator],
        body: &[Stmt],
    ) -> Result<(), ModelError> {
        let overload = decorators
            .iter()
            .any(|decorator| {
                let expression=match &decorator.expression { Expr::Call(call)=>&*call.func,expression=>expression };
                self.expr(expression).ok().and_then(|subject|lctx_model::domain::ruff::resolved_name(self.contextual,self.qualification,subject))
                    .is_some_and(|name| matches!(name, [module,member] if (module=="typing"||module=="typing_extensions") && member=="overload"))
            });
        self.records.declarations.push(DeclarationObservation {
            qualification: self.qualification,
            declaration,
            name: self.occ(name, SyntaxKind::Identifier)?,
            kind,
            parent: self.enclosing.last().copied(),
            overload,
            docstring: self.docstring(body)?,
        });
        for (ordinal, decorator) in decorators.iter().enumerate() {
            self.records.decorators.push(DeclarationDecorator {
                qualification: self.qualification,
                declaration,
                decorator: self.occ(decorator.range(), SyntaxKind::Decorator)?,
                ordinal: ordinal as i64,
            });
        }
        Ok(())
    }
    fn parameters(
        &mut self,
        function: Id<Occurrence>,
        parameters: &Parameters,
    ) -> Result<(), ModelError> {
        let mut ordinal = 0i64;
        let mut push = |walker: &mut Self,
                        parameter: Id<Occurrence>,
                        kind: ParameterKind,
                        default: Option<&Expr>,
                        annotation: Option<&Expr>|
         -> Result<(), ModelError> {
            let default_literal = default.and_then(crate::typed_syntax::literal);
            let row = ParameterSyntaxObservation {
                qualification: walker.qualification,
                function,
                parameter,
                ordinal,
                kind,
                default: default.map(|d| walker.expr(d)).transpose()?,
                default_literal: default_literal.as_ref().map(Record::id),
                annotation: annotation.map(|a| walker.expr(a)).transpose()?,
            };
            walker.records.literals.extend(default_literal);
            walker.records.parameters.push(row);
            ordinal += 1;
            Ok(())
        };
        for (group, kind) in [
            (&parameters.posonlyargs, ParameterKind::PositionalOnly),
            (&parameters.args, ParameterKind::PositionalOrKeyword),
        ] {
            for p in group {
                let id = self.occ(p.range(), SyntaxKind::ParameterWithDefault)?;
                let formal = self.occ(p.parameter.range(), SyntaxKind::Parameter)?;
                self.records.formals.push((id, formal));
                push(
                    self,
                    id,
                    kind,
                    p.default.as_deref(),
                    p.parameter.annotation.as_deref(),
                )?;
            }
        }
        if let Some(p) = &parameters.vararg {
            let id = self.occ(p.range(), SyntaxKind::Parameter)?;
            self.records.formals.push((id, id));
            push(
                self,
                id,
                ParameterKind::VarPositional,
                None,
                p.annotation.as_deref(),
            )?;
        }
        for p in &parameters.kwonlyargs {
            let id = self.occ(p.range(), SyntaxKind::ParameterWithDefault)?;
            let formal = self.occ(p.parameter.range(), SyntaxKind::Parameter)?;
            self.records.formals.push((id, formal));
            push(
                self,
                id,
                ParameterKind::KeywordOnly,
                p.default.as_deref(),
                p.parameter.annotation.as_deref(),
            )?;
        }
        if let Some(p) = &parameters.kwarg {
            let id = self.occ(p.range(), SyntaxKind::Parameter)?;
            self.records.formals.push((id, id));
            push(
                self,
                id,
                ParameterKind::VarKeyword,
                None,
                p.annotation.as_deref(),
            )?;
        }
        Ok(())
    }
    fn dunder_all(
        &mut self,
        statement: TextRange,
        kind: SyntaxKind,
        names: StringSubset,
    ) -> Result<(), ModelError> {
        let statement = self.occ(statement, kind)?;
        if !names.literal {
            self.records.computed_all.push(statement);
        }
        match (names.literal, names.names) {
            (literal, names) if literal || !names.is_empty() => {
                let literals: Vec<Literal> = names
                    .into_iter()
                    .map(|value| Literal::String {
                        value: value.into(),
                    })
                    .collect();
                let (set, members) = LiteralSet::of(literals.iter().map(Record::id));
                self.records.dunder_all.push(DunderAllObservation {
                    qualification: self.qualification,
                    statement,
                    literal,
                    names: Some(set.id()),
                });
                self.records.literals.extend(literals);
                self.records.sets.push((set, members));
            }
            _ => {
                self.records.dunder_all.push(DunderAllObservation {
                    qualification: self.qualification,
                    statement,
                    literal: false,
                    names: None,
                });
            }
        }
        Ok(())
    }
    fn statement(&mut self, stmt: &Stmt) -> Result<(), ModelError> {
        let module_level = self.enclosing.is_empty();
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let occurrence = self.occ(alias.range(), SyntaxKind::Alias)?;
                    self.records.imports.push(ImportAliasObservation {
                        qualification: self.qualification,
                        statement: self.occ(stmt.range(), SyntaxKind::StmtImport)?,
                        alias: occurrence,
                        level: 0,
                        resolved_module: Some(alias.name.to_string()),
                    });
                    self.records.import_lookups.push(ImportLookup {
                        qualification: self.qualification,
                        alias: occurrence,
                        base: Some(alias.name.to_string()),
                        member: None,
                        spelling: alias.name.to_string(),
                        binding_position: alias
                            .asname
                            .as_ref()
                            .unwrap_or(&alias.name)
                            .range()
                            .start(),
                    });
                }
            }
            Stmt::ImportFrom(from) => {
                for alias in &from.names {
                    let level = i64::from(from.level);
                    let occurrence = self.occ(alias.range(), SyntaxKind::Alias)?;
                    let base = absolute_module(
                        self.module,
                        self.is_package,
                        level,
                        from.module.as_deref(),
                    );
                    self.records.imports.push(ImportAliasObservation {
                        qualification: self.qualification,
                        statement: self.occ(stmt.range(), SyntaxKind::StmtImportFrom)?,
                        alias: occurrence,
                        level,
                        resolved_module: base.clone(),
                    });
                    self.records.import_lookups.push(ImportLookup {
                        qualification: self.qualification,
                        alias: occurrence,
                        base,
                        member: Some(alias.name.to_string()),
                        spelling: format!(
                            "{}{}{}",
                            ".".repeat(from.level as usize),
                            from.module
                                .as_deref()
                                .map(|module| format!("{module}."))
                                .unwrap_or_default(),
                            alias.name
                        ),
                        binding_position: alias
                            .asname
                            .as_ref()
                            .unwrap_or(&alias.name)
                            .range()
                            .start(),
                    });
                }
            }
            Stmt::Assign(assign) if module_level && assign.targets.iter().any(is_dunder_all) => {
                self.dunder_all(
                    stmt.range(),
                    SyntaxKind::StmtAssign,
                    string_sequence(&assign.value),
                )?
            }
            Stmt::AugAssign(assign) if module_level && is_dunder_all(&assign.target) => self
                .dunder_all(
                    stmt.range(),
                    SyntaxKind::StmtAugAssign,
                    string_sequence(&assign.value),
                )?,
            Stmt::AnnAssign(assign)
                if module_level && is_dunder_all(&assign.target) && assign.value.is_some() =>
            {
                self.dunder_all(
                    stmt.range(),
                    SyntaxKind::StmtAnnAssign,
                    string_sequence(assign.value.as_deref().expect("guarded __all__ value")),
                )?
            }
            Stmt::Expr(expr) if module_level => {
                // `__all__.extend([...])` and `__all__.append("x")` add literal names; any other
                // mutation (`remove`, a computed argument) is not stated by the syntax.
                if let Expr::Call(call) = expr.value.as_ref()
                    && let Expr::Attribute(attribute) = call.func.as_ref()
                    && is_dunder_all(&attribute.value)
                {
                    let argument = call
                        .arguments
                        .args
                        .first()
                        .filter(|_| call.arguments.len() == 1);
                    let names = match (attribute.attr.as_str(), argument) {
                        ("extend", Some(argument)) => string_sequence(argument),
                        ("append", Some(Expr::StringLiteral(s))) => StringSubset {
                            literal: true,
                            names: vec![s.value.to_str().to_owned()],
                        },
                        _ => StringSubset {
                            literal: false,
                            names: vec![],
                        },
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
        if self.error.is_some() {
            return;
        }
        match stmt {
            Stmt::FunctionDef(def) => {
                let declaration = match self.occ(stmt.range(), SyntaxKind::StmtFunctionDef) {
                    Ok(id) => id,
                    Err(e) => {
                        self.error = Some(e);
                        return;
                    }
                };
                let kind = if def.is_async {
                    DeclarationKind::AsyncFunction
                } else {
                    DeclarationKind::Function
                };
                self.run(|w| {
                    w.declaration(
                        declaration,
                        def.name.range(),
                        kind,
                        &def.decorator_list,
                        &def.body,
                    )
                });
                self.run(|w| w.parameters(declaration, &def.parameters));
                self.enclosing.push(declaration);
                source_order::walk_stmt(self, stmt);
                self.enclosing.pop();
            }
            Stmt::ClassDef(class) => {
                let declaration = match self.occ(stmt.range(), SyntaxKind::StmtClassDef) {
                    Ok(id) => id,
                    Err(e) => {
                        self.error = Some(e);
                        return;
                    }
                };
                self.run(|w| {
                    w.declaration(
                        declaration,
                        class.name.range(),
                        DeclarationKind::Class,
                        &class.decorator_list,
                        &class.body,
                    )
                });
                self.run(|w| {
                    for member in &class.body {
                        let (target, annotation, value) = match member {
                            Stmt::AnnAssign(a) if matches!(*a.target, Expr::Name(_)) => {
                                (&*a.target, Some(&*a.annotation), a.value.as_deref())
                            }
                            Stmt::Assign(a)
                                if a.targets.len() == 1
                                    && matches!(a.targets[0], Expr::Name(_)) =>
                            {
                                (&a.targets[0], None, Some(&*a.value))
                            }
                            _ => continue,
                        };
                        w.records.fields.push(ClassFieldSyntaxObservation {
                            qualification: w.qualification,
                            class: declaration,
                            target: w.expr(target)?,
                            annotation: annotation.map(|a| w.expr(a)).transpose()?,
                            value: value.map(|v| w.expr(v)).transpose()?,
                        });
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
        if self.error.is_some() {
            return;
        }
        match expr {
            Expr::Call(call) => self.run(|w| {
                let mut actuals = Vec::new();
                for argument in call.arguments.iter_source_order() {
                    actuals.push(match argument {
                        ArgOrKeyword::Arg(value @ Expr::Starred(_)) => Actual {
                            occurrence: w.expr(value)?,
                            kind: ArgumentKind::Starred,
                            keyword: None,
                        },
                        ArgOrKeyword::Arg(value) => Actual {
                            occurrence: w.expr(value)?,
                            kind: ArgumentKind::Positional,
                            keyword: None,
                        },
                        ArgOrKeyword::Keyword(keyword) => match &keyword.arg {
                            Some(name) => Actual {
                                occurrence: w.expr(&keyword.value)?,
                                kind: ArgumentKind::Keyword,
                                keyword: Some(name.to_string()),
                            },
                            None => Actual {
                                occurrence: w.expr(&keyword.value)?,
                                kind: ArgumentKind::DoubleStarred,
                                keyword: None,
                            },
                        },
                    });
                }
                let site = w.occ(expr.range(), SyntaxKind::ExprCall)?;
                w.records.calls.push(CallSyntax::new(
                    w.qualification,
                    site,
                    w.expr(&call.func)?,
                    w.annotation > 0,
                    &actuals,
                )?);
                Ok(())
            }),
            Expr::Lambda(lambda) => self.run(|w| match &lambda.parameters {
                Some(parameters) => {
                    let id = w.occ(expr.range(), SyntaxKind::ExprLambda)?;
                    w.parameters(id, parameters)
                }
                None => Ok(()),
            }),
            _ => {}
        }
        source_order::walk_expr(self, expr);
    }
}

type SpanEvents = HashMap<(i64, i64), Vec<(SyntaxKind, OccurrenceRole, Id<Occurrence>)>>;
