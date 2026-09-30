//! The `pyrefly` stage (cutover plan A4, phase 1; ADR-0046, ADR-0089). One pinned Pyrefly
//! transaction per captured input; the retained Ruff parse of each analyzed root is emitted as
//! typed syntax: occurrences with placement and details, identifier observations, declarations,
//! decorators, import aliases, `__all__`, parameters, class fields and call syntax. Later phases add
//! lexical (A5), symbol (A9), call (A10) and type (A11) facts from the same session.
//!
//! The analyzed roots are an installed input's release modules, a corpus's examples, tests and
//! derived blocks, and every Python source of a tree. Other captured sources are context: imports
//! resolve through them, and they are never stated. A root is admitted by size and decoded before
//! Pyrefly is given it; a refused or undecodable root is disclosed as unavailable. A refused module
//! that an admitted one imports may still be parsed by Pyrefly: that load stays provider-internal
//! and states nothing (P0 exit F08).
//!
//! The lexical recognizer (A5) runs over the same occurrences: scopes, binding events with their
//! static branches, reads and their resolutions against Pyrefly's builtins, implicit globals and
//! star-import sets.
//!
//! Exports and Signatures are covered as Partial: this phase states their syntax, and the symbol
//! producer (A9) states public names and signatures. A computed `__all__` is a subject boundary.
use std::{collections::BTreeMap, path::Path};
use lctx_model::domain::{ContentHash, ModelError, Record, admission::ArtifactClass, assertion::*, attribution::*, calls::{CallArgument, CallSyntax, CallSyntaxSupport},
    conditions::{Condition, ConditionNode, Diagram}, input::SourceRole, lexical::*, source::*, stages::{Effect, Profile, ProviderOutcome, RelationUse, Stage, StageSink},
    syntax::*, value::{Literal, LiteralSet, LiteralSetMember}};
use crate::{acquisition::{AcquiredInput, Acquisition}, bundle::{self, Declared, ProviderStage, StageContext}, lexical::{Outside, Stars}, lexical_records,
    syntax_records::{self, Spans},
    typed_syntax::{self, PYREFLY_REVISION, SyntaxInvocation, SyntaxLimits}};

pub const PYREFLY: &str = "pyrefly";
/// The families this phase covers.
pub const FAMILIES: [FactFamily; 4] = [FactFamily::Syntax, FactFamily::Lexical, FactFamily::Exports, FactFamily::Signatures];

/// The Pyrefly provider: the pinned fork and the sources that turn its session into facts.
pub fn pyrefly_provider() -> Provider {
    Provider { tool: "pyrefly".into(), revision: PYREFLY_REVISION.into(),
        build_digest: bundle::build_digest(&[include_str!("pyrefly_stage.rs"), include_str!("typed_syntax.rs"), include_str!("syntax_records.rs"),
            include_str!("lexical.rs"), include_str!("lexical_records.rs")]) }
}
fn invalid(message: String) -> ModelError { ModelError::Invalid(message) }

/// The `pyrefly` stage with its traversal limits.
pub struct Pyrefly { limits: SyntaxLimits }
impl Pyrefly {
    pub fn new(limits: SyntaxLimits) -> Self { Self { limits } }
}
macro_rules! uses { ($($ty:ty),+ $(,)?) => { vec![$(RelationUse::of::<$ty>()),+] }; }
fn outputs() -> Vec<RelationUse> {
    uses!(Module, Occurrence, SyntaxObservation, SyntaxSupport, SyntaxPlacement, SyntaxPlacementSupport, SyntaxDetailObservation, SyntaxDetailSupport,
        DeclarationObservation, DeclarationSupport, DeclarationDecorator, DeclarationDecoratorSupport, ImportAliasObservation, ImportAliasSupport,
        DunderAllObservation, DunderAllSupport, ParameterSyntaxObservation, ParameterSyntaxSupport, ClassFieldSyntaxObservation, ClassFieldSyntaxSupport,
        CallSyntax, CallSyntaxSupport, CallArgument, LexicalScope, BindingEvent, LexicalTarget, LexicalScopeObservation, LexicalScopeSupport,
        BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport)
}
/// Vocabulary the `assemble` stage writes once for every provider.
pub fn vocabulary() -> Vec<RelationUse> {
    uses!(Evidence, Literal, LiteralSet, LiteralSetMember, SyntaxDetail, AssertionQualification, Condition, ConditionNode, Provider, AnalysisContext,
        ProviderRun, RunFamily, ProviderSurface, CoverageScope, ProviderCoverage, SubjectBoundary)
}
impl Declared for Pyrefly {
    fn declaration(&self, _: Profile) -> Stage {
        let provider = pyrefly_provider();
        Stage { name: PYREFLY, inputs: vec![], outputs: outputs(), contributes: vocabulary(), coverage: FAMILIES.to_vec(), provider: Some(provider.id()),
            profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Extraction, code: provider.build_digest,
            configuration: ContentHash::of(format!("{:?}", self.limits).as_bytes()) }
    }
}

