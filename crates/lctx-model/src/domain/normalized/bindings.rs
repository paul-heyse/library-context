//! Whole argument binding and complete effective-variant admission are different assessments.
use crate::domain::{
    attribution::ObligationKind,
    calls::*,
    normalized::{callables::*, entities::*, events::*, signature_applicability::*},
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingOutcome {
    Bound = 0,
    ProvenIncompatible = 1,
    Undetermined = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingReason {
    Bound = 0,
    MissingSignature = 1,
    MissingSyntax = 2,
    UnprovedApplicability = 3,
    ArgumentMismatch = 4,
    UnsupportedShape = 5,
    ImplicitEvent = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingSetReason {
    Unique = 0,
    NoBoundVariant = 1,
    MultipleBoundVariants = 2,
    UndeterminedVariant = 3,
    IncompleteCoverage = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "call_binding_attempts", invariant_refs = super::binding_normalization::invariants_refs)]
pub struct CallBindingAttempt {
    #[model(key)]
    pub alternative: Id<NormalizedCallAlternative>,
    #[model(key)]
    pub variant: Option<Id<SignatureVariant>>,
    #[model(key)]
    pub syntax: Option<Id<CallSyntax>>,
    #[model(key)]
    pub policy: ContentHash,
    pub event: Id<NormalizedCallEvent>,
    pub signature: Option<Id<Signature>>,
    pub arguments: Option<ContentHash>,
    pub receiver: Id<Receiver>,
    pub receiver_assessment: Option<Id<super::receiver::ReceiverAssessment>>,
    pub dispatch_member: Option<Id<super::dispatch::DispatchMember>>,
    pub effective: Option<Id<EffectiveCallableAssessment>>,
    pub adjustment: SignatureAdjustment,
    pub authority: BindingAuthority,
    pub authority_reason: AuthorityReason,
    /// The source-shape result. Only effective authority promotes it into set classification.
    pub outcome: BindingOutcome,
    pub reason: BindingReason,
    pub refusal: Option<ObligationKind>,
    pub bindings: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "call_bindings")]
pub struct CallBinding {
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
    #[model(key)]
    pub ordinal: i64,
    pub slot: Id<SignatureSlot>,
    pub source: Id<BindingSource>,
    pub kind: BindingKind,
    pub projection: Id<BindingProjection>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "binding_set_assessments")]
pub struct BindingSetAssessment {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub entity: Option<Id<EntityRef>>,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub channel: Id<CallChannel>,
    #[model(key)]
    pub policy: ContentHash,
    pub members: ContentHash,
    pub bound: i64,
    pub incompatible: i64,
    pub undetermined: i64,
    pub coverage_complete: bool,
    pub unique: bool,
    pub reason: BindingSetReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "binding_variant_assessments")]
pub struct BindingVariantAssessment {
    #[model(key)]
    pub set: Id<BindingSetAssessment>,
    #[model(key)]
    pub variant: Option<Id<SignatureVariant>>,
    pub outcome: BindingOutcome,
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "binding_set_members")]
pub struct BindingSetMember {
    #[model(key)]
    pub variant: Id<BindingVariantAssessment>,
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "binding_set_coverage")]
pub struct BindingSetCoverage {
    #[model(key)]
    pub set: Id<BindingSetAssessment>,
    #[model(key)]
    pub coverage: Id<attribution::ProviderCoverage>,
}
pub fn relations() -> Vec<Relation> {
    macro_rules! declare { ($($field:ident: $ty:ty,)*) => { vec![$(Relation::of::<$ty>()),*] }; }
    crate::normalized_binding_outputs!(declare)
}
