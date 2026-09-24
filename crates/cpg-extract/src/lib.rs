//! Pyrefly + Ruff in-process extraction of the raw fact families (DESIGN §4.2, ADR-0012).
//!
//! One run: the constructed configuration, one `State` at `ThreadCount::Inline` on a driver-owned
//! thread, `run` at `Require::Everything` with a no-write Pysa reporter, then per module the Ruff
//! walk over Pyrefly's own parse and Pyrefly's own Pysa collectors. Any panic aborts the whole
//! extraction (§4.2.5): there is no per-module recovery.

mod config;
mod context;
mod docs;
mod facts;
mod flow;
mod lexical;
pub mod library;
pub mod logging;
mod public;
mod pysa_map;
mod syntax;
mod types;
mod walk;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use cpg_schema::codebook::ExtractionMode;
use cpg_schema::codebook::{
    BoundaryReason, CoverageStatus, FactFamily, Fidelity, Modality, Origin, ScopeKind, SourceRole,
};
use cpg_schema::id::{Id, IdHasher, content_digest, kind};
use cpg_schema::metrics::{Stage, Stages};
use cpg_schema::table::Table;
use cpg_schema::tables::{
    Arguments, Bindings, Boundaries, BoundariesRow, CallSyntax, ClassAncestry, CodeBlocks,
    ConditionLiterals, Conditions, ContextDefinitions, ContextModules, Contexts, ContextsRow,
    Coverage, CoverageRow, Declarations, Distributions, DistributionsRow, DocComponentAttributes,
    DocComponents, DocLinks, Documents, ExportSyntax, Facts, FlowDefinitions, FlowReaching,
    FlowAttributeLoads, FlowRegions, FlowTests, FlowUses, FlowValues, Mentions, ParameterDocs, ParameterSemantics,
    ParameterSyntax, Passages, Producers, ProducersRow, PublicNames, PysaCalls, PysaClasses,
    PysaFunctions, RecordFields, ReferenceResolutions, References, Releases, ReleasesRow, Runs,
    RunsRow, Scopes, SourceFiles, SourceFilesRow, SyntaxNodes, TypeObservations, TypeTermArgs,
    TypeTerms,
};
use pyrefly::commands::coverage::collect::is_public_name;
use pyrefly::export::exports::ExportLocation;
use pyrefly::report::pysa::captured_variable::collect_captured_variables_for_module;
use pyrefly::report::pysa::context::{ModuleAnswersContext, ModuleContext, PysaResolver};
use pyrefly::report::pysa::module::ModuleIds;
use pyrefly::report::pysa::override_graph::create_reversed_override_graph_for_module;
use pyrefly::report::pysa::{
    PysaFormat, PysaReporter, export_module_call_graphs, export_module_definitions,
};
use pyrefly::state::require::Require;
use pyrefly::state::state::{State, Transaction};
use pyrefly_build::handle::Handle;
use pyrefly_config::error_kind::ErrorKind;
use pyrefly_config::finder::ConfigFinder;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::module_path::ModulePath;
use pyrefly_types::globals::ImplicitGlobal;
use pyrefly_util::arc_id::ArcId;
use pyrefly_util::thread_pool::ThreadCount;
use ruff_python_ast::ModModule;
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_text_size::Ranged;
use serde_json::Value;

pub use config::{
    CorpusInput, DRIVER_STACK_BYTES, ExtractInput, PYREFLY_PATCH_SHA256, PYREFLY_REV, REFUSED_ENV,
    REFUSED_ENV_PREFIX, Release, ReleaseOrigin, TOOL, TestHooks,
};
use facts::{FactSink, Provenance, Surface, dedup_by_fact, fact_row};
use pysa_map::{Here, Locator, ModuleRefs, PysaOut};
use walk::{ModuleCtx, span};

/// The fact families this producer declares (coverage rows exist for each, per module).
pub const FAMILIES: [FactFamily; 7] = [
    FactFamily::Exports,
    FactFamily::Signatures,
    FactFamily::Calls,
    FactFamily::Syntax,
    FactFamily::Lexical,
    FactFamily::Types,
    // The flow IR's facts (ADR-0022 §The flow provider): release modules only.
    FactFamily::Flow,
];

/// The families a corpus run declares (C5): its documents, and every code family but `exports` for
/// its examples, tests and materialized code blocks (the usage run, C5b).
pub const CORPUS_FAMILIES: [FactFamily; 6] = [
    FactFamily::Signatures,
    FactFamily::Calls,
    FactFamily::Syntax,
    FactFamily::Lexical,
    FactFamily::Types,
    FactFamily::Docs,
];

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("refusing to run: ambient variable {0} is set")]
    Ambient(String),
    #[error("path must be absolute: {}", .0.display())]
    RelativePath(PathBuf),
    #[error("pyrefly configuration has {0} errors")]
    Config(usize),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("arrow: {0}")]
    Arrow(#[from] ArrowError),
    #[error("public names disagree with compute_public_fqns: {0}")]
    PublicMismatch(String),
    #[error("fact {0} was asserted twice with different provenance")]
    ProvenanceConflict(String),
    #[error("library: {0}")]
    Library(String),
    #[error("dependency context: {0}")]
    Context(String),
    #[error("extraction panicked; the attempt is aborted")]
    Panicked,
}

