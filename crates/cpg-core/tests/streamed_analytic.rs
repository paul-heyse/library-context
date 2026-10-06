//! Actual immutable text producer scopes; no frontends, store, provider or diagnostic admission replay.
use cpg_core::{
    analytic_text,
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::ParameterKind, conditions::Diagram,
    embedding::text::*, input::*, normalized::entities::*, source::*, syntax::*, *,
};
use std::sync::Arc;

fn fixture(budget: &resources::ResourceBudget, padding: usize, path: &str) -> TextData {
    let text = "def outer(a, /, *args, option=None, **kw):\n    \"\"\"Do source work.\"\"\"\n    return a\n";
    let text = format!("{text}{}", "# irrelevant source body\n".repeat(padding));
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: path.into(),
        content: ContentHash::of(text.as_bytes()),
        byte_len: text.len() as i64,
    }])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), path.into(), text.as_bytes()).unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"analytic text fixture"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: scope.id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let mut data = TextData::new(budget);
    data.artifacts.insert(source.clone()).unwrap();
    data.uses
        .insert(ArtifactUse {
            artifact: source.id(),
            input: input.id(),
            role: SourceRole::Release,
        })
        .unwrap();
    for chunk in ArtifactChunk::split(&source, text.as_bytes()).unwrap() {
        data.chunks.insert(chunk).unwrap();
    }
    data.modules
        .insert(Module {
            source: source.id(),
            qualified_name: "package.module".into(),
        })
        .unwrap();
    data.qualifications.insert(q.clone()).unwrap();
    let mut occurrence = |needle: &str, kind, role| {
        let start = text.find(needle).unwrap();
        let row = Occurrence {
            source: source.id(),
            start: start as i64,
            end: (start + needle.len()) as i64,
            syntax_kind: kind,
            role,
            structural_path: vec![start as i32],
        };
        let id = row.id();
        data.occurrences.insert(row).unwrap();
        id
    };
    let declaration = occurrence(
        &text,
        SyntaxKind::StmtFunctionDef,
        OccurrenceRole::Declaration,
    );
    let name = occurrence("outer", SyntaxKind::Identifier, OccurrenceRole::Syntax);
    let doc = occurrence(
        "\"\"\"Do source work.\"\"\"",
        SyntaxKind::StmtExpr,
        OccurrenceRole::Syntax,
    );
    let doc_literal = occurrence(
        "\"\"\"Do source work.\"\"\"",
        SyntaxKind::ExprStringLiteral,
        OccurrenceRole::Syntax,
    );
    let default = occurrence("None", SyntaxKind::ExprNoneLiteral, OccurrenceRole::Syntax);
    let params = [
        ("a,", ParameterKind::PositionalOnly),
        ("*args", ParameterKind::VarPositional),
        ("option=None", ParameterKind::KeywordOnly),
        ("**kw", ParameterKind::VarKeyword),
    ]
    .into_iter()
    .enumerate()
    .map(|(ordinal, (needle, kind))| {
        let parameter = if needle == "a," {
            let start = text.find(needle).unwrap();
            let row = Occurrence {
                source: source.id(),
                start: start as i64,
                end: (start + 1) as i64,
                syntax_kind: SyntaxKind::Parameter,
                role: OccurrenceRole::Parameter,
                structural_path: vec![start as i32],
            };
            let id = row.id();
            data.occurrences.insert(row).unwrap();
            id
        } else {
            let start = text.find(needle).unwrap();
            let row = Occurrence {
                source: source.id(),
                start: start as i64,
                end: (start + needle.len()) as i64,
                syntax_kind: SyntaxKind::Parameter,
                role: OccurrenceRole::Parameter,
                structural_path: vec![start as i32],
            };
            let id = row.id();
            data.occurrences.insert(row).unwrap();
            id
        };
        ParameterSyntaxObservation {
            qualification: q.id(),
            function: declaration,
            parameter,
            ordinal: ordinal as i64,
            kind,
            default: (needle == "option=None").then_some(default),
            default_literal: None,
            annotation: None,
        }
    })
    .collect::<Vec<_>>();
    for parameter in params {
        data.parameters.insert(parameter).unwrap();
    }
    let row = DeclarationObservation {
        qualification: q.id(),
        declaration,
        name,
        kind: DeclarationKind::Function,
        parent: None,
        overload: false,
        docstring: Some(doc),
    };
    data.declarations.insert(row).unwrap();
    let callable = CallableEntity::Source {
        declaration,
        kind: CallableKind::Function,
    };
    data.callables.insert(callable.clone()).unwrap();
    data.entities
        .insert(EntityRef::Callable {
            callable: callable.id(),
        })
        .unwrap();
    data.placements
        .insert(SyntaxPlacement {
            qualification: q.id(),
            occurrence: doc_literal,
            parent: Some(doc),
            field: lexical::SyntaxField::Value,
            ordinal: 0,
        })
        .unwrap();
    let literal = value::Literal::String {
        value: "Do source work.".into(),
    };
    data.literals.insert(literal.clone()).unwrap();
    let detail = SyntaxDetail::Literal {
        literal: literal.id(),
    };
    data.detail_values.insert(detail.clone()).unwrap();
    data.details
        .insert(SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: doc_literal,
            ordinal: 0,
            detail: detail.id(),
        })
        .unwrap();
    data
}

