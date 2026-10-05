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
    /// Validate completed native coverage against an independently constructed obligation set.
    /// The compiler derives expected keys from requested providers and captured analysis roots.
    /// No execution receipt, stage grant or persisted checkpoint is required.
    pub fn from_completed(
        profile: Profile,
        expected: &BTreeSet<super::Expected>,
        rows: &[ProviderCoverage],
        scopes: &BTreeMap<Id<CoverageScope>, CoverageScope>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut observed=BTreeSet::new();
        for row in rows {
            row.validate()?;
            if !scopes.contains_key(&row.scope) || !observed.insert(super::Expected {scope:row.scope,family:row.family,provider:row.provider}) {
                return Err(refuse("completed coverage has missing scope or duplicate obligation"));
            }
            let requested=FACTS_REQUIREMENTS.iter().find(|r|r.family==row.family)
                .is_some_and(|r|r.requested_in.contains(&profile));
            if requested == (row.status==CoverageStatus::NotRequested) {
                return Err(refuse("completed coverage differs from requested profile"));
            }
        }
        if &observed!=expected {return Err(refuse("completed coverage differs from exact expected obligations"));}
        let mut aggregate=BTreeMap::new();
        for requirement in FACTS_REQUIREMENTS {
            let statuses:Vec<_>=rows.iter().filter(|row|row.family==requirement.family).map(|row|row.status).collect();
            let status=if !requirement.requested_in.contains(&profile) {Availability::NotRequested}
                else if statuses.is_empty() {Availability::NoScope}
                else if statuses.iter().all(|s|*s==CoverageStatus::CompleteUnderStatedModel) {Availability::Complete}
                else if statuses.iter().all(|s|*s==CoverageStatus::Unavailable) {Availability::Unavailable}
                else {Availability::Partial};
            if requirement.required && status==Availability::Unavailable {return Err(refuse("required completed coverage is entirely unavailable"));}
            aggregate.insert(requirement.family,status);
        }
        Self::validated(rows,scopes,&aggregate,budget)
    }
    pub(super) fn validated(
        rows: &[ProviderCoverage],
        scopes: &BTreeMap<Id<CoverageScope>, CoverageScope>,
        aggregate: &BTreeMap<FactFamily, Availability>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "scoped-availability");
        charge.grow(
            rows.len()
                .checked_mul(size_of::<CoverageEvidence>())
                .ok_or_else(|| refuse("coverage size overflow"))?
                .saturating_add(
                    scopes
                        .len()
                        .saturating_mul(size_of::<(Id<CoverageScope>, CoverageScope)>() + 32),
                )
                .saturating_add(
                    aggregate
                        .len()
                        .saturating_mul(size_of::<(FactFamily, Availability)>() + 32),
                ),
        )?;
        let mut evidence = rows
            .iter()
            .map(|row| {
                Ok(CoverageEvidence {
                    coverage: row.id(),
                    scope: row.scope,
                    context: row.context,
                    provider: row.provider,
                    family: row.family,
                    availability: match row.status {
                        CoverageStatus::CompleteUnderStatedModel => Availability::Complete,
                        CoverageStatus::Partial => Availability::Partial,
                        CoverageStatus::Unavailable => Availability::Unavailable,
                        CoverageStatus::NotRequested => Availability::NotRequested,
                        CoverageStatus::Failed => {
                            return Err(refuse("failed coverage cannot authorize reads"));
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        evidence.sort_by_key(|row| row.coverage);
        // An empty expected universe is distinct from missing observations. Only admission,
        // after exact membership validation, can establish this family-level empty result.
        let empty = aggregate
            .iter()
            .filter(|(family, _)| !rows.iter().any(|r| r.family == **family))
            .map(|(family, value)| (*family, *value))
            .collect();
        Ok(Self {
            evidence,
            scopes: scopes.clone(),
            empty,
            _charge: charge,
        })
    }
    pub fn evidence(&self) -> &[CoverageEvidence] {
        &self.evidence
    }
    pub fn scope(&self, id: Id<CoverageScope>) -> Option<&CoverageScope> {
        self.scopes.get(&id)
    }
    pub fn empty_universe(&self, family: FactFamily) -> Option<Availability> {
        self.empty.get(&family).copied()
    }
    pub fn admit(&self, requirement: InputRequirement) -> Result<(), ModelError> {
        let present = self.evidence.iter().any(|r| r.family == requirement.group);
        if !present && !self.empty.contains_key(&requirement.group) {
            return Err(refuse("input group has no validated scoped availability"));
        }
        if requirement.policy == AvailabilityPolicy::RequireComplete
            && (!present
                || self.evidence.iter().any(|r| {
                    r.family == requirement.group && r.availability != Availability::Complete
                }))
        {
            return Err(refuse(format!(
                "{:?} input requires complete scoped evidence",
                requirement.group
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;
    use crate::domain::{
        attribution::{AnalysisContext, Provider},
        normalized::{
            Rows,
            coverage::{Capability, EvidenceAvailability, scoped_outcome},
        },
    };
    #[test]
    fn normalized_empty_observations_keep_unavailable_partial_and_unrequested_scope_premises() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let artifact = SourceArtifact::from_bytes(input.id(), "empty.py".into(), b"").unwrap();
        let artifact_scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let input_scope = CoverageScope::Input { input: input.id() };
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"scope"),
            environment_digest: ContentHash::of(b"scope"),
            lock_digest: None,
        }
        .id();
        let provider = Provider {
            tool: "fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"provider"),
        }
        .id();
        let mut artifacts = Rows::new(&budget);
        artifacts.insert(artifact).unwrap();
        let scopes = BTreeMap::from([
            (artifact_scope.id(), artifact_scope.clone()),
            (input_scope.id(), input_scope.clone()),
        ]);
        for (capability, status, expected) in [
            (
                Capability::Bindings,
                CoverageStatus::Unavailable,
                EvidenceAvailability::Unavailable,
            ),
            (
                Capability::Bindings,
                CoverageStatus::Partial,
                EvidenceAvailability::Partial,
            ),
            (
                Capability::Bindings,
                CoverageStatus::CompleteUnderStatedModel,
                EvidenceAvailability::Complete,
            ),
            (
                Capability::FlowLinks,
                CoverageStatus::NotRequested,
                EvidenceAvailability::NotRequested,
            ),
        ] {
            let rows: Vec<_> = capability
                .families()
                .iter()
                .map(|family| ProviderCoverage {
                    scope: if *family == FactFamily::Artifacts {
                        input_scope.id()
                    } else {
                        artifact_scope.id()
                    },
                    context,
                    provider: if *family == capability.anchor()
                        && status == CoverageStatus::NotRequested
                    {
                        None
                    } else {
                        Some(provider)
                    },
                    family: *family,
                    run: None,
                    status: if *family == capability.anchor() {
                        status
                    } else {
                        CoverageStatus::CompleteUnderStatedModel
                    },
                    reason: None,
                    diagnostic: None,
                })
                .collect();
            let evidence =
                ScopedAvailability::validated(&rows, &scopes, &BTreeMap::new(), &budget).unwrap();
            let outcome = scoped_outcome(
                capability,
                artifact_scope.id(),
                context,
                &evidence,
                &artifacts,
                &budget,
            )
            .unwrap();
            assert_eq!(outcome.availability, expected);
            assert_eq!(outcome.premises.len(), rows.len());
            assert!(rows.iter().all(|r| {
                outcome
                    .premises
                    .iter()
                    .any(|set| set.family == r.family && set.context == r.context)
            }));
            let missing = scoped_outcome(
                capability,
                input_scope.id(),
                context,
                &evidence,
                &artifacts,
                &budget,
            );
            assert!(
                missing.is_err(),
                "an absent artifact-grain anchor never means no observations"
            );
        }
        let rows: Vec<_> = [FactFamily::Artifacts, FactFamily::Syntax, FactFamily::Calls]
            .into_iter()
            .map(|family| ProviderCoverage {
                scope: if family == FactFamily::Artifacts {
                    input_scope.id()
                } else {
                    artifact_scope.id()
                },
                context,
                provider: Some(provider),
                family,
                run: None,
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            })
            .collect();
        let evidence =
            ScopedAvailability::validated(&rows, &scopes, &BTreeMap::new(), &budget).unwrap();
        assert_eq!(
            scoped_outcome(
                Capability::Bindings,
                artifact_scope.id(),
                context,
                &evidence,
                &artifacts,
                &budget
            )
            .unwrap()
            .availability,
            EvidenceAvailability::Partial,
            "missing signature scope cannot promote bindings to complete"
        );
        for missing_family in [FactFamily::Types, FactFamily::Lexical] {
            for capability in [Capability::Callables, Capability::Bindings] {
                let rows: Vec<_> = capability
                    .families()
                    .iter()
                    .map(|family| ProviderCoverage {
                        scope: if *family == FactFamily::Artifacts {
                            input_scope.id()
                        } else {
                            artifact_scope.id()
                        },
                        context,
                        provider: Some(provider),
                        family: *family,
                        run: None,
                        status: if *family == missing_family {
                            CoverageStatus::Unavailable
                        } else {
                            CoverageStatus::CompleteUnderStatedModel
                        },
                        reason: None,
                        diagnostic: None,
                    })
                    .collect();
                let evidence =
                    ScopedAvailability::validated(&rows, &scopes, &BTreeMap::new(), &budget)
                        .unwrap();
                let result = scoped_outcome(
                    capability,
                    artifact_scope.id(),
                    context,
                    &evidence,
                    &artifacts,
                    &budget,
                )
                .unwrap();
                assert_eq!(result.availability, EvidenceAvailability::Partial);
                assert!(
                    result
                        .premises
                        .iter()
                        .any(|set| set.family == missing_family)
                );
            }
        }
        // A complete local caller still depends on body evidence in another source file.
        let foreign = SourceArtifact::from_bytes(input.id(), "target.py".into(), b"").unwrap();
        let foreign_scope = CoverageScope::Artifact {
            artifact: foreign.id(),
        };
        artifacts.insert(foreign).unwrap();
        let mut expanded = scopes.clone();
        expanded.insert(foreign_scope.id(), foreign_scope.clone());
        let mut rows: Vec<_> = Capability::Bindings
            .families()
            .iter()
            .map(|family| ProviderCoverage {
                scope: if *family == FactFamily::Artifacts {
                    input_scope.id()
                } else {
                    artifact_scope.id()
                },
                context,
                provider: Some(provider),
                family: *family,
                run: None,
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            })
            .collect();
        rows.push(ProviderCoverage {
            scope: foreign_scope.id(),
            context,
            provider: Some(provider),
            family: FactFamily::Types,
            run: None,
            status: CoverageStatus::Unavailable,
            reason: None,
            diagnostic: None,
        });
        let evidence =
            ScopedAvailability::validated(&rows, &expanded, &BTreeMap::new(), &budget).unwrap();
        assert_eq!(
            scoped_outcome(
                Capability::Bindings,
                artifact_scope.id(),
                context,
                &evidence,
                &artifacts,
                &budget
            )
            .unwrap()
            .availability,
            EvidenceAvailability::Partial
        );
    }
}
