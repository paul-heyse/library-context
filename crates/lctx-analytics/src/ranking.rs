//! Centrality (DESIGN §9.5; ADR-0011): our own weighted PageRank over the usage projection, in
//! canonical order, recording iterations, the final L1 residual and a converged flag.
//!
//! **The usage projection** (H1 review F9) is the invocation projection (§5) restricted and
//! weighted by a named policy ([`WEIGHT_POLICY`]):
//! - vertices: every subsystem function, and every official-usage caller (a function or module
//!   of an example, test or doc block) with an arc into one;
//! - arcs: each invocation arc into a subsystem function from a subsystem function or a usage
//!   caller (a self-arc aside), weighted by how many such arcs join the ordered pair.
//!
//! So a function ranks by the official usage that reaches it, directly or through the library's
//! own delegation. The teleport and the dangling target are uniform, which makes
//! `leiden_rs::infomap::compute_flow` (teleport rate = 1 − damping) a reference oracle.

use std::collections::BTreeMap;

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, StringArray};
use cpg_schema::codebook::{Codebook, CoverageStatus, FindingKind, NodeKind, SourceRole};
use cpg_schema::findings::{FINDING_STATUS, FindingsRow, recipe::FindingKey};
use cpg_schema::id::{Digest, Id, IdHasher, content_digest};
use fixedbitset::FixedBitSet;
use serde::Serialize;

use crate::AnalyticsError;
use crate::graph::Projection;

/// The usage projection's weight policy: the count of invocation arcs per ordered pair.
pub const WEIGHT_POLICY: &str = "usage-projection: invocation arcs into subsystem functions from \
                                 subsystem functions or official-usage callers, weight = arc count";

/// The usage projection's identity: the invocation projection's and the policy.
pub fn projection_digest(invocation: Digest) -> Digest {
    IdHasher::new("usage-projection")
        .str(&invocation.hex())
        .str(WEIGHT_POLICY)
        .finish_digest()
}

/// The pre-registered parameters (slice 2.4; deviation log D29).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    pub damping: f64,
    /// Converged when the L1 change of one step falls below this.
    pub tolerance: f64,
    pub max_iterations: usize,
}

impl Params {
    pub fn preregistered() -> Self {
        Params {
            damping: 0.85,
            tolerance: 1e-10,
            max_iterations: 100,
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("parameters serialize")
    }

    pub fn digest(&self) -> Digest {
        content_digest(self.json().as_bytes())
    }
}

/// A directed weighted graph over a dense index of sorted ids.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageGraph {
    pub vertices: Vec<Id>,
    /// Which vertices are subsystem functions (the rest are usage callers).
    pub functions: Vec<bool>,
    /// Arc weights per ordered pair of dense indices.
    pub arcs: BTreeMap<(u32, u32), u64>,
}

impl UsageGraph {
    /// The usage projection from the invocation projection and the subsystem mask.
    pub fn build(projection: &Projection, subsystem: &FixedBitSet) -> Self {
        let function =
            |i: usize| projection.kinds[i] == NodeKind::Function && subsystem.contains(i);
        let usage = |i: usize| {
            matches!(
                projection.roles[i],
                Some(SourceRole::Example | SourceRole::Test | SourceRole::DocBlock)
            )
        };
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for arc in &projection.arcs {
            let (s, d) = (arc.src as usize, arc.dst as usize);
            if s != d && function(d) && (function(s) || usage(s)) {
                pairs.push((s, d));
            }
        }
        // Projection indices are already sorted by id: keep that order as the dense index.
        let mut keep = FixedBitSet::with_capacity(projection.ids.len());
        for i in 0..projection.ids.len() {
            if function(i) {
                keep.insert(i);
            }
        }
        for &(s, _) in &pairs {
            keep.insert(s);
        }
        let dense: BTreeMap<usize, u32> = keep
            .ones()
            .enumerate()
            .map(|(k, i)| (i, k as u32))
            .collect();
        let mut out = UsageGraph {
            vertices: keep.ones().map(|i| projection.ids[i]).collect(),
            functions: keep.ones().map(function).collect(),
            arcs: BTreeMap::new(),
        };
        for (s, d) in pairs {
            *out.arcs.entry((dense[&s], dense[&d])).or_insert(0) += 1;
        }
        out
    }
}

/// A power iteration's result.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranks {
    pub scores: Vec<f64>,
    pub iterations: i64,
    /// The L1 change of the last step.
    pub residual: f64,
    pub converged: bool,
}

