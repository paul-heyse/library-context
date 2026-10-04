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
async fn fixture(
    case: &str,
) -> (
    BindingData,
    ResourceBudget,
    entity_normalization::EntityData,
) {
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
    (data, budget, relations.facts)
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
    let (data, _, _) = fixture("pysa_keys").await;
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
    let (data, budget, mut facts) = fixture("unicode_bom").await;
    let exposed: Vec<_> = data
        .entity_exposures
        .iter()
        .filter(|e| {
            data.modules.get(e.access).unwrap().qualified_name == "lcfix"
                && data.public_names.get(e.observation).unwrap().name == "build"
        })
        .collect();
    assert_eq!(exposed.len(), 1);
    assert_eq!(
        exposed[0].status,
        ResolutionStatus::Resolved,
        "unicode exposure={:?}; raw public={:?}; origin={:?}; same-name native symbols/resolutions={:?}; public supports={:?}",
        exposed[0],
        data.public_names.get(exposed[0].observation),
        data.export_origins.get(exposed[0].origin),
        data.symbols
            .iter()
            .filter(|s| s.name == "build")
            .map(|s| (
                s,
                data.provider_modules.get(s.module),
                data.symbol_resolutions
                    .iter()
                    .filter(|r| r.symbol == s.id())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        data.public_supports
            .iter()
            .filter(|s| s.assertion == exposed[0].observation)
            .collect::<Vec<_>>()
    );
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
    let mut ordinals = std::collections::BTreeMap::<calls::SignatureRole, Vec<i64>>::new();
    for variant in data.callable_variants.iter() {
        let signature = data.signatures.get(variant.signature).unwrap();
        let symbol = data.symbols.get(signature.symbol).unwrap();
        if symbol.name == "build" {
            ordinals
                .entry(signature.role)
                .or_default()
                .push(signature.variant);
        }
    }
    for role in [
        calls::SignatureRole::Source,
        calls::SignatureRole::EffectiveTyped,
    ] {
        let mut variants = ordinals
            .remove(&role)
            .expect("both source and effective native roles retained");
        variants.sort();
        assert_eq!(
            variants,
            [0, 1],
            "{role:?} keeps its own native ordinal vector"
        );
    }
    assert!(ordinals.is_empty());
    assert!(
        data.public_names.iter().any(|p| !p.name.is_ascii())
            || data.symbols.iter().any(|s| !s.name.is_ascii())
    );
    let source = files("unicode_bom");
    let core = &source["lcfix/core.py"];
    let implementation = data
        .entity_exposure_candidates
        .iter()
        .find(|candidate| candidate.exposure == exposed[0].id())
        .map(|candidate| {
            data.symbol_resolutions
                .get(candidate.resolution)
                .unwrap()
                .symbol
        })
        .unwrap();
    let effective = data
        .native_signatures
        .iter()
        .filter(|native| {
            let signature = data.signatures.get(native.signature).unwrap();
            signature.symbol == implementation
                && signature.role == calls::SignatureRole::EffectiveTyped
        })
        .collect::<Vec<_>>();
    assert_eq!(effective.len(), 2);
    let origins = effective
        .iter()
        .map(|native| {
            assert_eq!(native.implementation, Some(implementation));
            let origin = native
                .metadata_origin
                .expect("original overload definition identity retained");
            assert_ne!(origin, implementation);
            assert_eq!(
                data.symbol_resolutions
                    .iter()
                    .find(|r| r.symbol == origin)
                    .unwrap()
                    .status,
                ResolutionStatus::Unresolved
            );
            origin
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(origins.len(), 2);
    let source_declaration = data
        .entity_declarations
        .iter()
        .find(|row| row.symbol == implementation)
        .unwrap();
    let declared = data
        .declarations
        .iter()
        .find(|row| row.declaration == source_declaration.declaration)
        .unwrap();
    let name = data.occurrences.get(declared.name).unwrap();
    assert_eq!(&core[name.start as usize..name.end as usize], b"build");
    assert!(matches!(data.export_origins.get(exposed[0].origin),
        Some(symbols::ExportOrigin::Traced { name, kind: Some(symbols::ExportKind::Function), .. }) if name == "build"));
    // A reported definition still needs its exact source declaration. Metadata-only member
    // origins are not public candidates, but missing report/declaration/support is no absence.
    let declarations = facts.declarations.iter().cloned().collect::<Vec<_>>();
    facts.declarations = Rows::new(&budget);
    for row in declarations
        .iter()
        .filter(|row| row.symbol != implementation)
    {
        facts.declarations.insert(row.clone()).unwrap();
    }
    let missing = entity_normalization::normalize(facts.inputs(), &budget).unwrap();
    assert_eq!(
        missing.exposures.get(exposed[0].id()).unwrap().status,
        ResolutionStatus::Unresolved
    );
    assert!(missing.exposure_candidates.iter().any(|candidate| {
        candidate.exposure == exposed[0].id()
            && missing
                .resolutions
                .get(candidate.resolution)
                .unwrap()
                .symbol
                == implementation
    }));
    for row in declarations {
        facts.declarations.insert(row).unwrap();
    }
    let reports = facts
        .symbol_observations
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    facts.symbol_observations = Rows::new(&budget);
    for row in reports.iter().filter(|row| row.symbol != implementation) {
        facts.symbol_observations.insert(row.clone()).unwrap();
    }
    let absent = entity_normalization::normalize(facts.inputs(), &budget).unwrap();
    assert_eq!(
        absent.exposures.get(exposed[0].id()).unwrap().status,
        ResolutionStatus::Unresolved
    );
    for row in reports {
        facts.symbol_observations.insert(row).unwrap();
    }
    let support_rows = facts.symbol_supports.iter().cloned().collect::<Vec<_>>();
    assert!(support_rows.iter().any(|support| {
        facts
            .symbol_observations
            .get(support.assertion)
            .unwrap()
            .symbol
            == implementation
            && support.fidelity == attribution::Fidelity::ReportProjection
            && support.origin == attribution::Origin::AnalyzerAssertion
            && support.mode == attribution::ExtractionMode::NativeTraversal
    }));
    // Definition inspection accepts the provider report's declared role. Display text,
    // recognizers and derived reports cannot establish that namespace candidate inventory.
    for (fidelity, origin, mode) in [
        (
            attribution::Fidelity::DisplayOnly,
            attribution::Origin::AnalyzerAssertion,
            attribution::ExtractionMode::NativeTraversal,
        ),
        (
            attribution::Fidelity::ReportProjection,
            attribution::Origin::AnalyzerAssertion,
            attribution::ExtractionMode::Recognizer,
        ),
        (
            attribution::Fidelity::ReportProjection,
            attribution::Origin::DerivedAnalysis,
            attribution::ExtractionMode::NativeTraversal,
        ),
    ] {
        facts.symbol_supports = Rows::new(&budget);
        for mut support in support_rows.iter().cloned() {
            if facts
                .symbol_observations
                .get(support.assertion)
                .unwrap()
                .symbol
                == implementation
            {
                support.fidelity = fidelity;
                support.origin = origin;
                support.mode = mode;
            }
            facts.symbol_supports.insert(support).unwrap();
        }
        let wrong_role = entity_normalization::normalize(facts.inputs(), &budget).unwrap();
        assert_eq!(
            wrong_role.exposures.get(exposed[0].id()).unwrap().status,
            ResolutionStatus::Unresolved
        );
    }
    facts.symbol_supports = Rows::new(&budget);
    for mut support in support_rows {
        if facts
            .symbol_observations
            .get(support.assertion)
            .unwrap()
            .symbol
            == implementation
        {
            let mut foreign = facts.runs.get(support.run).unwrap().clone();
            foreign.configuration = ContentHash::of(b"different native report invocation");
            support.run = facts.runs.insert(foreign).unwrap();
        }
        facts.symbol_supports.insert(support).unwrap();
    }
    let foreign = entity_normalization::normalize(facts.inputs(), &budget).unwrap();
    assert_eq!(
        foreign.exposures.get(exposed[0].id()).unwrap().status,
        ResolutionStatus::Unresolved
    );
}
#[tokio::test]
async fn recovered_dataclass_init_is_synthetic_and_dead_branch_declarations_stay_inspectable() {
    let (data, _, _) = fixture("derive_cases").await;
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
