//! Typed channel-specific application of immutable authored rules. Rule ordinals reference the
//! selected parser authority; channels with unsupported endpoints retain explicit uncertainty.
use super::model_application::{CheckedModelApplication, ModelApplicationData};
use crate::domain::{
    attribution::{Modality, ObligationKind},
    calls::{BindingProjection, BindingSource, SignatureParameter},
    models::{ChannelCoverage, InputPath, OutputPath, ResourcePath, Rule},
    normalized::Rows,
    resources::ResourceBudget,
    source::Occurrence,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ModelChannel {
    Transfers = 0,
    Effects = 1,
    Callbacks = 2,
    Resources = 3,
    Exceptions = 4,
}
impl ModelChannel {
    pub const ALL: &[Self] = &[
        Self::Transfers,
        Self::Effects,
        Self::Callbacks,
        Self::Resources,
        Self::Exceptions,
    ];
    pub fn declared(self, channels: &models::Channels) -> DeclaredCoverage {
        match match self {
            Self::Transfers => channels.transfers,
            Self::Effects => channels.effects,
            Self::Callbacks => channels.callbacks,
            Self::Resources => channels.resources,
            Self::Exceptions => channels.exceptions,
        } {
            ChannelCoverage::Complete => DeclaredCoverage::Complete,
            ChannelCoverage::Partial => DeclaredCoverage::Partial,
            ChannelCoverage::Unspecified => DeclaredCoverage::Unspecified,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DeclaredCoverage {
    Complete = 0,
    Partial = 1,
    Unspecified = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ActionPhase {
    Invocation = 0,
    Normal = 1,
    Exceptional = 2,
    Finally = 3,
}
impl From<models::Exit> for ActionPhase {
    fn from(v: models::Exit) -> Self {
        match v {
            models::Exit::Invocation => Self::Invocation,
            models::Exit::Normal => Self::Normal,
            models::Exit::Exceptional => Self::Exceptional,
            models::Exit::Finally => Self::Finally,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "model_value_paths")]
pub enum ModelValuePath {
    #[model(code = 0)]
    Argument {
        formal: Id<SignatureParameter>,
        source: Id<BindingSource>,
        projection: Id<BindingProjection>,
    },
    #[model(code = 1)]
    Returned { site: Id<Occurrence> },
    #[model(code = 2)]
    Field {
        formal: Id<SignatureParameter>,
        source: Id<BindingSource>,
        field: String,
    },
    #[model(code = 3)]
    Global { symbol: Id<calls::ProviderSymbol> },
    #[model(code = 4)]
    Raised { class: Id<calls::ProviderSymbol> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CallbackOperation {
    Stored = 0,
    Registered = 1,
    Forwarded = 2,
    Invoked = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ResourceOperation {
    Acquire = 0,
    Release = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ExceptionOperation {
    Raise = 0,
    Catch = 1,
    Convert = 2,
    Suppress = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "modeled_operations")]
pub enum ModeledOperation {
    #[model(code = 0)]
    Transfer { kind: transfer::TransferKind },
    #[model(code = 1)]
    IoRead,
    #[model(code = 2)]
    IoWrite,
    #[model(code = 3)]
    Net,
    #[model(code = 4)]
    Log,
    #[model(code = 5)]
    Timeout,
    #[model(code = 6)]
    ThreadDispatch,
    #[model(code = 7)]
    Compress { format: String },
    #[model(code = 8)]
    Serialize { format: String },
    #[model(code = 9)]
    ValidateClass {
        class: Option<Id<calls::ProviderSymbol>>,
    },
    #[model(code = 10)]
    ValidateValue { source: Option<Id<ModelValuePath>> },
    #[model(code = 11)]
    ValidateUnresolved,
    #[model(code = 12)]
    Register { container: String },
    #[model(code = 13)]
    Invoke { callable: String },
    #[model(code = 14)]
    Callback { action: CallbackOperation },
    #[model(code = 15)]
    Resource { action: ResourceOperation },
    #[model(code = 16)]
    Exception { action: ExceptionOperation },
}
/// A resource's value identity remains the exact argument projection or returned call value,
/// independently of whether its phase postcondition is currently established.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "modeled_resource_identities",
    rule = "modeled_resource_value_identity"
)]
pub struct ResourceIdentity {
    #[model(key, premise)]
    pub rule: Id<AppliedRule>,
    #[model(premise)]
    pub value: Id<ModelValuePath>,
    pub action: ResourceOperation,
    pub phase: ActionPhase,
}
/// Applicability of one authored rule; admission of its exit postcondition is a separate record.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_applied_rules")]
pub struct AppliedRule {
    #[model(key)]
    pub application: Id<super::model_production::ModelApplication>,
    #[model(key)]
    pub ordinal: i64,
    pub channel: ModelChannel,
    pub phase: ActionPhase,
    pub modality: Modality,
    pub operation: Id<ModeledOperation>,
    pub source: Option<Id<ModelValuePath>>,
    pub destination: Option<Id<ModelValuePath>>,
    pub exception: Option<Id<calls::ProviderSymbol>>,
    pub conversion: Option<Id<calls::ProviderSymbol>>,
    pub applicable: bool,
    pub reason: Option<ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "model_channel_assessments")]
pub struct ChannelAssessment {
    #[model(key)]
    pub application: Id<super::model_production::ModelApplication>,
    #[model(key)]
    pub channel: ModelChannel,
    pub declared: DeclaredCoverage,
    pub applicable: i64,
    pub refused: i64,
    pub complete: bool,
}
pub struct AppliedRules {
    pub rules: Rows<AppliedRule>,
    pub paths: Rows<ModelValuePath>,
    pub sources: Rows<BindingSource>,
    pub projections: Rows<BindingProjection>,
    pub channels: Rows<ChannelAssessment>,
    pub operations: Rows<ModeledOperation>,
    pub resources: Rows<ResourceIdentity>,
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ObligationKind> {
    rows.get(id).ok_or(ObligationKind::MissingEvidence)
}
fn one<'a, T: 'a>(mut rows: impl Iterator<Item = &'a T>) -> Result<&'a T, ObligationKind> {
    let row = rows.next().ok_or(ObligationKind::MissingEvidence)?;
    if rows.next().is_some() {
        return Err(ObligationKind::AmbiguousBinding);
    }
    Ok(row)
}
fn argument<'a>(
    application: &'a CheckedModelApplication<'_>,
    data: &ModelApplicationData,
    name: &str,
) -> Result<&'a calls::Binding, ObligationKind> {
    let signature = need(
        &data.bindings.signatures,
        application.bound().bound().signature(),
    )?;
    let formal = super::model_application::formal(&data.bindings, signature, Some(name))?
        .ok_or(ObligationKind::MissingEvidence)?;
    one(application
        .bound()
        .bound()
        .bindings()
        .iter()
        .filter(|b| b.formal == formal))
}
fn class(
    data: &ModelApplicationData,
    name: &str,
    application: &CheckedModelApplication<'_>,
    budget: &ResourceBudget,
) -> Result<Result<Id<calls::ProviderSymbol>, ObligationKind>, ModelError> {
    let Some((module, name)) = name.rsplit_once('.') else {
        return Ok(Err(ObligationKind::MissingEvidence));
    };
    let symbol = match one(data.bindings.symbols.iter().filter(|s| {
        s.name == name
            && s.context == application.shape().context()
            && s.kind == calls::SymbolKind::Class
            && match data.bindings.provider_modules.get(s.module) {
                Some(calls::ProviderModule::Bundled { name, .. }) => name == module,
                Some(calls::ProviderModule::Acquired { module: m }) => data
                    .bindings
                    .modules
                    .get(*m)
                    .is_some_and(|m| m.qualified_name == module),
                _ => false,
            }
    })) {
        Ok(s) => s,
        Err(r) => return Ok(Err(r)),
    };
    Ok(super::model_context::CheckedExactClass::derive(
        data,
        symbol.id(),
        application.shape().context(),
        budget,
    )?
    .map(|class| class.symbol()))
}
fn input(
    application: &CheckedModelApplication<'_>,
    data: &ModelApplicationData,
    path: &InputPath,
    out: &mut AppliedRules,
    budget: &ResourceBudget,
) -> Result<Result<Id<ModelValuePath>, ObligationKind>, ModelError> {
    let _scratch = budget.reserve(
        "model-input-path-scratch",
        match path {
            InputPath::ReceiverField { field, .. } => field.len(),
            _ => 0,
        },
    )?;
    let result = (|| -> Result<ModelValuePath, ObligationKind> {
        match path {
            InputPath::Parameter { name } => {
                let b = argument(application, data, name)?;
                Ok(ModelValuePath::Argument {
                    formal: b.formal,
                    source: b.source.id(),
                    projection: b.projection.id(),
                })
            }
            InputPath::ReceiverField { root, field } => {
                let b = argument(application, data, root)?;
                if b.projection != BindingProjection::Whole {
                    return Err(ObligationKind::UnsupportedUnpacking);
                }
                Ok(ModelValuePath::Field {
                    formal: b.formal,
                    source: b.source.id(),
                    field: field.clone(),
                })
            }
            InputPath::Global { module, name } => {
                let symbol = one(data.bindings.symbols.iter().filter(|s| {
                    s.name == *name
                        && s.context == application.shape().context()
                        && match data.bindings.provider_modules.get(s.module) {
                            Some(calls::ProviderModule::Acquired { module: m }) => data
                                .bindings
                                .modules
                                .get(*m)
                                .is_some_and(|m| m.qualified_name == *module),
                            Some(calls::ProviderModule::Bundled { name, .. }) => name == module,
                            _ => false,
                        }
                }))?;
                Ok(ModelValuePath::Global {
                    symbol: symbol.id(),
                })
            }
        }
    })();
    let _ = budget;
    match result {
        Ok(path) => Ok(Ok(out.paths.insert(path)?)),
        Err(r) => Ok(Err(r)),
    }
}
fn output(
    application: &CheckedModelApplication<'_>,
    data: &ModelApplicationData,
    path: &OutputPath,
    out: &mut AppliedRules,
    budget: &ResourceBudget,
) -> Result<Result<Id<ModelValuePath>, ObligationKind>, ModelError> {
    let bytes = match path {
        OutputPath::Parameter { name } => name.len(),
        OutputPath::ReceiverField { root, field } => root.len() + field.len(),
        OutputPath::Global { module, name } => module.len() + name.len(),
        _ => 0,
    };
    let _scratch = budget.reserve(
        "model-output-path-scratch",
        bytes
            .saturating_mul(2)
            .saturating_add(size_of::<InputPath>()),
    )?;
    match path {
        OutputPath::ReturnValue => Ok(Ok(out.paths.insert(ModelValuePath::Returned {
            site: application.bound().bound().site(),
        })?)),
        OutputPath::Raise { class: name } => match class(data, name, application, budget)? {
            Ok(class) => Ok(Ok(out.paths.insert(ModelValuePath::Raised { class })?)),
            Err(r) => Ok(Err(r)),
        },
        OutputPath::Parameter { name } => input(
            application,
            data,
            &InputPath::Parameter { name: name.clone() },
            out,
            budget,
        ),
        OutputPath::ReceiverField { root, field } => input(
            application,
            data,
            &InputPath::ReceiverField {
                root: root.clone(),
                field: field.clone(),
            },
            out,
            budget,
        ),
        OutputPath::Global { module, name } => input(
            application,
            data,
            &InputPath::Global {
                module: module.clone(),
                name: name.clone(),
            },
            out,
            budget,
        ),
    }
}
impl AppliedRules {
    pub fn derive(
        application: &CheckedModelApplication<'_>,
        id: Id<super::model_production::ModelApplication>,
        data: &ModelApplicationData,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut out = Self {
            rules: Rows::new(budget),
            paths: Rows::new(budget),
            sources: Rows::new(budget),
            projections: Rows::new(budget),
            channels: Rows::new(budget),
            operations: Rows::new(budget),
            resources: Rows::new(budget),
        };
        for binding in application.bound().bound().bindings() {
            out.sources.insert(binding.source.clone())?;
            out.projections.insert(binding.projection.clone())?;
        }
        for (ordinal, rule) in application.compiled().model().rules.iter().enumerate() {
            let (channel, phase, modality) = match rule {
                Rule::Transfer { modality, .. } => {
                    (ModelChannel::Transfers, ActionPhase::Normal, *modality)
                }
                Rule::Effect { exit, modality, .. } => {
                    (ModelChannel::Effects, (*exit).into(), *modality)
                }
                Rule::Callback { exit, modality, .. } => {
                    (ModelChannel::Callbacks, (*exit).into(), *modality)
                }
                Rule::Resource { exit, modality, .. } => {
                    (ModelChannel::Resources, (*exit).into(), *modality)
                }
                Rule::Exception { modality, .. } => (
                    ModelChannel::Exceptions,
                    ActionPhase::Exceptional,
                    *modality,
                ),
            };
            let mut row = AppliedRule {
                application: id,
                ordinal: ordinal as i64,
                channel,
                phase,
                operation: ModeledOperation::ValidateUnresolved.id(),
                modality: match modality {
                    models::RuleModality::Definite => Modality::Definite,
                    models::RuleModality::Potential => Modality::Potential,
                },
                source: None,
                destination: None,
                exception: None,
                conversion: None,
                applicable: true,
                reason: None,
            };
            let mut refusals = Vec::new();
            let mut charge = charged::StateCharge::new(budget, "model-rule-path-work");
            charge.grow(size_of::<ObligationKind>() * 8)?;
            macro_rules! path {
                ($field:ident,$value:expr) => {
                    match $value? {
                        Ok(path) => row.$field = Some(path),
                        Err(r) => refusals.push(r),
                    }
                };
            }
            match rule {
                Rule::Transfer { from, to, .. } => {
                    path!(source, input(application, data, from, &mut out, budget));
                    path!(destination, output(application, data, to, &mut out, budget));
                }
                Rule::Effect {
                    subject, effect, ..
                } => {
                    if let Some(subject) = subject {
                        path!(source, input(application, data, subject, &mut out, budget));
                    }
                    match effect.schema() {
                        Some(models::ValidationSchema::StaticClass { class: name }) => {
                            path!(exception, class(data, name, application, budget));
                        }
                        Some(models::ValidationSchema::RuntimeValue { source }) => {
                            path!(
                                destination,
                                input(application, data, source, &mut out, budget)
                            );
                        }
                        Some(models::ValidationSchema::Unresolved {}) => {
                            refusals.push(ObligationKind::OutsideProviderModel)
                        }
                        None => {}
                    }
                }
                Rule::Callback { callback, .. } => {
                    path!(source, input(application, data, callback, &mut out, budget));
                }
                Rule::Resource { resource, .. } => match resource {
                    ResourcePath::Input { path: p } => {
                        path!(source, input(application, data, p, &mut out, budget));
                    }
                    ResourcePath::Output { path: p } => {
                        path!(destination, output(application, data, p, &mut out, budget));
                    }
                },
                Rule::Exception {
                    class: name,
                    to_class,
                    ..
                } => {
                    path!(exception, class(data, name, application, budget));
                    if let Some(name) = to_class {
                        path!(conversion, class(data, name, application, budget));
                    }
                }
            }
            if let Some(reason) = refusals.first() {
                row.applicable = false;
                row.reason = Some(*reason);
            }
            let bytes = match rule {
                Rule::Effect {
                    effect:
                        models::Effect::Compress { format } | models::Effect::Serialize { format },
                    ..
                } => format.len(),
                Rule::Effect {
                    effect: models::Effect::Register { container },
                    ..
                } => container.len(),
                Rule::Effect {
                    effect: models::Effect::Invoke { callable },
                    ..
                } => callable.len(),
                _ => 0,
            };
            let _operation = budget.reserve(
                "model-operation-lowering",
                bytes.saturating_mul(2) + size_of::<ModeledOperation>(),
            )?;
            let operation = match rule {
                Rule::Transfer { transfer, .. } => ModeledOperation::Transfer {
                    kind: match transfer {
                        models::Transfer::Identity => transfer::TransferKind::Identity,
                        models::Transfer::Transform => transfer::TransferKind::Derived,
                    },
                },
                Rule::Effect { effect, .. } => match effect {
                    models::Effect::IoRead => ModeledOperation::IoRead,
                    models::Effect::IoWrite => ModeledOperation::IoWrite,
                    models::Effect::Net => ModeledOperation::Net,
                    models::Effect::Log => ModeledOperation::Log,
                    models::Effect::Timeout => ModeledOperation::Timeout,
                    models::Effect::ThreadDispatch => ModeledOperation::ThreadDispatch,
                    models::Effect::Compress { format } => ModeledOperation::Compress {
                        format: format.clone(),
                    },
                    models::Effect::Serialize { format } => ModeledOperation::Serialize {
                        format: format.clone(),
                    },
                    models::Effect::Register { container } => ModeledOperation::Register {
                        container: container.clone(),
                    },
                    models::Effect::Invoke { callable } => ModeledOperation::Invoke {
                        callable: callable.clone(),
                    },
                    models::Effect::Validate { schema } => match schema {
                        models::ValidationSchema::StaticClass { .. } => {
                            ModeledOperation::ValidateClass {
                                class: row.exception,
                            }
                        }
                        models::ValidationSchema::RuntimeValue { .. } => {
                            ModeledOperation::ValidateValue {
                                source: row.destination,
                            }
                        }
                        models::ValidationSchema::Unresolved {} => {
                            ModeledOperation::ValidateUnresolved
                        }
                    },
                },
                Rule::Callback { action, .. } => ModeledOperation::Callback {
                    action: match action {
                        models::CallbackAction::Stored => CallbackOperation::Stored,
                        models::CallbackAction::Registered => CallbackOperation::Registered,
                        models::CallbackAction::Forwarded => CallbackOperation::Forwarded,
                        models::CallbackAction::Invoked => CallbackOperation::Invoked,
                    },
                },
                Rule::Resource { action, .. } => ModeledOperation::Resource {
                    action: match action {
                        models::ResourceAction::Acquire => ResourceOperation::Acquire,
                        models::ResourceAction::Release => ResourceOperation::Release,
                    },
                },
                Rule::Exception { action, .. } => ModeledOperation::Exception {
                    action: match action {
                        models::ExceptionAction::Raise => ExceptionOperation::Raise,
                        models::ExceptionAction::Catch => ExceptionOperation::Catch,
                        models::ExceptionAction::Convert => ExceptionOperation::Convert,
                        models::ExceptionAction::Suppress => ExceptionOperation::Suppress,
                    },
                },
            };
            row.operation = out.operations.insert(operation.clone())?;
            if row.applicable {
                if let ModeledOperation::Resource { action } = operation {
                    if let Some(value) = row.source.or(row.destination) {
                        out.resources.insert(ResourceIdentity {
                            rule: row.id(),
                            value,
                            action,
                            phase: row.phase,
                        })?;
                    }
                }
            }
            out.rules.insert(row)?;
        }
        for channel in ModelChannel::ALL {
            let declared = channel.declared(application.channels());
            let applicable = out
                .rules
                .iter()
                .filter(|r| r.channel == *channel && r.applicable)
                .count() as i64;
            let refused = out
                .rules
                .iter()
                .filter(|r| r.channel == *channel && !r.applicable)
                .count() as i64;
            out.channels.insert(ChannelAssessment {
                application: id,
                channel: *channel,
                declared,
                applicable,
                refused,
                complete: declared == DeclaredCoverage::Complete && refused == 0,
            })?;
        }
        Ok(out)
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<AppliedRule>(),
        Relation::of::<ModelValuePath>(),
        Relation::of::<ChannelAssessment>(),
        Relation::of::<ModeledOperation>(),
        Relation::of::<ResourceIdentity>(),
    ]
}
