//! Query values are transient; none is a source observation or a persisted Literal row.
use crate::domain::{ModelError, Record, Utf8Text, value::Literal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExactScalar {
    None {},
    Bool { value: bool },
    Integer { decimal: String },
    String { value: String },
}
impl ExactScalar {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.literal().validate()
    }
    pub(super) fn literal(&self) -> Literal {
        match self {
            Self::None {} => Literal::None,
            Self::Bool { value } => Literal::Bool { value: *value },
            Self::Integer { decimal } => Literal::Integer { decimal: decimal.clone() },
            Self::String { value } => Literal::String { value: Utf8Text::from(value.clone()) },
        }
    }
    pub(super) fn bytes(&self) -> usize {
        match self {
            Self::String { value } => value.len(),
            Self::Integer { decimal } => decimal.len(),
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinNamespace {
    Unknown,
    StandardCpython,
}

/// Namespace admission is an explicit model assumption, never inferred from a spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assumptions {
    pub builtin_namespace: BuiltinNamespace,
}
impl Default for Assumptions {
    fn default() -> Self {
        Self { builtin_namespace: BuiltinNamespace::Unknown }
    }
}
