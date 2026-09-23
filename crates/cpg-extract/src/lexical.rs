//! Our lexical recognizer (CPG slice C3, DESIGN §3.2 `lexical`; surface `lctx-lexical`,
//! extraction mode `recognizer`): scopes, binding events with shadowed history, name references,
//! and their resolution under full Python scoping. It runs inside the Ruff walk, so a name's
//! syntax id is the walk's own.
//!
//! Scoping, as Python defines it: a name bound anywhere in a scope is local to it unless declared
//! `global` or `nonlocal`; a free name resolves in the nearest enclosing function scope that binds
//! it, skipping class scopes, then the module, then the builtins. A function's defaults,
//! annotations and decorators, and a class's bases, evaluate in the enclosing scope; a
//! comprehension's first iterable too. A walrus binds in the nearest non-comprehension scope.
//! Resolution is flow-insensitive: every binding of the name in the resolving scope is a
//! candidate, with its ordinal, so an order-aware consumer (Pass B's binding rule, §4.2.4) can
//! choose. Branches Pyrefly decides statically are kept and marked.

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
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::facts::{FactSink, Provenance, Surface, fact_row};
use crate::walk::span;

/// One resolution row: the binding, whether captured, the builtin, the reason.
type Resolved = (Option<Id>, bool, Option<String>, Option<BoundaryReason>);

/// Pyrefly's builtins: each exported name and its symbol kind.
pub(crate) type Builtins = HashMap<String, Option<SymbolKind>>;

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
    ordinal: i64,
}

struct BindRec {
    id: Id,
    kind: BindingKind,
}

struct RefRec {
    id: Id,
    scope: usize,
    name: String,
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
    builtins: &'b Builtins,
    scopes: Vec<ScopeRec>,
    open: Vec<usize>,
    stores: Vec<StoreCtx>,
    branches: Vec<(TextRange, StaticBranch, bool)>,
    pushed: Vec<Pushed>,
    binds: Vec<BindRec>,
    refs: Vec<RefRec>,
    has_star: bool,
    /// Inside a `from m import …` statement: its aliases bind as `from_import`.
    in_from_import: bool,
    out: LexicalOut,
}

/// A statically decided test: `TYPE_CHECKING`, `sys.version_info …`, `sys.platform …`.
fn static_test(test: &Expr, text: &str) -> Option<StaticBranch> {
    let src = text.get(test.start().to_usize()..test.end().to_usize())?;
    if src == "TYPE_CHECKING" || src.ends_with(".TYPE_CHECKING") {
        Some(StaticBranch::TypeChecking)
    } else if src.contains("version_info") {
        Some(StaticBranch::VersionInfo)
    } else if src.contains("sys.platform") {
        Some(StaticBranch::Platform)
    } else {
        None
    }
}

fn suite(body: &[Stmt]) -> Option<TextRange> {
    Some(TextRange::new(body.first()?.start(), body.last()?.end()))
}

