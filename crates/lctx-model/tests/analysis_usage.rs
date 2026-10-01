//! Independent arithmetic and nominal-boundary controls over explicit earlier normalized rows.
//! Native normalization and PostgreSQL publication have their own integration controls.
use lctx_model::domain::{
    analysis::usage::*,
    attribution::AnalysisContext,
    calls::*,
    input::{ArtifactUse, InputRevision, SourceRole},
    normalized::{
        Rows, entities::*, event_normalization::EventOutput, events::*, links::LinkReason,
    },
    resources::ResourceBudget,
    source::*,
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
struct Fixture {
    events: EventOutput,
    targets: Rows<CallTarget>,
    artifacts: Rows<SourceArtifact>,
    uses: Rows<ArtifactUse>,
    occurrences: Rows<Occurrence>,
    entities: Rows<EntityRef>,
    site: Id<Occurrence>,
    a: Id<EntityRef>,
    b: Id<EntityRef>,
}
impl Fixture {
    fn new(budget: &ResourceBudget, role: SourceRole) -> Self {
        let mut f = Self {
            events: EventOutput::new(budget),
            targets: Rows::new(budget),
            artifacts: Rows::new(budget),
            uses: Rows::new(budget),
            occurrences: Rows::new(budget),
            entities: Rows::new(budget),
            site: id(0),
            a: id(0),
            b: id(0),
        };
        let artifact =
            SourceArtifact::from_bytes(id::<InputRevision>(1), "usage.py".into(), b"f()\ng()\n")
                .unwrap();
        f.uses
            .insert(ArtifactUse {
                input: artifact.input,
                artifact: artifact.id(),
                role,
            })
            .unwrap();
        f.artifacts.insert(artifact.clone()).unwrap();
        f.site = f
            .occurrences
            .insert(Occurrence {
                source: artifact.id(),
                start: 0,
                end: 3,
                syntax_kind: SyntaxKind::ExprCall,
                role: OccurrenceRole::Call,
                structural_path: vec![0],
            })
            .unwrap();
        f.a = f
            .entities
            .insert(EntityRef::Callable { callable: id(10) })
            .unwrap();
        f.b = f
            .entities
            .insert(EntityRef::Callable { callable: id(11) })
            .unwrap();
        f
    }
    fn add(&mut self, origin: u8, targets: &[(Id<EntityRef>, CallPhase)], complete: bool) {
        let event = NormalizedCallEvent {
            site: self.site,
            origin: id(origin),
            context: id::<AnalysisContext>(2),
            owner: id(3),
        };
        self.events.events.insert(event.clone()).unwrap();
        let assessment = EventAssessment {
            event: event.id(),
            policy: ContentHash::of(b"normalized-policy"),
            members: ContentHash::of(b"fixture-members"),
            complete,
            unique: targets.len() == 1,
            exact: complete,
            known_receivers: true,
            group: Some(PhaseGroup::Call),
            unresolved: !complete,
            dispatch: !complete,
            disagreement: false,
            reason: if complete {
                EventReason::CompleteUnique
            } else {
                EventReason::OpenResolution
            },
        };
        self.events.assessments.insert(assessment.clone()).unwrap();
        let policy = CallPolicyAssessment {
            event: event.id(),
            policy: CallPolicy::Usage,
            event_assessment: assessment.id(),
            admitted: targets.len() as i64,
            members: ContentHash::of(b"fixture-policy-members"),
            reason: PolicyReason::Admitted,
        };
        self.events
            .policy_assessments
            .insert(policy.clone())
            .unwrap();
        for (n, (entity, phase)) in targets.iter().enumerate() {
            let raw = CallTarget {
                qualification: id(4),
                site: event.site,
                origin: event.origin,
                destination: id(30 + n as u8),
                channel: id(5),
                phase: *phase,
                receiver: id(6),
                implicit: false,
                receiver_class: None,
                passing: None,
                class_method: None,
                static_method: None,
            };
            self.targets.insert(raw.clone()).unwrap();
            let source = self
                .events
                .alternative_sources
                .insert(CallAlternativeSource::Native { target: raw.id() })
                .unwrap();
            let alternative = self
                .events
                .alternatives
                .insert(NormalizedCallAlternative {
                    event: event.id(),
                    source,
                    resolution: None,
                    correspondence: None,
                    entity: Some(*entity),
                    status: ResolutionStatus::Resolved,
                    reason: LinkReason::ExplicitIdentity,
                })
                .unwrap();
            self.events
                .admissions
                .insert(CallPolicyAdmission {
                    assessment: policy.id(),
                    alternative,
                })
                .unwrap();
        }
    }
    fn inputs(&self) -> Inputs<'_> {
        Inputs {
            input: id(1),
            context: id(2),
            events: &self.events,
            targets: &self.targets,
            artifacts: &self.artifacts,
            uses: &self.uses,
            occurrences: &self.occurrences,
            entities: &self.entities,
        }
    }
}
#[test]
fn distinct_site_shares_precede_public_filter_and_deduplicate_origin_phase_and_evidence() {
    let budget = ResourceBudget::fixed(1 << 22).unwrap();
    let mut f = Fixture::new(&budget, SourceRole::Example);
    f.add(
        7,
        &[
            (f.a, CallPhase::Call),
            (f.a, CallPhase::PropertyGet),
            (f.b, CallPhase::Call),
        ],
        true,
    );
    f.add(8, &[(f.b, CallPhase::Call)], true);
    let result = f.inputs().count(&[f.a], &budget).unwrap();
    assert_eq!(result.sites().len(), 1);
    assert_eq!(result.sites()[0].targets.len(), 2);
    assert_eq!(result.evidence().len(), 4);
    assert_eq!(
        result.scores(),
        [Score {
            target: f.a,
            share: FiniteF64::new(0.5).unwrap(),
            contributing_sites: 1
        }]
    );
    assert!(!result.has_open_sites());
    assert!(result.has_uncertain_sites());
    let reordered = f.inputs().count(&[f.b, f.a], &budget).unwrap();
    assert!(reordered.scores().iter().all(|s| s.share.get() == 0.5));
    assert_eq!(reordered.evidence(), result.evidence());
    assert_eq!(result.input(), id(1));
    assert_eq!(result.context(), id(2));
    drop(f);
    assert!(budget.reserved() > 0);
    drop(result);
    drop(reordered);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn open_membership_role_and_definition_boundaries_never_become_zero_or_certain_scores() {
    let budget = ResourceBudget::fixed(1 << 22).unwrap();
    let mut f = Fixture::new(&budget, SourceRole::DocBlock);
    f.add(
        7,
        &[(f.a, CallPhase::Call), (f.b, CallPhase::Definition)],
        false,
    );
    let result = f.inputs().count(&[f.a, f.b], &budget).unwrap();
    assert!(result.has_open_sites());
    assert!(result.has_uncertain_sites());
    assert_eq!(result.scores().len(), 1);
    assert_eq!(result.scores()[0].share.get(), 1.0);
    assert_eq!(result.evidence().len(), 1);
    for role in [SourceRole::Release, SourceRole::Dependency] {
        let mut g = Fixture::new(&budget, role);
        g.add(7, &[(g.a, CallPhase::Call)], true);
        let result = g.inputs().count(&[g.a], &budget).unwrap();
        assert!(result.sites().is_empty());
        assert!(result.scores().is_empty());
    }
    drop(result);
    drop(f);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn missing_policy_members_foreign_selectors_and_short_budgets_refuse() {
    let budget = ResourceBudget::fixed(1 << 22).unwrap();
    let mut f = Fixture::new(&budget, SourceRole::Test);
    f.add(7, &[(f.a, CallPhase::Call)], true);
    assert!(f.inputs().count(&[f.a, f.a], &budget).is_err());
    assert!(f.inputs().count(&[id(99)], &budget).is_err());
    assert!(
        f.inputs()
            .count(&[f.a], &ResourceBudget::fixed(1).unwrap())
            .is_err()
    );
    f.events.admissions = Rows::new(&budget);
    assert!(f.inputs().count(&[f.a], &budget).is_err());
    drop(f);
    assert_eq!(budget.reserved(), 0);
}
