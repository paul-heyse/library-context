//! Our lexical recognizer (CPG slice C3, DESIGN §3.2 `lexical`; surface `lctx-lexical`,
//! extraction mode `recognizer`): scopes, binding events with shadowed history, name references,
//! and their resolution under Python's scoping rules. It runs inside the Ruff walk, so a name's
//! syntax id is the walk's own.
//!
//! Scoping, as Python defines it: a name bound anywhere in a scope is local to it unless declared
//! `global` or `nonlocal`, and every binding event under such a declaration binds in the declared
//! scope; a free name resolves in the nearest enclosing function scope that binds it, skipping
//! class scopes, then the module, then the star imports that export it, then the builtins. A
//! function's defaults, annotations and decorators, and a class's bases, evaluate in the enclosing
//! scope; a comprehension's first iterable too; a scope's parent is the scope its position
//! evaluates in. A walrus binds in the nearest non-comprehension scope. A module or class body
//! reads a name it binds only later from outside (`LOAD_NAME`), so such a read is a candidate of
//! both. Resolution is otherwise flow-insensitive: every binding of the name in the resolving
//! scope is a candidate, with its ordinal, so an order-aware consumer (Pass B's binding rule,
//! §4.2.4) can choose. Branches Pyrefly decides statically are kept and marked.
//!
//! Every name set from outside the module's text is Pyrefly's (C3 review F1): the module's
//! implicit globals (`ImplicitGlobal`), the builtins (Pyrefly's real, public `builtins`
//! definitions) and each star import's wildcard set. A name in none of them is unresolved. Names
//! inside annotations are the `types` family's (C4): nothing there opens a scope or binds.

use std::collections::{HashMap, HashSet};

use cpg_schema::codebook::{
    BindingKind, BoundaryReason, ExtractionMode, Fidelity, LexicalScopeKind, Modality, Origin,
    StaticBranch, SymbolKind, SyntaxField,
};
use cpg_schema::id::{Id, recipe};
use cpg_schema::tables::{
    Bindings, BindingsRow, ReferenceResolutions, ReferenceResolutionsRow, References,
    ReferencesRow, Scopes, ScopesRow,
};
use pyrefly_python::ast::Ast;
use pyrefly_python::sys_info::SysInfo;
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::{Expr, Stmt, StmtIf};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::facts::{FactSink, Provenance, Surface, fact_row};
use crate::walk::span;

/// One resolution row: the binding, whether captured, the builtin, the reason.
type Resolved = (Option<Id>, bool, Option<String>, Option<BoundaryReason>);

/// Names from outside the module's text, as Pyrefly defines them: the builtins with their symbol
/// kind, and a module's implicit globals.
#[derive(Default)]
pub(crate) struct Outside {
    pub builtins: HashMap<String, Option<SymbolKind>>,
    pub implicit_globals: Vec<String>,
}

/// One module's star imports: each `*` alias (by its start) → the names it binds, or `None` when
/// Pyrefly cannot find the module.
pub(crate) type Stars = HashMap<u32, Option<HashSet<String>>>;

#[derive(Default)]
pub(crate) struct LexicalOut {
    pub scopes: Vec<ScopesRow>,
    pub bindings: Vec<BindingsRow>,
    pub references: Vec<ReferencesRow>,
    pub resolutions: Vec<ReferenceResolutionsRow>,
    /// Builtin functions and classes references resolve to: their definitions become
    /// `context_definitions` (external symbols).
    pub builtins_used: HashSet<String>,
}

fn provenance(modality: Modality) -> Provenance {
    Provenance {
        surface: Surface::Lexical,
        mode: ExtractionMode::Recognizer,
        origin: Origin::DerivedAnalysis,
        modality,
        fidelity: Fidelity::NormalizedStructural,
    }
}

struct ScopeRec {
    id: Id,
    kind: LexicalScopeKind,
    owner: Id,
    parent: Option<usize>,
    span: TextRange,
    /// Where names belong to this scope (a function's body, a class's body, a lambda's body, a
    /// whole comprehension).
    region: TextRange,
    /// Inside `region` but evaluated in the enclosing scope (a comprehension's first iterable).
    excluded: Option<TextRange>,
    /// A function's or lambda's parameter list: its parameters bind here.
    params: Option<TextRange>,
    /// Binding indices per name (in source order).
    names: HashMap<String, Vec<usize>>,
    globals: HashSet<String>,
    nonlocals: HashSet<String>,
}

/// A binding event: emitted in `finish`, once `nonlocal` targets are known.
struct BindRec {
    id: Id,
    kind: BindingKind,
    scope: usize,
    name: String,
    site: Id,
    range: TextRange,
    value: Option<TextRange>,
    branch: Option<(StaticBranch, bool)>,
    /// Bound under a `nonlocal` declaration: its scope is decided in `finish`.
    nonlocal: bool,
}

struct RefRec {
    id: Id,
    scope: usize,
    name: String,
    at: TextSize,
}

/// What an enclosing construct makes of a name stored inside `range`.
struct StoreCtx {
    range: TextRange,
    kind: BindingKind,
    value: Option<TextRange>,
}

