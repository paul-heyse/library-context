//! Probe (2026-09-23): Pyrefly 1.3.1 fork 6a93da34 + ruff crates 0.0.11, in-process.
use std::path::PathBuf;

use pyrefly::binding::binding::Key;
use pyrefly::state::lsp::FindPreference;
use pyrefly::state::require::Require;
use pyrefly::state::state::State;
use pyrefly_config::config::{ConfigFile, ConfigSource};
use pyrefly_config::finder::ConfigFinder;
use pyrefly_python::docstring::Docstring;
use pyrefly_python::module_path::ModulePath;
use pyrefly_python::sys_info::{PythonPlatform, PythonVersion, SysInfo};
use pyrefly_util::arc_id::ArcId;
use pyrefly_util::thread_pool::ThreadCount;
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

/// Verbatim copy of `crates/cpg-extract/src/lexical.rs` L154-165 (kind as text).
fn static_test(test: &Expr, text: &str) -> Option<&'static str> {
    let src = text.get(test.start().to_usize()..test.end().to_usize())?;
    if src == "TYPE_CHECKING" || src.ends_with(".TYPE_CHECKING") {
        Some("TypeChecking")
    } else if src.contains("version_info") {
        Some("VersionInfo")
    } else if src.contains("sys.platform") {
        Some("Platform")
    } else {
        None
    }
}

/// Verbatim logic of `walk.rs` L193-201 (range only).
fn our_docstring(body: &[Stmt]) -> Option<TextRange> {
    match body.first() {
        Some(Stmt::Expr(e)) => e.value.as_string_literal_expr().map(|_| e.range()),
        _ => None,
    }
}

