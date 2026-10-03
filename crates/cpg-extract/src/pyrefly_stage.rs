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
//! The symbol producer (A9) states Pysa's definitions of each analyzed module from the same
//! session: symbols and their nesting, traits, bases and MROs, undecorated signatures with each
//! parameter's displayed annotation, and declaration links at exact name spans. Signatures are
//! complete unless a declaration fails to attach, which is a boundary. Exports stay Partial until
//! the public names are stated. A computed `__all__` is a subject boundary.
use crate::{
    acquisition::{AcquiredInput, Acquisition},
    bundle::{self, Declared, ProviderStage, StageContext},
    lexical::{Outside, Stars},
    lexical_records,
    natives::Natives,
    public_records, symbol_records,
    syntax_records::{self, Spans},
    typed_syntax::{self, PYREFLY_REVISION, SyntaxInvocation, SyntaxLimits},
};
use lctx_model::domain::{
    ContentHash, ModelError, Record,
    admission::ArtifactClass,
    assertion::*,
    attribution::*,
    calls::*,
    class_metadata::*,
    conditions::{Condition, Diagram},
    declarations::{
        ParameterDeclaration, ParameterDeclarationSupport, SymbolDeclaration,
        SymbolDeclarationSupport,
    },
    lexical::*,
    source::*,
    stages::{Effect, Profile, ProviderOutcome, RelationUse, Stage, StageSink},
    symbols::*,
    syntax::*,
    ruff::*,
    types::*,
};
use std::{collections::BTreeMap, path::Path};

pub const PYREFLY: &str = "pyrefly";
/// The families this phase covers.
pub const FAMILIES: [FactFamily; 6] = [
    FactFamily::Syntax,
    FactFamily::Lexical,
    FactFamily::Exports,
    FactFamily::Signatures,
    FactFamily::Calls,
    FactFamily::Types,
];

/// The Pyrefly provider: the pinned fork and the sources that turn its session into facts.
pub fn pyrefly_provider() -> Provider {
    Provider {
        tool: "pyrefly".into(),
        revision: PYREFLY_REVISION.into(),
        build_digest: bundle::build_digest(&[
            include_str!("pyrefly_stage.rs"),
            include_str!("native_context.rs"),
            include_str!("typed_syntax.rs"),
            include_str!("syntax_records.rs"),
            include_str!("lexical.rs"),
            include_str!("lexical_records.rs"),
            include_str!("natives.rs"),
            include_str!("symbol_records.rs"),
            include_str!("public_records.rs"),
            include_str!("docstrings.rs"),
        ]),
    }
}
fn invalid(message: String) -> ModelError {
    ModelError::Invalid(message)
}

/// Pysa's reports of each analyzed module (definitions and call graphs as Pysa serializes them), by
/// module name: the session hook the Pysa-CLI harness oracle compares.
pub type PysaTap = std::sync::Arc<std::sync::Mutex<BTreeMap<String, serde_json::Value>>>;
/// The `pyrefly` stage with its traversal limits.
pub struct Pyrefly {
    limits: SyntaxLimits,
    tap: Option<PysaTap>,
}
impl Pyrefly {
    pub fn new(limits: SyntaxLimits) -> Self {
        Self { limits, tap: None }
    }
    /// The stage, also handing each module's Pysa reports to `tap`.
    pub fn with_tap(limits: SyntaxLimits, tap: PysaTap) -> Self {
        Self {
            limits,
            tap: Some(tap),
        }
    }
}
macro_rules! uses { ($($ty:ty),+ $(,)?) => { vec![$(RelationUse::of::<$ty>()),+] }; }
fn outputs() -> Vec<RelationUse> {
    uses!(
        Module,
        Occurrence,
        RuffContextObservation,
        RuffContextSupport,
        SyntaxObservation,
        SyntaxSupport,
        SyntaxPlacement,
        SyntaxPlacementSupport,
        SyntaxDetailObservation,
        SyntaxDetailSupport,
        DeclarationObservation,
        DeclarationSupport,
        DeclarationDecorator,
        DeclarationDecoratorSupport,
        ImportAliasObservation,
        ImportAliasSupport,
        DunderAllObservation,
        DunderAllSupport,
        ParameterSyntaxObservation,
        ParameterSyntaxSupport,
        ClassFieldSyntaxObservation,
        ClassFieldSyntaxSupport,
        CallSyntax,
        CallSyntaxSupport,
        CallArgument,
        LexicalScope,
        BindingEvent,
        LexicalTarget,
        LexicalScopeObservation,
        LexicalScopeSupport,
        BindingObservation,
        BindingSupport,
        ReferenceObservation,
        ReferenceSupport,
        LexicalResolution,
        LexicalResolutionSupport,
        ProviderModule,
        ProviderSymbol,
        ParameterShape,
        Signature,
        SignatureSupport,
        SignatureParameter,
        SignatureEnumerationObservation,
        SignatureEnumerationMember,
        SignatureEnumerationSupport,
        SymbolDeclaration,
        SymbolDeclarationSupport,
        ParameterDeclaration,
        ParameterDeclarationSupport,
        SymbolSequence,
        SymbolSequenceMember,
        SymbolObservation,
        SymbolSupport,
        FunctionTraitObservation,
        FunctionTraitSupport,
        ClassTraitObservation,
        ClassTraitSupport,
        ClassAncestryObservation,
        ClassAncestrySupport,
        ParameterAnnotationObservation,
        ParameterAnnotationSupport,
        ExportOrigin,
        PublicNameObservation,
        PublicNameSupport,
        ParameterDocObservation,
        ParameterDocSupport,
        ModuleResolutionObservation,
        ModuleResolutionSupport,
        ProviderCallable,
        CallOrigin,
        CallOriginStep,
        CallDestination,
        CallChannel,
        Receiver,
        ProviderCallSite,
        ProviderCallSiteSupport,
        CallTarget,
        CallTargetSupport,
        CallResolution,
        CallResolutionSupport,
        CallResolutionMember,
        ClassMetadataObservation,
        ClassMetadataSupport,
        ClassMemberObservation,
        ClassMemberSupport,
        RecordOptions,
        RecordTransformDefaults,
        RecordTransformFieldSpecifier,
        TypeTerm,
        TypeSequence,
        TypeSequenceMember,
        CallableParameterList,
        CallableParameter,
        TypedDictFieldList,
        TypedDictField,
        TypeVariable,
        TypeVariableRestriction,
        TypeRestrictionSupport,
        TypeObservation,
        TypeSupport,
        TypePresentation,
        TypePresentationSupport,
        FunctionBodyObservation,
        FunctionBodySupport,
        RecordFieldObservation,
        RecordFieldSupport
    )
}
impl Declared for Pyrefly {
    fn declaration(&self, _: Profile) -> Stage {
        let provider = pyrefly_provider();
        Stage {
            name: PYREFLY,
            inputs: vec![],
            outputs: outputs(),
            contributes: crate::assembly::vocabulary(),
            coverage: FAMILIES.to_vec().into_iter().map(|family| lctx_model::domain::stages::FamilyCoverage {family,provider:if family==FactFamily::Syntax {crate::ruff_context::provider().id()} else {provider.id()}}).collect(),
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Extraction,
            code: provider.build_digest,
            configuration: ContentHash::of(format!("{:?}", self.limits).as_bytes()),
        }
    }
}

/// The Python sources of an input its acquisition makes analysis roots, in path order.
pub fn roots(input: &AcquiredInput) -> Result<Vec<&SourceArtifact>, ModelError> {
    let selected =
        lctx_model::domain::admission::analysis_roots(input.captured().artifacts(), &input.uses())?;
    let mut roots: Vec<_> = input
        .captured()
        .artifacts()
        .iter()
        .filter(|a| {
            selected.contains(&a.id())
                && ArtifactClass::of(&a.path) == Some(ArtifactClass::PythonSource)
        })
        .collect();
    roots.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(roots)
}

