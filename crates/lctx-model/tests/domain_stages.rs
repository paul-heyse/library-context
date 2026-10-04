use lctx_model::domain::{input::*, stages::*, *};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};
fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("test effect unexpectedly pending"),
    }
}
fn stage_model() -> ValidatedModel {
    ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Release>()]).unwrap()
}
/// Hand off an explicitly empty `Package` output; its reader stage declares it as input.
fn execution_handoff_required(stage: &mut StageAccess<'_, '_>) -> bool {
    let empty = Batch::<Package>::new(
        &stage_model(),
        vec![],
        &lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap(),
    )
    .unwrap();
    stage.retain(std::sync::Arc::new(empty)).is_ok()
}
fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "packages",
            inputs: vec![],
            outputs: vec![RelationUse::of::<Package>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Pure,
            code: ContentHash::of(b"test-producer"),
            configuration: ContentHash::of(b"test-config"),
        },
        Stage {
            name: "releases",
            inputs: vec![RelationUse::of::<Package>()],
            outputs: vec![RelationUse::of::<Release>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Pure,
            code: ContentHash::of(b"test-producer"),
            configuration: ContentHash::of(b"test-config"),
        },
    ]
}

#[test]
fn completed_store_inputs_release_handoffs_and_require_acknowledged_sources() {
    use lctx_model::domain::{memory::MemoryGeneration, resources::ResourceBudget};
    use std::sync::Arc;
    let model = stage_model();
    let mut declarations = stages();
    declarations[1].inputs = vec![RelationUse::stored::<Package>()];
    let schedule = Schedule::build(&model, declarations, &[], Profile::Catalog).unwrap();
    assert_ne!(
        schedule.digest(),
        Schedule::build(&model, stages(), &[], Profile::Catalog)
            .unwrap()
            .digest()
    );
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    let mut execution = schedule.execute();
    let sink = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let batch = Arc::new(Batch::new(&model, vec![Package { name: "p".into() }], &budget).unwrap());
    let weak = Arc::downgrade(&batch);
    let mut source = execution.begin("packages").unwrap();
    ready(source.write::<Package, _>(async |permit| sink.copy(permit, &batch).await)).unwrap();
    source.retain(batch).unwrap();
    assert!(
        weak.upgrade().is_none(),
        "store transport must not retain the producer's batch"
    );
    ready(source.complete(&sink, ProviderOutcome::Partial)).unwrap();
    let target = execution.begin("releases").unwrap();
    assert!(target.handoff::<Package>().is_err());
    let permit = target.read::<Package>().unwrap();
    assert_eq!(permit.transport(), InputTransport::CompletedStore);
    let source = permit.source().unwrap();
    assert_eq!(source.producer(), "packages");
    assert_eq!(source.receipt().rows, 1);
    assert_eq!(source.identity().attempt(), permit.identity().attempt());

    let mut other = schedule.execute();
    let _sink = MemoryGeneration::bind(&model, &budget, &mut other).unwrap();
    let mut unacknowledged = other.begin("packages").unwrap();
    ready(unacknowledged.write::<Package, _>(async |_| Ok(()))).unwrap();
    assert!(unacknowledged.finish(ProviderOutcome::Complete).is_err());
    assert!(other.begin("releases").is_err());
}
#[test]
fn execution_capabilities_refuse_undeclared_reads_writes_and_incomplete_schedules() {
    let model = stage_model();
    let schedule = Schedule::build(
        &model,
        stages(),
        &[RelationUse::of::<Release>()],
        Profile::Catalog,
    )
    .unwrap();
    let mut reordered = stages();
    reordered.reverse();
    assert_eq!(
        schedule.digest(),
        Schedule::build(
            &model,
            reordered,
            &[RelationUse::of::<Release>()],
            Profile::Catalog
        )
        .unwrap()
        .digest()
    );
    let mut execution = schedule.execute();
    assert!(execution.begin("releases").is_err());
    assert!(execution.begin("unknown").is_err());
    let mut stage = execution.begin("packages").unwrap();
    assert!(stage.read::<Package>().is_err());
    assert!(ready(stage.write::<Release, _>(async |_| Ok(()))).is_err());
    ready(stage.write::<Package, _>(async |permit| {
        assert_eq!(permit.model(), model.digest());
        assert_eq!(permit.relation(), Package::NAME);
        Ok(())
    }))
    .unwrap();
    assert!(
        execution_handoff_required(&mut stage),
        "a raw writer must hand off an output that has readers"
    );
    stage.finish(ProviderOutcome::Complete).unwrap();
    assert!(execution.begin("packages").is_err());
    let mut stage = execution.begin("releases").unwrap();
    assert_eq!(stage.read::<Package>().unwrap().relation(), Package::NAME);
    assert!(stage.read::<Release>().is_err());
    ready(stage.write::<Release, _>(async |_| Ok(()))).unwrap();
    stage.finish(ProviderOutcome::Partial).unwrap();
    let receipt = execution.finish().unwrap();
    assert_eq!(receipt.model(), model.digest());
    assert_eq!(receipt.outcomes()["releases"], ProviderOutcome::Partial);
    assert!(schedule.execute().finish().is_err());
}
#[test]
fn failed_dropped_or_cancelled_effects_cannot_receive_execution_receipts() {
    let model = stage_model();
    let schedule = Schedule::build(&model, stages(), &[], Profile::Catalog).unwrap();
    for case in [
        "dropped",
        "missing-output",
        "failed-output",
        "failed-outcome",
        "cancelled",
    ] {
        let mut execution = schedule.execute();
        let mut stage = execution.begin("packages").unwrap();
        match case {
            "dropped" => drop(stage),
            "missing-output" => assert!(stage.finish(ProviderOutcome::Complete).is_err()),
            "failed-output" => {
                assert!(
                    ready(
                        stage.write::<Package, _>(async |_| Err::<(), _>(ModelError::Invalid(
                            "sink failed".into()
                        )))
                    )
                    .is_err()
                );
                assert!(ready(stage.write::<Package, _>(async |_| Ok(()))).is_err());
                drop(stage);
            }
            "failed-outcome" => {
                ready(stage.write::<Package, _>(async |_| Ok(()))).unwrap();
                assert!(stage.finish(ProviderOutcome::Failed).is_err());
            }
            _ => {
                {
                    let mut future = std::pin::pin!(stage.write::<Package, _>(async |_| {
                        std::future::pending::<()>().await;
                        Ok(())
                    }));
                    assert!(
                        future
                            .as_mut()
                            .poll(&mut Context::from_waker(Waker::noop()))
                            .is_pending()
                    );
                }
                assert!(stage.finish(ProviderOutcome::Complete).is_err());
            }
        }
        assert!(execution.begin("packages").is_err(), "{case}");
        assert!(execution.finish().is_err(), "{case}");
    }
}