async fn run(data: &TextData, requested: bool, memory: usize, batch_rows: usize) -> Arc<Workspace> {
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
    let output = workspace.output(
        "analytic-fixture",
        stages::Profile::Catalog,
        ContentHash::of(b"fixture"),
        workspace
            .inputs("analytic-fixture", stages::Profile::Catalog, [])
            .unwrap(),
    );
    let mut names = Vec::new();
    macro_rules! emit {($($field:ident:$ty:ty,)*)=>{$(names.push(<$ty>::NAME);output.declare::<$ty>().unwrap();for row in data.$field.iter() {output.push(row.clone()).await.unwrap();})*};}
    lctx_model::analytic_text_inputs!(emit);
    output
        .finish(stages::ProviderOutcome::Complete)
        .await
        .unwrap();
    workspace
        .freeze_inputs(stages::PublicationBoundary::Facts)
        .unwrap();
    let inputs = workspace
        .inputs("analytic_text", stages::Profile::Catalog, names)
        .unwrap();
    let output = workspace.output(
        "analytic_text",
        stages::Profile::Catalog,
        ContentHash::of(b"text"),
        inputs.clone(),
    );
    let definition = TextDefinition {
        requested,
        window_bytes: 32,
        ..TextDefinition::builtin()
    };
    let expected = prepare(data, &definition, workspace.budget()).unwrap();
    analytic_text::publish(inputs, output, &workspace, &model, definition)
        .await
        .unwrap();
    let mut actual = TextOutput::new(workspace.budget());
    macro_rules! read {
        ($field:ident,$ty:ty) => {{
            for batch in workspace.completed::<$ty>().unwrap().batches().unwrap() {
                actual.$field.decode(&batch.unwrap()).unwrap();
            }
            assert!(actual.$field.same(&expected.$field), stringify!($field));
        }};
    }
    read!(definitions, TextDefinition);
    read!(subjects, TextSubject);
    read!(assessments, TextAssessment);
    read!(windows, TextWindow);
    let invariant = lctx_model::domain::validation::invariants_for::<TextAssessment>()
        .into_iter()
        .find(|invariant| invariant.purpose == InvariantPurpose::Admission)
        .unwrap();
    let mut check = (invariant.create)(workspace.budget());
    for declaration in &invariant.inputs {
        macro_rules! facts {($($field:ident:$ty:ty,)*)=>{$(if declaration.name()==<$ty>::NAME {
            if <$ty>::NAME!=ArtifactChunk::NAME {for row in data.$field.iter() {check.visit(<$ty>::NAME,&<$ty as Record>::encode(&[row.clone()]).unwrap()).unwrap();}}
        })*};}
        lctx_model::analytic_text_inputs!(facts);
        if declaration.name() == ArtifactChunk::NAME {
            let mut chunks = data.chunks.iter().collect::<Vec<_>>();
            chunks.sort_by_key(|row| (row.artifact, row.ordinal));
            for row in chunks {
                check
                    .visit(
                        ArtifactChunk::NAME,
                        &<ArtifactChunk as Record>::encode(&[row.clone()]).unwrap(),
                    )
                    .unwrap();
            }
        }
        macro_rules! result {
            ($field:ident,$ty:ty) => {
                if declaration.name() == <$ty>::NAME {
                    for row in actual.$field.iter() {
                        check
                            .visit(
                                <$ty>::NAME,
                                &<$ty as Record>::encode(&[row.clone()]).unwrap(),
                            )
                            .unwrap();
                    }
                }
            };
        }
        result!(definitions, TextDefinition);
        result!(subjects, TextSubject);
        result!(assessments, TextAssessment);
        if declaration.name() == TextWindow::NAME {
            let mut windows = actual.windows.iter().collect::<Vec<_>>();
            windows.sort_by_key(|row| (row.assessment, row.ordinal));
            for row in windows {
                check
                    .visit(
                        TextWindow::NAME,
                        &<TextWindow as Record>::encode(&[row.clone()]).unwrap(),
                    )
                    .unwrap();
            }
        }
    }
    check.finish().unwrap();
    drop(actual);
    drop(expected);
    assert!(workspace.budget().reserved() < 1 << 20);
    workspace
}
#[tokio::test]
async fn source_subject_closure_renders_exact_parameters_docstring_and_windows_across_batches() {
    let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let data = fixture(&budget, 0, "module.py");
    let first = run(&data, true, 8 << 20, 1024).await;
    let second = run(&data, true, 8 << 20, 1).await;
    assert_eq!(
        first.completed::<TextWindow>().unwrap().content(),
        second.completed::<TextWindow>().unwrap().content()
    );
    assert!(first.completed::<TextWindow>().unwrap().rows() > 1);
}
#[tokio::test]
async fn large_unrelated_body_chunks_are_not_rich_render_premises() {
    let budget = resources::ResourceBudget::fixed(32 << 20).unwrap();
    let data = fixture(&budget, 250_000, "module.py");
    assert!(data.chunks.len() > 1);
    assert!(
        data.chunks
            .iter()
            .map(|chunk| chunk.body.0.len())
            .sum::<usize>()
            > 4 << 20
    );
    let workspace = run(&data, true, 4 << 20, 8).await;
    assert_eq!(workspace.completed::<TextAssessment>().unwrap().rows(), 1);
}
#[tokio::test]
async fn empty_and_unrequested_text_scopes_finish_without_rendering() {
    let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let data = TextData::new(&budget);
    let empty = run(&data, true, 4 << 20, 1).await;
    assert_eq!(empty.completed::<TextWindow>().unwrap().rows(), 0);
    let data = fixture(&budget, 0, "module.py");
    let disabled = run(&data, false, 4 << 20, 1).await;
    assert_eq!(disabled.completed::<TextAssessment>().unwrap().rows(), 0);
}

