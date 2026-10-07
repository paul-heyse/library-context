//! Pure ranking of admitted occurrences. Scores aid discovery; they are not requirement evidence.
use super::identity::{ChannelIdentity, PolicyIdentity, SnapshotHandle, policy_identity};
use crate::domain::{
    ContentHash, Id, KeySink, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    resources::{Reservation, ResourceBudget},
    retrieval::{ContentPart, Family, OriginalAnchor, SearchWindow, Unit, WindowBinding},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

const PREPARATION_BYTES_PER_ROW: usize = 768;
const FUSION_BYTES_PER_ROW: usize = 2048;

/// Answer-affecting native analyzer and BM25 settings belong to the sealed realization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LexicalPolicy {
    pub analyzer: super::Name,
    pub definition: ContentHash,
    pub k1: f64,
    pub b: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingPolicy {
    pub revision: u32,
    pub lexical: LexicalPolicy,
    pub rrf_k: u32,
}
impl Default for RankingPolicy {
    fn default() -> Self {
        Self {
            revision: 3,
            lexical: LexicalPolicy {
                analyzer: super::Name::new("lctx_discovery").expect("bounded analyzer name"),
                definition: ContentHash::of(b"lctx-discovery/v2:class,camel;lowercase"),
                k1: 1.5,
                b: 0.75,
            },
            rrf_k: 60,
        }
    }
}
impl RankingPolicy {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.revision != 3
            || self.rrf_k != 60
            || !self.lexical.k1.is_finite()
            || self.lexical.k1 <= 0.0
            || !self.lexical.b.is_finite()
            || !(0.0..=1.0).contains(&self.lexical.b)
        {
            return Err(invalid("unsupported native ranking policy"));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<PolicyIdentity, ModelError> {
        self.validate()?;
        policy_identity(self)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Lexical,
    Vector,
}
/// Availability belongs to the serving channel-state contract. This binds an active scorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBinding {
    channel: Channel,
    identity: ChannelIdentity,
    policy: PolicyIdentity,
}
impl ChannelBinding {
    pub fn lexical(policy: &RankingPolicy, query: &str) -> Result<Self, ModelError> {
        let policy = policy.identity()?;
        let mut sink = KeySink::new("serving-lexical-channel/v1");
        sink.part(b"policy", &policy.0.0);
        sink.part(b"query", query.as_bytes());
        Ok(Self {
            channel: Channel::Lexical,
            identity: ChannelIdentity(sink.finish()),
            policy,
        })
    }
    /// V0 validates the actual query vector and supplies its canonical digest, never a model label.
    pub fn vector(
        policy: &RankingPolicy,
        spec: ContentHash,
        query_vector: ContentHash,
        recipe: ContentHash,
        projection: Id<crate::domain::embedding::projection::ProjectionDefinition>,
    ) -> Result<Self, ModelError> {
        let policy = policy.identity()?;
        let mut sink = KeySink::new("serving-vector-channel/v1");
        sink.part(b"policy", &policy.0.0);
        sink.part(b"spec", &spec.0);
        sink.part(b"query-vector", &query_vector.0);
        sink.part(b"query-recipe", &recipe.0);
        sink.part(b"projection", projection.bytes());
        Ok(Self {
            channel: Channel::Vector,
            identity: ChannelIdentity(sink.finish()),
            policy,
        })
    }
    pub fn channel(&self) -> Channel {
        self.channel
    }
    pub fn identity(&self) -> ChannelIdentity {
        self.identity
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Member { member: Id<CatalogMember> },
    Unit { unit: Id<Unit> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    pub target: Target,
    pub unit: Id<Unit>,
    pub window: Id<SearchWindow>,
    pub part: Id<ContentPart>,
    pub binding: Option<Id<WindowBinding>>,
    pub context: Id<AnalysisContext>,
    pub anchor: Option<Id<OriginalAnchor>>,
    pub family: Family,
}
impl Ord for Occurrence {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            self.target,
            self.family as i16,
            self.unit,
            self.window,
            self.part,
            self.binding,
            self.context,
            self.anchor,
        )
            .cmp(&(
                other.target,
                other.family as i16,
                other.unit,
                other.window,
                other.part,
                other.binding,
                other.context,
                other.anchor,
            ))
    }
}
impl PartialOrd for Occurrence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateScore {
    pub snapshot: SnapshotHandle,
    pub occurrence: Occurrence,
    pub channel: Channel,
    pub channel_identity: ChannelIdentity,
    /// None is missing numerical work; lexical zero abstains. Finite cosine zero is a real score.
    pub score: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingWitness {
    pub occurrence: Occurrence,
    pub channel: Channel,
    pub channel_identity: ChannelIdentity,
    pub channel_score: f64,
    /// Contiguous one-based rank within this family/channel after contextual window grouping.
    pub rank: u32,
    pub snapshot: SnapshotHandle,
    pub policy: PolicyIdentity,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankedHit {
    pub target: Target,
    pub context: Id<AnalysisContext>,
    pub score: f64,
    pub promoted: bool,
    pub witnesses: Vec<RankingWitness>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelStatistics {
    pub channel: Channel,
    pub missing: usize,
    pub lexical_zero: usize,
    pub contributing_occurrences: usize,
}
/// The result owns its allocation reservation until its rows and witnesses are dropped.
#[derive(Debug)]
pub struct RankedResults {
    rows: Vec<RankedHit>,
    statistics: Vec<ChannelStatistics>,
    _reservation: Box<dyn Reservation>,
}
impl RankedResults {
    pub fn rows(&self) -> &[RankedHit] {
        &self.rows
    }
    pub fn statistics(&self) -> &[ChannelStatistics] {
        &self.statistics
    }
}

/// Native retrieval supplies only the bounded candidate union and its eligible occurrence witnesses.
/// This fold owns fusion, never corpus scoring or whole-store classification.
#[derive(Debug)]
pub struct CandidateFusion {
    snapshot: SnapshotHandle,
    policy: RankingPolicy,
    policy_identity: PolicyIdentity,
    channels: BTreeMap<Channel, ChannelBinding>,
    eligible: BTreeSet<Target>,
    occurrences: BTreeSet<Occurrence>,
    budget: ResourceBudget,
    _reservation: Box<dyn Reservation>,
}
impl CandidateFusion {
    pub fn new(
        snapshot: SnapshotHandle,
        policy: RankingPolicy,
        channels: &[ChannelBinding],
        eligible: &[Target],
        occurrences: &[Occurrence],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let policy_identity = policy.identity()?;
        let count = checked_add(
            checked_add(eligible.len(), occurrences.len())?,
            channels.len(),
        )?;
        let reservation = budget.reserve(
            "serving candidate preparation",
            checked_mul(count, PREPARATION_BYTES_PER_ROW)?,
        )?;
        let mut channel_map = BTreeMap::new();
        for binding in channels {
            if binding.policy != policy_identity
                || channel_map.insert(binding.channel, *binding).is_some()
            {
                return Err(invalid("duplicate or foreign-policy ranking channel"));
            }
        }
        let targets: BTreeSet<_> = eligible.iter().copied().collect();
        if targets.len() != eligible.len() {
            return Err(invalid("duplicate eligible ranking target"));
        }
        if targets.iter().any(|t| matches!(t, Target::Member { .. }))
            && targets.iter().any(|t| matches!(t, Target::Unit { .. }))
        {
            return Err(invalid(
                "member and independent evidence ranking universes must be separate",
            ));
        }
        for occurrence in occurrences {
            if !targets.contains(&occurrence.target) {
                return Err(invalid("occurrence outside eligible candidate set"));
            }
            if matches!(occurrence.target, Target::Member { .. }) && occurrence.binding.is_none() {
                return Err(invalid("member nomination requires actual primary binding"));
            }
            if matches!(occurrence.target, Target::Unit { unit } if unit != occurrence.unit) {
                return Err(invalid("evidence occurrence belongs to a different unit"));
            }
        }
        Ok(Self {
            snapshot,
            policy,
            policy_identity,
            channels: channel_map,
            eligible: targets,
            occurrences: occurrences.iter().copied().collect(),
            budget: budget.clone(),
            _reservation: reservation,
        })
    }
    pub fn policy(&self) -> &RankingPolicy {
        &self.policy
    }
    pub fn policy_identity(&self) -> PolicyIdentity {
        self.policy_identity
    }
    pub fn snapshot(&self) -> SnapshotHandle {
        self.snapshot.clone()
    }
    pub fn channel(&self, channel: Channel) -> Option<ChannelBinding> {
        self.channels.get(&channel).copied()
    }

    /// Validate all scores before mutation/fusion. Exact duplicates are neutral; conflicts refuse.
    pub fn rank(
        &self,
        scores: &[CandidateScore],
        promoted: &[Id<CatalogMember>],
    ) -> Result<RankedResults, ModelError> {
        let count = checked_add(
            checked_add(scores.len(), self.eligible.len())?,
            self.occurrences.len(),
        )?;
        let reservation = self.budget.reserve(
            "serving ranking fusion",
            checked_mul(count, FUSION_BYTES_PER_ROW)?,
        )?;
        let mut exact = BTreeSet::new();
        for member in promoted {
            let target = Target::Member { member: *member };
            if !self.eligible.contains(&target) {
                return Err(invalid(
                    "exact-path promotion outside eligible candidate set",
                ));
            }
            exact.insert(target);
        }
        let mut numerical = BTreeMap::<(Occurrence, Channel), Option<f64>>::new();
        for row in scores {
            if row.snapshot != self.snapshot {
                return Err(invalid("numerical snapshot differs"));
            }
            if !self.occurrences.contains(&row.occurrence) {
                return Err(invalid(
                    "numerical occurrence outside admitted candidate witnesses",
                ));
            }
            let binding = self
                .channels
                .get(&row.channel)
                .ok_or_else(|| invalid("inactive numerical channel"))?;
            if row.channel_identity != binding.identity {
                return Err(invalid("numerical channel identity differs"));
            }
            if row.score.is_some_and(|score| {
                !score.is_finite() || (row.channel == Channel::Lexical && score < 0.0)
            }) {
                return Err(invalid("nonfinite or negative lexical numerical score"));
            }
            let normalized = row
                .score
                .map(|value| if value == 0.0 { 0.0 } else { value });
            if let Some(previous) = numerical.insert((row.occurrence, row.channel), normalized)
                && previous != normalized
            {
                return Err(invalid("conflicting duplicate numerical score"));
            }
        }
        let mut statistics: Vec<_> = self
            .channels
            .keys()
            .map(|channel| ChannelStatistics {
                channel: *channel,
                missing: self.occurrences.len(),
                lexical_zero: 0,
                contributing_occurrences: 0,
            })
            .collect();
        let mut best =
            BTreeMap::<(i16, Channel, Target, Id<AnalysisContext>), RankingWitness>::new();
        for ((occurrence, channel), score) in numerical {
            let stat = statistics
                .iter_mut()
                .find(|s| s.channel == channel)
                .expect("admitted channel");
            let Some(score) = score else {
                continue;
            };
            stat.missing -= 1;
            if channel == Channel::Lexical && score == 0.0 {
                stat.lexical_zero += 1;
            }
            stat.contributing_occurrences += 1;
            let witness = RankingWitness {
                occurrence,
                channel,
                channel_identity: self.channels[&channel].identity,
                channel_score: score,
                rank: 0,
                snapshot: self.snapshot.clone(),
                policy: self.policy_identity,
            };
            let key = (
                occurrence.family as i16,
                channel,
                occurrence.target,
                occurrence.context,
            );
            if best.get(&key).is_none_or(|old| better(&witness, old)) {
                best.insert(key, witness);
            }
        }
        let mut winners: Vec<_> = best.into_values().collect();
        winners.sort_by(|a, b| {
            (a.occurrence.family as i16, a.channel)
                .cmp(&(b.occurrence.family as i16, b.channel))
                .then_with(|| b.channel_score.total_cmp(&a.channel_score))
                .then_with(|| {
                    (a.occurrence.target, a.occurrence.context)
                        .cmp(&(b.occurrence.target, b.occurrence.context))
                })
        });
        let mut previous = None;
        let mut rank = 0u32;
        let mut family_scores = BTreeMap::<(i16, Target, Id<AnalysisContext>), f64>::new();
        let mut witnesses = BTreeMap::<(Target, Id<AnalysisContext>), Vec<RankingWitness>>::new();
        for mut winner in winners {
            let group = (winner.occurrence.family as i16, winner.channel);
            if previous != Some(group) {
                previous = Some(group);
                rank = 0;
            }
            rank = rank
                .checked_add(1)
                .ok_or_else(|| invalid("ranking count exceeds u32"))?;
            winner.rank = rank;
            *family_scores
                .entry((group.0, winner.occurrence.target, winner.occurrence.context))
                .or_default() += reciprocal(self.policy.rrf_k, rank);
            witnesses
                .entry((winner.occurrence.target, winner.occurrence.context))
                .or_default()
                .push(winner);
        }
        // Normalize each family's channel fusion to ranks, then give every family one vote.
        let mut family_order: Vec<_> = family_scores.into_iter().collect();
        family_order.sort_by(|a, b| {
            a.0.0
                .cmp(&b.0.0)
                .then_with(|| b.1.total_cmp(&a.1))
                .then_with(|| (a.0.1, a.0.2).cmp(&(b.0.1, b.0.2)))
        });
        let mut totals = BTreeMap::<(Target, Id<AnalysisContext>), f64>::new();
        let mut previous_family = None;
        rank = 0;
        for ((family, target, context), _) in family_order {
            if previous_family != Some(family) {
                previous_family = Some(family);
                rank = 0;
            }
            rank = rank
                .checked_add(1)
                .ok_or_else(|| invalid("ranking count exceeds u32"))?;
            *totals.entry((target, context)).or_default() += reciprocal(self.policy.rrf_k, rank);
        }
        // Promotion is contextual only where an actual occurrence supplies the witness.
        for occurrence in &self.occurrences {
            if exact.contains(&occurrence.target) {
                totals
                    .entry((occurrence.target, occurrence.context))
                    .or_default();
            }
        }
        let mut rows: Vec<_> = totals
            .into_iter()
            .map(|((target, context), score)| RankedHit {
                target,
                context,
                score,
                promoted: exact.contains(&target),
                witnesses: witnesses.remove(&(target, context)).unwrap_or_default(),
            })
            .collect();
        rows.sort_by(|a, b| {
            b.promoted
                .cmp(&a.promoted)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| (a.target, a.context).cmp(&(b.target, b.context)))
        });
        Ok(RankedResults {
            rows,
            statistics,
            _reservation: reservation,
        })
    }
}
fn better(a: &RankingWitness, b: &RankingWitness) -> bool {
    a.channel_score.total_cmp(&b.channel_score) == Ordering::Greater
        || (a.channel_score == b.channel_score && a.occurrence < b.occurrence)
}
fn reciprocal(k: u32, rank: u32) -> f64 {
    1.0 / (f64::from(k) + f64::from(rank))
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn checked_add(a: usize, b: usize) -> Result<usize, ModelError> {
    a.checked_add(b)
        .ok_or_else(|| invalid("ranking allocation size overflow"))
}
fn checked_mul(a: usize, b: usize) -> Result<usize, ModelError> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("ranking allocation size overflow"))
}