#[test]
fn producer_identity_effect_configuration_and_profile_are_schedule_contracts() {
    let model = stage_model();
    let digest = |stages, profile| {
        Schedule::build(&model, stages, &[], profile)
            .unwrap()
            .digest()
    };
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
    assert!(
        Schedule::build(&model, selected.clone(), &[], Profile::Catalog).is_err(),
        "a skipped writer cannot supply an input"
    );
    assert!(Schedule::build(&model, selected, &[], Profile::Behavioral).is_ok());
    let mut invalid = stages();
    invalid[0].profiles.clear();
    assert!(Schedule::build(&model, invalid, &[], Profile::Catalog).is_err());
}

#[test]
fn write_permits_and_receipts_retain_the_exact_attempt() {
    let model = stage_model();
    let schedule = Schedule::build(&model, stages(), &[], Profile::Catalog).unwrap();
    let mut first = schedule.execute();
    let second = schedule.execute();
    assert_ne!(first.identity(), second.identity());
    let identity = first.identity();
    let mut source = first.begin("packages").unwrap();
    ready(source.write::<Package, _>(async |permit| {
        assert_eq!(permit.identity().attempt(), identity);
        Ok(())
    }))
    .unwrap();
    execution_handoff_required(&mut source);
    source.finish(ProviderOutcome::Complete).unwrap();
    let mut target = first.begin("releases").unwrap();
    ready(target.write::<Release, _>(async |permit| {
        assert_eq!(permit.identity().attempt(), identity);
        Ok(())
    }))
    .unwrap();
    target.finish(ProviderOutcome::Complete).unwrap();
    assert_eq!(first.finish().unwrap().identity(), identity);
}

