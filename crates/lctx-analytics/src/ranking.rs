//! Usage ranking (DESIGN §9.5; ADR-0011 and its increment-2 amendment): what official usage calls.
//!
//! - **Direct usage** (the default; the increment-2 review's U1): each subsystem function's
//!   definite calls from official usage code ([`USAGE_POLICY`]). Seed selection and the Related
//!   line order by it: the question they ask is which operations users call.
//! - **PageRank** (the `+pagerank` variant, kept for the §9.8 ablation): our own weighted power
//!   iteration over the usage projection, in canonical order, recording iterations, the final L1
//!   residual and a converged flag. It ranks what usage reaches through the library's own
//!   delegation, so implementation sinks rank high; it replaces direct usage as the order only in
//!   its variant.
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

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::RecordBatch;
use cpg_schema::codebook::{ArcKind, Codebook, CoverageStatus, FindingKind, NodeKind, SourceRole};
use cpg_schema::findings::{FINDING_STATUS, FindingsRow, recipe::FindingKey};
use cpg_schema::id::{Digest, Id, IdHasher, content_digest};
use fixedbitset::FixedBitSet;
use serde::Serialize;

use crate::AnalyticsError;
use crate::graph::Projection;

/// The usage projection's weight policy: the count of invocation arcs per ordered pair.
pub const WEIGHT_POLICY: &str = "usage-projection: invocation arcs (call and definition arcs, any \
                                 modality) into subsystem functions from subsystem functions or \
                                 official-usage callers, weight = arc count";

/// What a direct-usage count counts (the increment-2 review's U1 and F3(c)). A method call on an
/// instance is a `candidate` arc (Pysa's override marking), almost always with one target, so both
/// accepted modalities count; a site with several targets is one call, shared among them.
pub const USAGE_POLICY: &str = "direct-usage: call arcs (definite or candidate, any phase) from an \
                                official-usage caller (a function or module of an example, test \
                                or doc block); each call site counts once, split evenly among its \
                                targets; a subsystem function's count is the sum of its shares";

/// Direct usage's identity: the invocation projection's and the policy.
pub fn usage_digest(invocation: Digest) -> Digest {
    IdHasher::new("direct-usage")
        .str(&invocation.hex())
        .str(USAGE_POLICY)
        .finish_digest()
}

/// A subsystem function: the usage projection's own vertices.
fn is_function(projection: &Projection, subsystem: &FixedBitSet, i: usize) -> bool {
    projection.kinds[i] == NodeKind::Function && subsystem.contains(i)
}

/// An official-usage caller: a vertex of an example, test or doc block.
fn is_usage(projection: &Projection, i: usize) -> bool {
    matches!(
        projection.roles[i],
        Some(SourceRole::Example | SourceRole::Test | SourceRole::DocBlock)
    )
}

/// Each subsystem function's direct official-usage calls ([`USAGE_POLICY`]), for those with any.
pub fn usage_counts(projection: &Projection, subsystem: &FixedBitSet) -> BTreeMap<Id, f64> {
    // Each usage call site's targets, in canonical order.
    let mut sites: BTreeMap<Id, BTreeSet<usize>> = BTreeMap::new();
    for arc in &projection.arcs {
        if arc.arc_kind == ArcKind::Call && is_usage(projection, arc.src as usize) {
            sites
                .entry(arc.call_site)
                .or_default()
                .insert(arc.dst as usize);
        }
    }
    let mut counts: BTreeMap<Id, f64> = BTreeMap::new();
    for targets in sites.values() {
        let share = 1.0 / targets.len() as f64;
        for &d in targets {
            if is_function(projection, subsystem, d) {
                *counts.entry(projection.ids[d]).or_insert(0.0) += share;
            }
        }
    }
    counts
}

#[derive(Debug, Serialize)]
struct UsageDiagnostics {
    functions_called: usize,
    /// The call sites' shares that reach subsystem functions.
    calls: f64,
    reported: usize,
}

/// Direct usage's results: each public API's count, and its findings.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    pub counts: BTreeMap<Id, f64>,
    pub diagnostics: String,
    pub findings: Vec<FindingsRow>,
}

