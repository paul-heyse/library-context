//! One-shot authored configuration and native premise declarations, before result owners run.
use super::{AnalysisDefinition, MethodParameters, ProjectionDefinition, native::NativeInventory};
use crate::domain::{
    models::{Catalog, records::CatalogRecords},
    normalized::Rows,
    stages::*,
    *,
};

pub struct Configuration {
    budget: resources::ResourceBudget,
    catalogs: CatalogRecords,
    parameters: Rows<MethodParameters>,
    definitions: Rows<AnalysisDefinition>,
    projections: Rows<ProjectionDefinition>,
    analytics: Rows<super::settings::AnalyticsConfiguration>,
    retrieval: Rows<crate::domain::retrieval::RetrievalDefinition>,
}
impl Configuration {
    pub fn check_budget(&self, budget: &resources::ResourceBudget) -> Result<(), ModelError> {
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Invalid(
                "analysis configuration belongs to another attempt budget".into(),
            ));
        }
        Ok(())
    }
    pub fn catalogs(&self) -> &CatalogRecords {
        &self.catalogs
    }
    pub fn parameters(&self) -> &Rows<MethodParameters> {
        &self.parameters
    }
    pub fn definitions(&self) -> &Rows<AnalysisDefinition> {
        &self.definitions
    }
    pub fn analytics(&self) -> &Rows<super::settings::AnalyticsConfiguration> {
        &self.analytics
    }
    pub fn with_analytics(
        mut self,
        settings: super::settings::AnalyticsConfiguration,
    ) -> Result<Self, ModelError> {
        settings.validate()?;
        if !self.analytics.is_empty() {
            return Err(ModelError::Invalid(
                "analytics configuration already selected".into(),
            ));
        }
        self.analytics.insert(settings)?;
        Ok(self)
    }
    pub fn retrieval(&self) -> &Rows<crate::domain::retrieval::RetrievalDefinition> {
        &self.retrieval
    }
    pub fn with_retrieval(
        mut self,
        definition: crate::domain::retrieval::RetrievalDefinition,
    ) -> Result<Self, ModelError> {
        definition.validate()?;
        if !self.retrieval.is_empty() {
            return Err(ModelError::Invalid(
                "retrieval configuration already selected".into(),
            ));
        }
        self.retrieval.insert(definition)?;
        Ok(self)
    }
    pub fn projections(&self) -> &Rows<ProjectionDefinition> {
        &self.projections
    }
    pub fn new(
        catalog: &Catalog,
        definitions: impl IntoIterator<Item = (MethodParameters, AnalysisDefinition)>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut rows = Self {
            budget: budget.clone(),
            catalogs: CatalogRecords::new(budget),
            parameters: Rows::new(budget),
            definitions: Rows::new(budget),
            projections: Rows::new(budget),
            analytics: Rows::new(budget),
            retrieval: Rows::new(budget),
        };
        rows.catalogs.insert_borrowed(catalog)?;
        for (parameters, definition) in definitions {
            parameters.validate()?;
            definition.validate()?;
            if definition.parameters != parameters.id()
                || parameters
                    .model_catalog
                    .is_some_and(|id| id != catalog.declaration().id())
            {
                return Err(ModelError::Invalid(
                    "analysis definition has foreign parameters or authored catalog".into(),
                ));
            }
            rows.parameters.insert(parameters)?;
            rows.definitions.insert(definition)?;
        }
        for name in projection::ProjectionName::ALL {
            rows.projections
                .insert(ProjectionDefinition::builtin(name))?;
        }
        Ok(rows)
    }
    pub fn declarations(&self) -> Vec<Stage> {
        ConfigurationGroup::ALL
            .into_iter()
            .map(|group| self.declaration(group))
            .collect()
    }
    pub fn declaration(&self, group: ConfigurationGroup) -> Stage {
        let mut key = KeySink::new("analysis-configuration-group/v1");
        key.part(b"group", group.name().as_bytes());
        macro_rules! rows {
            ($rows:expr) => {
                for row in $rows.iter() {
                    row.id().encode(&mut key);
                    row.content_digest().encode(&mut key);
                }
            };
        }
        match group {
            ConfigurationGroup::Authored => {
                rows!(self.catalogs.catalogs);
                rows!(self.catalogs.targets);
                rows!(self.catalogs.models);
                rows!(self.catalogs.protocols);
            }
            ConfigurationGroup::Methods => {
                rows!(self.parameters);
                rows!(self.definitions);
            }
            ConfigurationGroup::Projections => rows!(self.projections),
            ConfigurationGroup::Analytics => rows!(self.analytics),
            ConfigurationGroup::Retrieval => rows!(self.retrieval),
        }
        Stage {
            captured_binding: None,
            name: group.name(),
            inputs: vec![],
            outputs: group
                .relations()
                .iter()
                .map(RelationUse::of_relation)
                .collect(),
            contributes: vec![],
            coverage: vec![],
            profiles: Profile::ALL.to_vec(),
            effect: Effect::Pure,
            code: ContentHash::of(include_bytes!("preparation.rs")),
            configuration: key.finish(),
        }
    }
}
/// Each group has one writer and an independently completed dependency. Method rows retain
/// a single writer; consumers select the definition and parameters of their actual method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationGroup {
    Authored,
    Methods,
    Projections,
    Analytics,
    Retrieval,
}
impl ConfigurationGroup {
    pub const ALL: [Self; 5] = [
        Self::Authored,
        Self::Methods,
        Self::Projections,
        Self::Analytics,
        Self::Retrieval,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Authored => "analysis_authored_configuration",
            Self::Methods => "analysis_method_configuration",
            Self::Projections => "analysis_projection_configuration",
            Self::Analytics => "analysis_analytics_configuration",
            Self::Retrieval => "analysis_retrieval_configuration",
        }
    }
    pub fn relations(self) -> Vec<Relation> {
        match self {
            Self::Authored => models::records::relations(),
            Self::Methods => vec![
                Relation::of::<MethodParameters>(),
                Relation::of::<AnalysisDefinition>(),
            ],
            Self::Projections => vec![Relation::of::<ProjectionDefinition>()],
            Self::Analytics => vec![Relation::of::<super::settings::AnalyticsConfiguration>()],
            Self::Retrieval => {
                vec![Relation::of::<crate::domain::retrieval::RetrievalDefinition>()]
            }
        }
    }
}
pub fn configuration_relations() -> Vec<Relation> {
    ConfigurationGroup::ALL
        .into_iter()
        .flat_map(ConfigurationGroup::relations)
        .collect()
}

/// Native attribution validation and its nominal reference closure are facts-owned. Bind every
/// vocabulary read to the facts prefix even when later consumers add further vocabulary.
pub fn native_stage(
    profile: Profile,
    model: &ValidatedModel,
    order: &PublicationOrder,
) -> Result<Stage, ModelError> {
    let outputs = super::native::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    let direct = NativeInventory::stage_inputs(profile);
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &direct)?;
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        direct,
        &outputs,
        PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::IncludeInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        captured_binding: None,
        name: "analysis_native_inventory",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("native.rs")),
        configuration: ContentHash::of(b"native-inventory-v1"),
    })
}
