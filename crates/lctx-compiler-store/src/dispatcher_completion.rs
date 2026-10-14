//! Operation delivery and actual dispatcher ownership are independent observations.
use lctx_model::domain::{
    ModelError,
    completion::{Completion, Finalization, LocalState, RemoteState},
};
use std::{error::Error, sync::Arc};

#[derive(Clone, Default)]
pub(crate) struct OperationResult {
    error: Option<Arc<ModelError>>,
    orphaned: bool,
    remote_unknown: bool,
}

impl OperationResult {
    pub(crate) fn deliver<T>(
        answer: tokio::sync::oneshot::Sender<Result<T, ModelError>>,
        mut result: Result<T, ModelError>,
    ) -> Self {
        let mut observed = Self::default();
        if let Err(error) = &result {
            observed.remote_unknown = unknown(error);
            if contains_completion(error) {
                // Preserve the original owned error (including panic payloads) for both
                // the caller and the ownership observation; never copy Display text.
                let original = match result {
                    Err(ModelError::SharedCause(error)) => error,
                    Err(error) => Arc::new(error),
                    Ok(_) => unreachable!(),
                };
                result = Err(ModelError::SharedCause(original.clone()));
                observed.error = Some(original);
            }
        }
        if let Err(Err(error)) = answer.send(result) {
            observed.error = Some(observed.error.unwrap_or_else(|| Arc::new(error)));
            observed.orphaned = true;
        }
        observed
    }

    pub(crate) fn retained(&self) -> bool {
        self.error.is_some() || self.remote_unknown
    }

    pub(crate) fn finality(&self, report: &mut Completion) {
        if self.remote_unknown {
            report.remote = RemoteState::Unknown;
        }
        if let Some(error) = &self.error {
            let mut next_failure = 0;
            collect_finality(error.as_ref(), error, report, &mut next_failure);
        }
    }

