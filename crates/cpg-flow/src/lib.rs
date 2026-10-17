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

mod admitted;
mod db;
mod events;
pub use admitted::FlowCondition;
use admitted::{Resources, admit, hash_slot, slots};
pub use events::*;
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
mod narrowing;
pub mod native;
mod predicate;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub use lctx_model::domain::{
    flow::FlowCallOperandRole,
    lexical::{BindingEventKind as BindingKind, LexicalScopeKind},
};
use native::Atom;
pub type Condition = lctx_model::domain::conditions::CondGraph<Atom>;
use ruff_db::Db as _;
use ruff_db::files::system_path_to_file;
use ruff_db::parsed::{ParsedModuleRef, parsed_module};
use ruff_db::system::{DbWithWritableSystem, SystemPathBuf};
use ruff_python_ast_ty::token::TokenKind;
use ruff_python_ast_ty::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
use ruff_python_ast_ty::{self as ast, AnyNodeRef, Expr, ExprContext, PySourceType, Stmt};
use ruff_text_size_ty::{Ranged, TextRange};
use serde::{Deserialize, Serialize};
use ty_module_resolver::SearchPathSettings;
use ty_python_core::ast_ids::HasScopedUseId;
use ty_python_core::definition::{Definition, DefinitionKind, DefinitionState};
use ty_python_core::place::PlaceExprRef;
use ty_python_core::platform::PythonPlatform;
use ty_python_core::predicate::PredicateNode;
use ty_python_core::program::{FallibleStrategy, Program, ProgramSettings};
use ty_python_core::reachability_constraints::ScopedReachabilityConstraintId;
use ty_python_core::scope::{NodeWithScopeKind, NodeWithScopeRef};
use ty_python_core::{
    EnclosingSnapshotResult, FileScopeId, ProgramFile, UseDefMap, semantic_index,
};

use crate::db::FlowDb;
use crate::predicate::{Translator, diagram, runtime_in_diagram};

/// The provider, as `producers.revision` names it (ADR-0022 §Identity).
pub use lctx_model::domain::producer_contract::facts::TY_REVISION as PROVIDER;
/// The word ty decides at index time, and the same-length name it is renamed to.
pub const WORD: &[u8] = b"TYPE_CHECKING";
pub const SENTINEL: &[u8] = b"TYPE_CHECKIN_";

/// The analyzed context the runtime view reads (§4.0).
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub python_version: (u32, u32, u32),
    pub platform: String,
}

