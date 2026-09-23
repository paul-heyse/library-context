//! Communities (DESIGN §9.4; ADR-0011): Leiden (leiden-rs, RBER) over the invocation and co-use
//! layers of the subsystem's functions, at a pre-registered resolution grid and seed set, with
//! seed-consensus stability; the stable communities are projected onto public APIs.
//!
//! **Input normal form.** Each layer is integer counts per unordered pair `(min, max)` over the
//! dense index (the sorted ids of the subsystem functions some pair touches), kept in a
//! `BTreeMap`, so shuffled or flipped input gives identical counts. Each pair keeps its least
//! contributing site (a call site, or a usage scope) as lineage (H1 review F9).
//!
//! **Weights,** in canonical pair order: a hub's pairs are down-weighted (each end whose strength
//! exceeds the layer's strength percentile scales the count by threshold / strength); each layer
//! is normalized to unit total weight; the layers are summed with their pre-registered weights.
//!
//! **Consensus.** Every resolution γ of the grid runs every seed. A resolution is degenerate when
//! its seed-0 partition puts more than half the vertices in one community, or has no community of
//! three. Among the others, the one with the highest mean pairwise ARI is chosen, ties going to
//! the γ nearest 1 (RBER's own scale), then the smaller. Each community of the chosen seed-0
//! partition has an agreement: its best Jaccard match in each other seed's partition, averaged.
//! A community with enough public members and agreement is a `community` finding
//! (`statistically_derived`); everything measured is in the consensus's diagnostics.

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, StringArray};
use cpg_schema::codebook::{Codebook, CoverageStatus, FindingKind, MemberRole, NodeKind};
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, recipe::FindingKey,
};
use cpg_schema::id::{Digest, Id, content_digest};
use fixedbitset::FixedBitSet;
use leiden_rs::{GraphDataBuilder, Leiden, LeidenConfig, QualityType};
use serde::Serialize;

use crate::AnalyticsError;
use crate::graph::Projection;

/// The pre-registered parameters (slice 2.3; deviation log D28): code, not the frozen analytics
/// config, recorded in every invocation's parameters and in the compiler digest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    /// The RBER resolutions tried.
    pub gammas: Vec<f64>,
    /// Seeds `0..seeds` at every resolution.
    pub seeds: u64,
    pub invocation_weight: f64,
    pub co_use_weight: f64,
    /// A pair end whose strength exceeds this percentile of its layer's strengths is a hub.
    pub hub_percentile: f64,
    pub max_iterations: usize,
    pub epsilon: f64,
    /// A community's least agreement to be reported.
    pub min_agreement: f64,
    /// A community's least public members to be reported.
    pub min_public_members: usize,
    /// The strongest pairs a community cites.
    pub max_supporting: usize,
}

impl Params {
    /// The parameters committed before any community output existed.
    pub fn preregistered() -> Self {
        Params {
            gammas: vec![0.5, 1.0, 2.0, 4.0],
            seeds: 10,
            invocation_weight: 0.5,
            co_use_weight: 0.5,
            hub_percentile: 0.95,
            max_iterations: 100,
            epsilon: 1e-10,
            min_agreement: 0.5,
            min_public_members: 2,
            max_supporting: 3,
        }
    }

    /// Canonical JSON (fields in declaration order, no whitespace).
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("parameters serialize")
    }

    pub fn digest(&self) -> Digest {
        content_digest(self.json().as_bytes())
    }
}

/// One layer: integer counts per unordered pair of dense indices, each with its least site.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layer {
    pub counts: BTreeMap<(u32, u32), (u64, Id)>,
}

impl Layer {
    /// Count one contribution to the pair `{a, b}` (a self-pair counts nothing).
    pub fn add(&mut self, a: u32, b: u32, site: Id) {
        if a == b {
            return;
        }
        let entry = self.counts.entry((a.min(b), a.max(b))).or_insert((0, site));
        entry.0 += 1;
        entry.1 = entry.1.min(site);
    }