impl<S: StageSink + 'static> ProviderStage<S> for Pyrefly {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        macro_rules! declare { ($($ty:ty),+) => { $( context.declare::<$ty>()?; )+ }; }
        declare!(
            Module,
            Occurrence,
            RuffContextObservation,
        RuffContextSupport,
        SyntaxObservation,
            SyntaxSupport,
            SyntaxPlacement,
            SyntaxPlacementSupport,
            SyntaxDetailObservation,
            SyntaxDetailSupport,
            DeclarationObservation,
            DeclarationSupport,
            DeclarationDecorator,
            DeclarationDecoratorSupport,
            ImportAliasObservation,
            ImportAliasSupport,
            DunderAllObservation,
            DunderAllSupport,
            ParameterSyntaxObservation,
            ParameterSyntaxSupport,
            ClassFieldSyntaxObservation,
            ClassFieldSyntaxSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            LexicalScope,
            BindingEvent,
            LexicalTarget,
            LexicalScopeObservation,
            LexicalScopeSupport,
            BindingObservation,
            BindingSupport,
            ReferenceObservation,
            ReferenceSupport,
            LexicalResolution,
            LexicalResolutionSupport,
            ProviderModule,
            ProviderSymbol,
            ParameterShape,
            Signature,
            SignatureSupport,
            SignatureParameter,
            SignatureEnumerationObservation,
            SignatureEnumerationMember,
            SignatureEnumerationSupport,
            SymbolDeclaration,
            SymbolDeclarationSupport,
            ParameterDeclaration,
            ParameterDeclarationSupport,
            SymbolSequence,
            SymbolSequenceMember,
            SymbolObservation,
            SymbolSupport,
            FunctionTraitObservation,
            FunctionTraitSupport,
            ClassTraitObservation,
            ClassTraitSupport,
            ClassAncestryObservation,
            ClassAncestrySupport,
            ParameterAnnotationObservation,
            ParameterAnnotationSupport,
            ExportOrigin,
            PublicNameObservation,
            PublicNameSupport,
            ParameterDocObservation,
            ParameterDocSupport,
            ModuleResolutionObservation,
            ModuleResolutionSupport,
            ProviderCallable,
            CallOrigin,
            CallOriginStep,
            CallDestination,
            CallChannel,
            Receiver,
            ProviderCallSite,
            ProviderCallSiteSupport,
            CallTarget,
            CallTargetSupport,
            CallResolution,
            CallResolutionSupport,
            CallResolutionMember,
            ClassMetadataObservation,
        ClassMetadataSupport,
        ClassMemberObservation,
        ClassMemberSupport,
        RecordOptions,
        RecordTransformDefaults,
        RecordTransformFieldSpecifier,
        TypeTerm,
            TypeSequence,
            TypeSequenceMember,
            CallableParameterList,
            CallableParameter,
            TypedDictFieldList,
            TypedDictField,
            TypeVariable,
            TypeVariableRestriction,
            TypeRestrictionSupport,
            TypeObservation,
            TypeSupport,
            TypePresentation,
            TypePresentationSupport,
            FunctionBodyObservation,
            FunctionBodySupport,
            RecordFieldObservation,
            RecordFieldSupport
        );
        let provider = pyrefly_provider();
        let (condition, nodes) = Diagram::always().records();
        context.contribute(provider.clone())?;
        context.contribute(condition.clone())?;
        for node in nodes {
            context.contribute(node)?;
        }
        let captured = context.captured();
        let mut partial = false;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                Acquisition::Corpus { library, .. } => Some(
                    captured
                        .inputs()
                        .get(*library)
                        .ok_or_else(|| invalid("a corpus names an absent library input".into()))?,
                ),
                _ => None,
            };
            partial |= session(
                context,
                &provider,
                &condition,
                input,
                library,
                self.limits,
                self.tap.as_ref(),
            )?;
        }
        Ok(if partial {
            ProviderOutcome::Partial
        } else {
            ProviderOutcome::Complete
        })
    }
}

