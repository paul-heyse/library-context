//! The symbol producer in the `pyrefly` stage (cutover plan A9): Pysa's definitions of each
//! analyzed module as symbols, traits, ancestry, signatures and displayed annotations, linked to
//! their declarations at exact name spans, in a generation the model validates. Answers are
//! written from the fixture sources.
#[path = "typed_driver/mod.rs"] mod typed_driver;
use std::collections::{BTreeMap, BTreeSet};
use lctx_model::domain::{*, attribution::*, calls::*, declarations::*, source::*, symbols::*, syntax::SubjectBoundary};
use typed_driver::{files, rows, run};

inspector!(Symbols, SourceArtifact, Module, Occurrence, ProviderModule, ProviderSymbol, SymbolObservation, FunctionTraitObservation, ClassTraitObservation,
    ClassAncestryObservation, SymbolSequenceMember, Signature, SignatureParameter, ParameterShape, ParameterAnnotationObservation, SymbolDeclaration,
    ParameterDeclaration, ProviderCoverage, SubjectBoundary);

struct Facts { tables: typed_driver::Tables, files: BTreeMap<String, Vec<u8>> }
impl Facts {
    async fn of(fixture: &str) -> Self {
        let tables = typed_driver::Tables::default();
        let files = files(fixture);
        run(&files, Symbols(tables.clone())).await.unwrap();
        Self { tables, files }
    }
    fn rows<R: Record>(&self) -> Vec<R> { rows::<R>(&self.tables) }
    fn symbol(&self, id: Id<ProviderSymbol>) -> ProviderSymbol { self.rows::<ProviderSymbol>().into_iter().find(|s| s.id() == id).unwrap() }
    /// The artifact path of an acquired provider module, or the module's spelling otherwise.
    fn module(&self, id: Id<ProviderModule>) -> String {
        match self.rows::<ProviderModule>().into_iter().find(|m| m.id() == id).unwrap() {
            ProviderModule::Acquired { module } => {
                let module = self.rows::<Module>().into_iter().find(|m| m.id() == module).unwrap();
                self.rows::<SourceArtifact>().into_iter().find(|a| a.id() == module.source).unwrap().path
            }
            ProviderModule::Bundled { name, bundle, .. } => format!("{bundle:?}:{name}"),
            ProviderModule::Namespace { name, .. } | ProviderModule::Unresolved { name, .. } => name,
        }
    }
    /// A symbol's dotted path through its observed parents, prefixed by its file.
    fn path(&self, id: Id<ProviderSymbol>) -> String {
        let parents: BTreeMap<_, _> = self.rows::<SymbolObservation>().into_iter().map(|o| (o.symbol, o.parent)).collect();
        let mut names = vec![]; let mut current = Some(id);
        while let Some(symbol) = current { names.push(self.symbol(symbol).name); current = parents.get(&symbol).copied().flatten(); }
        names.reverse();
        format!("{}:{}", self.module(self.symbol(id).module), names.join("."))
    }
    fn find(&self, path: &str) -> Id<ProviderSymbol> {
        let found: Vec<_> = self.rows::<SymbolObservation>().into_iter().map(|o| o.symbol).filter(|s| self.path(*s) == path).collect();
        assert_eq!(found.len(), 1, "one symbol at {path}"); found[0]
    }
    fn text(&self, id: Id<Occurrence>) -> String {
        let o = self.rows::<Occurrence>().into_iter().find(|o| o.id() == id).unwrap();
        let path = self.rows::<SourceArtifact>().into_iter().find(|a| a.id() == o.source).unwrap().path;
        String::from_utf8(self.files[&path][o.start as usize..o.end as usize].to_vec()).unwrap()
    }
    fn traits(&self, path: &str) -> FunctionTraitObservation { self.rows::<FunctionTraitObservation>().into_iter().find(|t| t.symbol == self.find(path)).unwrap() }
    fn ancestry(&self, path: &str, relation: AncestryRelation) -> (Vec<String>, Option<Linearization>) {
        let row = self.rows::<ClassAncestryObservation>().into_iter().find(|a| a.class == self.find(path) && a.relation == relation).unwrap();
        let mut members: Vec<_> = self.rows::<SymbolSequenceMember>().into_iter().filter(|m| m.sequence == row.ancestors).collect();
        members.sort_by_key(|m| m.ordinal);
        (members.into_iter().map(|m| format!("{}:{}", self.module(self.symbol(m.symbol).module), self.symbol(m.symbol).name)).collect(), row.linearization)
    }
}