/// The model syntax kind for the pinned latest Ruff family.
#[deny(clippy::wildcard_enum_match_arm)]
pub fn syntax_kind(k: ruff_python_ast_ty::NodeKind) -> lctx_model::domain::source::SyntaxKind {
    use lctx_model::domain::source::SyntaxKind;
    use ruff_python_ast_ty::NodeKind;
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

/// One module to index: its path relative to the release and captured original text.
#[derive(Debug, Clone)]
pub struct Input {
    pub path: String,
    pub text: String,
    /// Name-load spans whose lexical resolution proves a runtime-special binding.
    pub runtime: RuntimeBindings,
}

impl Input {
    /// Required coverage for already admitted owned backing; this does not admit allocation.
    pub fn owned_bytes(&self) -> usize {
        self.path
            .capacity()
            .saturating_add(self.text.capacity())
            .saturating_add(self.runtime.owned_bytes())
    }
}

/// A source and the admission acquired before constructing its owned backing.
/// Moving this value transfers its reservation without cloning source or runtime maps.
#[derive(Debug)]
pub struct AdmittedInput {
    input: Input,
    charge: StateCharge,
}
impl AdmittedInput {
    pub fn new(input: Input, charge: StateCharge) -> Result<Self, ModelError> {
        if charge.budget().is_none() || charge.reserved() < input.owned_bytes() {
            return Err(ModelError::Conflict("flow input backing was not admitted"));
        }
        Ok(Self { input, charge })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeBindings {
    pub checking_names: BTreeSet<Span>,
    pub typing_modules: BTreeSet<Span>,
    pub sys_modules: BTreeSet<Span>,
    pub os_modules: BTreeSet<Span>,
    /// Name loads resolved by the lexical provider to these exact builtins.
    pub builtin_type: BTreeSet<Span>,
    pub builtin_classes: BTreeMap<String, BTreeSet<Span>>,
}

impl RuntimeBindings {
    /// Conservative tree-node allowance plus its span payload, reserved before insertion.
    pub const SPAN_ENTRY_BYTES: usize = 128 + size_of::<Span>();
    /// Cover a whole node per distinct class, including spare key/value slots. The pinned
    /// Rust B-tree uses 11 slots and 12 child edges; 16 words cover edges, header and padding.
    /// This excludes separately admitted String/span-set backing and is not allocator/RSS usage.
    pub const CLASS_ENTRY_BYTES: usize =
        16 * size_of::<usize>() + 11 * (size_of::<String>() + size_of::<BTreeSet<Span>>());

    /// Required coverage of tree slots and strings; this does not admit allocation.
    pub fn owned_bytes(&self) -> usize {
        let spans = [
            &self.checking_names,
            &self.typing_modules,
            &self.sys_modules,
            &self.os_modules,
            &self.builtin_type,
        ]
        .into_iter()
        .fold(0usize, |bytes, set| {
            bytes.saturating_add(set.len().saturating_mul(Self::SPAN_ENTRY_BYTES))
        });
        self.builtin_classes
            .iter()
            .fold(spans, |bytes, (name, set)| {
                bytes
                    .saturating_add(Self::CLASS_ENTRY_BYTES)
                    .saturating_add(name.capacity())
                    .saturating_add(set.len().saturating_mul(Self::SPAN_ENTRY_BYTES))
            })
    }
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
    pub nested: bool,
    pub condition: FlowCondition,
    /// Native type narrowing is characterization, separate from path reachability.
    pub narrowing: FlowCondition,
    pub narrowing_precision_lost: bool,
    /// Reached around a loop's back edge (through ty's loop header): the condition is the use's
    /// side only.
    pub loop_carried: bool,
}

/// Native candidate enumeration before pruning/expansion. Provider indices are adapter-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Bound,
    Undefined,
    Deleted,
    Nested,
    LoopHeader,
}
#[derive(Debug, Clone)]
pub struct UseCandidate {
    /// Native formulas retained before pruning and loop expansion.
    pub condition: FlowCondition,
    pub narrowing: FlowCondition,
    pub narrowing_precision_lost: bool,
    pub use_ix: u32,
    pub ordinal: u32,
    pub kind: CandidateKind,
    pub reaching: std::ops::Range<usize>,
    pub pruned: bool,
    pub loop_expanded: bool,
    pub unattached: bool,
    pub reachability_lost: bool,
}

/// Snapshot characterization is independent of runtime capture value authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOrigin {
    OuterLocal,
    Global,
    Nonlocal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotState {
    FoundBindings,
    FoundConstraint,
    NotFound,
    NoLongerInEagerContext,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotBinding {
    Bound(u32),
    Undefined,
    Deleted,
    Nested,
    LoopHeader,
    Unattached,
}
#[derive(Debug, Clone)]
pub struct SnapshotCandidate {
    pub binding: SnapshotBinding,
    pub condition: FlowCondition,
    pub narrowing: FlowCondition,
    pub precision_lost: bool,
}
#[derive(Debug, Clone)]
pub struct CaptureSnapshot {
    pub use_ix: u32,
    pub enclosing: Scope,
    pub origin: CaptureOrigin,
    /// Lazy snapshots use upstream AssumeBound and remain candidates.
    pub lazy: bool,
    pub state: SnapshotState,
    pub candidates: Vec<SnapshotCandidate>,
    pub constraint: Option<FlowCondition>,
    pub constraint_precision_lost: bool,
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
    /// Outer-to-inner calls crossed by this use on its way to the sink. The call and operand
    /// spans are join coordinates, not identities until matched to Ruff source facts.
    pub call_path: Vec<CallFrame>,
    pub condition: FlowCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallFrame {
    pub call: Span,
    pub operand: Span,
    pub role: FlowCallOperandRole,
}

#[derive(Debug, Clone)]
pub struct Region {
    pub scope: Scope,
    pub span: Span,
    pub condition: FlowCondition,
}

/// A test ty records as a predicate (an `if`/`elif`/`while`/`assert` test, a conditional
/// expression's or boolean operand's, a `match` subject with its pattern): its span and its
/// condition. The uses inside the span are what the test reads (the Stage 2 end review's R8).
#[derive(Debug, Clone)]
pub struct Test {
    pub scope: Scope,
    pub span: Span,
    pub condition: FlowCondition,
}

/// One evaluation atom of one provider predicate, before the test-span projection merges rows.
/// This is the source identity for Stage 3's attributed type and value-link proofs.
#[derive(Debug, Clone)]
pub struct TestLeaf {
    pub scope: Scope,
    pub test_span: Span,
    pub condition: FlowCondition,
    pub atom: Atom,
    pub leaf_span: Span,
    /// The structurally selected place operand of this leaf, when the translator can identify
    /// one. A contained use is never chosen by proximity alone; synthetic patterns need their
    /// separate subject bridge.
    pub operand_span: Option<Span>,
}

/// An attribute load by name on any receiver (a place or not: `get_server()._worker`), or a
/// `getattr`/`hasattr` with a literal name. Outside annotations. The field and global premises
/// count these (the Stage 2 end review's R7).
#[derive(Debug, Clone)]
pub struct AttributeLoad {
    pub span: Span,
    pub name: String,
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
/// Tokens come from ruff 0.0.16's parser-mediated lexer recovery, so strings and comments keep
/// their text. Bare lexing differs on malformed interpolations: the parser re-lexes closing quotes
/// and format conversions. Keep that recovery even though only its tokens are consumed;
/// names inside f-string replacement fields are names. A module that already uses the sentinel as
/// a name is refused.
pub fn rename(text: &str) -> Result<(String, u32), String> {
    rename_impl(text, None)
}
fn rename_impl(text: &str, resources: Option<&Resources<'_>>) -> Result<(String, u32), String> {
    if let Some(resources) = resources {
        resources.checkpoint();
    }
    let parsed = ruff_python_parser_ty::parse_unchecked_source(text, PySourceType::Python);
    if let Some(resources) = resources {
        resources.checkpoint();
        resources.grow(text.len() + 128);
    }
    let sentinel = std::str::from_utf8(SENTINEL).expect("ASCII");
    let word = std::str::from_utf8(WORD).expect("ASCII");
    let mut out = text.as_bytes().to_vec();
    let mut n = 0;
    for t in parsed.tokens() {
        if let Some(resources) = resources {
            resources.checkpoint();
        }
        if t.kind() != TokenKind::Identifier {
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

/// Prepare the complete virtual universe, then synchronously visit one module at a time.
/// Inputs transfer their pre-admitted owned backing; each original/runtime buffer releases after
/// its module callbacks finish. The caller retains admission for the input vector's capacity.
/// Transformed virtual sources remain available to every later native dependency query.
/// Operational failures abort the attempt; model failures retain exact input correspondence.
pub fn visit_modules<E, G>(
    inputs: Vec<AdmittedInput>,
    context: &RuntimeContext,
    budget: &ResourceBudget,
    check: &dyn Fn() -> Result<(), ModelError>,
    register: impl Fn(salsa::CancellationToken) -> G,
    mut visit: impl for<'a> FnMut(usize, ModuleEvent<'a>) -> Result<(), E>,
) -> Result<(), VisitError<E>> {
    // The caller's E stays outside the unwind payload; no Send/'static bound is needed.
    let mut consumer_error = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut deliver =
            |input_index: usize, event: ModuleEvent<'_>| match visit(input_index, event) {
                Ok(()) => (),
                Err(error) => {
                    consumer_error = Some(error);
                    std::panic::resume_unwind(Box::new(admitted::ConsumerStopped));
                }
            };
        if inputs.is_empty() {
            return Ok(());
        }
        for input in &inputs {
            if !input
                .charge
                .budget()
                .is_some_and(|owner| owner.shares_pool(budget))
            {
                return Err(VisitError::Operational(FlowOperationalError::Model(
                    ModelError::Conflict("flow input foreign attempt pool"),
                )));
            }
        }
        use salsa::Database as _;
        let _registration;
        let mut db = FlowDb::new();
        _registration = register(db.cancellation_token());
        let resources = Resources::new(budget, check);
        let mut prepared = Vec::new();
        let mut preparation_charge = StateCharge::new(budget, "flow_prepared_sources");
        for admitted in &inputs {
            let input = &admitted.input;
            let _scratch = resources.scope();
            resources.checkpoint();
            slots(&mut prepared, &mut preparation_charge, 1);
            let path = resources.format(format_args!("/flow/{}", input.path));
            let result = rename_impl(&input.text, Some(&resources)).and_then(|(text, n)| {
                db.write_file(&path, &text)
                    .map(|()| {
                        (
                            path,
                            n,
                            lctx_model::domain::ContentHash::of(text.as_bytes()),
                            text.len(),
                        )
                    })
                    .map_err(|error| resources.format(format_args!("{error}")))
            });
            admit(
                &mut preparation_charge,
                match &result {
                    Ok((path, ..)) => path.capacity(),
                    Err(error) => error.capacity(),
                },
            );
            prepared.push(result);
            resources.checkpoint();
        }
        let _settings_scratch = resources.scope();
        resources.grow(context.platform.len() + 1024);
        let settings = program_settings(&db, context);
        let program = Program::from_settings(&db, &settings);
        for (input_index, (admitted, prepared)) in inputs.into_iter().zip(prepared).enumerate() {
            let input = &admitted.input;
            resources.checkpoint();
            match prepared {
                Ok((path, n, hash, byte_len)) => module(
                    &db,
                    program,
                    PreparedModule {
                        input_index,
                        path: &path,
                        original: &input.text,
                        runtime: &input.runtime,
                        renamed: n,
                        view_content: hash,
                        view_byte_len: byte_len,
                    },
                    context,
                    &resources,
                    &mut deliver,
                ),
                Err(detail) => deliver(
                    input_index,
                    ModuleEvent::ModuleFailure(&ModuleFailure { detail }),
                ),
            }
            drop(admitted);
            resources.checkpoint();
        }
        Ok(())
    }));
    match result {
        Ok(value) => value,
        Err(payload) if payload.is::<admitted::ConsumerStopped>() => Err(VisitError::Consumer(
            consumer_error
                .take()
                .expect("consumer stop retains original error"),
        )),
        Err(payload) => match payload.downcast::<admitted::Aborted>() {
            Ok(error) => Err(VisitError::Operational(error.0)),
            Err(payload) => match payload.downcast::<salsa::Cancelled>() {
                Ok(error) => Err(VisitError::Operational(
                    FlowOperationalError::NativeCancelled(*error),
                )),
                Err(payload) => std::panic::resume_unwind(payload),
            },
        },
    }
}

fn native_place(
    expr: &Expr,
    original: &str,
    depth: usize,
    resources: &Resources<'_>,
) -> Option<native::Place> {
    resources.checkpoint();
    if depth > 256 {
        return None;
    }
    match expr {
        Expr::Name(n) => Some(native::Place {
            root: resources
                .string(&original[usize::from(n.range().start())..usize::from(n.range().end())]),
            segments: vec![],
        }),
        Expr::Attribute(a) => {
            let mut place = native_place(&a.value, original, depth + 1, resources)?;
            let name = resources.string(a.attr.as_str());
            resources.push(&mut place.segments, native::PlaceSegment::Attribute(name));
            Some(place)
        }
        Expr::Subscript(s) => {
            let mut place = native_place(&s.value, original, depth + 1, resources)?;
            let value = predicate::literal(&s.slice, resources)?;
            resources.push(&mut place.segments, native::PlaceSegment::Item(value));
            Some(place)
        }
        Expr::Named(n) => native_place(&n.target, original, depth + 1, resources),
        _ => None,
    }
}

fn program_settings(db: &FlowDb, context: &RuntimeContext) -> ProgramSettings {
    let vendored = ty_vendored::file_system().clone();
    let mut settings = ProgramSettings::empty(&vendored);
    settings.python_version.version = ruff_python_ast_ty::PythonVersion::from((
        u8::try_from(context.python_version.0).expect("configured Python major version"),
        u8::try_from(context.python_version.1).expect("configured Python minor version"),
    ));
    settings.python_platform = PythonPlatform::from(context.platform.clone());
    settings.search_paths = SearchPathSettings::new(vec![SystemPathBuf::from("/flow")])
        .to_search_paths(db.system(), db.vendored(), &FallibleStrategy)
        .expect("the virtual flow root is valid");
    settings
}

struct PreparedModule<'a> {
    input_index: usize,
    path: &'a str,
    original: &'a str,
    runtime: &'a RuntimeBindings,
    renamed: u32,
    view_content: lctx_model::domain::ContentHash,
    view_byte_len: usize,
}

fn module(
    db: &FlowDb,
    program: Program<'_>,
    input: PreparedModule<'_>,
    context: &RuntimeContext,
    resources: &Resources<'_>,
    visit: &mut dyn for<'e> FnMut(usize, ModuleEvent<'e>),
) {
    let PreparedModule {
        input_index,
        path,
        original,
        runtime,
        renamed,
        view_content,
        view_byte_len,
    } = input;
    #[cfg(test)]
    settings_tests::record_query(path);
    resources.checkpoint();
    let file = match system_path_to_file(db, path) {
        Ok(file) => file,
        Err(error) => {
            let _scratch = resources.scope();
            let detail = resources.format(format_args!("{error}"));
            visit(
                input_index,
                ModuleEvent::ModuleFailure(&ModuleFailure { detail }),
            );
            return;
        }
    };
    let pf = ProgramFile::new(db, file, program);
    let parsed = parsed_module(db, pf.python_file(db)).load(db);
    resources.checkpoint();
    let syntax_errors = parsed.errors().len();
    let index = semantic_index(db, pf);
    resources.checkpoint();
    let t = Translator {
        db,
        module: &parsed,
        original,
        context,
        runtime,
        resources,
    };
    let mut w = Walk {
        db,
        pf,
        index,
        t: &t,
        parsed: &parsed,
        resources,
        visit,
        input_index,
        use_count: 0,
        def_count: 0,
        skips: SkipCounts::default(),
        current: None,
        use_of: HashMap::new(),
        def_of: HashMap::new(),
        diagrams: HashMap::new(),
        runtime_diagrams: HashMap::new(),
        regions: HashMap::new(),
        in_annotation: false,
        charge: StateCharge::new(resources.budget, "flow_module_correspondence"),
    };
    struct Places<'a> {
        original: &'a str,
        places: BTreeMap<Span, native::Place>,
        nodes: BTreeMap<Span, Vec<NodeEntry>>,
        resources: &'a Resources<'a>,
        charge: StateCharge,
        ordinal: usize,
    }
    impl<'a> SourceOrderVisitor<'a> for Places<'_> {
        fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
            use lctx_model::domain::source::OccurrenceRole;
            let _scratch = self.resources.scope();
            self.resources.checkpoint();
            let kind = Some(syntax_kind(node.kind()));
            let role = match node {
                AnyNodeRef::ExprName(n) => match n.ctx {
                    ExprContext::Load => OccurrenceRole::Read,
                    ExprContext::Store => OccurrenceRole::Binding,
                    _ => OccurrenceRole::Syntax,
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
            let span = Span::from(node.range());
            if !self.nodes.contains_key(&span) {
                admit(
                    &mut self.charge,
                    128 + size_of::<Span>() + size_of::<Vec<NodeEntry>>(),
                );
            }
            let rows = self.nodes.entry(span).or_default();
            slots(rows, &mut self.charge, 1);
            rows.push(NodeEntry {
                ordinal: self.ordinal,
                kind,
                role,
            });
            self.ordinal += 1;
            TraversalSignal::Traverse
        }
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Some(place) = native_place(expr, self.original, 0, self.resources) {
                admit(
                    &mut self.charge,
                    128 + size_of::<Span>() + size_of::<native::Place>() + place.heap_bytes(),
                );
                self.places.insert(Span::from(expr.range()), place);
            }
            source_order::walk_expr(self, expr);
        }
    }

    let mut places = Places {
        original,
        places: BTreeMap::new(),
        nodes: BTreeMap::new(),
        resources,
        charge: StateCharge::new(resources.budget, "flow_preamble"),
        ordinal: 0,
    };
    {
        let _scratch = resources.scope();
        places.visit_body(parsed.suite());
    }
    let mut prelude = ModulePrelude {
        original_content: lctx_model::domain::ContentHash::of(original.as_bytes()),
        view_content,
        view_byte_len,
        renamed,
        syntax_errors,
        nodes: places.nodes,
        places: places.places,
        operands: HashMap::new(),
        _charge: places.charge,
    };
    let mut predicates = Vec::new();
    let mut leaves = Vec::new();
    let mut tests = Vec::new();
    let mut predicate_charge = StateCharge::new(resources.budget, "flow_predicate_inventory");
    let mut seen = HashSet::new();
    for scope in index.scope_ids() {
        let fid = scope.file_scope_id(db);
        let Some(sc) = w.scope(fid) else { continue };
        for p in index.use_def_map(fid).predicates().iter() {
            let _scratch = resources.scope();
            resources.checkpoint();
            let (span, condition, test_expr) = match &p.node {
                PredicateNode::Expression(x)
                | PredicateNode::Condition(x)
                | PredicateNode::ChainedComparisonCondition(x) => {
                    let e = x.node_ref(db).node(&parsed);
                    (Span::from(e.range()), t.test(e), Some(e))
                }
                PredicateNode::Pattern(pattern) => {
                    let subject = pattern.subject(db).node_ref(db).node(&parsed);
                    let evaluation = t.pattern_site(pattern);
                    let mut c = t.pattern(subject, pattern.kind(db), &evaluation);
                    if let Some(guard) = pattern.guard(db) {
                        c = c.and(&t.test(guard.node_ref(db).node(&parsed)));
                    }
                    (
                        match evaluation {
                            native::EvaluationSite::Source(span) => span,
                            _ => Span::from(subject.range()),
                        },
                        c,
                        None,
                    )
                }
                _ => continue,
            };
            let predicate_ix = predicates.len();
            slots(&mut predicates, &mut predicate_charge, 1);
            predicates.push(Test {
                scope: sc,
                span,
                condition,
            });
            let condition = &predicates[predicate_ix].condition;
            for (node, atom) in condition
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| match n {
                    lctx_model::domain::conditions::CondNode::Leaf(atom) => Some((i, atom)),
                    _ => None,
                })
            {
                let Some(leaf_span) = atom.site() else {
                    continue;
                };
                if leaf_span.start < span.start || leaf_span.end > span.end {
                    continue;
                }
                slots(&mut leaves, &mut predicate_charge, 1);
                leaves.push((
                    span,
                    leaf_span,
                    leaves.len(),
                    predicate_ix,
                    node,
                    test_expr.and_then(|e| t.tested_place_operand(e, atom)),
                ));
            }
            if !seen.contains(&span) {
                admit(&mut predicate_charge, 128 + size_of::<Span>());
                seen.insert(span);
                slots(&mut tests, &mut predicate_charge, 1);
                tests.push(predicate_ix);
            }
        }
    }
    leaves.sort_unstable_by_key(|row| (row.0, row.1, row.2));
    tests.sort_unstable_by_key(|i| (predicates[*i].span, *i));
    for (_, _, _, predicate_ix, node, operand) in &leaves {
        prelude.record_operand(predicates[*predicate_ix].condition.clone(), *node, *operand);
    }
    w.emit(ModuleEvent::Prelude(prelude));
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
        for (_, d, _) in index.use_def_map(fid).definitions_with_usage() {
            let _scratch = resources.scope();
            resources.checkpoint();
            w.def(sc, d);
        }
    }
    let mut v = Visitor {
        w: &mut w,
        aug: HashSet::new(),
        stack: Vec::new(),
    };
    slots(&mut v.stack, &mut v.w.charge, 1);
    v.stack.push(FileScopeId::global());
    for stmt in parsed.suite() {
        v.visit_stmt(stmt);
    }
    drop(v);
    let skips = w.skips.clone();
    drop(w);
    visit(input_index, ModuleEvent::TraversalComplete);
    for i in tests {
        resources.checkpoint();
        visit(input_index, ModuleEvent::Test(&predicates[i]));
    }
    for (test_span, leaf_span, _, i, node, operand_span) in leaves {
        resources.checkpoint();
        let test = &predicates[i];
        let lctx_model::domain::conditions::CondNode::Leaf(atom) = &test.condition.nodes[node]
        else {
            unreachable!()
        };
        visit(
            input_index,
            ModuleEvent::TestLeaf(TestLeafRef {
                scope: test.scope,
                test_span,
                condition: &test.condition,
                atom,
                leaf_span,
                operand_span,
            }),
        );
    }
    resources.checkpoint();
    visit(input_index, ModuleEvent::End(skips))
}

