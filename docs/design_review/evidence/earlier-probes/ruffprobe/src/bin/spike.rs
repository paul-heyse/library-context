//! Stage 2.1 spike: ty_python_core 0.0.14's use-def over every release module of the pilot.
use ruff_db::Db as SourceDb;
use ruff_db::files::{Files, system_path_to_file};
use ruff_db::parsed::parsed_module;
use ruff_db::system::{DbWithTestSystem, DbWithWritableSystem, System, TestSystem};
use ruff_db::vendored::VendoredFileSystem;
use ruff_python_ast::{self as ast, visitor::source_order::{self, SourceOrderVisitor}};
use ruff_text_size::Ranged;
use std::io::Write;
use ty_python_core::ast_ids::HasScopedUseId;
use ty_python_core::definition::DefinitionState;
use ty_python_core::program::{Program, ProgramSettings};
use ty_python_core::{Db, ProgramFile, semantic_index};

#[salsa::db]
#[derive(Clone)]
struct SpikeDb { storage: salsa::Storage<Self>, files: Files, system: TestSystem, vendored: VendoredFileSystem }
impl DbWithTestSystem for SpikeDb {
    fn test_system(&self) -> &TestSystem { &self.system }
    fn test_system_mut(&mut self) -> &mut TestSystem { &mut self.system }
}
#[salsa::db]
impl SourceDb for SpikeDb {
    fn vendored(&self) -> &VendoredFileSystem { &self.vendored }
    fn system(&self) -> &dyn System { &self.system }
    fn files(&self) -> &Files { &self.files }
}
#[salsa::db]
impl ty_module_resolver::Db for SpikeDb {}
#[salsa::db]
impl Db for SpikeDb { fn should_check_file(&self, _f: ruff_db::files::File) -> bool { true } }
#[salsa::db]
impl salsa::Database for SpikeDb {}

struct Uses<'a> { out: Vec<(&'a ast::Expr, String, ruff_text_size::TextRange)> }
impl<'a> SourceOrderVisitor<'a> for Uses<'a> {
    fn visit_expr(&mut self, e: &'a ast::Expr) {
        if let ast::Expr::Name(n) = e { if n.ctx.is_load() { self.out.push((e, n.id.to_string(), n.range())); } }
        source_order::walk_expr(self, e);
    }
}

fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() { walk(&p, root, out); }
        else if p.extension().is_some_and(|x| x == "py") { out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned()); }
    }
}

fn peak_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status").unwrap().lines()
        .find(|l| l.starts_with("VmHWM")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let site = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let out_path = std::env::args().nth(2).unwrap();
    let mut rels = Vec::new();
    for pkg in ["fastmcp", "fastmcp_tasks"] { walk(&site.join(pkg), &site, &mut rels); }
    let vendored = ty_vendored::file_system().clone();
    let mut db = SpikeDb { storage: salsa::Storage::new(None), files: Files::default(), system: TestSystem::default(), vendored: vendored.clone() };
    for rel in &rels {
        let text = std::fs::read_to_string(site.join(rel))?;
        db.write_file(format!("/src/{rel}"), &text)?;
    }
    let settings = ProgramSettings::empty(&vendored);
    let program = Program::from_settings(&db, &settings);
    let mut out = std::io::BufWriter::new(std::fs::File::create(&out_path)?);
    let t0 = std::time::Instant::now();
    let (mut uses_n, mut modules, mut failed) = (0usize, 0usize, 0usize);
    for rel in &rels {
        let path = format!("/src/{rel}");
        let Ok(file) = system_path_to_file(&db, &path) else { failed += 1; continue; };
        let pf = ProgramFile::new(&db, file, program);
        let index = semantic_index(&db, pf);
        let module = parsed_module(&db, pf.python_file(&db)).load(&db);
        let mut v = Uses { out: vec![] };
        for s in module.suite() { v.visit_stmt(s); }
        for (e, name, rng) in &v.out {
            let Some(scope) = index.try_expression_scope_id(*e) else { continue };
            let map = index.use_def_map(scope);
            let use_id = ast::ExprRef::from(*e).scoped_use_id(&db, pf);
            let mut defs = Vec::new();
            let mut undefined = false;
            for b in map.bindings_at_use(use_id) {
                match b.binding {
                    DefinitionState::Defined(d) => {
                        let r = d.focus_range(&db, &module).range();
                        defs.push(format!("{}:{}", u32::from(r.start()), u32::from(r.end())));
                    }
                    _ => undefined = true,
                }
            }
            writeln!(out, "{rel}\t{}\t{}\t{name}\t{}\t{}", u32::from(rng.start()), u32::from(rng.end()), defs.join(","), undefined)?;
            uses_n += 1;
        }
        modules += 1;
    }
    out.flush()?;
    eprintln!("modules {modules} failed {failed} uses {uses_n} elapsed {:?} peak {} MiB", t0.elapsed(), peak_kib() / 1024);
    Ok(())
}
