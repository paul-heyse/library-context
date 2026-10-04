//! Seeded native-premise controls for named origin admission, separate from native PG qualification.
use lctx_model::domain::{
    analysis::{native::*, policy::EvidenceStatus},
    assertion::*,
    attribution::*,
    conditions::{Diagram, EvaluationAtom},
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
        Self::with_conditional_reaching(incomplete, false)
    }
    fn with_conditional_reaching(incomplete: bool, conditional: bool) -> Self {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = Data::new(&budget);
        let bytes = b"value = produce()\nconsume(value)\n";
        let input = InputRevision::from_entries(vec![ManifestEntry {
            path: "_lctx/blocks/guide/block_0.py".into(),
            content: ContentHash::of(bytes),
            byte_len: bytes.len() as i64,
        }])
        .unwrap();
        let source =
            SourceArtifact::from_bytes(input.id(), "_lctx/blocks/guide/block_0.py".into(), bytes)
                .unwrap();
        let module = Module {
            source: source.id(),
            qualified_name: "guide".into(),
        };
        let scopes = [
            CoverageScope::Input { input: input.id() },
            CoverageScope::Artifact {
                artifact: source.id(),
            },
            CoverageScope::Module {
                module: module.id(),
            },
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
        let (run, _) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Flow],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Flow,
            name: "flow".into(),
        };
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
        let root = Occurrence {
            source: source.id(),
            start: 0,
            end: bytes.len() as i64,
            syntax_kind: SyntaxKind::ModModule,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let binding = Occurrence {
            start: 0,
            end: 5,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Binding,
            structural_path: vec![0, 0, 0],
            ..root.clone()
        };
        let origin = Occurrence {
            start: 8,
            end: 17,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Call,
            structural_path: vec![0, 0, 1],
            ..root.clone()
        };
        let read = Occurrence {
            start: 26,
            end: 31,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![0, 1, 0, 1],
            ..root.clone()
        };
        let lexical = LexicalScope {
            owner: root.id(),
            kind: LexicalScopeKind::Module,
        };
        let place_root = PlaceRoot::Global {
            module: module.id(),
            name: "value".into(),
        };
        let path = AccessPath::empty();
        let place = Place {
            root: place_root.id(),
            path: path.id(),
        };
        let use_ = FlowUse {
            occurrence: read.id(),
            place: place.id(),
        };
        let observation = FlowUseObservation {
            qualification: q.id(),
            use_: use_.id(),
            scope: lexical.id(),
            annotation: false,
        };
        let definition = FlowDefinition {
            occurrence: binding.id(),
            place: place.id(),
        };
        let definition_observation = FlowDefinitionObservation {
            qualification: q.id(),
            definition: definition.id(),
            scope: lexical.id(),
            kind: BindingEventKind::Assignment,
            value: Some(origin.id()),
        };
        let target = ReachingDefinition::Bound {
            definition: definition.id(),
        };
        let reaching_q = if conditional {
            let predicate = Predicate::NonTerminalCall { awaiting: false };
            let atom = EvaluationAtom {
                evaluation: origin.id(),
                context: context.id(),
                predicate: predicate.id(),
                operand: None,
            };
            let (condition, nodes) = Diagram::from_atom(atom.id()).records();
            data.entry.predicates.insert(predicate).unwrap();
            data.entry.atoms.insert(atom).unwrap();
            data.entry.conditions.insert(condition.clone()).unwrap();
            for node in nodes {
                data.entry.condition_nodes.insert(node).unwrap();
            }
            AssertionQualification { condition: condition.id(), ..q.clone() }
        } else {
            q.clone()
        };
        let reaching = FlowReachingObservation {
            qualification: reaching_q.id(),
            use_: use_.id(),
            target: target.id(),
            loop_carried: false,
        };
        macro_rules! support {
            ($ty:ident, $row:expr) => {
                $ty {
                    assertion: $row.id(),
                    run: run.id(),
                    surface: surface.id(),
                    evidence: evidence.id(),
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                }
            };
        }
        let use_support = support!(FlowUseSupport, observation);
        let reaching_support = support!(FlowReachingSupport, reaching);
        let definition_support = support!(FlowDefinitionSupport, definition_observation);
        let view = FlowSourceViewObservation {
            qualification: q.id(),
            source: source.id(),
            original_content: source.content,
            view_content: source.content,
            byte_len: source.byte_len,
            renamed_type_checking: 0,
        };
        let view_support = support!(FlowSourceViewSupport, view);
        let mut states = vec![CandidateState {
            kind: FlowCandidateKind::Bound,
            pruned: false,
            loop_expanded: false,
            unattached: false,
            condition_unavailable: false,
            reachability_lost: false,
            mapped_count: 1,
        }];
        if incomplete {
            states.push(CandidateState {
                kind: FlowCandidateKind::Deleted,
                pruned: true,
                loop_expanded: false,
                unattached: false,
                condition_unavailable: false,
                reachability_lost: false,
                mapped_count: 0,
            });
        }
        let (inventory, candidates, members) = FlowUseInventoryObservation::new(
            q.id(),
            use_.id(),
            lexical.id(),
            view.id(),
            &states,
            &[(0, reaching.id(), reaching_support.id())],
        )
        .unwrap();
        let inventory_support = support!(FlowUseInventorySupport, inventory);
        let coverage = ProviderCoverage {
            scope: scopes[0].id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Flow,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        macro_rules! insert { ($field:ident, $($row:expr),+) => { $(data.entry.$field.insert($row.clone()).unwrap();)+ }; }
        insert!(artifacts, source);
        insert!(modules, module);
        for scope in &scopes {
            data.entry.scopes.insert(scope.clone()).unwrap();
        }
        insert!(providers, provider);
        insert!(runs, run);
        insert!(surfaces, surface);
        insert!(evidence, evidence);
        insert!(qualifications, q, reaching_q);
        insert!(conditions, condition);
        for node in nodes {
            data.entry.condition_nodes.insert(node).unwrap();
        }
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
        for candidate in candidates {
            data.entry.inventory_candidates.insert(candidate).unwrap();
        }
        for member in members {
            data.entry.inventory_members.insert(member).unwrap();
        }
        insert!(coverage, coverage);
        for (premise, qualification) in [
            (NativeAssertionPremise::Use {
                assertion: observation.id(),
                support: use_support.id(),
            }, q.id()),
            (NativeAssertionPremise::Reaching {
                assertion: reaching.id(),
                support: reaching_support.id(),
            }, reaching_q.id()),
            (NativeAssertionPremise::Definition {
                assertion: definition_observation.id(),
                support: definition_support.id(),
            }, q.id()),
            (NativeAssertionPremise::FlowUseInventory {
                assertion: inventory.id(),
                support: inventory_support.id(),
            }, q.id()),
        ] {
            data.native
                .insert(NativeQualification {
                    premise: premise.id(),
                    qualification,
                    family: FactFamily::Flow,
                    fidelity: Fidelity::NativeStructural,
                    status: EvidenceStatus::StructurallyObserved,
                })
                .unwrap();
        }
        Self {
            data,
            budget,
            read,
            origin,
            context: context.id(),
            coverage,
            scopes,
        }
    }
    fn coverage(&mut self, scope: CoverageScope, status: CoverageStatus) {
        self.data.entry.scopes.insert(scope.clone()).unwrap();
        self.coverage.scope = scope.id();
        self.coverage.status = status;
        self.coverage.reason = (status == CoverageStatus::Partial)
            .then_some(obligation::ObligationKind::IncompleteCoverage);
        self.data.entry.coverage = Rows::new(&self.budget);
        self.data
            .entry
            .coverage
            .insert(self.coverage.clone())
            .unwrap();
    }
    fn selected(&self) -> Option<(Id<Occurrence>, ValueSource)> {
        handoffs::named_definition(&self.data, &self.read, self.context, &self.budget).unwrap()
    }
    fn region(&mut self, domain: &Diagram) -> (FlowRegionObservation, FlowRegionSupport) {
        let observation = self.data.entry.use_observations.iter().next().unwrap().clone();
        let use_support = self.data.entry.use_supports.iter().next().unwrap().clone();
        let lexical = self.data.entry.lexical_scopes.get(observation.scope).unwrap();
        let root = self.data.entry.occurrences.get(lexical.owner).unwrap().clone();
        let CoverageScope::Module { module } = &self.scopes[2] else { unreachable!() };
        let entity = normalized::entities::EntityRef::Module { module: *module };
        let statement = Occurrence { start: 18, end: 32, syntax_kind: SyntaxKind::StmtExpr, structural_path: vec![0, 1], ..root.clone() };
        let (condition, nodes) = domain.records();
        let q = AssertionQualification { condition: condition.id(), ..self.data.entry.qualifications.get(observation.qualification).unwrap().clone() };
        self.data.entry.qualifications.insert(q.clone()).unwrap();
        self.data.entry.conditions.insert(condition).unwrap();
        for node in nodes { self.data.entry.condition_nodes.insert(node).unwrap(); }
        self.data.entry.refs.insert(entity.clone()).unwrap();
        for occurrence in [self.read.id(), statement.id()] {
            self.data.entry.owners.insert(normalized::entities::OccurrenceOwnership { occurrence, owner: root.id(), entity: entity.id() }).unwrap();
        }
        self.data.entry.occurrences.insert(statement.clone()).unwrap();
        let region = FlowRegionObservation { qualification: q.id(), statement: statement.id(), scope: observation.scope };
        let support = FlowRegionSupport { assertion: region.id(), run: use_support.run, surface: use_support.surface, evidence: use_support.evidence, origin: use_support.origin, mode: use_support.mode, fidelity: use_support.fidelity };
        self.data.entry.regions.insert(region.clone()).unwrap();
        self.data.entry.region_supports.insert(support.clone()).unwrap();
        self.data.native.insert(NativeQualification { premise: NativeAssertionPremise::Region { assertion: region.id(), support: support.id() }.id(), qualification: q.id(), family: FactFamily::Flow, fidelity: Fidelity::NativeStructural, status: EvidenceStatus::StructurallyObserved }).unwrap();
        (region, support)
    }
    fn reaching_domain(&self) -> Diagram {
        let q = self.data.entry.qualifications.get(self.data.entry.reaching.iter().next().unwrap().qualification).unwrap();
        Diagram::from_records(self.data.entry.conditions.get(q.condition).unwrap(), &self.data.entry.condition_nodes.iter().cloned().collect::<Vec<_>>()).unwrap()
    }
}

