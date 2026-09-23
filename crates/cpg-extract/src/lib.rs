//! Pyrefly + Ruff in-process extraction of the raw fact families (DESIGN §4.2, ADR-0012).
//!
//! One run: the constructed configuration, one `State` at `ThreadCount::Inline` on a driver-owned
//! thread, `run` at `Require::Everything` with a no-write Pysa reporter, then per module the Ruff
//! walk over Pyrefly's own parse and Pyrefly's own Pysa collectors. Any panic aborts the whole
//! extraction (§4.2.5): there is no per-module recovery.

mod config;
mod facts;
mod public;
mod pysa_map;
mod walk;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use cpg_schema::codebook::{
    BoundaryReason, CoverageStatus, FactFamily, Fidelity, Modality, Origin, ScopeKind,
};
use cpg_schema::id::{Id, IdHasher, content_digest, kind};
use cpg_schema::table::Table;
use cpg_schema::tables::{
    Arguments, Boundaries, BoundariesRow, CallSyntax, ClassAncestry, Contexts, ContextsRow,
    Coverage, CoverageRow, Declarations, ExportSyntax, Facts, ParameterSemantics, ParameterSyntax,
    Producers, ProducersRow, PublicNames, PysaCalls, PysaFunctions, Runs, RunsRow, SourceFiles,
    SourceFilesRow,
};
use pyrefly::report::pysa::captured_variable::collect_captured_variables_for_module;
use pyrefly::report::pysa::context::{ModuleAnswersContext, ModuleContext, PysaResolver};
use pyrefly::report::pysa::module::ModuleIds;
use pyrefly::report::pysa::override_graph::create_reversed_override_graph_for_module;
use pyrefly::report::pysa::{
    PysaFormat, PysaReporter, export_module_call_graphs, export_module_definitions,
};
use pyrefly::state::require::Require;
use pyrefly::state::state::State;
use pyrefly_build::handle::Handle;
use pyrefly_config::error_kind::ErrorKind;
use pyrefly_config::finder::ConfigFinder;
use pyrefly_python::module_path::ModulePath;
use pyrefly_util::arc_id::ArcId;
use pyrefly_util::thread_pool::ThreadCount;
use ruff_text_size::Ranged;
use serde_json::Value;

pub use config::{
    DRIVER_STACK_BYTES, ExtractInput, PYREFLY_PATCH_SHA256, PYREFLY_REV, REFUSED_ENV,
    REFUSED_ENV_PREFIX, TOOL, TestHooks,
};
use facts::{FactSink, Provenance, Surface, dedup_by_fact, fact_row};
use pysa_map::{Here, Locator, ModuleRefs, PysaOut};
use walk::{ModuleCtx, span};