    /// The layer's weights after hub down-weighting, normalized to unit total, and its hub
    /// threshold (0 for an empty layer).
    pub fn weights(&self, n: usize, percentile: f64) -> (BTreeMap<(u32, u32), f64>, f64) {
        let mut strength = vec![0u64; n];
        for (&(a, b), &(c, _)) in &self.counts {
            strength[a as usize] += c;
            strength[b as usize] += c;
        }
        let mut positive: Vec<u64> = strength.iter().copied().filter(|s| *s > 0).collect();
        if positive.is_empty() {
            return (BTreeMap::new(), 0.0);
        }
        positive.sort_unstable();
        let rank = ((percentile * positive.len() as f64).ceil() as usize).clamp(1, positive.len());
        let threshold = positive[rank - 1] as f64;
        let scale = |v: u32| (threshold / strength[v as usize] as f64).min(1.0);
        let mut weights: BTreeMap<(u32, u32), f64> = self
            .counts
            .iter()
            .map(|(&(a, b), &(c, _))| ((a, b), c as f64 * scale(a) * scale(b)))
            .collect();
        let total: f64 = weights.values().sum();
        for w in weights.values_mut() {
            *w /= total;
        }
        (weights, threshold)
    }
}

/// The layers over one dense index, and what a community reports.
#[derive(Debug, Clone, Default)]
pub struct Input {
    /// The dense index: the sorted ids of the subsystem functions some pair touches.
    pub vertices: Vec<Id>,
    pub invocation: Layer,
    pub co_use: Layer,
    /// The public APIs among all subsystem functions, with their access paths.
    pub public: BTreeMap<Id, String>,
}

fn ids<'a>(b: &'a RecordBatch, name: &str) -> Result<&'a FixedSizeBinaryArray, AnalyticsError> {
    b.column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or_else(|| AnalyticsError::Column(name.to_owned()))
}

fn id_at(a: &FixedSizeBinaryArray, i: usize) -> Id {
    Id(<[u8; 16]>::try_from(a.value(i)).expect("16 bytes"))
}

impl Input {
    /// The layers from the invocation projection (its arcs between two subsystem functions),
    /// the co-use occurrences and the public callables (`cpg_schema::communities`).
    pub fn build(
        projection: &Projection,
        subsystem: &FixedBitSet,
        co_use: &[RecordBatch],
        public: &[RecordBatch],
    ) -> Result<Self, AnalyticsError> {
        let function =
            |i: usize| projection.kinds[i] == NodeKind::Function && subsystem.contains(i);
        // Raw pairs by id, each with its site, per layer.
        let mut invocation: Vec<(Id, Id, Id)> = Vec::new();
        for arc in &projection.arcs {
            let (s, d) = (arc.src as usize, arc.dst as usize);
            if s != d && function(s) && function(d) {
                invocation.push((projection.ids[s], projection.ids[d], arc.call_site));
            }
        }
        let mut scopes: BTreeMap<Id, BTreeSet<Id>> = BTreeMap::new();
        for b in co_use {
            let (scope, target) = (ids(b, "scope_node_id")?, ids(b, "target_node_id")?);
            for i in 0..b.num_rows() {
                let t = id_at(target, i);
                if projection.dense(t).is_some_and(|d| function(d as usize)) {
                    scopes.entry(id_at(scope, i)).or_default().insert(t);
                }
            }
        }
        let mut co: Vec<(Id, Id, Id)> = Vec::new();
        for (scope, targets) in &scopes {
            let targets: Vec<&Id> = targets.iter().collect();
            for (k, a) in targets.iter().enumerate() {
                for b in &targets[k + 1..] {
                    co.push((**a, **b, *scope));
                }
            }
        }
        let vertices: Vec<Id> = invocation
            .iter()
            .chain(&co)
            .flat_map(|(a, b, _)| [*a, *b])
            .collect::<BTreeSet<Id>>()
            .into_iter()
            .collect();
        let dense = |id: &Id| vertices.binary_search(id).expect("a pair's end") as u32;
        let mut out = Input {
            vertices: vertices.clone(),
            ..Input::default()
        };
        for (a, b, site) in &invocation {
            out.invocation.add(dense(a), dense(b), *site);
        }
        for (a, b, scope) in &co {
            out.co_use.add(dense(a), dense(b), *scope);
        }
        for b in public {
            let node = ids(b, "node_id")?;
            let path = b
                .column_by_name("access_path")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| AnalyticsError::Column("access_path".to_owned()))?;
            for i in 0..b.num_rows() {
                out.public
                    .entry(id_at(node, i))
                    .or_insert_with(|| path.value(i).to_owned());
            }
        }
        Ok(out)
    }

    /// The combined weights in canonical pair order, and each layer's hub threshold.
    pub fn combined(&self, params: &Params) -> (BTreeMap<(u32, u32), f64>, [f64; 2]) {
        let n = self.vertices.len();
        let (inv, t_inv) = self.invocation.weights(n, params.hub_percentile);
        let (co, t_co) = self.co_use.weights(n, params.hub_percentile);
        let mut combined: BTreeMap<(u32, u32), f64> = BTreeMap::new();
        for (layer, weight) in [
            (&inv, params.invocation_weight),
            (&co, params.co_use_weight),
        ] {
            for (&pair, &w) in layer {
                *combined.entry(pair).or_insert(0.0) += weight * w;
            }
        }
        (combined, [t_inv, t_co])
    }
}

