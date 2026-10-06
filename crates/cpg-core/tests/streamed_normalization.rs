//! Actual immutable workspace scopes; these controls invoke no frontends, stores or diagnostic
//! admission replay. A small pure oracle independently checks the complete output relation sets.
use cpg_core::{
    normalize,
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    declarations::*,
    documents::*,
    lexical::*,
    normalized::{
        entities::*,
        entity_normalization::{self, EntityOutput},
        links::*,
        relation_normalization::{self, RelationData},
    },
    resources::ResourceBudget,
    source::*,
    stages::*,
    symbols::*,
    syntax::*,
    types::*,
    *,
};
use std::sync::Arc;
fn context() -> AnalysisContext {
    AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    }
}
fn module(
    data: &mut RelationData,
    input: Id<input::InputRevision>,
    path: &str,
    name: &str,
    ctx: Id<AnalysisContext>,
) -> (Module, Occurrence, AssertionQualification) {
    let source =
        SourceArtifact::from_bytes(input, path.into(), b"class F[T]:\n    U = int\nF\n").unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: name.into(),
    };
    let root = Occurrence {
        source: source.id(),
        start: 0,
        end: source.byte_len,
        syntax_kind: SyntaxKind::ModModule,
        role: OccurrenceRole::Syntax,
        structural_path: vec![0],
    };
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    let q = AssertionQualification {
        assumptions: assumptions::AssumptionSet::empty_id(),
        context: ctx,
        scope: scope.id(),
        condition: conditions::Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    data.artifacts.insert(source).unwrap();
    data.scopes.insert(scope).unwrap();
    data.facts.modules.insert(module.clone()).unwrap();
    data.facts.occurrences.insert(root.clone()).unwrap();
    data.facts.qualifications.insert(q.clone()).unwrap();
    (module, root, q)
}
fn dataset(budget: &ResourceBudget) -> RelationData {
    let mut data = RelationData::new(budget);
    let ctx = context();
    let provider = Provider {
        tool: "fixture".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"provider"),
    };
    let bytes = b"class F[T]:\n    U = int\nF\n";
    data.native_providers.insert(provider.clone()).unwrap();
    let revision = |paths: &[&str]| {
        input::InputRevision::from_entries(
            paths
                .iter()
                .map(|path| input::ManifestEntry {
                    path: (*path).into(),
                    content: ContentHash::of(bytes),
                    byte_len: bytes.len() as i64,
                })
                .collect(),
        )
        .unwrap()
        .id()
    };
    let library = revision(&["impl.py", "api.py", "empty.py"]);
    let other = revision(&["impl.py", "api.py", "other.py"]);
    let corpus = revision(&["docs.py"]);
    let mut symbols = Vec::new();
    for (index, input) in [library, other].into_iter().enumerate() {
        let (implementation, root, q) = module(&mut data, input, "impl.py", "impl", ctx.id());
        let (api, apiroot, apiq) = module(&mut data, input, "api.py", "api", ctx.id());
        let pm = ProviderModule::Acquired {
            module: implementation.id(),
        };
        data.facts.provider_modules.insert(pm.clone()).unwrap();
        let class = Occurrence {
            start: 0,
            end: 24,
            syntax_kind: SyntaxKind::StmtClassDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![0, 0],
            ..root.clone()
        };
        let name = Occurrence {
            start: 6,
            end: 7,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 0, 0],
            ..root.clone()
        };
        let read = Occurrence {
            start: 24,
            end: 25,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![0, 1],
            ..root.clone()
        };
        data.facts.occurrences.insert(class.clone()).unwrap();
        data.facts.occurrences.insert(name.clone()).unwrap();
        data.facts.occurrences.insert(read.clone()).unwrap();
        let symbol = ProviderSymbol {
            provider: provider.id(),
            context: ctx.id(),
            module: pm.id(),
            native_key: "F".into(),
            name: "F".into(),
            kind: SymbolKind::Class,
        };
        data.facts.symbols.insert(symbol.clone()).unwrap();
        let (run, _) = ProviderRun::new(
            provider.id(),
            ctx.id(),
            input,
            ContentHash::of(b"fixture"),
            [FactFamily::Signatures, FactFamily::Exports],
        )
        .unwrap();
        data.facts.runs.insert(run.clone()).unwrap();
        let declaration = SymbolDeclaration {
            qualification: q.id(),
            symbol: symbol.id(),
            declaration: class.id(),
        };
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Signatures,
            name: "declarations".into(),
        };
        data.native_surfaces.insert(surface.clone()).unwrap();
        data.native_evidence
            .insert(Evidence::Occurrence {
                occurrence: class.id(),
            })
            .unwrap();
        data.facts.declarations.insert(declaration.clone()).unwrap();
        data.facts
            .declaration_supports
            .insert(SymbolDeclarationSupport {
                assertion: declaration.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: Evidence::Occurrence {
                    occurrence: class.id(),
                }
                .id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        let report = SymbolObservation {
            qualification: q.id(),
            symbol: symbol.id(),
            parent: None,
        };
        data.facts
            .symbol_observations
            .insert(report.clone())
            .unwrap();
        data.symbol_observations.insert(report).unwrap();
        let origin = ExportOrigin::Traced {
            module: pm.id(),
            name: "F".into(),
            kind: Some(ExportKind::Class),
        };
        data.facts.export_origins.insert(origin.clone()).unwrap();
        let public = PublicNameObservation {
            qualification: apiq.id(),
            access: api.id(),
            name: "Exported".into(),
            via_dunder_all: false,
            origin: origin.id(),
        };
        data.facts.public_names.insert(public.clone()).unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Exports,
            name: "exports".into(),
        };
        data.native_surfaces.insert(surface.clone()).unwrap();
        data.native_evidence
            .insert(Evidence::Occurrence {
                occurrence: apiroot.id(),
            })
            .unwrap();
        data.facts
            .public_supports
            .insert(PublicNameSupport {
                assertion: public.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: Evidence::Occurrence {
                    occurrence: apiroot.id(),
                }
                .id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        // An empty module enumeration must be emitted even without any public-name root.
        let (empty, emptyroot, emptyq) = module(
            &mut data,
            input,
            if index == 0 { "empty.py" } else { "other.py" },
            "empty",
            ctx.id(),
        );
        let enumeration = ExportEnumerationObservation {
            qualification: emptyq.id(),
            access: empty.id(),
            names: value::LiteralSet::of([]).0.id(),
            status: ExportEnumerationStatus::Complete,
            basis: ExportEnumerationBasis::Inferred,
        };
        data.native_evidence
            .insert(Evidence::Occurrence {
                occurrence: emptyroot.id(),
            })
            .unwrap();
        data.facts
            .export_enumerations
            .insert(enumeration.clone())
            .unwrap();
        data.facts
            .export_enumeration_supports
            .insert(ExportEnumerationSupport {
                assertion: enumeration.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: Evidence::Occurrence {
                    occurrence: emptyroot.id(),
                }
                .id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        if index == 0 {
            let scope = LexicalScope {
                owner: root.id(),
                kind: LexicalScopeKind::Module,
            };
            let reference = ReferenceObservation {
                qualification: q.id(),
                read: read.id(),
                scope: scope.id(),
                parent: root.id(),
                field: SyntaxField::Body,
                name: "F".into(),
            };
            let event = BindingEvent {
                site: class.id(),
                name: "F".into(),
            };
            data.facts.bindings.insert(event.clone()).unwrap();
            let target = LexicalTarget::Binding { event: event.id() };
            data.lexical_targets.insert(target.clone()).unwrap();
            data.references.insert(reference).unwrap();
            data.lexical_resolutions
                .insert(LexicalResolution {
                    qualification: q.id(),
                    read: read.id(),
                    target: target.id(),
                    captured: false,
                })
                .unwrap();
            data.variables
                .insert(TypeVariable {
                    provider: provider.id(),
                    context: ctx.id(),
                    module: pm.id(),
                    anchor_start: 8,
                    anchor_end: 9,
                    slot: 0,
                    origin: TypeVariableOrigin::Pep695,
                    kind: TypeVariableKind::TypeVar,
                    name: "T".into(),
                    declared_variance: None,
                    inferred_variance: None,
                })
                .unwrap();
            let type_parameter = Occurrence {
                start: 8,
                end: 9,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Declaration,
                structural_path: vec![0, 0, 1],
                ..root.clone()
            };
            let assignment = Occurrence {
                start: 16,
                end: 23,
                syntax_kind: SyntaxKind::StmtAssign,
                role: OccurrenceRole::Syntax,
                structural_path: vec![0, 0, 2],
                ..root.clone()
            };
            let target = Occurrence {
                start: 16,
                end: 17,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Declaration,
                structural_path: vec![0, 0, 2, 0],
                ..root.clone()
            };
            for row in [&type_parameter, &assignment, &target] {
                data.facts.occurrences.insert(row.clone()).unwrap();
            }
            for (site, name, kind) in [
                (type_parameter.id(), "T", BindingEventKind::TypeParam),
                (target.id(), "U", BindingEventKind::Assignment),
            ] {
                let event = BindingEvent {
                    site,
                    name: name.into(),
                };
                data.facts.bindings.insert(event.clone()).unwrap();
                data.binding_observations
                    .insert(BindingObservation {
                        qualification: q.id(),
                        event: event.id(),
                        scope: scope.id(),
                        kind,
                        ordinal: 0,
                        value: None,
                        static_branch: None,
                        static_polarity: None,
                    })
                    .unwrap();
            }
            data.declaration_syntax
                .insert(DeclarationObservation {
                    qualification: q.id(),
                    declaration: class.id(),
                    name: name.id(),
                    kind: DeclarationKind::Class,
                    parent: None,
                    overload: false,
                    docstring: None,
                })
                .unwrap();
            let place_root = value::PlaceRoot::Occurrence {
                occurrence: class.id(),
            };
            data.roots.insert(place_root.clone()).unwrap();
            data.facts
                .places
                .insert(value::Place {
                    root: place_root.id(),
                    path: value::AccessPath::empty().id(),
                })
                .unwrap();
        }
        symbols.push(symbol);
    }
    data.facts
        .terms
        .insert(TypeTerm::ClassObject {
            class: symbols[1].id(),
        })
        .unwrap();
    let (_, root, q) = module(&mut data, corpus, "docs.py", "docs", ctx.id());
    data.corpus_libraries
        .insert(input::CorpusLibrary { corpus, library })
        .unwrap();
    let span = EvidenceSourceSpanId::of(&Evidence::SourceSpan {
        source: root.source,
        start: 0,
        end: 1,
    })
    .unwrap();
    let mention = DocumentNodeMentionId::of(&DocumentNode::Mention { span }).unwrap();
    let passage = DocumentNodePassageId::of(&DocumentNode::Passage { span, ordinal: 0 }).unwrap();
    data.mentions
        .insert(DocumentMentionObservation {
            qualification: q.id(),
            mention,
            passage,
            class: MentionClass::Exact,
            source: MentionSource::InlineCode,
            form: "api.Exported".into(),
            access_path: Some("api.Exported".into()),
            qualified_name: Some("impl.F".into()),
        })
        .unwrap();
    data
}
async fn run(data: &mut RelationData, memory: usize, batch_rows: usize) -> Arc<Workspace> {
    let model = Arc::new(model().unwrap());
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes: memory,
            batch_rows,
            partitions: 1,
        },
    )
    .unwrap();
    let facts = workspace.output(
        "scoped-fixture",
        Profile::Catalog,
        ContentHash::of(b"fixture"),
        workspace
            .inputs("scoped-fixture", Profile::Catalog, [])
            .unwrap(),
    );
    let mut names = std::collections::BTreeSet::new();
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if names.insert(<$ty>::NAME) { facts.declare::<$ty>().unwrap();for row in data.facts.$field.iter() {facts.push(row.clone()).await.unwrap();} })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    macro_rules! extra { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if names.insert(<$ty>::NAME) { facts.declare::<$ty>().unwrap();for row in data.$field.iter() {facts.push(row.clone()).await.unwrap();} })* }; }
    lctx_model::normalized_relation_inputs!(extra);
    facts.finish(ProviderOutcome::Complete).await.unwrap();
    workspace.freeze_inputs(PublicationBoundary::Facts).unwrap();
    let expected =
        entity_normalization::normalize(data.facts.inputs(), workspace.budget()).unwrap();
    let stage = entity_normalization::stage();
    let inputs = workspace.stage_inputs(&stage, Profile::Catalog).unwrap();
    let output = workspace.producer(&stage, Profile::Catalog, inputs.clone());
    normalize::entities(inputs, output, &workspace, &model)
        .await
        .unwrap();
    let mut actual = EntityOutput::new(workspace.budget());
    macro_rules! read { ($($field:ident: $ty:ty,)*) => { $(for batch in workspace.completed::<$ty>().unwrap().batches().unwrap() {actual.$field.decode(&batch.unwrap()).unwrap();})* }; }
    lctx_model::normalized_entity_outputs!(read);
    actual.matches(&expected).unwrap();
    drop(expected);
    data.entities = actual;
    let expected = relation_normalization::normalize(data, workspace.budget()).unwrap();
    let stage = relation_normalization::stage(Profile::Catalog);
    let inputs = workspace.stage_inputs(&stage, Profile::Catalog).unwrap();
    let output = workspace.producer(&stage, Profile::Catalog, inputs.clone());
    normalize::relations(inputs, output, &workspace, &model)
        .await
        .unwrap();
    let mut actual = relation_normalization::RelationOutput::new(workspace.budget());
    macro_rules! read { ($($field:ident: $ty:ty,)*) => { $(for batch in workspace.completed::<$ty>().unwrap().batches().unwrap() {actual.$field.decode(&batch.unwrap()).unwrap();})* }; }
    lctx_model::normalized_relation_outputs!(read);
    actual.matches(&expected).unwrap();
    if !data.mentions.is_empty() {
        assert_eq!(
            actual
                .mention_entity_assessments
                .iter()
                .next()
                .unwrap()
                .status,
            ResolutionStatus::Resolved
        );
        assert_eq!(
            actual.mention_entity_candidates.len(),
            1,
            "unlinked equal spellings must not enlarge the candidate universe"
        );
        assert_eq!(actual.mention_symbol_candidates.len(), 1);
        assert_eq!(
            actual.type_binder_assessments.iter().next().unwrap().status,
            ResolutionStatus::Resolved
        );
        assert_eq!(
            actual.type_binder_candidates.len(),
            2,
            "declaration and type-parameter premises agree; unrelated body assignment is excluded"
        );
        assert_eq!(
            actual
                .reference_entity_assessments
                .iter()
                .next()
                .unwrap()
                .status,
            ResolutionStatus::Resolved
        );
        assert_eq!(data.entities.public_enumerations.len(), 2);
    }
    workspace
}
#[tokio::test]
async fn multi_input_cross_module_scopes_preserve_exact_outputs_and_release_premises() {
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let mut ordinary = dataset(&budget);
    let mut tiny_batches = dataset(&budget);
    let first = run(&mut ordinary, 32 << 20, 1024).await;
    let second = run(&mut tiny_batches, 32 << 20, 1).await;
    macro_rules! compare { ($($field:ident: $ty:ty,)*) => { $(assert_eq!(first.completed::<$ty>().unwrap().content(),second.completed::<$ty>().unwrap().content());)* }; }
    lctx_model::normalized_entity_outputs!(compare);
    lctx_model::normalized_relation_outputs!(compare);
    drop(ordinary);
    drop(tiny_batches);
    assert!(first.budget().reserved() < 1 << 20);
    assert!(second.budget().reserved() < 1 << 20);
}
#[tokio::test]
async fn empty_completed_scopes_finish_without_diagnostic_replay() {
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let workspace = run(&mut data, 8 << 20, 1).await;
    assert_eq!(
        workspace
            .completed::<SymbolEntityResolution>()
            .unwrap()
            .rows(),
        0
    );
    assert_eq!(
        workspace
            .completed::<MentionEntityAssessment>()
            .unwrap()
            .rows(),
        0
    );
}
#[tokio::test]
async fn wide_source_uses_workspace_sort_and_streams_ownership_under_a_small_budget() {
    async fn run_wide(options: WorkspaceOptions) -> Vec<(&'static str, ContentHash, u64)> {
        let model = Arc::new(model().unwrap());
        let workspace = Workspace::new(model.clone(), options).unwrap();
        let facts = workspace.output(
            "wide-scope-fixture",
            Profile::Catalog,
            ContentHash::of(b"wide"),
            workspace
                .inputs("wide-scope-fixture", Profile::Catalog, [])
                .unwrap(),
        );
        macro_rules! declare { ($($field:ident: $ty:ty => $family:ident,)*) => { $(facts.declare::<$ty>().unwrap();)* }; }
        lctx_model::normalized_entity_inputs!(declare);
        let source = SourceArtifact::from_bytes(
            input::InputRevision::from_entries(vec![]).unwrap().id(),
            "wide.py".into(),
            b"pass\n",
        )
        .unwrap();
        let root = Occurrence {
            source: source.id(),
            start: 0,
            end: 5,
            syntax_kind: SyntaxKind::ModModule,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let module = Module {
            source: source.id(),
            qualified_name: "wide".into(),
        };
        facts.push(module.clone()).await.unwrap();
        facts.push(root.clone()).await.unwrap();
        // Reverse physical arrival requires the real external ordering route, not caller-preordered
        // pages. Keeping these rich occurrences would exceed the attempt's 2 MiB budget.
        for index in (0..60_000).rev() {
            facts
                .push(Occurrence {
                    syntax_kind: SyntaxKind::StmtPass,
                    structural_path: vec![0, index],
                    ..root.clone()
                })
                .await
                .unwrap();
        }
        facts.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.freeze_inputs(PublicationBoundary::Facts).unwrap();
        let stage = entity_normalization::stage();
        let inputs = workspace.stage_inputs(&stage, Profile::Catalog).unwrap();
        let output = workspace.producer(&stage, Profile::Catalog, inputs.clone());
        normalize::entities(inputs, output, &workspace, &model)
            .await
            .unwrap();
        assert_eq!(
            workspace.completed::<OccurrenceOwnership>().unwrap().rows(),
            60_001
        );
        assert!(
            workspace.budget().reserved() < 1 << 20,
            "completed scopes must release decoded inputs and owner ancestors"
        );
        // Independent owner and endpoint identities come from the fixture, not another normalizer.
        let expected_entity = EntityRef::Module {
            module: module.id(),
        }
        .id();
        let expected_ids = [
            root.id(),
            Occurrence {
                syntax_kind: SyntaxKind::StmtPass,
                structural_path: vec![0, 0],
                ..root.clone()
            }
            .id(),
            Occurrence {
                syntax_kind: SyntaxKind::StmtPass,
                structural_path: vec![0, 59_999],
                ..root.clone()
            }
            .id(),
        ];
        let mut seen = [false; 3];
        let mut count = 0;
        for batch in workspace
            .completed::<OccurrenceOwnership>()
            .unwrap()
            .batches()
            .unwrap()
        {
            for row in OccurrenceOwnership::decode(&batch.unwrap()).unwrap() {
                assert_eq!(row.owner, root.id());
                assert_eq!(row.entity, expected_entity);
                for (index, expected) in expected_ids.iter().enumerate() {
                    seen[index] |= row.occurrence == *expected;
                }
                count += 1;
            }
        }
        assert_eq!(count, 60_001);
        assert!(seen.into_iter().all(|found| found));
        let mut contents = Vec::new();
        macro_rules! contents { ($($field:ident: $ty:ty,)*) => { $(
            let completed = workspace.completed::<$ty>().unwrap();
            contents.push((<$ty>::NAME, completed.content(), completed.rows()));
        )* }; }
        lctx_model::normalized_entity_outputs!(contents);
        contents
    }
    let constrained = run_wide(WorkspaceOptions {
        memory_bytes: 2 << 20,
        batch_rows: 128,
        partitions: 1,
    })
    .await;
    let larger = run_wide(WorkspaceOptions {
        memory_bytes: 16 << 20,
        batch_rows: 4096,
        partitions: 2,
    })
    .await;
    assert_eq!(
        constrained, larger,
        "canonical normalization output changes with memory/partition configuration"
    );
}
