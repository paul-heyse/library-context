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
    use lctx_model::domain::{batching::*, input::*, resources::*, source::*, *};
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

    #[test]
    fn a_writer_refusal_fails_the_writer() {
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
        map.insert(&mut charge, 2, "c".repeat(1000)).unwrap();
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
    fn nested_growth_refuses_before_mutation_and_duplicates_need_no_new_charge() {
        use std::{cell::Cell, collections::BTreeSet};
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut charge = StateCharge::new(&budget, "nested-control");
        let mut vectors = ChargedMap::<i64, Vec<String>>::default();
        for _ in 0..4 {
            vectors.push(&mut charge, 1, String::new()).unwrap();
        }
        let mut sets = ChargedMap::<i64, BTreeSet<i64>>::default();
        assert!(sets.insert_set(&mut charge, 1, 7).unwrap());
        let mut counts = ChargedMap::<i64, usize>::default();
        counts.update_inline(&mut charge, 1, |n| *n = 1).unwrap();
        let before = charge.reserved();
        let capacity = vectors[&1].capacity();
        let filler = budget
            .reserve("refuse-growth", budget.limit() - budget.reserved())
            .unwrap();
        assert!(matches!(
            vectors.push(&mut charge, 1, String::new()),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(vectors[&1].len(), 4);
        assert_eq!(vectors[&1].capacity(), capacity);
        assert!(!sets.insert_set(&mut charge, 1, 7).unwrap());
        counts.update_inline(&mut charge, 1, |n| *n += 1).unwrap();
        assert_eq!(counts[&1], 2);
        let previous = counts.insert(&mut charge, 1, 3).unwrap().unwrap();
        assert_eq!(
            *previous, 2,
            "inline replacement needs no duplicate node admission"
        );
        drop(previous);
        let called = Cell::new(false);
        struct LargeHeap<'a>(&'a Cell<bool>);
        impl AdmittedMutation<Vec<String>> for LargeHeap<'_> {
            type Output = ();
            fn additional(&self, _: &Vec<String>) -> Result<usize, ModelError> {
                Ok(2048)
            }
            fn apply(self, value: &mut Vec<String>) {
                self.0.set(true);
                value.push("x".repeat(1024));
            }
        }
        assert!(vectors.mutate(&mut charge, 1, LargeHeap(&called)).is_err());
        assert!(!called.get(), "refusal precedes the allocating mutation");
        assert!(vectors.mutate(&mut charge, 2, LargeHeap(&called)).is_err());
        assert!(
            !vectors.contains_key(&2),
            "refusal does not install a default entry"
        );
        assert_eq!(charge.reserved(), before);
        drop(filler);
        vectors.mutate(&mut charge, 1, LargeHeap(&called)).unwrap();
        assert!(called.get());
        assert_eq!(vectors[&1].last().unwrap().len(), 1024);
        let pool_before_move = budget.reserved();
        let held = budget
            .reserve(
                "refuse-transfer-metadata",
                budget.limit() - pool_before_move,
            )
            .unwrap();
        assert!(
            vectors.remove(&mut charge, &1).is_err(),
            "guard metadata is admitted before removing actual backing"
        );
        assert_eq!(vectors[&1].len(), 5);
        assert_eq!(budget.reserved(), budget.limit());
        drop(held);
        assert!(sets.insert_set(&mut charge, 1, 8).unwrap());
    }

    #[test]
    fn removed_and_replaced_values_keep_backing_and_transfer_without_readmission() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut first = StateCharge::new(&budget, "first");
        let mut second = StateCharge::new(&budget, "second");
        let mut source = ChargedMap::<i64, String>::default();
        let mut destination = ChargedMap::<i64, String>::default();
        source.insert(&mut first, 1, "a".repeat(1024)).unwrap();
        let old = source
            .insert(&mut first, 1, "b".repeat(128))
            .unwrap()
            .unwrap();
        assert_eq!(old.len(), 1024);
        assert!(
            budget.reserved() >= 1024 + 128,
            "both live payloads remain admitted"
        );
        drop(old);
        destination.insert(&mut second, 2, String::new()).unwrap();
        let moved = source.remove(&mut first, &1).unwrap().unwrap();
        assert_eq!(moved.len(), 128);
        let node_bytes = size_of::<i64>() + size_of::<String>() + 32;
        let filler = budget
            .reserve(
                "no-extra-payload",
                budget.limit() - budget.reserved() - node_bytes,
            )
            .unwrap();
        let receiving_payload = second.reserved();
        destination.insert_owned(&mut second, 1, moved).unwrap();
        assert_eq!(
            second.reserved(),
            receiving_payload + node_bytes + 128,
            "only the new node and original live payload move into the receiver"
        );
        assert!(
            budget.reserved() <= budget.limit(),
            "transfer succeeds without readmitting payload; dead guard metadata may release"
        );
        drop(filler);
        let retained = destination.remove(&mut second, &1).unwrap().unwrap();
        drop((source, destination, first, second));
        assert_eq!(
            budget.reserved(),
            retained.accounted_bytes(),
            "removed backing and accounting metadata survive their old state owner"
        );
        assert_eq!(&**retained, &"b".repeat(128));
        drop(retained);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn moved_backing_refuses_a_foreign_pool_without_creating_a_receiver_entry() {
        let source_budget = ResourceBudget::fixed(4096).unwrap();
        let receiving_budget = ResourceBudget::fixed(4096).unwrap();
        let mut source_charge = StateCharge::new(&source_budget, "source");
        let mut receiving_charge = StateCharge::new(&receiving_budget, "receiver");
        let mut source = ChargedMap::<i64, String>::default();
        let mut receiver = ChargedMap::<i64, String>::default();
        source
            .insert(&mut source_charge, 1, "owned".into())
            .unwrap();
        let value = source.remove(&mut source_charge, &1).unwrap().unwrap();
        assert!(
            receiver
                .insert_owned(&mut receiving_charge, 1, value)
                .is_err()
        );
        assert!(receiver.is_empty());
        assert_eq!(source_charge.reserved(), 0);
        assert_eq!(
            source_budget.reserved(),
            source_charge.accounted_bytes(),
            "empty retained accounting capacity stays admitted"
        );
        assert_eq!(receiving_budget.reserved(), 0);
        drop((source_charge, receiving_charge));
        assert_eq!(source_budget.reserved(), 0);
    }

    #[test]
    fn prepared_accounting_transfer_needs_no_post_effect_allocation() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut donor = StateCharge::new(&budget, "donor");
        donor.grow(128).unwrap();
        let mut receiver = StateCharge::new(&budget, "receiver");
        receiver.prepare_absorb(&donor).unwrap();
        let filler = budget
            .reserve(
                "no-post-effect-allocation",
                budget.limit() - budget.reserved(),
            )
            .unwrap();
        receiver.absorb(donor).unwrap();
        assert_eq!(receiver.reserved(), 128);
        drop(filler);
        assert_eq!(budget.reserved(), receiver.accounted_bytes());
        drop(receiver);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn nested_maps_ordered_vectors_and_shrinking_preserve_exact_state() {
        use std::collections::BTreeMap;
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut charge = StateCharge::new(&budget, "owned-operations");
        let mut maps = ChargedMap::<i64, BTreeMap<i64, String>>::default();
        assert!(
            maps.insert_map(&mut charge, 1, 7, "x".repeat(128))
                .unwrap()
                .is_none()
        );
        let replaced = maps
            .insert_map(&mut charge, 1, 7, String::new())
            .unwrap()
            .unwrap();
        assert_eq!(replaced.len(), 128);
        drop(replaced);
        let mut vectors = ChargedMap::<i64, Vec<(usize, usize)>>::default();
        vectors
            .push_ordered(&mut charge, 1, (2, 0), |v| v.0)
            .unwrap();
        vectors
            .push_ordered(&mut charge, 1, (1, 1), |v| v.0)
            .unwrap();
        vectors
            .push_ordered(&mut charge, 1, (2, 2), |v| v.0)
            .unwrap();
        assert_eq!(vectors[&1], [(1, 1), (2, 0), (2, 2)]);
        let mut heaps = ChargedMap::<i64, Vec<String>>::default();
        heaps.push(&mut charge, 1, "x".repeat(256)).unwrap();
        let before_shrink = budget.reserved();
        let capacity = heaps[&1].capacity();
        heaps.truncate(&mut charge, 1, 0).unwrap();
        assert_eq!(heaps[&1].capacity(), capacity);
        assert_eq!(budget.reserved(), before_shrink - 256);
        let mut sequence = ChargedVec::<String>::default();
        sequence.push(&mut charge, "payload".into()).unwrap();
        let before = budget.reserved();
        let owned = sequence.take_last(&mut charge).unwrap().unwrap();
        drop(owned);
        assert_eq!(budget.reserved(), before - 7);
        assert!(sequence.is_empty());
        drop((maps, vectors, heaps, sequence, charge));
        assert_eq!(budget.reserved(), 0);
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

#[test]
fn checked_ingress_moves_canonical_buffers_and_releases_only_typed_row_charge() {
    use lctx_model::domain::{Batch, Record, input::Package, model};
    let model = model().unwrap();
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let rows = vec![
        Package {
            name: "long-canonical-payload".repeat(200),
        },
        Package {
            name: "other".into(),
        },
    ];
    let batch = Batch::new(&model, rows, &budget).unwrap();
    let initial = budget.reserved();
    let encoded = batch.arrow().get_array_memory_size();
    let pointers: Vec<_> = batch
        .arrow()
        .columns()
        .iter()
        .map(std::sync::Arc::as_ptr)
        .collect();
    let canonical_ids = batch.rows().iter().map(Record::id).collect::<Vec<_>>();
    assert!(
        initial > encoded,
        "typed rows and encoded buffers coexist before transfer"
    );
    let checked = batch.into_checked().unwrap();
    assert_eq!(checked.model(), model.digest());
    assert_eq!(checked.relation().name(), Package::NAME);
    assert_eq!(checked.reserved(), encoded);
    assert_eq!(budget.reserved(), encoded);
    assert_eq!(
        pointers,
        checked
            .arrow()
            .columns()
            .iter()
            .map(std::sync::Arc::as_ptr)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        canonical_ids,
        Package::decode(checked.arrow())
            .unwrap()
            .iter()
            .map(Record::id)
            .collect::<Vec<_>>()
    );
    drop(checked);
    assert_eq!(budget.reserved(), 0);
}
