//! `select_timeout(t)` calling `def select_timeout(timeout=None): if timeout is None: ...` with a
//! complete provenance chain: the callee guard atom, flow facts showing only the parameter
//! definition reaches the guard's read, complete flow coverage, the stability witness, the call's
//! syntax and the substituted caller-side `BoundGuard`. Mutators produce the refused variants.
#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*,
    assertion::*,
    attribution::*,
    calls::*,
    conditions::{rebase::*, stability::*, *},
    flow::*,
    input::*,
    lexical::*,
    memory::MemoryGeneration,
    source::*,
    value::*,
    *,
};
use std::collections::BTreeMap;

pub const CALLEE: &[u8] =
    include_bytes!("../../../../fixtures/python/semantic_stability/callee.py");
pub const CALLER: &[u8] =
    include_bytes!("../../../../fixtures/python/semantic_stability/caller.py");
fn at(bytes: &[u8], text: &str) -> i64 {
    std::str::from_utf8(bytes).unwrap().find(text).unwrap() as i64
}

pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub context: AnalysisContext,
    pub qualification: AssertionQualification,
    pub run: ProviderRun,
    pub surface: ProviderSurface,
    pub parameter: Occurrence,
    pub evaluation: Occurrence,
    pub read: Occurrence,
    pub site: Occurrence,
    pub callee_name: Occurrence,
    pub actual: Occurrence,
    pub formal: PlaceRoot,
    pub place: Place,
    pub guard: EvaluationAtom,
    pub predicate: Predicate,
    pub use_: FlowUse,
    pub definition: FlowDefinition,
    pub definition_observation: FlowDefinitionObservation,
    pub target: ReachingDefinition,
    pub reaching: FlowReachingObservation,
    pub coverage: ProviderCoverage,
    pub scope: LexicalScope,
    pub call: CallSyntax,
    pub arguments: Vec<CallArgument>,
    pub witness: StabilityWitness,
}
impl Fixture {
    pub fn new() -> Self {
        // This retained analysis fixture exercises the P0 composition contract. It does not
        // synthesize N1-N5 outputs; their totality is tested by normalized pipeline fixtures.
        let mut relations = facts_relations();
        relations.extend(analysis_relations());
        let model = ValidatedModel::validate(relations).unwrap();
        let input = InputRevision::from_entries(vec![
            ManifestEntry {
                path: "callee.py".into(),
                content: ContentHash::of(CALLEE),
                byte_len: CALLEE.len() as i64,
            },
            ManifestEntry {
                path: "caller.py".into(),
                content: ContentHash::of(CALLER),
                byte_len: CALLER.len() as i64,
            },
        ])
        .unwrap();
        let callee = SourceArtifact::from_bytes(input.id(), "callee.py".into(), CALLEE).unwrap();
        let caller = SourceArtifact::from_bytes(input.id(), "caller.py".into(), CALLER).unwrap();
        let origin = InputOrigin::Tree {
            label: "stability contract".into(),
        };
        let acquisition = InputAcquisition {
            input: input.id(),
            origin: origin.id(),
        };
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let provider = Provider {
            tool: "stability-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Flow, FactFamily::Syntax],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Flow,
            name: "flow".into(),
        };
        let syntax_surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Syntax,
            name: "call syntax".into(),
        };
        let coverage_scope = CoverageScope::Input { input: input.id() };
        let (condition, nodes) = Diagram::always().records();
        let qualification = AssertionQualification {
            context: context.id(),
            scope: coverage_scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let occurrence = |source: &SourceArtifact,
                          start: i64,
                          len: i64,
                          kind: SyntaxKind,
                          role: OccurrenceRole,
                          path: Vec<i32>| Occurrence {
            source: source.id(),
            start,
            end: start + len,
            syntax_kind: kind,
            role,
            structural_path: path,
        };
        let callee_root = occurrence(
            &callee,
            0,
            CALLEE.len() as i64,
            SyntaxKind::ModModule,
            OccurrenceRole::Syntax,
            vec![0],
        );
        let def = occurrence(
            &callee,
            0,
            CALLEE.len() as i64,
            SyntaxKind::StmtFunctionDef,
            OccurrenceRole::Declaration,
            vec![0, 0],
        );
        let parameter = occurrence(
            &callee,
            at(CALLEE, "timeout=None"),
            7,
            SyntaxKind::Parameter,
            OccurrenceRole::Parameter,
            vec![0, 0, 0],
        );
        let evaluation = occurrence(
            &callee,
            at(CALLEE, "timeout is None"),
            15,
            SyntaxKind::ExprCompare,
            OccurrenceRole::Predicate,
            vec![0, 0, 1, 0],
        );
        let statement = occurrence(
            &callee,
            at(CALLEE, "if timeout"),
            30,
            SyntaxKind::StmtIf,
            OccurrenceRole::Syntax,
            vec![0, 0, 1],
        );
        let read = occurrence(
            &callee,
            at(CALLEE, "timeout is None"),
            7,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 0, 1, 0, 0],
        );
        let caller_root = occurrence(
            &caller,
            0,
            CALLER.len() as i64,
            SyntaxKind::ModModule,
            OccurrenceRole::Syntax,
            vec![0],
        );
        let expression = occurrence(
            &caller,
            0,
            17,
            SyntaxKind::StmtExpr,
            OccurrenceRole::Syntax,
            vec![0, 0],
        );
        let site = occurrence(
            &caller,
            0,
            17,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            vec![0, 0, 0],
        );
        let callee_name = occurrence(
            &caller,
            0,
            14,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 0, 0, 0],
        );
        let actual = occurrence(
            &caller,
            at(CALLER, "t)"),
            1,
            SyntaxKind::ExprName,
            OccurrenceRole::Argument,
            vec![0, 0, 0, 1],
        );
        let scope = LexicalScope {
            owner: def.id(),
            kind: LexicalScopeKind::Function,
        };
        let formal = PlaceRoot::Formal {
            declaration: parameter.id(),
        };
        let place = Place {
            root: formal.id(),
            path: AccessPath::empty().id(),
        };
        let predicate = Predicate::IsNone;
        let guard = EvaluationAtom {
            evaluation: evaluation.id(),
            context: context.id(),
            predicate: predicate.id(),
            operand: Some(place.id()),
        };
        let use_ = FlowUse {
            occurrence: read.id(),
            place: place.id(),
        };
        let definition = FlowDefinition {
            occurrence: parameter.id(),
            place: place.id(),
        };
        let definition_observation = FlowDefinitionObservation {
            qualification: qualification.id(),
            definition: definition.id(),
            scope: scope.id(),
            kind: BindingEventKind::Parameter,
            value: None,
        };
        let target = ReachingDefinition::Bound {
            definition: definition.id(),
        };
        let reaching = FlowReachingObservation {
            qualification: qualification.id(),
            use_: use_.id(),
            target: target.id(),
            loop_carried: false,
        };
        let coverage = ProviderCoverage {
            scope: coverage_scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Flow,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        let (call, arguments) = CallSyntax::new(
            qualification.id(),
            site.id(),
            callee_name.id(),
            false,
            &[Actual {
                occurrence: actual.id(),
                kind: ArgumentKind::Positional,
                keyword: None,
            }],
        )
        .unwrap();
        let witness = StabilityWitness {
            atom: guard.id(),
            reaching: reaching.id(),
            definition: definition_observation.id(),
            coverage: coverage.id(),
            basis: StabilityBasis::ParameterOnlyReaching,
        };
        let mut f = Self {
            model,
            batches: BTreeMap::new(),
            context: context.clone(),
            qualification: qualification.clone(),
            run: run.clone(),
            surface: surface.clone(),
            parameter: parameter.clone(),
            evaluation: evaluation.clone(),
            read: read.clone(),
            site: site.clone(),
            callee_name: callee_name.clone(),
            actual: actual.clone(),
            formal: formal.clone(),
            place: place.clone(),
            guard: guard.clone(),
            predicate: predicate.clone(),
            use_: use_.clone(),
            definition: definition.clone(),
            definition_observation,
            target,
            reaching,
            coverage,
            scope: scope.clone(),
            call: call.clone(),
            arguments: arguments.clone(),
            witness,
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(f.put(vec![$row.clone()]);)+ }; }
        one!(
            input,
            origin,
            acquisition,
            context,
            provider,
            run,
            coverage_scope,
            condition,
            qualification,
            scope,
            use_,
            definition,
            call
        );
        f.put(families);
        f.put(nodes);
        f.put(vec![surface, syntax_surface.clone()]);
        f.put(arguments);
        f.put(vec![callee.clone(), caller.clone()]);
        f.put(
            ArtifactChunk::split(&callee, CALLEE)
                .unwrap()
                .chain(ArtifactChunk::split(&caller, CALLER).unwrap())
                .collect(),
        );
        f.put(vec![
            callee_root,
            def,
            parameter.clone(),
            statement,
            evaluation,
            read.clone(),
            caller_root,
            expression,
            site.clone(),
            callee_name,
            actual,
        ]);
        f.put(vec![CallSyntaxSupport {
            assertion: call.id(),
            run: run.id(),
            surface: syntax_surface.id(),
            evidence: Evidence::Occurrence {
                occurrence: site.id(),
            }
            .id(),
            origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        }]);
        f.store_flow();
        f.store_substitution();
        f
    }
    pub fn put<R: Record>(&mut self, rows: Vec<R>) {
        self.batches.insert(
            R::NAME,
            Batch::new(&self.model, rows, &budget())
                .unwrap()
                .arrow()
                .clone(),
        );
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        self.batches
            .get(R::NAME)
            .map(|b| R::decode(b).unwrap())
            .unwrap_or_default()
    }
    fn flow_support<S: Record>(
        &self,
        make: impl Fn(Id<ProviderRun>, Id<ProviderSurface>, Id<Evidence>) -> S,
        evidence: Id<Occurrence>,
    ) -> S {
        make(
            self.run.id(),
            self.surface.id(),
            Evidence::Occurrence {
                occurrence: evidence,
            }
            .id(),
        )
    }
    /// Store the flow facts, coverage and witness from the current fields.
    pub fn store_flow(&mut self) {
        self.witness = StabilityWitness {
            reaching: self.reaching.id(),
            definition: self.definition_observation.id(),
            coverage: self.coverage.id(),
            ..self.witness.clone()
        };
        let (definition, reaching) = (self.definition_observation.clone(), self.reaching.clone());
        let (o, m, fi) = (
            Origin::AnalyzerAssertion,
            ExtractionMode::NativeTraversal,
            Fidelity::NativeStructural,
        );
        let definition_support = self.flow_support(
            |run, surface, evidence| FlowDefinitionSupport {
                assertion: definition.id(),
                run,
                surface,
                evidence,
                origin: o,
                mode: m,
                fidelity: fi,
            },
            self.parameter.id(),
        );
        let reaching_support = self.flow_support(
            |run, surface, evidence| FlowReachingSupport {
                assertion: reaching.id(),
                run,
                surface,
                evidence,
                origin: o,
                mode: m,
                fidelity: fi,
            },
            self.read.id(),
        );
        self.put(vec![definition]);
        self.put(vec![definition_support]);
        self.put(vec![reaching]);
        self.put(vec![reaching_support]);
        self.put(vec![self.target.clone()]);
        self.put(vec![self.coverage.clone()]);
        self.put(vec![self.witness.clone()]);
        self.put(vec![
            Evidence::Occurrence {
                occurrence: self.parameter.id(),
            },
            Evidence::Occurrence {
                occurrence: self.read.id(),
            },
            Evidence::Occurrence {
                occurrence: self.site.id(),
            },
        ]);
    }
    /// Substitute the callee guard at the call and store the resulting caller-side rows.
    pub fn store_substitution(&mut self) {
        let rebased = self
            .substitute(
                Some(&BTreeMap::from([(self.guard.id(), self.witness.clone())])),
                &BTreeMap::from([(
                    self.formal.id(),
                    RootBinding::Actual(self.arguments[0].clone()),
                )]),
                &Diagram::from_atom(self.guard.id()),
            )
            .unwrap();
        let mut atoms = rebased.atoms;
        atoms.push(self.guard.clone());
        self.put(atoms);
        let mut predicates = rebased.predicates;
        predicates.push(self.predicate.clone());
        self.put(predicates);
        let mut roots = rebased.roots;
        roots.push(self.formal.clone());
        self.put(roots);
        let mut places = rebased.places;
        places.push(self.place.clone());
        self.put(places);
        self.put(vec![AccessPath::empty()]);
        self.put(rebased.substitutions);
    }
    pub fn substitute(
        &self,
        witnesses: Option<&BTreeMap<Id<EvaluationAtom>, StabilityWitness>>,
        bindings: &BTreeMap<Id<PlaceRoot>, RootBinding>,
        source: &Diagram,
    ) -> Result<RebasedGuards, ObligationKind> {
        let atoms = self
            .rows::<EvaluationAtom>()
            .into_iter()
            .chain([self.guard.clone()])
            .map(|a| (a.id(), a))
            .collect();
        let predicates = self
            .rows::<Predicate>()
            .into_iter()
            .chain([self.predicate.clone()])
            .map(|p| (p.id(), p))
            .collect();
        let places = self
            .rows::<Place>()
            .into_iter()
            .chain([self.place.clone()])
            .map(|p| (p.id(), p))
            .collect();
        let roots = self
            .rows::<PlaceRoot>()
            .into_iter()
            .chain([self.formal.clone()])
            .map(|r| (r.id(), r))
            .collect();
        substitute_call_guards(
            source,
            &self.site,
            self.context.id(),
            &GuardCatalog {
                atoms: &atoms,
                predicates: &predicates,
                places: &places,
                roots: &roots,
            },
            bindings,
            witnesses,
        )
    }
    /// Validate every stored relation and invariant, as the store would.
    pub fn validate(&self) -> Result<ContentHash, ModelError> {
        let generation = MemoryGeneration::conformance(&self.model, &budget());
        macro_rules! each { ($($ty:ty),+ $(,)?) => { $( generation.put(&Batch::new(&self.model, self.rows::<$ty>(), &budget()).unwrap())?; )+ }; }
        each!(
            InputRevision,
            InputOrigin,
            InputAcquisition,
            AnalysisContext,
            Provider,
            ProviderRun,
            RunFamily,
            ProviderSurface,
            CoverageScope,
            ProviderCoverage,
            Condition,
            ConditionNode,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            LexicalScope,
            PlaceRoot,
            AccessPath,
            Place,
            Predicate,
            EvaluationAtom,
            FlowUse,
            FlowDefinition,
            ReachingDefinition,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            Evidence,
            StabilityWitness,
            GuardSubstitution
        );
        generation.validate(&self.model, &budget())
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
