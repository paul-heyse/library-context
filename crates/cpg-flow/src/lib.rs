//! The flow provider (ADR-0022 §The flow provider; ADR-0012 amendment): ty's semantic index over
//! each release module, read as our runtime flow facts.
//!
//! For each module, from the same text Pyrefly parsed:
//! - **uses**: every place load ty records (a name, an attribute chain, a literal subscript) and
//!   an augmented assignment's target; a `del` target is an unbinding, as in our lexical model;
//! - **definitions**: every binding ty records, each with its place, kind, target span and value;
//! - **reaching definitions**: per use, the definitions that reach it, each under the condition of
//!   its reachability at the use (a path condition from the scope's entry). ty's loop-header
//!   definitions are replaced by the loop-body bindings they stand for (`loop_carried`);
//! - **value sources**: for each value a definition, call argument, `return` or `yield` takes,
//!   the uses it reads, each **identity** (the value passes unchanged: the expression itself, a
//!   conditional-expression branch, a boolean operand, a walrus) or **derived** (anything
//!   computed), under the condition inside the expression that selects it, nested conditional
//!   expressions and boolean operators included. A derived use inside a call (its callee, receiver
//!   or an argument) is **through a call**: it reaches the value only if the callee's result
//!   carries it, which is a summary's question (ADR-0022 §Verdicts);
//! - **regions**: every statement's reachability condition.
//!
//! **The runtime override.** ty decides `TYPE_CHECKING` as true while indexing. Before ty parses a
//! module, every `TYPE_CHECKING` word is renamed to [`SENTINEL`], of the same length, so no byte
//! range moves; the runtime view then decides the predicate as false ([`predicate`]).
//!
//! Nothing but spans, place text and our [`Condition`]s leaves this crate.

mod db;
mod predicate;

use std::collections::{BTreeSet, HashMap, HashSet};

pub use cpg_schema::codebook::{BindingKind, LexicalScopeKind};
use cpg_schema::condition::{Atom, EvaluationIdentity};
pub use cpg_schema::condition_kernel::BoundedCondition as Condition;
use cpg_schema::id::IdHasher;
use ruff_db::files::system_path_to_file;
use ruff_db::parsed::{ParsedModuleRef, parsed_module};
use ruff_db::system::DbWithWritableSystem;
use ruff_python_ast_ty::token::TokenKind;
use ruff_python_ast_ty::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
use ruff_python_ast_ty::{self as ast, AnyNodeRef, Expr, ExprContext, PySourceType, Stmt};
use ruff_text_size_ty::{Ranged, TextRange};
use serde::{Deserialize, Serialize};
use ty_python_core::ast_ids::HasScopedUseId;
use ty_python_core::definition::{Definition, DefinitionKind, DefinitionState};
use ty_python_core::place::PlaceExpr;
use ty_python_core::predicate::PredicateNode;
use ty_python_core::program::{Program, ProgramSettings};
use ty_python_core::reachability_constraints::ScopedReachabilityConstraintId;
use ty_python_core::scope::{NodeWithScopeKind, NodeWithScopeRef};
use ty_python_core::{FileScopeId, ProgramFile, UseDefMap, semantic_index};

use crate::db::FlowDb;
use crate::predicate::{Translator, diagram, runtime_in_diagram, synthetic_identity};

/// The provider, as `producers.revision` names it (ADR-0022 §Identity).
pub const PROVIDER: &str = "ty_python_core 0.0.14 (ruff 0.0.14, salsa 0.28.2)";
/// The word ty decides at index time, and the same-length name it is renamed to.
pub const WORD: &[u8] = b"TYPE_CHECKING";
pub const SENTINEL: &[u8] = b"TYPE_CHECKIN_";

/// The analyzed context the runtime view reads (§4.0).
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub python_version: (u32, u32, u32),
    pub platform: String,
}

/// One module to index: its path relative to the release, and the text Pyrefly parsed.
#[derive(Debug, Clone)]
pub struct Input {
    pub path: String,
    pub text: String,
    /// Name-load spans whose lexical resolution proves a runtime-special binding.
    pub runtime: RuntimeBindings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeBindings {
    pub checking_names: BTreeSet<Span>,
    pub typing_modules: BTreeSet<Span>,
    pub sys_modules: BTreeSet<Span>,
    pub os_modules: BTreeSet<Span>,
}

/// A byte span of the module's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl From<TextRange> for Span {
    fn from(r: TextRange) -> Self {
        Span {
            start: r.start().into(),
            end: r.end().into(),
        }
    }
}

/// The scope a use or definition sits in: its kind and the span that names it (a function's or
/// class's name, a lambda's or comprehension's expression; `None` for the module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    pub kind: LexicalScopeKind,
    pub name: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct Use {
    pub scope: Scope,
    pub place: String,
    pub span: Span,
    /// Inside an annotation: `references` does not model these as reads (the parity residue).
    pub annotation: bool,
}

#[derive(Debug, Clone)]
pub struct Def {
    pub scope: Scope,
    pub place: String,
    pub kind: BindingKind,
    /// The bound target: a name, an attribute chain, a subscript; for an import alias the bound
    /// name (normalized, ADR-0022).
    pub target: Span,
    /// The value it takes: an assignment's right-hand side, a walrus's value, an augmented
    /// assignment's operand, a `for`/comprehension iterable, a `with` context expression.
    pub value: Option<Span>,
}

