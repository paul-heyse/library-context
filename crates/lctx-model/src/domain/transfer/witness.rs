//! Finite Summary occurrences cite earlier proof occurrences, never Summary aggregates.
use super::*;
use crate::domain::{
    calls::{CallTarget, Signature, SignatureEnumerationObservation, SignatureEnumerationSupport},
    declarations::SymbolDeclaration,
    normalized::bindings::CallBindingAttempt,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "summary_transfer_premises", rule = "summary_transfer_premise")]
pub enum SummaryPremise {
    #[model(code = 0)]
    Local {
        #[model(premise)]
        alternative: Id<local::TransferAlternative>,
    },
    #[model(code = 1)]
    Model {
        #[model(premise)]
        alternative: Id<model::TransferAlternative>,
    },
    #[model(code = 2)]
    Witness {
        #[model(premise)]
        witness: Id<SummaryWitness>,
    },
    #[model(code = 3)]
    Path {
        #[model(premise)]
        witness: Id<crate::domain::execution::summary_path::SummaryPathWitness>,
    },
    #[model(code = 4)]
    Captured {
        #[model(premise)]
        witness: Id<crate::domain::execution::summary_capture::SummaryCaptureWitness>,
    },
}
impl SummaryPremise {
    pub fn reference(&self) -> RowRef {
        match self {
            Self::Local { alternative } => RowRef::of(*alternative),
            Self::Model { alternative } => RowRef::of(*alternative),
            Self::Witness { witness } => RowRef::of(*witness),
            Self::Path { witness } => RowRef::of(*witness),
            Self::Captured { witness } => RowRef::of(*witness),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="summary_transfer_witnesses",rule="compose_through_call",invariant_refs=crate::domain::composition::composition_invariants_refs)]
pub struct SummaryWitness {
    #[model(key)]
    pub invocation: Id<crate::domain::analysis::summary::AnalysisInvocation>,
    #[model(key)]
    pub transfer: Id<summary::TransferKey>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key, premise)]
    pub caller: Id<SummaryPremise>,
    #[model(key, premise)]
    pub callee: Id<SummaryPremise>,
    #[model(key, premise)]
    pub target: Id<CallTarget>,
    #[model(key, premise)]
    pub signature: Id<Signature>,
    #[model(key, premise)]
    pub callee_declaration: Id<SymbolDeclaration>,
    #[model(key, premise)]
    pub attempt: Id<CallBindingAttempt>,
    /// The exact selected source domain, absent for global-coverage admission.
    #[model(key, premise)]
    pub selected_signature_enumeration: Option<Id<SignatureEnumerationObservation>>,
    /// Native support for that domain; retained as inspection evidence.
    #[model(key, premise)]
    pub selected_signature_enumeration_support: Option<Id<SignatureEnumerationSupport>>,
    #[model(key)]
    pub bindings: ContentHash,
    /// Recomputed from all actual premise supports by the shared conservative policy.
    pub status: crate::domain::analysis::policy::EvidenceStatus,
    pub heuristic: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_transfer_contributions",rule="summary_transfer_contribution",conclusion=alternative)]
pub struct SummaryContribution {
    #[model(key)]
    pub alternative: Id<summary::TransferAlternative>,
    #[model(key, premise)]
    pub witness: Id<SummaryWitness>,
}
/// A checked finite premise with shared admitted condition storage. No aggregate alternative is
/// accepted as its proof identity, even if it has the same descriptor and qualification.
#[derive(Debug, Clone)]
pub struct WitnessBranch {
    branch: TransferBranch<summary::TransferKey>,
    witness: Id<SummaryWitness>,
}
impl WitnessBranch {
    pub fn new(
        witness: &SummaryWitness,
        key: summary::TransferKey,
        q: AssertionQualification,
        condition: Diagram,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if witness.transfer != key.id() || witness.qualification != q.id() {
            return Err(invalid("summary witness branch frame mismatch"));
        }
        Ok(Self {
            branch: TransferBranch::new(key, q, condition, budget)?,
            witness: witness.id(),
        })
    }
}
impl crate::domain::composition::CompositionOperand for WitnessBranch {
    fn descriptor(&self) -> TransferDescriptor {
        self.branch.descriptor()
    }
    fn qualification(&self) -> &AssertionQualification {
        self.branch.qualification()
    }
    fn condition(&self) -> &Diagram {
        self.branch.condition()
    }
    fn premise(&self) -> SummaryPremise {
        SummaryPremise::Witness {
            witness: self.witness,
        }
    }
}

