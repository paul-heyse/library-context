//! Typed syntax emission from Pyrefly's retained Ruff parse (ADR-0085).
//!
//! This is an identifier-observation subset, not complete syntax-family coverage. The caller
//! supplies the AST and text from the same pinned transaction, retains every captured byte,
//! and publishes only after the generation's shared invariants succeed. No legacy row or ID
//! enters this boundary. Emission is incremental: a sink or model error aborts the attempt; a
//! declared bound refuses the artifact, and [`SyntaxError::coverage`] states what it publishes.
use lctx_model::domain::{Batch, ContentHash, ModelError, Record, ValidatedModel, admission::ArtifactClass, assertion::*, attribution::*,
    batching::{BatchWriter, TransferLimits}, conditions::{Condition, ConditionNode, Diagram}, resources::ResourceBudget, source::*, stages::ProviderOutcome};
use crate::capture::CapturedInput;
use ruff_python_ast::{Alias, AnyNodeRef, Arguments, BoolOp, BytesLiteral, CmpOp, Comprehension, Decorator, ElifElseClause, ExceptHandler, Expr,
    ExprContext, FString, Identifier, InterpolatedStringElement, Keyword, MatchCase, Mod, ModModule, NodeKind, Operator, Parameter,
    ParameterWithDefault, Parameters, Pattern, PatternArguments, PatternKeyword, Singleton, Stmt, StringLiteral, TString, TypeParam,
    TypeParams, UnaryOp, WithItem};
use ruff_python_ast::visitor::source_order::{self, SourceOrderVisitor, TraversalSignal};
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

/// Deterministic refusal bounds on one artifact's syntax traversal, independent of display limits.
/// The source is admitted before parsing, which bounds the tree and so also the residual
/// callbacks Ruff makes into a halted visitor: one immediate return per remaining child slot.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxLimits { pub nodes: usize, pub depth: usize, pub source_bytes: usize, pub callbacks: usize }
impl Default for SyntaxLimits {
    fn default() -> Self { Self { nodes: 1_000_000, depth: 256, source_bytes: 16 << 20, callbacks: 8_000_000 } }
}
/// The traversal work of one emission: nodes emitted, visitor callbacks before the traversal
/// halted, residual callbacks after it halted, and the deepest structural path entered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyntaxWork { pub emitted: usize, pub callbacks: usize, pub residual: usize, pub deepest: usize }
/// The bound that refused an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxLimit { SourceBytes, Nodes, Depth, Callbacks }
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    /// A declared bound refused the artifact after `work`.
    #[error("syntax {limit:?} limit refused the artifact ({work:?})")]
    Refused { limit: SyntaxLimit, work: SyntaxWork },
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
            Self::Refused { limit: SyntaxLimit::SourceBytes, .. } => Some((CoverageStatus::Unavailable, ObligationKind::ResourceRefused)),
            Self::Refused { .. } => Some((CoverageStatus::Partial, ObligationKind::ResourceRefused)),
            Self::Model(_) => None,
        }
    }
}

/// Admit an artifact's source size before its syntax is traversed. An oversized source is never
/// parsed only if the provider calls this before it is handed the source; the Pyrefly stage owes
/// that call (plan E1/A4), and until then the parse is outside the budget.
pub fn admit(source: &SourceArtifact, limits: SyntaxLimits) -> Result<(), SyntaxError> {
    match usize::try_from(source.byte_len) {
        Ok(bytes) if bytes <= limits.source_bytes => Ok(()),
        _ => Err(SyntaxError::Refused { limit: SyntaxLimit::SourceBytes, work: SyntaxWork::default() }),
    }
}

pub fn emit(
    ast: &ModModule, text: &str, invocation: SyntaxInvocation<'_>, limits: SyntaxLimits,
    mut output: impl FnMut(SyntaxEvent) -> Result<(), ModelError>,
) -> Result<SyntaxWork, SyntaxError> {
    if limits.nodes == 0 || limits.depth == 0 || limits.depth > 256 || limits.callbacks == 0 {
        return Err(invalid("invalid syntax traversal limits").into());
    }
    admit(invocation.source, limits)?;
    if text.len() > limits.source_bytes { return Err(SyntaxError::Refused { limit: SyntaxLimit::SourceBytes, work: SyntaxWork::default() }); }
    if invocation.source.content != ContentHash::of(text.as_bytes())
        || usize::try_from(invocation.source.byte_len).ok() != Some(text.len()) {
        return Err(invalid("analyzer text differs from captured artifact").into());
    }
    if invocation.run.input != invocation.source.input
        || invocation.run.context != invocation.qualification.context
        || invocation.surface.provider != invocation.run.provider
        || invocation.surface.family != FactFamily::Syntax {
        return Err(invalid("syntax invocation attribution differs from its input/context/provider/family").into());
    }
    let mut visitor = Emitter { invocation, output: &mut output, limits, path: vec![],
        children: vec![0], work: SyntaxWork::default(), error: None, text };
    let root = AnyNodeRef::from(ast);
    if visitor.enter_node(root).is_traverse() { visitor.visit_body(&ast.body); }
    visitor.leave_node(root);
    match visitor.error {
        Some(Halt::Limit(limit)) => Err(SyntaxError::Refused { limit, work: visitor.work }),
        Some(Halt::Model(error)) => Err(error.into()),
        None => Ok(visitor.work),
    }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }

