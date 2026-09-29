//! Typed stage declarations are the sole writer authority.
use std::{any::TypeId, collections::{BTreeMap, BTreeSet, HashSet}};
use super::{ContentHash, KeySink, ModelError, Record, ValidatedModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOutcome { Complete, Partial, Unavailable, Failed, NotRequested }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Profile { Catalog, Behavioral }
impl Profile {
    pub fn name(self) -> &'static str { match self { Self::Catalog => "catalog", Self::Behavioral => "behavioral" } }
}
/// Effects are declared by the producer, never inferred from the relations it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect { Pure, Acquisition, Extraction, Store }
impl Effect {
    fn name(self) -> &'static str { match self { Self::Pure => "pure", Self::Acquisition => "acquisition", Self::Extraction => "extraction", Self::Store => "store" } }
}
#[derive(Debug, Clone, Copy)]
pub struct RelationUse { pub(crate) type_id: TypeId, name: &'static str }
impl RelationUse {
    pub fn of<R: Record>() -> Self { Self { type_id: TypeId::of::<R>(), name: R::NAME } }
    pub fn name(self) -> &'static str { self.name }
}
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: &'static str,
    pub inputs: Vec<RelationUse>,
    pub outputs: Vec<RelationUse>,
    pub profiles: Vec<Profile>,
    pub effect: Effect,
    pub code: ContentHash,
    pub configuration: ContentHash,
}
#[derive(Debug)]
pub struct Schedule { stages: Vec<Stage>, model: ContentHash, digest: ContentHash, profile: Profile }
impl Schedule {
    pub fn build(model: &ValidatedModel, stages: Vec<Stage>, required: &[RelationUse], profile: Profile) -> Result<Self, ModelError> {
        let members: HashSet<_> = model.relations().iter().map(super::Relation::type_id).collect();
        let mut names = HashSet::new();
        for stage in &stages {
            if !names.insert(stage.name) { return Err(ModelError::Invalid(format!("duplicate stage {}", stage.name))); }
            if stage.profiles.is_empty() || stage.profiles.iter().copied().collect::<BTreeSet<_>>().len() != stage.profiles.len() {
                return Err(ModelError::Invalid(format!("invalid profiles for {}", stage.name)));
            }
        }
        let stages: Vec<_> = stages.into_iter().filter(|s| s.profiles.contains(&profile)).collect();
        names.clear();
        let mut writers = std::collections::HashMap::new();
        for (i, stage) in stages.iter().enumerate() {
            if stage.name.is_empty() || stage.outputs.is_empty() { return Err(ModelError::Invalid("stage needs a name and declared outputs".into())); }
            if !names.insert(stage.name) { return Err(ModelError::Invalid(format!("duplicate stage {}", stage.name))); }
            for r in stage.inputs.iter().chain(&stage.outputs) {
                if !members.contains(&r.type_id) { return Err(ModelError::Invalid(format!("undeclared relation {}", r.name))); }
            }
            if stage.inputs.iter().map(|r| r.type_id).collect::<HashSet<_>>().len() != stage.inputs.len() {
                return Err(ModelError::Invalid(format!("duplicate input for {}", stage.name)));
            }
            for r in &stage.outputs {
                if writers.insert(r.type_id, i).is_some() { return Err(ModelError::Invalid(format!("multiple writers for {}", r.name))); }
            }
        }
        for r in required {
            if !members.contains(&r.type_id) || !writers.contains_key(&r.type_id) { return Err(ModelError::Invalid(format!("missing required writer {}", r.name))); }
        }
        let mut dependencies = BTreeMap::new();
        for (i, stage) in stages.iter().enumerate() {
            let mut deps = BTreeSet::new();
            for r in &stage.inputs {
                let writer = writers.get(&r.type_id).ok_or_else(|| ModelError::Invalid(format!("missing writer for {}", r.name)))?;
                deps.insert(*writer);
            }
            dependencies.insert(i, deps);
        }
        let mut ordered = Vec::new();
        while !dependencies.is_empty() {
            let next = dependencies.iter().filter(|(_, deps)| deps.is_empty()).map(|(i, _)| *i)
                .min_by_key(|i| stages[*i].name).ok_or_else(|| ModelError::Invalid("stage cycle".into()))?;
            dependencies.remove(&next);
            for deps in dependencies.values_mut() { deps.remove(&next); }
            ordered.push(stages[next].clone());
        }
        let mut digest = KeySink::new("stage-schedule");
        digest.part(b"model", &model.digest().0);
        digest.part(b"profile", profile.name().as_bytes());
        for stage in &ordered {
            digest.part(b"stage", stage.name.as_bytes());
            digest.part(b"effect", stage.effect.name().as_bytes());
            digest.part(b"code", &stage.code.0);
            digest.part(b"configuration", &stage.configuration.0);
            let mut reads: Vec<_> = stage.inputs.iter().map(|r| r.name).collect(); reads.sort();
            let mut writes: Vec<_> = stage.outputs.iter().map(|r| r.name).collect(); writes.sort();
            for name in reads { digest.part(b"read",name.as_bytes()); }
            for name in writes { digest.part(b"write",name.as_bytes()); }
        }
        Ok(Self { stages: ordered, model: model.digest(), digest: digest.finish(), profile })
    }
    pub fn stages(&self) -> &[Stage] { &self.stages }
    pub fn digest(&self) -> ContentHash { self.digest }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn profile(&self) -> Profile { self.profile }
    pub fn execute(&self) -> Execution<'_> {
        static NEXT_ATTEMPT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let identity = NEXT_ATTEMPT.try_update(std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed, |n| n.checked_add(1))
            .expect("attempt identity space exhausted");
        Execution { schedule: self, completed: BTreeMap::new(), failed: false, identity: AttemptIdentity(identity), sink_bound: false }
    }
}

