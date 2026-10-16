//! Effect-free operation completion. Workflow owners perform and report each finalization.
use super::ModelError;

/// The original thread panic payload remains owned, including non-string payloads.
/// A mutex supplies Sync for the standard library's Send-only payload; no task is run here.
pub struct ThreadPanic {
    pub owner: &'static str,
    payload: std::sync::Mutex<Box<dyn std::any::Any + Send>>,
}
impl ThreadPanic {
    pub fn new(owner: &'static str, payload: Box<dyn std::any::Any + Send>) -> Self {
        Self {
            owner,
            payload: std::sync::Mutex::new(payload),
        }
    }
}
impl std::fmt::Display for ThreadPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} thread panicked", self.owner)?;
        if let Ok(payload) = self.payload.lock() {
            if let Some(message) = payload.downcast_ref::<String>() {
                write!(f, ": {message}")?;
            } else if let Some(message) = payload.downcast_ref::<&'static str>() {
                write!(f, ": {message}")?;
            }
        }
        Ok(())
    }
}
impl std::fmt::Debug for ThreadPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for ThreadPanic {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalState {
    #[default]
    Terminal,
    Outstanding,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteState {
    #[default]
    Confirmed,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageState {
    Removed(String),
    Orphan(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedEffect {
    pub kind: &'static str,
    pub identity: String,
}
#[derive(Debug)]
pub struct Finalization {
    pub step: &'static str,
    pub error: ModelError,
}
#[derive(Debug, Default)]
pub struct Completion {
    pub local: LocalState,
    pub remote: RemoteState,
    pub storage: Vec<StorageState>,
    pub committed: Vec<CommittedEffect>,
    pub failures: Vec<Finalization>,
}
impl Completion {
    /// Whether this owner report has successful, conclusive finality on every axis.
    /// This does not change a primary operation failure or establish later work's finality.
    pub fn is_satisfied(&self) -> bool {
        self.satisfied()
    }
    /// Compose actual owner reports without converting successful metadata into `()`.
    /// Outstanding work and unknown acknowledgements remain sticky; outcomes retain order.
    pub fn merge(&mut self, report: Self) {
        if report.local == LocalState::Outstanding {
            self.local = report.local;
        }
        if report.remote == RemoteState::Unknown {
            self.remote = report.remote;
        }
        self.storage.extend(report.storage);
        self.committed.extend(report.committed);
        self.failures.extend(report.failures);
    }
    pub fn step(&mut self, step: &'static str, result: Result<(), ModelError>) {
        if let Err(error) = result {
            self.observe(&error);
            self.failures.push(Finalization { step, error });
        }
    }
    pub fn cleanup(&mut self, identity: impl Into<String>, result: Result<(), ModelError>) {
        let identity = identity.into();
        if result.is_ok() {
            self.storage.push(StorageState::Removed(identity));
        } else {
            self.storage.push(StorageState::Orphan(identity));
        }
        self.step("private storage cleanup", result);
    }
    pub fn committed(&mut self, kind: &'static str, identity: impl Into<String>) {
        self.committed.push(CommittedEffect {
            kind,
            identity: identity.into(),
        });
    }
    fn observe(&mut self, error: &ModelError) {
        if matches!(
            error,
            ModelError::Infrastructure {
                class: super::Infrastructure::Unconfirmed,
                ..
            }
        ) {
            self.remote = RemoteState::Unknown;
        }
        if let ModelError::SharedCause(error) = error {
            self.observe(error);
        }
        if let ModelError::Operations(errors) = error {
            self.observe(&errors.primary);
            for error in &errors.related {
                self.observe(error);
            }
        }
        if let ModelError::Cause(error) = error
            && let Some(error) = contextual_model(error.as_ref())
        {
            self.observe(error);
        }
        if let ModelError::Completion(outcome) = error {
            if outcome.completion.local == LocalState::Outstanding {
                self.local = LocalState::Outstanding;
            }
            if outcome.completion.remote == RemoteState::Unknown {
                self.remote = RemoteState::Unknown;
            }
        }
    }
    fn satisfied(&self) -> bool {
        self.failures.is_empty()
            && self.local == LocalState::Terminal
            && self.remote == RemoteState::Confirmed
            && !self
                .storage
                .iter()
                .any(|s| matches!(s, StorageState::Orphan(_)))
    }
}
/// Owned operation errors in observation order. Actual finality remains in each
/// nested typed Completion rather than being invented for a related diagnostic.
#[derive(Debug)]
pub struct OperationErrors {
    pub primary: ModelError,
    pub related: Vec<ModelError>,
}
impl std::fmt::Display for OperationErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.primary)?;
        for error in &self.related {
            write!(formatter, "; related operation: {error}")?;
        }
        Ok(())
    }
}
impl std::error::Error for OperationErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.primary)
    }
}
#[derive(Debug)]
pub struct OperationFailure {
    pub primary: Option<Box<ModelError>>,
    pub completion: Completion,
}
impl std::fmt::Display for OperationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(primary) = &self.primary {
            write!(f, "{primary}")?;
        } else {
            write!(f, "operation finalization failed")?;
        }
        for failure in &self.completion.failures {
            write!(f, "; {}: {}", failure.step, failure.error)?;
        }
        write!(
            f,
            "; local {:?}, remote {:?}",
            self.completion.local, self.completion.remote
        )?;
        for storage in &self.completion.storage {
            write!(f, "; storage {storage:?}")?;
        }
        for effect in &self.completion.committed {
            write!(f, "; committed {} {}", effect.kind, effect.identity)?;
        }
        Ok(())
    }
}
impl std::error::Error for OperationFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary
            .as_deref()
            .map(|e| e as _)
            .or_else(|| self.completion.failures.first().map(|e| &e.error as _))
    }
}
pub fn complete<T>(
    result: Result<T, ModelError>,
    mut completion: Completion,
) -> Result<T, ModelError> {
    match result {
        Ok(value) if completion.satisfied() => Ok(value),
        Err(error)
            if completion.satisfied()
                && completion.storage.is_empty()
                && completion.committed.is_empty() =>
        {
            Err(error)
        }
        result => {
            let primary = result.err().map(Box::new);
            if let Some(error) = &primary {
                completion.observe(error);
            }
            Err(ModelError::Completion(Box::new(OperationFailure {
                primary,
                completion,
            })))
        }
    }
}
// Foreign context chains can contain several wrappers. Stop at the first typed
// ModelError: its own interpretation must precede Error::source, which can skip
// completion metadata or expose a cleanup failure when there was no primary.
fn contextual_model<'a>(
    mut error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a ModelError> {
    loop {
        if let Some(model) = error.downcast_ref::<ModelError>() {
            return Some(model);
        }
        error = error.source()?;
    }
}
impl ModelError {
    /// Optional fallback for one cause must not hide other operation diagnostics.
    pub fn has_operation_group(&self) -> bool {
        match self {
            Self::Operations(_) => true,
            Self::SharedCause(error) => error.has_operation_group(),
            Self::Cause(error) => {
                contextual_model(error.as_ref()).is_some_and(Self::has_operation_group)
            }
            Self::Completion(outcome) => {
                outcome
                    .primary
                    .as_deref()
                    .is_some_and(Self::has_operation_group)
                    || outcome
                        .completion
                        .failures
                        .iter()
                        .any(|failure| failure.error.has_operation_group())
            }
            _ => false,
        }
    }
    /// Preserve the first operation and every related owned diagnostic. This does
    /// not assert that a task joined, a request was acknowledged or cleanup passed.
    pub fn alongside(self, related: Self) -> Self {
        match self {
            Self::Operations(mut errors) => {
                errors.related.push(related);
                Self::Operations(errors)
            }
            primary => Self::Operations(Box::new(OperationErrors {
                primary,
                related: vec![related],
            })),
        }
    }
    pub fn primary(&self) -> Option<&ModelError> {
        match self {
            Self::Operations(errors) => errors
                .primary
                .primary()
                .or_else(|| errors.related.iter().find_map(Self::primary)),
            Self::Completion(outcome) => outcome.primary.as_deref().and_then(Self::primary),
            Self::SharedCause(error) => error.primary(),
            Self::Cause(error) => match contextual_model(error.as_ref()) {
                Some(error) => error.primary(),
                None => Some(self),
            },
            other => Some(other),
        }
    }
    pub fn permits_storage_cleanup(&self) -> bool {
        match self {
            Self::Operations(errors) => {
                errors.primary.permits_storage_cleanup()
                    && errors.related.iter().all(Self::permits_storage_cleanup)
            }
            Self::Completion(outcome) => {
                outcome.completion.local == LocalState::Terminal
                    && outcome.completion.remote == RemoteState::Confirmed
                    && outcome
                        .primary
                        .as_deref()
                        .is_none_or(Self::permits_storage_cleanup)
                    && outcome
                        .completion
                        .failures
                        .iter()
                        .all(|f| f.error.permits_storage_cleanup())
            }
            Self::SharedCause(error) => error.permits_storage_cleanup(),
            Self::Cause(error) => {
                contextual_model(error.as_ref()).is_none_or(Self::permits_storage_cleanup)
            }
            Self::Infrastructure {
                class: super::Infrastructure::Unconfirmed,
                ..
            } => false,
            _ => true,
        }
    }
    pub fn has_committed_effect(&self) -> bool {
        match self {
            Self::Operations(errors) => {
                errors.primary.has_committed_effect()
                    || errors.related.iter().any(Self::has_committed_effect)
            }
            Self::Completion(outcome) => {
                !outcome.completion.committed.is_empty()
                    || outcome
                        .primary
                        .as_deref()
                        .is_some_and(Self::has_committed_effect)
                    || outcome
                        .completion
                        .failures
                        .iter()
                        .any(|f| f.error.has_committed_effect())
            }
            Self::SharedCause(error) => error.has_committed_effect(),
            Self::Cause(error) => {
                contextual_model(error.as_ref()).is_some_and(Self::has_committed_effect)
            }
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn related_operation_refusals_keep_primary_identity_without_inventing_finalizers() {
        let primary = std::sync::Arc::new(ModelError::Resource {
            owner: "operation",
            requested: 17,
            used: 0,
            limit: 16,
        });
        let error = ModelError::SharedCause(primary.clone())
            .alongside(ModelError::Invalid("acknowledged cancellation".into()));
        assert!(std::ptr::eq(error.primary().unwrap(), primary.as_ref()));
        assert!(error.permits_storage_cleanup());
        assert!(!error.has_committed_effect());
        let error = complete::<()>(Err(error), Completion::default()).unwrap_err();
        assert!(matches!(&error, ModelError::Operations(errors) if errors.related.len() == 1));
        assert!(error.to_string().contains("acknowledged cancellation"));
        let diagnostic =
            super::super::diagnostic::FailureDiagnostic::project("operation", &error, |_| None);
        assert_eq!(
            diagnostic
                .causes
                .iter()
                .map(|cause| cause.code)
                .collect::<Vec<_>>(),
            vec![
                super::super::diagnostic::DiagnosticCode::Context,
                super::super::diagnostic::DiagnosticCode::Context,
                super::super::diagnostic::DiagnosticCode::Resource,
                super::super::diagnostic::DiagnosticCode::Invalid,
            ]
        );
    }
    #[test]
    fn related_operation_preserves_nested_uncertainty_effects_and_true_finalizers() {
        let mut finality = Completion {
            local: LocalState::Outstanding,
            remote: RemoteState::Unknown,
            ..Default::default()
        };
        finality.committed("publication", "exact-effect");
        finality.cleanup("owned-scratch", Err(ModelError::Schema("cleanup refused")));
        let related = complete::<()>(
            Err(ModelError::Invalid("related operation".into())),
            finality,
        )
        .unwrap_err();
        let error = ModelError::Resource {
            owner: "primary",
            requested: 2,
            used: 0,
            limit: 1,
        }
        .alongside(related);
        assert!(matches!(
            error.primary(),
            Some(ModelError::Resource {
                owner: "primary",
                ..
            })
        ));
        assert!(!error.permits_storage_cleanup());
        assert!(error.has_committed_effect());
        // A successful owner report carries metadata, so composition observes
        // the group's nested uncertainty instead of returning it unchanged.
        let mut observed = Completion::default();
        observed.committed("observer", "completed-owner-effect");
        let error = complete::<()>(Err(error), observed).unwrap_err();
        let ModelError::Completion(outcome) = &error else {
            panic!("related uncertainty must remain observed")
        };
        assert_eq!(outcome.completion.local, LocalState::Outstanding);
        assert_eq!(outcome.completion.remote, RemoteState::Unknown);
        assert!(
            outcome.completion.failures.is_empty(),
            "observation does not relabel related operations as finalizers"
        );
        assert!(error.has_committed_effect());
        let ModelError::Operations(operations) = outcome.primary.as_deref().unwrap() else {
            panic!("owned operation group")
        };
        let ModelError::Completion(related) = &operations.related[0] else {
            panic!("original typed finality")
        };
        assert_eq!(
            related.completion.storage,
            [StorageState::Orphan("owned-scratch".into())]
        );
        assert_eq!(related.completion.failures.len(), 1);
        assert_eq!(related.completion.committed[0].identity, "exact-effect");
    }
    #[test]
    fn related_operation_primary_does_not_relabel_a_finalization_only_error() {
        let mut finality = Completion::default();
        finality.step("actual cleanup", Err(ModelError::Schema("cleanup refusal")));
        let cleanup = complete::<()>(Ok(()), finality).unwrap_err();
        assert!(cleanup.primary().is_none());
        let resource = std::sync::Arc::new(ModelError::Resource {
            owner: "operation",
            requested: 2,
            used: 0,
            limit: 1,
        });
        let error = cleanup.alongside(ModelError::SharedCause(resource.clone()));
        let wrapped = ModelError::Cause(Box::new(ModelError::SharedCause(std::sync::Arc::new(
            error,
        ))));
        assert!(std::ptr::eq(wrapped.primary().unwrap(), resource.as_ref()));
        assert!(wrapped.has_operation_group());
        assert!(wrapped.to_string().contains("actual cleanup"));
    }
    #[derive(Debug)]
    struct Context(ModelError);
    impl std::fmt::Display for Context {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "request context: {}", self.0)
        }
    }
    impl std::error::Error for Context {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }
    fn contextual(error: ModelError) -> ModelError {
        ModelError::Cause(Box::new(Context(error)))
    }
    #[derive(Debug)]
    struct ForeignContext(Box<dyn std::error::Error + Send + Sync>);
    impl std::fmt::Display for ForeignContext {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "foreign context: {}", self.0)
        }
    }
    impl std::error::Error for ForeignContext {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(self.0.as_ref())
        }
    }
    #[test]
    fn multiple_foreign_contexts_preserve_the_first_model_completion() {
        let original = std::sync::Arc::new(ModelError::Schema("exact primary"));
        for has_primary in [true, false] {
            let mut finality = Completion {
                local: LocalState::Outstanding,
                remote: RemoteState::Unknown,
                ..Default::default()
            };
            finality.committed("publication", "exact-view");
            finality.step("drain", Err(ModelError::Schema("cleanup cause")));
            let result = if has_primary {
                Err(ModelError::SharedCause(original.clone()))
            } else {
                Ok(())
            };
            let wrapped = ModelError::Cause(Box::new(ForeignContext(Box::new(ForeignContext(
                Box::new(complete(result, finality).unwrap_err()),
            )))));
            assert_eq!(wrapped.primary().is_some(), has_primary);
            if has_primary {
                assert!(std::ptr::eq(wrapped.primary().unwrap(), original.as_ref()));
            }
            assert!(!wrapped.permits_storage_cleanup());
            assert!(wrapped.has_committed_effect());
            let mut aggregate = Completion::default();
            aggregate.step("request", Err(wrapped));
            assert_eq!(aggregate.local, LocalState::Outstanding);
            assert_eq!(aggregate.remote, RemoteState::Unknown);
            assert!(aggregate.committed.is_empty());
            let aggregated = complete(Ok(()), aggregate).unwrap_err();
            assert!(aggregated.has_committed_effect());
            assert!(aggregated.to_string().contains("exact-view"));
        }
    }
    #[test]
    fn request_context_preserves_completion_certainty_effects_and_primary_identity() {
        let original = std::sync::Arc::new(ModelError::Schema("exact primary"));
        let mut finality = Completion {
            local: LocalState::Outstanding,
            remote: RemoteState::Unknown,
            ..Default::default()
        };
        finality.committed("published manifest", "exact-view");
        let inner =
            complete::<()>(Err(ModelError::SharedCause(original.clone())), finality).unwrap_err();
        let wrapped = contextual(contextual(inner));
        assert!(std::ptr::eq(wrapped.primary().unwrap(), original.as_ref()));
        assert!(!wrapped.permits_storage_cleanup());
        assert!(wrapped.has_committed_effect());
        assert!(wrapped.to_string().contains("request context"));
        let mut aggregate = Completion::default();
        aggregate.step("request", Err(wrapped));
        assert_eq!(aggregate.local, LocalState::Outstanding);
        assert_eq!(aggregate.remote, RemoteState::Unknown);
        let error = complete(Ok(()), aggregate).unwrap_err();
        assert!(error.primary().is_none());
        assert!(!error.permits_storage_cleanup());
        assert!(error.has_committed_effect());
    }
    #[test]
    fn directly_boxed_completion_keeps_metadata_before_its_error_source() {
        for (local, remote) in [
            (LocalState::Outstanding, RemoteState::Confirmed),
            (LocalState::Terminal, RemoteState::Unknown),
        ] {
            let mut finality = Completion {
                local,
                remote,
                ..Default::default()
            };
            finality.committed("publication", "view");
            let wrapped = ModelError::Cause(Box::new(
                complete::<()>(Err(ModelError::Schema("primary")), finality).unwrap_err(),
            ));
            assert!(!wrapped.permits_storage_cleanup());
            assert!(wrapped.has_committed_effect());
            assert!(matches!(
                wrapped.primary(),
                Some(ModelError::Schema("primary"))
            ));
            let mut aggregate = Completion::default();
            aggregate.step("request", Err(wrapped));
            assert_eq!(aggregate.local, local);
            assert_eq!(aggregate.remote, remote);
        }
    }
    #[test]
    fn contextual_cleanup_only_and_opaque_causes_keep_their_meaning() {
        let mut finality = Completion::default();
        finality.step("drain", Err(ModelError::Schema("cleanup cause")));
        let wrapped = contextual(complete(Ok(()), finality).unwrap_err());
        assert!(wrapped.primary().is_none());
        assert!(wrapped.permits_storage_cleanup());
        assert!(!wrapped.has_committed_effect());
        let opaque = ModelError::Cause(Box::new(std::io::Error::other("foreign cause")));
        assert!(std::ptr::eq(opaque.primary().unwrap(), &opaque));
        assert!(opaque.permits_storage_cleanup());
        assert!(!opaque.has_committed_effect());
        let mut aggregate = Completion::default();
        aggregate.step("foreign", Err(opaque));
        assert_eq!(aggregate.local, LocalState::Terminal);
        assert_eq!(aggregate.remote, RemoteState::Confirmed);
    }
    #[test]
    fn primary_and_all_finalization_causes_survive() {
        let mut finality = Completion::default();
        finality.step(
            "drain",
            Err(ModelError::Codec("completed failed join".into())),
        );
        finality.cleanup("private-db", Err(ModelError::Schema("remove refusal")));
        let error = complete::<()>(
            Err(ModelError::Limit {
                owner: "test",
                limit: "rows",
                observed: 2,
                bound: 1,
            }),
            finality,
        )
        .unwrap_err();
        assert!(matches!(error.primary(), Some(ModelError::Limit { .. })));
        let ModelError::Completion(outcome) = error else {
            panic!()
        };
        assert_eq!(outcome.completion.failures.len(), 2);
        assert_eq!(outcome.completion.local, LocalState::Terminal);
        assert_eq!(outcome.completion.remote, RemoteState::Confirmed);
        assert_eq!(
            outcome.completion.storage,
            vec![StorageState::Orphan("private-db".into())]
        );
    }
    #[test]
    fn cleanup_only_and_unknown_acknowledgement_refuse_success() {
        let finality = Completion {
            remote: RemoteState::Unknown,
            ..Default::default()
        };
        assert!(complete(Ok(17), finality).is_err());
        let mut finality = Completion::default();
        finality.step("session", Err(ModelError::Codec("invalidated".into())));
        let error = complete(Ok(17), finality).unwrap_err();
        assert!(error.primary().is_none());
        assert!(matches!(
            complete::<()>(Err(ModelError::Schema("primary")), Completion::default()),
            Err(ModelError::Schema("primary"))
        ));
    }
    #[test]
    fn committed_effect_survives_later_failure() {
        let mut finality = Completion::default();
        finality.committed("sealed unselected database", "owned-handle");
        finality.step("staging removal", Err(ModelError::Codec("cleanup".into())));
        let error = complete(Ok(17), finality).unwrap_err();
        assert!(error.has_committed_effect());
        assert!(error.to_string().contains("owned-handle"));
    }

    #[test]
    fn merged_owner_reports_preserve_success_metadata_and_prior_uncertainty() {
        let mut finality = Completion {
            local: LocalState::Outstanding,
            remote: RemoteState::Unknown,
            ..Default::default()
        };
        let mut closed = Completion::default();
        closed.cleanup("released-source", Ok(()));
        closed.committed("unselected publication", "actual-handle");
        finality.merge(closed);
        let mut failed = Completion::default();
        failed.cleanup(
            "unreleased-reader",
            Err(ModelError::Schema("close refused")),
        );
        finality.merge(failed);
        let error = complete(Ok(()), finality).unwrap_err();
        assert!(error.has_committed_effect());
        let ModelError::Completion(outcome) = error else {
            panic!("combined owner outcome")
        };
        assert_eq!(outcome.completion.local, LocalState::Outstanding);
        assert_eq!(outcome.completion.remote, RemoteState::Unknown);
        assert_eq!(
            outcome.completion.storage,
            [
                StorageState::Removed("released-source".into()),
                StorageState::Orphan("unreleased-reader".into())
            ]
        );
        assert_eq!(outcome.completion.failures.len(), 1);
        assert!(matches!(
            outcome.completion.failures[0].error,
            ModelError::Schema("close refused")
        ));
    }
}
