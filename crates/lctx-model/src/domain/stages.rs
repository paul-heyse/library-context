//! Static typed producer inventories and publication dependencies. Execution owns completed inputs.
use super::attribution::FactFamily;
use super::{ContentHash, Key, KeySink, ModelError, Record, ValidatedModel};
use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOutcome {
    Complete,
    Partial,
    Unavailable,
    Failed,
    NotRequested,
}
impl ProviderOutcome {
    pub const ALL: [Self; 5] = [
        Self::Complete,
        Self::Partial,
        Self::Unavailable,
        Self::Failed,
        Self::NotRequested,
    ];
    /// The stored code. Append-only: a new outcome takes the next code; none is renumbered.
    pub fn code(self) -> i16 {
        match self {
            Self::Complete => 0,
            Self::Partial => 1,
            Self::Unavailable => 2,
            Self::Failed => 3,
            Self::NotRequested => 4,
        }
    }
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Profile {
    Catalog,
    Behavioral,
}
impl Profile {
    pub const ALL: [Self; 2] = [Self::Catalog, Self::Behavioral];
    pub fn name(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Behavioral => "behavioral",
        }
    }
}
/// Effects are declared by the producer, never inferred from the relations it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Acquisition,
    Extraction,
    Store,
    Embedding,
}
impl Effect {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Acquisition => "acquisition",
            Self::Extraction => "extraction",
            Self::Store => "store",
            Self::Embedding => "embedding",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct RelationUse {
    pub(crate) type_id: TypeId,
    name: &'static str,
    transport: InputTransport,
    requirement: Option<InputRequirement>,
    validators: &'static [&'static str],
    prefix: Option<PublicationBoundary>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityPolicy {
    RequireComplete,
    ObserveAvailability,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRequirement {
    pub group: FactFamily,
    pub policy: AvailabilityPolicy,
}
/// Declared once with each input. A completed input creates a dependency without retaining batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTransport {
    Handoff,
    CompletedInput,
}
impl RelationUse {
    pub fn of<R: Record>() -> Self {
        Self {
            type_id: TypeId::of::<R>(),
            name: R::NAME,
            transport: InputTransport::Handoff,
            requirement: None,
            validators: &[],
            prefix: None,
        }
    }
    pub fn at_epoch(mut self, epoch: PublicationBoundary) -> Self {
        self.prefix = Some(epoch);
        self
    }
    pub fn prefix(self) -> Option<PublicationBoundary> {
        self.prefix
    }
    pub fn name(self) -> &'static str {
        self.name
    }
    pub fn completed<R: Record>() -> Self {
        Self {
            transport: InputTransport::CompletedInput,
            ..Self::of::<R>()
        }
    }
    pub fn completed_input(mut self) -> Self {
        self.transport = InputTransport::CompletedInput;
        self
    }
    pub fn validators(self) -> &'static [&'static str] {
        self.validators
    }
    pub fn transport(self) -> InputTransport {
        self.transport
    }
    pub fn availability(mut self, group: FactFamily, policy: AvailabilityPolicy) -> Self {
        self.requirement = Some(InputRequirement { group, policy });
        self
    }
    pub fn requirement(self) -> Option<InputRequirement> {
        self.requirement
    }
    pub fn validated_by(mut self, validators: &'static [&'static str]) -> Self {
        self.validators = validators;
        self
    }
    /// The use of a declared relation, for schedules built from a relation list.
    pub fn of_relation(relation: &super::Relation) -> Self {
        Self {
            type_id: relation.type_id(),
            name: relation.name(),
            transport: InputTransport::Handoff,
            requirement: None,
            validators: &[],
            prefix: None,
        }
    }
}
/// One scheduled family names its actual reporting provider. No implicit secondary supplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FamilyCoverage {
    pub family: FactFamily,
    pub provider: super::Id<super::attribution::Provider>,
}
#[derive(Debug, Clone)]
pub struct Stage {
    pub captured_binding: Option<super::producer_contract::CapturedProducerBinding>,
    pub name: &'static str,
    pub inputs: Vec<RelationUse>,
    pub outputs: Vec<RelationUse>,
    /// Shared vocabulary this stage hands to another stage's output (ADR-0089). The writer runs
    /// after every contributor and emits each identity once.
    pub contributes: Vec<RelationUse>,
    /// Exact provider per fact family reported by this stage.
    pub coverage: Vec<FamilyCoverage>,
    pub profiles: Vec<Profile>,
    pub effect: Effect,
    pub code: ContentHash,
    pub configuration: ContentHash,
}
impl Stage {
    /// Required checks come from relation owners plus explicitly named shared input checks.
    /// A stage cannot opt out of its completed input relation's own invariants.
    pub fn read_invariants<'m>(
        &self,
        model: &'m ValidatedModel,
    ) -> Result<Vec<&'m super::Invariant>, ModelError> {
        let mut names = BTreeSet::new();
        for input in self
            .inputs
            .iter()
            .filter(|i| i.transport == InputTransport::CompletedInput)
        {
            let relation = model
                .relation(input.name)
                .ok_or_else(|| ModelError::Invalid("undeclared completed input".into()))?;
            names.extend(relation.invariant_refs().iter().copied());
            names.extend(input.validators.iter().copied());
        }
        names
            .into_iter()
            .map(|name| {
                model
                    .invariants()
                    .iter()
                    .find(|i| i.name == name)
                    .ok_or_else(|| {
                        ModelError::Invalid(format!("unknown shared input invariant {name}"))
                    })
            })
            .collect()
    }
    pub fn reads<R: Record>(&self) -> bool {
        self.inputs.iter().any(|r| r.type_id == TypeId::of::<R>())
    }
    pub fn writes<R: Record>(&self) -> bool {
        self.outputs.iter().any(|r| r.type_id == TypeId::of::<R>())
    }
    pub fn contributes_to<R: Record>(&self) -> bool {
        self.contributes
            .iter()
            .any(|r| r.type_id == TypeId::of::<R>())
    }
    /// The schedule binds input declaration roles in order. Output inventories are sets.
    pub fn digest(&self) -> ContentHash {
        let mut digest = KeySink::new("stage-declaration");
        self.encode(&mut digest);
        digest.finish()
    }
    fn encode(&self, digest: &mut KeySink) {
        self.captured_binding
            .as_ref()
            .map(|binding| binding.contract)
            .encode(digest);
        digest.part(b"stage", self.name.as_bytes());
        digest.part(b"effect", self.effect.name().as_bytes());
        digest.part(b"code", &self.code.0);
        digest.part(b"configuration", &self.configuration.0);
        digest.part(b"input-roles", &(self.inputs.len() as u64).to_le_bytes());
        let mut writes: Vec<_> = self.outputs.iter().map(|r| r.name).collect();
        writes.sort();
        let mut contributes: Vec<_> = self.contributes.iter().map(|r| r.name).collect();
        contributes.sort();
        for input in &self.inputs {
            digest.part(b"read", input.name.as_bytes());
            if let Some(epoch) = input.prefix {
                digest.part(b"prefix", &[epoch.code()]);
            }
            digest.part(
                b"transport",
                match input.transport {
                    InputTransport::Handoff => b"handoff",
                    InputTransport::CompletedInput => b"completed-input",
                },
            );
            if let Some(requirement) = input.requirement {
                requirement.group.encode(digest);
                digest.part(
                    b"availability",
                    match requirement.policy {
                        AvailabilityPolicy::RequireComplete => b"require-complete",
                        AvailabilityPolicy::ObserveAvailability => b"observe-availability",
                    },
                );
            }
            let mut validators = input.validators.to_vec();
            validators.sort_unstable();
            for validator in validators {
                digest.part(b"input-validator", validator.as_bytes());
            }
        }
        for name in writes {
            digest.part(b"write", name.as_bytes());
        }
        for name in contributes {
            digest.part(b"contribute", name.as_bytes());
        }
        let mut coverage = self.coverage.clone();
        coverage.sort();
        for grant in coverage {
            Key::encode(&grant.family, digest);
            Key::encode(&grant.provider, digest);
        }
    }
}
/// Finite named publication identity. Codes are append-only, never execution order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum PublicationBoundary {
    Facts = 0,
    Dispatch = 1,
    BaseSemantic = 2,
    ExecutionModel = 3,
    Summary = 4,
    CatalogSynthesis = 5,
    Local = 6,
    BaseEvaluation = 7,
    BaseCompletion = 8,
    SourceCall = 9,
    EnrichedExecution = 10,
    Model = 11,
    Structural = 12,
    Analytic = 13,
    CatalogCore = 14,
    CatalogEvidence = 15,
    Selection = 16,
    Synthesis = 17,
    Retrieval = 18,
    AnalyticEmbedding = 19,
}
impl PublicationBoundary {
    pub const ALL: [Self; 20] = [
        Self::Facts,
        Self::Dispatch,
        Self::BaseSemantic,
        Self::ExecutionModel,
        Self::Summary,
        Self::CatalogSynthesis,
        Self::Local,
        Self::BaseEvaluation,
        Self::BaseCompletion,
        Self::SourceCall,
        Self::EnrichedExecution,
        Self::Model,
        Self::Structural,
        Self::Analytic,
        Self::CatalogCore,
        Self::CatalogEvidence,
        Self::Selection,
        Self::Synthesis,
        Self::Retrieval,
        Self::AnalyticEmbedding,
    ];
    /// Stable persisted boundary names; Rust Debug output is never an identity contract.
    pub fn name(self) -> &'static str {
        match self {
            Self::Facts => "Facts",
            Self::Dispatch => "Dispatch",
            Self::BaseSemantic => "BaseSemantic",
            Self::ExecutionModel => "ExecutionModel",
            Self::Summary => "Summary",
            Self::CatalogSynthesis => "CatalogSynthesis",
            Self::Local => "Local",
            Self::BaseEvaluation => "BaseEvaluation",
            Self::BaseCompletion => "BaseCompletion",
            Self::SourceCall => "SourceCall",
            Self::EnrichedExecution => "EnrichedExecution",
            Self::Model => "Model",
            Self::Structural => "Structural",
            Self::Analytic => "Analytic",
            Self::CatalogCore => "CatalogCore",
            Self::CatalogEvidence => "CatalogEvidence",
            Self::Selection => "Selection",
            Self::Synthesis => "Synthesis",
            Self::Retrieval => "Retrieval",
            Self::AnalyticEmbedding => "AnalyticEmbedding",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|boundary| boundary.name() == name)
    }
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|boundary| boundary.code() == code)
    }
}
/// Contiguous position in one registered schedule. Only the model's mapping constructs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixOrdinal {
    ordinal: u16,
    boundary: PublicationBoundary,
    schedule: ContentHash,
}
impl Ord for PrefixOrdinal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.schedule.0, self.ordinal, self.boundary).cmp(&(
            other.schedule.0,
            other.ordinal,
            other.boundary,
        ))
    }
}
impl PartialOrd for PrefixOrdinal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PrefixOrdinal {
    pub fn ordinal(self) -> u16 {
        self.ordinal
    }
    pub fn boundary(self) -> PublicationBoundary {
        self.boundary
    }
    pub fn schedule(self) -> ContentHash {
        self.schedule
    }
    pub fn view(self, relation: &str) -> String {
        format!("__v{}_{}", self.ordinal, relation)
    }
    pub fn earlier(self, other: Self) -> Result<Self, ModelError> {
        if self.schedule != other.schedule {
            return Err(ModelError::Invalid("foreign publication prefix".into()));
        }
        Ok(if self.ordinal <= other.ordinal {
            self
        } else {
            other
        })
    }
}
/// Checked persisted boundary-to-order mapping, bound to the registered schedule digest.
/// Restoring metadata does not grant completed input reads: completed sources and attempt permits do.
#[derive(Debug, Clone)]
pub struct PublicationOrder {
    schedule: ContentHash,
    boundaries: Vec<PublicationBoundary>,
}
impl PublicationOrder {
    /// Declaration planning compares ordinals but confers no runtime prefix authority.
    pub fn planning(groups: &[PublicationGroup]) -> Result<Self, ModelError> {
        let entries = Self::group_entries(groups)?;
        Self::registered(ContentHash::of(b"unbound publication order"), &entries)
    }
    fn group_entries(
        groups: &[PublicationGroup],
    ) -> Result<Vec<(u16, PublicationBoundary)>, ModelError> {
        groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                Ok((
                    u16::try_from(i)
                        .map_err(|_| ModelError::Invalid("publication order overflow".into()))?,
                    g.epoch,
                ))
            })
            .collect()
    }

    pub fn registered(
        schedule: ContentHash,
        entries: &[(u16, PublicationBoundary)],
    ) -> Result<Self, ModelError> {
        let mut seen = BTreeSet::new();
        for (index, (ordinal, boundary)) in entries.iter().enumerate() {
            if usize::from(*ordinal) != index
                || !seen.insert(*boundary)
                || (index == 0 && *boundary != PublicationBoundary::Facts)
            {
                return Err(ModelError::Invalid(
                    "publication mapping must be contiguous with unique boundaries and facts first"
                        .into(),
                ));
            }
        }
        Ok(Self {
            schedule,
            boundaries: entries.iter().map(|(_, b)| *b).collect(),
        })
    }
    pub fn decode(&self, ordinal: u16) -> Result<PrefixOrdinal, ModelError> {
        let boundary = *self
            .boundaries
            .get(usize::from(ordinal))
            .ok_or_else(|| ModelError::Invalid("unregistered publication ordinal".into()))?;
        Ok(PrefixOrdinal {
            ordinal,
            boundary,
            schedule: self.schedule,
        })
    }
    pub fn resolve(&self, boundary: PublicationBoundary) -> Result<PrefixOrdinal, ModelError> {
        let ordinal = self
            .boundaries
            .iter()
            .position(|b| *b == boundary)
            .ok_or_else(|| ModelError::Invalid("undeclared publication boundary".into()))?;
        self.decode(u16::try_from(ordinal).expect("finite boundary set"))
    }
    pub fn validate(&self, prefix: PrefixOrdinal) -> Result<(), ModelError> {
        if self.decode(prefix.ordinal)? != prefix {
            return Err(ModelError::Invalid("foreign publication prefix".into()));
        }
        Ok(())
    }
}
/// Semantic vocabulary families. Value sharing does not make a relation vocabulary.
pub fn is_vocabulary(name: &str) -> bool {
    use super::{
        assertion::AssertionQualification,
        assumptions::{Assumption, AssumptionSet, AssumptionSetMember, AssumptionUniverse},
        conditions::{Condition, ConditionNode, EvaluationAtom},
        value::{
            AccessPath, Literal, LiteralSet, LiteralSetMember, PathSegment, Place, PlaceRoot,
            Predicate,
        },
    };
    [
        Assumption::NAME,
        AssumptionSet::NAME,
        AssumptionSetMember::NAME,
        AssumptionUniverse::NAME,
        Literal::NAME,
        LiteralSet::NAME,
        LiteralSetMember::NAME,
        PlaceRoot::NAME,
        PathSegment::NAME,
        AccessPath::NAME,
        Place::NAME,
        Predicate::NAME,
        EvaluationAtom::NAME,
        ConditionNode::NAME,
        Condition::NAME,
        AssertionQualification::NAME,
    ]
    .contains(&name)
}
/// Relations whose immutable streams may grow across named publication epochs.
/// Canonical embedding winners share transport, never semantic vocabulary ownership.
pub fn is_epoch_shared(name: &str) -> bool {
    is_vocabulary(name)
        || matches!(
            name,
            "embedding_full_values" | "embedding_projected_values" | "uniform_not_requested"
        )
}
#[derive(Debug, Clone)]
pub struct PublicationGroup {
    pub epoch: PublicationBoundary,
    pub stages: Vec<&'static str>,
}
impl PublicationGroup {
    pub fn new(epoch: PublicationBoundary, stages: Vec<&'static str>) -> Self {
        Self { epoch, stages }
    }
}
#[derive(Debug, Clone)]
pub struct ScheduledPublication {
    /// Named identity, distinct from this group's schedule-assigned prefix position.
    pub epoch: PublicationBoundary,
    pub stages: Vec<&'static str>,
    prefix: PrefixOrdinal,
}
impl ScheduledPublication {
    pub fn prefix(&self) -> PrefixOrdinal {
        self.prefix
    }
}
/// One ordered role resolved to a finite immutable contributor selection. This recipe
/// grants no read authority until the runtime has completed and bound every owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSelection {
    pub role: usize,
    pub relation: &'static str,
    pub requested: Option<PublicationBoundary>,
    pub resolved: Option<PublicationBoundary>,
    pub producers: BTreeSet<&'static str>,
}
#[derive(Debug)]
pub struct Schedule {
    groups: Vec<ScheduledPublication>,
    publication_order: PublicationOrder,
    prerequisites: BTreeMap<&'static str, BTreeSet<&'static str>>,
    selections: BTreeMap<&'static str, Vec<InputSelection>>,
    prefixes: BTreeMap<PublicationBoundary, Vec<InputSelection>>,
    stages: Vec<Stage>,
    model: ContentHash,
    digest: ContentHash,
    profile: Profile,
}
impl super::HeapSize for Schedule {
    fn heap_bytes(&self) -> usize {
        fn selections(rows: &[InputSelection], capacity: usize) -> usize {
            rows.iter().fold(
                capacity.saturating_mul(size_of::<InputSelection>()),
                |bytes, row| {
                    bytes.saturating_add(
                        row.producers
                            .len()
                            .saturating_mul(size_of::<&'static str>() + 32),
                    )
                },
            )
        }
        let mut bytes = self
            .groups
            .capacity()
            .saturating_mul(size_of::<ScheduledPublication>());
        for group in &self.groups {
            bytes = bytes.saturating_add(
                group
                    .stages
                    .capacity()
                    .saturating_mul(size_of::<&'static str>()),
            );
        }
        bytes = bytes.saturating_add(
            self.publication_order
                .boundaries
                .capacity()
                .saturating_mul(size_of::<PublicationBoundary>()),
        );
        bytes = bytes.saturating_add(
            self.prerequisites
                .len()
                .saturating_mul(size_of::<(&'static str, BTreeSet<&'static str>)>() + 32),
        );
        for prerequisites in self.prerequisites.values() {
            bytes = bytes.saturating_add(
                prerequisites
                    .len()
                    .saturating_mul(size_of::<&'static str>() + 32),
            );
        }
        bytes = bytes.saturating_add(
            self.selections
                .len()
                .saturating_mul(size_of::<(&'static str, Vec<InputSelection>)>() + 32),
        );
        for rows in self.selections.values() {
            bytes = bytes.saturating_add(selections(rows, rows.capacity()));
        }
        bytes = bytes.saturating_add(
            self.prefixes
                .len()
                .saturating_mul(size_of::<(PublicationBoundary, Vec<InputSelection>)>() + 32),
        );
        for rows in self.prefixes.values() {
            bytes = bytes.saturating_add(selections(rows, rows.capacity()));
        }
        bytes = bytes.saturating_add(self.stages.capacity().saturating_mul(size_of::<Stage>()));
        for stage in &self.stages {
            bytes = bytes
                .saturating_add(
                    stage
                        .inputs
                        .capacity()
                        .saturating_mul(size_of::<RelationUse>()),
                )
                .saturating_add(
                    stage
                        .outputs
                        .capacity()
                        .saturating_mul(size_of::<RelationUse>()),
                )
                .saturating_add(
                    stage
                        .contributes
                        .capacity()
                        .saturating_mul(size_of::<RelationUse>()),
                )
                .saturating_add(
                    stage
                        .coverage
                        .capacity()
                        .saturating_mul(size_of::<FamilyCoverage>()),
                )
                .saturating_add(
                    stage
                        .profiles
                        .capacity()
                        .saturating_mul(size_of::<Profile>()),
                )
                .saturating_add(
                    stage
                        .captured_binding
                        .as_ref()
                        .map_or(0, |binding| binding.heap_bytes()),
                );
        }
        bytes
    }
}
impl Schedule {
    pub fn build(
        model: &ValidatedModel,
        stages: Vec<Stage>,
        required: &[RelationUse],
        profile: Profile,
    ) -> Result<Self, ModelError> {
        let assembly = stages
            .iter()
            .filter(|s| {
                s.profiles.contains(&profile) && s.outputs.iter().any(|r| is_epoch_shared(r.name()))
            })
            .map(|s| s.name)
            .collect::<Vec<_>>();
        let groups = if assembly.is_empty() {
            vec![]
        } else {
            vec![PublicationGroup::new(PublicationBoundary::Facts, assembly)]
        };
        Self::build_with_publications(model, stages, required, profile, groups)
    }
    pub fn build_with_publications(
        model: &ValidatedModel,
        stages: Vec<Stage>,
        required: &[RelationUse],
        profile: Profile,
        mut groups: Vec<PublicationGroup>,
    ) -> Result<Self, ModelError> {
        for group in &mut groups {
            group.stages.sort_unstable();
        }
        let entries = PublicationOrder::group_entries(&groups)?;
        let raw_order = PublicationOrder::planning(&groups)?;
        let mut grouped = BTreeMap::new();
        for (index, group) in groups.iter().enumerate() {
            if group.stages.is_empty() {
                return Err(ModelError::Invalid(
                    "publication epochs must be a contiguous finite prefix".into(),
                ));
            }
            for name in &group.stages {
                if grouped.insert(*name, index as u16).is_some()
                    || !stages
                        .iter()
                        .any(|s| s.name == *name && s.profiles.contains(&profile))
                {
                    return Err(ModelError::Invalid(
                        "unknown or duplicate publication member".into(),
                    ));
                }
            }
        }
        let members: HashSet<_> = model
            .relations()
            .iter()
            .map(super::Relation::type_id)
            .collect();
        let mut names = HashSet::new();
        for stage in &stages {
            if !names.insert(stage.name) {
                return Err(ModelError::Invalid(format!(
                    "duplicate stage {}",
                    stage.name
                )));
            }
            if stage.profiles.is_empty()
                || stage
                    .profiles
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != stage.profiles.len()
            {
                return Err(ModelError::Invalid(format!(
                    "invalid profiles for {}",
                    stage.name
                )));
            }
        }
        let stages: Vec<_> = stages
            .into_iter()
            .filter(|s| s.profiles.contains(&profile))
            .collect();
        names.clear();
        let mut writers: HashMap<TypeId, usize> = HashMap::new();
        let mut epoch_writers = BTreeSet::new();
        for (i, stage) in stages.iter().enumerate() {
            if stage.name.is_empty() || stage.outputs.is_empty() {
                return Err(ModelError::Invalid(
                    "stage needs a name and declared outputs".into(),
                ));
            }
            if stage
                .outputs
                .iter()
                .chain(&stage.contributes)
                .any(|r| r.transport != InputTransport::Handoff)
            {
                return Err(ModelError::Invalid(
                    "completed input transport applies only to inputs".into(),
                ));
            }
            let mut groups = BTreeMap::new();
            for requirement in stage.inputs.iter().filter_map(|input| input.requirement) {
                if groups
                    .insert(requirement.group, requirement.policy)
                    .is_some_and(|old| old != requirement.policy)
                {
                    return Err(ModelError::Invalid(
                        "one availability policy is required per semantic input group".into(),
                    ));
                }
            }
            if !names.insert(stage.name) {
                return Err(ModelError::Invalid(format!(
                    "duplicate stage {}",
                    stage.name
                )));
            }
            for r in stage.inputs.iter().chain(&stage.outputs) {
                if !members.contains(&r.type_id) {
                    return Err(ModelError::Invalid(format!(
                        "undeclared relation {}",
                        r.name
                    )));
                }
            }
            if stage
                .inputs
                .iter()
                .map(|r| (r.type_id, r.prefix))
                .collect::<BTreeSet<_>>()
                .len()
                != stage.inputs.len()
            {
                return Err(ModelError::Invalid(format!(
                    "duplicate input for {}",
                    stage.name
                )));
            }
            for r in &stage.contributes {
                if !members.contains(&r.type_id) {
                    return Err(ModelError::Invalid(format!(
                        "undeclared relation {}",
                        r.name
                    )));
                }
                if stage.outputs.iter().any(|o| o.type_id == r.type_id) {
                    return Err(ModelError::Invalid(format!(
                        "{} cannot contribute to its own output {}",
                        stage.name, r.name
                    )));
                }
            }
            if stage.coverage.iter().collect::<BTreeSet<_>>().len() != stage.coverage.len() {
                return Err(ModelError::Invalid(format!(
                    "{} repeats a family/provider coverage grant",
                    stage.name
                )));
            }
            if stage
                .contributes
                .iter()
                .map(|r| r.type_id)
                .collect::<HashSet<_>>()
                .len()
                != stage.contributes.len()
            {
                return Err(ModelError::Invalid(format!(
                    "duplicate contribution or coverage family for {}",
                    stage.name
                )));
            }
            for r in &stage.outputs {
                if is_epoch_shared(r.name)
                    && let Some(epoch) = grouped.get(stage.name)
                    && !epoch_writers.insert((r.type_id, *epoch))
                {
                    return Err(ModelError::Invalid(format!(
                        "multiple writers for {} in one publication epoch",
                        r.name
                    )));
                }
                if let Some(old) = writers.get(&r.type_id).copied() {
                    if !is_epoch_shared(r.name)
                        || !grouped.contains_key(stage.name)
                        || !grouped.contains_key(stages[old].name)
                        || grouped[stage.name] == grouped[stages[old].name]
                    {
                        return Err(ModelError::Invalid(format!(
                            "multiple writers for {}",
                            r.name
                        )));
                    }
                    if grouped[stage.name] < grouped[stages[old].name] {
                        writers.insert(r.type_id, i);
                    }
                } else {
                    writers.insert(r.type_id, i);
                }
            }
        }
        for r in required {
            if !members.contains(&r.type_id) || !writers.contains_key(&r.type_id) {
                return Err(ModelError::Invalid(format!(
                    "missing required writer {}",
                    r.name
                )));
            }
        }
        // Execution depends on exact relation owners, not on unrelated publication members.
        // Shared relations include every contributor through the selected semantic boundary.
        let owners_for =
            |kind: TypeId, prefix: Option<PublicationBoundary>| -> Result<_, ModelError> {
                let writer = *writers.get(&kind).ok_or_else(|| {
                    ModelError::Invalid("missing writer for input relation".into())
                })?;
                let shared = stages[writer]
                    .outputs
                    .iter()
                    .find(|r| r.type_id == kind)
                    .expect("writer output");
                let resolved = if is_epoch_shared(shared.name) {
                    prefix.or_else(|| {
                        grouped
                            .get(stages[writer].name)
                            .map(|index| groups[*index as usize].epoch)
                    })
                } else {
                    None
                };
                let limit = resolved
                    .map(|epoch| raw_order.resolve(epoch).map(|p| p.ordinal()))
                    .transpose()?;
                let first = grouped.get(stages[writer].name).copied();
                let owners = stages
                    .iter()
                    .enumerate()
                    .filter(|(_, stage)| {
                        let produces = stage
                            .outputs
                            .iter()
                            .chain(&stage.contributes)
                            .any(|r| r.type_id == kind);
                        produces
                            && match limit {
                                Some(limit) => grouped
                                    .get(stage.name)
                                    .copied()
                                    .or(first)
                                    .is_some_and(|epoch| epoch <= limit),
                                None => stage.name == stages[writer].name,
                            }
                    })
                    .map(|(i, _)| i)
                    .collect::<BTreeSet<_>>();
                if owners.is_empty() {
                    return Err(ModelError::Invalid(
                        "input has no owner at its requested boundary".into(),
                    ));
                }
                Ok((resolved, owners))
            };
        let mut dependencies: BTreeMap<usize, BTreeSet<usize>> =
            (0..stages.len()).map(|i| (i, BTreeSet::new())).collect();
        let mut selections = BTreeMap::new();
        for (i, stage) in stages.iter().enumerate() {
            let mut roles = Vec::new();
            for (role, input) in stage.inputs.iter().enumerate() {
                if input.prefix.is_some()
                    && (input.transport != InputTransport::CompletedInput
                        || !is_epoch_shared(input.name))
                {
                    return Err(ModelError::Invalid(
                        "epoch reads require a shared completed input".into(),
                    ));
                }
                if input.prefix.is_none()
                    && is_epoch_shared(input.name)
                    && stages
                        .iter()
                        .filter(|s| s.outputs.iter().any(|o| o.type_id == input.type_id))
                        .count()
                        > 1
                {
                    return Err(ModelError::Invalid(
                        "a growing shared input needs an explicit closed epoch".into(),
                    ));
                }
                let (resolved, owners) = owners_for(input.type_id, input.prefix)?;
                dependencies
                    .get_mut(&i)
                    .expect("stage index")
                    .extend(&owners);
                roles.push(InputSelection {
                    role,
                    relation: input.name,
                    requested: input.prefix,
                    resolved,
                    producers: owners.into_iter().map(|owner| stages[owner].name).collect(),
                });
            }
            for invariant in stage.read_invariants(model)? {
                for input in &invariant.inputs {
                    // Inferred ordinary Facts may be omitted by the selected profile.
                    // Their semantic coverage remains checked by admission; explicit ports
                    // above always require a real writer. Preserve this existing distinction.
                    if !writers.contains_key(&input.type_id()) {
                        continue;
                    }
                    let prefix = stage
                        .inputs
                        .iter()
                        .find(|r| r.type_id == input.type_id() && r.prefix == input.prefix())
                        .and_then(|r| r.prefix)
                        .or(input.prefix());
                    let (_, owners) = owners_for(input.type_id(), prefix)?;
                    dependencies
                        .get_mut(&i)
                        .expect("stage index")
                        .extend(owners);
                }
            }
            for relation in &stage.contributes {
                let writer = writers.get(&relation.type_id).ok_or_else(|| {
                    ModelError::Invalid(format!("contribution to {} has no writer", relation.name))
                })?;
                dependencies.get_mut(writer).expect("stage index").insert(i);
            }
            selections.insert(stage.name, roles);
        }
        if dependencies
            .iter()
            .any(|(stage, deps)| deps.contains(stage))
        {
            return Err(ModelError::Invalid(
                "stage cycle: a stage cannot read its unfinished output or assembly".into(),
            ));
        }
        let prerequisites = dependencies
            .iter()
            .map(|(stage, deps)| {
                (
                    stages[*stage].name,
                    deps.iter().map(|dep| stages[*dep].name).collect(),
                )
            })
            .collect();
        let mut prefixes = BTreeMap::new();
        for group in &groups {
            let mut roles = Vec::new();
            let mut relations = BTreeMap::new();
            for stage in &stages {
                for output in &stage.outputs {
                    if is_epoch_shared(output.name) {
                        relations.insert(output.name, output);
                    }
                }
            }
            for input in relations.into_values() {
                if let Ok((resolved, owners)) = owners_for(input.type_id, Some(group.epoch)) {
                    roles.push(InputSelection {
                        role: roles.len(),
                        relation: input.name,
                        requested: Some(group.epoch),
                        resolved,
                        producers: owners.into_iter().map(|owner| stages[owner].name).collect(),
                    });
                }
            }
            prefixes.insert(group.epoch, roles);
        }
        let mut ordered = Vec::new();
        while !dependencies.is_empty() {
            let next = dependencies
                .iter()
                .filter(|(_, deps)| deps.is_empty())
                .map(|(i, _)| *i)
                .min_by_key(|i| stages[*i].name)
                .ok_or_else(|| ModelError::Invalid("stage cycle".into()))?;
            dependencies.remove(&next);
            for deps in dependencies.values_mut() {
                deps.remove(&next);
            }
            ordered.push(stages[next].clone());
        }
        let mut digest = KeySink::new("stage-schedule/exact-owner-readiness-v2");
        digest.part(b"model", &model.digest().0);
        digest.part(b"profile", profile.name().as_bytes());
        for stage in &ordered {
            stage.encode(&mut digest);
        }
        for (ordinal, group) in groups.iter().enumerate() {
            digest.part(b"publication-ordinal", &(ordinal as u16).to_le_bytes());
            digest.part(b"publication-boundary", &[group.epoch.code()]);
            let mut names = group.stages.clone();
            names.sort();
            for name in names {
                digest.part(b"publication-member", name.as_bytes());
            }
        }
        let digest = digest.finish();
        let publication_order = PublicationOrder::registered(digest, &entries)?;
        let groups = groups
            .into_iter()
            .map(|g| ScheduledPublication {
                epoch: g.epoch,
                prefix: publication_order.resolve(g.epoch).expect("checked mapping"),
                stages: g.stages,
            })
            .collect();
        Ok(Self {
            groups,
            publication_order,
            prerequisites,
            selections,
            prefixes,
            stages: ordered,
            model: model.digest(),
            digest,
            profile,
        })
    }
    pub fn publication_groups(&self) -> &[ScheduledPublication] {
        &self.groups
    }
    pub fn epoch_for(&self, stage: &str) -> Option<PublicationBoundary> {
        self.groups
            .iter()
            .find(|g| g.stages.contains(&stage))
            .map(|g| g.epoch)
    }
    pub fn publication_order(&self) -> &PublicationOrder {
        &self.publication_order
    }
    pub fn prefix_for(&self, boundary: PublicationBoundary) -> Result<PrefixOrdinal, ModelError> {
        self.publication_order.resolve(boundary)
    }
    pub fn prefix_selections(
        &self,
        boundary: PublicationBoundary,
    ) -> Result<&[InputSelection], ModelError> {
        self.prefixes
            .get(&boundary)
            .map(Vec::as_slice)
            .ok_or(ModelError::Invalid("unknown publication boundary".into()))
    }
    pub fn prerequisites(&self, stage: &str) -> Result<&BTreeSet<&'static str>, ModelError> {
        self.prerequisites
            .get(stage)
            .ok_or(ModelError::Invalid("unknown scheduled stage".into()))
    }
    pub fn selections(&self, stage: &str) -> Result<&[InputSelection], ModelError> {
        self.selections
            .get(stage)
            .map(Vec::as_slice)
            .ok_or(ModelError::Invalid("unknown scheduled stage".into()))
    }
    pub fn ready(
        &self,
        stage: &str,
        completed: &BTreeSet<&'static str>,
    ) -> Result<bool, ModelError> {
        Ok(self.prerequisites(stage)?.is_subset(completed))
    }
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }
    pub fn digest(&self) -> ContentHash {
        self.digest
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
}
