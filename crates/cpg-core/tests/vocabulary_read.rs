//! Missing typed input loaders refuse and release their shared reservation.
use cpg_core::consumed_rows::ConsumedInputs;
use lctx_model::domain::{value::Literal,*};
#[test]
fn consumed_inventory_refuses_unloaded_rows_and_releases_its_reservation() {
    use lctx_model::domain::resources::ResourceBudget;
    let small = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        ConsumedInputs::new(vec![ValidationInput::of::<Literal>(&["id"])], &small),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(small.reserved(), 0);
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let inputs =
        ConsumedInputs::new(vec![ValidationInput::of::<Literal>(&["id"])], &budget).unwrap();
    assert!(budget.reserved() > 0);
    assert!(
        inputs.finish("test_reader").is_err(),
        "a missing typed loader must refuse instead of silently omitting consumption"
    );
    assert_eq!(budget.reserved(), 0);
}
