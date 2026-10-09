//! Native assignment/closure/loop twins through the shared entry and normalized guard operators.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    calls::*,
    conditions::{entry::*, rebase::*, stability::*, *},
    normalized::{
        Rows,
        binding_normalization::{self, BindingData, BindingOutput},
        callable_normalization, entity_normalization, event_normalization, receiver,
        relation_normalization,
    },
    resources::ResourceBudget,
    source::*,
    *,
};
use std::collections::BTreeMap;
inspector!(Facts => {
    let mut demand=typed_driver::ObservationDemand::selected();
    macro_rules! select { ($($field:ident:$ty:ty $(=> $family:ident)?,)*) => {$(demand.include::<$ty>();)*}; }
    lctx_model::entry_value_inputs!(select);
    lctx_model::normalized_binding_inputs!(select);
    lctx_model::normalized_entity_inputs!(select);
    lctx_model::normalized_relation_inputs!(select);
    demand.include::<lctx_model::domain::calls::CallTarget>();
    demand
});
async fn fixture() -> (EntryData, BindingData, BindingOutput, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(
        &typed_driver::files("entry_value_witnesses"),
        Facts(tables.clone()),
    )
    .await
    .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){relations.facts.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities =
        entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
    macro_rules! additional {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){relations.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &budget).unwrap();
    let mut bindings = BindingData::new(&budget);
    macro_rules! raw {($($field:ident:$ty:ty,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){bindings.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_binding_inputs!(raw);
    macro_rules! entities {($($field:ident:$ty:ty,)*)=>{$(bindings.visit(<$ty>::NAME,&<$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links {($($field:ident:$ty:ty,)*)=>{$(bindings.visit(<$ty>::NAME,&<$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_relation_outputs!(links);
    let mut callables = callable_normalization::CallableData::new(&budget);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(callables.visit(<$ty>::NAME,&<$ty as Record>::encode(&bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(input);
    let callables = callable_normalization::normalize(&callables, &budget).unwrap();
    macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(bindings.visit(<$ty>::NAME,&<$ty as Record>::encode(&callables.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_callable_outputs!(output);
    let mut receivers = receiver::ReceiverData::new(&budget);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(receivers.visit(<$ty>::NAME,&<$ty as Record>::encode(&bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(input);
    let receivers = receiver::normalize(&receivers, &budget).unwrap();
    macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(bindings.visit(<$ty>::NAME,&<$ty as Record>::encode(&receivers.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_receiver_outputs!(output);
    let mut events = event_normalization::EventData::new(&budget);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(events.visit(<$ty>::NAME,&<$ty as Record>::encode(&bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(input);
    let events = event_normalization::normalize(&events, &budget).unwrap();
    macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(bindings.visit(<$ty>::NAME,&<$ty as Record>::encode(&events.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_event_outputs!(output);
    let mut entry = EntryData::new(&budget);
    macro_rules! raw {($($field:ident:$ty:ty,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){entry.$field.insert(row).unwrap();})*};}
    lctx_model::entry_value_inputs!(raw);
    macro_rules! entities {($($field:ident:$ty:ty,)*)=>{$(entry.visit(<$ty>::NAME,&<$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    let output = binding_normalization::normalize(&bindings, &budget).unwrap();
    (entry, bindings, output, budget)
}
fn text(data: &EntryData, id: Id<Occurrence>) -> String {
    let row = data.occurrences.get(id).unwrap();
    let artifact = data.artifacts.get(row.source).unwrap();
    let bytes = typed_driver::files("entry_value_witnesses");
    String::from_utf8(bytes[&artifact.path][row.start as usize..row.end as usize].to_vec()).unwrap()
}
fn request(data: &EntryData, function: &str) -> EntryRequest {
    let owner=data.callables.iter().find(|c|matches!(c,lctx_model::domain::normalized::entities::CallableEntity::Source{declaration,..} if text(data,*declaration).lines().find(|line|line.trim_start().starts_with("def ")).is_some_and(|line|line.trim_start().starts_with(&format!("def {function}("))))).unwrap();
    let owner_ref = lctx_model::domain::normalized::entities::EntityRef::Callable {
        callable: owner.id(),
    };
    let read = data
        .uses
        .iter()
        .find(|u| {
            data.owners
                .iter()
                .any(|o| o.occurrence == u.occurrence && o.entity == owner_ref.id())
                && data.occurrences.iter().any(|e| {
                    e.syntax_kind == SyntaxKind::ExprCompare
                        && data.occurrences.get(u.occurrence).unwrap().source == e.source
                        && data
                            .occurrences
                            .get(u.occurrence)
                            .unwrap()
                            .structural_path
                            .len()
                            > e.structural_path.len()
                        && data
                            .occurrences
                            .get(u.occurrence)
                            .unwrap()
                            .structural_path
                            .starts_with(&e.structural_path)
                })
        })
        .unwrap();
    let lctx_model::domain::value::PlaceRoot::Formal { declaration } = data
        .roots
        .get(data.places.get(read.place).unwrap().root)
        .unwrap()
    else {
        panic!("formal read")
    };
    let formal = lctx_model::domain::normalized::entities::ParameterEntity::Source {
        declaration: *declaration,
    };
    let support = data
        .use_observations
        .iter()
        .filter(|o| o.use_ == read.id())
        .find_map(|o| data.use_supports.iter().find(|s| s.assertion == o.id()))
        .unwrap();
    let run = data.runs.get(support.run).unwrap();
    EntryRequest {
        owner: owner_ref.id(),
        formal: formal.id(),
        access: read.occurrence,
        context: run.context,
        run: run.id(),
    }
}
#[tokio::test]
async fn entry_identity_retains_parameter_and_receiver_but_refuses_rebinding_and_nested_reach() {
    let (data, _, _, budget) = fixture().await;
    for function in ["parameter", "method", "class_method"] {
        let req = request(&data, function);
        let refusal = EntryValueWitness::derive(&data, req, &budget)
            .unwrap()
            .err();
        assert!(refusal.is_none(), "{function}: {refusal:?}");
    }
    for function in ["assigned", "unbound", "loop", "nonlocal_write"] {
        let req = request(&data, function);
        assert!(
            EntryValueWitness::derive(&data, req, &budget)
                .unwrap()
                .is_err(),
            "{function}"
        );
    }
    // A captured outer formal is never the nested callable's entry port.
    let nested=data.callables.iter().find(|c|matches!(c,lctx_model::domain::normalized::entities::CallableEntity::Source{declaration,..} if text(&data,*declaration).starts_with("def nested("))).unwrap();
    let owner = lctx_model::domain::normalized::entities::EntityRef::Callable {
        callable: nested.id(),
    };
    let read = data
        .uses
        .iter()
        .find(|u| {
            data.owners
                .iter()
                .any(|o| o.occurrence == u.occurrence && o.entity == owner.id())
        })
        .unwrap();
    let outer=data.callables.iter().find(|c|matches!(c,lctx_model::domain::normalized::entities::CallableEntity::Source{declaration,..} if text(&data,*declaration).starts_with("def captured("))).unwrap();
    let formal=data.formals.iter().find(|f|matches!(f,lctx_model::domain::normalized::entities::ParameterEntity::Source{declaration} if {let normalized::entities::CallableEntity::Source{declaration:owner,..}=outer else{unreachable!()};let formal=data.occurrences.get(*declaration).unwrap();let owner=data.occurrences.get(*owner).unwrap();formal.source==owner.source && formal.structural_path.starts_with(&owner.structural_path)})).unwrap();
    let run = data
        .runs
        .iter()
        .find(|r| data.use_supports.iter().any(|s| s.run == r.id()))
        .unwrap();
    let req = EntryRequest {
        owner: owner.id(),
        formal: formal.id(),
        access: read.occurrence,
        context: run.context,
        run: run.id(),
    };
    assert!(
        EntryValueWitness::derive(&data, req, &budget)
            .unwrap()
            .is_err()
    );
}
fn guard(data: &EntryData, entry: &DerivedEntryValue) -> Id<EvaluationAtom> {
    let read = data.occurrences.get(entry.witness().access).unwrap();
    data.atoms
        .iter()
        .find(|a| {
            data.predicates.get(a.predicate) == Some(&lctx_model::domain::value::Predicate::IsNone)
                && data.occurrences.get(a.evaluation).is_some_and(|e| {
                    read.source == e.source
                        && read.structural_path.len() > e.structural_path.len()
                        && read.structural_path.starts_with(&e.structural_path)
                })
        })
        .unwrap()
        .id()
}
fn replay_substitution(
    entry: &EntryData,
    binding: (&BindingData, &BindingOutput),
    derived: &DerivedEntryValue,
    stability: &CheckedStability,
    rebased: &RebasedGuards,
    mutation: u8,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let (data, output) = binding;
    let invariant = guard_substitution_invariants().remove(0);
    let mut check = (invariant.create)(budget);
    macro_rules! visit_entry {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&entry.$field.iter().cloned().collect::<Vec<_>>())?)?;)*};}
    lctx_model::entry_value_inputs!(visit_entry);
    macro_rules! visit_binding {($($field:ident:$ty:ty,)*)=>{$(let input=ValidationInput::of::<$ty>(&["id"]);let input=if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>())?)?;)*};}
    lctx_model::normalized_binding_inputs!(visit_binding);
    macro_rules! visit_output {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>())?)?;)*};}
    lctx_model::normalized_binding_outputs!(visit_output);
    check.visit(
        EntryAccessSource::NAME,
        &<EntryAccessSource as Record>::encode(&[derived.source().clone()])?,
    )?;
    check.visit(
        EntryValueWitness::NAME,
        &EntryValueWitness::encode(&[derived.witness().clone()])?,
    )?;
    check.visit(
        StabilityWitness::NAME,
        &StabilityWitness::encode(&[stability.witness().clone()])?,
    )?;
    check.visit(
        EvaluationAtom::NAME,
        &EvaluationAtom::encode(&rebased.atoms)?,
    )?;
    check.visit(
        value::Predicate::NAME,
        &<value::Predicate as Record>::encode(&rebased.predicates)?,
    )?;
    check.visit(
        value::PlaceRoot::NAME,
        &<value::PlaceRoot as Record>::encode(&rebased.roots)?,
    )?;
    check.visit(value::Place::NAME, &value::Place::encode(&rebased.places)?)?;
    check.visit(
        assertion::AssertionQualification::NAME,
        &assertion::AssertionQualification::encode(&rebased.qualifications)?,
    )?;
    if mutation == 4 {
        let input = ValidationInput::of::<value::Place>(&["id"])
            .at_epoch(stages::PublicationBoundary::Facts);
        check.visit_input(&input, &value::Place::encode(&rebased.places)?)?;
    }
    let mut substitutions = rebased.substitutions.clone();
    if mutation == 1 {
        substitutions[0].binding = output
            .bindings
            .iter()
            .find(|b| b.id() != substitutions[0].binding)
            .unwrap()
            .id();
    }
    if mutation == 2 {
        substitutions[0].source_atom = entry
            .atoms
            .iter()
            .find(|a| a.id() != substitutions[0].source_atom)
            .unwrap()
            .id();
    }
    check.visit(
        GuardSubstitution::NAME,
        &GuardSubstitution::encode(&substitutions)?,
    )?;
    let influences = if mutation == 3 {
        vec![]
    } else {
        rebased
            .influences
            .iter()
            .map(|i| transfer::summary::ControlInfluence {
                qualification: i.qualification,
                input: i.input,
                atom: i.atom,
                evaluation: i.evaluation,
            })
            .collect::<Vec<_>>()
    };
    check.visit(
        transfer::summary::ControlInfluence::NAME,
        &transfer::summary::ControlInfluence::encode(&influences)?,
    )?;
    check.finish()
}
#[tokio::test]
async fn normalized_actual_and_receiver_guards_preserve_source_operations_and_qualified_influence()
{
    guard_controls(false).await;
}
#[tokio::test]
async fn stored_summary_substitution_replays_the_original_binding_prefix() {
    guard_controls(true).await;
}
async fn guard_controls(stored: bool) {
    use lctx_model::domain::{normalized::bindings::CallBinding, value::*};
    let (entry_data, data, output, budget) = fixture().await;
    let verified = binding_normalization::verify(&data, &output, &budget).unwrap();
    for (function, call, class_of) in [
        ("parameter", "parameter(None)", false),
        ("method", "obj.method()", false),
        ("class_method", "obj.class_method()", true),
    ] {
        let entry = EntryValueWitness::derive(&entry_data, request(&entry_data, function), &budget)
            .unwrap()
            .unwrap();
        let atom = guard(&entry_data, &entry);
        let leaf = entry_data
            .leaves
            .iter()
            .find(|leaf| leaf.atom == atom)
            .unwrap();
        let support = entry_data
            .leaf_supports
            .iter()
            .find(|support| support.assertion == leaf.id())
            .unwrap();
        let source = EntryAccessSource::guard(
            &entry_data,
            entry.witness().request(),
            leaf.id(),
            support.id(),
        )
        .unwrap();
        let entry =
            EntryValueWitness::derive_for(&entry_data, entry.witness().request(), &source, &budget)
                .unwrap()
                .unwrap();
        let proof = StabilityWitness::derive(&entry_data, atom, &entry).unwrap();
        let row = output
            .bindings
            .iter()
            .find(|b| {
                data.callable_slots.get(b.slot).unwrap().parameter == entry.parameter()
                    && text(
                        &entry_data,
                        data.event_events
                            .get(output.attempts.get(b.attempt).unwrap().event)
                            .unwrap()
                            .site,
                    ) == call
            })
            .unwrap_or_else(|| {
                let attempts = output.attempts.iter().filter(|a| {
                    data.event_events.get(a.event).is_some_and(|e| {
                        text(&entry_data, e.site) == call
                    })
                }).take(16).collect::<Vec<_>>();
                for attempt in &attempts {
                    eprintln!("guard binding {function}/{call}: {attempt:?}");
                    let bindings = output.bindings.iter().filter(|b| b.attempt == attempt.id()).take(16).collect::<Vec<_>>();
                    eprintln!("guard binding rows: {bindings:?}");
                    for slot in data.callable_slots.iter().filter(|s| {
                        attempt.variant == Some(s.variant)
                    }).take(16) {
                        let parameter = data.parameters.get(slot.parameter);
                        let shape = parameter.and_then(|p| data.shapes.get(p.shape));
                        eprintln!("guard slot={slot:?}; parameter={parameter:?}; shape={shape:?}");
                    }
                }
                panic!("no exact guard binding for {function}/{call}; entry parameter {:?}; attempts={}", entry.parameter(), attempts.len())
            });
        let attempt = output.attempts.get(row.attempt).unwrap();
        let event = data.event_events.get(attempt.event).unwrap();
        let checked = verified.bound(row.attempt).unwrap();
        let slot = data.callable_slots.get(row.slot).unwrap();
        let source = output.sources.get(row.source).unwrap();
        let projection = output.projections.get(row.projection).unwrap();
        let binding =
            CheckedGuardBinding::derive(checked, row, slot, source, projection, attempt, event)
                .unwrap();
        let witnesses = BTreeMap::from([(atom, proof.clone())]);
        let roots = BTreeMap::from([(
            entry_data
                .places
                .get(entry_data.atoms.get(atom).unwrap().operand.unwrap())
                .unwrap()
                .root,
            RootBinding::Actual(binding),
        )]);
        let atoms = entry_data
            .atoms
            .iter()
            .map(|r| (r.id(), r.clone()))
            .collect();
        let predicates = entry_data
            .predicates
            .iter()
            .map(|r| (r.id(), r.clone()))
            .collect();
        let places = entry_data
            .places
            .iter()
            .map(|r| (r.id(), r.clone()))
            .collect();
        let root_catalog = entry_data
            .roots
            .iter()
            .map(|r| (r.id(), r.clone()))
            .collect();
        let catalog = GuardCatalog {
            atoms: &atoms,
            predicates: &predicates,
            places: &places,
            roots: &root_catalog,
        };
        let site = entry_data.occurrences.get(event.site).unwrap();
        let call = data.syntax.get(attempt.syntax.unwrap()).unwrap();
        let caller = data.qualifications.get(call.qualification).unwrap();
        let before = budget.reserved();
        let mut rebased = substitute_call_guards(
            &Diagram::from_atom(atom),
            site,
            event.context,
            &catalog,
            &roots,
            Some(&witnesses),
            Some(caller),
            &budget,
        )
        .unwrap();
        assert!(budget.reserved() > before);
        assert_eq!(rebased.substitutions.len(), 1);
        assert_eq!(rebased.influences.len(), 1);
        assert_eq!(rebased.substitutions[0].binding, row.id());
        assert_eq!(rebased.substitutions[0].event, event.id());
        assert_eq!(rebased.substitutions[0].qualification, caller.id());
        assert_eq!(rebased.influences[0].input, rebased.places[0].id());
        assert_eq!(rebased.influences[0].qualification, caller.id());
        assert_eq!(
            matches!(rebased.roots[0], PlaceRoot::ClassOf { .. }),
            class_of
        );
        if class_of {
            assert!(matches!(source, BindingSource::ClassOf { .. }));
            assert!(!matches!(rebased.roots[0], PlaceRoot::Occurrence { .. }));
        }
        let mut wrong_event = event.clone();
        wrong_event.context = attribution::AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "foreign".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"foreign"),
            environment_digest: ContentHash::of(b"foreign"),
            lock_digest: None,
        }
        .id();
        let mut wrong_attempt = attempt.clone();
        wrong_attempt.event = wrong_event.id();
        assert!(
            CheckedGuardBinding::derive(
                checked,
                row,
                slot,
                source,
                projection,
                &wrong_attempt,
                &wrong_event
            )
            .is_err(),
            "valid shape cannot cross contexts/events"
        );
        let forged = CallBinding {
            kind: if row.kind == BindingKind::Implicit {
                BindingKind::Positional
            } else {
                BindingKind::Implicit
            },
            ..row.clone()
        };
        assert!(
            CheckedGuardBinding::derive(checked, &forged, slot, source, projection, attempt, event)
                .is_err()
        );
        if stored {
            replay_substitution(
                &entry_data,
                (&data, &output),
                &entry,
                &proof,
                &rebased,
                0,
                &budget,
            )
            .unwrap();
            for mutation in 1..=4 {
                assert!(
                    replay_substitution(
                        &entry_data,
                        (&data, &output),
                        &entry,
                        &proof,
                        &rebased,
                        mutation,
                        &budget
                    )
                    .is_err(),
                    "stored replay mutation {mutation}"
                );
            }
        }
        let admission = rebased.take_admission();
        drop(rebased);
        assert!(
            budget.reserved() > before,
            "publication retains restatement allowance"
        );
        drop(admission);
        assert_eq!(budget.reserved(), before, "restatement allowance lifetime");
        let tiny = ResourceBudget::fixed(64).unwrap();
        assert_eq!(
            substitute_call_guards(
                &Diagram::from_atom(atom),
                site,
                event.context,
                &catalog,
                &roots,
                Some(&witnesses),
                Some(caller),
                &tiny
            )
            .unwrap_err(),
            attribution::ObligationKind::ResourceRefused
        );
        assert_eq!(tiny.reserved(), 0);
    }
}
#[tokio::test]
async fn native_parameter_bridge_requires_the_exact_first_identifier_and_native_support() {
    for mutation in 0..3 {
        let (mut data, _, _, budget) = fixture().await;
        let req = request(&data, "parameter");
        let stored = EntryValueWitness::derive(&data, req, &budget)
            .unwrap()
            .unwrap()
            .witness()
            .clone();
        let support_id = stored
            .parameter_placement_support
            .expect("native identifier bridge support");
        let placement_id = stored
            .parameter_placement
            .expect("native identifier bridge placement");
        let original = data.placements.get(placement_id).unwrap().clone();
        let original_support = data.placement_supports.get(support_id).unwrap().clone();
        let supports = data
            .placement_supports
            .iter()
            .filter(|s| s.id() != support_id)
            .cloned()
            .collect::<Vec<_>>();
        data.placement_supports = Rows::new(&budget);
        for s in supports {
            data.placement_supports.insert(s).unwrap();
        }
        if mutation < 2 {
            let mut placement = original.clone();
            let support = original_support;
            if mutation == 0 {
                placement.ordinal = 1;
            } else {
                let wrong = Occurrence {
                    role: OccurrenceRole::Read,
                    ..data.occurrences.get(original.occurrence).unwrap().clone()
                };
                data.occurrences.insert(wrong.clone()).unwrap();
                placement.occurrence = wrong.id();
                let definition = data
                    .definitions
                    .iter()
                    .find(|d| d.occurrence == original.occurrence)
                    .unwrap()
                    .clone();
                let changed = flow::FlowDefinition {
                    occurrence: wrong.id(),
                    ..definition.clone()
                };
                data.definitions.insert(changed.clone()).unwrap();
                let observed = data
                    .definition_observations
                    .get(stored.definition)
                    .unwrap()
                    .clone();
                let changed_observed = flow::FlowDefinitionObservation {
                    definition: changed.id(),
                    ..observed
                };
                let mut definition_support = data
                    .definition_supports
                    .get(stored.definition_support)
                    .unwrap()
                    .clone();
                definition_support.assertion = changed_observed.id();
                data.definition_observations
                    .insert(changed_observed)
                    .unwrap();
                data.definition_supports.insert(definition_support).unwrap();
                let target = flow::ReachingDefinition::Bound {
                    definition: changed.id(),
                };
                data.targets.insert(target.clone()).unwrap();
                let reaching = flow::FlowReachingObservation {
                    target: target.id(),
                    ..data.reaching.get(stored.reaching).unwrap().clone()
                };
                let mut reaching_support = data
                    .reaching_supports
                    .get(stored.reaching_support)
                    .unwrap()
                    .clone();
                reaching_support.assertion = reaching.id();
                let keep = data
                    .reaching
                    .iter()
                    .filter(|r| r.id() != stored.reaching)
                    .cloned()
                    .collect::<Vec<_>>();
                data.reaching = Rows::new(&budget);
                for r in keep {
                    data.reaching.insert(r).unwrap();
                }
                data.reaching.insert(reaching).unwrap();
                data.reaching_supports.insert(reaching_support).unwrap();
            }
            let mut support = support;
            support.assertion = placement.id();
            data.placements.insert(placement).unwrap();
            data.placement_supports.insert(support).unwrap();
        }
        assert!(
            EntryValueWitness::derive(&data, req, &budget)
                .unwrap()
                .is_err(),
            "bridge mutation {mutation}"
        );
    }
}

#[tokio::test]
async fn partial_family_native_singleton_requires_its_complete_attributed_inventory() {
    use lctx_model::domain::{attribution::*, flow_inventory::*};
    let (mut data, _, _, budget) = fixture().await;
    let req = request(&data, "parameter");
    let coverage = data.coverage.iter().cloned().collect::<Vec<_>>();
    data.coverage = Rows::new(&budget);
    for mut row in coverage {
        if row.family == FactFamily::Flow && row.run.is_some() {
            row.status = CoverageStatus::Partial;
            row.reason = Some(obligation::ObligationKind::IncompleteCoverage);
        }
        data.coverage.insert(row).unwrap();
    }
    let proof = EntryValueWitness::derive(&data, req, &budget)
        .unwrap()
        .expect("complete native per-use singleton survives Partial family coverage");
    let witness = proof.witness().clone();
    let inventory = data.inventories.get(witness.inventory).unwrap().clone();
    let native_support = data
        .inventory_supports
        .get(witness.inventory_support)
        .unwrap()
        .clone();
    assert!(inventory.complete);
    assert_eq!(
        data.coverage.get(witness.coverage).unwrap().status,
        CoverageStatus::Partial
    );
    drop(proof);
    let originals = data.inventory_supports.iter().cloned().collect::<Vec<_>>();
    for change in 0..3 {
        data.inventory_supports = Rows::new(&budget);
        for mut row in originals.clone() {
            if row.id() == native_support.id() {
                match change {
                    0 => {
                        row.run = data
                            .runs
                            .iter()
                            .find(|r| r.id() != req.run)
                            .expect("independent analyzer run")
                            .id()
                    }
                    1 => row.origin = Origin::SourceObservation,
                    _ => row.mode = ExtractionMode::ReportDecode,
                }
            }
            data.inventory_supports.insert(row).unwrap();
        }
        assert!(
            EntryValueWitness::derive(&data, req, &budget)
                .unwrap()
                .is_err(),
            "foreign run or non-native origin/mode cannot certify native closure"
        );
    }
    data.inventory_supports = Rows::new(&budget);
    for row in originals {
        data.inventory_supports.insert(row).unwrap();
    }
    let old_inventories = data.inventories.iter().cloned().collect::<Vec<_>>();
    data.inventories = Rows::new(&budget);
    assert!(
        EntryValueWitness::derive(&data, req, &budget)
            .unwrap()
            .is_err(),
        "missing inventory must refuse"
    );
    for row in old_inventories {
        data.inventories.insert(row).unwrap();
    }
    let mut candidates = data
        .inventory_candidates
        .iter()
        .filter(|c| c.inventory == inventory.id())
        .map(FlowUseCandidate::state)
        .collect::<Vec<_>>();
    candidates[0].reachability_lost = true;
    let members = data
        .inventory_members
        .iter()
        .filter(|m| m.inventory == inventory.id())
        .map(|m| (m.ordinal, m.reaching, m.support))
        .collect::<Vec<_>>();
    let (incomplete, candidates, members) = FlowUseInventoryObservation::new(
        inventory.qualification,
        inventory.use_,
        inventory.scope,
        inventory.view,
        &candidates,
        &members,
    )
    .unwrap();
    assert!(!incomplete.complete);
    data.inventories = Rows::new(&budget);
    data.inventories.insert(incomplete.clone()).unwrap();
    data.inventory_candidates = Rows::new(&budget);
    for row in candidates {
        data.inventory_candidates.insert(row).unwrap();
    }
    data.inventory_members = Rows::new(&budget);
    for row in members {
        data.inventory_members.insert(row).unwrap();
    }
    data.inventory_supports = Rows::new(&budget);
    let mut support = native_support;
    support.assertion = incomplete.id();
    data.inventory_supports.insert(support).unwrap();
    assert!(
        EntryValueWitness::derive(&data, req, &budget)
            .unwrap()
            .is_err(),
        "Partial family plus incomplete per-use inventory cannot certify entry"
    );
}
