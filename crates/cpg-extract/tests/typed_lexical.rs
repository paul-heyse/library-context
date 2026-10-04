//! The lexical recognizer in the `pyrefly` stage (cutover plan A5): `lexical_shapes` is acquired
//! as a tree and run through `acquire`, `pyrefly` and `assemble` into a stage-bound memory
//! generation the model validates. Static polarity is checked against Pyrefly's own recursive
//! pruning, and each scoping rule against a known answer written from the fixture's comments.
use lctx_model::domain::{
    assertion::*, attribution::*, lexical::*, obligation::ObligationKind, source::*, *,
};
use pyrefly_python::{
    ast::Ast,
    sys_info::{PythonPlatform, PythonVersion, SysInfo},
};
use ruff_python_ast::{
    Expr, PySourceType, Stmt,
    visitor::source_order::{SourceOrderVisitor, walk_stmt},
};
use ruff_text_size::Ranged;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Clone)]
struct Facts {
    files: BTreeMap<String, Vec<u8>>,
    artifacts: Vec<SourceArtifact>,
    occurrences: Vec<Occurrence>,
    scopes: Vec<LexicalScope>,
    scope_observations: Vec<LexicalScopeObservation>,
    events: Vec<BindingEvent>,
    bindings: Vec<BindingObservation>,
    references: Vec<ReferenceObservation>,
    targets: Vec<LexicalTarget>,
    resolutions: Vec<LexicalResolution>,
    qualifications: Vec<AssertionQualification>,
    coverage: Vec<ProviderCoverage>,
}
/// A resolution as the answers state it: the target's binding kind and site line, a builtin, or
/// an unresolved reason; whether captured; whether a candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum To {
    Binding(i16, usize),
    Builtin(String),
    Unresolved(i16),
}
impl Facts {
    fn occurrence(&self, id: Id<Occurrence>) -> &Occurrence {
        self.occurrences.iter().find(|o| o.id() == id).unwrap()
    }
    fn path(&self, source: Id<SourceArtifact>) -> &str {
        &self
            .artifacts
            .iter()
            .find(|a| a.id() == source)
            .unwrap()
            .path
    }
    fn text(&self, id: Id<Occurrence>) -> String {
        let o = self.occurrence(id);
        String::from_utf8(
            self.files[self.path(o.source)][o.start as usize..o.end as usize].to_vec(),
        )
        .unwrap()
    }
    fn line(&self, id: Id<Occurrence>) -> usize {
        let o = self.occurrence(id);
        self.files[self.path(o.source)][..o.start as usize]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1
    }
    /// How the read of `name` on `line` of `path` resolves, sorted, each with (captured, candidate).
    fn resolves(&self, path: &str, line: usize, name: &str) -> Vec<(To, bool, bool)> {
        let reads: Vec<Id<Occurrence>> = self
            .references
            .iter()
            .map(|r| r.read)
            .filter(|read| {
                self.path(self.occurrence(*read).source) == path
                    && self.line(*read) == line
                    && self.text(*read) == name
            })
            .collect();
        assert_eq!(reads.len(), 1, "one read of {name} on {path}:{line}");
        let mut out: Vec<(To, bool, bool)> = self
            .resolutions
            .iter()
            .filter(|r| r.read == reads[0])
            .map(|r| {
                let to = match self.targets.iter().find(|t| t.id() == r.target).unwrap() {
                    LexicalTarget::Binding { event } => {
                        let binding = self.bindings.iter().find(|b| b.event == *event).unwrap();
                        To::Binding(
                            binding.kind.code(),
                            self.line(self.events.iter().find(|e| e.id() == *event).unwrap().site),
                        )
                    }
                    LexicalTarget::Builtin { name, .. } => To::Builtin(name.clone()),
                    LexicalTarget::Unresolved { reason } => To::Unresolved(reason.code()),
                };
                let candidate = self
                    .qualifications
                    .iter()
                    .find(|q| q.id() == r.qualification)
                    .unwrap()
                    .modality
                    == Modality::Candidate;
                (to, r.captured, candidate)
            })
            .collect();
        out.sort();
        out
    }
}
fn b(kind: BindingEventKind, line: usize) -> To {
    To::Binding(kind.code(), line)
}

