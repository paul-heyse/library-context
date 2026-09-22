//! Spike driver (S2–S5, S7): pyrefly 1.3.1 (patched) in-process → sorted Arrow tables.
//! Throwaway code; not production. Usage:
//!   spike-pyrefly --tree DIR --site DIR --out DIR [--threads N] [--shuffle]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use arrow_array::{ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use pyrefly::commands::coverage::collect::{
    EXCLUDED_MODULE_DUNDERS, compute_public_fqns, is_public_module, is_public_name,
    trace_export_origin,
};
use pyrefly::export::exports::ExportLocation;
use pyrefly::report::pysa::call_graph::{
    CallCallees, ExpressionCallees, ExpressionIdentifier, PysaCallTarget, Target, Unresolved,
    UnresolvedReason,
};
use pyrefly::report::pysa::captured_variable::collect_captured_variables_for_module;
use pyrefly::report::pysa::context::{ModuleAnswersContext, ModuleContext, PysaResolver};
use pyrefly::report::pysa::function::{FunctionParameter, FunctionParameters, FunctionRef};
use pyrefly::report::pysa::location::PysaLocation;
use pyrefly::report::pysa::module::ModuleIds;
use pyrefly::report::pysa::override_graph::create_reversed_override_graph_for_module;
use pyrefly::report::pysa::{
    PysaFormat, PysaReporter, export_module_call_graphs, export_module_definitions,
};
use pyrefly::state::require::Require;
use pyrefly::state::state::State;
use pyrefly_build::handle::Handle;
use pyrefly_config::config::{ConfigFile, ConfigSource};
use pyrefly_config::finder::ConfigFinder;
use pyrefly_python::module_path::ModulePath;
use pyrefly_python::sys_info::{PythonPlatform, PythonVersion};
use pyrefly_util::arc_id::ArcId;
use pyrefly_util::thread_pool::ThreadCount;
use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, walk_expr, walk_stmt};
use ruff_python_ast::{Expr, Stmt};
use ruff_source_file::{LineIndex, OneIndexed, PositionEncoding, SourceLocation};
use ruff_text_size::{Ranged, TextRange, TextSize};

const REFUSED_ENV: &[&str] = &["PYREFLY_STACK_SIZE", "PYREFLY_FIXPOINT_DETAILS"];

struct Args {
    tree: PathBuf,
    site: PathBuf,
    out: PathBuf,
    threads: usize,
    shuffle: bool,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let (mut tree, mut site, mut out) = (None, None, None);
    let (mut threads, mut shuffle) = (1usize, false);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tree" => tree = it.next().map(PathBuf::from),
            "--site" => site = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            "--threads" => threads = it.next().and_then(|s| s.parse().ok()).expect("--threads N"),
            "--shuffle" => shuffle = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let abs = |p: Option<PathBuf>, n: &str| {
        fs::canonicalize(p.unwrap_or_else(|| panic!("--{n} required"))).expect("path exists")
    };
    let out = out.expect("--out required");
    fs::create_dir_all(&out).expect("create out");
    Args {
        tree: abs(Some(PathBuf::from(tree.expect("--tree"))), "tree"),
        site: abs(site, "site"),
        out: fs::canonicalize(out).expect("out"),
        threads,
        shuffle,
    }
}

fn main() {
    // S2: refuse ambient knobs we cannot clear without `unsafe` (edition 2024).
    for (k, _) in std::env::vars_os() {
        let k = k.to_string_lossy();
        if REFUSED_ENV.contains(&k.as_ref()) || k.starts_with("PYSA_DUMP") {
            eprintln!("refusing to run: ambient variable {k} is set");
            std::process::exit(3);
        }
    }
    let args = parse_args();
    let worker = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || run(args))
        .expect("spawn");
    worker.join().expect("worker panicked");
}