/// A definition reaching a use (`def` is `None` where the place may be unbound on some path).
#[derive(Debug, Clone)]
pub struct Reach {
    pub use_ix: u32,
    pub def_ix: Option<u32>,
    pub condition: Condition,
    /// Reached around a loop's back edge (through ty's loop header): the condition is the use's
    /// side only.
    pub loop_carried: bool,
}

/// What kind of value a value source's sink is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sink {
    /// A definition's value (see [`Def::value`]).
    Definition,
    /// A call argument's value (positional, keyword, starred or double-starred).
    Argument,
    Return,
    Yield,
    /// A `raise`'s exception, or its `from` cause.
    Raise,
}

#[derive(Debug, Clone)]
pub struct ValueSource {
    pub sink: Sink,
    pub span: Span,
    pub use_ix: u32,
    /// The value passes unchanged (see the crate docs); `false`: it is computed from the use.
    pub identity: bool,
    /// The use is inside a call within the value (its callee, receiver or an argument).
    pub through_call: bool,
    pub condition: Condition,
}

#[derive(Debug, Clone)]
pub struct Region {
    pub scope: Scope,
    pub span: Span,
    pub condition: Condition,
}

/// A test ty records as a predicate (an `if`/`elif`/`while`/`assert` test, a conditional
/// expression's or boolean operand's, a `match` subject with its pattern): its span and its
/// condition. The uses inside the span are what the test reads (the Stage 2 end review's R8).
#[derive(Debug, Clone)]
pub struct Test {
    pub scope: Scope,
    pub span: Span,
    pub condition: Condition,
}

/// One evaluation atom of one provider predicate, before the test-span projection merges rows.
/// This is the source identity for Stage 3's attributed type and value-link proofs.
#[derive(Debug, Clone)]
pub struct TestLeaf {
    pub scope: Scope,
    pub predicate_key: String,
    pub test_span: Span,
    pub condition: Condition,
    pub atom: String,
    pub leaf_span: Span,
}

/// An attribute load by name on any receiver (a place or not: `get_server()._worker`), or a
/// `getattr`/`hasattr` with a literal name. Outside annotations. The field and global premises
/// count these (the Stage 2 end review's R7).
#[derive(Debug, Clone)]
pub struct AttributeLoad {
    pub span: Span,
    pub name: String,
}

/// One module's flow facts, or why there are none.
#[derive(Debug, Clone, Default)]
pub struct ModuleFlow {
    pub path: String,
    pub uses: Vec<Use>,
    pub defs: Vec<Def>,
    pub reaching: Vec<Reach>,
    pub values: Vec<ValueSource>,
    pub regions: Vec<Region>,
    pub tests: Vec<Test>,
    pub test_leaves: Vec<TestLeaf>,
    pub attribute_loads: Vec<AttributeLoad>,
    /// `TYPE_CHECKING` words renamed.
    pub renamed: u32,
    /// Syntax errors ty's parser recovered from: the facts come from a recovered tree, as every
    /// other family's do.
    pub syntax_errors: usize,
    pub error: Option<String>,
    /// Counted branches discarded because their translated condition is `false`.
    pub skips: SkipCounts,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkipCounts {
    pub reaching_ty_false: u32,
    pub reaching_runtime_view: u32,
    pub reaching_stable_contradiction: u32,
    /// These count candidate value-source branches, before expansion into uses.
    pub values_runtime_view: u32,
    pub values_stable_contradiction: u32,
}

#[derive(Clone, Copy)]
enum SkipCause {
    Ty,
    Runtime,
    Stable,
}

impl SkipCounts {
    fn reach(&mut self, cause: SkipCause) {
        match cause {
            SkipCause::Ty => self.reaching_ty_false += 1,
            SkipCause::Runtime => self.reaching_runtime_view += 1,
            SkipCause::Stable => self.reaching_stable_contradiction += 1,
        }
    }

