//! Source-written N2 controls, run through the real pinned native facts providers.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    lexical::*,
    normalized::{
        entities::ResolutionStatus, entity_normalization, links::*, relation_normalization::*,
    },
    resources::ResourceBudget,
    source::*,
    types::*,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, Occurrence);
async fn fixture() -> (RelationData, RelationOutput) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("normalized_relations"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { data.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    data.entities = entity_normalization::normalize(data.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_relation_inputs!(additional);
    let output = normalize(&data, &budget).unwrap();
    (data, output)
}
fn text(data: &RelationData, occurrence: Id<Occurrence>) -> String {
    let occurrence = data.facts.occurrences.get(occurrence).unwrap();
    let source = data.artifacts.get(occurrence.source).unwrap();
    String::from_utf8(
        files("normalized_relations")[&source.path]
            [occurrence.start as usize..occurrence.end as usize]
            .to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn lexical_builtins_import_gaps_ancestry_and_mentions_keep_their_evidence() {
    let (data, output) = fixture().await;
    assert_eq!(
        output.reference_entity_assessments.len(),
        data.references.len()
    );
    assert_eq!(output.import_module_assessments.len(), data.imports.len());
    assert_eq!(
        output.ancestry_entity_assessments.len(),
        data.ancestry.len()
    );
    assert_eq!(output.mention_entity_assessments.len(), data.mentions.len());
    assert!(output.reference_targets.iter().any(|t| matches!(t, ReferenceEntityTarget::Builtin { target } if matches!(data.lexical_targets.get(*target), Some(LexicalTarget::Builtin { name, .. }) if name == "len"))));
    let missing = data
        .imports
        .iter()
        .find(|i| i.resolved_module.as_deref() == Some("missing_package"))
        .unwrap();
    let assessment = output
        .import_module_assessments
        .iter()
        .find(|a| a.observation == missing.id())
        .unwrap();
    assert_eq!(assessment.status, ResolutionStatus::Unresolved);
    assert!(!output.ancestry_entity_members.is_empty());
    let mentions: Vec<_> = data
        .mentions
        .iter()
        .map(|m| (&m.form, &m.access_path, &m.qualified_name))
        .collect();
    assert!(!mentions.is_empty(), "source guide produces mentions");
    let private = data
        .mentions
        .iter()
        .find(|m| m.qualified_name.as_deref() == Some("relations.Box.method"))
        .unwrap_or_else(|| panic!("{mentions:?}"));
    let assessment = output
        .mention_entity_assessments
        .iter()
        .find(|a| a.observation == private.id())
        .unwrap();
    assert_eq!(assessment.status, ResolutionStatus::Resolved);
    assert!(
        output
            .mention_symbol_candidates
            .iter()
            .any(|m| m.assessment == assessment.id())
    );
}
#[tokio::test]
async fn native_variable_binders_stay_in_their_captured_source_or_stub() {
    let (data, output) = fixture().await;
    assert_eq!(output.type_binder_assessments.len(), data.variables.len());
    let mut matched = 0;
    let mut dual = std::collections::BTreeSet::new();
    for variable in data
        .variables
        .iter()
        .filter(|v| v.origin == TypeVariableOrigin::Pep695)
    {
        let assessment = output
            .type_binder_assessments
            .iter()
            .find(|a| a.variable == variable.id())
            .unwrap();
        if let calls::ProviderModule::Acquired { module } =
            data.facts.provider_modules.get(variable.module).unwrap()
        {
            let source = data.facts.modules.get(*module).unwrap().source;
            assert_eq!(
                assessment.status,
                ResolutionStatus::Resolved,
                "{variable:?} {assessment:?}"
            );
            let candidates: Vec<_> = output
                .type_binder_candidates
                .iter()
                .filter(|c| c.assessment == assessment.id())
                .collect();
            assert!(!candidates.is_empty());
            for candidate in candidates {
                assert_eq!(
                    data.facts
                        .occurrences
                        .get(candidate.declaration)
                        .unwrap()
                        .source,
                    source
                );
            }
            let path = &data.artifacts.get(source).unwrap().path;
            if path.starts_with("dual.") {
                dual.insert(path.clone());
            }
            matched += 1;
        }
    }
    assert!(
        matched >= 3,
        "captured PEP 695 binders exercised: {matched}; dual paths {dual:?}"
    );
    assert!(
        !dual.is_empty(),
        "the native dual module has a captured coordinate space"
    );
}
#[tokio::test]
async fn test_operand_links_use_exact_occurrence_context_and_role_and_places_keep_roots() {
    let (data, output) = fixture().await;
    assert!(!data.leaves.is_empty());
    assert_eq!(
        output.test_operand_type_assessments.len(),
        data.leaves.len()
    );
    assert!(!output.test_operand_type_links.is_empty());
    for link in output.test_operand_type_links.iter() {
        let assessment = output
            .test_operand_type_assessments
            .get(link.assessment)
            .unwrap();
        let leaf = data.leaves.get(assessment.leaf).unwrap();
        let observation = data.type_observations.get(link.observation).unwrap();
        assert_eq!(Some(observation.subject), leaf.operand);
        assert_eq!(observation.role, TypeRole::TestOperand);
        assert_eq!(
            data.facts
                .qualifications
                .get(observation.qualification)
                .unwrap()
                .context,
            data.facts
                .qualifications
                .get(leaf.qualification)
                .unwrap()
                .context
        );
        assert!(text(&data, observation.subject).contains("value"));
    }
    assert_eq!(output.place_entity_links.len(), data.facts.places.len());
    assert!(
        output
            .place_entity_links
            .iter()
            .any(|link| link.status == ResolutionStatus::Resolved)
    );
}

#[tokio::test]
async fn synthetic_and_external_coordinates_never_reuse_a_captured_binder() {
    let (mut data, _) = fixture().await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let native = data
        .variables
        .iter()
        .find(|v| {
            v.origin == TypeVariableOrigin::Pep695
                && matches!(
                    data.facts.provider_modules.get(v.module),
                    Some(calls::ProviderModule::Acquired { .. })
                )
        })
        .unwrap()
        .clone();
    let synthetic = TypeVariable {
        origin: TypeVariableOrigin::Synthetic,
        ..native.clone()
    };
    let module = calls::ProviderModule::Bundled {
        provider: native.provider,
        bundle: calls::ModuleBundle::Typeshed,
        name: "same_coordinates".into(),
    };
    data.facts.provider_modules.insert(module.clone()).unwrap();
    let external = TypeVariable {
        module: module.id(),
        ..native
    };
    data.variables.insert(synthetic.clone()).unwrap();
    data.variables.insert(external.clone()).unwrap();
    let output = normalize(&data, &budget).unwrap();
    for (variable, reason) in [
        (synthetic.id(), LinkReason::UnsupportedNativeOrigin),
        (external.id(), LinkReason::OutsideCapturedScope),
    ] {
        let assessment = output
            .type_binder_assessments
            .iter()
            .find(|a| a.variable == variable)
            .unwrap();
        assert_eq!(assessment.status, ResolutionStatus::Unresolved);
        assert_eq!(assessment.reason, reason);
        assert!(
            !output
                .type_binder_candidates
                .iter()
                .any(|c| c.assessment == assessment.id())
        );
    }
}
#[tokio::test]
async fn wrong_context_or_role_cannot_supply_a_missing_test_operand_type() {
    let (mut data, previous) = fixture().await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let link = previous.test_operand_type_links.iter().next().unwrap();
    let original = data
        .type_observations
        .get(link.observation)
        .unwrap()
        .clone();
    let assessment = previous
        .test_operand_type_assessments
        .get(link.assessment)
        .unwrap();
    let leaf = data.leaves.get(assessment.leaf).unwrap().clone();
    // A foreign context is a different semantic invocation even at identical captured bytes.
    let original_q = data
        .facts
        .qualifications
        .get(original.qualification)
        .unwrap()
        .clone();
    let context = attribution::AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "foreign-control".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"foreign"),
        environment_digest: ContentHash::of(b"foreign"),
        lock_digest: None,
    };
    let foreign_q = assertion::AssertionQualification {
        context: context.id(),
        ..original_q
    };
    data.facts.qualifications.insert(foreign_q.clone()).unwrap();
    data.type_observations = normalized::Rows::new(&budget);
    data.type_observations
        .insert(TypeObservation {
            qualification: foreign_q.id(),
            ..original.clone()
        })
        .unwrap();
    data.type_observations
        .insert(TypeObservation {
            role: TypeRole::Argument,
            ..original
        })
        .unwrap();
    let output = normalize(&data, &budget).unwrap();
    let assessment = output
        .test_operand_type_assessments
        .iter()
        .find(|a| a.leaf == leaf.id())
        .unwrap();
    assert_eq!(assessment.status, ResolutionStatus::Unresolved);
    assert!(
        !output
            .test_operand_type_links
            .iter()
            .any(|l| l.assessment == assessment.id())
    );
    let tiny = ResourceBudget::fixed(64).unwrap();
    assert!(matches!(
        normalize(&data, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
}
#[tokio::test]
async fn shared_relationship_validation_requires_all_outcomes_and_all_candidates() {
    let (data, output) = fixture().await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    for omit in [
        None,
        Some(ReferenceEntityAssessment::NAME),
        Some(TypeBinderCandidate::NAME),
        Some("tamper_test_operand"),
    ] {
        let mut check = (invariants().remove(0).create)(&budget);
        macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.facts.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_entity_inputs!(facts);
        macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_entity_outputs!(entities);
        macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(check.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
        lctx_model::normalized_relation_inputs!(additional);
        macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(if omit != Some(<$ty>::NAME) && !(omit == Some("tamper_test_operand") && <$ty>::NAME == TestOperandTypeLink::NAME) { check.visit(<$ty>::NAME, &<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
        lctx_model::normalized_relation_outputs!(outputs);
        if omit == Some("tamper_test_operand") {
            // Recover the dormant compile.rs operand/type proof-link mutation: nominally valid
            // row and type reference, but a different semantic role cannot prove this operand.
            let wrong = data
                .type_observations
                .iter()
                .find(|o| o.role == TypeRole::Parameter)
                .unwrap()
                .id();
            let altered: Vec<_> = output
                .test_operand_type_links
                .iter()
                .map(|link| TestOperandTypeLink {
                    observation: wrong,
                    ..link.clone()
                })
                .collect();
            check
                .visit(
                    TestOperandTypeLink::NAME,
                    &TestOperandTypeLink::encode(&altered).unwrap(),
                )
                .unwrap();
        }
        assert_eq!(check.finish().is_ok(), omit.is_none(), "{omit:?}");
    }
}

#[tokio::test]
async fn source_stub_coordinate_twins_select_only_the_explicit_provider_module() {
    let (mut data, _) = fixture().await;
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let native = data.variables.iter().find(|v| v.origin == TypeVariableOrigin::Pep695 &&
        matches!(data.facts.provider_modules.get(v.module), Some(calls::ProviderModule::Acquired { module }) if data.facts.modules.get(*module).unwrap().qualified_name == "dual")).unwrap().clone();
    let modules: Vec<_> = data
        .facts
        .modules
        .iter()
        .filter(|m| m.qualified_name == "dual")
        .cloned()
        .collect();
    assert_eq!(modules.len(), 2);
    // The native producer may report only its selected stub's type coordinate. These explicit
    // input twins exercise the normalizer's contract without claiming the provider emitted both.
    let mut expected = Vec::new();
    for module in modules {
        let provider_module = calls::ProviderModule::Acquired {
            module: module.id(),
        };
        data.facts
            .provider_modules
            .insert(provider_module.clone())
            .unwrap();
        let variable = TypeVariable {
            module: provider_module.id(),
            ..native.clone()
        };
        data.variables.insert(variable.clone()).unwrap();
        expected.push((variable.id(), module.source));
    }
    let output = normalize(&data, &budget).unwrap();
    let mut declarations = std::collections::BTreeSet::new();
    for (variable, source) in expected {
        let assessment = output
            .type_binder_assessments
            .iter()
            .find(|a| a.variable == variable)
            .unwrap();
        assert_eq!(assessment.status, ResolutionStatus::Resolved);
        for candidate in output
            .type_binder_candidates
            .iter()
            .filter(|c| c.assessment == assessment.id())
        {
            assert_eq!(
                data.facts
                    .occurrences
                    .get(candidate.declaration)
                    .unwrap()
                    .source,
                source
            );
            declarations.insert(candidate.declaration);
        }
    }
    assert_eq!(
        declarations.len(),
        2,
        "identical offsets do not join two captured modules"
    );
    let legacy: Vec<_> = data
        .variables
        .iter()
        .filter(|v| {
            v.origin == TypeVariableOrigin::ScopedLegacy
                && matches!(
                    data.facts.provider_modules.get(v.module),
                    Some(calls::ProviderModule::Acquired { .. })
                )
        })
        .collect();
    assert!(!legacy.is_empty());
    assert!(legacy.iter().all(|v| {
        output
            .type_binder_assessments
            .iter()
            .any(|a| a.variable == v.id() && a.status == ResolutionStatus::Resolved)
    }));
}
