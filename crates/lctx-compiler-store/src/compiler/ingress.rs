//! A bounded, owned ingestion driver. Acceptance is distinct from durable acknowledgement.
use super::*;
use tokio::sync::{mpsc, oneshot};

pub struct WriteAcknowledgement {
    pub sequence: u64,
    pub effects: crate::control::EffectReceipts,
}
pub struct IngressSequence {
    sequence: u64,
    result: oneshot::Receiver<Result<WriteAcknowledgement, ModelError>>,
}
impl IngressSequence {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub async fn await_committed(self) -> Result<WriteAcknowledgement, ModelError> {
        let result = self.result.await.map_err(|_| {
            ModelError::infrastructure(
                Infrastructure::Unconfirmed,
                "accepted native ingress lost its acknowledgement owner",
            )
        })??;
        if result.sequence != self.sequence {
            return Err(ModelError::Conflict("native ingress sequence"));
        }
        if result
            .effects
            .entries()
            .iter()
            .any(|receipt| receipt.disposition != crate::control::EffectDisposition::Committed)
        {
            return Err(ModelError::Conflict(
                "native ingress lacks committed disposition",
            ));
        }
        Ok(result)
    }
}
struct Accepted {
    sequence: u64,
    batch: CheckedCanonicalBatch,
    result: oneshot::Sender<Result<WriteAcknowledgement, ModelError>>,
}
pub struct NativeIngress {
    sender: Option<mpsc::Sender<Accepted>>,
    terminal: Option<tokio::task::JoinHandle<Result<(), ModelError>>>,
    next: u64,
    stopped: Arc<AtomicBool>,
}
/// The accepted driver's operation outcome is distinct from joining its actual owner.
/// Native operation diagnostics remain sticky in the store's admission owner.
pub struct IngressDrainReport {
    pub operation: Result<(), ModelError>,
    pub completion: d::completion::Completion,
}
impl NativeIngress {
    pub async fn accept(
        &mut self,
        batch: CheckedCanonicalBatch,
    ) -> Result<IngressSequence, ModelError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("native ingress closed"));
        }
        let sender = self
            .sender
            .as_ref()
            .ok_or(ModelError::Conflict("native ingress closed"))?;
        let permit = sender
            .reserve()
            .await
            .map_err(|_| ModelError::Conflict("native ingress closed"))?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("native ingress closed"));
        }
        let sequence = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(ModelError::Schema("native ingress sequence overflow"))?;
        let (send, result) = oneshot::channel();
        // The queue takes the nominal checked buffers and their charge. This is not
        // an effect acknowledgement; only await_committed observes that boundary.
        permit.send(Accepted {
            sequence,
            batch,
            result: send,
        });
        Ok(IngressSequence { sequence, result })
    }
    pub async fn finish(self) -> Result<(), ModelError> {
        let report = self.finish_report().await;
        d::completion::complete(report.operation, report.completion)
    }
    pub async fn finish_report(mut self) -> IngressDrainReport {
        self.sender.take();
        let mut completion = d::completion::Completion::default();
        let operation = match self.terminal.take() {
            Some(terminal) => match terminal.await {
                Ok(operation) => operation,
                Err(error) => {
                    completion.step(
                        "native ingress driver join",
                        Err(ModelError::Cause(Box::new(error)).alongside(
                            ModelError::infrastructure(
                                Infrastructure::Unconfirmed,
                                "native ingress driver lost",
                            ),
                        )),
                    );
                    Ok(())
                }
            },
            None => {
                completion.local = d::completion::LocalState::Outstanding;
                completion.step(
                    "native ingress driver join",
                    Err(ModelError::infrastructure(
                        Infrastructure::Unconfirmed,
                        "native ingress finish lacks its driver owner",
                    )),
                );
                Ok(())
            }
        };
        IngressDrainReport {
            operation,
            completion,
        }
    }
}
impl Drop for NativeIngress {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.sender.take();
        // Never abort the accepted worker. Its native operation lease remains in
        // the store's global drain until all already accepted work has settled.
    }
}

