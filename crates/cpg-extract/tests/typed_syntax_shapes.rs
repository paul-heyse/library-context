//! The `pyrefly` stage's typed syntax (cutover plan A4) over the pinned parse: fixtures are
//! acquired as trees and run through `acquire`, `pyrefly` and `assemble` into a stage-bound memory
//! generation the model validates. Every known answer is written from the fixture's source.
use std::{collections::BTreeMap, path::Path, sync::{Arc, Mutex}};
use cpg_extract::{acquisition::{AcquiredInput, Acquire}, assembly::Assemble, bundle::{CapturedInputs, Declared, ProviderStage, StageContext, run_stage},
    capture::CapturedInput, pyrefly_stage::Pyrefly, typed_syntax::SyntaxLimits};
use lctx_model::domain::{*, attribution::*, batching::TransferLimits, calls::*, memory::MemoryGeneration, obligation::ObligationKind, resources::ResourceBudget,
    source::*, stages::*, syntax::*, value::*};

fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 30).unwrap() }

/// Everything the stages wrote, and the captured bytes to read texts from.
#[derive(Default, Clone)]
struct Facts {
    files: BTreeMap<String, Vec<u8>>, artifacts: Vec<SourceArtifact>, modules: Vec<Module>, occurrences: Vec<Occurrence>,
    placements: Vec<SyntaxPlacement>, details: Vec<SyntaxDetailObservation>, syntax_details: Vec<SyntaxDetail>, declarations: Vec<DeclarationObservation>,
    decorators: Vec<DeclarationDecorator>, imports: Vec<ImportAliasObservation>, dunder_all: Vec<DunderAllObservation>, parameters: Vec<ParameterSyntaxObservation>,
    fields: Vec<ClassFieldSyntaxObservation>, calls: Vec<CallSyntax>, arguments: Vec<CallArgument>, literals: Vec<Literal>, members: Vec<LiteralSetMember>,
    boundaries: Vec<SubjectBoundary>, coverage: Vec<ProviderCoverage>,
}
impl Facts {
    fn occurrence(&self, id: Id<Occurrence>) -> &Occurrence { self.occurrences.iter().find(|o| o.id() == id).expect("a stated occurrence") }
    fn path(&self, source: Id<SourceArtifact>) -> &str { &self.artifacts.iter().find(|a| a.id() == source).unwrap().path }
    /// The captured bytes at an occurrence, as text.
    fn text(&self, id: Id<Occurrence>) -> String {
        let o = self.occurrence(id);
        String::from_utf8(self.files[self.path(o.source)][o.start as usize..o.end as usize].to_vec()).unwrap()
    }
    fn in_file<'a, T>(&'a self, rows: &'a [T], path: &'a str, subject: impl Fn(&T) -> Id<Occurrence> + 'a) -> impl Iterator<Item = &'a T> + 'a {
        rows.iter().filter(move |row| self.path(self.occurrence(subject(row)).source) == path)
    }
    /// Import aliases of one file, in source order.
    fn imports(&self, path: &str) -> Vec<(String, i64, Option<String>)> {
        let mut rows: Vec<&ImportAliasObservation> = self.in_file(&self.imports, path, |i| i.alias).collect();
        rows.sort_by_key(|i| self.occurrence(i.alias).start);
        rows.into_iter().map(|i| (self.text(i.alias), i.level, i.resolved_module.clone())).collect()
    }
    fn names(&self, set: Id<LiteralSet>) -> Vec<String> {
        let mut names: Vec<String> = self.members.iter().filter(|m| m.set == set).map(|m| match self.literals.iter().find(|l| l.id() == m.value) {
            Some(Literal::String { value }) => value.clone(), other => panic!("not a string: {other:?}") }).collect();
        names.sort();
        names
    }
    fn literal(&self, id: Id<Literal>) -> &Literal { self.literals.iter().find(|l| l.id() == id).unwrap() }
}

