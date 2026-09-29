use std::collections::{BTreeMap,BTreeSet};
use lctx_model::domain::{*, assertion::*, attribution::*, calls::*, conditions::*, input::*, source::*, transfer::*, value::*};

struct Fixture {
    model: ValidatedModel,input: InputRevision,source: SourceArtifact,scope: CoverageScope,
    context: AnalysisContext,provider: Provider,run: ProviderRun,families: Vec<RunFamily>,surface: ProviderSurface,
    symbol: ProviderSymbol,occurrences: Vec<Occurrence>,roots: Vec<PlaceRoot>,path: AccessPath,places: Vec<Place>,
    atom: EvaluationAtom,diagram: Diagram,qualification: AssertionQualification,key: TransferKey,
}
impl Fixture {
    fn new() -> Self {
        let input = InputRevision::from_entries(vec![ManifestEntry { path: "x.py".into(),content: ContentHash::of(b"x y"),byte_len: 3 }]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(),"x.py".into(),b"x y").unwrap();
        let scope = CoverageScope::Input { input: input.id() };
        let context = AnalysisContext { python_version: "3.14.7".into(),python_platform: "linux".into(),search_path: vec![],site_package_path: vec![],
            config_digest: ContentHash::of(b"config"),environment_digest: input.manifest,lock_digest: None };
        let provider = Provider { tool: "flow-fixture".into(),revision: "v1".into(),build_digest: ContentHash::of(b"build") };
        let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Flow]).unwrap();
        let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Flow,name: "flow".into() };
        let symbol = ProviderSymbol { provider: provider.id(),context: context.id(),module: ProviderModule::Bundled { provider: provider.id(),name: "x".into() }.id(),native_key: "f".into(),name: "f".into(),kind: SymbolKind::Function };
        let occurrences: Vec<_> = (0..3).map(|i| Occurrence { source: source.id(),start: i,end: i+1,syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,structural_path: vec![i as i32] }).collect();
        let roots: Vec<_> = occurrences.iter().map(|o| PlaceRoot::Occurrence { occurrence: o.id() }).collect();
        let path = AccessPath::empty(); let places: Vec<_> = roots.iter().map(|r| Place { root: r.id(),path: path.id() }).collect();
        let atom = EvaluationAtom { evaluation: occurrences[1].id(),context: context.id(),predicate: Predicate::Truthy.id(),operand: Some(places[1].id()) };
        let diagram = Diagram::from_atom(atom.id());
        let qualification = AssertionQualification { context: context.id(),scope: scope.id(),condition: diagram.id(),modality: Modality::Definite,approximation: Approximation::Exact };
        let key = TransferKey { owner: symbol.id(),input: places[0].id(),output: places[2].id(),context: context.id(),scope: scope.id(),
            modality: Modality::Definite,approximation: Approximation::Exact,kind: TransferKind::Identity,call_site: None,provenance: ProvenanceClass::FlowLocal };
        Self { model: model().unwrap(),input,source,scope,context,provider,run,families,surface,symbol,occurrences,roots,path,places,atom,diagram,qualification,key }
    }
    fn branch(&self) -> TransferBranch { TransferBranch::new(self.key.clone(),self.qualification.clone(),self.diagram.clone()).unwrap() }
}
#[test]
fn widening_conditions_keeps_stable_transfer_and_each_original_alternative() {
    let f = Fixture::new(); let yes = f.branch(); let no_diagram = f.diagram.not().unwrap();
    let no = TransferBranch::new(f.key.clone(),AssertionQualification { condition: no_diagram.id(),..f.qualification.clone() },no_diagram).unwrap();
    let ids = BTreeSet::from([yes.alternative().id(),no.alternative().id()]);
    let one = merge([yes.clone()]).unwrap(); let merged = merge([yes.clone(),no.clone()]).unwrap();
    assert_eq!(one[0].key.id(),merged[0].key.id()); assert_eq!(merged[0].key.id(),f.key.id());
    assert!(merged[0].condition.is_true()); assert_eq!(merged[0].alternatives,ids);
    assert!(!one[0].condition.is_true()); assert_eq!(merge([no,yes]).unwrap()[0].alternatives,ids);
    let mut approximate_key = f.key.clone(); approximate_key.approximation = Approximation::Over;
    let approximate = TransferBranch::new(approximate_key,AssertionQualification { approximation: Approximation::Over,..f.qualification.clone() },f.diagram.clone()).unwrap();
    assert_eq!(merge([f.branch(),approximate]).unwrap().len(),2,"approximation cannot silently strengthen exact flow");
    assert!(TransferBranch::new(f.key.clone(),f.qualification.clone(),Diagram::always()).is_err());
    assert_eq!(compose_kinds(TransferKind::Identity,TransferKind::Identity),TransferKind::Identity);
    assert_eq!(compose_kinds(TransferKind::Derived,TransferKind::Identity),TransferKind::Derived);
    let rows = Batch::new(&f.model,vec![f.branch().alternative()], &budget()).unwrap();
    assert_eq!(TransferAlternative::decode(rows.arrow()).unwrap(),rows.rows());
}
#[test]
fn influence_selects_a_guarded_alternative_without_becoming_a_value_transfer() {
    let f = Fixture::new(); let influence = ControlInfluence { qualification: f.qualification.id(),input: f.places[1].id(),atom: f.atom.id(),evaluation: f.atom.evaluation };
    let selection = f.branch().selection(&influence,&f.qualification).unwrap().unwrap();
    assert_eq!(selection.influence,influence.id()); assert_eq!(selection.atom,f.atom.id());
    assert_eq!(selection.transfer,f.key.id()); assert_eq!(selection.alternative,f.branch().alternative().id());
    let unconditional = TransferBranch::new(f.key.clone(),AssertionQualification { condition: Diagram::always().id(),..f.qualification.clone() },Diagram::always()).unwrap();
    assert!(unconditional.selection(&influence,&f.qualification).unwrap().is_none());
    let wrong_scope = CoverageScope::Artifact { artifact: f.source.id() };
    let q = AssertionQualification { scope: wrong_scope.id(),..f.qualification.clone() };
    let other = ControlInfluence { qualification: q.id(),..influence.clone() };
    assert!(f.branch().selection(&other,&q).is_err());
    let rows = Batch::new(&f.model,vec![selection], &budget()).unwrap(); assert_eq!(Selection::decode(rows.arrow()).unwrap(),rows.rows());
}
fn insert<R: Record>(f: &Fixture, data: &mut BTreeMap<&'static str,arrow_array::RecordBatch>,rows: Vec<R>) {
    data.insert(R::NAME,Batch::new(&f.model,rows, &budget()).unwrap().arrow().clone());
}
fn check(invariant: Invariant,data: &BTreeMap<&str,arrow_array::RecordBatch>) -> Result<(),ModelError> {
    let mut check = (invariant.create)(&budget());
    for input in invariant.inputs { if let Some(batch) = data.get(input.name()) { check.visit(input.name(),batch)?; } }
    check.finish()
}
fn records(f: &Fixture) -> BTreeMap<&'static str,arrow_array::RecordBatch> {
    let mut data = BTreeMap::new();
    macro_rules! one { ($($row:expr),+ $(,)?) => { $(insert(f,&mut data,vec![$row.clone()]);)+ }; }
    one!(f.input,f.source,f.scope,f.context,f.provider,f.run,f.surface,f.symbol,f.path,f.atom,f.qualification,f.key,Predicate::Truthy);
    insert(f,&mut data,f.families.clone()); insert(f,&mut data,f.occurrences.clone());
    insert(f,&mut data,f.roots.clone()); insert(f,&mut data,f.places.clone());
    let (condition,nodes) = f.diagram.records(); one!(condition); insert(f,&mut data,nodes);
    let alternative = f.branch().alternative(); let evidence = Evidence::Occurrence { occurrence: f.occurrences[0].id() };
    let support = TransferSupport { assertion: alternative.id(),run: f.run.id(),surface: f.surface.id(),evidence: evidence.id(),
        origin: Origin::DerivedAnalysis,mode: ExtractionMode::GraphAnalysis,fidelity: Fidelity::NormalizedStructural };
    one!(alternative,evidence,support); data
}
#[test]
fn transfer_support_validates_both_place_sources_and_keeps_ordinary_subject_inputs_local() {
    let f = Fixture::new(); let base = records(&f);
    check(TransferSupport::invariants().remove(0),&base).unwrap();
    assert!(!SyntaxSupport::invariants()[0].inputs.iter().any(|input| input.name() == TransferKey::NAME));
    assert!(TransferSupport::invariants()[0].inputs.iter().any(|input| input.name() == PlaceRoot::NAME));
    let foreign_input = InputRevision::from_entries(vec![]).unwrap();
    let foreign = SourceArtifact::from_bytes(foreign_input.id(),"foreign.py".into(),b"z").unwrap();
    for input_place in [true,false] {
        let mut data = base.clone();
        let occurrence = Occurrence { source: foreign.id(),start: 0,end: 1,structural_path: vec![],..f.occurrences[0].clone() };
        let root = PlaceRoot::Occurrence { occurrence: occurrence.id() }; let place = Place { root: root.id(),path: f.path.id() };
        let key = if input_place { TransferKey { input: place.id(),..f.key.clone() } } else { TransferKey { output: place.id(),..f.key.clone() } };
        let branch = TransferBranch::new(key.clone(),f.qualification.clone(),f.diagram.clone()).unwrap(); let alternative = branch.alternative();
        let evidence = Evidence::Occurrence { occurrence: f.occurrences[0].id() };
        let support = TransferSupport { assertion: alternative.id(),run: f.run.id(),surface: f.surface.id(),evidence: evidence.id(),
            origin: Origin::DerivedAnalysis,mode: ExtractionMode::GraphAnalysis,fidelity: Fidelity::NormalizedStructural };
        insert(&f,&mut data,vec![f.source.clone(),foreign.clone()]);
        let mut occurrences = f.occurrences.clone(); occurrences.push(occurrence); insert(&f,&mut data,occurrences);
        let mut roots = f.roots.clone(); roots.push(root); insert(&f,&mut data,roots);
        let mut places = f.places.clone(); places.push(place); insert(&f,&mut data,places);
        insert(&f,&mut data,vec![key]); insert(&f,&mut data,vec![alternative]); insert(&f,&mut data,vec![support]);
        assert!(check(TransferSupport::invariants().remove(0),&data).is_err(),"foreign input/output place cannot enter scoped transfer");
    }
}
#[test]
fn stored_selection_checks_typed_premises_guard_membership_and_qualification() {
    let f = Fixture::new(); let mut base = records(&f);
    let influence = ControlInfluence { qualification: f.qualification.id(),input: f.places[1].id(),atom: f.atom.id(),evaluation: f.atom.evaluation };
    let selection = f.branch().selection(&influence,&f.qualification).unwrap().unwrap();
    insert(&f,&mut base,vec![influence]); insert(&f,&mut base,vec![selection.clone()]);
    check(TransferKey::invariants().remove(0),&base).unwrap();
    let wrong_atom = EvaluationAtom { evaluation: f.occurrences[2].id(),..f.atom.clone() };
    for wrong in [Selection { atom: wrong_atom.id(),..selection.clone() },
        Selection { transfer: TransferKey { kind: TransferKind::Derived,..f.key.clone() }.id(),..selection }] {
        let mut data = base.clone(); insert(&f,&mut data,vec![wrong]);
        assert!(check(TransferKey::invariants().remove(0),&data).is_err());
    }
}

