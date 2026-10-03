//! Pure generation-qualified exact-input inspection. This module never executes Python.
//! A restricted diagram has request identity; its structural Condition ID is not a stored row.
mod evaluate;
mod ingress;
mod inventory;
mod preparation;
mod scalar;
pub use evaluate::{assess, unexamined};
pub use ingress::{CheckedAtom, NativeContext, NativePath};
pub use inventory::NativeInventory;
pub use preparation::{
    FormalDomain, PreparationInputs, PreparationRows, PreparedNativeSemantics,
    preparation_invariants,
};
pub use scalar::{Assumptions, BuiltinNamespace, ExactScalar};
use serde::{Deserialize, Serialize};

use crate::domain::{
    ContentHash, Id,
    conditions::Condition,
    derivation::RowRef,
    normalized::entities::{EntityRef, ParameterEntity},
    obligation::{ObligationKind, Verdict},
};

/// Section availability is neutral; each unexamined path retains its exact refusal cause.
pub const UNAVAILABLE_REASON: &str = "native_context_unavailable";
pub const MODEL_REVISION: u16 = 1;
/// Native-operation meaning participates in request derivation identity independently of stored
/// Condition identity. Preparation may use this same digest for its consumer invalidation.
pub fn definition() -> ContentHash {
    use crate::domain::Key;
    let mut key = crate::domain::KeySink::new("native exact-input operation definition");
    for source in [
        include_bytes!("scalar.rs").as_slice(),
        include_bytes!("ingress.rs").as_slice(),
        include_bytes!("inventory.rs").as_slice(),
        include_bytes!("preparation.rs").as_slice(),
        include_bytes!("evaluate.rs").as_slice(),
        include_bytes!("../local_theory.rs").as_slice(),
        include_bytes!("../conditions/entry.rs").as_slice(),
        include_bytes!("../conditions/stability.rs").as_slice(),
        include_bytes!("../conditions/rebase.rs").as_slice(),
        include_bytes!("../conditions/kernel.rs").as_slice(),
    ] {
        ContentHash::of(source).encode(&mut key);
    }
    key.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RestrictedResultId(#[schemars(with = "[u8; 32]")] pub ContentHash);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExactOutcome {
    RefutedPathUnderModel,
    CompatibleUnderMayModel,
    Unknown,
    NotAnalyzed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    AdmittedFinitePath,
    CheckedExactScalarRestriction,
    Unexamined,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Work {
    pub atoms_examined: usize,
    pub assignments_applied: usize,
    pub bdd_preflight_pairs: usize,
    pub peak_bdd_nodes: usize,
    pub predicate_comparisons: usize,
}

/// Every limit is operational admission, not evidence that an omitted path is absent.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub atoms: usize,
    pub assignments: usize,
    pub bdd_pairs: usize,
    pub render_terms: usize,
    pub predicate_comparisons: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            atoms: 128,
            assignments: 32,
            bdd_pairs: 1_000_000,
            render_terms: 20,
            predicate_comparisons: 4096,
        }
    }
}

/// Public member/formal resolution is a preparation prerequisite, not name matching here.
pub struct ExactRequest<'a> {
    pub generation: crate::domain::serving::GenerationKey,
    pub owner: Id<EntityRef>,
    pub formal: Id<ParameterEntity>,
    pub value: &'a ExactScalar,
    pub assumptions: Assumptions,
}

#[derive(Debug)]
pub struct Assessment {
    pub qualification:Id<crate::domain::assertion::AssertionQualification>,
    pub path: RowRef,
    pub original_condition: Id<Condition>,
    pub restricted_result: Option<RestrictedResultId>,
    pub verdict: Verdict,
    pub exact: ExactOutcome,
    pub basis: Basis,
    pub assumptions: Assumptions,
    pub proof: Vec<RowRef>,
    pub work: Work,
    pub reason: Option<ObligationKind>,
    /// Original support without an admitted exact assignment, including unsupported atoms.
    pub unexamined: usize,
    pub rendered: Option<crate::domain::conditions::RenderedCondition>,
    pub presentation_truncated: bool,
    /// Optional deletion minimization stopped; the cited refutation is still a valid superset.
    pub proof_truncated: bool,
    _charge: Option<Box<dyn crate::domain::resources::Reservation>>,
}