/// Sorted raw-family batches for one run, plus its identities.
#[derive(Debug)]
pub struct ExtractOutput {
    pub run_id: Id,
    pub context_id: Id,
    pub producer_id: Id,
    /// `(table name, canonically sorted batch)` for every table the producer writes.
    pub tables: Vec<(&'static str, RecordBatch)>,
    /// Per module name: the in-memory Pysa structs as JSON (`module_id` removed), when
    /// requested. A `.py`/`.pyi` pair shares a name, so the harness fixtures have none.
    pub pysa_json: BTreeMap<String, Value>,
    /// Wall time and peak RSS per extraction stage (DESIGN §4.3); not content.
    pub stages: Vec<Stage>,
}

impl ExtractOutput {
    pub fn table(&self, name: &str) -> Option<&RecordBatch> {
        self.tables.iter().find(|(n, _)| *n == name).map(|(_, b)| b)
    }
}

/// Run one extraction. Refuses ambient knobs and relative paths; runs on a driver-owned thread
/// with a declared stack; any panic aborts the extraction.
pub fn extract(input: &ExtractInput) -> Result<ExtractOutput, ExtractError> {
    config::refuse_ambient()?;
    config::require_absolute(input)?;
    let input = input.clone();
    std::thread::Builder::new()
        .name("lctx-extract".to_owned())
        .stack_size(DRIVER_STACK_BYTES)
        .spawn(move || run(&input))?
        .join()
        .map_err(|_| ExtractError::Panicked)?
}

struct SourceModule {
    handle: Handle,
    name: String,
    path: String,
    node_id: Id,
    bytes: Vec<u8>,
}

fn strip_module_ids(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("module_id");
            m.values_mut().for_each(strip_module_ids);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_module_ids),
        _ => {}
    }
}

struct Report {
    coverage: Vec<CoverageRow>,
    boundaries: Vec<BoundariesRow>,
}

impl Report {
    fn cover(
        &mut self,
        sink: &FactSink,
        module: Id,
        family: FactFamily,
        status: CoverageStatus,
        reason: Option<BoundaryReason>,
        detail: Option<String>,
    ) {
        self.cover_scope(
            sink,
            ScopeKind::Module,
            module,
            family,
            status,
            reason,
            detail,
        );
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a coverage row has this many facets"
    )]
    fn cover_scope(
        &mut self,
        sink: &FactSink,
        scope_kind: ScopeKind,
        scope: Id,
        family: FactFamily,
        status: CoverageStatus,
        reason: Option<BoundaryReason>,
        detail: Option<String>,
    ) {
        self.coverage.push(CoverageRow {
            snapshot_id: sink.snapshot_id,
            run_id: sink.run_id,
            scope_kind,
            scope_node_id: scope,
            fact_family: family,
            status,
            reason,
            detail,
        });
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a boundary row has this many facets"
    )]
    fn boundary(
        &mut self,
        sink: &mut FactSink,
        module: Id,
        subject: Option<Id>,
        family: FactFamily,
        reason: BoundaryReason,
        span: Option<(i64, i64)>,
        detail: Option<String>,
    ) {
        // A boundary compares two surfaces (a call Ruff sees and Pysa does not, an `__all__`
        // Pyrefly cannot read): the extractor's own relational derivation (slice-1 review O4).
        let provenance = Provenance {
            surface: Surface::Compare,
            mode: ExtractionMode::RelationalDerivation,
            origin: Origin::DerivedAnalysis,
            modality: Modality::Definite,
            fidelity: Fidelity::NativeStructural,
        };
        self.boundaries.push(fact_row!(
            sink,
            Boundaries,
            provenance,
            BoundariesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: module,
                subject_node_id: subject,
                fact_family: family,
                reason,
                start_byte: span.map(|s| s.0),
                end_byte: span.map(|s| s.1),
                detail,
            }
        ));
    }
}

/// The library run, then the corpus run when one is declared (C5), merged into one attempt.
fn run(input: &ExtractInput) -> Result<ExtractOutput, ExtractError> {
    let (library, vocabulary) = run_release(input, &FAMILIES, None)?;
    let Some(corpus) = &input.corpus else {
        return Ok(library);
    };
    let corpus_input = ExtractInput {
        release: corpus.release.clone(),
        corpus: None,
        ..input.clone()
    };
    let (corpus_out, _) = run_release(
        &corpus_input,
        &CORPUS_FAMILIES,
        Some((&corpus.documents, &vocabulary)),
    )?;
    merge(library, corpus_out)
}

/// Two runs' tables as one attempt's: every table concatenated and sorted by its key. A context
/// or producer both runs share is written once.
fn merge(a: ExtractOutput, b: ExtractOutput) -> Result<ExtractOutput, ExtractError> {
    macro_rules! keys {
        ($($t:ty),+) => { vec![$((<$t as Table>::NAME, <$t as Table>::key())),+] };
    }
    let keys: HashMap<&str, &[&str]> = cpg_schema::for_each_table!(keys).into_iter().collect();
    let shared = |name: &str| {
        (name == Producers::NAME && a.producer_id == b.producer_id)
            || ((name == Contexts::NAME || name == Distributions::NAME)
                && a.context_id == b.context_id)
    };
    let mut tables = Vec::new();
    for (name, batch) in &a.tables {
        let other = b.tables.iter().find(|(n, _)| n == name).map(|(_, x)| x);
        let merged = match other {
            Some(other) if !shared(name) && other.num_rows() > 0 => {
                let both = arrow_select::concat::concat_batches(&batch.schema(), [batch, other])?;
                cpg_schema::table::canonical_sort(&both, keys[name])?
            }
            Some(_) | None => batch.clone(),
        };
        tables.push((*name, merged));
    }
    let mut stages = a.stages;
    stages.extend(b.stages.into_iter().map(|mut s| {
        s.name = format!("corpus {}", s.name);
        s
    }));
    Ok(ExtractOutput {
        run_id: a.run_id,
        context_id: a.context_id,
        producer_id: a.producer_id,
        tables,
        pysa_json: a.pysa_json,
        stages,
    })
}

