#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
//! `def f(a, b=1)`, `class C` and the call `f(a, b=2)` with a complete provenance chain: symbol and
//! parameter declaration links, and the call's syntax and ordered arguments. Mutators produce the
//! refused variants.
use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::*, conditions::*, declarations::*, input::*,
     source::*, *,
};
use std::collections::BTreeMap;

pub const BYTES: &[u8] =
    include_bytes!("../../../../fixtures/python/semantic_declarations/example.py");
pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub qualification: AssertionQualification,
    pub run: ProviderRun,
    pub surface: ProviderSurface,
    pub definition: Occurrence,
    pub class: Occurrence,
    pub a: Occurrence,
    pub b: Occurrence,
    pub stray: Occurrence,
    pub call_site: Occurrence,
    pub function: ProviderSymbol,
    pub class_symbol: ProviderSymbol,
    pub members: Vec<SignatureParameter>,
    pub call: CallSyntax,
    pub arguments: Vec<CallArgument>,
    symbol_declarations: Vec<SymbolDeclaration>,
    parameter_declarations: Vec<ParameterDeclaration>,
}
fn at(text: &str) -> i64 {
    std::str::from_utf8(BYTES).unwrap().find(text).unwrap() as i64
}
impl Fixture {
    pub fn new() -> Self {
        let model = ValidatedModel::declared(facts_relations()).unwrap();
        let input = InputRevision::from_entries(vec![ManifestEntry {
            path: "example.py".into(),
            content: ContentHash::of(BYTES),
            byte_len: BYTES.len() as i64,
        }])
        .unwrap();
        let source = SourceArtifact::from_bytes(input.id(), "example.py".into(), BYTES).unwrap();
        let origin = InputOrigin::Tree {
            label: "declaration contract".into(),
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
            tool: "declaration-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Signatures, FactFamily::Syntax],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Signatures,
            name: "declarations".into(),
        };
        let syntax_surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Syntax,
            name: "call syntax".into(),
        };
        let scope = CoverageScope::Artifact {
            artifact: source.id(),
        };
        let (condition, nodes) = Diagram::always().records();
        let qualification = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let occurrence =
            |start: i64, end: i64, kind: SyntaxKind, role: OccurrenceRole, path: Vec<i32>| {
                Occurrence {
                    source: source.id(),
                    start,
                    end,
                    syntax_kind: kind,
                    role,
                    structural_path: path,
                }
            };
        let root = occurrence(
            0,
            BYTES.len() as i64,
            SyntaxKind::ModModule,
            OccurrenceRole::Syntax,
            vec![0],
        );
        let definition = occurrence(
            0,
            at("class"),
            SyntaxKind::StmtFunctionDef,
            OccurrenceRole::Declaration,
            vec![0, 0],
        );
        let a = occurrence(
            at("a,"),
            at("a,") + 1,
            SyntaxKind::Parameter,
            OccurrenceRole::Parameter,
            vec![0, 0, 0],
        );
        let b = occurrence(
            at("b=1"),
            at("b=1") + 3,
            SyntaxKind::ParameterWithDefault,
            OccurrenceRole::Parameter,
            vec![0, 0, 1],
        );
        let class = occurrence(
            at("class"),
            at("f(a, b=2)"),
            SyntaxKind::StmtClassDef,
            OccurrenceRole::Declaration,
            vec![0, 1],
        );
        let stray = occurrence(
            at("C:"),
            at("C:") + 1,
            SyntaxKind::Parameter,
            OccurrenceRole::Parameter,
            vec![0, 1, 0],
        );
        let call_site = occurrence(
            at("f(a, b=2)"),
            at("f(a, b=2)") + 9,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            vec![0, 2, 0],
        );
        let callee = occurrence(
            call_site.start,
            call_site.start + 1,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 2, 0, 0],
        );
        let a_arg = occurrence(
            call_site.start + 2,
            call_site.start + 3,
            SyntaxKind::ExprName,
            OccurrenceRole::Argument,
            vec![0, 2, 0, 1],
        );
        let two = occurrence(
            at("2)"),
            at("2)") + 1,
            SyntaxKind::ExprNumberLiteral,
            OccurrenceRole::Argument,
            vec![0, 2, 0, 2],
        );
        let module = Module {
            source: source.id(),
            qualified_name: "example".into(),
        };
        let provider_module = ProviderModule::Acquired {
            module: module.id(),
        };
        let symbol = |key: &str, kind: SymbolKind| ProviderSymbol {
            provider: provider.id(),
            context: context.id(),
            module: provider_module.id(),
            native_key: format!("example.{key}"),
            name: key.into(),
            kind,
        };
        let (function, class_symbol) = (
            symbol("f", SymbolKind::Function),
            symbol("C", SymbolKind::Class),
        );
        let shapes = vec![
            ParameterShape {
                name: Some("a".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            },
            ParameterShape {
                name: Some("b".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: false,
            },
        ];
        let (signature, members) = Signature::new(
            &qualification,
            lctx_model::domain::calls::SignatureRole::Source,
            None,
            function.id(),
            0,
            SignatureForm::List,
            &shapes,
        )
        .unwrap();
        let (call, arguments) = CallSyntax::new(
            qualification.id(),
            call_site.id(),
            callee.id(),
            false,
            &[
                Actual {
                    occurrence: a_arg.id(),
                    kind: ArgumentKind::Positional,
                    keyword: None,
                },
                Actual {
                    occurrence: two.id(),
                    kind: ArgumentKind::Keyword,
                    keyword: Some("b".into()),
                },
            ],
        )
        .unwrap();
        let evidence = |occurrence: &Occurrence| Evidence::Occurrence {
            occurrence: occurrence.id(),
        };
        let mut fixture = Self {
            model,
            batches: BTreeMap::new(),
            qualification: qualification.clone(),
            run: run.clone(),
            surface: surface.clone(),
            definition: definition.clone(),
            class: class.clone(),
            a: a.clone(),
            b: b.clone(),
            stray: stray.clone(),
            call_site: call_site.clone(),
            function: function.clone(),
            class_symbol: class_symbol.clone(),
            members: members.clone(),
            call: call.clone(),
            arguments: arguments.clone(),
            symbol_declarations: vec![],
            parameter_declarations: vec![],
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(fixture.put(vec![$row.clone()]);)+ }; }
        one!(
            input,
            origin,
            acquisition,
            context,
            provider,
            run,
            scope,
            condition,
            qualification,
            module,
            provider_module,
            signature,
            call
        );
        fixture.put(vec![lctx_model::domain::assumptions::AssumptionSet::empty()]);
        fixture.put(families);
        fixture.put(nodes);
        fixture.put(vec![surface, syntax_surface.clone()]);
        fixture.put(ArtifactChunk::split(&source, BYTES).unwrap().collect());
        fixture.put(vec![source]);
        fixture.put(vec![
            root,
            definition.clone(),
            a.clone(),
            b.clone(),
            class.clone(),
            stray.clone(),
            call_site.clone(),
            callee,
            a_arg,
            two,
        ]);
        fixture.put(vec![function.clone(), class_symbol.clone()]);
        fixture.put(shapes);
        fixture.put(members.clone());
        fixture.put(vec![
            evidence(&definition),
            evidence(&class),
            evidence(&a),
            evidence(&b),
            evidence(&stray),
            evidence(&call_site),
        ]);
        fixture.put(vec![SignatureSupport {
            assertion: signature.id(),
            run: run.id(),
            surface: fixture.surface.id(),
            evidence: evidence(&definition).id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        }]);
        fixture.put(vec![CallSyntaxSupport {
            assertion: call.id(),
            run: run.id(),
            surface: syntax_surface.id(),
            evidence: evidence(&call_site).id(),
            origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        }]);
        fixture.put(arguments);
        fixture.set_symbols(vec![
            (function.id(), definition.id()),
            (class_symbol.id(), class.id()),
        ]);
        fixture.set_parameters(vec![(members[0].id(), a.id()), (members[1].id(), b.id())]);
        fixture
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
    fn support(
        &self,
        declaration: Id<Occurrence>,
        run: Id<ProviderRun>,
        surface: Id<ProviderSurface>,
    ) -> (Id<ProviderRun>, Id<ProviderSurface>, Id<Evidence>) {
        (
            run,
            surface,
            Evidence::Occurrence {
                occurrence: declaration,
            }
            .id(),
        )
    }
    /// Replace the symbol declaration links (and their native support).
    pub fn set_symbols(&mut self, links: Vec<(Id<ProviderSymbol>, Id<Occurrence>)>) {
        self.set_symbols_with(
            links,
            self.qualification.id(),
            self.run.id(),
            self.surface.id(),
        );
    }
    pub fn set_symbols_with(
        &mut self,
        links: Vec<(Id<ProviderSymbol>, Id<Occurrence>)>,
        qualification: Id<AssertionQualification>,
        run: Id<ProviderRun>,
        surface: Id<ProviderSurface>,
    ) {
        self.symbol_declarations = links
            .into_iter()
            .map(|(symbol, declaration)| SymbolDeclaration {
                qualification,
                symbol,
                declaration,
            })
            .collect();
        let supports = self
            .symbol_declarations
            .iter()
            .map(|row| {
                let (run, surface, evidence) = self.support(row.declaration, run, surface);
                SymbolDeclarationSupport {
                    assertion: row.id(),
                    run,
                    surface,
                    evidence,
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                }
            })
            .collect();
        self.put(self.symbol_declarations.clone());
        self.put::<SymbolDeclarationSupport>(supports);
    }
    /// Replace the parameter declaration links (and their native support).
    pub fn set_parameters(&mut self, links: Vec<(Id<SignatureParameter>, Id<Occurrence>)>) {
        let (qualification, run, surface) =
            (self.qualification.id(), self.run.id(), self.surface.id());
        self.parameter_declarations = links
            .into_iter()
            .map(|(parameter, declaration)| ParameterDeclaration {
                qualification,
                parameter,
                declaration,
            })
            .collect();
        let supports = self
            .parameter_declarations
            .iter()
            .map(|row| {
                let (run, surface, evidence) = self.support(row.declaration, run, surface);
                ParameterDeclarationSupport {
                    assertion: row.id(),
                    run,
                    surface,
                    evidence,
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                }
            })
            .collect();
        self.put(self.parameter_declarations.clone());
        self.put::<ParameterDeclarationSupport>(supports);
    }
    /// A second provider whose run and surface are otherwise valid for this input.
    pub fn alien(&mut self) -> (Id<ProviderRun>, Id<ProviderSurface>) {
        let provider = Provider {
            tool: "other-provider".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"other"),
        };
        let context = self.rows::<AnalysisContext>()[0].clone();
        let input = self.rows::<InputRevision>()[0].id();
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input,
            context.config_digest,
            [FactFamily::Signatures, FactFamily::Syntax],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Signatures,
            name: "declarations".into(),
        };
        let mut providers = self.rows::<Provider>();
        providers.push(provider);
        self.put(providers);
        let mut runs = self.rows::<ProviderRun>();
        runs.push(run.clone());
        self.put(runs);
        let mut members = self.rows::<RunFamily>();
        members.extend(families);
        self.put(members);
        let mut surfaces = self.rows::<ProviderSurface>();
        surfaces.push(surface.clone());
        self.put(surfaces);
        (run.id(), surface.id())
    }
    /// A qualification in another analysis context, over the same scope.
    pub fn foreign_qualification(&mut self) -> Id<AssertionQualification> {
        let context = AnalysisContext {
            python_platform: "darwin".into(),
            ..self.rows::<AnalysisContext>()[0].clone()
        };
        let qualification = AssertionQualification {
            context: context.id(),
            ..self.qualification.clone()
        };
        let mut contexts = self.rows::<AnalysisContext>();
        contexts.push(context);
        self.put(contexts);
        let mut qualifications = self.rows::<AssertionQualification>();
        qualifications.push(qualification.clone());
        self.put(qualifications);
        qualification.id()
    }
    /// Validate every stored relation and invariant, as the store would.
    pub fn validate(&self) -> Result<ContentHash, ModelError> {
        let mut generation = Vec::new();
        macro_rules! each { ($($ty:ty),+ $(,)?) => { $( generation.push((<$ty>::NAME,Batch::new(&self.model,self.rows::<$ty>(),&budget()).unwrap().arrow().clone())); )+ }; }
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
            Condition,
            ConditionNode,
            AssertionQualification,
            lctx_model::domain::assumptions::AssumptionSet,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            Module,
            ProviderModule,
            ProviderSymbol,
            ParameterShape,
            Signature,
            SignatureParameter,
            SignatureSupport,
            SymbolDeclaration,
            SymbolDeclarationSupport,
            ParameterDeclaration,
            ParameterDeclarationSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            Evidence
        );
        validation::replay::replay(&self.model,&generation,&budget())
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