#[test]
fn native_containing_region_preserves_call_continuation_in_named_origin_proof() {
    for index in 0..3 {
        let mut f = Fixture::with_conditional_reaching(false, true);
        f.coverage(f.scopes[index].clone(), CoverageStatus::Partial);
        let domain = f.reaching_domain();
        let (region, support) = f.region(&domain);
        let (origin, proof) = f.selected().expect("same-run native execution domain implies exact reaching");
        assert_eq!(origin, f.origin.id());
        assert!(matches!(proof, ValueSource::Named { region: Some(r), region_support: Some(s), .. } if r == region.id() && s == support.id()));
        assert_ne!(domain.id(), Diagram::always().id(), "the continuation predicate remains conditional");
    }
}

#[test]
fn missing_foreign_ambiguous_false_or_unsupported_regions_refuse_conditional_named_origins() {
    for mutation in 0..10 {
        let mut f = Fixture::with_conditional_reaching(false, true);
        let domain = f.reaching_domain();
        let selected_domain = if mutation == 4 { Diagram::never() } else if mutation == 5 { Diagram::always() } else { domain };
        let (region, support) = f.region(&selected_domain);
        match mutation {
            0 => { f.data.entry.regions = Rows::new(&f.budget); }
            1 => {
                let mut foreign = f.data.entry.runs.get(support.run).unwrap().clone();
                foreign.configuration = ContentHash::of(b"foreign native region invocation");
                let altered = FlowRegionSupport { run: foreign.id(), ..support };
                f.data.entry.runs.insert(foreign).unwrap();
                f.data.entry.region_supports = Rows::new(&f.budget);
                f.data.entry.region_supports.insert(altered.clone()).unwrap();
                f.data.native.insert(NativeQualification { premise: NativeAssertionPremise::Region { assertion: region.id(), support: altered.id() }.id(), qualification: region.qualification, family: FactFamily::Flow, fidelity: Fidelity::NativeStructural, status: EvidenceStatus::StructurallyObserved }).unwrap();
            }
            2 => {
                let evidence = Evidence::SourceSpan { source: f.read.source, start: 18, end: 32 };
                f.data.entry.evidence.insert(evidence.clone()).unwrap();
                f.data.entry.region_supports.insert(FlowRegionSupport { evidence: evidence.id(), ..support }).unwrap();
            }
            3 => { f.data.entry.owners = Rows::new(&f.budget); }
            4 | 5 => {}
            6 => {
                let retained = f.data.native.iter().filter(|n| n.premise != (NativeAssertionPremise::Region { assertion: region.id(), support: support.id() }).id()).cloned().collect::<Vec<_>>();
                f.data.native = Rows::new(&f.budget);
                for native in retained { f.data.native.insert(native).unwrap(); }
            }
            7 => {
                let retained = f.data.native.iter().filter(|n| n.premise != (NativeAssertionPremise::Region { assertion: region.id(), support: support.id() }).id()).cloned().collect::<Vec<_>>();
                f.data.native = Rows::new(&f.budget);
                for native in retained { f.data.native.insert(native).unwrap(); }
                f.data.native.insert(NativeQualification { premise: NativeAssertionPremise::Region { assertion: region.id(), support: support.id() }.id(), qualification: region.qualification, family: FactFamily::Flow, fidelity: Fidelity::ReportProjection, status: EvidenceStatus::StructurallyObserved }).unwrap();
            }
            8 => {
                let source = f.data.entry.artifacts.get(f.read.source).unwrap();
                let foreign = SourceArtifact::from_bytes(source.input, "foreign.py".into(), b"foreign").unwrap();
                let scope = CoverageScope::Artifact { artifact: foreign.id() };
                let q = AssertionQualification { scope: scope.id(), ..f.data.entry.qualifications.get(region.qualification).unwrap().clone() };
                let altered = FlowRegionObservation { qualification: q.id(), ..region };
                let support = FlowRegionSupport { assertion: altered.id(), ..support };
                f.data.entry.artifacts.insert(foreign).unwrap();
                f.data.entry.scopes.insert(scope).unwrap();
                f.data.entry.qualifications.insert(q.clone()).unwrap();
                f.data.entry.regions = Rows::new(&f.budget);
                f.data.entry.regions.insert(altered.clone()).unwrap();
                f.data.entry.region_supports = Rows::new(&f.budget);
                f.data.entry.region_supports.insert(support.clone()).unwrap();
                f.data.native.insert(NativeQualification { premise: NativeAssertionPremise::Region { assertion: altered.id(), support: support.id() }.id(), qualification: q.id(), family: FactFamily::Flow, fidelity: Fidelity::NativeStructural, status: EvidenceStatus::StructurallyObserved }).unwrap();
            }
            _ => {
                let use_q = f.data.entry.use_observations.iter().next().unwrap().qualification;
                let alternative = FlowRegionObservation { qualification: use_q, ..region };
                let support = FlowRegionSupport { assertion: alternative.id(), ..support };
                f.data.entry.regions.insert(alternative.clone()).unwrap();
                f.data.entry.region_supports.insert(support.clone()).unwrap();
                f.data.native.insert(NativeQualification { premise: NativeAssertionPremise::Region { assertion: alternative.id(), support: support.id() }.id(), qualification: use_q, family: FactFamily::Flow, fidelity: Fidelity::NativeStructural, status: EvidenceStatus::StructurallyObserved }).unwrap();
            }
        }
        assert!(f.selected().is_none(), "conditional origin requires canonical exact native region; mutation {mutation}");
    }
}