/// Each public API (`public`: `cpg_schema::communities::public_callables_sql`'s rows) that official
/// usage calls directly is a `direct_usage` finding whose score is its count.
pub fn run_usage(
    projection: &Projection,
    subsystem: &FixedBitSet,
    public: &[RecordBatch],
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Usage, AnalyticsError> {
    let paths = crate::communities::preferred_paths(public)?;
    let all = usage_counts(projection, subsystem);
    let status = FINDING_STATUS
        .iter()
        .find(|(k, _)| *k == FindingKind::DirectUsage)
        .map(|(_, s)| *s)
        .ok_or_else(|| AnalyticsError::Graph("no status policy for direct usage".to_owned()))?;
    let counts: BTreeMap<Id, f64> = all
        .iter()
        .filter(|(id, _)| paths.contains_key(id))
        .map(|(id, n)| (*id, *n))
        .collect();
    let findings = counts
        .iter()
        .map(|(id, n)| FindingsRow {
            snapshot_id,
            finding_id: FindingKey {
                finding_kind: FindingKind::DirectUsage.code(),
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
            .id(),
            invocation_id,
            finding_kind: FindingKind::DirectUsage,
            subject_node_id: *id,
            related_node_id: None,
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            score: Some(*n),
            condition_node_id: None,
        })
        .collect();
    let diagnostics = serde_json::to_string(&UsageDiagnostics {
        functions_called: all.len(),
        calls: all.values().sum(),
        reported: counts.len(),
    })
    .expect("diagnostics serialize");
    Ok(Usage {
        counts,
        diagnostics,
        findings,
    })
}

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
        let function = |i: usize| is_function(projection, subsystem, i);
        let usage = |i: usize| is_usage(projection, i);
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
    let paths = crate::communities::preferred_paths(public)?;
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

    use crate::graph::{Arc, InvocationGraph};
    use cpg_schema::codebook::{ArcKind, InvocationPhase, Modality};

    fn id(k: u8) -> Id {
        Id([k; 16])
    }

    /// Vertices 1–3 subsystem functions, 4 a usage caller (an example module), 5 a release
    /// function outside the subsystem; arcs as `(src, dst, call site, modality, kind)`.
    fn projection(arcs: &[(u8, u8, u8, Modality, ArcKind)]) -> (Projection, FixedBitSet) {
        let ids: Vec<Id> = (1..=5).map(id).collect();
        let dense = |k: u8| u32::from(k - 1);
        let p = Projection {
            kinds: vec![
                NodeKind::Function,
                NodeKind::Function,
                NodeKind::Function,
                NodeKind::Module,
                NodeKind::Function,
            ],
            modules: vec![None; 5],
            roles: vec![
                Some(SourceRole::Release),
                Some(SourceRole::Release),
                Some(SourceRole::Release),
                Some(SourceRole::Example),
                Some(SourceRole::Release),
            ],
            arcs: arcs
                .iter()
                .map(|&(s, d, site, modality, arc_kind)| Arc {
                    src: dense(s),
                    dst: dense(d),
                    call_site: id(site),
                    edge_id: id(site.wrapping_add(100)),
                    phase: (arc_kind == ArcKind::Call).then_some(InvocationPhase::Call),
                    modality,
                    has_unresolved_remainder: false,
                    arc_kind,
                })
                .collect(),
            unresolved: Vec::new(),
            graph: InvocationGraph::default(),
            ids,
        };
        let mut subsystem = FixedBitSet::with_capacity(5);
        for i in 0..3 {
            subsystem.insert(i);
        }
        (p, subsystem)
    }

    const USAGE: &[(u8, u8, u8, Modality, ArcKind)] = &[
        // The example calls 1 at two sites, and at a third either 1 or 3 (a site with two
        // candidate targets); it calls 2 once, a candidate.
        (4, 1, 41, Modality::Definite, ArcKind::Call),
        (4, 1, 42, Modality::Definite, ArcKind::Call),
        (4, 1, 45, Modality::Candidate, ArcKind::Call),
        (4, 2, 43, Modality::Candidate, ArcKind::Call),
        (4, 3, 45, Modality::Candidate, ArcKind::Call),
        // 1 delegates to 2 and 3; 3 to 2; 2 calls outside the subsystem.
        (1, 2, 11, Modality::Definite, ArcKind::Call),
        (1, 3, 12, Modality::Definite, ArcKind::Call),
        (3, 2, 31, Modality::Definite, ArcKind::Call),
        (2, 5, 21, Modality::Definite, ArcKind::Call),
        // A definition arc from the example counts in the usage projection, never as a call.
        (4, 3, 44, Modality::Definite, ArcKind::Definition),
    ];

    /// The increment-2 review's U1 and F6(a): direct usage counts the usage caller's calls, each
    /// site once and shared among its targets, and so ranks the API usage calls above the sink
    /// its delegation reaches, where PageRank over the same projection ranks the sink first.
    #[test]
    fn direct_usage_ranks_what_usage_calls_above_its_delegation_sink() {
        let (p, subsystem) = projection(USAGE);
        let counts = usage_counts(&p, &subsystem);
        assert_eq!(
            counts,
            [(id(1), 2.5), (id(2), 1.0), (id(3), 0.5)]
                .into_iter()
                .collect()
        );
        let graph = UsageGraph::build(&p, &subsystem);
        assert_eq!(graph.functions, vec![true, true, true, false]);
        let ranks = pagerank(graph.vertices.len(), &graph.arcs, &Params::preregistered());
        assert!(ranks.scores[1] > ranks.scores[0], "{:?}", ranks.scores);
    }

    /// The increment-2 review's F6(b): the projection's arcs in another order give the same usage
    /// graph, scores and counts.
    #[test]
    fn a_permuted_projection_gives_identical_scores() {
        let (a, subsystem) = projection(USAGE);
        let mut reversed = USAGE.to_vec();
        reversed.reverse();
        let (b, _) = projection(&reversed);
        let (ga, gb) = (
            UsageGraph::build(&a, &subsystem),
            UsageGraph::build(&b, &subsystem),
        );
        assert_eq!(ga, gb);
        let p = Params::preregistered();
        assert_eq!(
            pagerank(ga.vertices.len(), &ga.arcs, &p),
            pagerank(gb.vertices.len(), &gb.arcs, &p)
        );
        assert_eq!(usage_counts(&a, &subsystem), usage_counts(&b, &subsystem));
    }

    /// The increment-2 review's probe 2 (F6(d)): `compute_flow` agrees on 30 random weighted
    /// digraphs of 5–44 vertices, about a quarter of them dangling. The kernel is run to its
    /// tolerance here (a slowly mixing graph can need more than the pre-registered 100 steps,
    /// which a compile records as `partial`).
    #[test]
    fn compute_flow_agrees_on_random_graphs() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move |bound: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % bound
        };
        let p = Params {
            max_iterations: 1000,
            ..Params::preregistered()
        };
        for _ in 0..30 {
            let n = 5 + next(40) as usize;
            let mut rows: BTreeMap<(u32, u32), u64> = BTreeMap::new();
            for s in 0..n {
                if next(4) == 0 {
                    continue;
                }
                for _ in 0..1 + next(4) {
                    let d = next(n as u64) as usize;
                    if d != s {
                        *rows.entry((s as u32, d as u32)).or_insert(0) += 1 + next(5);
                    }
                }
            }
            let ours = pagerank(n, &rows, &p);
            let mut builder = GraphDataBuilder::new(n).directed();
            for (&(s, d), &w) in &rows {
                builder.add_edge(s as usize, d as usize, w as f64).unwrap();
            }
            let flow = leiden_rs::infomap::compute_flow(
                &builder.build().unwrap(),
                1.0 - p.damping,
                1e-14,
                1000,
            );
            assert!(ours.converged);
            for (a, b) in ours.scores.iter().zip(&flow) {
                assert!((a - b.flow).abs() < 1e-9, "n = {n}");
            }
        }
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
