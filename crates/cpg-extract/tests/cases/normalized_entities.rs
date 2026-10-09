//! Expected identities come from fixture source, separately from the shared closure validator.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    normalized::{entities::*, entity_normalization::*},
    resources::ResourceBudget,
    source::*,
    symbols::*,
    *,
};
use typed_driver::{files, rows, run};
macro_rules! inspector_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { inspector!(Inputs, SourceArtifact, $($ty),*); }; }
lctx_model::normalized_entity_inputs!(inspector_inputs);
async fn fixture() -> (EntityData, EntityOutput, typed_driver::Tables) {
    let tables = typed_driver::Tables::default();
    run(&files("normalized_entities"), Inputs(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(128 << 20).unwrap();
    let mut data = EntityData::new(&budget);
    macro_rules! load { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(load);
    let output = normalize(data.inputs(), &budget).unwrap();
    (data, output, tables)
}
#[tokio::test]
async fn native_origin_separates_declared_callable_fields_from_synthesized_methods() {
    let (data, output, _) = fixture().await;
    let origins: Vec<_> = data
        .function_traits
        .iter()
        .map(|t| (data.symbols.get(t.symbol).unwrap().name.as_str(), t.origin))
        .collect();
    assert!(
        origins.contains(&("callback", FunctionOrigin::CallableField)),
        "{origins:?}"
    );
    assert!(
        origins.contains(&("__init__", FunctionOrigin::Synthesized)),
        "{origins:?}"
    );
    assert!(
        origins.contains(&("method", FunctionOrigin::DefStatement)),
        "{origins:?}"
    );
    for (name, origin) in origins {
        if name == "callback" {
            assert_ne!(origin, FunctionOrigin::Synthesized);
        }
    }
    for symbol in data.symbols.iter().filter(|s| s.name == "callback") {
        let resolution = output
            .resolutions
            .iter()
            .find(|r| r.symbol == symbol.id())
            .unwrap();
        assert_eq!(resolution.status, ResolutionStatus::Unresolved);
        assert_eq!(resolution.reason, EntityReason::MissingDeclaration);
    }
    assert!(output.callables.iter().any(|c| matches!(c, CallableEntity::Synthetic { symbol } if data.symbols.get(*symbol).unwrap().name == "__init__")));
}
#[tokio::test]
async fn source_and_stub_anchors_fields_owners_and_public_gaps_are_preserved() {
    let (data, output, tables) = fixture().await;
    assert_eq!(
        output.resolutions.len(),
        data.symbols.len(),
        "one outcome per provider symbol"
    );
    assert_eq!(
        output.owners.len(),
        data.occurrences.len(),
        "one owner per occurrence"
    );
    let artifacts = rows::<SourceArtifact>(&tables);
    let same: Vec<_> = data
        .symbols
        .iter()
        .filter(|s| s.name == "same")
        .map(|s| {
            output
                .resolutions
                .iter()
                .find(|r| r.symbol == s.id())
                .unwrap()
        })
        .collect();
    assert_eq!(same.len(), 2);
    assert!(same.iter().all(|r| r.status == ResolutionStatus::Resolved));
    assert_ne!(
        same[0].entity, same[1].entity,
        "source and stub are different definitions"
    );
    let mut paths = Vec::new();
    for resolution in same {
        let EntityRef::Callable { callable } = output.refs.get(resolution.entity.unwrap()).unwrap()
        else {
            panic!("callable")
        };
        let CallableEntity::Source { declaration, .. } = output.callables.get(*callable).unwrap()
        else {
            panic!("source")
        };
        paths.push(
            artifacts
                .iter()
                .find(|a| a.id() == data.occurrences.get(*declaration).unwrap().source)
                .unwrap()
                .path
                .clone(),
        );
    }
    paths.sort();
    assert_eq!(paths, ["dual.py", "dual.pyi"]);
    let field_names: std::collections::BTreeSet<_> =
        output.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        field_names.contains("value") && field_names.contains("callback"),
        "{field_names:?}"
    );
    assert!(
        output.field_declarations.len() >= 2,
        "syntax field evidence stays distinct"
    );
    let missing = data
        .public_names
        .iter()
        .find(|p| p.name == "missing")
        .unwrap();
    let exposure = output
        .exposures
        .iter()
        .find(|p| p.observation == missing.id())
        .unwrap();
    assert_eq!(exposure.status, ResolutionStatus::Unresolved);
    assert_eq!(output.exposures.len(), data.public_names.len());
    for owner in output.owners.iter() {
        let occurrence = data.occurrences.get(owner.occurrence).unwrap();
        let local: Vec<_> = data
            .occurrences
            .iter()
            .filter(|o| o.source == occurrence.source)
            .cloned()
            .collect();
        assert_eq!(
            owner.owner,
            occurrence_owner::owner_of(occurrence, &local).unwrap()
        );
    }
}
#[tokio::test]
async fn shared_validator_rejects_missing_and_extra_normalized_outcomes() {
    let (data, output, _) = fixture().await;
    let budget = ResourceBudget::fixed(128 << 20).unwrap();
    for variant in 0..3 {
        let omit = variant == 1;
        let invariant = invariants().remove(0);
        let mut check = (invariant.create)(&budget);
        macro_rules! visit_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_entity_inputs!(visit_inputs);
        macro_rules! visit_outputs { ($($field:ident: $ty:ty,)*) => { $(if !omit || <$ty>::NAME != SymbolEntityResolution::NAME { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_entity_outputs!(visit_outputs);
        if variant == 2 {
            let extra = CallableEntity::Synthetic {
                symbol: data
                    .symbols
                    .iter()
                    .find(|s| s.name == "factory")
                    .unwrap()
                    .id(),
            };
            check
                .visit(
                    CallableEntity::NAME,
                    &<CallableEntity as Record>::encode(&[extra]).unwrap(),
                )
                .unwrap();
        }
        assert_eq!(
            check.finish().is_ok(),
            variant == 0,
            "exact total output is required"
        );
    }
    let tiny = ResourceBudget::fixed(64).unwrap();
    assert!(matches!(
        normalize(data.inputs(), &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
}

#[tokio::test]
async fn repeated_support_never_hides_a_conflicting_anchor_or_merges_provider_externals() {
    use lctx_model::domain::{
        attribution::Provider,
        calls::{ModuleBundle, ProviderModule, ProviderSymbol, SymbolKind},
        declarations::*,
    };
    let (mut data, _, _) = fixture().await;
    let budget = ResourceBudget::fixed(128 << 20).unwrap();
    let original = data
        .symbols
        .iter()
        .find(|s| s.name == "factory")
        .unwrap()
        .clone();
    let declaration = data
        .declarations
        .iter()
        .find(|d| d.symbol == original.id())
        .unwrap()
        .clone();
    let support = data
        .declaration_supports
        .iter()
        .find(|s| s.assertion == declaration.id())
        .unwrap()
        .clone();
    let provider = Provider {
        tool: "independent-structural-control".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"control"),
    };
    let second = ProviderSymbol {
        provider: provider.id(),
        ..original.clone()
    };
    let second_decl = SymbolDeclaration {
        symbol: second.id(),
        ..declaration.clone()
    };
    let mut second_run = data.runs.get(support.run).unwrap().clone();
    second_run.provider = provider.id();
    data.runs.insert(second_run.clone()).unwrap();
    let mut second_support = support.clone();
    second_support.assertion = second_decl.id();
    second_support.run = second_run.id();
    data.symbols.insert(second.clone()).unwrap();
    data.declarations.insert(second_decl).unwrap();
    data.declaration_supports.insert(second_support).unwrap();
    let agreed = normalize(data.inputs(), &budget).unwrap();
    let resolution = |output: &EntityOutput, id| {
        output
            .resolutions
            .iter()
            .find(|r| r.symbol == id)
            .unwrap()
            .clone()
    };
    assert_eq!(
        resolution(&agreed, original.id()).entity,
        resolution(&agreed, second.id()).entity,
        "two providers identify the same declaration"
    );
    let other = data
        .declarations
        .iter()
        .find(|d| data.symbols.get(d.symbol).unwrap().name == "method")
        .unwrap()
        .declaration;
    let conflict = SymbolDeclaration {
        declaration: other,
        ..declaration.clone()
    };
    let mut conflicting_support = support;
    conflicting_support.assertion = conflict.id();
    data.declarations.insert(conflict).unwrap();
    data.declaration_supports
        .insert(conflicting_support)
        .unwrap();
    let conflicted = normalize(data.inputs(), &budget).unwrap();
    let outcome = resolution(&conflicted, original.id());
    assert_eq!(outcome.status, ResolutionStatus::Ambiguous);
    assert!(outcome.entity.is_none());
    assert_eq!(
        conflicted
            .candidates
            .iter()
            .filter(|c| c.resolution == outcome.id())
            .count(),
        2
    );
    assert_eq!(
        resolution(&conflicted, second.id()).status,
        ResolutionStatus::Resolved
    );
    for provider in [original.provider, provider.id()] {
        let module = ProviderModule::Bundled {
            provider,
            bundle: ModuleBundle::Typeshed,
            name: "native".into(),
        };
        data.provider_modules.insert(module.clone()).unwrap();
        data.symbols
            .insert(ProviderSymbol {
                provider,
                context: original.context,
                module: module.id(),
                native_key: "external".into(),
                name: "identical_spelling".into(),
                kind: SymbolKind::Function,
            })
            .unwrap();
    }
    let external = normalize(data.inputs(), &budget).unwrap();
    let ids: std::collections::BTreeSet<_> = external.callables.iter().filter(|c| matches!(c, CallableEntity::External { symbol } if data.symbols.get(*symbol).unwrap().name == "identical_spelling")).map(Record::id).collect();
    assert_eq!(
        ids.len(),
        2,
        "external spelling equality is not correspondence"
    );
}
