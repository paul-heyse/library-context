#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
//! `class Base` with a documented stub `run`, `class Service(Base, Generic[T])` with a static
//! `make`, an overriding `run` and a nested `inner`, and `__all__ = ["Service", "Missing"]`:
//! symbol, trait, ancestry, annotation, public-name, docstring and module-resolution records under
//! one input-scoped qualification with a complete provenance chain. Mutators produce the refused
//! variants.
use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::*, conditions::*, input::*,
    memory::MemoryGeneration, source::*, symbols::*, syntax::{ImportAliasObservation, ImportAliasSupport}, *,
};
use std::collections::BTreeMap;

pub const BYTES: &[u8] = include_bytes!("../../../../fixtures/python/semantic_symbols/example.py");
pub fn text() -> &'static str {
    std::str::from_utf8(BYTES).unwrap()
}
pub fn at(needle: &str) -> i64 {
    text().find(needle).unwrap() as i64
}

/// The relations the fixture fills, for a caller's typed put.
macro_rules! symbol_relations {
    ($apply:ident) => {
        $apply!(
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
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            Evidence,
            Module,
            ProviderModule,
            ProviderSymbol,
            ParameterShape,
            Signature,
            SignatureParameter,
            SignatureSupport,
            SymbolSequence,
            SymbolSequenceMember,
            SymbolObservation,
            SymbolSupport,
            FunctionTraitObservation,
            FunctionTraitSupport,
            ClassTraitObservation,
            ClassTraitSupport,
            ClassAncestryObservation,
            ClassAncestrySupport,
            ParameterAnnotationObservation,
            ParameterAnnotationSupport,
            ExportOrigin,
            PublicNameObservation,
            PublicNameSupport,
            ParameterDocObservation,
            ParameterDocSupport,
            ImportAliasObservation,
            ImportAliasSupport,
            ModuleResolutionObservation,
            ModuleResolutionSupport
        )
    };
}

pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub provider: Provider,
    pub alien: Provider,
    pub context: AnalysisContext,
    pub run: ProviderRun,
    pub alien_run: ProviderRun,
    pub surfaces: BTreeMap<FactFamily, ProviderSurface>,
    pub alien_surfaces: BTreeMap<FactFamily, ProviderSurface>,
    pub qualification: AssertionQualification,
    pub scope: CoverageScope,
    pub module: Module,
    pub other: Module,
    pub modules: BTreeMap<&'static str, ProviderModule>,
    pub sym: BTreeMap<&'static str, ProviderSymbol>,
    pub occ: BTreeMap<&'static str, Occurrence>,
    pub signature: Signature,
    pub members: Vec<SignatureParameter>,
    pub description: Evidence,
    pub sequences: Vec<(SymbolSequence, Vec<SymbolSequenceMember>)>,
    pub symbols: Vec<SymbolObservation>,
    pub functions: Vec<FunctionTraitObservation>,
    pub classes: Vec<ClassTraitObservation>,
    pub ancestry: Vec<ClassAncestryObservation>,
    pub annotations: Vec<ParameterAnnotationObservation>,
    pub origins: Vec<ExportOrigin>,
    pub public: Vec<PublicNameObservation>,
    pub docs: Vec<ParameterDocObservation>,
    pub module_resolutions: Vec<ModuleResolutionObservation>,
    pub imports: Vec<ImportAliasObservation>,
    /// The fidelity every annotation support states.
    pub annotation_fidelity: Fidelity,
    /// Supports state this run and surface family map (the alien provider's for a native-ownership control).
    pub supporting: Option<SupportingRun>,
}
impl Fixture {
    pub fn new() -> Self {
        let model = ValidatedModel::validate(facts_relations()).unwrap();
        let other_bytes: &[u8] = b"X = 1\n";
        let input = InputRevision::from_entries(vec![
            ManifestEntry {
                path: "example.py".into(),
                content: ContentHash::of(BYTES),
                byte_len: BYTES.len() as i64,
            },
            ManifestEntry {
                path: "other.py".into(),
                content: ContentHash::of(other_bytes),
                byte_len: other_bytes.len() as i64,
            },
        ])
        .unwrap();
        let source = SourceArtifact::from_bytes(input.id(), "example.py".into(), BYTES).unwrap();
        let other_source =
            SourceArtifact::from_bytes(input.id(), "other.py".into(), other_bytes).unwrap();
        let other = Module {
            source: other_source.id(),
            qualified_name: "other".into(),
        };
        let origin = InputOrigin::Tree {
            label: "symbol contract".into(),
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
            tool: "symbol-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let alien = Provider {
            tool: "other-provider".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"other"),
        };
        let families = [FactFamily::Signatures, FactFamily::Exports];
        let (run, run_families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            families,
        )
        .unwrap();
        let (alien_run, alien_families) = ProviderRun::new(
            alien.id(),
            context.id(),
            input.id(),
            context.config_digest,
            families,
        )
        .unwrap();
        let surfaces_of = |provider: &Provider| -> BTreeMap<FactFamily, ProviderSurface> {
            families
                .iter()
                .map(|family| {
                    (
                        *family,
                        ProviderSurface {
                            provider: provider.id(),
                            family: *family,
                            name: "symbols".into(),
                        },
                    )
                })
                .collect()
        };
        let (surfaces, alien_surfaces) = (surfaces_of(&provider), surfaces_of(&alien));
        let scope = CoverageScope::Input { input: input.id() };
        let (condition, nodes) = Diagram::always().records();
        let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let module = Module {
            source: source.id(),
            qualified_name: "example".into(),
        };
        let modules: BTreeMap<&'static str, ProviderModule> = [
            (
                "example",
                ProviderModule::Acquired {
                    module: module.id(),
                },
            ),
            ("other", ProviderModule::Acquired { module: other.id() }),
            (
                "typing",
                ProviderModule::Bundled {
                    provider: provider.id(),
                    bundle: ModuleBundle::Typeshed,
                    name: "typing".into(),
                },
            ),
            (
                "nspkg",
                ProviderModule::Namespace {
                    provider: provider.id(),
                    context: context.id(),
                    name: "nspkg".into(),
                },
            ),
            (
                "gone",
                ProviderModule::Unresolved {
                    provider: provider.id(),
                    context: context.id(),
                    name: "gone".into(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let symbol = |module: &str, key: &str, name: &str, kind: SymbolKind| ProviderSymbol {
            provider: provider.id(),
            context: context.id(),
            module: modules[module].id(),
            native_key: key.into(),
            name: name.into(),
            kind,
        };
        let sym: BTreeMap<&'static str, ProviderSymbol> = [
            ("Base", symbol("example", "C:0", "Base", SymbolKind::Class)),
            (
                "Base.run",
                symbol("example", "F:0", "run", SymbolKind::Method),
            ),
            (
                "Service",
                symbol("example", "C:1", "Service", SymbolKind::Class),
            ),
            (
                "Service.make",
                symbol("example", "F:1", "make", SymbolKind::Method),
            ),
            (
                "Service.run",
                symbol("example", "F:2", "run", SymbolKind::Method),
            ),
            (
                "inner",
                symbol("example", "F:3", "inner", SymbolKind::Function),
            ),
            (
                "Generic",
                symbol("typing", "C:40", "Generic", SymbolKind::Class),
            ),
        ]
        .into_iter()
        .collect();
        let base_end = at("\n\n\nclass Service");
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
        let occ: BTreeMap<&'static str, Occurrence> = [
            (
                "module",
                occurrence(
                    0,
                    BYTES.len() as i64,
                    SyntaxKind::ModModule,
                    OccurrenceRole::Syntax,
                    vec![0],
                ),
            ),
            (
                "Base",
                occurrence(
                    at("class Base"),
                    base_end,
                    SyntaxKind::StmtClassDef,
                    OccurrenceRole::Declaration,
                    vec![0, 3],
                ),
            ),
            (
                "Base.run",
                occurrence(
                    at("def run"),
                    base_end,
                    SyntaxKind::StmtFunctionDef,
                    OccurrenceRole::Declaration,
                    vec![0, 3, 0],
                ),
            ),
            (
                "Service",
                occurrence(
                    at("class Service"),
                    BYTES.len() as i64 - 1,
                    SyntaxKind::StmtClassDef,
                    OccurrenceRole::Declaration,
                    vec![0, 4],
                ),
            ),
        ]
        .into_iter()
        .collect();
        let shapes = vec![
            ParameterShape {
                name: Some("self".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            },
            ParameterShape {
                name: Some("timeout".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: false,
            },
        ];
        let (signature, members) = Signature::new(
            &qualification, lctx_model::domain::calls::SignatureRole::Source, None,
            sym["Base.run"].id(),
            0,
            SignatureForm::List,
            &shapes,
        )
        .unwrap();
        let described = at("Seconds to wait.");
        let description = Evidence::SourceSpan {
            source: source.id(),
            start: described,
            end: described + "Seconds to wait.".len() as i64,
        };
        let q = qualification.id();
        let id = |name: &str| sym[name].id();
        let empty = SymbolSequence::new(&[]).unwrap();
        let service_ancestors = SymbolSequence::new(&[id("Base"), id("Generic")]).unwrap();
        let traced = ExportOrigin::Traced {
            module: modules["example"].id(),
            name: "Service".into(),
            kind: Some(ExportKind::Class),
        };
        let traits = |symbol: &str, defining_class: Option<&str>| FunctionTraitObservation {
            qualification: q,
            symbol: id(symbol),
            overload: false,
            staticmethod: false,
            classmethod: false,
            property_getter: false,
            property_setter: false,
            stub: false,
            origin: FunctionOrigin::DefStatement,
            defining_class: defining_class.map(id),
            overrides: None,
        };
        let mut fixture = Self {
            model,
            batches: BTreeMap::new(),
            provider: provider.clone(),
            alien: alien.clone(),
            context: context.clone(),
            run: run.clone(),
            alien_run: alien_run.clone(),
            surfaces: surfaces.clone(),
            alien_surfaces: alien_surfaces.clone(),
            qualification: qualification.clone(),
            scope: scope.clone(),
            module: module.clone(),
            other: other.clone(),
            modules: modules.clone(),
            sym: sym.clone(),
            occ: occ.clone(),
            signature: signature.clone(),
            members: members.clone(),
            description: description.clone(),
            sequences: vec![empty.clone(), service_ancestors.clone()],
            symbols: vec![
                SymbolObservation {
                    qualification: q,
                    symbol: id("Base"),
                    parent: None,
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("Base.run"),
                    parent: Some(id("Base")),
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("Service"),
                    parent: None,
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("Service.make"),
                    parent: Some(id("Service")),
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("Service.run"),
                    parent: Some(id("Service")),
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("inner"),
                    parent: Some(id("Service.run")),
                },
                SymbolObservation {
                    qualification: q,
                    symbol: id("Generic"),
                    parent: None,
                },
            ],
            functions: vec![
                FunctionTraitObservation {
                    stub: true,
                    ..traits("Base.run", Some("Base"))
                },
                FunctionTraitObservation {
                    staticmethod: true,
                    ..traits("Service.make", Some("Service"))
                },
                FunctionTraitObservation {
                    overrides: Some(id("Base.run")),
                    ..traits("Service.run", Some("Service"))
                },
                FunctionTraitObservation {
                    stub: true,
                    ..traits("inner", None)
                },
            ],
            classes: ["Base", "Service"]
                .iter()
                .map(|name| ClassTraitObservation {
                    qualification: q,
                    symbol: id(name),
                    synthesized: false,
                    dataclass: false,
                    named_tuple: false,
                    typed_dict: false,
                })
                .collect(),
            ancestry: vec![
                ClassAncestryObservation {
                    qualification: q,
                    class: id("Base"),
                    relation: AncestryRelation::Bases,
                    ancestors: empty.0.id(),
                    linearization: None,
                },
                ClassAncestryObservation {
                    qualification: q,
                    class: id("Base"),
                    relation: AncestryRelation::Mro,
                    ancestors: empty.0.id(),
                    linearization: Some(Linearization::Complete),
                },
                ClassAncestryObservation {
                    qualification: q,
                    class: id("Service"),
                    relation: AncestryRelation::Bases,
                    ancestors: service_ancestors.0.id(),
                    linearization: None,
                },
                ClassAncestryObservation {
                    qualification: q,
                    class: id("Service"),
                    relation: AncestryRelation::Mro,
                    ancestors: service_ancestors.0.id(),
                    linearization: Some(Linearization::Complete),
                },
            ],
            annotations: vec![ParameterAnnotationObservation {
                qualification: q,
                scope: scope.id(),
                parameter: members[1].id(),
                display: "float".into(),
            }],
            origins: vec![traced.clone(), ExportOrigin::Untraced],
            public: vec![
                PublicNameObservation {
                    qualification: q,
                    access: module.id(),
                    name: "Service".into(),
                    via_dunder_all: true,
                    origin: traced.id(),
                },
                PublicNameObservation {
                    qualification: q,
                    access: module.id(),
                    name: "Missing".into(),
                    via_dunder_all: true,
                    origin: ExportOrigin::Untraced.id(),
                },
            ],
            docs: vec![ParameterDocObservation {
                qualification: q,
                declaration: occ["Base.run"].id(),
                name: "timeout".into(),
                text: "Seconds to wait.".into(),
                description: EvidenceSourceSpanId::of(&description).unwrap(),
            }],
            imports: vec![],
            module_resolutions: vec![
                ModuleResolutionObservation {
                    qualification: q,
                    alias: None,
                    module: modules["example"].id(),
                    location: None,
                },
                ModuleResolutionObservation {
                    qualification: q,
                    alias: None,
                    module: modules["typing"].id(),
                    location: Some("stdlib/typing.pyi".into()),
                },
                ModuleResolutionObservation {
                    qualification: q,
                    alias: None,
                    module: modules["nspkg"].id(),
                    location: Some("nspkg".into()),
                },
                ModuleResolutionObservation {
                    qualification: q,
                    alias: None,
                    module: modules["gone"].id(),
                    location: None,
                },
                ModuleResolutionObservation {
                    qualification: q,
                    alias: None,
                    module: modules["other"].id(),
                    location: None,
                },
            ],
            annotation_fidelity: Fidelity::DisplayOnly,
            supporting: None,
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(fixture.put(vec![$row.clone()]);)+ }; }
        one!(
            input,
            origin,
            acquisition,
            context,
            scope,
            condition,
            qualification,
            signature
        );
        fixture.put(vec![module, other]);
        fixture.put(vec![provider, alien]);
        fixture.put(vec![run.clone(), alien_run]);
        let mut families = run_families;
        families.extend(alien_families);
        fixture.put(families);
        fixture.put(nodes);
        fixture.put(
            surfaces
                .values()
                .chain(alien_surfaces.values())
                .cloned()
                .collect(),
        );
        let mut chunks: Vec<ArtifactChunk> =
            ArtifactChunk::split(&source, BYTES).unwrap().collect();
        chunks.extend(ArtifactChunk::split(&other_source, other_bytes).unwrap());
        fixture.put(chunks);
        fixture.put(vec![source, other_source]);
        fixture.put(occ.values().cloned().collect());
        fixture.put(shapes);
        fixture.put(members);
        fixture.put(vec![SignatureSupport {
            assertion: signature.id(),
            run: run.id(),
            surface: surfaces[&FactFamily::Signatures].id(),
            evidence: Evidence::Invocation { run: run.id() }.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        }]);
        fixture.sync();
        fixture
    }
    /// Actual `Generic` import bytes in the source, attached to a bundled module resolution.
    pub fn with_import_alias(mut self) -> Self {
        let start = at("from typing import");
        let end = text()[start as usize..].find('\n').unwrap() as i64 + start;
        let statement = Occurrence {
            source: self.module.source, start, end,
            syntax_kind: SyntaxKind::StmtImportFrom, role: OccurrenceRole::Syntax,
            structural_path: vec![90, 0],
        };
        let alias = Occurrence {
            start: at("Generic"), end: at("Generic") + 7,
            syntax_kind: SyntaxKind::Alias,
            structural_path: vec![90, 0, 0], ..statement.clone()
        };
        self.occ.insert("import_statement", statement.clone());
        self.occ.insert("import_alias", alias.clone());
        self.put(self.occ.values().cloned().collect());
        self.imports.push(ImportAliasObservation {
            qualification: self.qualification.id(), statement: statement.id(), alias: alias.id(),
            level: 0, resolved_module: Some("typing".into()),
        });
        self.module_resolutions.push(ModuleResolutionObservation {
            qualification: self.qualification.id(), module: self.modules["typing"].id(),
            alias: Some(alias.id()), location: Some("stdlib/typing.pyi".into()),
        });
        self.sync();
        self
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
    /// Store every record, its vocabulary and its support.
    pub fn sync(&mut self) {
        let (run, surfaces) = self.supporting.clone().unwrap_or_else(|| {
            (
                self.run.id(),
                self.surfaces
                    .iter()
                    .map(|(family, surface)| (*family, surface.id()))
                    .collect(),
            )
        });
        let invocation = Evidence::Invocation { run };
        let mut evidence = vec![
            invocation.clone(),
            self.description.clone(),
            Evidence::Invocation { run: self.run.id() },
        ];
        evidence.extend(self.imports.iter().map(|i| Evidence::Occurrence { occurrence: i.alias }));
        evidence.sort_by_key(Record::id);
        evidence.dedup();
        self.put(evidence);
        self.put(self.modules.values().cloned().collect());
        self.put(self.sym.values().cloned().collect());
        self.put(
            self.sequences
                .iter()
                .map(|(sequence, _)| sequence.clone())
                .collect(),
        );
        self.put(
            self.sequences
                .iter()
                .flat_map(|(_, members)| members.clone())
                .collect(),
        );
        self.put(self.origins.clone());
        macro_rules! with_support {
            ($rows:expr, $support:ident, $family:expr, $fidelity:expr, $evidence:expr) => {{
                let rows = $rows.clone();
                let supports: Vec<$support> = rows
                    .iter()
                    .map(|row| $support {
                        assertion: row.id(),
                        run,
                        surface: surfaces[&$family],
                        evidence: $evidence(row),
                        origin: Origin::AnalyzerAssertion,
                        mode: ExtractionMode::NativeTraversal,
                        fidelity: $fidelity,
                    })
                    .collect();
                self.put(rows);
                self.put(supports);
            }};
        }
        with_support!(
            self.symbols,
            SymbolSupport,
            FactFamily::Signatures,
            Fidelity::NativeStructural,
            |_: &SymbolObservation| invocation.id()
        );
        with_support!(
            self.functions,
            FunctionTraitSupport,
            FactFamily::Signatures,
            Fidelity::NativeStructural,
            |_: &FunctionTraitObservation| invocation.id()
        );
        with_support!(
            self.classes,
            ClassTraitSupport,
            FactFamily::Signatures,
            Fidelity::NativeStructural,
            |_: &ClassTraitObservation| invocation.id()
        );
        with_support!(
            self.ancestry,
            ClassAncestrySupport,
            FactFamily::Signatures,
            Fidelity::NativeStructural,
            |_: &ClassAncestryObservation| invocation.id()
        );
        with_support!(
            self.annotations,
            ParameterAnnotationSupport,
            FactFamily::Signatures,
            self.annotation_fidelity,
            |_: &ParameterAnnotationObservation| invocation.id()
        );
        with_support!(
            self.public,
            PublicNameSupport,
            FactFamily::Exports,
            Fidelity::NativeStructural,
            |_: &PublicNameObservation| invocation.id()
        );
        with_support!(
            self.docs,
            ParameterDocSupport,
            FactFamily::Signatures,
            Fidelity::NativeStructural,
            |row: &ParameterDocObservation| row.description.id()
        );
        with_support!(
            self.imports,
            ImportAliasSupport,
            FactFamily::Exports,
            Fidelity::NativeStructural,
            |row: &ImportAliasObservation| Evidence::Occurrence { occurrence: row.alias }.id()
        );
        with_support!(
            self.module_resolutions,
            ModuleResolutionSupport,
            FactFamily::Exports,
            Fidelity::NativeStructural,
            |_: &ModuleResolutionObservation| invocation.id()
        );
    }
    /// Support every record by the alien provider's run instead.
    pub fn alien_support(&mut self) {
        self.supporting = Some((
            self.alien_run.id(),
            self.alien_surfaces
                .iter()
                .map(|(family, surface)| (*family, surface.id()))
                .collect(),
        ));
        self.sync();
    }
    /// A symbol of the fixture's provider in another analysis context (with its context row stored).
    pub fn foreign_symbol(&mut self, name: &str) -> ProviderSymbol {
        let context = AnalysisContext {
            python_platform: "darwin".into(),
            ..self.context.clone()
        };
        let symbol = ProviderSymbol {
            context: context.id(),
            native_key: format!("foreign.{name}"),
            ..self.sym[name].clone()
        };
        self.put(vec![self.context.clone(), context]);
        symbol
    }
    /// Validate every stored relation and invariant, as the store would.
    pub fn validate(&self) -> Result<ContentHash, ModelError> {
        let generation = MemoryGeneration::conformance(&self.model, &budget());
        macro_rules! each { ($($ty:ty),+ $(,)?) => { $( generation.put(&Batch::new(&self.model, self.rows::<$ty>(), &budget()).unwrap())?; )+ }; }
        symbol_relations!(each);
        generation.validate(&self.model, &budget())
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
pub fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}

type SupportingRun = (Id<ProviderRun>, BTreeMap<FactFamily, Id<ProviderSurface>>);
