//! One fixed analytic policy supplies persisted settings, kernel bindings and definition identity.
use crate::domain::{
    FiniteF64, ModelError,
    analysis::{AnalysisMethod, MethodParameters},
};

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct RetainedPolicy {
    pub depth: i64,
    pub max_work: u64,
    pub damping: f64,
    pub tolerance: f64,
    pub iterations: usize,
    pub resolutions: [f64; 4],
    pub seeds: u64,
    pub selected_resolution: f64,
    pub hub_percentile: f64,
    pub concept_support: usize,
    pub concept_bound: usize,
    pub neighbour_floor: f64,
    pub neighbours: usize,
    pub mechanics: &'static str,
}
pub const RETAINED: RetainedPolicy = RetainedPolicy {
    depth: 256,
    max_work: 100_000_000,
    damping: 0.85,
    tolerance: 1e-10,
    iterations: 100,
    resolutions: [0.5, 1.0, 2.0, 4.0],
    seeds: 10,
    selected_resolution: 1.0,
    hub_percentile: 0.95,
    concept_support: 2,
    concept_bound: 20_000,
    neighbour_floor: 0.5,
    neighbours: 3,
    mechanics: "petgraph0.8.3/leiden0.8.1/fixedbitset0.5.7;directed-count-rank;RBER-quality-history;undirected-equal-retained-layers;canonical-accumulation;one-RCA-step;partition-largest-majority-or-less-than-three-degenerate",
};
impl RetainedPolicy {
    pub fn parameters(self, method: AnalysisMethod) -> Result<MethodParameters, ModelError> {
        Ok(MethodParameters {
            depth: Some(self.depth),
            proof_steps: matches!(
                method,
                AnalysisMethod::Concepts | AnalysisMethod::RelationalConcepts
            )
            .then_some(self.concept_bound as i64),
            work: Some(self.max_work as i64),
            members: None,
            seed: (method == AnalysisMethod::Communities).then_some(0),
            iterations: matches!(
                method,
                AnalysisMethod::PageRank | AnalysisMethod::Communities
            )
            .then_some(self.iterations as i64),
            threshold: Some(FiniteF64::new(if method == AnalysisMethod::Neighbours {
                self.neighbour_floor
            } else {
                self.tolerance
            })?),
            resolution: (method == AnalysisMethod::Communities)
                .then_some(FiniteF64::new(self.selected_resolution)?),
            damping: (method == AnalysisMethod::PageRank).then_some(FiniteF64::new(self.damping)?),
            model_catalog: None,
        })
    }
    pub fn ranking(self) -> Result<super::ranking::Parameters, ModelError> {
        Ok(super::ranking::Parameters {
            damping: FiniteF64::new(self.damping)?,
            tolerance: FiniteF64::new(self.tolerance)?,
            max_iterations: self.iterations as u64,
            max_work: self.max_work,
        })
    }
    pub fn community_work_multiplier(self) -> Option<u64> {
        (self.resolutions.len() as u64)
            .checked_mul(self.seeds)?
            .checked_mul(self.iterations as u64)
    }
    /// Generated from the same complete structured policy, not independently authored recipe text.
    pub fn recipe(self) -> Result<String, ModelError> {
        serde_json::to_string(&self).map_err(ModelError::codec)
    }
}