mod contributions {
    use lctx_model::domain::{
        artifact::ArtifactChunk, attribution::FactFamily, batching::TransferLimits, input::*,
        memory::MemoryGeneration, resources::ResourceBudget, source::SourceArtifact, stages::*, *,
    };
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    fn ready<T>(future: impl Future<Output = T>) -> T {
        match std::pin::pin!(future)
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("in-memory sink unexpectedly pending"),
        }
    }
    fn contribution_model() -> ValidatedModel {
        ValidatedModel::validate(vec![
            Relation::of::<Package>(),
            Relation::of::<Release>(),
            Relation::of::<InputRevision>(),
            Relation::of::<SourceArtifact>(),
            Relation::of::<ArtifactChunk>(),
            Relation::of::<CorpusLibrary>(),
        ])
        .unwrap()
    }
    fn stage(
        name: &'static str,
        inputs: Vec<RelationUse>,
        outputs: Vec<RelationUse>,
        contributes: Vec<RelationUse>,
    ) -> Stage {
        Stage {
            name,
            inputs,
            outputs,
            contributes,
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: Effect::Extraction,
            code: ContentHash::of(name.as_bytes()),
            configuration: ContentHash::of(b"config"),
        }
    }
    fn pipeline() -> Vec<Stage> {
        vec![
            stage(
                "left",
                vec![],
                vec![RelationUse::of::<InputRevision>()],
                vec![RelationUse::of::<Package>()],
            ),
            stage(
                "right",
                vec![],
                vec![RelationUse::of::<Release>()],
                vec![RelationUse::of::<Package>()],
            ),
            stage(
                "assemble",
                vec![],
                vec![RelationUse::of::<Package>()],
                vec![],
            ),
            stage(
                "reader",
                vec![RelationUse::of::<Package>()],
                vec![RelationUse::of::<CorpusLibrary>()],
                vec![],
            ),
        ]
    }

    #[test]
    fn contributions_are_declared_schedule_contracts() {
        let model = contribution_model();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let order: Vec<_> = schedule.stages().iter().map(|s| s.name).collect();
        assert!(
            order.iter().position(|n| *n == "assemble") > order.iter().position(|n| *n == "right"),
            "the writer runs after its contributors"
        );
        let own = vec![stage(
            "both",
            vec![],
            vec![RelationUse::of::<Package>()],
            vec![RelationUse::of::<Package>()],
        )];
        assert!(
            Schedule::build(&model, own, &[], Profile::Catalog).is_err(),
            "a stage cannot contribute to its own output"
        );
        let orphan = vec![stage(
            "left",
            vec![],
            vec![RelationUse::of::<InputRevision>()],
            vec![RelationUse::of::<CorpusLibrary>()],
        )];
        assert!(
            Schedule::build(&model, orphan, &[], Profile::Catalog).is_err(),
            "a contribution needs a scheduled writer"
        );
        let mut cyclic = pipeline();
        cyclic[0].inputs = vec![RelationUse::of::<Package>()];
        assert!(
            Schedule::build(&model, cyclic, &[], Profile::Catalog).is_err(),
            "a contributor cannot read its writer's output"
        );
        let digest = |stages| {
            Schedule::build(&model, stages, &[], Profile::Catalog)
                .unwrap()
                .digest()
        };
        let mut dropped = pipeline();
        dropped[1].contributes.clear();
        let provider = |tool: &str| {
            lctx_model::domain::attribution::Provider {
                tool: tool.into(),
                revision: "1".into(),
                build_digest: ContentHash::of(b"p"),
            }
            .id()
        };
        let mut unattributed = pipeline();
        unattributed[0].coverage = vec![
            FamilyCoverage {
                family: FactFamily::Syntax,
                provider: provider("a")
            };
            2
        ];
        assert!(
            Schedule::build(&model, unattributed, &[], Profile::Catalog).is_err(),
            "coverage names its provider"
        );
        let mut covered = pipeline();
        covered[0].coverage = vec![FamilyCoverage {
            family: FactFamily::Syntax,
            provider: provider("a"),
        }];
        let mut other = pipeline();
        other[0].coverage = vec![FamilyCoverage {
            family: FactFamily::Syntax,
            provider: provider("b"),
        }];
        assert_ne!(digest(pipeline()), digest(dropped));
        assert_ne!(digest(pipeline()), digest(covered.clone()));
        assert_ne!(
            digest(covered),
            digest(other),
            "the covering provider is part of the schedule"
        );
    }

    #[test]
    fn contributed_vocabulary_merges_once_and_handoffs_release_after_the_last_reader() {
        let model = contribution_model();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let (stage_budget, store_budget) = (
            ResourceBudget::fixed(1 << 30).unwrap(),
            ResourceBudget::fixed(1 << 30).unwrap(),
        );
        let mut execution = schedule.execute();
        let sink = MemoryGeneration::bind(&model, &store_budget, &mut execution).unwrap();
        let package = |n: usize| Package {
            name: format!("p{n}"),
        };
        for (name, rows) in [("left", [0, 1]), ("right", [1, 2])] {
            let mut output = StageOutput::new(
                execution.begin(name).unwrap(),
                &sink,
                &model,
                stage_budget.clone(),
                TransferLimits::default(),
            )
            .unwrap();
            if name == "left" {
                output.declare::<InputRevision>().unwrap();
            } else {
                output.declare::<Release>().unwrap();
            }
            assert!(
                output
                    .contribute(CorpusLibrary {
                        corpus: InputRevision::from_entries(vec![]).unwrap().id(),
                        library: InputRevision::from_entries(vec![]).unwrap().id()
                    })
                    .is_err()
            );
            for n in rows {
                output.contribute(package(n)).unwrap();
            }
            ready(output.finish(ProviderOutcome::Complete)).unwrap();
        }
        let mut output = StageOutput::new(
            execution.begin("assemble").unwrap(),
            &sink,
            &model,
            stage_budget.clone(),
            TransferLimits::default(),
        )
        .unwrap();
        assert!(
            output.handoff::<Release>().is_err(),
            "only declared inputs are handed off"
        );
        output.declare::<Package>().unwrap();
        ready(output.finish(ProviderOutcome::Complete)).unwrap();
        assert!(
            stage_budget.reserved() > 0,
            "the handoff is retained for its reader"
        );
        let mut output = StageOutput::new(
            execution.begin("reader").unwrap(),
            &sink,
            &model,
            stage_budget.clone(),
            TransferLimits::default(),
        )
        .unwrap();
        let mut rows: Vec<_> = output
            .handoff::<Package>()
            .unwrap()
            .iter()
            .flat_map(|b| b.rows().to_vec())
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            rows,
            vec![package(0), package(1), package(2)],
            "each contributed identity is written once"
        );
        output.declare::<CorpusLibrary>().unwrap();
        ready(output.finish(ProviderOutcome::Complete)).unwrap();
        assert_eq!(
            stage_budget.reserved(),
            0,
            "handoffs and contributions are released after the last reader"
        );
        execution.finish().unwrap();
        sink.validate(&model, &store_budget).unwrap();
    }

    /// P0 exit F04: a writer that emits prebuilt batches still owns the duplicates of what other
    /// stages contribute to its output.
    #[test]
    fn contributed_rows_deduplicate_against_a_batched_output_and_conflicts_refuse() {
        let model = contribution_model();
        let stages = vec![
            stage(
                "contributor",
                vec![],
                vec![RelationUse::of::<Package>()],
                vec![RelationUse::of::<SourceArtifact>()],
            ),
            stage(
                "artifacts",
                vec![],
                vec![RelationUse::of::<SourceArtifact>()],
                vec![],
            ),
            stage(
                "reader",
                vec![RelationUse::of::<SourceArtifact>()],
                vec![RelationUse::of::<CorpusLibrary>()],
                vec![],
            ),
        ];
        let schedule = Schedule::build(&model, stages, &[], Profile::Catalog).unwrap();
        let (budget, store) = (
            ResourceBudget::fixed(1 << 30).unwrap(),
            ResourceBudget::fixed(1 << 30).unwrap(),
        );
        let input = InputRevision::from_entries(vec![]).unwrap();
        let artifact = |path: &str| {
            SourceArtifact::from_bytes(input.id(), path.into(), path.as_bytes()).unwrap()
        };
        let batch = |paths: &[&str]| {
            Batch::new(&model, paths.iter().map(|p| artifact(p)).collect(), &budget).unwrap()
        };
        for case in ["equal", "conflict", "repeated"] {
            let mut execution = schedule.execute();
            let sink = MemoryGeneration::bind(&model, &store, &mut execution).unwrap();
            let mut output = StageOutput::new(
                execution.begin("contributor").unwrap(),
                &sink,
                &model,
                budget.clone(),
                TransferLimits::default(),
            )
            .unwrap();
            output.declare::<Package>().unwrap();
            let contributed = if case == "conflict" {
                SourceArtifact {
                    byte_len: 99,
                    ..artifact("a.py")
                }
            } else {
                artifact("a.py")
            };
            output.contribute(contributed).unwrap();
            output.contribute(artifact("c.py")).unwrap();
            ready(output.finish(ProviderOutcome::Complete)).unwrap();
            let mut output = StageOutput::new(
                execution.begin("artifacts").unwrap(),
                &sink,
                &model,
                budget.clone(),
                TransferLimits::default(),
            )
            .unwrap();
            output.declare::<SourceArtifact>().unwrap();
            ready(output.push_batch(batch(&["a.py", "b.py"]))).unwrap();
            if case == "repeated" {
                assert!(
                    matches!(
                        ready(output.push_batch(batch(&["b.py"]))),
                        Err(ModelError::Conflict(_))
                    ),
                    "a batched identity cannot repeat"
                );
                drop(output);
                assert!(execution.finish().is_err());
                continue;
            }
            let finished = ready(output.finish(ProviderOutcome::Complete));
            if case == "conflict" {
                assert!(
                    matches!(finished, Err(ModelError::Conflict(_))),
                    "a contributed payload differing from a batched row is refused: {finished:?}"
                );
                assert!(execution.finish().is_err());
                continue;
            }
            finished.unwrap();
            let mut output = StageOutput::new(
                execution.begin("reader").unwrap(),
                &sink,
                &model,
                budget.clone(),
                TransferLimits::default(),
            )
            .unwrap();
            let mut paths: Vec<_> = output
                .handoff::<SourceArtifact>()
                .unwrap()
                .iter()
                .flat_map(|b| b.rows().iter().map(|r| r.path.clone()).collect::<Vec<_>>())
                .collect();
            paths.sort();
            assert_eq!(
                paths,
                ["a.py", "b.py", "c.py"],
                "the contributed equal row is stored once"
            );
            output.declare::<CorpusLibrary>().unwrap();
            ready(output.finish(ProviderOutcome::Complete)).unwrap();
            execution.finish().unwrap();
        }
        assert_eq!(
            budget.reserved(),
            0,
            "batches, contributions and the duplicate index are released"
        );
    }

    #[test]
    fn raw_writers_must_hand_off_and_merge_what_the_schedule_declares() {
        let model = contribution_model();
        let schedule = Schedule::build(&model, pipeline(), &[], Profile::Catalog).unwrap();
        let mut execution = schedule.execute();
        for name in ["left", "right"] {
            let mut access = execution.begin(name).unwrap();
            if name == "left" {
                ready(access.write::<InputRevision, _>(async |_| Ok(()))).unwrap();
            } else {
                ready(access.write::<Release, _>(async |_| Ok(()))).unwrap();
            }
            let batch = Batch::new(
                &model,
                vec![Package { name: name.into() }],
                &ResourceBudget::fixed(1 << 20).unwrap(),
            )
            .unwrap();
            access.contribute(std::sync::Arc::new(batch)).unwrap();
            access.finish(ProviderOutcome::Complete).unwrap();
        }
        let mut access = execution.begin("assemble").unwrap();
        ready(access.write::<Package, _>(async |_| Ok(()))).unwrap();
        assert!(
            access.finish(ProviderOutcome::Complete).is_err(),
            "unmerged contributions and a reader without a handoff refuse"
        );
        assert!(execution.finish().is_err());
    }
}

