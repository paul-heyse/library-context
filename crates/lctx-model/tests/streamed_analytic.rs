//! Pure consumed-value and expected-window controls; no live service behavior is claimed.
use lctx_model::domain::{
    analysis::analytic_embedding::AnalysisInvocation,
    embedding::{analytic::*, text::*, value::*, *},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
fn spec() -> Spec {
    let mut spec = Spec::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/embedding/qwen3-embedding-8b.json"
    )))
    .unwrap();
    spec.reduction = "none".into();
    spec.admission = None;
    spec.dimensions = 3;
    spec.source_dimensions = 3;
    spec.document_template = "prefix: {text}".into();
    spec
}
fn fixture(
    budget: &ResourceBudget,
    selected: bool,
) -> (
    ConsumptionData,
    AnalysisInvocation,
    TextWindow,
    EmbeddingSpec,
) {
    let spec = spec();
    let specification = EmbeddingSpec::new(&spec).unwrap();
    let mut data = ConsumptionData::new(budget);
    let definition = TextDefinition {
        requested: selected,
        ..TextDefinition::builtin()
    };
    data.text_definitions.insert(definition.clone()).unwrap();
    data.specifications.insert(specification.clone()).unwrap();
    data.services
        .insert(configuration::ServiceConfiguration {
            specification: specification.id(),
            endpoint: "fixture://service".into(),
        })
        .unwrap();
    data.runs
        .insert(attribution::ProviderRun {
            provider: id(6),
            context: id(3),
            input: id(2),
            configuration: ContentHash::of(b"configuration"),
            requested_families: ContentHash::of(b"families"),
        })
        .unwrap();
    let assessment = TextAssessment {
        subject: id(1),
        definition: definition.id(),
        input: id(2),
        context: id(3),
        entity: Some(id(4)),
        source: id(5),
        availability: TextAvailability::Available,
        boundary: None,
    };
    data.assessments.insert(assessment.clone()).unwrap();
    let text = "original α bytes";
    let window = TextWindow {
        assessment: assessment.id(),
        ordinal: 0,
        start: 0,
        end: text.len() as i64,
        text: text.into(),
        content: ContentHash::of(text.as_bytes()),
    };
    data.windows.insert(window.clone()).unwrap();
    let (invocation, _) = AnalysisInvocation::new(
        assessment.input,
        assessment.context,
        analytic::definition().1.id(),
        None,
        [],
    );
    (data, invocation, window, specification)
}