/// Reads every syntax relation, and the assembled vocabulary, through its handoff.
struct Inspect(Arc<Mutex<Facts>>);
fn inspect_stage() -> Stage {
    macro_rules! uses { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    Stage { name: "inspect", inputs: uses!(SourceArtifact, Module, Occurrence, SyntaxPlacement, SyntaxDetailObservation, SyntaxDetail, DeclarationObservation,
            DeclarationDecorator, ImportAliasObservation, DunderAllObservation, ParameterSyntaxObservation, ClassFieldSyntaxObservation, CallSyntax, CallArgument,
            Literal, LiteralSetMember, SubjectBoundary, ProviderCoverage),
        outputs: uses!(lctx_model::domain::types::TypePresentationSupport), contributes: vec![], coverage: vec![], provider: None, profiles: vec![Profile::Catalog, Profile::Behavioral],
        effect: Effect::Pure, code: ContentHash::of(b"inspect"), configuration: ContentHash::of(b"inspect") }
}
impl Declared for Inspect { fn declaration(&self, _: Profile) -> Stage { inspect_stage() } }
impl ProviderStage<MemoryGeneration> for Inspect {
    fn run(&mut self, context: &mut StageContext<MemoryGeneration>) -> Result<ProviderOutcome, ModelError> {
        fn all<R: Record>(context: &mut StageContext<MemoryGeneration>) -> Result<Vec<R>, ModelError> {
            Ok(context.handoff::<R>()?.iter().flat_map(|b| b.rows().to_vec()).collect())
        }
        let mut f = self.0.lock().unwrap();
        f.artifacts = all(context)?; f.modules = all(context)?; f.occurrences = all(context)?; f.placements = all(context)?; f.details = all(context)?;
        f.syntax_details = all(context)?; f.declarations = all(context)?; f.decorators = all(context)?; f.imports = all(context)?; f.dunder_all = all(context)?;
        f.parameters = all(context)?; f.fields = all(context)?; f.calls = all(context)?; f.arguments = all(context)?; f.literals = all(context)?;
        f.members = all(context)?; f.boundaries = all(context)?; f.coverage = all(context)?;
        context.declare::<lctx_model::domain::types::TypePresentationSupport>()?;
        Ok(ProviderOutcome::Complete)
    }
}

/// Acquire `files` as a tree and run the syntax stages; the model validates everything written.
async fn run(files: BTreeMap<String, Vec<u8>>, limits: SyntaxLimits) -> Facts {
    let tree = tempfile::tempdir().unwrap();
    for (path, bytes) in &files {
        let target = tree.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let captured = CapturedInput::capture(tree.path(), &files.keys().cloned().collect::<Vec<_>>(), &budget()).unwrap();
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(captured, "syntax shapes")]));
    let model = Arc::new(model().unwrap());
    let facts = Arc::new(Mutex::new(Facts::default()));
    let providers: Vec<Box<dyn ProviderStage<MemoryGeneration>>> = vec![Box::new(Acquire::new(ContentHash::of(b"shapes"))), Box::new(Pyrefly::new(limits)),
        Box::new(Assemble), Box::new(Inspect(facts.clone()))];
    let schedule = Schedule::build(&model, providers.iter().map(|p| p.declaration(Profile::Catalog)).collect(), &[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute();
    let budget = budget();
    let generation = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let mut providers: BTreeMap<&str, Box<dyn ProviderStage<MemoryGeneration>>> = providers.into_iter().map(|p| (p.declaration(Profile::Catalog).name, p)).collect();
    for stage in schedule.stages() {
        let provider = providers.remove(stage.name).unwrap();
        run_stage(provider, execution.begin(stage.name).unwrap(), &generation, &model, &captured, &budget, TransferLimits::default()).await
            .unwrap_or_else(|e| panic!("{}: {e}", stage.name));
    }
    execution.finish().unwrap();
    generation.validate(&model, &budget).unwrap_or_else(|e| panic!("the model refuses the syntax facts: {e}"));
    let mut facts = facts.lock().unwrap().clone();
    facts.files = files;
    facts
}
fn fixture(case: &str) -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python").join(case);
    walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().map(|e| e.unwrap()).filter(|e| e.file_type().is_file())
        .map(|e| (e.path().strip_prefix(&root).unwrap().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap())).collect()
}

