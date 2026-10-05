//! Nominal identity follows supported source anchors, never names or signature similarity.
use crate::domain::{
    attribution::AnalysisContext,
    calls::{ProviderModule, ProviderSymbol, Signature, SignatureParameter, SymbolKind},
    declarations::{ParameterDeclaration, SymbolDeclaration, SymbolDeclarationSupport},
    source::{Module, Occurrence, SyntaxKind},
    symbols::{
        ClassTraitObservation, ExportEnumerationObservation, ExportOrigin,
        FunctionTraitObservation, PublicNameObservation,
    },
    types::{RecordFieldObservation, TypeTerm},
    value::Place,
    *,
};
use crate::{Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CallableKind {
    Function = 0,
    Lambda = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "callable_entities")]
pub enum CallableEntity {
    #[model(code = 0)]
    Source {
        declaration: Id<Occurrence>,
        kind: CallableKind,
    },
    #[model(code = 1)]
    Synthetic { symbol: Id<ProviderSymbol> },
    #[model(code = 2)]
    External { symbol: Id<ProviderSymbol> },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "class_entities")]
pub enum ClassEntity {
    #[model(code = 0)]
    Source { declaration: Id<Occurrence> },
    #[model(code = 1)]
    Synthetic { symbol: Id<ProviderSymbol> },
    #[model(code = 2)]
    External { symbol: Id<ProviderSymbol> },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "parameter_entities")]
pub enum ParameterEntity {
    #[model(code = 0)]
    Source { declaration: Id<Occurrence> },
    #[model(code = 1)]
    NativeSlot {
        callable: Id<CallableEntity>,
        signature: Id<Signature>,
        parameter: Id<SignatureParameter>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "field_entities")]
pub struct FieldEntity {
    #[model(key)]
    pub class: Id<ClassEntity>,
    #[model(key)]
    pub name: Utf8Text,
}
/// Finite heterogeneous endpoint vocabulary; module/class bodies retain their own kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "entity_refs")]
pub enum EntityRef {
    #[model(code = 0)]
    Module { module: Id<Module> },
    #[model(code = 1)]
    Callable { callable: Id<CallableEntity> },
    #[model(code = 2)]
    Class { class: Id<ClassEntity> },
    #[model(code = 3)]
    Parameter { parameter: Id<ParameterEntity> },
    #[model(code = 4)]
    Field { field: Id<FieldEntity> },
    #[model(code = 5)]
    Occurrence { occurrence: Id<Occurrence> },
    #[model(code = 6)]
    Type { term: Id<TypeTerm> },
    #[model(code = 7)]
    Place { place: Id<Place> },
}
/// Categories of the finite endpoint vocabulary, shared by projection policy and descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
#[model(inventory)]
pub enum EntityCategory {
    Module = 0, Callable = 1, Class = 2, Parameter = 3,
    Field = 4, Occurrence = 5, Type = 6, Place = 7,
}
impl EntityRef {
    pub fn category(&self) -> EntityCategory {
        match self {
            Self::Module {..} => EntityCategory::Module,
            Self::Callable {..} => EntityCategory::Callable,
            Self::Class {..} => EntityCategory::Class,
            Self::Parameter {..} => EntityCategory::Parameter,
            Self::Field {..} => EntityCategory::Field,
            Self::Occurrence {..} => EntityCategory::Occurrence,
            Self::Type {..} => EntityCategory::Type,
            Self::Place {..} => EntityCategory::Place,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ResolutionStatus {
    Resolved = 0,
    Ambiguous = 1,
    Unresolved = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum EntityReason {
    DeclarationAgreement = 0,
    ProviderSynthetic = 1,
    ProviderExternal = 2,
    AcquiredModule = 3,
    MissingDeclaration = 4,
    ConflictingDeclarations = 5,
    UnsupportedKind = 6,
    UntracedExposure = 7,
    MissingCorrespondence = 8,
    QualifiedUncertainty = 9,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "symbol_entity_resolutions", validate = validate_resolution, invariant_refs = super::entity_normalization::invariants_refs)]
pub struct SymbolEntityResolution {
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub policy: ContentHash,
    pub status: ResolutionStatus,
    pub entity: Option<Id<EntityRef>>,
    pub reason: EntityReason,
}
fn validate_resolution(row: &SymbolEntityResolution) -> Result<(), ModelError> {
    if (row.status == ResolutionStatus::Resolved) != row.entity.is_some() {
        return Err(ModelError::Invalid(
            "resolved symbol requires exactly one entity".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "symbol_entity_candidates")]
pub struct SymbolEntityCandidate {
    #[model(key)]
    pub resolution: Id<SymbolEntityResolution>,
    #[model(key)]
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "symbol_entity_premises")]
pub enum SymbolEntityPremise {
    #[model(code = 0)]
    Declaration {
        declaration: Id<SymbolDeclaration>,
        support: Id<SymbolDeclarationSupport>,
    },
    #[model(code = 1)]
    FunctionTraits {
        observation: Id<FunctionTraitObservation>,
    },
    #[model(code = 2)]
    ClassTraits {
        observation: Id<ClassTraitObservation>,
    },
    #[model(code = 3)]
    Module { module: Id<ProviderModule> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "symbol_entity_evidence")]
pub struct SymbolEntityEvidence {
    #[model(key)]
    pub candidate: Id<SymbolEntityCandidate>,
    #[model(key)]
    pub premise: Id<SymbolEntityPremise>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "occurrence_ownership", projection_roles = crate::domain::projection::containment_roles)]
pub struct OccurrenceOwnership {
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    pub owner: Id<Occurrence>,
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "parameter_entity_links")]
pub struct ParameterEntityLink {
    #[model(key)]
    pub parameter: Id<SignatureParameter>,
    #[model(key)]
    pub entity: Id<ParameterEntity>,
    #[model(key)]
    pub declaration: Option<Id<ParameterDeclaration>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "field_entity_links")]
pub struct FieldEntityLink {
    #[model(key)]
    pub field: Id<FieldEntity>,
    #[model(key)]
    pub observation: Id<RecordFieldObservation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "field_declaration_links")]
