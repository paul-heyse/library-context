//! Enclosing snapshots characterize capture timing, never call-time values or boundness.
use super::{
    assertion::{Approximation, AssertionQualification},
    attribution::{FactFamily, Modality},
    captures::CaptureTiming,
    charged::{ChargedMap, StateCharge},
    conditions::Condition,
    flow::{FlowDefinition, FlowUse},
    lexical::LexicalScope,
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

pub const MAX_CAPTURE_CANDIDATES: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowCaptureOrigin {
    OuterLocal = 0,
    Global = 1,
    Nonlocal = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowSnapshotState {
    FoundBindings = 0,
    FoundConstraint = 1,
    NotFound = 2,
    NoLongerInEagerContext = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(name = "flow_capture_targets")]
pub enum FlowCaptureTarget {
    #[model(code = 0)]
    Bound { definition: Id<FlowDefinition> },
    #[model(code = 1)]
    Undefined,
    #[model(code = 2)]
    Deleted,
    #[model(code = 3)]
    Nested,
    #[model(code = 4)]
    LoopHeader,
    /// A native candidate whose source definition could not be attached; not absence.
    #[model(code = 5)]
    Unattached,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name="flow_capture_inventories",invariant_refs=inventory_invariants_refs)]
pub struct FlowCaptureInventory {
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name="flow_capture_candidates",validate=validate_candidate)]
pub struct FlowCaptureCandidate {
    #[model(key)]
    pub inventory: Id<FlowCaptureInventory>,
    #[model(key)]
    pub ordinal: i64,
    pub target: Id<FlowCaptureTarget>,
    pub condition: Id<Condition>,
    pub narrowing: Id<Condition>,
    pub precision_lost: bool,
}
/// The candidate tuple includes every native state, even false/unbound competitors.
pub type CaptureCandidate = (Id<FlowCaptureTarget>, Id<Condition>, Id<Condition>, bool);
fn inventory_digest(values: &[CaptureCandidate]) -> ContentHash {
    let mut sink = KeySink::new("flow-capture-inventory");
    for (i, (target, condition, narrowing, lost)) in values.iter().enumerate() {
        (i as i64).encode(&mut sink);
        target.encode(&mut sink);
        condition.encode(&mut sink);
        narrowing.encode(&mut sink);
        lost.encode(&mut sink);
    }
    (values.len() as i64).encode(&mut sink);
    sink.finish()
}
impl FlowCaptureInventory {
    pub fn new(
        values: &[CaptureCandidate],
    ) -> Result<(Self, Vec<FlowCaptureCandidate>), ModelError> {
        if values.len() > MAX_CAPTURE_CANDIDATES {
            return Err(invalid("capture inventory work limit"));
        }
        let row = Self {
            members: inventory_digest(values),
        };
        let members = values
            .iter()
            .enumerate()
            .map(
                |(i, (target, condition, narrowing, lost))| FlowCaptureCandidate {
                    inventory: row.id(),
                    ordinal: i as i64,
                    target: *target,
                    condition: *condition,
                    narrowing: *narrowing,
                    precision_lost: *lost,
                },
            )
            .collect();
        Ok((row, members))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name="flow_capture_timing_observations",validate=validate_snapshot)]