/// The interpreter an input is analyzed for.
fn interpreter(
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
) -> Result<(String, String), ModelError> {
    Ok(match library.unwrap_or(input).acquisition() {
        Acquisition::Installed(library) => {
            (library.python_version.clone(), library.platform.clone())
        }
        _ => ("3.14.7".into(), "linux".into()),
    })
}
/// The shared, frozen analysis context used by both linked providers.
pub fn analysis_context(
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
    config: &crate::native_context::NativeContextConfig,
) -> Result<AnalysisContext, ModelError> {
    Ok(configured(input, library, config)?.1)
}
fn configured(
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
    native: &crate::native_context::NativeContextConfig,
) -> Result<(pyrefly_config::config::ConfigFile, AnalysisContext), ModelError> {
    use pyrefly_config::config::{ConfigFile, ConfigSource};
    use pyrefly_python::sys_info::{PythonPlatform, PythonVersion};
    let captured = input.captured();
    let root = captured.root();
    let (python, platform) = interpreter(input, library)?;
    let (major, minor, micro) =
        crate::library::version_triple(&python).map_err(|e| invalid(e.to_string()))?;
    let mut cfg = ConfigFile {
        source: ConfigSource::File(root.join("pyrefly.toml")),
        search_path_from_args: vec![root.to_path_buf()],
        disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true,
        enable_fallback_search_path: false,
        ..ConfigFile::default()
    };
    cfg.python_environment.python_version = Some(PythonVersion::new(major, minor, micro));
    cfg.python_environment.python_platform = Some(PythonPlatform::new(&platform));
    cfg.python_environment.site_package_path = Some(
        library
            .map(|l| vec![l.captured().root().to_path_buf()])
            .unwrap_or_default(),
    );
    cfg.interpreters.skip_interpreter_query = true;
    if !cfg.configure().is_empty() {
        return Err(invalid(
            "the pinned analyzer configuration does not validate".into(),
        ));
    }
    let mut config = serde_json::to_value(&cfg).map_err(ModelError::codec)?;
    relativize(&mut config, root, "$input");
    if let Some(library) = library {
        relativize(&mut config, library.captured().root(), "$library");
    }
    let environment = library.unwrap_or(input);
    let lock_digest = match environment.acquisition() {
        Acquisition::Installed(library) => Some(library.lock_digest),
        _ => None,
    };
    let mut identity = lctx_model::domain::KeySink::new("native-analysis-config");
    identity.part(
        b"analyzer",
        &serde_json::to_vec(&config).map_err(ModelError::codec)?,
    );
    identity.part(b"context", &native.digest().0);
    let analysis = AnalysisContext {
        python_version: python,
        python_platform: platform,
        search_path: vec!["$input".into()],
        site_package_path: library.map(|_| vec!["$library".into()]).unwrap_or_default(),
        config_digest: identity.finish(),
        environment_digest: environment.captured().revision().manifest,
        lock_digest,
    };
    Ok((cfg, analysis))
}
/// One input's session. Returns whether any of its coverage is less than complete.
fn session<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    provider: &Provider,
    condition: &Condition,
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
    limits: SyntaxLimits,
    tap: Option<&PysaTap>,
) -> Result<bool, ModelError> {
    use pyrefly::state::{require::Require, state::State};
    use pyrefly_config::{error_kind::ErrorKind, finder::ConfigFinder};
    use pyrefly_python::{
        module_path::ModulePath,
        sys_info::{PythonPlatform, PythonVersion},
    };
    use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
    let captured = input.captured();
    let root = captured.root();
    let selected = context.captured();
    let (cfg, analysis) = configured(input, library, selected.config())?;
    let requirements = selected
        .config()
        .requirements(&analysis.python_version, context.budget())?;
    let (major, minor, micro) = crate::library::version_triple(&analysis.python_version)
        .map_err(|e| invalid(e.to_string()))?;
    let (run, families) = ProviderRun::new(
        provider.id(),
        analysis.id(),
        captured.revision().id(),
        analysis.config_digest,
        FAMILIES.into_iter().filter(|family| *family!=FactFamily::Syntax),
    )?;
    let surfaces: BTreeMap<FactFamily, ProviderSurface> = FAMILIES
        .iter()
        .map(|family| {
            (
                *family,
                ProviderSurface {
                    provider: provider.id(),
                    family: *family,
                    name: "retained Ruff parse".into(),
                },
            )
        })
        .collect();
    let ruff=crate::ruff_context::provider();
    let (ruff_run,ruff_families)=ProviderRun::new(ruff.id(),analysis.id(),captured.revision().id(),analysis.config_digest,[FactFamily::Syntax])?;
    let ruff_surface=ProviderSurface { provider:ruff.id(),family:FactFamily::Syntax,name:"canonical parse and populated contextual pass".into() };
    context.contribute(ruff)?;
    context.contribute(ruff_run.clone())?;
    for family in ruff_families {context.contribute(family)?;}
    context.contribute(ruff_surface.clone())?;
    context.contribute(analysis.clone())?;
    context.contribute(run.clone())?;
    for family in families {
        context.contribute(family)?;
    }
    for surface in surfaces.values() {
        context.contribute(surface.clone())?;
    }
    // Admission precedes the analyzer: a refused or undecodable root is never handed to Pyrefly.
    let roots = roots(input)?;
    let mut withheld = BTreeMap::new();
    let mut analyzed = Vec::new();
    for artifact in &roots {
        if typed_syntax::admit(artifact, limits).is_err() {
            withheld.insert(artifact.id(), ObligationKind::ResourceRefused);
            continue;
        }
        let _held = context.budget().reserve(
            "syntax_source_check",
            usize::try_from(artifact.byte_len).unwrap_or(usize::MAX),
        )?;
        let bytes = std::fs::read(root.join(&artifact.path)).map_err(ModelError::codec)?;
        if std::str::from_utf8(&bytes).is_err() {
            withheld.insert(artifact.id(), ObligationKind::UndecodableSource);
            continue;
        }
        analyzed.push(*artifact);
    }
    let handles: Vec<_> = analyzed
        .iter()
        .map(|a| cfg.handle_from_module_path(ModulePath::filesystem(root.join(&a.path))))
        .collect();
    let state = State::new(
        ConfigFinder::new_constant(ArcId::new(cfg)),
        ThreadCount::Inline,
    );
    let mut transaction = state.new_transaction(Require::Exports, None);
    // Pysa numbers modules through a reporter that writes nothing; dependencies are numbered lazily.
    transaction.set_pysa_reporter(Some(Box::new(pyrefly::report::pysa::PysaReporter {
        module_ids: pyrefly::report::pysa::module::ModuleIds::new(&handles),
        pysa_directory: Default::default(),
        definitions_directory: Default::default(),
        type_of_expressions_directory: Default::default(),
        call_graphs_directory: Default::default(),
        format: pyrefly::report::pysa::PysaFormat::Json,
        write_files: false,
    })));
    transaction.run(&handles, Require::Everything, None);
    let mut frozen = vec![(root, captured.artifacts())];
    if let Some(library) = library {
        frozen.push((library.captured().root(), library.captured().artifacts()));
    }
    let mut natives = Natives::new(provider.id(), analysis.id(), frozen, context.budget())?;
    // Pysa's reports are evidence of the invocation, not of one span.
    let pysa_evidence = Evidence::Invocation { run: run.id() };
    context.contribute(pysa_evidence.clone())?;
    let sys = pyrefly_python::sys_info::SysInfo::new(
        PythonVersion::new(major, minor, micro),
        PythonPlatform::new(&analysis.python_platform),
    );
    let outside = outside_names(&transaction, handles.first());
    let mut partial = false;
    // What the public names and dependency context need after every module is stated.
    let mut accesses: Vec<(
        usize,
        lctx_model::domain::Id<Module>,
        lctx_model::domain::Id<AssertionQualification>,
    )> = vec![];
    let mut imports: Vec<(usize, syntax_records::ImportLookup)> = vec![];
    let mut root_modules = std::collections::BTreeSet::new();
    let coverage = |scope: &CoverageScope,
                    family: FactFamily,
                    status: CoverageStatus,
                    reason: Option<ObligationKind>,
                    diagnostic: Option<String>| ProviderCoverage {
        scope: scope.id(),
        provider: Some(if family==FactFamily::Syntax {ruff_surface.provider} else {provider.id()}),
        context: analysis.id(),
        family,
        run: Some(if family==FactFamily::Syntax {ruff_run.id()} else {run.id()}),
        status,
        reason,
        diagnostic,
    };
    for artifact in roots {
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        context.contribute(scope.clone())?;
        if let Some(reason) = withheld.get(&artifact.id()) {
            for family in FAMILIES {
                context.contribute(coverage(
                    &scope,
                    family,
                    CoverageStatus::Unavailable,
                    Some(*reason),
                    None,
                ))?;
            }
            partial = true;
            continue;
        }
        let index = analyzed
            .iter()
            .position(|a| a.id() == artifact.id())
            .expect("an analyzed root");
        let handle = &handles[index];
        let module_name = handle.module().to_string();
        let is_package = matches!(
            artifact.path.rsplit('/').next(),
            Some("__init__.py" | "__init__.pyi")
        );
        let typed_module = Module {
            source: artifact.id(),
            qualified_name: module_name.clone(),
        };
        let module_id = typed_module.id();
        context.emit(typed_module)?;
        root_modules.insert(natives.module(&module_name, handle.path())?);
        let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: analysis.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        context.contribute(qualification.clone())?;
        accesses.push((index, module_id, qualification.id()));
        let native_ast = transaction
            .get_ast(handle)
            .ok_or_else(|| invalid("the analyzer did not retain the module's AST".into()))?;
        let info = transaction
            .get_module_info(handle)
            .ok_or_else(|| invalid("the analyzer did not retain the module's text".into()))?;
        let text = info.lined_buffer().contents().clone();
        let canonical = crate::ruff_context::CanonicalSyntax::parse(artifact, &text, &analysis, selected.config().ruff_settings().clone(), context.budget())?;
        let ast = canonical.module();
        let static_decisions = crate::native_branches::NativeBranches::observe(&native_ast, &sys, context.budget())?;

        let invocation = SyntaxInvocation {
            source: artifact,
            qualification: &qualification,
            run: &ruff_run,
            surface: &ruff_surface,
        };
        let support = |family: FactFamily, subject: lctx_model::domain::Id<Occurrence>| {
            (
                if family==FactFamily::Syntax {ruff_run.id()} else {run.id()},
                if family==FactFamily::Syntax {ruff_surface.id()} else {surfaces[&family].id()},
                Evidence::Occurrence {
                    occurrence: subject,
                }
                .id(),
            )
        };
        let mut spans = Spans::new(context.budget());
        let emitted = typed_syntax::emit(&ast, &text, invocation, limits, |event| {
            let occurrence = event.occurrence.id();
            spans.insert(&event.occurrence)?;
            if let Some((parent, field, _)) = event.placement {
                spans.place(occurrence, parent, field)?;
            }
            context.contribute(Evidence::Occurrence { occurrence })?;
            context.emit(event.occurrence)?;
            if let Some((observation, _, observed)) = event.observation {
                context.emit(observation)?;
                context.emit(observed)?;
            }
            let (parent, field, ordinal) = event
                .placement
                .map_or((None, SyntaxField::Child, 0), |(parent, field, ordinal)| {
                    (Some(parent), field, ordinal)
                });
            let placement = SyntaxPlacement {
                qualification: qualification.id(),
                occurrence,
                parent,
                field,
                ordinal,
            };
            let (run, surface, evidence) = support(FactFamily::Syntax, occurrence);
            context.emit(SyntaxPlacementSupport {
                assertion: placement.id(),
                run,
                surface,
                evidence,
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })?;
            context.emit(placement)?;
            for (ordinal, (detail, literal)) in event.details.into_iter().enumerate() {
                let observation = SyntaxDetailObservation {
                    qualification: qualification.id(),
                    occurrence,
                    ordinal: ordinal as i64,
                    detail: detail.id(),
                };
                context.emit(SyntaxDetailSupport {
                    assertion: observation.id(),
                    run,
                    surface,
                    evidence,
                    origin: Origin::SourceObservation,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                })?;
                context.emit(observation)?;
                context.contribute(detail)?;
                if let Some(literal) = literal {
                    context.contribute(literal)?;
                }
            }
            Ok(())
        });
        let errors = transaction.get_errors([handle]).collect_errors();
        let native_parse_error = [
            &errors.ordinary,
            &errors.directives,
            &errors.suppressed,
            &errors.disabled,
            &errors.baseline,
        ]
        .into_iter()
        .flatten()
        .any(|error| error.error_kind() == ErrorKind::ParseError);
        let parse_error = !canonical.parsed().errors().is_empty();
        let mut ruff_context_incomplete = None;
        let (mut unattached, mut unlocated, mut computed_all, mut calls_partial, mut types_partial) =
            ((0, 0), 0, false, false, false);
        let syntax = match emitted {
            Ok(_) => {
                let contextual=canonical.context_rows(&spans,&qualification,context.budget())?;
                if contextual.incomplete.is_some() || contextual.unlocated>0 {ruff_context_incomplete=Some(format!("contextual pass: {:?}; {} unattached source contexts",contextual.incomplete,contextual.unlocated));}
                for row in &contextual.rows {
                    context.emit(RuffContextSupport { assertion:row.id(),run:ruff_run.id(),surface:ruff_surface.id(),evidence:Evidence::Occurrence {occurrence:row.subject}.id(),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural })?;
                    context.emit(row.clone())?;
                }
                let records = syntax_records::records(
                    &ast,
                    &module_name,
                    is_package,
                    &spans,
                    qualification.id(),
                    &contextual.rows,
                )?;
                let computed = records.computed_all.clone();
                computed_all = !computed.is_empty();
                imports.extend(records.import_lookups.iter().cloned().map(|lookup| (index, lookup)));
                let declared = symbol_records::Declared::new(
                    &records.declarations,
                    &records.parameters,
                    &records.formals,
                )?;
                let call_syntax = records.calls.clone();
                let type_syntax = records.clone();
                write_records(context, records, &|family, subject| {
                    support(family, subject)
                })?;
                let stars = star_imports(&transaction, handle, &module_name, is_package, &ast);
                let candidate = AssertionQualification {
                    modality: Modality::Candidate,
                    ..qualification.clone()
                };
                context.contribute(candidate.clone())?;
                let facts = lexical_records::facts(ast, &spans, &static_decisions, &outside, &stars)?;
                let lexical =
                    lexical_records::records(&facts, &spans, qualification.id(), candidate.id())?;
                write_lexical(context, lexical, &|subject| {
                    support(FactFamily::Lexical, subject)
                })?;
                let locator = symbol_records::Locator {
                    line_index: info.lined_buffer().line_index(),
                    text: &text,
                };
                let linking = symbol_records::Linking {
                    locator: &locator,
                    spans: &spans,
                    declared: &declared,
                };
                let definitions = definitions(
                    &transaction,
                    handle,
                    &qualification,
                    &mut natives,
                    Some(linking),
                    None,
                    tap,
                    false,
                    context.budget(),
                )?;
                unattached = write_symbols(
                    context,
                    definitions,
                    &scope,
                    provider,
                    &analysis,
                    &run,
                    &surfaces,
                    &pysa_evidence,
                )?;
                let calls = calls(
                    &transaction,
                    handle,
                    &qualification,
                    provider,
                    &locator,
                    &spans,
                    &call_syntax,
                    &mut natives,
                    context.budget(),
                )?;
                calls_partial = !calls.boundaries.is_empty() || !calls.unattached.is_empty();
                write_calls(
                    context,
                    calls,
                    &scope,
                    provider,
                    &analysis,
                    &run,
                    &surfaces,
                    &pysa_evidence,
                )?;
                let types = types(
                    &transaction,
                    handle,
                    &qualification,
                    &analysis,
                    provider,
                    &type_syntax,
                    &spans,
                    &mut natives,
                    context.budget(),
                    &analyzed,
                    limits,
                    &ruff_run,
                    &ruff_surface,
                    root,
                )?;
                types_partial = native_parse_error || !types.boundaries.is_empty();
                write_types(
                    context,
                    types,
                    &scope,
                    provider,
                    &analysis,
                    &run,
                    &surfaces,
                    &pysa_evidence,
                )?;
                let docs = symbol_records::parameter_docs(
                    &ast,
                    &text,
                    artifact.id(),
                    &spans,
                    qualification.id(),
                )?;
                unlocated = docs.unlocated.len();
                write_docs(context, docs, &scope, provider, &analysis, &run, &surfaces)?;
                for statement in computed {
                    context.contribute(SubjectBoundary {
                        scope: scope.id(),
                        provider: provider.id(),
                        context: analysis.id(),
                        family: FactFamily::Exports,
                        subject: Some(statement),
                        reason: ObligationKind::OutsideProviderModel,
                        detail: Some(
                            "__all__ is computed; its names are not stated by the syntax".into(),
                        ),
                    })?;
                }
                if parse_error {
                    (
                        CoverageStatus::Partial,
                        Some(ObligationKind::SyntaxError),
                        Some(
                            "the module parsed with errors; facts come from the recovered tree"
                                .into(),
                        ),
                    )
                } else if let Some(detail)=ruff_context_incomplete.take() {
                    (CoverageStatus::Partial,Some(ObligationKind::OutsideProviderModel),Some(detail))
                } else {
                    (CoverageStatus::CompleteUnderStatedModel, None, None)
                }
            }
            Err(error) => match error.coverage() {
                Some((status, reason)) => (status, Some(reason), Some(error.to_string())),
                None => {
                    return Err(match error {
                        typed_syntax::SyntaxError::Model(error) => error,
                        other => invalid(other.to_string()),
                    });
                }
            },
        };
        context.contribute(coverage(
            &scope,
            FactFamily::Syntax,
            syntax.0,
            syntax.1,
            syntax.2.clone(),
        ))?;
        context.contribute(coverage(
            &scope,
            FactFamily::Lexical,
            syntax.0,
            syntax.1,
            syntax.2.clone(),
        ))?;
        // Exports and Signatures follow the syntax; a computed `__all__`, a declaration that did not
        // attach and an unlocated parameter description each leave their family partial.
        let complete = syntax.0 == CoverageStatus::CompleteUnderStatedModel;
        let exports = if complete && computed_all {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::OutsideProviderModel),
                Some("__all__ is computed; its names are not stated".into()),
            )
        } else {
            syntax.clone()
        };
        let signatures = if complete && unattached.1 > 0 {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::OutsideProviderModel),
                Some(format!(
                    "{} native signature variants are unavailable for binding",
                    unattached.1
                )),
            )
        } else if complete && unattached.0 > 0 {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::AttachmentUnmatched),
                Some(format!(
                    "{} declarations did not attach at their name spans",
                    unattached.0
                )),
            )
        } else if complete && unlocated > 0 {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::ProviderDisagreement),
                Some(format!(
                    "{unlocated} parameter descriptions are not located"
                )),
            )
        } else {
            syntax.clone()
        };
        let calls = if complete && calls_partial {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::MissingEvidence),
                Some("native call attachment or Ruff/Pysa boundary".into()),
            )
        } else {
            syntax.clone()
        };
        context.contribute(coverage(
            &scope,
            FactFamily::Calls,
            calls.0,
            calls.1,
            calls.2.clone(),
        ))?;
        let types = if complete && types_partial {
            (
                CoverageStatus::Partial,
                Some(ObligationKind::MissingEvidence),
                Some("native type or attachment boundary".into()),
            )
        } else {
            syntax.clone()
        };
        context.contribute(coverage(
            &scope,
            FactFamily::Types,
            types.0,
            types.1,
            types.2.clone(),
        ))?;
        partial |= [&syntax, &exports, &signatures, &calls, &types]
            .iter()
            .any(|c| c.0 != CoverageStatus::CompleteUnderStatedModel);
        context.contribute(coverage(
            &scope,
            FactFamily::Exports,
            exports.0,
            exports.1,
            exports.2,
        ))?;
        context.contribute(coverage(
            &scope,
            FactFamily::Signatures,
            signatures.0,
            signatures.1,
            signatures.2,
        ))?;
    }
    // The public names of the analyzed modules, under each module's own qualification.
    let access: Vec<public_records::Access<'_>> = accesses
        .iter()
        .map(|(index, module, qualification)| public_records::Access {
            handle: &handles[*index],
            module: *module,
            qualification: *qualification,
        })
        .collect();
    let public = public_records::public_names(&access, &transaction, &mut natives)?;
    drop(access);
    let exports = surfaces[&FactFamily::Exports].id();
    for origin in public.origins {
        context.emit(origin)?;
    }
    for row in public.names {
        context.emit(PublicNameSupport {
            assertion: row.id(),
            run: run.id(),
            surface: exports,
            evidence: pysa_evidence.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        })?;
        context.emit(row)?;
    }
    // The input's dependency context: every module its facts or imports reference, and the
    // referenced definitions in them, under the input's qualification.
    let input_scope = CoverageScope::Input {
        input: captured.revision().id(),
    };
    let input_qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: analysis.id(),
        scope: input_scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let mut input_boundaries = (0, 0);
    context.contribute(input_scope.clone())?;
    context.contribute(input_qualification.clone())?;
    for (index, lookup) in imports {
        let resolution = resolve_import(&transaction, &handles[index], &lookup, &mut natives)?;
        let row = ModuleResolutionObservation {
            qualification: lookup.qualification,
            module: resolution.module,
            alias: Some(lookup.alias),
            location: resolution.location,
        };
        context.emit(ModuleResolutionSupport {
            assertion: row.id(), run: run.id(), surface: surfaces[&FactFamily::Exports].id(),
            evidence: Evidence::Occurrence { occurrence: lookup.alias }.id(),
            origin: Origin::AnalyzerAssertion, mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        })?;
        context.emit(row)?;
    }
    let mut keep: BTreeMap<
        lctx_model::domain::Id<ProviderModule>,
        (std::collections::BTreeSet<String>, Vec<String>),
    > = BTreeMap::new();
    for symbol in natives
        .symbols
        .values()
        .filter(|s| !root_modules.contains(&s.module))
    {
        keep.entry(symbol.module)
            .or_default()
            .0
            .insert(symbol.native_key.clone());
    }
    for (found, name) in public.traced {
        let module = natives.module(&found.module().to_string(), found.path())?;
        if !root_modules.contains(&module) {
            keep.entry(module).or_default().1.push(name);
        }
    }
    let mut work = lctx_model::domain::charged::StateCharge::new(
        context.budget(),
        "native-model-context-selection",
    );
    let mut requested = Vec::new();
    if let Some(requirements) = &requirements {
        for entry in requirements.entries() {
            work.grow(
                entry
                    .module
                    .len()
                    .saturating_add(entry.qualified_name.len())
                    .saturating_mul(4)
                    .saturating_add(384),
            )?;
            let mut problem =
                requirement_pin_problem(entry, input, library, &analysis.python_version);
            let found = handles.first().and_then(|anchor| {
                transaction
                    .import_handle(
                        anchor,
                        pyrefly_python::module_name::ModuleName::from_str(&entry.module),
                        None,
                    )
                    .finding()
            });
            let module = match found {
                Some(found) => {
                    let module = natives.module(&found.module().to_string(), found.path())?;
                    if problem.is_none() {
                        problem = requirement_location_problem(entry, found.path(), input, library);
                    }
                    if !root_modules.contains(&module) {
                        keep.entry(module)
                            .or_default()
                            .1
                            .push(entry.qualified_name.clone());
                    }
                    Some(module)
                }
                None => {
                    natives.unresolved(&entry.module)?;
                    problem.get_or_insert_with(|| {
                        "definition module is unavailable in the frozen analyzer context".into()
                    });
                    None
                }
            };
            requested.push((entry, module, problem));
        }
    }
    if let Some(anchor) = handles.first() {
        let mut visited = std::collections::BTreeSet::new();
        loop {
            let dependencies: Vec<_> = keep
                .keys()
                .filter(|m| !visited.contains(*m))
                .filter_map(|module| {
                    natives.paths.get(module).map(|(name, path)| {
                        (
                            *module,
                            pyrefly_build::handle::Handle::new(
                                pyrefly_python::module_name::ModuleName::from_str(name),
                                path.clone(),
                                *anchor.sys_info(),
                            ),
                        )
                    })
                })
                .collect();
            if dependencies.is_empty() {
                break;
            }
            transaction.run(
                &dependencies
                    .iter()
                    .map(|(_, h)| h.clone())
                    .collect::<Vec<_>>(),
                Require::Everything,
                None,
            );
            for (module, handle) in dependencies {
                visited.insert(module);
                let (keys, names) = &keep[&module];
                let records = definitions(
                    &transaction,
                    &handle,
                    &input_qualification,
                    &mut natives,
                    None,
                    Some((keys, names)),
                    None,
                    requirements.is_some(),
                    context.budget(),
                )?;
                // A class's complete native MRO remains structural evidence. Retain every ancestor
                // module's class declarations too, each module once in this same dependency pass.
                if requirements.is_some() {
                    for member in records.sequences.iter().flat_map(|(_, members)| members) {
                        let symbol = &natives.symbols[&member.symbol];
                        if !root_modules.contains(&symbol.module)
                            && !visited.contains(&symbol.module)
                        {
                            let keys = &mut keep.entry(symbol.module).or_default().0;
                            if keys.insert(symbol.native_key.clone()) {
                                work.grow(
                                    symbol
                                        .native_key
                                        .len()
                                        .saturating_mul(2)
                                        .saturating_add(128),
                                )?;
                            }
                        }
                    }
                }
                let boundaries = write_symbols(
                    context,
                    records,
                    &input_scope,
                    provider,
                    &analysis,
                    &run,
                    &surfaces,
                    &pysa_evidence,
                )?;
                input_boundaries.0 += boundaries.0;
                input_boundaries.1 += boundaries.1;
            }
        }
    }
    for (entry, module, mut problem) in requested {
        if problem.is_none() {
            let symbol = module.and_then(|m| {
                natives
                    .definitions
                    .get(&(m, entry.qualified_name.clone()))
                    .copied()
            });
            match symbol {
                None => {
                    problem = Some(
                        "required exact class/function/member definition is unavailable".into(),
                    )
                }
                Some(symbol)
                    if entry.require_mro
                        && natives.mro.get(&symbol) != Some(&Linearization::Complete) =>
                {
                    problem = Some("required class ancestry is not a complete native MRO".into())
                }
                Some(_) => {}
            }
        }
        if let Some(problem) = problem {
            input_boundaries.1 += 1;
            context.contribute(SubjectBoundary {
                scope: input_scope.id(),
                provider: provider.id(),
                context: analysis.id(),
                family: FactFamily::Signatures,
                subject: None,
                reason: ObligationKind::OutsideProviderModel,
                detail: Some(format!(
                    "model-context {:?} {:?} {}.{}: {}",
                    entry.owner, entry.pin, entry.module, entry.qualified_name, problem
                )),
            })?;
        }
    }
    let exports = surfaces[&FactFamily::Exports].id();
    let resolutions: Vec<_> = natives
        .resolutions
        .values()
        .cloned()
        .collect();
    for resolution in resolutions {
        let row = ModuleResolutionObservation {
            qualification: input_qualification.id(),
            module: resolution.module,
            alias: None,
            location: resolution.location,
        };
        context.emit(ModuleResolutionSupport {
            assertion: row.id(),
            run: run.id(),
            surface: exports,
            evidence: pysa_evidence.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        })?;
        context.emit(row)?;
    }
    for (module, typed) in natives.modules.values() {
        if let Some(typed) = typed {
            context.emit(typed.clone())?;
        }
        context.emit(module.clone())?;
    }
    for symbol in natives.symbols.values() {
        context.emit(symbol.clone())?;
    }
    context.contribute(coverage(&input_scope, FactFamily::Signatures,
        if input_boundaries == (0, 0) { CoverageStatus::CompleteUnderStatedModel } else { partial = true; CoverageStatus::Partial },
        if input_boundaries.1 > 0 { Some(ObligationKind::OutsideProviderModel) } else if input_boundaries.0 > 0 { Some(ObligationKind::AttachmentUnmatched) } else { None },
        Some("supporting definitions referenced by requested roots: imports, re-exports, call targets and nominal type references".into())))?;
    Ok(partial)
}