    fn value(&mut self, cause: SkipCause) {
        match cause {
            SkipCause::Runtime => self.values_runtime_view += 1,
            SkipCause::Ty | SkipCause::Stable => self.values_stable_contradiction += 1,
        }
    }
}

/// Rename every `TYPE_CHECKING` name token to [`SENTINEL`] (ADR-0022 §The flow provider).
/// Tokens come from ruff 0.0.14's lexer over the module, so strings and comments keep their text;
/// names inside f-string replacement fields are names. A module that already uses the sentinel as
/// a name is refused.
pub fn rename(text: &str) -> Result<(String, u32), String> {
    let parsed = ruff_python_parser_ty::parse_unchecked_source(text, PySourceType::Python);
    let sentinel = std::str::from_utf8(SENTINEL).expect("ASCII");
    let word = std::str::from_utf8(WORD).expect("ASCII");
    let mut out = text.as_bytes().to_vec();
    let mut n = 0;
    for t in parsed.tokens() {
        if t.kind() != TokenKind::Name {
            continue;
        }
        let r = t.range();
        let (start, end) = (usize::from(r.start()), usize::from(r.end()));
        match &text[start..end] {
            s if s == sentinel => {
                return Err(format!("the module already names `{sentinel}`"));
            }
            s if s == word => {
                out[start..end].copy_from_slice(SENTINEL);
                n += 1;
            }
            _ => {}
        }
    }
    Ok((
        String::from_utf8(out).expect("an ASCII name swapped for an ASCII name"),
        n,
    ))
}

/// Index every module. A module ty cannot index gets an `error` and no facts; the others are
/// unaffected.
pub fn index(inputs: &[Input], context: &RuntimeContext) -> Vec<ModuleFlow> {
    let mut db = FlowDb::new();
    let mut prepared = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        // A directory per module keeps paths distinct whatever the release's layout.
        let path = format!("/flow/{i}/{}", input.path);
        prepared.push(rename(&input.text).and_then(|(text, n)| {
            db.write_file(&path, &text)
                .map(|()| (path, n))
                .map_err(|e| e.to_string())
        }));
    }
    let vendored = ty_vendored::file_system().clone();
    let settings = ProgramSettings::empty(&vendored);
    let program = Program::from_settings(&db, &settings);
    // A panic aborts the extraction, as every analyzer's does (ADR-0012 §Panics).
    inputs
        .iter()
        .zip(prepared)
        .map(|(input, prepared)| {
            let result = prepared.and_then(|(path, n)| {
                let module_key = IdHasher::new("flow-evaluation-module")
                    .str(&input.path)
                    .str(&input.text)
                    .finish_id()
                    .hex();
                module(
                    &db,
                    program,
                    &path,
                    &input.text,
                    &module_key,
                    &input.runtime,
                    context,
                )
                .map(|flow| (flow, n))
            });
            match result {
                Ok((flow, n)) => ModuleFlow {
                    path: input.path.clone(),
                    renamed: n,
                    ..flow
                },
                Err(e) => ModuleFlow {
                    path: input.path.clone(),
                    error: Some(e),
                    ..ModuleFlow::default()
                },
            }
        })
        .collect()
}

fn module(
    db: &FlowDb,
    program: Program<'_>,
    path: &str,
    original: &str,
    module_key: &str,
    runtime: &RuntimeBindings,
    context: &RuntimeContext,
) -> Result<ModuleFlow, String> {
    let file = system_path_to_file(db, path).map_err(|e| e.to_string())?;
    let pf = ProgramFile::new(db, file, program);
    let parsed = parsed_module(db, pf.python_file(db)).load(db);
    let syntax_errors = parsed.errors().len();
    let index = semantic_index(db, pf);
    let t = Translator {
        db,
        module: &parsed,
        original,
        context,
        runtime,
        module_key: module_key.to_owned(),
    };
    let mut w = Walk {
        db,
        pf,
        index,
        t: &t,
        parsed: &parsed,
        flow: ModuleFlow::default(),
        use_of: HashMap::new(),
        def_of: HashMap::new(),
        diagrams: HashMap::new(),
        runtime_diagrams: HashMap::new(),
        regions: HashMap::new(),
        in_annotation: false,
    };
    // Definitions first, scope by scope, so reaching rows can name them. A class's or an alias's
    // PEP 695 type parameters are not modelled by our lexical recognizer (DESIGN §3.2), so they
    // are not definitions here either.
    for scope in index.scope_ids() {
        let fid = scope.file_scope_id(db);
        if matches!(
            index.scope(fid).node(),
            NodeWithScopeKind::ClassTypeParameters(_)
                | NodeWithScopeKind::TypeAliasTypeParameters(_)
        ) {
            continue;
        }
        let Some(sc) = w.scope(fid) else { continue };
        let map = index.use_def_map(fid);
        for (_, d, _) in map.definitions_with_usage() {
            w.def(sc, d);
        }
    }
    let mut v = Visitor {
        w: &mut w,
        aug: HashSet::new(),
        stack: vec![FileScopeId::global()],
    };
    for stmt in parsed.suite() {
        v.visit_stmt(stmt);
    }
    // Preserve every provider predicate and atom before the older span-only test projection.
    let mut seen: HashSet<Span> = HashSet::new();
    for scope in index.scope_ids() {
        let fid = scope.file_scope_id(db);
        let Some(sc) = w.scope(fid) else { continue };
        for (predicate_id, p) in index.use_def_map(fid).predicates().iter_enumerated() {
            let (span, condition) = match &p.node {
                PredicateNode::Expression(x)
                | PredicateNode::Condition(x)
                | PredicateNode::ChainedComparisonCondition(x) => {
                    let e = x.node_ref(db).node(&parsed);
                    (Span::from(e.range()), t.test(e))
                }
                PredicateNode::Pattern(pattern) => {
                    let subject = pattern.subject(db).node_ref(db).node(&parsed);
                    let evaluation = synthetic_identity(module_key, fid, predicate_id);
                    let mut c = t.pattern(subject, pattern.kind(db), &evaluation);
                    if let Some(guard) = pattern.guard(db) {
                        c = c.and(&t.test(guard.node_ref(db).node(&parsed)));
                    }
                    (Span::from(subject.range()), c)
                }
                _ => continue,
            };
            let predicate_key = match synthetic_identity(module_key, fid, predicate_id) {
                EvaluationIdentity::Synthetic { predicate, .. } => predicate,
                _ => unreachable!("the provider predicate identity is synthetic"),
            };
            if let Ok(diagram) = condition.diagram() {
                for encoded in diagram.support() {
                    let atom = Atom::parse_encoded(encoded)
                        .expect("the producer's atom encoding round-trips");
                    let Atom::Evaluated { identity, .. } = atom else {
                        continue;
                    };
                    // The path condition also contains earlier predicates. A leaf row belongs
                    // to this predicate only when its evaluation lies inside this test, or the
                    // synthetic predicate identity itself is this provider predicate.
                    let leaf_span = match identity {
                        EvaluationIdentity::Site { start, end, .. }
                            if start >= span.start && end <= span.end =>
                        {
                            Span { start, end }
                        }
                        EvaluationIdentity::Synthetic { predicate, .. }
                            if predicate == predicate_key =>
                        {
                            span
                        }
                        _ => continue,
                    };
                    w.flow.test_leaves.push(TestLeaf {
                        scope: sc,
                        predicate_key: predicate_key.clone(),
                        test_span: span,
                        condition: condition.clone(),
                        atom: encoded.clone(),
                        leaf_span,
                    });
                }
            }
            if seen.insert(span) {
                w.flow.tests.push(Test {
                    scope: sc,
                    span,
                    condition,
                });
            }
        }
    }
    w.flow.tests.sort_by_key(|t| (t.span.start, t.span.end));
    w.flow.test_leaves.sort_by(|a, b| {
        (a.test_span, &a.predicate_key, &a.atom).cmp(&(b.test_span, &b.predicate_key, &b.atom))
    });
    Ok(ModuleFlow {
        syntax_errors,
        ..w.flow
    })
}

