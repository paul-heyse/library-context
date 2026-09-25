//! Typed, committed behavior models (DESIGN §9.9; ADR-0022).
//!
//! Authored TOML selects variants and fields. It never contains an independently parsed access
//! path string: these enums own the shape, and [`InputPath::render`] / [`OutputPath::render`] are
//! the sole written form used by Arrow and display. This module parses and validates the
//! catalog; binding a target to a pinned source callable is the compiler's next boundary.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::behavior::{
    ModelCallbacksRow, ModelEffectsRow, ModelExceptionsRow, ModelFormalPathsRow, ModelResourcesRow,
    ModelTargetsRow, ModelTransfersRow,
};
use crate::codebook::{
    DefinitionKind, Modality, ModelCallbackAction, ModelChannelCoverage, ModelEffectKind,
    ModelExceptionAction, ModelExit, ModelPathKind, ModelPathRole, ModelResourceAction,
    ModelTransferKind, ModuleOrigin, Origin, SignatureForm,
};
use crate::id::{Digest, Id, IdHasher};
use crate::tables::{ContextDefinitionsRow, ContextModulesRow, ContextParametersRow, ContextsRow};

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
    pub coverage: Channels,
    /// Authored assertion that the pinned callable itself always completes normally after
    /// argument evaluation. This is separate from transfer modality and call-site dispatch.
    #[serde(default)]
    pub normal_return: bool,
    pub rules: Vec<Rule>,
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

impl ChannelCoverage {
    fn codebook(self) -> ModelChannelCoverage {
        match self {
            Self::Complete => ModelChannelCoverage::Complete,
            Self::Partial => ModelChannelCoverage::Partial,
            Self::Unspecified => ModelChannelCoverage::Unspecified,
        }
    }
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
    pub fn kind(&self) -> ModelPathKind {
        match self {
            Self::Parameter { .. } => ModelPathKind::Parameter,
            Self::ReceiverField { .. } => ModelPathKind::ReceiverField,
            Self::Global { .. } => ModelPathKind::Global,
        }
    }

