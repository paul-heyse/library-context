use ruff_db::Db as SourceDb;
use ruff_db::files::{Files, system_path_to_file};
use ruff_db::parsed::parsed_module;
use ruff_db::system::{DbWithTestSystem, DbWithWritableSystem, System, TestSystem};
use ruff_db::vendored::VendoredFileSystem;
use ruff_python_ast::{self as ast, visitor::source_order::{self, SourceOrderVisitor}};
use ruff_text_size::Ranged;
use ty_python_core::ast_ids::HasScopedUseId;
use ty_python_core::program::{Program, ProgramSettings};
use ty_python_core::definition::DefinitionState;
use ty_python_core::{Db, ProgramFile, semantic_index};

#[salsa::db]
#[derive(Clone)]
struct ProbeDb { storage: salsa::Storage<Self>, files: Files, system: TestSystem, vendored: VendoredFileSystem }
impl DbWithTestSystem for ProbeDb {
    fn test_system(&self) -> &TestSystem { &self.system }
    fn test_system_mut(&mut self) -> &mut TestSystem { &mut self.system }
}
#[salsa::db]
impl SourceDb for ProbeDb {
    fn vendored(&self) -> &VendoredFileSystem { &self.vendored }
    fn system(&self) -> &dyn System { &self.system }
    fn files(&self) -> &Files { &self.files }
}
#[salsa::db]
impl ty_module_resolver::Db for ProbeDb {}
#[salsa::db]
impl Db for ProbeDb { fn should_check_file(&self, _f: ruff_db::files::File) -> bool { true } }
#[salsa::db]
impl salsa::Database for ProbeDb {}

struct Uses<'a> { out: Vec<(&'a ast::Expr, String, ruff_text_size::TextRange)> }
impl<'a> SourceOrderVisitor<'a> for Uses<'a> {
    fn visit_expr(&mut self, e: &'a ast::Expr) {
        match e { ast::Expr::Name(n) if n.ctx.is_load() => self.out.push((e, n.id.to_string(), n.range())), ast::Expr::Attribute(a) if a.ctx.is_load() => { if let ast::Expr::Name(b) = &*a.value { if b.id.as_str() == "self" { self.out.push((e, format!("self.{}", a.attr), a.range())); } } } _ => {} }
        source_order::walk_expr(self, e);
    }
}

fn main() -> anyhow::Result<()> {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap())?;
    let vendored = ty_vendored::file_system().clone();
    let mut db = ProbeDb { storage: salsa::Storage::new(None), files: Files::default(), system: TestSystem::default(), vendored: vendored.clone() };
    db.write_file("/src/m.py", &src)?;
    let settings = ProgramSettings::empty(&vendored);
    let program = Program::from_settings(&db, &settings);
    let file = system_path_to_file(&db, "/src/m.py").map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let pf = ProgramFile::new(&db, file, program);
    let t = std::time::Instant::now();
    let index = semantic_index(&db, pf);
    let module = parsed_module(&db, pf.python_file(&db)).load(&db);
    let mut v = Uses { out: vec![] };
    for s in module.suite() { v.visit_stmt(s); }
    let line = |o: u32| src[..o as usize].matches('\n').count() + 1;
    for (e, name, rng) in &v.out {
        let scope = index.expression_scope_id(*e);
        let map = index.use_def_map(scope);
        let use_id = ast::ExprRef::from(*e).scoped_use_id(&db, pf);
        let mut defs = vec![];
        for b in map.bindings_at_use(use_id) {
            match b.binding {
                DefinitionState::Defined(d) => {
                    let r = d.full_range(&db, &module).range();
                    defs.push(format!("L{}[{:?}] reach={:?}", line(r.start().into()), d.kind(&db).category(false, &module), b.reachability_constraint));
                }
                other => defs.push(format!("{other:?} reach={:?}", b.reachability_constraint)),
            }
        }
        println!("use {:>10} L{:<3} <- {}", name, line(rng.start().into()), defs.join(" | "));
    }
    for sid in index.scope_ids() {
        let fs = sid.file_scope_id(&db);
        let map = index.use_def_map(fs);
        let n = map.range_reachability().count();
        println!("scope {:?}: {} ranges with reachability constraints", fs, n);
    }
    eprintln!("semantic_index + walk: {:?}", t.elapsed());
    Ok(())
}
