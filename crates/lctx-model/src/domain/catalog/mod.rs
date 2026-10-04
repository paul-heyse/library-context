//! Mandatory public catalog. Normalized/raw contracts remain their single semantic owners.
pub mod access_routes;
mod aliases;
pub mod build;
pub mod evidence;
mod inventory;
mod paths;
use crate::domain::{
    input::InputRevision,
    normalized::{callables::*, entities::*, links::*},
    source::{Module, Occurrence},
    symbols::FunctionTraitObservation,
    syntax::ParameterSyntaxObservation,
    types::RecordFieldObservation,
    value::Literal,
    *,
};
use crate::{Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_members",validate=validate_member,invariants=build::invariants,semantic_source=include_bytes!("build.rs"))]
pub struct CatalogMember {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub access: Id<Module>,
    #[model(key)]
    pub path: Vec<String>,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_exposures")]
pub struct CatalogExposure {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub exposure: Id<PublicExposure>,
}
/// The original exposure candidate remains visible even when it has no entity candidate.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_candidates")]
pub struct CatalogCandidate {
    #[model(key)]
    pub exposure: Id<CatalogExposure>,
    #[model(key)]
    pub candidate: Option<Id<PublicExposureCandidate>>,
    #[model(key)]
    pub entity: Option<Id<SymbolEntityCandidate>>,
    #[model(key)]
    pub path: Option<Id<CatalogPath>>,
    #[model(key)]
    pub alias: Option<Id<CatalogAlias>>,
}
/// Normalized assessment retains all component uncertainty; no catalog reclassification.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_callables")]
pub struct CatalogCallable {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub candidate: Id<CatalogCandidate>,
    #[model(key)]
    pub assessment: Id<EffectiveCallableAssessment>,
    pub basis: CatalogContractBasis,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_invocations")]
pub struct CatalogInvocation {
    #[model(key)]
    pub callable: Id<CatalogCallable>,
    #[model(key)]
    pub variant: Id<SignatureVariant>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_callable_aspects")]
pub struct CatalogCallableAspect {
    #[model(key)]
    pub callable: Id<CatalogCallable>,
    #[model(key)]
    pub aspect: Id<normalized::callable_aspects::CallableAspect>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_classes")]
pub struct CatalogClass {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub candidate: Id<CatalogCandidate>,
    #[model(key)]
    pub class: Id<ClassEntity>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_class_members", rule = "catalog_class_member_typing")]
pub struct CatalogClassMember {
    #[model(key)]
    pub class: Id<CatalogClass>,
    #[model(key, premise)]
    pub observation: Id<super::class_metadata::ClassMemberObservation>,
}
/// A catalog class's provider-owned typing characterization, preserving every support-qualified observation.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "catalog_class_metadata",
    rule = "catalog_class_typing_metadata"
)]
pub struct CatalogClassMetadata {
    #[model(key)]
    pub class: Id<CatalogClass>,
    #[model(key, premise)]
    pub observation: Id<super::class_metadata::ClassMetadataObservation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ConstructorOrigin {
    Own = 0,
    Inherited = 1,
    Synthetic = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ConstructorKind {
    Init = 0,
    New = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ConstructorDisposition {
    Effective = 0,
    Shadowed = 1,
    Candidate = 2,
}
/// Association evidence, not a claim that an open MRO has a unique runtime winner.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_constructors")]
pub struct CatalogConstructor {
    #[model(key)]
    pub class: Id<CatalogClass>,
    #[model(key)]
    pub callable: Id<CatalogCallable>,
    #[model(key)]
    pub traits: Id<FunctionTraitObservation>,
    #[model(key)]
    pub ancestry: Option<Id<AncestryEntityMember>>,
    pub origin: ConstructorOrigin,
    pub kind: ConstructorKind,
    pub applicability: Knowledge,
    pub disposition: ConstructorDisposition,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_option_subjects")]
pub enum CatalogOptionSubject {
    #[model(code = 0)]
    Parameter { slot: Id<SignatureSlot> },
    #[model(code = 1)]
    Field { field: Id<FieldEntity> },
    #[model(code = 2)]
    SourceParameter { parameter: Id<ParameterEntity> },
}
/// Missing evaluation and missing source evidence never mean a known absent default.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_defaults")]
pub enum CatalogDefault {
    #[model(code = 0)]
    Absent {},
    #[model(code = 1)]
    Unavailable {},
    #[model(code = 2)]
    Unknown {},
    #[model(code = 3)]
    Literal { literal: Id<Literal> },
    #[model(code = 4)]
    Expression { expression: Id<Occurrence> },
    /// Factory expression backed by the normalized field-default premise.
    #[model(code = 5)]
    Factory { expression: Id<Occurrence> },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_option_evidence")]
pub enum CatalogOptionEvidence {
    #[model(code = 0)]
    Parameter {
        entity: Id<SignatureSlotEntity>,
        syntax: Id<ParameterSyntaxObservation>,
    },
    #[model(code = 1)]
    NativeParameter { slot: Id<SignatureSlot> },
    #[model(code = 2)]
    DeclaredField {
        declaration: Id<FieldDeclarationLink>,
        assessment: Id<normalized::callable_aspects::FieldDefaultAssessment>,
    },
    #[model(code = 3)]
    NativeField {
        link: Id<FieldEntityLink>,
        observation: Id<RecordFieldObservation>,
    },
    #[model(code = 4)]
    SourceParameter {
        syntax: Id<ParameterSyntaxObservation>,
        placement: Option<Id<syntax::SyntaxPlacement>>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_options")]
pub struct CatalogOption {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub subject: Id<CatalogOptionSubject>,
    #[model(key)]
    pub evidence: Id<CatalogOptionEvidence>,
    pub default: Id<CatalogDefault>,
}

pub fn core_relations() -> Vec<Relation> {
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    {
        let mut rows = crate::catalog_outputs!(declare);
        rows.push(Relation::of::<CatalogMemberInvocation>());
        rows
    }
}
/// Each slot's contextual computation has exact admitted source receipts and coverage.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_member_invocations",invariants=build::invocation_invariants,semantic_source=include_bytes!("build.rs"))]
pub struct CatalogMemberInvocation {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub invocation: Id<analysis::catalog_core::Invocation>,
}

fn validate_member(row: &CatalogMember) -> Result<(), ModelError> {
    if row.path.is_empty()
        || row.path.iter().any(|s| s.is_empty())
        || row.name != row.path.join(".")
    {
        return Err(ModelError::Invalid(
            "catalog public slot path/rendering differs".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PublicPathDisposition {
    Effective = 0,
    Shadowed = 1,
    Candidate = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_paths",semantic_source=include_bytes!("paths.rs"))]
pub struct CatalogPath {
    #[model(key)]
    pub parent: Id<CatalogCandidate>,
    #[model(key)]
    pub declaration: Id<syntax::DeclarationObservation>,
    #[model(key)]
    pub binding: Id<lexical::BindingObservation>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    #[model(key)]
    pub ancestry: Option<Id<AncestryEntityMember>>,
    pub disposition: PublicPathDisposition,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PublicPathBoundaryReason {
    RecursiveClass = 0,
    Depth = 1,
    OpenAncestry = 2,
    Rebound = 3,
    MissingCorrespondence = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_path_boundaries")]
pub struct CatalogPathBoundary {
    #[model(key)]
    pub parent: Id<CatalogCandidate>,
    #[model(key)]
    pub reason: PublicPathBoundaryReason,
    #[model(key)]
    pub binding: Option<Id<lexical::BindingObservation>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CatalogContractBasis {
    PublicCandidate = 0,
    SourceOnlyAlias = 1,
}
/// A source alias premise does not improve its original public exposure or runtime equivalence.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_aliases",semantic_source=include_bytes!("aliases.rs"))]
pub struct CatalogAlias {
    #[model(key)]
    pub parent: Id<CatalogExposure>,
    #[model(key)]
    pub binding: Id<lexical::BindingObservation>,
    #[model(key)]
    pub ownership: Id<OccurrenceOwnership>,
    #[model(key)]
    pub reference: Id<ReferenceEntityCandidate>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    pub basis: CatalogContractBasis,
}

pub fn relations() -> Vec<Relation> {
    let mut rows = core_relations();
    rows.extend(evidence::relations());
    rows
}
