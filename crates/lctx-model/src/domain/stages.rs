//! Typed stage declarations are the sole writer authority.
use std::{any::TypeId, collections::{BTreeMap, BTreeSet, HashSet}};
use super::{ModelError, Record, ValidatedModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOutcome { Complete, Partial, Failed, NotRequested }
#[derive(Debug, Clone, Copy)]
pub struct RelationUse { pub(crate) type_id: TypeId, name: &'static str }
impl RelationUse {
    pub fn of<R: Record>() -> Self { Self { type_id: TypeId::of::<R>(), name: R::NAME } }
}
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: &'static str,
    pub inputs: Vec<RelationUse>,
    pub outputs: Vec<RelationUse>,
}
#[derive(Debug)]
pub struct Schedule { stages: Vec<Stage> }
impl Schedule {
    pub fn build(model: &ValidatedModel, stages: Vec<Stage>, required: &[RelationUse]) -> Result<Self, ModelError> {
        let members: HashSet<_> = model.relations().iter().map(super::Relation::type_id).collect();
        let mut names = HashSet::new();
        let mut writers = std::collections::HashMap::new();
        for (i, stage) in stages.iter().enumerate() {
            if !names.insert(stage.name) { return Err(ModelError::Invalid(format!("duplicate stage {}", stage.name))); }
            for r in stage.inputs.iter().chain(&stage.outputs) {
                if !members.contains(&r.type_id) { return Err(ModelError::Invalid(format!("undeclared relation {}", r.name))); }
            }
            for r in &stage.outputs {
                if writers.insert(r.type_id, i).is_some() { return Err(ModelError::Invalid(format!("multiple writers for {}", r.name))); }
            }
        }
        for r in required {
            if !members.contains(&r.type_id) || !writers.contains_key(&r.type_id) { return Err(ModelError::Invalid(format!("missing required writer {}", r.name))); }
        }
        let mut dependencies = BTreeMap::new();
        for (i, stage) in stages.iter().enumerate() {
            let mut deps = BTreeSet::new();
            for r in &stage.inputs {
                let writer = writers.get(&r.type_id).ok_or_else(|| ModelError::Invalid(format!("missing writer for {}", r.name)))?;
                deps.insert(*writer);
            }
            dependencies.insert(i, deps);
        }
        let mut ordered = Vec::new();
        while !dependencies.is_empty() {
            let next = dependencies.iter().filter(|(_, deps)| deps.is_empty()).map(|(i, _)| *i)
                .min_by_key(|i| stages[*i].name).ok_or_else(|| ModelError::Invalid("stage cycle".into()))?;
            dependencies.remove(&next);
            for deps in dependencies.values_mut() { deps.remove(&next); }
            ordered.push(stages[next].clone());
        }
        Ok(Self { stages: ordered })
    }
    pub fn stages(&self) -> &[Stage] { &self.stages }
}
