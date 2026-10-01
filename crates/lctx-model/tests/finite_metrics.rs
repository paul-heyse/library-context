use arrow_array::{Float64Array, RecordBatch};
use lctx_model::{Domain, domain::*};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "test_metrics", semantic_source = include_bytes!("finite_metrics.rs"))]
struct Metric {
    #[model(key)]
    subject: String,
    value: FiniteF64,
    residual: Option<FiniteF64>,
}

#[test]
fn metric_values_are_finite_and_zero_has_one_payload() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteF64::new(value).is_err());
    }
    let plus = FiniteF64::new(0.0).unwrap();
    let minus = FiniteF64::new(-0.0).unwrap();
    assert_eq!(plus, minus);
    assert_eq!(minus.bits(), 0);
    let row = |value| Metric { subject: "same-subject".into(), value, residual: None };
    assert_eq!(row(plus).content_digest(), row(minus).content_digest());
    let changed = row(FiniteF64::new(1.0).unwrap());
    assert_eq!(row(plus).id(), changed.id());
    assert_ne!(row(plus).content_digest(), changed.content_digest());
    let model = ValidatedModel::validate(vec![Relation::of::<Metric>()]).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    assert!(Batch::new(&model, vec![row(plus), changed], &budget).is_err());
}

#[test]
fn finite_float_arrow_roundtrip_and_nonfinite_input_refusal() {
    let model = ValidatedModel::validate(vec![Relation::of::<Metric>()]).unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
    let mut expected = [f64::MIN, -1.5, f64::from_bits(1), -0.0, f64::MAX]
        .into_iter().enumerate().map(|(index, value)| Metric {
            subject: index.to_string(), value: FiniteF64::new(value).unwrap(),
            residual: Some(FiniteF64::new(0.25).unwrap()),
        }).collect::<Vec<_>>();
    expected.sort_by_key(Record::id);
    let batch = Batch::new(&model, expected.clone(), &budget).unwrap();
    assert_eq!(Metric::decode(batch.arrow()).unwrap(), expected);
    assert_eq!(Metric::schema().field(2).data_type(), &arrow_schema::DataType::Float64);
    let mut columns = batch.arrow().columns().to_vec();
    columns[2] = Arc::new(Float64Array::from(vec![-0.0; expected.len()]));
    let negative_zero = RecordBatch::try_new(Metric::schema(), columns).unwrap();
    let canonical = Batch::<Metric>::read(&model, &negative_zero, &budget).unwrap();
    assert!(canonical.rows().iter().all(|row| row.value.bits() == 0));
    let values = canonical.arrow().column(2).as_any().downcast_ref::<Float64Array>().unwrap();
    assert!(values.values().iter().all(|value| value.to_bits() == 0));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut columns = batch.arrow().columns().to_vec();
        columns[2] = Arc::new(Float64Array::from(vec![value; expected.len()]));
        let corrupt = RecordBatch::try_new(Metric::schema(), columns).unwrap();
        assert!(Batch::<Metric>::read(&model, &corrupt, &budget).is_err());
    }
}
