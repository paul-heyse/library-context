//! Early authored applicability has no dependency on Enriched execution or Model publication.
//! Structural shape and effective source identity are different capabilities. An external
//! native definition needs the exact authored runtime contract; inspection alone cannot supply it.
use crate::domain::{
    assertion::{Approximation, AssertionQualification},
    attribution::{AnalysisContext, Modality, ObligationKind},
    calls::*,
    input::*,
    models::{AuthoredModel, Catalog, CompiledModel, ModelCatalog, NormalBody, Rule, Target},
    normalized::{
        Rows,
        binding_normalization::{
            BindingData, BindingShapeAdmission, EffectiveInvocationAdmission, ValidatedBoundCall,
        },
        callables::SignatureAdjustment,
        entities::{CallableEntity, EntityRef},
    },
    resources::ResourceBudget,
    symbols::FunctionOrigin,
    *,
};

#[macro_export]
macro_rules! model_pin_inputs {($apply:ident)=>{$apply!{
 contexts:$crate::domain::attribution::AnalysisContext,acquisitions:$crate::domain::input::InputAcquisition,fingerprints:$crate::domain::input::EnvironmentFingerprint,
 packages:$crate::domain::input::Package,releases:$crate::domain::input::Release,distributions:$crate::domain::input::InputDistribution,verifications:$crate::domain::input::DistributionVerification,
 ownership:$crate::domain::input::ArtifactOwnership,uses:$crate::domain::input::ArtifactUse,
 native:$crate::domain::analysis::native::NativeQualification,premises:$crate::domain::analysis::native::NativeAssertionPremise,

}};}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
pub struct ModelApplicationData {pub bindings:BindingData,$(pub $field:Rows<$ty>,)*}
impl ModelApplicationData{
 pub fn new(budget:&ResourceBudget)->Self{Self{bindings:BindingData::new(budget),$($field:Rows::new(budget),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{
  $(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*self.bindings.visit(name,batch)
 }
 pub fn validation_inputs()->Vec<ValidationInput>{let mut inputs=BindingData::validation_inputs();$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*inputs}
}
};}
crate::model_pin_inputs!(data);
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

/// Borrowed catalog and bound call retain their owners' parser/binding reservations. No duplicate
/// generic model tree is allocated and no public constructor can assert applicability.
pub struct CheckedModelApplication<'a> {
    data: &'a ModelApplicationData,
    syntax: Id<CallSyntax>,
    compiled: &'a CompiledModel,
    bound: &'a ValidatedBoundCall,
    shape: &'a BindingShapeAdmission,
    normal_parameter: Option<(Id<SignatureParameter>, Id<source::Occurrence>)>,
    premises: Rows<analysis::native::NativeAssertionPremise>,
    status: analysis::policy::EvidenceStatus,
    _charge: charged::StateCharge,
}
impl<'a> CheckedModelApplication<'a> {
    pub fn arguments(&self) -> impl Iterator<Item = &CallArgument> {
        self.data
            .bindings
            .arguments
            .iter()
            .filter(|a| a.call == self.syntax)
    }
    pub fn compiled(&self) -> &CompiledModel {
        self.compiled
    }
    pub fn model(&self) -> Id<AuthoredModel> {
        self.compiled.declaration().id()
    }
    pub fn catalog(&self) -> Id<ModelCatalog> {
        self.compiled.declaration().catalog
    }
    pub fn bound(&self) -> &ValidatedBoundCall {
        self.bound
    }
    pub fn shape(&self) -> &BindingShapeAdmission {
        self.shape
    }
    pub fn normal_parameter(&self) -> Option<(Id<SignatureParameter>, Id<source::Occurrence>)> {
        self.normal_parameter
    }
    pub fn call_defaults_available(&self) -> bool {
        self.compiled.model().call_defaults_available
    }
    pub fn premises(&self) -> &Rows<analysis::native::NativeAssertionPremise> {
        &self.premises
    }
    pub fn status(&self) -> analysis::policy::EvidenceStatus {
        self.status
    }
    pub fn channels(&self) -> &models::Channels {
        &self.compiled.model().coverage
    }
    pub fn derive(
        catalog: &'a Catalog,
        data: &'a ModelApplicationData,
        bound: &'a ValidatedBoundCall,
        shape: &'a BindingShapeAdmission,
        effective: Option<&EffectiveInvocationAdmission>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "checked-model-application");
        charge.grow(size_of::<Self>())?;
        let result = (|| {
            if !shape.admits(bound) {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let bindings = &data.bindings;
            let signature = need(&bindings.signatures, bound.bound().signature())?;
            let symbol = need(&bindings.symbols, signature.symbol)?;
            if symbol.context != shape.context() {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let target = need(&bindings.targets, shape.target())?;
            if target.phase != shape.phase() {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let compiled = one(catalog.models().iter().filter(|compiled| {
                compiled.declaration().phase == shape.phase()
                    && matches_target(
                        data,
                        &compiled.model().target,
                        symbol,
                        shape.input(),
                        shape.context(),
                    )
                    .is_ok()
            }))?;
            runtime_identity(data, bound, shape, effective)?;
            exact(bindings, signature.qualification, shape.context())?;
            for rule in &compiled.model().rules {
                match rule {
                    Rule::Transfer { from, to, .. } => {
                        formal(bindings, signature, from.formal())?;
                        formal(bindings, signature, to.formal())?;
                    }
                    Rule::Effect {
                        subject, effect, ..
                    } => {
                        formal(
                            bindings,
                            signature,
                            subject.as_ref().and_then(models::InputPath::formal),
                        )?;
                        formal(
                            bindings,
                            signature,
                            effect
                                .schema()
                                .and_then(models::ValidationSchema::source)
                                .and_then(models::InputPath::formal),
                        )?;
                    }
                    Rule::Callback { callback, .. } => {
                        formal(bindings, signature, callback.formal())?;
                    }
                    Rule::Resource { resource, .. } => {
                        formal(bindings, signature, resource.formal())?;
                    }
                    Rule::Exception { .. } => {}
                }
            }
            let normal_parameter = if let Some(NormalBody::DirectReturnParameter { name }) =
                &compiled.model().normal_body
            {
                let formal = formal(bindings, signature, Some(name))?
                    .ok_or(ObligationKind::MissingEvidence)?;
                let binding = one(bound
                    .bound()
                    .bindings()
                    .iter()
                    .filter(|b| b.formal == formal))?;
                match (&binding.source, &binding.projection) {
                    (BindingSource::Actual { occurrence }, BindingProjection::Whole) => {
                        Some((formal, *occurrence))
                    }
                    _ => None,
                }
            } else {
                None
            };
            let syntax = one(bindings.syntax.iter().filter(|s| {
                s.site == bound.bound().site()
                    && bindings
                        .qualifications
                        .get(s.qualification)
                        .is_some_and(|q| q.context == shape.context())
            }))?;
            Ok(Self {
                data,
                syntax: syntax.id(),
                compiled,
                bound,
                shape,
                normal_parameter,
                premises: Rows::new(budget),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                _charge: charge,
            })
        })();
        match result {
            Err(reason) => Ok(Err(reason)),
            Ok(mut application) => match runtime_evidence(data, bound, shape, budget) {
                Ok((premises, status)) => {
                    application.premises = premises;
                    application.status = status;
                    Ok(Ok(application))
                }
                Err(RuntimeEvidenceError::Boundary(reason)) => Ok(Err(reason)),
                Err(RuntimeEvidenceError::Model(error)) => Err(error),
            },
        }
    }
}
#[derive(Debug)]
pub(super) enum RuntimeEvidenceError {
    Boundary(ObligationKind),
    Model(ModelError),
}
impl From<ObligationKind> for RuntimeEvidenceError {
    fn from(v: ObligationKind) -> Self {
        Self::Boundary(v)
    }
}
impl From<ModelError> for RuntimeEvidenceError {
    fn from(v: ModelError) -> Self {
        Self::Model(v)
    }
}
pub(super) fn runtime_evidence(
    data: &ModelApplicationData,
    bound: &ValidatedBoundCall,
    shape: &BindingShapeAdmission,
    budget: &ResourceBudget,
) -> Result<
    (
        Rows<analysis::native::NativeAssertionPremise>,
        analysis::policy::EvidenceStatus,
    ),
    RuntimeEvidenceError,
> {
    let b = &data.bindings;
    let mut rows = Rows::new(budget);
    let mut status = analysis::policy::EvidenceStatus::StructurallyObserved;
    let mut add = |reference, q| {
        append_native_evidence(data, reference, q, shape.context(), &mut rows, &mut status)
    };
    let target = need(&b.targets, shape.target())?;
    add(derivation::RowRef::of(target.id()), target.qualification)?;
    let signature = need(&b.signatures, bound.bound().signature())?;
    add(
        derivation::RowRef::of(signature.id()),
        signature.qualification,
    )?;
    if let Some(enumeration) = shape.enumeration() {
        let enumeration = need(&b.signature_enumerations, enumeration)?;
        add(
            derivation::RowRef::of(enumeration.id()),
            enumeration.qualification,
        )?;
        for member in b
            .signature_enumeration_members
            .iter()
            .filter(|m| m.enumeration == enumeration.id())
        {
            let signature = need(&b.signatures, member.signature)?;
            add(
                derivation::RowRef::of(signature.id()),
                signature.qualification,
            )?;
        }
    }
    let symbol = need(&b.symbols, signature.symbol)?;
    let traits = one(b.traits.iter().filter(|t| t.symbol == symbol.id()))?;
    add(derivation::RowRef::of(traits.id()), traits.qualification)?;
    let mut current = symbol;
    let mut remaining = b.symbols.len() + 1;
    loop {
        if remaining == 0 {
            return Err(ObligationKind::MissingEvidence.into());
        }
        remaining -= 1;
        let observation = one(b.symbol_observations.iter().filter(|o| {
            o.symbol == current.id()
                && b.qualifications
                    .get(o.qualification)
                    .is_some_and(|q| q.context == shape.context())
        }))?;
        add(
            derivation::RowRef::of(observation.id()),
            observation.qualification,
        )?;
        match observation.parent {
            Some(parent) => current = need(&b.symbols, parent)?,
            None => break,
        }
    }
    Ok((rows, status))
}
/// Replays an exact early native row/support projection and accumulates its policy floor.
pub(super) fn append_native_evidence(
    data: &ModelApplicationData,
    reference: derivation::RowRef,
    q: Id<AssertionQualification>,
    context: Id<AnalysisContext>,
    rows: &mut Rows<analysis::native::NativeAssertionPremise>,
    status: &mut analysis::policy::EvidenceStatus,
) -> Result<(), RuntimeEvidenceError> {
    exact(&data.bindings, q, context)?;
    let mut found = false;
    for native in data.native.iter().filter(|n| n.qualification == q) {
        let premise = need(&data.premises, native.premise)?;
        if premise.assertion_and_support().0 != reference {
            continue;
        }
        if native.fidelity == attribution::Fidelity::DisplayOnly
            || native.status != analysis::policy::native_status(native.family, native.fidelity)
        {
            return Err(ObligationKind::MissingEvidence.into());
        }
        analysis::policy::behavioral_support(native.status, false)
            .map_err(|_| ObligationKind::MissingEvidence)?;
        rows.insert(premise.clone())?;
        *status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [*status, native.status],
        );
        found = true;
    }
    if !found {
        return Err(ObligationKind::MissingEvidence.into());
    }
    Ok(())
}
pub(super) fn runtime_identity(
    data: &ModelApplicationData,
    bound: &ValidatedBoundCall,
    shape: &BindingShapeAdmission,
    effective: Option<&EffectiveInvocationAdmission>,
) -> Result<(), ObligationKind> {
    let bindings = &data.bindings;
    let signature = need(&bindings.signatures, bound.bound().signature())?;
    let symbol = need(&bindings.symbols, signature.symbol)?;
    let target = need(&bindings.targets, shape.target())?;
    if !shape.admits(bound) || symbol.context != shape.context() {
        return Err(ObligationKind::IncompatibleContexts);
    }
    let callable = match need(&bindings.refs, shape.callee())? {
        EntityRef::Callable { callable } => need(&bindings.callables, *callable)?,
        _ => return Err(ObligationKind::CallTransfer),
    };
    match callable {
        CallableEntity::Source { .. } => {
            if effective.is_none_or(|e| {
                !e.admits(bound)
                    || e.callee() != shape.callee()
                    || e.input() != shape.input()
                    || e.phase() != shape.phase()
            }) {
                return Err(ObligationKind::MissingEvidence);
            }
        }
        CallableEntity::External { symbol: external } => {
            if *external != symbol.id() {
                return Err(ObligationKind::MissingEvidence);
            }
            let traits = one(bindings.traits.iter().filter(|t| t.symbol == symbol.id()))?;
            exact(bindings, traits.qualification, shape.context())?;
            if traits.origin != FunctionOrigin::DefStatement {
                return Err(ObligationKind::OutsideProviderModel);
            }
            if !bindings.trait_supports.iter().any(|s| {
                s.assertion == traits.id()
                    && bindings
                        .runs
                        .get(s.run)
                        .is_some_and(|r| r.input == shape.input() && r.context == shape.context())
            }) {
                return Err(ObligationKind::MissingEvidence);
            }
            let attempt_variant = one(bindings
                .callable_variants
                .iter()
                .filter(|v| v.signature == signature.id()))?;
            let adjustment = if traits.property_getter || traits.property_setter {
                SignatureAdjustment::PropertyAccess
            } else if traits.classmethod {
                SignatureAdjustment::BindClassReceiver
            } else if traits.defining_class.is_some() && !traits.staticmethod {
                SignatureAdjustment::BindInstanceReceiver
            } else {
                SignatureAdjustment::None
            };
            if attempt_variant.adjustment != SignatureAdjustment::Unknown
                && attempt_variant.adjustment != adjustment
            {
                return Err(ObligationKind::CallTransfer);
            }
            let receiver = need(&bindings.receivers, target.receiver)?;
            let receiver_matches = match adjustment {
                SignatureAdjustment::None => {
                    matches!(receiver, Receiver::None)
                        && matches!(target.passing, None | Some(ReceiverPassing::NotPassed))
                }
                SignatureAdjustment::BindInstanceReceiver => matches!(
                    (receiver, target.passing),
                    (Receiver::Bound { .. }, Some(ReceiverPassing::Object))
                        | (Receiver::None, Some(ReceiverPassing::NotPassed))
                ),
                SignatureAdjustment::BindClassReceiver => {
                    matches!(
                        (receiver, target.passing),
                        (Receiver::Bound { .. }, Some(ReceiverPassing::Class))
                    ) || matches!(receiver, Receiver::Bound { .. })
                        && target.class_method == Some(true)
                }
                SignatureAdjustment::PropertyAccess => {
                    matches!(
                        (receiver, target.passing),
                        (Receiver::Bound { .. }, Some(ReceiverPassing::Object))
                    ) && matches!(
                        target.phase,
                        CallPhase::PropertyGet | CallPhase::PropertySet
                    )
                }
                SignatureAdjustment::Unknown => false,
            };
            if !receiver_matches
                || target
                    .static_method
                    .is_some_and(|v| v != traits.staticmethod)
                || target.class_method.is_some_and(|v| v != traits.classmethod)
            {
                return Err(ObligationKind::CallTransfer);
            }
        }
        CallableEntity::Synthetic { .. } => return Err(ObligationKind::OutsideProviderModel),
    }
    Ok(())
}
pub(super) fn exact(
    data: &BindingData,
    id: Id<AssertionQualification>,
    context: Id<AnalysisContext>,
) -> Result<(), ObligationKind> {
    let q = need(&data.qualifications, id)?;
    if q.context != context {
        return Err(ObligationKind::IncompatibleContexts);
    }
    if q.modality != Modality::Definite
        || q.approximation != Approximation::Exact
        || q.condition != conditions::Diagram::always().id()
    {
        return Err(ObligationKind::Approximation);
    }
    Ok(())
}
pub(super) fn formal(
    data: &BindingData,
    signature: &Signature,
    name: Option<&str>,
) -> Result<Option<Id<SignatureParameter>>, ObligationKind> {
    let Some(name) = name else { return Ok(None) };
    Ok(Some(
        one(data.parameters.iter().filter(|p| {
            p.signature == signature.id()
                && data
                    .shapes
                    .get(p.shape)
                    .is_some_and(|s| s.name.as_ref().is_some_and(|n| n.as_str() == name))
        }))?
        .id(),
    ))
}
pub(super) fn matches_target(
    data: &ModelApplicationData,
    target: &Target,
    symbol: &ProviderSymbol,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
) -> Result<(), ObligationKind> {
    let bindings = &data.bindings;
    let module = need(&bindings.provider_modules, symbol.module)?;
    let (expected_module, expected_callable) = match target {
        Target::Stdlib {
            module, callable, ..
        }
        | Target::Dependency {
            module, callable, ..
        }
        | Target::Release { module, callable } => (module, callable),
    };
    let name = match module {
        ProviderModule::Acquired { module } => {
            need(&bindings.modules, *module)?.qualified_name.as_str()
        }
        ProviderModule::Bundled { name, .. } => name.as_str(),
        _ => return Err(ObligationKind::OutsideProviderModel),
    };
    if name != expected_module {
        return Err(ObligationKind::MissingEvidence);
    }
    // Native lexical parents, not native-key string conventions, own member qualification.
    let mut parts = expected_callable.rsplit('.');
    let mut current = symbol;
    let mut remaining = bindings.symbols.len() + 1;
    loop {
        if remaining == 0 {
            return Err(ObligationKind::MissingEvidence);
        }
        remaining -= 1;
        if parts.next() != Some(current.name.as_str()) {
            return Err(ObligationKind::MissingEvidence);
        }
        let observation = one(bindings.symbol_observations.iter().filter(|s| {
            s.symbol == current.id()
                && bindings
                    .qualifications
                    .get(s.qualification)
                    .is_some_and(|q| q.context == context)
        }))?;
        exact(bindings, observation.qualification, context)?;
        match observation.parent {
            Some(parent) => {
                current = need(&bindings.symbols, parent)?;
                if current.module != symbol.module || current.context != context {
                    return Err(ObligationKind::IncompatibleContexts);
                }
            }
            None => break,
        }
    }
    if parts.next().is_some() {
        return Err(ObligationKind::MissingEvidence);
    }
    match target {
        Target::Stdlib { python, .. } => {
            if need(&data.contexts, context)?.python_version != *python
                || !matches!(module,ProviderModule::Bundled{bundle:ModuleBundle::Typeshed,provider,..} if *provider==symbol.provider)
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
        }
        Target::Dependency {
            distribution,
            version,
            ..
        } => {
            let ProviderModule::Acquired { module } = module else {
                return Err(ObligationKind::OutsideProviderModel);
            };
            let source = need(&bindings.modules, *module)?.source;
            if need(&bindings.artifacts, source)?.input != input {
                return Err(ObligationKind::IncompatibleContexts);
            }
            if !data.ownership.iter().any(|o| {
                o.artifact == source
                    && data.verifications.get(o.distribution).is_some_and(|v| {
                        data.acquisitions
                            .get(v.acquisition)
                            .is_some_and(|a| a.input == input)
                            && data.releases.get(v.release).is_some_and(|r| {
                                r.version == *version
                                    && data
                                        .packages
                                        .get(r.package)
                                        .is_some_and(|p| p.name == *distribution)
                            })
                    })
            }) {
                return Err(ObligationKind::OutsideProviderModel);
            }
        }
        Target::Release { .. } => {
            let ProviderModule::Acquired { module } = module else {
                return Err(ObligationKind::OutsideProviderModel);
            };
            let source = need(&bindings.modules, *module)?.source;
            if need(&bindings.artifacts, source)?.input != input
                || !data.uses.iter().any(|u| {
                    u.input == input && u.artifact == source && u.role == SourceRole::Release
                })
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
        }
    }
    Ok(())
}