/// Resolve each source alias with actual provider lookups. Native module-valued bindings retain
/// their target (including re-exported modules); other names require a real export in the found
/// base module, or a successful submodule lookup. No assumed path is passed to the resolver.
fn resolve_import(
    transaction: &pyrefly::state::state::Transaction<'_>,
    importer: &pyrefly_build::handle::Handle,
    lookup: &syntax_records::ImportLookup,
    natives: &mut Natives,
) -> Result<crate::natives::Resolution, ModelError> {
    use pyrefly_python::module_name::ModuleName;
    use pyrefly_types::types::Type;
    let mut requested = lookup.base.clone().unwrap_or_else(|| lookup.spelling.clone());
    let found = if let Some(base) = &lookup.base {
        if let Some(member) = &lookup.member {
            if let Some(Type::Module(module)) = transaction.get_type_at_preserving_declaration(importer, ruff_text_size::TextSize::new(lookup.binding_position.to_u32())) {
                requested = module.parts().iter().map(|part| part.as_str()).collect::<Vec<_>>().join(".");
                transaction.import_handle(importer, ModuleName::from_str(&requested), None).finding()
            } else if let Some(base_handle) = transaction.import_handle(importer, ModuleName::from_str(base), None).finding() {
                if member == "*" || transaction.get_exports(&base_handle).contains_key(&ruff_python_ast::name::Name::new(member)) {
                    Some(base_handle)
                } else {
                    requested = format!("{base}.{member}");
                    transaction.import_handle(importer, ModuleName::from_str(&requested), None).finding()
                }
            } else { None }
        } else {
            transaction.import_handle(importer, ModuleName::from_str(base), None).finding()
        }
    } else { None };
    let module = match found {
        Some(found) => natives.module(&found.module().to_string(), found.path())?,
        None => natives.unresolved(&requested)?,
    };
    Ok(natives.resolutions[&module].clone())
}

