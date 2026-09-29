//! Obligations, budgets, verdicts and discharge (DESIGN §15.8).
//!
//! An **obligation** records what an established conclusion still lacks. One codebook unions the
//! former boundary, kernel, theory and selection refusal reasons; one [`priority`] orders them; one
//! [`verdict`] function maps a condition and its open obligations to the five verdicts; one
//! [`discharge`] function decides whether a claim's member origins are all proved. Producers,
//! validators and the native executor share these; none restates them.

use std::collections::{BTreeMap, BTreeSet};

use crate::decl::codebook::Codebook;
use crate::id::Id;

crate::codebook!(
    /// Everything an established conclusion can still lack. Codes 0–37 are the legacy boundary
    /// reasons with their codes and texts unchanged; later codes absorb the kernel, theory and
    /// selection refusals. Append-only.
    ObligationKind = "obligation_kind" {
        NativeUnavailable = 0 => "native_unavailable",
        UnresolvedTarget = 1 => "unresolved_target",
        UnsupportedUnpacking = 2 => "unsupported_unpacking",
        AmbiguousBinding = 3 => "ambiguous_binding",
        UnsupportedControlFlow = 4 => "unsupported_control_flow",
        ScopeBoundary = 5 => "scope_boundary",
        /// An unnamed budget (legacy); new producers name the budget they hit.
        BudgetReached = 6 => "budget_reached",
        MissingEvidence = 7 => "missing_evidence",
        NotRequested = 8 => "not_requested",
        ProviderDisagreement = 9 => "provider_disagreement",
        OutsideProviderModel = 10 => "outside_provider_model",
        SyntaxError = 11 => "syntax_error",
        UndecodableSource = 12 => "undecodable_source",
        NoSourceDeclaration = 13 => "no_source_declaration",
        UnreachableInContext = 14 => "unreachable_in_context",
        VariableOrigin = 15 => "variable_origin",
        DynamicAccess = 16 => "dynamic_access",
        OverrideDispatch = 17 => "override_dispatch",
        RuntimeUnreachable = 18 => "runtime_unreachable",
        /// A value crossing a call with no instantiable summary: an open call (§15.6).
        CallTransfer = 19 => "call_transfer",
        AbstractBody = 20 => "abstract_body",
        SummaryDepthLimit = 21 => "summary_depth_limit",
        ConditionWorkLimit = 22 => "condition_work_limit",
        ConditionNodeLimit = 23 => "condition_node_limit",
        ConditionAtomLimit = 24 => "condition_atom_limit",
        SummaryPairWorkLimit = 25 => "summary_pair_work_limit",
        ExpressionDepthLimit = 26 => "expression_depth_limit",
        ExpressionWorkLimit = 27 => "expression_work_limit",
        CompletionDepthLimit = 28 => "completion_depth_limit",
        CompletionWorkLimit = 29 => "completion_work_limit",
        SummaryProofLimit = 30 => "summary_proof_limit",
        DefaultUnavailable = 31 => "default_unavailable",
        DefaultStabilityUnknown = 32 => "default_stability_unknown",
        HandlerNameCleanup = 33 => "handler_name_cleanup",
        InvocationArgumentLimit = 34 => "invocation_argument_limit",
        ActionTriggerUnavailable = 35 => "action_trigger_unavailable",
        ResourceIdentityUnavailable = 36 => "resource_identity_unavailable",
        FrameExitCleanup = 37 => "frame_exit_cleanup",
        /// A condition could not be moved between atom vocabularies.
        ConditionTransferUnsupported = 38 => "condition_transfer_unsupported",
        /// Exact-input reasoning exceeded its assignment budget.
        TheoryAssignmentLimit = 39 => "theory_assignment_limit",
        /// Two checked proofs assign one atom different truth values.
        ConflictingProof = 40 => "conflicting_proof",
        NoApplicableDomain = 41 => "no_applicable_domain",
        IncompleteDomain = 42 => "incomplete_domain",
        ComparableConflict = 43 => "comparable_conflict",
        IncompatibleContexts = 44 => "incompatible_contexts",
        MissingContext = 45 => "missing_context",
        ResourceRefused = 46 => "resource_refused",
        /// A condition or proof was cut by its rendering or response budget.
        ResponseBudget = 47 => "response_budget",
    }
);

