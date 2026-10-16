use lctx_model::domain::{
    ContentHash, Record,
    attribution::{FactFamily, Provider},
    completed::{ContributionSpec, STATE_FORMAT_VERSION},
    graph::{ARTIFACT_FORMAT_VERSION, Manifest},
    input::Package,
    producer_contract::*,
    stages::{Effect, Profile, RelationUse},
};
use std::collections::{BTreeMap, BTreeSet};

fn contract() -> ProducerContract {
    ProducerContract {
        name: "independent-producer",
        semantic_revision: 1,
        inputs: vec![],
        outputs: vec![RelationUse::of::<Package>()],
        contributes: vec![],
        profiles: vec![Profile::Catalog],
        output_profile: OutputProfilePolicy::Specific,
        context_dependency: ContextDependencyPolicy::SemanticContext,
        effect: Effect::Extraction,
        configuration: ConfigurationBinding::ProducerSettings,
        suppliers: vec![SupplierRole {
            name: "independent-role",
            tool: "independent-analyzer",
            analyzer_revision: "7.1.0",
            semantic_revision: 1,
            families: vec![FactFamily::Artifacts],
            configuration: ConfigurationBinding::CapturedAnalysisContext,
        }],
        not_requested: vec![],
    }
}
fn provider(build: &str) -> Provider {
    Provider {
        tool: "independent-analyzer".into(),
        revision: "7.1.0".into(),
        build_digest: ContentHash::of(build.as_bytes()),
    }
}
#[test]
fn declarations_bind_new_roles_without_a_classifier_and_keep_builds_independent() {
    let contract = contract();
    let settings = ContentHash::of(b"captured settings");
    let old = provider("captured build");
    let current = provider("unrelated executable rebuild");
    let captured = contract.bind(
        old.build_digest,
        settings,
        [("independent-role", old.clone())],
    );
    let executable = contract.bind(
        current.build_digest,
        settings,
        [("independent-role", current)],
    );
    assert_eq!(
        captured.captured_binding.as_ref().unwrap().contract,
        executable.captured_binding.as_ref().unwrap().contract
    );
    assert_ne!(captured.coverage, executable.coverage);
    contract
        .validate_binding(
            captured.captured_binding.as_ref().unwrap(),
            Some(settings),
            std::slice::from_ref(&old),
        )
        .unwrap();
    let mut changed = contract.clone();
    changed.suppliers[0].semantic_revision += 1;
    assert!(
        changed
            .validate_binding(
                captured.captured_binding.as_ref().unwrap(),
                Some(settings),
                std::slice::from_ref(&old)
            )
            .is_err()
    );
    changed = contract.clone();
    changed.suppliers[0].analyzer_revision = "8.0.0";
    assert!(
        changed
            .validate_binding(
                captured.captured_binding.as_ref().unwrap(),
                Some(settings),
                std::slice::from_ref(&old)
            )
            .is_err()
    );
    changed = contract.clone();
    changed.configuration =
        ConfigurationBinding::Fixed(ContentHash::of(b"fixed contract settings"));
    assert_ne!(changed.identity(), contract.identity());
    let mut absent = captured.captured_binding.clone().unwrap();
    absent.suppliers.clear();
    assert!(
        contract
            .validate_binding(&absent, Some(settings), std::slice::from_ref(&old))
            .is_err()
    );
    let mut extra = captured.captured_binding.clone().unwrap();
    extra
        .suppliers
        .insert("undeclared-role".into(), provider("foreign").id());
    assert!(
        contract
            .validate_binding(&extra, Some(settings), std::slice::from_ref(&old))
            .is_err()
    );
}
#[test]
fn contribution_wire_requires_explicit_binding_and_its_identity_binds_roles() {
    let contract = contract();
    let provider = provider("capture");
    let settings = ContentHash::of(b"settings");
    let stage = contract.bind(
        provider.build_digest,
        settings,
        [("independent-role", provider)],
    );
    let mut spec = ContributionSpec {
        captured_binding: stage.captured_binding,
        producer: contract.name.into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"model"),
        implementation: ContentHash::of(b"implementation"),
        configuration: Some(settings),
        inputs: vec![],
        outputs: BTreeSet::from(["packages".into()]),
    };
    let identity = spec.identity().unwrap();
    spec.captured_binding.as_mut().unwrap().suppliers = BTreeMap::from([(
        "independent-role".into(),
        self::provider("changed capture").id(),
    )]);
    assert_ne!(identity, spec.identity().unwrap());
    let mut wire = serde_json::to_value(&spec).unwrap();
    wire.as_object_mut().unwrap().remove("captured_binding");
    assert!(serde_json::from_value::<ContributionSpec>(wire.clone()).is_err());
    wire["captured_binding"] = serde_json::Value::Null;
    assert!(
        serde_json::from_value::<ContributionSpec>(wire)
            .unwrap()
            .captured_binding
            .is_none()
    );
}
#[test]
fn old_headers_reject_before_malformed_descriptors_are_reconstructed() {
    for (artifact, state) in std::iter::once((2, 2))
        .chain((1..STATE_FORMAT_VERSION).map(|state| (ARTIFACT_FORMAT_VERSION, state)))
    {
        let bytes=serde_json::to_vec(&serde_json::json!({"format_version":artifact,"completed_state":{"format_version":state},"producers":"not a descriptor inventory"})).unwrap();
        let error = Manifest::decode(&bytes).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported artifact/completed-state format")
        );
    }
    use lctx_model::domain::completed::CompletedStateHeader;
    for version in 1..STATE_FORMAT_VERSION {
        let bytes = serde_json::to_vec(&serde_json::json!({"format_version": version})).unwrap();
        assert!(CompletedStateHeader::decode(&bytes).is_err());
    }
    assert!(
        CompletedStateHeader::decode(
            br#"{"table":"compiler_contribution","row":"old unframed descriptor"}"#
        )
        .is_err()
    );
    assert_eq!(
        CompletedStateHeader::decode(
            &serde_json::to_vec(&CompletedStateHeader::current()).unwrap()
        )
        .unwrap(),
        CompletedStateHeader::current()
    );
}
#[test]
fn semantic_contract_input_roles_preserve_order_and_cardinality() {
    let baseline = lctx_model::domain::producer_contract::facts::documents();
    assert!(baseline.inputs.len() > 1);
    let mut reordered = baseline.clone();
    reordered.inputs.reverse();
    assert_ne!(baseline.identity(), reordered.identity());
    let mut repeated = baseline.clone();
    repeated.inputs.push(baseline.inputs[0]);
    assert_ne!(baseline.identity(), repeated.identity());
    let mut profiles = baseline.clone();
    profiles.profiles.reverse();
    assert_eq!(baseline.identity(), profiles.identity());
}
#[test]
fn stage_declaration_roles_preserve_order_and_repetition_while_output_sets_remain_canonical() {
    let contract = lctx_model::domain::producer_contract::facts::documents();
    let supplier = &contract.suppliers[0];
    let provider = Provider {
        tool: supplier.tool.into(),
        revision: supplier.analyzer_revision.into(),
        build_digest: ContentHash::of(b"ordered stage control"),
    };
    let baseline = contract.bind(
        provider.build_digest,
        ContentHash::of(b"settings"),
        [(supplier.name, provider)],
    );
    let mut reordered = baseline.clone();
    reordered.inputs.reverse();
    assert_ne!(baseline.digest(), reordered.digest());
    let mut repeated = baseline.clone();
    repeated.inputs.push(baseline.inputs[0]);
    assert_ne!(baseline.digest(), repeated.digest());
    let mut output_set = baseline.clone();
    output_set.outputs.reverse();
    output_set.contributes.reverse();
    assert_eq!(baseline.digest(), output_set.digest());
}