fn visit<R: Record>(check: &mut dyn InvariantCheck, row: &R) -> Result<(), ModelError> {
    check.visit(R::NAME, &<R as Record>::encode(std::slice::from_ref(row))?)
}
fn admission(
    data: &ConsumptionData,
    invocation: &AnalysisInvocation,
    budget: &ResourceBudget,
) -> Box<dyn InvariantCheck> {
    let invariant = invariants()
        .into_iter()
        .find(|row| row.name == "analytic_embedding_consumption")
        .unwrap();
    assert_eq!(invariant.purpose, InvariantPurpose::Admission);
    assert_eq!(
        invariant
            .inputs
            .iter()
            .find(|input| input.name() == AnalysisEmbeddingUse::NAME)
            .unwrap()
            .order(),
        &["specification", "input", "invocation", "window"]
    );
    let mut check = (invariant.create)(budget);
    for row in data.specifications.iter() {
        visit(&mut *check, row).unwrap();
    }
    for row in data.services.iter() {
        visit(&mut *check, row).unwrap();
    }
    for row in data.text_definitions.iter() {
        visit(&mut *check, row).unwrap();
    }
    for row in data.runs.iter() {
        visit(&mut *check, row).unwrap();
    }
    for row in data.assessments.iter() {
        visit(&mut *check, row).unwrap();
    }
    visit(&mut *check, invocation).unwrap();
    check
}
#[test]
fn ordered_admission_drops_rich_windows_and_checks_exact_tokens_without_producer_replay() {
    let fixture_budget = ResourceBudget::fixed(8 << 20).unwrap();
    let (data, invocation, window, specification) = fixture(&fixture_budget, true);
    let spec = spec();
    let budget = ResourceBudget::fixed(2 << 20).unwrap();
    let mut check = admission(&data, &invocation, &budget);
    let text = "original α bytes\n".repeat(512);
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(&text),
        7,
        &[1.0, 0.0, -0.0],
        &fixture_budget,
    )
    .unwrap();
    let mut keys = Vec::new();
    for ordinal in 0..4000 {
        let row = TextWindow {
            ordinal,
            text: text.clone().into(),
            start: ordinal * text.len() as i64,
            end: (ordinal + 1) * text.len() as i64,
            content: ContentHash::of(text.as_bytes()),
            ..window.clone()
        };
        keys.push(row.id());
        visit(&mut *check, &row).unwrap();
    }
    assert!(
        budget.reserved() < 1 << 20,
        "only compact request membership may survive the 32 MiB text stream"
    );
    for window in keys {
        let mut uses = Rows::new(&fixture_budget);
        AnalysisEmbeddingUse::admit_into(
            &mut uses,
            invocation.id(),
            window,
            &specification,
            &value,
            &fixture_budget,
        )
        .unwrap();
        visit(&mut *check, uses.iter().next().unwrap()).unwrap();
    }
    visit(
        &mut *check,
        &FrameOutcome::new(true).outcome(invocation.id()),
    )
    .unwrap();
    check.finish().unwrap();
    assert_eq!(budget.reserved(), 0);

    for tokens in [7, 8] {
        let mut check = admission(&data, &invocation, &budget);
        let second = TextWindow {
            ordinal: 1,
            start: window.end,
            end: window.end * 2,
            ..window.clone()
        };
        visit(&mut *check, &window).unwrap();
        visit(&mut *check, &second).unwrap();
        let first = AdmittedValue::new(
            &spec,
            &spec.document_text(window.text.as_str()),
            7,
            &[1.0, 0.0, -0.0],
            &fixture_budget,
        )
        .unwrap();
        let conflicting = AdmittedValue::new(
            &spec,
            &spec.document_text(window.text.as_str()),
            tokens,
            &[1.0, 0.0, if tokens == 7 { 0.0 } else { -0.0 }],
            &fixture_budget,
        )
        .unwrap();
        let mut uses = Rows::new(&fixture_budget);
        AnalysisEmbeddingUse::admit_into(
            &mut uses,
            invocation.id(),
            window.id(),
            &specification,
            &first,
            &fixture_budget,
        )
        .unwrap();
        visit(&mut *check, uses.iter().next().unwrap()).unwrap();
        let mut uses = Rows::new(&fixture_budget);
        AnalysisEmbeddingUse::admit_into(
            &mut uses,
            invocation.id(),
            second.id(),
            &specification,
            &conflicting,
            &fixture_budget,
        )
        .unwrap();
        assert!(
            visit(&mut *check, uses.iter().next().unwrap()).is_err(),
            "signed-zero bytes and token receipts both belong to the exact winner"
        );
        drop(check);
        assert_eq!(budget.reserved(), 0);
    }
}
#[test]
fn streaming_window_boundaries_preserve_unicode_lines_and_release_without_an_index() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    for text in [
        "",
        "\n",
        "one\ntwo\n",
        "αβγδε\n🙂🙂🙂\nend",
        "abcdefghijk\na\nb\nc",
        "a\nbcdefghijklmnop",
    ] {
        for cap in 4..20 {
            let expected = windows(text, cap, &budget).unwrap();
            assert_eq!(
                stream_windows(text, cap).unwrap().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>(),
                "text={text:?}, cap={cap}"
            );
        }
    }
    assert_eq!(budget.reserved(), 0);
    let wide = "🙂\n".repeat(100_000);
    assert_eq!(stream_windows(&wide, 5).unwrap().count(), 100_000);
    assert!(
        windows(&wide, 5, &budget).is_err(),
        "the oracle's full index exceeds the small budget while production has no index"
    );
    assert!(stream_windows("test", 3).is_err());
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn independent_admission_refuses_missing_and_rewritten_window_consumption() {
    let fixture_budget = ResourceBudget::fixed(8 << 20).unwrap();
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let (data, invocation, window, specification) = fixture(&fixture_budget, true);
    let spec = spec();
    let mut missing = admission(&data, &invocation, &budget);
    visit(&mut *missing, &window).unwrap();
    visit(
        &mut *missing,
        &FrameOutcome::new(true).outcome(invocation.id()),
    )
    .unwrap();
    assert!(missing.finish().is_err());
    assert_eq!(budget.reserved(), 0);
    let mut rewritten = admission(&data, &invocation, &budget);
    visit(&mut *rewritten, &window).unwrap();
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text("rewritten bytes"),
        7,
        &[1.0, 0.0, -0.0],
        &fixture_budget,
    )
    .unwrap();
    let mut uses = Rows::new(&fixture_budget);
    AnalysisEmbeddingUse::admit_into(
        &mut uses,
        invocation.id(),
        window.id(),
        &specification,
        &value,
        &fixture_budget,
    )
    .unwrap();
    assert!(visit(&mut *rewritten, uses.iter().next().unwrap()).is_err());
    drop(rewritten);
    assert_eq!(budget.reserved(), 0);
}