struct Walk<'a, 'db> {
    db: &'db FlowDb,
    pf: ProgramFile<'db>,
    index: &'a ty_python_core::SemanticIndex<'db>,
    t: &'a Translator<'a>,
    parsed: &'a ParsedModuleRef,
    flow: ModuleFlow,
    use_of: HashMap<Span, u32>,
    def_of: HashMap<Definition<'db>, Option<u32>>,
    diagrams: HashMap<FileScopeId, HashMap<ScopedReachabilityConstraintId, Condition>>,
    runtime_diagrams: HashMap<FileScopeId, HashMap<ScopedReachabilityConstraintId, bool>>,
    /// Per scope, its range-reachability entries in recording order.
    regions: HashMap<FileScopeId, Vec<(TextRange, Condition)>>,
    /// The visitor is inside an annotation.
    in_annotation: bool,
}

impl<'db> Walk<'_, 'db> {
    /// Our scope for ty's, or `None` for the type-parameter and type-alias scopes (annotation
    /// residue).
    fn scope(&self, fid: FileScopeId) -> Option<Scope> {
        let node = self.index.scope(fid).node();
        let (kind, name) = match node {
            NodeWithScopeKind::Module => (LexicalScopeKind::Module, None),
            NodeWithScopeKind::Class(c) => (
                LexicalScopeKind::Class,
                Some(c.node(self.parsed).name.range()),
            ),
            NodeWithScopeKind::Function(f) => (
                LexicalScopeKind::Function,
                Some(f.node(self.parsed).name.range()),
            ),
            NodeWithScopeKind::Lambda(l) => {
                (LexicalScopeKind::Lambda, Some(l.node(self.parsed).range()))
            }
            NodeWithScopeKind::ListComprehension(c) => (
                LexicalScopeKind::Comprehension,
                Some(c.node(self.parsed).range()),
            ),
            NodeWithScopeKind::SetComprehension(c) => (
                LexicalScopeKind::Comprehension,
                Some(c.node(self.parsed).range()),
            ),
            NodeWithScopeKind::DictComprehension(c) => (
                LexicalScopeKind::Comprehension,
                Some(c.node(self.parsed).range()),
            ),
            NodeWithScopeKind::GeneratorExpression(c) => (
                LexicalScopeKind::Comprehension,
                Some(c.node(self.parsed).range()),
            ),
            // PEP 695 type parameters and alias values: attributed to the enclosing scope, as
            // our lexical recognizer does (it does not model annotation scopes), and read lazily.
            NodeWithScopeKind::ClassTypeParameters(_)
            | NodeWithScopeKind::FunctionTypeParameters(_)
            | NodeWithScopeKind::TypeAliasTypeParameters(_)
            | NodeWithScopeKind::TypeAlias(_) => {
                return self.index.parent_scope_id(fid).and_then(|p| self.scope(p));
            }
        };
        Some(Scope {
            kind,
            name: name.map(Span::from),
        })
    }

    /// A PEP 695 type-parameter or alias scope: its reads are evaluated lazily, like annotations.
    fn lazy(&self, fid: FileScopeId) -> bool {
        matches!(
            self.index.scope(fid).node(),
            NodeWithScopeKind::ClassTypeParameters(_)
                | NodeWithScopeKind::FunctionTypeParameters(_)
                | NodeWithScopeKind::TypeAliasTypeParameters(_)
                | NodeWithScopeKind::TypeAlias(_)
        )
    }

    /// Record a definition once; loop headers and nested-binding markers are not definitions of
    /// ours (`None`).
    fn def(&mut self, scope: Scope, d: Definition<'db>) -> Option<u32> {
        if let Some(ix) = self.def_of.get(&d) {
            return *ix;
        }
        if matches!(
            self.index.scope(d.file_scope(self.db)).node(),
            NodeWithScopeKind::ClassTypeParameters(_)
                | NodeWithScopeKind::TypeAliasTypeParameters(_)
        ) {
            // Not modelled by our lexical recognizer (see `module`).
            self.def_of.insert(d, None);
            return None;
        }
        let m = self.parsed;
        let kind = d.kind(self.db);
        let (binding, value): (Option<BindingKind>, Option<TextRange>) = match kind {
            DefinitionKind::Import(_) => (Some(BindingKind::Import), None),
            DefinitionKind::ImportFrom(_) | DefinitionKind::ImportFromSubmodule(_) => {
                (Some(BindingKind::FromImport), None)
            }
            DefinitionKind::StarImport(_) => (Some(BindingKind::StarImport), None),
            DefinitionKind::Function(_) => (Some(BindingKind::FunctionDef), None),
            DefinitionKind::Class(_) => (Some(BindingKind::ClassDef), None),
            DefinitionKind::TypeAlias(_) => (Some(BindingKind::TypeAlias), None),
            DefinitionKind::NamedExpression(n) => {
                (Some(BindingKind::Walrus), Some(n.node(m).value.range()))
            }
            DefinitionKind::Assignment(a) => {
                (Some(BindingKind::Assignment), Some(a.value(m).range()))
            }
            DefinitionKind::AnnotatedAssignment(_) => match kind.value(m) {
                Some(v) => (Some(BindingKind::Assignment), Some(v.range())),
                None => (Some(BindingKind::AnnotationOnly), None),
            },
            DefinitionKind::AugmentedAssignment(a) => (
                Some(BindingKind::AugAssignment),
                Some(a.node(m).value.range()),
            ),
            DefinitionKind::DictKeyAssignment(k) => {
                (Some(BindingKind::Assignment), Some(k.value(m).range()))
            }
            DefinitionKind::For(f) => (Some(BindingKind::ForTarget), Some(f.iterable(m).range())),
            DefinitionKind::Comprehension(c) => (
                Some(BindingKind::ComprehensionTarget),
                Some(c.iterable(m).range()),
            ),
            DefinitionKind::Parameter(_) | DefinitionKind::LambdaParameter(_) => {
                (Some(BindingKind::Parameter), None)
            }
            DefinitionKind::WithItem(w) => (
                Some(BindingKind::WithTarget),
                Some(w.context_expr(m).range()),
            ),
            DefinitionKind::MatchPattern(_) => (Some(BindingKind::MatchCapture), None),
            DefinitionKind::ExceptHandler(_) => (Some(BindingKind::ExceptHandler), None),
            DefinitionKind::TypeVar(_)
            | DefinitionKind::ParamSpec(_)
            | DefinitionKind::TypeVarTuple(_) => (Some(BindingKind::TypeParam), None),
            DefinitionKind::LoopHeader(_) | DefinitionKind::NestedBindings(_) => (None, None),
        };
        let Some(binding) = binding else {
            self.def_of.insert(d, None);
            return None;
        };
        let target = match kind {
            // The bound name, where our bindings put it (ADR-0022 normalization).
            DefinitionKind::Import(i) => {
                // `import a.b` binds `a`, and our bindings place it on the whole dotted name.
                let alias = i.alias(m);
                alias
                    .asname
                    .as_ref()
                    .map_or(alias.name.range(), Ranged::range)
            }
            DefinitionKind::ImportFrom(i) => {
                let alias = i.alias(m);
                alias
                    .asname
                    .as_ref()
                    .map_or(alias.name.range(), Ranged::range)
            }
            _ => kind.target_range(m),
        };
        let place_id = d.place(self.db);
        let fid = d.file_scope(self.db);
        let place = self.index.place_table(fid).place(place_id).to_string();
        let place = self.unsentinel(&place, target);
        let ix = self.flow.defs.len() as u32;
        self.flow.defs.push(Def {
            scope,
            place,
            kind: binding,
            target: target.into(),
            value: value.map(Span::from),
        });
        self.def_of.insert(d, Some(ix));
        Some(ix)
    }

    /// A place spelled with the sentinel, spelled as written.
    fn unsentinel(&self, place: &str, span: TextRange) -> String {
        if place.contains(std::str::from_utf8(SENTINEL).expect("ASCII")) {
            self.t.original[usize::from(span.start())..usize::from(span.end())]
                .split_whitespace()
                .collect()
        } else {
            place.to_owned()
        }
    }

    fn map(&self, fid: FileScopeId) -> &UseDefMap<'db> {
        self.index.use_def_map(fid)
    }

    /// A reachability diagram's condition, memoized per scope.
    fn condition(&mut self, fid: FileScopeId, id: ScopedReachabilityConstraintId) -> Condition {
        let map = self.index.use_def_map(fid);
        let memo = self.diagrams.entry(fid).or_default();
        diagram(self.t, fid, map, memo, id)
    }

    fn skip_cause(&mut self, fid: FileScopeId, id: ScopedReachabilityConstraintId) -> SkipCause {
        if id == ScopedReachabilityConstraintId::ALWAYS_FALSE {
            return SkipCause::Ty;
        }
        let memo = self.runtime_diagrams.entry(fid).or_default();
        if runtime_in_diagram(self.t, self.index.use_def_map(fid), memo, id) {
            SkipCause::Runtime
        } else {
            SkipCause::Stable
        }
    }

    /// Record a use and the definitions reaching it.
    fn use_(&mut self, e: &Expr) {
        let span = Span::from(e.range());
        if self.use_of.contains_key(&span) {
            return;
        }
        let Some(fid) = self.index.try_expression_scope_id(e) else {
            return;
        };
        let Some(scope) = self.scope(fid) else { return };
        let Some(place) = PlaceExpr::try_from_expr(e) else {
            return;
        };
        let place = self.unsentinel(&place.to_string(), e.range());
        let use_id = ast::ExprRef::from(e).scoped_use_id(self.db, self.pf);
        let ix = self.flow.uses.len() as u32;
        self.flow.uses.push(Use {
            scope,
            place,
            span,
            annotation: self.in_annotation || self.lazy(fid),
        });
        self.use_of.insert(span, ix);
        let bindings: Vec<(DefinitionState<'db>, _)> = self
            .map(fid)
            .bindings_at_use(use_id)
            .map(|b| (b.binding, b.reachability_constraint))
            .collect();
        for (state, reach) in bindings {
            let condition = self.condition(fid, reach);
            if condition.is_never() {
                let cause = self.skip_cause(fid, reach);
                self.flow.skips.reach(cause);
                continue;
            }
            match state {
                DefinitionState::Defined(d) => {
                    if let DefinitionKind::LoopHeader(h) = d.kind(self.db) {
                        let mut seen = HashSet::new();
                        for def_ix in self.loop_bindings(fid, h, &mut seen) {
                            self.flow.reaching.push(Reach {
                                use_ix: ix,
                                def_ix,
                                condition: condition.clone(),
                                loop_carried: true,
                            });
                        }
                    } else {
                        let def_ix = self.def(scope, d);
                        self.flow.reaching.push(Reach {
                            use_ix: ix,
                            def_ix,
                            condition,
                            loop_carried: false,
                        });
                    }
                }
                DefinitionState::Undefined | DefinitionState::Deleted => {
                    self.flow.reaching.push(Reach {
                        use_ix: ix,
                        def_ix: None,
                        condition,
                        loop_carried: false,
                    });
                }
            }
        }
    }

    /// The loop-body definitions a loop header stands for, nested headers expanded.
    fn loop_bindings(
        &mut self,
        fid: FileScopeId,
        h: &ty_python_core::definition::LoopHeaderDefinitionKind,
        seen: &mut HashSet<Definition<'db>>,
    ) -> Vec<Option<u32>> {
        let scope = self.scope(fid).expect("a loop is in a real scope");
        let live: Vec<_> = self
            .map(fid)
            .loop_header(h.loop_header_id())
            .bindings_for_place(h.place())
            .map(|b| self.map(fid).definition(b.binding()))
            .collect();
        let mut out = Vec::new();
        for state in live {
            match state {
                DefinitionState::Defined(d) if seen.insert(d) => {
                    if let DefinitionKind::LoopHeader(inner) = d.kind(self.db) {
                        out.extend(self.loop_bindings(fid, inner, seen));
                    } else {
                        out.push(self.def(scope, d));
                    }
                }
                DefinitionState::Defined(_) => {}
                DefinitionState::Undefined | DefinitionState::Deleted => out.push(None),
            }
        }
        out
    }

    /// The reachability of the statement at `range` in scope `fid`: the last recorded entry
    /// containing its start (entries are recorded in visit order, a body after its header).
    fn region(&mut self, fid: FileScopeId, range: TextRange) -> Condition {
        if !self.regions.contains_key(&fid) {
            let entries: Vec<_> = self.map(fid).range_reachability().collect();
            let mut out = Vec::with_capacity(entries.len());
            for (r, id) in entries {
                out.push((r, self.condition(fid, id)));
            }
            self.regions.insert(fid, out);
        }
        self.regions[&fid]
            .iter()
            .rev()
            .find(|(r, _)| r.contains_range(TextRange::empty(range.start())))
            .map_or_else(Condition::always, |(_, c)| c.clone())
    }

    /// The value sources of `e` (see the crate docs).
    #[expect(
        clippy::too_many_arguments,
        reason = "source traversal carries its sink, path, and skip provenance"
    )]
    fn sources(
        &mut self,
        sink: Sink,
        sink_span: Span,
        e: &Expr,
        cond: &Condition,
        false_cause: Option<SkipCause>,
        identity: bool,
        through_call: bool,
    ) {
        if cond.is_never() {
            self.flow
                .skips
                .value(false_cause.unwrap_or(SkipCause::Stable));
            return;
        }
        match e {
            Expr::Name(_) | Expr::Attribute(_) | Expr::Subscript(_)
                if PlaceExpr::try_from_expr(e).is_some() =>
            {
                if let Some(&u) = self.use_of.get(&Span::from(e.range())) {
                    self.flow.values.push(ValueSource {
                        sink,
                        span: sink_span,
                        use_ix: u,
                        identity: identity && !through_call,
                        through_call,
                        condition: cond.clone(),
                    });
                }
                // The receiver or container is read to compute the value.
                match e {
                    Expr::Attribute(a) => {
                        self.sources(
                            sink,
                            sink_span,
                            &a.value,
                            cond,
                            false_cause,
                            false,
                            through_call,
                        );
                    }
                    Expr::Subscript(s) => {
                        self.sources(
                            sink,
                            sink_span,
                            &s.value,
                            cond,
                            false_cause,
                            false,
                            through_call,
                        );
                        self.sources(
                            sink,
                            sink_span,
                            &s.slice,
                            cond,
                            false_cause,
                            false,
                            through_call,
                        );
                    }
                    _ => {}
                }
            }
            Expr::If(i) => {
                let test = self.t.test(&i.test);
                let (body, orelse) = (cond.and(&test), cond.and(&test.not()));
                let cause = if self.t.runtime_decides(&i.test) {
                    Some(SkipCause::Runtime)
                } else {
                    false_cause
                };
                self.sources(
                    sink,
                    sink_span,
                    &i.body,
                    &body,
                    cause,
                    identity,
                    through_call,
                );
                self.sources(
                    sink,
                    sink_span,
                    &i.orelse,
                    &orelse,
                    cause,
                    identity,
                    through_call,
                );
                self.sources(
                    sink,
                    sink_span,
                    &i.test,
                    cond,
                    false_cause,
                    false,
                    through_call,
                );
            }
            Expr::BoolOp(b) => {
                // `a or b`: `a` when truthy, else `b`; `a and b`: `a` when falsy, else `b`.
                let mut before = Condition::always();
                let n = b.values.len();
                for (k, v) in b.values.iter().enumerate() {
                    let truth = self.t.test(v);
                    let here = if k + 1 == n {
                        before.clone()
                    } else {
                        match b.op {
                            ast::BoolOp::Or => before.and(&truth),
                            ast::BoolOp::And => before.and(&truth.not()),
                        }
                    };
                    let here = cond.and(&here);
                    self.sources(
                        sink,
                        sink_span,
                        v,
                        &here,
                        false_cause,
                        identity,
                        through_call,
                    );
                    before = match b.op {
                        ast::BoolOp::Or => before.and(&truth.not()),
                        ast::BoolOp::And => before.and(&truth),
                    };
                }
            }
            Expr::Named(n) => {
                self.sources(
                    sink,
                    sink_span,
                    &n.value,
                    cond,
                    false_cause,
                    identity,
                    through_call,
                );
            }
            _ => {
                // Computed: each sub-expression's uses, derived, under the conditions nested
                // conditional expressions and boolean operators select them by. Everything inside
                // a call is through it.
                let through_call = through_call || matches!(e, Expr::Call(_));
                for child in children(e) {
                    self.sources(
                        sink,
                        sink_span,
                        child,
                        cond,
                        false_cause,
                        false,
                        through_call,
                    );
                }
            }
        }
    }
}