/// One Leiden run's record, for its invocation row.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub gamma: f64,
    pub seed: u64,
    /// The run's own parameters as canonical JSON.
    pub parameters: String,
    pub iterations: i64,
    /// The run stopped before its iteration budget.
    pub converged: bool,
    pub quality_history: Vec<f64>,
    pub communities: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct RunParameters<'a> {
    method: &'a str,
    gamma: f64,
    seed: u64,
    max_iterations: usize,
    epsilon: f64,
}

/// One resolution's stability.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stability {
    pub gamma: f64,
    pub mean_ari: f64,
    pub mean_nmi: f64,
    pub min_nmi: f64,
    /// The seed-0 partition's community count and largest community.
    pub communities: usize,
    pub largest: usize,
    pub degenerate: bool,
}

/// The consensus over every run.
#[derive(Debug, Clone, PartialEq)]
pub struct Consensus {
    pub runs: Vec<Run>,
    pub grid: Vec<Stability>,
    pub chosen: Option<f64>,
    /// The chosen resolution's seed-0 membership (canonical labels) and each community's
    /// agreement, by label.
    pub reference: Vec<usize>,
    pub agreement: Vec<f64>,
}

/// Labels in order of first appearance along the dense index.
fn canonical(labels: &[usize]) -> Vec<usize> {
    let mut map: BTreeMap<usize, usize> = BTreeMap::new();
    labels
        .iter()
        .map(|l| {
            let next = map.len();
            *map.entry(*l).or_insert(next)
        })
        .collect()
}

fn sizes(labels: &[usize]) -> Vec<usize> {
    let mut sizes = vec![0usize; labels.iter().max().map_or(0, |m| m + 1)];
    for l in labels {
        sizes[*l] += 1;
    }
    sizes
}

fn graph_error(e: impl std::fmt::Display) -> AnalyticsError {
    AnalyticsError::Graph(format!("leiden: {e}"))
}

