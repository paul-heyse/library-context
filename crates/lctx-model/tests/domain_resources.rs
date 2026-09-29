use lctx_model::domain::{ModelError,resources::*};
#[test]
fn reservations_share_capacity_and_failed_resize_preserves_existing_charge() {
    let budget = ResourceBudget::fixed(100).unwrap(); let same = budget.clone();
    assert!(budget.shares_pool(&same));
    let mut producer = budget.reserve("producer",60).unwrap(); let reader = same.reserve("reader",40).unwrap();
    assert_eq!(budget.reserved(),100);
    assert!(matches!(producer.try_resize(61),Err(ModelError::Resource { .. })));
    assert_eq!(producer.size(),60); assert_eq!(budget.reserved(),100);
    drop(reader); producer.try_resize(100).unwrap(); producer.try_resize(20).unwrap();
    assert_eq!(budget.reserved(),20); drop(producer); assert_eq!(budget.reserved(),0);
    assert!(budget.reserve("overflow",usize::MAX).is_err()); assert_eq!(budget.reserved(),0);
    assert!(ResourceBudget::fixed(0).is_err());
}
#[test]
fn concurrent_reservations_cannot_overbook_the_attempt() {
    let budget = ResourceBudget::fixed(100).unwrap();
    let start = std::sync::Arc::new(std::sync::Barrier::new(8));
    let held = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8).map(|_| {
        let budget = budget.clone(); let start = start.clone(); let held = held.clone();
        std::thread::spawn(move || { start.wait(); let allocation = budget.reserve("worker",60); held.wait(); allocation.is_ok() })
    }).collect();
    let successes = threads.into_iter().map(|thread| thread.join().unwrap()).filter(|success| *success).count();
    assert_eq!(successes,1); assert_eq!(budget.reserved(),0);
}
