//! Independent invalidation boundaries; no serving change mutates canonical fact identity.
use crate::domain::{ContentHash, Key, KeySink};
use super::Name;
use serde::{Deserialize, Serialize};
/// Exact physical database names; a reader cannot silently switch either component.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatabaseIdentity {
    pub namespace: Name,
    pub database: Name,
}
/// One immutable semantic snapshot and its executable native realization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotHandle {
    pub semantic: ContentHash,
    pub realization: ContentHash,
    pub database: DatabaseIdentity,
}
impl Key for SnapshotHandle {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"snapshot-handle/v1", &[]);
        self.semantic.encode(sink);
        self.realization.encode(sink);
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
    if relations.windows(2).any(|w| w[0].operation == w[1].operation) {
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
