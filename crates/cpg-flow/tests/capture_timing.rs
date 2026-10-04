//! Actual public enclosing_snapshot API characterization; no fixtures are executed.
use cpg_flow::{
    CaptureOrigin, Input, RuntimeBindings, RuntimeContext, SnapshotBinding, SnapshotState,
};
fn flow(source: &str) -> cpg_flow::ModuleFlow {
    cpg_flow::index(
        &[Input {
            path: "capture.py".into(),
            text: source.into(),
            runtime: RuntimeBindings::default(),
        }],
        &RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".into(),
        },
    )
    .pop()
    .unwrap()
}
#[test]
fn capture_timing_preserves_lazy_candidates_and_definition_owner() {
    let source = include_str!("../../../fixtures/python/stable_capture_shapes/cases.py");
    let f = flow(&source[..source.find("def nonlocal_write").unwrap()]);
    assert!(f.error.is_none(), "{:?}", f.error);
    let inner_read = source.find("return value").unwrap() + 7;
    let snapshot = f
        .captures
        .iter()
        .find(|c| f.uses[c.use_ix as usize].span.start == inner_read as u32)
        .expect("actual captured read");
    assert!(snapshot.lazy, "function execution is lazy in ty");
    assert_eq!(snapshot.origin, CaptureOrigin::OuterLocal);
    assert_eq!(snapshot.state, SnapshotState::FoundBindings);
    assert_eq!(snapshot.candidates.len(), 1);
    let SnapshotBinding::Bound(index) = snapshot.candidates[0].binding else {
        panic!("expected formal candidate: {:?}", snapshot.candidates)
    };
    let definition = &f.defs[index as usize];
    assert_eq!(definition.scope, snapshot.enclosing);
    assert_ne!(definition.scope, f.uses[snapshot.use_ix as usize].scope);
    assert_eq!(definition.kind, cpg_flow::BindingKind::Parameter);
    let combined = flow(source);
    assert!(
        combined
            .captures
            .iter()
            .any(|c| c.origin == CaptureOrigin::Global)
    );
    assert!(
        combined
            .captures
            .iter()
            .any(|c| c.origin == CaptureOrigin::Nonlocal)
    );
    let invalidated = combined
        .captures
        .iter()
        .find(|c| combined.uses[c.use_ix as usize].span.start == inner_read as u32)
        .unwrap();
    assert_eq!(
        invalidated.state,
        SnapshotState::NoLongerInEagerContext,
        "upstream module-wide same-name nonlocal sweep remains explicit"
    );
    let mutation_start = source.find("def mutation").unwrap() as u32;
    let mutation_end = source.find("def call_before_assignment").unwrap() as u32;
    let mutation = f
        .captures
        .iter()
        .find(|c| {
            let span = f.uses[c.use_ix as usize].span;
            span.start > mutation_start && span.start < mutation_end
        })
        .unwrap();
    assert!(
        mutation.candidates.len() > 1,
        "all competing writes remain candidates: {:?}",
        mutation.candidates
    );
}
#[test]
fn capture_timing_eager_class_and_missing_lazy_snapshot_are_distinct() {
    let f = flow(
        "def outer(value):\n    class C:\n        result = value\n    def inner():\n        return absent\n    return C\n",
    );
    assert!(f.error.is_none());
    let eager = f.captures.iter().find(|c| !c.lazy).expect("class snapshot");
    assert_eq!(eager.state, SnapshotState::FoundBindings);
    assert_eq!(eager.origin, CaptureOrigin::OuterLocal);
    let absent = f
        .captures
        .iter()
        .find(|c| f.uses[c.use_ix as usize].place == "absent")
        .expect("located missing global snapshot");
    assert_eq!(absent.state, SnapshotState::NoLongerInEagerContext);
}
