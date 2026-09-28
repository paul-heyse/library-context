//! Contract foundation for PR4 selection. These types do not classify evidence or change legacy
//! facet semantics, and are not accepted by the existing tool requests until that consumer lands.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Quantifier {
    AnyApplicable,
    AllApplicable,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StructuralTypeOperator {
    CanonicalTerm,
    DeclaredUnionMember,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOutcome {
    Supported,
    Contradicted,
    Unresolved,
    Conflicting,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBasis {
    SourceDeclaration,
    ProviderDeclaration,
    BoundedModel,
    ObservedScenario,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionDomain {
    PublicExposures,
    SignatureVariants,
    ConfigurationFields,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JointApplicability {
    IndependentRecords,
    CompatibleModeledContext,
    DemonstratedCombination,
    ContradictoryModeledContext,
    NotEstablished,
}

/// Each variant fixes its domain instead of accepting contradictory domain/operator combinations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "predicate", rename_all = "snake_case", deny_unknown_fields)]
pub enum Requirement {
    PublicPath {
        path: Text<1, 500>,
        quantifier: Quantifier,
    },
    DeclaresParameter {
        name: Text<1, 500>,
        quantifier: Quantifier,
    },
    DeclaresParameterType {
        name: Text<1, 500>,
        operator: StructuralTypeOperator,
        term: TypeTermId,
        quantifier: Quantifier,
    },
    DeclaresConfigurationField {
        name: Text<1, 500>,
        quantifier: Quantifier,
    },
}
impl Requirement {
    pub fn domain(&self) -> SelectionDomain {
        match self {
            Self::PublicPath { .. } => SelectionDomain::PublicExposures,
            Self::DeclaresParameter { .. } | Self::DeclaresParameterType { .. } => {
                SelectionDomain::SignatureVariants
            }
            Self::DeclaresConfigurationField { .. } => SelectionDomain::ConfigurationFields,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementWitness {
    pub member: PublicMemberId,
    #[serde(default)]
    pub binding: Option<BindingId>,
    #[serde(default)]
    pub signature: Option<SignatureId>,
    #[serde(default)]
    pub configuration_owner: Option<crate::Id>,
    #[serde(default)]
    pub configuration_field: Option<crate::Id>,
    pub evidence: Vec<EvidenceId>,
    pub basis: EvidenceBasis,
    #[serde(default)]
    pub model: Option<crate::Digest>,
    #[serde(default)]
    pub context: Option<crate::Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementResult {
    pub requirement: Requirement,
    pub outcome: SelectionOutcome,
    pub witnesses: Vec<RequirementWitness>,
    pub examined: u64,
    #[serde(default)]
    pub total: Option<u64>,
    pub domain_complete: bool,
    #[serde(default)]
    pub reason: Option<Text<1, 500>>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requirement_contract_is_finite_and_does_not_extend_legacy_facets() {
        let raw =
            r#"{"predicate":"declares_parameter","name":"timeout","quantifier":"any_applicable"}"#;
        let term: Requirement = serde_json::from_str(raw).unwrap();
        assert!(matches!(term.domain(), SelectionDomain::SignatureVariants));
        let validator = jsonschema::options()
            .offline()
            .build(&schema_for::<Requirement>(false))
            .unwrap();
        assert!(validator.is_valid(&serde_json::from_str(raw).unwrap()));
        for invalid in [
            raw.replace("declares_parameter", "accepts_keyword"),
            raw.replace("any_applicable", "any"),
            raw.replace("\"timeout\"", "null"),
        ] {
            assert!(serde_json::from_str::<Requirement>(&invalid).is_err());
            assert!(!validator.is_valid(&serde_json::from_str(&invalid).unwrap()));
        }
        assert!(decode("Where", r#"{"requirements":[]}"#).is_err());
    }
}
