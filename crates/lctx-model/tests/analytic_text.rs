use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::ParameterKind, conditions::Diagram,
    embedding::text::*, input::*, normalized::entities::*, source::*, syntax::*, *,
};

fn fixture(budget: &resources::ResourceBudget) -> TextData {
    let text = "def outer(a, /, *args, option=None, **kw):\n    \"\"\"Do source work.\"\"\"\n    return a\n";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "module.py".into(),
        content: ContentHash::of(text.as_bytes()),
        byte_len: text.len() as i64,
    }])
    .unwrap();
    let source =
        SourceArtifact::from_bytes(input.id(), "module.py".into(), text.as_bytes()).unwrap();
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
#[test]
fn source_declaration_text_keeps_parameters_docstring_and_explicit_selection() {
    let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
    let data = fixture(&budget);
    let mut definition = TextDefinition::builtin();
    assert!(
        prepare(&data, &definition, &budget)
            .unwrap()
            .assessments
            .is_empty()
    );
    definition.requested = true;
    let out = prepare(&data, &definition, &budget).unwrap();
    assert_eq!(out.assessments.len(), 1);
    assert_eq!(out.windows.len(), 1);
    assert_eq!(
        out.windows.iter().next().unwrap().text.as_str(),
        "package.module.outer(a, /, *args, option=None, **kw)\nDo source work."
    );
    assert_eq!(
        out.assessments.iter().next().unwrap().availability,
        TextAvailability::Available
    );
    drop(out);
    drop(data);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn windows_preserve_all_bytes_across_newlines_and_unicode_and_release_their_index() {
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    for (text, cap, expected) in [
        ("ab\ncd\nef", 5, vec!["ab\n", "cd\nef"]),
        ("αβγδε\nz", 5, vec!["αβ", "γδ", "ε\nz"]),
        ("", 4, vec![""]),
        ("abcdefghi", 4, vec!["abcd", "efgh", "i"]),
    ] {
        let windows = windows(text, cap, &budget).unwrap();
        let actual = windows.iter().map(|(_, text)| text).collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(actual.concat(), text);
        for (start, part) in windows.iter() {
            assert_eq!(&text[start..start + part.len()], part);
            assert!(part.len() <= cap);
        }
    }
    assert_eq!(budget.reserved(), 0);
    assert!(windows("hello", 3, &budget).is_err());
    assert!(windows("hello", 4, &resources::ResourceBudget::fixed(1).unwrap()).is_err());
}
#[test]
fn missing_normalized_entity_or_captured_bytes_is_unavailable_without_invented_text() {
    let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
    let mut data = fixture(&budget);
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    data.entities = normalized::Rows::new(&budget);
    let out = prepare(&data, &definition, &budget).unwrap();
    assert!(out.windows.is_empty());
    assert_eq!(
        out.assessments.iter().next().unwrap().boundary,
        Some(TextBoundary::MissingEntity)
    );
    drop(out);
    drop(data);
    let mut data = fixture(&budget);
    data.chunks = normalized::Rows::new(&budget);
    let out = prepare(&data, &definition, &budget).unwrap();
    assert!(out.windows.is_empty());
    assert_eq!(
        out.assessments.iter().next().unwrap().boundary,
        Some(TextBoundary::MissingSource)
    );
    drop(out);
    drop(data);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn stored_replay_refuses_rewritten_text_missing_windows_and_foreign_contexts() {
    let budget = resources::ResourceBudget::fixed(1 << 26).unwrap();
    let data = fixture(&budget);
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    let output = prepare(&data, &definition, &budget).unwrap();
    let mut relations = normalized_relations();
    relations.extend(lctx_model::domain::embedding::text::relations());
    let model = ValidatedModel::validate(relations).unwrap();
    let check = |kind| {
        let invariant = TextAssessment::invariants().remove(0);
        let mut checker = (invariant.create)(&budget);
        macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(let batch=Batch::new(&model,data.$field.iter().cloned().collect::<Vec<_>>(),&budget).unwrap();checker.visit(<$ty>::NAME,batch.arrow()).unwrap();)*};}
        lctx_model::analytic_text_inputs!(input);
        macro_rules! feed {
            ($ty:ty,$rows:expr) => {{
                let batch = Batch::new(&model, $rows, &budget).unwrap();
                checker.visit(<$ty>::NAME, batch.arrow()).unwrap();
            }};
        }
        feed!(TextDefinition, vec![definition.clone()]);
        feed!(
            TextSubject,
            output.subjects.iter().cloned().collect::<Vec<_>>()
        );
        let mut assessments = output.assessments.iter().cloned().collect::<Vec<_>>();
        if kind == 3 {
            let context = AnalysisContext {
                python_version: "3.14.7".into(),
                python_platform: "foreign".into(),
                search_path: vec![],
                site_package_path: vec![],
                config_digest: ContentHash::of(b"foreign"),
                environment_digest: ContentHash::of(b"foreign"),
                lock_digest: None,
            };
            assessments[0].context = context.id();
        }
        feed!(TextAssessment, assessments);
        let mut windows = output.windows.iter().cloned().collect::<Vec<_>>();
        if kind == 1 {
            windows[0].text = "plausible but invented source".into();
            windows[0].end = windows[0].text.len() as i64;
            windows[0].content = ContentHash::of(windows[0].text.as_bytes());
        }
        if kind == 2 {
            windows.clear();
        }
        feed!(TextWindow, windows);
        checker.finish()
    };
    check(0).unwrap();
    for kind in 1..=3 {
        assert!(check(kind).is_err(), "forged text kind {kind}");
    }
    drop(output);
    drop(data);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn analytic_text_declaration_has_only_completed_earlier_inputs_and_frozen_vocabulary() {
    let definition = TextDefinition {
        requested: true,
        ..TextDefinition::builtin()
    };
    for profile in stages::Profile::ALL {
        let declared = stage(profile, &definition).unwrap();
        assert_eq!(declared.effect, stages::Effect::Pure);
        let earlier = normalized_relations()
            .into_iter()
            .map(|r| r.name())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(declared.inputs.iter().all(|r| earlier.contains(r.name())));
        assert!(declared.reads::<ArtifactChunk>());
        assert!(declared.reads::<CallableEntity>());
        assert!(declared.reads::<documents::PassageObservation>());
    }
}
