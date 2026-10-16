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
        let terminal = tokio::spawn(crate::control::with_retry_context(
            cancellation,
            async move {
                let _retained = retained;
                let result = lease
                    .within(producing_future(producing, async {
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
                                store.write_checked_batch(&contribution, &accepted.batch, &budget),
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
            },
        ));
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
