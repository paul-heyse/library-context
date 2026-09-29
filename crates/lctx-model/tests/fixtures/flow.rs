#[allow(dead_code)] #[path = "lexical.rs"] mod lexical_fixture;
use lctx_model::domain::{*,assertion::*,attribution::*,flow::*,lexical::*,source::*,value::*,transfer::TransferKind};
pub struct Fixture {
    pub base: lexical_fixture::Fixture,
    pub use_: FlowUse, pub definition: FlowDefinition,
    pub reaching: FlowReachingObservation, pub reaching_support: FlowReachingSupport,
}
impl Fixture {
    pub fn new() -> Self {
        let mut base = lexical_fixture::Fixture::new();
        let input = base.rows::<lctx_model::domain::input::InputRevision>().pop().unwrap();
        let context = base.rows::<AnalysisContext>().pop().unwrap();
        let provider = Provider { tool: "flow-fixture".into(),revision: "1".into(),build_digest: ContentHash::of(b"flow-fixture") };
        let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Flow]).unwrap();
        let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Flow,name: "flow observations".into() };
        let q = base.rows::<AssertionQualification>().pop().unwrap().id();
        let occurrences = base.rows::<Occurrence>();
        let read = occurrences.iter().find(|o| o.id() == base.resolution.read).unwrap().clone();
        let site = base.rows::<BindingEvent>().pop().unwrap().site;
        let module = Module { source: read.source,qualified_name: "example".into() };
        let root = PlaceRoot::Global { module: module.id(),name: "x".into() };
        let path = AccessPath::empty();
        let place = Place { root: root.id(),path: path.id() };
        let use_ = FlowUse { occurrence: read.id(),place: place.id() };
        let definition = FlowDefinition { occurrence: site,place: place.id() };
        let target = ReachingDefinition::Bound { definition: definition.id() };
        let reaching = FlowReachingObservation { qualification: q,use_: use_.id(),target: target.id(),loop_carried: false };
        let module_scope = base.rows::<LexicalScope>().into_iter().find(|s| s.kind == LexicalScopeKind::Module).unwrap().id();
        let use_obs = FlowUseObservation { qualification: q,use_: use_.id(),scope: base.function_scope.scope,annotation: false };
        let def_obs = FlowDefinitionObservation { qualification: q,definition: definition.id(),scope: module_scope,kind: BindingEventKind::Assignment,value: base.binding.value };
        let returned = Occurrence { start: read.start-7,syntax_kind: SyntaxKind::StmtReturn,role: OccurrenceRole::Return,structural_path: vec![0,1,0],..read.clone() };
        let value = FlowValueObservation { qualification: q,use_: use_.id(),sink: returned.id(),kind: FlowSinkKind::Return,transfer: TransferKind::Identity,through_call: false };
        let region = FlowRegionObservation { qualification: q,statement: returned.id(),scope: base.function_scope.scope };
        let use_evidence = Evidence::Occurrence { occurrence: read.id() };
        let def_evidence = Evidence::Occurrence { occurrence: site };
        let return_evidence = Evidence::Occurrence { occurrence: returned.id() };
        macro_rules! support { ($ty:ident,$row:expr,$e:expr) => { $ty { assertion: $row.id(),run: run.id(),surface: surface.id(),evidence: $e.id(),
            origin: Origin::AnalyzerAssertion,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural } }; }
        let reaching_support = support!(FlowReachingSupport,reaching,use_evidence);
        let coverage = ProviderCoverage { scope: base.rows::<AssertionQualification>()[0].scope,provider: provider.id(),context: context.id(),family: FactFamily::Flow,
            run: Some(run.id()),status: CoverageStatus::CompleteUnderStatedModel,reason: None,diagnostic: None };
        macro_rules! append { ($ty:ty,$rows:expr) => { let mut rows = base.rows::<$ty>(); rows.extend($rows); base.put(rows); }; }
        append!(Provider,vec![provider.clone()]); append!(ProviderRun,vec![run.clone()]); append!(RunFamily,families);
        append!(ProviderSurface,vec![surface.clone()]); append!(Module,vec![module]); append!(PlaceRoot,vec![root]);
        append!(AccessPath,vec![path]); append!(Place,vec![place]); append!(Occurrence,vec![returned]); append!(Evidence,vec![return_evidence.clone()]);
        append!(ProviderCoverage,vec![coverage]);
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(base.put(vec![$row.clone()]);)+ }; }
        one!(use_,definition,target,reaching,reaching_support,use_obs,support!(FlowUseSupport,use_obs,use_evidence),
            def_obs,support!(FlowDefinitionSupport,def_obs,def_evidence),value,support!(FlowValueSupport,value,return_evidence),
            region,support!(FlowRegionSupport,region,return_evidence));
        Self { base,use_,definition,reaching,reaching_support }
    }
    pub fn foreign_place(&mut self) {
        let root = PlaceRoot::Occurrence { occurrence: self.base.foreign.id() };
        let place = Place { root: root.id(),path: AccessPath::empty().id() };
        let mut roots = self.base.rows::<PlaceRoot>(); roots.push(root); self.base.put(roots);
        let mut places = self.base.rows::<Place>(); places.push(place.clone()); self.base.put(places);
        self.definition.place = place.id();
        let mut def = self.base.rows::<FlowDefinitionObservation>().pop().unwrap(); def.definition = self.definition.id();
        let mut support = self.base.rows::<FlowDefinitionSupport>().pop().unwrap(); support.assertion = def.id();
        self.base.put(vec![self.definition.clone()]); self.base.put(vec![def]); self.base.put(vec![support]);
        let target = ReachingDefinition::Bound { definition: self.definition.id() };
        self.reaching.target = target.id(); self.reaching_support.assertion = self.reaching.id();
        self.base.put(vec![target]); self.base.put(vec![self.reaching.clone()]); self.base.put(vec![self.reaching_support.clone()]);
    }
}