/// The per-node undo record: what entering the node pushed.
#[derive(Default)]
struct Pushed {
    stores: usize,
    branches: usize,
    scope: bool,
}

pub(crate) struct Lexical<'b> {
    module_node_id: Id,
    /// The module's Python version and platform, for Pyrefly's static tests.
    sys: &'b SysInfo,
    outside: &'b Outside,
    stars: &'b Stars,
    scopes: Vec<ScopeRec>,
    open: Vec<usize>,
    stores: Vec<StoreCtx>,
    branches: Vec<(TextRange, StaticBranch, bool)>,
    pushed: Vec<Pushed>,
    binds: Vec<BindRec>,
    /// Binding ids already recorded: a repeated name in one `global`/`nonlocal` is one event.
    bound: HashSet<Id>,
    /// Each star-import binding's wildcard set.
    star_sets: HashMap<usize, Option<HashSet<String>>>,
    /// Each class scope's `__class__` cell, made when a method reads `__class__`.
    class_cells: HashMap<usize, usize>,
    refs: Vec<RefRec>,
    /// Inside a `from m import …` statement: its aliases bind as `from_import`.
    in_from_import: bool,
    out: LexicalOut,
}

/// Each clause of an `if` statement as Pyrefly decides it (H1 C1): `SysInfo::evaluate_bool` per
/// clause, applied as `SysInfo::pruned_if_branches` applies it. `Some((kind, kept))`: the clause is
/// statically decided, and Pyrefly analyzes it (`kept`) or prunes it; `None`: it depends on the
/// runtime. An `else` is decided when every earlier test was decided false, and every clause after
/// one decided true is pruned. The kind is the deciding test's (the true one's, for the clauses it
/// prunes).
pub(crate) fn clause_marks(sys: &SysInfo, i: &StmtIf) -> Vec<Option<(StaticBranch, bool)>> {
    let mut marks = Vec::new();
    let mut taken: Option<StaticBranch> = None;
    let mut all_false = true;
    let mut last_false: Option<StaticBranch> = None;
    for (test, _) in Ast::if_branches(i) {
        let mark = if let Some(kind) = taken {
            Some((kind, false))
        } else {
            match test {
                None if all_false => last_false.map(|kind| (kind, true)),
                None => None,
                Some(test) => match sys.evaluate_bool(test) {
                    Some(holds) => {
                        let kind = static_kind(test);
                        if holds {
                            taken = Some(kind);
                        } else {
                            last_false = Some(kind);
                        }
                        Some((kind, holds))
                    }
                    None => {
                        all_false = false;
                        None
                    }
                },
            }
        };
        marks.push(mark);
    }
    marks
}

/// What a decided test reads: the kind is from the expression tree, never its text.
fn static_kind(test: &Expr) -> StaticBranch {
    let named = |e: &Expr, module: &str| matches!(e, Expr::Name(n) if n.id.as_str() == module);
    let checking = any_over_expr(test, |e| match e {
        Expr::Name(n) => SysInfo::is_type_checking_constant_name(n.id.as_str()),
        Expr::Attribute(a) => {
            a.value.is_name_expr() && SysInfo::is_type_checking_constant_name(a.attr.as_str())
        }
        _ => false,
    });
    let version = any_over_expr(
        test,
        |e| matches!(e, Expr::Attribute(a) if a.attr.as_str() == "version_info" && named(&a.value, "sys")),
    );
    let platform = any_over_expr(test, |e| {
        matches!(e, Expr::Attribute(a)
            if (a.attr.as_str() == "platform" && named(&a.value, "sys"))
                || (a.attr.as_str() == "name" && named(&a.value, "os")))
    });
    match (checking, version, platform) {
        (true, false, false) => StaticBranch::TypeChecking,
        (false, true, false) => StaticBranch::VersionInfo,
        (false, false, true) => StaticBranch::Platform,
        (false, false, false) => StaticBranch::Constant,
        _ => StaticBranch::Combined,
    }
}

fn suite(body: &[Stmt]) -> Option<TextRange> {
    Some(TextRange::new(body.first()?.start(), body.last()?.end()))
}

impl<'b> Lexical<'b> {
    pub(crate) fn new(
        module_node_id: Id,
        module_span: TextRange,
        sys: &'b SysInfo,
        outside: &'b Outside,
        stars: &'b Stars,
    ) -> Self {
        let mut this = Self {
            module_node_id,
            sys,
            outside,
            stars,
            scopes: Vec::new(),
            open: Vec::new(),
            stores: Vec::new(),
            branches: Vec::new(),
            pushed: Vec::new(),
            binds: Vec::new(),
            bound: HashSet::new(),
            star_sets: HashMap::new(),
            class_cells: HashMap::new(),
            refs: Vec::new(),
            in_from_import: false,
            out: LexicalOut::default(),
        };
        this.open_scope(
            LexicalScopeKind::Module,
            module_node_id,
            module_span,
            module_span,
            None,
            None,
        );
        // The module's implicit globals are bound before its first statement.
        let start = TextRange::empty(module_span.start());
        for name in &outside.implicit_globals {
            this.bind(0, name, BindingKind::Implicit, module_node_id, start, None);
        }
        this
    }

