//! Typed domain authority. Physical layouts are lowerings of these definitions (ADR-0085).
mod identity;
mod model;
mod record;
pub mod source;
pub mod artifact;
pub mod value;
pub mod conditions;
pub mod assertion;
pub mod input;
pub mod attribution;
pub mod attachment;
pub mod stages;
pub mod resources;
pub mod obligation;

pub use identity::{ArmId, ContentHash, EvidenceBytes, Id, Key, KeySink};
pub use model::{Invariant, InvariantCheck, ValidationInput, Relation, RelationContent, ValidatedModel};
pub use record::{Arm, ArmField, Sum, SumRecord, Batch, Field, FieldValue, FlatValue, Record, Scalar};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("{owner} memory reservation refused: requested {requested} bytes with {used}/{limit} reserved")]
    Resource { owner: &'static str, requested: usize, used: usize, limit: usize },
    #[error("invalid model: {0}")]
    Invalid(String),
    #[error("wrong schema for {0}")]
    Schema(&'static str),
    #[error("identity does not match semantic key for {0}")]
    Identity(&'static str),
    #[error("equal semantic key has conflicting payload in {0}")]
    Conflict(&'static str),
    #[error("codec: {0}")]
    Codec(String),
}
impl ModelError {
    pub fn codec(error: impl std::fmt::Display) -> Self { Self::Codec(error.to_string()) }
}

#[doc(hidden)]
pub mod __private {
    pub use arrow_array::RecordBatch;
    pub use serde;
    pub use serde_arrow;
}

/// Declaration controls are paired: a reference collection must be a relationship record.
///
/// ```compile_fail
/// use lctx_model::{Domain, domain::{Id, input::Package}};
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "bad_collection")]
/// struct Collection { #[model(key)] name: String, members: Vec<Id<Package>> }
/// ```
///
/// ```
/// use lctx_model::{Domain, domain::{Id, input::Package}};
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "package_memberships")]
/// struct Membership { #[model(key)] owner: Id<Package>, #[model(key)] member: Id<Package> }
/// ```
///
/// ```compile_fail
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "missing_key")]
/// struct MissingKey { name: String }
/// ```
///
/// ```
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "has_key")]
/// struct HasKey { #[model(key)] name: String }
/// ```
///
/// ```compile_fail
/// use lctx_model::DomainCode;
/// #[derive(DomainCode)]
/// enum ImplicitCode { One, Two }
/// ```
///
/// ```
/// use lctx_model::DomainCode;
/// #[derive(DomainCode)]
/// #[repr(i16)]
/// enum ExplicitCode { One = 0, Two = 1 }
/// ```
///
/// ```compile_fail
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "nested_optional")]
/// struct Nested { #[model(key)] name: String, value: Option<Option<String>> }
/// ```
///
/// ```
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "ordinary_optional")]
/// struct Ordinary { #[model(key)] name: String, value: Option<String> }
/// ```
pub mod declaration_controls {}

/// The sole production relation membership manifest; physical inventories are derived.
pub fn model() -> Result<ValidatedModel, ModelError> {
    use input::*;
    use attribution::*;
    use source::*;
    use value::*;
    use conditions::*;
    use assertion::*;
    ValidatedModel::validate(vec![
        Relation::of::<AssertionQualification>(), Relation::of::<ProviderSurface>(), Relation::of::<Evidence>(),
        Relation::of::<Literal>(), Relation::of::<LiteralSet>(), Relation::of::<LiteralSetMember>(),
        Relation::of::<PlaceRoot>(), Relation::of::<PathSegment>(), Relation::of::<AccessPath>(), Relation::of::<Place>(),
        Relation::of::<Predicate>(), Relation::of::<EvaluationAtom>(), Relation::of::<ConditionNode>(), Relation::of::<Condition>(),
        Relation::of::<Package>(), Relation::of::<Release>(), Relation::of::<InputRevision>(),
        Relation::of::<InputOrigin>(), Relation::of::<InputAcquisition>(), Relation::of::<CorpusLibrary>(),
        Relation::of::<InputDistribution>(), Relation::of::<DistributionVerification>(),
        Relation::of::<ArtifactOwnership>(), Relation::of::<ArtifactUse>(), Relation::of::<SourceArtifact>(), Relation::of::<artifact::ArtifactChunk>(), Relation::of::<Module>(),
        Relation::of::<Occurrence>(), Relation::of::<Provider>(), Relation::of::<AnalysisContext>(),
        Relation::of::<ProviderRun>(), Relation::of::<RunFamily>(), Relation::of::<SyntaxObservation>(), Relation::of::<SyntaxSupport>(),
        Relation::of::<CoverageScope>(), Relation::of::<ProviderCoverage>(),
    ])
}
