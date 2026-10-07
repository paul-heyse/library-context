//! Selected embedding service/specification is completed before either consumption owner.
use super::*;
use crate::domain::{
    resources::{Reservation, ResourceBudget},
    stages::*,
};

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="embedding_service_configurations",validate=validate_service)]
pub struct ServiceConfiguration {
    #[model(key)]
    pub specification: Id<EmbeddingSpec>,
    #[model(key)]
    pub document: Id<DocumentRecipe>,
    #[model(key)]
    pub projection: Id<projection::ProjectionDefinition>,
    #[model(key)]
    pub endpoint: String,
}
fn validate_service(row: &ServiceConfiguration) -> Result<(), ModelError> {
    if row.endpoint.trim().is_empty() {
        return Err(ModelError::Invalid(
            "embedding service endpoint is empty".into(),
        ));
    }
    Ok(())
}
/// A private admitted selection. Subsequent mutation of a caller's Spec cannot change it.
pub struct Configuration {
    spec: Spec,
    row: EmbeddingSpec,
    document: DocumentRecipe,
    projection: projection::ProjectionDefinition,
    service: ServiceConfiguration,
    budget: ResourceBudget,
    _reservation: Box<dyn Reservation>,
}
impl Configuration {
    pub fn new(spec: &Spec, endpoint: &str, budget: &ResourceBudget) -> Result<Self, ModelError> {
        spec.validate().map_err(ModelError::Invalid)?;
        // The configuration is bounded before retaining its two mechanical representations.
        let bytes = spec
            .heap_bytes()
            .checked_add(size_of::<Spec>())
            .and_then(|n| n.checked_mul(3))
            .and_then(|n| n.checked_add(endpoint.len()))
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .ok_or_else(|| {
                ModelError::Invalid("embedding configuration allocation overflow".into())
            })?;
        let reservation = budget.reserve("embedding-configuration", bytes)?;
        let row = EmbeddingSpec::new(spec)?;
        let document = DocumentRecipe::new(spec)?;
        let projection = projection::ProjectionDefinition::initial(spec);
        projection.validate()?;
        let service = ServiceConfiguration {
            specification: row.id(),
            document: document.id(),
            projection: projection.id(),
            endpoint: endpoint.to_owned(),
        };
        service.validate()?;
        Ok(Self {
            spec: spec.clone(),
            row,
            document,
            projection,
            service,
            budget: budget.clone(),
            _reservation: reservation,
        })
    }
    pub fn from_selected(encoder:&EmbeddingSpec,document:&DocumentRecipe,projection:&projection::ProjectionDefinition,service:&ServiceConfiguration,budget:&ResourceBudget)->Result<Self,ModelError> {
        encoder.validate()?;document.validate()?;projection.validate()?;service.validate()?;
        if service.specification!=encoder.id() || service.document!=document.id() || service.projection!=projection.id() || projection.dimensions>encoder.dimensions {return Err(ModelError::Invalid("selected embedding recipes differ".into()));}
        let mut selected=Self::new(&document.configuration(encoder)?,&service.endpoint,budget)?;
        selected.projection=projection.clone();selected.service=service.clone();Ok(selected)
    }
    pub fn specification(&self) -> &Spec {
        &self.spec
    }
    pub fn row(&self) -> &EmbeddingSpec {
        &self.row
    }
    pub fn document(&self) -> &DocumentRecipe { &self.document }
    pub fn projection(&self) -> &projection::ProjectionDefinition { &self.projection }
    pub fn service(&self) -> &ServiceConfiguration {
        &self.service
    }
    pub fn check_budget(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Invalid(
                "embedding configuration belongs to another attempt budget".into(),
            ));
        }
        Ok(())
    }
}
/// Disabled embedding still has one completed, empty configuration publication.
pub fn stage(configuration: Option<&Configuration>) -> Stage {
    let mut key = KeySink::new("embedding-configuration-v1");
    configuration.map(|c| c.service.id()).encode(&mut key);
    Stage {
        name: "embedding_configuration",
        inputs: vec![],
        outputs: super::configuration_relations()
            .iter()
            .map(RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: Profile::ALL.to_vec(),
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("configuration.rs")),
        configuration: key.finish(),
    }
}
