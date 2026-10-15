//! One authoritative provider request constructs portable and contribution identities.
use super::super::{
    compilation_product::{DependencyToken, ProductKind, ProductRequest},
    completed::{CompletedContribution, ContributionSpec},
    *,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    contribution: ContributionSpec,
    product: ProductRequest,
}
impl CapturedRequest {
    pub fn new(
        contribution: ContributionSpec,
        result_contract: ContentHash,
        dependencies: Vec<DependencyToken>,
    ) -> Result<Self, ModelError> {
        contribution.identity()?;
        // Retain the full conservative contract until narrower projections have explicit contracts.
        // Ordered dependencies preserve roles; the exact captured acquisition inventory includes
        // empty manifests, absent selections and source order/multiplicity.
        let parameters = serde_json::to_vec(&contribution).map_err(ModelError::codec)?;
        let product = ProductRequest {
            kind: ProductKind::PureRows,
            operation: contribution.producer.clone(),
            model: contribution.model,
            implementation: contribution.implementation,
            policy: ContentHash::of(&SEMANTIC_POLICY_REVISION.to_le_bytes()),
            result_contract,
            configuration: contribution.configuration,
            profile: contribution.profile.name().into(),
            parameters,
            dependencies,
            outputs: contribution.outputs.clone(),
        };
        product.validate()?;
        Ok(Self {
            contribution,
            product,
        })
    }
    pub fn contribution(&self) -> &ContributionSpec {
        &self.contribution
    }
    pub fn product(&self) -> &ProductRequest {
        &self.product
    }
    pub fn into_product(self) -> ProductRequest {
        self.product
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.product.identity()
    }
    /// Semantic preflight only: a Hit still needs native held-incarnation protection/admission.
    pub fn eligibility(&self, candidate: Candidate<'_>) -> Eligibility {
        let Candidate::Completed {
            request,
            descriptor,
            admitted,
        } = candidate
        else {
            return Eligibility::Miss("candidate is not a terminal reusable completion");
        };
        let expected = match self.identity() {
            Ok(value) => value,
            Err(error) => return Eligibility::Corrupt(error),
        };
        if request != expected {
            return Eligibility::Miss("captured request differs");
        }
        if descriptor.spec != self.contribution {
            return Eligibility::Corrupt(ModelError::Conflict(
                "equal request with inconsistent contribution specification",
            ));
        }
        match descriptor.outcome {
            0 => {}
            // Completed compact declarations use physical Complete and independently admitted
            // semantic NotRequested domains. A stage outcome alone supplies no such proof.
            4 => {
                return Eligibility::Miss(
                    "stage NotRequested has no admitted compact output proof",
                );
            }
            1..=3 => return Eligibility::Miss("partial unavailable or failed provider outcome"),
            _ => return Eligibility::Corrupt(ModelError::Conflict("unknown provider outcome")),
        }
        if let Err(error) = descriptor.identity() {
            return Eligibility::Corrupt(error);
        }
        if !admitted {
            return Eligibility::Miss("candidate requires admission before reuse");
        }
        Eligibility::Hit
    }
}
pub enum Candidate<'a> {
    Completed {
        request: ContentHash,
        descriptor: &'a CompletedContribution,
        admitted: bool,
    },
    Unfinished,
    Abandoned,
    Poisoned,
    ResourcePartial,
}
#[derive(Debug)]
pub enum Eligibility {
    Hit,
    Miss(&'static str),
    Corrupt(ModelError),
}
