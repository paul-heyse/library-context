//! Explicit undirected conversion and retained seeded Leiden RBER profiles.
use crate::domain::{
    normalized::entities::EntityRef,
    resources::{Reservation, ResourceBudget},
    *,
};
use leiden_rs::{GraphDataBuilder, Leiden, LeidenConfig, QualityType};
use std::collections::BTreeMap;
fn invalid(m: impl Into<String>) -> ModelError {
    ModelError::Invalid(m.into())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub left: Id<EntityRef>,
    pub right: Id<EntityRef>,
    pub weight: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub resolution: FiniteF64,
    pub seed: u64,
    pub iterations: usize,
    pub converged: bool,
    pub quality: Vec<FiniteF64>,
    pub labels: Vec<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub resolution: FiniteF64,
    pub mean_ari: FiniteF64,
    pub sd_ari: FiniteF64,
    pub mean_nmi: FiniteF64,
    pub min_nmi: FiniteF64,
    pub largest: usize,
    pub communities: usize,
    pub degenerate: bool,
}
pub struct Partition {
    vertices: Vec<Id<EntityRef>>,
    runs: Vec<Run>,
    profiles: Vec<Profile>,
    chosen: bool,
    _reservation: Box<dyn Reservation>,
}
impl Partition {
    pub fn vertices(&self) -> &[Id<EntityRef>] {
        &self.vertices
    }
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
    pub fn chosen(&self) -> bool {
        self.chosen
    }
}
/// Canonical membership labels are presentation only, formed by first sorted entity appearance.
fn canonical(labels: &[usize]) -> Vec<usize> {
    let mut map = BTreeMap::new();
    labels
        .iter()
        .map(|v| {
            let next = map.len();
            *map.entry(*v).or_insert(next)
        })
        .collect()
}
/// Preflight reserves opaque library workspace and all 40 histories/partitions before conversion.
/// Max work is the declared vertex+edge upper bound times iterations and profile/seed count.
pub fn partition(
    vertices: &[Id<EntityRef>],
    pairs: &[Pair],
    max_work: u64,
    budget: &ResourceBudget,
) -> Result<Partition, ModelError> {
    let work = (vertices.len() as u64)
        .checked_add(pairs.len() as u64)
        .and_then(|n| n.checked_mul(super::policy::RETAINED.community_work_multiplier()?))
        .ok_or_else(|| invalid("Leiden work overflow"))?;
    if work > max_work {
        return Err(invalid("Leiden declared work bound reached"));
    }
    let bytes = vertices
        .len()
        .checked_mul(16384)
        .and_then(|n| n.checked_add(pairs.len().checked_mul(4096)?))
        .and_then(|n| n.checked_add(1024 * 1024))
        .ok_or_else(|| invalid("Leiden workspace overflow"))?;
    let reservation = budget.reserve("analytic-leiden-workspace", bytes)?;
    let mut vertices = vertices.to_vec();
    vertices.sort_unstable();
    if vertices.windows(2).any(|p| p[0] == p[1]) {
        return Err(invalid("duplicate Leiden vertex"));
    }
    let mut combined = BTreeMap::<(usize, usize), f64>::new();
    for p in pairs {
        let a = vertices
            .binary_search(&p.left)
            .map_err(|_| invalid("Leiden foreign endpoint"))?;
        let b = vertices
            .binary_search(&p.right)
            .map_err(|_| invalid("Leiden foreign endpoint"))?;
        if a == b || p.weight.get() <= 0.0 {
            return Err(invalid("Leiden requires positive non-self unordered pairs"));
        }
        let w = combined.entry((a.min(b), a.max(b))).or_default();
        *w += p.weight.get();
        FiniteF64::new(*w)?;
    }
    let mut out = Partition {
        vertices,
        runs: vec![],
        profiles: vec![],
        chosen: false,
        _reservation: reservation,
    };
    if out.vertices.len() < 2 || combined.is_empty() {
        return Ok(out);
    }
    let mut builder = GraphDataBuilder::new(out.vertices.len());
    for ((a, b), w) in combined {
        builder.add_edge(a, b, w).map_err(ModelError::codec)?;
    }
    let graph = builder.build().map_err(ModelError::codec)?;
    for resolution in super::policy::RETAINED.resolutions {
        let start = out.runs.len();
        for seed in 0..super::policy::RETAINED.seeds {
            let r = Leiden::new(LeidenConfig {
                resolution,
                seed: Some(seed),
                quality: QualityType::RBER,
                max_iterations: super::policy::RETAINED.iterations,
                epsilon: super::policy::RETAINED.tolerance,
                track_quality_history: true,
                ..Default::default()
            })
            .run(&graph)
            .map_err(ModelError::codec)?;
            let iterations = r.quality_history.len();
            out.runs.push(Run {
                resolution: FiniteF64::new(resolution)?,
                seed,
                iterations,
                converged: iterations < super::policy::RETAINED.iterations,
                quality: r
                    .quality_history
                    .into_iter()
                    .map(FiniteF64::new)
                    .collect::<Result<_, _>>()?,
                labels: canonical(r.partition.as_slice()),
            });
        }
        let rows = &out.runs[start..];
        let mut ari = vec![];
        let mut nmi = vec![];
        for a in 0..rows.len() {
            for b in a + 1..rows.len() {
                ari.push(
                    leiden_rs::metrics::try_ari(&rows[a].labels, &rows[b].labels)
                        .map_err(ModelError::codec)?,
                );
                nmi.push(
                    leiden_rs::metrics::try_nmi(&rows[a].labels, &rows[b].labels)
                        .map_err(ModelError::codec)?,
                );
            }
        }
        let mean = ari.iter().sum::<f64>() / ari.len() as f64;
        let sd = (ari.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / ari.len() as f64).sqrt();
        let mut sizes = BTreeMap::<usize, usize>::new();
        for l in &rows[0].labels {
            *sizes.entry(*l).or_default() += 1;
        }
        let largest = sizes.values().copied().max().unwrap_or(0);
        let degenerate = 2 * largest > out.vertices.len() || largest < 3;
        out.profiles.push(Profile {
            resolution: FiniteF64::new(resolution)?,
            mean_ari: FiniteF64::new(mean)?,
            sd_ari: FiniteF64::new(sd)?,
            mean_nmi: FiniteF64::new(nmi.iter().sum::<f64>() / nmi.len() as f64)?,
            min_nmi: FiniteF64::new(nmi.into_iter().fold(f64::INFINITY, f64::min))?,
            largest,
            communities: sizes.len(),
            degenerate,
        });
        if resolution == super::policy::RETAINED.selected_resolution && !degenerate {
            out.chosen = true;
        }
    }
    Ok(out)
}
/// Retained hub scaling and unit-total normalization for a selected layer; overflow refuses.
pub struct Weights {
    pairs: Vec<Pair>,
    _reservation: Box<dyn Reservation>,
}
impl Weights {
    pub fn pairs(&self) -> &[Pair] {
        &self.pairs
    }
}
pub fn normalize(
    vertices: &[Id<EntityRef>],
    counts: &[(Id<EntityRef>, Id<EntityRef>, u64)],
    budget: &ResourceBudget,
) -> Result<Weights, ModelError> {
    let reservation = budget.reserve(
        "analytic-layer-normalization",
        vertices
            .len()
            .checked_mul(256)
            .and_then(|n| n.checked_add(counts.len().checked_mul(512)?))
            .ok_or_else(|| invalid("layer size overflow"))?,
    )?;
    let mut strength = BTreeMap::<Id<EntityRef>, u64>::new();
    let mut pairs = BTreeMap::<(Id<EntityRef>, Id<EntityRef>), u64>::new();
    for &(a, b, c) in counts {
        if c == 0 || a == b || !vertices.contains(&a) || !vertices.contains(&b) {
            return Err(invalid("invalid community count"));
        }
        let value = pairs.entry((a.min(b), a.max(b))).or_default();
        *value = value
            .checked_add(c)
            .ok_or_else(|| invalid("community count overflow"))?;
        for v in [a, b] {
            let value = strength.entry(v).or_default();
            *value = value
                .checked_add(c)
                .ok_or_else(|| invalid("community strength overflow"))?;
        }
    }
    let mut positive = strength.values().copied().collect::<Vec<_>>();
    positive.sort_unstable();
    if positive.is_empty() {
        return Ok(Weights {
            pairs: vec![],
            _reservation: reservation,
        });
    }
    let threshold = positive[((positive.len() as f64 * super::policy::RETAINED.hub_percentile)
        .ceil() as usize)
        .clamp(1, positive.len())
        - 1] as f64;
    let raw = pairs
        .into_iter()
        .map(|((a, b), c)| {
            (
                a,
                b,
                c as f64
                    * (threshold / strength[&a] as f64).min(1.0)
                    * (threshold / strength[&b] as f64).min(1.0),
            )
        })
        .collect::<Vec<_>>();
    let total = raw.iter().map(|r| r.2).sum::<f64>();
    let pairs = raw
        .into_iter()
        .map(|(left, right, w)| {
            Ok(Pair {
                left,
                right,
                weight: FiniteF64::new(w / total)?,
            })
        })
        .collect::<Result<_, ModelError>>()?;
    Ok(Weights {
        pairs,
        _reservation: reservation,
    })
}