/// Weighted PageRank over `n` vertices: `r'ⱼ = (1−d)/n + d·(Σᵢ rᵢ·wᵢⱼ/Wᵢ + D/n)`, where `D` is the
/// mass on vertices with no out-arcs, from the uniform start, in canonical arc order.
pub fn pagerank(n: usize, arcs: &BTreeMap<(u32, u32), u64>, params: &Params) -> Ranks {
    if n == 0 {
        return Ranks {
            scores: Vec::new(),
            iterations: 0,
            residual: 0.0,
            converged: true,
        };
    }
    let mut out_weight = vec![0u64; n];
    for (&(s, _), &w) in arcs {
        out_weight[s as usize] += w;
    }
    let uniform = 1.0 / n as f64;
    let mut scores = vec![uniform; n];
    let (mut iterations, mut residual, mut converged) = (0i64, f64::INFINITY, false);
    while (iterations as usize) < params.max_iterations {
        let dangling: f64 = scores
            .iter()
            .zip(&out_weight)
            .filter(|(_, w)| **w == 0)
            .map(|(r, _)| *r)
            .sum();
        let mut flow = vec![0.0f64; n];
        for (&(s, d), &w) in arcs {
            flow[d as usize] += scores[s as usize] * w as f64 / out_weight[s as usize] as f64;
        }
        let base = (1.0 - params.damping) * uniform + params.damping * dangling * uniform;
        let next: Vec<f64> = flow.iter().map(|f| base + params.damping * f).collect();
        residual = next.iter().zip(&scores).map(|(a, b)| (a - b).abs()).sum();
        scores = next;
        iterations += 1;
        if residual < params.tolerance {
            converged = true;
            break;
        }
    }
    Ranks {
        scores,
        iterations,
        residual,
        converged,
    }
}

#[derive(Debug, Serialize)]
struct Diagnostics {
    vertices: usize,
    subsystem_functions: usize,
    usage_callers: usize,
    weighted_arcs: usize,
    arc_weight: u64,
    dangling: usize,
    reported: usize,
}

/// The ranking's results.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub ranks: Ranks,
    pub vertices: usize,
    pub arcs: usize,
    pub completion: CoverageStatus,
    pub diagnostics: String,
    pub findings: Vec<FindingsRow>,
}