/// The class of an obligation, which orders it before its code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObligationClass {
    /// A named proof budget stopped the derivation: distinguishable from dominance and from
    /// missing evidence (review F07).
    Budget,
    /// The conclusion crosses something no relation resolves: an unknown or ambiguous callee,
    /// an unbound argument, a provider disagreement, an open call.
    Resolution,
    /// The conclusion depends on behavior outside the analysis model.
    Model,
    /// A fact the conclusion needs is absent or unreadable.
    Evidence,
    /// A requirement's domain or context cannot be applied.
    Selection,
    /// The question was not asked or lies outside the analyzed scope.
    Scope,
}

impl ObligationKind {
    pub fn class(self) -> ObligationClass {
        use ObligationClass as C;
        use ObligationKind as K;
        match self {
            K::BudgetReached
            | K::SummaryDepthLimit
            | K::ConditionWorkLimit
            | K::ConditionNodeLimit
            | K::ConditionAtomLimit
            | K::SummaryPairWorkLimit
            | K::ExpressionDepthLimit
            | K::ExpressionWorkLimit
            | K::CompletionDepthLimit
            | K::CompletionWorkLimit
            | K::SummaryProofLimit
            | K::InvocationArgumentLimit
            | K::TheoryAssignmentLimit
            | K::ResponseBudget => C::Budget,
            K::UnresolvedTarget
            | K::AmbiguousBinding
            | K::UnsupportedUnpacking
            | K::ProviderDisagreement
            | K::CallTransfer
            | K::ConflictingProof
            | K::ConditionTransferUnsupported => C::Resolution,
            K::NativeUnavailable
            | K::UnsupportedControlFlow
            | K::OutsideProviderModel
            | K::NoSourceDeclaration
            | K::UnreachableInContext
            | K::VariableOrigin
            | K::DynamicAccess
            | K::OverrideDispatch
            | K::RuntimeUnreachable
            | K::AbstractBody
            | K::HandlerNameCleanup
            | K::ActionTriggerUnavailable
            | K::ResourceIdentityUnavailable
            | K::FrameExitCleanup => C::Model,
            K::MissingEvidence
            | K::SyntaxError
            | K::UndecodableSource
            | K::DefaultUnavailable
            | K::DefaultStabilityUnknown => C::Evidence,
            K::NoApplicableDomain
            | K::IncompleteDomain
            | K::ComparableConflict
            | K::IncompatibleContexts
            | K::MissingContext
            | K::ResourceRefused => C::Selection,
            K::ScopeBoundary | K::NotRequested => C::Scope,
        }
    }
}

/// The one obligation order (DESIGN §15.8): by class — budget, resolution, model, evidence,
/// selection, scope — then by code. When several obligations are open, the first is the reason a
/// response names. The four legacy rankings are declared quirks of their phase-4 adapters.
pub fn priority(kind: ObligationKind) -> (ObligationClass, i16) {
    (kind.class(), kind.code())
}

/// The first obligation in [`priority`] order.
pub fn first(kinds: impl IntoIterator<Item = ObligationKind>) -> Option<ObligationKind> {
    kinds.into_iter().min_by_key(|k| priority(*k))
}

/// The obligation a kernel budget hit records.
pub fn from_kernel(boundary: crate::condition::KernelBoundary) -> ObligationKind {
    use crate::condition::KernelBoundary as B;
    match boundary {
        B::AtomLimit => ObligationKind::ConditionAtomLimit,
        B::WorkPreflight => ObligationKind::ConditionWorkLimit,
        B::NodeLimit => ObligationKind::ConditionNodeLimit,
        B::TransferUnsupported => ObligationKind::ConditionTransferUnsupported,
    }
}

