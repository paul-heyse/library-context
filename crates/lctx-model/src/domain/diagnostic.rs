//! Bounded, presentation-independent failure evidence. Inspection never grants release authority.
use super::{Infrastructure, ModelError};
use serde::{Deserialize, Serialize};
use std::error::Error;

pub const MAX_CAUSES: usize = 16;
pub const MAX_STAGE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticClass {
    Model,
    Infrastructure,
    Native,
    Opaque,
}
/// Append new codes; do not repurpose an existing serialized meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    Completion,
    Context,
    Serving,
    Resource,
    Invalid,
    Limit,
    Frontier,
    Schema,
    Identity,
    Conflict,
    Codec,
    Transport,
    Unconfirmed,
    Refused,
    Contention,
    State,
    Contract,
    Io,
    Exhausted,
    NativeRequest,
    NativeAttempt,
    NativeLifecycle,
    Opaque,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    Operation,
    Request,
    Attempt,
    Lifecycle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticIdentity {
    pub kind: IdentityKind,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuanceIdentity {
    pub generation: String,
    pub era: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionRoute {
    NativeRequest,
    NativeAttempt,
    NativeLifecycle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleKind {
    ReaderPin,
    BackupHold,
    RetirementInvocation,
    RetirementSuccessor,
    CleanupObligation,
    HistoryInvocation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticCause {
    pub class: DiagnosticClass,
    pub code: DiagnosticCode,
    pub stage: String,
    pub identities: Vec<DiagnosticIdentity>,
    pub issuance: Option<IssuanceIdentity>,
    pub lifecycle: Option<LifecycleKind>,
    /// An inspection operation scoped by this node's identities; never a release capability.
    pub inspect: Option<InspectionRoute>,
}
impl DiagnosticCause {
    pub fn new(class: DiagnosticClass, code: DiagnosticCode) -> Self {
        Self {
            class,
            code,
            stage: String::new(),
            identities: vec![],
            issuance: None,
            lifecycle: None,
            inspect: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureDiagnostic {
    pub schema: u8,
    pub causes: Vec<DiagnosticCause>,
    pub truncated: bool,
}
impl FailureDiagnostic {
    /// The owner may enrich known wrappers. Arbitrary Display strings are deliberately excluded:
    /// they may contain secrets and are neither stable classification nor identity authority.
    pub fn project(
        stage: &str,
        error: &(dyn Error + 'static),
        enrich: impl Fn(&(dyn Error + 'static)) -> Option<DiagnosticCause>,
    ) -> Self {
        let mut result = Self {
            schema: 1,
            causes: Vec::new(),
            truncated: false,
        };
        result.visit(stage, error, &enrich);
        result
    }
    fn visit(
        &mut self,
        stage: &str,
        error: &(dyn Error + 'static),
        enrich: &impl Fn(&(dyn Error + 'static)) -> Option<DiagnosticCause>,
    ) {
        if self.causes.len() == MAX_CAUSES {
            self.truncated = true;
            return;
        }
        let mut cause = enrich(error).unwrap_or_else(|| model_cause(error));
        let mut end = stage.len().min(MAX_STAGE_BYTES);
        while !stage.is_char_boundary(end) {
            end -= 1;
        }
        cause.stage = stage[..end].into();
        self.truncated |= end < stage.len();
        self.causes.push(cause);
        if let Some(model) = error.downcast_ref::<ModelError>() {
            match model {
                ModelError::Operations(errors) => {
                    self.visit(stage, &errors.primary, enrich);
                    for error in &errors.related {
                        if self.causes.len() == MAX_CAUSES {
                            self.truncated = true;
                            break;
                        }
                        self.visit(stage, error, enrich);
                    }
                    return;
                }
                ModelError::Completion(outcome) => {
                    if let Some(primary) = &outcome.primary {
                        self.visit(stage, primary.as_ref(), enrich);
                    }
                    for failure in &outcome.completion.failures {
                        if self.causes.len() == MAX_CAUSES {
                            self.truncated = true;
                            break;
                        }
                        self.visit(failure.step, &failure.error, enrich);
                    }
                    return;
                }
                ModelError::SharedCause(cause) => {
                    self.visit(stage, cause.as_ref(), enrich);
                    return;
                }
                ModelError::Cause(cause) => {
                    self.visit(stage, cause.as_ref(), enrich);
                    return;
                }
                _ => {}
            }
        }
        if let Some(source) = error.source() {
            self.visit(stage, source, enrich);
        }
    }
}
fn model_cause(error: &(dyn Error + 'static)) -> DiagnosticCause {
    use DiagnosticCode as C;
    let Some(error) = error.downcast_ref::<ModelError>() else {
        return DiagnosticCause::new(DiagnosticClass::Opaque, C::Opaque);
    };
    let (class, code) = match error {
        ModelError::Infrastructure { class, .. } => (
            DiagnosticClass::Infrastructure,
            match class {
                Infrastructure::Transport => C::Transport,
                Infrastructure::Unconfirmed => C::Unconfirmed,
                Infrastructure::Refused => C::Refused,
                Infrastructure::Contention => C::Contention,
                Infrastructure::State => C::State,
                Infrastructure::Contract => C::Contract,
                Infrastructure::Io => C::Io,
                Infrastructure::Exhausted => C::Exhausted,
            },
        ),
        error => (
            DiagnosticClass::Model,
            match error {
                ModelError::Completion(_) => C::Completion,
                ModelError::Operations(_) | ModelError::SharedCause(_) | ModelError::Cause(_) => {
                    C::Context
                }
                ModelError::Serving(_) => C::Serving,
                ModelError::Resource { .. } => C::Resource,
                ModelError::Invalid(_) => C::Invalid,
                ModelError::Limit { .. } => C::Limit,
                ModelError::Frontier(_) => C::Frontier,
                ModelError::Schema(_) => C::Schema,
                ModelError::Identity(_) => C::Identity,
                ModelError::Conflict(_) => C::Conflict,
                ModelError::Codec(_) => C::Codec,
                ModelError::Infrastructure { .. } => unreachable!(),
            },
        ),
    };
    DiagnosticCause::new(class, code)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_finalization_and_unconfirmed_are_bounded_without_display_text() {
        let mut completion = super::super::completion::Completion::default();
        completion.step(
            "session close",
            Err(ModelError::infrastructure(
                Infrastructure::Unconfirmed,
                "secret",
            )),
        );
        let error = ModelError::Completion(Box::new(super::super::completion::OperationFailure {
            primary: Some(Box::new(ModelError::Invalid("secret".into()))),
            completion,
        }));
        let diagnostic = FailureDiagnostic::project("operation", &error, |_| None);
        assert_eq!(
            diagnostic.causes.iter().map(|c| c.code).collect::<Vec<_>>(),
            vec![
                DiagnosticCode::Completion,
                DiagnosticCode::Invalid,
                DiagnosticCode::Unconfirmed
            ]
        );
        assert_eq!(diagnostic.causes[2].stage, "session close");
        assert!(
            !serde_json::to_string(&diagnostic)
                .unwrap()
                .contains("secret")
        );
        let mut deep = error;
        for _ in 0..MAX_CAUSES {
            deep = ModelError::SharedCause(std::sync::Arc::new(deep));
        }
        let truncated = FailureDiagnostic::project(&"é".repeat(200), &deep, |_| None);
        assert_eq!(truncated.causes.len(), MAX_CAUSES);
        assert!(truncated.truncated);
        assert!(
            truncated
                .causes
                .iter()
                .all(|c| c.stage.len() <= MAX_STAGE_BYTES)
        );
        assert_eq!(
            serde_json::from_str::<FailureDiagnostic>(&serde_json::to_string(&diagnostic).unwrap())
                .unwrap(),
            diagnostic
        );
    }
    #[test]
    fn foreign_before_admission_is_opaque_without_guessed_identity() {
        let error = std::io::Error::other("request abc generation def password secret");
        let diagnostic = FailureDiagnostic::project("capture", &error, |_| None);
        assert_eq!(diagnostic.causes[0].code, DiagnosticCode::Opaque);
        assert!(diagnostic.causes[0].identities.is_empty());
        assert!(diagnostic.causes[0].issuance.is_none());
        assert!(diagnostic.causes[0].inspect.is_none());
    }
}
