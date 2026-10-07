//! Independent capture decoder. It receives exact emitted bytes, never a task/oracle/map.
//! The finite packet is a supported fixture format, not an MCP normalization/compatibility route.
use std::collections::BTreeSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::contracts::{Assignment, CandidateStatus, ObserverFormat};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicQualification { pub id: String, pub text: String }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicEvidence {
    pub anchor: String,
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

pub fn decode(format: &ObserverFormat, bytes: &str, realization: &str) -> Result<FinitePacket, String> {
    let packet: FinitePacket = match format {
        ObserverFormat::FinitePacketV1 => serde_json::from_str(bytes).map_err(|e| format!("invalid captured public packet: {e}"))?,
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
    Ok(packet)
}