/// Attempt-local execution authority. A dropped or failed stage poisons the whole attempt.
pub struct Execution<'s> {
    schedule: &'s Schedule, completed: BTreeMap<&'static str,ProviderOutcome>, failed: bool, identity: AttemptIdentity,
    sink_bound: bool,
}
impl<'s> Execution<'s> {
    pub fn identity(&self) -> AttemptIdentity { self.identity }
    pub fn schedule(&self) -> &Schedule { self.schedule }
    /// One execution has one output sink. If constructing that sink fails, start a fresh
    /// execution; reusing it could mix a partially created sink with another generation.
    pub fn bind_sink(&mut self) -> Result<(), ModelError> {
        if self.failed || self.sink_bound || !self.completed.is_empty() {
            return Err(ModelError::Invalid("execution sink already bound or execution started".into()));
        }
        self.sink_bound = true;
        Ok(())
    }
    pub fn begin(&mut self, name: &str) -> Result<StageAccess<'_, 's>, ModelError> {
        if self.failed || self.completed.contains_key(name) { return Err(ModelError::Invalid("stage already completed or attempt failed".into())); }
        let stage = self.schedule.stages.iter().find(|s| s.name == name).ok_or_else(|| ModelError::Invalid("stage is not scheduled".into()))?;
        for input in &stage.inputs {
            let writer = self.schedule.stages.iter().find(|s| s.outputs.iter().any(|r| r.type_id == input.type_id)).expect("validated writer");
            if !self.completed.contains_key(writer.name) { return Err(ModelError::Invalid("stage input has not completed".into())); }
        }
        Ok(StageAccess { stage, execution: self, written: HashSet::new(), finished: false })
    }
    pub fn finish(self) -> Result<ExecutionReceipt, ModelError> {
        if self.failed || self.completed.len() != self.schedule.stages.len() { return Err(ModelError::Invalid("attempt schedule incomplete or failed".into())); }
        Ok(ExecutionReceipt { model: self.schedule.model, schedule: self.schedule.digest, outcomes: self.completed, identity: self.identity })
    }
}
/// A receipt proves this schedule ran, not that its model is the complete production facts model.
/// Production frontier admission separately requires the complete declared producer contract.
#[derive(Debug)]
pub struct ExecutionReceipt { model: ContentHash, schedule: ContentHash, outcomes: BTreeMap<&'static str,ProviderOutcome>, identity: AttemptIdentity }
impl ExecutionReceipt {
    pub fn identity(&self) -> AttemptIdentity { self.identity }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn schedule(&self) -> ContentHash { self.schedule }
    pub fn outcomes(&self) -> &BTreeMap<&'static str,ProviderOutcome> { &self.outcomes }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct AttemptIdentity(u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct StageIdentity { attempt: AttemptIdentity, stage: &'static str }
impl StageIdentity { pub fn attempt(self) -> AttemptIdentity { self.attempt } }
pub struct ReadPermit<'a,R> { model: ContentHash, stage: &'static str, identity: StageIdentity, marker: std::marker::PhantomData<&'a R> }
pub struct WritePermit<'a,R> { model: ContentHash, stage: &'static str, identity: StageIdentity, marker: std::marker::PhantomData<&'a R> }
impl<R: Record> ReadPermit<'_,R> {
    pub fn identity(&self) -> StageIdentity { self.identity }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn stage(&self) -> &'static str { self.stage }
    pub fn relation(&self) -> &'static str { R::NAME }
}
impl<R: Record> WritePermit<'_,R> {
    pub fn identity(&self) -> StageIdentity { self.identity }
    pub fn model(&self) -> ContentHash { self.model }
    pub fn stage(&self) -> &'static str { self.stage }
    pub fn relation(&self) -> &'static str { R::NAME }
}
pub struct StageAccess<'e,'s> {
    stage: &'s Stage, execution: &'e mut Execution<'s>, written: HashSet<TypeId>, finished: bool,
}
impl StageAccess<'_, '_> {
    pub fn identity(&self) -> StageIdentity { StageIdentity { attempt: self.execution.identity,stage: self.stage.name } }
    pub fn read<R: Record>(&self) -> Result<ReadPermit<'_,R>, ModelError> {
        if self.execution.failed || !self.stage.inputs.iter().any(|r| r.type_id == TypeId::of::<R>()) { return Err(ModelError::Invalid(format!("{} cannot read {}",self.stage.name,R::NAME))); }
        Ok(ReadPermit { model: self.execution.schedule.model,stage: self.stage.name,identity: self.identity(),marker: std::marker::PhantomData })
    }
    /// The sink obtains a nominal capability. Only a successful effect marks this output written;
    /// explicit empty batches use the same path. Repeated calls stream further batches.
    pub async fn write<R: Record,T>(&mut self, effect: impl AsyncFnOnce(WritePermit<'_,R>) -> Result<T,ModelError>) -> Result<T,ModelError> {
        if self.execution.failed || !self.stage.outputs.iter().any(|r| r.type_id == TypeId::of::<R>()) {
            return Err(ModelError::Invalid(format!("{} cannot write {}",self.stage.name,R::NAME)));
        }
        // Cancellation between effect and completion must fail the attempt, including when the
        // caller retains the access object after dropping the write future.
        self.execution.failed = true;
        let value = effect(WritePermit { model: self.execution.schedule.model,stage: self.stage.name,identity: self.identity(),marker: std::marker::PhantomData }).await?;
        self.written.insert(TypeId::of::<R>());
        self.execution.failed = false;
        Ok(value)
    }
    pub fn finish(mut self, outcome: ProviderOutcome) -> Result<(),ModelError> {
        if self.execution.failed || outcome == ProviderOutcome::Failed || self.written.len() != self.stage.outputs.len() {
            return Err(ModelError::Invalid("stage outputs incomplete or stage failed".into()));
        }
        self.execution.completed.insert(self.stage.name,outcome); self.finished = true; Ok(())
    }
}
impl Drop for StageAccess<'_, '_> { fn drop(&mut self) { if !self.finished { self.execution.failed = true; } } }