/// One extractor run over one release: the declared `families`, and for a corpus its documents
/// with the library's vocabulary. Returns the run's tables and the vocabulary its public names
/// and declarations make.
fn run_release(
    input: &ExtractInput,
    families: &[FactFamily],
    documents: Option<(&[PathBuf], &docs::Vocabulary)>,
) -> Result<(ExtractOutput, docs::Vocabulary), ExtractError> {
    let mut stages = Stages::new();
    let cfg = config::pyrefly_config(input)?;
    let context = config::context(&cfg, input)?;
    let producer = config::producer();
    let family_names: Vec<&str> = families
        .iter()
        .map(|f| cpg_schema::Codebook::text(*f))
        .collect();
    // The code families, whose unit is a module (the `docs` family's is a document).
    let code_families: Vec<FactFamily> = families
        .iter()
        .copied()
        .filter(|f| *f != FactFamily::Docs)
        .collect();
    let run_id = config::run_id(
        input.release.release_id,
        context.id,
        &producer,
        &family_names,
    );
    let mut sink = FactSink::new(run_id, input.snapshot_id, producer.id);

    // Modules of the release, sorted by name.
    let mut modules = Vec::new();
    for file in &input.release.files {
        let rel = file
            .strip_prefix(&input.release.root)
            .map_err(|_| ExtractError::RelativePath(file.clone()))?
            .display()
            .to_string();
        let handle = cfg.handle_from_module_path(ModulePath::filesystem(file.clone()));
        let node_id = IdHasher::new(kind::MODULE)
            .id(input.release.release_id)
            .str(&rel)
            .finish_id();
        modules.push(SourceModule {
            name: handle.module().to_string(),
            handle,
            path: rel,
            node_id,
            bytes: fs_err::read(file)?,
        });
    }
    modules.sort_by(|a, b| (&a.name, &a.path).cmp(&(&b.name, &b.path)));
    if input.test_hooks.reverse_module_order {
        modules.reverse();
    }
    let handles: Vec<Handle> = modules.iter().map(|m| m.handle.clone()).collect();
    // A corpus reaches the library through its installed files: those the release's
    // distributions' `RECORD`s own. They are the library run's own modules, so the corpus names
    // them as the library run does, by their site-relative `@path` (C5 review F2): one release
    // class is one type term, and a usage call's target is the release's own node.
    let library_files: Vec<(Handle, String)> = match &input.release.origin {
        ReleaseOrigin::Corpus {
            library: Some(l), ..
        } => {
            let release: std::collections::BTreeSet<String> = l
                .release
                .iter()
                .map(|r| library::normalize(r.split("==").next().unwrap_or(r)))
                .collect();
            l.owners
                .iter()
                .filter(|(p, d)| {
                    release.contains(d.as_str()) && (p.ends_with(".py") || p.ends_with(".pyi"))
                })
                .filter_map(|(p, _)| {
                    let file = input
                        .site_packages
                        .iter()
                        .map(|s| s.join(p))
                        .find(|f| f.is_file())?;
                    Some((
                        cfg.handle_from_module_path(ModulePath::filesystem(file)),
                        p.clone(),
                    ))
                })
                .collect()
        }
        ReleaseOrigin::Corpus { library: None, .. }
        | ReleaseOrigin::Tree { .. }
        | ReleaseOrigin::Library(_) => Vec::new(),
    };
    let mut id_handles = handles.clone();
    id_handles.extend(library_files.iter().map(|(h, _)| h.clone()));

    // One check at `Everything`, single-threaded, with the no-write Pysa reporter installed.
    let finder = ConfigFinder::new_constant(ArcId::new(cfg));
    let state = State::new(finder, ThreadCount::Inline);
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.set_pysa_reporter(Some(Box::new(PysaReporter {
        module_ids: ModuleIds::new(&id_handles),
        pysa_directory: PathBuf::new(),
        definitions_directory: PathBuf::new(),
        type_of_expressions_directory: PathBuf::new(),
        call_graphs_directory: PathBuf::new(),
        format: PysaFormat::Json,
        write_files: false,
    })));
    txn.run(&handles, Require::Everything, None);
    stages.mark("extract: pyrefly check (release)");
    // The corpus must reach those files and not a copy of the package in the tree, which its
    // search path puts first (a flat layout): that would silently cut every usage off the release
    // (C5 review F3), so it fails the compile.
    if let Some(anchor) = handles.first() {
        let installed: std::collections::HashSet<&Handle> =
            library_files.iter().map(|(h, _)| h).collect();
        for (h, _) in &library_files {
            if let Some(found) = txn.import_handle(anchor, h.module(), None).finding()
                && !installed.contains(&found)
            {
                return Err(ExtractError::Context(format!(
                    "the corpus tree shadows the release: `{}` resolves to {}, not the installed {}",
                    h.module(),
                    found.path(),
                    h.path()
                )));
            }
        }
    }
    // The names from outside a module's text, as Pyrefly defines them (C3; review F1): a
    // module's implicit globals, and the builtins, which are the real, public definitions of
    // `builtins` (not the stub's implicit globals, private helpers or imports), with their kinds.
    let builtins_handle = handles.first().and_then(|h| {
        txn.import_handle(h, ModuleName::from_str("builtins"), None)
            .finding()
    });
    let implicit_globals: Vec<String> = ImplicitGlobal::implicit_globals(false)
        .map(|g| g.name().to_string())
        .collect();
    let builtins = builtins_handle
        .as_ref()
        .map(|h| {
            txn.get_exports(h)
                .iter()
                .filter_map(|(name, loc)| {
                    let n = name.as_str();
                    // Pyrefly's definition of public (§4.2.3; H1 C9), not a second one.
                    match loc {
                        ExportLocation::ThisModule(e)
                            if is_public_name(n) && !implicit_globals.iter().any(|g| g == n) =>
                        {
                            Some((n.to_owned(), e.symbol_kind.map(public::symbol_kind)))
                        }
                        ExportLocation::ThisModule(_) | ExportLocation::OtherModule(..) => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let outside = lexical::Outside {
        builtins,
        implicit_globals,
    };
    let mut builtins_used: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut lexical_out = lexical::LexicalOut::default();
    let (mut walk_time, mut pysa_time, mut types_time) =
        (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut types_out = types::TypesOut::default();
    let txn = txn;
    // The reporter stays installed: dependency modules solve lazily during extraction.
    let module_ids = &txn
        .pysa_reporter()
        .expect("the reporter was installed before run")
        .module_ids;
    // Project handles hold ids pre-assigned in sorted order; only dependencies are numbered lazily.
    let refs = ModuleRefs::new(
        modules
            .iter()
            .map(|m| (module_ids.get_from_handle(&m.handle), m.path.clone()))
            .chain(
                library_files
                    .iter()
                    .map(|(h, p)| (module_ids.get_from_handle(h), p.clone())),
            )
            .collect(),
    );

    let mut source_files = Vec::new();
    let mut walked = walk::WalkOut::default();
    let mut pysa = PysaOut::default();
    let mut report = Report {
        coverage: Vec::new(),
        boundaries: Vec::new(),
    };
    let mut pysa_json = BTreeMap::new();
    let mut flow_modules: Vec<flow::FlowModule> = Vec::new();
    let source = Provenance {
        surface: Surface::Source,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::InputContext,
        modality: Modality::Definite,
        fidelity: Fidelity::Raw,
    };

    for m in &modules {
        let text = std::str::from_utf8(&m.bytes).ok();
        let utf8 = text.is_some();
        source_files.push(fact_row!(
            sink,
            SourceFiles,
            Provenance { ..source },
            SourceFilesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: m.node_id,
                release_id: input.release.release_id,
                module_name: m.name.clone(),
                path: m.path.clone(),
                // Pyrefly's own package test (H1 C9), not a path suffix.
                is_package: m.handle.path().is_init(),
                is_stub: m.path.ends_with(".pyi"),
                content_digest: content_digest(&m.bytes),
                byte_len: m.bytes.len() as i64,
                utf8,
                distribution: match &input.release.origin {
                    ReleaseOrigin::Library(l) => l.owners.get(&m.path).cloned(),
                    ReleaseOrigin::Tree { .. } | ReleaseOrigin::Corpus { .. } => None,
                },
                role: match &input.release.origin {
                    ReleaseOrigin::Corpus { roles, .. } =>
                        *roles.get(&m.path).ok_or_else(|| {
                            ExtractError::Context(format!("the corpus has no role for {}", m.path))
                        })?,
                    ReleaseOrigin::Library(_) | ReleaseOrigin::Tree { .. } => SourceRole::Release,
                },
                text: text.map(str::to_owned),
            }
        ));
        if !utf8 {
            // Pyrefly would load it as an empty module, which must not read as "no API".
            for &family in &code_families {
                report.cover(
                    &sink,
                    m.node_id,
                    family,
                    CoverageStatus::Unavailable,
                    Some(BoundaryReason::UndecodableSource),
                    Some("source is not UTF-8".to_owned()),
                );
            }
            report.boundary(
                &mut sink,
                m.node_id,
                None,
                FactFamily::Exports,
                BoundaryReason::UndecodableSource,
                None,
                Some("source is not UTF-8".to_owned()),
            );
            continue;
        }
        if input.test_hooks.fault_at_module.as_deref() == Some(m.name.as_str()) {
            panic!("fault hook: injected panic in module {}", m.name);
        }

        let info = txn
            .get_module_info(&m.handle)
            .expect("loaded at Require::Everything");
        let ast = txn.get_ast(&m.handle).expect("kept at Require::Everything");
        let text = info.lined_buffer().contents().clone();
        if families.contains(&FactFamily::Flow) {
            flow_modules.push(flow::FlowModule {
                node_id: m.node_id,
                path: m.path.clone(),
                text: text.to_string(),
            });
        }
        let ctx = ModuleCtx {
            release_id: input.release.release_id,
            path: &m.path,
            module_name: &m.name,
            module_node_id: m.node_id,
            text: &text,
            sys_info: m.handle.sys_info(),
            is_package: m.handle.path().is_init(),
        };
        let clock = Instant::now();

        let resolver = PysaResolver::new(&txn, module_ids, m.handle.clone());
        let context = ModuleContext {
            answers_context: ModuleAnswersContext::create(m.handle.clone(), &txn, module_ids),
            resolver: &resolver,
        };
        let captured = collect_captured_variables_for_module(&context);
        let overrides = create_reversed_override_graph_for_module(&context);
        let defs = export_module_definitions(&context, &captured, &overrides);
        let graphs = export_module_call_graphs(&context, &captured);
        let here = Here {
            module_name: &m.name,
            module_node_id: m.node_id,
            refs: &refs,
            loc: Locator {
                line_index: info.lined_buffer().line_index(),
                text: &text,
            },
        };
        let mut module_pysa = PysaOut::default();
        pysa_map::map_definitions(&here, &defs, &mut sink, &mut module_pysa);
        pysa_map::map_call_graphs(&here, &graphs, &mut sink, &mut module_pysa);
        pysa_time += clock.elapsed();
        let clock = Instant::now();
        let stars = star_imports(&txn, m, &ast);
        let mut module_walk = walk::walk_module(&ctx, &ast, &mut sink, &outside, &stars);
        let lex = std::mem::take(&mut module_walk.lexical);
        builtins_used.extend(lex.builtins_used);
        lexical_out.scopes.extend(lex.scopes);
        lexical_out.bindings.extend(lex.bindings);
        lexical_out.references.extend(lex.references);
        lexical_out.resolutions.extend(lex.resolutions);
        walk_time += clock.elapsed();
        let clock = Instant::now();
        let solutions = txn.get_solutions(&m.handle);
        let type_misses = types::module_types(
            &types::ModuleTypes {
                context: &context,
                solutions: solutions.as_deref(),
                refs: &refs,
                module_node_id: m.node_id,
                walk: &module_walk,
            },
            &mut sink,
            &mut types_out,
        );
        types_time += clock.elapsed();
        if input.keep_pysa_json {
            let mut d = serde_json::to_value(&defs).unwrap_or(Value::Null);
            let mut g = serde_json::to_value(&graphs).unwrap_or(Value::Null);
            strip_module_ids(&mut d);
            strip_module_ids(&mut g);
            pysa_json.insert(
                m.name.clone(),
                serde_json::json!({"definitions": d, "call_graphs": g}),
            );
        }

        // Coverage: parse errors, unmatched calls, and the `__all__` completeness detector. A
        // partial family keeps its first cause as the coverage reason; a syntax error outranks
        // the rest.
        let mut partial: HashMap<FactFamily, BoundaryReason> = HashMap::new();
        let errors = txn.get_errors([&m.handle]).collect_errors();
        let parse_errors: Vec<_> = [
            &errors.ordinary,
            &errors.directives,
            &errors.suppressed,
            &errors.disabled,
            &errors.baseline,
        ]
        .into_iter()
        .flatten()
        .filter(|e| e.error_kind() == ErrorKind::ParseError)
        .collect();
        if !parse_errors.is_empty() {
            partial.extend(
                code_families
                    .iter()
                    .map(|&f| (f, BoundaryReason::SyntaxError)),
            );
            for e in &parse_errors {
                report.boundary(
                    &mut sink,
                    m.node_id,
                    None,
                    FactFamily::Exports,
                    BoundaryReason::SyntaxError,
                    Some(span(e.range())),
                    Some(e.msg_header().to_owned()),
                );
            }
        }
        for miss in type_misses {
            partial.entry(FactFamily::Types).or_insert(miss.reason);
            report.boundary(
                &mut sink,
                m.node_id,
                miss.subject,
                FactFamily::Types,
                miss.reason,
                Some(miss.span),
                Some(miss.detail),
            );
        }
        for call in &module_walk.call_syntax {
            if module_pysa
                .regular_call_ranges
                .contains(&(call.start_byte, call.end_byte))
            {
                continue;
            }
            let reason = if call.in_annotation {
                BoundaryReason::OutsideProviderModel
            } else {
                partial
                    .entry(FactFamily::Calls)
                    .or_insert(BoundaryReason::MissingEvidence);
                BoundaryReason::MissingEvidence
            };
            report.boundary(
                &mut sink,
                m.node_id,
                Some(call.node_id),
                FactFamily::Calls,
                reason,
                Some((call.start_byte, call.end_byte)),
                Some(if call.in_annotation {
                    "call inside an annotation".to_owned()
                } else {
                    "no Pysa record for this call".to_owned()
                }),
            );
        }
        let mut unresolvable_all: Vec<(i64, i64)> = module_walk
            .nonliteral_dunder_all
            .iter()
            .map(|r| span(*r))
            .collect();
        // Pyrefly's own signal, unless the walker already flagged the enclosing statement.
        if let Some(r) = txn
            .get_exports_data(&m.handle)
            .unresolvable_dunder_all_range()
        {
            let (start, end) = span(r);
            if !unresolvable_all
                .iter()
                .any(|(s, e)| *s <= start && end <= *e)
            {
                unresolvable_all.push((start, end));
            }
        }
        unresolvable_all.sort_unstable();
        unresolvable_all.dedup();
        for s in unresolvable_all {
            partial
                .entry(FactFamily::Exports)
                .or_insert(BoundaryReason::OutsideProviderModel);
            report.boundary(
                &mut sink,
                m.node_id,
                None,
                FactFamily::Exports,
                BoundaryReason::OutsideProviderModel,
                Some(s),
                Some("__all__ is not a literal list or tuple of strings".to_owned()),
            );
        }
        for (function, name, docstring) in &module_walk.unlocated_parameter_docs {
            partial
                .entry(FactFamily::Signatures)
                .or_insert(BoundaryReason::ProviderDisagreement);
            report.boundary(
                &mut sink,
                m.node_id,
                Some(*function),
                FactFamily::Signatures,
                BoundaryReason::ProviderDisagreement,
                Some(*docstring),
                Some(format!(
                    "the description of parameter `{name}` is not located in the docstring's bytes"
                )),
            );
        }
        // The flow family's coverage comes from its own indexing, below.
        for &family in code_families.iter().filter(|f| **f != FactFamily::Flow) {
            let (status, reason) = match partial.get(&family) {
                Some(reason) => (CoverageStatus::Partial, Some(*reason)),
                None => (CoverageStatus::CompleteUnderStatedModel, None),
            };
            report.cover(&sink, m.node_id, family, status, reason, None);
        }

        walked.declarations.extend(module_walk.declarations);
        walked.export_syntax.extend(module_walk.export_syntax);
        walked.parameter_syntax.extend(module_walk.parameter_syntax);
        walked.parameter_docs.extend(module_walk.parameter_docs);
        walked.call_syntax.extend(module_walk.call_syntax);
        walked.arguments.extend(module_walk.arguments);
        walked.syntax_nodes.extend(module_walk.syntax_nodes);
        pysa.functions.extend(module_pysa.functions);
        pysa.parameters.extend(module_pysa.parameters);
        pysa.ancestry.extend(module_pysa.ancestry);
        pysa.classes.extend(module_pysa.classes);
        pysa.calls.extend(module_pysa.calls);
    }

    stages.mark("extract: per-module extraction");
    stages.push("extract:   of which the ruff walk", walk_time);
    stages.push("extract:   of which the pysa collectors", pysa_time);
    stages.push("extract:   of which the types", types_time);
    // The flow IR over the release (ADR-0022 §The flow provider): one ty database, every module.
    let mut flow_out = flow::FlowOut::default();
    if families.contains(&FactFamily::Flow) {
        let (major, minor, micro) = input.python_version;
        flow_out = flow::run(
            &mut sink,
            &flow_modules,
            &cpg_flow::RuntimeContext {
                python_version: (major, minor, micro),
                platform: input.python_platform.clone(),
            },
        );
        for m in &flow_modules {
            let (status, reason, detail) = match (
                flow_out.errors.get(&m.node_id),
                flow_out.recovered.get(&m.node_id),
            ) {
                (Some(e), _) => (
                    CoverageStatus::Failed,
                    Some(BoundaryReason::OutsideProviderModel),
                    Some(e.clone()),
                ),
                (None, Some(_)) => (
                    CoverageStatus::Partial,
                    Some(BoundaryReason::SyntaxError),
                    None,
                ),
                (None, None) => (CoverageStatus::CompleteUnderStatedModel, None, None),
            };
            report.cover(&sink, m.node_id, FactFamily::Flow, status, reason, detail);
        }
        stages.mark("extract: flow (ty)");
    }
    let readable: Vec<Handle> = modules
        .iter()
        .filter(|m| std::str::from_utf8(&m.bytes).is_ok())
        .map(|m| m.handle.clone())
        .collect();
    let release_files: HashMap<Handle, Id> = modules
        .iter()
        .map(|m| (m.handle.clone(), m.node_id))
        .collect();
    let (mut public, mut export_origins) = if families.contains(&FactFamily::Exports) {
        public::public_names(&readable, &release_files, &txn, &mut sink)?
    } else {
        (Vec::new(), Vec::new())
    };
    // Every module an import names that is not the release's (C3's `imports_module`).
    let release_modules: std::collections::BTreeSet<&str> =
        modules.iter().map(|m| m.name.as_str()).collect();
    // The library's modules a corpus imports are the library run's release modules, not context.
    let library_modules: std::collections::BTreeSet<String> = library_files
        .iter()
        .map(|(h, _)| h.module().to_string())
        .collect();
    let imported: std::collections::BTreeSet<String> = walked
        .export_syntax
        .iter()
        .filter_map(|r| r.resolved_module.clone())
        .filter(|m| !release_modules.contains(m.as_str()) && !library_modules.contains(m))
        .collect();
    // The builtin functions and classes names resolve to are described like re-exports (C3).
    if let Some(h) = &builtins_handle {
        export_origins.extend(builtins_used.iter().map(|n| (h.clone(), n.clone())));
    }

    // The dependency context the facts reference (ADR-0014): a second check, over those modules.
    stages.mark("extract: public names");
    let mut txn = txn;
    let mut context_out = context::context_facts(
        &mut txn,
        handles.first(),
        &refs,
        &export_origins,
        &imported,
        input,
        &mut sink,
        &mut stages,
    )?;

    // A corpus's documents (C5), recognized against the library's vocabulary.
    let mut docs_out = docs::DocsOut::default();
    if let Some((documents, vocabulary)) = documents {
        for path in documents {
            let rel = path
                .strip_prefix(&input.release.root)
                .map_err(|_| ExtractError::RelativePath(path.clone()))?
                .display()
                .to_string();
            let bytes = fs_err::read(path)?;
            let c = docs::document(
                &mut sink,
                input.release.release_id,
                &rel,
                &bytes,
                vocabulary,
                &mut docs_out,
            );
            report.cover_scope(
                &sink,
                ScopeKind::Document,
                c.document,
                FactFamily::Docs,
                c.status,
                c.reason,
                c.detail,
            );
        }
        stages.mark("extract: documents");
    }
    let vocabulary = docs::Vocabulary::new(&public, &walked.declarations);

    let snapshot_id = input.snapshot_id;
    let runs = vec![RunsRow {
        snapshot_id,
        run_id,
        release_id: input.release.release_id,
        context_id: context.id,
        producer_id: producer.id,
        families: {
            let mut f: Vec<String> = family_names.iter().map(|s| (*s).to_owned()).collect();
            f.sort();
            f
        },
        config_digest: producer.config_digest,
    }];
    let contexts = vec![ContextsRow {
        snapshot_id,
        context_id: context.id,
        python_version: context.python_version.clone(),
        python_platform: context.python_platform.clone(),
        search_path: context.search_path.clone(),
        site_package_path: context.site_package_path.clone(),
        config_digest: context.config_digest,
        environment_digest: context.environment_digest,
        lock_digest: context.lock_digest,
    }];
    let (releases, distributions) = release_rows(input, snapshot_id, context.id);
    let producers = vec![ProducersRow {
        snapshot_id,
        producer_id: producer.id,
        tool: TOOL.to_owned(),
        revision: producer.revision.clone(),
        build_digest: producer.build_digest,
    }];

    dedup_by_fact(&mut source_files, |r| r.fact_id);
    dedup_by_fact(&mut walked.declarations, |r| r.fact_id);
    dedup_by_fact(&mut walked.export_syntax, |r| r.fact_id);
    dedup_by_fact(&mut walked.parameter_syntax, |r| r.fact_id);
    dedup_by_fact(&mut walked.parameter_docs, |r| r.fact_id);
    dedup_by_fact(&mut walked.call_syntax, |r| r.fact_id);
    dedup_by_fact(&mut walked.arguments, |r| r.fact_id);
    dedup_by_fact(&mut walked.syntax_nodes, |r| r.fact_id);
    dedup_by_fact(&mut lexical_out.scopes, |r| r.fact_id);
    dedup_by_fact(&mut lexical_out.bindings, |r| r.fact_id);
    dedup_by_fact(&mut lexical_out.references, |r| r.fact_id);
    dedup_by_fact(&mut lexical_out.resolutions, |r| r.fact_id);
    dedup_by_fact(&mut types_out.terms, |r| r.fact_id);
    dedup_by_fact(&mut types_out.args, |r| r.fact_id);
    dedup_by_fact(&mut types_out.observations, |r| r.fact_id);
    dedup_by_fact(&mut types_out.fields, |r| r.fact_id);
    dedup_by_fact(&mut pysa.functions, |r| r.fact_id);
    dedup_by_fact(&mut pysa.parameters, |r| r.fact_id);
    dedup_by_fact(&mut pysa.ancestry, |r| r.fact_id);
    dedup_by_fact(&mut pysa.classes, |r| r.fact_id);
    dedup_by_fact(&mut context_out.modules, |r| r.fact_id);
    dedup_by_fact(&mut context_out.definitions, |r| r.fact_id);
    dedup_by_fact(&mut pysa.calls, |r| r.fact_id);
    dedup_by_fact(&mut public, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.documents, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.passages, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.code_blocks, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.links, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.mentions, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.components, |r| r.fact_id);
    dedup_by_fact(&mut docs_out.component_attributes, |r| r.fact_id);
    dedup_by_fact(&mut report.boundaries, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.uses, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.definitions, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.reaching, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.values, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.regions, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.conditions, |r| r.fact_id);
    dedup_by_fact(&mut flow_out.literals, |r| r.fact_id);

    let tables = vec![
        (Runs::NAME, Runs::to_sorted_batch(&runs)?),
        (Contexts::NAME, Contexts::to_sorted_batch(&contexts)?),
        (Producers::NAME, Producers::to_sorted_batch(&producers)?),
        (Releases::NAME, Releases::to_sorted_batch(&releases)?),
        (
            Distributions::NAME,
            Distributions::to_sorted_batch(&distributions)?,
        ),
        (
            SourceFiles::NAME,
            SourceFiles::to_sorted_batch(&source_files)?,
        ),
        (
            ContextModules::NAME,
            ContextModules::to_sorted_batch(&context_out.modules)?,
        ),
        (
            ContextDefinitions::NAME,
            ContextDefinitions::to_sorted_batch(&context_out.definitions)?,
        ),
        (
            Declarations::NAME,
            Declarations::to_sorted_batch(&walked.declarations)?,
        ),
        (
            ExportSyntax::NAME,
            ExportSyntax::to_sorted_batch(&walked.export_syntax)?,
        ),
        (PublicNames::NAME, PublicNames::to_sorted_batch(&public)?),
        (
            ParameterSyntax::NAME,
            ParameterSyntax::to_sorted_batch(&walked.parameter_syntax)?,
        ),
        (
            ParameterDocs::NAME,
            ParameterDocs::to_sorted_batch(&walked.parameter_docs)?,
        ),
        (
            PysaFunctions::NAME,
            PysaFunctions::to_sorted_batch(&pysa.functions)?,
        ),
        (
            ParameterSemantics::NAME,
            ParameterSemantics::to_sorted_batch(&pysa.parameters)?,
        ),
        (
            ClassAncestry::NAME,
            ClassAncestry::to_sorted_batch(&pysa.ancestry)?,
        ),
        (
            PysaClasses::NAME,
            PysaClasses::to_sorted_batch(&pysa.classes)?,
        ),
        (
            CallSyntax::NAME,
            CallSyntax::to_sorted_batch(&walked.call_syntax)?,
        ),
        (
            Arguments::NAME,
            Arguments::to_sorted_batch(&walked.arguments)?,
        ),
        (
            SyntaxNodes::NAME,
            SyntaxNodes::to_sorted_batch(&walked.syntax_nodes)?,
        ),
        (Scopes::NAME, Scopes::to_sorted_batch(&lexical_out.scopes)?),
        (
            Bindings::NAME,
            Bindings::to_sorted_batch(&lexical_out.bindings)?,
        ),
        (
            References::NAME,
            References::to_sorted_batch(&lexical_out.references)?,
        ),
        (
            ReferenceResolutions::NAME,
            ReferenceResolutions::to_sorted_batch(&lexical_out.resolutions)?,
        ),
        (
            TypeTerms::NAME,
            TypeTerms::to_sorted_batch(&types_out.terms)?,
        ),
        (
            TypeTermArgs::NAME,
            TypeTermArgs::to_sorted_batch(&types_out.args)?,
        ),
        (
            TypeObservations::NAME,
            TypeObservations::to_sorted_batch(&types_out.observations)?,
        ),
        (
            RecordFields::NAME,
            RecordFields::to_sorted_batch(&types_out.fields)?,
        ),
        (
            Documents::NAME,
            Documents::to_sorted_batch(&docs_out.documents)?,
        ),
        (
            Passages::NAME,
            Passages::to_sorted_batch(&docs_out.passages)?,
        ),
        (
            CodeBlocks::NAME,
            CodeBlocks::to_sorted_batch(&docs_out.code_blocks)?,
        ),
        (DocLinks::NAME, DocLinks::to_sorted_batch(&docs_out.links)?),
        (
            Mentions::NAME,
            Mentions::to_sorted_batch(&docs_out.mentions)?,
        ),
        (
            DocComponents::NAME,
            DocComponents::to_sorted_batch(&docs_out.components)?,
        ),
        (
            DocComponentAttributes::NAME,
            DocComponentAttributes::to_sorted_batch(&docs_out.component_attributes)?,
        ),
        (PysaCalls::NAME, PysaCalls::to_sorted_batch(&pysa.calls)?),
        (FlowUses::NAME, FlowUses::to_sorted_batch(&flow_out.uses)?),
        (
            FlowDefinitions::NAME,
            FlowDefinitions::to_sorted_batch(&flow_out.definitions)?,
        ),
        (
            FlowReaching::NAME,
            FlowReaching::to_sorted_batch(&flow_out.reaching)?,
        ),
        (
            FlowValues::NAME,
            FlowValues::to_sorted_batch(&flow_out.values)?,
        ),
        (
            FlowRegions::NAME,
            FlowRegions::to_sorted_batch(&flow_out.regions)?,
        ),
        (
            FlowTests::NAME,
            FlowTests::to_sorted_batch(&flow_out.tests)?,
        ),
        (
            FlowAttributeLoads::NAME,
            FlowAttributeLoads::to_sorted_batch(&flow_out.attribute_loads)?,
        ),
        (
            Conditions::NAME,
            Conditions::to_sorted_batch(&flow_out.conditions)?,
        ),
        (
            ConditionLiterals::NAME,
            ConditionLiterals::to_sorted_batch(&flow_out.literals)?,
        ),
        (Coverage::NAME, Coverage::to_sorted_batch(&report.coverage)?),
        (
            Boundaries::NAME,
            Boundaries::to_sorted_batch(&report.boundaries)?,
        ),
        (Facts::NAME, Facts::to_sorted_batch(&sink.into_rows()?)?),
    ];
    stages.mark("extract: batches");
    Ok((
        ExtractOutput {
            run_id,
            context_id: context.id,
            producer_id: producer.id,
            tables,
            pysa_json,
            stages: stages.stages,
        },
        vocabulary,
    ))
}

/// Each `from m import *` of a module → the names Pyrefly's wildcard set for `m` holds, or `None`
/// when Pyrefly cannot find `m` (C3 review F1). Star imports are module-level only.
fn star_imports(txn: &Transaction<'_>, m: &SourceModule, ast: &ModModule) -> lexical::Stars {
    struct Found<'a>(Vec<&'a ruff_python_ast::StmtImportFrom>);
    impl<'a> StatementVisitor<'a> for Found<'a> {
        fn visit_stmt(&mut self, stmt: &'a ruff_python_ast::Stmt) {
            if let ruff_python_ast::Stmt::ImportFrom(i) = stmt
                && i.names.iter().any(|a| a.name.as_str() == "*")
            {
                self.0.push(i);
            }
            walk_stmt(self, stmt);
        }
    }
    let mut found = Found(Vec::new());
    found.visit_body(&ast.body);
    let is_package = m.handle.path().is_init();
    let mut stars = lexical::Stars::new();
    for i in found.0 {
        let Some(star) = i.names.iter().find(|a| a.name.as_str() == "*") else {
            continue;
        };
        let names = walk::absolute_module(
            &m.name,
            is_package,
            i64::from(i.level),
            i.module.as_ref().map(|n| n.as_str()),
        )
        .and_then(|name| {
            txn.import_handle(&m.handle, ModuleName::from_str(&name), None)
                .finding()
        })
        .map(|h| {
            txn.get_wildcard(&h)
                .iter()
                .map(ToString::to_string)
                .collect()
        });
        stars.insert(star.start().to_u32(), names);
    }
    stars
}

/// The `releases` row and, for an acquired library, one `distributions` row per installed
/// distribution of the context's environment (ADR-0013). The extractor run writes both once per
/// attempt, carrying Stage A's output; later producers reference `release_id`, never append.
fn release_rows(
    input: &ExtractInput,
    snapshot_id: Id,
    context_id: Id,
) -> (Vec<ReleasesRow>, Vec<DistributionsRow>) {
    let release_id = input.release.release_id;
    match &input.release.origin {
        ReleaseOrigin::Tree { label } => (
            vec![ReleasesRow {
                snapshot_id,
                release_id,
                library: None,
                requirement: None,
                lock_digest: None,
                distributions: Vec::new(),
                installer: None,
                label: Some(label.clone()),
            }],
            Vec::new(),
        ),
        // The corpus runs in the library's environment: its context lists the same distributions.
        ReleaseOrigin::Corpus { label, library, .. } => (
            vec![ReleasesRow {
                snapshot_id,
                release_id,
                library: library.as_ref().map(|l| l.name.clone()),
                requirement: None,
                lock_digest: None,
                distributions: Vec::new(),
                installer: None,
                label: Some(label.clone()),
            }],
            library
                .iter()
                .flat_map(|lib| &lib.distributions)
                .map(|d| DistributionsRow {
                    snapshot_id,
                    context_id,
                    name: d.name.clone(),
                    version: d.version.clone(),
                    artifact_sha256: d.artifact_sha256.clone(),
                    record_digest: d.record_digest,
                })
                .collect(),
        ),
        ReleaseOrigin::Library(lib) => (
            vec![ReleasesRow {
                snapshot_id,
                release_id,
                library: Some(lib.name.clone()),
                requirement: Some(lib.requirement.clone()),
                lock_digest: Some(lib.lock_digest),
                distributions: lib.release.clone(),
                installer: lib.installer.clone(),
                label: None,
            }],
            lib.distributions
                .iter()
                .map(|d| DistributionsRow {
                    snapshot_id,
                    context_id,
                    name: d.name.clone(),
                    version: d.version.clone(),
                    artifact_sha256: d.artifact_sha256.clone(),
                    record_digest: d.record_digest,
                })
                .collect(),
        ),
    }
}

/// Write each table as an Arrow IPC file `<dir>/<table>.arrow`.
pub fn write_ipc(dir: &Path, output: &ExtractOutput) -> Result<(), ExtractError> {
    fs_err::create_dir_all(dir)?;
    for (name, batch) in &output.tables {
        let file = fs_err::File::create(dir.join(format!("{name}.arrow")))?;
        let mut w = arrow_ipc::writer::FileWriter::try_new(file, &batch.schema())?;
        w.write(batch)?;
        w.finish()?;
    }
    Ok(())
}