struct Walk<'a, 'db> {
    db: &'db FlowDb,
    pf: ProgramFile<'db>,
    index: &'a ty_python_core::SemanticIndex<'db>,
    t: &'a Translator<'a>,
    parsed: &'a ParsedModuleRef,
    resources: &'a Resources<'a>,
    visit: &'a mut dyn for<'e> FnMut(usize, ModuleEvent<'e>),
    input_index: usize,
    use_count: u32,
    def_count: u32,
    skips: SkipCounts,
    current: Option<UseGroup>,
    use_of: HashMap<Span, u32>,
    def_of: HashMap<Definition<'db>, Option<u32>>,
    diagrams: HashMap<FileScopeId, HashMap<ScopedReachabilityConstraintId, FlowCondition>>,
    runtime_diagrams: HashMap<FileScopeId, HashMap<ScopedReachabilityConstraintId, bool>>,
    regions: HashMap<FileScopeId, Vec<(TextRange, FlowCondition)>>,
    in_annotation: bool,
    charge: StateCharge,
}

impl<'db> Walk<'_, 'db> {
    fn emit(&mut self, event: ModuleEvent<'_>) {
        self.resources.checkpoint();
        (self.visit)(self.input_index, event);
    }
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
        let resources = self.resources;
        let _scratch = resources.scope();
        resources.checkpoint();
        if let Some(ix) = self.def_of.get(&d) {
            return *ix;
        }
        if matches!(
            self.index.scope(d.file_scope(self.db)).node(),
            NodeWithScopeKind::ClassTypeParameters(_)
                | NodeWithScopeKind::TypeAliasTypeParameters(_)
        ) {
            // Not modelled by our lexical recognizer (see `module`).
            hash_slot(&mut self.def_of, &mut self.charge);
            self.def_of.insert(d, None);
            return None;
        }
        let m = self.parsed;
        let kind = d.kind(self.db);
        let (binding, value): (Option<BindingKind>, Option<TextRange>) = match kind {
            DefinitionKind::Import(_) => (Some(BindingKind::Import), None),
            DefinitionKind::ImportFrom(_) => (Some(BindingKind::FromImport), None),
            DefinitionKind::ImportFromSubmodule(_) => {
                (Some(BindingKind::ImportFromSubmodule), None)
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
            hash_slot(&mut self.def_of, &mut self.charge);
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
        let place = self.resources.format(format_args!(
            "{}",
            self.index.place_table(fid).place(place_id)
        ));
        let place = self.unsentinel(&place, target);
        let ix = self.def_count;
        self.def_count = self
            .def_count
            .checked_add(1)
            .expect("flow definition ordinal");
        let definition = Def {
            scope,
            place,
            kind: binding,
            target: target.into(),
            value: value.map(Span::from),
        };
        hash_slot(&mut self.def_of, &mut self.charge);
        self.def_of.insert(d, Some(ix));
        self.emit(ModuleEvent::Definition {
            ordinal: ix,
            value: &definition,
        });
        Some(ix)
    }

    /// A place spelled with the sentinel, spelled as written.
    fn unsentinel(&self, place: &str, span: TextRange) -> String {
        if place.contains(std::str::from_utf8(SENTINEL).expect("ASCII")) {
            self.resources.compact(
                &self.t.original[usize::from(span.start())..usize::from(span.end())],
                "",
            )
        } else {
            self.resources.string(place)
        }
    }

    fn map(&self, fid: FileScopeId) -> &UseDefMap<'db> {
        self.index.use_def_map(fid)
    }

