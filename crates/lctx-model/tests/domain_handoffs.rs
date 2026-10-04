//! Seeded native-premise controls for named origin admission, separate from native PG qualification.
use lctx_model::domain::{
    analysis::{native::*, policy::EvidenceStatus},
    assertion::*,
    attribution::*,
    conditions::Diagram,
    flow::*,
    flow_inventory::*,
    input::*,
    lexical::*,
    normalized::Rows,
    resources::ResourceBudget,
    source::*,
    structural::handoffs::{self, Data, ValueSource},
    value::*,
    *,
};

struct Fixture {
    data: Data,
    budget: ResourceBudget,
    read: Occurrence,
    origin: Occurrence,
    context: Id<AnalysisContext>,
    coverage: ProviderCoverage,
    scopes: [CoverageScope; 3],
}
impl Fixture {
    fn new(incomplete: bool) -> Self {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = Data::new(&budget);
        let bytes = b"value = produce()\nconsume(value)\n";
        let input = InputRevision::from_entries(vec![ManifestEntry {
            path: "_lctx/blocks/guide/block_0.py".into(),
            content: ContentHash::of(bytes),
            byte_len: bytes.len() as i64,
        }]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(), "_lctx/blocks/guide/block_0.py".into(), bytes).unwrap();
        let module = Module { source: source.id(), qualified_name: "guide".into() };
        let scopes = [
            CoverageScope::Input { input: input.id() },
            CoverageScope::Artifact { artifact: source.id() },
            CoverageScope::Module { module: module.id() },
        ];
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"named origin fixture"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let provider = Provider {
            tool: "native-premise fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"named origin fixture"),
        };
        let (run, _) = ProviderRun::new(provider.id(), context.id(), input.id(), context.config_digest, [FactFamily::Flow]).unwrap();
        let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Flow, name: "flow".into() };
        let evidence = Evidence::Invocation { run: run.id() };
        let (condition, nodes) = Diagram::always().records();
        let q = AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scopes[0].id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let root = Occurrence { source: source.id(), start: 0, end: bytes.len() as i64, syntax_kind: SyntaxKind::ModModule, role: OccurrenceRole::Syntax, structural_path: vec![0] };
        let binding = Occurrence { start: 0, end: 5, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Binding, structural_path: vec![0, 0, 0], ..root.clone() };
        let origin = Occurrence { start: 8, end: 17, syntax_kind: SyntaxKind::ExprCall, role: OccurrenceRole::Call, structural_path: vec![0, 0, 1], ..root.clone() };
        let read = Occurrence { start: 26, end: 31, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: vec![0, 1, 0, 1], ..root.clone() };
        let lexical = LexicalScope { owner: root.id(), kind: LexicalScopeKind::Module };
        let place_root = PlaceRoot::Global { module: module.id(), name: "value".into() };
        let path = AccessPath::empty();
        let place = Place { root: place_root.id(), path: path.id() };
        let use_ = FlowUse { occurrence: read.id(), place: place.id() };
        let observation = FlowUseObservation { qualification: q.id(), use_: use_.id(), scope: lexical.id(), annotation: false };
        let definition = FlowDefinition { occurrence: binding.id(), place: place.id() };
        let definition_observation = FlowDefinitionObservation { qualification: q.id(), definition: definition.id(), scope: lexical.id(), kind: BindingEventKind::Assignment, value: Some(origin.id()) };
        let target = ReachingDefinition::Bound { definition: definition.id() };
        let reaching = FlowReachingObservation { qualification: q.id(), use_: use_.id(), target: target.id(), loop_carried: false };
        macro_rules! support {
            ($ty:ident, $row:expr) => { $ty { assertion: $row.id(), run: run.id(), surface: surface.id(), evidence: evidence.id(), origin: Origin::AnalyzerAssertion, mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural } };
        }
        let use_support = support!(FlowUseSupport, observation);
        let reaching_support = support!(FlowReachingSupport, reaching);
        let definition_support = support!(FlowDefinitionSupport, definition_observation);
        let view = FlowSourceViewObservation { qualification: q.id(), source: source.id(), original_content: source.content, view_content: source.content, byte_len: source.byte_len, renamed_type_checking: 0 };
        let view_support = support!(FlowSourceViewSupport, view);
        let mut states = vec![CandidateState { kind: FlowCandidateKind::Bound, pruned: false, loop_expanded: false, unattached: false, condition_unavailable: false, reachability_lost: false, mapped_count: 1 }];
        if incomplete {
            states.push(CandidateState { kind: FlowCandidateKind::Deleted, pruned: true, loop_expanded: false, unattached: false, condition_unavailable: false, reachability_lost: false, mapped_count: 0 });
        }
        let (inventory, candidates, members) = FlowUseInventoryObservation::new(q.id(), use_.id(), lexical.id(), view.id(), &states, &[(0, reaching.id(), reaching_support.id())]).unwrap();
        let inventory_support = support!(FlowUseInventorySupport, inventory);
        let coverage = ProviderCoverage { scope: scopes[0].id(), provider: Some(provider.id()), context: context.id(), family: FactFamily::Flow, run: Some(run.id()), status: CoverageStatus::CompleteUnderStatedModel, reason: None, diagnostic: None };
        macro_rules! insert { ($field:ident, $($row:expr),+) => { $(data.entry.$field.insert($row.clone()).unwrap();)+ }; }
        insert!(artifacts, source);
        insert!(modules, module);
        for scope in &scopes { data.entry.scopes.insert(scope.clone()).unwrap(); }
        insert!(providers, provider);
        insert!(runs, run);
        insert!(surfaces, surface);
        insert!(evidence, evidence);
        insert!(qualifications, q);
        insert!(conditions, condition);
        for node in nodes { data.entry.condition_nodes.insert(node).unwrap(); }
        insert!(occurrences, root, binding, origin, read);
        insert!(lexical_scopes, lexical);
        insert!(roots, place_root);
        insert!(paths, path);
        insert!(places, place);
        insert!(uses, use_);
        insert!(use_observations, observation);
        insert!(use_supports, use_support);
        insert!(definitions, definition);
        insert!(definition_observations, definition_observation);
        insert!(definition_supports, definition_support);
        insert!(targets, target);
        insert!(reaching, reaching);
        insert!(reaching_supports, reaching_support);
        insert!(source_views, view);
        insert!(source_view_supports, view_support);
        insert!(inventories, inventory);
        insert!(inventory_supports, inventory_support);
        for candidate in candidates { data.entry.inventory_candidates.insert(candidate).unwrap(); }
        for member in members { data.entry.inventory_members.insert(member).unwrap(); }
        insert!(coverage, coverage);
        for premise in [
            NativeAssertionPremise::Use { assertion: observation.id(), support: use_support.id() },
            NativeAssertionPremise::Reaching { assertion: reaching.id(), support: reaching_support.id() },
            NativeAssertionPremise::Definition { assertion: definition_observation.id(), support: definition_support.id() },
            NativeAssertionPremise::FlowUseInventory { assertion: inventory.id(), support: inventory_support.id() },
        ] {
            data.native.insert(NativeQualification { premise: premise.id(), qualification: q.id(), family: FactFamily::Flow, fidelity: Fidelity::NativeStructural, status: EvidenceStatus::StructurallyObserved }).unwrap();
        }
        Self { data, budget, read, origin, context: context.id(), coverage, scopes }
    }
    fn coverage(&mut self, scope: CoverageScope, status: CoverageStatus) {
        self.data.entry.scopes.insert(scope.clone()).unwrap();
        self.coverage.scope = scope.id();
        self.coverage.status = status;
        self.coverage.reason = (status == CoverageStatus::Partial).then_some(obligation::ObligationKind::IncompleteCoverage);
        self.data.entry.coverage = Rows::new(&self.budget);
        self.data.entry.coverage.insert(self.coverage.clone()).unwrap();
    }
    fn selected(&self) -> Option<(Id<Occurrence>, ValueSource)> {
        handoffs::named_definition(&self.data, &self.read, self.context, &self.budget).unwrap()
    }
}

