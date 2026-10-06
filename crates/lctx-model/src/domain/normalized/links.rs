//! Total attributed relationship outcomes retain their original observation and alternatives.
use crate::domain::{normalized::entities::*, *};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum LinkReason {
    ExplicitIdentity = 0,
    MissingCorrespondence = 1,
    MissingResolution = 2,
    ConflictingCandidates = 3,
    OutsideCapturedScope = 4,
    UnsupportedNativeOrigin = 5,
    MissingOperand = 6,
    NoTypeObservation = 7,
    IncompleteInput = 8,
    NotRequested = 9,
    Unavailable = 10,
    UntracedName = 11,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "reference_entity_targets")]
pub enum ReferenceEntityTarget {
    #[model(code = 0)]
    Binding {
        event: Id<lexical::BindingEvent>,
        entity: Id<EntityRef>,
    },
    #[model(code = 1)]
    Builtin { target: Id<lexical::LexicalTarget> },
    #[model(code = 2)]
    Unresolved { target: Id<lexical::LexicalTarget> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "reference_entity_assessments", invariant_refs = super::relation_normalization::invariants_refs)]
pub struct ReferenceEntityAssessment {
    #[model(key)]
    pub reference: Id<lexical::ReferenceObservation>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "reference_entity_candidates", projection_roles = crate::domain::projection::reference_roles)]
pub struct ReferenceEntityCandidate {
    #[model(key)]
    pub assessment: Id<ReferenceEntityAssessment>,
    #[model(key)]
    pub resolution: Id<lexical::LexicalResolution>,
    #[model(key)]
    pub target: Id<ReferenceEntityTarget>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "import_module_assessments")]
pub struct ImportModuleAssessment {
    #[model(key)]
    pub observation: Id<syntax::ImportAliasObservation>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "import_module_candidates", projection_roles = crate::domain::projection::import_roles)]
pub struct ImportModuleCandidate {
    #[model(key)]
    pub assessment: Id<ImportModuleAssessment>,
    #[model(key)]
    pub observation: Id<symbols::ModuleResolutionObservation>,
    pub module: Id<calls::ProviderModule>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "ancestry_entity_assessments")]
pub struct AncestryEntityAssessment {
    #[model(key)]
    pub observation: Id<symbols::ClassAncestryObservation>,
    pub class: Id<SymbolEntityResolution>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "ancestry_entity_members")]
pub struct AncestryEntityMember {
    #[model(key)]
    pub assessment: Id<AncestryEntityAssessment>,
    #[model(key)]
    pub member: Id<symbols::SymbolSequenceMember>,
    pub resolution: Id<SymbolEntityResolution>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "mention_entity_assessments")]
pub struct MentionEntityAssessment {
    #[model(key)]
    pub observation: Id<documents::DocumentMentionObservation>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "mention_entity_candidates")]
pub struct MentionEntityCandidate {
    #[model(key)]
    pub assessment: Id<MentionEntityAssessment>,
    #[model(key)]
    pub exposure: Id<PublicExposure>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "type_entity_links")]
pub struct TypeEntityLink {
    #[model(key)]
    pub term: Id<types::TypeTerm>,
    pub resolution: Option<Id<SymbolEntityResolution>>,
    pub module: Option<Id<calls::ProviderModule>>,
    pub entity: Option<Id<EntityRef>>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "type_binder_assessments")]
pub struct TypeBinderAssessment {
    #[model(key)]
    pub variable: Id<types::TypeVariable>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "type_binder_candidates")]
pub struct TypeBinderCandidate {
    #[model(key)]
    pub assessment: Id<TypeBinderAssessment>,
    #[model(key)]
    pub declaration: Id<source::Occurrence>,
    #[model(key)]
    pub premise: Id<TypeBinderPremise>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "place_entity_links")]
pub struct PlaceEntityLink {
    #[model(key)]
    pub place: Id<value::Place>,
    pub entity: Option<Id<EntityRef>>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "test_operand_type_assessments")]
pub struct TestOperandTypeAssessment {
    #[model(key)]
    pub leaf: Id<flow::FlowTestLeafObservation>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "test_operand_type_links")]
pub struct TestOperandTypeLink {
    #[model(key)]
    pub assessment: Id<TestOperandTypeAssessment>,
    #[model(key)]
    pub observation: Id<types::TypeObservation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "test_operand_coverage")]
pub struct TestOperandCoverage {
    #[model(key)]
    pub assessment: Id<TestOperandTypeAssessment>,
    #[model(key)]
    pub coverage: Id<attribution::ProviderCoverage>,
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<MentionSymbolCandidate>(),
        Relation::of::<TypeBinderPremise>(),
        Relation::of::<ReferenceEntityTarget>(),
        Relation::of::<ReferenceEntityAssessment>(),
        Relation::of::<ReferenceEntityCandidate>(),
        Relation::of::<ReferenceBindingCharacterization>(),
        Relation::of::<DeclarationNativeCharacterization>(),
        Relation::of::<ImportModuleAssessment>(),
        Relation::of::<ImportModuleCandidate>(),
        Relation::of::<AncestryEntityAssessment>(),
        Relation::of::<AncestryEntityMember>(),
        Relation::of::<MentionEntityAssessment>(),
        Relation::of::<MentionEntityCandidate>(),
        Relation::of::<TypeEntityLink>(),
        Relation::of::<TypeBinderAssessment>(),
        Relation::of::<TypeBinderCandidate>(),
        Relation::of::<PlaceEntityLink>(),
        Relation::of::<TestOperandTypeAssessment>(),
        Relation::of::<TestOperandTypeLink>(),
        Relation::of::<TestOperandCoverage>(),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "type_binder_premises")]
pub enum TypeBinderPremise {
    #[model(code = 0)]
    Declaration {
        observation: Id<syntax::DeclarationObservation>,
    },
    #[model(code = 1)]
    Binding {
        observation: Id<lexical::BindingObservation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "mention_symbol_candidates")]
pub struct MentionSymbolCandidate {
    #[model(key)]
    pub assessment: Id<MentionEntityAssessment>,
    #[model(key)]
    pub observation: Id<symbols::SymbolObservation>,
    #[model(key)]
    pub resolution: Id<SymbolEntityResolution>,
}

/// Located native characterization accompanies a source association; it grants no capture or body authority.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "reference_binding_characterizations")]
pub struct ReferenceBindingCharacterization {
    #[model(key)]
    pub reference: Id<lexical::ReferenceObservation>,
    #[model(key)]
    pub native_context: Id<ruff::RuffContextObservation>,
    #[model(key)]
    pub binding: Id<ruff::RuffBindingObservation>,
    #[model(key)]
    pub context_support: Option<Id<ruff::RuffContextSupport>>,
    #[model(key)]
    pub support: Option<Id<ruff::RuffBindingSupport>>,
    #[model(key)]
    pub candidate: Option<Id<ReferenceEntityCandidate>>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "declaration_native_characterizations")]
pub struct DeclarationNativeCharacterization {
    #[model(key)]
    pub declaration: Id<syntax::DeclarationObservation>,
    #[model(key)]
    pub native_definition: Id<ruff::RuffDefinitionObservation>,
    #[model(key)]
    pub support: Option<Id<ruff::RuffDefinitionSupport>>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