#[assertion(support=FlowCaptureTimingSupport,name="flow_capture_timing_supports",family=FactFamily::Flow,subjects(use_,nested_scope,enclosing_scope))]
pub struct FlowCaptureTimingObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub use_: Id<FlowUse>,
    #[model(key)]
    pub nested_scope: Id<LexicalScope>,
    #[model(key)]
    pub enclosing_scope: Id<LexicalScope>,
    #[model(key)]
    pub origin: FlowCaptureOrigin,
    #[model(key)]
    pub timing: CaptureTiming,
    #[model(key)]
    pub state: FlowSnapshotState,
    #[model(key)]
    pub inventory: Option<Id<FlowCaptureInventory>>,
    #[model(key)]
    pub constraint: Option<Id<Condition>>,
    #[model(key)]
    pub constraint_precision_lost: bool,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_candidate(row: &FlowCaptureCandidate) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.ordinal >= MAX_CAPTURE_CANDIDATES as i64 {
        return Err(invalid("capture candidate ordinal outside bound"));
    }
    Ok(())
}
fn validate_snapshot(row: &FlowCaptureTimingObservation) -> Result<(), ModelError> {
    if row.nested_scope == row.enclosing_scope
        || row.timing == CaptureTiming::Unknown
        || (row.state == FlowSnapshotState::FoundBindings) != row.inventory.is_some()
        || (row.state == FlowSnapshotState::FoundConstraint) != row.constraint.is_some()
        || (row.constraint_precision_lost && row.constraint.is_none())
    {
        return Err(invalid("inconsistent enclosing snapshot state"));
    }
    Ok(())
}
pub(crate) fn inventory_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "flow_capture_inventory",
        inputs: vec![
            ValidationInput::of::<FlowCaptureInventory>(&["id"]),
            ValidationInput::of::<FlowCaptureCandidate>(&["inventory", "ordinal"]),
            ValidationInput::of::<FlowCaptureTimingObservation>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InventoryCheck {
                charge: StateCharge::new(budget, "flow_capture_inventory"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct InventoryCheck {
    charge: StateCharge,
    inventories: ChargedMap<Id<FlowCaptureInventory>, ContentHash>,
    current: Option<(Id<FlowCaptureInventory>, Vec<CaptureCandidate>)>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    snapshots: ChargedMap<Id<FlowCaptureTimingObservation>, FlowCaptureTimingObservation>,
}
impl InventoryCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, values)) = self.current.take()
            && self.inventories.remove(&mut self.charge, &id) != Some(inventory_digest(&values))
        {
            return Err(invalid("capture inventory membership differs"));
        }
        Ok(())
    }
}
impl InvariantCheck for InventoryCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == FlowCaptureInventory::NAME {
            for row in FlowCaptureInventory::decode(batch)? {
                self.inventories
                    .insert(&mut self.charge, row.id(), row.members)?;
            }
        } else if relation == FlowCaptureCandidate::NAME {
            for row in FlowCaptureCandidate::decode(batch)? {
                row.validate()?;
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _)| *id != row.inventory)
                {
                    self.flush()?;
                    self.current = Some((row.inventory, Vec::new()));
                }
                let (_, values) = self.current.as_mut().expect("current inventory");
                if row.ordinal != values.len() as i64 {
                    return Err(invalid("capture inventory gaps or duplicate ordinals"));
                }
                self.charge.grow(size_of::<CaptureCandidate>() + 64)?;
                values.push((row.target, row.condition, row.narrowing, row.precision_lost));
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == FlowCaptureTimingObservation::NAME {
            for row in FlowCaptureTimingObservation::decode(batch)? {
                row.validate()?;
                self.snapshots.insert(&mut self.charge, row.id(), row)?;
            }
        } else {
            return Err(invalid("undeclared capture inventory input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .inventories
            .values()
            .any(|digest| *digest != inventory_digest(&[]))
        {
            return Err(invalid("capture inventory missing members"));
        }
        for row in self.snapshots.values() {
            let q = self
                .qualifications
                .get(&row.qualification)
                .ok_or_else(|| invalid("capture qualification missing"))?;
            if row.timing == CaptureTiming::LazySnapshot && q.modality != Modality::Candidate {
                return Err(invalid("lazy AssumeBound snapshot is only a candidate"));
            }
            if row.constraint_precision_lost
                && matches!(q.approximation, Approximation::Exact | Approximation::Under)
            {
                return Err(invalid(
                    "precision-lost snapshot constraint cannot be exact",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn inventory_invariants_refs() -> Vec<&'static str> {
    vec!["flow_capture_inventory"]
}
