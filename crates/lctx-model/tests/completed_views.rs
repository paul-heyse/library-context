use lctx_model::domain::{
    ContentHash, Relation, analysis::sources::SourceSnapshot, completed::*, input::Package,
    stages::Profile,
};
use std::collections::{BTreeMap, BTreeSet};

fn spec(producer: &str) -> ContributionSpec {
    ContributionSpec {
        captured_binding: None,
        producer: producer.into(),
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: ContentHash::of(b"model"),
        implementation: ContentHash::of(b"implementation"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from(["packages".into()]),
    }
}
#[test]
fn completed_empty_is_explicit_and_failed_or_missing_outputs_are_not_completed() {
    let mut contribution = CompletedContribution {
        assurance:
            lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
        spec: spec("provider"),
        outcome: 0,
        outputs: BTreeMap::from([(
            "packages".into(),
            OutputContent {
                rows: 0,
                content: ContentHash::of(b"empty"),
            },
        )]),
    };
    let complete = contribution.identity().unwrap();
    contribution.outcome = 2;
    assert_ne!(complete, contribution.identity().unwrap());
    contribution.outcome = 3;
    assert!(contribution.identity().is_err());
    contribution.outcome = 0;
    contribution.outputs.clear();
    assert!(contribution.identity().is_err());
}
#[test]
fn union_identity_and_source_do_not_depend_on_completion_order() {
    let a = spec("a").identity().unwrap();
    let b = spec("b").identity().unwrap();
    let relation = Relation::of::<Package>();
    let first = CompletedView::new(relation.name().into(), BTreeSet::from([a, b]), 0).unwrap();
    let second = CompletedView::new(relation.name().into(), BTreeSet::from([b, a]), 0).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        SourceSnapshot::of_completed_view(&relation, ContentHash::of(b"model"), &first).unwrap(),
        SourceSnapshot::of_completed_view(&relation, ContentHash::of(b"model"), &second).unwrap()
    );
    let mut forged = first.clone();
    forged.rows = 1;
    assert!(forged.validate().is_err());
}
#[test]
fn binding_validates_exact_view_and_freezes_boundary_identity() {
    let relation = Relation::of::<Package>();
    let view = CompletedView::new(
        relation.name().into(),
        BTreeSet::from([spec("a").identity().unwrap()]),
        0,
    )
    .unwrap();
    let source =
        SourceSnapshot::of_completed_view(&relation, ContentHash::of(b"model"), &view).unwrap();
    let mut binding = CompletedBinding {
        boundary: None,
        source,
        view,
        configuration: None,
    };
    binding.validate().unwrap();
    let current = binding.key();
    binding.boundary = Some("facts".into());
    binding.validate().unwrap();
    assert_ne!(current, binding.key());
    binding.view.rows = 1;
    assert!(binding.validate().is_err());
}
#[test]
fn contribution_input_roles_preserve_order_and_multiplicity() {
    let relation = Relation::of::<Package>();
    let model = ContentHash::of(b"model");
    let source = |producer| {
        let view = CompletedView::new(
            relation.name().into(),
            BTreeSet::from([spec(producer).identity().unwrap()]),
            0,
        )
        .unwrap();
        SourceSnapshot::of_completed_view(&relation, model, &view).unwrap()
    };
    let first = source("first source");
    let second = source("second source");
    let mut baseline = spec("consumer");
    baseline.inputs = vec![first.clone(), second.clone()];
    let identity = baseline.identity().unwrap();
    let mut reordered = baseline.clone();
    reordered.inputs.reverse();
    assert_ne!(identity, reordered.identity().unwrap());
    let mut repeated = baseline.clone();
    repeated.inputs.push(first);
    assert_ne!(identity, repeated.identity().unwrap());
    let mut missing = baseline.clone();
    missing.inputs.pop();
    assert_ne!(identity, missing.identity().unwrap());
    assert_eq!(identity, baseline.identity().unwrap());
}

#[test]
fn retained_binding_identity_distinguishes_source_configuration_and_role() {
    let relation = Relation::of::<Package>();
    let model = ContentHash::of(b"binding model");
    let view = CompletedView::new(
        relation.name().into(),
        BTreeSet::from([ContentHash::of(b"contributor")]),
        0,
    )
    .unwrap();
    let source = SourceSnapshot::of_relation(
        &relation,
        "original supplier",
        model,
        ContentHash::of(b"supplier implementation"),
        view.identity,
        0,
    )
    .unwrap();
    let binding = CompletedBinding {
        boundary: None,
        source,
        view,
        configuration: None,
    };
    let expected = binding.descriptor_identity().unwrap();
    let inventory = binding_inventory_identity(std::slice::from_ref(&binding)).unwrap();
    for source in [
        SourceSnapshot::of_relation(
            &relation,
            "different supplier",
            model,
            ContentHash::of(b"supplier implementation"),
            binding.view.identity,
            0,
        )
        .unwrap(),
        SourceSnapshot::of_relation(
            &relation,
            "original supplier",
            ContentHash::of(b"different model"),
            ContentHash::of(b"supplier implementation"),
            binding.view.identity,
            0,
        )
        .unwrap(),
        SourceSnapshot::of_relation(
            &relation,
            "original supplier",
            model,
            ContentHash::of(b"different implementation"),
            binding.view.identity,
            0,
        )
        .unwrap(),
    ] {
        let changed = CompletedBinding {
            source,
            ..binding.clone()
        };
        assert_eq!(binding.key(), changed.key());
        assert_ne!(expected, changed.descriptor_identity().unwrap());
        assert_ne!(inventory, binding_inventory_identity(&[changed]).unwrap());
    }
    let configured = CompletedBinding {
        configuration: Some(ContentHash::of(b"configuration")),
        ..binding.clone()
    };
    assert_eq!(binding.key(), configured.key());
    assert_ne!(expected, configured.descriptor_identity().unwrap());
    let boundary = CompletedBinding {
        boundary: Some("analysis".into()),
        ..binding.clone()
    };
    assert_ne!(expected, boundary.descriptor_identity().unwrap());
    assert_eq!(expected, binding.clone().descriptor_identity().unwrap());
    assert!(binding_inventory_identity(&[binding.clone(), configured]).is_err());
}