/// The Python sources of an input its acquisition makes analysis roots, in path order.
pub fn roots(input: &AcquiredInput) -> Vec<&SourceArtifact> {
    let python = |path: &str| ArtifactClass::of(path) == Some(ArtifactClass::PythonSource);
    let derived: Vec<&str> = input.captured().derivations().iter().map(|d| d.path.as_str()).collect();
    let mut roots: Vec<&SourceArtifact> = input.captured().artifacts().iter().filter(|a| python(&a.path)).filter(|a| match input.acquisition() {
        Acquisition::Installed(library) => library.files.iter().any(|f| f.path == a.path && f.role == SourceRole::Release),
        Acquisition::Corpus { uses, .. } => derived.contains(&a.path.as_str())
            || uses.get(&a.path).is_some_and(|roles| roles.iter().any(|r| matches!(r, SourceRole::Example | SourceRole::Test))),
        Acquisition::Tree { .. } => true,
    }).collect();
    roots.sort_by(|a, b| a.path.cmp(&b.path));
    roots
}

impl<S: StageSink + 'static> ProviderStage<S> for Pyrefly {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        macro_rules! declare { ($($ty:ty),+) => { $( context.declare::<$ty>()?; )+ }; }
        declare!(Module, Occurrence, SyntaxObservation, SyntaxSupport, SyntaxPlacement, SyntaxPlacementSupport, SyntaxDetailObservation, SyntaxDetailSupport,
            DeclarationObservation, DeclarationSupport, DeclarationDecorator, DeclarationDecoratorSupport, ImportAliasObservation, ImportAliasSupport,
            DunderAllObservation, DunderAllSupport, ParameterSyntaxObservation, ParameterSyntaxSupport, ClassFieldSyntaxObservation, ClassFieldSyntaxSupport,
            CallSyntax, CallSyntaxSupport, CallArgument, LexicalScope, BindingEvent, LexicalTarget, LexicalScopeObservation, LexicalScopeSupport,
            BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport);
        let provider = pyrefly_provider();
        let (condition, nodes) = Diagram::always().records();
        context.contribute(provider.clone())?;
        context.contribute(condition.clone())?;
        for node in nodes { context.contribute(node)?; }
        let captured = context.captured();
        let mut partial = false;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                Acquisition::Corpus { library, .. } => Some(captured.inputs().get(*library).ok_or_else(|| invalid("a corpus names an absent library input".into()))?),
                _ => None,
            };
            partial |= session(context, &provider, &condition, input, library, self.limits)?;
        }
        Ok(if partial { ProviderOutcome::Partial } else { ProviderOutcome::Complete })
    }
}

