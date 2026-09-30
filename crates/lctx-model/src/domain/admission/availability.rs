//! Immutable scoped evidence minted only by exact coverage admission.
use super::*;
use crate::domain::stages::{AvailabilityPolicy, InputRequirement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageEvidence {
    pub coverage: Id<ProviderCoverage>,
    pub scope: Id<CoverageScope>,
    pub context: Id<AnalysisContext>,
    pub provider: Option<Id<Provider>>,
    pub family: FactFamily,
    pub availability: Availability,
}
#[derive(Debug)]
pub struct ScopedAvailability {
    evidence: Vec<CoverageEvidence>,
    scopes: BTreeMap<Id<CoverageScope>, CoverageScope>,
    empty: BTreeMap<FactFamily, Availability>,
    _charge: StateCharge,
}
impl PartialEq for ScopedAvailability {
    fn eq(&self, other: &Self) -> bool {
        self.evidence == other.evidence && self.scopes == other.scopes && self.empty == other.empty
    }
}
impl Eq for ScopedAvailability {}
impl ScopedAvailability {
    pub(super) fn validated(rows: &[ProviderCoverage], scopes: &BTreeMap<Id<CoverageScope>, CoverageScope>,
        aggregate: &BTreeMap<FactFamily, Availability>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "scoped-availability");
        charge.grow(rows.len().checked_mul(size_of::<CoverageEvidence>()).ok_or_else(|| refuse("coverage size overflow"))?
            .saturating_add(scopes.len().saturating_mul(size_of::<(Id<CoverageScope>, CoverageScope)>() + 32))
            .saturating_add(aggregate.len().saturating_mul(size_of::<(FactFamily, Availability)>() + 32)))?;
        let mut evidence = rows.iter().map(|row| Ok(CoverageEvidence {
            coverage: row.id(), scope: row.scope, context: row.context, provider: row.provider, family: row.family,
            availability: match row.status {
                CoverageStatus::CompleteUnderStatedModel => Availability::Complete,
                CoverageStatus::Partial => Availability::Partial,
                CoverageStatus::Unavailable => Availability::Unavailable,
                CoverageStatus::NotRequested => Availability::NotRequested,
                CoverageStatus::Failed => return Err(refuse("failed coverage cannot authorize reads")),
            },
        })).collect::<Result<Vec<_>, ModelError>>()?;
        evidence.sort_by_key(|row| row.coverage);
        // An empty expected universe is distinct from missing observations. Only admission,
        // after exact membership validation, can establish this family-level empty result.
        let empty = aggregate.iter().filter(|(family, _)| !rows.iter().any(|r| r.family == **family))
            .map(|(family, value)| (*family, *value)).collect();
        Ok(Self { evidence, scopes: scopes.clone(), empty, _charge: charge })
    }
    pub fn evidence(&self) -> &[CoverageEvidence] { &self.evidence }
    pub fn scope(&self, id: Id<CoverageScope>) -> Option<&CoverageScope> { self.scopes.get(&id) }
    pub fn empty_universe(&self, family: FactFamily) -> Option<Availability> { self.empty.get(&family).copied() }
    pub fn admit(&self, requirement: InputRequirement) -> Result<(), ModelError> {
        let present = self.evidence.iter().any(|r| r.family == requirement.group);
        if !present && !self.empty.contains_key(&requirement.group) {
            return Err(refuse("input group has no validated scoped availability"));
        }
        if requirement.policy == AvailabilityPolicy::RequireComplete
            && (!present || self.evidence.iter().any(|r| r.family == requirement.group && r.availability != Availability::Complete)) {
            return Err(refuse(format!("{:?} input requires complete scoped evidence", requirement.group)));
        }
        Ok(())
    }
}
