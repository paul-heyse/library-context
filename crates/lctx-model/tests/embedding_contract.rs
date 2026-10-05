use lctx_model::domain::{
    embedding::{value::*, *},
    *,
};

fn spec() -> Spec {
    let mut spec = Spec::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/embedding/qwen3-embedding-8b.json"
    )))
    .unwrap();
    spec.reduction = "none".into();
    spec.admission = None;
    spec.dimensions = 3;
    spec.source_dimensions = 3;
    spec
}
#[test]
fn typed_spec_reconstructs_exact_configuration_and_refuses_payload_drift() {
    let spec = spec();
    let row = EmbeddingSpec::new(&spec).unwrap();
    row.validate().unwrap();
    assert_eq!(row.configuration().unwrap(), spec);
    let model = ValidatedModel::declared(configuration_relations()).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let batch = Batch::new(&model, vec![row.clone()], &budget).unwrap();
    assert_eq!(
        EmbeddingSpec::decode(batch.arrow()).unwrap(),
        std::slice::from_ref(&row)
    );
    let mut foreign = row.clone();
    foreign.revision.push_str("-changed");
    assert!(foreign.validate().is_err());
    let mut admission = row.clone();
    admission.use_activation = Some(true);
    assert!(admission.validate().is_err());
    let mut dimensions = row;
    dimensions.dimensions = -1;
    assert!(dimensions.validate().is_err());
}
#[test]
fn exact_value_keeps_signed_zero_and_owns_reservations() {
    let spec = spec();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let value = AdmittedValue::new(&spec, "same request", 3, &[1.0, 0.0, -0.0], &budget).unwrap();
    assert_eq!(value.bytes(), [0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 128]);
    assert_ne!(value.digest(), value_digest(&[1.0, 0.0, 0.0]));
    let decoded = value.decode(&spec, &budget).unwrap();
    assert_eq!(decoded.values()[2].to_bits(), (-0.0f32).to_bits());
    let held = budget.reserved();
    assert!(held > 0);
    drop(decoded);
    assert!(budget.reserved() > 0 && budget.reserved() < held);
    let mut other = spec.clone();
    other.revision.push_str("-other");
    assert!(value.decode(&other, &budget).is_err());
    assert!(decode(&spec, value.bytes(), ContentHash::of(b"forged"), 3, &budget).is_err());
    assert!(decode(&spec, &value.bytes()[..8], value.digest(), 3, &budget).is_err());
    drop(value);
    assert_eq!(budget.reserved(), 0);
    assert!(
        AdmittedValue::new(
            &spec,
            "too many",
            spec.max_document_tokens + 1,
            &[1.0, 0.0, 0.0],
            &budget
        )
        .is_err()
    );
    assert!(AdmittedValue::new(&spec, "not unit", 1, &[0.5, 0.0, 0.0], &budget).is_err());
    assert!(
        AdmittedValue::new(&spec, "nonfinite", 1, &[f32::INFINITY, 0.0, 0.0], &budget).is_err()
    );
    assert!(
        AdmittedValue::new(
            &spec,
            "budget",
            1,
            &[1.0, 0.0, 0.0],
            &resources::ResourceBudget::fixed(1).unwrap()
        )
        .is_err()
    );
    assert_eq!(budget.reserved(), 0);
}
