use lctx_model::domain::{ModelError, resources::*};
#[test]
fn reservations_share_capacity_and_failed_resize_preserves_existing_charge() {
    let budget = ResourceBudget::fixed(100).unwrap();
    let same = budget.clone();
    assert!(budget.shares_pool(&same));
    let mut producer = budget.reserve("producer", 60).unwrap();
    let reader = same.reserve("reader", 40).unwrap();
    assert_eq!(budget.reserved(), 100);
    assert!(matches!(
        producer.try_resize(61),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(producer.size(), 60);
    assert_eq!(budget.reserved(), 100);
    drop(reader);
    producer.try_resize(100).unwrap();
    producer.try_resize(20).unwrap();
    assert_eq!(budget.reserved(), 20);
    drop(producer);
    assert_eq!(budget.reserved(), 0);
    assert!(budget.reserve("overflow", usize::MAX).is_err());
    assert_eq!(budget.reserved(), 0);
    assert!(ResourceBudget::fixed(0).is_err());
}
#[test]
fn concurrent_reservations_cannot_overbook_the_attempt() {
    let budget = ResourceBudget::fixed(100).unwrap();
    let start = std::sync::Arc::new(std::sync::Barrier::new(8));
    let held = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let budget = budget.clone();
            let start = start.clone();
            let held = held.clone();
            std::thread::spawn(move || {
                start.wait();
                let allocation = budget.reserve("worker", 60);
                held.wait();
                allocation.is_ok()
            })
        })
        .collect();
    let successes = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn attachment_buffers_share_budget_and_ambiguity_owns_its_reservation() {
    use lctx_model::domain::{attachment::*, input::*, source::*, *};
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "x.py".into(), b"x").unwrap();
    let rows: Vec<_> = (0..3)
        .map(|i| Occurrence {
            source: source.id(),
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![i],
        })
        .collect();
    let query = AttachmentQuery {
        source: source.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,
        structural_path: None,
    };
    let budget = ResourceBudget::fixed(4096).unwrap();
    let blocker = budget.reserve("other stage", 4096).unwrap();
    assert!(matches!(
        OccurrenceIndex::new(&rows, budget.clone()),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(budget.reserved(), 4096);
    drop(blocker);
    let index = OccurrenceIndex::new(&rows, budget.clone()).unwrap();
    let index_bytes = budget.reserved();
    assert!(index_bytes > 0);
    let blocker = budget.reserve("other stage", 4096 - index_bytes).unwrap();
    assert!(matches!(
        index.attach(&query, AttachmentBudget::default()),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(budget.reserved(), 4096);
    drop(blocker);
    let result = index.attach(&query, AttachmentBudget::default()).unwrap();
    assert!(matches!(result.value(),Attachment::Ambiguous(ids) if ids.len() == 3));
    assert_eq!(
        budget.reserved(),
        index_bytes + 3 * std::mem::size_of::<Id<Occurrence>>()
    );
    drop(index);
    assert_eq!(budget.reserved(), 3 * std::mem::size_of::<Id<Occurrence>>());
    drop(result);
    assert_eq!(budget.reserved(), 0);
    let duplicate = vec![rows[0].clone(), rows[0].clone()];
    assert!(OccurrenceIndex::new(&duplicate, budget.clone()).is_err());
    assert_eq!(budget.reserved(), 0);
    // Work refusal is semantic uncertainty; memory refusal is an attempt error. Neither retains scratch.
    let index = OccurrenceIndex::new(&rows, budget.clone()).unwrap();
    let before = budget.reserved();
    let result = index
        .attach(
            &query,
            AttachmentBudget {
                visited_nodes: 0,
                alternatives: 3,
            },
        )
        .unwrap();
    assert_eq!(result.value(), &Attachment::BudgetExceeded);
    assert_eq!(budget.reserved(), before);
}

mod batching {
    use lctx_model::domain::{batching::*, input::*, resources::*, source::*, stages::*, *};
    use std::{
        future::Future,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };
    fn ready<T>(future: impl Future<Output = T>) -> T {
        match std::pin::pin!(future)
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test sink unexpectedly pending"),
        }
    }
    fn sizes(
        writer: BatchWriter<Package>,
        model: &ValidatedModel,
        rows: impl IntoIterator<Item = Package>,
    ) -> Result<Vec<usize>, ModelError> {
        let mut writer = writer;
        let mut sizes = Vec::new();
        for row in rows {
            if let Some(batch) = writer.push(model, row)? {
                sizes.push(batch.rows().len());
            }
        }
        if let Some(batch) = writer.finish(model)? {
            sizes.push(batch.rows().len());
        }
        Ok(sizes)
    }
    /// Capacity is charged, so large names are built at their exact length.
    fn named(bytes: usize, suffix: usize) -> Package {
        let mut name = "a".repeat(bytes);
        name.push_str(&suffix.to_string());
        name.shrink_to_fit();
        Package { name }
    }

    #[test]
    fn writer_flushes_at_row_and_byte_targets_and_admits_oversized_rows_alone() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let writer = || BatchWriter::<Package>::new(&budget, TransferLimits::default()).unwrap();
        assert_eq!(
            sizes(
                writer(),
                &model,
                (0..10_000).map(|n| Package {
                    name: format!("p{n}")
                })
            )
            .unwrap(),
            [4096, 4096, 1808]
        );
        assert_eq!(
            sizes(writer(), &model, (0..3).map(|n| named(3 << 20, n))).unwrap(),
            [2, 1]
        );
        assert_eq!(
            sizes(
                writer(),
                &model,
                [named(8, 0), named(20 << 20, 1), named(8, 2)]
            )
            .unwrap(),
            [1, 1, 1],
            "a 20 MiB row travels alone"
        );
        assert!(
            matches!(
                sizes(writer(), &model, [named(70 << 20, 0)]),
                Err(ModelError::Limit {
                    limit: "row bytes",
                    bound: MAX_ROW_BYTES,
                    ..
                })
            ),
            "a 70 MiB row exceeds the row limit"
        );
        assert_eq!(budget.reserved(), 0);
        assert!(
            BatchWriter::<Package>::new(
                &budget,
                TransferLimits {
                    rows: 0,
                    ..TransferLimits::default()
                }
            )
            .is_err()
        );
        assert_eq!(
            sizes(
                BatchWriter::new(
                    &budget,
                    TransferLimits {
                        rows: 1,
                        ..TransferLimits::default()
                    }
                )
                .unwrap(),
                &model,
                (0..3).map(|n| Package {
                    name: format!("p{n}")
                })
            )
            .unwrap(),
            [1, 1, 1]
        );
    }

    #[test]
    fn writer_emits_equal_rows_once_and_refuses_conflicting_payloads_across_flushes() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let limits = TransferLimits {
            rows: 2,
            ..TransferLimits::default()
        };
        let rows = (0..3).map(|n| Package {
            name: format!("p{n}"),
        });
        assert_eq!(
            sizes(
                BatchWriter::new(&budget, limits).unwrap(),
                &model,
                rows.clone().chain(rows)
            )
            .unwrap(),
            [2, 1],
            "repeats after a flush are dropped"
        );
        let input = InputRevision::from_entries(vec![]).unwrap();
        let artifact = SourceArtifact::from_bytes(input.id(), "x.py".into(), b"x").unwrap();
        let mut writer =
            BatchWriter::<SourceArtifact>::new(&budget, TransferLimits { rows: 1, ..limits })
                .unwrap();
        assert!(writer.push(&model, artifact.clone()).unwrap().is_none());
        assert!(
            writer.push(&model, artifact.clone()).unwrap().is_none(),
            "equal row after admission is dropped"
        );
        let conflicting = SourceArtifact {
            byte_len: 2,
            ..artifact
        };
        assert!(matches!(
            writer.push(&model, conflicting),
            Err(ModelError::Conflict(_))
        ));
        drop(writer);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn batches_hold_their_reservation_and_refuse_before_encoding() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let rows: Vec<_> = (0..100)
            .map(|n| Package {
                name: format!("p{n}"),
            })
            .collect();
        let batch = Batch::new(&model, rows.clone(), &budget).unwrap();
        assert!(batch.reserved() >= batch.arrow().get_array_memory_size());
        assert_eq!(budget.reserved(), batch.reserved());
        let read = Batch::<Package>::read(&model, batch.arrow(), &budget).unwrap();
        assert_eq!(read.rows(), batch.rows());
        assert_eq!(budget.reserved(), batch.reserved() + read.reserved());
        drop(batch);
        drop(read);
        assert_eq!(budget.reserved(), 0);
        let short = ResourceBudget::fixed(1024).unwrap();
        assert!(matches!(
            Batch::new(&model, rows, &short),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(short.reserved(), 0);
        let short = ResourceBudget::fixed(8 << 10).unwrap();
        let mut writer = BatchWriter::<Package>::new(&short, TransferLimits::default()).unwrap();
        let refused = (0..1000).try_for_each(|n| {
            writer
                .push(
                    &model,
                    Package {
                        name: format!("p{n}"),
                    },
                )
                .map(drop)
        });
        assert!(
            matches!(refused, Err(ModelError::Resource { .. })),
            "pending rows are admitted before they are held"
        );
        drop(writer);
        assert_eq!(short.reserved(), 0);
    }

    struct Recorder(Mutex<Vec<(&'static str, usize)>>);
    impl StageSink for Recorder {
        fn copy<R: Record>(
            &self,
            permit: WritePermit<'_, R>,
            batch: &Batch<R>,
        ) -> impl Future<Output = Result<(), ModelError>> + Send {
            self.0
                .lock()
                .unwrap()
                .push((permit.relation(), batch.rows().len()));
            std::future::ready(Ok(()))
        }
    }
    fn stage() -> Stage {
        Stage {
            name: "inputs",
            inputs: vec![],
            outputs: vec![RelationUse::of::<Package>(), RelationUse::of::<Release>()],
            contributes: vec![],
            coverage: vec![],
            provider: None,
            profiles: vec![Profile::Catalog],
            effect: Effect::Extraction,
            code: ContentHash::of(b"producer"),
            configuration: ContentHash::of(b"config"),
        }
    }

    #[test]
    fn stage_output_streams_declared_outputs_and_writes_empty_ones_explicitly() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let schedule = Schedule::build(&model, vec![stage()], &[], Profile::Catalog).unwrap();
        let sink = Recorder(Mutex::new(Vec::new()));
        let mut execution = schedule.execute();
        let mut output = StageOutput::new(
            execution.begin("inputs").unwrap(),
            &sink,
            &model,
            budget.clone(),
            TransferLimits::default(),
        )
        .unwrap();
        output.declare::<Package>().unwrap();
        assert!(
            output.declare::<Package>().is_err(),
            "an output is declared once"
        );
        assert!(
            output.declare::<SourceArtifact>().is_err(),
            "only stage outputs can be declared"
        );
        assert!(
            ready(output.push(Release {
                package: Package { name: "p".into() }.id(),
                version: "1".into()
            }))
            .is_err(),
            "undeclared output"
        );
        output.declare::<Release>().unwrap();
        for n in 0..5000 {
            ready(output.push(Package {
                name: format!("p{n}"),
            }))
            .unwrap();
        }
        ready(output.finish(ProviderOutcome::Complete)).unwrap();
        assert_eq!(
            *sink.0.lock().unwrap(),
            [("packages", 4096), ("packages", 904), ("releases", 0)]
        );
        assert_eq!(
            execution.finish().unwrap().outcomes()["inputs"],
            ProviderOutcome::Complete
        );
        assert_eq!(budget.reserved(), 0);

        let mut execution = schedule.execute();
        let mut output = StageOutput::new(
            execution.begin("inputs").unwrap(),
            &sink,
            &model,
            budget.clone(),
            TransferLimits::default(),
        )
        .unwrap();
        output.declare::<Package>().unwrap();
        assert!(
            ready(output.finish(ProviderOutcome::Complete)).is_err(),
            "every stage output must be declared"
        );
        assert!(
            execution.finish().is_err(),
            "a refused stage output fails the attempt"
        );
    }

    #[test]
    fn a_writer_refusal_fails_the_writer_and_the_stage() {
        let model = model().unwrap();
        let short = ResourceBudget::fixed(8 << 10).unwrap();
        let mut writer = BatchWriter::<Package>::new(
            &short,
            TransferLimits {
                rows: 4,
                ..TransferLimits::default()
            },
        )
        .unwrap();
        let refused = (0..1000).try_for_each(|n| {
            writer
                .push(
                    &model,
                    Package {
                        name: format!("p{n}"),
                    },
                )
                .map(drop)
        });
        assert!(matches!(refused, Err(ModelError::Resource { .. })));
        assert!(
            writer.push(&model, Package { name: "x".into() }).is_err(),
            "a refused writer takes no further rows"
        );
        assert!(
            writer.push(&model, Package { name: "p0".into() }).is_err(),
            "not even an earlier row it may have lost"
        );
        assert!(
            writer.finish(&model).is_err(),
            "and never finishes as if complete"
        );
        assert_eq!(short.reserved(), 0);
        // Through a stage: the attempt fails and cannot be receipted.
        let schedule = Schedule::build(&model, vec![stage()], &[], Profile::Catalog).unwrap();
        let sink = Recorder(Mutex::new(Vec::new()));
        let mut execution = schedule.execute();
        let mut output = StageOutput::new(
            execution.begin("inputs").unwrap(),
            &sink,
            &model,
            short.clone(),
            TransferLimits {
                rows: 4,
                ..TransferLimits::default()
            },
        )
        .unwrap();
        output.declare::<Package>().unwrap();
        output.declare::<Release>().unwrap();
        let refused = (0..1000).try_for_each(|n| {
            ready(output.push(Package {
                name: format!("p{n}"),
            }))
        });
        assert!(matches!(refused, Err(ModelError::Resource { .. })));
        assert!(ready(output.finish(ProviderOutcome::Complete)).is_err());
        assert!(
            execution.finish().is_err(),
            "a stage that lost rows fails the attempt"
        );
        assert_eq!(short.reserved(), 0);
    }
}

mod charged_state {
    use lctx_model::domain::{charged::*, input::*, resources::*, source::*, *};

    #[test]
    fn charged_containers_admit_before_retaining_and_release_on_removal() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut charge = StateCharge::new(&budget, "test");
        let mut map = ChargedMap::<i64, String>::default();
        assert!(
            map.insert(&mut charge, 1, "a".repeat(100))
                .unwrap()
                .is_none()
        );
        let one = budget.reserved();
        assert!(one >= 100);
        map.insert(&mut charge, 1, "b".repeat(10)).unwrap();
        assert!(
            budget.reserved() < one,
            "replacing a value releases the old payload"
        );
        map.update(&mut charge, 2, |value| value.push_str(&"c".repeat(1000)))
            .unwrap();
        assert!(budget.reserved() >= 1000);
        map.remove(&mut charge, &2).unwrap();
        let mut set = ChargedSet::<i64>::default();
        assert!(set.insert(&mut charge, 7).unwrap());
        assert!(!set.insert(&mut charge, 7).unwrap());
        let mut vec = ChargedVec::<i64>::default();
        for n in 0..100 {
            vec.push(&mut charge, n).unwrap();
        }
        assert_eq!(vec.len(), 100);
        drop((map, set, vec));
        drop(charge);
        assert_eq!(budget.reserved(), 0);
        let mut unbound = StateCharge::default();
        assert!(
            ChargedSet::<i64>::default()
                .insert(&mut unbound, 1)
                .is_err(),
            "a check without its budget cannot retain state"
        );
        let tiny = ResourceBudget::fixed(64).unwrap();
        let mut charge = StateCharge::new(&tiny, "test");
        assert!(matches!(
            ChargedMap::<i64, String>::default().insert(&mut charge, 1, "x".repeat(128)),
            Err(ModelError::Resource { .. })
        ));
        drop(charge);
        assert_eq!(tiny.reserved(), 0);
    }

    #[test]
    fn stored_invariant_state_is_charged_to_the_validation_budget() {
        let model = model().unwrap();
        let invariant = model
            .invariants()
            .iter()
            .find(|i| i.name == "occurrence_source_bounds")
            .unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let sources: Vec<_> = (0..1000)
            .map(|n| SourceArtifact::from_bytes(input.id(), format!("m{n}.py"), b"x").unwrap())
            .collect();
        let arrow = Batch::new(&model, sources, &ResourceBudget::fixed(1 << 30).unwrap())
            .unwrap()
            .arrow()
            .clone();
        let tiny = ResourceBudget::fixed(4096).unwrap();
        let mut check = (invariant.create)(&tiny);
        assert!(matches!(
            check.visit(SourceArtifact::NAME, &arrow),
            Err(ModelError::Resource { .. })
        ));
        drop(check);
        assert_eq!(tiny.reserved(), 0);
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let mut check = (invariant.create)(&budget);
        check.visit(SourceArtifact::NAME, &arrow).unwrap();
        assert!(
            budget.reserved() > 1000 * 16,
            "retained lengths are charged"
        );
        check.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
    }
}
