//! Versioned retrieval policy, separate from canonical vectors and generation identity.
use crate::Error;
use cpg_schema::serving_projection::corrupt;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Exact,
    Hnsw,
    Mixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub format: u32,
    pub route: Route,
    pub metric: String,
    pub ties: String,
    pub rrf_k: u32,
    pub hnsw: Option<Hnsw>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<Routing>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Routing {
    pub count_floor: u32,
    pub classes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
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
            routing: None,
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
    pub fn mixed(count_floor: u32, classes: Vec<String>) -> Self {
        Self {
            format: 2,
            route: Route::Mixed,
            routing: Some(Routing {
                count_floor,
                classes,
            }),
            ..Self::hnsw()
        }
    }
    pub fn choose(&self, operations: bool, eligible: u64, universe: u64) -> (Route, &'static str) {
        let Some(routing) = &self.routing else {
            return (self.route, "explicit_profile");
        };
        if !operations {
            return (Route::Exact, "briefs_exact");
        }
        if eligible <= u64::from(routing.count_floor) {
            return (Route::Exact, "small_eligible_set");
        }
        if universe == 0 || eligible.saturating_mul(10) <= universe {
            return (Route::Exact, "selective_exact");
        }
        let class = if eligible == universe {
            "unfiltered"
        } else {
            "broad"
        };
        if routing.classes.iter().any(|s| s == class) {
            (Route::Hnsw, class)
        } else {
            (Route::Exact, "class_not_admitted")
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        let mixed = self.routing.as_ref().is_some_and(|r| {
            matches!(r.count_floor, 1024 | 4096)
                && !r.classes.is_empty()
                && r.classes
                    .iter()
                    .all(|c| matches!(c.as_str(), "broad" | "unfiltered"))
                && r.classes.windows(2).all(|w| w[0] < w[1])
                && self == &Self::mixed(r.count_floor, r.classes.clone())
        });
        if self != &Self::exact() && self != &Self::hnsw() && !mixed {
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
    fn exact_identity_and_mixed_boundaries() {
        assert_eq!(
            Policy::exact().canonical().unwrap(),
            r#"{"format":1,"route":"exact","metric":"pgvector-cosine-f32-v1","ties":"entity-id-ascending","rrf_k":60,"hnsw":null}"#
        );
        let p = Policy::mixed(1024, vec!["broad".into(), "unfiltered".into()]);
        p.validate().unwrap();
        assert_eq!(p.choose(false, 5000, 5000).0, Route::Exact);
        assert_eq!(p.choose(true, 1024, 1024).0, Route::Exact);
        assert_eq!(p.choose(true, 1025, 10250).0, Route::Exact);
        assert_eq!(p.choose(true, 1025, 10249).0, Route::Hnsw);
        assert_eq!(p.choose(true, 1025, 1025).0, Route::Hnsw);
        assert_eq!(
            Policy::mixed(1024, vec!["unfiltered".into()])
                .choose(true, 2048, 4096)
                .0,
            Route::Exact
        );
        assert!(Policy::mixed(7, vec!["broad".into()]).validate().is_err());
    }
}
