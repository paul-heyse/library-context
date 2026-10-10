//! Independent invalidation boundaries; no serving change mutates canonical fact identity.
use super::Name;
use crate::domain::{ContentHash, Key, KeySink};
use serde::{Deserialize, Serialize};
/// Exact physical database names; a reader cannot silently switch either component.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct DatabaseIdentity {
    pub namespace: Name,
    pub database: Name,
}
/// One immutable semantic snapshot and its executable native realization.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SnapshotHandle {
    /// Immutable publication identity, never a database-wide current marker.
    pub publication: ContentHash,
    pub semantic: ContentHash,
    pub realization: ContentHash,
    /// Exact ordered completed-view inventory admitted by this publication.
    pub view: ContentHash,
    /// Installed physical storage/schema generation, independent of client build provenance.
    pub service_generation: ContentHash,
    /// Immutable answer-affecting executable definition epoch.
    pub definition_epoch: ContentHash,
    pub database: DatabaseIdentity,
}
impl SnapshotHandle {
    pub fn expected_publication(&self)->ContentHash {
        let mut sink=KeySink::new("native-publication/v2");
        self.semantic.encode(&mut sink);self.realization.encode(&mut sink);self.view.encode(&mut sink);
        self.service_generation.encode(&mut sink);self.definition_epoch.encode(&mut sink);sink.finish()
    }
    pub fn validate_identity(&self)->Result<(),crate::domain::ModelError>{
        if self.publication!=self.expected_publication(){return Err(crate::domain::ModelError::Conflict("publication identity derivation"));}Ok(())
    }
    /// Names are fixed trusted operation names; the epoch is a full content address.
    pub fn operation_definition_function(&self)->String {format!("fn::lctx_e{}_operation_definition",self.definition_epoch.hex())}
    pub fn library_roots_function(&self)->String {format!("fn::lctx_e{}_library_roots",self.definition_epoch.hex())}
}
impl Key for SnapshotHandle {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"snapshot-handle/v2", &[]);
        self.publication.encode(sink);
        self.semantic.encode(sink);
        self.realization.encode(sink);
        self.view.encode(sink);
        self.service_generation.encode(sink);
        self.definition_epoch.encode(sink);
        sink.part(b"namespace", self.database.namespace.as_str().as_bytes());
        sink.part(b"database", self.database.database.as_str().as_bytes());
    }
}
macro_rules! identity {
    ($($name:ident),* $(,)?) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub ContentHash);
    )* };
}
identity!(
    MappingIdentity,
    PolicyIdentity,
    WireIdentity,
    ConsumerIdentity,
    RequestIdentity,
    CursorIdentity,
    ChannelIdentity
);
/// A mapping/policy/wire identity is never the model digest.
pub fn policy_identity<T: Serialize>(
    policy: &T,
) -> Result<PolicyIdentity, crate::domain::ModelError> {
    let bytes = postcard::to_allocvec(policy).map_err(crate::domain::ModelError::codec)?;
    let mut sink = KeySink::new("serving-policy/v1");
    sink.part(b"policy", &bytes);
    Ok(PolicyIdentity(sink.finish()))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationDependency {
    pub operation: Name,
    pub definition: ContentHash,
}
pub fn consumer_identity(
    snapshot: &SnapshotHandle,
    consumed: &[OperationDependency],
    mapping: MappingIdentity,
    policy: PolicyIdentity,
    wire: WireIdentity,
    spec: Option<ContentHash>,
) -> Result<ConsumerIdentity, crate::domain::ModelError> {
    let mut relations = consumed.to_vec();
    relations.sort_by(|a, b| a.operation.cmp(&b.operation));
    if relations
        .windows(2)
        .any(|w| w[0].operation == w[1].operation)
    {
        return Err(crate::domain::ModelError::Invalid(
            "duplicate operation dependency".into(),
        ));
    }
    let mut sink = KeySink::new("serving-consumer/v2");
    snapshot.encode(&mut sink);
    mapping.0.encode(&mut sink);
    policy.0.encode(&mut sink);
    wire.0.encode(&mut sink);
    spec.encode(&mut sink);
    for r in relations {
        sink.part(b"operation", r.operation.as_str().as_bytes());
        r.definition.encode(&mut sink);
    }
    Ok(ConsumerIdentity(sink.finish()))
}
