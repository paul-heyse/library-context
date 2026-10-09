//! Native event normalization preserves Pysa alternatives and qualified ty call paths.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    attribution::Modality,
    calls::*,
    normalized::{entity_normalization, event_normalization::*, events::*},
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts => {
    let mut demand=typed_driver::ObservationDemand::selected();
    macro_rules! select { ($($field:ident:$ty:ty $(=> $family:ident)?,)*) => {$(demand.include::<$ty>();)*}; }
    lctx_model::normalized_entity_inputs!(select);
    lctx_model::normalized_event_inputs!(select);
    demand.include::<lctx_model::domain::calls::CallTarget>();
    demand
});
async fn fixture() -> (EventData, EventOutput) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("pysa_variants"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut raw = entity_normalization::EntityData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { raw.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    let entities = entity_normalization::normalize(raw.inputs(), &budget).unwrap();
    let mut data = EventData::new(&budget);
    macro_rules! inputs { ($($field:ident: $ty:ty,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_event_inputs!(inputs);
    macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_entity_outputs!(entities);
    let output = normalize(&data, &budget).unwrap();
    (data, output)
}
fn policies(
    output: &EventOutput,
    alternative: Id<NormalizedCallAlternative>,
) -> std::collections::BTreeSet<CallPolicy> {
    output
        .admissions
        .iter()
        .filter(|a| a.alternative == alternative)
        .map(|a| output.policy_assessments.get(a.assessment).unwrap().policy)
        .collect()
}
#[tokio::test]
async fn native_higher_order_dispatch_and_definition_evidence_keep_their_policy_meaning() {
    let (data, output) = fixture().await;
    assert_eq!(output.policy_assessments.len(), 5 * output.events.len());
    assert_eq!(output.assessments.len(), output.events.len());
    let mut higher = 0;
    let mut dispatch = 0;
    let mut potential = 0;
    for alternative in output.alternatives.iter() {
        let target = data
            .targets
            .get(
                output
                    .alternative_sources
                    .get(alternative.source)
                    .unwrap()
                    .target(),
            )
            .unwrap();
        let admitted = policies(&output, alternative.id());
        assert!(admitted.contains(&CallPolicy::Association));
        if matches!(
            data.channels.get(target.channel),
            Some(CallChannel::HigherOrder { .. })
        ) {
            higher += 1;
            assert!(!admitted.contains(&CallPolicy::Invocation));
            assert!(!admitted.contains(&CallPolicy::Dataflow));
            assert!(!admitted.contains(&CallPolicy::Summary));
        }
        if matches!(
            data.destinations.get(target.destination),
            Some(CallDestination::Overrides { .. })
        ) {
            dispatch += 1;
            assert!(!admitted.contains(&CallPolicy::Dataflow));
            assert!(!admitted.contains(&CallPolicy::Summary));
        }
        if data
            .qualifications
            .get(target.qualification)
            .unwrap()
            .modality
            == Modality::Potential
        {
            potential += 1;
            assert_eq!(
                admitted,
                std::collections::BTreeSet::from([CallPolicy::Association])
            );
        }
        if target.phase == CallPhase::Definition {
            assert!(!admitted.contains(&CallPolicy::Invocation));
        }
    }
    assert!(higher > 0 && dispatch > 0 && potential > 0);
    assert!(output.alternatives.iter().any(|a| {
        data.targets
            .get(output.alternative_sources.get(a.source).unwrap().target())
            .unwrap()
            .phase
            == CallPhase::New
    }));
    assert!(output.alternatives.iter().any(|a| {
        data.targets
            .get(output.alternative_sources.get(a.source).unwrap().target())
            .unwrap()
            .phase
            == CallPhase::Init
    }));
    assert_eq!(output.source_evidence.len(), data.site_supports.len());
    for source in output.sources.iter() {
        let raw = data.call_sites.get(source.observation).unwrap();
        let event = output.events.get(source.event).unwrap();
        assert_eq!(
            (event.site, event.origin, event.context),
            (
                raw.site,
                raw.origin,
                data.qualifications.get(raw.qualification).unwrap().context
            )
        );
    }
}
#[tokio::test]
async fn every_qualified_native_call_path_step_has_a_total_link_and_premises_are_validated() {
    let (data, output) = fixture().await;
    let expected = data
        .paths
        .iter()
        .map(|p| data.steps.iter().filter(|s| s.path == p.path).count())
        .sum::<usize>();
    assert!(expected > 0);
    assert_eq!(output.flow_links.len(), expected);
    for link in output.flow_links.iter().filter(|l| l.event.is_some()) {
        let event = output.events.get(link.event.unwrap()).unwrap();
        let step = data.steps.get(link.step).unwrap();
        let path = data.paths.get(link.observation).unwrap();
        assert_eq!(event.site, step.call);
        assert_eq!(event.origin, CallOrigin::explicit());
        assert_eq!(
            event.context,
            data.qualifications.get(path.qualification).unwrap().context
        );
    }
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    for omitted in [
        None,
        Some(CallAlternativeEvidence::NAME),
        Some(FlowCallEventLink::NAME),
        Some(CallPolicyAssessment::NAME),
    ] {
        let mut check = (invariants().remove(0).create)(&budget);
        macro_rules! inputs { ($($field:ident: $ty:ty,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_event_inputs!(inputs);
        macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(if omitted != Some(<$ty>::NAME) { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_event_outputs!(outputs);
        assert_eq!(check.finish().is_ok(), omitted.is_none(), "{omitted:?}");
    }
}