pub struct FieldDeclarationLink {
    #[model(key)]
    pub field: Id<FieldEntity>,
    #[model(key)]
    pub declaration: Id<crate::domain::syntax::ClassFieldSyntaxObservation>,
    #[model(key)]
    pub binding: Id<crate::domain::lexical::BindingEvent>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PublicPathKnowledge {
    Known = 0,
    Candidate = 1,
    Unknown = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "public_enumeration_assessments")]
pub struct PublicEnumerationAssessment {
    #[model(key)]
    pub observation: Id<ExportEnumerationObservation>,
    pub access: Id<Module>,
    pub context: Id<AnalysisContext>,
    /// Exactly a supported, unconditional complete enumeration can establish absence.
    pub closed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "public_exposures")]
pub struct PublicExposure {
    #[model(key)]
    pub access: Id<Module>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub observation: Id<PublicNameObservation>,
    pub origin: Id<ExportOrigin>,
    pub enumeration: Option<Id<ExportEnumerationObservation>>,
    pub publicity: PublicPathKnowledge,
    pub status: ResolutionStatus,
    pub reason: EntityReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "public_exposure_candidates", projection_roles = crate::domain::projection::exposure_roles)]
pub struct PublicExposureCandidate {
    #[model(key)]
    pub exposure: Id<PublicExposure>,
    #[model(key)]
    pub resolution: Id<SymbolEntityResolution>,
    #[model(key)]
    pub support: Id<crate::domain::symbols::PublicNameSupport>,
}

pub fn source_callable(occurrence: &Occurrence) -> Option<CallableEntity> {
    let kind = match occurrence.syntax_kind {
        SyntaxKind::StmtFunctionDef => CallableKind::Function,
        SyntaxKind::ExprLambda => CallableKind::Lambda,
        _ => return None,
    };
    Some(CallableEntity::Source {
        declaration: occurrence.id(),
        kind,
    })
}
pub fn kind_accepts(kind: SymbolKind, occurrence: &Occurrence) -> bool {
    match kind {
        SymbolKind::Function | SymbolKind::Method => source_callable(occurrence).is_some(),
        SymbolKind::Class => occurrence.syntax_kind == SyntaxKind::StmtClassDef,
        SymbolKind::Module => occurrence.syntax_kind == SyntaxKind::ModModule,
        _ => false,
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<CallableEntity>(),
        Relation::of::<ClassEntity>(),
        Relation::of::<ParameterEntity>(),
        Relation::of::<FieldEntity>(),
        Relation::of::<EntityRef>(),
        Relation::of::<SymbolEntityResolution>(),
        Relation::of::<SymbolEntityCandidate>(),
        Relation::of::<SymbolEntityPremise>(),
        Relation::of::<SymbolEntityEvidence>(),
        Relation::of::<OccurrenceOwnership>(),
        Relation::of::<ParameterEntityLink>(),
        Relation::of::<FieldEntityLink>(),
        Relation::of::<FieldDeclarationLink>(),
        Relation::of::<PublicExposure>(),
        Relation::of::<PublicEnumerationAssessment>(),
        Relation::of::<PublicExposureCandidate>(),
    ]
}