    /// A reachability diagram's condition, memoized per scope.
    fn condition(&mut self, fid: FileScopeId, id: ScopedReachabilityConstraintId) -> FlowCondition {
        let map = self.index.use_def_map(fid);
        if !self.diagrams.contains_key(&fid) {
            hash_slot(&mut self.diagrams, &mut self.charge);
        }
        let memo = self.diagrams.entry(fid).or_default();
        diagram(self.t, fid, map, memo, id, &mut self.charge)
    }

    fn skip_cause(&mut self, fid: FileScopeId, id: ScopedReachabilityConstraintId) -> SkipCause {
        if id == ScopedReachabilityConstraintId::ALWAYS_FALSE {
            return SkipCause::Ty;
        }
        if !self.runtime_diagrams.contains_key(&fid) {
            hash_slot(&mut self.runtime_diagrams, &mut self.charge);
        }
        let memo = self.runtime_diagrams.entry(fid).or_default();
        if runtime_in_diagram(
            self.t,
            self.index.use_def_map(fid),
            memo,
            id,
            &mut self.charge,
        ) {
            SkipCause::Runtime
        } else {
            SkipCause::Stable
        }
    }

    /// Record a use and the definitions reaching it.
    fn use_(&mut self, e: &Expr) {
        let resources = self.resources;
        let _scratch = resources.scope();
        resources.checkpoint();
        let span = Span::from(e.range());
        if self.use_of.contains_key(&span) {
            return;
        }
        let Some(fid) = self.index.try_expression_scope_id(e) else {
            self.emit(ModuleEvent::UnavailableUse(span));
            return;
        };
        let Some(scope) = self.scope(fid) else {
            self.emit(ModuleEvent::UnavailableUse(span));
            return;
        };
        let Some(place) = admitted::place_expr(e, self.resources) else {
            return;
        };
        let displayed = self.resources.format(format_args!("{place}"));
        let place = self.unsentinel(&displayed, e.range());
        let use_id = ast::ExprRef::from(e).scoped_use_id(self.db, self.pf);
        let ix = self.use_count;
        self.use_count = self.use_count.checked_add(1).expect("flow use ordinal");
        self.current = Some(UseGroup {
            use_: Use {
                scope,
                place,
                span,
                annotation: self.in_annotation || self.lazy(fid),
            },
            capture: None,
            candidates: Vec::new(),
            reaching: Vec::new(),
            _charge: StateCharge::new(self.resources.budget, "flow_use_group"),
        });
        hash_slot(&mut self.use_of, &mut self.charge);
        self.use_of.insert(span, ix);
        self.capture_snapshot(e, fid, ix);
        struct Binding<'db> {
            state: DefinitionState<'db>,
            reach: ScopedReachabilityConstraintId,
            narrowing: FlowCondition,
            lost: bool,
            condition: Option<FlowCondition>,
        }
        let mut bindings = Vec::new();
        for b in self.map(fid).bindings_at_use(use_id) {
            let value = Binding {
                state: b.binding,
                reach: b.reachability_constraint,
                narrowing: narrowing::lower(self.t, &b.narrowing_constraint),
                lost: b
                    .narrowing_constraint
                    .narrowing_constraints()
                    .precision_lost(),
                condition: None,
            };
            self.resources.push(&mut bindings, value);
        }
        // Preserve the native schedule: finish all narrowing preparation before any reaching
        // condition demand, but reuse the same admitted lane instead of a second full Vec.
        for binding in &mut bindings {
            binding.condition = Some(self.condition(fid, binding.reach));
        }
        // False-only admission requires the complete inventory, including Unbound competitors.
        let retain_false_only=!bindings.is_empty()&&bindings.iter().all(|binding| {
            let c=binding.condition.as_ref().expect("lowered reaching condition");
            c.is_never()&&!c.approximated()&&!matches!(binding.state,
                DefinitionState::Defined(d) if matches!(d.kind(self.db),DefinitionKind::LoopHeader(_)))
        });
        for (ordinal, binding) in bindings.into_iter().enumerate() {
            self.resources.checkpoint();
            let Binding {
                state,
                reach,
                narrowing,
                lost: narrowing_precision_lost,
                condition,
            } = binding;
            let condition = condition.expect("complete native candidate preparation");
            let kind = match state {
                DefinitionState::Undefined => CandidateKind::Undefined,
                DefinitionState::Deleted => CandidateKind::Deleted,
                DefinitionState::Defined(d) => match d.kind(self.db) {
                    DefinitionKind::LoopHeader(_) => CandidateKind::LoopHeader,
                    DefinitionKind::NestedBindings(_) => CandidateKind::Nested,
                    _ => CandidateKind::Bound,
                },
            };
            let mut candidate = UseCandidate {
                condition: condition.clone(),
                narrowing: narrowing.clone(),
                narrowing_precision_lost: narrowing_precision_lost || narrowing.approximated(),
                use_ix: ix,
                ordinal: ordinal as u32,
                kind,
                reaching: 0..0,
                pruned: false,
                loop_expanded: kind == CandidateKind::LoopHeader,
                unattached: false,
                reachability_lost: condition.approximated(),
            };
            let before = self.current.as_mut().unwrap().reaching.len();
            if condition.is_never() && !retain_false_only {
                let cause = self.skip_cause(fid, reach);
                self.skips.reach(cause);
                candidate.pruned = true;
                self.push_candidate(candidate);
                continue;
            }
            match state {
                DefinitionState::Defined(d) => {
                    if let DefinitionKind::LoopHeader(h) = d.kind(self.db) {
                        let mut seen = HashSet::new();
                        let expanded = self.loop_bindings(fid, h, &mut seen);
                        for (def_ix, nested, unattached) in expanded {
                            if unattached {
                                candidate.unattached = true;
                                continue;
                            }
                            self.push_reach(Reach {
                                use_ix: ix,
                                def_ix,
                                nested,
                                condition: condition.clone(),
                                narrowing: narrowing.clone(),
                                narrowing_precision_lost,
                                loop_carried: true,
                            });
                        }
                    } else {
                        let def_ix = self.def(scope, d);
                        candidate.unattached = def_ix.is_none() && kind == CandidateKind::Bound;
                        if candidate.unattached {
                            self.push_candidate(candidate);
                            continue;
                        }
                        self.push_reach(Reach {
                            use_ix: ix,
                            def_ix,
                            nested: matches!(d.kind(self.db), DefinitionKind::NestedBindings(_)),
                            condition,
                            narrowing,
                            narrowing_precision_lost,
                            loop_carried: false,
                        });
                    }
                }
                DefinitionState::Undefined | DefinitionState::Deleted => {
                    self.push_reach(Reach {
                        use_ix: ix,
                        def_ix: None,
                        nested: false,
                        condition,
                        narrowing,
                        narrowing_precision_lost,
                        loop_carried: false,
                    });
                }
            }
            candidate.reaching = before..self.current.as_mut().unwrap().reaching.len();
            self.push_candidate(candidate);
        }
        let group = self.current.take().expect("current use group");
        self.emit(ModuleEvent::UseGroup {
            ordinal: ix,
            value: &group,
        });
    }
    fn push_reach(&mut self, value: Reach) {
        self.resources.checkpoint();
        let group = self.current.as_mut().unwrap();
        slots(&mut group.reaching, &mut group._charge, 1);
        group.reaching.push(value);
    }
    fn push_candidate(&mut self, value: UseCandidate) {
        self.resources.checkpoint();
        let group = self.current.as_mut().unwrap();
        slots(&mut group.candidates, &mut group._charge, 1);
        group.candidates.push(value);
    }

    /// Ask ty for the actual enclosing snapshot. Never reinterpret AssumeBound as a
    /// call-time proof and never expand synthetic loop/nested states into convenient values.
    fn capture_snapshot(&mut self, e: &Expr, nested: FileScopeId, use_ix: u32) {
        let Expr::Name(name) = e else { return };
        if nested.is_global() {
            return;
        }
        let table = self.index.place_table(nested);
        let Some(symbol) = table.symbol_by_name(&name.id) else {
            return;
        };
        if symbol.is_local() {
            return;
        }
        let origin = if symbol.is_nonlocal() {
            CaptureOrigin::Nonlocal
        } else if self.index.symbol_resolves_to_global_scope(
            table.symbol_id(&name.id).expect("known symbol"),
            nested,
        ) {
            CaptureOrigin::Global
        } else {
            CaptureOrigin::OuterLocal
        };
        let enclosing =
            self.index
                .visible_ancestor_scopes(nested)
                .skip(1)
                .find_map(|(scope, _)| {
                    if origin == CaptureOrigin::Global {
                        return scope.is_global().then_some(scope);
                    }
                    self.index
                        .place_table(scope)
                        .symbol_by_name(&name.id)
                        .filter(|s| s.is_local())
                        .map(|_| scope)
                });
        let Some(enclosing) = enclosing else { return };
        let Some(scope) = self.scope(enclosing) else {
            return;
        };
        let lazy = self
            .index
            .ancestor_scopes(nested)
            .take_while(|(id, _)| *id != enclosing)
            .any(|(_, scope)| !scope.is_eager());
        let mut constraint = None;
        let mut constraint_precision_lost = false;
        // Collect before calling mutable mapping helpers: the iterator borrows the index.
        let (state, bindings) =
            match self
                .index
                .enclosing_snapshot(enclosing, PlaceExprRef::Symbol(symbol), nested)
            {
                EnclosingSnapshotResult::FoundBindings(bindings) => {
                    let mut owned = Vec::new();
                    for b in bindings {
                        let value = (
                            b.binding,
                            b.reachability_constraint,
                            narrowing::lower(self.t, &b.narrowing_constraint),
                            b.narrowing_constraint
                                .narrowing_constraints()
                                .precision_lost(),
                        );
                        self.resources.push(&mut owned, value);
                    }
                    (SnapshotState::FoundBindings, owned)
                }
                EnclosingSnapshotResult::FoundConstraint(c) => {
                    let evaluator = self.map(enclosing).narrowing_evaluator(c);
                    constraint_precision_lost = evaluator.narrowing_constraints().precision_lost();
                    constraint = Some(narrowing::lower(self.t, &evaluator));
                    (SnapshotState::FoundConstraint, Vec::new())
                }
                EnclosingSnapshotResult::NotFound => (SnapshotState::NotFound, Vec::new()),
                EnclosingSnapshotResult::NoLongerInEagerContext => {
                    (SnapshotState::NoLongerInEagerContext, Vec::new())
                }
            };
        let mut candidates = Vec::new();
        self.resources
            .grow(bindings.len() * size_of::<SnapshotCandidate>());
        candidates.reserve_exact(bindings.len());
        for (binding, reach, narrowing, precision_lost) in bindings {
            let condition = self.condition(enclosing, reach);
            let binding = match binding {
                DefinitionState::Undefined => SnapshotBinding::Undefined,
                DefinitionState::Deleted => SnapshotBinding::Deleted,
                DefinitionState::Defined(d) => match d.kind(self.db) {
                    DefinitionKind::LoopHeader(_) => SnapshotBinding::LoopHeader,
                    DefinitionKind::NestedBindings(_) => SnapshotBinding::Nested,
                    _ => self
                        .scope(d.file_scope(self.db))
                        .and_then(|scope| self.def(scope, d))
                        .map_or(SnapshotBinding::Unattached, SnapshotBinding::Bound),
                },
            };
            candidates.push(SnapshotCandidate {
                binding,
                condition,
                narrowing,
                precision_lost,
            });
        }
        self.current.as_mut().unwrap().capture = Some(CaptureSnapshot {
            use_ix,
            enclosing: scope,
            origin,
            lazy,
            state,
            candidates,
            constraint,
            constraint_precision_lost,
        });
    }

    /// The loop-body definitions a loop header stands for, nested headers expanded.
    fn loop_bindings(
        &mut self,
        fid: FileScopeId,
        h: &ty_python_core::definition::LoopHeaderDefinitionKind,
        seen: &mut HashSet<Definition<'db>>,
    ) -> Vec<(Option<u32>, bool, bool)> {
        let scope = self.scope(fid).expect("a loop is in a real scope");
        let mut pending = Vec::new();
        self.resources.push(&mut pending, h.clone());
        let mut out = Vec::new();
        while let Some(header) = pending.pop() {
            self.resources.checkpoint();
            let mut live = Vec::new();
            for b in self
                .map(fid)
                .loop_header(header.loop_header_id())
                .bindings_for_place(header.place())
            {
                self.resources
                    .push(&mut live, self.map(fid).definition(b.binding()));
            }
            for state in live {
                match state {
                    DefinitionState::Defined(d) if !seen.contains(&d) => {
                        self.resources.grow(128 + size_of::<Definition<'_>>());
                        seen.insert(d);
                        if let DefinitionKind::LoopHeader(inner) = d.kind(self.db) {
                            self.resources.push(&mut pending, inner.clone());
                        } else {
                            let nested =
                                matches!(d.kind(self.db), DefinitionKind::NestedBindings(_));
                            let def_ix = self.def(scope, d);
                            self.resources
                                .push(&mut out, (def_ix, nested, def_ix.is_none() && !nested));
                        }
                    }
                    DefinitionState::Defined(_) => {}
                    DefinitionState::Undefined | DefinitionState::Deleted => {
                        self.resources.push(&mut out, (None, false, false))
                    }
                }
            }
        }
        out
    }

    /// The reachability of the statement at `range` in scope `fid`: the last recorded entry
    /// containing its start (entries are recorded in visit order, a body after its header).
    fn region(&mut self, fid: FileScopeId, range: TextRange) -> FlowCondition {
        if !self.regions.contains_key(&fid) {
            let mut entries = Vec::new();
            for entry in self.map(fid).range_reachability() {
                self.resources.push(&mut entries, entry);
            }
            let mut out = Vec::new();
            admit(
                &mut self.charge,
                entries.len() * size_of::<(TextRange, FlowCondition)>(),
            );
            out.reserve_exact(entries.len());
            for (r, id) in entries {
                out.push((r, self.condition(fid, id)));
            }
            hash_slot(&mut self.regions, &mut self.charge);
            self.regions.insert(fid, out);
        }
        self.regions[&fid]
            .iter()
            .rev()
            .find(|(r, _)| r.contains_range(TextRange::empty(range.start())))
            .map_or_else(
                || FlowCondition::terminal(self.resources.budget, Some(true)),
                |(_, c)| c.clone(),
            )
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
        cond: &FlowCondition,
        false_cause: Option<SkipCause>,
        identity: bool,
        call_path: &[CallFrame],
    ) {
        let resources = self.resources;
        let _scratch = resources.scope();
        resources.checkpoint();
        if cond.is_never() {
            // Preserve an explicit source candidate with its checked-false condition. Its
            // actual native use and reaching evidence still govern Entry/Local admission;
            // neither a missing use nor a missing formal origin is reconstructed here.
            // Computed values and call crossings retain the existing refusal boundary.
            if !identity || !call_path.is_empty() || !matches!(e, Expr::Name(_)) {
                self.skips.value(false_cause.unwrap_or(SkipCause::Stable));
                return;
            }
        }
        match e {
            Expr::Name(_) | Expr::Attribute(_) | Expr::Subscript(_)
                if admitted::place_expr(e, self.resources).is_some() =>
            {
                if let Some(&u) = self.use_of.get(&Span::from(e.range())) {
                    self.emit(ModuleEvent::Value(ValueSourceRef {
                        sink,
                        span: sink_span,
                        use_ix: u,
                        identity: identity && call_path.is_empty(),
                        through_call: !call_path.is_empty(),
                        call_path,
                        condition: cond,
                    }));
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
                            call_path,
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
                            call_path,
                        );
                        self.sources(
                            sink,
                            sink_span,
                            &s.slice,
                            cond,
                            false_cause,
                            false,
                            call_path,
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
                self.sources(sink, sink_span, &i.body, &body, cause, identity, call_path);
                self.sources(
                    sink, sink_span, &i.orelse, &orelse, cause, identity, call_path,
                );
                self.sources(
                    sink,
                    sink_span,
                    &i.test,
                    cond,
                    false_cause,
                    false,
                    call_path,
                );
            }
            Expr::BoolOp(b) => {
                // `a or b`: `a` when truthy, else `b`; `a and b`: `a` when falsy, else `b`.
                let mut before = FlowCondition::terminal(self.resources.budget, Some(true));
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
                    self.sources(sink, sink_span, v, &here, false_cause, identity, call_path);
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
                    call_path,
                );
            }
            Expr::Call(call) => {
                let call_span = call.range().into();
                self.resources
                    .grow((call_path.len() + 1) * size_of::<CallFrame>());
                let mut callee_path = Vec::with_capacity(call_path.len() + 1);
                callee_path.extend_from_slice(call_path);
                callee_path.push(CallFrame {
                    call: call_span,
                    operand: call.func.range().into(),
                    role: FlowCallOperandRole::Callee,
                });
                self.sources(
                    sink,
                    sink_span,
                    &call.func,
                    cond,
                    false_cause,
                    false,
                    &callee_path,
                );
                for arg in &call.arguments.args {
                    // The call syntax owns the actual operand, including a starred wrapper.
                    // Uses inside that wrapper remain descendants of this crossing.
                    let operand = arg.range();
                    self.resources
                        .grow((call_path.len() + 1) * size_of::<CallFrame>());
                    let mut argument_path = Vec::with_capacity(call_path.len() + 1);
                    argument_path.extend_from_slice(call_path);
                    argument_path.push(CallFrame {
                        call: call_span,
                        operand: operand.into(),
                        role: FlowCallOperandRole::Argument,
                    });
                    self.sources(
                        sink,
                        sink_span,
                        arg,
                        cond,
                        false_cause,
                        false,
                        &argument_path,
                    );
                }
                for keyword in &call.arguments.keywords {
                    self.resources
                        .grow((call_path.len() + 1) * size_of::<CallFrame>());
                    let mut argument_path = Vec::with_capacity(call_path.len() + 1);
                    argument_path.extend_from_slice(call_path);
                    argument_path.push(CallFrame {
                        call: call_span,
                        operand: keyword.value.range().into(),
                        role: FlowCallOperandRole::Argument,
                    });
                    self.sources(
                        sink,
                        sink_span,
                        &keyword.value,
                        cond,
                        false_cause,
                        false,
                        &argument_path,
                    );
                }
            }
            _ => {
                // Computed: each sub-expression's uses, derived, under the conditions nested
                // conditional expressions and boolean operators select them by. Everything inside
                // a call is handled above with an attributed operand role.
                for child in children(e, self.resources) {
                    self.sources(sink, sink_span, child, cond, false_cause, false, call_path);
                }
            }
        }
    }
}