#[test]
fn source_covering_input_artifact_and_module_receipts_admit_complete_native_singletons() {
    for status in [CoverageStatus::CompleteUnderStatedModel, CoverageStatus::Partial] {
        for index in 0..3 {
            let mut f = Fixture::new(false);
            f.coverage(f.scopes[index].clone(), status);
            let (origin, value) = f.selected().expect("source-covering evidence and native singleton closure");
            assert_eq!(origin, f.origin.id());
            assert!(matches!(value, ValueSource::Named { coverage, .. } if coverage == f.coverage.id()));
        }
    }
}

#[test]
fn foreign_coverage_and_missing_or_incomplete_inventory_never_admit_named_origins() {
    for status in [CoverageStatus::CompleteUnderStatedModel, CoverageStatus::Partial] {
        let foreign_input = InputRevision::from_entries(vec![ManifestEntry { path: "foreign.py".into(), content: ContentHash::of(b"foreign"), byte_len: 7 }]).unwrap();
        let foreign = SourceArtifact::from_bytes(foreign_input.id(), "foreign.py".into(), b"foreign").unwrap();
        let module = Module { source: foreign.id(), qualified_name: "foreign".into() };
        for scope in [CoverageScope::Input { input: foreign_input.id() }, CoverageScope::Artifact { artifact: foreign.id() }, CoverageScope::Module { module: module.id() }] {
            let mut f = Fixture::new(false);
            f.data.entry.artifacts.insert(foreign.clone()).unwrap();
            f.data.entry.modules.insert(module.clone()).unwrap();
            f.coverage(scope, status);
            assert!(f.selected().is_none(), "foreign coverage never covers the selected source");
        }
        for index in 0..3 {
            let mut incomplete = Fixture::new(true);
            incomplete.coverage(incomplete.scopes[index].clone(), status);
            assert!(incomplete.selected().is_none(), "family coverage cannot close a pruned per-use inventory");
            let mut missing = Fixture::new(false);
            missing.coverage(missing.scopes[index].clone(), status);
            missing.data.entry.inventories = Rows::new(&missing.budget);
            assert!(missing.selected().is_none(), "family coverage cannot replace missing per-use authority");
        }
    }
}