    fn open_scope(
        &mut self,
        kind: LexicalScopeKind,
        owner: Id,
        span: TextRange,
        region: TextRange,
        excluded: Option<TextRange>,
        params: Option<TextRange>,
    ) {
        // The scope the new one's position evaluates in (review F4): a lambda in a default or a
        // decorator belongs to the enclosing scope, not to the function it decorates.
        let parent = (!self.open.is_empty()).then(|| self.scope_at(span.start()));
        self.scopes.push(ScopeRec {
            id: recipe::scope(owner),
            kind,
            owner,
            parent,
            span,
            region,
            excluded,
            params,
            names: HashMap::new(),
            globals: HashSet::new(),
            nonlocals: HashSet::new(),
        });
        self.open.push(self.scopes.len() - 1);
    }

    /// The scope a name at `at` belongs to: the innermost open scope whose region holds it.
    fn scope_at(&self, at: TextSize) -> usize {
        for &i in self.open.iter().rev() {
            let s = &self.scopes[i];
            if s.region.contains_inclusive(at) && !s.excluded.is_some_and(|x| x.contains(at)) {
                return i;
            }
        }
        self.open[0]
    }

    /// The scope a parameter at `at` binds in: the innermost open function or lambda whose
    /// parameter list holds it.
    fn params_scope(&self, at: TextSize) -> Option<usize> {
        self.open.iter().rev().copied().find(|&i| {
            self.scopes[i]
                .params
                .is_some_and(|p| p.contains_inclusive(at))
        })
    }

