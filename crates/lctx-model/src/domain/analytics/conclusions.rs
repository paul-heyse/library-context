//! Replay-sealed analytic navigation observations. Exact FCA means the declared finite context.
use super::{build::need, *};
use crate::domain::{
    analysis::{
        self, analytic as owner,
        policy::{EvidenceStatus, FindingKind},
        support::SourceFacts,
    },
    assertion::{Approximation, AssertionQualification},
    attribution::Modality,
    conditions::Diagram,
    normalized::entities::EntityRef,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "analytic_conclusion_sources")]
pub enum ConclusionSource {
    #[model(code = 0)]
    Rank { score: Id<RankScore> },
    #[model(code = 1)]
    Community { community: Id<Community> },
    #[model(code = 2)]
    Concept { concept: Id<Concept> },
    #[model(code = 3)]
    Implication { implication: Id<Implication> },
    #[model(code = 4)]
    Neighbour { neighbour: Id<Neighbour> },
    #[model(code = 5)]
    Document { neighbour: Id<DocumentNeighbour> },
    #[model(code = 6)]
    Label { label: Id<CommunityLabel> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_conclusions", rule = "analytic_result_observation")]
pub struct Conclusion {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key, premise)]
    pub source: Id<ConclusionSource>,
    #[model(key)]
    pub subject: Id<EntityRef>,
    pub invocation: Id<owner::Invocation>,
    pub kind: FindingKind,
    qualification: Id<AssertionQualification>,
    heuristic: bool,
}
impl Conclusion {
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
    pub fn heuristic(&self) -> bool {
        self.heuristic
    }
}
impl analysis::support::sealed::DerivedEvidence for Conclusion {}
impl analysis::support::DerivedEvidence for Conclusion {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: if self.heuristic {
                EvidenceStatus::StatisticallyDerived
            } else {
                EvidenceStatus::StructurallyObserved
            },
            heuristic: self.heuristic,
        }
    }
}
struct Candidate {
    source: ConclusionSource,
    result: Id<TechniqueResult>,
    subject: Id<EntityRef>,
    kind: FindingKind,
    heuristic: bool,
}
impl HeapSize for Candidate {}
pub(super) fn produce(
    d: &build::Data,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut candidates = charged::ChargedVec::default();
    let mut charge = charged::StateCharge::new(b, "analytic-conclusions");
    for r in out.ranks.iter() {
        if out
            .universe
            .iter()
            .any(|u| u.public && u.entity == r.target)
        {
            candidates.push(
                &mut charge,
                Candidate {
                    source: ConclusionSource::Rank { score: r.id() },
                    result: r.result,
                    subject: r.target,
                    kind: FindingKind::Centrality,
                    heuristic: true,
                },
            )?;
        }
    }
    for c in out.communities.iter() {
        for m in out
            .community_members
            .iter()
            .filter(|m| m.community == c.id())
        {
            candidates.push(
                &mut charge,
                Candidate {
                    source: ConclusionSource::Community { community: c.id() },
                    result: c.result,
                    subject: m.entity,
                    kind: FindingKind::Community,
                    heuristic: true,
                },
            )?;
        }
    }
    for c in out.concepts.iter() {
        let scope = need(&out.scopes, c.scope)?;
        for m in out.extents.iter().filter(|m| m.concept == c.id()) {
            candidates.push(
                &mut charge,
                Candidate {
                    source: ConclusionSource::Concept { concept: c.id() },
                    result: scope.result,
                    subject: m.entity,
                    kind: FindingKind::ApplicableCase,
                    heuristic: false,
                },
            )?;
        }
    }
    for i in out.implications.iter() {
        let scope = need(&out.scopes, i.scope)?;
        for entity in out
            .objects
            .iter()
            .filter(|o| o.scope == i.scope)
            .map(|o| o.entity)
        {
            candidates.push(
                &mut charge,
                Candidate {
                    source: ConclusionSource::Implication {
                        implication: i.id(),
                    },
                    result: scope.result,
                    subject: entity,
                    kind: FindingKind::Implication,
                    heuristic: false,
                },
            )?;
        }
    }
    for n in out.document_neighbours.iter() {
        candidates.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Document { neighbour: n.id() },
                result: n.result,
                subject: n.query,
                kind: FindingKind::DocLink,
                heuristic: true,
            },
        )?;
    }
    for label in out.labels.iter() {
        let c = need(&out.communities, label.community)?;
        candidates.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Label { label: label.id() },
                result: c.result,
                subject: c.representative,
                kind: FindingKind::CommunityLabel,
                heuristic: true,
            },
        )?;
    }
    for c in candidates.iter() {
        let r = need(&out.results, c.result)?;
        let frame = need(&out.frames, r.frame)?;
        let sf = need(&d.structural.frames, frame.structural)?;
        let parent = need(&d.structural_invocations, sf.invocation)?;
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: parent.context,
            scope: source::CoverageScope::Input {
                input: parent.input,
            }
            .id(),
            condition: Diagram::always().id(),
            modality: Modality::Candidate,
            approximation: Approximation::Over,
        };
        out.qualifications.insert(q.clone())?;
        out.conclusion_sources.insert(c.source.clone())?;
        out.conclusions.insert(Conclusion {
            frame: r.frame,
            source: c.source.id(),
            subject: c.subject,
            invocation: r.invocation,
            kind: c.kind,
            qualification: q.id(),
            heuristic: c.heuristic,
        })?;
    }
    Ok(())
}
