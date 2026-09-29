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

#[test]
fn attachment_buffers_share_budget_and_ambiguity_owns_its_reservation() {
    use lctx_model::domain::{*,attachment::*,source::*,input::*};
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(),"x.py".into(),b"x").unwrap();
    let rows: Vec<_> = (0..3).map(|i| Occurrence { source: source.id(),start: 0,end: 1,
        syntax_kind: SyntaxKind::ExprName,role: OccurrenceRole::Read,structural_path: vec![i] }).collect();
    let query = AttachmentQuery { source: source.id(),start: 0,end: 1,syntax_kind: SyntaxKind::ExprName,role: OccurrenceRole::Read,structural_path: None };
    let budget = ResourceBudget::fixed(4096).unwrap();
    let blocker = budget.reserve("other stage",4096).unwrap();
    assert!(matches!(OccurrenceIndex::new(&rows,budget.clone()),Err(ModelError::Resource { .. })));
    assert_eq!(budget.reserved(),4096); drop(blocker);
    let index = OccurrenceIndex::new(&rows,budget.clone()).unwrap();
    let index_bytes = budget.reserved(); assert!(index_bytes > 0);
    let blocker = budget.reserve("other stage",4096-index_bytes).unwrap();
    assert!(matches!(index.attach(&query,AttachmentBudget::default()),Err(ModelError::Resource { .. })));
    assert_eq!(budget.reserved(),4096); drop(blocker);
    let result = index.attach(&query,AttachmentBudget::default()).unwrap();
    assert!(matches!(result.value(),Attachment::Ambiguous(ids) if ids.len() == 3));
    assert_eq!(budget.reserved(),index_bytes+3*std::mem::size_of::<Id<Occurrence>>());
    drop(index);
    assert_eq!(budget.reserved(),3*std::mem::size_of::<Id<Occurrence>>());
    drop(result); assert_eq!(budget.reserved(),0);
    let duplicate = vec![rows[0].clone(),rows[0].clone()];
    assert!(OccurrenceIndex::new(&duplicate,budget.clone()).is_err()); assert_eq!(budget.reserved(),0);
    // Work refusal is semantic uncertainty; memory refusal is an attempt error. Neither retains scratch.
    let index = OccurrenceIndex::new(&rows,budget.clone()).unwrap();
    let before = budget.reserved();
    let result = index.attach(&query,AttachmentBudget { visited_nodes: 0,alternatives: 3 }).unwrap();
    assert_eq!(result.value(),&Attachment::BudgetExceeded); assert_eq!(budget.reserved(),before);
}
