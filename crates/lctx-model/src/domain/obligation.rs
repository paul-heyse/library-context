//! Shared semantic refusal vocabulary, named budgets, discharge and conservative verdict policy.
use super::{
    assertion::Approximation,
    attribution::{CoverageStatus, Modality},
    conditions::Diagram,
};
use crate::DomainCode;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum ObligationKind {
    NativeUnavailable = 0,
    UnresolvedTarget = 1,
    UnsupportedUnpacking = 2,
    AmbiguousBinding = 3,
    UnsupportedControlFlow = 4,
    ScopeBoundary = 5,
    BudgetReached = 6,
    MissingEvidence = 7,
    NotRequested = 8,
    ProviderDisagreement = 9,
    OutsideProviderModel = 10,
    /// The module parsed with errors; facts come from a recovered tree.
    SyntaxError = 11,
    /// The module's bytes are not UTF-8; nothing was analyzed.
    UndecodableSource = 12,
    /// A function the provider describes that has no `def` of its own: a synthesized member
    /// (a dataclass `__init__`) or a callable class field (`fn = staticmethod(f)`).
    NoSourceDeclaration = 13,
    /// A definition the analyzer's context never binds: a branch it decides statically
    /// (`sys.version_info`, `TYPE_CHECKING`) while a same-name definition is bound.
    UnreachableInContext = 14,
    /// An export whose origin is a variable, not a `def` or `class` (until the lexical family
    /// gives it a binding node; DESIGN §3.2).
    VariableOrigin = 15,
    /// A name- or string-driven access (`getattr` by a non-literal name, `vars()`,
    /// `__dict__`, `importlib`, `exec`/`eval`, a module `__getattr__`) that could reach a place
    /// a negative claim names (ADR-0022 §Verdicts).
    DynamicAccess = 16,
    /// A `self.m(...)` call a subclass may override (ADR-0022 §Verdicts).
    OverrideDispatch = 17,
    /// An operation whose seed declaration the runtime view cannot reach (ADR-0022 §Composed
    /// layers); `unreachable_in_context` is the checker's never-bound.
    RuntimeUnreachable = 18,
    /// A value that reaches a sink only inside a call (its callee, receiver or an argument):
    /// whether the callee's result carries it is a summary's question, Stage 3's (ADR-0022
    /// §Verdicts).
    CallTransfer = 19,
    /// A body the operation's callers may not run: abstract, a stub (only `pass`, `...` or a
    /// docstring), or one that only raises, so an override or caller supplies the behavior
    /// and "never read" cannot be refuted there (ADR-0022 §Verdicts; the Stage 2 end
    /// review's R1).
    AbstractBody = 20,
    /// Finite summary composition reached its maximum cited call-path depth.
    SummaryDepthLimit = 21,
    /// A condition operation exceeded its deterministic BDD pair-work preflight.
    ConditionWorkLimit = 22,
    /// A condition operation exceeded its BDD result-node cap.
    ConditionNodeLimit = 23,
    /// A condition operation exceeded its named-atom cap.
    ConditionAtomLimit = 24,
    /// The recursive summary worklist exhausted its deterministic source/callee pair cap.
    SummaryPairWorkLimit = 25,
    ExpressionDepthLimit = 26,
    ExpressionWorkLimit = 27,
    CompletionDepthLimit = 28,
    CompletionWorkLimit = 29,
    SummaryProofLimit = 30,
    DefaultUnavailable = 31,
    DefaultStabilityUnknown = 32,
    /// Entering a named exception handler binds and later deletes its name. The
    /// associated lifetime/finalization actions have no admitted completion proof.
    HandlerNameCleanup = 33,
    /// More than 128 explicit arguments in a reached-invocation proof request.
    InvocationArgumentLimit = 34,
    ActionTriggerUnavailable = 35,
    ResourceIdentityUnavailable = 36,
    FrameExitCleanup = 37,
    ConditionTransferUnsupported = 38,
    TheoryAssignmentLimit = 39,
    ConflictingProof = 40,
    NoApplicableDomain = 41,
    IncompleteDomain = 42,
    ComparableConflict = 43,
    IncompatibleContexts = 44,
    MissingContext = 45,
    ResourceRefused = 46,
    /// Retained presentation code; cannot weaken a semantic verdict.
    ResponseBudget = 47,
    Approximation = 48,
    /// A callee transfer end is rooted at a parameter variable whose value at that access is
    /// not shown to be the caller's entry value; the body may have rebound it.
    EntryValueUnknown = 49,
    /// The claim rests on a candidate or potential alternative. Only discharge over every
    /// alternative of its site can establish or refute it.
    NonDefiniteAlternative = 50,
    /// A refutation needs complete coverage of the relation's scope; the coverage is partial.
    IncompleteCoverage = 51,
    /// A provider event matched several occurrences at its span; none was chosen.
    AttachmentAmbiguous = 52,
    /// A provider event matched no occurrence exactly: none, or only a containing one.
    AttachmentUnmatched = 53,
    /// A composed output refers to a captured cell without an admitted state mapping.
    CapturedStateUnavailable = 54,
    EmbeddingServiceUnavailable = 55,
    EmbeddingTokenLimit = 56,
    AnalyticTextUnavailable = 57,
    /// Field location is known but allocation, alias and mutation state is not proven.
    HeapFieldStateUnavailable = 58,
}

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
            | K::ResponseBudget
            | K::EmbeddingTokenLimit => C::Budget,
            K::UnresolvedTarget
            | K::AmbiguousBinding
            | K::UnsupportedUnpacking
            | K::ProviderDisagreement
            | K::CallTransfer
            | K::ConflictingProof
            | K::ConditionTransferUnsupported
            | K::NonDefiniteAlternative
            | K::AttachmentAmbiguous
            | K::AttachmentUnmatched => C::Resolution,
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
            | K::FrameExitCleanup
            | K::Approximation
            | K::HeapFieldStateUnavailable
            | K::CapturedStateUnavailable
            | K::EmbeddingServiceUnavailable => C::Model,
            K::MissingEvidence
            | K::SyntaxError
            | K::UndecodableSource
            | K::DefaultUnavailable
            | K::DefaultStabilityUnknown
            | K::EntryValueUnknown
            | K::IncompleteCoverage
            | K::AnalyticTextUnavailable => C::Evidence,
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
/// response names. The order puts the most specific cause first: a stopped proof explains more
/// than an unresolved alternative, which explains more than the model's limits, missing evidence,
/// an inapplicable domain or an unasked question. Presentation-only response truncation is
/// excluded before verdict evaluation.
pub fn priority(kind: ObligationKind) -> (ObligationClass, i16) {
    (kind.class(), kind as i16)
}

