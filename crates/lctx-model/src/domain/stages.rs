//! Typed stage declarations are the sole writer authority.
use super::attribution::FactFamily;
use super::batching::{BatchWriter, TransferLimits};
use super::resources::ResourceBudget;
use super::{Batch, ContentHash, Key, KeySink, ModelError, Record, ValidatedModel};
use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::Arc,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
}
impl Effect {
    fn name(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Acquisition => "acquisition",
            Self::Extraction => "extraction",
            Self::Store => "store",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct RelationUse {
    pub(crate) type_id: TypeId,
    name: &'static str,
    transport: InputTransport,
}
/// Declared once with each input. A store read creates a dependency without retaining batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTransport { Handoff, CompletedStore }
impl RelationUse {
    pub fn of<R: Record>() -> Self {
        Self {
            type_id: TypeId::of::<R>(),
            name: R::NAME,
            transport: InputTransport::Handoff,
        }
    }
    pub fn name(self) -> &'static str {
        self.name
    }
    pub fn stored<R: Record>() -> Self {
        Self { transport: InputTransport::CompletedStore, ..Self::of::<R>() }
    }
    pub fn transport(self) -> InputTransport { self.transport }
    /// The use of a declared relation, for schedules built from a relation list.
    pub fn of_relation(relation: &super::Relation) -> Self {
        Self {
            type_id: relation.type_id(),
            name: relation.name(),
            transport: InputTransport::Handoff,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: &'static str,
    pub inputs: Vec<RelationUse>,
    pub outputs: Vec<RelationUse>,
    /// Shared vocabulary this stage hands to another stage's output (ADR-0089). The writer runs
    /// after every contributor and emits each identity once.
    pub contributes: Vec<RelationUse>,
    /// Fact families whose provider coverage this stage reports, and the provider that reports it.
    pub coverage: Vec<FactFamily>,
    pub provider: Option<super::Id<super::attribution::Provider>>,
    pub profiles: Vec<Profile>,
    pub effect: Effect,
    pub code: ContentHash,
    pub configuration: ContentHash,
}
impl Stage {
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
    /// The declaration's identity, as the schedule digest includes it; independent of list order.
    pub fn digest(&self) -> ContentHash {
        let mut digest = KeySink::new("stage-declaration");
        self.encode(&mut digest);
        digest.finish()
    }
    fn encode(&self, digest: &mut KeySink) {
        digest.part(b"stage", self.name.as_bytes());
        digest.part(b"effect", self.effect.name().as_bytes());
        digest.part(b"code", &self.code.0);
        digest.part(b"configuration", &self.configuration.0);
        let mut reads: Vec<_> = self.inputs.iter().collect();
        reads.sort_by_key(|r| r.name);
        let mut writes: Vec<_> = self.outputs.iter().map(|r| r.name).collect();
        writes.sort();
        let mut contributes: Vec<_> = self.contributes.iter().map(|r| r.name).collect();
        contributes.sort();
        for input in reads {
            digest.part(b"read", input.name.as_bytes());
            digest.part(b"transport", match input.transport {
                InputTransport::Handoff => b"handoff",
                InputTransport::CompletedStore => b"completed-store",
            });
        }
        for name in writes {
            digest.part(b"write", name.as_bytes());
        }
        for name in contributes {
            digest.part(b"contribute", name.as_bytes());
        }
        let mut coverage = self.coverage.clone();
        coverage.sort();
        for family in coverage {
            Key::encode(&family, digest);
        }
        if let Some(provider) = self.provider {
            Key::encode(&provider, digest);
        }
    }
}
#[derive(Debug)]
pub struct Schedule {
    stages: Vec<Stage>,
    model: ContentHash,
    digest: ContentHash,
    profile: Profile,
    dependencies: BTreeMap<&'static str, BTreeSet<&'static str>>,
    readers: HashMap<TypeId, usize>,
    /// Outputs that receive contributed vocabulary from another stage.
    contributed: HashSet<TypeId>,
}
impl Schedule {
    pub fn build(
        model: &ValidatedModel,
        stages: Vec<Stage>,
        required: &[RelationUse],
        profile: Profile,
    ) -> Result<Self, ModelError> {
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
        let mut writers = std::collections::HashMap::new();
        for (i, stage) in stages.iter().enumerate() {
            if stage.name.is_empty() || stage.outputs.is_empty() {
                return Err(ModelError::Invalid(
                    "stage needs a name and declared outputs".into(),
                ));
            }
            if stage.outputs.iter().chain(&stage.contributes).any(|r| r.transport != InputTransport::Handoff) {
                return Err(ModelError::Invalid("store transport applies only to inputs".into()));
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
                .map(|r| r.type_id)
                .collect::<HashSet<_>>()
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
            if stage.coverage.is_empty() != stage.provider.is_none() {
                return Err(ModelError::Invalid(format!(
                    "{} reports coverage exactly when it names its provider",
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
                || stage.coverage.iter().collect::<BTreeSet<_>>().len() != stage.coverage.len()
            {
                return Err(ModelError::Invalid(format!(
                    "duplicate contribution or coverage family for {}",
                    stage.name
                )));
            }
            for r in &stage.outputs {
                if writers.insert(r.type_id, i).is_some() {
                    return Err(ModelError::Invalid(format!(
                        "multiple writers for {}",
                        r.name
                    )));
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
        let mut dependencies: BTreeMap<usize, BTreeSet<usize>> =
            (0..stages.len()).map(|i| (i, BTreeSet::new())).collect();
        let mut readers = HashMap::new();
        let mut contributed = HashSet::new();
        for (i, stage) in stages.iter().enumerate() {
            for r in &stage.inputs {
                let writer = writers
                    .get(&r.type_id)
                    .ok_or_else(|| ModelError::Invalid(format!("missing writer for {}", r.name)))?;
                dependencies
                    .get_mut(&i)
                    .expect("stage index")
                    .insert(*writer);
                if r.transport == InputTransport::Handoff {
                    *readers.entry(r.type_id).or_insert(0usize) += 1;
                }
            }
            for r in &stage.contributes {
                let writer = writers.get(&r.type_id).ok_or_else(|| {
                    ModelError::Invalid(format!("contribution to {} has no writer", r.name))
                })?;
                dependencies.get_mut(writer).expect("stage index").insert(i);
                contributed.insert(r.type_id);
            }
        }
        let named: BTreeMap<_, _> = dependencies
            .iter()
            .map(|(i, deps)| {
                (
                    stages[*i].name,
                    deps.iter()
                        .map(|d| stages[*d].name)
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect();
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
        let mut digest = KeySink::new("stage-schedule");
        digest.part(b"model", &model.digest().0);
        digest.part(b"profile", profile.name().as_bytes());
        for stage in &ordered {
            stage.encode(&mut digest);
        }
        Ok(Self {
            stages: ordered,
            model: model.digest(),
            digest: digest.finish(),
            profile,
            dependencies: named,
            readers,
            contributed,
        })
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
    pub fn execute(&self) -> Execution<'_> {
        static NEXT_ATTEMPT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let identity = NEXT_ATTEMPT
            .try_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |n| n.checked_add(1),
            )
            .expect("attempt identity space exhausted");
        Execution {
            schedule: self,
            completed: BTreeMap::new(),
            sources: BTreeMap::new(),
            failed: false,
            identity: AttemptIdentity(identity),
            sink_bound: false,
            handoffs: HashMap::new(),
            contributions: HashMap::new(),
        }
    }
}

/// Attempt-local execution authority. A dropped or failed stage poisons the whole attempt.
/// Handoffs keep a written relation's batches, and their reservations, for its declared readers;
/// contributions keep shared vocabulary for its writer. Both are released when consumed.
pub struct Execution<'s> {
    schedule: &'s Schedule,
    completed: BTreeMap<&'static str, ProviderOutcome>,
    sources: BTreeMap<&'static str, CompletedRelation>,
    failed: bool,
    identity: AttemptIdentity,
    sink_bound: bool,
    handoffs: HashMap<TypeId, Handoff>,
    contributions: HashMap<TypeId, Vec<Shared>>,
}
type Shared = Arc<dyn Any + Send + Sync>;
struct Handoff {
    batches: Vec<Shared>,
    readers: usize,
}
/// A stage's declared inputs as their writers handed them off, for a provider that runs apart from
/// the stage's access. Each batch keeps its reservation while any holder keeps it.
#[derive(Clone, Default)]
pub struct Handoffs {
    inputs: HashMap<TypeId, Vec<Shared>>,
}
impl Handoffs {
    pub fn get<R: Record>(&self) -> Result<Vec<Arc<Batch<R>>>, ModelError> {
        self.inputs
            .get(&TypeId::of::<R>())
            .map(|batches| shared::<R>(batches))
            .ok_or_else(|| {
                ModelError::Invalid(format!("{} is not a declared input of this stage", R::NAME))
            })
    }
}
fn shared<R: Record>(values: &[Shared]) -> Vec<Arc<Batch<R>>> {
    values
        .iter()
        .map(|value| {
            value
                .clone()
                .downcast::<Batch<R>>()
                .expect("handoff entry matches its type id")
        })
        .collect()
}
impl<'s> Execution<'s> {
    /// A completed frontier prefix without ending the cumulative attempt. This carries the
    /// original schedule identity, and refuses mixed-boundary stages or unacknowledged outputs.
    pub fn checkpoint_receipt(&self, contract: &super::admission::FrontierContract) -> Result<ExecutionReceipt, ModelError> {
        contract.checkpoint_preflight(self.schedule)?;
        if self.failed || !self.sink_bound {
            return Err(ModelError::Invalid("checkpoint requires a healthy bound attempt".into()));
        }
        let mut outcomes = BTreeMap::new();
        for stage in self.schedule.stages() {
            if stage.outputs.iter().all(|r| !contract.contains(r.name())) { continue; }
            if stage.outputs.iter().any(|r| !contract.contains(r.name()) || !self.sources.contains_key(r.name())) {
                return Err(ModelError::Invalid("checkpoint outputs incomplete or outside frontier".into()));
            }
            outcomes.insert(stage.name, *self.completed.get(stage.name).ok_or_else(|| ModelError::Invalid("checkpoint stage incomplete".into()))?);
        }
        Ok(ExecutionReceipt { identity: self.identity, model: self.schedule.model, schedule: self.schedule.digest, outcomes })
    }
    pub fn identity(&self) -> AttemptIdentity {
        self.identity
    }
    pub fn schedule(&self) -> &Schedule {
        self.schedule
    }
    /// One execution has one output sink. If constructing that sink fails, start a fresh
    /// execution; reusing it could mix a partially created sink with another generation.
    pub fn bind_sink(&mut self) -> Result<(), ModelError> {
        if self.failed || self.sink_bound || !self.completed.is_empty() {
            return Err(ModelError::Invalid(
                "execution sink already bound or execution started".into(),
            ));
        }
        self.sink_bound = true;
        Ok(())
    }
    pub fn begin(&mut self, name: &str) -> Result<StageAccess<'_, 's>, ModelError> {
        if self.failed || self.completed.contains_key(name) {
            return Err(ModelError::Invalid(
                "stage already completed or attempt failed".into(),
            ));
        }
        let stage = self
            .schedule
            .stages
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| ModelError::Invalid("stage is not scheduled".into()))?;
        if self.schedule.dependencies[stage.name]
            .iter()
            .any(|dependency| !self.completed.contains_key(dependency))
        {
            return Err(ModelError::Invalid(
                "stage input or contributor has not completed".into(),
            ));
        }
        Ok(StageAccess {
            stage,
            execution: self,
            written: HashSet::new(),
            retained: HashSet::new(),
            finished: false,
        })
    }
    pub fn finish(self) -> Result<ExecutionReceipt, ModelError> {
        if self.failed || self.completed.len() != self.schedule.stages.len() {
            return Err(ModelError::Invalid(
                "attempt schedule incomplete or failed".into(),
            ));
        }
        Ok(ExecutionReceipt {
            model: self.schedule.model,
            schedule: self.schedule.digest,
            outcomes: self.completed,
            identity: self.identity,
        })
    }
}
/// A receipt proves this schedule ran, not that its model is the complete production facts model.
/// Production frontier admission separately requires the complete declared producer contract.
#[derive(Debug)]
pub struct ExecutionReceipt {
    model: ContentHash,
    schedule: ContentHash,
    outcomes: BTreeMap<&'static str, ProviderOutcome>,
    identity: AttemptIdentity,
}
impl ExecutionReceipt {
    pub fn identity(&self) -> AttemptIdentity {
        self.identity
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn schedule(&self) -> ContentHash {
        self.schedule
    }
    pub fn outcomes(&self) -> &BTreeMap<&'static str, ProviderOutcome> {
        &self.outcomes
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptIdentity(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageIdentity {
    attempt: AttemptIdentity,
    stage: &'static str,
}
impl StageIdentity {
    pub fn attempt(self) -> AttemptIdentity {
        self.attempt
    }
}
pub struct ReadPermit<'a, R> {
    model: ContentHash,
    stage: &'static str,
    identity: StageIdentity,
    transport: InputTransport,
    source: Option<CompletedRelation>,
    marker: std::marker::PhantomData<&'a R>,
}
pub struct WritePermit<'a, R> {
    model: ContentHash,
    stage: &'static str,
    identity: StageIdentity,
    marker: std::marker::PhantomData<&'a R>,
}
impl<R: Record> ReadPermit<'_, R> {
    pub fn transport(&self) -> InputTransport { self.transport }
    pub fn source(&self) -> Option<&CompletedRelation> { self.source.as_ref() }
    pub fn identity(&self) -> StageIdentity {
        self.identity
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn stage(&self) -> &'static str {
        self.stage
    }
    pub fn relation(&self) -> &'static str {
        R::NAME
    }
}
impl<R: Record> WritePermit<'_, R> {
    pub fn identity(&self) -> StageIdentity {
        self.identity
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn stage(&self) -> &'static str {
        self.stage
    }
    pub fn relation(&self) -> &'static str {
        R::NAME
    }
}
pub struct StageAccess<'e, 's> {
    stage: &'s Stage,
    execution: &'e mut Execution<'s>,
    written: HashSet<TypeId>,
    retained: HashSet<TypeId>,
    finished: bool,
}
impl StageAccess<'_, '_> {
    pub fn profile(&self) -> Profile {
        self.execution.schedule.profile()
    }
    pub fn identity(&self) -> StageIdentity {
        StageIdentity {
            attempt: self.execution.identity,
            stage: self.stage.name,
        }
    }
    pub fn stage(&self) -> &Stage {
        self.stage
    }
    /// Every declared input's handed-off batches.
    pub fn handoffs(&self) -> Result<Handoffs, ModelError> {
        if self.execution.failed {
            return Err(ModelError::Invalid("attempt failed".into()));
        }
        let mut inputs = HashMap::new();
        for input in self.stage.inputs.iter().filter(|i| i.transport == InputTransport::Handoff) {
            let handoff =
                self.execution.handoffs.get(&input.type_id).ok_or_else(|| {
                    ModelError::Invalid(format!("{} was not handed off", input.name))
                })?;
            inputs.insert(input.type_id, handoff.batches.clone());
        }
        Ok(Handoffs { inputs })
    }
    pub fn read<R: Record>(&self) -> Result<ReadPermit<'_, R>, ModelError> {
        if self.execution.failed
            || !self
                .stage
                .inputs
                .iter()
                .any(|r| r.type_id == TypeId::of::<R>())
        {
            return Err(ModelError::Invalid(format!(
                "{} cannot read {}",
                self.stage.name,
                R::NAME
            )));
        }
        let input = self.stage.inputs.iter().find(|r| r.type_id == TypeId::of::<R>()).expect("declared input");
        let source = self.execution.sources.get(R::NAME).cloned();
        if input.transport == InputTransport::CompletedStore && source.is_none() {
            return Err(ModelError::Invalid(format!("{} has no acknowledged stored producer", R::NAME)));
        }
        Ok(ReadPermit {
            model: self.execution.schedule.model,
            stage: self.stage.name,
            identity: self.identity(),
            transport: input.transport,
            source,
            marker: std::marker::PhantomData,
        })
    }
    /// The sink obtains a nominal capability. Only a successful effect marks this output written;
    /// explicit empty batches use the same path. Repeated calls stream further batches.
    pub async fn write<R: Record, T>(
        &mut self,
        effect: impl AsyncFnOnce(WritePermit<'_, R>) -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        if self.execution.failed
            || !self
                .stage
                .outputs
                .iter()
                .any(|r| r.type_id == TypeId::of::<R>())
        {
            return Err(ModelError::Invalid(format!(
                "{} cannot write {}",
                self.stage.name,
                R::NAME
            )));
        }
        // Cancellation between effect and completion must fail the attempt, including when the
        // caller retains the access object after dropping the write future.
        self.execution.failed = true;
        let value = effect(WritePermit {
            model: self.execution.schedule.model,
            stage: self.stage.name,
            identity: self.identity(),
            marker: std::marker::PhantomData,
        })
        .await?;
        self.written.insert(TypeId::of::<R>());
        self.execution.failed = false;
        Ok(value)
    }
    fn has(list: &[RelationUse], type_id: TypeId) -> bool {
        list.iter().any(|r| r.type_id == type_id)
    }
    /// Keep a written output for the stages that declare it as input. Outputs without readers
    /// are released at once.
    pub fn retain<R: Record>(&mut self, batch: Arc<Batch<R>>) -> Result<(), ModelError> {
        let id = TypeId::of::<R>();
        if self.execution.failed || !Self::has(&self.stage.outputs, id) {
            return Err(ModelError::Invalid(format!(
                "{} cannot hand off {}",
                self.stage.name,
                R::NAME
            )));
        }
        self.retained.insert(id);
        let Some(readers) = self.execution.schedule.readers.get(&id).copied() else {
            return Ok(());
        };
        self.execution
            .handoffs
            .entry(id)
            .or_insert_with(|| Handoff {
                batches: Vec::new(),
                readers,
            })
            .batches
            .push(batch);
        Ok(())
    }
    /// The batches an earlier stage handed off for one of this stage's declared inputs.
    pub fn handoff<R: Record>(&self) -> Result<Vec<Arc<Batch<R>>>, ModelError> {
        let id = TypeId::of::<R>();
        if self.execution.failed || !Self::has(&self.stage.inputs, id) {
            return Err(ModelError::Invalid(format!(
                "{} cannot read {}",
                self.stage.name,
                R::NAME
            )));
        }
        Ok(shared::<R>(
            &self
                .execution
                .handoffs
                .get(&id)
                .ok_or_else(|| ModelError::Invalid(format!("{} was not handed off", R::NAME)))?
                .batches,
        ))
    }
    /// Hand shared vocabulary to the relation's writer, which runs after this stage.
    pub fn contribute<R: Record>(&mut self, batch: Arc<Batch<R>>) -> Result<(), ModelError> {
        let id = TypeId::of::<R>();
        if self.execution.failed || !Self::has(&self.stage.contributes, id) {
            return Err(ModelError::Invalid(format!(
                "{} cannot contribute {}",
                self.stage.name,
                R::NAME
            )));
        }
        self.execution
            .contributions
            .entry(id)
            .or_default()
            .push(batch);
        Ok(())
    }
    /// Take every contribution to one of this stage's outputs; the writer must merge them.
    pub fn take_contributions<R: Record>(&mut self) -> Result<Vec<Arc<Batch<R>>>, ModelError> {
        let id = TypeId::of::<R>();
        if self.execution.failed || !Self::has(&self.stage.outputs, id) {
            return Err(ModelError::Invalid(format!(
                "{} does not write {}",
                self.stage.name,
                R::NAME
            )));
        }
        Ok(shared::<R>(
            &self.execution.contributions.remove(&id).unwrap_or_default(),
        ))
    }
    /// Pure execution controls may finish without a sink. Bound attempts must acknowledge the
    /// durable completion effect through `complete`; a synchronous call cannot bypass it.
    pub fn finish(self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        if self.execution.sink_bound {
            return Err(ModelError::Invalid("bound stage requires acknowledged sink completion".into()));
        }
        self.check_finish(outcome)?;
        self.advance(outcome);
        Ok(())
    }
    fn check_finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        if self.execution.failed
            || outcome == ProviderOutcome::Failed
            || self.written.len() != self.stage.outputs.len()
        {
            return Err(ModelError::Invalid(
                "stage outputs incomplete or stage failed".into(),
            ));
        }
        for output in &self.stage.outputs {
            if self
                .execution
                .schedule
                .readers
                .contains_key(&output.type_id)
                && !self.retained.contains(&output.type_id)
            {
                return Err(ModelError::Invalid(format!(
                    "{} has readers but was not handed off",
                    output.name
                )));
            }
            if self.execution.contributions.contains_key(&output.type_id) {
                return Err(ModelError::Invalid(format!(
                    "contributions to {} were not merged",
                    output.name
                )));
            }
        }
        Ok(())
    }
    /// Freeze this stage's outputs before allowing a dependent stage to start. Cancellation,
    /// refusal or an uncertain acknowledgement poisons execution, including an empty output.
    pub async fn complete(self, sink: &impl StageSink, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.check_finish(outcome)?;
        self.execution.failed = true;
        let completion = StageCompletion {
            identity: self.identity(), model: self.execution.schedule.model,
            schedule: self.execution.schedule.digest, outcome,
            outputs: self.stage.outputs.iter().map(|r| r.name).collect(),
        };
        let completed = sink.complete(completion).await?;
        if completed.identity != self.identity() || completed.model != self.execution.schedule.model
            || completed.schedule != self.execution.schedule.digest
        { return Err(ModelError::Invalid("foreign stage completion acknowledgement".into())); }
        for (name, receipt) in completed.outputs {
            self.execution.sources.insert(name, CompletedRelation {
                identity: completed.identity, model: completed.model, schedule: completed.schedule,
                relation: name, receipt,
            });
        }
        self.execution.failed = false;
        self.advance(outcome);
        Ok(())
    }
    fn advance(mut self, outcome: ProviderOutcome) {
        for input in self.stage.inputs.iter().filter(|i| i.transport == InputTransport::Handoff) {
            if let Some(handoff) = self.execution.handoffs.get_mut(&input.type_id) {
                handoff.readers -= 1;
                if handoff.readers == 0 {
                    self.execution.handoffs.remove(&input.type_id);
                }
            }
        }
        self.execution.completed.insert(self.stage.name, outcome);
        self.finished = true;
    }
}
impl Drop for StageAccess<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            self.execution.failed = true;
        }
    }
}

/// A generation sink that receives a stage's typed batches under that stage's write permits.
/// PostgreSQL attempts and in-memory generations implement it; producers depend only on this.
pub trait StageSink: Sync {
    fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> impl Future<Output = Result<(), ModelError>> + Send;
    fn complete(&self, completion: StageCompletion) -> impl Future<Output = Result<CompletedStage, ModelError>> + Send;
}

/// Content acknowledged by the sink after freezing a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationReceipt { pub rows: u64, pub content: ContentHash }
/// Only a completed StageAccess constructs a completion request.
pub struct StageCompletion {
    identity: StageIdentity,
    model: ContentHash,
    schedule: ContentHash,
    outcome: ProviderOutcome,
    outputs: BTreeSet<&'static str>,
}
impl StageCompletion {
    pub fn identity(&self) -> StageIdentity { self.identity }
    pub fn stage(&self) -> &'static str { self.identity.stage }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn schedule(&self) -> ContentHash { self.schedule }
    pub fn outcome(&self) -> ProviderOutcome { self.outcome }
    pub fn outputs(&self) -> &BTreeSet<&'static str> { &self.outputs }
    /// Called by the sink only after the completion transaction commits. No partial output set
    /// can become a completed source, and replaying a stage remains an execution error.
    pub fn acknowledge(self, outputs: BTreeMap<&'static str, RelationReceipt>) -> Result<CompletedStage, ModelError> {
        if outputs.keys().copied().collect::<BTreeSet<_>>() != self.outputs {
            return Err(ModelError::Invalid("completion receipts differ from stage outputs".into()));
        }
        Ok(CompletedStage { identity: self.identity, model: self.model, schedule: self.schedule, outputs })
    }
}
pub struct CompletedStage {
    identity: StageIdentity,
    model: ContentHash,
    schedule: ContentHash,
    outputs: BTreeMap<&'static str, RelationReceipt>,
}
/// Source identity, never an arbitrary table provider. Constructed only from an acknowledged
/// completion; consumers receive it with their declared read permit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedRelation {
    identity: StageIdentity,
    model: ContentHash,
    schedule: ContentHash,
    relation: &'static str,
    receipt: RelationReceipt,
}
impl CompletedRelation {
    pub fn identity(&self) -> StageIdentity { self.identity }
    pub fn producer(&self) -> &'static str { self.identity.stage }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn schedule(&self) -> ContentHash { self.schedule }
    pub fn relation(&self) -> &'static str { self.relation }
    pub fn receipt(&self) -> RelationReceipt { self.receipt }
}

/// Streams one stage's declared outputs into a sink through transfer-bounded writers. Every
/// declared output is written, explicitly empty when the stage produced no rows. Dropping it
/// before `finish` fails the attempt, like dropping its `StageAccess`.
pub struct StageOutput<'o, 'e, 's, S: StageSink> {
    access: StageAccess<'e, 's>,
    sink: &'o S,
    model: &'o ValidatedModel,
    budget: ResourceBudget,
    limits: TransferLimits,
    outputs: Vec<(TypeId, Box<dyn PendingOutput<S>>)>,
    contributions: Vec<(TypeId, Box<dyn PendingContribution>)>,
}
impl<'o, 'e, 's, S: StageSink> StageOutput<'o, 'e, 's, S> {
    pub fn new(
        access: StageAccess<'e, 's>,
        sink: &'o S,
        model: &'o ValidatedModel,
        budget: ResourceBudget,
        limits: TransferLimits,
    ) -> Result<Self, ModelError> {
        if access.execution.schedule.model != model.digest() {
            return Err(ModelError::Invalid(
                "stage output model differs from its schedule".into(),
            ));
        }
        Ok(Self {
            access,
            sink,
            model,
            budget,
            limits,
            outputs: Vec::new(),
            contributions: Vec::new(),
        })
    }
    pub fn identity(&self) -> StageIdentity {
        self.access.identity()
    }
    pub fn stage(&self) -> &Stage {
        self.access.stage
    }
    pub fn read<R: Record>(&self) -> Result<ReadPermit<'_, R>, ModelError> {
        self.access.read::<R>()
    }
    pub fn handoff<R: Record>(&self) -> Result<Vec<Arc<Batch<R>>>, ModelError> {
        self.access.handoff::<R>()
    }
    pub fn handoffs(&self) -> Result<Handoffs, ModelError> {
        self.access.handoffs()
    }
    /// Hand a batch of shared vocabulary, built by the producer's own transfer-bounded writer, to
    /// its writer. The batch keeps its reservation until the writer merges it.
    pub fn contribute_batch<R: Record>(&mut self, batch: Batch<R>) -> Result<(), ModelError> {
        let result = self.access.contribute(Arc::new(batch));
        if result.is_err() {
            self.access.execution.failed = true;
        }
        result
    }
    /// Hand one row of shared vocabulary to its writer through a transfer-bounded writer.
    pub fn contribute<R: Record>(&mut self, row: R) -> Result<(), ModelError> {
        if !StageAccess::has(&self.access.stage.contributes, TypeId::of::<R>()) {
            return Err(ModelError::Invalid(format!(
                "{} cannot contribute {}",
                self.access.stage.name,
                R::NAME
            )));
        }
        if !self
            .contributions
            .iter()
            .any(|(t, _)| *t == TypeId::of::<R>())
        {
            let writer = BatchWriter::<R>::new(&self.budget, self.limits)?;
            self.contributions
                .push((TypeId::of::<R>(), Box::new(Contribution { writer })));
        }
        let entry = self
            .contributions
            .iter_mut()
            .find(|(t, _)| *t == TypeId::of::<R>())
            .expect("inserted above");
        let contribution = entry
            .1
            .as_any()
            .downcast_mut::<Contribution<R>>()
            .expect("contribution entry matches its type id");
        let result = match contribution.writer.push(self.model, row) {
            Ok(Some(batch)) => self.access.contribute(Arc::new(batch)),
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        };
        // A lost row cannot be recovered by the producer: the attempt fails.
        if result.is_err() {
            self.access.execution.failed = true;
        }
        result
    }
    /// Open a writer for one of the stage's declared outputs.
    pub fn declare<R: Record>(&mut self) -> Result<(), ModelError> {
        let declared = self
            .access
            .stage
            .outputs
            .iter()
            .any(|r| r.type_id == TypeId::of::<R>());
        if !declared || self.outputs.iter().any(|(t, _)| *t == TypeId::of::<R>()) {
            return Err(ModelError::Invalid(format!(
                "{} cannot declare output {} twice or outside its stage",
                self.access.stage.name,
                R::NAME
            )));
        }
        let writer = BatchWriter::<R>::new(&self.budget, self.limits)?;
        self.outputs.push((
            TypeId::of::<R>(),
            Box::new(Output {
                writer,
                written: false,
                pushed: false,
                batched: false,
            }),
        ));
        Ok(())
    }
    pub async fn push<R: Record>(&mut self, row: R) -> Result<(), ModelError> {
        let stage = self.access.stage.name;
        let output = self
            .outputs
            .iter_mut()
            .find(|(t, _)| *t == TypeId::of::<R>())
            .ok_or_else(|| {
                ModelError::Invalid(format!("{} has not declared output {}", stage, R::NAME))
            })?;
        let output = output
            .1
            .as_any()
            .downcast_mut::<Output<R>>()
            .expect("output entry matches its type id");
        if output.batched {
            return Err(ModelError::Invalid(format!(
                "{} writes {} by batches; rows cannot join them",
                stage,
                R::NAME
            )));
        }
        output.pushed = true;
        match output.writer.push(self.model, row) {
            Ok(Some(batch)) => {
                emit(&mut self.access, self.sink, batch).await?;
                output.written = true;
                Ok(())
            }
            Ok(None) => Ok(()),
            // A lost row cannot be recovered by the producer: the attempt fails.
            Err(error) => {
                self.access.execution.failed = true;
                Err(error)
            }
        }
    }
    /// Write a batch a producer already built with its own transfer-bounded writer; the batch keeps
    /// its reservation until the sink has it. An output is written by rows or by batches, never
    /// both, so one writer owns its duplicates. When other stages contribute to the output, its
    /// writer indexes the batch's identities: contributed rows equal to a batched row are dropped,
    /// and a different payload for a batched identity is refused.
    pub async fn push_batch<R: Record>(&mut self, batch: Batch<R>) -> Result<(), ModelError> {
        let stage = self.access.stage.name;
        let contributed = self
            .access
            .execution
            .schedule
            .contributed
            .contains(&TypeId::of::<R>());
        let output = self
            .outputs
            .iter_mut()
            .find(|(t, _)| *t == TypeId::of::<R>())
            .ok_or_else(|| {
                ModelError::Invalid(format!("{} has not declared output {}", stage, R::NAME))
            })?;
        let output = output
            .1
            .as_any()
            .downcast_mut::<Output<R>>()
            .expect("output entry matches its type id");
        if output.pushed {
            return Err(ModelError::Invalid(format!(
                "{} writes {} by rows; batches cannot join them",
                stage,
                R::NAME
            )));
        }
        output.batched = true;
        if contributed && let Err(error) = output.writer.observe(&batch) {
            self.access.execution.failed = true;
            return Err(error);
        }
        emit(&mut self.access, self.sink, batch).await?;
        output.written = true;
        Ok(())
    }
    /// Flush every declared output, then record the provider outcome for the stage.
    pub async fn finish(mut self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        if self.outputs.len() != self.access.stage.outputs.len() {
            return Err(ModelError::Invalid(format!(
                "{} has undeclared outputs",
                self.access.stage.name
            )));
        }
        for (_, contribution) in std::mem::take(&mut self.contributions) {
            contribution.flush(self.model, &mut self.access)?;
        }
        for (_, output) in std::mem::take(&mut self.outputs) {
            output
                .flush(self.model, &mut self.access, self.sink)
                .await?;
        }
        self.access.complete(self.sink, outcome).await
    }
}
/// Write one batch through the sink and keep it for the stage's declared readers.
async fn emit<R: Record, S: StageSink>(
    access: &mut StageAccess<'_, '_>,
    sink: &S,
    batch: Batch<R>,
) -> Result<(), ModelError> {
    access
        .write::<R, _>(async |permit| sink.copy(permit, &batch).await)
        .await?;
    access.retain(Arc::new(batch))
}
struct Contribution<R: Record> {
    writer: BatchWriter<R>,
}
trait PendingContribution: Send {
    fn as_any(&mut self) -> &mut dyn Any;
    fn flush(
        self: Box<Self>,
        model: &ValidatedModel,
        access: &mut StageAccess<'_, '_>,
    ) -> Result<(), ModelError>;
}
impl<R: Record> PendingContribution for Contribution<R> {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn flush(
        self: Box<Self>,
        model: &ValidatedModel,
        access: &mut StageAccess<'_, '_>,
    ) -> Result<(), ModelError> {
        match self.writer.finish(model)? {
            Some(batch) => access.contribute(Arc::new(batch)),
            None => Ok(()),
        }
    }
}
struct Output<R: Record> {
    writer: BatchWriter<R>,
    written: bool,
    pushed: bool,
    batched: bool,
}
trait PendingOutput<S: StageSink>: Send {
    fn as_any(&mut self) -> &mut dyn Any;
    fn flush<'x>(
        self: Box<Self>,
        model: &'x ValidatedModel,
        access: &'x mut StageAccess<'_, '_>,
        sink: &'x S,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModelError>> + Send + 'x>>;
}
impl<R: Record, S: StageSink> PendingOutput<S> for Output<R> {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn flush<'x>(
        self: Box<Self>,
        model: &'x ValidatedModel,
        access: &'x mut StageAccess<'_, '_>,
        sink: &'x S,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModelError>> + Send + 'x>> {
        Box::pin(async move {
            let Output {
                mut writer,
                mut written,
                ..
            } = *self;
            // Contributed vocabulary merges into this stage's own output; equal rows appear once.
            for batch in access.take_contributions::<R>()? {
                for row in batch.rows() {
                    if let Some(full) = writer.push(model, row.clone())? {
                        emit(access, sink, full).await?;
                        written = true;
                    }
                }
            }
            let budget = writer.budget().clone();
            match writer.finish(model)? {
                Some(batch) => emit(access, sink, batch).await,
                None if written => Ok(()),
                None => emit(access, sink, Batch::<R>::new(model, Vec::new(), &budget)?).await,
            }
        })
    }
}
