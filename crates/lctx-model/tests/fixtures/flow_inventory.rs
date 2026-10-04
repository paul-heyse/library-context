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
