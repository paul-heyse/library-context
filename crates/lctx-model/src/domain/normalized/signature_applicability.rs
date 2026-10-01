//! The sole binder's normalized ownership boundary. Raw evidence is never relabeled to make
//! provider symbol IDs compare equal; supported entity correspondence supplies applicability.
use crate::DomainCode;
use crate::domain::{
    assertion::{Approximation, AssertionQualification},
    attribution::{AnalysisContext, Modality},
    calls::*,
    input::InputRevision,
    normalized::{Rows, callables::*, entities::*},
    source::{CoverageScope, Module, SourceArtifact},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingAuthority {
    SourceInspection = 0,
    EffectiveInvocation = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AuthorityReason {
    Established = 0,
    EffectiveUnknown = 1,
    EffectiveConflicting = 2,
    SignatureUnknown = 3,
    DescriptorUnknown = 4,
    ReceiverUnknown = 5,
    ReceiverDisagreement = 6,
    ObjectToClassUnsupported = 7,
    QualifiedUncertainty = 8,
    AnnotationSyntax = 9,
}

pub struct ScopeCatalog<'a> {
    pub scopes: &'a Rows<CoverageScope>,
    pub artifacts: &'a Rows<SourceArtifact>,
    pub modules: &'a Rows<Module>,
}
impl ScopeCatalog<'_> {
    pub fn input(&self, scope: Id<CoverageScope>) -> Option<Id<InputRevision>> {
        match self.scopes.get(scope)? {
            CoverageScope::Input { input } => Some(*input),
            CoverageScope::Artifact { artifact } => Some(self.artifacts.get(*artifact)?.input),
            CoverageScope::Module { module } => {
                Some(self.artifacts.get(self.modules.get(*module)?.source)?.input)
            }
            CoverageScope::Release { .. } => None,
        }
    }
}
/// Exact original records are borrowed through binding. Context and input equality are checked
/// against nominal scope records, including caller/callee artifact scopes in the same input.
pub struct Application<'a> {
    pub target: &'a CallTarget,
    pub qualification: &'a AssertionQualification,
    pub destination: &'a CallDestination,
    pub channel: &'a CallChannel,
    pub receiver: &'a Receiver,
    pub receiver_proof: Option<&'a super::receiver::ApplicableReceiver>,
    pub signature: &'a Signature,
    pub signature_qualification: &'a AssertionQualification,
    pub call: &'a CallSyntax,
    pub call_qualification: &'a AssertionQualification,
    pub target_resolution: &'a SymbolEntityResolution,
    pub signature_resolution: &'a SymbolEntityResolution,
    pub entity: &'a EntityRef,
    pub callable: &'a CallableEntity,
    pub variant: &'a SignatureVariant,
    pub effective: Option<&'a EffectiveCallableAssessment>,
    pub scopes: ScopeCatalog<'a>,
}
pub struct ApplicableSignature<'a> {
    raw: RawBinding<'a>,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
    callable: Id<CallableEntity>,
    variant: Id<SignatureVariant>,
    authority: BindingAuthority,
    reason: AuthorityReason,
    effective: Option<Id<EffectiveCallableAssessment>>,
}
pub(crate) struct RawBinding<'a> {
    pub class_of: Option<Id<source::Occurrence>>,
    pub target: &'a CallTarget,
    pub channel: &'a CallChannel,
    pub receiver: &'a Receiver,
    pub signature: &'a Signature,
    pub call: &'a CallSyntax,
}
impl ApplicableSignature<'_> {
    pub fn input(&self) -> Id<InputRevision> {
        self.input
    }
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn callable(&self) -> Id<CallableEntity> {
        self.callable
    }
    pub fn variant(&self) -> Id<SignatureVariant> {
        self.variant
    }
    pub fn authority(&self) -> BindingAuthority {
        self.authority
    }
    pub fn reason(&self) -> AuthorityReason {
        self.reason
    }
    pub fn effective(&self) -> Option<Id<EffectiveCallableAssessment>> {
        self.effective
    }
    pub(crate) fn raw(&self) -> &RawBinding<'_> {
        &self.raw
    }
}
pub fn establish(
    application: Application<'_>,
) -> Result<ApplicableSignature<'_>, attribution::ObligationKind> {
    use attribution::ObligationKind::MissingEvidence;
    let a = application;
    let CallDestination::Resolved { symbol } = a.destination else {
        return Err(attribution::ObligationKind::CallTransfer);
    };
    let EntityRef::Callable { callable } = a.entity else {
        return Err(MissingEvidence);
    };
    let context = a.qualification.context;
    let input = a
        .scopes
        .input(a.qualification.scope)
        .ok_or(MissingEvidence)?;
    if a.target.qualification != a.qualification.id()
        || a.target.destination != a.destination.id()
        || a.target.channel != a.channel.id()
        || a.target.receiver != a.receiver.id()
        || a.signature.qualification != a.signature_qualification.id()
        || a.signature.scope != a.signature_qualification.scope
        || a.call.qualification != a.call_qualification.id()
        || a.call.site != a.target.site
        || a.signature_qualification.context != context
        || a.call_qualification.context != context
        || a.scopes.input(a.signature.scope) != Some(input)
        || a.scopes.input(a.call_qualification.scope) != Some(input)
        || a.target_resolution.symbol != *symbol
        || a.signature_resolution.symbol != a.signature.symbol
        || *callable != a.callable.id()
        || a.variant.signature != a.signature.id()
        || a.variant.context != context
        || a.variant.resolution != a.signature_resolution.id()
        || a.variant.callable != Some(*callable)
        || [a.target_resolution, a.signature_resolution]
            .iter()
            .any(|r| {
                r.context != context
                    || r.status != ResolutionStatus::Resolved
                    || r.entity != Some(a.entity.id())
                    || r.policy != super::policy_revision()
            })
    {
        return Err(MissingEvidence);
    }
    if let Some(effective) = a.effective {
        if Some(effective.id()) != a.variant.assessment
            || effective.callable != *callable
            || effective.context != context
            || effective.policy != super::policy_revision()
        {
            return Err(MissingEvidence);
        }
    } else if a.variant.assessment.is_some() {
        return Err(MissingEvidence);
    }
    if *symbol != a.signature.symbol
        && (!matches!(a.callable, CallableEntity::Source { .. })
            || [a.target_resolution, a.signature_resolution]
                .iter()
                .any(|r| r.reason != EntityReason::DeclarationAgreement))
    {
        return Err(MissingEvidence);
    }
    if a.receiver_proof.is_some_and(|proof| !proof.matches(a.target,a.call,a.variant,input,context)) { return Err(MissingEvidence); }
    let reason = authority(&a);
    Ok(ApplicableSignature {
        raw: RawBinding {
            class_of: if a.variant.adjustment == SignatureAdjustment::BindClassReceiver && a.effective.is_some_and(|e|e.identity==Knowledge::Known && e.descriptor==Knowledge::Known && e.descriptor_kind==Some(DescriptorKind::ClassMethod)) && a.target.class_method==Some(true) && a.target.static_method!=Some(true) && a.target.passing==Some(ReceiverPassing::Object) { a.receiver_proof.map(|p|p.actual()) } else {None},
            target: a.target,
            channel: a.channel,
            receiver: a.receiver,
            signature: a.signature,
            call: a.call,
        },
        input,
        context,
        callable: *callable,
        variant: a.variant.id(),
        authority: if reason == AuthorityReason::Established {
            BindingAuthority::EffectiveInvocation
        } else {
            BindingAuthority::SourceInspection
        },
        reason,
        effective: a.effective.map(Record::id),
    })
}
fn authority(a: &Application<'_>) -> AuthorityReason {
    if a.call.in_annotation {
        return AuthorityReason::AnnotationSyntax;
    }
    let Some(effective) = a.effective else {
        return AuthorityReason::EffectiveUnknown;
    };
    if effective.identity == Knowledge::Conflicting {
        return AuthorityReason::EffectiveConflicting;
    }
    if effective.identity != Knowledge::Known {
        return AuthorityReason::EffectiveUnknown;
    }
    if effective.signatures != Knowledge::Known || a.signature.form != SignatureForm::List {
        return AuthorityReason::SignatureUnknown;
    }
    if [
        a.qualification,
        a.signature_qualification,
        a.call_qualification,
    ]
    .iter()
    .any(|q| q.modality != Modality::Definite || q.approximation != Approximation::Exact)
    {
        return AuthorityReason::QualifiedUncertainty;
    }
    if matches!(a.receiver, Receiver::Unknown { .. }) && a.receiver_proof.is_none() {
        return AuthorityReason::ReceiverUnknown;
    }
    let Some(kind) = effective
        .descriptor_kind
        .filter(|_| effective.descriptor == Knowledge::Known)
    else {
        return AuthorityReason::DescriptorUnknown;
    };
    let expected = match kind {
        DescriptorKind::Function | DescriptorKind::StaticMethod => SignatureAdjustment::None,
        DescriptorKind::InstanceMethod => SignatureAdjustment::BindInstanceReceiver,
        DescriptorKind::ClassMethod => SignatureAdjustment::BindClassReceiver,
        DescriptorKind::Property => SignatureAdjustment::PropertyAccess,
    };
    if a.variant.adjustment != expected {
        return AuthorityReason::ReceiverDisagreement;
    }
    if kind == DescriptorKind::ClassMethod && a.target.passing == Some(ReceiverPassing::Object) && a.receiver_proof.is_none() {
        return AuthorityReason::ObjectToClassUnsupported;
    }
    let flags = match kind {
        DescriptorKind::Function | DescriptorKind::InstanceMethod => {
            a.target.static_method != Some(true) && a.target.class_method != Some(true)
        }
        DescriptorKind::StaticMethod => {
            a.target.static_method != Some(false) && a.target.class_method != Some(true)
        }
        DescriptorKind::ClassMethod => {
            a.target.class_method != Some(false) && a.target.static_method != Some(true)
        }
        DescriptorKind::Property => true,
    };
    let receiver = match (kind, a.receiver, a.target.passing) {
        (
            DescriptorKind::Function | DescriptorKind::StaticMethod,
            Receiver::None,
            Some(ReceiverPassing::NotPassed) | None,
        ) => true,
        (DescriptorKind::InstanceMethod, Receiver::None, Some(ReceiverPassing::NotPassed)) => true,
        (DescriptorKind::InstanceMethod, Receiver::Bound { .. }, Some(ReceiverPassing::Object)) => {
            true
        }
        (DescriptorKind::ClassMethod, Receiver::Unknown { .. }, Some(ReceiverPassing::Object)) => a.receiver_proof.is_some() && a.target.class_method == Some(true),
        (DescriptorKind::ClassMethod, Receiver::Bound { .. }, Some(ReceiverPassing::Class)) => true,
        (DescriptorKind::ClassMethod, Receiver::Bound { .. }, None) => {
            a.target.class_method == Some(true)
        }
        (DescriptorKind::Property, Receiver::Bound { .. }, Some(ReceiverPassing::Object)) => {
            matches!(
                a.target.phase,
                CallPhase::PropertyGet | CallPhase::PropertySet
            )
        }
        _ => false,
    };
    if flags && receiver {
        AuthorityReason::Established
    } else {
        AuthorityReason::ReceiverDisagreement
    }
}