/// The expressions directly inside `e`, in source order (a comprehension's, a lambda's and an
/// f-string's included).
fn children<'a>(e: &'a Expr, resources: &Resources<'_>) -> Vec<&'a Expr> {
    struct V<'a, 'r>(Vec<&'a Expr>, &'r Resources<'r>);
    impl<'a> SourceOrderVisitor<'a> for V<'a, '_> {
        fn visit_expr(&mut self, e: &'a Expr) {
            self.1.push(&mut self.0, e);
        }
    }
    let mut v = V(Vec::new(), resources);
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
            &FlowCondition::terminal(self.w.resources.budget, Some(true)),
            None,
            identity,
            &[],
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
                slots(&mut self.stack, &mut self.w.charge, 1);
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
        self.w.resources.checkpoint();
        TraversalSignal::Traverse
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        let resources = self.w.resources;
        let _scratch = resources.scope();
        resources.checkpoint();
        let fid = *self
            .stack
            .last()
            .expect("the module scope is at the bottom");
        if let Some(scope) = self.w.scope(fid) {
            let condition = self.w.region(fid, stmt.range());
            self.w.emit(ModuleEvent::Region(&Region {
                scope,
                span: stmt.range().into(),
                condition,
            }));
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
            Stmt::AugAssign(a) if !self.aug.contains(&a.target.range()) => {
                admit(&mut self.w.charge, 128 + size_of::<TextRange>());
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
                    &FlowCondition::terminal(self.w.resources.budget, Some(true)),
                    None,
                    false,
                    &[],
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
        let resources = self.w.resources;
        let _scratch = resources.scope();
        resources.checkpoint();
        source_order::walk_expr(self, e);
        if !self.w.in_annotation {
            match e {
                Expr::Attribute(a) if matches!(a.ctx, ExprContext::Load) => {
                    self.w.emit(ModuleEvent::Attribute(AttributeLoadRef {
                        span: Span::from(e.range()),
                        name: a.attr.id.as_str(),
                    }));
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
        if is_use && admitted::place_expr(e, self.w.resources).is_some() {
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
                            &FlowCondition::terminal(self.w.resources.budget, Some(true)),
                            None,
                            false,
                            &[],
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

#[cfg(test)]
extern crate self as cpg_flow;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_support;
#[cfg(test)]
mod settings_tests {
    use super::{
        BindingKind, FlowDb, FlowOperationalError, Input, ModelError, ModuleEvent, Program,
        ResourceBudget, RuntimeBindings, RuntimeContext, VisitError, program_settings,
    };
    use ruff_db::system::DbWithWritableSystem;
    use ty_module_resolver::{ModuleName, resolve_module_confident};

    thread_local! {
        static EVENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    pub(super) fn record_query(path: &str) {
        EVENTS.with_borrow_mut(|events| events.push(format!("query:{path}")));
    }

    fn budget() -> super::ResourceBudget {
        super::ResourceBudget::fixed(64 << 20).unwrap()
    }
    fn index(inputs: &[Input], context: &RuntimeContext) -> crate::test_support::CollectedModules {
        crate::test_support::index(inputs, context)
    }
    #[test]
    fn empty_module_inventory_needs_no_virtual_import_root() {
        let context = RuntimeContext {
            python_version: (3, 14, 0),
            platform: "linux".to_owned(),
        };
        assert!(index(&[], &context).is_empty());
    }

    fn context() -> RuntimeContext {
        RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".into(),
        }
    }

    fn input(path: &str, text: &str) -> Input {
        Input {
            path: path.into(),
            text: text.into(),
            runtime: RuntimeBindings::default(),
        }
    }

    // Raw fixtures are copied only after admission; production has no borrowed-input runner.
    fn visit_modules<E, G>(
        inputs: &[Input],
        context: &RuntimeContext,
        budget: &ResourceBudget,
        check: &dyn Fn() -> Result<(), ModelError>,
        register: impl Fn(salsa::CancellationToken) -> G,
        visit: impl for<'a> FnMut(usize, ModuleEvent<'a>) -> Result<(), E>,
    ) -> Result<(), VisitError<E>> {
        let (inputs, _backing) = crate::test_support::admit_inputs(inputs, budget)
            .map_err(|error| VisitError::Operational(FlowOperationalError::Model(error)))?;
        super::visit_modules(inputs, context, budget, check, register, visit)
    }

    #[test]
    fn syntax_kind_mapping_preserves_scalar_and_uncommon_kinds() {
        use lctx_model::domain::{Codebook, source::SyntaxKind};
        use ruff_python_ast_ty::NodeKind;
        for (native, expected, code) in [
            (NodeKind::ModModule, SyntaxKind::ModModule, 0),
            (NodeKind::ExprName, SyntaxKind::ExprName, 55),
            (
                NodeKind::InterpolatedElement,
                SyntaxKind::InterpolatedElement,
                61,
            ),
            (
                NodeKind::PatternMatchMapping,
                SyntaxKind::PatternMatchMapping,
                66,
            ),
            (
                NodeKind::TypeParamParamSpec,
                SyntaxKind::TypeParamParamSpec,
                73,
            ),
            (
                NodeKind::ParameterWithDefault,
                SyntaxKind::ParameterWithDefault,
                81,
            ),
            (NodeKind::TString, SyntaxKind::TString, 90),
            (NodeKind::Identifier, SyntaxKind::Identifier, 93),
        ] {
            assert_eq!(super::syntax_kind(native), expected);
            assert_eq!(SyntaxKind::from_code(code), Some(expected));
        }
    }

    #[test]
    fn admitted_inputs_reject_missing_backing_and_foreign_pool_before_native_work() {
        let unadmitted = input("source.py", "x = 1\n");
        assert!(matches!(
            super::AdmittedInput::new(unadmitted, Default::default()),
            Err(ModelError::Conflict(_))
        ));
        let owner = budget();
        let other = budget();
        let (inputs, backing) =
            crate::test_support::admit_inputs(&[input("source.py", "x = 1\n")], &owner).unwrap();
        EVENTS.with_borrow_mut(Vec::clear);
        let result = super::visit_modules::<(), _>(
            inputs,
            &context(),
            &other,
            &|| panic!("foreign input reached checkpoint"),
            |_| panic!("foreign input registered native handle"),
            |_, _| panic!("foreign input emitted"),
        );
        assert!(matches!(
            result,
            Err(VisitError::Operational(FlowOperationalError::Model(
                ModelError::Conflict(_)
            )))
        ));
        EVENTS.with_borrow(|events| assert!(events.is_empty()));
        drop(backing);
        assert_eq!(owner.reserved(), 0);
        assert_eq!(other.reserved(), 0);
    }

    #[test]
    fn original_source_charges_end_after_each_module_callback() {
        use lctx_model::domain::resources::{Reservation, ResourcePool};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        #[derive(Debug)]
        struct SourcePool {
            inner: ResourceBudget,
            sources: Arc<AtomicUsize>,
        }
        #[derive(Debug)]
        struct SourceReservation {
            inner: Box<dyn Reservation>,
            sources: Arc<AtomicUsize>,
        }
        impl Drop for SourceReservation {
            fn drop(&mut self) {
                self.sources.fetch_sub(self.inner.size(), Ordering::SeqCst);
            }
        }
        impl Reservation for SourceReservation {
            fn size(&self) -> usize {
                self.inner.size()
            }
            fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
                let previous = self.inner.size();
                self.inner.try_resize(bytes)?;
                if bytes >= previous {
                    self.sources.fetch_add(bytes - previous, Ordering::SeqCst);
                } else {
                    self.sources.fetch_sub(previous - bytes, Ordering::SeqCst);
                }
                Ok(())
            }
        }
        impl ResourcePool for SourcePool {
            fn reserve(
                &self,
                owner: &'static str,
                bytes: usize,
            ) -> Result<Box<dyn Reservation>, ModelError> {
                let inner = self.inner.reserve(owner, bytes)?;
                if owner == "flow-test-input" {
                    self.sources.fetch_add(bytes, Ordering::SeqCst);
                    Ok(Box::new(SourceReservation {
                        inner,
                        sources: self.sources.clone(),
                    }))
                } else {
                    Ok(inner)
                }
            }
            fn reserved(&self) -> usize {
                self.inner.reserved()
            }
            fn limit(&self) -> usize {
                self.inner.limit()
            }
        }
        for first in ["x = 1\ny = x\n", "TYPE_CHECKIN_ = 1\n"] {
            let sources = Arc::new(AtomicUsize::new(0));
            let resources = ResourceBudget::from_pool(Arc::new(SourcePool {
                inner: budget(),
                sources: sources.clone(),
            }))
            .unwrap();
            let inputs = [input("first.py", first), input("later.py", "z = 2\n")];
            let first_bytes = inputs[0].owned_bytes();
            let later_bytes = inputs[1].owned_bytes();
            let mut completed = 0;
            visit_modules(
                &inputs,
                &context(),
                &resources,
                &|| Ok(()),
                |_| (),
                |ix, event| {
                    assert_eq!(
                        sources.load(Ordering::SeqCst),
                        if ix == 0 {
                            first_bytes + later_bytes
                        } else {
                            later_bytes
                        }
                    );
                    if matches!(event, ModuleEvent::End(_) | ModuleEvent::ModuleFailure(_)) {
                        completed += 1;
                    }
                    Ok::<_, ()>(())
                },
            )
            .unwrap();
            assert_eq!(completed, 2);
            assert_eq!(sources.load(Ordering::SeqCst), 0);
            assert_eq!(resources.reserved(), 0);
        }
    }

    #[test]
    fn clearing_native_ast_reparses_for_a_later_wildcard_query() {
        use ruff_db::{files::system_path_to_file, parsed::parsed_module};
        use ty_python_core::{ProgramFile, semantic_index};
        let mut db = FlowDb::new();
        db.write_file("/flow/exporter.py", "VALUE = 1\n").unwrap();
        db.write_file(
            "/flow/consumer.py",
            "from exporter import *\nresult = VALUE\n",
        )
        .unwrap();
        let program = Program::from_settings(&db, &program_settings(&db, &context()));
        let exporter = ProgramFile::new(
            &db,
            system_path_to_file(&db, "/flow/exporter.py").unwrap(),
            program,
        );
        let consumer = ProgramFile::new(
            &db,
            system_path_to_file(&db, "/flow/consumer.py").unwrap(),
            program,
        );
        let native = parsed_module(&db, exporter.python_file(&db));
        let outstanding = native.load(&db);
        semantic_index(&db, exporter);
        native.clear();
        // Pinned ParsedModule Debug exposes ArcSwap's stored option without calling load.
        // A subsequent load would itself reparse, so it cannot establish query timing.
        assert!(format!("{native:?}").contains("ArcSwapAny(None)"));
        semantic_index(&db, consumer);
        assert!(
            !format!("{native:?}").contains("ArcSwapAny(None)"),
            "the late exported-name query rebuilt the cleared dependency AST"
        );
        let rebuilt = native.load(&db);
        assert!(!std::ptr::eq(outstanding.syntax(), rebuilt.syntax()));
        assert_eq!(
            outstanding.suite().len(),
            1,
            "existing borrower remains valid"
        );
        assert_eq!(rebuilt.suite().len(), 1);
        assert_eq!(outstanding.tokens().as_ref(), rebuilt.tokens().as_ref());
    }

    #[test]
    fn visitor_emits_before_later_queries_and_refusal_stops_them() {
        let inputs = [
            input("first.py", "a = 1\nb = a\n"),
            input("later.py", "c = 2\nd = c\n"),
        ];
        EVENTS.with_borrow_mut(Vec::clear);
        visit_modules(
            &inputs,
            &context(),
            &budget(),
            &|| Ok(()),
            |_| (),
            |ix, event| {
                if matches!(event, super::ModuleEvent::End(_)) {
                    EVENTS.with_borrow_mut(|events| events.push(format!("emit:{ix}")));
                }
                Ok::<_, &str>(())
            },
        )
        .unwrap();
        EVENTS.with_borrow(|events| {
            assert_eq!(
                events,
                &[
                    "query:/flow/first.py",
                    "emit:0",
                    "query:/flow/later.py",
                    "emit:1",
                ]
            )
        });
        for refusal in ["sink refused", "cancelled"] {
            EVENTS.with_borrow_mut(Vec::clear);
            let result = visit_modules(
                &inputs,
                &context(),
                &budget(),
                &|| Ok(()),
                |_| (),
                |ix, event| {
                    assert_eq!(ix, 0);
                    assert!(matches!(event, super::ModuleEvent::Prelude(_)));
                    Err(refusal)
                },
            );
            assert!(matches!(result,Err(super::VisitError::Consumer(error)) if error==refusal));
            EVENTS.with_borrow(|events| assert_eq!(events, &["query:/flow/first.py"]));
        }
    }

    #[test]
    fn visitor_preserves_failed_module_correspondence_and_continues() {
        let inputs = [
            input("bad.py", "TYPE_CHECKIN_ = 1\n"),
            input("recovered.py", "x = (\ny = 1\n"),
            input("good.py", "x = 1\ny = x\n"),
        ];
        let flows = index(&inputs, &context());
        assert_eq!(flows.len(), 3);
        assert_eq!(flows[0].path, "bad.py");
        assert!(flows[0].error.as_deref().unwrap().contains("already names"));
        assert!(flows[0].original_content.is_none());
        assert_eq!(flows[1].path, "recovered.py");
        assert!(flows[1].error.is_none());
        assert!(flows[1].syntax_errors > 0);
        assert_eq!(flows[2].path, "good.py");
        assert!(flows[2].error.is_none());
        assert!(flows[2].uses.iter().any(|u| u.place == "x"));
        assert_eq!(flows[2].view_content, flows[2].original_content);
        let result = visit_modules::<(), _>(
            &[],
            &context(),
            &budget(),
            &|| Ok(()),
            |_| (),
            |_, _| panic!("empty inventory queried"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn visitor_preserves_cross_module_facts_and_runtime_view_identity() {
        // The consumer is queried first, while its dependency already exists in the same Program.
        let inputs = [
            input("consumer.py", "from helper import VALUE\nresult = VALUE\n"),
            input(
                "helper.py",
                "TYPE_CHECKING = False\nVALUE = TYPE_CHECKING\n",
            ),
        ];
        let flows = index(&inputs, &context());
        assert!(flows.iter().all(|flow| flow.error.is_none()));
        assert!(
            flows[0]
                .defs
                .iter()
                .any(|d| d.kind == BindingKind::FromImport && d.place == "VALUE")
        );
        let read = flows[0]
            .uses
            .iter()
            .position(|u| u.place == "VALUE")
            .unwrap();
        assert!(
            flows[0]
                .reaching
                .iter()
                .any(|r| r.use_ix as usize == read && r.def_ix.is_some())
        );
        assert!(flows[1].uses.iter().any(|u| u.place == "TYPE_CHECKING"));
        for (input, flow) in inputs.iter().zip(flows.iter()) {
            let (view, renamed) = super::rename(&input.text).unwrap();
            assert_eq!(
                flow.original_content,
                Some(lctx_model::domain::ContentHash::of(input.text.as_bytes()))
            );
            assert_eq!(
                flow.view_content,
                Some(lctx_model::domain::ContentHash::of(view.as_bytes()))
            );
            assert_eq!(flow.view_byte_len, Some(input.text.len()));
            assert_eq!(flow.renamed, renamed);
        }
        assert_ne!(flows[1].original_content, flows[1].view_content);
    }

    #[test]
    fn operational_refusal_and_native_cancellation_are_not_model_failures() {
        let inputs = [input("source.py", "x = 1\ny = x\n")];
        let tiny = super::ResourceBudget::fixed(1).unwrap();
        let calls = std::cell::Cell::new(0);
        let result = visit_modules(
            &inputs,
            &context(),
            &tiny,
            &|| Ok(()),
            |_| (),
            |_, _| {
                calls.set(calls.get() + 1);
                Ok::<_, ()>(())
            },
        );
        assert!(matches!(
            result,
            Err(super::VisitError::Operational(
                super::FlowOperationalError::Model(lctx_model::domain::ModelError::Resource { .. })
            ))
        ));
        assert_eq!(calls.get(), 0);
        assert_eq!(tiny.reserved(), 0);
        let resources = budget();
        let result = visit_modules(
            &inputs,
            &context(),
            &resources,
            &|| Ok(()),
            |token| token.cancel(),
            |_, _| {
                calls.set(calls.get() + 1);
                Ok::<_, ()>(())
            },
        );
        assert!(matches!(
            result,
            Err(super::VisitError::Operational(
                super::FlowOperationalError::NativeCancelled(_)
            ))
        ));
        assert_eq!(calls.get(), 0);
        assert_eq!(resources.reserved(), 0);
    }

    #[test]
    fn callback_panics_are_rethrown_and_actual_ownership_drains() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct Guard(Arc<AtomicBool>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        for public_payload in [false, true] {
            let dropped = Arc::new(AtomicBool::new(false));
            let resources = budget();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                visit_modules::<(), _>(
                    &[input("a.py", "x = 1\n")],
                    &context(),
                    &resources,
                    &|| Ok(()),
                    |_| Guard(dropped.clone()),
                    |_, _| {
                        if public_payload {
                            std::panic::panic_any(super::FlowOperationalError::Model(
                                lctx_model::domain::ModelError::Invalid(
                                    "foreign public error".into(),
                                ),
                            ));
                        }
                        panic!("ordinary callback panic");
                    },
                )
            }));
            let payload = result.unwrap_err();
            if public_payload {
                assert!(payload.is::<super::FlowOperationalError>());
            } else {
                assert!(payload.is::<&str>());
            }
            assert!(dropped.load(Ordering::Acquire));
            assert_eq!(resources.reserved(), 0);
        }
    }

    #[test]
    fn cancellation_between_complete_groups_prevents_module_end_and_later_query() {
        let cancelled = std::cell::Cell::new(false);
        let groups = std::cell::Cell::new(0);
        let ends = std::cell::Cell::new(0);
        let resources = budget();
        EVENTS.with_borrow_mut(Vec::clear);
        let result = visit_modules(
            &[
                input("first.py", "x = 1\na = x\nb = x\n"),
                input("later.py", "z = 2\n"),
            ],
            &context(),
            &resources,
            &|| {
                if cancelled.get() {
                    Err(lctx_model::domain::ModelError::Invalid("cancelled".into()))
                } else {
                    Ok(())
                }
            },
            |_| (),
            |_, event| {
                match event {
                    super::ModuleEvent::UseGroup { .. } => {
                        groups.set(groups.get() + 1);
                        cancelled.set(true);
                    }
                    super::ModuleEvent::End(_) => ends.set(ends.get() + 1),
                    _ => {}
                }
                Ok::<_, ()>(())
            },
        );
        assert!(matches!(
            result,
            Err(super::VisitError::Operational(
                super::FlowOperationalError::Model(_)
            ))
        ));
        assert_eq!(groups.get(), 1);
        assert_eq!(ends.get(), 0);
        assert_eq!(resources.reserved(), 0);
        EVENTS.with_borrow(|events| assert_eq!(events, &["query:/flow/first.py"]));
    }

    #[test]
    fn first_value_refusal_stops_boolean_and_call_siblings_with_original_non_send_error() {
        use std::{cell::Cell, rc::Rc};
        for expression in ["a or b or c or d or e or f", "callee(a, b, c, d, e, f)"] {
            let resources = budget();
            let checkpoints = Cell::new(0);
            let stopped = Cell::new(false);
            let at_stop = Cell::new(0);
            let peak_at_stop = Cell::new(0);
            let ends = Cell::new(0);
            let borrowed = Cell::new(7);
            let marker = Rc::new(String::from("original value refusal"));
            let source = format!(
                "def example(a,b,c,d,e,f,callee):\n    result = {expression}\n    later = a\n"
            );
            let result = visit_modules(
                &[input("first.py", &source), input("later.py", "x = 1\n")],
                &context(),
                &resources,
                &|| {
                    checkpoints.set(checkpoints.get() + 1);
                    if stopped.get() {
                        Err(lctx_model::domain::ModelError::Invalid(
                            "late checkpoint replaced callback".into(),
                        ))
                    } else {
                        Ok(())
                    }
                },
                |_| (),
                |_, event| {
                    if matches!(event, super::ModuleEvent::End(_)) {
                        ends.set(ends.get() + 1);
                    }
                    if matches!(event, super::ModuleEvent::Value(_)) {
                        stopped.set(true);
                        at_stop.set(checkpoints.get());
                        peak_at_stop.set(resources.peak().unwrap());
                        Err((&borrowed, marker.clone()))
                    } else {
                        Ok(())
                    }
                },
            );
            let Err(super::VisitError::Consumer((returned, reason))) = result else {
                panic!("callback error was replaced");
            };
            assert!(std::ptr::eq(returned, &borrowed));
            assert!(Rc::ptr_eq(&reason, &marker));
            assert_eq!(checkpoints.get(), at_stop.get());
            assert_eq!(resources.peak().unwrap(), peak_at_stop.get());
            assert_eq!(ends.get(), 0);
            assert_eq!(resources.reserved(), 0);
        }
    }

    #[test]
    fn complete_groups_serve_growth_without_retaining_the_expanded_module() {
        let mut text = String::from("def source(x, p):\n    if p:\n        x = 1\n");
        for i in 0..1500 {
            use std::fmt::Write;
            writeln!(&mut text, "    value_{i} = x").unwrap();
        }
        let inputs = [input("growing.py", &text)];
        let run = |resources: &super::ResourceBudget| {
            let mut prelude = None;
            let mut count = 0;
            let mut reaching = 0;
            visit_modules(
                &inputs,
                &context(),
                resources,
                &|| Ok(()),
                |_| (),
                |_, event| {
                    match event {
                        super::ModuleEvent::Prelude(value) => prelude = Some(value),
                        super::ModuleEvent::UseGroup { value, .. } => {
                            count += 1;
                            reaching += value.reaching.len();
                        }
                        super::ModuleEvent::End(_) => {
                            drop(prelude.take());
                        }
                        _ => {}
                    }
                    Ok::<_, ()>(())
                },
            )
            .unwrap();
            assert!(count >= 1500);
            assert!(reaching >= 1500);
            assert_eq!(resources.reserved(), 0);
            (count, reaching)
        };
        let roomy = super::ResourceBudget::fixed(256 << 20).unwrap();
        let expected = run(&roomy);
        let allowance = roomy.peak().unwrap() + 4096;
        let narrow = super::ResourceBudget::fixed(allowance).unwrap();
        assert_eq!(run(&narrow), expected);
        let collected = crate::test_support::index_with_budget(&inputs, &context(), &narrow);
        assert!(matches!(
            collected,
            Err(super::VisitError::Consumer(
                lctx_model::domain::ModelError::Resource { .. }
            )) | Err(super::VisitError::Operational(
                super::FlowOperationalError::Model(lctx_model::domain::ModelError::Resource { .. })
            ))
        ));
        assert_eq!(narrow.reserved(), 0);
    }

    fn lexed_tokens(text: &str) -> Vec<ruff_python_ast_ty::token::Token> {
        use ruff_python_ast_ty::token::Token;
        let mut lexer =
            ruff_python_parser_ty::lexer::lex(text, ruff_python_parser_ty::Mode::Module);
        let mut tokens = Vec::new();
        loop {
            let kind = lexer.next_token();
            if kind.is_eof() {
                break;
            }
            tokens.push(Token::new(
                kind,
                lexer.current_range(),
                lexer.current_flags(),
            ));
        }
        tokens
    }

    #[test]
    fn lexer_only_matches_valid_tokens_but_not_malformed_interpolation_recovery() {
        use ruff_python_ast_ty::PySourceType;
        let valid = [
            "TYPE_CHECKING = 1\n",
            "# TYPE_CHECKING TYPE_CHECKIN_\nx = 'TYPE_CHECKING TYPE_CHECKIN_'\n",
            "x = f'{TYPE_CHECKING!r:>{TYPE_CHECKING}}'\n",
            "x = t'{TYPE_CHECKING}'\n",
            "x = f'{f\"{TYPE_CHECKING}\"}'\n",
            "\u{feff}α = TYPE_CHECKING\r\nβ = 'TYPE_CHECKING'\r\n",
            "TYPE_CHECKIN_ = 1\n",
            "x = f'{TYPE_CHECKIN_}'\n",
        ];
        for text in valid {
            let parsed = ruff_python_parser_ty::parse_unchecked_source(text, PySourceType::Python);
            assert!(
                parsed.errors().is_empty(),
                "{text:?}: {:?}",
                parsed.errors()
            );
            assert_eq!(
                lexed_tokens(text).as_slice(),
                parsed.tokens().as_ref(),
                "{text:?}"
            );
        }
        for (text, expected) in [
            (
                "x = f'{TYPE_CHECKING'\nTYPE_CHECKING = 1\n",
                "x = f'{TYPE_CHECKIN_'\nTYPE_CHECKIN_ = 1\n",
            ),
            (
                "x = t'{TYPE_CHECKING'\nTYPE_CHECKING = 1\n",
                "x = t'{TYPE_CHECKIN_'\nTYPE_CHECKIN_ = 1\n",
            ),
            (
                "x = f'{TYPE_CHECKING!r'\nTYPE_CHECKING = 1\n",
                "x = f'{TYPE_CHECKIN_!r'\nTYPE_CHECKIN_ = 1\n",
            ),
        ] {
            let parsed = ruff_python_parser_ty::parse_unchecked_source(text, PySourceType::Python);
            assert!(!parsed.errors().is_empty());
            assert_ne!(
                lexed_tokens(text).as_slice(),
                parsed.tokens().as_ref(),
                "{text:?}"
            );
            let (renamed, count) = super::rename(text).unwrap();
            assert_eq!(count, 2, "{text:?}");
            assert_eq!(renamed.len(), text.len());
            assert_eq!(renamed, expected);
        }
        assert!(super::rename("x = f'{TYPE_CHECKING'\nTYPE_CHECKIN_ = 1\n").is_err());
    }

    #[test]
    fn virtual_import_root_and_python_version_affect_resolution() {
        let mut db = FlowDb::new();
        db.write_file("/flow/consumer.py", "import helper\n")
            .unwrap();
        db.write_file("/flow/helper.py", "VALUE = 1\n").unwrap();
        let context = |minor| RuntimeContext {
            python_version: (3, minor, 0),
            platform: "linux".to_owned(),
        };
        let old = Program::from_settings(&db, &program_settings(&db, &context(8)));
        let current = Program::from_settings(&db, &program_settings(&db, &context(14)));
        let resolved = |program: Program<'_>, name| {
            resolve_module_confident(
                &db,
                program.resolver_environment(&db),
                &ModuleName::new(name).unwrap(),
            )
            .is_some()
        };
        assert!(resolved(current, "helper"));
        assert!(!resolved(old, "tomllib"));
        assert!(resolved(current, "tomllib"));
    }

    #[test]
    fn package_submodule_definition_keeps_its_provider_kind() {
        let source = "from pkg.child import Item\n";
        let inputs = [
            Input {
                path: "pkg/__init__.py".to_owned(),
                text: source.to_owned(),
                runtime: RuntimeBindings::default(),
            },
            Input {
                path: "pkg/child.py".to_owned(),
                text: "class Item: pass\n".to_owned(),
                runtime: RuntimeBindings::default(),
            },
        ];
        let flows = index(
            &inputs,
            &RuntimeContext {
                python_version: (3, 14, 0),
                platform: "linux".to_owned(),
            },
        );
        assert!(flows.iter().all(|flow| flow.error.is_none()), "{flows:?}");
        let defs = &flows[0].defs;
        assert!(
            defs.iter().any(|d| {
                d.kind == BindingKind::ImportFromSubmodule
                && d.place == "child"
                // At ty 0.0.14 this variant's fallback range is the full module path.
                && &source[d.target.start as usize..d.target.end as usize] == "pkg.child"
            }),
            "{defs:?}"
        );
        assert!(
            defs.iter().any(|d| {
                d.kind == BindingKind::FromImport
                    && d.place == "Item"
                    && &source[d.target.start as usize..d.target.end as usize] == "Item"
            }),
            "{defs:?}"
        );
    }
}
