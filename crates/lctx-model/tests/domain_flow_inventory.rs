#[path="fixtures/flow_inventory.rs"]
mod fixture;
use lctx_model::domain::{flow::*,flow_inventory::*,resources::ResourceBudget,*};
#[test]
fn inventory_replays_complete_and_incomplete_sets_and_refuses_forgery() {
    for incomplete in [false,true] {
        let mut f=fixture::Fixture::new(incomplete);f.check().unwrap();assert_eq!(f.inventory.complete,!incomplete);
        let budget=ResourceBudget::fixed(1<<20).unwrap();
        let explanation=explain(f.flow.use_.id(),&f.inventory,&f.candidates,&f.members,&budget).unwrap();assert_eq!(explanation.candidates.len(),f.inventory.native_count as usize);drop(explanation);assert_eq!(budget.reserved(),0);
        f.flow.base.put(Vec::<FlowUseInventoryMember>::new());assert!(f.check().is_err());f.flow.base.put(f.members.clone());
        f.flow.base.put(Vec::<FlowReachingSupport>::new());assert!(f.check().is_err());f.flow.base.put(vec![f.flow.reaching_support.clone()]);
        let mut wrong=f.candidates.clone();wrong[0].kind=FlowCandidateKind::Deleted;f.flow.base.put(wrong);assert!(f.check().is_err());f.flow.base.put(f.candidates.clone());
        let mut wrong=f.inventory.clone();wrong.complete=!wrong.complete;let id=wrong.id();
        let altered_candidates=f.candidates.iter().cloned().map(|mut c|{c.inventory=id;c}).collect::<Vec<_>>();let altered_members=f.members.iter().cloned().map(|mut m|{m.inventory=id;m}).collect::<Vec<_>>();
        let original_support=f.flow.base.rows::<FlowUseInventorySupport>()[0].clone();let mut altered_support=original_support.clone();altered_support.assertion=id;
        f.flow.base.put(vec![wrong]);f.flow.base.put(altered_candidates);f.flow.base.put(altered_members);f.flow.base.put(vec![altered_support]);assert!(f.check().is_err());
        f.flow.base.put(vec![f.inventory.clone()]);f.flow.base.put(f.candidates.clone());f.flow.base.put(f.members.clone());f.flow.base.put(vec![original_support]);
        let mut view=f.flow.base.rows::<FlowSourceViewObservation>()[0].clone();view.source=f.flow.base.foreign.source;f.flow.base.put(vec![view]);assert!(f.check().is_err());
    }
}
#[test]
fn missing_hidden_boundary_and_partial_explanation_never_certify_closure() {
    let mut f=fixture::Fixture::new(true);f.check().unwrap();
    let budget=ResourceBudget::fixed(1<<20).unwrap();
    assert!(explain(f.flow.use_.id(),&f.inventory,&f.candidates[..1],&f.members,&budget).is_err());
    f.flow.base.put(f.candidates[..1].to_vec());assert!(f.check().is_err());
    f.flow.base.put(f.candidates.clone());f.check().unwrap();
    let tiny=ResourceBudget::fixed(1).unwrap();assert!(explain(f.flow.use_.id(),&f.inventory,&f.candidates,&f.members,&tiny).is_err());assert_eq!(tiny.reserved(),0);
}
#[test]
fn each_enumeration_or_lowering_boundary_blocks_complete_closure() {
    let f=fixture::Fixture::new(false);let base=f.candidates[0].state();
    let variants=[
        CandidateState {pruned:true,mapped_count:0,..base.clone()},
        CandidateState {kind:FlowCandidateKind::LoopHeader,loop_expanded:true,mapped_count:0,..base.clone()},
        CandidateState {unattached:true,mapped_count:0,..base.clone()},
        CandidateState {condition_unavailable:true,mapped_count:0,..base.clone()},
        CandidateState {reachability_lost:true,..base.clone()},
    ];
    for state in variants {
        let members=if state.mapped_count==0 { vec![] } else {vec![(0,f.flow.reaching.id(),f.flow.reaching_support.id())]};
        let (inventory,_,_)=FlowUseInventoryObservation::new(f.inventory.qualification,f.inventory.use_,f.inventory.scope,f.inventory.view,&[state],&members).unwrap();assert!(!inventory.complete);
    }
    let (empty,_,_)=FlowUseInventoryObservation::new(f.inventory.qualification,f.inventory.use_,f.inventory.scope,f.inventory.view,&[],&[]).unwrap();assert!(!empty.complete,"empty enumeration is not a completeness proof");
    let too_many=vec![base;MAX_USE_CANDIDATES+1];assert!(FlowUseInventoryObservation::new(f.inventory.qualification,f.inventory.use_,f.inventory.scope,f.inventory.view,&too_many,&[]).is_err());
}
