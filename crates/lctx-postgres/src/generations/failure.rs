//! Why an attempt failed, stored with its failed generation (store-lifecycle review F04, F05). The
//! class enum renders the control schema's CHECK and is the only source of stored class names.
use lctx_model::domain::{Infrastructure, ModelError};
use super::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureClass {
    /// The connection or transport was lost.
    Transport,
    /// A commit, rollback or COPY abort went unconfirmed.
    Unconfirmed,
    /// The server refused: privileges or another refusal outside the content.
    Refused,
    /// A lock or statement timeout, a deadlock or a serialization conflict.
    Contention,
    /// A generation's state or lock excluded the step.
    State,
    /// The stored model, lowering or registry differs from this binary's.
    Contract,
    /// Local input or randomness was unavailable.
    Io,
    /// The schedule or coverage cannot be admitted to the generation's frontier.
    Frontier,
    /// The attempt's memory budget refused.
    Resource,
    /// A declared ceiling refused, or the server exhausted a resource (SQLSTATE 53, 54).
    Limit,
    /// The stored content violates the model: an invariant, identity or reference (SQLSTATE 23).
    Invalid,
    /// An encoding defect.
    Codec,
}
impl FailureClass {
    pub const ALL: [Self; 12] = [Self::Transport, Self::Unconfirmed, Self::Refused, Self::Contention, Self::State, Self::Contract, Self::Io,
        Self::Frontier, Self::Resource, Self::Limit, Self::Invalid, Self::Codec];
    pub fn name(self) -> &'static str {
        match self {
            Self::Transport => "transport", Self::Unconfirmed => "unconfirmed", Self::Refused => "refused", Self::Contention => "contention",
            Self::State => "state", Self::Contract => "contract", Self::Io => "io", Self::Frontier => "frontier", Self::Resource => "resource",
            Self::Limit => "limit", Self::Invalid => "invalid", Self::Codec => "codec",
        }
    }
    pub fn parse(name: &str) -> Option<Self> { Self::ALL.into_iter().find(|c| c.name() == name) }
    fn infrastructure(class: Infrastructure) -> Self {
        match class {
            Infrastructure::Transport => Self::Transport, Infrastructure::Unconfirmed => Self::Unconfirmed, Infrastructure::Refused => Self::Refused,
            Infrastructure::Contention => Self::Contention, Infrastructure::State => Self::State, Infrastructure::Contract => Self::Contract,
            Infrastructure::Io => Self::Io,
        }
    }
    pub fn of_model(error: &ModelError) -> Self {
        match error {
            ModelError::Resource { .. } => Self::Resource,
            ModelError::Limit { .. } => Self::Limit,
            ModelError::Frontier(_) => Self::Frontier,
            ModelError::Codec(_) => Self::Codec,
            ModelError::Infrastructure { class, .. } => Self::infrastructure(*class),
            ModelError::Invalid(_) | ModelError::Schema(_) | ModelError::Identity(_) | ModelError::Conflict(_) => Self::Invalid,
        }
    }
    pub fn of(error: &Error) -> Self {
        match error {
            Error::Model(error) => Self::of_model(error),
            Error::Frontier(_) => Self::Frontier,
            Error::Codec(_) => Self::Codec,
            Error::Database(database) => {
                let code = database.as_database_error().and_then(|e| e.code()).unwrap_or_default();
                if code.starts_with("23") { Self::Invalid }
                else if code.starts_with("53") || code.starts_with("54") { Self::Limit }
                else { Self::infrastructure(error.class()) }
            },
            other => Self::infrastructure(other.class()),
        }
    }
}

/// A failure's class and a bounded, safe detail. A server refusal keeps its SQLSTATE and the
/// constraint and table it names, never the server's message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure { pub class: FailureClass, pub detail: String }
impl Failure {
    pub fn of(error: &Error) -> Self {
        let detail = match error {
            Error::Database(database) => match database.as_database_error() {
                Some(server) => {
                    let mut detail = format!("SQLSTATE {}", server.code().unwrap_or_default());
                    if let Some(constraint) = server.constraint() { detail.push_str(&format!(" constraint {constraint}")); }
                    if let Some(table) = server.table() { detail.push_str(&format!(" table {table}")); }
                    detail
                },
                None => database.to_string(),
            },
            other => other.to_string(),
        };
        Self { class: FailureClass::of(error), detail: bounded(detail) }
    }
    pub fn of_model(error: &ModelError) -> Self { Self { class: FailureClass::of_model(error), detail: bounded(error.to_string()) } }
}
fn bounded(mut detail: String) -> String {
    if detail.len() > 4096 {
        let mut end = 4096;
        while !detail.is_char_boundary(end) { end -= 1; }
        detail.truncate(end);
    }
    detail
}
