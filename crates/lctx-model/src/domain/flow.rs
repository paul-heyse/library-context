//! Raw flow observations. A reaching definition is not a value-transfer or call-summary proof.
//! Occurrences and structural places are shared with syntax and conditions; no rendered-place IDs.
use crate::{Assertion,Domain,DomainCode,DomainSum};
use super::{*,assertion::AssertionQualification,attribution::FactFamily,lexical::{BindingEventKind,LexicalScope},
    source::Occurrence,value::Place,transfer::TransferKind};

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "flow_uses", invariants = flow_invariants)]
pub struct FlowUse {
    #[model(key)] pub occurrence: Id<Occurrence>,
    #[model(key)] pub place: Id<Place>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "flow_definitions")]
pub struct FlowDefinition {
    #[model(key)] pub occurrence: Id<Occurrence>,
    #[model(key)] pub place: Id<Place>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "flow_use_observations")]
#[assertion(support = FlowUseSupport, name = "flow_use_supports", family = FactFamily::Flow, subjects(use_, scope))]
pub struct FlowUseObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub use_: Id<FlowUse>,
    #[model(key)] pub scope: Id<LexicalScope>,
    #[model(key)] pub annotation: bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "flow_definition_observations")]
#[assertion(support = FlowDefinitionSupport, name = "flow_definition_supports", family = FactFamily::Flow, subjects(definition, scope, value))]
pub struct FlowDefinitionObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub definition: Id<FlowDefinition>,
    #[model(key)] pub scope: Id<LexicalScope>,
    #[model(key)] pub kind: BindingEventKind,
    #[model(key)] pub value: Option<Id<Occurrence>>,
}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name = "reaching_definitions")]
pub enum ReachingDefinition {
    #[model(code = 0)] Bound { definition: Id<FlowDefinition> },
    /// The provider observes a possibly unbound place on this path. This is not missing coverage.
    #[model(code = 1)] Unbound,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "flow_reaching_observations")]
#[assertion(support = FlowReachingSupport, name = "flow_reaching_supports", family = FactFamily::Flow, subjects(use_, target))]
pub struct FlowReachingObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub use_: Id<FlowUse>,
    #[model(key)] pub target: Id<ReachingDefinition>,
    #[model(key)] pub loop_carried: bool,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum FlowSinkKind { Definition = 0, Argument = 1, Return = 2, Yield = 3, Raise = 4 }
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "flow_value_observations", validate = validate_value)]
#[assertion(support = FlowValueSupport, name = "flow_value_supports", family = FactFamily::Flow, subjects(use_, sink))]
pub struct FlowValueObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub use_: Id<FlowUse>,
    #[model(key)] pub sink: Id<Occurrence>,
    #[model(key)] pub kind: FlowSinkKind,
    #[model(key)] pub transfer: TransferKind,
    /// A raw dependency crossing a call requires later call-transfer evidence.
    #[model(key)] pub through_call: bool,
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
fn validate_value(row: &FlowValueObservation) -> Result<(),ModelError> {
    if row.transfer == TransferKind::Identity && row.through_call {
        return Err(invalid("identity flow cannot cross an unresolved call"));
    }
    Ok(())
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "flow_region_observations")]
#[assertion(support = FlowRegionSupport, name = "flow_region_supports", family = FactFamily::Flow, subjects(statement, scope))]
pub struct FlowRegionObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub statement: Id<Occurrence>,
    #[model(key)] pub scope: Id<LexicalScope>,
}

