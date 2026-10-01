//! Explicit, frozen model selection shared by every provider in an extraction attempt.
use lctx_model::domain::{
    ContentHash, KeySink, ModelError,
    models::{Catalog, requirements::ModelContextRequirements},
    resources::{Reservation, ResourceBudget},
    stages::Profile,
};
use std::sync::Arc;

/// Owns the admitted parser/catalog allocation for as long as any caller uses the selection.
pub struct CatalogSelection {
    catalog: Catalog,
    budget: ResourceBudget,
    _reservation: Box<dyn Reservation>,
}
impl CatalogSelection {
    pub fn parse(
        name: &str,
        source: &str,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let reservation = budget.reserve(
            "native-authored-catalog",
            source.len().saturating_mul(32).saturating_add(65536),
        )?;
        let catalog = Catalog::parse(name, source).map_err(ModelError::Invalid)?;
        Ok(Arc::new(Self {
            catalog,
            budget: budget.clone(),
            _reservation: reservation,
        }))
    }
    pub fn committed(budget: &ResourceBudget) -> Result<Arc<Self>, ModelError> {
        Self::parse("external.toml", Catalog::committed_source(), budget)
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
}
#[derive(Clone)]
pub struct NativeContextConfig {
    profile: Profile,
    catalog: Arc<CatalogSelection>,
}
impl NativeContextConfig {
    pub fn new(profile: Profile, catalog: Arc<CatalogSelection>) -> Self {
        Self { profile, catalog }
    }
    /// An outer caller explicitly selects the embedded committed catalog.
    pub fn committed(profile: Profile, budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self::new(profile, CatalogSelection::committed(budget)?))
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.catalog.budget
    }
    pub fn check_budget(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.budget().shares_pool(budget) {
            return Err(ModelError::Invalid(
                "selected native catalog allocation belongs to a different attempt resource pool"
                    .into(),
            ));
        }
        Ok(())
    }
    pub fn catalog(&self) -> &Catalog {
        self.catalog.catalog()
    }
    pub fn check_profile(&self, profile: Profile) -> Result<(), ModelError> {
        if profile != self.profile {
            return Err(ModelError::Invalid(
                "scheduled profile differs from captured native context configuration".into(),
            ));
        }
        Ok(())
    }
    pub fn digest(&self) -> ContentHash {
        let mut hash = KeySink::new("native-context-configuration");
        hash.part(b"profile", self.profile.name().as_bytes());
        hash.part(b"authored-catalog", &self.catalog().digest().0);
        hash.finish()
    }
    pub fn requirements(
        &self,
        python: &str,
        budget: &ResourceBudget,
    ) -> Result<Option<ModelContextRequirements>, ModelError> {
        self.check_budget(budget)?;
        if self.profile == Profile::Catalog {
            return Ok(None);
        }
        Ok(Some(ModelContextRequirements::derive(
            self.catalog(),
            python,
            budget,
        )?))
    }
}