fn config(args: &Args) -> ConfigFile {
    let mut cfg = ConfigFile {
        source: ConfigSource::File(args.tree.join("pyrefly.toml")),
        search_path_from_args: vec![args.tree.clone()],
        disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true,
        enable_fallback_search_path: false,
        ..ConfigFile::default()
    };
    cfg.python_environment.python_version = Some(PythonVersion::new(3, 14, 0));
    cfg.python_environment.python_platform = Some(PythonPlatform::linux());
    // Explicit Some prevents configure() from discovering typings/ or querying an interpreter.
    cfg.python_environment.site_package_path = Some(vec![args.site.clone()]);
    cfg.interpreters.skip_interpreter_query = true;
    let errors = cfg.configure();
    assert!(errors.is_empty(), "{} config errors", errors.len());
    cfg
}

fn python_files(root: &Path, acc: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            python_files(&p, acc);
        } else if matches!(p.extension().and_then(|e| e.to_str()), Some("py" | "pyi")) {
            acc.push(p);
        }
    }
}

fn run(args: Args) {
    let t0 = Instant::now();
    let cfg = config(&args);

    // Context digest: canonical JSON of the configured file plus the #[serde(skip)] inputs.
    let mut ctx = serde_json::to_value(&cfg).expect("config serializes");
    let search: Vec<String> = cfg.search_path().map(|p| p.display().to_string()).collect();
    let site: Vec<String> = cfg
        .site_package_path()
        .map(|p| p.display().to_string())
        .collect();
    ctx["lctx.search_path"] = serde_json::json!(search);
    ctx["lctx.site_package_path"] = serde_json::json!(site);
    ctx["lctx.sys_info"] = serde_json::json!(format!("{:?}", cfg.get_sys_info()));
    let ctx_text = serde_json::to_string_pretty(&ctx).unwrap();
    fs::write(args.out.join("context.json"), &ctx_text).unwrap();
    let ctx_digest = blake3::hash(ctx_text.as_bytes()).to_hex().to_string();

    let mut files = Vec::new();
    python_files(&args.tree, &mut files);
    let mut handles: Vec<Handle> = files
        .iter()
        .map(|p| cfg.handle_from_module_path(ModulePath::filesystem(p.clone())))
        .collect();
    handles.sort_by(|a, b| a.module().as_str().cmp(b.module().as_str()));
    let mut run_order = handles.clone();
    if args.shuffle {
        run_order.reverse();
        let third = run_order.len() / 3;
        run_order.rotate_left(third);
    }

    let finder = ConfigFinder::new_constant(ArcId::new(cfg.clone()));
    // --threads 0 = ThreadCount::Inline: everything on this driver-owned thread (review F8).
    let threads = match NonZeroUsize::new(args.threads) {
        Some(n) => ThreadCount::NumThreads(n),
        None => ThreadCount::Inline,
    };
    let state = State::new(finder, threads);
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.set_pysa_reporter(Some(Box::new(PysaReporter {
        module_ids: ModuleIds::new(&run_order),
        pysa_directory: PathBuf::new(),
        definitions_directory: PathBuf::new(),
        type_of_expressions_directory: PathBuf::new(),
        call_graphs_directory: PathBuf::new(),
        format: PysaFormat::Json,
        write_files: false,
    })));
    let t_run = Instant::now();
    txn.run(&run_order, Require::Everything, None);
    let run_secs = t_run.elapsed().as_secs_f64();
    let module_ids = &txn.pysa_reporter().expect("reporter installed").module_ids;

    let t_extract = Instant::now();
    let mut decls = Vec::new();
    let mut calls = Vec::new();
    let mut pysa = Vec::new();
    let mut params = Vec::new();
    let mut s5 = S5::default();
    fs::create_dir_all(args.out.join("pysa/definitions")).unwrap();
    fs::create_dir_all(args.out.join("pysa/call_graphs")).unwrap();

    for handle in &handles {
        let module = handle.module().to_string();
        let info = txn.get_module_info(handle).expect("module info");
        let ast = txn.get_ast(handle).expect("ast kept at Everything");
        let text = info.lined_buffer().contents().clone();
        let line_index = info.lined_buffer().line_index();

        // Syntax families: one walk over pyrefly's own parse.
        let mut w = Walker {
            module: &module,
            names: Vec::new(),
            decls: &mut decls,
            calls: Vec::new(),
        };
        for stmt in &ast.body {
            w.visit_stmt(stmt);
        }
        let module_calls = std::mem::take(&mut w.calls);

        // Semantic families: pyrefly's own Pysa collectors, in memory.
        let resolver = PysaResolver::new(&txn, module_ids, handle.clone());
        let context = ModuleContext {
            answers_context: ModuleAnswersContext::create(handle.clone(), &txn, module_ids),
            resolver: &resolver,
        };
        let captured = collect_captured_variables_for_module(&context);
        let overrides = create_reversed_override_graph_for_module(&context);
        let defs = export_module_definitions(&context, &captured, &overrides);
        let graphs = export_module_call_graphs(&context, &captured);
        write_parity(
            &args.out.join("pysa/definitions"),
            &module,
            serde_json::to_value(&defs).unwrap(),
        );
        write_parity(
            &args.out.join("pysa/call_graphs"),
            &module,
            serde_json::to_value(&graphs).unwrap(),
        );

        for (fid, def) in defs.function_definitions.as_map().iter() {
            let b = &def.base;
            let flags = format!(
                "overload={} static={} class={} getter={} setter={} stub={}",
                b.is_overload,
                b.is_staticmethod,
                b.is_classmethod,
                b.is_property_getter,
                b.is_property_setter,
                b.is_stub
            );
            for (si, sig) in def.undecorated_signatures.iter().enumerate() {
                let base = (
                    module.clone(),
                    fid.serialize_to_string(),
                    b.name.to_string(),
                    si as i64,
                );
                match &sig.parameters {
                    FunctionParameters::List(ps) => {
                        for (ord, p) in ps.iter().enumerate() {
                            let (kind, name, ann, req) = param_cols(p);
                            params.push(ParamRow {
                                key: base.clone(),
                                ordinal: ord as i64,
                                kind,
                                name,
                                annotation: ann,
                                required: req,
                                flags: flags.clone(),
                            });
                        }
                    }
                    FunctionParameters::Ellipsis => params.push(ParamRow {
                        key: base,
                        ordinal: -1,
                        kind: "ellipsis",
                        name: None,
                        annotation: String::new(),
                        required: None,
                        flags: flags.clone(),
                    }),
                    FunctionParameters::ParamSpec => params.push(ParamRow {
                        key: base,
                        ordinal: -1,
                        kind: "paramspec",
                        name: None,
                        annotation: String::new(),
                        required: None,
                        flags: flags.clone(),
                    }),
                }
            }
        }

        let mut regular_ranges = BTreeSet::new();
        for (fid, graph) in graphs.call_graphs {
            let caller = fid.serialize_to_string();
            for (eid, callees) in graph.as_map().iter() {
                let (ekind, loc, detail) = match &eid {
                    ExpressionIdentifier::Regular(l) => ("regular", l.clone(), None),
                    ExpressionIdentifier::ArtificialCall(o) => (
                        "artificial_call",
                        o.location.clone(),
                        Some(o.kind.to_string()),
                    ),
                    ExpressionIdentifier::ArtificialAttributeAccess(o) => (
                        "artificial_attribute_access",
                        o.location.clone(),
                        Some(o.kind.to_string()),
                    ),
                    ExpressionIdentifier::FormatStringArtificial(l) => {
                        ("format_string_artificial", l.clone(), None)
                    }
                    ExpressionIdentifier::FormatStringStringify(l) => {
                        ("format_string_stringify", l.clone(), None)
                    }
                    ExpressionIdentifier::Identifier {
                        location,
                        identifier,
                    } => ("identifier", location.clone(), Some(identifier.to_string())),
                };
                let range = to_range(line_index, &text, &loc);
                // S5: the conversion must be the exact inverse of PysaLocation::from_text_range.
                s5.locations += 1;
                if PysaLocation::from_text_range(range, &info) != loc {
                    s5.roundtrip_mismatch += 1;
                }
                if ekind == "regular" && matches!(callees, ExpressionCallees::Call(_)) {
                    regular_ranges.insert((range.start().to_u32(), range.end().to_u32()));
                }
                let site = SiteCols {
                    module: module.clone(),
                    caller: caller.clone(),
                    ekind,
                    start: range.start().to_u32() as i64,
                    end: range.end().to_u32() as i64,
                    detail,
                };
                push_callees(&mut pysa, &site, callees);
            }
        }
        for c in &module_calls {
            s5.call_syntax += 1;
            let call = (c.1 as u32, c.2 as u32);
            let func = (c.3 as u32, c.4 as u32);
            if regular_ranges.contains(&call) {
                s5.matched_call_range += 1;
            } else if regular_ranges.contains(&func) {
                s5.matched_func_range += 1;
            } else {
                s5.unmatched += 1;
                if s5.unmatched_examples.len() < 15 {
                    let snippet: String =
                        text[c.1 as usize..c.2 as usize].chars().take(60).collect();
                    s5.unmatched_examples
                        .push(format!("{module}@{}: {snippet}", c.1));
                }
            }
        }
        calls.extend(module_calls);
    }

    // Public names: pairs (access path → origin) via pyrefly's own helpers, checked against
    // compute_public_fqns so pyrefly stays the authority for what "public" means.
    let mut public = Vec::new();
    for handle in handles.iter().filter(|h| is_public_module(h.module())) {
        let data = txn.get_exports_data(handle);
        let exports = txn.get_exports(handle);
        let (names, via_all): (Vec<_>, bool) = match data.explicit_dunder_all_names() {
            Some(all) => (all.iter().cloned().collect(), true),
            None => (
                exports
                    .iter()
                    .filter(|(n, loc)| {
                        is_public_name(n.as_str())
                            && (matches!(loc, ExportLocation::ThisModule(_))
                                || data.is_explicit_reexport(n))
                    })
                    .map(|(n, _)| n.clone())
                    .collect(),
                false,
            ),
        };
        for name in names {
            if EXCLUDED_MODULE_DUNDERS.contains(&name.as_str()) {
                continue;
            }
            let origin = trace_export_origin(handle, name.clone(), &txn)
                .map(|(h, n)| format!("{}.{}", h.module(), n));
            public.push((format!("{}.{}", handle.module(), name), origin, via_all));
        }
    }
    let flattened: BTreeSet<String> = public
        .iter()
        .flat_map(|(a, o, _)| std::iter::once(a.clone()).chain(o.clone()))
        .collect();
    let (authority, _) = compute_public_fqns(&handles, &txn);
    let authority: BTreeSet<String> = authority.into_iter().collect();
    assert_eq!(
        flattened, authority,
        "public pairs disagree with compute_public_fqns"
    );
    let extract_secs = t_extract.elapsed().as_secs_f64();

    // Sorted tables (total keys: every column), Arrow IPC.
    decls.sort();
    calls.sort();
    pysa.sort();
    params.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    public.sort();
    let mut digests = BTreeMap::new();
    let mut put = |name: &str, batch: RecordBatch| {
        let path = args.out.join(format!("{name}.arrow"));
        let mut buf = Vec::new();
        {
            let mut w = arrow_ipc::writer::FileWriter::try_new(&mut buf, &batch.schema()).unwrap();
            w.write(&batch).unwrap();
            w.finish().unwrap();
        }
        fs::write(&path, &buf).unwrap();
        digests.insert(
            name.to_owned(),
            (batch.num_rows(), blake3::hash(&buf).to_hex().to_string()),
        );
    };
    put(
        "declarations",
        batch(vec![
            ("module", s(decls.iter().map(|d| Some(d.0.clone())))),
            ("qualname", s(decls.iter().map(|d| Some(d.1.clone())))),
            ("kind", s(decls.iter().map(|d| Some(d.2.to_owned())))),
            ("start_byte", i(decls.iter().map(|d| d.3))),
            ("end_byte", i(decls.iter().map(|d| d.4))),
            ("docstring", s(decls.iter().map(|d| d.5.clone()))),
            ("is_overload", b(decls.iter().map(|d| d.6))),
        ]),
    );
    put(
        "call_syntax",
        batch(vec![
            ("module", s(calls.iter().map(|c| Some(c.0.clone())))),
            ("start_byte", i(calls.iter().map(|c| c.1))),
            ("end_byte", i(calls.iter().map(|c| c.2))),
            ("func_start", i(calls.iter().map(|c| c.3))),
            ("func_end", i(calls.iter().map(|c| c.4))),
            ("n_args", i(calls.iter().map(|c| c.5))),
            ("n_keywords", i(calls.iter().map(|c| c.6))),
        ]),
    );
    put(
        "pysa_calls",
        batch(vec![("row", s(pysa.iter().map(|r| Some(r.clone()))))]),
    );
    put(
        "parameter_semantics",
        batch(vec![(
            "row",
            s(params.iter().map(|r| Some(format!("{r:?}")))),
        )]),
    );
    put(
        "public_names",
        batch(vec![
            ("access_path", s(public.iter().map(|p| Some(p.0.clone())))),
            ("origin", s(public.iter().map(|p| p.1.clone()))),
            ("via_dunder_all", b(public.iter().map(|p| p.2))),
        ]),
    );

    let rss_kb = fs::read_to_string("/proc/self/status").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("VmHWM"))
            .map(|l| l.to_owned())
    });
    let summary = serde_json::json!({
        "modules": handles.len(),
        "threads": args.threads,
        "shuffle": args.shuffle,
        "context_digest": ctx_digest,
        "tables": digests,
        "s5": s5.to_json(),
        "seconds": {"run": run_secs, "extract": extract_secs, "total": t0.elapsed().as_secs_f64()},
        "peak_rss": rss_kb,
    });
    fs::write(
        args.out.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

fn write_parity(dir: &Path, module: &str, mut v: serde_json::Value) {
    strip_module_ids(&mut v);
    fs::write(
        dir.join(format!("{module}.json")),
        serde_json::to_string_pretty(&v).unwrap(),
    )
    .unwrap();
}

/// Dependency module ids come from a parallel atomic counter; names are kept beside them.
fn strip_module_ids(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(m) => {
            m.remove("module_id");
            m.values_mut().for_each(strip_module_ids);
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(strip_module_ids),
        _ => {}
    }
}

fn to_range(li: &LineIndex, text: &str, loc: &PysaLocation) -> TextRange {
    let at = |line: u32, col: u32| -> TextSize {
        li.offset(
            SourceLocation {
                line: OneIndexed::new(line as usize).unwrap(),
                character_offset: OneIndexed::new(col as usize).unwrap(),
            },
            text,
            PositionEncoding::Utf8,
        )
    };
    TextRange::new(at(loc.line(), loc.col()), at(loc.end_line(), loc.end_col()))
}

#[derive(Default)]
struct S5 {
    locations: u64,
    roundtrip_mismatch: u64,
    call_syntax: u64,
    matched_call_range: u64,
    matched_func_range: u64,
    unmatched: u64,
    unmatched_examples: Vec<String>,
}

impl S5 {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pysa_locations": self.locations,
            "roundtrip_mismatch": self.roundtrip_mismatch,
            "call_syntax_rows": self.call_syntax,
            "matched_on_call_range": self.matched_call_range,
            "matched_on_func_range": self.matched_func_range,
            "unmatched": self.unmatched,
            "unmatched_examples": self.unmatched_examples,
        })
    }
}