/// Rank the usage graph; each public API among its functions is a `centrality` finding whose
/// score is its rank (`public`: `cpg_schema::communities::public_callables_sql`'s rows).
pub fn run(
    graph: &UsageGraph,
    public: &[RecordBatch],
    params: &Params,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Outcome, AnalyticsError> {
    let mut paths: BTreeMap<Id, String> = BTreeMap::new();
    for b in public {
        let node = b
            .column_by_name("node_id")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
            .ok_or_else(|| AnalyticsError::Column("node_id".to_owned()))?;
        let path = b
            .column_by_name("access_path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| AnalyticsError::Column("access_path".to_owned()))?;
        for i in 0..b.num_rows() {
            paths
                .entry(Id(<[u8; 16]>::try_from(node.value(i)).expect("16 bytes")))
                .or_insert_with(|| path.value(i).to_owned());
        }
    }
    let n = graph.vertices.len();
    let ranks = pagerank(n, &graph.arcs, params);
    let status = FINDING_STATUS
        .iter()
        .find(|(k, _)| *k == FindingKind::Centrality)
        .map(|(_, s)| *s)
        .ok_or_else(|| AnalyticsError::Graph("no status policy for centrality".to_owned()))?;
    let mut findings = Vec::new();
    for (v, id) in graph.vertices.iter().enumerate() {
        if !graph.functions[v] || !paths.contains_key(id) {
            continue;
        }
        let finding_id = FindingKey {
            finding_kind: FindingKind::Centrality.code(),
            subject: *id,
            related: None,
            condition: None,
            evidence_status: status.code(),
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            paths: &[],
            members: &[],
        }
        .id();
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: FindingKind::Centrality,
            subject_node_id: *id,
            related_node_id: None,
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            score: Some(ranks.scores[v]),
            condition_node_id: None,
        });
    }
    let mut has_out = vec![false; n];
    for &(s, _) in graph.arcs.keys() {
        has_out[s as usize] = true;
    }
    let diagnostics = serde_json::to_string(&Diagnostics {
        vertices: n,
        subsystem_functions: graph.functions.iter().filter(|f| **f).count(),
        usage_callers: graph.functions.iter().filter(|f| !**f).count(),
        weighted_arcs: graph.arcs.len(),
        arc_weight: graph.arcs.values().sum(),
        dangling: has_out.iter().filter(|h| !**h).count(),
        reported: findings.len(),
    })
    .expect("diagnostics serialize");
    Ok(Outcome {
        completion: if ranks.converged {
            CoverageStatus::CompleteUnderStatedModel
        } else {
            CoverageStatus::Partial
        },
        vertices: n,
        arcs: graph.arcs.len(),
        ranks,
        diagnostics,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use leiden_rs::GraphDataBuilder;

    fn arcs(list: &[(u32, u32, u64)]) -> BTreeMap<(u32, u32), u64> {
        let mut m = BTreeMap::new();
        for &(s, d, w) in list {
            *m.entry((s, d)).or_insert(0) += w;
        }
        m
    }

    /// A hand-computed fixture: 0 → 1, 0 → 2, 1 → 2, and 2 dangling, at d = 0.85.
    ///
    /// With `b = 0.05 + 0.85·r₂/3`: r₀ = b, r₁ = b + 0.425·r₀, r₂ = b + 0.425·r₀ + 0.85·r₁, and
    /// the scores sum to one, which solves to r₀ ≈ 0.1976, r₁ ≈ 0.2816, r₂ ≈ 0.5209.
    #[test]
    fn a_three_node_graph_matches_its_hand_computation() {
        let r = pagerank(
            3,
            &arcs(&[(0, 1, 1), (0, 2, 1), (1, 2, 1)]),
            &Params::preregistered(),
        );
        assert!(r.converged && r.residual < 1e-10, "{r:?}");
        // Solve the fixed point exactly.
        let (a, b0) = (0.425, 0.85);
        // r0 = b; r1 = b(1 + a); r2 = b(1 + a) + b0·b(1 + a) = b(1 + a)(1 + b0).
        // b = 0.05 + 0.85/3 · r2, and r0 + r1 + r2 = 1.
        let k = 1.0 + (1.0 + a) + (1.0 + a) * (1.0 + b0);
        let b = 1.0 / k;
        let expected = [b, b * (1.0 + a), b * (1.0 + a) * (1.0 + b0)];
        for (got, want) in r.scores.iter().zip(expected) {
            assert!((got - want).abs() < 1e-9, "{:?} vs {expected:?}", r.scores);
        }
        assert!((0.05 + 0.85 / 3.0 * expected[2] - b).abs() < 1e-12);
    }

    /// Two parallel arcs weigh twice one: 0 ⇉ 1 and 0 → 2 split 0's rank 2 : 1.
    #[test]
    fn parallel_arcs_count_with_their_weights() {
        let twice = pagerank(
            3,
            &arcs(&[(0, 1, 1), (0, 1, 1), (0, 2, 1)]),
            &Params::preregistered(),
        );
        let once = pagerank(3, &arcs(&[(0, 1, 2), (0, 2, 1)]), &Params::preregistered());
        assert_eq!(twice, once);
        assert!(twice.scores[1] > twice.scores[2]);
    }

    #[test]
    fn a_budget_too_small_to_converge_is_reported() {
        let params = Params {
            max_iterations: 2,
            ..Params::preregistered()
        };
        let r = pagerank(
            3,
            &arcs(&[(0, 1, 1), (1, 2, 1), (2, 0, 1), (0, 2, 5)]),
            &params,
        );
        assert!(
            !r.converged && r.iterations == 2 && r.residual > params.tolerance,
            "{r:?}"
        );
    }

    #[test]
    fn shuffled_rows_give_identical_scores() {
        let rows = [
            (3, 1, 2),
            (0, 1, 1),
            (2, 3, 1),
            (1, 0, 4),
            (0, 3, 1),
            (2, 1, 1),
        ];
        let mut reversed = rows;
        reversed.reverse();
        let p = Params::preregistered();
        assert_eq!(
            pagerank(4, &arcs(&rows), &p),
            pagerank(4, &arcs(&reversed), &p)
        );
    }

    /// ADR-0011's oracle: `compute_flow` (uniform teleport and dangling target) agrees.
    #[test]
    fn compute_flow_agrees() {
        let rows = arcs(&[
            (0, 1, 3),
            (0, 2, 1),
            (1, 2, 2),
            (2, 0, 1),
            (3, 2, 1),
            (4, 0, 2),
            (4, 3, 1),
        ]);
        let p = Params::preregistered();
        let ours = pagerank(6, &rows, &p);
        let mut builder = GraphDataBuilder::new(6).directed();
        for (&(s, d), &w) in &rows {
            builder.add_edge(s as usize, d as usize, w as f64).unwrap();
        }
        let flow = leiden_rs::infomap::compute_flow(
            &builder.build().unwrap(),
            1.0 - p.damping,
            1e-14,
            1000,
        );
        for (a, b) in ours.scores.iter().zip(&flow) {
            assert!((a - b.flow).abs() < 1e-9, "{:?} vs {flow:?}", ours.scores);
        }
    }
}
