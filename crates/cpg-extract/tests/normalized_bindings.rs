//! Independent Python examples through all native facts and N1-N5, including private replay.
#[path = "typed_driver/mod.rs"] mod typed_driver;
use lctx_model::domain::{*, calls::*, normalized::{Rows, bindings::*, signature_applicability::*,
    entity_normalization, relation_normalization, callable_normalization, event_normalization, binding_normalization::*}, resources::ResourceBudget};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);
async fn fixture() -> (BindingData, BindingOutput, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("normalized_bindings"), Facts(tables.clone())).await.unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities = entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &budget).unwrap();
    let mut data = BindingData::new(&budget);
    macro_rules! raw { ($($field:ident: $ty:ty,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_binding_inputs!(raw);
    macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_relation_outputs!(links);
    rebuild_callables(&mut data, &budget);
    rebuild_events(&mut data, &budget);
    let output = normalize(&data, &budget).unwrap(); (data, output, budget)
}
fn rebuild_events(data: &mut BindingData, budget: &ResourceBudget) {
    let mut events = event_normalization::EventData::new(budget);
    macro_rules! event_inputs { ($($field:ident: $ty:ty,)*) => { $(events.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(event_inputs);
    let events = event_normalization::normalize(&events, budget).unwrap();
    macro_rules! event_outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&events.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_event_outputs!(event_outputs);
}
fn rebuild_callables(data: &mut BindingData, budget: &ResourceBudget) {
    let mut callables = callable_normalization::CallableData::new(budget);
    macro_rules! inputs { ($($field:ident: $ty:ty,)*) => { $(callables.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(inputs);
    let callables = callable_normalization::normalize(&callables, budget).unwrap();
    data.callable_assessments = Rows::new(budget); data.callable_decorators = Rows::new(budget); data.callable_premises = Rows::new(budget);
    data.callable_evidence = Rows::new(budget); data.callable_variants = Rows::new(budget); data.callable_slots = Rows::new(budget); data.callable_slot_entities = Rows::new(budget);
    macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&callables.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_callable_outputs!(outputs);
}
fn text(data: &BindingData, attempt: &CallBindingAttempt) -> String {
    let event = data.event_events.get(attempt.event).unwrap(); let occurrence = data.occurrences.get(event.site).unwrap();
    String::from_utf8(files("normalized_bindings")["calls.py"][occurrence.start as usize..occurrence.end as usize].to_vec()).unwrap()
}
#[tokio::test]
async fn native_binding_defaults_refusals_and_source_authority() {
    let (data, output, budget) = fixture().await;
    let verified = verify(&data, &output, &budget).unwrap();
    let at = |s: &str| output.attempts.iter().filter(|a| text(&data, a) == s).collect::<Vec<_>>();
    let plain = at("plain(1)"); assert_eq!(plain.len(), 1);
    let a = plain[0]; assert_eq!(a.outcome, BindingOutcome::Bound); assert_eq!(a.authority, BindingAuthority::EffectiveInvocation);
    let target = data.targets.get(data.event_alternatives.get(a.alternative).unwrap().target).unwrap();
    assert_eq!(verified.bound(a.id()).unwrap().bound().target(), target.id(), "the original native target is retained");
    assert!(verified.bound(a.id()).is_some()); assert!(verified.composition(a.id()).is_some(), "{:#?}", output.sets.iter().collect::<Vec<_>>());
    assert!(output.bindings.iter().filter(|b| b.attempt == a.id()).any(|b| matches!(output.sources.get(b.source), Some(BindingSource::Default))));
    assert!(at("plain(1, 2, 3)").iter().all(|a| a.outcome == BindingOutcome::ProvenIncompatible));
    assert!(at("plain(*items)").iter().all(|a| a.outcome == BindingOutcome::Undetermined && a.refusal == Some(attribution::ObligationKind::UnsupportedUnpacking)));
    let wrapped = at("decorated(1)"); assert!(!wrapped.is_empty());
    assert!(wrapped.iter().any(|a| a.outcome == BindingOutcome::Bound));
    for a in wrapped { assert_eq!(a.authority, BindingAuthority::SourceInspection); assert!(verified.composition(a.id()).is_none()); }
    let variants = at("variant(1)"); assert_eq!(variants.len(), 2);
    assert_eq!(variants.iter().filter(|a| a.outcome == BindingOutcome::Bound).count(), 1);
    assert_eq!(variants.iter().filter(|a| a.outcome == BindingOutcome::ProvenIncompatible).count(), 1);
    assert!(output.sets.iter().any(|s| s.event == variants[0].event && s.unique && s.incompatible == 1));
    for alternative in data.event_alternatives.iter() { assert!(output.attempts.iter().any(|a| a.alternative == alternative.id()), "every alternative has an outcome"); }
}
#[tokio::test]
async fn shared_replay_refuses_omitted_variants_bindings_and_forged_authority() {
    let (data, output, budget) = fixture().await;
    for omit in [None, Some(CallBinding::NAME), Some(BindingVariantAssessment::NAME), Some(BindingSetMember::NAME)] {
        let mut check = (invariants().remove(0).create)(&budget);
        macro_rules! input { ($($field:ident: $ty:ty,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_binding_inputs!(input);
        macro_rules! out { ($($field:ident: $ty:ty,)*) => { $(if omit != Some(<$ty>::NAME) { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_binding_outputs!(out);
        assert_eq!(check.finish().is_ok(), omit.is_none(), "{omit:?}");
    }
    let tiny = ResourceBudget::fixed(64).unwrap(); assert!(matches!(normalize(&data, &tiny), Err(ModelError::Resource { .. }))); assert_eq!(tiny.reserved(), 0);
    let mut forged = normalize(&data, &budget).unwrap(); forged.attempts = Rows::new(&budget);
    for row in output.attempts.iter() {
        let mut row = row.clone();
        if text(&data, &row) == "decorated(1)" { row.authority = BindingAuthority::EffectiveInvocation; row.authority_reason = AuthorityReason::Established; }
        forged.attempts.insert(row).unwrap();
    }
    assert!(verify(&data, &forged, &budget).is_err());
}

#[tokio::test]
async fn a_bound_source_plus_unknown_variant_or_missing_coverage_never_becomes_unique() {
    for form in [SignatureForm::NativeUnavailable, SignatureForm::Ellipsis, SignatureForm::ParamSpec, SignatureForm::List] {
        let (mut data, output, budget) = fixture().await;
        let original = output.attempts.iter().find(|a| text(&data, a) == "plain(1)").unwrap();
        let event = original.event; let raw = data.signatures.get(original.signature.unwrap()).unwrap().clone();
        let shapes = if form == SignatureForm::List { vec![ParameterShape { name: Some("x".into()), kind: ParameterKind::PositionalOrKeyword, required: true }] } else { vec![] };
        let (signature, parameters) = Signature::new(data.qualifications.get(raw.qualification).unwrap(), raw.symbol, 1, form, &shapes).unwrap();
        let support = data.signature_supports.iter().find(|s| s.assertion == raw.id()).unwrap().clone();
        data.signature_supports.insert(SignatureSupport { assertion: signature.id(), ..support }).unwrap();
        data.signatures.insert(signature).unwrap();
        for shape in shapes { data.shapes.insert(shape).unwrap(); } for parameter in parameters { data.parameters.insert(parameter).unwrap(); }
        rebuild_callables(&mut data, &budget);
        let output = normalize(&data, &budget).unwrap(); let verified = verify(&data, &output, &budget).unwrap();
        assert_eq!(output.attempts.iter().filter(|a| a.event == event).count(), 2);
        let set = output.sets.iter().find(|s| s.event == event).unwrap(); assert!(!set.unique, "{form:?}: {set:?}");
        if form == SignatureForm::List { assert_eq!(set.bound, 2); } else { assert!(set.undetermined > 0); }
        for a in output.attempts.iter().filter(|a| a.event == event) { assert!(verified.composition(a.id()).is_none()); }
        // Recomputing N5 from an omitted stored variant cannot mint tokens: upstream replay
        // still sees the complete raw signature set and rejects the forged N3 projection.
        let omitted = data.callable_variants.iter().find(|v| v.signature == raw.id()).unwrap().id();
        let retained = data.callable_variants.iter().filter(|v| v.id() != omitted).cloned().collect::<Vec<_>>();
        data.callable_variants = Rows::new(&budget); for row in retained { data.callable_variants.insert(row).unwrap(); }
        let forged = normalize(&data, &budget).unwrap(); assert!(verify(&data, &forged, &budget).is_err());
    }
    let (mut data, _, budget) = fixture().await;
    let retained = data.coverage.iter().filter(|c| c.family != attribution::FactFamily::Signatures).cloned().collect::<Vec<_>>();
    data.coverage = Rows::new(&budget); for row in retained { data.coverage.insert(row).unwrap(); }
    let output = normalize(&data, &budget).unwrap(); let verified = verify(&data, &output, &budget).unwrap();
    for a in output.attempts.iter().filter(|a| text(&data, a) == "plain(1)") { assert_eq!(a.outcome, BindingOutcome::Bound); assert!(verified.composition(a.id()).is_none()); }
}

#[tokio::test]
async fn bound_effective_invocation_still_requires_positive_body_admission() {
    let (mut data, output, budget) = fixture().await;
    let a = output.attempts.iter().find(|a| text(&data, a) == "plain(1)").unwrap();
    let callable = data.callable_assessments.get(a.effective.unwrap()).unwrap().callable;
    let lctx_model::domain::normalized::entities::CallableEntity::Source { declaration, .. } = data.callables.get(callable).unwrap() else { panic!("source") };
    let bodies = data.bodies.iter().cloned().map(|mut b| { if b.declaration == *declaration { b.abstract_method = true; } b }).collect::<Vec<_>>();
    data.bodies = Rows::new(&budget); for body in bodies { data.bodies.insert(body).unwrap(); }
    rebuild_callables(&mut data, &budget);
    let output = normalize(&data, &budget).unwrap(); let verified = verify(&data, &output, &budget).unwrap();
    let a = output.attempts.iter().find(|a| text(&data, a) == "plain(1)").unwrap();
    assert_eq!(a.outcome, BindingOutcome::Bound); assert_eq!(a.authority, BindingAuthority::EffectiveInvocation);
    assert!(verified.bound(a.id()).is_some()); assert!(verified.composition(a.id()).is_none());
}

#[tokio::test]
async fn recomputing_higher_layers_cannot_certify_forged_correspondence_owner_or_lexical_premises() {
    use lctx_model::domain::normalized::{entities::EntityReason, links::LinkReason};
    for mutation in 0..4 {
        let (mut data, output, budget) = fixture().await;
        let attempt = output.attempts.iter().find(|a| text(&data, a) == "plain(1)").unwrap();
        match mutation {
            0 => {
                let raw = data.signatures.get(attempt.signature.unwrap()).unwrap();
                let rows = data.symbol_resolutions.iter().cloned().map(|mut r| { if r.symbol == raw.symbol { r.reason = EntityReason::ProviderExternal; } r }).collect::<Vec<_>>();
                data.symbol_resolutions = Rows::new(&budget); for row in rows { data.symbol_resolutions.insert(row).unwrap(); }
            }
            1 => {
                let event = data.event_events.get(attempt.event).unwrap(); let other = data.owners.iter().find(|o| o.occurrence != event.site && o.owner != data.owners.get(event.owner).unwrap().owner).unwrap().clone();
                let rows = data.owners.iter().cloned().map(|mut o| { if o.id() == event.owner { o.owner = other.owner; o.entity = other.entity; } o }).collect::<Vec<_>>();
                data.owners = Rows::new(&budget); for row in rows { data.owners.insert(row).unwrap(); }
            }
            2 => {
                let rows = data.reference_assessments.iter().cloned().map(|mut r| { r.reason = LinkReason::MissingCorrespondence; r }).collect::<Vec<_>>();
                assert!(!rows.is_empty()); data.reference_assessments = Rows::new(&budget); for row in rows { data.reference_assessments.insert(row).unwrap(); }
            }
            _ => { data.declaration_supports = Rows::new(&budget); }
        }
        rebuild_callables(&mut data, &budget); rebuild_events(&mut data, &budget);
        let forged = normalize(&data, &budget).unwrap();
        let positive = forged.attempts.iter().find(|a| text(&data, a) == "plain(1)").unwrap();
        assert_eq!(positive.outcome, BindingOutcome::Bound, "higher layers still bind in mutation {mutation}");
        assert!(verify(&data, &forged, &budget).is_err(), "lower proof must reject mutation {mutation}");
    }
}
