//! Continuations bind actual admitted channels and representation independently of row count.
use super::{identity::*, *};
use crate::domain::{ContentHash, Id, KeySink, catalog::CatalogMember};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorChannel {
    Disabled {},
    Available {
        spec: ContentHash,
        query_vector: ContentHash,
    },
    Degraded {
        reason: Name,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelState {
    pub lexical: bool,
    pub vector: VectorChannel,
}
impl ChannelState {
    pub fn identity(&self) -> ChannelIdentity {
        let mut sink = KeySink::new("serving-channels/v1");
        sink.part(
            b"channels",
            &postcard::to_allocvec(self).expect("finite channel encoding"),
        );
        ChannelIdentity(sink.finish())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CursorBinding {
    pub snapshot: SnapshotHandle,
    pub request: RequestIdentity,
    pub policy: PolicyIdentity,
    pub wire: WireIdentity,
    pub channels: ChannelIdentity,
    pub group: Name,
    pub section: Name,
    pub member: Option<Id<CatalogMember>>,
    pub ordering: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub binding: CursorBinding,
    pub after: CursorPosition,
}
/// Resume a canonical key range or the same bounded ranking result, never an offset prefix scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CursorPosition {
    Key { key: ContentHash },
    Ranked { score_bits: u64, key: ContentHash },
    Original { source: ContentHash, byte: u64 },
}
impl Cursor {
    pub fn identity(&self) -> CursorIdentity {
        let mut sink = KeySink::new("serving-cursor/v1");
        sink.part(
            b"cursor",
            &postcard::to_allocvec(self).expect("finite cursor encoding"),
        );
        CursorIdentity(sink.finish())
    }
    pub fn encode(&self) -> Result<CursorToken, WireError> {
        CursorToken::new(hex::encode(serde_json::to_vec(self)?))
    }
    pub fn decode(token: &CursorToken, expected: &CursorBinding) -> Result<Self, WireError> {
        let bytes = hex::decode(token.as_str())
            .map_err(|_| WireError::Invalid("cursor encoding".into()))?;
        let cursor: Self = serde_json::from_slice(&bytes)?;
        if &cursor.binding != expected {
            return Err(WireError::Continuation("snapshot, request, policy, representation, group, section, member, ordering or channel changed".into()));
        }
        if let CursorPosition::Ranked { score_bits, .. } = &cursor.after
            && (!f64::from_bits(*score_bits).is_finite()
                || *score_bits == (-0.0f64).to_bits())
        {
            return Err(WireError::Continuation("noncanonical ranking position".into()));
        }
        Ok(cursor)
    }
}