/// The interpreter an input is analyzed for.
fn interpreter(input: &AcquiredInput, library: Option<&AcquiredInput>) -> Result<(String, String), ModelError> {
    Ok(match library.unwrap_or(input).acquisition() {
        Acquisition::Installed(library) => (library.python_version.clone(), library.platform.clone()),
        _ => ("3.14.7".into(), "linux".into()),
    })
}
/// One input's session. Returns whether any of its coverage is less than complete.
fn session<S: StageSink + 'static>(context: &mut StageContext<S>, provider: &Provider, condition: &Condition, input: &AcquiredInput,
    library: Option<&AcquiredInput>, limits: SyntaxLimits) -> Result<bool, ModelError> {
    use pyrefly::state::{require::Require, state::State};
    use pyrefly_config::{config::{ConfigFile, ConfigSource}, error_kind::ErrorKind, finder::ConfigFinder};
    use pyrefly_python::{module_path::ModulePath, sys_info::{PythonPlatform, PythonVersion}};
    use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
    let captured = input.captured();
    let root = captured.root();
    let (python, platform) = interpreter(input, library)?;
    let (major, minor, micro) = crate::library::version_triple(&python).map_err(|e| invalid(e.to_string()))?;
    let mut cfg = ConfigFile { source: ConfigSource::File(root.join("pyrefly.toml")), search_path_from_args: vec![root.to_path_buf()],
        disable_search_path_heuristics: true, disable_project_excludes_heuristics: true, enable_fallback_search_path: false, ..ConfigFile::default() };
    cfg.python_environment.python_version = Some(PythonVersion::new(major, minor, micro));
    cfg.python_environment.python_platform = Some(PythonPlatform::new(&platform));
    cfg.python_environment.site_package_path = Some(library.map(|l| vec![l.captured().root().to_path_buf()]).unwrap_or_default());
    cfg.interpreters.skip_interpreter_query = true;
    if !cfg.configure().is_empty() { return Err(invalid("the pinned analyzer configuration does not validate".into())); }
    let mut config = serde_json::to_value(&cfg).map_err(ModelError::codec)?;
    relativize(&mut config, root, "$input");
    if let Some(library) = library { relativize(&mut config, library.captured().root(), "$library"); }
    let environment = library.unwrap_or(input);
    let lock_digest = match environment.acquisition() { Acquisition::Installed(library) => Some(library.lock_digest), _ => None };
    let analysis = AnalysisContext { python_version: python, python_platform: platform, search_path: vec!["$input".into()],
        site_package_path: library.map(|_| vec!["$library".into()]).unwrap_or_default(),
        config_digest: ContentHash::of(&serde_json::to_vec(&config).map_err(ModelError::codec)?),
        environment_digest: environment.captured().revision().manifest, lock_digest };
    let (run, families) = ProviderRun::new(provider.id(), analysis.id(), captured.revision().id(), analysis.config_digest, FAMILIES)?;
    let surfaces: BTreeMap<FactFamily, ProviderSurface> = FAMILIES.iter()
        .map(|family| (*family, ProviderSurface { provider: provider.id(), family: *family, name: "retained Ruff parse".into() })).collect();
    context.contribute(analysis.clone())?;
    context.contribute(run.clone())?;
    for family in families { context.contribute(family)?; }
    for surface in surfaces.values() { context.contribute(surface.clone())?; }
    // Admission precedes the analyzer: a refused or undecodable root is never handed to Pyrefly.
    let roots = roots(input);
    let mut withheld = BTreeMap::new();
    let mut analyzed = Vec::new();
    for artifact in &roots {
        if typed_syntax::admit(artifact, limits).is_err() { withheld.insert(artifact.id(), ObligationKind::ResourceRefused); continue; }
        let _held = context.budget().reserve("syntax_source_check", usize::try_from(artifact.byte_len).unwrap_or(usize::MAX))?;
        let bytes = std::fs::read(root.join(&artifact.path)).map_err(ModelError::codec)?;
        if std::str::from_utf8(&bytes).is_err() { withheld.insert(artifact.id(), ObligationKind::UndecodableSource); continue; }
        analyzed.push(*artifact);
    }
    let handles: Vec<_> = analyzed.iter().map(|a| cfg.handle_from_module_path(ModulePath::filesystem(root.join(&a.path)))).collect();
    let state = State::new(ConfigFinder::new_constant(ArcId::new(cfg)), ThreadCount::Inline);
    let mut transaction = state.new_transaction(Require::Exports, None);
    transaction.run(&handles, Require::Everything, None);
    let sys = pyrefly_python::sys_info::SysInfo::new(PythonVersion::new(major, minor, micro), PythonPlatform::new(&analysis.python_platform));
    let outside = outside_names(&transaction, handles.first());
    let mut partial = false;
    let coverage = |scope: &CoverageScope, family: FactFamily, status: CoverageStatus, reason: Option<ObligationKind>, diagnostic: Option<String>| ProviderCoverage {
        scope: scope.id(), provider: provider.id(), context: analysis.id(), family, run: Some(run.id()), status, reason, diagnostic };
    for artifact in roots {
        let scope = CoverageScope::Artifact { artifact: artifact.id() };
        context.contribute(scope.clone())?;
        if let Some(reason) = withheld.get(&artifact.id()) {
            for family in FAMILIES { context.contribute(coverage(&scope, family, CoverageStatus::Unavailable, Some(*reason), None))?; }
            partial = true;
            continue;
        }
        let handle = &handles[analyzed.iter().position(|a| a.id() == artifact.id()).expect("an analyzed root")];
        let module_name = handle.module().to_string();
        let is_package = matches!(artifact.path.rsplit('/').next(), Some("__init__.py" | "__init__.pyi"));
        context.emit(Module { source: artifact.id(), qualified_name: module_name.clone() })?;
        let qualification = AssertionQualification { context: analysis.id(), scope: scope.id(), condition: condition.id(),
            modality: Modality::Definite, approximation: Approximation::Exact };
        context.contribute(qualification.clone())?;
        let ast = transaction.get_ast(handle).ok_or_else(|| invalid("the analyzer did not retain the module's AST".into()))?;
        let info = transaction.get_module_info(handle).ok_or_else(|| invalid("the analyzer did not retain the module's text".into()))?;
        let text = info.lined_buffer().contents().clone();
        let invocation = SyntaxInvocation { source: artifact, qualification: &qualification, run: &run, surface: &surfaces[&FactFamily::Syntax] };
        let support = |family: FactFamily, subject: lctx_model::domain::Id<Occurrence>| (run.id(), surfaces[&family].id(), Evidence::Occurrence { occurrence: subject }.id());
        let mut spans = Spans::default();
        let _index = context.budget().reserve("syntax_spans", 0)?;
        let emitted = typed_syntax::emit(&ast, &text, invocation, limits, |event| {
            let occurrence = event.occurrence.id();
            spans.insert(&event.occurrence);
            if let Some((parent, field, _)) = event.placement { spans.place(occurrence, parent, field); }
            context.contribute(Evidence::Occurrence { occurrence })?;
            context.emit(event.occurrence)?;
            if let Some((observation, _, observed)) = event.observation { context.emit(observation)?; context.emit(observed)?; }
            let (parent, field, ordinal) = event.placement.map_or((None, SyntaxField::Child, 0), |(parent, field, ordinal)| (Some(parent), field, ordinal));
            let placement = SyntaxPlacement { qualification: qualification.id(), occurrence, parent, field, ordinal };
            let (run, surface, evidence) = support(FactFamily::Syntax, occurrence);
            context.emit(SyntaxPlacementSupport { assertion: placement.id(), run, surface, evidence, origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural })?;
            context.emit(placement)?;
            for (ordinal, (detail, literal)) in event.details.into_iter().enumerate() {
                let observation = SyntaxDetailObservation { qualification: qualification.id(), occurrence, ordinal: ordinal as i64, detail: detail.id() };
                context.emit(SyntaxDetailSupport { assertion: observation.id(), run, surface, evidence, origin: Origin::SourceObservation,
                    mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural })?;
                context.emit(observation)?;
                context.contribute(detail)?;
                if let Some(literal) = literal { context.contribute(literal)?; }
            }
            Ok(())
        });
        let errors = transaction.get_errors([handle]).collect_errors();
        let parse_error = [&errors.ordinary, &errors.directives, &errors.suppressed, &errors.disabled, &errors.baseline]
            .into_iter().flatten().any(|error| error.error_kind() == ErrorKind::ParseError);
        let syntax = match emitted {
            Ok(_) => {
                let records = syntax_records::records(&ast, &module_name, is_package, &spans, qualification.id())?;
                let computed = records.computed_all.clone();
                write_records(context, records, &|family, subject| support(family, subject))?;
                let stars = star_imports(&transaction, handle, &module_name, is_package, &ast);
                let candidate = AssertionQualification { modality: Modality::Candidate, ..qualification.clone() };
                context.contribute(candidate.clone())?;
                let facts = lexical_records::facts(&ast, &spans, &sys, &outside, &stars)?;
                let lexical = lexical_records::records(&facts, &spans, qualification.id(), candidate.id())?;
                write_lexical(context, lexical, &|subject| support(FactFamily::Lexical, subject))?;
                for statement in computed {
                    context.contribute(SubjectBoundary { scope: scope.id(), provider: provider.id(), context: analysis.id(), family: FactFamily::Exports,
                        subject: Some(statement), reason: ObligationKind::OutsideProviderModel, detail: Some("__all__ is computed; its names are not stated by the syntax".into()) })?;
                }
                if parse_error { (CoverageStatus::Partial, Some(ObligationKind::SyntaxError), Some("the module parsed with errors; facts come from the recovered tree".into())) }
                else { (CoverageStatus::CompleteUnderStatedModel, None, None) }
            }
            Err(error) => match error.coverage() {
                Some((status, reason)) => (status, Some(reason), Some(error.to_string())),
                None => return Err(match error { typed_syntax::SyntaxError::Model(error) => error, other => invalid(other.to_string()) }),
            },
        };
        context.contribute(coverage(&scope, FactFamily::Syntax, syntax.0, syntax.1, syntax.2.clone()))?;
        context.contribute(coverage(&scope, FactFamily::Lexical, syntax.0, syntax.1, syntax.2))?;
        let pending = |family: &str| Some(format!("syntax only; the symbol producer (plan A9) states {family}"));
        let (status, reason) = if syntax.0 == CoverageStatus::Unavailable { (CoverageStatus::Unavailable, syntax.1) }
            else { (CoverageStatus::Partial, Some(ObligationKind::OutsideProviderModel)) };
        context.contribute(coverage(&scope, FactFamily::Exports, status, reason, pending("public names")))?;
        context.contribute(coverage(&scope, FactFamily::Signatures, status, reason, pending("signatures")))?;
        // Exports and Signatures stay Partial until the symbol producer (A9).
        partial = true;
    }
    Ok(partial)
}