    /// Enter a node: open scopes and contexts, record bindings and references. `syntax_id` is the
    /// node's structural id, `parent` and `field` where the nearest placed ancestor has it.
    #[allow(
        clippy::too_many_arguments,
        reason = "a registry row or binding event is these fields"
    )]
    pub(crate) fn enter(
        &mut self,
        node: ruff_python_ast::AnyNodeRef<'_>,
        syntax_id: Id,
        own_id: Option<Id>,
        param_id: Option<Id>,
        parent: (Id, SyntaxField),
        in_annotation: bool,
        sink: &mut FactSink,
    ) {
        use ruff_python_ast::AnyNodeRef as N;
        let mut pushed = Pushed::default();
        let r = node.range();
        // Annotations are the `types` family's (C4): no scope, binding or reference there, so a
        // lambda in `Annotated[..., Field(default_factory=lambda: [])]` opens nothing (review F6).
        if in_annotation {
            self.pushed.push(pushed);
            return;
        }
        match node {
            N::StmtFunctionDef(f) => {
                let decl = own_id.expect("a def has its declaration id");
                let here = self.scope_at(r.start());
                self.bind(
                    here,
                    &f.name,
                    BindingKind::FunctionDef,
                    decl,
                    f.name.range(),
                    None,
                );
                let region = suite(&f.body).map_or(r, |b| TextRange::new(b.start(), r.end()));
                self.open_scope(
                    LexicalScopeKind::Function,
                    decl,
                    r,
                    region,
                    None,
                    Some(f.parameters.range()),
                );
                pushed.scope = true;
                if let Some(tp) = &f.type_params {
                    for p in &tp.type_params {
                        let name = p.name();
                        let s = *self.open.last().expect("just opened");
                        self.bind(s, name, BindingKind::TypeParam, decl, name.range(), None);
                    }
                }
            }
            N::StmtClassDef(c) => {
                let decl = own_id.expect("a class has its declaration id");
                let here = self.scope_at(r.start());
                self.bind(
                    here,
                    &c.name,
                    BindingKind::ClassDef,
                    decl,
                    c.name.range(),
                    None,
                );
                let region = suite(&c.body).map_or(r, |b| TextRange::new(b.start(), r.end()));
                self.open_scope(LexicalScopeKind::Class, decl, r, region, None, None);
                pushed.scope = true;
            }
            N::ExprLambda(l) => {
                let region = TextRange::new(l.body.start(), r.end());
                self.open_scope(
                    LexicalScopeKind::Lambda,
                    syntax_id,
                    r,
                    region,
                    None,
                    l.parameters.as_ref().map(|p| p.range()),
                );
                pushed.scope = true;
            }
            N::ExprListComp(_) | N::ExprSetComp(_) | N::ExprDictComp(_) | N::ExprGenerator(_) => {
                let generators = match node {
                    N::ExprListComp(x) => &x.generators,
                    N::ExprSetComp(x) => &x.generators,
                    N::ExprDictComp(x) => &x.generators,
                    N::ExprGenerator(x) => &x.generators,
                    _ => unreachable!("matched above"),
                };
                let first_iter = generators.first().map(|g| g.iter.range());
                self.open_scope(
                    LexicalScopeKind::Comprehension,
                    syntax_id,
                    r,
                    r,
                    first_iter,
                    None,
                );
                pushed.scope = true;
            }
            N::Parameter(p) => {
                if let Some(s) = self.params_scope(r.start()) {
                    let site = param_id.unwrap_or(syntax_id);
                    self.bind(
                        s,
                        &p.name,
                        BindingKind::Parameter,
                        site,
                        p.name.range(),
                        None,
                    );
                }
            }
            N::StmtAssign(s) => {
                for t in &s.targets {
                    self.stores.push(StoreCtx {
                        range: t.range(),
                        kind: BindingKind::Assignment,
                        value: Some(s.value.range()),
                    });
                    pushed.stores += 1;
                }
            }
            N::StmtAnnAssign(s) => {
                self.stores.push(StoreCtx {
                    range: s.target.range(),
                    kind: if s.value.is_some() {
                        BindingKind::Assignment
                    } else {
                        BindingKind::AnnotationOnly
                    },
                    value: s.value.as_ref().map(|v| v.range()),
                });
                pushed.stores += 1;
            }
            N::StmtAugAssign(s) => {
                self.stores.push(StoreCtx {
                    range: s.target.range(),
                    kind: BindingKind::AugAssignment,
                    value: Some(s.value.range()),
                });
                pushed.stores += 1;
            }
            N::StmtFor(s) => {
                self.stores.push(StoreCtx {
                    range: s.target.range(),
                    kind: BindingKind::ForTarget,
                    value: Some(s.iter.range()),
                });
                pushed.stores += 1;
            }
            N::WithItem(w) => {
                if let Some(v) = &w.optional_vars {
                    self.stores.push(StoreCtx {
                        range: v.range(),
                        kind: BindingKind::WithTarget,
                        value: Some(w.context_expr.range()),
                    });
                    pushed.stores += 1;
                }
            }
            N::Comprehension(c) => {
                self.stores.push(StoreCtx {
                    range: c.target.range(),
                    kind: BindingKind::ComprehensionTarget,
                    value: Some(c.iter.range()),
                });
                pushed.stores += 1;
            }
            N::ExprNamed(n) => {
                self.stores.push(StoreCtx {
                    range: n.target.range(),
                    kind: BindingKind::Walrus,
                    value: Some(n.value.range()),
                });
                pushed.stores += 1;
            }
            N::StmtTypeAlias(t) => {
                self.stores.push(StoreCtx {
                    range: t.name.range(),
                    kind: BindingKind::TypeAlias,
                    value: Some(t.value.range()),
                });
                pushed.stores += 1;
            }
            // An `if` inside a clause Pyrefly prunes is never walked by Pyrefly, whatever its own
            // test decides: its marks are not pushed, so a binding in it finds the pruning clause
            // (H1 review F1).
            N::StmtIf(i)
                if !self
                    .branches
                    .iter()
                    .any(|(b, _, kept)| !kept && b.contains_range(r)) =>
            {
                let ranges = std::iter::once(suite(&i.body))
                    .chain(i.elif_else_clauses.iter().map(|c| Some(c.range())));
                for (mark, range) in clause_marks(self.sys, i).into_iter().zip(ranges) {
                    if let (Some((kind, kept)), Some(range)) = (mark, range) {
                        self.branches.push((range, kind, kept));
                        pushed.branches += 1;
                    }
                }
            }
            N::ExceptHandlerExceptHandler(h) => {
                if let Some(name) = &h.name {
                    let here = self.scope_at(r.start());
                    self.bind(
                        here,
                        name,
                        BindingKind::ExceptHandler,
                        syntax_id,
                        name.range(),
                        None,
                    );
                }
            }
            N::StmtImport(_) | N::StmtImportFrom(_) => {}
            N::Alias(a) => {
                let here = self.scope_at(r.start());
                if a.name.as_str() == "*" {
                    let set = self.stars.get(&a.start().to_u32()).cloned().flatten();
                    let index = self.binds.len();
                    self.bind(
                        here,
                        "*",
                        BindingKind::StarImport,
                        syntax_id,
                        a.range(),
                        None,
                    );
                    self.star_sets.insert(index, set);
                } else {
                    // `import a.b` binds `a`; `import a.b as c` and `from m import x [as y]` bind
                    // the alias or the name.
                    let (name, kind) = match &a.asname {
                        Some(asname) => (asname.to_string(), BindingKind::Import),
                        None => (
                            a.name.split('.').next().unwrap_or(&a.name).to_owned(),
                            BindingKind::Import,
                        ),
                    };
                    let kind = if self.in_from_import {
                        BindingKind::FromImport
                    } else {
                        kind
                    };
                    let range = a.asname.as_ref().map_or(a.name.range(), Ranged::range);
                    self.bind(here, &name, kind, syntax_id, range, None);
                }
            }
            N::StmtGlobal(g) => {
                let here = self.scope_at(r.start());
                for n in &g.names {
                    self.scopes[here].globals.insert(n.to_string());
                    self.bind(here, n, BindingKind::Global, syntax_id, n.range(), None);
                }
            }
            N::StmtNonlocal(g) => {
                let here = self.scope_at(r.start());
                for n in &g.names {
                    self.scopes[here].nonlocals.insert(n.to_string());
                    self.bind(here, n, BindingKind::Nonlocal, syntax_id, n.range(), None);
                }
            }
            N::PatternMatchAs(p) => {
                if let Some(name) = &p.name {
                    let here = self.scope_at(r.start());
                    self.bind(
                        here,
                        name,
                        BindingKind::MatchCapture,
                        syntax_id,
                        name.range(),
                        None,
                    );
                }
            }
            N::PatternMatchStar(p) => {
                if let Some(name) = &p.name {
                    let here = self.scope_at(r.start());
                    self.bind(
                        here,
                        name,
                        BindingKind::MatchCapture,
                        syntax_id,
                        name.range(),
                        None,
                    );
                }
            }
            N::PatternMatchMapping(p) => {
                if let Some(name) = &p.rest {
                    let here = self.scope_at(r.start());
                    self.bind(
                        here,
                        name,
                        BindingKind::MatchCapture,
                        syntax_id,
                        name.range(),
                        None,
                    );
                }
            }
            N::ExprName(n) => {
                let here = self.scope_at(r.start());
                match n.ctx {
                    ruff_python_ast::ExprContext::Load => {
                        self.reference(here, syntax_id, &n.id, r, parent, sink);
                    }
                    ruff_python_ast::ExprContext::Store => {
                        let ctx = self.stores.iter().rev().find(|s| s.range.contains_range(r));
                        let (kind, value) =
                            ctx.map_or((BindingKind::Assignment, None), |c| (c.kind, c.value));
                        let target = if kind == BindingKind::Walrus {
                            self.non_comprehension(here)
                        } else {
                            here
                        };
                        // `x += 1` reads `x` before it binds it (and Pysa reports the read).
                        if kind == BindingKind::AugAssignment {
                            self.reference(here, syntax_id, &n.id, r, parent, sink);
                        }
                        self.bind(target, &n.id, kind, syntax_id, r, value);
                    }
                    ruff_python_ast::ExprContext::Del => {
                        self.bind(here, &n.id, BindingKind::Del, syntax_id, r, None);
                    }
                    ruff_python_ast::ExprContext::Invalid => {}
                }
            }
            _ => {}
        }
        if matches!(node, N::StmtImportFrom(_)) {
            self.in_from_import = true;
        }
        self.pushed.push(pushed);
    }

    /// A name read in scope `here`: its `references` row, resolved in `finish`.
    fn reference(
        &mut self,
        here: usize,
        syntax_id: Id,
        name: &str,
        r: TextRange,
        parent: (Id, SyntaxField),
        sink: &mut FactSink,
    ) {
        let id = recipe::reference(syntax_id);
        let (start, end) = span(r);
        let row = fact_row!(
            sink,
            References,
            provenance(Modality::Definite),
            ReferencesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id: id,
                name_node_id: syntax_id,
                scope_id: self.scopes[here].id,
                module_node_id: self.module_node_id,
                name: name.to_owned(),
                parent_node_id: parent.0,
                field: parent.1,
                start_byte: start,
                end_byte: end,
            }
        );
        self.out.references.push(row);
        self.refs.push(RefRec {
            id,
            scope: here,
            name: name.to_owned(),
            at: r.start(),
        });
    }

    /// Leave a node: undo what entering it pushed.
    pub(crate) fn leave(&mut self, node: ruff_python_ast::AnyNodeRef<'_>) {
        if matches!(node, ruff_python_ast::AnyNodeRef::StmtImportFrom(_)) {
            self.in_from_import = false;
        }
        let p = self.pushed.pop().expect("enter and leave pair");
        self.stores.truncate(self.stores.len() - p.stores);
        self.branches.truncate(self.branches.len() - p.branches);
        if p.scope {
            self.open.pop();
        }
    }

    /// Where an event binding `name` in scope `s` binds (review F5): the module under `global`,
    /// the declaring scope itself under `nonlocal` until `finish` knows the target, else `s`.
    /// Returns the scope and whether the event waits for its `nonlocal` target.
    fn declared_scope(&self, s: usize, name: &str) -> (usize, bool) {
        let scope = &self.scopes[s];
        if scope.globals.contains(name) {
            (self.module_scope(), false)
        } else if scope.nonlocals.contains(name) {
            (s, true)
        } else {
            (s, false)
        }
    }

    fn non_comprehension(&self, mut s: usize) -> usize {
        while self.scopes[s].kind == LexicalScopeKind::Comprehension {
            match self.scopes[s].parent {
                Some(p) => s = p,
                None => break,
            }
        }
        s
    }

    /// Record a binding event of `name` made in scope `here`: every kind but the declarations
    /// themselves, the star marker, parameters and implicit names follows `global`/`nonlocal`.
    /// The same (site, name) is one event (`global g, g`).
    fn bind(
        &mut self,
        here: usize,
        name: &str,
        kind: BindingKind,
        site: Id,
        range: TextRange,
        value: Option<TextRange>,
    ) {
        let id = recipe::binding(site, name);
        if !self.bound.insert(id) {
            return;
        }
        let (scope, nonlocal) = match kind {
            BindingKind::ImportFromSubmodule => {
                unreachable!("ty-only package submodule definitions are not lexical bindings")
            }
            BindingKind::Global
            | BindingKind::Nonlocal
            | BindingKind::StarImport
            | BindingKind::Parameter
            | BindingKind::TypeParam
            | BindingKind::Implicit => (here, false),
            BindingKind::FunctionDef
            | BindingKind::ClassDef
            | BindingKind::Assignment
            | BindingKind::AugAssignment
            | BindingKind::AnnotationOnly
            | BindingKind::ForTarget
            | BindingKind::WithTarget
            | BindingKind::ExceptHandler
            | BindingKind::Import
            | BindingKind::FromImport
            | BindingKind::Walrus
            | BindingKind::ComprehensionTarget
            | BindingKind::MatchCapture
            | BindingKind::Del
            | BindingKind::TypeAlias => self.declared_scope(here, name),
        };
        let branch = self
            .branches
            .iter()
            .rev()
            .find(|(b, _, _)| b.contains_range(range))
            .map(|b| (b.1, b.2));
        let index = self.binds.len();
        self.binds.push(BindRec {
            id,
            kind,
            scope,
            name: name.to_owned(),
            site,
            range,
            value,
            branch,
            nonlocal,
        });
        if !nonlocal {
            self.note(scope, index);
        }
    }

    /// Make binding `index` one of its scope's events for its name, unless it is a declaration.
    fn note(&mut self, scope: usize, index: usize) {
        let b = &self.binds[index];
        // A `global`/`nonlocal` declaration does not make the name local; every other event does.
        if !matches!(
            b.kind,
            BindingKind::Global | BindingKind::Nonlocal | BindingKind::StarImport
        ) {
            let name = b.name.clone();
            self.scopes[scope]
                .names
                .entry(name)
                .or_default()
                .push(index);
        }
    }

    /// The bindings a scope holds for `name`, when it is local there.
    fn local(&self, s: usize, name: &str) -> Option<&Vec<usize>> {
        let scope = &self.scopes[s];
        if scope.globals.contains(name) || scope.nonlocals.contains(name) {
            return None;
        }
        scope.names.get(name)
    }

    fn module_scope(&self) -> usize {
        0
    }

    /// A `nonlocal` name's scope: the nearest enclosing function scope that binds it, through the
    /// scopes that declare it `nonlocal` themselves; the declaring scope when none does.
    fn nonlocal_target(&self, s: usize, name: &str) -> usize {
        let mut cur = self.scopes[s].parent;
        while let Some(p) = cur {
            let ps = &self.scopes[p];
            match ps.kind {
                LexicalScopeKind::Module => break,
                LexicalScopeKind::Class => {}
                LexicalScopeKind::Function
                | LexicalScopeKind::Lambda
                | LexicalScopeKind::Comprehension => {
                    if ps.globals.contains(name) {
                        break;
                    }
                    if !ps.nonlocals.contains(name) && ps.names.contains_key(name) {
                        return p;
                    }
                }
            }
            cur = ps.parent;
        }
        s
    }

    /// The `__class__` cell of class scope `c`, made on first use: a binding of the class.
    fn class_cell(&mut self, c: usize) -> usize {
        if let Some(&i) = self.class_cells.get(&c) {
            return i;
        }
        let (owner, start) = (self.scopes[c].owner, self.scopes[c].span.start());
        let index = self.binds.len();
        self.binds.push(BindRec {
            id: recipe::binding(owner, "__class__"),
            kind: BindingKind::Implicit,
            scope: c,
            name: "__class__".to_owned(),
            site: owner,
            range: TextRange::empty(start),
            value: None,
            branch: None,
            nonlocal: false,
        });
        self.class_cells.insert(c, index);
        index
    }

    /// Resolve every reference, then emit the scope, binding and resolution rows (DESIGN §3.2
    /// `lexical`).
    pub(crate) fn finish(mut self, sink: &mut FactSink) -> LexicalOut {
        // `nonlocal` targets, now that every scope's bindings are known (review F5).
        let waiting: Vec<usize> = (0..self.binds.len())
            .filter(|&i| self.binds[i].nonlocal)
            .collect();
        for i in waiting {
            let target = self.nonlocal_target(self.binds[i].scope, &self.binds[i].name.clone());
            self.binds[i].scope = target;
            self.note(target, i);
        }
        // Each scope's events in source order.
        for s in 0..self.scopes.len() {
            let binds = &self.binds;
            for v in self.scopes[s].names.values_mut() {
                v.sort_by_key(|&i| (binds[i].range.start(), i));
            }
        }
        // `__class__` read in a method: its class's cell.
        let cells: Vec<usize> = self
            .refs
            .iter()
            .filter(|r| r.name == "__class__")
            .filter_map(|r| self.enclosing_class(r.scope))
            .collect();
        for c in cells {
            self.class_cell(c);
        }

        let refs = std::mem::take(&mut self.refs);
        let mut resolutions = Vec::new();
        for r in &refs {
            let (rows, modality) = self.resolve_ref(r);
            for (binding, captured, builtin, reason) in rows {
                resolutions.push(fact_row!(
                    sink,
                    ReferenceResolutions,
                    provenance(modality),
                    ReferenceResolutionsRow {
                        snapshot_id: Id::ZERO,
                        fact_id: Id::ZERO,
                        reference_id: r.id,
                        binding_id: binding,
                        captured,
                        builtin_name: builtin,
                        reason,
                    }
                ));
            }
        }

        for s in &self.scopes {
            let (start, end) = span(s.span);
            let row = fact_row!(
                sink,
                Scopes,
                provenance(Modality::Definite),
                ScopesRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    node_id: s.id,
                    module_node_id: self.module_node_id,
                    kind: s.kind,
                    owner_node_id: s.owner,
                    parent_scope_id: s.parent.map(|p| self.scopes[p].id),
                    start_byte: start,
                    end_byte: end,
                }
            );
            self.out.scopes.push(row);
        }
        // Ordinals: each scope's events in source order.
        let mut order: Vec<usize> = (0..self.binds.len()).collect();
        order.sort_by_key(|&i| (self.binds[i].scope, self.binds[i].range.start(), i));
        let mut next: HashMap<usize, i64> = HashMap::new();
        for i in order {
            let b = &self.binds[i];
            let ordinal = next.entry(b.scope).or_insert(0);
            let (start, end) = span(b.range);
            let row = fact_row!(
                sink,
                Bindings,
                provenance(Modality::Definite),
                BindingsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    node_id: b.id,
                    scope_id: self.scopes[b.scope].id,
                    module_node_id: self.module_node_id,
                    name: b.name.clone(),
                    kind: b.kind,
                    ordinal: *ordinal,
                    site_node_id: b.site,
                    start_byte: start,
                    end_byte: end,
                    value_start_byte: b.value.map(|v| span(v).0),
                    value_end_byte: b.value.map(|v| span(v).1),
                    static_branch: b.branch.map(|x| x.0),
                    static_polarity: b.branch.map(|x| x.1),
                }
            );
            *ordinal += 1;
            self.out.bindings.push(row);
        }
        self.out.resolutions = resolutions;
        self.out
    }

    /// The class scope whose `__class__` cell a read in scope `s` names: the nearest class
    /// enclosing a function scope on the way out.
    fn enclosing_class(&self, s: usize) -> Option<usize> {
        let mut in_function = false;
        let mut cur = Some(s);
        while let Some(p) = cur {
            match self.scopes[p].kind {
                LexicalScopeKind::Class if in_function => return Some(p),
                LexicalScopeKind::Class | LexicalScopeKind::Module => {}
                LexicalScopeKind::Function
                | LexicalScopeKind::Lambda
                | LexicalScopeKind::Comprehension => in_function = true,
            }
            cur = self.scopes[p].parent;
        }
        None
    }

    fn binding_rows(&self, hits: &[usize], captured: bool) -> Vec<Resolved> {
        hits.iter()
            .map(|&i| (Some(self.binds[i].id), captured, None, None))
            .collect()
    }

    /// A reference's resolution rows and their modality.
    fn resolve_ref(&mut self, r: &RefRec) -> (Vec<Resolved>, Modality) {
        if r.name == "__class__"
            && let Some(c) = self.enclosing_class(r.scope)
            && let Some(&cell) = self.class_cells.get(&c)
        {
            return (self.binding_rows(&[cell], true), Modality::Definite);
        }
        let (hits, captured, local) = self.resolve(r.scope, &r.name);
        if hits.is_empty() {
            return self.outside(&r.name);
        }
        let mut rows = self.binding_rows(&hits, captured);
        // A module or class body reads a name it binds only later from outside (review F3).
        let kind = self.scopes[r.scope].kind;
        let first = hits.iter().map(|&i| self.binds[i].range.start()).min();
        if local
            && matches!(kind, LexicalScopeKind::Module | LexicalScopeKind::Class)
            && first.is_some_and(|f| r.at < f)
        {
            let (outer, outer_captured) = match self.scopes[r.scope].parent {
                Some(p) if kind == LexicalScopeKind::Class => self.free(p, &r.name),
                Some(_) | None => (Vec::new(), false),
            };
            if outer.is_empty() {
                // A name nothing outside binds adds no candidate: a module-level loop can read
                // the later binding.
                let (outside, _) = self.outside(&r.name);
                rows.extend(
                    outside
                        .into_iter()
                        .filter(|x| x.3 != Some(BoundaryReason::UnresolvedTarget)),
                );
            } else {
                rows.extend(self.binding_rows(&outer, outer_captured));
            }
        }
        let modality = if rows.len() > 1 {
            Modality::Candidate
        } else {
            Modality::Definite
        };
        (rows, modality)
    }

    /// Past the module: the star imports whose wildcard set holds `name`, then the builtins. A star
    /// import Pyrefly cannot find may bind any name, so it stays a candidate beside the rest.
    fn outside(&mut self, name: &str) -> (Vec<Resolved>, Modality) {
        let mut known = Vec::new();
        let mut unknown = Vec::new();
        let mut stars: Vec<(&usize, &Option<HashSet<String>>)> = self.star_sets.iter().collect();
        stars.sort_by_key(|(i, _)| **i);
        for (&i, set) in stars {
            match set {
                Some(names) if names.contains(name) => known.push(i),
                Some(_) => {}
                None => unknown.push(i),
            }
        }
        let mut rows = self.binding_rows(&known, false);
        if rows.is_empty()
            && let Some(kind) = self.outside.builtins.get(name)
        {
            let variable_like = !matches!(
                kind,
                Some(SymbolKind::Function | SymbolKind::Class | SymbolKind::Method)
            );
            if !variable_like {
                self.out.builtins_used.insert(name.to_owned());
            }
            rows.push((
                None,
                false,
                Some(name.to_owned()),
                variable_like.then_some(BoundaryReason::VariableOrigin),
            ));
        }
        let exact = rows.len() == 1 && unknown.is_empty();
        rows.extend(self.binding_rows(&unknown, false));
        if rows.is_empty() {
            let unresolved = (None, false, None, Some(BoundaryReason::UnresolvedTarget));
            return (vec![unresolved], Modality::Definite);
        }
        (
            rows,
            if exact {
                Modality::Definite
            } else {
                Modality::Candidate
            },
        )
    }

    /// The bindings `name` resolves to from scope `s`, whether they are in an enclosing function
    /// scope (a closure capture), and whether they are `s`'s own.
    fn resolve(&self, s: usize, name: &str) -> (Vec<usize>, bool, bool) {
        let scope = &self.scopes[s];
        if scope.globals.contains(name) {
            return (
                self.local(self.module_scope(), name)
                    .cloned()
                    .unwrap_or_default(),
                false,
                false,
            );
        }
        if !scope.nonlocals.contains(name)
            && let Some(hits) = self.local(s, name)
        {
            return (hits.clone(), false, true);
        }
        let (hits, captured) = match scope.parent {
            Some(p) => self.free(p, name),
            None => (Vec::new(), false),
        };
        (hits, captured, false)
    }

    /// A free name from scope `p` outward: the enclosing function scopes (class scopes skipped),
    /// then the module.
    fn free(&self, p: usize, name: &str) -> (Vec<usize>, bool) {
        let mut cur = Some(p);
        while let Some(p) = cur {
            let ps = &self.scopes[p];
            match ps.kind {
                LexicalScopeKind::Class => {}
                LexicalScopeKind::Module => {
                    return (self.local(p, name).cloned().unwrap_or_default(), false);
                }
                LexicalScopeKind::Function
                | LexicalScopeKind::Lambda
                | LexicalScopeKind::Comprehension => {
                    if ps.globals.contains(name) {
                        return (
                            self.local(self.module_scope(), name)
                                .cloned()
                                .unwrap_or_default(),
                            false,
                        );
                    }
                    if let Some(hits) = self.local(p, name) {
                        return (hits.clone(), true);
                    }
                }
            }
            cur = ps.parent;
        }
        (Vec::new(), false)
    }
}

