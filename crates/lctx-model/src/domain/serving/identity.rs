//! Independent invalidation boundaries; no serving change mutates canonical fact identity.
use crate::domain::{ContentHash, Key, KeySink};
/// The actual canonical PostgreSQL generation identifier, never a hashed alias.
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,Serialize,Deserialize,schemars::JsonSchema)]
#[serde(transparent)]
pub struct GenerationKey(pub [u8;16]);
impl GenerationKey {pub fn bytes(&self)->&[u8;16]{&self.0}}
impl Key for GenerationKey {fn encode(&self,sink:&mut KeySink){sink.part(b"generation",&self.0);}}
use serde::{Deserialize, Serialize};
macro_rules! identity {
    ($($name:ident),* $(,)?) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub ContentHash);
    )* };
}
identity!(MappingIdentity, PolicyIdentity, WireIdentity, ConsumerIdentity, RequestIdentity, CursorIdentity, ChannelIdentity);
/// A mapping/policy/wire identity is never the model digest.
pub fn policy_identity<T: Serialize>(policy: &T) -> Result<PolicyIdentity, crate::domain::ModelError> {
    let bytes = postcard::to_allocvec(policy).map_err(crate::domain::ModelError::codec)?;
    let mut sink = KeySink::new("serving-policy/v1"); sink.part(b"policy", &bytes);
    Ok(PolicyIdentity(sink.finish()))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsumedRelation {
    pub relation: String,
    pub epoch: ContentHash,
    pub content: ContentHash,
}
pub fn consumer_identity(generation: GenerationKey, consumed: &[ConsumedRelation], mapping: MappingIdentity,
    policy: PolicyIdentity, wire: WireIdentity, spec: Option<ContentHash>) -> Result<ConsumerIdentity, crate::domain::ModelError> {
    let mut relations = consumed.to_vec(); relations.sort_by(|a,b| a.relation.cmp(&b.relation));
    if relations.windows(2).any(|w| w[0].relation == w[1].relation) {
        return Err(crate::domain::ModelError::Invalid("duplicate consumed relation".into()));
    }
    let mut sink = KeySink::new("serving-consumer/v1"); generation.encode(&mut sink);
    mapping.0.encode(&mut sink); policy.0.encode(&mut sink); wire.0.encode(&mut sink); spec.encode(&mut sink);
    for r in relations { r.relation.encode(&mut sink); r.epoch.encode(&mut sink); r.content.encode(&mut sink); }
    Ok(ConsumerIdentity(sink.finish()))
}