type Support = (lctx_model::domain::Id<ProviderRun>, lctx_model::domain::Id<ProviderSurface>, lctx_model::domain::Id<Evidence>);
/// Emit one module's records, each with its native support.
fn write_records<S: StageSink + 'static>(context: &mut StageContext<S>, records: syntax_records::Records,
    support: &dyn Fn(FactFamily, lctx_model::domain::Id<Occurrence>) -> Support) -> Result<(), ModelError> {
    macro_rules! with_support {
        ($rows:expr, $support:ident, $family:expr, $subject:ident) => {
            for row in $rows {
                let (run, surface, evidence) = support($family, row.$subject);
                context.emit($support { assertion: row.id(), run, surface, evidence, origin: Origin::SourceObservation,
                    mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural })?;
                context.emit(row)?;
            }
        };
    }
    with_support!(records.declarations, DeclarationSupport, FactFamily::Syntax, declaration);
    with_support!(records.decorators, DeclarationDecoratorSupport, FactFamily::Syntax, decorator);
    with_support!(records.imports, ImportAliasSupport, FactFamily::Exports, alias);
    with_support!(records.dunder_all, DunderAllSupport, FactFamily::Exports, statement);
    with_support!(records.parameters, ParameterSyntaxSupport, FactFamily::Signatures, parameter);
    with_support!(records.fields, ClassFieldSyntaxSupport, FactFamily::Syntax, target);
    for (call, arguments) in records.calls {
        let (run, surface, evidence) = support(FactFamily::Syntax, call.site);
        context.emit(CallSyntaxSupport { assertion: call.id(), run, surface, evidence, origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural })?;
        context.emit(call)?;
        for argument in arguments { context.emit(argument)?; }
    }
    for literal in records.literals { context.contribute(literal)?; }
    for (set, members) in records.sets { context.contribute(set)?; for member in members { context.contribute(member)?; } }
    Ok(())
}

