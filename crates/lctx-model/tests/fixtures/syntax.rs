#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
//! `@dec def f(a, b=1, *, c: int = 2)` with a docstring and two imports, `__all__ = ["f"]` and
//! `class C: y: int = 3`, with a complete provenance chain for every typed syntax record. Every
//! occurrence is located in the real bytes, so each record's texts are those bytes. Mutators
//! produce the refused variants.
use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::ParameterKind, conditions::*, input::*,
    lexical::SyntaxField, memory::MemoryGeneration, source::*, syntax::*, value::*, *,
};
use std::collections::BTreeMap;

pub const BYTES: &[u8] = include_bytes!("../../../../fixtures/python/semantic_syntax/example.py");
pub fn text() -> &'static str {
    std::str::from_utf8(BYTES).unwrap()
}
/// The byte offset of the `nth` occurrence of `needle`.
pub fn at_nth(needle: &str, nth: usize) -> i64 {
    text().match_indices(needle).nth(nth).unwrap().0 as i64
}
pub fn at(needle: &str) -> i64 {
    at_nth(needle, 0)
}
pub fn slice(occurrence: &Occurrence) -> &'static str {
    &text()[occurrence.start as usize..occurrence.end as usize]
}

/// The relations the fixture fills, for a caller's typed put.
macro_rules! syntax_relations {
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
            assumptions::AssumptionSet,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            Evidence,
            Literal,
            LiteralSet,
            LiteralSetMember,
            SyntaxDetail,
            SyntaxPlacement,
            SyntaxPlacementSupport,
            SyntaxDetailObservation,
            SyntaxDetailSupport,
            DeclarationObservation,
            DeclarationSupport,
            DeclarationDecorator,
            DeclarationDecoratorSupport,
            ImportAliasObservation,
            ImportAliasSupport,
            DunderAllObservation,
            DunderAllSupport,
            ParameterSyntaxObservation,
            ParameterSyntaxSupport,
            ClassFieldSyntaxObservation,
            ClassFieldSyntaxSupport,
            ProviderCoverage,
            SubjectBoundary,
            AttachmentOutcome,
            AttachmentCandidate
        )
    };
}
pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub source: SourceArtifact,
    pub other: SourceArtifact,
    pub qualification: AssertionQualification,
    pub scope: CoverageScope,
    pub run: ProviderRun,
    pub provider: Provider,
    pub context: AnalysisContext,
    pub surfaces: BTreeMap<FactFamily, ProviderSurface>,
    pub occ: BTreeMap<&'static str, Occurrence>,
    pub placements: Vec<SyntaxPlacement>,
    pub details: Vec<SyntaxDetailObservation>,
    pub declarations: Vec<DeclarationObservation>,
    pub decorators: Vec<DeclarationDecorator>,
    pub imports: Vec<ImportAliasObservation>,
    pub dunder_all: Vec<DunderAllObservation>,
    pub parameters: Vec<ParameterSyntaxObservation>,
    pub fields: Vec<ClassFieldSyntaxObservation>,
    pub coverage: Vec<ProviderCoverage>,
    pub boundaries: Vec<SubjectBoundary>,
    pub outcomes: Vec<AttachmentOutcome>,
    pub candidates: Vec<AttachmentCandidate>,
}
impl Fixture {
    pub fn new() -> Self {
        let model = ValidatedModel::validate(facts_relations()).unwrap();
        let other_bytes: &[u8] = b"z = 0\n";
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
        let other = SourceArtifact::from_bytes(input.id(), "other.py".into(), other_bytes).unwrap();
        let origin = InputOrigin::Tree {
            label: "syntax contract".into(),
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
            tool: "syntax-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let families = [
            FactFamily::Syntax,
            FactFamily::Exports,
            FactFamily::Signatures,
        ];
        let (run, run_families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            families,
        )
        .unwrap();
        let surfaces: BTreeMap<FactFamily, ProviderSurface> = families
            .iter()
            .map(|family| {
                (
                    *family,
                    ProviderSurface {
                        provider: provider.id(),
                        family: *family,
                        name: "syntax".into(),
                    },
                )
            })
            .collect();
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
        let body_end = at("x\n") + 1;
        let mut path = 0;
        let mut occ = BTreeMap::new();
        let mut put =
            |name: &'static str, start: i64, end: i64, kind: SyntaxKind, role: OccurrenceRole| {
                path += 1;
                occ.insert(
                    name,
                    Occurrence {
                        source: source.id(),
                        start,
                        end,
                        syntax_kind: kind,
                        role,
                        structural_path: vec![0, path],
                    },
                );
            };
        put(
            "module",
            0,
            BYTES.len() as i64,
            SyntaxKind::ModModule,
            OccurrenceRole::Syntax,
        );
        put(
            "f",
            0,
            body_end,
            SyntaxKind::StmtFunctionDef,
            OccurrenceRole::Declaration,
        );
        put(
            "decorator",
            0,
            4,
            SyntaxKind::Decorator,
            OccurrenceRole::Decorator,
        );
        put(
            "f.name",
            at("f("),
            at("f(") + 1,
            SyntaxKind::Identifier,
            OccurrenceRole::Syntax,
        );
        put(
            "a",
            at("a,"),
            at("a,") + 1,
            SyntaxKind::Parameter,
            OccurrenceRole::Parameter,
        );
        put(
            "b",
            at("b=1"),
            at("b=1") + 3,
            SyntaxKind::ParameterWithDefault,
            OccurrenceRole::Parameter,
        );
        put(
            "b.default",
            at("1,"),
            at("1,") + 1,
            SyntaxKind::ExprNumberLiteral,
            OccurrenceRole::Syntax,
        );
        put(
            "c",
            at("c: int"),
            at("= 2") + 3,
            SyntaxKind::ParameterWithDefault,
            OccurrenceRole::Parameter,
        );
        put(
            "c.annotation",
            at("int"),
            at("int") + 3,
            SyntaxKind::ExprName,
            OccurrenceRole::Syntax,
        );
        put(
            "c.default",
            at("2)"),
            at("2)") + 1,
            SyntaxKind::ExprNumberLiteral,
            OccurrenceRole::Syntax,
        );
        put(
            "docstring",
            at("\"\"\"Doc"),
            at("Doc.\"\"\"") + 7,
            SyntaxKind::StmtExpr,
            OccurrenceRole::Syntax,
        );
        put(
            "import",
            at("import os"),
            at("as p") + 4,
            SyntaxKind::StmtImport,
            OccurrenceRole::Syntax,
        );
        put(
            "import.alias",
            at("os.path"),
            at("as p") + 4,
            SyntaxKind::Alias,
            OccurrenceRole::Binding,
        );
        put(
            "from",
            at("from ."),
            body_end,
            SyntaxKind::StmtImportFrom,
            OccurrenceRole::Syntax,
        );
        put(
            "from.alias",
            at("x\n"),
            at("x\n") + 1,
            SyntaxKind::Alias,
            OccurrenceRole::Binding,
        );
        put(
            "all",
            at("__all__"),
            at("\"f\"]") + 4,
            SyntaxKind::StmtAssign,
            OccurrenceRole::Syntax,
        );
        put(
            "C",
            at("class C"),
            BYTES.len() as i64 - 1,
            SyntaxKind::StmtClassDef,
            OccurrenceRole::Declaration,
        );
        put(
            "C.name",
            at("C:"),
            at("C:") + 1,
            SyntaxKind::Identifier,
            OccurrenceRole::Syntax,
        );
        put(
            "y",
            at("y:"),
            at("y:") + 1,
            SyntaxKind::ExprName,
            OccurrenceRole::Binding,
        );
        put(
            "y.annotation",
            at_nth("int", 1),
            at_nth("int", 1) + 3,
            SyntaxKind::ExprName,
            OccurrenceRole::Syntax,
        );
        put(
            "y.value",
            at("3\n"),
            at("3\n") + 1,
            SyntaxKind::ExprNumberLiteral,
            OccurrenceRole::Syntax,
        );
        let q = qualification.id();
        let id = |name: &str| occ[name].id();
        let one = Literal::Integer {
            decimal: "1".into(),
        };
        let (names, members) = LiteralSet::of([Literal::String { value: "f".into() }.id()]);
        let mut fixture = Self {
            model,
            batches: BTreeMap::new(),
            source: source.clone(),
            other: other.clone(),
            qualification: qualification.clone(),
            scope: scope.clone(),
            run: run.clone(),
            provider: provider.clone(),
            context: context.clone(),
            surfaces: surfaces.clone(),
            occ: occ.clone(),
            placements: vec![
                SyntaxPlacement {
                    qualification: q,
                    occurrence: id("module"),
                    parent: None,
                    field: SyntaxField::Body,
                    ordinal: 0,
                },
                SyntaxPlacement {
                    qualification: q,
                    occurrence: id("f"),
                    parent: Some(id("module")),
                    field: SyntaxField::Body,
                    ordinal: 0,
                },
                SyntaxPlacement {
                    qualification: q,
                    occurrence: id("all"),
                    parent: Some(id("module")),
                    field: SyntaxField::Body,
                    ordinal: 1,
                },
                SyntaxPlacement {
                    qualification: q,
                    occurrence: id("C"),
                    parent: Some(id("module")),
                    field: SyntaxField::Body,
                    ordinal: 2,
                },
                SyntaxPlacement {
                    qualification: q,
                    occurrence: id("b.default"),
                    parent: Some(id("b")),
                    field: SyntaxField::Default,
                    ordinal: 0,
                },
            ],
            details: vec![SyntaxDetailObservation {
                qualification: q,
                occurrence: id("b.default"),
                ordinal: 0,
                detail: SyntaxDetail::Literal { literal: one.id() }.id(),
            }],
            declarations: vec![
                DeclarationObservation {
                    qualification: q,
                    declaration: id("f"),
                    name: id("f.name"),
                    kind: DeclarationKind::Function,
                    parent: None,
                    overload: false,
                    docstring: Some(id("docstring")),
                },
                DeclarationObservation {
                    qualification: q,
                    declaration: id("C"),
                    name: id("C.name"),
                    kind: DeclarationKind::Class,
                    parent: None,
                    overload: false,
                    docstring: None,
                },
            ],
            decorators: vec![DeclarationDecorator {
                qualification: q,
                declaration: id("f"),
                decorator: id("decorator"),
                ordinal: 0,
            }],
            imports: vec![
                ImportAliasObservation {
                    qualification: q,
                    statement: id("import"),
                    alias: id("import.alias"),
                    level: 0,
                    resolved_module: Some("os.path".into()),
                },
                ImportAliasObservation {
                    qualification: q,
                    statement: id("from"),
                    alias: id("from.alias"),
                    level: 1,
                    resolved_module: None,
                },
            ],
            dunder_all: vec![DunderAllObservation {
                qualification: q,
                statement: id("all"),
                literal: true,
                names: Some(names.id()),
            }],
            parameters: vec![
                ParameterSyntaxObservation {
                    qualification: q,
                    function: id("f"),
                    parameter: id("a"),
                    ordinal: 0,
                    kind: ParameterKind::PositionalOrKeyword,
                    default: None,
                    default_literal: None,
                    annotation: None,
                },
                ParameterSyntaxObservation {
                    qualification: q,
                    function: id("f"),
                    parameter: id("b"),
                    ordinal: 1,
                    kind: ParameterKind::PositionalOrKeyword,
                    default: Some(id("b.default")),
                    default_literal: Some(one.id()),
                    annotation: None,
                },
                ParameterSyntaxObservation {
                    qualification: q,
                    function: id("f"),
                    parameter: id("c"),
                    ordinal: 2,
                    kind: ParameterKind::KeywordOnly,
                    default: Some(id("c.default")),
                    default_literal: Some(
                        Literal::Integer {
                            decimal: "2".into(),
                        }
                        .id(),
                    ),
                    annotation: Some(id("c.annotation")),
                },
            ],
            fields: vec![ClassFieldSyntaxObservation {
                qualification: q,
                class: id("C"),
                target: id("y"),
                annotation: Some(id("y.annotation")),
                value: Some(id("y.value")),
            }],
            coverage: vec![],
            boundaries: vec![],
            outcomes: vec![],
            candidates: vec![],
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(fixture.put(vec![$row.clone()]);)+ }; }
        one!(
            input,
            origin,
            acquisition,
            context,
            provider,
            run,
            condition,
            qualification,
            names
        );
        fixture.put(vec![assumptions::AssumptionSet::empty()]);
        fixture.put(vec![scope.clone()]);
        fixture.put(run_families);
        fixture.put(nodes);
        fixture.put(surfaces.values().cloned().collect());
        fixture.put(vec![source.clone(), other.clone()]);
        let mut chunks: Vec<ArtifactChunk> =
            ArtifactChunk::split(&source, BYTES).unwrap().collect();
        chunks.extend(ArtifactChunk::split(&other, other_bytes).unwrap());
        fixture.put(chunks);
        fixture.put(occ.values().cloned().collect());
        fixture.put(vec![
            one,
            Literal::Integer {
                decimal: "2".into(),
            },
            Literal::String { value: "f".into() },
        ]);
        fixture.put(members);
        fixture.put(vec![SyntaxDetail::Literal {
            literal: Literal::Integer {
                decimal: "1".into(),
            }
            .id(),
        }]);
        fixture.put(
            occ.values()
                .map(|o| Evidence::Occurrence { occurrence: o.id() })
                .collect(),
        );
        fixture.sync();
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
    /// Add an occurrence (and its evidence) to the stored ones.
    pub fn add(&mut self, name: &'static str, occurrence: Occurrence) {
        self.occ.insert(name, occurrence.clone());
        let mut rows = self.rows::<Occurrence>();
        rows.push(occurrence.clone());
        self.put(rows);
        let mut evidence = self.rows::<Evidence>();
        evidence.push(Evidence::Occurrence {
            occurrence: occurrence.id(),
        });
        self.put(evidence);
    }
    fn support<S>(
        &self,
        family: FactFamily,
        subject: Id<Occurrence>,
        build: impl Fn(Id<ProviderRun>, Id<ProviderSurface>, Id<Evidence>) -> S,
    ) -> S {
        build(
            self.run.id(),
            self.surfaces[&family].id(),
            Evidence::Occurrence {
                occurrence: subject,
            }
            .id(),
        )
    }
    /// Store every record and its native support.
    pub fn sync(&mut self) {
        macro_rules! with_support {
            ($rows:expr, $support:ident, $family:expr, $subject:ident) => {{
                let rows = $rows.clone();
                let supports: Vec<$support> = rows
                    .iter()
                    .map(|row| {
                        self.support($family, row.$subject, |run, surface, evidence| $support {
                            assertion: row.id(),
                            run,
                            surface,
                            evidence,
                            origin: Origin::SourceObservation,
                            mode: ExtractionMode::NativeTraversal,
                            fidelity: Fidelity::NativeStructural,
                        })
                    })
                    .collect();
                self.put(rows);
                self.put(supports);
            }};
        }
        with_support!(
            self.placements,
            SyntaxPlacementSupport,
            FactFamily::Syntax,
            occurrence
        );
        with_support!(
            self.details,
            SyntaxDetailSupport,
            FactFamily::Syntax,
            occurrence
        );
        with_support!(
            self.declarations,
            DeclarationSupport,
            FactFamily::Syntax,
            declaration
        );
        with_support!(
            self.decorators,
            DeclarationDecoratorSupport,
            FactFamily::Syntax,
            declaration
        );
        with_support!(
            self.imports,
            ImportAliasSupport,
            FactFamily::Exports,
            statement
        );
        with_support!(
            self.dunder_all,
            DunderAllSupport,
            FactFamily::Exports,
            statement
        );
        with_support!(
            self.parameters,
            ParameterSyntaxSupport,
            FactFamily::Signatures,
            function
        );
        with_support!(
            self.fields,
            ClassFieldSyntaxSupport,
            FactFamily::Syntax,
            class
        );
        let (coverage, boundaries, outcomes, candidates) = (
            self.coverage.clone(),
            self.boundaries.clone(),
            self.outcomes.clone(),
            self.candidates.clone(),
        );
        self.put(coverage);
        self.put(boundaries);
        self.put(outcomes);
        self.put(candidates);
    }
    /// A partial Syntax coverage row for the example artifact, with a boundary and its attachment
    /// outcome of `kind` keeping `candidates`.
    pub fn attachment(
        &mut self,
        kind: AttachmentKind,
        candidates: &[&str],
    ) -> (SubjectBoundary, AttachmentOutcome) {
        let scope = CoverageScope::Artifact {
            artifact: self.source.id(),
        };
        self.coverage.push(ProviderCoverage {
            scope: scope.id(),
            provider: Some(self.provider.id()),
            context: self.context.id(),
            family: FactFamily::Syntax,
            run: Some(self.run.id()),
            status: CoverageStatus::Partial,
            reason: Some(kind.reason()),
            diagnostic: None,
        });
        let boundary = SubjectBoundary {
            scope: scope.id(),
            provider: self.provider.id(),
            context: self.context.id(),
            family: FactFamily::Syntax,
            subject: None,
            reason: kind.reason(),
            detail: Some(format!("{kind:?} event")),
        };
        let outcome = AttachmentOutcome {
            boundary: boundary.id(),
            source: self.source.id(),
            start: self.occ["f.name"].start,
            end: self.occ["f.name"].end,
            syntax_kind: Some(SyntaxKind::ExprName),
            role: Some(OccurrenceRole::Read),
            outcome: kind,
        };
        self.candidates
            .extend(candidates.iter().map(|name| AttachmentCandidate {
                outcome: outcome.id(),
                candidate: self.occ[name].id(),
            }));
        self.boundaries.push(boundary.clone());
        self.outcomes.push(outcome.clone());
        self.sync();
        (boundary, outcome)
    }
    /// Validate every stored relation and invariant, as the store would.
    pub fn validate(&self) -> Result<ContentHash, ModelError> {
        let generation = MemoryGeneration::conformance(&self.model, &budget());
        macro_rules! each { ($($ty:ty),+ $(,)?) => { $( generation.put(&Batch::new(&self.model, self.rows::<$ty>(), &budget()).unwrap())?; )+ }; }
        syntax_relations!(each);
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
