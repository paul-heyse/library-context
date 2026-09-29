use std::{future::Future,task::{Context,Poll,Waker}};
use lctx_model::domain::{*, input::*, stages::*};
fn ready<T>(future: impl Future<Output=T>) -> T {
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value, Poll::Pending => panic!("test effect unexpectedly pending"),
    }
}
/// Hand off an explicitly empty `Package` output; its reader stage declares it as input.
fn execution_handoff_required(stage: &mut StageAccess<'_, '_>) -> bool {
    let empty = Batch::<Package>::new(&model().unwrap(), vec![], &lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap()).unwrap();
    stage.retain(std::sync::Arc::new(empty)).is_ok()
}
fn stages() -> Vec<Stage> {
    vec![Stage { name: "packages", inputs: vec![], outputs: vec![RelationUse::of::<Package>()], contributes: vec![], coverage: vec![], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") },
        Stage { name: "releases", inputs: vec![RelationUse::of::<Package>()],outputs: vec![RelationUse::of::<Release>()], contributes: vec![], coverage: vec![], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") }]
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
    assert!(execution_handoff_required(&mut stage), "a raw writer must hand off an output that has readers");
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
    execution_handoff_required(&mut source);
    source.finish(ProviderOutcome::Complete).unwrap();
    let mut target = first.begin("releases").unwrap();
    ready(target.write::<Release,_>(async |permit| {
        assert_eq!(permit.identity().attempt(), identity); Ok(())
    })).unwrap();
    target.finish(ProviderOutcome::Complete).unwrap();
    assert_eq!(first.finish().unwrap().identity(), identity);
}

mod contributions {
    use std::{future::Future, task::{Context, Poll, Waker}};
    use lctx_model::domain::{*, attribution::FactFamily, batching::TransferLimits, input::*, memory::MemoryGeneration, resources::ResourceBudget, stages::*};
    fn ready<T>(future: impl Future<Output = T>) -> T {
        match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value, Poll::Pending => panic!("in-memory sink unexpectedly pending"),
        }
    }
    fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>, contributes: Vec<RelationUse>) -> Stage {
        Stage { name, inputs, outputs, contributes, coverage: vec![], profiles: vec![Profile::Catalog], effect: Effect::Extraction,
            code: ContentHash::of(name.as_bytes()), configuration: ContentHash::of(b"config") }
    }
    fn pipeline() -> Vec<Stage> {
        vec![stage("left", vec![], vec![RelationUse::of::<InputRevision>()], vec![RelationUse::of::<Package>()]),
            stage("right", vec![], vec![RelationUse::of::<Release>()], vec![RelationUse::of::<Package>()]),
            stage("assemble", vec![], vec![RelationUse::of::<Package>()], vec![]),
            stage("reader", vec![RelationUse::of::<Package>()], vec![RelationUse::of::<CorpusLibrary>()], vec![])]
    }

    #[test]
    fn contributions_are_declared_schedule_contracts() {
        let model = model().unwrap();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let order: Vec<_> = schedule.stages().iter().map(|s| s.name).collect();
        assert!(order.iter().position(|n| *n == "assemble") > order.iter().position(|n| *n == "right"), "the writer runs after its contributors");
        let own = vec![stage("both", vec![], vec![RelationUse::of::<Package>()], vec![RelationUse::of::<Package>()])];
        assert!(Schedule::build(&model, own, &[], Profile::Catalog).is_err(), "a stage cannot contribute to its own output");
        let orphan = vec![stage("left", vec![], vec![RelationUse::of::<InputRevision>()], vec![RelationUse::of::<CorpusLibrary>()])];
        assert!(Schedule::build(&model, orphan, &[], Profile::Catalog).is_err(), "a contribution needs a scheduled writer");
        let mut cyclic = pipeline(); cyclic[0].inputs = vec![RelationUse::of::<Package>()];
        assert!(Schedule::build(&model, cyclic, &[], Profile::Catalog).is_err(), "a contributor cannot read its writer's output");
        let digest = |stages| Schedule::build(&model, stages, &[], Profile::Catalog).unwrap().digest();
        let mut dropped = pipeline(); dropped[1].contributes.clear();
        let mut covered = pipeline(); covered[0].coverage = vec![FactFamily::Syntax];
        assert_ne!(digest(pipeline()), digest(dropped)); assert_ne!(digest(pipeline()), digest(covered));
    }

    #[test]
    fn contributed_vocabulary_merges_once_and_handoffs_release_after_the_last_reader() {
        let model = model().unwrap();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let (stage_budget, store_budget) = (ResourceBudget::fixed(1 << 30).unwrap(), ResourceBudget::fixed(1 << 30).unwrap());
        let mut execution = schedule.execute();
        let sink = MemoryGeneration::bind(&model, &store_budget, &mut execution).unwrap();
        let package = |n: usize| Package { name: format!("p{n}") };
        for (name, rows) in [("left", [0, 1]), ("right", [1, 2])] {
            let mut output = StageOutput::new(execution.begin(name).unwrap(), &sink, &model, stage_budget.clone(), TransferLimits::default()).unwrap();
            if name == "left" { output.declare::<InputRevision>().unwrap(); } else { output.declare::<Release>().unwrap(); }
            assert!(output.contribute(CorpusLibrary { corpus: InputRevision::from_entries(vec![]).unwrap().id(), library: InputRevision::from_entries(vec![]).unwrap().id() }).is_err());
            for n in rows { output.contribute(package(n)).unwrap(); }
            ready(output.finish(ProviderOutcome::Complete)).unwrap();
        }
        let mut output = StageOutput::new(execution.begin("assemble").unwrap(), &sink, &model, stage_budget.clone(), TransferLimits::default()).unwrap();
        assert!(output.handoff::<Release>().is_err(), "only declared inputs are handed off");
        output.declare::<Package>().unwrap();
        ready(output.finish(ProviderOutcome::Complete)).unwrap();
        assert!(stage_budget.reserved() > 0, "the handoff is retained for its reader");
        let mut output = StageOutput::new(execution.begin("reader").unwrap(), &sink, &model, stage_budget.clone(), TransferLimits::default()).unwrap();
        let mut rows: Vec<_> = output.handoff::<Package>().unwrap().iter().flat_map(|b| b.rows().to_vec()).collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(rows, vec![package(0), package(1), package(2)], "each contributed identity is written once");
        output.declare::<CorpusLibrary>().unwrap();
        ready(output.finish(ProviderOutcome::Complete)).unwrap();
        assert_eq!(stage_budget.reserved(), 0, "handoffs and contributions are released after the last reader");
        execution.finish().unwrap();
        sink.validate(&model, &store_budget).unwrap();
    }

    #[test]
    fn raw_writers_must_hand_off_and_merge_what_the_schedule_declares() {
        let model = model().unwrap();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let mut execution = schedule.execute();
        for name in ["left", "right"] {
            let mut access = execution.begin(name).unwrap();
            if name == "left" { ready(access.write::<InputRevision, _>(async |_| Ok(()))).unwrap(); } else { ready(access.write::<Release, _>(async |_| Ok(()))).unwrap(); }
            let batch = Batch::new(&model, vec![Package { name: name.into() }], &ResourceBudget::fixed(1 << 20).unwrap()).unwrap();
            access.contribute(std::sync::Arc::new(batch)).unwrap();
            access.finish(ProviderOutcome::Complete).unwrap();
        }
        let mut access = execution.begin("assemble").unwrap();
        ready(access.write::<Package, _>(async |_| Ok(()))).unwrap();
        assert!(access.finish(ProviderOutcome::Complete).is_err(), "unmerged contributions and a reader without a handoff refuse");
        assert!(execution.finish().is_err());
    }
}