#[test]
fn complete_inventory_does_not_make_a_call_continuation_predicate_unconditional() {
    for index in 0..3 {
        let mut f = Fixture::with_conditional_reaching(false, true);
        f.coverage(f.scopes[index].clone(), CoverageStatus::Partial);
        let observation = f.data.entry.use_observations.iter().next().unwrap();
        let support = f.data.entry.use_supports.iter().next().unwrap();
        let reaching = f.data.entry.reaching.iter().next().unwrap();
        let reaching_support = f.data.entry.reaching_supports.iter().next().unwrap();
        assert!(complete_native_singleton(
            &f.data.entry, observation, support, reaching, reaching_support, &f.budget,
        ).unwrap().is_some(), "the native enumeration itself is complete and authentic");
        assert!(f.selected().is_none(), "an Always structural use cannot imply an independent call-continuation predicate");
    }
}

#[test]
fn source_covering_input_artifact_and_module_receipts_admit_complete_native_singletons() {
    for status in [
        CoverageStatus::CompleteUnderStatedModel,
        CoverageStatus::Partial,
    ] {
        for index in 0..3 {
            let mut f = Fixture::new(false);
            f.coverage(f.scopes[index].clone(), status);
            let (origin, value) = f
                .selected()
                .expect("source-covering evidence and native singleton closure");
            assert_eq!(origin, f.origin.id());
            assert!(
                matches!(value, ValueSource::Named { coverage, .. } if coverage == f.coverage.id())
            );
        }
    }
}

