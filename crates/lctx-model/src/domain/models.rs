//! Authored behavior models: typed parsing, exact applicability and dependency requirements.
//! No observed fact or legacy identifier is created by this module.
use super::calls::CallPhase;
use super::{ContentHash, Id, KeySink, ModelError, Record};
use crate::{Domain, DomainSum};
use serde::Deserialize;
use std::collections::BTreeSet;
pub mod records;
pub mod requirements;

pub const FORMAT: u32 = 7;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogFile {
    pub version: u32,
    pub models: Vec<Model>,
    #[serde(default)]
    pub context_protocols: Vec<ContextProtocolModel>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub revision: u32,
    pub target: Target,
    /// Authored applicability, independently checked against a provider's observed phase.
    /// Different phases of the same callable may have different channel contracts.
    pub phase: Phase,
    pub coverage: Channels,
    /// Exact body domain. Frame release and caller continuation require a separate proof.
    #[serde(default)]
    pub normal_body: Option<NormalBody>,
    /// Every omitted optional fixed formal has an already-created runtime default under
    /// this exact pinned implementation. No default value or normal outcome is promised.
    #[serde(default)]
    pub call_defaults_available: bool,
    pub rules: Vec<Rule>,
}

/// The complete pinned body directly returns this parameter, creates no other roots and
/// runs no user code (including implicit releases) that can invalidate caller retainers.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NormalBody {
    DirectReturnParameter { name: String },
}
impl NormalBody {
    pub fn parameter(&self) -> &str {
        match self {
            Self::DirectReturnParameter { name } => name,
        }
    }
}