struct SiteCols {
    module: String,
    caller: String,
    ekind: &'static str,
    start: i64,
    end: i64,
    detail: Option<String>,
}

/// Exhaustive on purpose: a new upstream variant must fail the build (DM-42, DM-51).
fn reason(r: UnresolvedReason) -> &'static str {
    match r {
        UnresolvedReason::LambdaArgument => "LambdaArgument",
        UnresolvedReason::UnexpectedPyreflyTarget => "UnexpectedPyreflyTarget",
        UnresolvedReason::EmptyPyreflyCallTarget => "EmptyPyreflyCallTarget",
        UnresolvedReason::UnknownClassField => "UnknownClassField",
        UnresolvedReason::ClassFieldOnlyExistInObject => "ClassFieldOnlyExistInObject",
        UnresolvedReason::UnsupportedFunctionTarget => "UnsupportedFunctionTarget",
        UnresolvedReason::UnexpectedDefiningClass => "UnexpectedDefiningClass",
        UnresolvedReason::UnexpectedInitMethod => "UnexpectedInitMethod",
        UnresolvedReason::UnexpectedNewMethod => "UnexpectedNewMethod",
        UnresolvedReason::UnexpectedCalleeExpression => "UnexpectedCalleeExpression",
        UnresolvedReason::UnresolvedMagicDunderAttr => "UnresolvedMagicDunderAttr",
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoBase => {
            "UnresolvedMagicDunderAttrDueToNoBase"
        }
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoAttribute => {
            "UnresolvedMagicDunderAttrDueToNoAttribute"
        }
        UnresolvedReason::Mixed => "Mixed",
    }
}

