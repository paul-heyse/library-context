//! Source contracts and effective invocation authority are separate attributed results.
use crate::domain::{
    attribution::AnalysisContext,
    calls::*,
    normalized::{entities::*, links::ReferenceEntityAssessment},
    symbols::FunctionTraitObservation,
    syntax::*,
    types::FunctionBodyObservation,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Knowledge {
    Known = 0,
    Unknown = 1,
    Conflicting = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DescriptorKind {
    Function = 0,
    InstanceMethod = 1,
    StaticMethod = 2,
    ClassMethod = 3,
    Property = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CallableReason {
    EvidenceAgreement = 0,
    MissingSignature = 1,
    IncompleteSignature = 2,
    MissingTraits = 3,
    IncompleteSyntax = 4,
    UnsupportedDecorator = 5,
    ShadowedOrUnresolved = 6,
    ConflictingEvidence = 7,
    NoSourceBody = 8,
    BodyExcluded = 9,
    MissingBodyEvidence = 10,
    QualifiedUncertainty = 11,
    UnsupportedNativeOrigin = 12,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SignatureAdjustment {
    None = 0,
    BindInstanceReceiver = 1,
    BindClassReceiver = 2,
    PropertyAccess = 3,
    Unknown = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DefaultSlot {
    Required = 0,
    DefinitionTime = 1,
    Collector = 2,
    NativeUnknown = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "effective_callable_assessments", validate = validate_assessment, invariant_refs = super::callable_normalization::invariants_refs)]
pub struct EffectiveCallableAssessment {
    #[model(key)]
    pub callable: Id<CallableEntity>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub decorators: ContentHash,
    #[model(key)]
    pub policy: ContentHash,
    pub identity: Knowledge,
    pub identity_reason: CallableReason,
    pub signatures: Knowledge,
    pub signature_reason: CallableReason,
    pub descriptor: Knowledge,
    pub descriptor_kind: Option<DescriptorKind>,
    pub descriptor_reason: CallableReason,
    pub body: Knowledge,
    pub body_admitted: bool,
    pub body_reason: CallableReason,
    pub asynchronous: Option<bool>,
    pub generator: Option<bool>,
}
fn validate_assessment(row: &EffectiveCallableAssessment) -> Result<(), ModelError> {
    if (row.descriptor == Knowledge::Known) != row.descriptor_kind.is_some()
        || (row.identity == Knowledge::Known
            && (row.descriptor != Knowledge::Known || row.signatures != Knowledge::Known))
        || (row.body_admitted && (row.body != Knowledge::Known || row.identity != Knowledge::Known))
    {
        return Err(ModelError::Invalid(
            "effective callable components require their own established premises".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "effective_decorator_members")]
pub struct EffectiveDecoratorMember {
    #[model(key)]
    pub assessment: Id<EffectiveCallableAssessment>,
    #[model(key)]
    pub observation: Id<DeclarationDecorator>,
    pub source_ordinal: i64,
    pub application_ordinal: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "effective_callable_premises")]
pub enum EffectiveCallablePremise {
    #[model(code = 0)]
    Traits {
        observation: Id<FunctionTraitObservation>,
    },
    #[model(code = 1)]
    Body {
        observation: Id<FunctionBodyObservation>,
    },
    #[model(code = 2)]
    Signature { signature: Id<Signature> },
    #[model(code = 3)]
    Lexical {
        assessment: Id<ReferenceEntityAssessment>,
    },
    #[model(code = 4)]
    Syntax {
        declaration: Id<DeclarationObservation>,
    },
    #[model(code = 5)]
    Coverage {
        coverage: Id<attribution::ProviderCoverage>,
    },
    #[model(code = 6)]
    Resolution {
        resolution: Id<SymbolEntityResolution>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "effective_callable_evidence")]
pub struct EffectiveCallableEvidence {
    #[model(key)]
    pub assessment: Id<EffectiveCallableAssessment>,
    #[model(key)]
    pub premise: Id<EffectiveCallablePremise>,
}
/// Total over raw signatures, including those whose normalized correspondence is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_variants")]
pub struct SignatureVariant {
    pub role: SignatureRole,
    pub native: Option<Id<crate::domain::types::NativeSignatureObservation>>,
    #[model(key)]
    pub signature: Id<Signature>,
    pub context: Id<AnalysisContext>,
    pub resolution: Id<SymbolEntityResolution>,
    pub callable: Option<Id<CallableEntity>>,
    pub assessment: Option<Id<EffectiveCallableAssessment>>,
    pub adjustment: SignatureAdjustment,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_slots")]
pub struct SignatureSlot {
    #[model(key)]
    pub parameter: Id<SignatureParameter>,
    pub variant: Id<SignatureVariant>,
    pub ordinal: i64,
    pub default: DefaultSlot,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_slot_entities")]
pub struct SignatureSlotEntity {
    #[model(key)]
    pub slot: Id<SignatureSlot>,
    #[model(key)]
    pub link: Id<ParameterEntityLink>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_slot_types")]
pub struct SignatureSlotType {
    #[model(key)]
    pub slot: Id<SignatureSlot>,
    #[model(key)]
    pub observation: Id<crate::domain::types::SignatureTypeObservation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_return_types")]
pub struct SignatureReturnType {
    #[model(key)]
    pub variant: Id<SignatureVariant>,
    #[model(key)]
    pub observation: Id<crate::domain::types::SignatureTypeObservation>,
}
pub fn relations() -> Vec<Relation> {
    macro_rules! declare { ($($field:ident: $ty:ty,)*) => { vec![$(Relation::of::<$ty>()),*] }; }
    crate::normalized_callable_outputs!(declare)
}