/// The obligation a primitive-theory refusal records.
pub fn from_theory(boundary: crate::condition::theory::TheoryBoundary) -> ObligationKind {
    use crate::condition::theory::TheoryBoundary as B;
    match boundary {
        B::Kernel(k) => from_kernel(k),
        B::AssignmentBudget => ObligationKind::TheoryAssignmentLimit,
        B::ConflictingProof => ObligationKind::ConflictingProof,
    }
}

/// The legacy mapping (cutover plan §3.4): a legacy boundary-reason code is the obligation of the
/// same code; a legacy selection reason maps by name. `Witness`, `Counterexample` and
/// `ClosedAbsence` are outcomes, not obligations.
pub mod legacy {
    use super::ObligationKind;
    use crate::decl::codebook::Codebook;

    /// A legacy `boundary_reason` code.
    pub fn from_boundary_code(code: i16) -> Option<ObligationKind> {
        (0..=37).contains(&code).then(|| ObligationKind::from_code(code)).flatten()
    }

    /// A legacy kernel boundary code (`KernelBoundary::code` before v2).
    pub fn from_kernel_code(code: &str) -> Option<ObligationKind> {
        Some(match code {
            "source_over_budget" => ObligationKind::BudgetReached,
            "atom_limit" => ObligationKind::ConditionAtomLimit,
            "work_preflight" => ObligationKind::ConditionWorkLimit,
            "node_limit" => ObligationKind::ConditionNodeLimit,
            "transfer_unsupported" | "atom_name_collision" => {
                ObligationKind::ConditionTransferUnsupported
            }
            _ => return None,
        })
    }

    /// A legacy wire `SelectionReason` variant name.
    pub fn from_selection(reason: &str) -> Option<ObligationKind> {
        Some(match reason {
            "NoApplicableDomain" => ObligationKind::NoApplicableDomain,
            "IncompleteDomain" => ObligationKind::IncompleteDomain,
            "ComparableConflict" => ObligationKind::ComparableConflict,
            "IncompatibleContexts" => ObligationKind::IncompatibleContexts,
            "MissingContext" => ObligationKind::MissingContext,
            "ResourceRefused" => ObligationKind::ResourceRefused,
            _ => return None,
        })
    }
}

/// A named, deterministic budget. Charging is by declared units; exhaustion is the budget's own
/// obligation, never `false` and never a silent cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub obligation: ObligationKind,
    pub limit: u64,
}

impl Budget {
    pub const fn new(obligation: ObligationKind, limit: u64) -> Self {
        Self { obligation, limit }
    }

    pub fn meter(self) -> Meter {
        Meter {
            budget: self,
            used: 0,
        }
    }
}

/// A budget being charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meter {
    budget: Budget,
    used: u64,
}

impl Meter {
    /// Charge `units`, or return the budget's obligation when they do not fit.
    pub fn charge(&mut self, units: u64) -> Result<(), ObligationKind> {
        match self.used.checked_add(units) {
            Some(total) if total <= self.budget.limit => {
                self.used = total;
                Ok(())
            }
            _ => Err(self.budget.obligation),
        }
    }

    pub fn used(&self) -> u64 {
        self.used
    }
}

crate::codebook!(
    /// The five verdicts (DESIGN §15.8). Never a null.
    Verdict = "verdict" {
        /// Holds under the model with no open obligation.
        Established = 0 => "established",
        /// Holds under a stated, satisfiable condition, with no open obligation.
        Conditional = 1 => "conditional",
        /// Refuted within complete coverage of the relation's scope, exactly.
        RefutedUnderModel = 2 => "refuted_under_model",
        /// An obligation is open; its first reason is named.
        Unknown = 3 => "unknown",
        /// Not asked, or outside the analyzed scope.
        NotAnalyzed = 4 => "not_analyzed",
    }
);