fn push_callees(rows: &mut Vec<String>, site: &SiteCols, callees: &ExpressionCallees<FunctionRef>) {
    let mut emit = |phase: &str,
                    t: Option<&PysaCallTarget<FunctionRef>>,
                    unresolved: Option<&'static str>| {
        let target = t.map(|t| {
            let (kind, f) = match &t.target {
                Target::Function(f) => ("function", Some(f)),
                Target::Overrides(f) => ("overrides", Some(f)),
                Target::FormatString => ("format_string", None),
            };
            let f = f.map(|f| {
                format!(
                    "{}::{}::{}",
                    f.module_name,
                    f.function_id.serialize_to_string(),
                    f.function_name
                )
            });
            let recv = t
                .receiver_class
                .as_ref()
                .map(|c| format!("{}.{}", c.class.module_name(), c.class.name()));
            format!(
                "{kind}|{f:?}|recv={recv:?}|implicit={:?}|dunder={}|cm={}|sm={}",
                t.implicit_receiver, t.implicit_dunder_call, t.is_class_method, t.is_static_method
            )
        });
        rows.push(format!(
            "{}|{:08}|{:08}|{}|{}|{:?}|{phase}|{target:?}|{unresolved:?}",
            site.module, site.start, site.end, site.caller, site.ekind, site.detail
        ));
    };
    let call = |phase: &str,
                    cc: &CallCallees<FunctionRef>,
                    emit: &mut dyn FnMut(
        &str,
        Option<&PysaCallTarget<FunctionRef>>,
        Option<&'static str>,
    )| {
        for t in &cc.call_targets {
            emit(phase, Some(t), None);
        }
        for t in &cc.init_targets {
            emit(&format!("{phase}:init"), Some(t), None);
        }
        for t in &cc.new_targets {
            emit(&format!("{phase}:new"), Some(t), None);
        }
        let mut hops: Vec<_> = cc.higher_order_parameters.values().collect();
        hops.sort_by_key(|h| h.index);
        for h in hops {
            for t in &h.call_targets {
                emit(&format!("{phase}:higher_order:{}", h.index), Some(t), None);
            }
            if let Unresolved::True(r) = &h.unresolved {
                emit(
                    &format!("{phase}:higher_order:{}", h.index),
                    None,
                    Some(reason(*r)),
                );
            }
        }
        if let Unresolved::True(r) = &cc.unresolved {
            emit(phase, None, Some(reason(*r)));
        }
    };
    match callees {
        ExpressionCallees::Call(cc) => call("call", &cc, &mut emit),
        ExpressionCallees::Identifier(ic) => call("if_called", &ic.if_called, &mut emit),
        ExpressionCallees::AttributeAccess(ac) => {
            call("if_called", &ac.if_called, &mut emit);
            for t in &ac.property_getters {
                emit("property_get", Some(t), None);
            }
            for t in &ac.property_setters {
                emit("property_set", Some(t), None);
            }
        }
        ExpressionCallees::Define(dc) => {
            for t in &dc.define_targets {
                emit("define", Some(t), None);
            }
        }
        ExpressionCallees::FormatStringArtificial(f) => {
            for t in &f.targets {
                emit("format_string_artificial", Some(t), None);
            }
        }
        ExpressionCallees::FormatStringStringify(f) => {
            for t in &f.targets {
                emit("format_string_stringify", Some(t), None);
            }
            if let Unresolved::True(r) = &f.unresolved {
                emit("format_string_stringify", None, Some(reason(*r)));
            }
        }
        ExpressionCallees::Return(r) => {
            for t in &r.targets {
                emit("return_shim", Some(t), None);
            }
        }
    }
}

