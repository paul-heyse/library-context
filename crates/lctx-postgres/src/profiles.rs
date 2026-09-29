//! Versioned retrieval policy, separate from canonical vectors and generation identity.
use crate::Error;
use cpg_schema::serving_projection::corrupt;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Exact,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub format: u32,
    pub route: Route,
    pub metric: String,
    pub ties: String,
    pub rrf_k: u32,
}
impl Policy {
    pub fn exact() -> Self {
        Self {
            format: 3,
            route: Route::Exact,
            metric: "pgvector-cosine-f32-v1".into(),
            ties: "member-unit-fragment-ascending;family-rrf-v1;promotion-primary".into(),
            rrf_k: 60,
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self != &Self::exact() {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_current_exact_policy_is_admitted() {
        let mut policy = Policy::exact();
        policy.validate().unwrap();
        assert!(
            serde_json::from_str::<Policy>(&policy.canonical().unwrap().replace("exact", "hnsw"))
                .is_err()
        );
        policy.format = 2;
        assert!(policy.validate().is_err());
    }
}
