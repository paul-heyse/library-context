//! Typed native wrappers enrich the model-owned, inspect-only receipt projection.
use crate::control::{NativeAttemptError, NativeLifecycleError, NativeRequestError};
use lctx_model::domain::{
    ModelError,
    diagnostic::{
        DiagnosticCause, DiagnosticClass, DiagnosticCode, DiagnosticIdentity, FailureDiagnostic,
        IdentityKind, InspectionRoute, IssuanceIdentity, LifecycleKind,
    },
};

pub fn failure(stage: &str, error: &ModelError) -> FailureDiagnostic {
    FailureDiagnostic::project(stage, error, |error| {
        let identity = |kind, value: lctx_model::domain::ContentHash| DiagnosticIdentity {
            kind,
            value: value.hex(),
        };
        let (code, identities, issuance, lifecycle, inspect) =
            if let Some(error) = error.downcast_ref::<NativeRequestError>() {
                (
                    DiagnosticCode::NativeRequest,
                    vec![
                        identity(IdentityKind::Operation, error.request.operation),
                        identity(IdentityKind::Request, error.request.request),
                    ],
                    error.request.issuance,
                    None,
                    Some(InspectionRoute::NativeRequest),
                )
            } else if let Some(error) = error.downcast_ref::<NativeAttemptError>() {
                (
                    DiagnosticCode::NativeAttempt,
                    vec![identity(IdentityKind::Attempt, error.attempt)],
                    error.issuance,
                    None,
                    Some(InspectionRoute::NativeAttempt),
                )
            } else {
                let error = error.downcast_ref::<NativeLifecycleError>()?;
                let lifecycle = match error.kind {
                    "reader pin" => Some(LifecycleKind::ReaderPin),
                    "backup hold" => Some(LifecycleKind::BackupHold),
                    "retirement invocation" => Some(LifecycleKind::RetirementInvocation),
                    "retirement successor" => Some(LifecycleKind::RetirementSuccessor),
                    "cleanup obligation" => Some(LifecycleKind::CleanupObligation),
                    "history invocation" => Some(LifecycleKind::HistoryInvocation),
                    _ => None,
                };
                (
                    DiagnosticCode::NativeLifecycle,
                    vec![identity(IdentityKind::Lifecycle, error.identity)],
                    error.issuance,
                    lifecycle,
                    lifecycle.map(|_| InspectionRoute::NativeLifecycle),
                )
            };
        Some(DiagnosticCause {
            class: DiagnosticClass::Native,
            code,
            stage: String::new(),
            identities,
            issuance: Some(IssuanceIdentity {
                generation: issuance.generation.hex(),
                era: issuance.era,
            }),
            lifecycle,
            inspect,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{IssuanceEra, NativeRequest};
    use lctx_model::domain::{
        ContentHash, Infrastructure,
        completion::{Completion, OperationFailure},
    };
    #[test]
    fn actual_wrappers_preserve_identity_nested_failure_and_absent_identity() {
        let issuance = IssuanceEra {
            generation: ContentHash([1; 32]),
            era: 7,
        };
        let request = NativeRequest {
            issuance,
            operation: ContentHash([2; 32]),
            request: ContentHash([3; 32]),
        };
        let mut completion = Completion::default();
        completion.cleanup(
            "held-storage",
            Err(ModelError::Cause(Box::new(NativeAttemptError {
                attempt: ContentHash([4; 32]),
                issuance,
                source: ModelError::infrastructure(
                    Infrastructure::Unconfirmed,
                    "lost acknowledgement",
                ),
            }))),
        );
        let error = ModelError::Completion(Box::new(OperationFailure {
            primary: Some(Box::new(ModelError::Cause(Box::new(NativeRequestError {
                request,
                source: ModelError::Conflict("refused"),
            })))),
            completion,
        }));
        let value = failure("finalize", &error);
        let request = value
            .causes
            .iter()
            .find(|c| c.code == DiagnosticCode::NativeRequest)
            .unwrap();
        assert_eq!(request.identities[0].value, "02".repeat(32));
        assert_eq!(request.identities[1].value, "03".repeat(32));
        assert_eq!(request.issuance.as_ref().unwrap().era, 7);
        assert_eq!(
            request.issuance.as_ref().unwrap().generation,
            "01".repeat(32)
        );
        assert_eq!(request.inspect, Some(InspectionRoute::NativeRequest));
        let attempt = value
            .causes
            .iter()
            .find(|c| c.code == DiagnosticCode::NativeAttempt)
            .unwrap();
        assert_eq!(attempt.stage, "private storage cleanup");
        assert_eq!(attempt.identities[0].value, "04".repeat(32));
        assert!(
            value
                .causes
                .iter()
                .any(|c| c.code == DiagnosticCode::Unconfirmed)
        );
        let opaque = failure(
            "capture",
            &ModelError::Cause(Box::new(std::io::Error::other("unadmitted"))),
        );
        assert!(
            opaque
                .causes
                .iter()
                .all(|c| c.identities.is_empty() && c.issuance.is_none())
        );
        assert_eq!(opaque.causes.last().unwrap().code, DiagnosticCode::Opaque);
    }
    #[test]
    fn lifecycle_kind_is_typed_and_unknown_kind_has_no_guessed_route() {
        let issuance = IssuanceEra {
            generation: ContentHash([1; 32]),
            era: 7,
        };
        for (kind, expected) in [
            ("reader pin", Some(LifecycleKind::ReaderPin)),
            ("future owner", None),
        ] {
            let error = ModelError::Cause(Box::new(NativeLifecycleError {
                kind,
                identity: ContentHash([5; 32]),
                issuance,
                source: ModelError::Conflict("refused"),
            }));
            let diagnostic = failure("lifecycle", &error);
            let cause = diagnostic
                .causes
                .iter()
                .find(|c| c.code == DiagnosticCode::NativeLifecycle)
                .unwrap();
            assert_eq!(cause.lifecycle, expected);
            assert_eq!(
                cause.inspect,
                expected.map(|_| InspectionRoute::NativeLifecycle)
            );
            assert_eq!(cause.identities[0].value, "05".repeat(32));
        }
    }
}