struct Changing {
    inner: cpg_core::embedding_service::FakeEmbedder,
    calls: std::sync::Mutex<(
        std::collections::BTreeMap<ContentHash, usize>,
        usize,
        Vec<usize>,
    )>,
}
impl cpg_core::embedding_service::Embedder for Changing {
    fn spec(&self) -> &embedding::Spec {
        cpg_core::embedding_service::Embedder::spec(&self.inner)
    }
    fn endpoint(&self) -> &str {
        cpg_core::embedding_service::Embedder::endpoint(&self.inner)
    }
    fn count_tokens<'a>(
        &'a self,
        _: &'a str,
    ) -> cpg_core::embedding_service::EmbedFuture<'a, usize> {
        self.calls.lock().unwrap().1 += 1;
        Box::pin(async { Ok(7) })
    }
    fn embed<'a>(
        &'a self,
        requests: &'a [String],
    ) -> cpg_core::embedding_service::EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            let mut calls = self.calls.lock().unwrap();
            calls.2.push(requests.len());
            Ok(requests
                .iter()
                .map(|request| {
                    let count = calls
                        .0
                        .entry(embedding::value::input_hash(request))
                        .or_default();
                    let mut value = vec![0.0; self.spec().dimensions as usize];
                    value[*count % self.spec().dimensions as usize] = 1.0;
                    *count += 1;
                    value
                })
                .collect())
        })
    }
}
#[tokio::test]
async fn actual_embedding_frames_share_nonadjacent_exact_winners_without_a_cache() {
    use cpg_core::embedding_service::Embedder;
    use lctx_model::domain::{
        analysis::{AnalysisDefinition, MethodParameters},
        attribution::*,
        embedding::{
            EmbeddingSpec,
            analytic::{self, AnalysisEmbeddingUse, VectorAvailability},
            configuration::ServiceConfiguration,
        },
    };
    let provider = Changing {
        inner: Default::default(),
        calls: Default::default(),
    };
    let model = Arc::new(model().unwrap());
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes: 4 << 20,
            batch_rows: 8,
            partitions: 1,
        },
    )
    .unwrap();
    let facts = workspace.output(
        "embedding-fixture",
        stages::Profile::Catalog,
        ContentHash::of(b"fixture"),
        workspace
            .inputs("embedding-fixture", stages::Profile::Catalog, [])
            .unwrap(),
    );
    let mut names = std::collections::BTreeSet::new();
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(if names.insert(<$ty>::NAME) {facts.declare::<$ty>().unwrap();})*};}
    lctx_model::expected_domain_inputs!(declare);
    lctx_model::analytic_consumption_inputs!(declare);
    for relation in [
        Relation::of::<AnalysisDefinition>(),
        Relation::of::<MethodParameters>(),
    ] {
        names.insert(relation.name());
    }
    facts.declare::<AnalysisDefinition>().unwrap();
    facts.declare::<MethodParameters>().unwrap();
    let (parameters, definition) = analytic::definition();
    facts.push(parameters).await.unwrap();
    facts.push(definition).await.unwrap();
    let text_definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    facts.push(text_definition.clone()).await.unwrap();
    let specification = EmbeddingSpec::new(provider.spec()).unwrap();
    facts.push(specification.clone()).await.unwrap();
    facts
        .push(ServiceConfiguration {
            specification: specification.id(),
            endpoint: provider.endpoint().into(),
        })
        .await
        .unwrap();
    let original = "shared documentary bytes";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "guide.md".into(),
        content: ContentHash::of(original.as_bytes()),
        byte_len: original.len() as i64,
    }])
    .unwrap();
    let artifact =
        SourceArtifact::from_bytes(input.id(), "guide.md".into(), original.as_bytes()).unwrap();
    let scope = CoverageScope::Artifact {
        artifact: artifact.id(),
    };
    facts.push(input.clone()).await.unwrap();
    facts.push(artifact.clone()).await.unwrap();
    facts.push(scope.clone()).await.unwrap();
    facts
        .push(ArtifactUse {
            input: input.id(),
            artifact: artifact.id(),
            role: SourceRole::Document,
        })
        .await
        .unwrap();
    let native = Provider {
        tool: "source-written contract fixture".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"fixture"),
    };
    for version in ["3.13", "3.14"] {
        let context = AnalysisContext {
            python_version: version.into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"fixture"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let (run, _) = ProviderRun::new(
            native.id(),
            context.id(),
            input.id(),
            ContentHash::of(b"run"),
            [FactFamily::Docs],
        )
        .unwrap();
        facts.push(run.clone()).await.unwrap();
        facts
            .push(ProviderCoverage {
                scope: scope.id(),
                provider: Some(native.id()),
                context: context.id(),
                family: FactFamily::Docs,
                run: Some(run.id()),
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            })
            .await
            .unwrap();
        // This control owns consumed-window facts, rather than claiming documentary extraction.
        let subject: Id<TextSubject> =
            serde_json::from_value(serde_json::json!(vec![1; 16])).unwrap();
        let assessment = TextAssessment {
            subject,
            definition: text_definition.id(),
            input: input.id(),
            context: context.id(),
            entity: None,
            source: artifact.id(),
            availability: TextAvailability::Available,
            boundary: None,
        };
        facts.push(assessment.clone()).await.unwrap();
        for ordinal in 0..140 {
            let text = format!("shared window {}", ordinal % 20);
            facts
                .push(TextWindow {
                    assessment: assessment.id(),
                    ordinal,
                    start: ordinal * 32,
                    end: ordinal * 32 + text.len() as i64,
                    content: ContentHash::of(text.as_bytes()),
                    text: text.into(),
                })
                .await
                .unwrap();
        }
    }
    facts
        .finish(stages::ProviderOutcome::Complete)
        .await
        .unwrap();
    workspace
        .freeze_inputs(stages::PublicationBoundary::Facts)
        .unwrap();
    let inputs = workspace
        .inputs("analytic_embedding", stages::Profile::Catalog, names)
        .unwrap();
    let output = workspace.output(
        "analytic_embedding",
        stages::Profile::Catalog,
        ContentHash::of(b"embedding"),
        inputs.clone(),
    );
    cpg_core::analytic_embedding::produce(
        inputs,
        output,
        &workspace,
        &model,
        Some(&provider),
        None,
    )
    .await
    .unwrap();
    let mut receipts = std::collections::BTreeMap::new();
    let mut count = 0;
    for batch in workspace
        .completed::<AnalysisEmbeddingUse>()
        .unwrap()
        .batches()
        .unwrap()
    {
        for row in AnalysisEmbeddingUse::decode(&batch.unwrap()).unwrap() {
            assert_eq!(row.availability, VectorAvailability::Available);
            count += 1;
            let receipt = (row.admitted_tokens, row.value_digest, row.bytes);
            if let Some(first) = receipts.insert(row.input, receipt.clone()) {
                assert_eq!(first, receipt);
            }
        }
    }
    assert_eq!(count, 280);
    assert_eq!(receipts.len(), 20);
    let calls = provider.calls.lock().unwrap();
    assert_eq!(calls.1, 20);
    assert!(calls.0.values().all(|count| *count == 1));
    assert!(calls.2.iter().all(|count| *count <= 64));
    assert_eq!(
        workspace
            .completed::<analysis::analytic_embedding::AnalysisInvocation>()
            .unwrap()
            .rows(),
        2
    );
    drop(calls);
    let invariant = analytic::invariants()
        .into_iter()
        .find(|invariant| invariant.purpose == InvariantPurpose::Admission)
        .unwrap();
    let inspection = workspace
        .inputs(
            "receipt-inspection",
            stages::Profile::Catalog,
            invariant.inputs.iter().map(ValidationInput::name),
        )
        .unwrap();
    let session = inspection.session(&workspace).await.unwrap();
    let mut check = (invariant.create)(workspace.budget());
    use futures::TryStreamExt;
    for declaration in &invariant.inputs {
        let table = inspection.table_for(declaration).unwrap();
        let ordering = declaration
            .order()
            .iter()
            .map(|field| format!("\"{field}\""))
            .collect::<Vec<_>>()
            .join(",");
        let query = format!("SELECT * FROM \"{table}\" ORDER BY {ordering}");
        let mut stream = session
            .sql(&query)
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            check.visit_input(declaration, &batch).unwrap();
        }
    }
    check.finish().unwrap();
    drop(session);
    drop(inspection);
    assert!(
        workspace.budget().reserved() < 1 << 20,
        "frame text/vector transfers and private winner payloads are released"
    );
}