#[tokio::test]
async fn definitions_nest_carry_their_traits_and_attach_at_their_name_spans() {
    let f = Facts::of("semantic_symbols").await;
    let observed: BTreeSet<String> = f.rows::<SymbolObservation>().into_iter().map(|o| f.path(o.symbol)).collect();
    let expected = ["Base", "Base.run", "Service", "Service.make", "Service.run", "Service.run.inner"].map(|s| format!("example.py:{s}"));
    assert_eq!(observed, BTreeSet::from(expected));
    // Traits: a docstring-only body outside an interface-like context is no stub; the static
    // factory; the override; the nested `...` function is a stub.
    let run = f.traits("example.py:Base.run");
    assert!(!run.stub && run.def_statement && run.defining_class == Some(f.find("example.py:Base")) && run.overrides.is_none());
    let make = f.traits("example.py:Service.make");
    assert!(make.staticmethod && !make.stub && make.defining_class == Some(f.find("example.py:Service")));
    assert_eq!(f.traits("example.py:Service.run").overrides, Some(f.find("example.py:Base.run")));
    let inner = f.traits("example.py:Service.run.inner");
    assert!(inner.stub && inner.defining_class.is_none());
    assert_eq!(f.symbol(f.find("example.py:Service.run.inner")).kind, SymbolKind::Function);
    assert_eq!(f.symbol(f.find("example.py:Service.run")).kind, SymbolKind::Method);
    // Ancestry: Base has no bases and a complete empty MRO; Service's MRO excludes itself and object.
    assert_eq!(f.ancestry("example.py:Base", AncestryRelation::Bases), (vec![], None));
    assert_eq!(f.ancestry("example.py:Base", AncestryRelation::Mro), (vec![], Some(Linearization::Complete)));
    // Pyrefly's stated model leaves `Generic` out of inheritance (class_metadata.rs: such a base
    // "does not participate in inheritance related computation"), though the runtime MRO holds it.
    let (mro, linearization) = f.ancestry("example.py:Service", AncestryRelation::Mro);
    assert_eq!(linearization, Some(Linearization::Complete));
    assert_eq!(mro, vec!["example.py:Base".to_owned()]);
    assert_eq!(f.ancestry("example.py:Service", AncestryRelation::Bases), (vec!["example.py:Base".to_owned()], None));
    // Every definition attaches to its own statement.
    let declarations: BTreeMap<String, String> = f.rows::<SymbolDeclaration>().into_iter()
        .map(|d| (f.path(d.symbol), f.text(d.declaration).lines().next().unwrap().trim().to_owned())).collect();
    assert_eq!(declarations["example.py:Base"], "class Base:");
    assert_eq!(declarations["example.py:Service.run"], "def run(self, timeout: float = 1.0) -> None:");
    assert_eq!(declarations["example.py:Service.run.inner"], "def inner() -> None: ...");
    assert_eq!(declarations.len(), 6);
    // Base.run's signature parameters link to its formals; the annotation is displayed.
    let signature = f.rows::<Signature>().into_iter().find(|s| s.symbol == f.find("example.py:Base.run")).unwrap();
    let members: Vec<_> = f.rows::<SignatureParameter>().into_iter().filter(|m| m.signature == signature.id()).collect();
    let formals: BTreeMap<i64, String> = f.rows::<ParameterDeclaration>().into_iter().filter_map(|d| members.iter().find(|m| m.id() == d.parameter)
        .map(|m| (m.ordinal, f.text(d.declaration)))).collect();
    assert_eq!(formals, BTreeMap::from([(0, "self".to_owned()), (1, "timeout: float".to_owned())]));
    let displays: Vec<String> = f.rows::<ParameterAnnotationObservation>().into_iter().filter(|a| members.iter().any(|m| m.id() == a.parameter && m.ordinal == 1))
        .map(|a| a.display).collect();
    assert_eq!(displays, vec!["float".to_owned()]);
    // Nothing failed to attach: the module's signatures are complete.
    assert!(f.rows::<SubjectBoundary>().iter().all(|b| b.family != FactFamily::Signatures));
    let signatures: Vec<_> = f.rows::<ProviderCoverage>().into_iter().filter(|c| c.family == FactFamily::Signatures).map(|c| c.status).collect();
    assert_eq!(signatures, vec![CoverageStatus::CompleteUnderStatedModel]);
}

#[tokio::test]
async fn keys_are_per_file_and_nested_classes_stay_apart() {
    let f = Facts::of("pysa_keys").await;
    // A `.py` and its `.pyi` share a module name and each defines its own `F:0`.
    for path in ["keys/dual.py", "keys/dual.pyi"] {
        let f0 = f.find(&format!("{path}:f"));
        assert_eq!(f.symbol(f0).native_key, "F:0");
    }
    assert_ne!(f.find("keys/dual.py:f"), f.find("keys/dual.pyi:f"));
    // The two nested `Config` classes and their `m` methods are four symbols.
    let (a, b) = (f.find("keys/nested.py:A.Config"), f.find("keys/nested.py:B.Config"));
    assert_ne!(a, b);
    assert_eq!(f.traits("keys/nested.py:A.Config.m").defining_class, Some(a));
    assert_eq!(f.traits("keys/nested.py:B.Config.m").defining_class, Some(b));
    // `use(a: A.Config, b: B.Config)` displays both annotations.
    let signature = f.rows::<Signature>().into_iter().find(|s| s.symbol == f.find("keys/nested.py:use")).unwrap();
    let members: Vec<_> = f.rows::<SignatureParameter>().into_iter().filter(|m| m.signature == signature.id()).map(|m| m.id()).collect();
    let displays: BTreeSet<String> = f.rows::<ParameterAnnotationObservation>().into_iter().filter(|a| members.contains(&a.parameter)).map(|a| a.display).collect();
    assert_eq!(displays.len(), 2, "{displays:?}");
    assert!(f.rows::<SubjectBoundary>().iter().all(|b| b.family != FactFamily::Signatures), "every definition attaches");
}
