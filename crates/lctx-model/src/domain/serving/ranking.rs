//! Pure ranking of admitted occurrences. Scores aid discovery; they are not requirement evidence.
use super::identity::{ChannelIdentity, SnapshotHandle, PolicyIdentity, policy_identity};
use crate::domain::{
    ContentHash, Id, KeySink, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    resources::{Reservation, ResourceBudget},
    retrieval::{Family, Fragment, OriginalAnchor, Unit},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

const PREPARATION_BYTES_PER_ROW: usize = 768;
const FUSION_BYTES_PER_ROW: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NumericalLibrary {
    Bm25s0311,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NumericalBackend {
    Numpy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Bm25Method {
    Lucene,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tokenizer {
    LowercaseAsciiAlphanumeric,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWords {
    Discriminating,
    Present,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NumericalSettings {
    pub library: NumericalLibrary,
    pub backend: NumericalBackend,
    pub method: Bm25Method,
    pub k1: f64,
    pub b: f64,
    pub tokenizer: Tokenizer,
    pub member_query_words: QueryWords,
    pub unit_query_words: QueryWords,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingPolicy {
    pub revision: u32,
    pub numerical: NumericalSettings,
    pub rrf_k: u32,
}
impl Default for RankingPolicy {
    fn default() -> Self {
        Self {
            revision: 1,
            numerical: NumericalSettings {
                library: NumericalLibrary::Bm25s0311,
                backend: NumericalBackend::Numpy,
                method: Bm25Method::Lucene,
                k1: 1.5,
                b: 0.75,
                tokenizer: Tokenizer::LowercaseAsciiAlphanumeric,
                member_query_words: QueryWords::Discriminating,
                unit_query_words: QueryWords::Present,
            },
            rrf_k: 60,
        }
    }
}
impl RankingPolicy {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self != &Self::default() {
            return Err(invalid("unsupported ranking policy"));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<PolicyIdentity, ModelError> {
        self.validate()?;
        policy_identity(self)
    }
}

/// Matches the declared lowercase `[a-z0-9]+` tokenizer, including Unicode lowercase first.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
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
    ) -> Result<Self, ModelError> {
        let policy = policy.identity()?;
        let mut sink = KeySink::new("serving-vector-channel/v1");
        sink.part(b"policy", &policy.0.0);
        sink.part(b"spec", &spec.0);
        sink.part(b"query-vector", &query_vector.0);
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
    pub fragment: Id<Fragment>,
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
            self.fragment,
            self.context,
            self.anchor,
        )
            .cmp(&(
                other.target,
                other.family as i16,
                other.unit,
                other.fragment,
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
pub struct NumericalScore {
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
    pub numerical_score: f64,
    /// Contiguous one-based rank within this family/channel, after best-fragment selection.
    pub rank: u32,
    pub snapshot: SnapshotHandle,
    pub policy: PolicyIdentity,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankedHit {
    pub target: Target,
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

/// C0/E0 supplies the complete eligible universe and canonical joins. This owner never classifies.
#[derive(Debug)]
pub struct PreparedRanking {
    snapshot: SnapshotHandle,
    policy: RankingPolicy,
    policy_identity: PolicyIdentity,
    channels: BTreeMap<Channel, ChannelBinding>,
    eligible: BTreeSet<Target>,
    occurrences: BTreeSet<Occurrence>,
    budget: ResourceBudget,
    _reservation: Box<dyn Reservation>,
}
impl PreparedRanking {
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
            "serving ranking preparation",
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
                return Err(invalid("occurrence outside eligible universe"));
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
        scores: &[NumericalScore],
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
                return Err(invalid("exact-path promotion outside eligible universe"));
            }
            exact.insert(target);
        }
        let mut numerical = BTreeMap::<(Occurrence, Channel), Option<f64>>::new();
        for row in scores {
            if row.snapshot != self.snapshot {
                return Err(invalid("numerical snapshot differs"));
            }
            if !self.occurrences.contains(&row.occurrence) {
                return Err(invalid("numerical occurrence outside admitted closure"));
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
        let mut best = BTreeMap::<(i16, Channel, Target), RankingWitness>::new();
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
                continue;
            }
            stat.contributing_occurrences += 1;
            let witness = RankingWitness {
                occurrence,
                channel,
                channel_identity: self.channels[&channel].identity,
                numerical_score: score,
                rank: 0,
                snapshot: self.snapshot.clone(),
                policy: self.policy_identity,
            };
            let key = (occurrence.family as i16, channel, occurrence.target);
            if best.get(&key).is_none_or(|old| better(&witness, old)) {
                best.insert(key, witness);
            }
        }
        let mut winners: Vec<_> = best.into_values().collect();
        winners.sort_by(|a, b| {
            (a.occurrence.family as i16, a.channel)
                .cmp(&(b.occurrence.family as i16, b.channel))
                .then_with(|| b.numerical_score.total_cmp(&a.numerical_score))
                .then_with(|| a.occurrence.target.cmp(&b.occurrence.target))
        });
        let mut previous = None;
        let mut rank = 0u32;
        let mut family_scores = BTreeMap::<(i16, Target), f64>::new();
        let mut witnesses = BTreeMap::<Target, Vec<RankingWitness>>::new();
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
                .entry((group.0, winner.occurrence.target))
                .or_default() += reciprocal(self.policy.rrf_k, rank);
            witnesses
                .entry(winner.occurrence.target)
                .or_default()
                .push(winner);
        }
        // Normalize each family's channel fusion to ranks, then give every family one vote.
        let mut family_order: Vec<_> = family_scores.into_iter().collect();
        family_order.sort_by(|a, b| {
            a.0.0
                .cmp(&b.0.0)
                .then_with(|| b.1.total_cmp(&a.1))
                .then_with(|| a.0.1.cmp(&b.0.1))
        });
        let mut totals = BTreeMap::<Target, f64>::new();
        let mut previous_family = None;
        rank = 0;
        for ((family, target), _) in family_order {
            if previous_family != Some(family) {
                previous_family = Some(family);
                rank = 0;
            }
            rank = rank
                .checked_add(1)
                .ok_or_else(|| invalid("ranking count exceeds u32"))?;
            *totals.entry(target).or_default() += reciprocal(self.policy.rrf_k, rank);
        }
        for target in &exact {
            totals.entry(*target).or_default();
        }
        let mut rows: Vec<_> = totals
            .into_iter()
            .map(|(target, score)| RankedHit {
                target,
                score,
                promoted: exact.contains(&target),
                witnesses: witnesses.remove(&target).unwrap_or_default(),
            })
            .collect();
        rows.sort_by(|a, b| {
            b.promoted
                .cmp(&a.promoted)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.target.cmp(&b.target))
        });
        Ok(RankedResults {
            rows,
            statistics,
            _reservation: reservation,
        })
    }
}
fn better(a: &RankingWitness, b: &RankingWitness) -> bool {
    a.numerical_score.total_cmp(&b.numerical_score) == Ordering::Greater
        || (a.numerical_score == b.numerical_score && a.occurrence < b.occurrence)
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

#[derive(Debug, Clone)]
pub struct TextOccurrence {
    pub occurrence: Occurrence,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LexicalDocument {
    pub id: ContentHash,
    pub family: Family,
    pub text: String,
    pub tokens: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentScore {
    pub document: ContentHash,
    pub score: Option<f64>,
}
#[derive(Debug)]
pub struct OccurrenceScores {
    rows: Vec<NumericalScore>,
    _reservation: Box<dyn Reservation>,
}
impl OccurrenceScores {
    pub fn rows(&self) -> &[NumericalScore] {
        &self.rows
    }
}
#[derive(Debug)]
pub struct QueryTokens {
    tokens: Vec<String>,
    _reservation: Box<dyn Reservation>,
}
impl QueryTokens {
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}
/// One numerical document per family/text, retaining every admitted contextual occurrence.
#[derive(Debug)]
pub struct LexicalCorpus {
    snapshot: SnapshotHandle,
    policy_identity: PolicyIdentity,
    eligible: BTreeSet<Target>,
    admitted: BTreeSet<Occurrence>,
    documents: Vec<LexicalDocument>,
    occurrences: BTreeMap<ContentHash, BTreeSet<Occurrence>>,
    document_frequency: BTreeMap<(i16, String), usize>,
    family_size: BTreeMap<i16, usize>,
    query_words: QueryWords,
    _reservation: Box<dyn Reservation>,
}
impl PreparedRanking {
    pub fn prepare_lexical(&self, inputs: &[TextOccurrence]) -> Result<LexicalCorpus, ModelError> {
        let text_bytes = inputs.iter().try_fold(0usize, |size, input| {
            checked_add(size, checked_mul(input.text.len(), 128)?)
        })?;
        let bytes = checked_add(
            text_bytes,
            checked_mul(
                checked_add(inputs.len(), self.eligible.len())?,
                PREPARATION_BYTES_PER_ROW,
            )?,
        )?;
        let reservation = self.budget.reserve("serving lexical preparation", bytes)?;
        let mut documents = BTreeMap::<ContentHash, LexicalDocument>::new();
        let mut occurrences = BTreeMap::<ContentHash, BTreeSet<Occurrence>>::new();
        let mut fragment_text = BTreeMap::<Id<Fragment>, &str>::new();
        let mut covered = BTreeSet::new();
        for input in inputs {
            if !self.occurrences.contains(&input.occurrence) {
                return Err(invalid("lexical input outside admitted closure"));
            }
            if let Some(previous) = fragment_text.insert(input.occurrence.fragment, &input.text)
                && previous != input.text
            {
                return Err(invalid("one fragment has conflicting lexical text"));
            }
            let mut sink = KeySink::new("serving-lexical-document/v1");
            sink.part(b"family", &(input.occurrence.family as i16).to_le_bytes());
            sink.part(b"text", input.text.as_bytes());
            let id = sink.finish();
            if let Some(previous) = documents.get(&id) {
                if previous.text != input.text || previous.family != input.occurrence.family {
                    return Err(invalid("lexical document identity collision"));
                }
            } else {
                documents.insert(
                    id,
                    LexicalDocument {
                        id,
                        family: input.occurrence.family,
                        text: input.text.clone(),
                        tokens: tokenize(&input.text),
                    },
                );
            }
            occurrences.entry(id).or_default().insert(input.occurrence);
            covered.insert(input.occurrence);
        }
        if covered != self.occurrences {
            return Err(invalid("lexical preparation omits admitted occurrences"));
        }
        let mut documents: Vec<_> = documents.into_values().collect();
        documents.sort_by_key(|doc| (doc.family as i16, doc.id));
        let mut document_frequency = BTreeMap::new();
        let mut family_size = BTreeMap::new();
        for document in &documents {
            *family_size.entry(document.family as i16).or_default() += 1;
            for token in document.tokens.iter().collect::<BTreeSet<_>>() {
                *document_frequency
                    .entry((document.family as i16, token.clone()))
                    .or_default() += 1;
            }
        }
        let query_words = if self
            .eligible
            .iter()
            .any(|target| matches!(target, Target::Unit { .. }))
        {
            self.policy.numerical.unit_query_words
        } else {
            self.policy.numerical.member_query_words
        };
        Ok(LexicalCorpus {
            snapshot: self.snapshot.clone(),
            policy_identity: self.policy_identity,
            eligible: self.eligible.clone(),
            admitted: covered,
            documents,
            occurrences,
            document_frequency,
            family_size,
            query_words,
            _reservation: reservation,
        })
    }
}
impl LexicalCorpus {
    pub fn documents(&self) -> &[LexicalDocument] {
        &self.documents
    }
    /// Members abstain on fully shared vocabulary; independent evidence accepts present words.
    pub fn query_tokens(
        &self,
        request: &PreparedRanking,
        family: Family,
        query: &str,
    ) -> Result<QueryTokens, ModelError> {
        let channel = self.validate_request(request)?;
        let mut sink = KeySink::new("serving-lexical-channel/v1");
        sink.part(b"policy", &channel.policy.0.0);
        sink.part(b"query", query.as_bytes());
        if ChannelIdentity(sink.finish()) != channel.identity {
            return Err(invalid("lexical query differs from admitted channel"));
        }
        let reservation = request.budget.reserve(
            "serving lexical query conversion",
            checked_mul(query.len(), 128)?,
        )?;
        let size = self.family_size.get(&(family as i16)).copied().unwrap_or(0);
        let tokens = tokenize(query)
            .into_iter()
            .filter(|token| {
                let frequency = self
                    .document_frequency
                    .get(&(family as i16, token.clone()))
                    .copied()
                    .unwrap_or(0);
                frequency > 0 && (self.query_words == QueryWords::Present || frequency < size)
            })
            .collect();
        Ok(QueryTokens {
            tokens,
            _reservation: reservation,
        })
    }
    /// Validate the service-wide scorer output, then select exact request-admitted occurrences.
    /// Document frequencies stay service-wide; C0/E0 alone supplies request eligibility.
    pub fn expand_scores(
        &self,
        request: &PreparedRanking,
        scores: &[DocumentScore],
    ) -> Result<OccurrenceScores, ModelError> {
        let channel = self.validate_request(request)?;
        let count = request.occurrences.len();
        let bytes = checked_mul(checked_add(count, scores.len())?, PREPARATION_BYTES_PER_ROW)?;
        let reservation = request
            .budget
            .reserve("serving lexical score conversion", bytes)?;
        let mut numerical = BTreeMap::<ContentHash, Option<f64>>::new();
        for row in scores {
            if !self.occurrences.contains_key(&row.document) {
                return Err(invalid(
                    "numerical document outside admitted lexical corpus",
                ));
            }
            if row
                .score
                .is_some_and(|score| !score.is_finite() || score < 0.0)
            {
                return Err(invalid("invalid lexical document score"));
            }
            let normalized = row
                .score
                .map(|score| if score == 0.0 { 0.0 } else { score });
            if let Some(previous) = numerical.insert(row.document, normalized)
                && previous != normalized
            {
                return Err(invalid("conflicting duplicate lexical document score"));
            }
        }
        let mut rows = Vec::with_capacity(count);
        for (document, occurrences) in &self.occurrences {
            for occurrence in occurrences {
                if !request.occurrences.contains(occurrence) {
                    continue;
                }
                rows.push(NumericalScore {
                    snapshot: self.snapshot.clone(),
                    occurrence: *occurrence,
                    channel: Channel::Lexical,
                    channel_identity: channel.identity,
                    score: numerical.get(document).copied().flatten(),
                });
            }
        }
        Ok(OccurrenceScores {
            rows,
            _reservation: reservation,
        })
    }
    fn validate_request(&self, request: &PreparedRanking) -> Result<ChannelBinding, ModelError> {
        if request.snapshot != self.snapshot.clone()
            || request.policy_identity != self.policy_identity
            || !request.eligible.is_subset(&self.eligible)
        {
            return Err(invalid(
                "lexical request differs from admitted corpus snapshot, policy or universe",
            ));
        }
        let channel = request
            .channel(Channel::Lexical)
            .ok_or_else(|| invalid("lexical request channel is inactive"))?;
        // No untrusted string or cross-relation ID dispatch: these are admitted occurrence keys.
        if !request.occurrences.is_subset(&self.admitted) {
            return Err(invalid(
                "request occurrence absent from admitted lexical corpus",
            ));
        }
        Ok(channel)
    }
}

/// Member ranking shares the numerical document domain and document frequencies of independent
/// evidence. Only canonical member occurrence joins differ; no second text index is constructed.
#[derive(Debug)]
pub struct MemberLexicalCorpus {
    source: std::sync::Arc<LexicalCorpus>,
    eligible: BTreeSet<Target>,
    admitted: BTreeSet<Occurrence>,
    occurrences: BTreeMap<ContentHash, BTreeSet<Occurrence>>,
    _reservation: Box<dyn Reservation>,
}
impl PreparedRanking {
    pub fn project_member_lexical(
        &self,
        source: std::sync::Arc<LexicalCorpus>,
    ) -> Result<MemberLexicalCorpus, ModelError> {
        if self.snapshot != source.snapshot
            || self.policy_identity != source.policy_identity
            || self
                .eligible
                .iter()
                .any(|t| !matches!(t, Target::Member { .. }))
        {
            return Err(invalid("member lexical projection domain"));
        }
        let reservation = self.budget.reserve(
            "member lexical occurrence projection",
            checked_mul(
                checked_add(
                    checked_add(self.eligible.len(), self.occurrences.len())?,
                    source.occurrences.values().map(BTreeSet::len).sum(),
                )?,
                PREPARATION_BYTES_PER_ROW,
            )?,
        )?;
        let mut document_by_occurrence = BTreeMap::new();
        for (document, occurrences) in &source.occurrences {
            for occurrence in occurrences {
                if !matches!(occurrence.target,Target::Unit{unit} if unit==occurrence.unit) {
                    return Err(invalid(
                        "member projection requires independent evidence corpus",
                    ));
                }
                document_by_occurrence.insert(*occurrence, *document);
            }
        }
        let mut occurrences = BTreeMap::<_, BTreeSet<_>>::new();
        for occurrence in &self.occurrences {
            let original = Occurrence {
                target: Target::Unit {
                    unit: occurrence.unit,
                },
                ..*occurrence
            };
            let document = document_by_occurrence
                .get(&original)
                .ok_or_else(|| invalid("member occurrence lacks canonical evidence occurrence"))?;
            occurrences
                .entry(*document)
                .or_default()
                .insert(*occurrence);
        }
        Ok(MemberLexicalCorpus {
            source,
            eligible: self.eligible.clone(),
            admitted: self.occurrences.clone(),
            occurrences,
            _reservation: reservation,
        })
    }
}
impl MemberLexicalCorpus {
    pub fn documents(&self) -> &[LexicalDocument] {
        self.source.documents()
    }
    fn validate(&self, request: &PreparedRanking) -> Result<ChannelBinding, ModelError> {
        if request.snapshot != self.source.snapshot
            || request.policy_identity != self.source.policy_identity
            || !request.eligible.is_subset(&self.eligible)
            || !request.occurrences.is_subset(&self.admitted)
        {
            return Err(invalid(
                "member lexical request differs from admitted corpus",
            ));
        }
        request
            .channel(Channel::Lexical)
            .ok_or_else(|| invalid("member lexical channel inactive"))
    }
    pub fn query_tokens(
        &self,
        request: &PreparedRanking,
        family: Family,
        query: &str,
    ) -> Result<QueryTokens, ModelError> {
        let channel = self.validate(request)?;
        if ChannelBinding::lexical(request.policy(), query)?.identity() != channel.identity() {
            return Err(invalid("member lexical query changed"));
        }
        let reservation = request.budget.reserve(
            "member lexical query conversion",
            checked_mul(query.len(), 128)?,
        )?;
        let size = self
            .source
            .family_size
            .get(&(family as i16))
            .copied()
            .unwrap_or(0);
        let tokens = tokenize(query)
            .into_iter()
            .filter(|token| {
                let frequency = self
                    .source
                    .document_frequency
                    .get(&(family as i16, token.clone()))
                    .copied()
                    .unwrap_or(0);
                frequency > 0 && frequency < size
            })
            .collect();
        Ok(QueryTokens {
            tokens,
            _reservation: reservation,
        })
    }
    pub fn expand_scores(
        &self,
        request: &PreparedRanking,
        scores: &[DocumentScore],
    ) -> Result<OccurrenceScores, ModelError> {
        let channel = self.validate(request)?;
        let reservation = request.budget.reserve(
            "member lexical score conversion",
            checked_mul(
                checked_add(request.occurrences.len(), scores.len())?,
                PREPARATION_BYTES_PER_ROW,
            )?,
        )?;
        let mut numerical = BTreeMap::new();
        for row in scores {
            if !self.source.occurrences.contains_key(&row.document)
                || row.score.is_some_and(|v| !v.is_finite() || v < 0.0)
            {
                return Err(invalid("member lexical score outside numerical corpus"));
            }
            let value = row.score.map(|v| if v == 0.0 { 0.0 } else { v });
            if let Some(previous) = numerical.insert(row.document, value)
                && previous != value
            {
                return Err(invalid("conflicting member lexical document score"));
            }
        }
        let mut rows = Vec::with_capacity(request.occurrences.len());
        for (document, occurrences) in &self.occurrences {
            for occurrence in occurrences {
                if request.occurrences.contains(occurrence) {
                    rows.push(NumericalScore {
                        snapshot: request.snapshot.clone(),
                        occurrence: *occurrence,
                        channel: Channel::Lexical,
                        channel_identity: channel.identity,
                        score: numerical.get(document).copied().flatten(),
                    });
                }
            }
        }
        Ok(OccurrenceScores {
            rows,
            _reservation: reservation,
        })
    }
}