#[tokio::test]
async fn syntax_shapes_states_declarations_parameters_calls_and_operators() {
    let f = run(fixture("syntax_shapes"), SyntaxLimits::default()).await;
    let path = "syn/__init__.py";
    assert_eq!(f.modules.iter().map(|m| m.qualified_name.as_str()).collect::<Vec<_>>(), ["syn"]);
    let mut declared: Vec<(String, DeclarationKind)> = f.declarations.iter().map(|d| (f.text(d.name), d.kind)).collect();
    declared.sort_by_key(|d| f.declarations.iter().find(|x| f.text(x.name) == d.0).map(|x| f.occurrence(x.declaration).start));
    use DeclarationKind::*;
    assert_eq!(declared, [("ConfigError".into(), Class), ("guarded".into(), Function), ("handled".into(), Function), ("matched".into(), Function),
        ("looped".into(), Function), ("decorated".into(), Function)]);
    assert!(f.declarations.iter().all(|d| d.parent.is_none() && d.docstring.is_none() && !d.overload));
    assert_eq!(f.decorators.iter().map(|d| f.text(d.decorator)).collect::<Vec<_>>(), ["@functools.lru_cache(maxsize=8)"]);
    assert_eq!(f.imports.iter().map(|i| (f.text(i.alias), i.level, i.resolved_module.clone())).collect::<Vec<_>>(), [("functools".into(), 0, Some("functools".into()))]);
    let guarded = f.declarations.iter().find(|d| f.text(d.name) == "guarded").unwrap().declaration;
    let mut parameters: Vec<_> = f.parameters.iter().filter(|p| p.function == guarded).collect();
    parameters.sort_by_key(|p| p.ordinal);
    let shown: Vec<(String, ParameterKind, Option<Literal>)> = parameters.iter().map(|p| (f.text(p.parameter), p.kind, p.default_literal.map(|l| f.literal(l).clone()))).collect();
    assert_eq!(shown, [("x".into(), ParameterKind::PositionalOrKeyword, None),
        ("mode=\"fast\"".into(), ParameterKind::PositionalOrKeyword, Some(Literal::String { value: "fast".into() })),
        ("limit=0".into(), ParameterKind::PositionalOrKeyword, Some(Literal::Integer { decimal: "0".into() }))]);
    let mut callees: Vec<String> = f.calls.iter().map(|c| f.text(c.callee)).collect();
    callees.sort();
    assert_eq!(callees, ["ConfigError", "ConfigError", "ValueError", "fh.read", "functools.lru_cache", "key.upper", "len", "len", "open", "open", "open(path).read"]);
    let cache = f.calls.iter().find(|c| f.text(c.callee) == "functools.lru_cache").unwrap();
    let arguments: Vec<_> = f.arguments.iter().filter(|a| a.call == cache.id()).map(|a| (a.kind, a.keyword.clone(), f.text(a.value))).collect();
    assert_eq!(arguments, [(ArgumentKind::Keyword, Some("maxsize".into()), "8".into())]);
    // `0 <= limit < 10` states two comparison operators in order.
    let chain = f.occurrences.iter().find(|o| o.syntax_kind == SyntaxKind::ExprCompare && f.text(o.id()) == "0 <= limit < 10").unwrap();
    let mut chained: Vec<_> = f.details.iter().filter(|d| d.occurrence == chain.id()).collect();
    chained.sort_by_key(|d| d.ordinal);
    let operator = |d: &&SyntaxDetailObservation| f.syntax_details.iter().find(|s| s.id() == d.detail).cloned().unwrap();
    assert_eq!(chained.iter().map(operator).collect::<Vec<_>>(), [SyntaxDetail::Operator { operator: OperatorKind::LtE }, SyntaxDetail::Operator { operator: OperatorKind::Lt }]);
    // Every occurrence is placed once, under its parse parent; only the module has none.
    assert_eq!(f.placements.len(), f.occurrences.len());
    let roots: Vec<_> = f.placements.iter().filter(|p| p.parent.is_none()).collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(f.occurrence(roots[0].occurrence).syntax_kind, SyntaxKind::ModModule);
    let raise = f.occurrences.iter().find(|o| o.syntax_kind == SyntaxKind::StmtRaise && f.text(o.id()).starts_with("raise ConfigError(\"cannot")).unwrap();
    let cause = f.placements.iter().find(|p| p.parent == Some(raise.id()) && p.field == lctx_model::domain::lexical::SyntaxField::Cause).unwrap();
    assert_eq!(f.text(cause.occurrence), "err");
    let syntax = f.coverage.iter().find(|c| c.family == FactFamily::Syntax).unwrap();
    assert_eq!((syntax.status, f.coverage.len()), (CoverageStatus::CompleteUnderStatedModel, 4), "Syntax, Lexical, Exports and Signatures");
    assert!(f.coverage.iter().filter(|c| matches!(c.family, FactFamily::Exports | FactFamily::Signatures))
        .all(|c| c.status == CoverageStatus::Partial && c.reason == Some(ObligationKind::OutsideProviderModel)));
    let _ = path;
}

