//! Shared publication checks for closed candidate inventories; these never prove capture values.
#[allow(
    dead_code,
    reason = "This inventory control uses a subset of the shared native flow fixture."
)]
#[path = "fixtures/flow.rs"]
mod fixture;
use lctx_model::domain::{
    assertion::*, attribution::*, captures::CaptureTiming, conditions::Diagram, flow_capture::*,
    lexical::*, *,
};
fn check(f: &fixture::Fixture) -> Result<(), ModelError> {
    let invariant = FlowCaptureInventory::invariants().pop().unwrap();
    f.base.check(&invariant)
}
#[test]
fn capture_timing_inventory_checks_every_member_and_candidate_only_qualification() {
    let mut f = fixture::Fixture::new();
    let condition = Diagram::always().id();
    let bound = FlowCaptureTarget::Bound {
        definition: f.definition.id(),
    };
    let unbound = FlowCaptureTarget::Undefined;
    let values = [
        (bound.id(), condition, condition, false),
        (unbound.id(), condition, condition, false),
    ];
    let (inventory, members) = FlowCaptureInventory::new(&values).unwrap();
    f.base.put(vec![inventory.clone()]);
    f.base.put(members.clone());
    f.base.put(vec![bound, unbound]);
    check(&f).unwrap();
    f.base.put(vec![members[0].clone()]);
    assert!(
        check(&f).is_err(),
        "missing competitor cannot become uniqueness"
    );
    f.base.put(members.clone());
    let mut gapped = members.clone();
    gapped[1].ordinal = 2;
    f.base.put(gapped);
    assert!(check(&f).is_err());
    f.base.put(members);
    let existing = f.base.rows::<AssertionQualification>()[0].clone();
    let nested = LexicalScope {
        owner: f.base.resolution.read,
        kind: LexicalScopeKind::Function,
    };
    let enclosing = f.base.rows::<LexicalScope>()[0].id();
    let q = AssertionQualification {
        modality: Modality::Candidate,
        ..existing.clone()
    };
    f.base.put(vec![q.clone()]);
    let snapshot = FlowCaptureTimingObservation {
        qualification: q.id(),
        use_: f.use_.id(),
        nested_scope: nested.id(),
        enclosing_scope: enclosing,
        origin: FlowCaptureOrigin::OuterLocal,
        timing: CaptureTiming::LazySnapshot,
        state: FlowSnapshotState::FoundBindings,
        inventory: Some(inventory.id()),
        constraint: None,
        constraint_precision_lost: false,
    };
    snapshot.validate().unwrap();
    f.base.put(vec![snapshot.clone()]);
    check(&f).unwrap();
    let definite = FlowCaptureTimingObservation {
        qualification: existing.id(),
        ..snapshot
    };
    f.base.put(vec![existing]);
    f.base.put(vec![definite]);
    assert!(check(&f).is_err(), "AssumeBound is never definite");
    let over = vec![values[0]; MAX_CAPTURE_CANDIDATES + 1];
    assert!(FlowCaptureInventory::new(&over).is_err());
}