/// Emit one module's lexical records, each assertion with its support.
fn write_lexical<S: StageSink + 'static>(context: &mut StageContext<S>, records: lexical_records::LexicalRecords,
    support: &dyn Fn(lctx_model::domain::Id<Occurrence>) -> Support) -> Result<(), ModelError> {
    let owners: std::collections::HashMap<_, _> = records.scopes.iter().map(|s| (s.id(), s.owner)).collect();
    let sites: std::collections::HashMap<_, _> = records.events.iter().map(|e| (e.id(), e.site)).collect();
    macro_rules! native { ($support:ident, $row:expr, $subject:expr) => {{
        let (run, surface, evidence) = support($subject);
        context.emit($support { assertion: $row.id(), run, surface, evidence, origin: Origin::DerivedAnalysis, mode: ExtractionMode::Recognizer,
            fidelity: Fidelity::NormalizedStructural })?;
    }}; }
    for scope in records.scopes { context.emit(scope)?; }
    for event in records.events { context.emit(event)?; }
    for target in records.targets { context.emit(target)?; }
    for row in records.scope_observations { native!(LexicalScopeSupport, row, owners[&row.scope]); context.emit(row)?; }
    for row in records.bindings { native!(BindingSupport, row, sites[&row.event]); context.emit(row)?; }
    for row in records.references { native!(ReferenceSupport, row, row.read); context.emit(row)?; }
    for row in records.resolutions { native!(LexicalResolutionSupport, row, row.read); context.emit(row)?; }
    Ok(())
}
/// The names from outside a module's text, as Pyrefly defines them: its implicit globals, and the
/// real, public definitions of `builtins` (not the stub's implicit globals, private helpers or
/// imports), each with whether it is variable-like.
fn outside_names(transaction: &pyrefly::state::state::Transaction<'_>, anchor: Option<&pyrefly_build::handle::Handle>) -> Outside {
    use pyrefly::export::exports::ExportLocation;
    use pyrefly_python::module_name::ModuleName;
    use pyrefly_types::globals::ImplicitGlobal;
    let implicit_globals: Vec<String> = ImplicitGlobal::implicit_globals(false).map(|g| g.name().to_string()).collect();
    let builtins = anchor.and_then(|h| transaction.import_handle(h, ModuleName::from_str("builtins"), None).finding()).map(|h| {
        transaction.get_exports(&h).iter().filter_map(|(name, location)| {
            let name = name.as_str();
            match location {
                ExportLocation::ThisModule(export) if pyrefly::commands::coverage::collect::is_public_name(name) && !implicit_globals.iter().any(|g| g == name) => {
                    use pyrefly_python::symbol_kind::SymbolKind as Kind;
                    Some((name.to_owned(), !matches!(export.symbol_kind, Some(Kind::Function | Kind::Class | Kind::Method))))
                }
                ExportLocation::ThisModule(_) | ExportLocation::OtherModule(..) => None,
            }
        }).collect()
    }).unwrap_or_default();
    Outside { builtins, implicit_globals }
}
/// Each `from m import *` of a module → the names Pyrefly's wildcard set for `m` holds, or `None`
/// when Pyrefly cannot find `m`. Star imports are module-level only.
fn star_imports(transaction: &pyrefly::state::state::Transaction<'_>, handle: &pyrefly_build::handle::Handle, module: &str, is_package: bool,
    ast: &ruff_python_ast::ModModule) -> Stars {
    use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
    struct Found<'a>(Vec<&'a ruff_python_ast::StmtImportFrom>);
    impl<'a> StatementVisitor<'a> for Found<'a> {
        fn visit_stmt(&mut self, stmt: &'a ruff_python_ast::Stmt) {
            if let ruff_python_ast::Stmt::ImportFrom(i) = stmt && i.names.iter().any(|a| a.name.as_str() == "*") { self.0.push(i); }
            walk_stmt(self, stmt);
        }
    }
    let mut found = Found(Vec::new());
    found.visit_body(&ast.body);
    let mut stars = Stars::new();
    for import in found.0 {
        let Some(star) = import.names.iter().find(|a| a.name.as_str() == "*") else { continue; };
        let names = syntax_records::absolute_module(module, is_package, i64::from(import.level), import.module.as_ref().map(|n| n.as_str()))
            .and_then(|name| transaction.import_handle(handle, pyrefly_python::module_name::ModuleName::from_str(&name), None).finding())
            .map(|h| transaction.get_wildcard(&h).iter().map(ToString::to_string).collect());
        stars.insert(u32::from(ruff_text_size::Ranged::start(star)), names);
    }
    stars
}

/// Paths under a captured root are written relative to it, so the configuration digest does not
/// depend on where the input was captured.
fn relativize(value: &mut serde_json::Value, root: &Path, label: &str) {
    match value {
        serde_json::Value::String(s) => {
            if let Ok(relative) = Path::new(s).strip_prefix(root) { *s = format!("{label}/{}", relative.display()); }
            else if Path::new(s) == root { *s = label.to_owned(); }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|item| relativize(item, root, label)),
        serde_json::Value::Object(items) => items.values_mut().for_each(|item| relativize(item, root, label)),
        _ => {},
    }
}
