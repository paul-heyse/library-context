//! Direct official usage counts distinct native call sites, sharing each site among its distinct
//! Usage-policy targets before applying the explicit output selector. Shares are navigation
//! evidence; they never establish target certainty or behavioral absence.
use crate::domain::{
    attribution::AnalysisContext,
    input::{ArtifactUse, InputRevision, SourceRole},
    normalized::{Rows, entities::EntityRef, event_normalization::EventOutput, events::*},
    resources::{Reservation, ResourceBudget},
    source::{Occurrence, SourceArtifact},
    *,
};
use std::collections::{BTreeMap, BTreeSet};
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("direct usage premise is absent"))
}
/// Earlier canonical rows have already passed their normalization owner. This operator preserves
/// those exact policy/alternative references rather than rebuilding policy from target spelling.
pub struct Inputs<'a> {
    pub input: Id<InputRevision>,
    pub context: Id<AnalysisContext>,
    pub events: &'a EventOutput,
    pub targets: &'a Rows<calls::CallTarget>,
    pub artifacts: &'a Rows<SourceArtifact>,
    pub uses: &'a Rows<ArtifactUse>,
    pub occurrences: &'a Rows<Occurrence>,
    pub entities: &'a Rows<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub site: Id<Occurrence>,
    pub targets: Vec<Id<EntityRef>>,
    pub complete: bool,
    pub uncertain: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Evidence {
    pub site: Id<Occurrence>,
    pub event: Id<NormalizedCallEvent>,
    pub policy: Id<CallPolicyAssessment>,
    pub admission: Id<CallPolicyAdmission>,
    pub alternative: Id<NormalizedCallAlternative>,
    pub target: Id<EntityRef>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub target: Id<EntityRef>,
    pub share: FiniteF64,
    pub contributing_sites: u64,
}
pub struct Counts {
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
    sites: Vec<Site>,
    evidence: Vec<Evidence>,
    scores: Vec<Score>,
    _reservation: Box<dyn Reservation>,
}
impl Counts {
    pub fn input(&self) -> Id<InputRevision> {
        self.input
    }
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn sites(&self) -> &[Site] {
        &self.sites
    }
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }
    pub fn scores(&self) -> &[Score] {
        &self.scores
    }
    pub fn has_open_sites(&self) -> bool {
        self.sites.iter().any(|s| !s.complete)
    }
    pub fn has_uncertain_sites(&self) -> bool {
        self.sites.iter().any(|s| s.uncertain)
    }
}
struct Draft {
    targets: BTreeSet<Id<EntityRef>>,
    complete: bool,
    uncertain: bool,
}
impl Inputs<'_> {
    pub fn count(
        &self,
        selected: &[Id<EntityRef>],
        budget: &ResourceBudget,
    ) -> Result<Counts, ModelError> {
        // Upper bounds cover temporary ordered indexes and retained vectors at the same time.
        let bytes = self
            .events
            .events
            .len()
            .checked_mul(size_of::<Site>() + size_of::<Draft>() + 320)
            .and_then(|n| {
                n.checked_add(
                    self.events
                        .admissions
                        .len()
                        .checked_mul(2 * size_of::<Evidence>() + 192)?,
                )
            })
            .and_then(|n| n.checked_add(selected.len().checked_mul(size_of::<Score>() + 128)?))
            .and_then(|n| n.checked_add(size_of::<Counts>() + 4096))
            .ok_or_else(|| invalid("direct usage allocation overflow"))?;
        let reservation = budget.reserve("direct-usage-counts", bytes)?;
        let mut selector = BTreeSet::new();
        for target in selected {
            if !matches!(need(self.entities, *target)?, EntityRef::Callable { .. }) {
                return Err(invalid("direct usage output selector is not a callable"));
            }
            if !selector.insert(*target) {
                return Err(invalid("duplicate direct usage output selector"));
            }
        }
        let mut sites: BTreeMap<Id<Occurrence>, Draft> = BTreeMap::new();
        let mut evidence = Vec::with_capacity(self.events.admissions.len());
        for event in self
            .events
            .events
            .iter()
            .filter(|e| e.context == self.context)
        {
            let occurrence = need(self.occurrences, event.site)?;
            let artifact = need(self.artifacts, occurrence.source)?;
            if artifact.input != self.input
                || !self.uses.iter().any(|u| {
                    u.input == self.input
                        && u.artifact == artifact.id()
                        && matches!(
                            u.role,
                            SourceRole::Example | SourceRole::Test | SourceRole::DocBlock
                        )
                })
            {
                continue;
            }
            let mut policies = self
                .events
                .policy_assessments
                .iter()
                .filter(|p| p.event == event.id() && p.policy == CallPolicy::Usage);
            let policy = policies
                .next()
                .ok_or_else(|| invalid("official usage event has no Usage policy assessment"))?;
            if policies.next().is_some() {
                return Err(invalid(
                    "official usage event has ambiguous policy assessments",
                ));
            }
            let assessment = need(&self.events.assessments, policy.event_assessment)?;
            if assessment.event != event.id() {
                return Err(invalid("usage policy refers to a foreign event assessment"));
            }
            let site = sites.entry(event.site).or_insert_with(|| Draft {
                targets: BTreeSet::new(),
                complete: true,
                uncertain: false,
            });
            site.complete &=
                assessment.complete && !assessment.unresolved && !assessment.disagreement;
            site.uncertain |= !assessment.exact
                || !assessment.unique
                || assessment.dispatch
                || assessment.unresolved
                || assessment.disagreement;
            let mut admitted = 0i64;
            for member in self
                .events
                .admissions
                .iter()
                .filter(|m| m.assessment == policy.id())
            {
                let alternative = need(&self.events.alternatives, member.alternative)?;
                if alternative.event != event.id() {
                    return Err(invalid("usage membership crosses events"));
                }
                admitted = admitted
                    .checked_add(1)
                    .ok_or_else(|| invalid("usage membership count overflow"))?;
                let raw = need(
                    self.targets,
                    need(&self.events.alternative_sources, alternative.source)?.target(),
                )?;
                // Definition-only references remain in the policy evidence but contribute no call.
                if raw.phase == calls::CallPhase::Definition {
                    continue;
                }
                let Some(target) = alternative.entity else {
                    site.complete = false;
                    site.uncertain = true;
                    continue;
                };
                need(self.entities, target)?;
                site.targets.insert(target);
                evidence.push(Evidence {
                    site: event.site,
                    event: event.id(),
                    policy: policy.id(),
                    admission: member.id(),
                    alternative: alternative.id(),
                    target,
                });
            }
            if admitted != policy.admitted {
                return Err(invalid("usage admission membership is incomplete"));
            }
        }
        let mut totals: BTreeMap<Id<EntityRef>, (f64, u64)> = BTreeMap::new();
        for site in sites.values() {
            if site.targets.is_empty() {
                continue;
            }
            let share = 1.0 / site.targets.len() as f64;
            for target in site.targets.iter().filter(|t| selector.contains(t)) {
                let (total, count) = totals.entry(*target).or_default();
                *total += share;
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| invalid("usage site count overflow"))?;
            }
        }
        let scores = totals
            .into_iter()
            .map(|(target, (share, contributing_sites))| {
                Ok(Score {
                    target,
                    share: FiniteF64::new(share)?,
                    contributing_sites,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let sites = sites
            .into_iter()
            .map(|(site, draft)| Site {
                site,
                targets: draft.targets.into_iter().collect(),
                complete: draft.complete,
                uncertain: draft.uncertain,
            })
            .collect();
        evidence.sort_unstable();
        Ok(Counts {
            input: self.input,
            context: self.context,
            sites,
            evidence,
            scores,
            _reservation: reservation,
        })
    }
}