/// A pinned runtime class assertion. Constructor roles retain their own identities and phases;
/// protocol entry/exit are source lifecycle actions, not purported provider call observations.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextProtocolModel {
    pub revision: u32,
    pub target: Target,
    pub allocation: Target,
    pub initialization: Target,
    pub entry: ContextEntry,
    pub exit: ContextExit,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContextEntry {
    NoneValue,
    ArgumentOrNone { formal: String },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContextExit {
    Preserve,
    SuppressClasses { formal: String },
}

pub struct CompiledContextProtocol {
    declaration: AuthoredContextProtocol,
    model: ContextProtocolModel,
}
impl CompiledContextProtocol {
    pub fn declaration(&self) -> &AuthoredContextProtocol {
        &self.declaration
    }
    pub fn model(&self) -> &ContextProtocolModel {
        &self.model
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Call,
    New,
    Init,
    Decorator,
    PropertyGet,
    PropertySet,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channels {
    pub transfers: ChannelCoverage,
    pub effects: ChannelCoverage,
    pub callbacks: ChannelCoverage,
    pub resources: ChannelCoverage,
    pub exceptions: ChannelCoverage,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelCoverage {
    Complete,
    Partial,
    Unspecified,
}

impl Channels {
    fn validate_rule(&self, rule: &Rule) -> Result<(), String> {
        let (family, coverage) = match rule {
            Rule::Transfer { .. } => ("transfer", self.transfers),
            Rule::Effect { .. } => ("effect", self.effects),
            Rule::Callback { .. } => ("callback", self.callbacks),
            Rule::Resource { .. } => ("resource", self.resources),
            Rule::Exception { .. } => ("exception", self.exceptions),
        };
        if matches!(coverage, ChannelCoverage::Unspecified) {
            return Err(format!(
                "authored {family} rule requires partial or complete channel coverage"
            ));
        }
        Ok(())
    }
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
    pub fn visit_context_references<'a, E>(
        &'a self,
        visit: &mut impl FnMut(ContextReference<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Global { module, name } => visit(ContextReference::Global { module, name }),
            Self::Parameter { .. } | Self::ReceiverField { .. } => Ok(()),
        }
    }

    pub fn formal(&self) -> Option<&str> {
        match self {
            Self::Parameter { name } => Some(name),
            Self::ReceiverField { root, .. } => Some(root),
            Self::Global { .. } => None,
        }
    }

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
    pub fn visit_context_references<'a, E>(
        &'a self,
        visit: &mut impl FnMut(ContextReference<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Global { module, name } => visit(ContextReference::Global { module, name }),
            Self::Raise { class } => visit(ContextReference::Class(class)),
            Self::ReturnValue | Self::Parameter { .. } | Self::ReceiverField { .. } => Ok(()),
        }
    }

    pub fn formal(&self) -> Option<&str> {
        match self {
            Self::Parameter { name } => Some(name),
            Self::ReceiverField { root, .. } => Some(root),
            Self::ReturnValue | Self::Global { .. } | Self::Raise { .. } => None,
        }
    }

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
        modality: RuleModality,
    },
    Effect {
        effect: Effect,
        subject: Option<InputPath>,
        exit: Exit,
        modality: RuleModality,
    },
    Callback {
        callback: InputPath,
        action: CallbackAction,
        exit: Exit,
        modality: RuleModality,
    },
    Resource {
        resource: ResourcePath,
        action: ResourceAction,
        exit: Exit,
        modality: RuleModality,
    },
    Exception {
        class: String,
        action: ExceptionAction,
        to_class: Option<String>,
        modality: RuleModality,
    },
}

/// Exact definition references in the authored language. A class also requires its ancestry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextReference<'a> {
    Global { module: &'a str, name: &'a str },
    Class(&'a str),
}
impl Rule {
    /// Traverse every typed reference-bearing path without interpreting rendered labels.
    pub fn visit_context_references<'a, E>(
        &'a self,
        visit: &mut impl FnMut(ContextReference<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Transfer { from, to, .. } => {
                from.visit_context_references(visit)?;
                to.visit_context_references(visit)
            }
            Self::Effect {
                effect, subject, ..
            } => {
                if let Some(subject) = subject {
                    subject.visit_context_references(visit)?;
                }
                match effect.schema() {
                    Some(ValidationSchema::StaticClass { class }) => {
                        visit(ContextReference::Class(class))
                    }
                    Some(ValidationSchema::RuntimeValue { source }) => {
                        source.visit_context_references(visit)
                    }
                    Some(ValidationSchema::Unresolved {}) | None => Ok(()),
                }
            }
            Self::Callback { callback, .. } => callback.visit_context_references(visit),
            Self::Resource { resource, .. } => match resource {
                ResourcePath::Input { path } => path.visit_context_references(visit),
                ResourcePath::Output { path } => path.visit_context_references(visit),
            },
            Self::Exception {
                class, to_class, ..
            } => {
                visit(ContextReference::Class(class))?;
                if let Some(class) = to_class {
                    visit(ContextReference::Class(class))?;
                }
                Ok(())
            }
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Transfer { from, to, .. } => {
                from.validate()?;
                to.validate()
            }
            Self::Effect {
                effect, subject, ..
            } => {
                effect.validate()?;
                subject.as_ref().map_or(Ok(()), InputPath::validate)
            }
            Self::Callback { callback, .. } => callback.validate(),
            Self::Resource { resource, exit, .. } => {
                resource.validate()?;
                if matches!(
                    resource,
                    ResourcePath::Output {
                        path: OutputPath::ReturnValue
                    }
                ) && !matches!(exit, Exit::Normal)
                {
                    return Err("a returned resource requires a normal trigger".into());
                }
                Ok(())
            }
            Self::Exception {
                class,
                action,
                to_class,
                ..
            } if dotted_name(class)
                && match action {
                    ExceptionAction::Convert => to_class.as_deref().is_some_and(dotted_name),
                    _ => to_class.is_none(),
                } =>
            {
                Ok(())
            }
            Self::Exception { class, .. } => Err(format!(
                "invalid exception class or conversion target: {class}"
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourcePath {
    Input { path: InputPath },
    Output { path: OutputPath },
}

impl ResourcePath {
    pub fn formal(&self) -> Option<&str> {
        match self {
            Self::Input { path } => path.formal(),
            Self::Output { path } => path.formal(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Input { path } => path.validate(),
            Self::Output {
                path: OutputPath::Raise { .. },
            } => Err("a raised exception is not a resource path".into()),
            Self::Output { path } => path.validate(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Input { path } => path.render(),
            Self::Output { path } => path.render(),
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
    Validate { schema: ValidationSchema },
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
            Self::Validate { schema } => schema.validate(),
            Self::Register { container } if dotted_name(container) => Ok(()),
            Self::Invoke { callable } if dotted_name(callable) => Ok(()),
            _ => Err("invalid effect argument".to_owned()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValidationSchema {
    /// Initial statically resolved domain: a class in the pinned context, not an arbitrary label.
    StaticClass {
        class: String,
    },
    /// The value at a proved input path determines the schema. It need not be a type object.
    RuntimeValue {
        source: InputPath,
    },
    Unresolved {},
}

impl ValidationSchema {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::StaticClass { class } if dotted_name(class) => Ok(()),
            Self::StaticClass { .. } => Err("invalid schema class".to_owned()),
            Self::RuntimeValue { source } => source.validate(),
            Self::Unresolved {} => Ok(()),
        }
    }

    pub fn source(&self) -> Option<&InputPath> {
        match self {
            Self::RuntimeValue { source } => Some(source),
            _ => None,
        }
    }
}

impl Effect {
    pub fn schema(&self) -> Option<&ValidationSchema> {
        match self {
            Self::Validate { schema } => Some(schema),
            _ => None,
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
pub enum RuleModality {
    Definite,
    Potential,
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
    Invocation,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionAction {
    Raise,
    Catch,
    Convert,
    Suppress,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_catalogs", validate = validate_catalog, invariant_refs = records::invariants_refs)]
pub struct ModelCatalog {
    #[model(key)]
    pub source_name: String,
    #[model(key)]
    pub format: i64,
    #[model(key)]
    pub content: ContentHash,
    /// Authored input bytes, parsed only through this module's typed source language.
    pub source: String,
}
fn validate_catalog(row: &ModelCatalog) -> Result<(), ModelError> {
    // The cross-relation invariant parses with an explicit attempt reservation. Row-local
    // validation checks bytes and format without constructing an uncharged catalog tree.
    if row.source_name.is_empty()
        || row.format != i64::from(FORMAT)
        || ContentHash::of(row.source.as_bytes()) != row.content
    {
        return Err(ModelError::Invalid(
            "model catalog content or format mismatch".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "authored_model_targets")]
pub enum AuthoredTarget {
    #[model(code = 0)]
    Stdlib {
        python: String,
        module: String,
        callable: String,
    },
    #[model(code = 1)]
    Dependency {
        distribution: String,
        version: String,
        module: String,
        callable: String,
    },
    #[model(code = 2)]
    Release { module: String, callable: String },
}
impl Target {
    pub fn declaration(&self) -> AuthoredTarget {
        match self {
            Self::Stdlib {
                python,
                module,
                callable,
            } => AuthoredTarget::Stdlib {
                python: python.clone(),
                module: module.clone(),
                callable: callable.clone(),
            },
            Self::Dependency {
                distribution,
                version,
                module,
                callable,
            } => AuthoredTarget::Dependency {
                distribution: distribution.clone(),
                version: version.clone(),
                module: module.clone(),
                callable: callable.clone(),
            },
            Self::Release { module, callable } => AuthoredTarget::Release {
                module: module.clone(),
                callable: callable.clone(),
            },
        }
    }
}
impl Phase {
    pub fn phase(self) -> CallPhase {
        match self {
            Self::Call => CallPhase::Call,
            Self::New => CallPhase::New,
            Self::Init => CallPhase::Init,
            Self::Decorator => CallPhase::Decorator,
            Self::PropertyGet => CallPhase::PropertyGet,
            Self::PropertySet => CallPhase::PropertySet,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "authored_models")]
pub struct AuthoredModel {
    #[model(key)]
    pub catalog: Id<ModelCatalog>,
    #[model(key)]
    pub target: Id<AuthoredTarget>,
    #[model(key)]
    pub revision: i64,
    #[model(key)]
    pub phase: CallPhase,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "authored_context_protocols")]
pub struct AuthoredContextProtocol {
    #[model(key)]
    pub catalog: Id<ModelCatalog>,
    #[model(key)]
    pub target: Id<AuthoredTarget>,
    #[model(key)]
    pub revision: i64,
    #[model(key)]
    pub allocation: Id<AuthoredTarget>,
    #[model(key)]
    pub initialization: Id<AuthoredTarget>,
}
pub struct CompiledModel {
    declaration: AuthoredModel,
    model: Model,
}
impl CompiledModel {
    pub fn declaration(&self) -> &AuthoredModel {
        &self.declaration
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
}
/// Validated authored meaning and its source identity cannot be mutated independently.
///
/// ```
/// use lctx_model::domain::models::Catalog;
/// let catalog = Catalog::committed().unwrap();
/// assert!(!catalog.models().is_empty());
/// let mut detached = catalog.models()[0].model().clone();
/// detached.rules.clear();
/// assert!(!catalog.models()[0].model().rules.is_empty());
/// ```
/// ```compile_fail
/// use lctx_model::domain::models::Catalog;
/// let mut catalog = Catalog::committed().unwrap();
/// catalog.models[0].model.rules.clear();
/// ```
/// ```compile_fail
/// use lctx_model::domain::models::Catalog;
/// let catalog = Catalog::committed().unwrap();
/// catalog.models()[0].model().rules.clear();
/// ```
pub struct Catalog {
    declaration: ModelCatalog,
    models: Vec<CompiledModel>,
    context_protocols: Vec<CompiledContextProtocol>,
}
impl Catalog {
    pub fn declaration(&self) -> &ModelCatalog {
        &self.declaration
    }
    pub fn models(&self) -> &[CompiledModel] {
        &self.models
    }
    pub fn context_protocols(&self) -> &[CompiledContextProtocol] {
        &self.context_protocols
    }
    pub fn parse(source_name: &str, source: &str) -> Result<Self, String> {
        if source_name.is_empty() {
            return Err("model catalog needs a source name".into());
        }
        let parsed: CatalogFile =
            toml::from_str(source).map_err(|e| format!("{source_name}: {e}"))?;
        if parsed.version != FORMAT {
            return Err(format!(
                "{source_name}: unsupported model format {}",
                parsed.version
            ));
        }
        let catalog = ModelCatalog {
            source_name: source_name.into(),
            format: i64::from(FORMAT),
            content: ContentHash::of(source.as_bytes()),
            source: source.into(),
        };
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
                model.coverage.validate_rule(rule)?;
            }
            if model.normal_body.is_some()
                && (model.coverage.exceptions != ChannelCoverage::Complete
                    || model
                        .rules
                        .iter()
                        .any(|rule| matches!(rule, Rule::Exception { .. })))
            {
                return Err(format!(
                    "{}: body_return_parameter requires complete no-exception coverage",
                    model.target.key()
                ));
            }
            if let Some(body) = &model.normal_body
                && (model.phase != Phase::Call
                    || body.parameter().is_empty()
                    || !body
                        .parameter()
                        .chars()
                        .all(|c| c == '_' || c.is_alphanumeric()))
            {
                return Err("normal body needs an exact parameter name".into());
            }
            let key = model.target.key();
            if !seen.insert((key.clone(), model.phase)) {
                return Err(format!(
                    "{source_name}: duplicate model target and phase {key}"
                ));
            }
            let declaration = AuthoredModel {
                catalog: catalog.id(),
                target: model.target.declaration().id(),
                revision: i64::from(model.revision),
                phase: model.phase.phase(),
            };
            models.push(CompiledModel { declaration, model });
        }
        models.sort_by_key(|row| (row.model.target.key(), row.model.phase));
        let mut context_protocols = Vec::new();
        let mut context_classes = BTreeSet::new();
        for model in parsed.context_protocols {
            for target in [&model.target, &model.allocation, &model.initialization] {
                target.validate()?;
            }
            if model.revision == 0 || !context_classes.insert(model.target.key()) {
                return Err(
                    "context protocol needs a positive revision and unique class target".into(),
                );
            }
            let entry = match &model.entry {
                ContextEntry::NoneValue => None,
                ContextEntry::ArgumentOrNone { formal } => Some(formal),
            };
            let exit = match &model.exit {
                ContextExit::Preserve => None,
                ContextExit::SuppressClasses { formal } => Some(formal),
            };
            if entry.into_iter().chain(exit).any(|name| !identifier(name)) {
                return Err("context protocol formal must be an identifier".into());
            }
            let declaration = AuthoredContextProtocol {
                catalog: catalog.id(),
                target: model.target.declaration().id(),
                revision: i64::from(model.revision),
                allocation: model.allocation.declaration().id(),
                initialization: model.initialization.declaration().id(),
            };
            context_protocols.push(CompiledContextProtocol { declaration, model });
        }
        context_protocols.sort_by_key(|p| p.model.target.key());
        Ok(Self {
            declaration: catalog,
            models,
            context_protocols,
        })
    }

    /// Embedded committed bytes, independent of the operator's ambient filesystem.
    pub fn committed() -> Result<Self, String> {
        Self::parse("external.toml", Self::committed_source())
    }
    /// Allows a runtime caller to reserve parse/retained-data capacity before allocation.
    pub fn committed_source() -> &'static str {
        include_str!("../../models/external.toml")
    }

    pub fn committed_digest() -> ContentHash {
        let mut sink = KeySink::new("authored-model-catalog");
        sink.part(b"source", b"external.toml");
        sink.part(b"bytes", include_bytes!("../../models/external.toml"));
        sink.finish()
    }
    pub fn digest(&self) -> ContentHash {
        let mut sink = KeySink::new("authored-model-catalog");
        sink.part(b"source", self.declaration.source_name.as_bytes());
        sink.part(b"bytes", self.declaration.source.as_bytes());
        sink.finish()
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
        assert_eq!(catalog.digest(), Catalog::committed_digest());
        assert_eq!(catalog.models.len(), 18);
        let model = &catalog
            .models
            .iter()
            .find(|m| m.model.target.key() == "stdlib:3.14.7:typing.cast")
            .unwrap()
            .model;
        assert_eq!(model.target.key(), "stdlib:3.14.7:typing.cast");
        assert!(model.normal_body.is_some());
        let Rule::Transfer {
            from,
            to,
            transfer: Transfer::Identity,
            modality: RuleModality::Definite,
        } = &model.rules[0]
        else {
            panic!("typing.cast must be an identity transfer");
        };
        assert_eq!(from.render(), "Parameter[val]");
        assert_eq!(to.render(), "ReturnValue");
        for (target, formal) in [
            ("stdlib:3.14.7:json.loads", "s"),
            ("stdlib:3.14.7:gzip.compress", "data"),
            ("stdlib:3.14.7:gzip.decompress", "data"),
            (
                "dependency:pydantic==2.13.5:pydantic.type_adapter.TypeAdapter.validate_python",
                "object",
            ),
        ] {
            let added = &catalog
                .models
                .iter()
                .find(|m| m.model.target.key() == target)
                .unwrap()
                .model;
            assert!(
                added.normal_body.is_none(),
                "fallible {target} cannot assert total completion"
            );
            assert!(
                added.rules.iter().any(|rule| matches!(rule,
                Rule::Transfer {
                    from: InputPath::Parameter { name },
                    to: OutputPath::ReturnValue,
                    transfer: Transfer::Transform,
                    modality: RuleModality::Potential,
                } if name == formal)),
                "{target} must retain its exact input formal"
            );
        }
        let logging = &catalog
            .models
            .iter()
            .find(|m| m.model.target.key() == "stdlib:3.14.7:logging.Logger.warning")
            .unwrap()
            .model;
        assert!(logging.normal_body.is_none());
        assert!(logging.rules.iter().any(|rule| matches!(rule,
            Rule::Effect {
                effect: Effect::Log,
                subject: Some(InputPath::Parameter { name }),
                exit: Exit::Invocation,
                modality: RuleModality::Potential,
            } if name == "msg")));
        let adapter = &catalog.models.iter().find(|m| {
            m.model.target.key()
                == "dependency:pydantic==2.13.5:pydantic.type_adapter.TypeAdapter.validate_python"
        }).unwrap().model;
        assert!(matches!(
            adapter.coverage.effects,
            ChannelCoverage::Unspecified
        ));
        assert!(
            !adapter
                .rules
                .iter()
                .any(|rule| matches!(rule, Rule::Effect { .. }))
        );
    }

    #[test]
    fn callback_resource_and_exception_models_preserve_distinct_claims() {
        let catalog = Catalog::committed().unwrap();
        let register = &catalog
            .models
            .iter()
            .find(|m| m.model.target.key() == "stdlib:3.14.7:atexit.register")
            .unwrap()
            .model;
        assert!(register.rules.iter().any(|rule| matches!(
            rule,
            Rule::Callback {
                callback: InputPath::Parameter { name },
                action: CallbackAction::Registered,
                exit: Exit::Normal,
                modality: RuleModality::Definite,
            } if name == "func"
        )));
        assert!(!register.rules.iter().any(|rule| matches!(
            rule,
            Rule::Callback {
                action: CallbackAction::Invoked,
                ..
            }
        )));
        let open = &catalog
            .models
            .iter()
            .find(|m| m.model.target.key() == "stdlib:3.14.7:builtins.open")
            .unwrap()
            .model;
        assert!(open.rules.iter().any(|rule| matches!(
            rule,
            Rule::Resource {
                resource: ResourcePath::Output {
                    path: OutputPath::ReturnValue,
                },
                action: ResourceAction::Acquire,
                exit: Exit::Normal,
                modality: RuleModality::Definite,
            }
        )));
        assert!(open.rules.iter().any(|rule| matches!(
            rule,
            Rule::Exception {
                class,
                action: ExceptionAction::Raise,
                to_class: None,
                modality: RuleModality::Potential,
            } if class == "builtins.OSError"
        )));
        assert!(
            !open
                .rules
                .iter()
                .any(|rule| matches!(rule, Rule::Effect { .. }))
        );
    }

    #[test]
    fn conversion_needs_a_target_and_resource_cannot_be_a_raise_path() {
        let header = r#"version = 7
[[models]]
phase = "call"
revision = 1
target = { scope = "stdlib", python = "3.14.7", module = "builtins", callable = "open" }
coverage = { transfers = "unspecified", effects = "unspecified", callbacks = "unspecified", resources = "partial", exceptions = "partial" }
[[models.rules]]
"#;
        assert!(
            Catalog::parse(
                "bad.toml",
                &format!(
                    "{header}kind = \"exception\"\nclass = \"builtins.OSError\"\naction = \"convert\"\nmodality = \"potential\"\n"
                )
            )
            .is_err()
        );
        assert!(
            Catalog::parse(
                "bad.toml",
                &format!(
                    "{header}kind = \"resource\"\nresource = {{ role = \"output\", path = {{ kind = \"raise\", class = \"builtins.OSError\" }} }}\naction = \"acquire\"\nexit = \"normal\"\nmodality = \"definite\"\n"
                )
            )
            .is_err()
        );
    }

    #[test]
    fn action_triggers_are_authored_and_returned_resources_require_normal() {
        let source = include_str!("../../models/external.toml");
        assert!(
            Catalog::parse(
                "missing-trigger.toml",
                &source.replacen("exit = \"invocation\"\n", "", 1)
            )
            .is_err()
        );
        for exit in ["invocation", "exceptional", "finally"] {
            let changed = source.replacen(
                "action = \"acquire\"\nexit = \"normal\"",
                &format!("action = \"acquire\"\nexit = \"{exit}\""),
                1,
            );
            assert!(
                Catalog::parse("early-resource.toml", &changed).is_err(),
                "{exit}"
            );
        }
    }

    #[test]
    fn body_return_parameter_requires_complete_exception_coverage_without_exception_rules() {
        let header = r#"version = 7
[[models]]
phase = "call"
revision = 1
target = { scope = "stdlib", python = "3.14.7", module = "typing", callable = "cast" }
normal_body = { kind = "direct_return_parameter", name = "val" }
coverage = { transfers = "complete", effects = "complete", callbacks = "complete", resources = "complete", exceptions = "partial" }
[[models.rules]]
kind = "transfer"
from = { kind = "parameter", name = "val" }
to = { kind = "return_value" }
transfer = "identity"
modality = "definite"
"#;
        assert!(Catalog::parse("bad.toml", header).is_err());
        let with_complete = header.replace("exceptions = \"partial\"", "exceptions = \"complete\"");
        assert!(Catalog::parse("good.toml", &with_complete).is_ok());
        let with_exception = format!(
            "{with_complete}\n[[models.rules]]\nkind = \"exception\"\nclass = \"builtins.Exception\"\naction = \"raise\"\nmodality = \"potential\"\n"
        );
        assert!(Catalog::parse("bad.toml", &with_exception).is_err());
    }

    #[test]
    fn unknown_fields_bad_paths_and_duplicate_targets_are_rejected() {
        let valid = include_str!("../../models/external.toml");
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
        let second = valid.trim_start_matches("version = 7").trim();
        assert!(Catalog::parse("bad.toml", &format!("{valid}\n{second}\n")).is_err());
    }

    #[test]
    fn source_or_revision_change_invalidates_model_identity() {
        let original = include_str!("../../models/external.toml");
        let revised = original.replace("revision = 1", "revision = 2");
        let before = Catalog::parse("external.toml", original).unwrap();
        let after = Catalog::parse("external.toml", &revised).unwrap();
        assert_ne!(before.digest(), after.digest());
        assert_ne!(
            before.models[0].declaration.id(),
            after.models[0].declaration.id()
        );
    }

    #[test]
    fn phase_is_mandatory_and_same_target_can_have_distinct_phase_contracts() {
        let original = include_str!("../../models/external.toml");
        assert!(
            Catalog::parse("missing.toml", &original.replace("phase = \"call\"\n", "")).is_err()
        );
        assert!(
            Catalog::parse(
                "unknown.toml",
                &original.replace("phase = \"call\"", "phase = \"later\"")
            )
            .is_err()
        );
        let first = original
            .split("# Python 3.14 typing.assert_type")
            .next()
            .unwrap();
        let init = first
            .trim_start_matches("version = 7")
            .replace("phase = \"call\"", "phase = \"init\"")
            .replace(
                r#"normal_body = { kind = "direct_return_parameter", name = "val" }"#,
                "",
            )
            .replace("exceptions = \"complete\"", "exceptions = \"partial\"");
        let catalog = Catalog::parse("phases.toml", &format!("{first}\n{init}")).unwrap();
        assert_eq!(catalog.models.len(), 2);
        assert_eq!(catalog.models[0].model.phase, Phase::Call);
        assert_eq!(catalog.models[1].model.phase, Phase::Init);
        assert!(catalog.models[0].model.normal_body.is_some());
        assert!(catalog.models[1].model.normal_body.is_none());
        assert_ne!(
            catalog.models[0].declaration.id(),
            catalog.models[1].declaration.id()
        );
        assert!(
            Catalog::parse(
                "duplicate.toml",
                &format!("{first}\n{}", first.trim_start_matches("version = 7"))
            )
            .is_err()
        );
    }
}
