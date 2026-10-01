//! Explicit normalized premises for argument-shape controls. These external entities deliberately
//! grant source inspection only; production applicability and body authority have separate tests.
use lctx_model::domain::{
    assertion::AssertionQualification,
    attribution::ObligationKind,
    calls::*,
    input::InputRevision,
    normalized::{Rows, callables::*, entities::*, signature_applicability::*},
    source::*,
    *,
};
use std::collections::BTreeMap;
pub struct InspectionCase<'a> {
    pub input: Id<InputRevision>,
    pub target: &'a CallTarget,
    pub qualification: &'a AssertionQualification,
    pub signature_qualification: &'a AssertionQualification,
    pub destination: &'a CallDestination,
    pub channel: &'a CallChannel,
    pub receiver: &'a Receiver,
    pub signature: &'a Signature,
    pub parameters: &'a [SignatureParameter],
    pub shapes: &'a BTreeMap<Id<ParameterShape>, ParameterShape>,
    pub call: &'a CallSyntax,
    pub arguments: &'a [CallArgument],
}
pub fn bind_inspection(c: InspectionCase<'_>) -> Result<BoundCall, BindingFailure> {
    let budget = resources::ResourceBudget::fixed(resources::DEFAULT_MEMORY_BYTES).unwrap();
    let mut scopes = Rows::new(&budget);
    scopes
        .insert(CoverageScope::Input { input: c.input })
        .unwrap();
    let artifacts = Rows::new(&budget);
    let modules = Rows::new(&budget);
    let symbol = c
        .destination
        .symbol()
        .ok_or(ObligationKind::MissingEvidence)?;
    let callable = CallableEntity::External { symbol };
    let entity = EntityRef::Callable {
        callable: callable.id(),
    };
    let resolution = |symbol| SymbolEntityResolution {
        symbol,
        context: c.qualification.context,
        policy: lctx_model::domain::normalized::policy_revision(),
        status: ResolutionStatus::Resolved,
        entity: Some(
            EntityRef::Callable {
                callable: CallableEntity::External { symbol }.id(),
            }
            .id(),
        ),
        reason: EntityReason::ProviderExternal,
    };
    let target_resolution = resolution(symbol);
    let signature_resolution = resolution(c.signature.symbol);
    let variant = SignatureVariant {
        signature: c.signature.id(),
        context: c.signature_qualification.context,
        resolution: signature_resolution.id(),
        callable: Some(callable.id()),
        assessment: None,
        adjustment: SignatureAdjustment::Unknown,
    };
    let call_qualification = [c.qualification, c.signature_qualification]
        .into_iter()
        .find(|q| q.id() == c.call.qualification)
        .ok_or(ObligationKind::MissingEvidence)?;
    let application = establish(Application {
        target: c.target,
        qualification: c.qualification,
        destination: c.destination,
        channel: c.channel,
        receiver: c.receiver,
        receiver_proof: None,
        dispatch_proof: None,
        signature: c.signature,
        signature_qualification: c.signature_qualification,
        call: c.call,
        call_qualification,
        target_resolution: &target_resolution,
        signature_resolution: &signature_resolution,
        entity: &entity,
        callable: &callable,
        variant: &variant,
        effective: None,
        scopes: ScopeCatalog {
            scopes: &scopes,
            artifacts: &artifacts,
            modules: &modules,
        },
    })?;
    bind(BindingInput {
        application: &application,
        parameters: c.parameters,
        shapes: c.shapes,
        arguments: c.arguments,
    })
}
