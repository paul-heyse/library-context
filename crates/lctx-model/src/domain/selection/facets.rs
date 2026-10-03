//! Typed structural questions. The existing predicate evaluator owns their meaning.
//! Unsupported behavioral facet forms remain rejected before any service effects.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FacetValue {
    ParameterName { name: String },
    ParameterType { role: calls::SignatureRole, name: String, r#type: StructuralType },
    ReturnType { role: calls::SignatureRole, r#type: StructuralType },
    SpecializedParameterType { site: Id<source::Occurrence>, declaration: Id<types::NativeSignatureObservation>, parameter: Id<calls::SignatureParameter>, r#type: StructuralType },
    SpecializedReturnType { site: Id<source::Occurrence>, declaration: Id<types::NativeSignatureObservation>, signature: Id<calls::Signature>, r#type: StructuralType },
    DecoratorQualifiedName { module: String, path: Vec<String> },
    Async { asynchronous: bool },
    ModulePath { module: String },
    MemberKind { kind: MemberKind },
    RaisedClass { r#type: StructuralType },
    ClassMetadata { trait_kind: ClassFacet, present: bool },
    Deprecation { role: calls::SignatureRole, deprecated: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, crate::DomainCode)]
#[repr(i16)]
pub enum ClassFacet {
    FinalDeclaration = 0, Protocol = 1, RuntimeCheckable = 2, Enumeration = 3,
    ExplicitlyAbstract = 4, AbstractMembers = 5, ExplicitSlots = 6,
}

impl FacetValue {
    pub fn facet(&self) -> Facet {
        match self {
            Self::ParameterName { .. } => Facet::Parameter,
            Self::ParameterType { .. } | Self::SpecializedParameterType { .. } => Facet::ParameterType,
            Self::ReturnType { .. } | Self::SpecializedReturnType { .. } => Facet::Returns,
            Self::DecoratorQualifiedName { .. } => Facet::Decorator,
            Self::Async { .. } => Facet::Async,
            Self::ModulePath { .. } => Facet::Module,
            Self::MemberKind { .. } => Facet::Kind,
            Self::RaisedClass { .. } => Facet::Raises,
            Self::ClassMetadata { .. } => Facet::ClassMetadata,
            Self::Deprecation { .. } => Facet::Deprecation,
        }
    }
    pub fn domain(&self) -> DomainKind {
        match self {
            Self::ModulePath { .. } | Self::MemberKind { .. } | Self::ClassMetadata { .. } => DomainKind::PublicExposures,
            _ => DomainKind::SignatureVariants,
        }
    }
    /// Lower equivalent questions mechanically, preserving explicit callable roles.
    pub(crate) fn predicate(&self) -> Option<Predicate> {
        Some(match self {
            Self::ParameterName { name } => Predicate::DeclaresParameter { name: name.clone() },
            Self::ParameterType { role, name, r#type } => Predicate::VariantParameterType { role: *role, name: name.clone(), r#type: r#type.clone() },
            Self::ReturnType { role, r#type } => Predicate::VariantReturnType { role: *role, r#type: r#type.clone() },
            Self::SpecializedParameterType { site, declaration, parameter, r#type } => Predicate::SpecializedType {site:*site,declaration:*declaration,subject:types::SignatureTypeSubject::Parameter {parameter:*parameter}.id(),r#type:r#type.clone()},
            Self::SpecializedReturnType { site, declaration, signature, r#type } => Predicate::SpecializedType {site:*site,declaration:*declaration,subject:types::SignatureTypeSubject::Return {signature:*signature}.id(),r#type:r#type.clone()},
            Self::ModulePath { module } => Predicate::PublicModule { module: module.clone() },
            Self::MemberKind { kind } => Predicate::MemberKind { kind: *kind },
            _ => return None,
        })
    }
    pub fn validate(&self, facet: Facet) -> Result<(), ModelError> {
        if self.facet() != facet { return Err(ModelError::Invalid("facet membership and typed value disagree or have no supported operator".into())); }
        if let Some(predicate) = self.predicate() { return predicate.validate(); }
        match self {
            Self::DecoratorQualifiedName{module,path} if !module.is_empty() && !path.is_empty() && path.iter().all(|s|!s.is_empty())=>Ok(()),
            Self::Async{..}|Self::ClassMetadata{..}=>Ok(()),
            Self::Deprecation{role,..} if *role!=calls::SignatureRole::Specialized=>Ok(()),
            Self::RaisedClass{r#type}=>Predicate::VariantReturnType{role:calls::SignatureRole::Source,r#type:r#type.clone()}.validate(),
            _=>Err(ModelError::Invalid("facet value has no supported located question".into())),
        }
    }
}
impl HeapSize for FacetValue {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::ParameterName { name } => name.heap_bytes(),
            Self::ParameterType { name, r#type, .. } => name.heap_bytes() + r#type.heap_bytes(),
            Self::ReturnType { r#type, .. } | Self::SpecializedParameterType { r#type, .. } | Self::SpecializedReturnType { r#type, .. } | Self::RaisedClass { r#type } => r#type.heap_bytes(),
            Self::DecoratorQualifiedName { module, path } => module.heap_bytes() + path.heap_bytes(),
            Self::ModulePath { module } => module.heap_bytes(),
            _ => 0,
        }
    }
}
