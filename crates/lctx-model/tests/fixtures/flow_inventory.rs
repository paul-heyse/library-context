#![allow(
    dead_code,
    reason = "Shared inventory fixture supplies model and PostgreSQL controls"
)]
#[path = "flow.rs"]
mod flow_fixture;
use lctx_model::domain::{flow::*, flow_inventory::*, source::*, *};
pub struct Fixture {
    pub flow: flow_fixture::Fixture,
    pub inventory: FlowUseInventoryObservation,
    pub candidates: Vec<FlowUseCandidate>,
    pub members: Vec<FlowUseInventoryMember>,
}
impl Fixture {
    pub fn new(incomplete: bool) -> Self {
        let mut flow = flow_fixture::Fixture::new();
        let source = flow
            .base
            .rows::<SourceArtifact>()
            .into_iter()
            .find(|s| {
                s.id()
                    == flow
                        .base
                        .rows::<Occurrence>()
                        .into_iter()
                        .find(|o| o.id() == flow.use_.occurrence)
                        .unwrap()
                        .source
            })
            .unwrap();
        let view = FlowSourceViewObservation {
            qualification: flow.reaching.qualification,
            source: source.id(),
            original_content: source.content,
            view_content: source.content,
            byte_len: source.byte_len,
            renamed_type_checking: 0,
        };
        let support = &flow.reaching_support;
        let view_support = FlowSourceViewSupport {
            assertion: view.id(),
            run: support.run,
            surface: support.surface,
            evidence: support.evidence,
            origin: support.origin,
            mode: support.mode,
            fidelity: support.fidelity,
        };
        let scope = flow.base.rows::<FlowUseObservation>()[0].scope;
        let mut states = vec![CandidateState {
            kind: FlowCandidateKind::Bound,
            pruned: false,
            loop_expanded: false,
            unattached: false,
            reachability: Some(flow.reaching.qualification),
            narrowing: Some(flow.reaching.qualification),
            narrowing_unavailable: false,
            narrowing_precision_lost: false,
            condition_unavailable: false,
            reachability_lost: false,
            mapped_count: 1,
        }];
        if incomplete {
            states.push(CandidateState {
                kind: FlowCandidateKind::Deleted,
                pruned: true,
                loop_expanded: false,
                unattached: false,
                reachability: Some(flow.reaching.qualification),
                narrowing: Some(flow.reaching.qualification),
                narrowing_unavailable: false,
                narrowing_precision_lost: false,
                condition_unavailable: false,
                reachability_lost: false,
                mapped_count: 0,
            });
        }
        let (inventory, candidates, members) = FlowUseInventoryObservation::new(
            flow.reaching.qualification,
            flow.use_.id(),
            scope,
            view.id(),
            &states,
            &[(0, flow.reaching.id(), support.id())],
        )
        .unwrap();
        let inventory_support = FlowUseInventorySupport {
            assertion: inventory.id(),
            run: support.run,
            surface: support.surface,
            evidence: support.evidence,
            origin: support.origin,
            mode: support.mode,
            fidelity: support.fidelity,
        };
        flow.base.put(vec![view]);
        flow.base.put(vec![view_support]);
        flow.base.put(vec![inventory.clone()]);
        flow.base.put(vec![inventory_support]);
        flow.base.put(candidates.clone());
        flow.base.put(members.clone());
        Self {
            flow,
            inventory,
            candidates,
            members,
        }
    }
    pub fn check(&self) -> Result<(), ModelError> {
        self.flow
            .base
            .check(&FlowUseInventoryObservation::invariants()[0])
    }
}

impl Fixture {
    pub fn replace_inventory(
        &mut self,
        states: &[CandidateState],
        members: &[(i64, Id<FlowReachingObservation>, Id<FlowReachingSupport>)],
    ) {
        let (inventory, candidates, members) = FlowUseInventoryObservation::new(
            self.inventory.qualification,
            self.inventory.use_,
            self.inventory.scope,
            self.inventory.view,
            states,
            members,
        )
        .unwrap();
        let mut support = self.flow.base.rows::<FlowUseInventorySupport>()[0].clone();
        support.assertion = inventory.id();
        self.flow.base.put(vec![inventory.clone()]);
        self.flow.base.put(vec![support]);
        self.flow.base.put(candidates.clone());
        self.flow.base.put(members.clone());
        self.inventory = inventory;
        self.candidates = candidates;
        self.members = members;
    }
    pub fn bound_unattached() -> Self {
        let mut fixture = Self::new(false);
        let state = CandidateState {
            unattached: true,
            mapped_count: 0,
            ..fixture.candidates[0].state()
        };
        fixture.replace_inventory(&[state], &[]);
        fixture.flow.base.put(Vec::<FlowReachingObservation>::new());
        fixture.flow.base.put(Vec::<FlowReachingSupport>::new());
        let q = fixture
            .flow
            .base
            .rows::<assertion::AssertionQualification>()[0]
            .clone();
        let run = fixture
            .flow
            .base
            .rows::<attribution::ProviderRun>()
            .into_iter()
            .find(|r| r.id() == fixture.flow.reaching_support.run)
            .unwrap();
        fixture.flow.base.put(vec![syntax::SubjectBoundary {
            scope: q.scope,
            provider: run.provider,
            context: q.context,
            family: attribution::FactFamily::Flow,
            subject: Some(fixture.flow.use_.occurrence),
            reason: obligation::ObligationKind::NativeUnavailable,
            detail: Some("native candidate definition unattached".into()),
        }]);
        let coverage = fixture
            .flow
            .base
            .rows::<attribution::ProviderCoverage>()
            .into_iter()
            .map(|mut c| {
                if c.run == Some(run.id()) && c.family == attribution::FactFamily::Flow {
                    c.status = attribution::CoverageStatus::Partial;
                    c.reason = Some(obligation::ObligationKind::NativeUnavailable);
                }
                c
            })
            .collect();
        fixture
            .flow
            .base
            .put::<attribution::ProviderCoverage>(coverage);
        fixture
    }
}
