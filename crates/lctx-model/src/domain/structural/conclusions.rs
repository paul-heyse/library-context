//! Static source observations for S0. Each exact source keeps its own execution conditions.
//! The outer qualification describes a source-graph observation, never executable completion.
use super::{
    build::{invalid, need},
    *,
};
use crate::domain::{
    analysis::{
        self,
        policy::{EvidenceStatus, FindingKind},
        structural as owner,
        support::SourceFacts,
    },
    assertion::{Approximation, AssertionQualification},
    attribution::Modality,
    conditions::Diagram,
    normalized::entities::EntityRef,
    source::CoverageScope,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "structural_conclusion_sources")]
pub enum ConclusionSource {
    #[model(code = 0)]
    Public { candidate: Id<PublicCandidate> },
    #[model(code = 1)]
    Path { path: Id<Path> },
    #[model(code = 2)]
    Unresolved { event: Id<UnresolvedEvent> },
    #[model(code = 3)]
    Stop { traversal: Id<Traversal> },
    #[model(code = 4)]
    Usage { score: Id<UsageScore> },
    #[model(code = 5)]
    Handoff { group: Id<handoffs::Group> },
    #[model(code = 6)]
    Forward { path: Id<controls::ControlPath> },
    #[model(code = 7)]
    Literal {
        argument: Id<controls::LiteralArgument>,
    },
    #[model(code = 8)]
    Raise {
        observation: Id<controls::ConditionalRaise>,
    },
    #[model(code = 9)]
    Unfollowed {
        observation: Id<controls::UnfollowedPath>,
    },
    #[model(code = 10)]
    ControlStop {
        traversal: Id<controls::ControlTraversal>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "structural_conclusions",
    rule = "structural_source_observation"
)]
pub struct Conclusion {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key, premise)]
    pub source: Id<ConclusionSource>,
    pub invocation: Id<owner::Invocation>,
    pub subject: Id<EntityRef>,
    pub kind: FindingKind,
    qualification: Id<AssertionQualification>,
}
impl Conclusion {
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
}
impl analysis::support::sealed::DerivedEvidence for Conclusion {}
impl analysis::support::DerivedEvidence for Conclusion {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: EvidenceStatus::StructurallyObserved,
            heuristic: false,
        }
    }
}
/// This scope is intentionally static. Conditions on each supporting hop remain on PathStep.
pub fn qualification(
    invocation: &owner::Invocation,
    modality: Modality,
    approximation: Approximation,
) -> AssertionQualification {
    AssertionQualification {
        context: invocation.context,
        scope: CoverageScope::Input {
            input: invocation.input,
        }
        .id(),
        condition: Diagram::always().id(),
        modality,
        approximation,
    }
}
pub(super) fn produce(
    out: &mut Output,
    invocation: &owner::Invocation,
    frame: &StructuralFrame,
    data: &build::Data,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut observations = charged::ChargedVec::default();
    let mut charge = charged::StateCharge::new(budget, "structural-conclusions");
    for row in out.public.iter() {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Public {
                    candidate: row.id(),
                },
                subject: row.entity,
                kind: FindingKind::PublicAlias,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Delegation,
            },
        )?;
    }
    for path in out.paths.iter() {
        let reach = need(&out.reaches, path.reach)?;
        let traversal = need(&out.traversals, reach.traversal)?;
        let mut modality = Modality::Definite;
        let mut approximation = Approximation::Exact;
        for step in out.steps.iter().filter(|s| s.path == path.id()) {
            if let StepEvidence::Call {
                qualification,
                modality: m,
                ..
            } = need(&out.evidence, step.evidence)?
            {
                let q = need(&data.projection.qualifications, *qualification)?;
                if q.context != invocation.context {
                    return Err(invalid("structural step has a foreign context"));
                }
                modality = modality.weakest(*m).weakest(q.modality);
                approximation = approximation.join(q.approximation);
            }
        }
        let kind = match reach.kind {
            ReachKind::Direct => FindingKind::DirectDelegation,
            ReachKind::BoundedPath => FindingKind::BoundedDelegationPath,
            _ => FindingKind::ImplementationBoundary,
        };
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Path { path: path.id() },
                subject: traversal.seed,
                kind,
                modality,
                approximation,
                method: analysis::AnalysisMethod::Delegation,
            },
        )?;
    }
    for event in out.unresolved.iter() {
        let t = need(&out.traversals, event.traversal)?;
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Unresolved { event: event.id() },
                subject: t.seed,
                kind: FindingKind::IncompleteResolution,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Delegation,
            },
        )?;
    }
    for row in out.traversals.iter().filter(|r| r.stop.is_some()) {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Stop {
                    traversal: row.id(),
                },
                subject: row.seed,
                kind: FindingKind::TraversalStop,
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                method: analysis::AnalysisMethod::Delegation,
            },
        )?;
    }
    for row in out.usage_scores.iter() {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Usage { score: row.id() },
                subject: row.target,
                kind: FindingKind::DirectUsage,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::DirectUsage,
            },
        )?;
    }
    for row in out.handoff_groups.iter() {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Handoff { group: row.id() },
                subject: row.seed,
                kind: FindingKind::Handoff,
                modality: row.producer_modality.weakest(row.consumer_modality),
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Handoffs,
            },
        )?;
    }
    for row in out.control_paths.iter().filter(|r| r.length > 0) {
        let traversal = need(&out.control_traversals, row.traversal)?;
        let mut modality = Modality::Definite;
        let mut approximation = Approximation::Exact;
        for step in out.control_steps.iter().filter(|s| s.path == row.id()) {
            let flow = need(&out.argument_flows, step.flow)?;
            let q = need(&data.projection.qualifications, flow.qualification)?;
            modality = modality.weakest(flow.modality).weakest(q.modality);
            approximation = approximation.join(q.approximation);
        }
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Forward { path: row.id() },
                subject: traversal.seed,
                kind: FindingKind::Forwarding,
                modality,
                approximation,
                method: analysis::AnalysisMethod::Controls,
            },
        )?;
    }
    for row in out.literal_arguments.iter().filter(|r| {
        out.public
            .iter()
            .any(|c| c.entity == r.caller && c.in_subsystem)
    }) {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Literal { argument: row.id() },
                subject: row.caller,
                kind: FindingKind::TransformedArgument,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Controls,
            },
        )?;
    }
    for row in out.conditional_raises.iter() {
        let path = need(&out.control_paths, row.path)?;
        let traversal = need(&out.control_traversals, path.traversal)?;
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Raise {
                    observation: row.id(),
                },
                subject: traversal.seed,
                kind: FindingKind::ConditionalRaise,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Controls,
            },
        )?;
    }
    for row in out.unfollowed_paths.iter() {
        let path = need(&out.control_paths, row.path)?;
        let traversal = need(&out.control_traversals, path.traversal)?;
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::Unfollowed {
                    observation: row.id(),
                },
                subject: traversal.seed,
                kind: FindingKind::UnfollowedArgument,
                modality: Modality::Candidate,
                approximation: Approximation::Over,
                method: analysis::AnalysisMethod::Controls,
            },
        )?;
    }
    for row in out.control_traversals.iter().filter(|r| r.stop.is_some()) {
        observations.push(
            &mut charge,
            Candidate {
                source: ConclusionSource::ControlStop {
                    traversal: row.id(),
                },
                subject: row.seed,
                kind: FindingKind::TraversalStop,
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                method: analysis::AnalysisMethod::Controls,
            },
        )?;
    }
    for row in observations.iter() {
        let q = qualification(invocation, row.modality, row.approximation);
        out.conclusion_qualifications.insert(q.clone())?;
        out.conclusion_sources.insert(row.source.clone())?;
        out.conclusions.insert(Conclusion {
            frame: frame.id(),
            source: row.source.id(),
            invocation: match row.method {
                analysis::AnalysisMethod::Controls => frame.control_invocation,
                analysis::AnalysisMethod::Handoffs => frame.handoff_invocation,
                analysis::AnalysisMethod::DirectUsage => frame.usage_invocation,
                _ => frame.invocation,
            },
            subject: row.subject,
            kind: row.kind,
            qualification: q.id(),
        })?;
    }
    Ok(())
}
// A charged temporary inventory avoids retaining an unaccounted vector while the result grows.
struct Candidate {
    source: ConclusionSource,
    subject: Id<EntityRef>,
    kind: FindingKind,
    modality: Modality,
    approximation: Approximation,
    method: analysis::AnalysisMethod,
}

impl HeapSize for Candidate {}
