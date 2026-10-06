//! Pure consumed-value and expected-window controls; no live service behavior is claimed.
use lctx_model::domain::{
    analysis::analytic_embedding::{AnalysisInvocation, AnalysisOutcome},
    embedding::{analytic::*, consumption::*, text::*, value::*, *},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
fn receipt(v: &AdmittedValue) -> ValueReceipt<'_> {
    ValueReceipt {
        input: v.input(),
        codec: VALUE_CODEC,
        digest: v.digest(),
        admitted_tokens: v.tokens(),
        bytes: v.bytes(),
    }
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
fn outcome(
    data: &ConsumptionData,
    invocation: &AnalysisInvocation,
    uses: &Rows<AnalysisEmbeddingUse>,
    budget: &ResourceBudget,
) -> (Rows<AnalysisInvocation>, Rows<AnalysisOutcome>) {
    let mut invocations = Rows::new(budget);
    invocations.insert(invocation.clone()).unwrap();
    let mut outcomes = Rows::new(budget);
    outcomes
        .insert(data.outcome(invocation, uses).unwrap())
        .unwrap();
    (invocations, outcomes)
}
#[test]
fn canonical_winner_replays_signed_zero_and_refuses_second_values_or_changed_requests() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let spec = spec();
    let text = "same text";
    let request = spec.document_text(text);
    let value = AdmittedValue::new(&spec, &request, 4, &[1.0, 0.0, -0.0], &budget).unwrap();
    let mut winners = Winners::new(&budget);
    let decoded = winners.replay(&spec, text, receipt(&value)).unwrap();
    assert_eq!(decoded.values()[2].to_bits(), (-0.0f32).to_bits());
    drop(decoded);
    assert!(winners.replay(&spec, "changed", receipt(&value)).is_err());
    let conflict = AdmittedValue::new(&spec, &request, 4, &[1.0, 0.0, 0.0], &budget).unwrap();
    assert!(winners.replay(&spec, text, receipt(&conflict)).is_err());
    let mut corrupt = receipt(&value);
    corrupt.digest = ContentHash::of(b"corrupt");
    assert!(winners.replay(&spec, text, corrupt).is_err());
    let mut codec = receipt(&value);
    codec.codec = 2;
    assert!(winners.replay(&spec, text, codec).is_err());
    drop(winners);
    drop(value);
    drop(conflict);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn analytic_window_consumption_is_exact_complete_and_releases_its_retained_bytes() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let (data, invocation, window, specification) = fixture(&budget, true);
    let spec = spec();
    let mut uses = Rows::new(&budget);
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(window.text.as_str()),
        7,
        &[1.0, 0.0, -0.0],
        &budget,
    )
    .unwrap();
    let key = AnalysisEmbeddingUse::admit_into(
        &mut uses,
        invocation.id(),
        window.id(),
        &specification,
        &value,
        &budget,
    )
    .unwrap();
    drop(value);
    let (invocations, outcomes) = outcome(&data, &invocation, &uses, &budget);
    data.validate(&invocations, &outcomes, &uses, &budget)
        .unwrap();
    let empty = Rows::new(&budget);
    assert!(
        data.validate(&invocations, &outcomes, &empty, &budget)
            .is_err()
    );
    assert!(
        data.validate(&Rows::new(&budget), &Rows::new(&budget), &empty, &budget)
            .is_err(),
        "coupled removal cannot erase a captured source frame"
    );
    let mut forged = uses.get(key).unwrap().clone();
    forged.input = ContentHash::of(b"changed request");
    let mut changed = Rows::new(&budget);
    changed.insert(forged).unwrap();
    assert!(
        data.validate(&invocations, &outcomes, &changed, &budget)
            .is_err()
    );
    drop(data);
    drop(uses);
    drop(invocations);
    drop(outcomes);
    drop(empty);
    drop(changed);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn token_limits_and_service_availability_remain_explicit_partial_outcomes() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let (data, invocation, window, specification) = fixture(&budget, true);
    let spec = spec();
    for (availability, tokens, reason) in [
        (
            VectorAvailability::ServiceUnavailable,
            None,
            obligation::ObligationKind::EmbeddingServiceUnavailable,
        ),
        (
            VectorAvailability::TokenLimit,
            Some(i64::from(spec.max_document_tokens) + 1),
            obligation::ObligationKind::EmbeddingTokenLimit,
        ),
    ] {
        let mut uses = Rows::new(&budget);
        let row = AnalysisEmbeddingUse {
            invocation: invocation.id(),
            window: window.id(),
            specification: specification.id(),
            input: input_hash(&spec.document_text(window.text.as_str())),
            availability,
            admitted_tokens: tokens,
            codec: None,
            value_digest: None,
            bytes: None,
        };
        uses.insert(row.clone()).unwrap();
        let (invocations, outcomes) = outcome(&data, &invocation, &uses, &budget);
        assert_eq!(outcomes.iter().next().unwrap().reason, Some(reason));
        data.validate(&invocations, &outcomes, &uses, &budget)
            .unwrap();
        if tokens.is_some() {
            let mut wrong = Rows::new(&budget);
            wrong
                .insert(AnalysisEmbeddingUse {
                    admitted_tokens: Some(1),
                    ..row
                })
                .unwrap();
            assert!(
                data.validate(&invocations, &outcomes, &wrong, &budget)
                    .is_err()
            );
        }
    }
    drop(data);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn unrequested_owner_has_no_uses_and_missing_selection_or_foreign_spec_refuses() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let (mut data, invocation, window, specification) = fixture(&budget, false);
    let mut uses = Rows::new(&budget);
    let (invocations, outcomes) = outcome(&data, &invocation, &uses, &budget);
    assert_eq!(
        outcomes.iter().next().unwrap().status,
        analysis::AnalysisStatus::NotRequested
    );
    data.validate(&invocations, &outcomes, &uses, &budget)
        .unwrap();
    let spec = spec();
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(window.text.as_str()),
        1,
        &[1.0, 0.0, 0.0],
        &budget,
    )
    .unwrap();
    AnalysisEmbeddingUse::admit_into(
        &mut uses,
        invocation.id(),
        window.id(),
        &specification,
        &value,
        &budget,
    )
    .unwrap();
    assert!(
        data.validate(&invocations, &outcomes, &uses, &budget)
            .is_err()
    );
    let mut foreign = spec.clone();
    foreign.revision.push('x');
    assert!(
        AnalysisEmbeddingUse::admit_into(
            &mut uses,
            invocation.id(),
            window.id(),
            &EmbeddingSpec::new(&foreign).unwrap(),
            &value,
            &budget
        )
        .is_err()
    );
    data.text_definitions = Rows::new(&budget);
    assert!(data.selected().is_err());
    drop(data);
    drop(uses);
    drop(invocations);
    drop(outcomes);
    drop(value);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn value_copy_is_admitted_before_allocation() {
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let (_, invocation, window, specification) = fixture(&budget, true);
    let spec = spec();
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(window.text.as_str()),
        1,
        &[1.0, 0.0, 0.0],
        &budget,
    )
    .unwrap();
    let tiny = ResourceBudget::fixed(1).unwrap();
    let mut rows = Rows::new(&tiny);
    assert!(
        AnalysisEmbeddingUse::admit_into(
            &mut rows,
            invocation.id(),
            window.id(),
            &specification,
            &value,
            &tiny
        )
        .is_err()
    );
    assert!(rows.is_empty());
    assert_eq!(tiny.reserved(), 0);
    drop(value);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn canonical_selected_service_preserves_analytic_consumption_and_refuses_foreign_specification() {
    use lctx_model::domain::graph::{self, Entity, EntityKind};
    let budget = ResourceBudget::fixed(2 << 20).unwrap();
    let (data, invocation, window, specification) = fixture(&budget, true);
    let service = data.services.iter().next().unwrap();
    let entity = Entity::from(service.clone());
    assert_eq!(entity.kind(), EntityKind::Definition);
    let restored: Entity = serde_json::from_slice(&serde_json::to_vec(&entity).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(entity, restored);
    let service =
        graph::record::entity_record::<configuration::ServiceConfiguration>(&restored).unwrap();
    assert_eq!(data.services.iter().next(), Some(&service));
    let mut selected = std::collections::BTreeSet::new();
    macro_rules! inventory { ($($variant:ident:$ty:ty,)*) => {$(selected.insert(<$ty>::NAME);)*}; }
    lctx_model::graph_entity_records!(inventory);
    lctx_model::graph_assertion_records!(inventory);
    let invariant = invariants()
        .into_iter()
        .find(|i| i.name == "analytic_embedding_consumption")
        .unwrap();
    assert_eq!(invariant.purpose, InvariantPurpose::Admission);
    for input in &invariant.inputs {
        assert!(
            selected.contains(input.name()),
            "missing canonical consumption premise {}",
            input.name()
        );
    }
    let spec = spec();
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(window.text.as_str()),
        7,
        &[1.0, 0.0, -0.0],
        &budget,
    )
    .unwrap();
    let mut uses = Rows::new(&budget);
    AnalysisEmbeddingUse::admit_into(
        &mut uses,
        invocation.id(),
        window.id(),
        &specification,
        &value,
        &budget,
    )
    .unwrap();
    let outcome = AnalysisOutcome {
        invocation: invocation.id(),
        status: analysis::AnalysisStatus::Completed,
        reason: None,
    };
    let check = |service: Option<&configuration::ServiceConfiguration>| -> Result<(), ModelError> {
        let mut check = (invariant.create)(&budget);
        macro_rules! visit {
            ($ty:ty,$rows:expr) => {
                check.visit(<$ty>::NAME, &<$ty>::encode($rows)?)?;
            };
        }
        visit!(EmbeddingSpec, std::slice::from_ref(&specification));
        visit!(
            configuration::ServiceConfiguration,
            &service.into_iter().cloned().collect::<Vec<_>>()
        );
        for row in data.text_definitions.iter() {
            visit!(TextDefinition, std::slice::from_ref(row));
        }
        for row in data.runs.iter() {
            visit!(attribution::ProviderRun, std::slice::from_ref(row));
        }
        for row in data.assessments.iter() {
            visit!(TextAssessment, std::slice::from_ref(row));
        }
        visit!(AnalysisInvocation, std::slice::from_ref(&invocation));
        visit!(TextWindow, std::slice::from_ref(&window));
        for row in uses.iter() {
            visit!(AnalysisEmbeddingUse, std::slice::from_ref(row));
        }
        visit!(AnalysisOutcome, std::slice::from_ref(&outcome));
        check.finish()
    };
    check(Some(&service)).unwrap();
    assert!(
        check(None)
            .unwrap_err()
            .to_string()
            .contains("one completed selected service")
    );
    let foreign = configuration::ServiceConfiguration {
        specification: id(99),
        endpoint: service.endpoint.clone(),
    };
    Entity::from(foreign.clone()).validate().unwrap();
    assert!(
        check(Some(&foreign))
            .unwrap_err()
            .to_string()
            .contains("another specification")
    );
    let invalid = configuration::ServiceConfiguration {
        endpoint: " ".into(),
        ..service
    };
    assert!(Entity::from(invalid).validate().is_err());
}