    pub(crate) fn append_diagnostics(&self, stage: &'static str, report: &mut Completion) {
        if self.orphaned
            && let Some(error) = &self.error
        {
            report.step(stage, Err(ModelError::SharedCause(error.clone())));
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct DispatcherReport {
    operations: Vec<OperationResult>,
    joins: Vec<Arc<ModelError>>,
}
impl DispatcherReport {
    pub(crate) fn joined(&mut self, result: Result<OperationResult, tokio::task::JoinError>) {
        match result {
            Ok(operation) if operation.retained() => self.operations.push(operation),
            Ok(_) => {}
            Err(error) => self
                .joins
                .push(Arc::new(ModelError::Cause(Box::new(error)))),
        }
    }
    pub(crate) fn finality(&self) -> Completion {
        let mut report = Completion::default();
        for operation in &self.operations {
            operation.finality(&mut report);
        }
        for error in &self.joins {
            report.step(
                "native bridge task join",
                Err(ModelError::SharedCause(error.clone())),
            );
        }
        report
    }
    pub(crate) fn append_diagnostics(&self, report: &mut Completion) {
        for operation in &self.operations {
            operation.append_diagnostics("native bridge operation", report);
        }
    }
}

fn children<'a>(
    error: &'a (dyn Error + 'static),
    mut visit: impl FnMut(&'a (dyn Error + 'static)),
) {
    if let Some(ModelError::Completion(outcome)) = error.downcast_ref::<ModelError>() {
        if let Some(primary) = &outcome.primary {
            visit(primary.as_ref());
        }
        for failure in &outcome.completion.failures {
            visit(&failure.error);
        }
    } else if let Some(ModelError::SharedCause(error)) = error.downcast_ref::<ModelError>() {
        visit(error.as_ref());
    } else if let Some(ModelError::Cause(error)) = error.downcast_ref::<ModelError>() {
        visit(error.as_ref());
    } else if let Some(error) = error.downcast_ref::<datafusion::error::DataFusionError>() {
        use datafusion::error::DataFusionError;
        match error {
            DataFusionError::Collection(errors) => {
                for error in errors {
                    visit(error);
                }
            }
            DataFusionError::External(error) => visit(error.as_ref()),
            DataFusionError::Context(_, error) => visit(error.as_ref()),
            DataFusionError::Shared(error) => visit(error.as_ref()),
            error => {
                if let Some(source) = error.source() {
                    visit(source);
                }
            }
        }
    } else if let Some(source) = error.source() {
        visit(source);
    }
}
fn contains_completion(error: &(dyn Error + 'static)) -> bool {
    let mut found = matches!(
        error.downcast_ref::<ModelError>(),
        Some(ModelError::Completion(_))
    );
    if !found {
        children(error, |child| found |= contains_completion(child));
    }
    found
}
fn unknown(error: &(dyn Error + 'static)) -> bool {
    let mut found = matches!(
        error.downcast_ref::<ModelError>(),
        Some(ModelError::Infrastructure {
            class: lctx_model::domain::Infrastructure::Unconfirmed,
            ..
        })
    );
    children(error, |child| found |= unknown(child));
    found
}
/// A reference into an immutable owned error chain, preserving the exact selected
/// finalizer rather than relabelling the operation's primary error as cleanup.
#[derive(Debug)]
struct FinalizationCause {
    operation: Arc<ModelError>,
    index: usize,
}
impl std::fmt::Display for FinalizationCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.failure(), formatter)
    }
}
impl Error for FinalizationCause {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.failure())
    }
}
impl FinalizationCause {
    fn failure(&self) -> &ModelError {
        let mut index = self.index;
        nth_finalization(self.operation.as_ref(), &mut index)
            .expect("immutable finalization reference")
    }
}
fn nth_finalization<'a>(
    error: &'a (dyn Error + 'static),
    index: &mut usize,
) -> Option<&'a ModelError> {
    if let Some(ModelError::Completion(outcome)) = error.downcast_ref::<ModelError>() {
        for failure in &outcome.completion.failures {
            if *index == 0 {
                return Some(&failure.error);
            }
            *index -= 1;
        }
    }
    let mut found = None;
    children(error, |child| {
        if found.is_none() {
            found = nth_finalization(child, index);
        }
    });
    found
}
fn collect_finality(
    error: &(dyn Error + 'static),
    original: &Arc<ModelError>,
    report: &mut Completion,
    next_failure: &mut usize,
) {
    if let Some(ModelError::Completion(outcome)) = error.downcast_ref::<ModelError>() {
        let actual = &outcome.completion;
        if actual.local == LocalState::Outstanding {
            report.local = actual.local;
        }
        if actual.remote == RemoteState::Unknown {
            report.remote = actual.remote;
        }
        report.storage.extend(actual.storage.clone());
        report.committed.extend(actual.committed.clone());
        for failure in &actual.failures {
            // The original owned chain retains the exact finalizer error and any
            // non-cloneable payload; a plain acknowledged primary adds no failure here.
            report.failures.push(Finalization {
                step: failure.step,
                error: ModelError::Cause(Box::new(FinalizationCause {
                    operation: original.clone(),
                    index: *next_failure,
                })),
            });
            *next_failure += 1;
        }
    }
    children(error, |child| {
        collect_finality(child, original, report, next_failure)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::completion::{self, StorageState};

    #[test]
    fn acknowledged_refusal_is_diagnostic_only_for_a_lost_receiver() {
        for dropped in [false, true] {
            let (sender, receiver) = tokio::sync::oneshot::channel::<Result<(), ModelError>>();
            let receiver = (!dropped).then_some(receiver);
            let operation = OperationResult::deliver(sender, Err(ModelError::Schema("refused")));
            if let Some(receiver) = receiver {
                assert!(matches!(
                    receiver.blocking_recv().unwrap(),
                    Err(ModelError::Schema("refused"))
                ));
            }
            let mut finality = Completion::default();
            operation.finality(&mut finality);
            assert!(finality.failures.is_empty());
            assert_eq!(finality.local, LocalState::Terminal);
            assert_eq!(finality.remote, RemoteState::Confirmed);
            operation.append_diagnostics("operation", &mut finality);
            assert_eq!(finality.failures.len(), usize::from(dropped));
        }
    }

    #[test]
    fn live_and_lost_receivers_preserve_complete_nested_finality() {
        for dropped in [false, true] {
            let mut inner = Completion {
                local: LocalState::Outstanding,
                remote: RemoteState::Unknown,
                ..Default::default()
            };
            inner.storage.push(StorageState::Removed("scratch".into()));
            inner.storage.push(StorageState::Orphan("held".into()));
            inner.committed("publication", "exact");
            inner.step("actual cleanup", Err(ModelError::Schema("cleanup failed")));
            let error =
                completion::complete::<()>(Err(ModelError::Schema("refused")), inner).unwrap_err();
            let error = ModelError::Cause(Box::new(datafusion::error::DataFusionError::Context(
                "context".into(),
                Box::new(datafusion::error::DataFusionError::External(Box::new(
                    error,
                ))),
            )));
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let receiver = (!dropped).then_some(receiver);
            let operation = OperationResult::deliver(sender, Err::<(), _>(error));
            let mut actual = Completion::default();
            operation.finality(&mut actual);
            assert_eq!(actual.local, LocalState::Outstanding);
            assert_eq!(actual.remote, RemoteState::Unknown);
            assert_eq!(actual.storage.len(), 2);
            assert_eq!(actual.committed[0].identity, "exact");
            assert_eq!(actual.failures.len(), 1);
            assert_eq!(actual.failures[0].step, "actual cleanup");
            assert!(
                actual.failures[0]
                    .error
                    .to_string()
                    .contains("cleanup failed")
            );
            assert!(matches!(
                actual.failures[0].error.primary(),
                Some(ModelError::Schema("cleanup failed"))
            ));
            if let Some(receiver) = receiver {
                assert!(
                    !receiver
                        .blocking_recv()
                        .unwrap()
                        .unwrap_err()
                        .permits_storage_cleanup()
                );
            }
        }
    }
}