#[path = "typed_driver/mod.rs"]
mod typed_driver;
inspector!(Inspect, SourceArtifact, Occurrence);
async fn run(case: &str) -> Facts {
    let files = typed_driver::files(case);
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files, Inspect(tables.clone()))
        .await
        .unwrap();
    Facts {
        files,
        artifacts: typed_driver::rows(&tables),
        occurrences: typed_driver::rows(&tables),
        scopes: typed_driver::rows(&tables),
        scope_observations: typed_driver::rows(&tables),
        events: typed_driver::rows(&tables),
        bindings: typed_driver::rows(&tables),
        references: typed_driver::rows(&tables),
        targets: typed_driver::rows(&tables),
        resolutions: typed_driver::rows(&tables),
        qualifications: typed_driver::rows(&tables),
        coverage: typed_driver::rows(&tables),
    }
}

/// Every assignment-target name span; with `sys`, only in the clauses Pyrefly's binding pass keeps.
struct Targets<'a> {
    sys: Option<&'a SysInfo>,
    found: BTreeSet<(i64, i64)>,
}
impl<'b> SourceOrderVisitor<'b> for Targets<'_> {
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
                        self.found.insert((
                            i64::from(n.range().start().to_u32()),
                            i64::from(n.range().end().to_u32()),
                        ));
                    }
                }
                walk_stmt(self, stmt);
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