#[derive(Default)]
struct Collect<'a> {
    ifs: Vec<&'a ruff_python_ast::StmtIf>,
    bodies: Vec<&'a [Stmt]>,
}
impl<'a> StatementVisitor<'a> for Collect<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::If(i) => self.ifs.push(i),
            Stmt::FunctionDef(f) => self.bodies.push(&f.body),
            Stmt::ClassDef(c) => self.bodies.push(&c.body),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixture");
    let file = root.join("pkg/mod.py");
    let mut cfg = ConfigFile {
        source: ConfigSource::File(root.join("pyrefly.toml")),
        search_path_from_args: vec![root.clone()],
        disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true,
        enable_fallback_search_path: false,
        ..ConfigFile::default()
    };
    cfg.python_environment.python_version = Some(PythonVersion::new(3, 14, 0));
    cfg.python_environment.python_platform = Some(PythonPlatform::new("linux"));
    cfg.python_environment.site_package_path = Some(vec![]);
    cfg.interpreters.skip_interpreter_query = true;
    assert!(cfg.configure().is_empty());
    let handle = cfg.handle_from_module_path(ModulePath::filesystem(file));
    let state = State::new(
        ConfigFinder::new_constant(ArcId::new(cfg)),
        ThreadCount::Inline,
    );
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.run(&[handle.clone()], Require::Everything, None);
    let ast = txn.get_ast(&handle).unwrap();
    let info = txn.get_module_info(&handle).unwrap();
    let text = info.lined_buffer().contents().clone();
    let sys_info: &SysInfo = handle.sys_info();
    let at = |r: TextRange| text[r].to_owned();

    let mut c = Collect::default();
    c.visit_body(&ast.body);

    println!("== P1 static tests: lexical.rs static_test vs SysInfo (py3.14, linux)");
    for i in &c.ifs {
        let lex = static_test(&i.test, &text);
        println!(
            "if   {:<40} lexical={:<13} evaluate_bool={:<12} with_sys_info={:<12} tc_guard={} not_tc_guard={}",
            at(i.test.range()),
            format!("{lex:?}"),
            format!("{:?}", sys_info.evaluate_bool(&i.test)),
            format!("{:?}", sys_info.evaluate_bool_with_sys_info(&i.test)),
            SysInfo::is_type_checking_guard(&i.test),
            SysInfo::is_not_type_checking_guard(&i.test),
        );
        for clause in &i.elif_else_clauses {
            if let Some(t) = &clause.test {
                println!(
                    "elif {:<40} lexical=inherits {:?}/false evaluate_bool={:?}",
                    at(t.range()),
                    lex,
                    sys_info.evaluate_bool(t)
                );
            }
        }
    }

    println!("\n== P2 docstring: walk.rs docstring() vs Docstring::range_from_stmts");
    let mut bodies = vec![&ast.body[..]];
    bodies.extend(c.bodies.iter().copied());
    let same = bodies
        .iter()
        .filter(|b| our_docstring(b) == Docstring::range_from_stmts(b))
        .count();
    println!("{same}/{} bodies agree", bodies.len());

    println!("\n== P3 Pyrefly Bindings: Key::Definition names (flow-sensitive, pruned)");
    let bindings = txn.get_bindings(&handle).unwrap();
    let mut defs: Vec<(u32, String)> = bindings
        .keys::<Key>()
        .filter_map(|idx| match bindings.idx_to_key(idx) {
            Key::Definition(sid) => Some((sid.range().start().to_u32(), at(sid.range()))),
            _ => None,
        })
        .collect();
    defs.sort();
    println!(
        "{:?}",
        defs.iter().map(|d| d.1.as_str()).collect::<Vec<_>>()
    );

    println!("\n== P4 find_definition at each use in `print(...)` and `return x`");
    let print_line = text.find("print(D").unwrap();
    for name in ["D", "V", "W", "N", "R", "T2"] {
        let off = print_line
            + text[print_line..]
                .find(&format!(" {name},"))
                .map_or(6, |o| o + 1);
        let found = txn.find_definition(
            &handle,
            TextSize::new(off as u32),
            FindPreference::default(),
        );
        match found {
            Ok(v) => println!(
                "{name}: {} target(s) {:?}",
                v.len(),
                v.iter()
                    .map(|d| format!("{}@{:?}", d.module.name(), d.definition_range))
                    .collect::<Vec<_>>()
            ),
            Err(e) => println!("{name}: {e:?}"),
        }
    }
    let ret_x = text.find("return x").unwrap() + 7;
    let v = txn
        .find_definition(
            &handle,
            TextSize::new(ret_x as u32),
            FindPreference::default(),
        )
        .map(|v| v.iter().map(|d| at(d.definition_range)).collect::<Vec<_>>());
    println!("x in `return x`: {v:?}");

    println!("\n== P5 Glean in-process (report::glean::convert::glean)");
    let g = pyrefly::report::glean::convert::glean(&txn, &handle);
    let json = serde_json::to_value(&g).unwrap();
    if let Some(entries) = json.get("entries").and_then(|e| e.as_array()) {
        for e in entries {
            if let Some(p) = e.get("predicate").and_then(|p| p.as_str()) {
                let n = e
                    .get("facts")
                    .and_then(|f| f.as_array())
                    .map_or(0, Vec::len);
                println!("{p}: {n}");
            }
        }
        let s = serde_json::to_string(&json).unwrap();
        for needle in [
            "pkg.mod.C.helper",
            "pkg.mod.f",
            "pkg.mod.f.<locals>.x",
            "Doc of f",
        ] {
            println!("mentions {needle:?}: {}", s.matches(needle).count());
        }
    }
    std::fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("glean.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();

    println!("\n== P6 ruff_python_semantic 0.0.11: public model, caller-driven");
    use ruff_python_semantic::{
        BindingFlags, BindingKind, Module, ModuleKind, ModuleSource, SemanticModel,
    };
    let path = std::path::Path::new("mod.py");
    let mut model = SemanticModel::new(
        &[],
        &[],
        ruff_python_ast::PythonVersion::PY314,
        ruff_python_ast::PySourceType::Python,
        path,
        Module {
            kind: ModuleKind::Module,
            source: ModuleSource::File(path),
            python_ast: &ast.body,
            name: None,
        },
    );
    let r1 = TextRange::new(TextSize::new(0), TextSize::new(1));
    let r2 = TextRange::new(TextSize::new(10), TextSize::new(11));
    let b1 = model.push_binding("x", r1, BindingKind::Assignment, BindingFlags::empty());
    model.current_scope_mut().add("x", b1);
    let b2 = model.push_binding("x", r2, BindingKind::Assignment, BindingFlags::empty());
    model.current_scope_mut().add("x", b2);
    let scope = model.current_scope();
    println!(
        "get(x)={:?} (b1={b1:?}, b2={b2:?}); get_all(x)={:?}; global scope bindings before any driver: {}",
        scope.get("x"),
        scope.get_all("x").collect::<Vec<_>>(),
        scope.binding_ids().count()
    );
}
