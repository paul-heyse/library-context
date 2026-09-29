use std::collections::BTreeMap;
use arrow_array::RecordBatch;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, conditions::*, input::*, lexical::*, source::*};

pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str,RecordBatch>,
    pub binding: BindingObservation, pub binding_support: BindingSupport,
    pub function_scope: LexicalScopeObservation, pub scope_support: LexicalScopeSupport,
    pub resolution: LexicalResolution, pub resolution_support: LexicalResolutionSupport,
    pub foreign: Occurrence,
}
impl Fixture {
    pub fn new() -> Self {
        let model = model().unwrap();
        let bytes = include_bytes!("../../../../fixtures/python/semantic_lexical/example.py");
        let input = InputRevision::from_entries(vec![
            ManifestEntry { path: "example.py".into(), content: ContentHash::of(bytes), byte_len: bytes.len() as i64 },
            ManifestEntry { path: "other.py".into(), content: ContentHash::of(b"y"), byte_len: 1 },
        ]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(),"example.py".into(),bytes).unwrap();
        let other = SourceArtifact::from_bytes(input.id(),"other.py".into(),b"y").unwrap();
        let origin = InputOrigin::Tree { label: "lexical contract".into() };
        let acquisition = InputAcquisition { input: input.id(),origin: origin.id() };
        let context = AnalysisContext { python_version: "3.14.7".into(),python_platform: "linux".into(),search_path: vec![],site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),environment_digest: input.manifest,lock_digest: None };
        let provider = Provider { tool: "lexical-fixture".into(),revision: "1".into(),build_digest: ContentHash::of(b"fixture") };
        let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Lexical]).unwrap();
        let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Lexical,name: "lexical".into() };
        let coverage_scope = CoverageScope::Artifact { artifact: source.id() };
        let (condition,nodes) = Diagram::always().records();
        let qualification = AssertionQualification { context: context.id(),scope: coverage_scope.id(),condition: condition.id(),modality: Modality::Definite,approximation: Approximation::Exact };
        let root = Occurrence { source: source.id(),start: 0,end: bytes.len() as i64,syntax_kind: SyntaxKind::ModModule,role: OccurrenceRole::Syntax,structural_path: vec![0] };
        let function = Occurrence { start: 6,syntax_kind: SyntaxKind::StmtFunctionDef,role: OccurrenceRole::Declaration,structural_path: vec![0,1],..root.clone() };
        let site = Occurrence { end: 1,syntax_kind: SyntaxKind::ExprName,role: OccurrenceRole::Binding,structural_path: vec![0,0,0],..root.clone() };
        let value = Occurrence { start: 4,end: 5,syntax_kind: SyntaxKind::ExprNumberLiteral,structural_path: vec![0,0,1],..root.clone() };
        let at = bytes.iter().rposition(|b| *b == b'x').unwrap() as i64;
        let read = Occurrence { start: at,end: at+1,syntax_kind: SyntaxKind::ExprName,role: OccurrenceRole::Read,structural_path: vec![0,1,0,0],..root.clone() };
        let foreign = Occurrence { source: other.id(),start: 0,end: 1,..read.clone() };
        let module = LexicalScope { owner: root.id(),kind: LexicalScopeKind::Module };
        let function_lexical = LexicalScope { owner: function.id(),kind: LexicalScopeKind::Function };
        let event = BindingEvent { site: site.id(),name: "x".into() };
        let module_scope = LexicalScopeObservation { qualification: qualification.id(),scope: module.id(),parent: None };
        let function_scope = LexicalScopeObservation { qualification: qualification.id(),scope: function_lexical.id(),parent: Some(module.id()) };
        let binding = BindingObservation { qualification: qualification.id(),event: event.id(),scope: module.id(),kind: BindingEventKind::Assignment,
            ordinal: 0,value: Some(value.id()),static_branch: None,static_polarity: None };
        let reference = ReferenceObservation { qualification: qualification.id(),read: read.id(),scope: function_lexical.id(),parent: function.id(),field: SyntaxField::Value,name: "x".into() };
        let target = LexicalTarget::Binding { event: event.id() };
        let resolution = LexicalResolution { qualification: qualification.id(),read: read.id(),target: target.id(),captured: false };
        let root_evidence = Evidence::Occurrence { occurrence: root.id() };
        let scope_evidence = Evidence::Occurrence { occurrence: function.id() };
        let binding_evidence = Evidence::Occurrence { occurrence: site.id() };
        let read_evidence = Evidence::Occurrence { occurrence: read.id() };
        macro_rules! support { ($ty:ident,$row:expr,$evidence:expr) => { $ty { assertion: $row.id(),run: run.id(),surface: surface.id(),evidence: $evidence.id(),
            origin: Origin::SourceObservation,mode: ExtractionMode::Recognizer,fidelity: Fidelity::NativeStructural } }; }
        let module_support = support!(LexicalScopeSupport,module_scope,root_evidence);
        let scope_support = support!(LexicalScopeSupport,function_scope,scope_evidence);
        let binding_support = support!(BindingSupport,binding,binding_evidence);
        let reference_support = support!(ReferenceSupport,reference,read_evidence);
        let resolution_support = support!(LexicalResolutionSupport,resolution,read_evidence);
        let coverage = ProviderCoverage { scope: coverage_scope.id(),provider: provider.id(),context: context.id(),family: FactFamily::Lexical,
            run: Some(run.id()),status: CoverageStatus::CompleteUnderStatedModel,reason: None,diagnostic: None };
        let mut fixture = Self { model,batches: BTreeMap::new(),binding,binding_support,function_scope,scope_support,resolution,resolution_support,foreign };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(fixture.put(vec![$row.clone()]);)+ }; }
        one!(input,origin,acquisition,context,provider,run,surface,coverage_scope,condition,qualification,event,target,reference,reference_support,coverage);
        one!(fixture.binding,fixture.binding_support,fixture.resolution,fixture.resolution_support);
        fixture.put(families); fixture.put(nodes);
        fixture.put(vec![source.clone(),other.clone()]);
        fixture.put(vec![root,function,site,value,read,fixture.foreign.clone()]);
        fixture.put(vec![module,function_lexical]);
        fixture.put(vec![module_scope,fixture.function_scope.clone()]);
        fixture.put(vec![module_support,fixture.scope_support.clone()]);
        fixture.put(vec![root_evidence,scope_evidence,binding_evidence,read_evidence]);
        fixture.put(ArtifactChunk::split(&source,bytes).unwrap().chain(ArtifactChunk::split(&other,b"y").unwrap()).collect());
        fixture
    }
    pub fn put<R: Record>(&mut self, rows: Vec<R>) { self.batches.insert(R::NAME,Batch::new(&self.model,rows, &budget()).unwrap().arrow().clone()); }
    pub fn rows<R: Record>(&self) -> Vec<R> { self.batches.get(R::NAME).map(|b| R::decode(b).unwrap()).unwrap_or_default() }
    pub fn check(&self, invariant: &Invariant) -> Result<(),ModelError> {
        let mut check = (invariant.create)();
        for input in &invariant.inputs { if let Some(batch) = self.batches.get(input.name()) { check.visit(input.name(),batch)?; } }
        check.finish()
    }
    pub fn foreign_value(&mut self) {
        self.binding.value = Some(self.foreign.id()); self.binding_support.assertion = self.binding.id();
        self.put(vec![self.binding.clone()]); self.put(vec![self.binding_support.clone()]);
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
