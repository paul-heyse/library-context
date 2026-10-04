//! Independent written callable examples through the pinned native providers.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    calls::*,
    normalized::{
        callable_normalization::*, callables::*, entities::*, entity_normalization,
        relation_normalization,
    },
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, Occurrence);
async fn fixture() -> (CallableData, CallableOutput) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("effective_callables"), Facts(tables.clone()))
        .await
        .unwrap();
    normalize_tables(&tables, false)
}
fn normalize_tables(
    tables: &typed_driver::Tables,
    incomplete_syntax: bool,
) -> (CallableData, CallableOutput) {
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities =
        entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &budget).unwrap();
    let mut data = CallableData::new(&budget);
    macro_rules! raw { ($($field:ident: $ty:ty,)*) => { $(for row in rows::<$ty>(&tables) { if <$ty>::NAME != attribution::ProviderCoverage::NAME { data.$field.insert(row).unwrap(); } })* }; }
    lctx_model::normalized_callable_inputs!(raw);
    for mut row in rows::<attribution::ProviderCoverage>(tables) {
        if incomplete_syntax && row.family == attribution::FactFamily::Syntax {
            row.status = attribution::CoverageStatus::Partial;
            row.reason = Some(obligation::ObligationKind::MissingEvidence);
            row.diagnostic = Some("selected structural-syntax negative control".into());
        }
        data.coverage.insert(row).unwrap();
    }
    macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(let batch = <$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap(); data.visit(<$ty>::NAME, &batch).unwrap();)* }; }
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links { ($($field:ident: $ty:ty,)*) => { $(let batch = <$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap(); data.visit(<$ty>::NAME, &batch).unwrap();)* }; }
    lctx_model::normalized_relation_outputs!(links);
    let output = normalize(&data, &budget).unwrap();
    (data, output)
}
fn named<'a>(
    data: &CallableData,
    output: &'a CallableOutput,
    name: &str,
) -> &'a EffectiveCallableAssessment {
    let source = files("effective_callables");
    let bytes = &source["surfaces.py"];
    let declaration = data
        .declarations
        .iter()
        .find(|d| {
            let o = data.occurrences.get(d.name).unwrap();
            &bytes[o.start as usize..o.end as usize] == name.as_bytes()
        })
        .unwrap_or_else(|| panic!("source declaration {name}"));
    let entity = data.callables.iter().find(|c| matches!(c, CallableEntity::Source { declaration: occurrence, .. } if *occurrence == declaration.declaration)).unwrap();
    output
        .assessments
        .iter()
        .find(|a| a.callable == entity.id())
        .unwrap()
}
#[tokio::test]
async fn undecorated_signatures_and_exact_descriptors_have_separate_body_authority() {
    let (data, output) = fixture().await;
    for (name, kind) in [
        ("plain", DescriptorKind::Function),
        ("ordinary", DescriptorKind::InstanceMethod),
        ("static", DescriptorKind::StaticMethod),
        ("create", DescriptorKind::ClassMethod),
        ("value", DescriptorKind::Property),
    ] {
        let a = named(&data, &output, name);
        assert_eq!(a.identity, Knowledge::Known, "{name}: {a:?}");
        assert_eq!(a.descriptor_kind, Some(kind), "{name}: {a:?}");
        assert!(a.body_admitted, "{name}: {a:?}");
    }
    assert_eq!(
        named(&data, &output, "asynchronous").asynchronous,
        Some(true)
    );
    assert_eq!(named(&data, &output, "generator").generator, Some(true));
    assert_eq!(named(&data, &output, "plain").generator, Some(false));
}
#[tokio::test]
async fn arbitrary_shadowed_and_stacked_decorators_keep_source_contracts_and_order() {
    let (data, output) = fixture().await;
    for name in ["decorated", "wrapped", "managed", "stacked", "shadow"] {
        let a = named(&data, &output, name);
        assert_eq!(a.identity, Knowledge::Unknown, "{name}: {a:?}");
        assert!(!a.body_admitted);
        assert!(
            output
                .variants
                .iter()
                .any(|v| v.callable == Some(a.callable)),
            "source variants for {name}"
        );
    }
    let stacked = named(&data, &output, "stacked");
    let mut order: Vec<_> = output
        .decorators
        .iter()
        .filter(|d| d.assessment == stacked.id())
        .map(|d| (d.source_ordinal, d.application_ordinal))
        .collect();
    order.sort();
    assert_eq!(order, [(0, 1), (1, 0)]);
}
#[tokio::test]
async fn signature_slots_preserve_raw_order_defaults_collectors_and_entity_links() {
    let (data, output) = fixture().await;
    assert_eq!(output.variants.len(), data.signatures.len());
    assert_eq!(output.slots.len(), data.parameters.len());
    assert_eq!(output.slot_entities.len(), data.parameter_links.len());
    let a = named(&data, &output, "plain");
    let variant = output
        .variants
        .iter()
        .find(|v| v.callable == Some(a.callable) && v.role == SignatureRole::Source)
        .unwrap();
    let mut slots: Vec<_> = output
        .slots
        .iter()
        .filter(|s| s.variant == variant.id())
        .collect();
    slots.sort_by_key(|s| s.ordinal);
    assert_eq!(
        slots.iter().map(|s| s.default).collect::<Vec<_>>(),
        [
            DefaultSlot::Required,
            DefaultSlot::DefinitionTime,
            DefaultSlot::Required,
            DefaultSlot::DefinitionTime,
            DefaultSlot::Collector
        ]
    );
    assert_eq!(
        slots
            .iter()
            .map(|s| data
                .shapes
                .get(data.parameters.get(s.parameter).unwrap().shape)
                .unwrap()
                .kind)
            .collect::<Vec<_>>(),
        [
            ParameterKind::PositionalOnly,
            ParameterKind::PositionalOrKeyword,
            ParameterKind::KeywordOnly,
            ParameterKind::KeywordOnly,
            ParameterKind::VarKeyword
        ]
    );
    let class = named(&data, &output, "create");
    let variant = output
        .variants
        .iter()
        .find(|v| v.callable == Some(class.callable))
        .unwrap();
    assert_eq!(variant.adjustment, SignatureAdjustment::BindClassReceiver);
    assert_eq!(
        output
            .slots
            .iter()
            .filter(|s| s.variant == variant.id())
            .count(),
        2,
        "raw cls formal remains; presentation never rewrites it"
    );
}
#[tokio::test]
async fn shared_validation_refuses_missing_and_falsely_admitted_callable_results() {
    let (data, output) = fixture().await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    for omit in [
        None,
        Some(EffectiveCallableAssessment::NAME),
        Some(SignatureSlot::NAME),
        Some("tampered-body"),
    ] {
        let mut check = (invariants().remove(0).create)(&budget);
        macro_rules! input { ($($field:ident: $ty:ty,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_callable_inputs!(input);
        macro_rules! out { ($($field:ident: $ty:ty,)*) => { $(if omit != Some(<$ty>::NAME) && !(omit == Some("tampered-body") && <$ty>::NAME == EffectiveCallableAssessment::NAME) { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_callable_outputs!(out);
        if omit == Some("tampered-body") {
            let target = named(&data, &output, "decorated").id();
            let rows: Vec<_> = output
                .assessments
                .iter()
                .cloned()
                .map(|mut a| {
                    if a.id() == target {
                        a.body_admitted = true;
                        a.body = Knowledge::Known;
                        a.identity = Knowledge::Known;
                        a.descriptor = Knowledge::Known;
                        a.descriptor_kind = Some(DescriptorKind::Function);
                    }
                    a
                })
                .collect();
            check
                .visit(
                    EffectiveCallableAssessment::NAME,
                    &EffectiveCallableAssessment::encode(&rows).unwrap(),
                )
                .unwrap();
        }
        assert_eq!(check.finish().is_ok(), omit.is_none(), "{omit:?}");
    }
    let tiny = ResourceBudget::fixed(64).unwrap();
    assert!(matches!(
        normalize(&data, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
    assert!(output.assessments.len() > 10);
}

/// Real row-budget refusal must not erase independent source declaration completeness.
#[tokio::test]
async fn contextual_partial_preserves_source_identity_without_contextual_absence() {
    use attribution::*;
    use cpg_extract::{
        pyrefly_stage::Pyrefly, ruff_context::ContextSettings, typed_syntax::SyntaxLimits,
    };
    use ruff::*;
    let baseline = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("effective_callables"), Facts(baseline.clone()))
        .await
        .unwrap();
    let supports = rows::<RuffContextSupport>(&baseline);
    assert!(!supports.is_empty());
    let surfaces = rows::<assertion::ProviderSurface>(&baseline);
    let runs = rows::<ProviderRun>(&baseline);
    for support in supports {
        let surface = surfaces.iter().find(|s| s.id() == support.surface).unwrap();
        let run = runs.iter().find(|r| r.id() == support.run).unwrap();
        assert_eq!(surface.family, FactFamily::Lexical);
        assert_eq!(surface.provider, cpg_extract::ruff_context::provider().id());
        assert_eq!(run.provider, surface.provider);
        assert!(
            rows::<RunFamily>(&baseline)
                .iter()
                .any(|f| f.run == run.id() && f.family == FactFamily::Lexical)
        );
    }
    let (data, output) = normalize_tables(&baseline, false);
    assert_eq!(named(&data, &output, "plain").identity, Knowledge::Known);
    let tables = typed_driver::Tables::default();
    let captured = typed_driver::capture_with_ruff(
        &files("effective_callables"),
        "context-row-budget",
        stages::Profile::Behavioral,
        ContextSettings {
            maximum_rows: 1,
            ..Default::default()
        },
    );
    typed_driver::run_profile(
        captured,
        Pyrefly::new(SyntaxLimits::default()),
        Facts(tables.clone()),
        stages::Profile::Behavioral,
    )
    .await
    .unwrap();
    let ruff = cpg_extract::ruff_context::provider().id();
    let coverage = rows::<ProviderCoverage>(&tables);
    assert!(coverage.iter().any(|c| c.provider == Some(ruff)
        && c.family == FactFamily::Lexical
        && c.status == CoverageStatus::Partial));
    assert!(
        coverage
            .iter()
            .filter(|c| c.family == FactFamily::Syntax)
            .all(|c| c.provider == Some(ruff)
                && c.status == CoverageStatus::CompleteUnderStatedModel)
    );
    assert!(coverage.iter().any(|c| c.provider != Some(ruff)
        && c.family == FactFamily::Lexical
        && c.status == CoverageStatus::CompleteUnderStatedModel));
    assert!(rows::<RuffContextObservation>(&tables).is_empty());
    let (data, output) = normalize_tables(&tables, false);
    let plain = named(&data, &output, "plain");
    assert_eq!(plain.identity, Knowledge::Known, "{plain:?}");
    assert!(plain.body_admitted, "{plain:?}");
    let declaration = data.declarations.iter().next().unwrap();
    assert_eq!(
        resolved_name(&[], declaration.qualification, declaration.name),
        None
    );
    // The same source facts with the selected structural inventory incomplete cannot
    // establish decorator absence. This is a model negative control, not a provider claim.
    let (data, output) = normalize_tables(&tables, true);
    let plain = named(&data, &output, "plain");
    assert_eq!(plain.identity, Knowledge::Unknown, "{plain:?}");
    assert_eq!(plain.identity_reason, CallableReason::IncompleteSyntax);
    assert!(!plain.body_admitted);
}