#[test]
fn transfer_call_site_must_belong_to_both_scope_and_acquired_inputs() {
    let f = Fixture::new();
    let foreign_input = InputRevision::from_entries(vec![ManifestEntry { path: "other.py".into(),content: ContentHash::of(b"z"),byte_len: 1 }]).unwrap();
    let foreign = SourceArtifact::from_bytes(foreign_input.id(),"other.py".into(),b"z").unwrap();
    let foreign_site = Occurrence { source: foreign.id(),start: 0,end: 1,structural_path: vec![],..f.occurrences[0].clone() };
    // Same-input call site succeeds. An unrelated input fails even though the occurrence exists.
    // Explicit corpus membership authorizes it for input scope, but not for local artifact scope.
    for (site,corpus,artifact_scope,expected) in [
        (f.occurrences[0].clone(),false,false,true),
        (foreign_site.clone(),false,false,false),
        (foreign_site.clone(),true,false,true),
        (foreign_site.clone(),true,true,false),
    ] {
        let mut data = records(&f);
        let scope = if artifact_scope { CoverageScope::Artifact { artifact: f.source.id() } } else { f.scope.clone() };
        let qualification = AssertionQualification { scope: scope.id(),..f.qualification.clone() };
        let key = TransferKey { call_site: Some(site.id()),scope: scope.id(),..f.key.clone() };
        let alternative = TransferBranch::new(key.clone(),qualification.clone(),f.diagram.clone()).unwrap().alternative();
        let evidence = Evidence::Occurrence { occurrence: f.occurrences[0].id() };
        let support = TransferSupport { assertion: alternative.id(),run: f.run.id(),surface: f.surface.id(),evidence: evidence.id(),
            origin: Origin::DerivedAnalysis,mode: ExtractionMode::GraphAnalysis,fidelity: Fidelity::NormalizedStructural };
        insert(&f,&mut data,vec![f.source.clone(),foreign.clone()]);
        let mut occurrences = f.occurrences.clone(); occurrences.push(foreign_site.clone()); insert(&f,&mut data,occurrences);
        insert(&f,&mut data,vec![scope]); insert(&f,&mut data,vec![qualification]);
        insert(&f,&mut data,vec![key]); insert(&f,&mut data,vec![alternative]); insert(&f,&mut data,vec![support]);
        if corpus { insert(&f,&mut data,vec![CorpusLibrary { corpus: f.input.id(),library: foreign_input.id() }]); }
        assert_eq!(check(TransferSupport::invariants().remove(0),&data).is_ok(),expected);
    }
}