#[tokio::test]
async fn multi_source_documentary_and_dependency_scopes_keep_original_bytes_and_selection() {
    use lctx_model::domain::documents::*;
    let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = fixture(&budget, 0, "api.py");
    let release = fixture(&budget, 0, "other/api.py");
    let mut dependency = fixture(&budget, 0, "dependency/api.py");
    let artifact = dependency.artifacts.iter().next().unwrap().clone();
    dependency.uses = normalized::Rows::new(&budget);
    dependency
        .uses
        .insert(ArtifactUse {
            artifact: artifact.id(),
            input: artifact.input,
            role: SourceRole::Dependency,
        })
        .unwrap();
    macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in release.$field.iter().chain(dependency.$field.iter()) {data.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::analytic_text_inputs!(merge);
    let text = "α documentary bytes\n🙂 second line\n";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "guide.md".into(),
        content: ContentHash::of(text.as_bytes()),
        byte_len: text.len() as i64,
    }])
    .unwrap();
    let source =
        SourceArtifact::from_bytes(input.id(), "guide.md".into(), text.as_bytes()).unwrap();
    let q = AssertionQualification {
        scope: CoverageScope::Artifact {
            artifact: source.id(),
        }
        .id(),
        ..data.qualifications.iter().next().unwrap().clone()
    };
    let evidence = Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: text.len() as i64,
    };
    let span = EvidenceSourceSpanId::of(&evidence).unwrap();
    let node = DocumentNode::Passage { span, ordinal: 0 };
    let passage = PassageObservation {
        qualification: q.id(),
        passage: DocumentNodePassageId::of(&node).unwrap(),
        level: 0,
        heading: None,
        heading_path: vec![],
        text: text.into(),
    };
    data.artifacts.insert(source.clone()).unwrap();
    data.uses
        .insert(ArtifactUse {
            artifact: source.id(),
            input: input.id(),
            role: SourceRole::Document,
        })
        .unwrap();
    data.qualifications.insert(q).unwrap();
    data.evidence.insert(evidence).unwrap();
    data.nodes.insert(node).unwrap();
    data.passages.insert(passage).unwrap();
    let workspace = run(&data, true, 8 << 20, 1).await;
    assert_eq!(
        workspace.completed::<TextAssessment>().unwrap().rows(),
        3,
        "two release declarations and documentary passage, without the dependency declaration"
    );
}