/// The fact families this producer declares (coverage rows exist for each, per module).
pub const FAMILIES: [FactFamily; 3] = [
    FactFamily::Exports,
    FactFamily::Signatures,
    FactFamily::Calls,
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

fn python_files(root: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if p.is_dir() {
            if !name.starts_with('.') && name != "__pycache__" {
                python_files(&p, acc)?;
            }
        } else if name.ends_with(".py") || name.ends_with(".pyi") {
            acc.push(p);
        }
    }
    Ok(())
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
        self.coverage.push(CoverageRow {
            snapshot_id: sink.snapshot_id,
            run_id: sink.run_id,
            scope_kind: ScopeKind::Module,
            scope_node_id: module,
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
        let provenance = Provenance {
            surface: Surface::Source,
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

fn run(input: &ExtractInput) -> Result<ExtractOutput, ExtractError> {
    let cfg = config::pyrefly_config(input)?;
    let context = config::context(&cfg, input)?;
    let producer = config::producer();
    let family_names: Vec<&str> = FAMILIES
        .iter()
        .map(|f| cpg_schema::Codebook::text(*f))
        .collect();
    let run_id = config::run_id(input.release_id, context.id, &producer, &family_names);
    let mut sink = FactSink::new(run_id, input.snapshot_id, producer.id);

    // Modules of the release, sorted by name.
    let mut files = Vec::new();
    python_files(&input.release_root, &mut files)?;
    let mut modules = Vec::new();
    for file in files {
        let rel = file
            .strip_prefix(&input.release_root)
            .map_err(|_| ExtractError::RelativePath(file.clone()))?
            .display()
            .to_string();
        let handle = cfg.handle_from_module_path(ModulePath::filesystem(file.clone()));
        let node_id = IdHasher::new(kind::MODULE)
            .id(input.release_id)
            .str(&rel)
            .finish_id();
        modules.push(SourceModule {
            name: handle.module().to_string(),
            handle,
            path: rel,
            node_id,
            bytes: std::fs::read(&file)?,
        });
    }
    modules.sort_by(|a, b| (&a.name, &a.path).cmp(&(&b.name, &b.path)));
    if input.test_hooks.reverse_module_order {
        modules.reverse();
    }
    let handles: Vec<Handle> = modules.iter().map(|m| m.handle.clone()).collect();

    // One check at `Everything`, single-threaded, with the no-write Pysa reporter installed.
    let finder = ConfigFinder::new_constant(ArcId::new(cfg));
    let state = State::new(finder, ThreadCount::Inline);
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.set_pysa_reporter(Some(Box::new(PysaReporter {
        module_ids: ModuleIds::new(&handles),
        pysa_directory: PathBuf::new(),
        definitions_directory: PathBuf::new(),
        type_of_expressions_directory: PathBuf::new(),
        call_graphs_directory: PathBuf::new(),
        format: PysaFormat::Json,
        write_files: false,
    })));
    txn.run(&handles, Require::Everything, None);
    let txn = txn;
    // The reporter stays installed: dependency modules solve lazily during extraction.
    let module_ids = &txn
        .pysa_reporter()
        .expect("the reporter was installed before run")
        .module_ids;
    // Project handles hold ids pre-assigned in sorted order; only dependencies are numbered lazily.
    let refs = ModuleRefs {
        release_files: modules
            .iter()
            .map(|m| (module_ids.get_from_handle(&m.handle), m.path.clone()))
            .collect(),
    };

    let mut source_files = Vec::new();
    let mut walked = walk::WalkOut::default();
    let mut pysa = PysaOut::default();
    let mut report = Report {
        coverage: Vec::new(),
        boundaries: Vec::new(),
    };
    let mut pysa_json = BTreeMap::new();
    let source = Provenance {
        surface: Surface::Source,
        origin: Origin::InputContext,
        modality: Modality::Definite,
        fidelity: Fidelity::Raw,
    };

    for m in &modules {
        let utf8 = std::str::from_utf8(&m.bytes).is_ok();
        source_files.push(fact_row!(
            sink,
            SourceFiles,
            Provenance { ..source },
            SourceFilesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: m.node_id,
                release_id: input.release_id,
                module_name: m.name.clone(),
                path: m.path.clone(),
                is_package: m.path.ends_with("__init__.py") || m.path.ends_with("__init__.pyi"),
                is_stub: m.path.ends_with(".pyi"),
                content_digest: content_digest(&m.bytes),
                byte_len: m.bytes.len() as i64,
                utf8,
            }
        ));
        if !utf8 {
            // Pyrefly would load it as an empty module, which must not read as "no API".
            for family in FAMILIES {
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
        let ctx = ModuleCtx {
            release_id: input.release_id,
            path: &m.path,
            module_name: &m.name,
            module_node_id: m.node_id,
            text: &text,
        };
        let module_walk = walk::walk_module(&ctx, &ast, &mut sink);

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
            partial.extend(FAMILIES.map(|f| (f, BoundaryReason::SyntaxError)));
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
        for family in FAMILIES {
            let (status, reason) = match partial.get(&family) {
                Some(reason) => (CoverageStatus::Partial, Some(*reason)),
                None => (CoverageStatus::CompleteUnderStatedModel, None),
            };
            report.cover(&sink, m.node_id, family, status, reason, None);
        }

        walked.declarations.extend(module_walk.declarations);
        walked.export_syntax.extend(module_walk.export_syntax);
        walked.parameter_syntax.extend(module_walk.parameter_syntax);
        walked.call_syntax.extend(module_walk.call_syntax);
        walked.arguments.extend(module_walk.arguments);
        pysa.functions.extend(module_pysa.functions);
        pysa.parameters.extend(module_pysa.parameters);
        pysa.ancestry.extend(module_pysa.ancestry);
        pysa.calls.extend(module_pysa.calls);
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
    let mut public = public::public_names(&readable, &release_files, &txn, &mut sink)?;

    let snapshot_id = input.snapshot_id;
    let runs = vec![RunsRow {
        snapshot_id,
        run_id,
        release_id: input.release_id,
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
        site_packages_digest: context.site_packages_digest,
    }];
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
    dedup_by_fact(&mut walked.call_syntax, |r| r.fact_id);
    dedup_by_fact(&mut walked.arguments, |r| r.fact_id);
    dedup_by_fact(&mut pysa.functions, |r| r.fact_id);
    dedup_by_fact(&mut pysa.parameters, |r| r.fact_id);
    dedup_by_fact(&mut pysa.ancestry, |r| r.fact_id);
    dedup_by_fact(&mut pysa.calls, |r| r.fact_id);
    dedup_by_fact(&mut public, |r| r.fact_id);
    dedup_by_fact(&mut report.boundaries, |r| r.fact_id);

    let tables = vec![
        (Runs::NAME, Runs::to_sorted_batch(&runs)?),
        (Contexts::NAME, Contexts::to_sorted_batch(&contexts)?),
        (Producers::NAME, Producers::to_sorted_batch(&producers)?),
        (
            SourceFiles::NAME,
            SourceFiles::to_sorted_batch(&source_files)?,
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
            CallSyntax::NAME,
            CallSyntax::to_sorted_batch(&walked.call_syntax)?,
        ),
        (
            Arguments::NAME,
            Arguments::to_sorted_batch(&walked.arguments)?,
        ),
        (PysaCalls::NAME, PysaCalls::to_sorted_batch(&pysa.calls)?),
        (Coverage::NAME, Coverage::to_sorted_batch(&report.coverage)?),
        (
            Boundaries::NAME,
            Boundaries::to_sorted_batch(&report.boundaries)?,
        ),
        (Facts::NAME, Facts::to_sorted_batch(&sink.into_rows()?)?),
    ];
    Ok(ExtractOutput {
        run_id,
        context_id: context.id,
        producer_id: producer.id,
        tables,
        pysa_json,
    })
}

/// Write each table as an Arrow IPC file `<dir>/<table>.arrow`.
pub fn write_ipc(dir: &Path, output: &ExtractOutput) -> Result<(), ExtractError> {
    std::fs::create_dir_all(dir)?;
    for (name, batch) in &output.tables {
        let file = std::fs::File::create(dir.join(format!("{name}.arrow")))?;
        let mut w = arrow_ipc::writer::FileWriter::try_new(file, &batch.schema())?;
        w.write(batch)?;
        w.finish()?;
    }
    Ok(())
}