/// Pysa's definitions of one module as symbol records: an analyzed module linked to its
fn requirement_pin_problem(
    entry: &lctx_model::domain::models::requirements::RequiredDefinition,
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
    python: &str,
) -> Option<String> {
    use lctx_model::domain::models::requirements::RequirementPin;
    match &entry.pin {
        RequirementPin::Python(expected) if expected != python => Some(format!(
            "model requires Python {expected}, captured interpreter is {python}"
        )),
        RequirementPin::Distribution { name, version } => {
            match library.unwrap_or(input).acquisition() {
                Acquisition::Installed(inventory) => {
                    match inventory.distributions.iter().find(|d| &d.name == name) {
                        Some(distribution) if &distribution.version == version => None,
                        Some(distribution) => Some(format!(
                            "model requires {name}=={version}, captured distribution is {}",
                            distribution.version
                        )),
                        None => Some(format!(
                            "required distribution {name}=={version} is absent from captured inventory"
                        )),
                    }
                }
                _ => Some(format!(
                    "required distribution {name}=={version} has no pinned captured inventory"
                )),
            }
        }
        _ => None,
    }
}
fn requirement_location_problem(
    entry: &lctx_model::domain::models::requirements::RequiredDefinition,
    path: &pyrefly_python::module_path::ModulePath,
    input: &AcquiredInput,
    library: Option<&AcquiredInput>,
) -> Option<String> {
    use lctx_model::domain::models::requirements::RequirementPin;
    use pyrefly_python::module_path::ModulePathDetails;
    match (&entry.pin, path.details()) {
        (RequirementPin::Python(_), ModulePathDetails::BundledTypeshed(_)) => None,
        (RequirementPin::Python(_), _) => {
            Some("stdlib requirement resolved outside the pinned provider stdlib".into())
        }
        (RequirementPin::Distribution { name, .. }, ModulePathDetails::FileSystem(file)) => {
            let environment = library.unwrap_or(input);
            let Acquisition::Installed(inventory) = environment.acquisition() else {
                return Some("dependency has no captured installed inventory".into());
            };
            let relative = file
                .as_path()
                .strip_prefix(environment.captured().root())
                .ok()
                .map(|p| p.to_string_lossy());
            if relative.is_some_and(|p| {
                inventory
                    .files
                    .iter()
                    .any(|f| f.path == p && f.owners.contains(name))
            }) {
                None
            } else {
                Some("resolved module is not owned by the required captured distribution".into())
            }
        }
        (RequirementPin::Release, ModulePathDetails::FileSystem(file)) => {
            let environment = library.unwrap_or(input);
            let relative = file
                .as_path()
                .strip_prefix(environment.captured().root())
                .ok()
                .map(|p| p.to_string_lossy());
            match environment.acquisition() {
                Acquisition::Installed(inventory)
                    if relative.as_ref().is_some_and(|p| {
                        inventory.files.iter().any(|f| {
                            f.path == p.as_ref()
                                && f.role == lctx_model::domain::input::SourceRole::Release
                                && f.owners.iter().any(|owner| {
                                    inventory
                                        .distributions
                                        .iter()
                                        .any(|d| d.name == *owner && d.first_party)
                                })
                        })
                    }) =>
                {
                    None
                }
                Acquisition::Tree { .. } if relative.is_some() => None,
                _ => Some("release model target is not an acquired release definition".into()),
            }
        }
        (RequirementPin::CapturedEnvironment, ModulePathDetails::BundledTypeshed(_)) => None,
        (RequirementPin::CapturedEnvironment, ModulePathDetails::FileSystem(_)) => None,
        _ => Some("required definition is not present in the captured pinned environment".into()),
    }
}

