use std::{future::Future,task::{Context,Poll,Waker}};
use lctx_model::domain::{*, input::*, stages::*};
fn ready<T>(future: impl Future<Output=T>) -> T {
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value, Poll::Pending => panic!("test effect unexpectedly pending"),
    }
}
fn stages() -> Vec<Stage> {
    vec![Stage { name: "packages", inputs: vec![], outputs: vec![RelationUse::of::<Package>()], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") },
        Stage { name: "releases", inputs: vec![RelationUse::of::<Package>()],outputs: vec![RelationUse::of::<Release>()], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") }]
}
#[test]
fn execution_capabilities_refuse_undeclared_reads_writes_and_incomplete_schedules() {
    let model = model().unwrap(); let schedule = Schedule::build(&model,stages(),&[RelationUse::of::<Release>()], Profile::Catalog).unwrap();
    let mut reordered = stages(); reordered.reverse();
    assert_eq!(schedule.digest(),Schedule::build(&model,reordered,&[RelationUse::of::<Release>()], Profile::Catalog).unwrap().digest());
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
    let model = model().unwrap(); let schedule = Schedule::build(&model,stages(),&[], Profile::Catalog).unwrap();
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

#[test]
fn producer_identity_effect_configuration_and_profile_are_schedule_contracts() {
    let model = model().unwrap();
    let digest = |stages, profile| Schedule::build(&model, stages, &[], profile).unwrap().digest();
    let original = digest(stages(), Profile::Catalog);
    for change in ["code", "configuration", "effect"] {
        let mut next = stages();
        match change {
            "code" => next[0].code = ContentHash::of(b"changed producer"),
            "configuration" => next[0].configuration = ContentHash::of(b"changed configuration"),
            _ => next[0].effect = Effect::Extraction,
        }
        assert_ne!(original, digest(next, Profile::Catalog), "{change}");
    }
    assert_ne!(original, digest(stages(), Profile::Behavioral));
    let mut selected = stages();
    selected[0].profiles = vec![Profile::Behavioral];
    assert!(Schedule::build(&model, selected.clone(), &[], Profile::Catalog).is_err(), "a skipped writer cannot supply an input");
    assert!(Schedule::build(&model, selected, &[], Profile::Behavioral).is_ok());
    let mut invalid = stages(); invalid[0].profiles.clear();
    assert!(Schedule::build(&model, invalid, &[], Profile::Catalog).is_err());
}

#[test]
fn write_permits_and_receipts_retain_the_exact_attempt() {
    let model = model().unwrap();
    let schedule = Schedule::build(&model, stages(), &[], Profile::Catalog).unwrap();
    let mut first = schedule.execute();
    let second = schedule.execute();
    assert_ne!(first.identity(), second.identity());
    let identity = first.identity();
    let mut source = first.begin("packages").unwrap();
    ready(source.write::<Package,_>(async |permit| {
        assert_eq!(permit.identity().attempt(), identity); Ok(())
    })).unwrap();
    source.finish(ProviderOutcome::Complete).unwrap();
    let mut target = first.begin("releases").unwrap();
    ready(target.write::<Release,_>(async |permit| {
        assert_eq!(permit.identity().attempt(), identity); Ok(())
    })).unwrap();
    target.finish(ProviderOutcome::Complete).unwrap();
    assert_eq!(first.finish().unwrap().identity(), identity);
}