#[derive(Debug)]
struct ParamRow {
    key: (String, String, String, i64),
    ordinal: i64,
    kind: &'static str,
    name: Option<String>,
    annotation: String,
    required: Option<bool>,
    flags: String,
}

fn param_cols(p: &FunctionParameter) -> (&'static str, Option<String>, String, Option<bool>) {
    match p {
        FunctionParameter::PosOnly {
            name,
            annotation,
            required,
        } => (
            "pos_only",
            name.as_ref().map(|n| n.to_string()),
            annotation.string.clone(),
            Some(*required),
        ),
        FunctionParameter::Pos {
            name,
            annotation,
            required,
        } => (
            "pos",
            Some(name.to_string()),
            annotation.string.clone(),
            Some(*required),
        ),
        FunctionParameter::VarArg { name, annotation } => (
            "var_arg",
            name.as_ref().map(|n| n.to_string()),
            annotation.string.clone(),
            None,
        ),
        FunctionParameter::KwOnly {
            name,
            annotation,
            required,
        } => (
            "kw_only",
            Some(name.to_string()),
            annotation.string.clone(),
            Some(*required),
        ),
        FunctionParameter::Kwargs { name, annotation } => (
            "kwargs",
            name.as_ref().map(|n| n.to_string()),
            annotation.string.clone(),
            None,
        ),
    }
}