#[tokio::test]
async fn unicode_names_a_bom_and_crlf_keep_byte_spans() {
    let f = run(fixture("unicode_bom"), SyntaxLimits::default()).await;
    let mut modules: Vec<&str> = f.modules.iter().map(|m| m.qualified_name.as_str()).collect();
    modules.sort();
    assert_eq!(modules, ["lcfix", "lcfix.core"]);
    let core = "lcfix/core.py";
    let names: Vec<String> = f.in_file(&f.declarations, core, |d| d.declaration).map(|d| f.text(d.name)).collect();
    for expected in ["Base", "méthode", "Écrivain", "__new__", "__init__", "étiquette", "build", "λ_helper", "utilise"] {
        assert!(names.iter().any(|n| n == expected), "{expected} in {names:?}");
    }
    let builds: Vec<bool> = f.in_file(&f.declarations, core, |d| d.declaration).filter(|d| f.text(d.name) == "build").map(|d| d.overload).collect();
    assert_eq!((builds.len(), builds.iter().filter(|o| **o).count()), (3, 2), "two @overload signatures and the implementation");
    let ecrivain = f.declarations.iter().find(|d| f.text(d.name) == "Écrivain").unwrap();
    assert_eq!(f.text(ecrivain.docstring.unwrap()), "\"\"\"Écrit des données — “quoted”.\"\"\"");
    let methods: Vec<_> = f.declarations.iter().filter(|d| d.parent == Some(ecrivain.declaration)).map(|d| f.text(d.name)).collect();
    assert_eq!(methods.len(), 4, "{methods:?}");
    let property = f.declarations.iter().find(|d| f.text(d.name) == "étiquette").unwrap();
    assert_eq!(f.decorators.iter().filter(|d| d.declaration == property.declaration).map(|d| f.text(d.decorator)).collect::<Vec<_>>(), ["@property"]);
    let new = f.declarations.iter().find(|d| f.text(d.name) == "__new__").unwrap();
    let mut kinds: Vec<_> = f.parameters.iter().filter(|p| p.function == new.declaration).map(|p| (p.ordinal, p.kind, f.text(p.parameter))).collect();
    kinds.sort_by_key(|k| k.0);
    // Ruff's variadic parameter node spans its star.
    assert_eq!(kinds, [(0, ParameterKind::PositionalOrKeyword, "cls".into()), (1, ParameterKind::VarPositional, "*args: object".into()),
        (2, ParameterKind::VarKeyword, "**kwargs: object".into())]);
    let init = f.declarations.iter().find(|d| f.text(d.name) == "__init__").unwrap();
    let nom = f.parameters.iter().find(|p| p.function == init.declaration && p.ordinal == 1).unwrap();
    assert_eq!(f.literal(nom.default_literal.unwrap()), &Literal::String { value: "défaut".into() });
    let lambda = f.parameters.iter().find(|p| f.occurrence(p.function).syntax_kind == SyntaxKind::ExprLambda).unwrap();
    assert_eq!(f.text(lambda.parameter), "z");
    // The package's BOM and CRLF lines: spans are byte offsets in the captured bytes.
    let init_py = "lcfix/__init__.py";
    let imports = f.imports(init_py);
    assert_eq!(imports, [("Écrivain".into(), 0, Some("lcfix.core".into())), ("build".into(), 0, Some("lcfix.core".into())),
        ("λ_helper as helper".into(), 0, Some("lcfix.core".into())), ("core as core".into(), 0, Some("lcfix".into()))]);
    let all = f.in_file(&f.dunder_all, init_py, |d| d.statement).next().unwrap();
    assert_eq!((f.text(all.statement).as_str(), f.names(all.names.unwrap())), ("__all__ = [\"Écrivain\", \"build\", \"helper\"]", vec!["build".into(), "helper".into(), "Écrivain".into()]));
}