/// occurrences, or a dependency module's kept definitions (native keys, and top-level names an
/// export traces to).
#[allow(
    clippy::too_many_arguments,
    reason = "One definition walk carries the transaction, handle, qualification, natives, linking, keep set, tap and MRO flag"
)]
fn definitions(
    transaction: &pyrefly::state::state::Transaction<'_>,
    handle: &pyrefly_build::handle::Handle,
    qualification: &AssertionQualification,
    natives: &mut Natives,
    linking: Option<symbol_records::Linking<'_>>,
    keep: Option<(&std::collections::BTreeSet<String>, &[String])>,
    tap: Option<&PysaTap>,
    expand_mro: bool,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<symbol_records::SymbolRecords, ModelError> {
    let module_name = handle.module().to_string();
    let module_name = module_name.as_str();
    use pyrefly::report::pysa::{
        captured_variable::collect_captured_variables_for_module,
        class::{ClassId, get_all_classes, get_class_mro},
        context::{ModuleAnswersContext, ModuleContext, PysaResolver},
        export_module_definitions,
        override_graph::create_reversed_override_graph_for_module,
    };
    let module_ids = &transaction
        .pysa_reporter()
        .ok_or_else(|| invalid("the Pysa reporter is not installed".into()))?
        .module_ids;
    let this = natives.module(module_name, handle.path())?;
    let current = module_ids.get_from_handle(handle);
    let resolver = PysaResolver::new(transaction, module_ids, handle.clone());
    let pysa = ModuleContext {
        answers_context: ModuleAnswersContext::create(handle.clone(), transaction, module_ids),
        resolver: &resolver,
    };
    let captured = collect_captured_variables_for_module(&pysa);
    let overrides = create_reversed_override_graph_for_module(&pysa);
    let definitions = export_module_definitions(&pysa, &captured, &overrides);
    if let Some(tap) = tap {
        let graphs = pyrefly::report::pysa::export_module_call_graphs(&pysa, &captured);
        let report = serde_json::json!({ "definitions": serde_json::to_value(&definitions).map_err(ModelError::codec)?,
            "call_graphs": serde_json::to_value(&graphs).map_err(ModelError::codec)? });
        tap.lock()
            .map_err(|_| invalid("the Pysa tap is poisoned".into()))?
            .insert(module_name.to_owned(), report);
    }
    // The report drops the MRO's completeness; the analyzer states it.
    let complete: std::collections::HashMap<u32, bool> = get_all_classes(&pysa.answers_context)
        .map(|class| {
            (
                ClassId::from_class(&class).to_int(),
                get_class_mro(&class, &pysa.answers_context).linearization_complete(),
            )
        })
        .collect();
    // Pysa intentionally omits name_location for every ClassField, including source callable
    // attributes. Read the native declaration map to distinguish them from synthesized methods.
    let native_classes: std::collections::HashMap<_, _> = get_all_classes(&pysa.answers_context)
        .map(|class| (ClassId::from_class(&class), class))
        .collect();
    let origins = definitions
        .function_definitions
        .as_map()
        .iter()
        .map(|(id, function)| {
            use lctx_model::domain::symbols::FunctionOrigin;
            use pyrefly::report::pysa::function::FunctionId;
            let origin = match id {
                FunctionId::Function { .. } => FunctionOrigin::DefStatement,
                FunctionId::ClassField { class_id, .. } => {
                    native_classes
                        .get(class_id)
                        .map_or(FunctionOrigin::Unavailable, |class| {
                            if pyrefly::report::pysa::class::get_class_field_declaration(
                                class,
                                &function.base.name,
                                &pysa.answers_context,
                            )
                            .is_some()
                            {
                                FunctionOrigin::CallableField
                            } else {
                                FunctionOrigin::Synthesized
                            }
                        })
                }
                FunctionId::ModuleTopLevel
                | FunctionId::ClassTopLevel { .. }
                | FunctionId::FunctionDecoratedTarget { .. } => FunctionOrigin::Unavailable,
            };
            (id.serialize_to_string(), origin)
        })
        .collect();
    let mut resolve = |natives: &mut Natives,
                       id: pyrefly::report::pysa::module::ModuleId,
                       name: &pyrefly_python::module_name::ModuleName| {
        if id == current {
            return Ok(this);
        }
        let found = transaction
            .import_handle(handle, *name, None)
            .finding()
            .ok_or_else(|| {
                invalid(format!(
                    "Pysa references {name}, which does not resolve from {module_name}"
                ))
            })?;
        if module_ids.get_from_handle(&found) != id {
            return Err(invalid(format!(
                "{name} resolves to another module than the one Pysa referenced"
            )));
        }
        natives.module(&name.to_string(), found.path())
    };
    let kept = keep.map(|(keys, names)| {
        let mut kept = keys.clone();
        for name in names {
            kept.extend(symbol_records::qualified(&definitions, name));
        }
        if expand_mro {
            kept.extend(symbol_records::class_keys(&definitions));
        }
        kept
    });
    symbol_records::records(
        &definitions,
        this,
        qualification,
        natives,
        &mut resolve,
        &complete,
        &origins,
        linking,
        kept.as_ref(),
        budget,
    )
}

/// Emit one module's parameter documentation with its supports; an unlocated description is a
/// boundary of the module's Signatures coverage.
#[allow(
    clippy::too_many_arguments,
    reason = "the session's attribution, each distinct"
)]
fn write_docs<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    docs: symbol_records::ParameterDocs,
    scope: &CoverageScope,
    provider: &Provider,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    surfaces: &BTreeMap<FactFamily, ProviderSurface>,
) -> Result<(), ModelError> {
    for (row, evidence) in docs.docs {
        context.contribute(evidence.clone())?;
        context.emit(ParameterDocSupport {
            assertion: row.id(),
            run: run.id(),
            surface: surfaces[&FactFamily::Signatures].id(),
            evidence: evidence.id(),
            origin: Origin::SourceObservation,
            mode: ExtractionMode::Recognizer,
            fidelity: Fidelity::NormalizedStructural,
        })?;
        context.emit(row)?;
    }
    for detail in docs.unlocated {
        context.contribute(SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Signatures,
            subject: None,
            reason: ObligationKind::ProviderDisagreement,
            detail: Some(detail),
        })?;
    }
    Ok(())
}
/// Emit one module's symbol records with their supports; returns how many declarations did not
/// attach, each a boundary of the module's Signatures coverage.
#[allow(
    clippy::too_many_arguments,
    reason = "the session's attribution, each distinct"
)]
fn write_symbols<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    records: symbol_records::SymbolRecords,
    scope: &CoverageScope,
    provider: &Provider,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    surfaces: &BTreeMap<FactFamily, ProviderSurface>,
    invocation: &Evidence,
) -> Result<(usize, usize), ModelError> {
    let surface = surfaces[&FactFamily::Signatures].id();
    macro_rules! pysa {
        ($support:ident, $row:expr, $evidence:expr, $fidelity:expr) => {{
            let row = $row;
            context.emit($support {
                assertion: row.id(),
                run: run.id(),
                surface,
                evidence: $evidence,
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: $fidelity,
            })?;
            context.emit(row)?;
        }};
    }
    for (sequence, members) in records.sequences {
        context.emit(sequence)?;
        for member in members {
            context.emit(member)?;
        }
    }
    for shape in records.shapes {
        context.emit(shape)?;
    }
    for (signature, members) in records.signatures {
        pysa!(
            SignatureSupport,
            signature,
            invocation.id(),
            Fidelity::ReportProjection
        );
        for member in members {
            context.emit(member)?;
        }
    }
    for (enumeration, members) in records.enumerations {
        pysa!(
            SignatureEnumerationSupport,
            enumeration,
            invocation.id(),
            Fidelity::ReportProjection
        );
        for member in members {
            context.emit(member)?;
        }
    }
    for row in records.symbols {
        pysa!(
            SymbolSupport,
            row,
            invocation.id(),
            Fidelity::ReportProjection
        );
    }
    for row in records.functions {
        pysa!(
            FunctionTraitSupport,
            row,
            invocation.id(),
            Fidelity::ReportProjection
        );
    }
    for row in records.classes {
        pysa!(
            ClassTraitSupport,
            row,
            invocation.id(),
            Fidelity::ReportProjection
        );
    }
    for row in records.ancestry {
        pysa!(
            ClassAncestrySupport,
            row,
            invocation.id(),
            Fidelity::ReportProjection
        );
    }
    for row in records.annotations {
        pysa!(
            ParameterAnnotationSupport,
            row,
            invocation.id(),
            Fidelity::DisplayOnly
        );
    }
    for row in records.declarations {
        let evidence = Evidence::Occurrence {
            occurrence: row.declaration,
        }
        .id();
        pysa!(
            SymbolDeclarationSupport,
            row,
            evidence,
            Fidelity::ReportProjection
        );
    }
    for row in records.parameter_declarations {
        let evidence = Evidence::Occurrence {
            occurrence: row.declaration,
        }
        .id();
        pysa!(
            ParameterDeclarationSupport,
            row,
            evidence,
            Fidelity::ReportProjection
        );
    }
    let unattached = records.unattached.len();
    for (_, detail) in records.unattached {
        context.contribute(SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Signatures,
            subject: None,
            reason: ObligationKind::AttachmentUnmatched,
            detail: Some(detail),
        })?;
    }
    let unavailable = records.native_unavailable.len();
    for (_, detail) in records.native_unavailable {
        context.contribute(SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Signatures,
            subject: None,
            reason: ObligationKind::OutsideProviderModel,
            detail: Some(detail),
        })?;
    }
    Ok((unattached, unavailable))
}

