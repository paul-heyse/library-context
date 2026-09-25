//! Typed, committed behavior models (DESIGN §9.9; ADR-0022).
//!
//! Authored TOML selects variants and fields. It never contains an independently parsed access
//! path string: these enums own the shape, and [`InputPath::render`] / [`OutputPath::render`] are
//! the sole written form used by Arrow and display. This module parses and validates the
//! catalog; binding a target to a pinned source callable is the compiler's next boundary.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::id::{Digest, Id, IdHasher};

pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogFile {
    pub version: u32,
    pub models: Vec<Model>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub revision: u32,
    pub target: Target,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Stdlib {
        python: String,
        module: String,
        callable: String,
    },
    Dependency {
        distribution: String,
        version: String,
        module: String,
        callable: String,
    },
    Release {
        module: String,
        callable: String,
    },
}

impl Target {
    pub fn key(&self) -> String {
        match self {
            Self::Stdlib {
                python,
                module,
                callable,
            } => format!("stdlib:{python}:{module}.{callable}"),
            Self::Dependency {
                distribution,
                version,
                module,
                callable,
            } => format!("dependency:{distribution}=={version}:{module}.{callable}"),
            Self::Release { module, callable } => format!("release:{module}.{callable}"),
        }
    }

    fn validate(&self) -> Result<(), String> {
        let (module, callable) = match self {
            Self::Stdlib {
                python,
                module,
                callable,
            } => {
                if python.split('.').count() != 3
                    || !python
                        .split('.')
                        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
                {
                    return Err(format!(
                        "stdlib model needs an exact Python patch pin: {python}"
                    ));
                }
                (module, callable)
            }
            Self::Dependency {
                distribution,
                version,
                module,
                callable,
            } => {
                if distribution.is_empty()
                    || !distribution
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                    || version.is_empty()
                    || !version.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'.' | b'!' | b'+' | b'_' | b'-')
                    })
                {
                    return Err("dependency target needs a distribution and exact version".into());
                }
                (module, callable)
            }
            Self::Release { module, callable } => (module, callable),
        };
        if !dotted_name(module) || !dotted_name(callable) {
            return Err(format!("invalid model target: {}", self.key()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputPath {
    Parameter { name: String },
    ReceiverField { root: String, field: String },
    Global { module: String, name: String },
}

impl InputPath {
    pub fn render(&self) -> String {
        match self {
            Self::Parameter { name } => format!("Parameter[{name}]"),
            Self::ReceiverField { root, field } => format!("Parameter[{root}].Field[{field}]"),
            Self::Global { module, name } => format!("Global[{module}.{name}]"),
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Parameter { name } if identifier(name) => Ok(()),
            Self::ReceiverField { root, field } if identifier(root) && identifier(field) => Ok(()),
            Self::Global { module, name } if dotted_name(module) && identifier(name) => Ok(()),
            _ => Err(format!("invalid input path: {}", self.render())),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputPath {
    ReturnValue,
    Parameter { name: String },
    ReceiverField { root: String, field: String },
    Global { module: String, name: String },
    Raise { class: String },
}

impl OutputPath {
    pub fn render(&self) -> String {
        match self {
            Self::ReturnValue => "ReturnValue".to_owned(),
            Self::Parameter { name } => format!("Parameter[{name}]"),
            Self::ReceiverField { root, field } => format!("Parameter[{root}].Field[{field}]"),
            Self::Global { module, name } => format!("Global[{module}.{name}]"),
            Self::Raise { class } => format!("Raise[{class}]"),
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::ReturnValue => Ok(()),
            Self::Parameter { name } if identifier(name) => Ok(()),
            Self::ReceiverField { root, field } if identifier(root) && identifier(field) => Ok(()),
            Self::Global { module, name } if dotted_name(module) && identifier(name) => Ok(()),
            Self::Raise { class } if dotted_name(class) => Ok(()),
            _ => Err(format!("invalid output path: {}", self.render())),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Rule {
    Transfer {
        from: InputPath,
        to: OutputPath,
        transfer: Transfer,
    },
    Effect {
        effect: Effect,
        subject: Option<InputPath>,
    },
    Callback {
        callback: InputPath,
        action: CallbackAction,
    },
    Resource {
        resource: InputPath,
        action: ResourceAction,
        exit: Exit,
    },
    Exception {
        class: String,
        action: ExceptionAction,
    },
}

impl Rule {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Transfer { from, to, .. } => {
                from.validate()?;
                to.validate()
            }
            Self::Effect { effect, subject } => {
                effect.validate()?;
                subject.as_ref().map_or(Ok(()), InputPath::validate)
            }
            Self::Callback { callback, .. } => callback.validate(),
            Self::Resource { resource, .. } => resource.validate(),
            Self::Exception { class, .. } if dotted_name(class) => Ok(()),
            Self::Exception { class, .. } => Err(format!("invalid exception class: {class}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transfer {
    Identity,
    Transform,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    IoRead,
    IoWrite,
    Net,
    Log,
    Timeout,
    ThreadDispatch,
    Compress { format: String },
    Serialize { format: String },
    Validate { schema: String },
    Register { container: String },
    Invoke { callable: String },
}

impl Effect {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::IoRead
            | Self::IoWrite
            | Self::Net
            | Self::Log
            | Self::Timeout
            | Self::ThreadDispatch => Ok(()),
            Self::Compress { format } | Self::Serialize { format } if identifier(format) => Ok(()),
            Self::Validate { schema } if dotted_name(schema) => Ok(()),
            Self::Register { container } if dotted_name(container) => Ok(()),
            Self::Invoke { callable } if dotted_name(callable) => Ok(()),
            _ => Err("invalid effect argument".to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallbackAction {
    Stored,
    Registered,
    Forwarded,
    Invoked,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    Acquire,
    Release,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exit {
    Normal,
    Exceptional,
    Finally,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionAction {
    Raise,
    Catch,
    Convert,
    Suppress,
}

pub struct CompiledModel {
    pub model_id: Id,
    pub model: Model,
}

pub struct Catalog {
    pub digest: Digest,
    pub models: Vec<CompiledModel>,
}

impl Catalog {
    pub fn parse(source_name: &str, source: &str) -> Result<Self, String> {
        let parsed: CatalogFile =
            toml::from_str(source).map_err(|e| format!("{source_name}: {e}"))?;
        if parsed.version != FORMAT {
            return Err(format!(
                "{source_name}: unsupported model format {}",
                parsed.version
            ));
        }
        let mut seen = BTreeSet::new();
        let mut models = Vec::new();
        for model in parsed.models {
            model.target.validate()?;
            if model.revision == 0 || model.rules.is_empty() {
                return Err(format!(
                    "{}: revision and rules must be nonzero",
                    model.target.key()
                ));
            }
            for rule in &model.rules {
                rule.validate()?;
            }
            let key = model.target.key();
            if !seen.insert(key.clone()) {
                return Err(format!("{source_name}: duplicate model target {key}"));
            }
            let model_id = IdHasher::new("behavior-model")
                .str(source_name)
                .bytes(source.as_bytes())
                .str(&key)
                .i64(i64::from(model.revision))
                .finish_id();
            models.push(CompiledModel { model_id, model });
        }
        models.sort_by_key(|row| row.model.target.key());
        Ok(Self {
            digest: IdHasher::new("behavior-model-catalog")
                .str(source_name)
                .bytes(source.as_bytes())
                .finish_digest(),
            models,
        })
    }

    /// Embedded committed bytes, independent of the operator's ambient filesystem.
    pub fn committed() -> Result<Self, String> {
        Self::parse("external.toml", include_str!("../models/external.toml"))
    }

    pub fn committed_digest() -> Digest {
        IdHasher::new("behavior-model-catalog")
            .str("external.toml")
            .bytes(include_bytes!("../models/external.toml"))
            .finish_digest()
    }
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn dotted_name(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(identifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_catalog_has_typed_identity_path_and_digest() {
        let catalog = Catalog::committed().unwrap();
        assert_eq!(catalog.digest, Catalog::committed_digest());
        assert_eq!(catalog.models.len(), 1);
        let model = &catalog.models[0].model;
        assert_eq!(model.target.key(), "stdlib:3.14.7:typing.cast");
        let Rule::Transfer {
            from,
            to,
            transfer: Transfer::Identity,
        } = &model.rules[0]
        else {
            panic!("typing.cast must be an identity transfer");
        };
        assert_eq!(from.render(), "Parameter[val]");
        assert_eq!(to.render(), "ReturnValue");
    }

    #[test]
    fn unknown_fields_bad_paths_and_duplicate_targets_are_rejected() {
        let valid = include_str!("../models/external.toml");
        assert!(
            Catalog::parse(
                "bad.toml",
                &valid.replace("revision = 1", "revision = 1\nextra = 1")
            )
            .is_err()
        );
        assert!(
            Catalog::parse(
                "bad.toml",
                &valid.replace("name = \"val\"", "name = \"x.y\"")
            )
            .is_err()
        );
        let second = valid.trim_start_matches("version = 1").trim();
        assert!(Catalog::parse("bad.toml", &format!("{valid}\n{second}\n")).is_err());
    }

    #[test]
    fn source_or_revision_change_invalidates_model_identity() {
        let original = include_str!("../models/external.toml");
        let revised = original.replace("revision = 1", "revision = 2");
        let before = Catalog::parse("external.toml", original).unwrap();
        let after = Catalog::parse("external.toml", &revised).unwrap();
        assert_ne!(before.digest, after.digest);
        assert_ne!(before.models[0].model_id, after.models[0].model_id);
    }
}