#[cfg(test)]
mod tests {
    use pyrefly_python::ast::Ast;
    use pyrefly_python::sys_info::{PythonPlatform, PythonVersion, SysInfo};
    use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, walk_stmt};
    use ruff_python_ast::{PySourceType, Stmt, StmtIf};

    use super::clause_marks;

    /// Every `if` statement of a module, nested ones included.
    #[derive(Default)]
    struct Ifs(Vec<StmtIf>);

    impl<'a> SourceOrderVisitor<'a> for Ifs {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::If(i) = stmt {
                self.0.push(i.clone());
            }
            walk_stmt(self, stmt);
        }
    }

    /// Differential (H1 C1): on every `if` of the `lexical_shapes` fixture, the clauses our marks
    /// keep are exactly those Pyrefly's own `pruned_if_branches` analyzes, under the fixture
    /// compile's configuration (3.14, Linux).
    #[test]
    fn static_marks_agree_with_pyrefly_pruning() {
        let sys = SysInfo::new(PythonVersion::new(3, 14, 0), PythonPlatform::new("linux"));
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/python/lexical_shapes/lex");
        let mut checked = 0;
        for module in ["__init__.py", "branches.py", "flow.py", "helpers.py"] {
            let text = std::fs::read_to_string(dir.join(module)).unwrap();
            let (ast, errors, _) = Ast::parse(&text, PySourceType::Python);
            assert!(errors.is_empty(), "{module}: {errors:?}");
            let mut ifs = Ifs::default();
            for stmt in &ast.body {
                ifs.visit_stmt(stmt);
            }
            for i in &ifs.0 {
                let bodies: Vec<*const Stmt> =
                    Ast::if_branches(i).map(|(_, body)| body.as_ptr()).collect();
                let ours: Vec<*const Stmt> = clause_marks(&sys, i)
                    .iter()
                    .zip(&bodies)
                    .filter(|(mark, _)| !matches!(mark, Some((_, false))))
                    .map(|(_, body)| *body)
                    .collect();
                let pyrefly: Vec<*const Stmt> = sys
                    .pruned_if_branches(i)
                    .map(|(_, body)| body.as_ptr())
                    .collect();
                assert_eq!(ours, pyrefly, "{module}: the `if` at {:?}", i.range);
                checked += 1;
            }
        }
        assert!(checked >= 8, "only {checked} `if` statements checked");
    }
}
