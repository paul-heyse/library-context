//! Independent Python examples through all native facts and N1-N5, including private replay.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    calls::*,
    normalized::{
        Rows, binding_normalization::*, bindings::*, callable_normalization, entity_normalization,
        event_normalization, relation_normalization, signature_applicability::*,
    },
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);
async fn fixture_tables() -> (
    BindingData,
    BindingOutput,
    ResourceBudget,
    typed_driver::Tables,
) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("normalized_bindings"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities =
        entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
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
    let output = normalize(&data, &budget).unwrap();
    (data, output, budget, tables)
}
async fn fixture() -> (BindingData, BindingOutput, ResourceBudget) {
    let (data, output, budget, _) = fixture_tables().await;
    (data, output, budget)
}
fn rebuild_events(data: &mut BindingData, budget: &ResourceBudget) {
    let mut receivers = lctx_model::domain::normalized::receiver::ReceiverData::new(budget);
    macro_rules! receiver_inputs {($($field:ident: $ty:ty,)*)=>{$(receivers.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(receiver_inputs);
    let receivers =
        lctx_model::domain::normalized::receiver::normalize(&receivers, budget).unwrap();
    data.receiver_assessments = Rows::new(budget);
    data.receiver_evidence = Rows::new(budget);
    data.receiver_premises = Rows::new(budget);
    macro_rules! receiver_outputs {($($field:ident: $ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&receivers.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_receiver_outputs!(receiver_outputs);

    let mut events = event_normalization::EventData::new(budget);
    macro_rules! event_inputs { ($($field:ident: $ty:ty,)*) => { $(events.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(event_inputs);
    let events = event_normalization::normalize(&events, budget).unwrap();
    let mut retained = BindingData::new(budget);
    let event_relations = event_normalization::EventOutput::validation_inputs();
    macro_rules! retain_inputs { ($($field:ident: $ty:ty,)*) => { $(if !event_relations.iter().any(|input|input.name()==<$ty>::NAME) { retained.$field.decode(&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
    lctx_model::normalized_binding_inputs!(retain_inputs);
    *data = retained;
    macro_rules! event_outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&events.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_event_outputs!(event_outputs);
}
fn rebuild_callables(data: &mut BindingData, budget: &ResourceBudget) {
    let mut callables = callable_normalization::CallableData::new(budget);
    macro_rules! inputs { ($($field:ident: $ty:ty,)*) => { $(callables.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(inputs);
    let callables = callable_normalization::normalize(&callables, budget).unwrap();
    data.callable_assessments = Rows::new(budget);
    data.callable_decorators = Rows::new(budget);
    data.callable_premises = Rows::new(budget);
    data.callable_evidence = Rows::new(budget);
    data.callable_variants = Rows::new(budget);
    data.callable_slots = Rows::new(budget);
    data.callable_slot_entities = Rows::new(budget);
    macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&callables.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_callable_outputs!(outputs);
}
// Failure-only bounded support frames for the retained native ClassOf operation.
fn diagnose_receiver(data: &BindingData, target: &CallTarget) {
    eprintln!("RECEIVER_TARGET {target:?}");
    for assessment in data
        .receiver_assessments
        .iter()
        .filter(|a| a.target() == target.id())
        .take(4)
    {
        eprintln!("RECEIVER_ASSESSMENT {assessment:?}");
    }
    for support in data
        .target_supports
        .iter()
        .filter(|s| s.assertion == target.id())
        .take(4)
    {
        eprintln!(
            "RECEIVER_TARGET_SUPPORT {support:?} run={:?}",
            data.runs.get(support.run)
        );
    }
    for syntax in data.syntax.iter().filter(|s| s.site == target.site).take(4) {
        eprintln!("RECEIVER_SYNTAX {syntax:?}");
        for support in data
            .syntax_supports
            .iter()
            .filter(|s| s.assertion == syntax.id())
            .take(4)
        {
            eprintln!(
                "RECEIVER_SYNTAX_SUPPORT {support:?} run={:?}",
                data.runs.get(support.run)
            );
        }
        for placement in data
            .placements
            .iter()
            .filter(|p| p.parent == Some(syntax.callee) && p.field == lexical::SyntaxField::Value)
            .take(4)
        {
            eprintln!("RECEIVER_VALUE {placement:?}");
            for support in data
                .placement_supports
                .iter()
                .filter(|s| s.assertion == placement.id())
                .take(4)
            {
                eprintln!(
                    "RECEIVER_VALUE_SUPPORT {support:?} run={:?}",
                    data.runs.get(support.run)
                );
            }
        }
    }
    let qualification = data.qualifications.get(target.qualification).unwrap();
    for coverage in data
        .coverage
        .iter()
        .filter(|c| {
            c.context == qualification.context
                && c.scope == qualification.scope
                && matches!(
                    c.family,
                    attribution::FactFamily::Calls | attribution::FactFamily::Syntax
                )
        })
        .take(12)
    {
        eprintln!("RECEIVER_COVERAGE {coverage:?}");
    }
}
fn text(data: &BindingData, attempt: &CallBindingAttempt) -> String {
    let event = data.event_events.get(attempt.event).unwrap();
    let occurrence = data.occurrences.get(event.site).unwrap();
    String::from_utf8(
        files("normalized_bindings")["calls.py"]
            [occurrence.start as usize..occurrence.end as usize]
            .to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn native_binding_defaults_refusals_and_source_authority() {
    let (data, output, budget) = fixture().await;
    let verified = verify(&data, &output, &budget).unwrap();
    let at = |s: &str| {
        output
            .attempts
            .iter()
            .filter(|a| text(&data, a) == s)
            .collect::<Vec<_>>()
    };
    let plain = at("plain(1)");
    assert_eq!(plain.len(), 1);
    let a = plain[0];
    assert_eq!(a.outcome, BindingOutcome::Bound);
    assert_eq!(a.authority, BindingAuthority::EffectiveInvocation);
    let target = data
        .targets
        .get(
            data.event_alternative_sources
                .get(data.event_alternatives.get(a.alternative).unwrap().source)
                .unwrap()
                .target(),
        )
        .unwrap();
    assert_eq!(
        verified.bound(a.id()).unwrap().bound().target(),
        target.id(),
        "the original native target is retained"
    );
    assert!(verified.bound(a.id()).is_some());
    let effective = verified
        .effective_invocation(a.id())
        .expect("effective invocation survives independently of source-body admission");
    assert!(effective.admits(verified.bound(a.id()).unwrap()));
    assert_eq!(
        (
            effective.attempt(),
            effective.event(),
            effective.context(),
            effective.target()
        ),
        (
            a.id(),
            a.event,
            verified.bound(a.id()).unwrap().context(),
            verified.bound(a.id()).unwrap().bound().target()
        )
    );
    assert!(
        verified.composition(a.id()).is_some(),
        "{:#?}",
        output.sets.iter().collect::<Vec<_>>()
    );
    assert!(
        output
            .bindings
            .iter()
            .filter(|b| b.attempt == a.id())
            .any(|b| matches!(output.sources.get(b.source), Some(BindingSource::Default)))
    );
    assert!(
        at("plain(1, 2, 3)")
            .iter()
            .all(|a| a.outcome == BindingOutcome::ProvenIncompatible)
    );
    assert!(
        at("plain(*items)")
            .iter()
            .all(|a| a.outcome == BindingOutcome::Undetermined
                && a.refusal == Some(attribution::ObligationKind::UnsupportedUnpacking))
    );
    let wrapped = at("decorated(1)");
    assert!(!wrapped.is_empty());
    assert!(wrapped.iter().any(|a| a.outcome == BindingOutcome::Bound));
    for a in wrapped {
        assert_eq!(a.authority, BindingAuthority::SourceInspection);
        assert!(verified.composition(a.id()).is_none());
        assert!(verified.effective_invocation(a.id()).is_none());
    }
    let variants = at("variant(1)");
    assert_eq!(variants.len(), 2);
    assert_eq!(
        variants
            .iter()
            .filter(|a| a.outcome == BindingOutcome::Bound)
            .count(),
        1
    );
    assert_eq!(
        variants
            .iter()
            .filter(|a| a.outcome == BindingOutcome::ProvenIncompatible)
            .count(),
        1
    );
    assert!(
        output
            .sets
            .iter()
            .any(|s| s.event == variants[0].event && s.unique && s.incompatible == 1)
    );
    for alternative in data.event_alternatives.iter() {
        assert!(
            output
                .attempts
                .iter()
                .any(|a| a.alternative == alternative.id()),
            "every alternative has an outcome"
        );
    }
}
#[tokio::test]
async fn shared_replay_refuses_omitted_variants_bindings_and_forged_authority() {
    let (data, output, budget) = fixture().await;
    for omit in [
        None,
        Some(CallBinding::NAME),
        Some(BindingVariantAssessment::NAME),
        Some(BindingSetMember::NAME),
    ] {
        let mut check = (invariants().remove(0).create)(&budget);
        macro_rules! input { ($($field:ident: $ty:ty,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_binding_inputs!(input);
        macro_rules! out { ($($field:ident: $ty:ty,)*) => { $(if omit != Some(<$ty>::NAME) { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_binding_outputs!(out);
        assert_eq!(check.finish().is_ok(), omit.is_none(), "{omit:?}");
    }
    let tiny = ResourceBudget::fixed(64).unwrap();
    assert!(matches!(
        normalize(&data, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
    let mut forged = normalize(&data, &budget).unwrap();
    forged.attempts = Rows::new(&budget);
    for row in output.attempts.iter() {
        let mut row = row.clone();
        if text(&data, &row) == "decorated(1)" {
            row.authority = BindingAuthority::EffectiveInvocation;
            row.authority_reason = AuthorityReason::Established;
        }
        forged.attempts.insert(row).unwrap();
    }
    assert!(verify(&data, &forged, &budget).is_err());
}

#[tokio::test]
async fn a_bound_source_plus_unknown_variant_or_missing_coverage_never_becomes_unique() {
    for form in [
        SignatureForm::NativeUnavailable,
        SignatureForm::Ellipsis,
        SignatureForm::ParamSpec,
        SignatureForm::List,
    ] {
        let (mut data, output, budget) = fixture().await;
        let original = output
            .attempts
            .iter()
            .find(|a| text(&data, a) == "plain(1)")
            .unwrap();
        let event = original.event;
        let raw = data
            .signatures
            .get(original.signature.unwrap())
            .unwrap()
            .clone();
        let shapes = if form == SignatureForm::List {
            vec![ParameterShape {
                name: Some("x".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            }]
        } else {
            vec![]
        };
        let (signature, parameters) = Signature::new(
            data.qualifications.get(raw.qualification).unwrap(),
            lctx_model::domain::calls::SignatureRole::Source,
            None,
            raw.symbol,
            1,
            form,
            &shapes,
        )
        .unwrap();
        let support = data
            .signature_supports
            .iter()
            .find(|s| s.assertion == raw.id())
            .unwrap()
            .clone();
        data.signature_supports
            .insert(SignatureSupport {
                assertion: signature.id(),
                ..support
            })
            .unwrap();
        data.signatures.insert(signature).unwrap();
        for shape in shapes {
            data.shapes.insert(shape).unwrap();
        }
        for parameter in parameters {
            data.parameters.insert(parameter).unwrap();
        }
        rebuild_callables(&mut data, &budget);
        let output = normalize(&data, &budget).unwrap();
        // A new native variant must also replace the exact qualified enumeration. Keeping
        // the old header is invalid before any callable/binding authority can be admitted.
        assert!(verify(&data, &output, &budget).is_err());
        let previous = data
            .signature_enumerations
            .iter()
            .find(|e| e.symbol == raw.symbol && e.qualification == raw.qualification)
            .unwrap()
            .clone();
        let mut variants = data
            .signatures
            .iter()
            .filter(|s| {
                s.symbol == raw.symbol && s.qualification == raw.qualification && s.role == raw.role
            })
            .collect::<Vec<_>>();
        variants.sort_by_key(|s| s.variant);
        let (enumeration, members) = SignatureEnumerationObservation::new(
            data.qualifications.get(raw.qualification).unwrap(),
            raw.symbol,
            variants.iter().copied(),
            previous.complete,
        )
        .unwrap();
        let retained = data
            .signature_enumerations
            .iter()
            .filter(|e| e.id() != previous.id())
            .cloned()
            .collect::<Vec<_>>();
        data.signature_enumerations = Rows::new(&budget);
        for row in retained {
            data.signature_enumerations.insert(row).unwrap();
        }
        data.signature_enumerations
            .insert(enumeration.clone())
            .unwrap();
        let retained = data
            .signature_enumeration_members
            .iter()
            .filter(|m| m.enumeration != previous.id())
            .cloned()
            .collect::<Vec<_>>();
        data.signature_enumeration_members = Rows::new(&budget);
        for row in retained.into_iter().chain(members) {
            data.signature_enumeration_members.insert(row).unwrap();
        }
        let supports = data
            .signature_enumeration_supports
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        data.signature_enumeration_supports = Rows::new(&budget);
        for mut row in supports {
            if row.assertion == previous.id() {
                row.assertion = enumeration.id();
            }
            data.signature_enumeration_supports.insert(row).unwrap();
        }
        let verified = verify(&data, &output, &budget).unwrap();
        assert_eq!(
            output.attempts.iter().filter(|a| a.event == event).count(),
            2
        );
        let set = output.sets.iter().find(|s| s.event == event).unwrap();
        assert!(!set.unique, "{form:?}: {set:?}");
        if form == SignatureForm::List {
            assert_eq!(set.bound, 2);
        } else {
            assert!(set.undetermined > 0);
        }
        for a in output.attempts.iter().filter(|a| a.event == event) {
            assert!(verified.composition(a.id()).is_none());
            assert!(verified.effective_invocation(a.id()).is_none());
        }
        // Recomputing N5 from an omitted stored variant cannot mint tokens: upstream replay
        // still sees the complete raw signature set and rejects the forged N3 projection.
        let omitted = data
            .callable_variants
            .iter()
            .find(|v| v.signature == raw.id())
            .unwrap()
            .id();
        let retained = data
            .callable_variants
            .iter()
            .filter(|v| v.id() != omitted)
            .cloned()
            .collect::<Vec<_>>();
        data.callable_variants = Rows::new(&budget);
        for row in retained {
            data.callable_variants.insert(row).unwrap();
        }
        let refused = match normalize(&data, &budget) {
            Err(_) => true,
            Ok(forged) => verify(&data, &forged, &budget).is_err(),
        };
        assert!(
            refused,
            "omitting an upstream variant must not mint authority"
        );
    }
    let (mut data, _, budget) = fixture().await;
    let retained = data
        .coverage
        .iter()
        .filter(|c| c.family != attribution::FactFamily::Signatures)
        .cloned()
        .collect::<Vec<_>>();
    data.coverage = Rows::new(&budget);
    for row in retained {
        data.coverage.insert(row).unwrap();
    }
    rebuild_events(&mut data, &budget);
    let output = normalize(&data, &budget).unwrap();
    let verified = verify(&data, &output, &budget).unwrap();
    for a in output
        .attempts
        .iter()
        .filter(|a| text(&data, a) == "plain(1)")
    {
        assert_eq!(a.outcome, BindingOutcome::Bound);
        assert!(verified.composition(a.id()).is_none());
        assert!(verified.effective_invocation(a.id()).is_none());
    }
}

#[tokio::test]
async fn bound_effective_invocation_still_requires_positive_body_admission() {
    let (mut data, output, budget) = fixture().await;
    let a = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "plain(1)")
        .unwrap();
    let callable = data
        .callable_assessments
        .get(a.effective.unwrap())
        .unwrap()
        .callable;
    let lctx_model::domain::normalized::entities::CallableEntity::Source { declaration, .. } =
        data.callables.get(callable).unwrap()
    else {
        panic!("source")
    };
    let bodies = data
        .bodies
        .iter()
        .cloned()
        .map(|mut b| {
            if b.declaration == *declaration {
                b.abstract_method = true;
            }
            b
        })
        .collect::<Vec<_>>();
    data.bodies = Rows::new(&budget);
    for body in bodies {
        data.bodies.insert(body).unwrap();
    }
    rebuild_callables(&mut data, &budget);
    let output = normalize(&data, &budget).unwrap();
    let verified = verify(&data, &output, &budget).unwrap();
    let a = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "plain(1)")
        .unwrap();
    assert_eq!(a.outcome, BindingOutcome::Bound);
    assert_eq!(a.authority, BindingAuthority::EffectiveInvocation);
    assert!(verified.bound(a.id()).is_some());
    let effective = verified
        .effective_invocation(a.id())
        .expect("effective invocation survives independently of source-body admission");
    assert!(effective.admits(verified.bound(a.id()).unwrap()));
    assert_eq!(
        (
            effective.attempt(),
            effective.event(),
            effective.context(),
            effective.target()
        ),
        (
            a.id(),
            a.event,
            verified.bound(a.id()).unwrap().context(),
            verified.bound(a.id()).unwrap().bound().target()
        )
    );
    assert!(verified.composition(a.id()).is_none());
}

#[tokio::test]
async fn recomputing_higher_layers_cannot_certify_forged_correspondence_owner_or_lexical_premises()
{
    use lctx_model::domain::normalized::{entities::EntityReason, links::LinkReason};
    for mutation in 0..4 {
        let (mut data, output, budget) = fixture().await;
        let attempt = output
            .attempts
            .iter()
            .find(|a| text(&data, a) == "plain(1)")
            .unwrap();
        match mutation {
            0 => {
                let raw = data.signatures.get(attempt.signature.unwrap()).unwrap();
                let rows = data
                    .symbol_resolutions
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        if r.symbol == raw.symbol {
                            r.reason = EntityReason::ProviderExternal;
                        }
                        r
                    })
                    .collect::<Vec<_>>();
                data.symbol_resolutions = Rows::new(&budget);
                for row in rows {
                    data.symbol_resolutions.insert(row).unwrap();
                }
            }
            1 => {
                let event = data.event_events.get(attempt.event).unwrap();
                let other = data
                    .owners
                    .iter()
                    .find(|o| {
                        o.occurrence != event.site
                            && o.owner != data.owners.get(event.owner).unwrap().owner
                    })
                    .unwrap()
                    .clone();
                let rows = data
                    .owners
                    .iter()
                    .cloned()
                    .map(|mut o| {
                        if o.id() == event.owner {
                            o.owner = other.owner;
                            o.entity = other.entity;
                        }
                        o
                    })
                    .collect::<Vec<_>>();
                data.owners = Rows::new(&budget);
                for row in rows {
                    data.owners.insert(row).unwrap();
                }
            }
            2 => {
                let rows = data
                    .reference_assessments
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        r.reason = LinkReason::MissingCorrespondence;
                        r
                    })
                    .collect::<Vec<_>>();
                assert!(!rows.is_empty());
                data.reference_assessments = Rows::new(&budget);
                for row in rows {
                    data.reference_assessments.insert(row).unwrap();
                }
            }
            _ => {
                data.declaration_supports = Rows::new(&budget);
            }
        }
        rebuild_callables(&mut data, &budget);
        rebuild_events(&mut data, &budget);
        let forged = normalize(&data, &budget).unwrap();
        let positive = forged
            .attempts
            .iter()
            .find(|a| text(&data, a) == "plain(1)")
            .unwrap();
        assert_eq!(
            positive.outcome,
            BindingOutcome::Bound,
            "higher layers still bind in mutation {mutation}"
        );
        assert!(
            verify(&data, &forged, &budget).is_err(),
            "lower proof must reject mutation {mutation}"
        );
    }
}

#[tokio::test]
async fn class_of_receiver_twins_preserve_raw_unknown_and_target_uncertainty() {
    use lctx_model::domain::normalized::receiver::{ReceiverAssessment, ReceiverPremise};
    let (data, output, budget) = fixture().await;
    let find = |name: &str| {
        output
            .attempts
            .iter()
            .find(|a| text(&data, a) == name)
            .unwrap()
    };
    let class = find("ReceiverOwner.class_call(1)");
    let object = find("obj.class_call(1)");
    let target = data
        .targets
        .get(
            data.event_alternative_sources
                .get(
                    data.event_alternatives
                        .get(object.alternative)
                        .unwrap()
                        .source,
                )
                .unwrap()
                .target(),
        )
        .unwrap();
    assert!(
        matches!(
            data.receivers.get(target.receiver),
            Some(Receiver::Unknown { .. })
        ),
        "raw receiver must stay unknown"
    );
    if object.outcome != BindingOutcome::Bound {
        diagnose_receiver(&data, target);
    }
    assert_eq!(class.outcome, BindingOutcome::Bound);
    assert_eq!(class.authority, BindingAuthority::EffectiveInvocation);
    assert_eq!(
        object.outcome,
        BindingOutcome::Bound,
        "captured named shape binds while dispatch remains open"
    );
    assert_eq!(object.authority, BindingAuthority::SourceInspection);
    assert_eq!(
        data.qualifications
            .get(target.qualification)
            .unwrap()
            .modality,
        attribution::Modality::Candidate
    );
    let assessment = data
        .receiver_assessments
        .iter()
        .find(|a| a.target() == target.id())
        .unwrap();
    let ReceiverAssessment::ClassOf {
        actual,
        placement,
        syntax,
        ..
    } = assessment
    else {
        panic!("{assessment:?}")
    };
    let call = data.syntax.get(*syntax).unwrap();
    let place = data.placements.get(*placement).unwrap();
    assert_eq!(place.parent, Some(call.callee));
    assert_eq!(place.field, lexical::SyntaxField::Value);
    assert_eq!(place.occurrence, *actual);
    for premise in ["target", "syntax", "placement", "coverage"] {
        assert!(
            data.receiver_evidence
                .iter()
                .filter(|m| m.assessment == assessment.id())
                .any(|m| matches!(
                    (premise, data.receiver_premises.get(m.premise)),
                    ("target", Some(ReceiverPremise::Target { .. }))
                        | ("syntax", Some(ReceiverPremise::Syntax { .. }))
                        | ("placement", Some(ReceiverPremise::Placement { .. }))
                        | ("coverage", Some(ReceiverPremise::Coverage { .. }))
                )),
            "{premise}"
        );
    }
    assert!(
        data.event_assessments
            .iter()
            .any(|a| a.event == object.event && a.known_receivers && !a.exact)
    );
    assert!(!data.event_admissions.iter().any(|a| {
        a.alternative == object.alternative
            && data
                .event_policy_assessments
                .get(a.assessment)
                .unwrap()
                .policy
                == lctx_model::domain::normalized::events::CallPolicy::Summary
    }));
    let verified = verify(&data, &output, &budget).unwrap();
    assert!(verified.bound(class.id()).is_some());
    assert!(verified.composition(object.id()).is_none());
    assert!(
        output
            .bindings
            .iter()
            .filter(|b| b.attempt == object.id() && b.kind == BindingKind::Receiver)
            .all(|b| matches!(
                output.sources.get(b.source),
                Some(BindingSource::ClassOf { .. })
            ))
    );
}

fn receiver_input(
    data: &BindingData,
    budget: &ResourceBudget,
) -> lctx_model::domain::normalized::receiver::ReceiverData {
    let mut inputs = lctx_model::domain::normalized::receiver::ReceiverData::new(budget);
    macro_rules! collect {($($field:ident: $ty:ty,)*)=>{$(inputs.$field.decode(&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_receiver_inputs!(collect);
    inputs
}
#[tokio::test]
async fn class_of_replay_refuses_missing_ambiguous_foreign_and_contradictory_premises() {
    use lctx_model::domain::normalized::receiver::{self, ReceiverAssessment};
    use lctx_model::domain::syntax::SyntaxPlacement;
    let (data, output, budget) = fixture().await;
    let attempt = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "obj.class_call(1)")
        .unwrap();
    let target = data
        .targets
        .get(
            data.event_alternative_sources
                .get(
                    data.event_alternatives
                        .get(attempt.alternative)
                        .unwrap()
                        .source,
                )
                .unwrap()
                .target(),
        )
        .unwrap();
    let baseline = receiver_input(&data, &budget);
    let stored = receiver::normalize(&baseline, &budget).unwrap();
    assert!(
        receiver::verify(&baseline, &stored, &budget)
            .unwrap()
            .has_class_of(target.id())
    );
    let assessment = stored
        .receiver_assessments
        .iter()
        .find(|a| a.target() == target.id())
        .unwrap();
    let ReceiverAssessment::ClassOf {
        placement,
        effective,
        ..
    } = assessment
    else {
        panic!("{assessment:?}")
    };
    let original = data.placements.get(*placement).unwrap();
    for mutation in 0..11 {
        let mut inputs = receiver_input(&data, &budget);
        match mutation {
            0 => {
                inputs.placements = Rows::new(&budget);
                for p in data.placements.iter().filter(|p| p.id() != *placement) {
                    inputs.placements.insert(p.clone()).unwrap();
                }
            }
            1 => {
                inputs
                    .placements
                    .insert(SyntaxPlacement {
                        ordinal: 1,
                        ..original.clone()
                    })
                    .unwrap();
            }
            2 => {
                let mut q = data
                    .qualifications
                    .get(original.qualification)
                    .unwrap()
                    .clone();
                q.context = attribution::AnalysisContext {
                    python_version: "3.14.7".into(),
                    python_platform: "foreign".into(),
                    search_path: vec![],
                    site_package_path: vec![],
                    config_digest: ContentHash::of(b"foreign"),
                    environment_digest: ContentHash::of(b"foreign"),
                    lock_digest: None,
                }
                .id();
                inputs.qualifications.insert(q.clone()).unwrap();
                inputs.placements = Rows::new(&budget);
                for p in data.placements.iter() {
                    let mut p = p.clone();
                    if p.id() == *placement {
                        p.qualification = q.id();
                    }
                    inputs.placements.insert(p).unwrap();
                }
            }
            3 => {
                inputs.target_supports = Rows::new(&budget);
                for p in data
                    .target_supports
                    .iter()
                    .filter(|p| p.assertion != target.id())
                {
                    inputs.target_supports.insert(p.clone()).unwrap();
                }
            }
            4 => {
                inputs.callable_assessments = Rows::new(&budget);
                for p in data.callable_assessments.iter() {
                    let mut p = p.clone();
                    if p.id() == *effective {
                        p.descriptor_kind=Some(lctx_model::domain::normalized::callables::DescriptorKind::InstanceMethod);
                    }
                    inputs.callable_assessments.insert(p).unwrap();
                }
            }
            5 => {
                inputs.targets = Rows::new(&budget);
                for t in data.targets.iter() {
                    let mut t = t.clone();
                    if t.id() == target.id() {
                        t.static_method = Some(true);
                    }
                    inputs.targets.insert(t).unwrap();
                }
            }
            6 => {
                inputs.coverage = Rows::new(&budget);
                for c in data.coverage.iter() {
                    let mut c = c.clone();
                    if c.family == attribution::FactFamily::Calls {
                        c.status = attribution::CoverageStatus::Partial;
                        c.reason = Some(attribution::ObligationKind::MissingEvidence);
                    }
                    inputs.coverage.insert(c).unwrap();
                }
            }
            7 => {
                inputs.placement_supports = Rows::new(&budget);
                for p in data
                    .placement_supports
                    .iter()
                    .filter(|p| p.assertion != *placement)
                {
                    inputs.placement_supports.insert(p.clone()).unwrap();
                }
            }
            8 => {
                let support = data
                    .placement_supports
                    .iter()
                    .find(|s| s.assertion == *placement)
                    .unwrap();
                let original_run = support.run;
                let mut run = data.runs.get(original_run).unwrap().clone();
                run.configuration = ContentHash::of(b"foreign-extraction-configuration");
                let foreign = run.id();
                inputs.runs.insert(run).unwrap();
                inputs.syntax_supports = Rows::new(&budget);
                for s in data.syntax_supports.iter() {
                    let mut s = s.clone();
                    if s.run == original_run {
                        s.run = foreign;
                    }
                    inputs.syntax_supports.insert(s).unwrap();
                }
                inputs.placement_supports = Rows::new(&budget);
                for s in data.placement_supports.iter() {
                    let mut s = s.clone();
                    if s.run == original_run {
                        s.run = foreign;
                    }
                    inputs.placement_supports.insert(s).unwrap();
                }
                for c in data.coverage.iter().filter(|c| c.run == Some(original_run)) {
                    let mut c = c.clone();
                    c.run = Some(foreign);
                    inputs.coverage.insert(c).unwrap();
                }
            }
            9 => {
                inputs.coverage = Rows::new(&budget);
                for c in data.coverage.iter() {
                    let mut c = c.clone();
                    if c.family == attribution::FactFamily::Syntax {
                        c.status = attribution::CoverageStatus::Partial;
                        c.reason = Some(attribution::ObligationKind::MissingEvidence);
                    }
                    inputs.coverage.insert(c).unwrap();
                }
            }
            10 => {
                inputs.syntax_supports = Rows::new(&budget);
                for s in data.syntax_supports.iter() {
                    let mut s = s.clone();
                    s.fidelity = attribution::Fidelity::ReportProjection;
                    inputs.syntax_supports.insert(s).unwrap();
                }
            }
            _ => unreachable!(),
        }
        let derived = receiver::normalize(&inputs, &budget).unwrap();
        assert!(
            match receiver::verify(&inputs, &derived, &budget) {
                Ok(checked) => !checked.has_class_of(target.id()),
                Err(_) => true,
            },
            "mutation {mutation} cannot mint authority"
        );
        assert!(
            receiver::verify(&inputs, &stored, &budget).is_err(),
            "old assessment must not survive mutation {mutation}"
        );
    }
    let mut forged = receiver::normalize(&baseline, &budget).unwrap();
    forged.receiver_assessments = Rows::new(&budget);
    for row in stored.receiver_assessments.iter() {
        let mut row = row.clone();
        if let ReceiverAssessment::ClassOf { actual, .. } = &mut row {
            *actual = target.site;
        }
        forged.receiver_assessments.insert(row).unwrap();
    }
    assert!(receiver::verify(&baseline, &forged, &budget).is_err());
    let mut forged = receiver::normalize(&baseline, &budget).unwrap();
    forged.receiver_evidence = Rows::new(&budget);
    assert!(receiver::verify(&baseline, &forged, &budget).is_err());
}

#[tokio::test]
async fn captured_dispatch_diamond_retains_named_and_known_overriders_without_summary() {
    use lctx_model::domain::normalized::{
        dispatch::DispatchReason,
        events::{CallAlternativeSource, CallPolicy},
    };
    let (data, output, budget) = fixture().await;
    let original = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "obj.class_call(1)")
        .unwrap();
    let assessment = data
        .dispatch_assessments
        .iter()
        .find(|a| a.event == original.event)
        .unwrap();
    assert!(assessment.open);
    assert_eq!(
        assessment.reason,
        DispatchReason::OpenRuntimeSubclasses,
        "{assessment:?}"
    );
    let members: Vec<_> = data
        .dispatch_members
        .iter()
        .filter(|m| m.assessment == assessment.id())
        .collect();
    assert_eq!(members.len(), 4, "{members:?}");
    assert_eq!(members.iter().filter(|m| m.named).count(), 1);
    let alternatives: Vec<_> = data
        .event_alternatives
        .iter()
        .filter(|a| a.event == original.event)
        .collect();
    assert_eq!(alternatives.len(), 4);
    assert_eq!(
        alternatives
            .iter()
            .filter(|a| matches!(
                data.event_alternative_sources.get(a.source),
                Some(CallAlternativeSource::Native { .. })
            ))
            .count(),
        1
    );
    let verified = verify(&data, &output, &budget).unwrap();
    for alternative in alternatives {
        let attempts: Vec<_> = output
            .attempts
            .iter()
            .filter(|a| a.alternative == alternative.id())
            .collect();
        assert!(!attempts.is_empty());
        for attempt in attempts {
            assert_eq!(attempt.outcome, BindingOutcome::Bound, "{attempt:?}");
            assert_eq!(attempt.authority, BindingAuthority::SourceInspection);
            assert!(attempt.dispatch_member.is_some());
            assert!(verified.bound(attempt.id()).is_some());
            assert!(verified.composition(attempt.id()).is_none());
            assert!(
                output
                    .bindings
                    .iter()
                    .filter(|b| b.attempt == attempt.id() && b.kind == BindingKind::Receiver)
                    .all(|b| matches!(
                        output.sources.get(b.source),
                        Some(BindingSource::ClassOf { .. })
                    ))
            );
        }
        assert!(!data.event_admissions.iter().any(|a| {
            a.alternative == alternative.id()
                && data
                    .event_policy_assessments
                    .get(a.assessment)
                    .unwrap()
                    .policy
                    == CallPolicy::Summary
        }));
    }
}

fn event_input(data: &BindingData, budget: &ResourceBudget) -> event_normalization::EventData {
    let mut input = event_normalization::EventData::new(budget);
    macro_rules! collect {($($field:ident: $ty:ty,)*)=>{$(input.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(collect);
    input
}
#[tokio::test]
async fn dispatch_replay_refuses_forged_members_and_retains_incomplete_ancestry_open() {
    use lctx_model::domain::{
        normalized::{dispatch::DispatchReason, events::CallAlternativeSource},
        symbols::*,
    };
    let (data, output, budget) = fixture().await;
    let original = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "obj.class_call(1)")
        .unwrap();
    let input = event_input(&data, &budget);
    let stored = event_normalization::normalize(&input, &budget).unwrap();
    let member = stored
        .dispatch_members
        .iter()
        .find(|m| !m.named && input.symbols.get(m.defining_class).unwrap().name == "ReceiverLeft")
        .unwrap();
    let native_target = stored
        .dispatch_assessments
        .get(member.assessment)
        .unwrap()
        .target;
    for mutation in 0..4 {
        let mut changed = event_input(&data, &budget);
        match mutation {
            0 => {
                let trait_ = changed
                    .traits
                    .iter()
                    .find(|t| t.symbol == member.symbol)
                    .unwrap()
                    .id();
                changed.trait_supports = Rows::new(&budget);
                for s in input
                    .trait_supports
                    .iter()
                    .filter(|s| s.assertion != trait_)
                {
                    changed.trait_supports.insert(s.clone()).unwrap();
                }
            }
            1 => {
                let class = changed
                    .targets
                    .get(native_target)
                    .unwrap()
                    .receiver_class
                    .unwrap();
                changed.ancestry = Rows::new(&budget);
                for a in input.ancestry.iter() {
                    let mut a = a.clone();
                    if a.class == class && a.relation == AncestryRelation::Mro {
                        a.linearization = Some(Linearization::Prefix);
                    }
                    changed.ancestry.insert(a).unwrap();
                }
            }
            2 => {
                let ancestry = changed
                    .ancestry
                    .iter()
                    .find(|a| {
                        a.class == member.defining_class && a.relation == AncestryRelation::Mro
                    })
                    .unwrap();
                let omitted = changed
                    .sequence_members
                    .iter()
                    .find(|m| m.sequence == ancestry.ancestors)
                    .unwrap()
                    .id();
                changed.sequence_members = Rows::new(&budget);
                for m in input.sequence_members.iter().filter(|m| m.id() != omitted) {
                    changed.sequence_members.insert(m.clone()).unwrap();
                }
            }
            _ => {
                let trait_ = changed
                    .traits
                    .iter()
                    .find(|t| t.symbol == member.symbol)
                    .unwrap();
                let mut q = changed
                    .qualifications
                    .get(trait_.qualification)
                    .unwrap()
                    .clone();
                q.context = attribution::AnalysisContext {
                    python_version: "3.14.7".into(),
                    python_platform: "foreign".into(),
                    search_path: vec![],
                    site_package_path: vec![],
                    config_digest: ContentHash::of(b"foreign"),
                    environment_digest: ContentHash::of(b"foreign"),
                    lock_digest: None,
                }
                .id();
                changed.qualifications.insert(q.clone()).unwrap();
                changed.traits = Rows::new(&budget);
                for t in input.traits.iter() {
                    let mut t = t.clone();
                    if t.symbol == member.symbol {
                        t.qualification = q.id();
                    }
                    changed.traits.insert(t).unwrap();
                }
            }
        }
        assert!(
            event_normalization::validate(&changed, &stored, &budget).is_err(),
            "old dispatch cannot survive mutation {mutation}"
        );
        if let Ok(rebuilt) = event_normalization::normalize(&changed, &budget) {
            assert!(
                !rebuilt
                    .dispatch_members
                    .iter()
                    .any(|m| m.symbol == member.symbol)
            );
            assert!(
                rebuilt
                    .dispatch_assessments
                    .iter()
                    .filter(|a| a.event == original.event)
                    .all(|a| a.open)
            );
            assert!(
                rebuilt
                    .alternatives
                    .iter()
                    .any(|a| a.event == original.event
                        && matches!(
                            rebuilt.alternative_sources.get(a.source),
                            Some(CallAlternativeSource::Native { .. })
                        ))
            );
            if mutation == 1 {
                assert!(
                    rebuilt
                        .dispatch_assessments
                        .iter()
                        .any(|a| a.event == original.event
                            && a.reason == DispatchReason::IncompleteAncestry)
                );
            }
        }
    }
    let mut forged = event_normalization::normalize(&input, &budget).unwrap();
    forged.dispatch_members = Rows::new(&budget);
    let other = stored
        .dispatch_members
        .iter()
        .find(|m| m.entity != member.entity)
        .unwrap()
        .entity;
    for row in stored.dispatch_members.iter() {
        let mut row = row.clone();
        if row.id() == member.id() {
            row.entity = other;
        }
        forged.dispatch_members.insert(row).unwrap();
    }
    assert!(event_normalization::validate(&input, &forged, &budget).is_err());
    let mut forged = event_normalization::normalize(&input, &budget).unwrap();
    forged.dispatch_evidence = Rows::new(&budget);
    assert!(event_normalization::validate(&input, &forged, &budget).is_err());
    let mut forged_input = BindingData::new(&budget);
    macro_rules! copy {($($field:ident: $ty:ty,)*)=>{$(forged_input.$field.decode(&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(copy);
    forged_input.dispatch_members = Rows::new(&budget);
    for row in stored.dispatch_members.iter() {
        let mut row = row.clone();
        if row.id() == member.id() {
            row.entity = other;
        }
        forged_input.dispatch_members.insert(row).unwrap();
    }
    assert!(
        normalize(&forged_input, &budget).is_err(),
        "stored membership cannot mint signature applicability"
    );
    assert!(verify(&forged_input, &output, &budget).is_err());
}

#[tokio::test]
async fn authored_runtime_contract_requires_complete_shape_and_keeps_normal_defaults_separate() {
    use lctx_model::domain::attribution::AnalysisContext;
    use lctx_model::domain::execution::model_application::*;
    let (data, output, budget, tables) = fixture_tables().await;
    let verified = verify(&data, &output, &budget).unwrap();
    let attempt = output
        .attempts
        .iter()
        .find(|a| text(&data, a) == "cast(int, value)")
        .expect("native cast attempt");
    assert_eq!(attempt.authority, BindingAuthority::SourceInspection);
    assert!(verified.effective_invocation(attempt.id()).is_none());
    let shape=verified.shape(attempt.id()).unwrap_or_else(||panic!("external cast shape absent: attempt={attempt:?}; sets={:?}; event={:?}; alternatives={:?}",output.sets.iter().filter(|s|s.event==attempt.event).collect::<Vec<_>>(),data.event_assessments.iter().filter(|s|s.event==attempt.event).collect::<Vec<_>>(),data.event_alternatives.iter().filter(|s|s.event==attempt.event).collect::<Vec<_>>()));
    let bound = verified.bound(attempt.id()).unwrap();
    assert!(shape.admits(bound));
    let mut model_data = ModelApplicationData::new(&budget);
    model_data.bindings = data;
    for context in rows::<AnalysisContext>(&tables) {
        model_data.contexts.insert(context).unwrap();
    }
    let mut inventory = analysis::native::NativeInventory::new(&budget);
    for input in analysis::native::NativeInventory::inputs() {
        if let Some(batch) = tables.lock().unwrap().get(input.name()) {
            inventory.visit(input.name(), batch).unwrap();
        }
    }
    let native = inventory.collect().unwrap();
    model_data.native = native.qualifications;
    model_data.premises = native.premises;
    let catalog = models::Catalog::committed().unwrap();
    let checked =
        CheckedModelApplication::derive(&catalog, &model_data, bound, shape, None, &budget)
            .unwrap()
            .unwrap_or_else(|r| panic!("cast applicability {r:?}"));
    assert_eq!(
        checked.compiled().model().target.key(),
        "stdlib:3.14.7:typing.cast"
    );
    let enumeration = shape
        .enumeration()
        .expect("private shape pins exact captured enumeration");
    assert_eq!(
        model_data
            .bindings
            .signature_enumeration_members
            .iter()
            .filter(|m| m.enumeration == enumeration)
            .count(),
        3
    );
    assert!(checked.premises().iter().any(|p|matches!(p,analysis::native::NativeAssertionPremise::SignatureEnumerationObservation{assertion,..} if *assertion==enumeration)));
    assert!(
        model_data
            .bindings
            .coverage
            .iter()
            .any(|c| c.family == attribution::FactFamily::Signatures
                && c.status == attribution::CoverageStatus::Partial)
    );

    assert!(checked.normal_parameter().is_some());
    assert!(!checked.call_defaults_available());
    assert_eq!(
        checked.bound().bound().bindings().len(),
        2,
        "type argument is retained independently of returned value"
    );
    let changed = models::Catalog::parse(
        "external.toml",
        &include_str!("../../lctx-model/models/external.toml").replace("3.14.7", "3.14.8"),
    )
    .unwrap();
    assert!(
        CheckedModelApplication::derive(&changed, &model_data, bound, shape, None, &budget)
            .unwrap()
            .is_err()
    );
    assert!(
        CheckedModelApplication::derive(
            &catalog,
            &model_data,
            bound,
            shape,
            None,
            &ResourceBudget::fixed(1).unwrap()
        )
        .is_err()
    );
    // Equivalent type-only overloads preserve every source member; deleting one or all refuses.
    for erase_all in [false, true] {
        let mut changed = BindingData::new(&budget);
        macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in model_data.bindings.$field.iter(){if <$ty>::NAME!=SignatureEnumerationMember::NAME{changed.$field.insert(row.clone()).unwrap();}})*};}
        lctx_model::normalized_binding_inputs!(copy);
        if !erase_all {
            for member in model_data
                .bindings
                .signature_enumeration_members
                .iter()
                .filter(|m| m.enumeration != enumeration || m.ordinal != 0)
            {
                changed
                    .signature_enumeration_members
                    .insert(member.clone())
                    .unwrap();
            }
        }
        assert!(
            verify(&changed, &output, &budget).is_err(),
            "erased enumeration members all={erase_all}"
        );
    }
    let divergent = output
        .attempts
        .iter()
        .filter(|a| text(&model_data.bindings, a) == "divergent(1)")
        .collect::<Vec<_>>();
    assert!(divergent.len() >= 2);
    assert!(
        divergent.iter().all(|a| verified.shape(a.id()).is_none()),
        "different formal shapes cannot become arbitrary first overload"
    );
}

#[tokio::test]
async fn protocol_construction_uses_allocated_receiver_and_retains_every_native_variant() {
    use execution::{
        model_application::ModelApplicationData,
        model_construction::CheckedContextConstruction,
        model_context::{CheckedContextProtocol, ContextValue},
    };
    let (data, _, budget, tables) = fixture_tables().await;
    let mut model = ModelApplicationData::new(&budget);
    model.bindings = data;
    macro_rules! pins{($($field:ident:$ty:ty,)*)=>{$(for row in rows::<$ty>(&tables){model.$field.insert(row).unwrap();})*};}
    lctx_model::model_pin_inputs!(pins);
    let mut inventory = analysis::native::NativeInventory::new(&budget);
    for input in analysis::native::NativeInventory::inputs() {
        if let Some(batch) = tables.lock().unwrap().get(input.name()) {
            inventory.visit(input.name(), batch).unwrap();
        }
    }
    let native = inventory.collect().unwrap();
    model.native = native.qualifications;
    model.premises = native.premises;
    let catalog = models::Catalog::committed().unwrap();
    let input = rows::<input::InputRevision>(&tables)[0].id();
    let context = rows::<attribution::AnalysisContext>(&tables)[0].id();
    let source = files("normalized_bindings");
    for (expected, class_name, none, variants) in [
        ("nullcontext()", "nullcontext", true, 2),
        ("nullcontext(7)", "nullcontext", false, 2),
        ("suppress(TypeError)", "suppress", true, 1),
    ] {
        let b = &model.bindings;
        let site = b
            .occurrences
            .iter()
            .find(|o| {
                o.syntax_kind == source::SyntaxKind::ExprCall
                    && &source["calls.py"][o.start as usize..o.end as usize] == expected.as_bytes()
            })
            .unwrap();
        let owner = b
            .owners
            .iter()
            .find(|o| o.occurrence == site.id())
            .unwrap()
            .entity;
        let class=b.symbols.iter().find(|s|s.name==class_name&&s.kind==SymbolKind::Class&&matches!(b.provider_modules.get(s.module),Some(ProviderModule::Bundled{name,..})if name=="contextlib")).unwrap();
        let protocol =
            CheckedContextProtocol::derive(&catalog, &model, class.id(), input, context, &budget)
                .unwrap()
                .unwrap_or_else(|r| panic!("protocol {expected}: {r:?}"));
        if class_name == "suppress" {
            assert!(
                protocol.entry_declaration().is_none(),
                "unavailable local native entry declaration does not change authored lifecycle semantics"
            );
        }
        let construction = CheckedContextConstruction::derive(
            &protocol,
            &model,
            site.id(),
            owner,
            input,
            context,
            &budget,
        )
        .unwrap()
        .unwrap_or_else(|r| panic!("construction {expected}: {r:?}"));
        assert_eq!(construction.outcomes().len(), variants);
        assert_eq!(
            construction
                .outcomes()
                .iter()
                .filter(|o| o.outcome.is_ok())
                .count(),
            1
        );
        assert_eq!(
            matches!(construction.entry_value(), ContextValue::None),
            none
        );
        assert!(!construction.premises().is_empty());
        assert_eq!(construction.resource().class, class.id());
        assert!(
            CheckedContextConstruction::derive(
                &protocol,
                &model,
                site.id(),
                owner,
                input,
                context,
                &ResourceBudget::fixed(1).unwrap()
            )
            .is_err()
        );
        let mut missing = ModelApplicationData::new(&budget);
        macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in model.bindings.$field.iter(){if <$ty>::NAME!=symbols::FunctionTraitObservation::NAME{missing.bindings.$field.insert(row.clone()).unwrap();}})*};}
        lctx_model::normalized_binding_inputs!(copy);
        macro_rules! pins_copy{($($field:ident:$ty:ty,)*)=>{$(for row in model.$field.iter(){missing.$field.insert(row.clone()).unwrap();})*};}
        lctx_model::model_pin_inputs!(pins_copy);
        assert!(
            CheckedContextConstruction::derive(
                &protocol,
                &missing,
                site.id(),
                owner,
                input,
                context,
                &budget
            )
            .unwrap()
            .is_err()
        );
        let subclass = b
            .occurrences
            .iter()
            .find(|o| {
                o.syntax_kind == source::SyntaxKind::ExprCall
                    && &source["calls.py"][o.start as usize..o.end as usize] == b"ChangedNull()"
            })
            .unwrap();
        let subclass_owner = b
            .owners
            .iter()
            .find(|o| o.occurrence == subclass.id())
            .unwrap()
            .entity;
        assert!(
            CheckedContextConstruction::derive(
                &protocol,
                &model,
                subclass.id(),
                subclass_owner,
                input,
                context,
                &budget
            )
            .unwrap()
            .is_err(),
            "subclass may override protocol methods"
        );
        let other_class=b.symbols.iter().find(|s|s.name==if class_name=="suppress"{"nullcontext"}else{"suppress"}&&s.kind==SymbolKind::Class&&matches!(b.provider_modules.get(s.module),Some(ProviderModule::Bundled{name,..})if name=="contextlib")).unwrap();
        let wrong_protocol = CheckedContextProtocol::derive(
            &catalog,
            &model,
            other_class.id(),
            input,
            context,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert!(
            CheckedContextConstruction::derive(
                &wrong_protocol,
                &model,
                site.id(),
                owner,
                input,
                context,
                &budget
            )
            .unwrap()
            .is_err(),
            "different authored protocol class cannot admit this callee"
        );
        for omit_class in [false, true] {
            let mut missing = ModelApplicationData::new(&budget);
            macro_rules! copy_required{($($field:ident:$ty:ty,)*)=>{$(for row in model.bindings.$field.iter(){if !((omit_class&&<$ty>::NAME==ProviderSymbol::NAME&&derivation::RowRef::of(row.id())==derivation::RowRef::of(class.id()))||(!omit_class&&<$ty>::NAME==CallTarget::NAME&&derivation::RowRef::of(row.id())==derivation::RowRef::of(construction.initialization()))){missing.bindings.$field.insert(row.clone()).unwrap();}})*};}
            lctx_model::normalized_binding_inputs!(copy_required);
            macro_rules! copy_pins{($($field:ident:$ty:ty,)*)=>{$(for row in model.$field.iter(){missing.$field.insert(row.clone()).unwrap();})*};}
            lctx_model::model_pin_inputs!(copy_pins);
            assert!(
                CheckedContextConstruction::derive(
                    &protocol,
                    &missing,
                    site.id(),
                    owner,
                    input,
                    context,
                    &budget
                )
                .unwrap()
                .is_err(),
                "required class or initializer evidence omitted"
            );
        }
        let changed = models::Catalog::parse(
            "changed-allocator.toml",
            &models::Catalog::committed_source().replace("object.__new__", "object.__init__"),
        )
        .unwrap();
        if let Ok(changed_protocol) =
            CheckedContextProtocol::derive(&changed, &model, class.id(), input, context, &budget)
                .unwrap()
        {
            assert!(
                CheckedContextConstruction::derive(
                    &changed_protocol,
                    &model,
                    site.id(),
                    owner,
                    input,
                    context,
                    &budget
                )
                .unwrap()
                .is_err()
            );
        }
    }
}