impl NativeCompilerStore {
    pub fn ingress(
        self: &Arc<Self>,
        contribution: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<NativeIngress, ModelError> {
        self.check_failed()?;
        // One pending charged batch plus the executing batch; this bounds retained
        // transfer storage, independently of provider or runtime worker concurrency.
        let retained = budget.reserve("native ingress queue", 4096)?;
        let lease = self.admit_mutation("native ingress driver", true)?;
        let producing = self.producing_binding()?;
        let (sender, mut receiver) = mpsc::channel::<Accepted>(1);
        let stopped = Arc::new(AtomicBool::new(false));
        let driver_stopped = stopped.clone();
        let store = self.clone();
        let budget = budget.clone();
        let cancellation = crate::control::capture_retry_cancellation();
        // Accepted batches can outlive their submitting bridge request. Its private
        // runtime may shut down on cancellation before this driver acknowledges.
        // The store runtime is a separate ownership boundary; carry the submitting
        // tracing origin through polling and destruction of the whole accepted driver.
        let terminal = self.runtime.spawn(
            crate::control::with_retry_context(cancellation, async move {
                let _retained = retained;
                let result = lease
                    .within(producing_future(producing, async {
                        // This frame fixes the client, attempt, contribution,
                        // producing scope and pool for the driver's entire life.
                        // Only issuance/fixed sizing survive a flushed window.
                        let mut writer = crate::control::WindowWriter::new(
                            &store.client,
                            Some(store.attempt),
                            Some(&budget),
                        );
                        let mut failure: Option<Arc<ModelError>> = None;
                        while let Some(accepted) = receiver.recv().await {
                            if let Some(error) = &failure {
                                let _ = accepted
                                    .result
                                    .send(Err(ModelError::SharedCause(error.clone())));
                                continue;
                            }
                            let captured = crate::control::capture_effect_receipts(
                                &budget,
                                store.boxed_write_batch_with_writer(
                                    &contribution,
                                    accepted.batch.relation(),
                                    accepted.batch.arrow(),
                                    Some(accepted.batch.model()),
                                    Some(&budget),
                                    &mut writer,
                                ),
                            )
                            .await;
                            let result = match captured {
                                Ok((Ok(()), effects)) => Ok(WriteAcknowledgement {
                                    sequence: accepted.sequence,
                                    effects,
                                }),
                                Ok((Err(source), effects)) => {
                                    Err(ModelError::Cause(Box::new(IngressFailure {
                                        sequence: accepted.sequence,
                                        source,
                                        effects,
                                    })))
                                }
                                Err(error) => Err(error),
                            };
                            match result {
                                Ok(acknowledgement) => {
                                    let _ = accepted.result.send(Ok(acknowledgement));
                                }
                                Err(error) => {
                                    let error = Arc::new(error);
                                    driver_stopped.store(true, Ordering::Release);
                                    receiver.close();
                                    let _ = accepted
                                        .result
                                        .send(Err(ModelError::SharedCause(error.clone())));
                                    failure = Some(error);
                                }
                            }
                        }
                        match failure {
                            Some(error) => Err(ModelError::SharedCause(error)),
                            None => Ok(()),
                        }
                    }))
                    .await;
                driver_stopped.store(true, Ordering::Release);
                lease.finish_with(result)
            })
            .in_tracing_context(),
        );
        Ok(NativeIngress {
            sender: Some(sender),
            terminal: Some(terminal),
            next: 0,
            stopped,
        })
    }
}

pub struct IngressFailure {
    pub sequence: u64,
    pub source: ModelError,
    pub effects: crate::control::EffectReceipts,
}
impl std::fmt::Debug for IngressFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngressFailure")
            .field("sequence", &self.sequence)
            .field("source", &self.source)
            .field("effects", &self.effects.entries())
            .finish()
    }
}
impl std::fmt::Display for IngressFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native ingress {}: {}", self.sequence, self.source)
    }
}
impl std::error::Error for IngressFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn local_store() -> Arc<NativeCompilerStore> {
        NativeCompilerStore::from_existing(
            Arc::new(Surreal::init()),
            d::serving::Name::new("ingress_control").unwrap(),
            d::serving::Name::new("owned_control").unwrap(),
        )
    }
    fn register_local_output(store: &NativeCompilerStore, relation: &Relation) -> ContentHash {
        let spec = ContributionSpec {
            captured_binding: None,
            producer: "ingress-lifetime-control".into(),
            profile: OutputProfile::Specific(d::stages::Profile::Catalog),
            model: d::model().unwrap().digest(),
            implementation: ContentHash::of(b"ingress-lifetime-control"),
            configuration: None,
            inputs: vec![],
            outputs: [relation.name().into()].into(),
        };
        let id = attempt_contribution(store.attempt, spec.identity().unwrap());
        store.specifications.lock().unwrap().insert(
            id,
            RegisteredContribution {
                spec,
                ownership: ContributionOwnership::Producing,
                _transport_charge: None,
            },
        );
        id
    }
    #[tokio::test]
    async fn accepted_ingress_outlives_submitting_runtime_and_joins_on_store_owner() {
        let store = local_store();
        let relation = Relation::of::<d::analytics::QualityStep>();
        let contribution = register_local_output(&store, &relation);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = Batch::<d::analytics::QualityStep>::new(&d::model().unwrap(), vec![], &budget)
            .unwrap()
            .into_checked()
            .unwrap();
        let submitted_store = store.clone();
        let submitted_budget = budget.clone();
        let (ingress, accepted) = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let mut ingress = submitted_store
                    .ingress(contribution, &submitted_budget)
                    .unwrap();
                let accepted = ingress.accept(batch).await.unwrap();
                (ingress, accepted)
            })
            // Dropping this submitting runtime must not destroy the accepted driver.
        })
        .join()
        .unwrap();
        let acknowledged = accepted.await_committed().await.unwrap();
        assert_eq!(acknowledged.sequence, 0);
        assert!(
            acknowledged.effects.entries().is_empty(),
            "DB-free empty batch only"
        );
        ingress.finish().await.unwrap();
        d::completion::complete::<()>(Ok(()), store.drain_finality_report().await).unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn reused_ingress_preserves_each_batch_validation_ack_and_release() {
        // Empty canonical batches exercise the real driver and lowering without
        // fabricating a native effect or requiring a connected database.
        let store = local_store();
        let relation = Relation::of::<d::analytics::QualityStep>();
        let contribution = register_local_output(&store, &relation);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut ingress = store.ingress(contribution, &budget).unwrap();
        for sequence in 0..2 {
            let batch =
                Batch::<d::analytics::QualityStep>::new(&d::model().unwrap(), vec![], &budget)
                    .unwrap()
                    .into_checked()
                    .unwrap();
            let accepted = ingress.accept(batch).await.unwrap();
            assert_eq!(accepted.sequence(), sequence);
            let committed = accepted.await_committed().await.unwrap();
            assert_eq!(committed.sequence, sequence);
            assert!(
                committed.effects.entries().is_empty(),
                "no native effect was requested"
            );
            assert_eq!(
                budget.reserved(),
                4096,
                "flushed batches leave only the queue owner"
            );
        }
        let undeclared =
            Batch::<d::input::InputRevision>::new(&d::model().unwrap(), vec![], &budget)
                .unwrap()
                .into_checked()
                .unwrap();
        let refused = ingress.accept(undeclared).await.unwrap();
        assert_eq!(refused.sequence(), 2);
        assert!(refused.await_committed().await.is_err());
        let report = ingress.finish_report().await;
        assert!(
            report.operation.is_err(),
            "later validation cannot reuse earlier eligibility"
        );
        assert!(report.completion.is_satisfied());
        d::completion::complete::<()>(Ok(()), store.drain_finality_report().await).unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn shared_writer_refuses_foreign_context_before_batch_validation() {
        let store = local_store();
        let relation = Relation::of::<d::analytics::QualityStep>();
        let contribution = register_local_output(&store, &relation);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let foreign = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = d::analytics::QualityStep::encode(&[]).unwrap();
        let mut writer =
            crate::control::WindowWriter::new(&store.client, Some(store.attempt), Some(&budget));
        let reader = crate::NativeReader::private(store.client.clone()).with_budget(&budget);
        assert!(std::ptr::eq(reader.client(), store.client.as_ref()));
        writer
            .ensure_context(reader.client(), Some(store.attempt), Some(&budget))
            .unwrap();
        let foreign_client = Surreal::init();
        assert!(
            writer
                .ensure_context(&foreign_client, Some(store.attempt), Some(&budget))
                .is_err()
        );
        assert!(
            store
                .write_batch_inner(
                    &contribution,
                    &relation,
                    &batch,
                    Some(d::model().unwrap().digest()),
                    Some(&foreign),
                    &mut writer
                )
                .await
                .is_err()
        );
        assert!(
            !store.failed.load(Ordering::Acquire),
            "context refusal precedes mutation admission"
        );
        assert_eq!(budget.reserved(), 0);
        assert_eq!(foreign.reserved(), 0);
    }
    #[tokio::test]
    async fn native_summary_pair_outcome_batch_has_committed_window_receipts() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing_subscriber::{Layer, layer::Context, prelude::*};

        struct BatchSpans(Arc<AtomicUsize>);
        impl<S: tracing::Subscriber> Layer<S> for BatchSpans {
            fn on_new_span(
                &self,
                attributes: &tracing::span::Attributes<'_>,
                _id: &tracing::Id,
                _context: Context<'_, S>,
            ) {
                if attributes.metadata().name() == "native_batch" {
                    self.0.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        let batch_spans = Arc::new(AtomicUsize::new(0));
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .with_env_filter("warn,lctx_phase=info")
                .with_writer(std::io::stderr)
                .finish()
                .with(BatchSpans(batch_spans.clone())),
        );
        async {
            // This tests actual guarded persistent ingress of the timed-out relation's
            // physical shape. Synthetic nominal endpoints establish no Summary semantics.
            use d::execution::summary_production::{PairDisposition, PairOutcome};
            fn nominal<R>(attempt: ContentHash, role: &str, index: u64) -> d::Id<R> {
                let mut key = KeySink::new("native-pair-batch-control");
                attempt.encode(&mut key);
                key.part(b"role", role.as_bytes());
                key.part(b"index", &index.to_le_bytes());
                serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                    _,
                    serde::de::value::Error,
                >::new(key.finish().0[..16].iter().copied()))
                .unwrap()
            }
            let budget = ResourceBudget::fixed(512 << 20).unwrap();
            let config = RuntimeConfig::read(std::path::Path::new(
                &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"),
            ))
            .unwrap();
            let factory = recovery_factory(&config, budget.clone()).unwrap();
            let store = NativeCompilerStore::begin(&config, Frontier::Facts, &factory)
                .await
                .unwrap();
            let connection = store
                .recovery_scope()
                .unwrap()
                .client()
                .connection_completion()
                .unwrap();
            let reader = crate::NativeReader::private(store.client.clone()).with_budget(&budget);
            let result = std::panic::AssertUnwindSafe(async {
                let model = d::model()?;
                let relation = Relation::of::<PairOutcome>();
                let contribution = store
                    .begin_contribution(ContributionSpec {
                        captured_binding: None,
                        producer: format!("pair-batch-control-{}", store.attempt.hex()),
                        profile: OutputProfile::Specific(d::stages::Profile::Behavioral),
                        model: model.digest(),
                        implementation: ContentHash::of(b"pair-batch-physical-control"),
                        configuration: None,
                        inputs: vec![],
                        outputs: [relation.name().into()].into(),
                    })
                    .await?;
                let rows = (0..4096)
                    .map(|index| PairOutcome {
                        invocation: nominal(store.attempt, "invocation", 0),
                        attempt: nominal(store.attempt, "attempt", 0),
                        caller_origin: nominal(store.attempt, "origin", 0),
                        callee_origin: nominal(store.attempt, "origin", 1),
                        caller: nominal(store.attempt, "caller", index),
                        callee: nominal(store.attempt, "callee", index),
                        disposition: PairDisposition::Disjoint,
                        witness: None,
                        reason: None,
                    })
                    .collect();
                let batch = Batch::new(&model, rows, &budget)?.into_checked()?;
                let mut ingress = store.ingress(contribution, &budget)?;
                let phase = crate::phase::Phase::begin("native_pair_outcome_4096_ingress");
                let accepted = ingress.accept(batch).await?;
                let committed = accepted.await_committed().await;
                phase.finish_result(&committed);
                let committed = committed?;
                assert_eq!(batch_spans.load(Ordering::SeqCst), 1,
                    "the accepted store-runtime driver retains its submitting subscriber");
                assert_eq!(committed.sequence, 0);
                assert!(!committed.effects.entries().is_empty());
                assert!(committed.effects.entries().iter().all(|effect| {
                    effect.disposition == crate::control::EffectDisposition::Committed
                }));
                tracing::info!(target: "lctx_phase", observation = "native_pair_outcome_batch_committed",
                    logical_rows = 4096, physical_effects = committed.effects.entries().len());
                let report = ingress.finish_report().await;
                assert!(report.completion.is_satisfied());
                report.operation?;
                let mut variables = Variables::new();
                variables.insert("owner", RecordId::new("compiler_contribution", contribution.hex()));
                variables.insert("relation", PairOutcome::NAME);
                let persisted = reader.query::<Vec<u64>>(
                    "SELECT VALUE count() FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation GROUP ALL",
                    variables,
                ).await?;
                assert_eq!(persisted, vec![4096], "the exact contribution owns every committed physical row");
                Ok::<(), ModelError>(())
            })
            .catch_unwind()
            .await;
            let mut completion = d::completion::Completion::default();
            completion.step("pair batch reader close", reader.close().await);
            completion.merge(store.abandon_finality_report().await);
            let closed = d::completion::complete::<()>(Ok(()), completion);
            drop(reader);
            drop(store);
            drop(factory);
            if closed.is_ok() {
                connection.wait().await.unwrap();
                drop(connection);
                assert_eq!(budget.reserved(), 0);
            }
            closed.unwrap();
            match result {
                Ok(result) => result.unwrap(),
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }
        .with_tracing_dispatch(dispatch)
        .await;
    }
    #[tokio::test]
    async fn native_reused_ingress_spans_pending_source_replay_and_later_refusal() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        let factory = recovery_factory(&config, budget.clone()).unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts, &factory)
            .await
            .unwrap();
        let connection = store
            .recovery_scope()
            .unwrap()
            .client()
            .connection_completion()
            .unwrap();
        let reader = crate::NativeReader::private(store.client.clone()).with_budget(&budget);
        let result = std::panic::AssertUnwindSafe(async {
            let model = d::model()?;
            let source_relation = Relation::of::<d::source::SourceArtifact>();
            let chunk_relation = Relation::of::<d::artifact::ArtifactChunk>();
            let contribution = store
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: format!("ingress-reuse-{}", store.attempt.hex()),
                    profile: OutputProfile::Specific(d::stages::Profile::Catalog),
                    model: model.digest(),
                    implementation: ContentHash::of(b"ingress-reuse-control"),
                    configuration: None,
                    inputs: vec![],
                    outputs: [source_relation.name().into(), chunk_relation.name().into()].into(),
                })
                .await?;
            let input = d::input::InputRevision::from_entries(vec![])?;
            let mut bytes = vec![9; lctx_native::originals::CHUNK_BYTES + 7];
            bytes[..32].copy_from_slice(&store.attempt.0);
            let source =
                d::source::SourceArtifact::from_bytes(input.id(), "ingress.py".into(), &bytes)?;
            let chunks = d::artifact::ArtifactChunk::split(&source, &bytes)?.collect::<Vec<_>>();
            let mut ingress = store.ingress(contribution, &budget)?;
            // Deliberately arrive before the source header: the next batch must
            // drain the pending original using the same writer's preparation.
            let first = ingress
                .accept(Batch::new(&model, chunks.clone(), &budget)?.into_checked()?)
                .await?;
            let second = ingress
                .accept(Batch::new(&model, vec![source.clone()], &budget)?.into_checked()?)
                .await?;
            for (sequence, accepted) in [first, second].into_iter().enumerate() {
                let committed = accepted.await_committed().await?;
                assert_eq!(committed.sequence, sequence as u64);
                assert!(!committed.effects.entries().is_empty());
                assert!(committed.effects.entries().iter().all(
                    |effect| effect.disposition == crate::control::EffectDisposition::Committed
                ));
            }
            let replay = ingress
                .accept(Batch::new(&model, chunks, &budget)?.into_checked()?)
                .await?;
            assert_eq!(replay.await_committed().await?.sequence, 2);
            let source_id = lctx_native::originals::layout_id(d::graph::EntityId::of(source.id()))?;
            // Completion keeps its independent writer and proves exact source EOF.
            crate::control::with_recovery_scope(
                store.recovery_scope()?,
                lctx_native::originals::complete_selected(
                    &reader,
                    Some(store.attempt),
                    std::slice::from_ref(&source_id),
                    &budget,
                ),
            )
            .await?;
            let refused = ingress
                .accept(
                    Batch::<d::analytics::QualityStep>::new(&model, vec![], &budget)?
                        .into_checked()?,
                )
                .await?;
            assert_eq!(refused.sequence(), 3);
            assert!(
                refused.await_committed().await.is_err(),
                "later output validation is mandatory"
            );
            let report = ingress.finish_report().await;
            assert!(report.operation.is_err());
            assert!(
                report.completion.is_satisfied(),
                "acknowledged refusal still joins its owner"
            );
            let ranges = lctx_native::originals::read_ranges(
                &reader,
                &[(source_id, 0, bytes.len())],
                &budget,
            )
            .await?;
            assert_eq!(
                ranges,
                vec![bytes],
                "later refusal preserves earlier committed source bytes"
            );
            Ok::<(), ModelError>(())
        })
        .catch_unwind()
        .await;
        let mut completion = d::completion::Completion::default();
        completion.step("ingress control reader close", reader.close().await);
        completion.merge(store.abandon_finality_report().await);
        let closed = d::completion::complete::<()>(Ok(()), completion);
        drop(reader);
        drop(store);
        drop(factory);
        if closed.is_ok() {
            connection.wait().await.unwrap();
            drop(connection);
            assert_eq!(budget.reserved(), 0);
        }
        closed.unwrap();
        match result {
            Ok(result) => result.unwrap(),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
    #[tokio::test]
    async fn admission_precedes_worker_poll_and_drop_keeps_global_drain_owned() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16384).unwrap();
        let ingress = store
            .ingress(ContentHash::of(b"contribution"), &budget)
            .unwrap();
        assert!(budget.reserved() >= 4096);
        let mut drain = Box::pin(store.drain());
        assert!(drain.as_mut().now_or_never().is_none());
        drop(ingress);
        drain.await.unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn acceptance_is_not_success_and_failed_driver_cannot_finish_successfully() {
        let store = local_store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut ingress = store
            .ingress(ContentHash::of(b"unknown contribution"), &budget)
            .unwrap();
        let batch = Batch::<d::analytics::QualityStep>::new(&d::model().unwrap(), vec![], &budget)
            .unwrap()
            .into_checked()
            .unwrap();
        let accepted = ingress.accept(batch).await.unwrap();
        assert_eq!(accepted.sequence(), 0);
        assert!(accepted.await_committed().await.is_err());
        let drained = ingress.finish_report().await;
        assert!(
            drained.operation.is_err(),
            "the joined driver's write refusal remains an operation error"
        );
        assert!(
            drained.completion.is_satisfied(),
            "a conclusive write refusal is not a failed driver join"
        );
        assert!(d::completion::complete(drained.operation, drained.completion).is_err());
        assert_eq!(budget.reserved(), 0);
        d::completion::complete::<()>(Ok(()), store.drain_finality_report().await).unwrap();
    }
    #[tokio::test]
    async fn lost_acknowledgement_and_failed_driver_join_remain_unconfirmed() {
        let (sender, result) = oneshot::channel();
        drop(sender);
        let sequence = IngressSequence {
            sequence: 7,
            result,
        };
        let Err(error) = sequence.await_committed().await else {
            panic!("lost acknowledgement cannot commit")
        };
        assert!(matches!(
            error,
            ModelError::Infrastructure {
                class: Infrastructure::Unconfirmed,
                ..
            }
        ));

        let terminal: tokio::task::JoinHandle<Result<(), ModelError>> = tokio::spawn(async {
            panic!("controlled ingress driver panic");
        });
        let ingress = NativeIngress {
            sender: None,
            terminal: Some(terminal),
            next: 0,
            stopped: Arc::new(AtomicBool::new(false)),
        };
        let report = ingress.finish_report().await;
        assert_eq!(
            report.completion.remote,
            d::completion::RemoteState::Unknown
        );
        assert!(!report.completion.is_satisfied());
        assert_eq!(report.completion.failures.len(), 1);
        assert_eq!(
            report.completion.failures[0].step,
            "native ingress driver join"
        );
        let ModelError::Operations(errors) = &report.completion.failures[0].error else {
            panic!("the failed join retains its typed cause and uncertainty");
        };
        let ModelError::Cause(cause) = &errors.primary else {
            panic!("original join error")
        };
        assert!(
            cause
                .downcast_ref::<tokio::task::JoinError>()
                .unwrap()
                .is_panic()
        );
        assert!(d::completion::complete(report.operation, report.completion).is_err());
    }
}