impl<'b> Lexical<'b> {
    pub(crate) fn new(module_node_id: Id, module_span: TextRange, builtins: &'b Builtins) -> Self {
        let mut this = Self {
            module_node_id,
            builtins,
            scopes: Vec::new(),
            open: Vec::new(),
            stores: Vec::new(),
            branches: Vec::new(),
            pushed: Vec::new(),
            binds: Vec::new(),
            refs: Vec::new(),
            has_star: false,
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
        let parent = self.open.last().copied();
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
            ordinal: 0,
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
        text: &str,
        sink: &mut FactSink,
    ) {
        use ruff_python_ast::AnyNodeRef as N;
        let mut pushed = Pushed::default();
        let r = node.range();
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
                    sink,
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
                        self.bind(
                            s,
                            name,
                            BindingKind::TypeParam,
                            decl,
                            name.range(),
                            None,
                            sink,
                        );
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
                    sink,
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
                        sink,
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
            N::StmtIf(i) => {
                if let Some(kind) = static_test(&i.test, text) {
                    if let Some(b) = suite(&i.body) {
                        self.branches.push((b, kind, true));
                        pushed.branches += 1;
                    }
                    for c in &i.elif_else_clauses {
                        self.branches.push((c.range(), kind, false));
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
                        sink,
                    );
                }
            }
            N::StmtImport(_) | N::StmtImportFrom(_) => {}
            N::Alias(a) => {
                let here = self.scope_at(r.start());
                if a.name.as_str() == "*" {
                    self.has_star = true;
                    self.bind(
                        here,
                        "*",
                        BindingKind::StarImport,
                        syntax_id,
                        a.range(),
                        None,
                        sink,
                    );
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
                    self.bind(here, &name, kind, syntax_id, range, None, sink);
                }
            }
            N::StmtGlobal(g) => {
                let here = self.scope_at(r.start());
                for n in &g.names {
                    self.scopes[here].globals.insert(n.to_string());
                    self.bind(
                        here,
                        n,
                        BindingKind::Global,
                        syntax_id,
                        n.range(),
                        None,
                        sink,
                    );
                }
            }
            N::StmtNonlocal(g) => {
                let here = self.scope_at(r.start());
                for n in &g.names {
                    self.scopes[here].nonlocals.insert(n.to_string());
                    self.bind(
                        here,
                        n,
                        BindingKind::Nonlocal,
                        syntax_id,
                        n.range(),
                        None,
                        sink,
                    );
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
                        sink,
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
                        sink,
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
                        sink,
                    );
                }
            }
            N::ExprName(n) => {
                let here = self.scope_at(r.start());
                match n.ctx {
                    // A name in an annotation is a type reference: the `types` family's (C4).
                    ruff_python_ast::ExprContext::Load if in_annotation => {}
                    ruff_python_ast::ExprContext::Load => {
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
                                name: n.id.to_string(),
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
                            name: n.id.to_string(),
                        });
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
                        let target = self.declared_scope(target, &n.id);
                        self.bind(target, &n.id, kind, syntax_id, r, value, sink);
                    }
                    ruff_python_ast::ExprContext::Del => {
                        let target = self.declared_scope(here, &n.id);
                        self.bind(target, &n.id, BindingKind::Del, syntax_id, r, None, sink);
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

    /// Where a store of `name` in scope `s` binds: the module under `global`, the nearest
    /// enclosing function scope that holds the name under `nonlocal`, else `s` itself.
    fn declared_scope(&self, s: usize, name: &str) -> usize {
        let scope = &self.scopes[s];
        if scope.globals.contains(name) {
            return self.module_scope();
        }
        if scope.nonlocals.contains(name) {
            let mut cur = scope.parent;
            while let Some(p) = cur {
                let ps = &self.scopes[p];
                if ps.kind != LexicalScopeKind::Class && ps.names.contains_key(name) {
                    return p;
                }
                cur = ps.parent;
            }
        }
        s
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

    #[allow(
        clippy::too_many_arguments,
        reason = "a registry row or binding event is these fields"
    )]
    fn bind(
        &mut self,
        scope: usize,
        name: &str,
        kind: BindingKind,
        site: Id,
        range: TextRange,
        value: Option<TextRange>,
        sink: &mut FactSink,
    ) {
        let id = recipe::binding(site, name);
        let ordinal = self.scopes[scope].ordinal;
        self.scopes[scope].ordinal += 1;
        let branch = self
            .branches
            .iter()
            .rev()
            .find(|(b, _, _)| b.contains_range(range));
        let (start, end) = span(range);
        let row = fact_row!(
            sink,
            Bindings,
            provenance(Modality::Definite),
            BindingsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id: id,
                scope_id: self.scopes[scope].id,
                module_node_id: self.module_node_id,
                name: name.to_owned(),
                kind,
                ordinal,
                site_node_id: site,
                start_byte: start,
                end_byte: end,
                value_start_byte: value.map(|v| span(v).0),
                value_end_byte: value.map(|v| span(v).1),
                static_branch: branch.map(|b| b.1),
                static_polarity: branch.map(|b| b.2),
            }
        );
        self.out.bindings.push(row);
        let index = self.binds.len();
        self.binds.push(BindRec { id, kind });
        // A `global`/`nonlocal` declaration does not make the name local; every other event does.
        if !matches!(
            kind,
            BindingKind::Global | BindingKind::Nonlocal | BindingKind::StarImport
        ) {
            self.scopes[scope]
                .names
                .entry(name.to_owned())
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

    /// Resolve every reference and emit the scope, resolution rows (DESIGN §3.2 `lexical`).
    pub(crate) fn finish(mut self, sink: &mut FactSink) -> LexicalOut {
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
        let star: Vec<usize> = self
            .binds
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == BindingKind::StarImport)
            .map(|(i, _)| i)
            .collect();
        let refs = std::mem::take(&mut self.refs);
        for r in &refs {
            let (hits, captured) = self.resolve(r.scope, &r.name);
            let rows: Vec<Resolved> = if !hits.is_empty() {
                hits.iter()
                    .map(|&i| (Some(self.binds[i].id), captured, None, None))
                    .collect()
            } else if let Some(kind) = self.builtins.get(&r.name) {
                let variable_like = !matches!(
                    kind,
                    Some(SymbolKind::Function | SymbolKind::Class | SymbolKind::Method)
                );
                if !variable_like {
                    self.out.builtins_used.insert(r.name.clone());
                }
                vec![(
                    None,
                    false,
                    Some(r.name.clone()),
                    variable_like.then_some(BoundaryReason::VariableOrigin),
                )]
            } else if !star.is_empty() {
                star.iter()
                    .map(|&i| (Some(self.binds[i].id), false, None, None))
                    .collect()
            } else {
                vec![(None, false, None, Some(BoundaryReason::UnresolvedTarget))]
            };
            let modality = if rows.len() > 1 {
                Modality::Candidate
            } else {
                Modality::Definite
            };
            for (binding, captured, builtin, reason) in rows {
                let row = fact_row!(
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
                );
                self.out.resolutions.push(row);
            }
        }
        self.out
    }

    /// The bindings `name` resolves to from scope `s`, and whether they are in an enclosing
    /// function scope (a closure capture).
    fn resolve(&self, s: usize, name: &str) -> (Vec<usize>, bool) {
        let scope = &self.scopes[s];
        if scope.globals.contains(name) {
            return (
                self.local(self.module_scope(), name)
                    .cloned()
                    .unwrap_or_default(),
                false,
            );
        }
        if !scope.nonlocals.contains(name)
            && let Some(hits) = self.local(s, name)
        {
            return (hits.clone(), false);
        }
        // Free: the enclosing function scopes (class scopes skipped), then the module.
        let mut cur = scope.parent;
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