/// The expressions directly inside `e`, in source order (a comprehension's, a lambda's and an
/// f-string's included).
fn children(e: &Expr) -> Vec<&Expr> {
    struct V<'a>(Vec<&'a Expr>);
    impl<'a> SourceOrderVisitor<'a> for V<'a> {
        fn visit_expr(&mut self, e: &'a Expr) {
            self.0.push(e);
        }
    }
    let mut v = V(Vec::new());
    source_order::walk_expr(&mut v, e);
    v.0
}

struct Visitor<'w, 'a, 'db> {
    w: &'w mut Walk<'a, 'db>,
    /// Augmented-assignment targets: a store that is also a use.
    aug: HashSet<TextRange>,
    /// The scope the statements being visited run in.
    stack: Vec<FileScopeId>,
}

impl Visitor<'_, '_, '_> {
    fn sources(&mut self, sink: Sink, e: &Expr, identity: bool) {
        self.w.sources(
            sink,
            e.range().into(),
            e,
            &Condition::always(),
            None,
            identity,
            false,
        );
    }

    fn generators(&mut self, generators: &[ast::Comprehension]) {
        for g in generators {
            self.sources(Sink::Definition, &g.iter, false);
        }
    }

    fn body(&mut self, scope: NodeWithScopeRef<'_>, body: &[Stmt]) {
        match self.w.index.try_node_scope(scope) {
            Some(fid) => {
                self.stack.push(fid);
                for s in body {
                    self.visit_stmt(s);
                }
                self.stack.pop();
            }
            None => {
                for s in body {
                    self.visit_stmt(s);
                }
            }
        }
    }
}