/// H1 review F1: every assignment binding's static polarity is Pyrefly's own recursive pruning.
#[tokio::test]
async fn static_polarity_is_pyrefly_recursive_pruning() {
    let f = run("lexical_shapes").await;
    let sys = SysInfo::new(PythonVersion::new(3, 14, 7), PythonPlatform::new("linux"));
    let mut checked = 0;
    for (path, bytes) in &f.files {
        let text = std::str::from_utf8(bytes).unwrap();
        let (ast, errors, _) = Ast::parse(text, PySourceType::Python);
        assert!(errors.is_empty(), "{path}");
        let targets = |sys| {
            let mut t = Targets {
                sys,
                found: BTreeSet::new(),
            };
            for s in &ast.body {
                t.visit_stmt(s);
            }
            t.found
        };
        let analyzed = targets(Some(&sys));
        for span in targets(None) {
            let Some(binding) = f.bindings.iter().find(|b| {
                let site = f.occurrence(f.events.iter().find(|e| e.id() == b.event).unwrap().site);
                f.path(site.source) == path
                    && (site.start, site.end) == span
                    && b.kind == BindingEventKind::Assignment
            }) else {
                continue;
            };
            assert_eq!(
                binding.static_polarity == Some(false),
                !analyzed.contains(&span),
                "{path} {:?}",
                &text[span.0 as usize..span.1 as usize]
            );
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} assignment bindings checked");
}

#[tokio::test]
async fn each_scoping_rule_resolves_as_python_defines_it() {
    use BindingEventKind as K;
    let f = run("lexical_shapes").await;
    let init = "lex/__init__.py";
    // Closure capture and `nonlocal`.
    assert_eq!(
        f.resolves(init, 29, "scale"),
        [(b(K::Parameter, 24), true, false)]
    );
    assert_eq!(
        f.resolves(init, 30, "total"),
        [
            (b(K::Assignment, 25), true, true),
            (b(K::AugAssignment, 29), true, true)
        ]
    );
    // A method cannot see its class's names; a comprehension element skips the class too.
    assert_eq!(
        f.resolves(init, 40, "name_or_default"),
        [(b(K::FunctionDef, 43), false, false)]
    );
    assert_eq!(
        f.resolves(init, 37, "name"),
        [(
            To::Unresolved(ObligationKind::UnresolvedTarget.code()),
            false,
            false
        )]
    );
    assert_eq!(
        f.resolves(init, 37, "range"),
        [(To::Builtin("range".into()), false, false)],
        "the first iterable sees the class's scope and builtins"
    );
    // A walrus binds in the enclosing function; a lambda reads the module's two bindings.
    assert_eq!(
        f.resolves(init, 49, "y"),
        [(b(K::Walrus, 48), false, false)]
    );
    assert_eq!(
        f.resolves(init, 49, "limit"),
        [
            (b(K::Assignment, 14), false, true),
            (b(K::Assignment, 15), false, true)
        ]
    );
    // A local shadows the builtin; `max` is the builtin; `os` is the import's binding.
    assert_eq!(
        f.resolves(init, 54, "len"),
        [(b(K::Assignment, 53), false, false)]
    );
    assert_eq!(
        f.resolves(init, 54, "max"),
        [(To::Builtin("max".into()), false, false)]
    );
    assert_eq!(
        f.resolves(init, 54, "os"),
        [(b(K::Import, 3), false, false)]
    );
    // The star import's wildcard set binds both, over the builtin `open`.
    assert_eq!(
        f.resolves(init, 58, "helper_from_star"),
        [(b(K::StarImport, 6), false, false)]
    );
    assert_eq!(
        f.resolves(init, 58, "open"),
        [(b(K::StarImport, 6), false, false)]
    );
    assert!(f.bindings.iter().any(|b| b.kind == K::Del
        && f.text(f.events.iter().find(|e| e.id() == b.event).unwrap().site) == "tmp"));
    let flow = "lex/flow.py";
    assert_eq!(
        f.resolves(flow, 5, "__name__"),
        [(b(K::Implicit, 1), false, false)],
        "an implicit global of the module"
    );
    assert_eq!(
        f.resolves(flow, 7, "len"),
        [
            (b(K::Assignment, 8), false, true),
            (To::Builtin("len".into()), false, true)
        ],
        "read before the module binds it"
    );
    assert_eq!(
        f.resolves(flow, 12, "type"),
        [
            (b(K::Assignment, 13), false, true),
            (To::Builtin("type".into()), false, true)
        ]
    );
    assert_eq!(
        f.resolves(flow, 17, "__class__"),
        [(b(K::Implicit, 11), true, false)],
        "the method's class cell"
    );
    assert_eq!(
        f.resolves(flow, 21, "Callable"),
        [(
            To::Unresolved(ObligationKind::UnresolvedTarget.code()),
            false,
            false
        )]
    );
    assert_eq!(
        f.resolves(flow, 30, "json_mod"),
        [(b(K::Import, 26), false, false)],
        "bound at module scope under `global`"
    );
    assert_eq!(
        f.resolves(flow, 39, "v"),
        [
            (b(K::Assignment, 36), false, true),
            (b(K::Assignment, 38), false, true)
        ],
        "the nonlocal target is the enclosing scope"
    );
    assert_eq!(
        f.resolves(flow, 47, "v"),
        [(b(K::Parameter, 46), true, false)],
        "a decorator's lambda evaluates in the enclosing scope"
    );
    assert_eq!(
        f.bindings
            .iter()
            .filter(|b| b.kind == K::Global
                && f.text(f.events.iter().find(|e| e.id() == b.event).unwrap().site)
                    .contains("g1"))
            .count(),
        1,
        "`global g1, g1` is one event"
    );
    // Nothing opens inside an annotation: the only lambdas with scopes are outside them.
    let annotation_lambda = f
        .occurrences
        .iter()
        .find(|o| {
            o.syntax_kind == SyntaxKind::ExprLambda
                && f.path(o.source) == flow
                && f.line(o.id()) == 14
        })
        .unwrap();
    assert!(f.scopes.iter().all(|s| s.owner != annotation_lambda.id()));
    // Every scope has its parent where its position evaluates; only modules have none.
    assert!(f.scope_observations.iter().all(|o| o.parent.is_none()
        == (f.scopes.iter().find(|s| s.id() == o.scope).unwrap().kind
            == LexicalScopeKind::Module)));
    assert!(
        f.coverage
            .iter()
            .filter(|c| c.family == FactFamily::Lexical)
            .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel),
        "{:?}",
        f.coverage
    );
}
