//! Communities (DESIGN §9.4; ADR-0011): Leiden (leiden-rs, RBER) over the invocation and co-use
//! layers of the subsystem's functions, at one pre-registered resolution and seed set, with
//! seed-consensus stability; the stable communities are projected onto public APIs.
//!
//! **Input normal form.** Each layer is integer counts per unordered pair `(min, max)` over the
//! dense index (the sorted ids of the subsystem functions some pair touches), kept in a
//! `BTreeMap`, so shuffled or flipped input gives identical counts. Each pair keeps its least
//! contributing site (a call site, or a usage scope) as lineage (H1 review F9). What each layer
//! counts is a named policy in [`Params`] (ADR-0011 review F3).
//!
//! **Weights,** in canonical pair order: a hub's pairs are down-weighted (each end whose strength
//! exceeds the layer's strength percentile scales the count by threshold / strength); each layer
//! is normalized to unit total weight; the layers are summed with their pre-registered weights.
//!
//! **Consensus.** RBER at γ = 1, its own density scale, runs every seed (ADR-0011 review F2: a
//! grid choice was decided by the seed block, not the data). The other profile resolutions run
//! too, and their stability is recorded, never chosen from. γ = 1 is degenerate when its seed-0
//! partition puts more than half the vertices in one community or has no community of three:
//! then no community is reported, and the diagnostics say so. A community of the seed-0 partition
//! is reported when it has enough public members and they stay together across the other seeds:
//! its score is the mean, over those seeds, of the share of its public-member pairs that share a
//! community there (review F5: the score measures the published claim).

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::{Array, BooleanArray, FixedSizeBinaryArray, RecordBatch, StringArray};
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

/// The pre-registered parameters (slice 2.3, revised by the ADR-0011 review; deviation log D28):
/// code, not the frozen analytics config, recorded in every invocation, in the compiler digest
/// and in the gold freeze (`eval/gold/analytics-freeze.json`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    /// The RBER resolution the reported partition uses.
    pub gamma: f64,
    /// Resolutions whose stability is recorded as a profile (the chosen one among them).
    pub profile_gammas: Vec<f64>,
    /// Seeds `0..seeds` at every resolution.
    pub seeds: u64,
    /// What the invocation layer counts.
    pub invocation_policy: &'static str,
    /// What the co-use layer counts.
    pub co_use_policy: &'static str,
    pub invocation_weight: f64,
    pub co_use_weight: f64,
    /// A pair end whose strength exceeds this percentile of its layer's strengths is a hub.
    pub hub_percentile: f64,
    pub max_iterations: usize,
    pub epsilon: f64,
    /// A community's least public co-assignment to be reported.
    pub min_agreement: f64,
    /// A community's least public members to be reported.
    pub min_public_members: usize,
    /// The strongest pairs a community cites.
    pub max_supporting: usize,
}