/// Run every resolution and seed over `n` vertices and the combined weights, and form the
/// consensus.
pub fn consensus(
    n: usize,
    combined: &BTreeMap<(u32, u32), f64>,
    params: &Params,
) -> Result<Consensus, AnalyticsError> {
    let mut out = Consensus {
        runs: Vec::new(),
        grid: Vec::new(),
        chosen: None,
        reference: Vec::new(),
        agreement: Vec::new(),
    };
    if n < 2 || combined.is_empty() {
        return Ok(out);
    }
    let mut builder = GraphDataBuilder::new(n);
    for (&(a, b), &w) in combined {
        builder
            .add_edge(a as usize, b as usize, w)
            .map_err(graph_error)?;
    }
    let graph = builder.build().map_err(graph_error)?;
    let mut memberships: Vec<Vec<Vec<usize>>> = Vec::new();
    for &gamma in &params.gammas {
        let mut per_seed = Vec::new();
        for seed in 0..params.seeds {
            let config = LeidenConfig {
                max_iterations: params.max_iterations,
                resolution: gamma,
                seed: Some(seed),
                quality: QualityType::RBER,
                epsilon: params.epsilon,
                track_quality_history: true,
                ..LeidenConfig::default()
            };
            let result = Leiden::new(config).run(&graph).map_err(graph_error)?;
            let labels = canonical(result.partition.as_slice());
            let iterations = result.quality_history.len();
            out.runs.push(Run {
                gamma,
                seed,
                parameters: serde_json::to_string(&RunParameters {
                    method: "leiden-rber",
                    gamma,
                    seed,
                    max_iterations: params.max_iterations,
                    epsilon: params.epsilon,
                })
                .expect("run parameters serialize"),
                iterations: iterations as i64,
                converged: iterations < params.max_iterations,
                quality_history: result.quality_history,
                communities: sizes(&labels).len(),
            });
            per_seed.push(labels);
        }
        let (mut ari, mut nmi, mut min_nmi, mut pairs) = (0.0, 0.0, f64::INFINITY, 0usize);
        for s in 0..per_seed.len() {
            for t in s + 1..per_seed.len() {
                let a =
                    leiden_rs::metrics::try_ari(&per_seed[s], &per_seed[t]).map_err(graph_error)?;
                let m =
                    leiden_rs::metrics::try_nmi(&per_seed[s], &per_seed[t]).map_err(graph_error)?;
                ari += a;
                nmi += m;
                min_nmi = f64::min(min_nmi, m);
                pairs += 1;
            }
        }
        let (mean_ari, mean_nmi, min_nmi) = if pairs == 0 {
            (1.0, 1.0, 1.0)
        } else {
            (ari / pairs as f64, nmi / pairs as f64, min_nmi)
        };
        let reference = sizes(&per_seed[0]);
        let largest = reference.iter().copied().max().unwrap_or(0);
        out.grid.push(Stability {
            gamma,
            mean_ari,
            mean_nmi,
            min_nmi,
            communities: reference.len(),
            largest,
            degenerate: 2 * largest > n || largest < 3,
        });
        memberships.push(per_seed);
    }
    let chosen = out
        .grid
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.degenerate)
        .min_by(|(_, a), (_, b)| {
            b.mean_ari
                .total_cmp(&a.mean_ari)
                .then(a.gamma.log2().abs().total_cmp(&b.gamma.log2().abs()))
                .then(a.gamma.total_cmp(&b.gamma))
        })
        .map(|(k, _)| k);
    let Some(k) = chosen else {
        return Ok(out);
    };
    out.chosen = Some(params.gammas[k]);
    let seeds = &memberships[k];
    out.reference = seeds[0].clone();
    let groups = sizes(&out.reference);
    for (label, &size) in groups.iter().enumerate() {
        let mut total = 0.0;
        for other in &seeds[1..] {
            let mut inter: BTreeMap<usize, usize> = BTreeMap::new();
            for (v, &l) in out.reference.iter().enumerate() {
                if l == label {
                    *inter.entry(other[v]).or_insert(0) += 1;
                }
            }
            let other_sizes = sizes(other);
            let best = inter
                .iter()
                .map(|(&d, &i)| i as f64 / (size + other_sizes[d] - i) as f64)
                .fold(0.0, f64::max);
            total += best;
        }
        out.agreement.push(if seeds.len() > 1 {
            total / (seeds.len() - 1) as f64
        } else {
            1.0
        });
    }
    Ok(out)
}

/// What the consensus measured (its invocation's diagnostics).
#[derive(Debug, Serialize)]
struct Diagnostics<'a> {
    vertices: usize,
    invocation_pairs: usize,
    co_use_pairs: usize,
    combined_pairs: usize,
    invocation_hub_threshold: f64,
    co_use_hub_threshold: f64,
    grid: &'a [Stability],
    chosen_gamma: Option<f64>,
    communities: usize,
    reported: usize,
    below_agreement: usize,
    too_few_public: usize,
}

/// The communities' results: every run's record, the consensus's diagnostics, and the findings.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub vertices: usize,
    pub pairs: usize,
    pub runs: Vec<Run>,
    pub completion: CoverageStatus,
    pub diagnostics: String,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
}