#[test]
fn source_covering_receipts_do_not_replace_same_run_native_inventory_evidence() {
    for index in 0..3 {
        for mutation in 0..2 {
            let mut f = Fixture::new(false);
            f.coverage(f.scopes[index].clone(), CoverageStatus::Partial);
            let inventory = f.data.entry.inventories.iter().next().unwrap();
            let mut support = f.data.entry.inventory_supports.iter().next().unwrap().clone();
            let premise = NativeAssertionPremise::FlowUseInventory { assertion: inventory.id(), support: support.id() };
            let qualification = inventory.qualification;
            let retained = f.data.native.iter().filter(|n| n.premise != premise.id()).cloned().collect::<Vec<_>>();
            f.data.native = Rows::new(&f.budget);
            for native in retained { f.data.native.insert(native).unwrap(); }
            if mutation == 0 {
                let mut run = f.data.entry.runs.get(support.run).unwrap().clone();
                run.configuration = ContentHash::of(b"foreign native invocation");
                support.run = run.id();
                f.data.entry.runs.insert(run).unwrap();
                f.data.entry.inventory_supports = Rows::new(&f.budget);
                f.data.entry.inventory_supports.insert(support.clone()).unwrap();
                // Re-pair the foreign support: refusal must be about its actual run, not a stale ID.
                f.data.native.insert(NativeQualification {
                    premise: NativeAssertionPremise::FlowUseInventory { assertion: inventory.id(), support: support.id() }.id(),
                    qualification,
                    family: FactFamily::Flow,
                    fidelity: Fidelity::NativeStructural,
                    status: EvidenceStatus::StructurallyObserved,
                }).unwrap();
            }
            assert!(f.selected().is_none(), "covering Partial evidence cannot replace same-run native inventory premises");
        }
    }
}