type Support = (
    lctx_model::domain::Id<ProviderRun>,
    lctx_model::domain::Id<ProviderSurface>,
    lctx_model::domain::Id<Evidence>,
);
/// Emit one module's records, each with its native support.
fn write_records<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    records: syntax_records::Records,
    support: &dyn Fn(FactFamily, lctx_model::domain::Id<Occurrence>) -> Support,
) -> Result<(), ModelError> {
    macro_rules! with_support {
        ($rows:expr, $support:ident, $family:expr, $subject:ident) => {
            for row in $rows {
                let (run, surface, evidence) = support($family, row.$subject);
                context.emit($support {
                    assertion: row.id(),
                    run,
                    surface,
                    evidence,
                    origin: Origin::SourceObservation,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                })?;
                context.emit(row)?;
            }
        };
    }
    with_support!(
        records.declarations,
        DeclarationSupport,
        FactFamily::Syntax,
        declaration
    );
    with_support!(
        records.decorators,
        DeclarationDecoratorSupport,
        FactFamily::Syntax,
        decorator
    );
    with_support!(
        records.imports,
        ImportAliasSupport,
        FactFamily::Exports,
        alias
    );
    with_support!(
        records.dunder_all,
        DunderAllSupport,
        FactFamily::Exports,
        statement
    );
    with_support!(
        records.parameters,
        ParameterSyntaxSupport,
        FactFamily::Signatures,
        parameter
    );
    with_support!(
        records.fields,
        ClassFieldSyntaxSupport,
        FactFamily::Syntax,
        target
    );
    for (call, arguments) in records.calls {
        let (run, surface, evidence) = support(FactFamily::Syntax, call.site);
        context.emit(CallSyntaxSupport {
            assertion: call.id(),
            run,
            surface,
            evidence,
            origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        })?;
        context.emit(call)?;
        for argument in arguments {
            context.emit(argument)?;
        }
    }
    for literal in records.literals {
        context.contribute(literal)?;
    }
    for (set, members) in records.sets {
        context.contribute(set)?;
        for member in members {
            context.contribute(member)?;
        }
    }
    Ok(())
}

