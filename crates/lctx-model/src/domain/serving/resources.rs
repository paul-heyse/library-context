//! Shared admission defaults; these allowances do not claim a measured RSS bound.
use crate::domain::ModelError;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub query_connections: u32,
    pub cpu_jobs: u32,
    pub request_deadline_ms: u64,
    pub admission_wait_ms: u64,
    pub shared_bytes: u64,
    pub preparation_bytes: u64,
    pub request_bytes: u64,
    pub default_page_rows: u32,
    pub maximum_page_rows: u32,
    pub maximum_requirements: u32,
    pub maximum_comparison_candidates: u32,
    pub default_response_bytes: u64,
    pub expanded_response_bytes: u64,
    pub explanation_depth: u32,
    pub explanation_nodes: u32,
    pub explanation_edges: u32,
}
impl Default for ResourceLimits {
    fn default() -> Self { Self {
        query_connections: 2, cpu_jobs: 2, request_deadline_ms: 30_000,
        admission_wait_ms: 1_000, shared_bytes: 256 * 1024 * 1024,
        preparation_bytes: 128 * 1024 * 1024, request_bytes: 64 * 1024 * 1024,
        default_page_rows: 20, maximum_page_rows: 100, maximum_requirements: 16,
        maximum_comparison_candidates: 5, default_response_bytes: 32 * 1024,
        expanded_response_bytes: 256 * 1024, explanation_depth: 8,
        explanation_nodes: 256, explanation_edges: 1024,
    } }
}
impl ResourceLimits {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.query_connections == 0 || self.cpu_jobs == 0 || self.request_deadline_ms == 0
            || self.admission_wait_ms > self.request_deadline_ms || self.shared_bytes == 0
            || self.preparation_bytes == 0 || self.preparation_bytes > self.shared_bytes
            || self.request_bytes == 0 || self.request_bytes > self.shared_bytes
            || self.default_page_rows == 0 || self.default_page_rows > self.maximum_page_rows
            || self.maximum_requirements == 0 || self.maximum_comparison_candidates == 0
            || self.default_response_bytes == 0 || self.default_response_bytes > self.expanded_response_bytes
            || self.explanation_depth == 0 || self.explanation_nodes == 0 || self.explanation_edges == 0
        { return Err(ModelError::Invalid("invalid serving resource allowances".into())); }
        Ok(())
    }
}
impl ResourceLimits {
    pub fn response_bytes(&self,expanded:bool)->u64 {
        if expanded {self.expanded_response_bytes} else {self.default_response_bytes}
    }
}
/// Admit the actual final serialized MCP envelope, including text/structured duplication.
/// This operates on complete bytes and never edits or truncates structured content.
pub fn admit_final_mcp_bytes(bytes:&[u8],expanded:bool,limits:&ResourceLimits)->Result<(),super::WireError>{
    limits.validate().map_err(|e|super::WireError::Invalid(e.to_string()))?;
    if bytes.len() as u64>limits.response_bytes(expanded) {
        return Err(super::WireError::ResourceRefused("final serialized MCP bytes".into()));
    }Ok(())
}
