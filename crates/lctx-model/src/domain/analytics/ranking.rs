//! Bounded weighted power iteration over explicitly selected nominal entities and weight pairs.
//! The producing model owns graph/layer selection and exact arc lineage; numerical rank is heuristic.
use crate::domain::{
    normalized::entities::EntityRef,
    resources::{Reservation, ResourceBudget},
    *,
};
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub source: Id<EntityRef>,
    pub target: Id<EntityRef>,
    pub weight: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    pub damping: FiniteF64,
    pub tolerance: FiniteF64,
    pub max_iterations: u64,
    pub max_work: u64,
}
impl Parameters {
    pub fn retained(max_work: u64) -> Self {
        let mut policy = super::policy::RETAINED;
        policy.max_work = max_work;
        policy.ranking().expect("retained finite analytic policy")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Converged,
    Empty,
    IterationLimit,
    WorkLimit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub entity: Id<EntityRef>,
    pub value: FiniteF64,
}
pub struct Ranks {
    scores: Vec<Score>,
    iterations: u64,
    residual: Option<FiniteF64>,
    stop: Stop,
    work: u64,
    _reservation: Box<dyn Reservation>,
}
impl Ranks {
    pub fn scores(&self) -> &[Score] {
        &self.scores
    }
    pub fn iterations(&self) -> u64 {
        self.iterations
    }
    pub fn residual(&self) -> Option<FiniteF64> {
        self.residual
    }
    pub fn stop(&self) -> Stop {
        self.stop
    }
    pub fn work(&self) -> u64 {
        self.work
    }
}
/// Uniform teleport/dangling mass and uniform initial scores, canonical accumulation order.
/// Parallel pairs add checked integer weights; unknown endpoints and zero weights refuse.
/// A work unit is one vertex update or one aggregated weighted pair per iteration.
pub fn rank(
    vertices: &[Id<EntityRef>],
    pairs: &[Pair],
    parameters: Parameters,
    budget: &ResourceBudget,
) -> Result<Ranks, ModelError> {
    let damping = parameters.damping.get();
    let tolerance = parameters.tolerance.get();
    if !(0.0..=1.0).contains(&damping) || tolerance <= 0.0 {
        return Err(invalid("invalid weighted ranking parameters"));
    }
    let bytes = vertices
        .len()
        .checked_mul(
            size_of::<Score>()
                + size_of::<Id<EntityRef>>()
                + 4 * size_of::<f64>()
                + size_of::<u64>(),
        )
        .and_then(|n| {
            n.checked_add(
                pairs
                    .len()
                    .checked_mul(2 * size_of::<Pair>() + size_of::<(usize, usize, u64)>())?,
            )
        })
        .and_then(|n| n.checked_add(size_of::<Ranks>() + 4096))
        .ok_or_else(|| invalid("weighted ranking allocation overflow"))?;
    let reservation = budget.reserve("native-weighted-ranking", bytes)?;
    let mut vertices = vertices.to_vec();
    vertices.sort_unstable();
    if vertices.windows(2).any(|p| p[0] == p[1]) {
        return Err(invalid("duplicate weighted ranking vertex"));
    }
    let mut pairs = pairs.to_vec();
    pairs.sort_unstable_by_key(|p| (p.source, p.target, p.weight));
    let mut arcs: Vec<(usize, usize, u64)> = Vec::with_capacity(pairs.len());
    for pair in &pairs {
        if pair.weight == 0 {
            return Err(invalid("zero weighted ranking pair"));
        }
        let source = vertices
            .binary_search(&pair.source)
            .map_err(|_| invalid("ranking source outside selected universe"))?;
        let target = vertices
            .binary_search(&pair.target)
            .map_err(|_| invalid("ranking target outside selected universe"))?;
        if let Some(last) = arcs
            .last_mut()
            .filter(|last| last.0 == source && last.1 == target)
        {
            last.2 = last
                .2
                .checked_add(pair.weight)
                .ok_or_else(|| invalid("ranking parallel weight overflow"))?;
        } else {
            arcs.push((source, target, pair.weight));
        }
    }
    drop(pairs);
    let n = vertices.len();
    if n == 0 {
        return Ok(Ranks {
            scores: vec![],
            iterations: 0,
            residual: None,
            stop: Stop::Empty,
            work: 0,
            _reservation: reservation,
        });
    }
    let mut out_weight = vec![0u64; n];
    for &(source, _, weight) in &arcs {
        out_weight[source] = out_weight[source]
            .checked_add(weight)
            .ok_or_else(|| invalid("ranking outgoing weight overflow"))?;
    }
    let uniform = 1.0 / n as f64;
    let mut scores = vec![uniform; n];
    let mut next = vec![0.0; n];
    let mut flow = vec![0.0; n];
    let cost = u64::try_from(n)
        .ok()
        .and_then(|n| {
            u64::try_from(arcs.len())
                .ok()
                .and_then(|a| n.checked_add(a))
        })
        .ok_or_else(|| invalid("ranking work overflow"))?;
    let mut work = 0u64;
    let mut iterations = 0u64;
    let mut residual = None;
    let mut stop = Stop::IterationLimit;
    while iterations < parameters.max_iterations {
        if parameters.max_work.saturating_sub(work) < cost {
            stop = Stop::WorkLimit;
            break;
        }
        work = work
            .checked_add(cost)
            .ok_or_else(|| invalid("ranking work overflow"))?;
        let dangling: f64 = scores
            .iter()
            .zip(&out_weight)
            .filter(|(_, w)| **w == 0)
            .map(|(score, _)| score)
            .sum();
        flow.fill(0.0);
        for &(source, target, weight) in &arcs {
            flow[target] += scores[source] * weight as f64 / out_weight[source] as f64;
        }
        let base = (1.0 - damping) * uniform + damping * dangling * uniform;
        for (value, flow) in next.iter_mut().zip(&flow) {
            *value = base + damping * flow;
        }
        let delta = FiniteF64::new(next.iter().zip(&scores).map(|(a, b)| (a - b).abs()).sum())?;
        std::mem::swap(&mut scores, &mut next);
        iterations += 1;
        residual = Some(delta);
        if delta.get() < tolerance {
            stop = Stop::Converged;
            break;
        }
    }
    let scores = vertices
        .into_iter()
        .zip(scores)
        .map(|(entity, value)| {
            Ok(Score {
                entity,
                value: FiniteF64::new(value)?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    Ok(Ranks {
        scores,
        iterations,
        residual,
        stop,
        work,
        _reservation: reservation,
    })
}
