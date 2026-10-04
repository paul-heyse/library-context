//! Exact minimal native premises for the shared parameter-entry operation.
#![allow(dead_code, reason = "Shared targeted stability fixture")]
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    conditions::{entry::*, *},
    declarations::*,
    flow::*,
    input::*,
    lexical::*,
    normalized::entities::*,
    resources::ResourceBudget,
    source::*,
    value::*,
    *,
};
pub struct Fixture {
    pub data: EntryData,
    pub request: EntryRequest,
    pub budget: ResourceBudget,
    pub guard: EvaluationAtom,
    pub q: AssertionQualification,
    pub definition: FlowDefinitionObservation,
    pub reaching: FlowReachingObservation,
    pub use_: FlowUse,
    pub coverage: ProviderCoverage,
}
impl Fixture {
    pub fn new() -> Self {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = EntryData::new(&budget);
        let bytes = b"def f(value):\n    if value is None:\n        return value\n";
        let input = InputRevision::from_entries(vec![ManifestEntry {
            path: "case.py".into(),
            content: ContentHash::of(bytes),
            byte_len: bytes.len() as i64,
        }])
        .unwrap();
        let artifact = SourceArtifact::from_bytes(input.id(), "case.py".into(), bytes).unwrap();
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
            tool: "exact-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let (run, _) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Flow, FactFamily::Signatures],
        )
        .unwrap();
        let flow_surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Flow,
            name: "flow".into(),
        };
        let signature_surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Signatures,
            name: "signature".into(),
        };
        let evidence = Evidence::Invocation { run: run.id() };
        let scope = CoverageScope::Input { input: input.id() };
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let occurrence = |start, end, syntax_kind, role, path| Occurrence {
            source: artifact.id(),
            start,
            end,
            syntax_kind,
            role,
            structural_path: path,
        };
        let owner = occurrence(
            0,
            bytes.len() as i64,
            SyntaxKind::StmtFunctionDef,
            OccurrenceRole::Declaration,
            vec![0, 0],
        );
        let parameter = occurrence(
            6,
            11,
            SyntaxKind::Parameter,
            OccurrenceRole::Parameter,
            vec![0, 0, 0],
        );
        let evaluation = occurrence(
            21,
            34,
            SyntaxKind::ExprCompare,
            OccurrenceRole::Predicate,
            vec![0, 0, 1, 0],
        );
        let read = occurrence(
            21,
            26,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 0, 1, 0, 0],
        );
        let callable = CallableEntity::Source {
            declaration: owner.id(),
            kind: CallableKind::Function,
        };
        let owner_ref = EntityRef::Callable {
            callable: callable.id(),
        };
        let formal = ParameterEntity::Source {
            declaration: parameter.id(),
        };
        let root = PlaceRoot::Formal {
            declaration: parameter.id(),
        };
        let path = AccessPath::empty();
        let place = Place {
            root: root.id(),
            path: path.id(),
        };
        let lexical = LexicalScope {
            owner: owner.id(),
            kind: LexicalScopeKind::Function,
        };
        let module = ProviderModule::Unresolved {
            provider: provider.id(),
            context: context.id(),
            name: "case".into(),
        };
        let symbol = ProviderSymbol {
            provider: provider.id(),
            context: context.id(),
            module: module.id(),
            native_key: "f".into(),
            name: "f".into(),
            kind: SymbolKind::Function,
        };
        let (signature, parameters) = Signature::new(
            &q,
            lctx_model::domain::calls::SignatureRole::Source,
            None,
            symbol.id(),
            0,
            SignatureForm::List,
            &[ParameterShape {
                name: Some("value".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            }],
        )
        .unwrap();
        let parameter_row = parameters[0].clone();
        let declaration = ParameterDeclaration {
            qualification: q.id(),
            parameter: parameter_row.id(),
            declaration: parameter.id(),
        };
        let owner_declaration = SymbolDeclaration {
            qualification: q.id(),
            symbol: symbol.id(),
            declaration: owner.id(),
        };
        let link = ParameterEntityLink {
            parameter: parameter_row.id(),
            entity: formal.id(),
            declaration: Some(declaration.id()),
        };
        let use_ = FlowUse {
            occurrence: read.id(),
            place: place.id(),
        };
        let use_observation = FlowUseObservation {
            qualification: q.id(),
            use_: use_.id(),
            scope: lexical.id(),
            annotation: false,
        };
        let definition_row = FlowDefinition {
            occurrence: parameter.id(),
            place: place.id(),
        };
        let definition = FlowDefinitionObservation {
            qualification: q.id(),
            definition: definition_row.id(),
            scope: lexical.id(),
            kind: BindingEventKind::Parameter,
            value: None,
        };
        let target = ReachingDefinition::Bound {
            definition: definition_row.id(),
        };
        let reaching = FlowReachingObservation {
            qualification: q.id(),
            use_: use_.id(),
            target: target.id(),
            loop_carried: false,
        };
        let coverage = ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Flow,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        let predicate = Predicate::IsNone;
        let guard = EvaluationAtom {
            evaluation: evaluation.id(),
            context: context.id(),
            predicate: predicate.id(),
            operand: Some(place.id()),
        };
        macro_rules! insert {($field:ident,$($row:expr),+)=>{$(data.$field.insert($row.clone()).unwrap();)+};}
        insert!(artifacts, artifact);
        insert!(scopes, scope);
        insert!(qualifications, q);
        insert!(occurrences, owner, parameter, evaluation, read);
        insert!(callables, callable);
        insert!(refs, owner_ref);
        insert!(formals, formal);
        insert!(links, link);
        insert!(roots, root);
        insert!(paths, path);
        insert!(places, place);
        insert!(lexical_scopes, lexical);
        insert!(providers, provider);
        insert!(runs, run);
        insert!(surfaces, flow_surface, signature_surface);
        insert!(evidence, evidence);
        insert!(symbols, symbol);
        insert!(signatures, signature);
        insert!(parameters, parameter_row);
        insert!(declarations, declaration);
        insert!(symbol_declarations, owner_declaration);
        insert!(uses, use_);
        insert!(use_observations, use_observation);
        insert!(definitions, definition_row);
        insert!(definition_observations, definition);
        insert!(targets, target);
        insert!(reaching, reaching);
        insert!(coverage, coverage);
        insert!(predicates, predicate);
        insert!(atoms, guard);
        let (c, n) = Diagram::always().records();
        data.conditions.insert(c).unwrap();
        for node in n {
            data.condition_nodes.insert(node).unwrap();
        }
        for occurrence in [parameter.id(), read.id()] {
            data.owners
                .insert(OccurrenceOwnership {
                    occurrence,
                    owner: owner.id(),
                    entity: owner_ref.id(),
                })
                .unwrap();
        }
        macro_rules! support {
            ($field:ident,$ty:ident,$assertion:expr,$surface:expr,$fidelity:expr) => {
                data.$field
                    .insert($ty {
                        assertion: $assertion,
                        run: run.id(),
                        surface: $surface,
                        evidence: evidence.id(),
                        origin: Origin::AnalyzerAssertion,
                        mode: ExtractionMode::NativeTraversal,
                        fidelity: $fidelity,
                    })
                    .unwrap();
            };
        }
        support!(
            use_supports,
            FlowUseSupport,
            use_observation.id(),
            flow_surface.id(),
            Fidelity::NativeStructural
        );
        support!(
            definition_supports,
            FlowDefinitionSupport,
            definition.id(),
            flow_surface.id(),
            Fidelity::NativeStructural
        );
        support!(
            reaching_supports,
            FlowReachingSupport,
            reaching.id(),
            flow_surface.id(),
            Fidelity::NativeStructural
        );
        support!(
            declaration_supports,
            ParameterDeclarationSupport,
            declaration.id(),
            signature_surface.id(),
            Fidelity::ReportProjection
        );
        support!(
            symbol_declaration_supports,
            SymbolDeclarationSupport,
            owner_declaration.id(),
            signature_surface.id(),
            Fidelity::ReportProjection
        );
        let statement = occurrence(
            17,
            46,
            SyntaxKind::StmtIf,
            OccurrenceRole::Syntax,
            vec![0, 0, 1],
        );
        data.occurrences.insert(statement.clone()).unwrap();
        data.owners
            .insert(OccurrenceOwnership {
                occurrence: statement.id(),
                owner: owner.id(),
                entity: owner_ref.id(),
            })
            .unwrap();
        let region = FlowRegionObservation {
            qualification: q.id(),
            statement: statement.id(),
            scope: lexical.id(),
        };
        data.regions.insert(region.clone()).unwrap();
        support!(
            region_supports,
            FlowRegionSupport,
            region.id(),
            flow_surface.id(),
            Fidelity::NativeStructural
        );
        let leaf_condition = Diagram::from_atom(guard.id());
        let (condition, nodes) = leaf_condition.records();
        data.conditions.insert(condition).unwrap();
        for row in nodes {
            data.condition_nodes.insert(row).unwrap();
        }
        let leaf_q = AssertionQualification {
            condition: leaf_condition.id(),
            ..q.clone()
        };
        data.qualifications.insert(leaf_q.clone()).unwrap();
        let leaf = FlowTestLeafObservation {
            qualification: leaf_q.id(),
            test: evaluation.id(),
            atom: guard.id(),
            operand: Some(read.id()),
        };
        data.leaves.insert(leaf.clone()).unwrap();
        support!(
            leaf_supports,
            FlowTestLeafSupport,
            leaf.id(),
            flow_surface.id(),
            Fidelity::NativeStructural
        );
        Self {
            data,
            request: EntryRequest {
                owner: owner_ref.id(),
                formal: formal.id(),
                access: read.id(),
                context: context.id(),
                run: run.id(),
            },
            budget,
            guard,
            q,
            definition,
            reaching,
            use_,
            coverage,
        }
    }
    pub fn derive(&self) -> Result<DerivedEntryValue, ObligationKind> {
        EntryValueWitness::derive(&self.data, self.request, &self.budget).unwrap()
    }
    pub fn guard_entry(&self) -> Result<DerivedEntryValue, ObligationKind> {
        let leaf = self.data.leaves.iter().next().unwrap();
        let support = self
            .data
            .leaf_supports
            .iter()
            .find(|s| s.assertion == leaf.id())
            .unwrap();
        let source = EntryAccessSource::guard(&self.data, self.request, leaf.id(), support.id())?;
        EntryValueWitness::derive_for(&self.data, self.request, &source, &self.budget).unwrap()
    }
    pub fn validate(&self, witness: &EntryValueWitness) -> Result<(), ModelError> {
        let mut check = (entry_invariants().remove(0).create)(&self.budget);
        macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&self.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(input);
        let use_source = EntryAccessSource::Use {
            observation: witness.use_observation,
            support: witness.use_support,
        };
        let source = if use_source.id() == witness.access_source {
            use_source
        } else {
            let leaf = self.data.leaves.iter().next().unwrap();
            let support = self
                .data
                .leaf_supports
                .iter()
                .find(|s| s.assertion == leaf.id())
                .unwrap();
            let region = self.data.regions.iter().next().unwrap();
            let region_support = self
                .data
                .region_supports
                .iter()
                .find(|s| s.assertion == region.id())
                .unwrap();
            EntryAccessSource::Guard {
                observation: leaf.id(),
                support: support.id(),
                region: region.id(),
                region_support: region_support.id(),
            }
        };
        check
            .visit(
                EntryAccessSource::NAME,
                &<EntryAccessSource as Record>::encode(&[source]).unwrap(),
            )
            .unwrap();
        check
            .visit(
                EntryValueWitness::NAME,
                &EntryValueWitness::encode(std::slice::from_ref(witness)).unwrap(),
            )
            .unwrap();
        check.finish()
    }
}
