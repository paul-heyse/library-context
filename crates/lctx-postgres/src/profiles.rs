//! Versioned retrieval policy, separate from canonical vectors and generation identity.
use crate::Error;
use cpg_schema::serving_projection::corrupt;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Exact,
    Hnsw,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub format: u32,
    pub route: Route,
    pub metric: String,
    pub ties: String,
    pub rrf_k: u32,
    pub hnsw: Option<Hnsw>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hnsw {
    pub m: u32,
    pub ef_construction: u32,
    pub ef_search: u32,
    pub iterative_scan: String,
    pub max_scan_tuples: u32,
    pub work_mem_kib: u32,
    pub scan_mem_multiplier: u32,
    pub depths: Vec<u32>,
    pub minimum_recall_basis_points: u32,
}
impl Policy {
    pub fn exact() -> Self {
        Self {
            format: 1,
            route: Route::Exact,
            metric: "pgvector-cosine-f32-v1".into(),
            ties: "entity-id-ascending".into(),
            rrf_k: 60,
            hnsw: None,
        }
    }
    pub fn hnsw() -> Self {
        Self {
            route: Route::Hnsw,
            hnsw: Some(Hnsw {
                m: 16,
                ef_construction: 128,
                ef_search: 100,
                iterative_scan: "strict_order".into(),
                max_scan_tuples: 20000,
                work_mem_kib: 8192,
                scan_mem_multiplier: 2,
                depths: vec![200, 800, 3200],
                minimum_recall_basis_points: 9900,
            }),
            ..Self::exact()
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self != &Self::exact() && self != &Self::hnsw() {
            return Err(corrupt("unsupported retrieval policy").into());
        }
        Ok(())
    }
    pub fn canonical(&self) -> Result<String, Error> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| corrupt("profile encoding").into())
    }
    pub fn digest(&self) -> Result<String, Error> {
        Ok(hex(Sha256::digest(self.canonical()?.as_bytes())))
    }
}
pub(crate) fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
