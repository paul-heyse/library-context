//! Finite PR4 selection contract. Evidence interpretation belongs to `selection`.
use super::*;

macro_rules! enumeration {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all="snake_case")]
        pub enum $name { $($variant),+ }
    };
}
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Quantifier {
    #[default]
    AnyApplicable,
    AllApplicable,
}
enumeration!(SelectionOutcome {
    Supported,
    Contradicted,
    Unresolved,
    Conflicting
});
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    #[default]
    Discovery,
    Strict,
}
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum JointPolicy {
    #[default]
    IndependentRecords,
    RequireCompatible,
}
enumeration!(EvidenceBasis {
    SourceDeclaration,
    ProviderDeclaration,
    BoundedModel,
    ObservedScenario
});
enumeration!(SelectionDomain {
    PublicExposures,
    SignatureVariants,
    ConfigurationFields,
    Relationships,
    Scenarios,
    SourceArtifacts,
    ReleaseDeclarations
});
enumeration!(JointApplicability {
    IndependentRecords,
    CompatibleModeledContext,
    DemonstratedCombination,
    ContradictoryModeledContext,
    NotEstablished
});
enumeration!(MemberKind {
    Function,
    Method,
    Class,
    Property,
    Module,
    Variable,
    Unknown
});
enumeration!(InvocationForm {
    Function,
    Method,
    Class,
    Static,
    Property,
    ContextManager,
    AsyncContextManager
});
enumeration!(ParameterMatchKind {
    PositionalOnly,
    Positional,
    VarArgs,
    KeywordOnly,
    VarKwargs
});
enumeration!(DefaultMatchState {
    Absent,
    LiteralNone,
    Literal,
    SourceExpression,
    OptionalExpressionUnavailable,
    Unknown,
    FactoryExpression
});
enumeration!(ConfigurationScopeKind { Object, PerCall });
enumeration!(FieldRelationship {
    DeclaredParameter,
    ExactStorage,
    ExactReader
});
enumeration!(ScenarioCheck {
    Parse,
    Binding,
    Environment,
    Execution
});
enumeration!(DeploymentField {
    RequiresDist,
    ProvidesExtra,
    RequiresPython,
    EntryPoint,
    Launch,
    Configuration
});
enumeration!(RelationRole {
    Declares,
    Surface,
    Configuration,
    Reader,
    Documents,
    Demonstrates,
    Invokes,
    TestsFailure,
    Suggests,
    Observes
});
enumeration!(RelationFidelity {
    SourceFact,
    ResolvedTarget,
    CandidateTargets,
    ExactTextualReference,
    AmbiguousTextualMention,
    SameDocumentPassage,
    OwningDistribution,
    TaskObservation,
    ReleaseDistribution,
    ExplicitConfigReference
});

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operator", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructuralType {
    CanonicalTerm {
        term: TypeTermId,
    },
    Category {
        #[serde(deserialize_with = "decode_type_kind")]
        #[schemars(with = "TypeKind")]
        category: String,
    },
    NominalIdentity {
        module: Text<1, 500>,
        name: Text<1, 500>,
    },
    DeclaredUnionMember {
        term: TypeTermId,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelationTarget {
    Declaration {
        node: crate::Id,
    },
    Member {
        member: PublicMemberId,
    },
    Evidence {
        evidence: crate::evidence::EvidenceRef,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldTarget {
    Node {
        node: crate::Id,
    },
    Parameter {
        signature: SignatureId,
        ordinal: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "predicate", rename_all = "snake_case", deny_unknown_fields)]
pub enum Requirement {
    /// Behavioral may-facet evidence; unknown and not-analyzed never prove absence.
    FacetMembership {
        facet: FacetName,
        value: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Exact public access path; aliases retain separate member identities.
    PublicPath {
        path: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Exact owning module, attributed by the exposure source.
    PublicModule {
        module: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Exact qualified class owner of this member.
    ClassOwner {
        path: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Declared public member category.
    MemberKind {
        kind: MemberKind,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// A recognized binding or protocol surface; opaque decorators remain unresolved.
    InvocationForm {
        form: InvocationForm,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Named declaration across applicable signature variants, not arbitrary keyword acceptance.
    DeclaresParameter {
        name: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ParameterKind {
        name: Text<1, 500>,
        kind: ParameterMatchKind,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ParameterRequired {
        name: Text<1, 500>,
        required: bool,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ParameterDefaultState {
        name: Text<1, 500>,
        state: DefaultMatchState,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ParameterDefault {
        name: Text<1, 500>,
        value: ExactPrimitive,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Structural declared type matching without implicit subtype or runtime-value inference.
    ParameterType {
        name: Text<1, 500>,
        r#type: StructuralType,
        #[serde(default)]
        quantifier: Quantifier,
    },
    DeclaresConfigurationField {
        name: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ConfigurationOwner {
        path: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Object settings versus per-call parameter records; does not establish shared runtime state.
    ConfigurationScope {
        scope: ConfigurationScopeKind,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ConfigurationRecordKind {
        #[serde(deserialize_with = "decode_record_model")]
        #[schemars(with = "RecordModel")]
        model: String,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ConfigurationDefault {
        name: Text<1, 500>,
        value: ExactPrimitive,
        #[serde(default)]
        quantifier: Quantifier,
    },
    ConfigurationLiteral {
        name: Text<1, 500>,
        value: ExactPrimitive,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// An exact cited field link at its typed formal or reader endpoint.
    ConfigurationRelationship {
        name: Text<1, 500>,
        kind: FieldRelationship,
        target: FieldTarget,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Attributed role, target and fidelity; candidate targets never become resolved edges.
    Relationship {
        role: RelationRole,
        target: RelationTarget,
        fidelity: RelationFidelity,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Original scenario intent, independent of its checks or execution status.
    ScenarioIntent {
        intent: crate::evidence::Intent,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// One explicit check axis: parse, binding, environment or execution.
    ScenarioCheck {
        check: ScenarioCheck,
        status: crate::evidence::CheckStatus,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Alignment of evidence associated with this member, not all artifacts in its release.
    SourceAlignment {
        alignment: crate::evidence::Alignment,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Exact version for the named owning distribution. Absence without closure is unknown.
    ReleaseVersion {
        distribution: Text<1, 500>,
        version: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
    /// Declared metadata value, exact configuration pointer, or original launch text; no installability or execution inference.
    DeploymentDeclaration {
        field: DeploymentField,
        name: Text<1, 500>,
        #[serde(default)]
        quantifier: Quantifier,
    },
}
impl Requirement {
    /// Canonical domain contexts actually consumed by this predicate. Output classification
    /// domains also label facets and source deployment observations that use other inputs.
    pub fn input_domain(&self) -> Option<SelectionDomain> {
        match self {
            Self::FacetMembership { .. }
            | Self::DeploymentDeclaration {
                field: DeploymentField::Launch | DeploymentField::Configuration,
                ..
            } => None,
            _ => Some(self.domain()),
        }
    }
    /// Finite source dependencies shared by hydration and predicate documentation.
    pub fn dependencies(&self) -> &'static [&'static str] {
        match self {
            Self::FacetMembership { .. } => return &["operation_facets", "operation_facet_status"],
            Self::DeclaresParameter { .. }
            | Self::ParameterKind { .. }
            | Self::ParameterRequired { .. }
            | Self::ParameterDefaultState { .. }
            | Self::ParameterDefault { .. } => {
                return &[
                    "catalog_bindings",
                    "catalog_signatures",
                    "catalog_parameters",
                ];
            }
            _ => {}
        }
        match self.domain() {
            SelectionDomain::PublicExposures => &["catalog_bindings", "catalog_surfaces"],
            SelectionDomain::SignatureVariants => &[
                "catalog_bindings",
                "catalog_surfaces",
                "catalog_signatures",
                "catalog_parameters",
                "catalog_type_observations",
                "catalog_types",
                "catalog_type_args",
            ],
            SelectionDomain::ConfigurationFields => &[
                "catalog_bindings",
                "catalog_surfaces",
                "catalog_signatures",
                "catalog_parameters",
                "catalog_types",
                "catalog_type_args",
                "catalog_configurations",
                "catalog_field_links",
            ],
            SelectionDomain::Relationships => &[
                "catalog_associations",
                "catalog_bindings",
                "catalog_signatures",
                "catalog_parameters",
                "catalog_spans",
                "catalog_scenarios",
                "catalog_deployments",
            ],
            SelectionDomain::Scenarios => &["catalog_associations", "catalog_scenarios"],
            SelectionDomain::SourceArtifacts => {
                if matches!(self, Self::DeploymentDeclaration { .. }) {
                    &[
                        "catalog_spans",
                        "catalog_artifacts",
                        "catalog_associations",
                        "catalog_deployments",
                    ]
                } else {
                    &["catalog_spans", "catalog_artifacts"]
                }
            }
            SelectionDomain::ReleaseDeclarations => {
                &["catalog_associations", "catalog_deployments"]
            }
        }
    }
    pub fn quantifier(&self) -> Quantifier {
        match self {
            Self::FacetMembership { quantifier, .. }
            | Self::PublicPath { quantifier, .. }
            | Self::PublicModule { quantifier, .. }
            | Self::ClassOwner { quantifier, .. }
            | Self::MemberKind { quantifier, .. }
            | Self::InvocationForm { quantifier, .. }
            | Self::DeclaresParameter { quantifier, .. }
            | Self::ParameterKind { quantifier, .. }
            | Self::ParameterRequired { quantifier, .. }
            | Self::ParameterDefaultState { quantifier, .. }
            | Self::ParameterDefault { quantifier, .. }
            | Self::ParameterType { quantifier, .. }
            | Self::DeclaresConfigurationField { quantifier, .. }
            | Self::ConfigurationOwner { quantifier, .. }
            | Self::ConfigurationScope { quantifier, .. }
            | Self::ConfigurationRecordKind { quantifier, .. }
            | Self::ConfigurationDefault { quantifier, .. }
            | Self::ConfigurationLiteral { quantifier, .. }
            | Self::ConfigurationRelationship { quantifier, .. }
            | Self::Relationship { quantifier, .. }
            | Self::ScenarioIntent { quantifier, .. }
            | Self::ScenarioCheck { quantifier, .. }
            | Self::SourceAlignment { quantifier, .. }
            | Self::ReleaseVersion { quantifier, .. }
            | Self::DeploymentDeclaration { quantifier, .. } => *quantifier,
        }
    }
    pub fn domain(&self) -> SelectionDomain {
        match self {
            Self::PublicPath { .. }
            | Self::PublicModule { .. }
            | Self::ClassOwner { .. }
            | Self::MemberKind { .. }
            | Self::InvocationForm { .. } => SelectionDomain::PublicExposures,
            Self::DeclaresParameter { .. }
            | Self::ParameterKind { .. }
            | Self::ParameterRequired { .. }
            | Self::ParameterDefaultState { .. }
            | Self::ParameterDefault { .. }
            | Self::ParameterType { .. } => SelectionDomain::SignatureVariants,
            Self::DeclaresConfigurationField { .. }
            | Self::ConfigurationOwner { .. }
            | Self::ConfigurationScope { .. }
            | Self::ConfigurationRecordKind { .. }
            | Self::ConfigurationDefault { .. }
            | Self::ConfigurationLiteral { .. }
            | Self::ConfigurationRelationship { .. } => SelectionDomain::ConfigurationFields,
            Self::Relationship { .. } | Self::FacetMembership { .. } => {
                SelectionDomain::Relationships
            }
            Self::ScenarioIntent { .. } | Self::ScenarioCheck { .. } => SelectionDomain::Scenarios,
            Self::SourceAlignment { .. }
            | Self::DeploymentDeclaration {
                field: DeploymentField::Launch | DeploymentField::Configuration,
                ..
            } => SelectionDomain::SourceArtifacts,
            Self::ReleaseVersion { .. } | Self::DeploymentDeclaration { .. } => {
                SelectionDomain::ReleaseDeclarations
            }
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
/// Up to sixteen conjunctive predicates. Any-applicable is existential; all-applicable
/// needs nonempty closed all-positive evidence or a clean counterexample. Default discovery
/// retains unresolved/conflicting candidates. Require-compatible intersects the entire conjunction.
pub struct Selection {
    #[serde(default)]
    #[schemars(length(max = 16))]
    pub requirements: Vec<Requirement>,
    #[serde(default)]
    pub mode: SelectionMode,
    #[serde(default)]
    pub joint: JointPolicy,
}
impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Input {
            #[serde(default)]
            requirements: Vec<Requirement>,
            #[serde(default)]
            mode: SelectionMode,
            #[serde(default)]
            joint: JointPolicy,
        }
        let i = Input::deserialize(d)?;
        if i.requirements.len() > 16 {
            return Err(serde::de::Error::custom("at most sixteen requirements"));
        }
        Ok(Self {
            requirements: i.requirements,
            mode: i.mode,
            joint: i.joint,
        })
    }
}
/// A context is admissible by construction; an absent runtime instance is never a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionContext {
    Member {
        member: PublicMemberId,
    },
    Binding {
        member: PublicMemberId,
        binding: BindingId,
    },
    Signature {
        member: PublicMemberId,
        binding: BindingId,
        signature: SignatureId,
    },
    Configuration {
        member: PublicMemberId,
        owner: crate::Id,
        scope: ConfigurationScopeKind,
    },
    Scenario {
        member: PublicMemberId,
        scenario: ScenarioId,
    },
    Source {
        member: PublicMemberId,
        span: SpanId,
    },
    Release {
        release: crate::Id,
    },
    Runtime {
        member: PublicMemberId,
        binding: BindingId,
        signature: SignatureId,
        owner: crate::Id,
        instance: crate::Id,
        evaluation: crate::Id,
        model: crate::Digest,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WitnessEvidence {
    Fact {
        id: crate::Id,
    },
    Catalog {
        id: EvidenceId,
    },
    FieldLink {
        id: crate::Id,
        field: crate::Id,
        signature: Option<crate::Id>,
        ordinal: u32,
        formal: Option<crate::Id>,
        reader: Option<crate::Id>,
        relationship: FieldRelationship,
    },
    Association {
        id: crate::Id,
        role: RelationRole,
        fidelity: RelationFidelity,
        target: RelationTarget,
        edge: Option<crate::Id>,
        modality: Option<String>,
        phase: Option<String>,
    },
    Original {
        evidence: crate::evidence::EvidenceRef,
    },
    Domain {
        id: crate::Id,
    },
    Facet {
        facet: String,
        value: String,
        verdict: String,
    },
    FacetCoverage {
        facet: String,
        verdict: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequirementWitness {
    Positive {
        context: SelectionContext,
        basis: EvidenceBasis,
        evidence: Vec<WitnessEvidence>,
    },
    Negative {
        context: SelectionContext,
        basis: EvidenceBasis,
        evidence: Vec<WitnessEvidence>,
    },
    Conflict {
        context: SelectionContext,
        basis: EvidenceBasis,
        positive: Vec<WitnessEvidence>,
        negative: Vec<WitnessEvidence>,
    },
}
enumeration!(SelectionReason {
    Witness,
    Counterexample,
    ClosedAbsence,
    NoApplicableDomain,
    IncompleteDomain,
    ComparableConflict,
    IncompatibleContexts,
    MissingContext,
    ResourceRefused
});
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionCoverage {
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub evaluation_complete: bool,
    pub examined: u64,
    pub total: Option<u64>,
    pub closure: Vec<WitnessEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementResult {
    pub requirement: Requirement,
    pub outcome: SelectionOutcome,
    pub witnesses: Vec<RequirementWitness>,
    pub coverage: SelectionCoverage,
    pub reason: SelectionReason,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CandidateSelection {
    pub member_id: PublicMemberId,
    pub operation_id: Option<OperationId>,
    pub access_path: String,
    pub kind: String,
    pub outcome: SelectionOutcome,
    pub requirements: Vec<RequirementResult>,
    pub joint: JointApplicability,
    #[serde(default)]
    pub ranking: Option<crate::retrieval::RankedMember>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionGroup {
    pub items: Vec<CandidateSelection>,
    pub total: u64,
    pub page_complete: bool,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionResults {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub supported: SelectionGroup,
    pub unresolved: SelectionGroup,
    pub conflicting: SelectionGroup,
    pub contradicted_count: u64,
    pub ranked_discovery: bool,
    pub policy_revision: u32,
    #[serde(default)]
    pub retrieval: Option<RetrievalMetadata>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_terms_and_defaults() {
        let selection: Selection = serde_json::from_str(
            r#"{"requirements":[{"predicate":"declares_parameter","name":"timeout"}]}"#,
        )
        .unwrap();
        assert_eq!(selection.mode, SelectionMode::Discovery);
        assert_eq!(
            selection.requirements[0].quantifier(),
            Quantifier::AnyApplicable
        );
        assert!(
            serde_json::from_str::<Selection>(
                r#"{"requirements":[{"predicate":"accepts_keyword","name":"timeout"}]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Selection>(&format!(
                "{{\"requirements\":[{}]}}",
                vec![r#"{"predicate":"declares_parameter","name":"x"}"#; 17].join(",")
            ))
            .is_err()
        );
        assert!(decode("Selection", r#"{"facets":[]}"#).is_err());
    }
}