#[test]
fn foreign_coverage_and_missing_or_incomplete_inventory_never_admit_named_origins() {
    for status in [
        CoverageStatus::CompleteUnderStatedModel,
        CoverageStatus::Partial,
    ] {
        let foreign_input = InputRevision::from_entries(vec![ManifestEntry {
            path: "foreign.py".into(),
            content: ContentHash::of(b"foreign"),
            byte_len: 7,
        }])
        .unwrap();
        let foreign =
            SourceArtifact::from_bytes(foreign_input.id(), "foreign.py".into(), b"foreign")
                .unwrap();
        let module = Module {
            source: foreign.id(),
            qualified_name: "foreign".into(),
        };
        for scope in [
            CoverageScope::Input {
                input: foreign_input.id(),
            },
            CoverageScope::Artifact {
                artifact: foreign.id(),
            },
            CoverageScope::Module {
                module: module.id(),
            },
        ] {
            let mut f = Fixture::new(false);
            f.data.entry.artifacts.insert(foreign.clone()).unwrap();
            f.data.entry.modules.insert(module.clone()).unwrap();
            f.coverage(scope, status);
            assert!(
                f.selected().is_none(),
                "foreign coverage never covers the selected source"
            );
        }
        for index in 0..3 {
            let mut incomplete = Fixture::new(true);
            incomplete.coverage(incomplete.scopes[index].clone(), status);
            assert!(
                incomplete.selected().is_none(),
                "family coverage cannot close a pruned per-use inventory"
            );
            let mut missing = Fixture::new(false);
            missing.coverage(missing.scopes[index].clone(), status);
            missing.data.entry.inventories = Rows::new(&missing.budget);
            assert!(
                missing.selected().is_none(),
                "family coverage cannot replace missing per-use authority"
            );
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
            let mut support = f
                .data
                .entry
                .inventory_supports
                .iter()
                .next()
                .unwrap()
                .clone();
            let premise = NativeAssertionPremise::FlowUseInventory {
                assertion: inventory.id(),
                support: support.id(),
            };
            let qualification = inventory.qualification;
            let retained = f
                .data
                .native
                .iter()
                .filter(|n| n.premise != premise.id())
                .cloned()
                .collect::<Vec<_>>();
            f.data.native = Rows::new(&f.budget);
            for native in retained {
                f.data.native.insert(native).unwrap();
            }
            if mutation == 0 {
                let mut run = f.data.entry.runs.get(support.run).unwrap().clone();
                run.configuration = ContentHash::of(b"foreign native invocation");
                support.run = run.id();
                f.data.entry.runs.insert(run).unwrap();
                f.data.entry.inventory_supports = Rows::new(&f.budget);
                f.data
                    .entry
                    .inventory_supports
                    .insert(support.clone())
                    .unwrap();
                // Re-pair the foreign support: refusal must be about its actual run, not a stale ID.
                f.data
                    .native
                    .insert(NativeQualification {
                        premise: NativeAssertionPremise::FlowUseInventory {
                            assertion: inventory.id(),
                            support: support.id(),
                        }
                        .id(),
                        qualification,
                        family: FactFamily::Flow,
                        fidelity: Fidelity::NativeStructural,
                        status: EvidenceStatus::StructurallyObserved,
                    })
                    .unwrap();
            }
            assert!(
                f.selected().is_none(),
                "covering Partial evidence cannot replace same-run native inventory premises"
            );
        }
    }
}