impl crate::domain::analysis::support::sealed::DerivedEvidence for SummaryWitness {}
impl crate::domain::analysis::support::DerivedEvidence for SummaryWitness {
    fn source_facts(&self) -> crate::domain::analysis::support::SourceFacts {
        crate::domain::analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: self.heuristic,
        }
    }
}
/// Evidence metadata comes from a typed immutable source index, never desired output status.
#[derive(Debug)]
pub struct TransferEvidence {
    pub(crate) premise: SummaryPremise,
    pub(crate) facts: Vec<crate::domain::analysis::support::SourceFacts>,
    _charge: Box<dyn Reservation>,
}
impl TransferEvidence {
    pub fn local(
        alternative: &local::TransferAlternative,
        supports: &[local::TransferSupport],
        index: &crate::domain::analysis::local::support::EvidenceIndex,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "transfer_witness_evidence",
            supports
                .len()
                .checked_mul(size_of::<crate::domain::analysis::support::SourceFacts>() + 64)
                .and_then(|n| n.checked_add(size_of::<Self>()))
                .ok_or_else(|| invalid("witness evidence allowance overflow"))?,
        )?;
        let mut facts = Vec::new();
        for support in supports {
            if support.assertion != alternative.id() {
                return Err(invalid("witness evidence changes local alternative"));
            }
            let (_, source) = index.get(support.source)?;
            if source.qualification != alternative.qualification {
                return Err(invalid(
                    "witness source qualification differs from alternative",
                ));
            }
            facts.push(source);
        }
        if facts.is_empty() {
            return Err(invalid("witness premise has no support"));
        }
        Ok(Self {
            premise: SummaryPremise::Local {
                alternative: alternative.id(),
            },
            facts,
            _charge: charge,
        })
    }
    pub fn model(
        alternative: &model::TransferAlternative,
        supports: &[model::TransferSupport],
        index: &crate::domain::analysis::model::support::EvidenceIndex,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "transfer_witness_evidence",
            supports
                .len()
                .checked_mul(size_of::<crate::domain::analysis::support::SourceFacts>() + 64)
                .and_then(|n| n.checked_add(size_of::<Self>()))
                .ok_or_else(|| invalid("witness evidence allowance overflow"))?,
        )?;
        let mut facts = Vec::new();
        for support in supports {
            if support.assertion != alternative.id() {
                return Err(invalid("witness evidence changes model alternative"));
            }
            let (_, source) = index.get(support.source)?;
            if source.qualification != alternative.qualification {
                return Err(invalid(
                    "witness source qualification differs from alternative",
                ));
            }
            facts.push(source);
        }
        if facts.is_empty() {
            return Err(invalid("witness premise has no support"));
        }
        Ok(Self {
            premise: SummaryPremise::Model {
                alternative: alternative.id(),
            },
            facts,
            _charge: charge,
        })
    }
    pub fn path(
        row: &crate::domain::execution::summary_path::SummaryPathWitness,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "transfer_witness_evidence",
            size_of::<Self>() + size_of::<crate::domain::analysis::support::SourceFacts>(),
        )?;
        Ok(Self {
            premise: SummaryPremise::Path { witness: row.id() },
            facts: vec![crate::domain::analysis::support::DerivedEvidence::source_facts(row)],
            _charge: charge,
        })
    }
    pub fn captured(
        row: &crate::domain::execution::summary_capture::SummaryCaptureWitness,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "captured_transfer_evidence",
            size_of::<Self>() + size_of::<crate::domain::analysis::support::SourceFacts>(),
        )?;
        Ok(Self {
            premise: SummaryPremise::Captured { witness: row.id() },
            facts: vec![crate::domain::analysis::support::DerivedEvidence::source_facts(row)],
            _charge: charge,
        })
    }
    pub fn witness(row: &SummaryWitness, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "transfer_witness_evidence",
            size_of::<Self>() + size_of::<crate::domain::analysis::support::SourceFacts>(),
        )?;
        Ok(Self {
            premise: SummaryPremise::Witness { witness: row.id() },
            facts: vec![crate::domain::analysis::support::DerivedEvidence::source_facts(row)],
            _charge: charge,
        })
    }
}
