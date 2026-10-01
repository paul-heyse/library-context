//! Conditional outcomes of an entered statement. These are semantic values, not occurrence or
//! reachability claims; the producing owner's immutable proof records carry those premises.
use super::ExactRuntimeException;
use crate::domain::{Id, obligation::ObligationKind, source::Occurrence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOutcome {
    Normal,
    Return {
        site: Id<Occurrence>,
    },
    Raise {
        site: Id<Occurrence>,
        exception: ExactRuntimeException,
    },
    Break {
        site: Id<Occurrence>,
    },
    Continue {
        site: Id<Occurrence>,
    },
}
impl PendingOutcome {
    /// Python retains the pending outcome only when the finalizer completes normally. Any
    /// abrupt finalizer replaces it, including a return replacing an exception or vice versa.
    pub const fn after_finalizer(self, finalizer: Self) -> Self {
        match finalizer {
            Self::Normal => self,
            replacement => replacement,
        }
    }
    pub const fn exception(self) -> Option<ExactRuntimeException> {
        match self {
            Self::Raise { exception, .. } => Some(exception),
            _ => None,
        }
    }
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// Ordered suite composition stops at the first abrupt outcome. A refusal is a boundary,
/// never a normal or empty completion; a skipped later statement contributes no premise.
pub fn ordered_suite(
    outcomes: impl IntoIterator<Item = Result<PendingOutcome, ObligationKind>>,
) -> Result<PendingOutcome, ObligationKind> {
    for outcome in outcomes {
        let outcome = outcome?;
        if !outcome.is_normal() {
            return Ok(outcome);
        }
    }
    Ok(PendingOutcome::Normal)
}

/// The lexical handler lifetime ends before caller continuation. A bound handler name requires
/// an independently proven harmless cleanup; a matching exception alone cannot discharge it.
pub fn after_handler(
    body: Result<PendingOutcome, ObligationKind>,
    name_cleanup: Result<(), ObligationKind>,
) -> Result<PendingOutcome, ObligationKind> {
    let outcome = body?;
    name_cleanup?;
    Ok(outcome)
}
