use std::{future::Future,task::{Context,Poll,Waker}};
use lctx_model::domain::{*, input::*, stages::*};
fn ready<T>(future: impl Future<Output=T>) -> T {
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value, Poll::Pending => panic!("test effect unexpectedly pending"),
    }
}
fn stages() -> Vec<Stage> {
    vec![Stage { name: "packages", inputs: vec![], outputs: vec![RelationUse::of::<Package>()] },
        Stage { name: "releases", inputs: vec![RelationUse::of::<Package>()],outputs: vec![RelationUse::of::<Release>()] }]
}
#[test]
fn execution_capabilities_refuse_undeclared_reads_writes_and_incomplete_schedules() {
    let model = model().unwrap(); let schedule = Schedule::build(&model,stages(),&[RelationUse::of::<Release>()]).unwrap();
    let mut reordered = stages(); reordered.reverse();
    assert_eq!(schedule.digest(),Schedule::build(&model,reordered,&[RelationUse::of::<Release>()]).unwrap().digest());
    let mut execution = schedule.execute();
    assert!(execution.begin("releases").is_err()); assert!(execution.begin("unknown").is_err());
    let mut stage = execution.begin("packages").unwrap();
    assert!(stage.read::<Package>().is_err());
    assert!(ready(stage.write::<Release,_>(async |_| Ok(()))).is_err());
    ready(stage.write::<Package,_>(async |permit| {
        assert_eq!(permit.model(),model.digest()); assert_eq!(permit.relation(),Package::NAME); Ok(())
    })).unwrap();
    stage.finish(ProviderOutcome::Complete).unwrap();
    assert!(execution.begin("packages").is_err());
    let mut stage = execution.begin("releases").unwrap();
    assert_eq!(stage.read::<Package>().unwrap().relation(),Package::NAME);
    assert!(stage.read::<Release>().is_err());
    ready(stage.write::<Release,_>(async |_| Ok(()))).unwrap();
    stage.finish(ProviderOutcome::Partial).unwrap();
    let receipt = execution.finish().unwrap(); assert_eq!(receipt.model(),model.digest());
    assert_eq!(receipt.outcomes()["releases"],ProviderOutcome::Partial);
    assert!(schedule.execute().finish().is_err());
}
#[test]
fn failed_dropped_or_cancelled_effects_cannot_receive_execution_receipts() {
    let model = model().unwrap(); let schedule = Schedule::build(&model,stages(),&[]).unwrap();
    for case in ["dropped","missing-output","failed-output","failed-outcome","cancelled"] {
        let mut execution = schedule.execute(); let mut stage = execution.begin("packages").unwrap();
        match case {
            "dropped" => drop(stage),
            "missing-output" => assert!(stage.finish(ProviderOutcome::Complete).is_err()),
            "failed-output" => {
                assert!(ready(stage.write::<Package,_>(async |_| Err::<(),_>(ModelError::Invalid("sink failed".into())))).is_err());
                assert!(ready(stage.write::<Package,_>(async |_| Ok(()))).is_err()); drop(stage);
            }
            "failed-outcome" => { ready(stage.write::<Package,_>(async |_| Ok(()))).unwrap(); assert!(stage.finish(ProviderOutcome::Failed).is_err()); }
            _ => {
                {
                    let mut future = std::pin::pin!(stage.write::<Package,_>(async |_| { std::future::pending::<()>().await; Ok(()) }));
                    assert!(future.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
                }
                assert!(stage.finish(ProviderOutcome::Complete).is_err());
            }
        }
        assert!(execution.begin("packages").is_err(),"{case}"); assert!(execution.finish().is_err(),"{case}");
    }
}
