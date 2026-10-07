//! Independent capture decoder. It receives exact emitted bytes, never a task/oracle/map.
//! The finite packet is a supported fixture format, not an MCP normalization/compatibility route.
use std::collections::BTreeSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::contracts::{Assignment, CandidateStatus, ObserverFormat};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicQualification {
    pub id: String,
    pub text: String,
    /// Preserved public epistemic qualifiers; readable source is not definite behavior.
    #[serde(default)]
    pub attributes: Assignment,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicEvidence {
    pub anchor: String,
    /// Evidence-instance coordinates authenticate this leaf, never constrain semantic joins.
    #[serde(default)]
    pub provenance: Assignment,
    pub role: String,
    pub text: String,
    pub qualifications: Vec<PublicQualification>,
    pub candidate_status: CandidateStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicGroup {
    #[serde(deserialize_with = "unique_context")]
    pub context: Assignment,
    pub evidence: Vec<PublicEvidence>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinitePacket {
    pub realization: String,
    pub groups: Vec<PublicGroup>,
    pub references: Vec<String>,
}
fn unique_context<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Assignment, D::Error> {
    struct ContextVisitor;
    impl<'de> serde::de::Visitor<'de> for ContextVisitor {
        type Value = Assignment;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("unique context bindings") }
        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut context = Assignment::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if context.insert(key, value).is_some() { return Err(serde::de::Error::custom("duplicate public context binding")); }
            }
            Ok(context)
        }
    }
    deserializer.deserialize_map(ContextVisitor)
}

/// Internal decoded facts shared by observers; never serialized as a substitute MCP response.
pub struct DecodedPacket {
    pub tool: Option<String>,
    pub semantic_snapshot: Option<String>,
    pub database_identity: Option<String>,
    pub groups: Vec<PublicGroup>,
    pub references: Vec<String>,
}

pub fn decode(format: &ObserverFormat, bytes: &str, realization: &str) -> Result<DecodedPacket, String> {
    if matches!(format, ObserverFormat::McpToolResultV1) { return crate::mcp_observer::decode(bytes, realization); }
    let packet: FinitePacket = match format {
        ObserverFormat::FinitePacketV1 => serde_json::from_str(bytes).map_err(|e| format!("invalid captured public packet: {e}"))?,
        ObserverFormat::McpToolResultV1 => unreachable!("MCP has its own independent decoder"),
    };
    if packet.realization != realization { return Err("captured packet realization mismatch".into()); }
    if packet.groups.len() > 256 || packet.groups.iter().map(|g| g.evidence.len()).sum::<usize>() > 256 || packet.references.len() > 256 { return Err("captured public packet finite bound exceeded".into()); }
    for group in &packet.groups {
        if group.context.len() > 32 || group.context.iter().any(|(key, value)| key.is_empty() || value.is_empty()) { return Err("invalid public context bindings".into()); }
        for evidence in &group.evidence {
            if evidence.qualifications.len() > 32 { return Err("public qualification bound exceeded".into()); }
            let ids: BTreeSet<_> = evidence.qualifications.iter().map(|q| &q.id).collect();
            if ids.len() != evidence.qualifications.len() { return Err("duplicate public qualification identity".into()); }
        }
    }
    Ok(DecodedPacket { tool: None, semantic_snapshot: None, database_identity: None, groups: packet.groups, references: packet.references })
}