/// A conclusion's condition as the kernel decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionState {
    Always,
    /// Satisfiable but not always: the condition id.
    When(Id),
    Never,
}

/// The inputs of the one verdict function.
#[derive(Clone, Debug)]
pub struct VerdictInput<'a> {
    pub condition: ConditionState,
    pub open: &'a [ObligationKind],
    /// Coverage of the relation's scope is complete, so absence within it is meaningful.
    pub coverage_complete: bool,
    /// A may-path crossed a provider ambiguity or a declared runtime assumption.
    pub approximated: bool,
}

/// The one verdict function (DESIGN §15.8).
///
/// - An open scope obligation alone is **not analysed**; any other open obligation is
///   **unknown**, named by [`first`] (a budget hit included: a cut proof is not a negative).
/// - With nothing open: always is **established**, a satisfiable condition **conditional**.
/// - A never-holding condition is **refuted under model** only with complete coverage and no
///   approximation; otherwise it stays **unknown**.
pub fn verdict(input: &VerdictInput<'_>) -> (Verdict, Option<ObligationKind>) {
    if let Some(reason) = first(input.open.iter().copied()) {
        let all_scope = input
            .open
            .iter()
            .all(|k| k.class() == ObligationClass::Scope);
        return if all_scope {
            (Verdict::NotAnalyzed, Some(reason))
        } else {
            (Verdict::Unknown, Some(reason))
        };
    }
    match input.condition {
        ConditionState::Always => (Verdict::Established, None),
        ConditionState::When(_) => (Verdict::Conditional, None),
        ConditionState::Never if input.coverage_complete && !input.approximated => {
            (Verdict::RefutedUnderModel, None)
        }
        ConditionState::Never => (Verdict::Unknown, Some(ObligationKind::IncompleteDomain)),
    }
}

/// One origin's standing under discharge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Proved { proof: Id },
    Open { obligation: ObligationKind },
}

/// Per-origin proofs and open obligations over one outcome, independent of input order.
#[derive(Clone, Debug, Default)]
pub struct Decisions {
    proved: BTreeMap<Id, Id>,
    open: BTreeMap<Id, ObligationKind>,
}

impl Decisions {
    /// A proof cites an origin. Only an established or conditional proof counts; the lowest
    /// proof id is kept.
    pub fn proof(&mut self, origin: Id, proof: Id, verdict: Verdict) {
        if matches!(verdict, Verdict::Established | Verdict::Conditional) {
            self.proved
                .entry(origin)
                .and_modify(|p| *p = (*p).min(proof))
                .or_insert(proof);
        }
    }

    /// An obligation keeps an origin open; the first in [`priority`] order is kept.
    pub fn open(&mut self, origin: Id, obligation: ObligationKind) {
        self.open
            .entry(origin)
            .and_modify(|o| {
                if priority(obligation) < priority(*o) {
                    *o = obligation;
                }
            })
            .or_insert(obligation);
    }

    /// An origin is proved only when a proof cites it **and** no obligation keeps it open. An
    /// origin nothing considered stays open as an open call.
    pub fn decide(&self, origin: Id) -> Standing {
        if let Some(&obligation) = self.open.get(&origin) {
            return Standing::Open { obligation };
        }
        match self.proved.get(&origin) {
            Some(&proof) => Standing::Proved { proof },
            None => Standing::Open {
                obligation: ObligationKind::CallTransfer,
            },
        }
    }
}

/// The one discharge-validity function: a claim is discharged exactly when it has members and
/// every member is proved. An open member is never a negative conclusion.
pub fn discharge(members: &BTreeSet<Id>, decisions: &Decisions) -> (bool, Vec<(Id, Standing)>) {
    let standings: Vec<(Id, Standing)> = members.iter().map(|&m| (m, decisions.decide(m))).collect();
    let proved = !standings.is_empty()
        && standings
            .iter()
            .all(|(_, s)| matches!(s, Standing::Proved { .. }));
    (proved, standings)
}