/// Communities over `input`; the findings cite the consensus invocation.
pub fn run(
    input: &Input,
    params: &Params,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Outcome, AnalyticsError> {
    let n = input.vertices.len();
    let (combined, thresholds) = input.combined(params);
    let consensus = consensus(n, &combined, params)?;
    let status = FINDING_STATUS
        .iter()
        .find(|(k, _)| *k == FindingKind::Community)
        .map(|(_, s)| *s)
        .ok_or_else(|| AnalyticsError::Graph("no status policy for community".to_owned()))?;
    let mut findings = Vec::new();
    let mut members = Vec::new();
    let (mut below, mut few) = (0usize, 0usize);
    let groups = sizes(&consensus.reference);
    for (label, &agreement) in consensus.agreement.iter().enumerate() {
        let inside: Vec<u32> = consensus
            .reference
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == label)
            .map(|(v, _)| v as u32)
            .collect();
        let strength = |v: u32| -> f64 {
            inside
                .iter()
                .filter_map(|&u| combined.get(&(v.min(u), v.max(u))))
                .sum()
        };
        let public: Vec<(Id, &String, f64)> = inside
            .iter()
            .filter_map(|&v| {
                let id = input.vertices[v as usize];
                input.public.get(&id).map(|p| (id, p, strength(v)))
            })
            .collect();
        if public.len() < params.min_public_members {
            few += 1;
            continue;
        }
        if agreement < params.min_agreement {
            below += 1;
            continue;
        }
        let subject = public
            .iter()
            .min_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)))
            .map(|p| p.0)
            .expect("at least one public member");
        // The strongest pairs inside, each with the site of every layer it comes from.
        let mut pairs: Vec<((u32, u32), f64)> = combined
            .iter()
            .filter(|((a, b), _)| {
                consensus.reference[*a as usize] == label
                    && consensus.reference[*b as usize] == label
            })
            .map(|(p, w)| (*p, *w))
            .collect();
        pairs.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let omitted = pairs.len() > params.max_supporting;
        let mut rows: Vec<(MemberRole, Id, String, Option<f64>)> = public
            .iter()
            .map(|(id, path, s)| (MemberRole::CommunityMember, *id, (*path).clone(), Some(*s)))
            .collect();
        for (pair, _) in pairs.iter().take(params.max_supporting) {
            for (layer, name) in [(&input.invocation, "invocation"), (&input.co_use, "co_use")] {
                if let Some((_, site)) = layer.counts.get(pair) {
                    rows.push((MemberRole::SupportingSite, *site, name.to_owned(), None));
                }
            }
        }
        let keys: Vec<MemberKey> = rows
            .iter()
            .enumerate()
            .map(|(ordinal, (role, node, label, _))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: Some(*node),
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let finding_id = FindingKey {
            finding_kind: FindingKind::Community.code(),
            subject,
            related: None,
            condition: None,
            evidence_status: status.code(),
            depth: None,
            stop_reason: None,
            witnesses_omitted: omitted,
            paths: &[],
            members: &keys,
        }
        .id();
        for (ordinal, (role, node, label, weight)) in rows.into_iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role,
                ordinal: ordinal as i64,
                node_id: Some(node),
                cited_fact_id: None,
                label: Some(label),
                weight,
            });
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: FindingKind::Community,
            subject_node_id: subject,
            related_node_id: None,
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: omitted,
            score: Some(agreement),
            condition_node_id: None,
        });
    }
    let diagnostics = serde_json::to_string(&Diagnostics {
        vertices: n,
        invocation_pairs: input.invocation.counts.len(),
        co_use_pairs: input.co_use.counts.len(),
        combined_pairs: combined.len(),
        invocation_hub_threshold: thresholds[0],
        co_use_hub_threshold: thresholds[1],
        grid: &consensus.grid,
        chosen_gamma: consensus.chosen,
        communities: groups.len(),
        reported: findings.len(),
        below_agreement: below,
        too_few_public: few,
    })
    .expect("diagnostics serialize");
    let completion = if consensus.chosen.is_some() || combined.is_empty() {
        CoverageStatus::CompleteUnderStatedModel
    } else {
        CoverageStatus::Partial
    };
    Ok(Outcome {
        vertices: n,
        pairs: combined.len(),
        runs: consensus.runs,
        completion,
        diagnostics,
        findings,
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use leiden_rs::{LfrConfig, generate_lfr_graph};

    /// An LFR planted partition as one layer, from an edge list in the given order.
    fn layer_of(edges: &[(usize, usize)]) -> Layer {
        let mut layer = Layer::default();
        for (k, &(a, b)) in edges.iter().enumerate() {
            let mut site = [0u8; 16];
            site[8..].copy_from_slice(&(k as u64).to_be_bytes());
            layer.add(a as u32, b as u32, Id(site));
        }
        layer
    }

    fn lfr(mu: f64) -> (usize, Vec<(usize, usize)>, Vec<usize>) {
        let g = generate_lfr_graph(LfrConfig {
            n: 250,
            tau1: 2.5,
            tau2: 1.5,
            mu,
            average_degree: Some(10.0),
            min_degree: None,
            max_degree: Some(20),
            min_community: Some(25),
            max_community: Some(60),
            seed: Some(7),
        })
        .expect("an LFR graph");
        let edges = g.edges.iter().map(|&(a, b, _)| (a, b)).collect();
        (g.node_count, edges, g.ground_truth)
    }

    fn partition(n: usize, edges: &[(usize, usize)]) -> Consensus {
        let input = Input {
            vertices: (0..n)
                .map(|i| {
                    let mut id = [0u8; 16];
                    id[8..].copy_from_slice(&(i as u64).to_be_bytes());
                    Id(id)
                })
                .collect(),
            invocation: layer_of(edges),
            ..Input::default()
        };
        let params = Params::preregistered();
        let (combined, _) = input.combined(&params);
        consensus(n, &combined, &params).unwrap()
    }

    /// ADR-0011's revisit trigger: the planted partitions are recovered at the chosen resolution.
    #[test]
    fn lfr_planted_partitions_are_recovered() {
        for mu in [0.1, 0.3] {
            let (n, edges, truth) = lfr(mu);
            let c = partition(n, &edges);
            let nmi = leiden_rs::metrics::try_nmi(&c.reference, &truth).unwrap();
            assert!(
                nmi >= 0.9,
                "mu {mu}: NMI {nmi} at {:?}: {:?}",
                c.chosen,
                c.grid
            );
            assert!(c.grid.iter().all(|s| s.min_nmi > 0.0), "{:?}", c.grid);
        }
    }

    /// Shuffled and flipped input gives the identical partition, runs and stability (the
    /// library-leverage review's probe as a fixture).
    #[test]
    fn shuffled_and_flipped_input_gives_the_identical_partition() {
        let (n, edges, _) = lfr(0.5);
        let a = partition(n, &edges);
        let mut shuffled: Vec<(usize, usize)> = edges
            .iter()
            .enumerate()
            .map(|(k, &(x, y))| if k % 2 == 0 { (y, x) } else { (x, y) })
            .collect();
        // A deterministic permutation: reverse, then interleave halves.
        shuffled.reverse();
        let (left, right) = shuffled.split_at(shuffled.len() / 2);
        let mixed: Vec<(usize, usize)> = right
            .iter()
            .zip(left)
            .flat_map(|(r, l)| [*r, *l])
            .chain(right.iter().skip(left.len()).copied())
            .collect();
        assert_eq!(mixed.len(), edges.len());
        let b = partition(n, &mixed);
        assert_eq!(a, b);
    }

    #[test]
    fn a_hub_is_down_weighted_and_a_layer_sums_to_one() {
        // A star around 0 plus one pair: 0 is the only hub above the 50th percentile.
        let mut layer = Layer::default();
        for v in 1..5u32 {
            layer.add(0, v, Id::ZERO);
        }
        layer.add(5, 6, Id::ZERO);
        let (w, threshold) = layer.weights(7, 0.5);
        assert_eq!(threshold, 1.0);
        assert!((w.values().sum::<f64>() - 1.0).abs() < 1e-12);
        // Each star pair counts 1 · (1/4) · 1; the lone pair 1 · 1 · 1.
        assert!((w[&(0, 1)] * 4.0 - w[&(5, 6)]).abs() < 1e-12, "{w:?}");
    }

    #[test]
    fn a_trivial_partition_is_degenerate_and_nothing_is_chosen() {
        // Two vertices joined: every partition is one community or two singletons.
        let c = partition(2, &[(0, 1)]);
        assert_eq!(c.chosen, None, "{:?}", c.grid);
        assert!(c.grid.iter().all(|s| s.degenerate));
    }
}
