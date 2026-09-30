//! Independent recovery of the dormant P3 derived-table expectations, using typed ownership.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    normalized::{
        Rows, binding_normalization::BindingData, callable_normalization, entities::*,
        entity_normalization, event_normalization, relation_normalization,
    },
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);
async fn fixture(case: &str) -> (BindingData, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files(case), Facts(tables.clone()))
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
    (data, budget)
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

fn path(data: &BindingData, occurrence: Id<source::Occurrence>) -> &str {
    data.artifacts
        .get(data.occurrences.get(occurrence).unwrap().source)
        .unwrap()
        .path
        .as_str()
}
fn declaration(data: &BindingData, entity: Id<EntityRef>) -> Option<Id<source::Occurrence>> {
    match data.refs.get(entity).unwrap() {
        EntityRef::Callable { callable } => match data.callables.get(*callable).unwrap() {
            CallableEntity::Source { declaration, .. } => Some(*declaration),
            _ => None,
        },
        EntityRef::Class { class } => match data.entity_classes.get(*class).unwrap() {
            ClassEntity::Source { declaration } => Some(*declaration),
            _ => None,
        },
        _ => None,
    }
}
#[tokio::test]
async fn recovered_source_stub_call_coordinates_and_public_exposure_seeds_remain_distinct() {
    let (data, _) = fixture("pysa_keys").await;
    let targets: Vec<_> = data
        .event_alternatives
        .iter()
        .filter_map(|a| {
            Some((
                path(&data, data.event_events.get(a.event)?.site),
                path(&data, declaration(&data, a.entity?)?),
            ))
        })
        .collect();
    assert!(
        targets.contains(&("keys/use_dual.py", "keys/dual.pyi")),
        "{targets:?}"
    );
    assert!(
        targets.contains(&("keys/dual.py", "keys/dual.py")),
        "{targets:?}"
    );
    let mut seeds = std::collections::BTreeSet::new();
    for exposure in data.entity_exposures.iter() {
        let public = data.public_names.get(exposure.observation).unwrap();
        let module = data.modules.get(exposure.access).unwrap();
        if public.name != "f" || module.qualified_name != "keys.dual" {
            continue;
        }
        for candidate in data
            .entity_exposure_candidates
            .iter()
            .filter(|c| c.exposure == exposure.id())
        {
            let resolution = data.symbol_resolutions.get(candidate.resolution).unwrap();
            if let Some(decl) = resolution.entity.and_then(|e| declaration(&data, e)) {
                seeds.insert((
                    data.artifacts.get(module.source).unwrap().path.as_str(),
                    path(&data, decl),
                ));
            }
        }
    }
    assert_eq!(
        seeds,
        std::collections::BTreeSet::from([
            ("keys/dual.py", "keys/dual.py"),
            ("keys/dual.pyi", "keys/dual.pyi")
        ])
    );
}
#[tokio::test]
async fn recovered_unicode_reexport_uses_the_implementation_and_keeps_signature_ordinals() {
    let (data, _) = fixture("unicode_bom").await;
    let exposed: Vec<_> = data
        .entity_exposures
        .iter()
        .filter(|e| {
            data.modules.get(e.access).unwrap().qualified_name == "lcfix"
                && data.public_names.get(e.observation).unwrap().name == "build"
        })
        .collect();
    assert_eq!(exposed.len(), 1);
    assert_eq!(exposed[0].status, ResolutionStatus::Resolved);
    for candidate in data
        .entity_exposure_candidates
        .iter()
        .filter(|c| c.exposure == exposed[0].id())
    {
        let resolution = data.symbol_resolutions.get(candidate.resolution).unwrap();
        let decl = declaration(&data, resolution.entity.unwrap()).unwrap();
        assert_eq!(path(&data, decl), "lcfix/core.py");
        assert!(
            data.declarations
                .iter()
                .any(|d| d.declaration == decl && !d.overload)
        );
    }
    let mut ordinals = Vec::new();
    for variant in data.callable_variants.iter() {
        let signature = data.signatures.get(variant.signature).unwrap();
        let symbol = data.symbols.get(signature.symbol).unwrap();
        if symbol.name == "build" {
            ordinals.push(signature.variant);
        }
    }
    ordinals.sort();
    assert_eq!(ordinals, vec![0, 1]);
    assert!(
        data.public_names.iter().any(|p| !p.name.is_ascii())
            || data.symbols.iter().any(|s| !s.name.is_ascii())
    );
}
#[tokio::test]
async fn recovered_dataclass_init_is_synthetic_and_dead_branch_declarations_stay_inspectable() {
    let (data, _) = fixture("derive_cases").await;
    let synthesized: Vec<_>=data.event_alternatives.iter().filter_map(|a|a.entity).filter(|e|matches!(data.refs.get(*e),Some(EntityRef::Callable {callable}) if matches!(data.callables.get(*callable),Some(CallableEntity::Synthetic {symbol}) if data.symbols.get(*symbol).unwrap().name=="__init__"))).collect();
    assert!(!synthesized.is_empty());
    let source = files("derive_cases");
    let bytes = &source["dc/compat.py"];
    let load_declarations: Vec<_> = data
        .declarations
        .iter()
        .filter(|d| {
            path(&data, d.declaration) == "dc/compat.py" && {
                let name = data.occurrences.get(d.name).unwrap();
                &bytes[name.start as usize..name.end as usize] == b"load"
            }
        })
        .collect();
    assert_eq!(
        load_declarations.len(),
        2,
        "both branch declarations remain source entities"
    );
    let public = data
        .entity_exposures
        .iter()
        .find(|e| {
            data.modules.get(e.access).unwrap().qualified_name == "dc"
                && data.public_names.get(e.observation).unwrap().name == "load"
        })
        .unwrap();
    let earliest = load_declarations
        .iter()
        .map(|d| data.occurrences.get(d.declaration).unwrap().start)
        .min()
        .unwrap();
    let starts: Vec<_> = data
        .entity_exposure_candidates
        .iter()
        .filter(|c| c.exposure == public.id())
        .filter_map(|c| data.symbol_resolutions.get(c.resolution).unwrap().entity)
        .filter_map(|e| declaration(&data, e))
        .map(|d| data.occurrences.get(d).unwrap().start)
        .collect();
    assert!(!starts.is_empty());
    assert!(starts.iter().all(|start| *start == earliest), "{starts:?}");
    // Complete raw signatures, including native unknown forms, are retained as variants. No
    // synthetic legacy 'dead signature' rows or inferred runtime absence is introduced.
    assert_eq!(data.callable_variants.len(), data.signatures.len());
}
