//! Bounded public failure meaning. Driver/configuration text is never an input to this DTO.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    ResourceRefused,
    Incompatible,
    Corrupt,
    Unavailable,
    UnknownLibrary,
}
impl FailureKind {
    pub const ALL: [Self; 5] = [
        Self::ResourceRefused,
        Self::Incompatible,
        Self::Corrupt,
        Self::Unavailable,
        Self::UnknownLibrary,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::ResourceRefused => "resource_refused",
            Self::Incompatible => "incompatible",
            Self::Corrupt => "corrupt",
            Self::Unavailable => "unavailable",
            Self::UnknownLibrary => "unknown_library",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
    pub fn message(self) -> &'static str {
        match self {
            Self::ResourceRefused => "Canonical serving resources refused the request.",
            Self::Incompatible => {
                "The request is incompatible with the canonical serving contract."
            }
            Self::Corrupt => "Canonical serving evidence failed validation.",
            Self::Unavailable => "Canonical serving is unavailable.",
            Self::UnknownLibrary => "The requested library is not admitted to this snapshot.",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicFailure {
    /// Coarse recognized cause; it does not imply retryability.
    pub kind: FailureKind,
    /// Fixed public message, without driver text, paths, configuration or credentials.
    pub message: String,
}
impl PublicFailure {
    pub fn new(kind: FailureKind) -> Self {
        Self {
            kind,
            message: kind.message().into(),
        }
    }
}

impl std::fmt::Display for FailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
impl std::fmt::Display for PublicFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}