/// The first obligation in [`priority`] order.
pub fn first(kinds: impl IntoIterator<Item = ObligationKind>) -> Option<ObligationKind> {
    kinds.into_iter().min_by_key(|k| priority(*k))
}

/// The obligation a kernel budget hit records.
pub fn from_kernel(boundary: super::conditions::KernelBoundary) -> ObligationKind {
    use super::conditions::KernelBoundary as B;
    match boundary {
        B::AtomLimit => ObligationKind::ConditionAtomLimit,
        B::WorkPreflight => ObligationKind::ConditionWorkLimit,
        B::NodeLimit => ObligationKind::ConditionNodeLimit,
        B::TransferUnsupported | B::MalformedGraph => ObligationKind::ConditionTransferUnsupported,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Verdict {
    Established = 0,
    Conditional = 1,
    RefutedUnderModel = 2,
    Unknown = 3,
    NotAnalyzed = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conclusion {
    pub verdict: Verdict,
    pub reason: Option<ObligationKind>,
}
pub struct VerdictInput<'a> {
    pub condition: Option<&'a Diagram>,
    pub open: &'a [ObligationKind],
    pub coverage: CoverageStatus,
    pub approximation: Approximation,
    /// The modality of the alternative the claim rests on.
    pub modality: Modality,
}
/// Positive witnesses can hold under partial coverage, but no approximate or refused proof is
/// promoted. A candidate or potential alternative is never established, conditional or refuted:
/// it is unknown until discharge over all of its site's alternatives. ScopeBoundary is unknown;
/// only an explicitly unrequested question is not analyzed. Display truncation never enters the
/// semantic condition or changes its verdict.
pub fn verdict(input: VerdictInput<'_>) -> Conclusion {
    let alternative =
        (input.modality != Modality::Definite).then_some(ObligationKind::NonDefiniteAlternative);
    let approximation =
        (input.approximation != Approximation::Exact).then_some(ObligationKind::Approximation);
    let coverage = match input.coverage {
        CoverageStatus::NotRequested => Some(ObligationKind::NotRequested),
        CoverageStatus::Unavailable | CoverageStatus::Failed => {
            Some(ObligationKind::NativeUnavailable)
        }
        _ => None,
    };
    let first = first(
        input
            .open
            .iter()
            .copied()
            .filter(|reason| *reason != ObligationKind::ResponseBudget)
            .chain(alternative)
            .chain(approximation)
            .chain(coverage),
    );
    if let Some(reason) = first {
        return Conclusion {
            verdict: if reason == ObligationKind::NotRequested {
                Verdict::NotAnalyzed
            } else {
                Verdict::Unknown
            },
            reason: Some(reason),
        };
    }
    let Some(condition) = input.condition else {
        return Conclusion {
            verdict: Verdict::Unknown,
            reason: Some(ObligationKind::MissingEvidence),
        };
    };
    if condition.is_false() {
        if input.coverage == CoverageStatus::CompleteUnderStatedModel {
            Conclusion {
                verdict: Verdict::RefutedUnderModel,
                reason: None,
            }
        } else {
            Conclusion {
                verdict: Verdict::Unknown,
                reason: Some(ObligationKind::IncompleteCoverage),
            }
        }
    } else {
        Conclusion {
            verdict: if condition.is_true() {
                Verdict::Established
            } else {
                Verdict::Conditional
            },
            reason: None,
        }
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

/// Whether one origin of a claim is proved or held open, and by what.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing<P> {
    Proved { proof: P },
    Open { obligation: ObligationKind },
}
/// Per-origin proofs and open obligations over one outcome, independent of input order.
#[derive(Clone, Debug)]
pub struct Decisions<O, P> {
    proved: BTreeMap<O, P>,
    open: BTreeMap<O, ObligationKind>,
}
impl<O, P> Default for Decisions<O, P> {
    fn default() -> Self {
        Self {
            proved: BTreeMap::new(),
            open: BTreeMap::new(),
        }
    }
}
impl<O: Ord + Copy, P: Ord + Copy> Decisions<O, P> {
    /// A proof cites an origin. Only an established or conditional proof counts; the lowest proof
    /// identity is kept.
    pub fn proof(&mut self, origin: O, proof: P, verdict: Verdict) {
        if matches!(verdict, Verdict::Established | Verdict::Conditional) {
            self.proved
                .entry(origin)
                .and_modify(|p| *p = (*p).min(proof))
                .or_insert(proof);
        }
    }
    /// An obligation keeps an origin open; the first in [`priority`] order is kept.
    pub fn open(&mut self, origin: O, obligation: ObligationKind) {
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
    pub fn decide(&self, origin: O) -> Standing<P> {
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
pub fn discharge<O: Ord + Copy, P: Ord + Copy>(
    members: &BTreeSet<O>,
    decisions: &Decisions<O, P>,
) -> (bool, Vec<(O, Standing<P>)>) {
    let standings: Vec<_> = members
        .iter()
        .map(|&member| (member, decisions.decide(member)))
        .collect();
    let proved = !standings.is_empty()
        && standings
            .iter()
            .all(|(_, standing)| matches!(standing, Standing::Proved { .. }));
    (proved, standings)
}
