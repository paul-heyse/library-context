//! Native normalized facts -> persisted petgraph -> analysis, with independent topology answers.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    normalized::{
        Rows, binding_normalization::BindingData, callable_normalization, entities::*,
        entity_normalization, event_normalization, relation_normalization,
    },
    projection::{normalization::*, snapshot::*, *},
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);
async fn fixture() -> (ProjectionData, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("normalized_projections"), Facts(tables.clone()))
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
    let mut projection = ProjectionData::new(&budget);
    macro_rules! project { ($($field:ident: $ty:ty,)*) => { $(projection.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(project);
    (projection, budget)
}
fn rebuild_events(data: &mut BindingData, budget: &ResourceBudget) {
    let mut receivers=lctx_model::domain::normalized::receiver::ReceiverData::new(budget);
    macro_rules! receiver_inputs {($($field:ident: $ty:ty,)*)=>{$(receivers.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(receiver_inputs);
    let receivers=lctx_model::domain::normalized::receiver::normalize(&receivers,budget).unwrap();
    data.receiver_assessments=Rows::new(budget);data.receiver_evidence=Rows::new(budget);data.receiver_premises=Rows::new(budget);
    macro_rules! receiver_outputs {($($field:ident: $ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&receivers.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_receiver_outputs!(receiver_outputs);

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

#[tokio::test]
async fn native_snapshot_retains_isolates_parallel_calls_cycles_and_unresolved_evidence() {
    let (data, budget) = fixture().await;
    let output = normalize(&data, &budget).unwrap();
    validate(&data, &output, &budget).unwrap();
    assert_eq!(output.assessments.len(), 4);
    assert_eq!(output.snapshots.len(), 4);
    let assessment = output
        .assessments
        .iter()
        .find(|a| a.projection == ProjectionName::CallableInvocation)
        .unwrap();
    let header = output
        .snapshots
        .iter()
        .find(|s| s.assessment == assessment.id())
        .unwrap();
    let graph = hydrate(header, assessment, &output.chunks, &budget).unwrap();
    let name = |entity: &EntityRef| -> Option<String> {
        let EntityRef::Callable { callable } = entity else {
            return None;
        };
        let CallableEntity::Source { declaration, .. } = data.callables.get(*callable).unwrap()
        else {
            return None;
        };
        let occurrence = data.occurrences.get(*declaration).unwrap();
        let artifact = data.artifacts.get(occurrence.source).unwrap();
        let sources = files("normalized_projections");
        let bytes = &sources[&artifact.path];
        let source =
            std::str::from_utf8(&bytes[occurrence.start as usize..occurrence.end as usize])
                .unwrap();
        source
            .strip_prefix("def ")
            .map(|s| s.split('(').next().unwrap().to_owned())
    };
    let names: std::collections::BTreeMap<_, _> = graph
        .entities()
        .filter_map(|e| name(e).map(|n| (e.id(), n)))
        .collect();
    let edges: Vec<_> = graph
        .arcs()
        .filter_map(|arc| {
            Some((
                names.get(&arc.source)?.as_str(),
                names.get(&arc.target)?.as_str(),
            ))
        })
        .collect();
    assert_eq!(
        edges.iter().filter(|e| **e == ("a", "b")).count(),
        2,
        "{edges:?}"
    );
    for pair in [
        ("a", "c"),
        ("b", "d"),
        ("c", "d"),
        ("d", "a"),
        ("recursive", "recursive"),
        ("a", "outside"),
    ] {
        assert!(edges.contains(&pair), "{pair:?}: {edges:?}");
    }
    let entity = |name: &str| *names.iter().find(|(_, n)| n.as_str() == name).unwrap().0;
    assert!(graph.outgoing(entity("isolated")).unwrap().is_empty());
    let selected = graph
        .reachable(
            entity("a"),
            Direction::Outgoing,
            None,
            |e| e.id() == entity("d"),
            &budget,
        )
        .unwrap();
    assert_eq!(selected.as_slice(), &[entity("d")]);
    assert!(output.gaps.iter().any(|g| g.assessment==assessment.id() && g.reason==ProjectionGapReason::Unresolved));
    assert!(
        output
            .coverage
            .iter()
            .any(|c| c.assessment == assessment.id())
    );
    for snapshot in output.snapshots.iter() {
        let source = output.assessments.get(snapshot.assessment).unwrap();
        let graph = hydrate(snapshot, source, &output.chunks, &budget).unwrap();
        assert!(graph.entities().next().is_some());
        assert!(graph.arcs().all(|a| {
            ProjectionSpec::builtin(source.projection)
                .roles()
                .contains(&a.id.role())
        }));
    }
}
#[tokio::test]
async fn missing_chunks_wrong_sources_and_omitted_gaps_fail_shared_publication_validation() {
    let (data, budget) = fixture().await;
    for omitted in [
        ProjectionSnapshotChunk::NAME,
        ProjectionSnapshot::NAME,
        ProjectionGap::NAME,
        ProjectionSourceCoverage::NAME,
    ] {
        let mut output = normalize(&data, &budget).unwrap();
        match omitted {
            ProjectionSnapshotChunk::NAME => output.chunks = Rows::new(&budget),
            ProjectionSnapshot::NAME => output.snapshots = Rows::new(&budget),
            ProjectionGap::NAME => output.gaps = Rows::new(&budget),
            _ => output.coverage = Rows::new(&budget),
        }
        assert!(validate(&data, &output, &budget).is_err(), "{omitted}");
    }
}

#[tokio::test]
async fn known_invocation_edges_retain_whole_event_open_remainder() {
    use lctx_model::domain::normalized::events::{EventAssessment, EventReason};
    let (mut data, budget) = fixture().await;
    let first = normalize(&data, &budget).unwrap();
    let event = data
        .alternatives
        .iter()
        .find(|a| {
            a.entity.is_some()
                && data.admissions.iter().any(|m| {
                    m.alternative == a.id()
                        && data.policies.get(m.assessment).unwrap().policy
                            == lctx_model::domain::normalized::events::CallPolicy::Invocation
                })
                && data.event_assessments.iter().any(|e| {
                    e.event == a.event
                        && e.complete
                        && e.exact
                        && e.known_receivers
                        && !e.unresolved
                        && !e.dispatch
                        && !e.disagreement
                })
        })
        .unwrap()
        .event;
    let original = data
        .event_assessments
        .iter()
        .find(|a| a.event == event)
        .unwrap()
        .id();
    let mut assessments = Rows::<EventAssessment>::new(&budget);
    for mut row in data.event_assessments.iter().cloned() {
        if row.event == event {
            row.complete = false;
            row.reason = EventReason::OpenResolution;
        }
        assessments.insert(row).unwrap();
    }
    data.event_assessments = assessments;
    let open = normalize(&data, &budget).unwrap();
    let assessment = open
        .assessments
        .iter()
        .find(|a| a.projection == ProjectionName::CallableInvocation)
        .unwrap();
    assert_eq!(assessment.availability, ProjectionAvailability::Partial);
    assert_eq!(
        assessment.arcs,
        first.assessments.get(assessment.id()).unwrap().arcs
    );
    let subject = ProjectionGapSubject::EventAssessment {
        assessment: original,
    }
    .id();
    assert!(
        open.gaps
            .iter()
            .any(|g| g.subject == subject && g.reason == ProjectionGapReason::EventUncertainty)
    );
    assert!(
        !first
            .gaps
            .iter()
            .any(|g| g.subject == subject && g.reason == ProjectionGapReason::EventUncertainty)
    );
    let mut gaps = Rows::new(&budget);
    for row in open.gaps.iter().filter(|g| g.subject != subject) {
        gaps.insert(row.clone()).unwrap();
    }
    let mut missing = open;
    missing.gaps = gaps;
    assert!(validate(&data, &missing, &budget).is_err());
    let mut admissions = Rows::new(&budget);
    for row in data.admissions.iter().filter(|m| {
        !(data.alternatives.get(m.alternative).unwrap().event == event
            && data.policies.get(m.assessment).unwrap().policy
                == lctx_model::domain::normalized::events::CallPolicy::Invocation)
    }) {
        admissions.insert(row.clone()).unwrap();
    }
    data.admissions = admissions;
    let excluded = normalize(&data, &budget).unwrap();
    assert!(
        !excluded
            .gaps
            .iter()
            .any(|g| g.subject == subject && g.reason == ProjectionGapReason::EventUncertainty),
        "wholly outside-policy evidence does not taint invocation availability"
    );
}

#[tokio::test]
async fn captured_overrider_adds_a_stored_scc_edge_and_keeps_dispatch_open() {
    use lctx_model::domain::normalized::events::CallAlternativeSource;
    let (data,budget)=fixture().await;
    let member=data.dispatch_members.iter().find(|m|data.symbols.get(m.defining_class).unwrap().name=="DispatchLeft").unwrap();
    let alternative=data.alternatives.iter().find(|a|matches!(data.alternative_sources.get(a.source),Some(CallAlternativeSource::DerivedDispatch{member:observed,..}) if *observed==member.id())).unwrap();
    let event=data.events.get(alternative.event).unwrap();let owner=data.owners.get(event.owner).unwrap().entity;
    assert_ne!(owner,member.entity);
    let output=normalize(&data,&budget).unwrap();validate(&data,&output,&budget).unwrap();
    let assessment=output.assessments.iter().find(|a|a.projection==ProjectionName::CallableInvocation).unwrap();
    assert_eq!(assessment.version,2);assert_eq!(assessment.availability,ProjectionAvailability::Partial);
    let header=output.snapshots.iter().find(|s|s.assessment==assessment.id()).unwrap();let graph=hydrate(header,assessment,&output.chunks,&budget).unwrap();
    assert!(graph.arcs().any(|a|a.id==ArcId::Invocation(alternative.id()) && a.source==owner && a.target==member.entity));
    let schedule=lctx_analytics::native_schedule::invocation_sccs(&graph,&budget).unwrap();let component=schedule.components().iter().find(|ids|ids.contains(&owner)).unwrap();
    assert!(component.contains(&member.entity),"override member must join the relay SCC: {component:?}");
    assert!(output.gaps.iter().any(|g|g.subject==ProjectionGapSubject::Alternative{alternative:alternative.id()}.id() && g.reason==ProjectionGapReason::OverrideDispatch));
    // Native named evidence alone has no reverse edge from the relay to DispatchLeft.
    let mut native=ProjectionData::new(&budget);
    macro_rules! copy {($($field:ident: $ty:ty,)*)=>{$(native.$field.decode(&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::projection_inputs!(copy);
    native.alternatives=Rows::new(&budget);for row in data.alternatives.iter().filter(|a|!matches!(data.alternative_sources.get(a.source),Some(CallAlternativeSource::DerivedDispatch{..}))){native.alternatives.insert(row.clone()).unwrap();}
    native.admissions=Rows::new(&budget);for row in data.admissions.iter().filter(|a|native.alternatives.get(a.alternative).is_some()){native.admissions.insert(row.clone()).unwrap();}
    let native_output=normalize(&native,&budget).unwrap();let source=native_output.assessments.iter().find(|a|a.projection==ProjectionName::CallableInvocation).unwrap();let header=native_output.snapshots.iter().find(|s|s.assessment==source.id()).unwrap();let graph=hydrate(header,source,&native_output.chunks,&budget).unwrap();
    let schedule=lctx_analytics::native_schedule::invocation_sccs(&graph,&budget).unwrap();let component=schedule.components().iter().find(|ids|ids.contains(&owner)).unwrap();
    assert!(!component.contains(&member.entity));
}
