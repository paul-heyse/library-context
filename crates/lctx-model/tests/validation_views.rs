#[path="fixtures/validation_views.rs"] mod fixture;
use fixture::*;
use lctx_model::domain::{*,stages::*,value::Literal,resources::ResourceBudget,memory::MemoryGeneration};

fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
    use std::task::{Context, Poll, Waker};
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory sink unexpectedly pending"),
    }
}
#[test]
fn memory_final_replay_keeps_both_vocabulary_frames_and_their_reservations() {
    ready(async {
    let model=ValidatedModel::validate(vec![Relation::of::<Literal>(),Relation::of::<Probe>(),Relation::of::<ReadProbe>()]).unwrap();let schedule=schedule(&model);
    let budget=ResourceBudget::fixed(1<<24).unwrap();let mut execution=schedule.execute();let memory=MemoryGeneration::bind(&model,&budget,&mut execution).unwrap();
    for (name,row) in [("facts",Literal::None),("later",Literal::Bool {value:true})] {let batch=Batch::new(&model,vec![row],&budget).unwrap();let mut stage=execution.begin(name).unwrap();stage.write::<Literal,_>(async |permit|memory.copy(permit,&batch).await).await.unwrap();stage.complete(&memory,ProviderOutcome::Complete).await.unwrap();}
    let mut stage=execution.begin("probe").unwrap();let batch=Batch::new(&model,vec![Probe {marker:true}],&budget).unwrap();stage.write::<Probe,_>(async |permit|memory.copy(permit,&batch).await).await.unwrap();stage.complete(&memory,ProviderOutcome::Complete).await.unwrap();drop(batch);
    let mut stage=execution.begin("reader").unwrap();let batch=Batch::new(&model,vec![ReadProbe {marker:true}],&budget).unwrap();stage.write::<ReadProbe,_>(async |permit|memory.copy(permit,&batch).await).await.unwrap();stage.complete(&memory,ProviderOutcome::Complete).await.unwrap();drop(batch);
    execution.finish().unwrap();memory.validate(&model,&budget).unwrap();assert!(budget.reserved()>0);drop(memory);assert_eq!(budget.reserved(),0);
    });
}

#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="duplicate_validation_frames",invariants=duplicate_frames,semantic_source=include_bytes!("validation_views.rs"))]
struct DuplicateFrames { #[model(key)] marker:bool }
fn duplicate_frames()->Vec<Invariant> {
    vec![Invariant {name:"repeated_frame",inputs:vec![ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts),ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts)],create:std::sync::Arc::new(|_|Box::new(Noop))}]
}
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="ordinary_validation_frame",invariants=ordinary_frame,semantic_source=include_bytes!("validation_views.rs"))]
struct OrdinaryFrame { #[model(key)] marker:bool }
fn ordinary_frame()->Vec<Invariant> {
    vec![Invariant {name:"ordinary_frame",inputs:vec![ValidationInput::of::<OrdinaryFrame>(&["id"]).at_epoch(PublicationBoundary::Facts)],create:std::sync::Arc::new(|_|Box::new(Noop))}]
}
struct Noop;
impl InvariantCheck for Noop {
    fn visit(&mut self,_:&str,_:&arrow_array::RecordBatch)->Result<(),ModelError>{Ok(())}
    fn finish(self:Box<Self>)->Result<(),ModelError>{Ok(())}
}
#[test]
fn malformed_frames_refuse_before_a_schedule_can_acquire_authority() {
    assert!(ValidatedModel::validate(vec![Relation::of::<Literal>(),Relation::of::<DuplicateFrames>()]).is_err());
    assert!(ValidatedModel::validate(vec![Relation::of::<OrdinaryFrame>()]).is_err());
}