/// Emit one module's lexical records, each assertion with its support.
fn write_lexical<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    records: lexical_records::LexicalRecords,
    support: &dyn Fn(lctx_model::domain::Id<Occurrence>) -> Support,
) -> Result<(), ModelError> {
    let owners: std::collections::HashMap<_, _> =
        records.scopes.iter().map(|s| (s.id(), s.owner)).collect();
    let sites: std::collections::HashMap<_, _> =
        records.events.iter().map(|e| (e.id(), e.site)).collect();
    macro_rules! native {
        ($support:ident, $row:expr, $subject:expr) => {{
            let (run, surface, evidence) = support($subject);
            context.emit($support {
                assertion: $row.id(),
                run,
                surface,
                evidence,
                origin: Origin::DerivedAnalysis,
                mode: ExtractionMode::Recognizer,
                fidelity: Fidelity::NormalizedStructural,
            })?;
        }};
    }
    for scope in records.scopes {
        context.emit(scope)?;
    }
    for event in records.events {
        context.emit(event)?;
    }
    for target in records.targets {
        context.emit(target)?;
    }
    for row in records.scope_observations {
        native!(LexicalScopeSupport, row, owners[&row.scope]);
        context.emit(row)?;
    }
    for row in records.bindings {
        native!(BindingSupport, row, sites[&row.event]);
        context.emit(row)?;
    }
    for row in records.references {
        native!(ReferenceSupport, row, row.read);
        context.emit(row)?;
    }
    for row in records.resolutions {
        native!(LexicalResolutionSupport, row, row.read);
        context.emit(row)?;
    }
    Ok(())
}
/// The names from outside a module's text, as Pyrefly defines them: its implicit globals, and the
/// real, public definitions of `builtins` (not the stub's implicit globals, private helpers or
/// imports), each with whether it is variable-like.
fn outside_names(
    transaction: &pyrefly::state::state::Transaction<'_>,
    anchor: Option<&pyrefly_build::handle::Handle>,
) -> Outside {
    use pyrefly::export::exports::ExportLocation;
    use pyrefly_python::module_name::ModuleName;
    use pyrefly_types::globals::ImplicitGlobal;
    let implicit_globals: Vec<String> = ImplicitGlobal::implicit_globals(false)
        .map(|g| g.name().to_string())
        .collect();
    let builtins = anchor
        .and_then(|h| {
            transaction
                .import_handle(h, ModuleName::from_str("builtins"), None)
                .finding()
        })
        .map(|h| {
            transaction
                .get_exports(&h)
                .iter()
                .filter_map(|(name, location)| {
                    let name = name.as_str();
                    match location {
                        ExportLocation::ThisModule(export)
                            if pyrefly::commands::coverage::collect::is_public_name(name)
                                && !implicit_globals.iter().any(|g| g == name) =>
                        {
                            use pyrefly_python::symbol_kind::SymbolKind as Kind;
                            Some((
                                name.to_owned(),
                                !matches!(
                                    export.symbol_kind,
                                    Some(Kind::Function | Kind::Class | Kind::Method)
                                ),
                            ))
                        }
                        ExportLocation::ThisModule(_) | ExportLocation::OtherModule(..) => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Outside {
        builtins,
        implicit_globals,
    }
}
/// Each `from m import *` of a module → the names Pyrefly's wildcard set for `m` holds, or `None`
/// when Pyrefly cannot find `m`. Star imports are module-level only.
fn star_imports(
    transaction: &pyrefly::state::state::Transaction<'_>,
    handle: &pyrefly_build::handle::Handle,
    module: &str,
    is_package: bool,
    ast: &ruff_python_ast_latest::ModModule,
) -> Stars {
    use ruff_python_ast_latest::statement_visitor::{StatementVisitor, walk_stmt};
    struct Found<'a>(Vec<&'a ruff_python_ast_latest::StmtImportFrom>);
    impl<'a> StatementVisitor<'a> for Found<'a> {
        fn visit_stmt(&mut self, stmt: &'a ruff_python_ast_latest::Stmt) {
            if let ruff_python_ast_latest::Stmt::ImportFrom(i) = stmt
                && i.names.iter().any(|a| a.name.as_str() == "*")
            {
                self.0.push(i);
            }
            walk_stmt(self, stmt);
        }
    }
    let mut found = Found(Vec::new());
    found.visit_body(&ast.body);
    let mut stars = Stars::new();
    for import in found.0 {
        let Some(star) = import.names.iter().find(|a| a.name.as_str() == "*") else {
            continue;
        };
        let names = syntax_records::absolute_module(
            module,
            is_package,
            i64::from(import.level),
            import.module.as_ref().map(|n| n.as_str()),
        )
        .and_then(|name| {
            transaction
                .import_handle(
                    handle,
                    pyrefly_python::module_name::ModuleName::from_str(&name),
                    None,
                )
                .finding()
        })
        .map(|h| {
            transaction
                .get_wildcard(&h)
                .iter()
                .map(ToString::to_string)
                .collect()
        });
        stars.insert(u32::from(ruff_text_size_latest::Ranged::start(star)), names);
    }
    stars
}

/// Paths under a captured root are written relative to it, so the configuration digest does not
/// depend on where the input was captured.
fn relativize(value: &mut serde_json::Value, root: &Path, label: &str) {
    match value {
        serde_json::Value::String(s) => {
            if let Ok(relative) = Path::new(s).strip_prefix(root) {
                *s = format!("{label}/{}", relative.display());
            } else if Path::new(s) == root {
                *s = label.to_owned();
            }
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .for_each(|item| relativize(item, root, label)),
        serde_json::Value::Object(items) => items
            .values_mut()
            .for_each(|item| relativize(item, root, label)),
        _ => {}
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
fn calls(
    transaction: &pyrefly::state::state::Transaction<'_>,
    handle: &pyrefly_build::handle::Handle,
    qualification: &AssertionQualification,
    provider: &Provider,
    locator: &symbol_records::Locator<'_>,
    spans: &Spans,
    syntax: &[(CallSyntax, Vec<CallArgument>)],
    natives: &mut Natives,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<crate::call_records::Records, ModelError> {
    use pyrefly::report::pysa::{
        captured_variable::collect_captured_variables_for_module,
        context::{ModuleAnswersContext, ModuleContext, PysaResolver},
        export_module_call_graphs,
    };
    let module_ids = &transaction
        .pysa_reporter()
        .ok_or_else(|| invalid("Pysa reporter absent".into()))?
        .module_ids;
    let current = module_ids.get_from_handle(handle);
    let resolver = PysaResolver::new(transaction, module_ids, handle.clone());
    let context = ModuleContext {
        answers_context: ModuleAnswersContext::create(handle.clone(), transaction, module_ids),
        resolver: &resolver,
    };
    let captured = collect_captured_variables_for_module(&context);
    let graphs = export_module_call_graphs(&context, &captured);
    let module = natives.module(&handle.module().to_string(), handle.path())?;
    let mut resolve =
        |natives: &mut Natives, function: &pyrefly::report::pysa::function::FunctionRef| {
            let found = if function.module_id == current {
                handle.clone()
            } else {
                transaction
                    .import_handle(handle, function.module_name, None)
                    .finding()
                    .ok_or_else(|| {
                        invalid(format!(
                            "native call target module {} did not resolve",
                            function.module_name
                        ))
                    })?
            };
            if module_ids.get_from_handle(&found) != function.module_id {
                return Err(invalid(
                    "native call target resolved to another module".into(),
                ));
            }
            let module = natives.module(&found.module().to_string(), found.path())?;
            let solutions = transaction.resolve_pysa_solutions(&found);
            let definition = solutions
                .function_base_definitions
                .get(&function.function_id)
                .ok_or_else(|| {
                    invalid(format!(
                        "native call target {} has no definition",
                        function.function_id.serialize_to_string()
                    ))
                })?;
            let kind = if definition.defining_class.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            natives.symbol(
                module,
                function.function_id.serialize_to_string(),
                definition.name.to_string(),
                kind,
            )
        };
    crate::call_records::records(
        &graphs,
        module,
        provider.id(),
        qualification,
        locator,
        spans,
        syntax,
        natives,
        &mut resolve,
        budget,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
fn write_calls<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    records: crate::call_records::Records,
    scope: &CoverageScope,
    provider: &Provider,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    surfaces: &BTreeMap<FactFamily, ProviderSurface>,
    evidence: &Evidence,
) -> Result<(), ModelError> {
    let surface = surfaces[&FactFamily::Calls].id();
    macro_rules! supported {
        ($ty:ident, $row:expr) => {{
            let row = $row;
            context.emit($ty {
                assertion: row.id(),
                run: run.id(),
                surface,
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })?;
            context.emit(row)?;
        }};
    }
    for row in records.callers {
        context.emit(row)?;
    }
    for row in records.origins {
        context.emit(row)?;
    }
    for row in records.steps {
        context.emit(row)?;
    }
    for row in records.sites {
        supported!(ProviderCallSiteSupport, row);
    }
    for rows in records.normalized {
        for row in rows.qualifications {
            context.contribute(row)?;
        }
        for row in rows.channels {
            context.emit(row)?;
        }
        for row in rows.destinations {
            context.emit(row)?;
        }
        for row in rows.receivers {
            context.emit(row)?;
        }
        for row in rows.targets {
            supported!(CallTargetSupport, row);
        }
        for row in rows.resolutions {
            supported!(CallResolutionSupport, row);
        }
        for row in rows.members {
            context.emit(row)?;
        }
    }
    for event in records.unattached {
        let boundary = SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Calls,
            subject: None,
            reason: event.reason,
            detail: Some(event.detail),
        };
        let CoverageScope::Artifact {
            artifact: source, ..
        } = scope
        else {
            return Err(invalid(
                "native call attachment needs an artifact scope".into(),
            ));
        };
        let outcome = AttachmentOutcome {
            boundary: boundary.id(),
            source: *source,
            start: i64::from(event.range.start().to_u32()),
            end: i64::from(event.range.end().to_u32()),
            syntax_kind: event.syntax_kind,
            role: None,
            outcome: if event.candidates.len() > 1 {
                AttachmentKind::Ambiguous
            } else {
                AttachmentKind::Unmatched
            },
        };
        for candidate in event.candidates {
            context.contribute(AttachmentCandidate {
                outcome: outcome.id(),
                candidate,
            })?;
        }
        context.contribute(outcome)?;
        context.contribute(boundary)?;
    }
    for (subject, reason, detail) in records.boundaries {
        context.contribute(SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Calls,
            subject,
            reason,
            detail: Some(detail),
        })?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
fn types(
    transaction: &pyrefly::state::state::Transaction<'_>,
    handle: &pyrefly_build::handle::Handle,
    qualification: &AssertionQualification,
    context_info: &AnalysisContext,
    provider: &Provider,
    syntax: &syntax_records::Records,
    spans: &Spans,
    natives: &mut Natives,
    budget: &lctx_model::domain::resources::ResourceBudget,
    roots: &[&SourceArtifact],
    limits: SyntaxLimits,
    run: &ProviderRun,
    surface: &ProviderSurface,
    root: &Path,
) -> Result<crate::type_records::Records, ModelError> {
    use pyrefly::report::pysa::context::{ModuleAnswersContext, ModuleContext, PysaResolver};
    let module_ids = &transaction
        .pysa_reporter()
        .ok_or_else(|| invalid("Pysa reporter absent".into()))?
        .module_ids;
    let resolver = PysaResolver::new(transaction, module_ids, handle.clone());
    let context = ModuleContext {
        answers_context: ModuleAnswersContext::create(handle.clone(), transaction, module_ids),
        resolver: &resolver,
    };
    let solutions = transaction.get_solutions(handle);
    let mut resolve =
        |natives: &mut Natives, name: pyrefly_python::module_name::ModuleName| match transaction
            .import_handle(handle, name, None)
            .finding()
        {
            Some(found) => natives.module(&found.module().to_string(), found.path()),
            None => natives.unresolved(&name.to_string()),
        };
    let mut module_context = |class: &pyrefly_types::class::Class| {
        let found = transaction
            .import_handle(handle, class.module().name(), None)
            .finding()?;
        if found.path() != class.module().path()
            || transaction.get_answers(&found).is_none()
        {
            return None;
        }
        Some(ModuleAnswersContext::create(found, transaction, module_ids))
    };
    let mut cached = BTreeMap::new();
    let mut cache_charge =
        lctx_model::domain::charged::StateCharge::new(budget, "inherited_field_spans");
    let mut field_occurrence = |class: &pyrefly_types::class::Class,
                                range: ruff_text_size::TextRange| {
        let found = transaction
            .import_handle(handle, class.module().name(), None)
            .finding();
        let Some(found) = found else {
            return Ok(None);
        };
        let Some(artifact) = roots
            .iter()
            .find(|a| found.path().as_path() == root.join(&a.path))
        else {
            return Ok(None);
        };
        if let std::collections::btree_map::Entry::Vacant(e) = cached.entry(artifact.id()) {
            let info = transaction
                .get_module_info(&found)
                .ok_or_else(|| invalid("inherited field text unavailable".into()))?;
            let canonical = crate::ruff_context::CanonicalSyntax::parse(artifact, info.lined_buffer().contents(), &context_info, crate::ruff_context::ContextSettings::default(), budget)?;
            let ast = canonical.module();
            let mut spans = Spans::new(budget);
            let invocation = SyntaxInvocation {
                source: artifact,
                qualification,
                run,
                surface,
            };
            let complete = match typed_syntax::emit(
                &ast,
                info.lined_buffer().contents(),
                invocation,
                limits,
                |event| spans.insert(&event.occurrence),
            ) {
                Ok(_) => true,
                Err(typed_syntax::SyntaxError::Refused { .. }) => false,
                Err(typed_syntax::SyntaxError::Model(error)) => return Err(error),
            };
            cache_charge.grow(256)?;
            e.insert(if complete { Some(spans) } else { None });
        }
        Ok(cached[&artifact.id()]
            .as_ref()
            .and_then(|s| s.event(range, Some(SyntaxKind::ExprName))))
    };
    crate::type_records::records(
        &context,
        qualification,
        provider,
        natives,
        &mut resolve,
        syntax,
        spans,
        solutions.as_deref(),
        budget,
        &mut module_context,
        &mut field_occurrence,
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
fn write_types<S: StageSink + 'static>(
    context: &mut StageContext<S>,
    records: crate::type_records::Records,
    scope: &CoverageScope,
    provider: &Provider,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    surfaces: &BTreeMap<FactFamily, ProviderSurface>,
    evidence: &Evidence,
) -> Result<(), ModelError> {
    let surface = surfaces[&FactFamily::Types].id();
    macro_rules! supported {
        ($ty:ident,$row:expr,$fidelity:expr) => {{
            let row = $row;
            context.emit($ty {
                assertion: row.id(),
                run: run.id(),
                surface,
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: $fidelity,
            })?;
            context.emit(row)?;
        }};
    }
    for row in records.record_options { context.emit(row)?; }
    for row in records.transforms { context.emit(row)?; }
    for row in records.transform_specifiers { context.emit(row)?; }
    for (row, fidelity) in records.class_members { supported!(ClassMemberSupport, row, fidelity); }
    for row in records.class_metadata { supported!(ClassMetadataSupport, row, Fidelity::NativeStructural); }
    for row in records.terms {
        context.emit(row)?;
    }
    for row in records.sequences {
        context.emit(row)?;
    }
    for row in records.members {
        context.emit(row)?;
    }
    for row in records.lists {
        context.emit(row)?;
    }
    for row in records.slots {
        context.emit(row)?;
    }
    for row in records.dict_lists {
        context.emit(row)?;
    }
    for row in records.dict_fields {
        context.emit(row)?;
    }
    for row in records.variables {
        context.emit(row)?;
    }
    for row in records.literals {
        context.contribute(row)?;
    }
    for (row, fidelity) in records.observations {
        supported!(TypeSupport, row, fidelity);
    }
    for (row, fidelity) in records.presentations {
        supported!(TypePresentationSupport, row, fidelity);
    }
    for (row, fidelity) in records.restrictions {
        supported!(TypeRestrictionSupport, row, fidelity);
    }
    for row in records.bodies {
        supported!(FunctionBodySupport, row, Fidelity::NativeStructural);
    }
    for (row, fidelity) in records.fields {
        supported!(RecordFieldSupport, row, fidelity);
    }
    for (subject, reason, detail) in records.boundaries {
        context.contribute(SubjectBoundary {
            scope: scope.id(),
            provider: provider.id(),
            context: analysis.id(),
            family: FactFamily::Types,
            subject,
            reason,
            detail: Some(detail),
        })?;
    }
    Ok(())
}