type Decl = (String, String, &'static str, i64, i64, Option<String>, bool);
type Call = (String, i64, i64, i64, i64, i64, i64);

struct Walker<'m> {
    module: &'m str,
    names: Vec<String>,
    decls: &'m mut Vec<Decl>,
    calls: Vec<Call>,
}

fn docstring(body: &[Stmt]) -> Option<String> {
    match body.first() {
        Some(Stmt::Expr(e)) => e
            .value
            .as_string_literal_expr()
            .map(|s| s.value.to_str().to_owned()),
        _ => None,
    }
}

impl<'a> SourceOrderVisitor<'a> for Walker<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let (name, kind, body, overload) = match stmt {
            Stmt::FunctionDef(f) => {
                let overload = f.decorator_list.iter().any(|d| match &d.expression {
                    Expr::Name(n) => n.id.as_str() == "overload",
                    Expr::Attribute(a) => a.attr.as_str() == "overload",
                    _ => false,
                });
                (f.name.as_str(), "function", &f.body, overload)
            }
            Stmt::ClassDef(c) => (c.name.as_str(), "class", &c.body, false),
            _ => return walk_stmt(self, stmt),
        };
        self.names.push(name.to_owned());
        let r = stmt.range();
        self.decls.push((
            self.module.to_owned(),
            format!("{}.{}", self.module, self.names.join(".")),
            kind,
            r.start().to_u32() as i64,
            r.end().to_u32() as i64,
            docstring(body),
            overload,
        ));
        walk_stmt(self, stmt);
        self.names.pop();
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(c) = expr {
            let r = c.range();
            let f = c.func.range();
            self.calls.push((
                self.module.to_owned(),
                r.start().to_u32() as i64,
                r.end().to_u32() as i64,
                f.start().to_u32() as i64,
                f.end().to_u32() as i64,
                c.arguments.args.len() as i64,
                c.arguments.keywords.len() as i64,
            ));
        }
        walk_expr(self, expr);
    }
}

fn s(v: impl Iterator<Item = Option<String>>) -> ArrayRef {
    Arc::new(StringArray::from(v.collect::<Vec<_>>()))
}
fn i(v: impl Iterator<Item = i64>) -> ArrayRef {
    Arc::new(Int64Array::from(v.collect::<Vec<_>>()))
}
fn b(v: impl Iterator<Item = bool>) -> ArrayRef {
    Arc::new(BooleanArray::from(v.collect::<Vec<_>>()))
}
fn batch(cols: Vec<(&str, ArrayRef)>) -> RecordBatch {
    let fields: Vec<Field> = cols
        .iter()
        .map(|(n, a)| {
            Field::new(
                *n,
                a.data_type().clone(),
                a.null_count() > 0 || matches!(a.data_type(), DataType::Utf8),
            )
        })
        .collect();
    RecordBatch::try_new(
        Arc::new(Schema::new(fields)),
        cols.into_iter().map(|(_, a)| a).collect(),
    )
    .unwrap()
}