fn flow_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "flow_reaching_places",inputs: vec![
        ValidationInput::of::<FlowUse>(&["id"]),ValidationInput::of::<FlowDefinition>(&["id"]),
        ValidationInput::of::<ReachingDefinition>(&["id"]),ValidationInput::of::<FlowReachingObservation>(&["id"]),
    ],create: std::sync::Arc::new(|| Box::new(ReachingCheck::default())) },
    Invariant { name: "flow_source_structure",inputs: vec![
        ValidationInput::of::<Occurrence>(&["id"]),ValidationInput::of::<LexicalScope>(&["id"]),
        ValidationInput::of::<FlowUse>(&["id"]),ValidationInput::of::<FlowDefinition>(&["id"]),
        ValidationInput::of::<FlowUseObservation>(&["id"]),ValidationInput::of::<FlowDefinitionObservation>(&["id"]),
        ValidationInput::of::<FlowValueObservation>(&["id"]),ValidationInput::of::<FlowRegionObservation>(&["id"]),
    ],create: std::sync::Arc::new(|| Box::new(FlowStructure::default())) }]
}
#[derive(Default)]
struct ReachingCheck {
    uses: std::collections::BTreeMap<Id<FlowUse>,Id<Place>>,
    definitions: std::collections::BTreeMap<Id<FlowDefinition>,Id<Place>>,
    targets: std::collections::BTreeMap<Id<ReachingDefinition>,ReachingDefinition>,
}
impl InvariantCheck for ReachingCheck {
    fn visit(&mut self, relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == FlowUse::NAME { for row in FlowUse::decode(batch)? { self.uses.insert(row.id(),row.place); } }
        else if relation == FlowDefinition::NAME { for row in FlowDefinition::decode(batch)? { self.definitions.insert(row.id(),row.place); } }
        else if relation == ReachingDefinition::NAME { for row in ReachingDefinition::decode(batch)? { self.targets.insert(row.id(),row); } }
        else if relation == FlowReachingObservation::NAME { for row in FlowReachingObservation::decode(batch)? {
            let place = self.uses.get(&row.use_).ok_or_else(|| invalid("reaching use missing"))?;
            if let ReachingDefinition::Bound { definition } = self.targets.get(&row.target).ok_or_else(|| invalid("reaching target missing"))? {
                if self.definitions.get(definition) != Some(place) { return Err(invalid("reaching definition belongs to a different place")); }
            }
        } }
        else { return Err(invalid("undeclared reaching validation input")); }
        if self.uses.len()+self.definitions.len()+self.targets.len() > 1_000_000 { return Err(invalid("reaching validation cardinality limit")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}

#[derive(Default)]
struct FlowStructure {
    occurrences: std::collections::BTreeMap<Id<Occurrence>,Id<super::source::SourceArtifact>>,
    scopes: std::collections::BTreeMap<Id<LexicalScope>,Id<Occurrence>>,
    uses: std::collections::BTreeMap<Id<FlowUse>,Id<Occurrence>>,
    definitions: std::collections::BTreeMap<Id<FlowDefinition>,Id<Occurrence>>,
}
impl FlowStructure {
    fn same_source(&self, left: Id<Occurrence>,right: Id<Occurrence>) -> Result<(),ModelError> {
        let left = self.occurrences.get(&left).ok_or_else(|| invalid("flow occurrence missing"))?;
        if self.occurrences.get(&right) != Some(left) { return Err(invalid("flow source structure crosses source artifacts")); }
        Ok(())
    }
    fn scope(&self, id: Id<LexicalScope>) -> Result<Id<Occurrence>,ModelError> {
        self.scopes.get(&id).copied().ok_or_else(|| invalid("flow lexical scope missing"))
    }
    fn use_site(&self, id: Id<FlowUse>) -> Result<Id<Occurrence>,ModelError> {
        self.uses.get(&id).copied().ok_or_else(|| invalid("flow use missing"))
    }
}
impl InvariantCheck for FlowStructure {
    fn visit(&mut self, relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == Occurrence::NAME { for row in Occurrence::decode(batch)? { self.occurrences.insert(row.id(),row.source); } }
        else if relation == LexicalScope::NAME { for row in LexicalScope::decode(batch)? { self.scopes.insert(row.id(),row.owner); } }
        else if relation == FlowUse::NAME { for row in FlowUse::decode(batch)? { self.uses.insert(row.id(),row.occurrence); } }
        else if relation == FlowDefinition::NAME { for row in FlowDefinition::decode(batch)? { self.definitions.insert(row.id(),row.occurrence); } }
        else if relation == FlowUseObservation::NAME { for row in FlowUseObservation::decode(batch)? {
            self.same_source(self.use_site(row.use_)?,self.scope(row.scope)?)?;
        } }
        else if relation == FlowDefinitionObservation::NAME { for row in FlowDefinitionObservation::decode(batch)? {
            let site = *self.definitions.get(&row.definition).ok_or_else(|| invalid("flow definition missing"))?;
            self.same_source(site,self.scope(row.scope)?)?;
            if let Some(value) = row.value { self.same_source(site,value)?; }
        } }
        else if relation == FlowValueObservation::NAME { for row in FlowValueObservation::decode(batch)? { self.same_source(self.use_site(row.use_)?,row.sink)?; } }
        else if relation == FlowRegionObservation::NAME { for row in FlowRegionObservation::decode(batch)? { self.same_source(row.statement,self.scope(row.scope)?)?; } }
        else { return Err(invalid("undeclared flow structure input")); }
        if self.occurrences.len()+self.scopes.len()+self.uses.len()+self.definitions.len() > 1_000_000 { return Err(invalid("flow structure cardinality limit")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}