#[tokio::test]
async fn literal_all_is_stated_and_computed_all_is_a_boundary() {
    let f = run(fixture("dunder_all"), SyntaxLimits::default()).await;
    let lit: Vec<Vec<String>> = f.in_file(&f.dunder_all, "dunder/lit.py", |d| d.statement).map(|d| f.names(d.names.unwrap())).collect();
    let mut lit = lit; lit.sort();
    assert_eq!(lit, [vec!["also".to_owned()], vec!["exported".to_owned()], vec!["third".to_owned()]], "assign, += and .append state literal names");
    assert_eq!(f.in_file(&f.dunder_all, "dunder/sub/__init__.py", |d| d.statement).map(|d| f.names(d.names.unwrap())).collect::<Vec<_>>(), [vec!["alpha".to_owned(), "beta".to_owned()]]);
    let mut computed: Vec<String> = f.dunder_all.iter().filter(|d| !d.literal).map(|d| f.text(d.statement)).collect();
    computed.sort();
    assert_eq!(computed, ["__all__ = _names()", "__all__ = sub.__all__ + [\"top\"]"]);
    let mut boundaries: Vec<(String, ObligationKind, FactFamily)> = f.boundaries.iter().map(|b| (f.text(b.subject.unwrap()), b.reason, b.family)).collect();
    boundaries.sort();
    assert_eq!(boundaries, [("__all__ = _names()".into(), ObligationKind::OutsideProviderModel, FactFamily::Exports),
        ("__all__ = sub.__all__ + [\"top\"]".into(), ObligationKind::OutsideProviderModel, FactFamily::Exports)]);
    let imports = f.imports("dunder/__init__.py");
    assert_eq!(imports, [("sub".into(), 1, Some("dunder".into())), ("*".into(), 1, Some("dunder.sub".into())), ("dyn_public".into(), 1, Some("dunder.dyn".into()))]);
}

#[tokio::test]
async fn with_items_are_distinct_occurrences_and_limits_disclose() {
    let source = b"with open(a) as x, open(b) as y:\n    pass\n".to_vec();
    let f = run(BTreeMap::from([("w.py".to_owned(), source.clone())]), SyntaxLimits::default()).await;
    let items: Vec<String> = f.occurrences.iter().filter(|o| o.syntax_kind == SyntaxKind::WithItem).map(|o| f.text(o.id())).collect();
    assert_eq!(items, ["open(a) as x", "open(b) as y"]);
    assert!(f.occurrences.iter().filter(|o| o.syntax_kind == SyntaxKind::WithItem).all(|o| o.role == OccurrenceRole::WithItem));
    // A traversal bound keeps the emitted prefix and discloses Partial Syntax coverage.
    let bounded = run(BTreeMap::from([("w.py".to_owned(), source)]), SyntaxLimits { nodes: 4, ..SyntaxLimits::default() }).await;
    assert_eq!(bounded.occurrences.len(), 4);
    let syntax = bounded.coverage.iter().find(|c| c.family == FactFamily::Syntax).unwrap();
    assert_eq!((syntax.status, syntax.reason), (CoverageStatus::Partial, Some(ObligationKind::ResourceRefused)));
    assert!(bounded.calls.is_empty(), "records are stated only from a complete traversal");
}