/// A relation the production model does not declare.
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "outside_the_model")]
struct Outside {
    #[model(key)]
    name: String,
}

#[test]
fn the_schedule_refuses_double_or_missing_writers_foreign_relations_and_cycles() {
    let model = stage_model();
    let of = |name: &'static str,
              inputs: Vec<RelationUse>,
              outputs: Vec<RelationUse>,
              profiles: Vec<Profile>| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles,
        effect: Effect::Pure,
        code: ContentHash::of(b"test-producer"),
        configuration: ContentHash::of(b"test-config"),
    };
    let both = || vec![Profile::Catalog, Profile::Behavioral];
    let (package, release) = (RelationUse::of::<Package>, RelationUse::of::<Release>);
    let refused =
        |stages: Vec<Stage>, required: &[RelationUse], profile: Profile, expected: &str| {
            let error = Schedule::build(&model, stages, required, profile)
                .map(|_| ())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(expected),
                "expected `{expected}`, got `{error}`"
            );
        };
    refused(
        vec![
            of("a", vec![], vec![package()], both()),
            of("b", vec![], vec![package()], both()),
        ],
        &[],
        Profile::Catalog,
        "multiple writers",
    );
    // One writer per profile is not a double writer.
    let per_profile = || {
        vec![
            of(
                "behavior",
                vec![],
                vec![package()],
                vec![Profile::Behavioral],
            ),
            of("populate", vec![], vec![package()], vec![Profile::Catalog]),
        ]
    };
    for (profile, writer) in [
        (Profile::Catalog, "populate"),
        (Profile::Behavioral, "behavior"),
    ] {
        let schedule = Schedule::build(&model, per_profile(), &[package()], profile).unwrap();
        assert_eq!(
            schedule.stages().iter().map(|s| s.name).collect::<Vec<_>>(),
            [writer]
        );
    }
    refused(
        vec![of("a", vec![package()], vec![release()], both())],
        &[],
        Profile::Catalog,
        "missing writer for",
    );
    refused(
        vec![of("a", vec![], vec![package()], both())],
        &[release()],
        Profile::Catalog,
        "missing required writer",
    );
    refused(
        vec![of("a", vec![], vec![RelationUse::of::<Outside>()], both())],
        &[],
        Profile::Catalog,
        "undeclared relation",
    );
    refused(
        vec![of(
            "a",
            vec![RelationUse::of::<Outside>()],
            vec![package()],
            both(),
        )],
        &[],
        Profile::Catalog,
        "undeclared relation",
    );
    refused(
        vec![of("a", vec![package()], vec![package()], both())],
        &[],
        Profile::Catalog,
        "stage cycle",
    );
    refused(
        vec![
            of("a", vec![release()], vec![package()], both()),
            of("b", vec![package()], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "stage cycle",
    );
    refused(
        vec![
            of("a", vec![], vec![package()], both()),
            of("a", vec![], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "duplicate stage",
    );
    refused(
        vec![of("a", vec![], vec![package()], vec![])],
        &[],
        Profile::Catalog,
        "invalid profiles",
    );
    // A writer the profile does not request leaves its reader unscheduled, never reading an empty
    // relation as if it were complete.
    refused(
        vec![
            of("flow", vec![], vec![package()], vec![Profile::Behavioral]),
            of("reader", vec![package()], vec![release()], both()),
        ],
        &[],
        Profile::Catalog,
        "missing writer for",
    );
}