/// A plain assignment target: its value passes unchanged (an unpacking does not).
fn plain(target: &Expr) -> bool {
    matches!(
        target,
        Expr::Name(_) | Expr::Attribute(_) | Expr::Subscript(_)
    )
}

impl<'ast> SourceOrderVisitor<'ast> for Visitor<'_, '_, '_> {
    fn enter_node(&mut self, _node: AnyNodeRef<'ast>) -> TraversalSignal {
        TraversalSignal::Traverse
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        let fid = *self
            .stack
            .last()
            .expect("the module scope is at the bottom");
        if let Some(scope) = self.w.scope(fid) {
            let condition = self.w.region(fid, stmt.range());
            self.w.flow.regions.push(Region {
                scope,
                span: stmt.range().into(),
                condition,
            });
        }
        match stmt {
            // Decorators, defaults, annotations and bases run in the enclosing scope; the body in
            // the new one.
            Stmt::FunctionDef(f) => {
                for d in &f.decorator_list {
                    self.visit_decorator(d);
                }
                if let Some(tp) = &f.type_params {
                    self.visit_type_params(tp);
                }
                self.visit_parameters(&f.parameters);
                if let Some(r) = &f.returns {
                    self.visit_annotation(r);
                }
                self.body(NodeWithScopeRef::Function(f), &f.body);
                return;
            }
            Stmt::ClassDef(c) => {
                for d in &c.decorator_list {
                    self.visit_decorator(d);
                }
                if let Some(tp) = &c.type_params {
                    self.visit_type_params(tp);
                }
                if let Some(a) = &c.arguments {
                    self.visit_arguments(a);
                }
                self.body(NodeWithScopeRef::Class(c), &c.body);
                return;
            }
            Stmt::AugAssign(a) => {
                self.aug.insert(a.target.range());
            }
            _ => {}
        }
        // Children first, so every use a sink reads is recorded before the sink's sources.
        source_order::walk_stmt(self, stmt);
        match stmt {
            Stmt::Return(r) => {
                if let Some(v) = &r.value {
                    self.sources(Sink::Return, v, true);
                }
            }
            Stmt::Raise(r) => {
                for v in [&r.exc, &r.cause].into_iter().flatten() {
                    self.sources(Sink::Raise, v, false);
                }
            }
            Stmt::Assign(a) => {
                let identity = a.targets.iter().all(plain);
                self.sources(Sink::Definition, &a.value, identity);
            }
            Stmt::AnnAssign(a) => {
                if let Some(v) = &a.value {
                    self.sources(Sink::Definition, v, plain(&a.target));
                }
            }
            Stmt::AugAssign(a) => {
                // The new value is computed from the operand and the old value.
                self.sources(Sink::Definition, &a.value, false);
                self.w.sources(
                    Sink::Definition,
                    a.value.range().into(),
                    &a.target,
                    &Condition::always(),
                    None,
                    false,
                    false,
                );
            }
            Stmt::For(f) => self.sources(Sink::Definition, &f.iter, false),
            Stmt::With(w) => {
                for item in &w.items {
                    self.sources(Sink::Definition, &item.context_expr, false);
                }
            }
            _ => {}
        }
    }

    fn visit_annotation(&mut self, e: &'ast Expr) {
        let outer = std::mem::replace(&mut self.w.in_annotation, true);
        self.visit_expr(e);
        self.w.in_annotation = outer;
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        source_order::walk_expr(self, e);
        if !self.w.in_annotation {
            match e {
                Expr::Attribute(a) if matches!(a.ctx, ExprContext::Load) => {
                    self.w.flow.attribute_loads.push(AttributeLoad {
                        span: Span::from(e.range()),
                        name: a.attr.id.to_string(),
                    });
                }
                // `getattr(x, "f")` and `hasattr(x, "f")` load `f` by a literal name.
                Expr::Call(call) => {
                    if let Expr::Name(f) = &*call.func
                        && matches!(f.id.as_str(), "getattr" | "hasattr")
                        && let Some(Expr::StringLiteral(name)) = call.arguments.args.get(1)
                    {
                        self.w.flow.attribute_loads.push(AttributeLoad {
                            span: Span::from(e.range()),
                            name: name.value.to_str().to_owned(),
                        });
                    }
                }
                _ => {}
            }
        }
        // A `del` target is an unbinding in our model (`bindings` kind `del`), not a read.
        let is_use = match e {
            Expr::Name(n) => matches!(n.ctx, ExprContext::Load),
            Expr::Attribute(a) => matches!(a.ctx, ExprContext::Load),
            Expr::Subscript(s) => matches!(s.ctx, ExprContext::Load),
            _ => false,
        } || self.aug.contains(&e.range());
        if is_use && PlaceExpr::try_from_expr(e).is_some() {
            self.w.use_(e);
        }
        match e {
            Expr::Call(call) => {
                for a in &call.arguments.args {
                    match a {
                        Expr::Starred(s) => self.w.sources(
                            Sink::Argument,
                            a.range().into(),
                            &s.value,
                            &Condition::always(),
                            None,
                            false,
                            false,
                        ),
                        v => self.sources(Sink::Argument, v, true),
                    }
                }
                for k in &call.arguments.keywords {
                    self.sources(Sink::Argument, &k.value, k.arg.is_some());
                }
            }
            Expr::Yield(y) => {
                if let Some(v) = &y.value {
                    self.sources(Sink::Yield, v, true);
                }
            }
            Expr::Named(n) => self.sources(Sink::Definition, &n.value, true),
            Expr::ListComp(c) => self.generators(&c.generators),
            Expr::SetComp(c) => self.generators(&c.generators),
            Expr::DictComp(c) => self.generators(&c.generators),
            Expr::Generator(c) => self.generators(&c.generators),
            _ => {}
        }
    }
}