    fn formal(&self) -> Option<&str> {
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

    pub fn id(&self) -> Id {
        let mut hash = IdHasher::new("behavior-model-input-path");
        match self {
            Self::Parameter { name } => hash.str("parameter").str(name).finish_id(),
            Self::ReceiverField { root, field } => {
                hash.str("receiver_field").str(root).str(field).finish_id()
            }
            Self::Global { module, name } => hash.str("global").str(module).str(name).finish_id(),
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
    pub fn kind(&self) -> ModelPathKind {
        match self {
            Self::ReturnValue => ModelPathKind::ReturnValue,
            Self::Parameter { .. } => ModelPathKind::Parameter,
            Self::ReceiverField { .. } => ModelPathKind::ReceiverField,
            Self::Global { .. } => ModelPathKind::Global,
            Self::Raise { .. } => ModelPathKind::Raise,
        }
    }

    fn formal(&self) -> Option<&str> {
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

    pub fn id(&self) -> Id {
        let mut hash = IdHasher::new("behavior-model-output-path");
        match self {
            Self::ReturnValue => hash.str("return_value").finish_id(),
            Self::Parameter { name } => hash.str("parameter").str(name).finish_id(),
            Self::ReceiverField { root, field } => {
                hash.str("receiver_field").str(root).str(field).finish_id()
            }
            Self::Global { module, name } => hash.str("global").str(module).str(name).finish_id(),
            Self::Raise { class } => hash.str("raise").str(class).finish_id(),
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

impl Rule {
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
            Self::Resource { resource, .. } => resource.validate(),
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
    pub fn kind(&self) -> ModelPathKind {
        match self {
            Self::Input { path } => path.kind(),
            Self::Output { path } => path.kind(),
        }
    }

    fn formal(&self) -> Option<&str> {
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

    pub fn id(&self) -> Id {
        match self {
            Self::Input { path } => path.id(),
            Self::Output { path } => path.id(),
        }
    }

    pub fn role(&self) -> ModelPathRole {
        match self {
            Self::Input { .. } => ModelPathRole::Input,
            Self::Output { .. } => ModelPathRole::Output,
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
    fn kind_argument(&self) -> (ModelEffectKind, Option<&str>) {
        match self {
            Self::IoRead => (ModelEffectKind::IoRead, None),
            Self::IoWrite => (ModelEffectKind::IoWrite, None),
            Self::Net => (ModelEffectKind::Net, None),
            Self::Log => (ModelEffectKind::Log, None),
            Self::Timeout => (ModelEffectKind::Timeout, None),
            Self::ThreadDispatch => (ModelEffectKind::ThreadDispatch, None),
            Self::Compress { format } => (ModelEffectKind::Compress, Some(format)),
            Self::Serialize { format } => (ModelEffectKind::Serialize, Some(format)),
            Self::Validate { schema } => (ModelEffectKind::Validate, Some(schema)),
            Self::Register { container } => (ModelEffectKind::Register, Some(container)),
            Self::Invoke { callable } => (ModelEffectKind::Invoke, Some(callable)),
        }
    }

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
pub enum RuleModality {
    Definite,
    Potential,
}

impl RuleModality {
    fn codebook(self) -> Modality {
        match self {
            Self::Definite => Modality::Definite,
            Self::Potential => Modality::Potential,
        }
    }
}

impl CallbackAction {
    fn codebook(self) -> ModelCallbackAction {
        match self {
            Self::Stored => ModelCallbackAction::Stored,
            Self::Registered => ModelCallbackAction::Registered,
            Self::Forwarded => ModelCallbackAction::Forwarded,
            Self::Invoked => ModelCallbackAction::Invoked,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    Acquire,
    Release,
}

impl ResourceAction {
    fn codebook(self) -> ModelResourceAction {
        match self {
            Self::Acquire => ModelResourceAction::Acquire,
            Self::Release => ModelResourceAction::Release,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exit {
    Normal,
    Exceptional,
    Finally,
}

impl Exit {
    fn codebook(self) -> ModelExit {
        match self {
            Self::Normal => ModelExit::Normal,
            Self::Exceptional => ModelExit::Exceptional,
            Self::Finally => ModelExit::Finally,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionAction {
    Raise,
    Catch,
    Convert,
    Suppress,
}

impl ExceptionAction {
    fn codebook(self) -> ModelExceptionAction {
        match self {
            Self::Raise => ModelExceptionAction::Raise,
            Self::Catch => ModelExceptionAction::Catch,
            Self::Convert => ModelExceptionAction::Convert,
            Self::Suppress => ModelExceptionAction::Suppress,
        }
    }
}

pub struct CompiledModel {
    pub model_id: Id,
    pub model: Model,
}

pub struct Catalog {
    pub digest: Digest,
    pub models: Vec<CompiledModel>,
}

#[derive(Default)]
pub struct CompiledRules {
    pub transfers: Vec<ModelTransfersRow>,
    pub effects: Vec<ModelEffectsRow>,
    pub callbacks: Vec<ModelCallbacksRow>,
    pub resources: Vec<ModelResourcesRow>,
    pub exceptions: Vec<ModelExceptionsRow>,
    pub formals: Vec<ModelFormalPathsRow>,
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
                model.coverage.validate_rule(rule)?;
            }
            if model.normal_return
                && (model.coverage.exceptions != ChannelCoverage::Complete
                    || model.rules.iter().any(|rule| matches!(rule, Rule::Exception { .. })))
            {
                return Err(format!(
                    "{}: normal_return requires complete no-exception coverage",
                    model.target.key()
                ));
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

    /// Bind each applicable authored target to its pinned context definition. A model for a
    /// different Python or dependency version is dormant. Context definitions are sparse: a
    /// pinned module without this referenced callable also leaves the model dormant.
    /// This does not apply the model's rules or assert that its formal paths resolve.
    pub fn bind_targets(
        &self,
        snapshot_id: Id,
        contexts: &[ContextsRow],
        modules: &[ContextModulesRow],
        definitions: &[ContextDefinitionsRow],
    ) -> Result<Vec<ModelTargetsRow>, String> {
        let mut out: BTreeMap<(Id, Id), ModelTargetsRow> = BTreeMap::new();
        for compiled in &self.models {
            let target = &compiled.model.target;
            let applicable_modules: Vec<_> = modules
                .iter()
                .filter(|m| match target {
                    Target::Stdlib { python, module, .. } => {
                        m.module_name == *module
                            && m.origin == ModuleOrigin::BundledTypeshed
                            && contexts.iter().any(|c| c.python_version == *python)
                    }
                    Target::Dependency {
                        distribution,
                        version,
                        module,
                        ..
                    } => {
                        m.module_name == *module
                            && m.origin == ModuleOrigin::SitePackages
                            && m.distribution.as_deref() == Some(distribution)
                            && m.version.as_deref() == Some(version)
                    }
                    Target::Release { .. } => false,
                })
                .collect();
            if matches!(target, Target::Release { .. }) {
                return Err(format!(
                    "release model target binding is not implemented: {}",
                    target.key()
                ));
            }
            if applicable_modules.is_empty() {
                continue;
            }
            if matches!(target, Target::Stdlib { .. })
                && contexts
                    .iter()
                    .any(|c| !matches!(target, Target::Stdlib { python, .. } if c.python_version == *python))
            {
                return Err(format!(
                    "mixed Python versions cannot safely bind {}",
                    target.key()
                ));
            }
            let callable = match target {
                Target::Stdlib { callable, .. } | Target::Dependency { callable, .. } => callable,
                Target::Release { .. } => unreachable!(),
            };
            for module in applicable_modules {
                let matches: Vec<_> = definitions
                    .iter()
                    .filter(|d| {
                        d.module_node_id == module.module_node_id
                            && d.qualified_name == *callable
                            && matches!(d.kind, DefinitionKind::Function | DefinitionKind::Class)
                    })
                    .collect();
                for definition in matches {
                    if compiled.model.normal_return && definition.kind != DefinitionKind::Function {
                        return Err(format!(
                            "{}: normal_return requires a function target",
                            target.key()
                        ));
                    }
                    let row = ModelTargetsRow {
                        snapshot_id,
                        model_id: compiled.model_id,
                        target_node_id: definition.symbol_node_id,
                        target_module_fact_id: module.fact_id,
                        target_definition_fact_id: definition.fact_id,
                        target_key: target.key(),
                        revision: i64::from(compiled.model.revision),
                        transfer_coverage: compiled.model.coverage.transfers.codebook(),
                        effect_coverage: compiled.model.coverage.effects.codebook(),
                        callback_coverage: compiled.model.coverage.callbacks.codebook(),
                        resource_coverage: compiled.model.coverage.resources.codebook(),
                        exception_coverage: compiled.model.coverage.exceptions.codebook(),
                        normal_return: compiled.model.normal_return,
                        origin: Origin::SyntheticModel,
                    };
                    let key = (row.model_id, row.target_node_id);
                    match out.get(&key) {
                        Some(old)
                            if (old.target_module_fact_id, old.target_definition_fact_id)
                                <= (row.target_module_fact_id, row.target_definition_fact_id) => {}
                        _ => {
                            out.insert(key, row);
                        }
                    }
                }
            }
        }
        Ok(out.into_values().collect())
    }

    /// Compile every authored family in one pass, against the same pinned target and complete
    /// signature set. The caller publishes these together, or publishes none of them.
    pub fn compile_rules(
        &self,
        targets: &[ModelTargetsRow],
        definitions: &[ContextDefinitionsRow],
        parameters: &[ContextParametersRow],
    ) -> Result<CompiledRules, String> {
        let by_id: BTreeMap<Id, &Model> = self
            .models
            .iter()
            .map(|compiled| (compiled.model_id, &compiled.model))
            .collect();
        let mut out = CompiledRules::default();
        for target in targets {
            let model = by_id
                .get(&target.model_id)
                .ok_or_else(|| format!("unknown model id {}", target.model_id.hex()))?;
            let definition = definitions
                .iter()
                .find(|d| d.fact_id == target.target_definition_fact_id)
                .ok_or_else(|| format!("missing definition for {}", target.target_key))?;
            let signature_count = definition
                .signature_count
                .filter(|count| *count > 0)
                .ok_or_else(|| format!("no complete signature for {}", target.target_key))?;
            for (index, rule) in model.rules.iter().enumerate() {
                let formals: Vec<Option<&str>> = match rule {
                    Rule::Transfer { from, to, .. } => vec![from.formal(), to.formal()],
                    Rule::Effect { subject, .. } => {
                        vec![subject.as_ref().and_then(InputPath::formal)]
                    }
                    Rule::Callback { callback, .. } => vec![callback.formal()],
                    Rule::Resource { resource, .. } => vec![resource.formal()],
                    Rule::Exception { .. } => Vec::new(),
                };
                validate_formals(target, signature_count, parameters, &formals)?;
                let rule_id = IdHasher::new("behavior-model-rule")
                    .opt_id(Some(target.model_id))
                    .i64(index as i64)
                    .finish_id();
                let paths: Vec<(ModelPathRole, Id, Option<&str>)> = match rule {
                    Rule::Transfer { from, to, .. } => vec![
                        (ModelPathRole::Input, from.id(), from.formal()),
                        (ModelPathRole::Output, to.id(), to.formal()),
                    ],
                    Rule::Effect { subject, .. } => subject
                        .as_ref()
                        .map(|path| (ModelPathRole::Input, path.id(), path.formal()))
                        .into_iter()
                        .collect(),
                    Rule::Callback { callback, .. } => {
                        vec![(ModelPathRole::Input, callback.id(), callback.formal())]
                    }
                    Rule::Resource { resource, .. } => {
                        vec![(resource.role(), resource.id(), resource.formal())]
                    }
                    Rule::Exception { .. } => Vec::new(),
                };
                for (path_role, path_id, formal_name) in paths {
                    if let Some(formal_name) = formal_name {
                        out.formals.push(ModelFormalPathsRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            path_id,
                            path_role,
                            formal_name: formal_name.to_owned(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                }
                match rule {
                    Rule::Transfer {
                        from,
                        to,
                        transfer,
                        modality,
                    } => {
                        out.transfers.push(ModelTransfersRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            input_path_id: from.id(),
                            input_path_kind: from.kind(),
                            input_path: from.render(),
                            output_path_id: to.id(),
                            output_path_kind: to.kind(),
                            output_path: to.render(),
                            transfer: match transfer {
                                Transfer::Identity => ModelTransferKind::Identity,
                                Transfer::Transform => ModelTransferKind::Transform,
                            },
                            modality: modality.codebook(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                    Rule::Effect {
                        effect,
                        subject,
                        modality,
                    } => {
                        let (kind, argument) = effect.kind_argument();
                        out.effects.push(ModelEffectsRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            effect: kind,
                            argument: argument.map(str::to_owned),
                            subject_path_id: subject.as_ref().map(InputPath::id),
                            subject_path_kind: subject.as_ref().map(InputPath::kind),
                            subject_path: subject.as_ref().map(InputPath::render),
                            modality: modality.codebook(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                    Rule::Callback {
                        callback,
                        action,
                        exit,
                        modality,
                    } => {
                        out.callbacks.push(ModelCallbacksRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            callback_path_id: callback.id(),
                            callback_path: callback.render(),
                            action: action.codebook(),
                            exit: exit.codebook(),
                            modality: modality.codebook(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                    Rule::Resource {
                        resource,
                        action,
                        exit,
                        modality,
                    } => {
                        out.resources.push(ModelResourcesRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            resource_path_id: resource.id(),
                            resource_path: resource.render(),
                            resource_role: resource.role(),
                            resource_path_kind: resource.kind(),
                            action: action.codebook(),
                            exit: exit.codebook(),
                            modality: modality.codebook(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                    Rule::Exception {
                        class,
                        action,
                        to_class,
                        modality,
                    } => {
                        let source_class = resolve_exception_class(class, definitions)?;
                        let replacement = to_class
                            .as_deref()
                            .map(|name| resolve_exception_class(name, definitions))
                            .transpose()?;
                        out.exceptions.push(ModelExceptionsRow {
                            snapshot_id: target.snapshot_id,
                            model_id: target.model_id,
                            target_node_id: target.target_node_id,
                            rule_id,
                            target_definition_fact_id: target.target_definition_fact_id,
                            revision: target.revision,
                            class: class.clone(),
                            class_node_id: source_class.symbol_node_id,
                            class_fact_id: source_class.fact_id,
                            action: action.codebook(),
                            to_class: to_class.clone(),
                            to_class_node_id: replacement.map(|row| row.symbol_node_id),
                            to_class_fact_id: replacement.map(|row| row.fact_id),
                            modality: modality.codebook(),
                            origin: Origin::SyntheticModel,
                        });
                    }
                }
            }
        }
        Ok(out)
    }
}

fn resolve_exception_class<'a>(
    name: &str,
    definitions: &'a [ContextDefinitionsRow],
) -> Result<&'a ContextDefinitionsRow, String> {
    let mut matches = definitions.iter().filter(|d| {
        d.kind == DefinitionKind::Class && format!("{}.{}", d.module_name, d.qualified_name) == name
    });
    let class = matches
        .next()
        .ok_or_else(|| format!("model exception class {name} has no pinned context definition"))?;
    if matches.next().is_some() {
        return Err(format!(
            "model exception class {name} has multiple pinned context definitions"
        ));
    }
    Ok(class)
}

fn validate_formals(
    target: &ModelTargetsRow,
    signature_count: i64,
    parameters: &[ContextParametersRow],
    formals: &[Option<&str>],
) -> Result<(), String> {
    for signature_index in 0..signature_count {
        let rows: Vec<_> = parameters
            .iter()
            .filter(|p| {
                p.symbol_node_id == target.target_node_id && p.signature_index == signature_index
            })
            .collect();
        if rows.is_empty() || rows.iter().any(|p| p.form != SignatureForm::List) {
            return Err(format!(
                "unresolved signature {signature_index} for {}",
                target.target_key
            ));
        }
        for formal in formals.iter().flatten() {
            if rows
                .iter()
                .filter(|p| p.name.as_deref() == Some(*formal))
                .count()
                != 1
            {
                return Err(format!(
                    "unresolved formal {formal} in signature {signature_index} of {}",
                    target.target_key
                ));
            }
        }
    }
    Ok(())
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
    fn binding_requires_the_exact_context_and_a_cited_definition() {
        let catalog = Catalog::committed().unwrap();
        let snapshot_id = Id([1; 16]);
        let module_node_id = Id([2; 16]);
        let context = ContextsRow {
            snapshot_id,
            context_id: Id([3; 16]),
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: Vec::new(),
            site_package_path: Vec::new(),
            config_digest: Digest([0; 32]),
            environment_digest: Digest([0; 32]),
            lock_digest: None,
        };
        let module = ContextModulesRow {
            snapshot_id,
            fact_id: Id([4; 16]),
            module_node_id,
            module_name: "typing".into(),
            origin: ModuleOrigin::BundledTypeshed,
            path: Some("stdlib/typing.pyi".into()),
            distribution: None,
            version: None,
        };
        let definition = ContextDefinitionsRow {
            snapshot_id,
            fact_id: Id([5; 16]),
            symbol_node_id: Id([6; 16]),
            module_node_id,
            module_name: "typing".into(),
            kind: DefinitionKind::Function,
            key: "cast-key".into(),
            name: "cast".into(),
            qualified_name: "cast".into(),
            is_top_level: true,
            signature_count: Some(1),
        };
        let bound = catalog
            .bind_targets(
                snapshot_id,
                std::slice::from_ref(&context),
                std::slice::from_ref(&module),
                std::slice::from_ref(&definition),
            )
            .unwrap();
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0].target_node_id, definition.symbol_node_id);
        assert_eq!(bound[0].target_definition_fact_id, definition.fact_id);
        assert!(bound[0].normal_return);
        assert_eq!(bound[0].origin, Origin::SyntheticModel);
        let class_definition = ContextDefinitionsRow {
            kind: DefinitionKind::Class,
            ..definition.clone()
        };
        assert!(catalog
            .bind_targets(
                snapshot_id,
                std::slice::from_ref(&context),
                std::slice::from_ref(&module),
                &[class_definition],
            )
            .is_err());
        let parameter = ContextParametersRow {
            snapshot_id,
            fact_id: Id([7; 16]),
            symbol_node_id: definition.symbol_node_id,
            module_node_id,
            signature_index: 0,
            form: SignatureForm::List,
            ordinal: Some(1),
            kind: Some(crate::codebook::ParameterKind::PositionalOrKeyword),
            name: Some("val".into()),
            required: Some(true),
        };
        let compiled = catalog
            .compile_rules(
                &bound,
                std::slice::from_ref(&definition),
                std::slice::from_ref(&parameter),
            )
            .unwrap();
        let transfers = compiled.transfers;
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].target_node_id, definition.symbol_node_id);
        assert_eq!(transfers[0].input_path, "Parameter[val]");
        assert_eq!(transfers[0].output_path, "ReturnValue");
        assert_eq!(transfers[0].transfer, ModelTransferKind::Identity);
        assert!(
            catalog
                .compile_rules(&bound, std::slice::from_ref(&definition), &[])
                .is_err()
        );
        let renamed = ContextParametersRow {
            name: Some("value".into()),
            ..parameter
        };
        assert!(
            catalog
                .compile_rules(&bound, std::slice::from_ref(&definition), &[renamed])
                .is_err()
        );

        assert!(
            catalog
                .bind_targets(
                    snapshot_id,
                    std::slice::from_ref(&context),
                    std::slice::from_ref(&module),
                    &[],
                )
                .unwrap()
                .is_empty()
        );
        let wrong_context = ContextsRow {
            python_version: "3.14.6".into(),
            ..context
        };
        assert!(
            catalog
                .bind_targets(
                    snapshot_id,
                    &[wrong_context],
                    std::slice::from_ref(&module),
                    &[],
                )
                .unwrap()
                .is_empty()
        );
        let wrong_origin = ContextModulesRow {
            origin: ModuleOrigin::SitePackages,
            ..module
        };
        assert!(
            catalog
                .bind_targets(snapshot_id, &[], &[wrong_origin], &[])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn committed_catalog_has_typed_identity_path_and_digest() {
        let catalog = Catalog::committed().unwrap();
        assert_eq!(catalog.digest, Catalog::committed_digest());
        assert_eq!(catalog.models.len(), 12);
        let model = &catalog
            .models
            .iter()
            .find(|m| m.model.target.key() == "stdlib:3.14.7:typing.cast")
            .unwrap()
            .model;
        assert_eq!(model.target.key(), "stdlib:3.14.7:typing.cast");
        assert!(model.normal_return);
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
            ("dependency:pydantic==2.13.5:pydantic.type_adapter.TypeAdapter.validate_python", "object"),
        ] {
            let added = &catalog.models.iter().find(|m| m.model.target.key() == target)
                .unwrap().model;
            assert!(!added.normal_return, "fallible {target} cannot assert total completion");
            assert!(added.rules.iter().any(|rule| matches!(rule,
                Rule::Transfer {
                    from: InputPath::Parameter { name },
                    to: OutputPath::ReturnValue,
                    transfer: Transfer::Transform,
                    modality: RuleModality::Potential,
                } if name == formal)), "{target} must retain its exact input formal");
        }
        let logging = &catalog.models.iter().find(|m| {
            m.model.target.key() == "stdlib:3.14.7:logging.Logger.warning"
        }).unwrap().model;
        assert!(!logging.normal_return);
        assert!(logging.rules.iter().any(|rule| matches!(rule,
            Rule::Effect {
                effect: Effect::Log,
                subject: Some(InputPath::Parameter { name }),
                modality: RuleModality::Potential,
            } if name == "msg")));
        let adapter = &catalog.models.iter().find(|m| {
            m.model.target.key()
                == "dependency:pydantic==2.13.5:pydantic.type_adapter.TypeAdapter.validate_python"
        }).unwrap().model;
        assert!(matches!(adapter.coverage.effects, ChannelCoverage::Unspecified));
        assert!(!adapter.rules.iter().any(|rule| matches!(rule, Rule::Effect { .. })));
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
        let header = r#"version = 1
[[models]]
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
    fn normal_return_requires_complete_exception_coverage_without_exception_rules() {
        let header = r#"version = 1
[[models]]
revision = 1
target = { scope = "stdlib", python = "3.14.7", module = "typing", callable = "cast" }
normal_return = true
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