impl Params {
    /// The parameters frozen from the ADR-0011 review's fixes (their digest is in the gold
    /// freeze).
    pub fn preregistered() -> Self {
        Params {
            gamma: 1.0,
            profile_gammas: vec![0.5, 1.0, 2.0, 4.0],
            seeds: 10,
            invocation_policy: "every invocation-projection arc between two distinct subsystem \
                                functions (calls, property accesses and definitions; definite and \
                                candidate), 1 per arc",
            co_use_policy: "every official-usage scope (function or module) calling two distinct \
                            subsystem functions, 1 per scope",
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

/// Each public callable's preferred path (`cpg_schema::communities::public_callables_sql`'s
/// `preferred` rows; the increment-2 review's F4): the one name every consumer shows it by.
pub fn preferred_paths(public: &[RecordBatch]) -> Result<BTreeMap<Id, String>, AnalyticsError> {
    let mut out = BTreeMap::new();
    for b in public {
        let node = ids(b, "node_id")?;
        let path = b
            .column_by_name("access_path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| AnalyticsError::Column("access_path".to_owned()))?;
        let preferred = b
            .column_by_name("preferred")
            .and_then(|c| c.as_any().downcast_ref::<BooleanArray>())
            .ok_or_else(|| AnalyticsError::Column("preferred".to_owned()))?;
        for i in 0..b.num_rows() {
            if preferred.value(i)
                && out
                    .insert(id_at(node, i), path.value(i).to_owned())
                    .is_some()
            {
                return Err(AnalyticsError::Graph(format!(
                    "two preferred paths for one callable ({})",
                    path.value(i)
                )));
            }
        }
    }
    Ok(out)
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
        out.public = preferred_paths(public)?;
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

/// One resolution's stability across the seeds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stability {
    pub gamma: f64,
    pub mean_ari: f64,
    /// The standard deviation of the pairwise ARIs (the review's F2: a margin needs its spread).
    pub sd_ari: f64,
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
    pub profile: Vec<Stability>,
    /// `Params::gamma`, unless it is degenerate.
    pub chosen: Option<f64>,
    /// Every seed's membership at the chosen resolution (canonical labels), seed 0 first: the
    /// reported partition is seed 0's.
    pub memberships: Vec<Vec<usize>>,
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

/// Run every profile resolution and seed over `n` vertices and the combined weights, and form
/// the consensus at `params.gamma`.
pub fn consensus(
    n: usize,
    combined: &BTreeMap<(u32, u32), f64>,
    params: &Params,
) -> Result<Consensus, AnalyticsError> {
    let mut out = Consensus {
        runs: Vec::new(),
        profile: Vec::new(),
        chosen: None,
        memberships: Vec::new(),
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
    for &gamma in &params.profile_gammas {
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
        let mut aris = Vec::new();
        let (mut nmi, mut min_nmi) = (0.0, f64::INFINITY);
        for s in 0..per_seed.len() {
            for t in s + 1..per_seed.len() {
                aris.push(
                    leiden_rs::metrics::try_ari(&per_seed[s], &per_seed[t]).map_err(graph_error)?,
                );
                let m =
                    leiden_rs::metrics::try_nmi(&per_seed[s], &per_seed[t]).map_err(graph_error)?;
                nmi += m;
                min_nmi = f64::min(min_nmi, m);
            }
        }
        let (mean_ari, sd_ari, mean_nmi, min_nmi) = if aris.is_empty() {
            (1.0, 0.0, 1.0, 1.0)
        } else {
            let k = aris.len() as f64;
            let mean: f64 = aris.iter().sum::<f64>() / k;
            let var: f64 = aris.iter().map(|a| (a - mean) * (a - mean)).sum::<f64>() / k;
            (mean, var.sqrt(), nmi / k, min_nmi)
        };
        let reference = sizes(&per_seed[0]);
        let largest = reference.iter().copied().max().unwrap_or(0);
        let degenerate = 2 * largest > n || largest < 3;
        out.profile.push(Stability {
            gamma,
            mean_ari,
            sd_ari,
            mean_nmi,
            min_nmi,
            communities: reference.len(),
            largest,
            degenerate,
        });
        if gamma == params.gamma && !degenerate {
            out.chosen = Some(gamma);
            out.memberships = per_seed;
        }
    }
    Ok(out)
}

/// The mean, over the other seeds, of the share of the public members' pairs that share a
/// community there: how often the published grouping holds (ADR-0011 review F5). One member, or
/// one seed, scores 1.
pub fn co_assignment(members: &[usize], others: &[Vec<usize>]) -> f64 {
    let pairs: Vec<(usize, usize)> = members
        .iter()
        .enumerate()
        .flat_map(|(k, &a)| members[k + 1..].iter().map(move |&b| (a, b)))
        .collect();
    if pairs.is_empty() || others.is_empty() {
        return 1.0;
    }
    let together = |m: &Vec<usize>| {
        pairs.iter().filter(|(a, b)| m[*a] == m[*b]).count() as f64 / pairs.len() as f64
    };
    others.iter().map(together).sum::<f64>() / others.len() as f64
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
    profile: &'a [Stability],
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
    let reference: &[usize] = consensus.memberships.first().map_or(&[], Vec::as_slice);
    let others: &[Vec<usize>] = consensus.memberships.get(1..).unwrap_or_default();
    let groups = sizes(reference);
    for label in 0..groups.len() {
        let inside: Vec<u32> = reference
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
        let public: Vec<(u32, Id, &String, f64)> = inside
            .iter()
            .filter_map(|&v| {
                let id = input.vertices[v as usize];
                input.public.get(&id).map(|p| (v, id, p, strength(v)))
            })
            .collect();
        if public.len() < params.min_public_members {
            few += 1;
            continue;
        }
        let dense: Vec<usize> = public.iter().map(|p| p.0 as usize).collect();
        let agreement = co_assignment(&dense, others);
        if agreement < params.min_agreement {
            below += 1;
            continue;
        }
        let subject = public
            .iter()
            .min_by(|a, b| b.3.total_cmp(&a.3).then(a.1.cmp(&b.1)))
            .map(|p| p.1)
            .expect("at least one public member");
        // The strongest pairs inside, each with the site of every layer it comes from.
        let mut pairs: Vec<((u32, u32), f64)> = combined
            .iter()
            .filter(|((a, b), _)| {
                reference[*a as usize] == label && reference[*b as usize] == label
            })
            .map(|(p, w)| (*p, *w))
            .collect();
        pairs.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let omitted = pairs.len() > params.max_supporting;
        let mut ordered = public.clone();
        ordered.sort_by_key(|p| p.1);
        let mut rows: Vec<(MemberRole, Id, String, Option<f64>)> = ordered
            .iter()
            .map(|(_, id, path, s)| (MemberRole::CommunityMember, *id, (*path).clone(), Some(*s)))
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
        profile: &consensus.profile,
        chosen_gamma: consensus.chosen,
        communities: groups.len(),
        reported: findings.len(),
        below_agreement: below,
        too_few_public: few,
    })
    .expect("diagnostics serialize");
    Ok(Outcome {
        vertices: n,
        pairs: combined.len(),
        runs: consensus.runs,
        // A degenerate resolution is a stated outcome, not a cut-short run (review O3).
        completion: CoverageStatus::CompleteUnderStatedModel,
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
            assert_eq!(c.chosen, Some(1.0), "{:?}", c.profile);
            let nmi = leiden_rs::metrics::try_nmi(&c.memberships[0], &truth).unwrap();
            assert!(nmi >= 0.9, "mu {mu}: NMI {nmi}: {:?}", c.profile);
            assert!(c.profile.iter().all(|s| s.min_nmi > 0.0), "{:?}", c.profile);
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
        assert_eq!(c.chosen, None, "{:?}", c.profile);
        assert!(c.profile.iter().all(|s| s.degenerate) && c.memberships.is_empty());
    }

    /// ADR-0011 review F5: the score is how often the public members stay together, not how
    /// their whole community does.
    #[test]
    fn the_score_is_the_public_members_co_assignment() {
        // Public members 0 and 1: together in one of two other seeds.
        let others = vec![vec![0, 0, 1, 1], vec![0, 1, 1, 1]];
        assert!((co_assignment(&[0, 1], &others) - 0.5).abs() < 1e-12);
        // Three members: pairs (0,1), (0,2), (1,2); together in 3 of 3, then 1 of 3.
        let others = vec![vec![0, 0, 0], vec![0, 0, 1]];
        assert!((co_assignment(&[0, 1, 2], &others) - (1.0 + 1.0 / 3.0) / 2.0).abs() < 1e-12);
        assert_eq!(co_assignment(&[2], &others), 1.0);
        assert_eq!(co_assignment(&[0, 1], &[]), 1.0);
    }

    /// ADR-0011 review F4: the parameters are frozen with the analytics config; an edit is an
    /// ADR-0004 amendment and a new recorded digest.
    #[test]
    fn the_parameters_match_their_gold_freeze() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../eval/gold/analytics-freeze.json");
        let freeze: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(
            freeze["community_parameters"].as_str(),
            Some(Params::preregistered().digest().hex().as_str())
        );
        assert_eq!(
            freeze["pagerank_parameters"].as_str(),
            Some(
                crate::ranking::Params::preregistered()
                    .digest()
                    .hex()
                    .as_str()
            )
        );
        assert_eq!(
            freeze["knn_parameters"].as_str(),
            Some(
                crate::neighbours::Params::preregistered()
                    .digest()
                    .hex()
                    .as_str()
            )
        );
        assert_eq!(
            freeze["selection_parameters"].as_str(),
            Some(
                crate::selection::Params::preregistered()
                    .digest()
                    .hex()
                    .as_str()
            )
        );
        assert_eq!(
            freeze["fca_parameters"].as_str(),
            Some(
                crate::concepts::Params::preregistered()
                    .digest()
                    .hex()
                    .as_str()
            )
        );
    }
}
