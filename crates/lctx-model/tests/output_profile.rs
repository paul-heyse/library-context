use lctx_model::domain::{
    completed::{ContributionSpec, OutputProfile},
    producer_contract::{facts, *},
    stages::Profile,
    *,
};

fn acquisition_spec(profile: OutputProfile) -> ContributionSpec {
    let contract = facts::acquisition();
    let settings = ContentHash::of(b"focused acquisition settings");
    let role = &contract.suppliers[0];
    let supplier = attribution::Provider {
        tool: role.tool.into(),
        revision: role.analyzer_revision.into(),
        build_digest: ContentHash::of(b"focused executable"),
    };
    let mut binding = contract
        .bind(supplier.build_digest, settings, [(role.name, supplier)])
        .captured_binding
        .unwrap();
    binding.context_dependency = CapturedContextDependency::AcquisitionOnly;
    ContributionSpec {
        captured_binding: Some(binding),
        producer: contract.name.into(),
        profile,
        model: ContentHash::of(b"focused model"),
        implementation: ContentHash::of(b"focused executable"),
        configuration: Some(settings),
        inputs: vec![],
        outputs: contract
            .outputs
            .iter()
            .chain(&contract.contributes)
            .map(|relation| relation.name().to_owned())
            .collect(),
    }
}
#[test]
fn neutral_output_is_checked_against_the_actual_contract_and_exact_inventory() {
    let spec = acquisition_spec(OutputProfile::Neutral);
    spec.identity().unwrap();
    spec.validate_profile(Profile::Catalog).unwrap();
    spec.validate_profile(Profile::Behavioral).unwrap();
    let mut changed = spec.clone();
    changed.producer = "an unrecognized producer".into();
    assert!(changed.identity().is_err());
    changed = spec.clone();
    changed.outputs.pop_first();
    assert!(changed.identity().is_err());
    changed = spec.clone();
    changed.captured_binding.as_mut().unwrap().semantic_revision += 1;
    assert!(changed.identity().is_err());
    changed = spec.clone();
    changed
        .captured_binding
        .as_mut()
        .unwrap()
        .context_dependency = CapturedContextDependency::SemanticContext;
    assert!(changed.identity().is_err());
    changed = spec.clone();
    changed
        .captured_binding
        .as_mut()
        .unwrap()
        .context_dependency = CapturedContextDependency::ConservativeFullContext;
    assert!(changed.identity().is_err());
    let specific = acquisition_spec(OutputProfile::Specific(Profile::Catalog));
    specific.validate_profile(Profile::Catalog).unwrap();
    assert!(specific.validate_profile(Profile::Behavioral).is_err());
    assert_ne!(spec.identity().unwrap(), specific.identity().unwrap());
}
#[test]
fn target_profile_and_context_dependency_wire_are_explicit_and_strict() {
    for wire in [
        serde_json::json!("catalog"),
        serde_json::json!({"kind":"specific"}),
        serde_json::json!({"kind":"neutral","unrecognized":true}),
    ] {
        assert!(serde_json::from_value::<OutputProfile>(wire).is_err());
    }
    for profile in [
        OutputProfile::Neutral,
        OutputProfile::Specific(Profile::Catalog),
        OutputProfile::Specific(Profile::Behavioral),
    ] {
        let encoded = serde_json::to_value(profile).unwrap();
        assert_eq!(
            serde_json::from_value::<OutputProfile>(encoded).unwrap(),
            profile
        );
    }
    let spec = acquisition_spec(OutputProfile::Neutral);
    let mut wire = serde_json::to_value(spec.captured_binding.as_ref().unwrap()).unwrap();
    wire.as_object_mut().unwrap().remove("context_dependency");
    assert!(serde_json::from_value::<CapturedProducerBinding>(wire).is_err());
    let mut binding = spec.captured_binding.unwrap();
    binding.context_dependency = CapturedContextDependency::ModelSelection {
        catalog: Some(ContentHash::of(b"catalog")),
        requirements: vec![],
    };
    assert!(
        facts::pyrefly()
            .validate_context_dependency(&binding, OutputProfile::Specific(Profile::Catalog))
            .is_err()
    );
}