#[test]
fn ordinary_assertions_validate_guard_operand_sources_even_when_evaluation_is_local() {
    let f = Fixture::new();
    let other_input = InputRevision::from_entries(vec![]).unwrap();
    let other = SourceArtifact::from_bytes(other_input.id(),"other.py".into(),b"z").unwrap();
    let other_occurrence = Occurrence { source: other.id(),start: 0,end: 1,..f.occurrences[0].clone() };
    let other_root = PlaceRoot::Occurrence { occurrence: other_occurrence.id() };
    let other_place = Place { root: other_root.id(),path: f.path.id() };
    let (run,families) = ProviderRun::new(f.provider.id(),f.context.id(),f.input.id(),f.context.config_digest,[FactFamily::Syntax]).unwrap();
    let surface = ProviderSurface { family: FactFamily::Syntax,..f.surface.clone() };
    for (foreign,corpus,artifact_scope,expected) in [(false,false,false,true),(true,false,false,false),(true,true,false,true),(true,true,true,false)] {
        let mut data = records(&f);
        let scope = if artifact_scope { CoverageScope::Artifact { artifact: f.source.id() } } else { f.scope.clone() };
        let atom = EvaluationAtom { operand: Some(if foreign { other_place.id() } else { f.places[1].id() }),..f.atom.clone() };
        let (condition,nodes) = Diagram::from_atom(atom.id()).records();
        let q = AssertionQualification { scope: scope.id(),condition: condition.id(),..f.qualification.clone() };
        let assertion = SyntaxObservation { qualification: q.id(),occurrence: f.occurrences[0].id(),spelling: "x".into() };
        let evidence = Evidence::Occurrence { occurrence: f.occurrences[0].id() };
        let support = SyntaxSupport { assertion: assertion.id(),run: run.id(),surface: surface.id(),evidence: evidence.id(),
            origin: Origin::SourceObservation,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(insert(&f,&mut data,vec![$row.clone()]);)+ }; }
        one!(scope,atom,condition,q,assertion,support,run,surface,evidence); insert(&f,&mut data,nodes);
        insert(&f,&mut data,families.clone()); insert(&f,&mut data,vec![f.source.clone(),other.clone()]);
        let mut occurrences = f.occurrences.clone(); occurrences.push(other_occurrence.clone()); insert(&f,&mut data,occurrences);
        let mut roots = f.roots.clone(); roots.push(other_root.clone()); insert(&f,&mut data,roots);
        let mut places = f.places.clone(); places.push(other_place.clone()); insert(&f,&mut data,places);
        if corpus { insert(&f,&mut data,vec![CorpusLibrary { corpus: f.input.id(),library: other_input.id() }]); }
        assert_eq!(check(SyntaxSupport::invariants().remove(0),&data).is_ok(),expected);
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}

#[test]
fn transfer_identity_is_the_semantic_key() {
    let f = Fixture::new();
    let base = f.key.clone();
    assert_eq!(base.id(), f.key.clone().id());
    // An authored model and our own summary of the same flow stay distinct rows, as do the
    // flow-local key and an instantiation at a call site.
    let authored = TransferKey { provenance: ProvenanceClass::AuthoredModel, ..base.clone() };
    let at_site = TransferKey { call_site: Some(f.occurrences[1].id()), ..base.clone() };
    let derived = TransferKey { kind: TransferKind::Derived, ..base.clone() };
    let candidate = TransferKey { modality: Modality::Candidate, ..base.clone() };
    let ids: BTreeSet<_> = [&base, &authored, &at_site, &derived, &candidate].iter().map(|key| key.id()).collect();
    assert_eq!(ids.len(), 5);
    // The accumulating condition is not part of the key: alternatives differ, the key does not.
    let widened = AssertionQualification { condition: Diagram::always().id(), ..f.qualification.clone() };
    let other = TransferBranch::new(base.clone(), widened, Diagram::always()).unwrap();
    assert_eq!(other.key().id(), f.branch().key().id());
    assert_ne!(other.alternative().id(), f.branch().alternative().id());
}
