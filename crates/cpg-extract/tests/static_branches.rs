//! H1 review F1: every binding's static polarity equals Pyrefly's own pruning, applied recursively
//! as its binding pass applies it. A clause inside a pruned clause is never walked, whatever its
//! own test decides.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use pyrefly_python::ast::Ast;
use pyrefly_python::sys_info::{PythonPlatform, PythonVersion, SysInfo};
use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, walk_stmt};
use ruff_python_ast::{Expr, PySourceType, Stmt};
use ruff_text_size::Ranged;

type Span = (i64, i64);

/// Every assignment-target name the walk reaches. With `sys`, an `if` descends only into the
/// clauses `pruned_if_branches` keeps (Pyrefly's binding pass); without, into every clause.
struct Targets<'a> {
    sys: Option<&'a SysInfo>,
    found: BTreeSet<Span>,
}

impl<'a, 'b> SourceOrderVisitor<'b> for Targets<'a> {
    fn visit_stmt(&mut self, stmt: &'b Stmt) {
        match (stmt, self.sys) {
            (Stmt::If(i), Some(sys)) => {
                for (_, body) in sys.pruned_if_branches(i) {
                    for s in body {
                        self.visit_stmt(s);
                    }
                }
            }
            (Stmt::Assign(a), _) => {
                for t in &a.targets {
                    if let Expr::Name(n) = t {
                        let r = n.range();
                        self.found
                            .insert((i64::from(r.start().to_u32()), i64::from(r.end().to_u32())));
                    }
                }
                walk_stmt(self, stmt);
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

fn targets(sys: Option<&SysInfo>, body: &[Stmt]) -> BTreeSet<Span> {
    let mut t = Targets {
        sys,
        found: BTreeSet::new(),
    };
    for s in body {
        t.visit_stmt(s);
    }
    t.found
}

#[test]
fn static_polarity_is_pyrefly_recursive_pruning() {
    let (_dir, out) = common::run("lexical_shapes");
    let sys = SysInfo::new(PythonVersion::new(3, 14, 0), PythonPlatform::new("linux"));
    let files = out.table("source_files").unwrap();
    let paths: BTreeMap<String, String> = common::column(files, "module_node_id")
        .into_iter()
        .zip(common::column(files, "path"))
        .collect();
    let bindings = out.table("bindings").unwrap();
    let mut marks: BTreeMap<(String, Span), String> = BTreeMap::new();
    for r in 0..bindings.num_rows() {
        let path = paths[&common::cell(bindings, "module_node_id", r)].clone();
        let span = (
            common::cell(bindings, "start_byte", r).parse().unwrap(),
            common::cell(bindings, "end_byte", r).parse().unwrap(),
        );
        marks.insert((path, span), common::cell(bindings, "static_polarity", r));
    }
    let mut checked = 0;
    for path in paths.values() {
        let text = std::fs::read_to_string(common::fixture("lexical_shapes").join(path)).unwrap();
        let (ast, errors, _) = Ast::parse(&text, PySourceType::Python);
        assert!(errors.is_empty(), "{path}");
        let analyzed = targets(Some(&sys), &ast.body);
        for span in targets(None, &ast.body) {
            let Some(polarity) = marks.get(&(path.clone(), span)) else {
                continue;
            };
            let pruned = polarity == "false";
            assert_eq!(
                pruned,
                !analyzed.contains(&span),
                "{path} {:?}: marked {polarity:?}",
                &text[span.0 as usize..span.1 as usize]
            );
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} assignment bindings checked");
}