/// Why the traversal halted.
enum Halt { Limit(SyntaxLimit), Model(ModelError) }
struct Emitter<'a, F> {
    invocation: SyntaxInvocation<'a>, output: &'a mut F, limits: SyntaxLimits,
    path: Vec<i32>, children: Vec<i32>, work: SyntaxWork, error: Option<Halt>, text: &'a str,
}
impl<F: FnMut(SyntaxEvent) -> Result<(), ModelError>> Emitter<'_, F> {
    /// Count one callback; `true` when the traversal has halted and the callback must return.
    fn halted(&mut self) -> bool {
        if self.error.is_some() { self.work.residual += 1; return true; }
        self.work.callbacks += 1;
        if self.work.callbacks > self.limits.callbacks { self.error = Some(Halt::Limit(SyntaxLimit::Callbacks)); return true; }
        false
    }
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
impl<'tree, F: FnMut(SyntaxEvent) -> Result<(), ModelError>> SourceOrderVisitor<'tree> for Emitter<'_, F> {
    fn enter_node(&mut self, node: AnyNodeRef<'tree>) -> TraversalSignal {
        // Ruff calls leave_node even after Skip, so every entry owns a balanced stack frame.
        let ordinal = *self.children.last().expect("root counter");
        self.path.push(ordinal); self.children.push(0);
        if self.error.is_some() { return TraversalSignal::Skip; }
        self.work.deepest = self.work.deepest.max(self.path.len());
        if self.path.len() > self.limits.depth { self.error = Some(Halt::Limit(SyntaxLimit::Depth)); return TraversalSignal::Skip; }
        if self.work.emitted >= self.limits.nodes { self.error = Some(Halt::Limit(SyntaxLimit::Nodes)); return TraversalSignal::Skip; }
        match self.event(node).and_then(&mut self.output) {
            Ok(()) => { self.work.emitted += 1; TraversalSignal::Traverse }
            Err(error) => { self.error = Some(Halt::Model(error)); TraversalSignal::Skip }
        }
    }
    fn leave_node(&mut self, _: AnyNodeRef<'tree>) {
        self.children.pop(); self.path.pop();
        let next = self.children.last_mut().expect("parent counter");
        match next.checked_add(1) {
            Some(value) => *next = value,
            None if self.error.is_none() => self.error = Some(Halt::Model(invalid("syntax child ordinal overflow"))),
            None => {},
        }
    }
    /// The one sibling loop the visitor owns stops outright after a halt.
    fn visit_body(&mut self, body: &'tree [Stmt]) {
        for stmt in body {
            if self.halted() { return; }
            source_order::walk_stmt(self, stmt);
        }
    }
    fn visit_singleton(&mut self, _: &'tree Singleton) { self.halted(); }
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
pub const PYREFLY_REVISION: &str = "a07b7baead9e0c7b496346d879b88e2fff9cbda7;ruff=0.0.11";

/// The provider that reports Syntax coverage. Its build identity covers the lockfile and this
/// source, so a changed emitter or pin is a different provider.
pub fn syntax_provider() -> Provider {
    Provider { tool: "pyrefly-retained-ruff-ast".into(), revision: PYREFLY_REVISION.into(),
        build_digest: ContentHash::of(concat!(include_str!("../../../Cargo.lock"), include_str!("typed_syntax.rs")).as_bytes()) }
}

/// The typed syntax facts of one captured input. Occurrence and observation rows travel in
/// reserved transfer batches; the per-artifact vocabulary is small and travels as rows.
pub struct SyntaxFacts {
    pub provider: Provider, pub context: AnalysisContext, pub run: ProviderRun, pub families: Vec<RunFamily>,
    pub surface: ProviderSurface, pub condition: Condition, pub nodes: Vec<ConditionNode>,
    pub modules: Vec<Module>, pub scopes: Vec<CoverageScope>, pub qualifications: Vec<AssertionQualification>,
    pub coverage: Vec<ProviderCoverage>,
    pub occurrences: Vec<Batch<Occurrence>>, pub observations: Vec<Batch<SyntaxObservation>>,
    pub evidence: Vec<Batch<Evidence>>, pub supports: Vec<Batch<SyntaxSupport>>,
    /// The artifacts handed to Pyrefly, and the traversal work of each one emitted.
    pub analyzed: Vec<String>, pub work: Vec<(String, SyntaxWork)>,
}
impl SyntaxFacts {
    /// Every Syntax row is complete under the stated model, or the stage is Partial.
    pub fn outcome(&self) -> ProviderOutcome {
        if self.coverage.iter().all(|row| row.status == CoverageStatus::CompleteUnderStatedModel) { ProviderOutcome::Complete }
        else if self.coverage.iter().all(|row| row.status == CoverageStatus::Unavailable) && !self.coverage.is_empty() { ProviderOutcome::Unavailable }
        else { ProviderOutcome::Partial }
    }
}

/// Run the pinned syntax provider over a captured input, on a thread with room for Pyrefly's
/// recursion. Every Python source is admitted before Pyrefly is given it. An admission-refused,
/// undecodable or traversal-bounded artifact is disclosed in its coverage, never dropped.
pub fn extract(captured: &CapturedInput, model: &ValidatedModel, limits: SyntaxLimits, budget: &ResourceBudget) -> Result<SyntaxFacts, ModelError> {
    std::thread::scope(|scope| {
        std::thread::Builder::new().stack_size(512 << 20).spawn_scoped(scope, || extract_on_thread(captured, model, limits, budget))
            .map_err(ModelError::codec)?.join().map_err(|_| invalid("the syntax provider panicked"))?
    })
}
fn extract_on_thread(captured: &CapturedInput, model: &ValidatedModel, limits: SyntaxLimits, budget: &ResourceBudget) -> Result<SyntaxFacts, ModelError> {
    use pyrefly::state::{require::Require, state::State};
    use pyrefly_config::{config::{ConfigFile, ConfigSource}, error_kind::ErrorKind, finder::ConfigFinder};
    use pyrefly_python::{module_path::ModulePath, sys_info::{PythonPlatform, PythonVersion}};
    use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
    for (name, _) in std::env::vars_os() {
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), "PYREFLY_STACK_SIZE" | "PYREFLY_FIXPOINT_DETAILS") || name.starts_with("PYSA_DUMP") {
            return Err(invalid(&format!("ambient analyzer setting {name} would change the provider's output")));
        }
    }
    let root = captured.root();
    let mut cfg = ConfigFile { source: ConfigSource::File(root.join("pyrefly.toml")), search_path_from_args: vec![root.to_path_buf()],
        disable_search_path_heuristics: true, disable_project_excludes_heuristics: true, enable_fallback_search_path: false, ..ConfigFile::default() };
    cfg.python_environment.python_version = Some(PythonVersion::new(3, 14, 7));
    cfg.python_environment.python_platform = Some(PythonPlatform::new("linux"));
    cfg.python_environment.site_package_path = Some(vec![]);
    cfg.interpreters.skip_interpreter_query = true;
    if !cfg.configure().is_empty() { return Err(invalid("the pinned analyzer configuration does not validate")); }
    // The configuration digest is independent of where the input was captured.
    let mut config = serde_json::to_value(&cfg).map_err(ModelError::codec)?; relativize(&mut config, root);
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["$input".into()],
        site_package_path: vec![], config_digest: ContentHash::of(&serde_json::to_vec(&config).map_err(ModelError::codec)?),
        environment_digest: captured.revision().manifest, lock_digest: None };
    let provider = syntax_provider();
    let (run, families) = ProviderRun::new(provider.id(), context.id(), captured.revision().id(), context.config_digest, [FactFamily::Syntax])?;
    let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Syntax, name: "retained AST identifier observations".into() };
    let (condition, nodes) = Diagram::always().records();
    // Admission precedes the analyzer: a refused or undecodable source is never handed to Pyrefly.
    let mut python: Vec<&SourceArtifact> = captured.artifacts().iter().filter(|a| ArtifactClass::of(&a.path) == Some(ArtifactClass::PythonSource)).collect();
    python.sort_by(|a, b| a.path.cmp(&b.path));
    let mut withheld = std::collections::BTreeMap::new(); let mut analyzed = Vec::new();
    for artifact in &python {
        if admit(artifact, limits).is_err() { withheld.insert(artifact.id(), ObligationKind::ResourceRefused); continue; }
        let _held = budget.reserve("syntax_source_check", usize::try_from(artifact.byte_len).unwrap_or(usize::MAX))?;
        let bytes = std::fs::read(root.join(&artifact.path)).map_err(ModelError::codec)?;
        if std::str::from_utf8(&bytes).is_err() { withheld.insert(artifact.id(), ObligationKind::UndecodableSource); continue; }
        analyzed.push(*artifact);
    }
    let handles: Vec<_> = analyzed.iter().map(|a| cfg.handle_from_module_path(ModulePath::filesystem(root.join(&a.path)))).collect();
    let state = State::new(ConfigFinder::new_constant(ArcId::new(cfg)), ThreadCount::Inline);
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.run(&handles, Require::Everything, None);
    let transfer = TransferLimits::default();
    let (mut occurrences, mut observations, mut evidence, mut supports) = (BatchWriter::new(budget, transfer)?, BatchWriter::new(budget, transfer)?,
        BatchWriter::new(budget, transfer)?, BatchWriter::new(budget, transfer)?);
    let mut facts = SyntaxFacts { provider: provider.clone(), context: context.clone(), run: run.clone(), families, surface: surface.clone(), condition: condition.clone(),
        nodes, modules: vec![], scopes: vec![], qualifications: vec![], coverage: vec![], occurrences: vec![], observations: vec![], evidence: vec![],
        supports: vec![], analyzed: analyzed.iter().map(|a| a.path.clone()).collect(), work: vec![] };
    let covered = |scope: &CoverageScope, status: CoverageStatus, reason: ObligationKind, diagnostic: Option<String>| ProviderCoverage {
        scope: scope.id(), provider: provider.id(), context: context.id(), family: FactFamily::Syntax, run: Some(run.id()), status, reason: Some(reason), diagnostic };
    for artifact in python {
        let scope = CoverageScope::Artifact { artifact: artifact.id() };
        if let Some(reason) = withheld.get(&artifact.id()) {
            facts.coverage.push(covered(&scope, CoverageStatus::Unavailable, *reason, None)); facts.scopes.push(scope); continue;
        }
        let handle = &handles[analyzed.iter().position(|a| a.id() == artifact.id()).expect("analyzed artifact")];
        let module = Module { source: artifact.id(), qualified_name: handle.module().to_string() };
        let qualification = AssertionQualification { context: context.id(), scope: scope.id(), condition: condition.id(),
            modality: Modality::Definite, approximation: Approximation::Exact };
        let ast = txn.get_ast(handle).ok_or_else(|| invalid("the analyzer did not retain the module's AST"))?;
        let info = txn.get_module_info(handle).ok_or_else(|| invalid("the analyzer did not retain the module's text"))?;
        let text = info.lined_buffer().contents().clone();
        let invocation = SyntaxInvocation { source: artifact, qualification: &qualification, run: &run, surface: &surface };
        let result = emit(&ast, &text, invocation, limits, |event| {
            if let Some(batch) = occurrences.push(model, event.occurrence)? { facts.occurrences.push(batch); }
            if let Some((assertion, cited, support)) = event.observation {
                if let Some(batch) = observations.push(model, assertion)? { facts.observations.push(batch); }
                if let Some(batch) = evidence.push(model, cited)? { facts.evidence.push(batch); }
                if let Some(batch) = supports.push(model, support)? { facts.supports.push(batch); }
            }
            Ok(())
        });
        let errors = txn.get_errors([handle]).collect_errors();
        let parse_error = [&errors.ordinary, &errors.directives, &errors.suppressed, &errors.disabled, &errors.baseline]
            .into_iter().flatten().any(|error| error.error_kind() == ErrorKind::ParseError);
        match result {
            Ok(work) => {
                facts.coverage.push(covered(&scope, CoverageStatus::Partial, if parse_error { ObligationKind::SyntaxError } else { ObligationKind::OutsideProviderModel },
                    Some("Identifier observation subset; complete syntax family not implemented".into())));
                facts.work.push((artifact.path.clone(), work));
            },
            Err(error) => match error.coverage() {
                Some((status, reason)) => {
                    facts.coverage.push(covered(&scope, status, reason, Some(error.to_string())));
                    if let SyntaxError::Refused { work, .. } = error { facts.work.push((artifact.path.clone(), work)); }
                },
                None => return Err(match error { SyntaxError::Model(error) => error, other => invalid(&other.to_string()) }),
            },
        }
        facts.modules.push(module); facts.qualifications.push(qualification); facts.scopes.push(scope);
    }
    macro_rules! finish { ($($writer:ident => $out:ident),+) => { $( if let Some(batch) = $writer.finish(model)? { facts.$out.push(batch); } )+ }; }
    finish!(occurrences => occurrences, observations => observations, evidence => evidence, supports => supports);
    Ok(facts)
}
/// Paths under the captured root are written relative to it, so the configuration digest does
/// not depend on where the input was captured.
fn relativize(value: &mut serde_json::Value, root: &std::path::Path) {
    match value {
        serde_json::Value::String(s) => {
            if let Ok(relative) = std::path::Path::new(s).strip_prefix(root) { *s = format!("$input/{}", relative.display()); }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|item| relativize(item, root)),
        serde_json::Value::Object(items) => items.values_mut().for_each(|item| relativize(item, root)),
        _ => {},
    }
}
