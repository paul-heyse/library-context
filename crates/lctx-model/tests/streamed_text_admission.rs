//! Independent required text admission; these controls never replay the producer in admission.
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::ParameterKind, conditions::Diagram,
    embedding::text::*, input::*, normalized::entities::*, source::*, syntax::*, *,
};

fn fixture(budget: &resources::ResourceBudget, path: &str) -> TextData {
    let text = "def αouter(a, /, *args, option=None, **kw):\n    \"\"\"α🙂 source work.\"\"\"\n    return a\n";
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
        text,
        SyntaxKind::StmtFunctionDef,
        OccurrenceRole::Declaration,
    );
    let name = occurrence("αouter", SyntaxKind::Identifier, OccurrenceRole::Syntax);
    let doc = occurrence(
        "\"\"\"α🙂 source work.\"\"\"",
        SyntaxKind::StmtExpr,
        OccurrenceRole::Syntax,
    );
    let doc_literal = occurrence(
        "\"\"\"α🙂 source work.\"\"\"",
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
        value: "α🙂 source work.".into(),
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

fn check(
    data: &TextData,
    output: &TextOutput,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let invariant = validation::invariants_for::<TextAssessment>()
        .into_iter()
        .find(|row| row.purpose == InvariantPurpose::Admission)
        .unwrap();
    assert_eq!(invariant.name, "analytic_text_original_membership");
    let mut check = (invariant.create)(budget);
    for declaration in &invariant.inputs {
        macro_rules! facts {($($field:ident:$ty:ty,)*)=>{$(if declaration.name()==<$ty>::NAME && <$ty>::NAME!=ArtifactChunk::NAME {for row in data.$field.iter() {check.visit(<$ty>::NAME,&<$ty as Record>::encode(std::slice::from_ref(row))?)?;}})*};}
        lctx_model::analytic_text_inputs!(facts);
        if declaration.name() == ArtifactChunk::NAME {
            let mut chunks = data.chunks.iter().collect::<Vec<_>>();
            chunks.sort_by_key(|row| (row.artifact, row.ordinal));
            for row in chunks {
                check.visit(
                    ArtifactChunk::NAME,
                    &<ArtifactChunk as Record>::encode(std::slice::from_ref(row))?,
                )?;
            }
        }
        macro_rules! result {
            ($field:ident,$ty:ty) => {
                if declaration.name() == <$ty>::NAME {
                    for row in output.$field.iter() {
                        check.visit(<$ty>::NAME, &<$ty as Record>::encode(std::slice::from_ref(row))?)?;
                    }
                }
            };
        }
        result!(definitions, TextDefinition);
        result!(subjects, TextSubject);
        result!(assessments, TextAssessment);
        if declaration.name() == TextWindow::NAME {
            let mut windows = output.windows.iter().collect::<Vec<_>>();
            windows.sort_by_key(|row| (row.assessment, row.ordinal));
            for row in windows {
                check.visit(
                    TextWindow::NAME,
                    &<TextWindow as Record>::encode(std::slice::from_ref(row))?,
                )?;
            }
        }
    }
    check.finish()
}
fn copied(output: &TextOutput, budget: &resources::ResourceBudget) -> TextOutput {
    let mut copy = TextOutput::new(budget);
    for row in output.definitions.iter() {
        copy.definitions.insert(row.clone()).unwrap();
    }
    for row in output.subjects.iter() {
        copy.subjects.insert(row.clone()).unwrap();
    }
    for row in output.assessments.iter() {
        copy.assessments.insert(row.clone()).unwrap();
    }
    for row in output.windows.iter() {
        copy.windows.insert(row.clone()).unwrap();
    }
    copy
}
#[test]
fn required_text_admission_checks_original_unicode_bytes_source_and_canonical_windows() {
    let fixture_budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let mut data = fixture(&fixture_budget, "api.py");
    let other = fixture(&fixture_budget, "other.py");
    macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in other.$field.iter() {data.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::analytic_text_inputs!(merge);
    let definition = TextDefinition {
        requested: true,
        window_bytes: 32,
        ..TextDefinition::builtin()
    };
    let output = prepare(&data, &definition, &fixture_budget).unwrap();
    check(&data, &output, &budget).unwrap();
    assert_eq!(budget.reserved(), 0);
    for mode in 0..4 {
        let mut forged = copied(&output, &fixture_budget);
        let assessment = output.assessments.iter().next().unwrap();
        if mode == 0 {
            forged.assessments = normalized::Rows::new(&fixture_budget);
            for row in output.assessments.iter() {
                let mut row = row.clone();
                if row.id() == assessment.id() {
                    row.source = data
                        .artifacts
                        .iter()
                        .find(|source| source.id() != row.source)
                        .unwrap()
                        .id();
                }
                forged.assessments.insert(row).unwrap();
            }
        } else {
            forged.windows = normalized::Rows::new(&fixture_budget);
            for row in output.windows.iter() {
                let mut row = row.clone();
                if row.assessment == assessment.id() && row.ordinal == 0 {
                    match mode {
                        1 => {
                            row.text = "rewritten α bytes".into();
                            row.end = row.start + row.text.len() as i64;
                            row.content = ContentHash::of(row.text.as_bytes());
                        }
                        2 => row.ordinal = 7,
                        3 => continue,
                        _ => unreachable!(),
                    }
                }
                forged.windows.insert(row).unwrap();
            }
        }
        assert!(
            check(&data, &forged, &budget).is_err(),
            "forgery mode {mode}"
        );
        assert_eq!(budget.reserved(), 0);
    }
    let mut fragmented = copied(&output, &fixture_budget);
    fragmented.windows = normalized::Rows::new(&fixture_budget);
    for assessment in output.assessments.iter() {
        let mut windows = output
            .windows
            .iter()
            .filter(|row| row.assessment == assessment.id())
            .collect::<Vec<_>>();
        windows.sort_by_key(|row| row.ordinal);
        let text = windows
            .iter()
            .map(|row| row.text.as_str())
            .collect::<String>();
        for (ordinal, (start, character)) in text.char_indices().enumerate() {
            let part = character.to_string();
            fragmented
                .windows
                .insert(TextWindow {
                    assessment: assessment.id(),
                    ordinal: ordinal as i64,
                    start: start as i64,
                    end: (start + part.len()) as i64,
                    text: part.clone().into(),
                    content: ContentHash::of(part.as_bytes()),
                })
                .unwrap();
        }
    }
    assert!(
        check(&data, &fragmented, &budget).is_err(),
        "equal concatenated bytes cannot change canonical window boundaries"
    );
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn altered_captured_span_and_existing_unselected_subject_refuse_without_replay() {
    let fixture_budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let data = fixture(&fixture_budget, "api.py");
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    let output = prepare(&data, &definition, &fixture_budget).unwrap();
    let mut changed = fixture(&fixture_budget, "api.py");
    let name = changed.declarations.iter().next().unwrap().name;
    let original = changed.occurrences.iter().cloned().collect::<Vec<_>>();
    changed.occurrences = normalized::Rows::new(&fixture_budget);
    for mut row in original {
        if row.id() == name {
            row.start += 1;
        }
        changed.occurrences.insert(row).unwrap();
    }
    assert!(
        check(&changed, &output, &budget).is_err(),
        "original name span changes cannot admit old rendered bytes"
    );
    assert_eq!(budget.reserved(), 0);
    let mut data = fixture(&fixture_budget, "api.py");
    let mut dependency = fixture(&fixture_budget, "dependency.py");
    let artifact = dependency.artifacts.iter().next().unwrap().clone();
    dependency.uses = normalized::Rows::new(&fixture_budget);
    dependency
        .uses
        .insert(ArtifactUse {
            artifact: artifact.id(),
            input: artifact.input,
            role: SourceRole::Dependency,
        })
        .unwrap();
    macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in dependency.$field.iter() {data.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::analytic_text_inputs!(merge);
    let mut forged = copied(&output, &fixture_budget);
    forged
        .subjects
        .insert(TextSubject::Declaration {
            declaration: dependency.declarations.iter().next().unwrap().id(),
        })
        .unwrap();
    assert!(
        check(&data, &forged, &budget).is_err(),
        "an existing dependency declaration is outside admitted text roots"
    );
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn empty_and_not_requested_text_are_legitimate_without_payloads() {
    let fixture_budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let data = TextData::new(&fixture_budget);
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    let output = prepare(&data, &definition, &fixture_budget).unwrap();
    check(&data, &output, &budget).unwrap();
    assert_eq!(budget.reserved(), 0);
    let data = fixture(&fixture_budget, "api.py");
    let output = prepare(&data, &TextDefinition::builtin(), &fixture_budget).unwrap();
    check(&data, &output, &budget).unwrap();
    assert_eq!(budget.reserved(), 0);
    for missing_entity in [false, true] {
        let mut data = fixture(&fixture_budget, "api.py");
        if missing_entity {
            data.entities = normalized::Rows::new(&fixture_budget);
        } else {
            data.chunks = normalized::Rows::new(&fixture_budget);
        }
        let output = prepare(&data, &definition, &fixture_budget).unwrap();
        assert_eq!(
            output.assessments.iter().next().unwrap().availability,
            TextAvailability::Unavailable
        );
        assert!(output.windows.is_empty());
        check(&data, &output, &budget).unwrap();
        assert_eq!(budget.reserved(), 0);
    }
}

#[test]
fn original_passage_payloads_and_completed_window_payloads_do_not_accumulate_in_admission() {
    use lctx_model::domain::documents::*;
    let fixture_budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
    let budget = resources::ResourceBudget::fixed(2 << 20).unwrap();
    let text = "α🙂 source bytes\n".repeat(512);
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "guide.md".into(),
        content: ContentHash::of(text.as_bytes()),
        byte_len: text.len() as i64,
    }])
    .unwrap();
    let artifact =
        SourceArtifact::from_bytes(input.id(), "guide.md".into(), text.as_bytes()).unwrap();
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"fixture"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let q = AssertionQualification {
        assumptions: assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: CoverageScope::Artifact {
            artifact: artifact.id(),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let evidence = Evidence::SourceSpan {
        source: artifact.id(),
        start: 0,
        end: artifact.byte_len,
    };
    let span = EvidenceSourceSpanId::of(&evidence).unwrap();
    let mut data = TextData::new(&fixture_budget);
    data.artifacts.insert(artifact.clone()).unwrap();
    data.uses
        .insert(ArtifactUse {
            artifact: artifact.id(),
            input: input.id(),
            role: SourceRole::Document,
        })
        .unwrap();
    data.qualifications.insert(q.clone()).unwrap();
    data.evidence.insert(evidence).unwrap();
    for ordinal in 0..1500 {
        let node = DocumentNode::Passage { span, ordinal };
        let passage = PassageObservation {
            qualification: q.id(),
            passage: DocumentNodePassageId::of(&node).unwrap(),
            level: 0,
            heading: None,
            heading_path: vec![],
            text: text.clone(),
        };
        data.nodes.insert(node).unwrap();
        data.passages.insert(passage).unwrap();
    }
    let node = DocumentNode::Passage {
        span,
        ordinal: 1500,
    };
    let passage = PassageObservation {
        qualification: q.id(),
        passage: DocumentNodePassageId::of(&node).unwrap(),
        level: 0,
        heading: None,
        heading_path: vec![],
        text: String::new(),
    };
    data.nodes.insert(node).unwrap();
    data.passages.insert(passage).unwrap();
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    let output = prepare(&data, &definition, &fixture_budget).unwrap();
    assert_eq!(
        output
            .windows
            .iter()
            .filter(|window| window.text.is_empty())
            .count(),
        1,
        "a legitimately empty captured passage has one empty window"
    );
    assert!(output.windows.len() > 3000);
    assert!(text.len() * data.passages.len() > 8 << 20);
    check(&data, &output, &budget).unwrap();
    assert_eq!(
        budget.reserved(),
        0,
        "all admission dictionaries and current window fragments must release"
    );
}
