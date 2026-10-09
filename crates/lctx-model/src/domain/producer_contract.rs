//! Producer semantics are independent of executable builds and immutable captured bindings.
use super::{ContentHash, Id, Key, KeySink, ModelError, Record, attribution::{AnalysisContext, FactFamily, Provider}, input::InputRevision, stages::{Effect, FamilyCoverage, Profile, RelationUse, Stage}};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationBinding {
    ProducerSettings,
    CapturedAnalysisContext,
    Fixed(ContentHash),
}
impl ConfigurationBinding {
    fn encode(self, sink:&mut KeySink) {
        match self { Self::ProducerSettings=>sink.part(b"configuration-binding",b"producer-settings"), Self::CapturedAnalysisContext=>sink.part(b"configuration-binding",b"captured-analysis-context"), Self::Fixed(value)=>{sink.part(b"configuration-binding",b"fixed");value.encode(sink);} }
    }
}
#[derive(Debug, Clone)]
pub struct SupplierRole {
    pub name: &'static str,
    pub tool: &'static str,
    pub analyzer_revision: &'static str,
    pub semantic_revision: u32,
    pub families: Vec<FactFamily>,
    pub configuration: ConfigurationBinding,
}
#[derive(Debug, Clone)]
pub struct ProducerContract {
    pub name: &'static str,
    pub semantic_revision: u32,
    pub inputs: Vec<RelationUse>,
    pub outputs: Vec<RelationUse>,
    pub contributes: Vec<RelationUse>,
    pub profiles: Vec<Profile>,
    pub effect: Effect,
    pub configuration: ConfigurationBinding,
    pub suppliers: Vec<SupplierRole>,
    /// Semantic reporting ownership is independent of the stage that materializes the rows.
    pub not_requested: Vec<FactFamily>,
}
impl ProducerContract {
    pub fn identity(&self)->ContentHash {
        let mut sink=KeySink::new("producer-semantic-contract/v1");
        self.name.to_string().encode(&mut sink);sink.part(b"revision",&self.semantic_revision.to_le_bytes());
        sink.part(b"effect",self.effect.name().as_bytes());self.configuration.encode(&mut sink);
        let mut profiles=self.profiles.clone();profiles.sort();for profile in profiles {profile.name().to_string().encode(&mut sink);}
        let mut inputs=self.inputs.clone();inputs.sort_by_key(|input|(input.name(),input.prefix()));
        for input in inputs {
            input.name().to_string().encode(&mut sink);
            input.prefix().map(|prefix|i16::from(prefix.code())).encode(&mut sink);
            sink.part(b"transport",match input.transport(){super::stages::InputTransport::Handoff=>b"handoff",super::stages::InputTransport::CompletedInput=>b"completed-input"});
            if let Some(requirement)=input.requirement(){requirement.group.encode(&mut sink);sink.part(b"availability",&[match requirement.policy{super::stages::AvailabilityPolicy::RequireComplete=>0,super::stages::AvailabilityPolicy::ObserveAvailability=>1}]);}
            let mut validators=input.validators().to_vec();validators.sort();for validator in validators {validator.to_string().encode(&mut sink);}
        }
        for (tag,relations) in [(b"output".as_slice(),&self.outputs),(b"contributes".as_slice(),&self.contributes)] {
            let names=relations.iter().map(|relation|relation.name()).collect::<BTreeSet<_>>();for name in names {sink.part(tag,name.as_bytes());}
        }
        let mut suppliers=self.suppliers.iter().collect::<Vec<_>>();suppliers.sort_by_key(|role|role.name);
        for role in suppliers {
            role.name.to_string().encode(&mut sink);role.tool.to_string().encode(&mut sink);role.analyzer_revision.to_string().encode(&mut sink);sink.part(b"supplier-semantic-revision",&role.semantic_revision.to_le_bytes());role.configuration.encode(&mut sink);
            for family in role.families.iter().copied().collect::<BTreeSet<_>>() {family.encode(&mut sink);}
        }
        for family in self.not_requested.iter().copied().collect::<BTreeSet<_>>() {sink.part(b"not-requested",&[family as u8]);}
        sink.finish()
    }
    pub fn validate(&self)->Result<(),ModelError> {
        if self.name.is_empty() || self.semantic_revision==0 || self.profiles.is_empty() || self.profiles.iter().copied().collect::<BTreeSet<_>>().len()!=self.profiles.len() || matches!(self.configuration,ConfigurationBinding::CapturedAnalysisContext) {
            return Err(ModelError::Invalid("invalid producer semantic contract".into()));
        }
        for inventory in [&self.inputs,&self.outputs,&self.contributes] {
            if inventory.iter().map(|relation|relation.name()).collect::<BTreeSet<_>>().len()!=inventory.len(){return Err(ModelError::Invalid("duplicate producer relation inventory".into()));}
        }
        if self.not_requested.iter().copied().collect::<BTreeSet<_>>().len()!=self.not_requested.len(){return Err(ModelError::Invalid("duplicate unrequested reporting inventory".into()));}
        let mut roles=BTreeSet::new();
        for role in &self.suppliers {
            if role.name.is_empty() || role.tool.is_empty() || role.analyzer_revision.is_empty() || role.semantic_revision==0 || role.families.is_empty() || !roles.insert(role.name) || role.families.iter().copied().collect::<BTreeSet<_>>().len()!=role.families.len() {
                return Err(ModelError::Invalid("invalid producer supplier role".into()));
            }
        }
        Ok(())
    }
    pub fn bind(&self,implementation:ContentHash,settings:ContentHash,suppliers:impl IntoIterator<Item=(&'static str,Provider)>)->Stage {
        let suppliers=suppliers.into_iter().map(|(role,provider)|(role.to_owned(),provider.id())).collect::<BTreeMap<_,_>>();
        let coverage=self.suppliers.iter().flat_map(|role|role.families.iter().map(|family|FamilyCoverage{family:*family,provider:*suppliers.get(role.name).expect("executable binds every declared role")})).collect();
        Stage {name:self.name,inputs:self.inputs.clone(),outputs:self.outputs.clone(),contributes:self.contributes.clone(),coverage,profiles:self.profiles.clone(),effect:self.effect,code:implementation,configuration:settings,
            captured_binding:Some(CapturedProducerBinding{contract:self.identity(),semantic_revision:self.semantic_revision,producer_settings:settings,suppliers,sources:vec![]})}
    }
    pub fn validate_binding(&self,binding:&CapturedProducerBinding,settings:Option<ContentHash>,providers:&[Provider])->Result<(),ModelError> {
        self.validate()?;binding.validate()?;
        if binding.contract!=self.identity() || binding.semantic_revision!=self.semantic_revision || settings!=Some(binding.producer_settings) || matches!(self.configuration,ConfigurationBinding::Fixed(value) if value!=binding.producer_settings)
            || binding.suppliers.keys().map(String::as_str).collect::<BTreeSet<_>>()!=self.suppliers.iter().map(|role|role.name).collect() {
            return Err(ModelError::Conflict("captured producer semantic binding"));
        }
        for role in &self.suppliers {
            let id=binding.suppliers[role.name];
            let mut matches=providers.iter().filter(|provider|provider.id()==id);
            let provider=matches.next().ok_or(ModelError::Conflict("missing captured supplier"))?;
            if matches.next().is_some() || provider.tool!=role.tool || provider.revision!=role.analyzer_revision {
                return Err(ModelError::Conflict("captured supplier role or analyzer revision"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedSourceBinding {
    pub input:Id<InputRevision>,
    pub context:Id<AnalysisContext>,
    pub configuration:ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedProducerBinding {
    pub contract:ContentHash,
    pub semantic_revision:u32,
    pub producer_settings:ContentHash,
    pub suppliers:BTreeMap<String,Id<Provider>>,
    pub sources:Vec<CapturedSourceBinding>,
}
impl CapturedProducerBinding {
    pub fn validate(&self)->Result<(),ModelError> {
        if self.semantic_revision==0 || self.suppliers.keys().any(String::is_empty) || self.suppliers.values().copied().collect::<BTreeSet<_>>().len()!=self.suppliers.len() || !self.sources.windows(2).all(|pair|pair[0].input<pair[1].input) {
            return Err(ModelError::Conflict("captured producer binding shape"));
        }
        Ok(())
    }
    pub fn identity(&self)->Result<ContentHash,ModelError> {
        self.validate()?;let mut sink=KeySink::new("captured-producer-binding/v1");
        self.contract.encode(&mut sink);sink.part(b"revision",&self.semantic_revision.to_le_bytes());self.producer_settings.encode(&mut sink);
        for (role,provider) in &self.suppliers {role.encode(&mut sink);provider.encode(&mut sink);}
        for source in &self.sources {source.input.encode(&mut sink);source.context.encode(&mut sink);source.configuration.encode(&mut sink);}
        Ok(sink.finish())
    }
    pub fn heap_bytes(&self)->usize {
        self.suppliers.keys().map(|role|role.len()+std::mem::size_of::<(String,Id<Provider>)>()+32).sum::<usize>()+self.sources.capacity()*std::mem::size_of::<CapturedSourceBinding>()
    }
}
/// An optional field is still required on the wire: explicit null denotes a non-facts producer.
pub fn required_optional<'de,D,T>(deserializer:D)->Result<Option<T>,D::Error> where D:serde::Deserializer<'de>,T:Deserialize<'de> {Option::<T>::deserialize(deserializer)}
