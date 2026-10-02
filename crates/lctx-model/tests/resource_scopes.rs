use lctx_model::domain::resources::ResourceBudget;
#[test]
fn nested_reservations_share_process_capacity_and_refusals_preserve_charges() {
    let shared = ResourceBudget::fixed(100).unwrap();
    let preparation = ResourceBudget::scoped(&shared, 80).unwrap();
    let request = ResourceBudget::scoped(&shared, 60).unwrap();
    let mut prepared = preparation.reserve("prepared", 70).unwrap();
    assert!(preparation.reserve("prepared", 11).is_err());
    let result = request.reserve("request", 20).unwrap();
    assert!(request.reserve("request", 20).is_err());
    assert_eq!(request.reserved(),20);
    assert_eq!(shared.reserved(),90);
    assert!(prepared.try_resize(81).is_err());
    assert_eq!(prepared.size(),70);
    drop(result);
    prepared.try_resize(80).unwrap();
    assert_eq!(shared.reserved(),80);
    drop(prepared);
    assert_eq!(shared.reserved(),0);
    assert_eq!(preparation.reserved(),0);
}
