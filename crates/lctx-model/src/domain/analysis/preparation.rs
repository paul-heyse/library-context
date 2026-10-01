//! One-shot authored configuration and native premise declarations, before result owners run.
use super::{AnalysisDefinition, MethodParameters, ProjectionDefinition, native::NativeInventory};
use crate::domain::{*, models::{Catalog, records::CatalogRecords}, normalized::Rows, stages::*};

pub struct Configuration {
    budget: resources::ResourceBudget,
    catalogs: CatalogRecords,
    parameters: Rows<MethodParameters>,
    definitions: Rows<AnalysisDefinition>,
    projections: Rows<ProjectionDefinition>,
}
impl Configuration {
    pub fn check_budget(&self,budget:&resources::ResourceBudget)->Result<(),ModelError> {
        if !self.budget.shares_pool(budget) {return Err(ModelError::Invalid("analysis configuration belongs to another attempt budget".into()));}Ok(())
    }
    pub fn catalogs(&self)->&CatalogRecords {&self.catalogs}
    pub fn parameters(&self)->&Rows<MethodParameters> {&self.parameters}
    pub fn definitions(&self)->&Rows<AnalysisDefinition> {&self.definitions}
    pub fn projections(&self)->&Rows<ProjectionDefinition> {&self.projections}
    pub fn new(catalog: &Catalog, definitions: impl IntoIterator<Item=(MethodParameters,AnalysisDefinition)>, budget: &resources::ResourceBudget) -> Result<Self,ModelError> {
        let mut rows=Self {budget:budget.clone(),catalogs:CatalogRecords::new(budget),parameters:Rows::new(budget),definitions:Rows::new(budget),projections:Rows::new(budget)};
        rows.catalogs.insert_borrowed(catalog)?;
        for (parameters,definition) in definitions {
            parameters.validate()?; definition.validate()?;
            if definition.parameters!=parameters.id() || parameters.model_catalog.is_some_and(|id|id!=catalog.declaration().id()) {
                return Err(ModelError::Invalid("analysis definition has foreign parameters or authored catalog".into()));
            }
            rows.parameters.insert(parameters)?;rows.definitions.insert(definition)?;
        }
        for name in projection::ProjectionName::ALL {rows.projections.insert(ProjectionDefinition::builtin(name))?;}
        Ok(rows)
    }
    pub fn declaration(&self) -> Stage {
        let mut key=KeySink::new("analysis-configuration");
        for row in self.catalogs.catalogs.iter() {row.id().encode(&mut key);}
        for row in self.definitions.iter() {row.id().encode(&mut key);}
        for row in self.projections.iter() {row.id().encode(&mut key);}
        Stage {name:"analysis_configuration",inputs:vec![],outputs:configuration_relations().iter().map(RelationUse::of_relation).collect(),
            contributes:vec![],coverage:vec![],provider:None,profiles:Profile::ALL.to_vec(),effect:Effect::Pure,
            code:ContentHash::of(include_bytes!("preparation.rs")),configuration:key.finish()}
    }
}
pub fn configuration_relations()->Vec<Relation> {
    let mut rows=models::records::relations();
    rows.extend([Relation::of::<MethodParameters>(),Relation::of::<AnalysisDefinition>(),Relation::of::<ProjectionDefinition>()]);rows
}
/// Native attribution validation and its nominal reference closure are facts-owned. Bind every
/// vocabulary read to the facts prefix even when later consumers add further vocabulary.
pub fn native_stage(profile:Profile)->Stage {
    let mut inputs=NativeInventory::stage_inputs(profile).into_iter().map(|input|(input.name(),input)).collect::<std::collections::BTreeMap<_,_>>();
    let facts=facts_relations().into_iter().map(|relation|(relation.name(),relation)).collect::<std::collections::BTreeMap<_,_>>();
    let mut pending=inputs.keys().copied().collect::<Vec<_>>();
    while let Some(name)=pending.pop() {
        let relation=&facts[name];
        let references=relation.fields().iter().filter_map(|field|field.target().map(|(_,name)|name));
        let invariants=relation.invariants().iter().flat_map(|invariant|invariant.inputs.iter().map(ValidationInput::name));
        for required in references.chain(invariants) {
            if !inputs.contains_key(required) {inputs.insert(required,RelationUse::of_relation(&facts[required]).completed_store());pending.push(required);}
        }
    }
    let inputs=inputs.into_values().map(|input|if is_vocabulary(input.name()) {input.at_epoch(PublicationBoundary::Facts)}else{input}).collect();
    Stage {name:"analysis_native_inventory",inputs,outputs:super::native::relations().iter().map(RelationUse::of_relation).collect(),
        contributes:vec![],coverage:vec![],provider:None,profiles:vec![profile],effect:Effect::Pure,
        code:ContentHash::of(include_bytes!("native.rs")),configuration:ContentHash::of(b"native-inventory-v1")}
}
