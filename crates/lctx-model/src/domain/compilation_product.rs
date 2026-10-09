//! Portable acceleration data. These contracts never carry a completed owner or admission grant.
use super::{ContentHash, Key, KeySink, ModelError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PRODUCT_FORMAT: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductKind { Program, Membership, Topology, PureRows, PureValue }
impl ProductKind {
    fn code(self) -> u8 { match self {Self::Program=>0,Self::Membership=>1,Self::Topology=>2,Self::PureRows=>3,Self::PureValue=>4} }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind { ExactView, CompleteDomain, Topology, PureValue, Provenance }
impl DependencyKind {
    fn code(self)->u8 {match self {Self::ExactView=>0,Self::CompleteDomain=>1,Self::Topology=>2,Self::PureValue=>3,Self::Provenance=>4}}
}
/// Order is the operation's declared input order; role and prefix distinguish equal views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyToken {
    pub kind: DependencyKind,
    pub role: String,
    pub relation: String,
    pub prefix: Option<String>,
    pub identity: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductRequest {
    pub kind: ProductKind,
    pub operation: String,
    pub model: ContentHash,
    pub implementation: ContentHash,
    pub policy: ContentHash,
    pub result_contract: ContentHash,
    pub configuration: Option<ContentHash>,
    pub profile: String,
    pub parameters: Vec<u8>,
    pub dependencies: Vec<DependencyToken>,
    pub outputs: BTreeSet<String>,
}
impl ProductRequest {
    pub fn validate(&self)->Result<(),ModelError> {
        if self.operation.is_empty() || self.profile.is_empty() || self.outputs.iter().any(String::is_empty)
            || self.dependencies.iter().any(|d|d.role.is_empty()||d.relation.is_empty()||d.prefix.as_ref().is_some_and(String::is_empty)) {
            return Err(ModelError::Invalid("invalid compiled-product declaration".into()));
        }
        let mut roles=BTreeSet::new();
        if self.dependencies.iter().any(|d|!roles.insert((&d.role,&d.relation,&d.prefix))) {
            return Err(ModelError::Conflict("compiled-product dependency role"));
        }
        Ok(())
    }
    fn encode(&self,sink:&mut KeySink) {
        sink.part(b"format",&PRODUCT_FORMAT.to_le_bytes());
        sink.part(b"kind",&[self.kind.code()]);self.operation.encode(sink);
        self.model.encode(sink);self.implementation.encode(sink);self.policy.encode(sink);
        self.result_contract.encode(sink);self.configuration.encode(sink);self.profile.encode(sink);
        sink.part(b"parameters",&self.parameters);
        sink.part(b"dependency-count",&(self.dependencies.len() as u64).to_le_bytes());
        for d in &self.dependencies {sink.part(b"dependency-kind",&[d.kind.code()]);d.role.encode(sink);d.relation.encode(sink);d.prefix.encode(sink);d.identity.encode(sink);}
        sink.part(b"output-count",&(self.outputs.len() as u64).to_le_bytes());
        for output in &self.outputs {output.encode(sink);}
    }
    pub fn identity(&self)->Result<ContentHash,ModelError> {
        self.validate()?;let mut sink=KeySink::new("compiled-product-request/v1");self.encode(&mut sink);Ok(sink.finish())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductOutcome { Complete, NotRequested }
/// The owner supplies canonical typed bytes, not live providers, pointers or verified values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductSection {pub name:String,pub rows:u64,pub bytes:Vec<u8>}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableProduct {
    pub request:ProductRequest,
    pub outcome:ProductOutcome,
    pub sections:Vec<ProductSection>,

}
impl PortableProduct {
    pub fn validate(&self)->Result<(),ModelError> {
        self.request.validate()?;
        if self.sections.iter().map(|s|s.name.clone()).collect::<BTreeSet<_>>()!=self.request.outputs
            || self.sections.windows(2).any(|w|w[0].name>=w[1].name) {
            return Err(ModelError::Conflict("compiled-product complete section inventory"));
        }
        Ok(())
    }
    /// Value identity excludes request/provenance; only explicitly value-dependent consumers
    /// may use it for propagation cutoff. Ordinary completed views keep their own identities.
    pub fn value_identity(&self)->Result<ContentHash,ModelError> {
        self.validate()?;let mut sink=KeySink::new("compiled-product-value/v1");
        self.request.result_contract.encode(&mut sink);
        sink.part(b"outcome",&[match self.outcome {ProductOutcome::Complete=>0,ProductOutcome::NotRequested=>1}]);
        sink.part(b"sections",&(self.sections.len() as u64).to_le_bytes());
        for s in &self.sections {s.name.encode(&mut sink);sink.part(b"rows",&s.rows.to_le_bytes());sink.part(b"body",&s.bytes);}
        Ok(sink.finish())
    }
    pub fn identity(&self)->Result<ContentHash,ModelError> {
        let mut sink=KeySink::new("compiled-product-entry/v1");self.request.identity()?.encode(&mut sink);self.value_identity()?.encode(&mut sink);Ok(sink.finish())
    }
    /// Includes owned containers; transport/native copies require their own reservations.
    pub fn retained_bytes(&self)->Result<usize,ModelError> {
        let request=&self.request;
        let fields=[request.operation.len(),request.profile.len(),request.parameters.capacity(),request.outputs.iter().map(|s|s.len()+64).sum(),request.dependencies.iter().map(|d|size_of::<DependencyToken>()+d.role.len()+d.relation.len()+d.prefix.as_ref().map_or(0,String::len)).sum(),self.sections.iter().map(|s|size_of::<ProductSection>()+s.name.len()+s.bytes.capacity()).sum()];
        fields.into_iter().try_fold(size_of::<Self>(),|total,n|total.checked_add(n).ok_or_else(||ModelError::Invalid("compiled-product size overflow".into())))
    }
}

/// Canonical complete membership, including explicit missing roots and coverage/context token.
/// The producing owner establishes completeness against its current immutable inputs.
pub fn domain_identity(program:ContentHash,context:ContentHash,roots:&[(String,[u8;16],u8)],members:&[(String,[u8;16],ContentHash)])->ContentHash {
    let mut sink=KeySink::new("compiled-selection-domain/v1");program.encode(&mut sink);context.encode(&mut sink);
    sink.part(b"roots",&(roots.len() as u64).to_le_bytes());
    for (relation,key,outcome) in roots {relation.encode(&mut sink);sink.part(b"root",key);sink.part(b"outcome",&[*outcome]);}
    sink.part(b"members",&(members.len() as u64).to_le_bytes());
    for (relation,key,content) in members {relation.encode(&mut sink);sink.part(b"member",key);content.encode(&mut sink);}sink.finish()
}
/// Complete vertices and identity-bearing directed arcs, including isolated vertices and
/// edges internal to an SCC. The owner supplies canonical order and semantic multiplicity.
pub fn topology_identity(vertices:&[(String,[u8;16])],arcs:&[(String,[u8;16],[u8;16],[u8;16])])->ContentHash {
    let mut sink=KeySink::new("compiled-topology/v1");sink.part(b"vertices",&(vertices.len() as u64).to_le_bytes());
    for (kind,key) in vertices {kind.encode(&mut sink);sink.part(b"vertex",key);}
    sink.part(b"arcs",&(arcs.len() as u64).to_le_bytes());
    for (kind,key,source,target) in arcs {kind.encode(&mut sink);sink.part(b"arc",key);sink.part(b"source",source);sink.part(b"target",target);}sink.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request()->ProductRequest {ProductRequest{kind:ProductKind::PureRows,operation:"normalize".into(),model:ContentHash::of(b"m"),implementation:ContentHash::of(b"i"),policy:ContentHash::of(b"p"),result_contract:ContentHash::of(b"r"),configuration:None,profile:"catalog".into(),parameters:vec![],dependencies:vec![DependencyToken{kind:DependencyKind::ExactView,role:"syntax".into(),relation:"modules".into(),prefix:None,identity:ContentHash::of(b"view")}],outputs:BTreeSet::from(["rows".into()])}}
    #[test]fn roles_and_provenance_are_not_value_equality() {
        let a=PortableProduct{request:request(),outcome:ProductOutcome::Complete,sections:vec![ProductSection{name:"rows".into(),rows:1,bytes:b"canonical".to_vec()}]};
        let mut b=a.clone();b.request.dependencies[0].identity=ContentHash::of(b"new-provenance");
        assert_ne!(a.identity().unwrap(),b.identity().unwrap());assert_eq!(a.value_identity().unwrap(),b.value_identity().unwrap());
        b.request.dependencies[0].prefix=Some("normalized".into());assert_ne!(a.request.identity().unwrap(),b.request.identity().unwrap());
        b.request.dependencies.push(b.request.dependencies[0].clone());assert!(b.validate().is_err());
    }
    #[test]fn internal_scc_edges_parallel_arcs_and_isolates_change_topology() {
        let vertices=vec![("node".into(),[1;16]),("node".into(),[2;16]),("node".into(),[3;16])];
        let arcs=vec![("edge".into(),[4;16],[1;16],[2;16]),("edge".into(),[5;16],[2;16],[1;16])];
        let full=topology_identity(&vertices,&arcs);assert_ne!(full,topology_identity(&vertices[..2],&arcs));assert_ne!(full,topology_identity(&vertices,&arcs[..1]));
        let mut parallel=arcs.clone();parallel.push(("edge".into(),[6;16],[1;16],[2;16]));assert_ne!(full,topology_identity(&vertices,&parallel));
    }
    #[test]fn negative_empty_domain_and_inventory_are_explicit() {
        let p=ContentHash::of(b"program");let c=ContentHash::of(b"coverage");let root=vec![("callable".into(),[1;16],0)];
        assert_ne!(domain_identity(p,c,&root,&[]),domain_identity(p,c,&[],&[]));
        assert_ne!(domain_identity(p,c,&root,&[]),domain_identity(p,c,&root,&[("callable".into(),[1;16],c)]));
        let invalid=PortableProduct{request:request(),outcome:ProductOutcome::Complete,sections:vec![]};assert!(invalid.validate().is_err());
    }
}
