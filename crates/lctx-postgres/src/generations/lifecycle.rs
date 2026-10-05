//! Attempt-owned generations (cutover plan P1.7, T10; store-lifecycle review F01). Every
//! generation belongs to the attempt that registers it, and only that attempt advances it:
//! `GenerationAttempt` → `SealedAttempt` → `ValidatedAttempt` → published. Its lifecycle
//! connection holds the attempt lock from registration until the attempt ends, so a crashed or
//! dropped attempt leaves an *interrupted* generation that can only be aborted. A refusal at any
//! step records the generation `failed` (terminal, abort only) with its class and returns the
//! refusal; nothing is repaired in place, and a retry is a new attempt.
use super::{
    CleanupOutcome, Error, GenerationId, GenerationStore, failure::Failure, receipts::Admitting,
    transaction_on,
};
use lctx_model::domain::resources::ResourceBudget;
use lctx_model::domain::stages::{
    AttemptIdentity, ClosedGroup, CompletedStage, ComputedStage, Execution, ExecutionReceipt,
    GroupCompletion, StageCompletion, StageSink, WritePermit,
};
use lctx_model::domain::{
    Batch, ContentHash, ModelError, Record,
    admission::{Frontier, FrontierAdmission, FrontierContract, Preflight},
};
use sqlx::{PgPool, Postgres, pool::PoolConnection};
use std::{collections::BTreeSet, sync::Mutex};

impl GenerationStore {
    /// Begin a facts attempt. Preflight refuses a schedule that cannot produce the contract's
    /// frontier before any store effect.
    pub async fn begin(
        &self,
        writer: PgPool,
        execution: &mut Execution<'_>,
        contract: &FrontierContract,
        budget: ResourceBudget,
    ) -> Result<GenerationAttempt, Error> {
        if contract.model() != self.model.digest() {
            return Err(Error::Contract);
        }
        let preflight = contract
            .preflight(execution.schedule())
            .map_err(|error| match error {
                ModelError::Frontier(message) => Error::Frontier(message),
                other => Error::Model(other),
            })?;
        self.scope(contract.frontier())?;
        self.open_attempt(
            writer,
            execution,
            contract.frontier(),
            Some(preflight),
            budget,
        )
        .await
    }
    /// Begin an attempt over a subset schedule. It exercises the permanent stage-bound sink and
    /// lifecycle without claiming the facts frontier.
    pub async fn begin_conformance(
        &self,
        writer: PgPool,
        execution: &mut Execution<'_>,
        budget: ResourceBudget,
    ) -> Result<GenerationAttempt, Error> {
        self.open_attempt(writer, execution, Frontier::Conformance, None, budget)
            .await
    }
    async fn open_attempt(
        &self,
        writer: PgPool,
        execution: &mut Execution<'_>,
        frontier: Frontier,
        preflight: Option<Preflight>,
        budget: ResourceBudget,
    ) -> Result<GenerationAttempt, Error> {
        if execution.schedule().model() != self.model.digest() {
            return Err(Error::Contract);
        }
        execution.bind_sink()?;
        let schedule = execution.schedule();
        let expected: BTreeSet<(&'static str, &'static str)> = schedule
            .stages()
            .iter()
            .flat_map(|s| s.outputs.iter().map(move |r| (s.name, r.name())))
            .collect();
        let lifecycle = self
            .register(
                frontier,
                schedule.digest(),
                schedule.profile().name(),
                preflight,
                &expected,
                schedule.publication_groups(),
                schedule.stages(),
            )
            .await?;
        Ok(GenerationAttempt {
            generation: lifecycle.generation,
            frontier,
            store: self.clone(),
            lifecycle: tokio::sync::Mutex::new(lifecycle),
            poisoned: std::sync::atomic::AtomicBool::new(false),
            checkpoint: Mutex::new(None),
            reader: std::sync::Arc::default(),
            writer,
            identity: Some(execution.identity()),
            schedule: schedule.digest(),
            budget,
            expected,
            written: Mutex::new(BTreeSet::new()),
        })
    }
    /// Register a generation on a new lifecycle connection that takes the attempt lock before
    /// the registry row is visible, so no one observes the generation without its attempt.
    #[allow(
        clippy::too_many_arguments,
        reason = "Registration atomically carries the frontier, schedule identity, planned outputs and publication groups"
    )]
    async fn register(
        &self,
        frontier: Frontier,
        schedule: ContentHash,
        profile: &str,
        preflight: Option<Preflight>,
        planned: &BTreeSet<(&str, &str)>,
        groups: &[lctx_model::domain::stages::ScheduledPublication],
        stages: &[lctx_model::domain::stages::Stage],
    ) -> Result<Lifecycle, Error> {
        let g = GenerationId::new()?;
        let mut connection = self.owner.acquire().await?;
        // Closing, never pooling, releases the attempt lock with the attempt.
        connection.close_on_drop();
        transaction_on(&mut connection, async |tx| {
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(g.attempt_lock())
                .execute(&mut *tx)
                .await?;
            self.create(tx, g, frontier, schedule, profile, schedule)
                .await?;
            self.prepare_publications(tx, g, groups, stages).await?;
            for (stage, relation) in planned {
                sqlx::query("INSERT INTO lctx_model_store.planned_outputs VALUES($1,$2,$3)")
                    .bind(g.0.to_vec())
                    .bind(stage)
                    .bind(relation)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
        .await?;
        Ok(Lifecycle {
            store: self.clone(),
            connection,
            generation: g,
            preflight,
        })
    }
}

/// The attempt's hold on its generation: the store, the lock-holding lifecycle connection and,
/// for a facts attempt, its preflight.
struct Lifecycle {
    store: GenerationStore,
    connection: PoolConnection<Postgres>,
    generation: GenerationId,
    preflight: Option<Preflight>,
}
impl Lifecycle {
    /// Record `error` as the generation's failure, then end the attempt and return the error.
    /// If the record cannot be written (the connection is gone), the generation stays in its
    /// state and lists as interrupted.
    async fn fail(mut self, error: Error) -> Error {
        let failure = Failure::of(&error);
        let (store, g) = (self.store.clone(), self.generation);
        let _ = transaction_on(&mut self.connection, async |tx| {
            store.fail_step(tx, g, &failure).await
        })
        .await;
        self.end().await;
        error
    }
    /// Record a failure the producer reports, then end the attempt.
    async fn failed(mut self, failure: Failure) -> Result<GenerationId, Error> {
        let (store, g) = (self.store.clone(), self.generation);
        let recorded = transaction_on(&mut self.connection, async |tx| {
            store.fail_step(tx, g, &failure).await
        })
        .await;
        self.end().await;
        recorded.map(|()| g)
    }
    /// Remove the generation on this connection, which already holds its attempt lock.
    async fn abort(mut self) -> Result<GenerationId, Error> {
        let (store, g) = (self.store.clone(), self.generation);
        let removed = transaction_on(&mut self.connection, async |tx| {
            store.cleanup_on(tx, g, false, false).await
        })
        .await;
        self.end().await;
        match removed? {
            CleanupOutcome::Removed | CleanupOutcome::AlreadyAbsent => Ok(g),
        }
    }
    /// End the attempt: release the attempt lock with a confirmed round trip, then close the
    /// lifecycle connection. (Closing alone releases the lock only when the server's backend
    /// exits, after the close returns.) On a lost connection the server has released it already.
    async fn end(mut self) {
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(self.generation.attempt_lock())
            .execute(&mut *self.connection)
            .await;
        let _ = self.connection.close().await;
    }
}

/// An unpublished generation belongs to exactly one execution. Neither a copied schedule nor a
/// different invocation of that schedule can write or seal it.
pub struct GenerationAttempt {
    lifecycle: tokio::sync::Mutex<Lifecycle>,
    store: GenerationStore,
    generation: GenerationId,
    frontier: Frontier,
    poisoned: std::sync::atomic::AtomicBool,
    checkpoint: Mutex<Option<FrontierAdmission>>,
    reader: std::sync::Arc<tokio::sync::Mutex<()>>,
    writer: PgPool,
    /// The execution that owns the attempt; a test harness attempt has none and writes with `put`.
    identity: Option<AttemptIdentity>,
    schedule: ContentHash,
    budget: ResourceBudget,
    expected: BTreeSet<(&'static str, &'static str)>,
    written: Mutex<BTreeSet<(&'static str, &'static str)>>,
}
impl std::fmt::Debug for GenerationAttempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GenerationAttempt")
            .field(&self.generation.hex())
            .finish()
    }
}
impl GenerationAttempt {
    pub fn reserve_reader(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, ModelError> {
        self.reader
            .clone()
            .try_lock_owned()
            .map_err(|_| ModelError::Invalid("another attempt reader is active or draining".into()))
    }
    pub async fn read_contract(
        &self,
        stage: &lctx_model::domain::stages::StageAccess<'_, '_>,
    ) -> Result<super::lease::AttemptReadContract, ModelError> {
        if self.identity != Some(stage.identity().attempt())
            || self.poisoned.load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ModelError::Invalid(
                "foreign or poisoned attempt reader".into(),
            ));
        }
        let sources = stage.stored_sources()?;
        let checkpoint = self
            .checkpoint
            .lock()
            .map_err(|_| ModelError::Invalid("checkpoint state poisoned".into()))?
            .clone();
        if self.frontier != Frontier::Conformance && checkpoint.is_none() {
            return Err(ModelError::Invalid(
                "product store reads require the validated facts checkpoint".into(),
            ));
        }
        for requirement in stage
            .stage()
            .inputs
            .iter()
            .filter_map(|input| input.requirement())
        {
            checkpoint
                .as_ref()
                .ok_or_else(|| {
                    ModelError::Invalid("scoped input requires a validated facts checkpoint".into())
                })?
                .scoped()
                .admit(requirement)?;
        }
        for source in &sources {
            if source.model() != self.store.model.digest()
                || source.schedule() != self.schedule
                || Some(source.identity().attempt()) != self.identity
                || !self
                    .expected
                    .contains(&(source.producer(), source.relation()))
            {
                return Err(ModelError::Invalid("foreign completed source".into()));
            }
        }
        if self
            .poisoned
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelError::Invalid("attempt is poisoned".into()));
        }
        let mut lifecycle = self.lifecycle.lock().await;
        let checks = transaction_on(&mut lifecycle.connection, async |tx| {
            self.store
                .check_stage_inputs(
                    tx,
                    self.generation,
                    self.schedule,
                    stage.stage(),
                    &sources,
                    checkpoint.as_ref(),
                    &self.budget,
                )
                .await
        })
        .await
        .map_err(ModelError::from)?;
        self.poisoned
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(super::lease::AttemptReadContract::new(
            self.store.lease_contract(self.generation),
            stage.identity(),
            self.schedule,
            sources,
            checkpoint,
            checks,
        ))
    }
    pub fn generation(&self) -> GenerationId {
        self.generation
    }
    /// Validate and freeze a completed frontier prefix without publishing it or ending execution.
    pub async fn checkpoint(
        &self,
        execution: &Execution<'_>,
        contract: &FrontierContract,
    ) -> Result<CompletedCheckpoint, ModelError> {
        if Some(execution.identity()) != self.identity
            || contract.model() != self.store.model.digest()
        {
            return Err(ModelError::Invalid(
                "foreign checkpoint attempt or model".into(),
            ));
        }
        if self
            .poisoned
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelError::Invalid("attempt is poisoned".into()));
        }
        let receipt = execution.checkpoint_receipt(contract)?;
        let preflight = contract.checkpoint_preflight(execution.schedule())?;
        let mut lifecycle = self.lifecycle.lock().await;
        let admission = transaction_on(&mut lifecycle.connection, async |tx| {
            self.store
                .checkpoint_step(tx, self.generation, &self.budget, &preflight, &receipt)
                .await
        })
        .await
        .map_err(ModelError::from)?;
        *self
            .checkpoint
            .lock()
            .map_err(|_| ModelError::Invalid("checkpoint state poisoned".into()))? =
            Some(admission.clone());
        self.poisoned
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(CompletedCheckpoint {
            generation: self.generation,
            admission,
        })
    }
    pub async fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> Result<(), ModelError> {
        if self.poisoned.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ModelError::Invalid("attempt is poisoned".into()));
        }
        if Some(permit.identity().attempt()) != self.identity
            || permit.model() != self.store.model.digest()
        {
            return Err(ModelError::Invalid(
                "write permit belongs to another generation attempt".into(),
            ));
        }
        let grouped = {
            let mut tx = self
                .writer
                .acquire()
                .await
                .map_err(super::Error::from)
                .map_err(ModelError::from)?;
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3)").bind(self.generation.0.to_vec()).bind(permit.stage()).bind(R::NAME).fetch_one(&mut *tx).await.map_err(super::Error::from).map_err(ModelError::from)?
        };
        let physical = if grouped {
            super::ddl::delta_name(permit.stage(), R::NAME)
        } else {
            R::NAME.to_owned()
        };
        self.store
            .copy_into(
                &self.writer,
                self.generation,
                batch,
                self.schedule,
                &self.budget,
                &physical,
            )
            .await
            .map_err(ModelError::from)?;
        self.written
            .lock()
            .map_err(|_| ModelError::Invalid("generation output completion poisoned".into()))?
            .insert((permit.stage(), R::NAME));
        Ok(())
    }
    /// Seal with the execution's receipt: every planned output written, the receipt's stage
    /// outcomes recorded, the writer revoked.
    pub async fn seal(self, receipt: ExecutionReceipt) -> Result<SealedAttempt, Error> {
        let Self {
            lifecycle,
            poisoned,
            identity,
            schedule,
            budget,
            expected,
            written,
            ..
        } = self;
        let mut lifecycle = lifecycle.into_inner();
        let result = async {
            if poisoned.into_inner() {
                return Err(Error::State);
            }
            if identity != Some(receipt.identity())
                || receipt.model() != lifecycle.store.model.digest()
                || receipt.schedule() != schedule
            {
                return Err(Error::Contract);
            }
            let written = written.into_inner().map_err(|_| Error::State)?;
            if written != expected {
                return Err(Error::State);
            }
            let Lifecycle {
                store,
                connection,
                generation,
                ..
            } = &mut lifecycle;
            transaction_on(connection, async |tx| {
                store
                    .seal_step(
                        tx,
                        *generation,
                        schedule,
                        &written,
                        Some(receipt.outcomes()),
                    )
                    .await
            })
            .await
        }
        .await;
        match result {
            Ok(()) => Ok(SealedAttempt {
                lifecycle,
                receipt: Some(receipt),
                budget,
            }),
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    /// The producer failed the attempt: record `cause` with its class.
    pub async fn fail(self, cause: &ModelError) -> Result<GenerationId, Error> {
        self.lifecycle
            .into_inner()
            .failed(Failure::of_model(cause))
            .await
    }
    /// Remove the generation and every record of it (T9: a required provider failed).
    pub async fn abort(self) -> Result<GenerationId, Error> {
        self.lifecycle.into_inner().abort().await
    }
}
/// Private-prefix evidence. It grants no publication or selection authority.
#[derive(Debug)]
pub struct CompletedCheckpoint {
    generation: GenerationId,
    admission: FrontierAdmission,
}
impl CompletedCheckpoint {
    pub fn generation(&self) -> GenerationId {
        self.generation
    }
    pub fn admission(&self) -> &FrontierAdmission {
        &self.admission
    }
}
impl StageSink for GenerationAttempt {
    async fn compute(&self, completion: StageCompletion) -> Result<ComputedStage, ModelError> {
        if Some(completion.identity().attempt()) != self.identity
            || completion.model() != self.store.model.digest()
            || completion.schedule() != self.schedule
            || self
                .poisoned
                .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelError::Invalid(
                "foreign or poisoned computation".into(),
            ));
        }
        {
            let written = self
                .written
                .lock()
                .map_err(|_| ModelError::Invalid("output state poisoned".into()))?;
            if completion
                .outputs()
                .iter()
                .any(|r| !written.contains(&(completion.stage(), *r)))
            {
                return Err(ModelError::Invalid("unwritten publication output".into()));
            }
        }
        let mut lifecycle = self.lifecycle.lock().await;
        let receipts = transaction_on(&mut lifecycle.connection, async |tx| {
            self.store
                .compute_stage_step(tx, self.generation, &completion, &self.budget)
                .await
        })
        .await
        .map_err(ModelError::from)?;
        let computed = completion.seal(receipts)?;
        self.poisoned
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(computed)
    }
    async fn close_group(&self, group: GroupCompletion) -> Result<ClosedGroup, ModelError> {
        if group.stages().iter().any(|s| {
            Some(s.completion().identity().attempt()) != self.identity
                || s.completion().model() != self.store.model.digest()
                || s.completion().schedule() != self.schedule
        }) || self
            .poisoned
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelError::Invalid(
                "foreign or poisoned publication".into(),
            ));
        }
        let checkpoint = self
            .checkpoint
            .lock()
            .map_err(|_| ModelError::Invalid("checkpoint state poisoned".into()))?
            .clone();
        let mut lifecycle = self.lifecycle.lock().await;
        let receipts = transaction_on(&mut lifecycle.connection, async |tx| {
            self.store
                .close_vocabulary_step(
                    tx,
                    self.generation,
                    &group,
                    checkpoint.as_ref(),
                    &self.budget,
                )
                .await
        })
        .await
        .map_err(ModelError::from)?;
        let closed = group.acknowledge(receipts.0, receipts.1)?;
        self.poisoned
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(closed)
    }

    async fn complete(&self, completion: StageCompletion) -> Result<CompletedStage, ModelError> {
        if Some(completion.identity().attempt()) != self.identity
            || completion.model() != self.store.model.digest()
            || completion.schedule() != self.schedule
        {
            return Err(ModelError::Invalid("foreign stage completion".into()));
        }
        // Leave poisoned on every failure or cancellation, including uncertain COMMIT. Never
        // retry a transition on the same attempt even if the server may have committed it.
        if self
            .poisoned
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelError::Invalid("attempt is poisoned".into()));
        }
        {
            let written = self
                .written
                .lock()
                .map_err(|_| ModelError::Invalid("generation output completion poisoned".into()))?;
            if completion
                .outputs()
                .iter()
                .any(|name| !written.contains(&(completion.stage(), *name)))
            {
                return Err(ModelError::Invalid(
                    "completion output was not written".into(),
                ));
            }
        }
        let mut lifecycle = self.lifecycle.lock().await;
        let receipts = transaction_on(&mut lifecycle.connection, async |tx| {
            self.store
                .complete_stage_step(
                    tx,
                    self.generation,
                    self.schedule,
                    completion.stage(),
                    completion.outputs(),
                    completion.outcome(),
                    Some(&completion),
                    &self.budget,
                )
                .await
        })
        .await
        .map_err(ModelError::from)?;
        let acknowledged = completion.acknowledge(receipts)?;
        self.poisoned
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(acknowledged)
    }
    fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> impl Future<Output = Result<(), ModelError>> + Send {
        GenerationAttempt::copy(self, permit, batch)
    }
}

/// A sealed attempt: its content is fixed and its writer revoked.
pub struct SealedAttempt {
    lifecycle: Lifecycle,
    receipt: Option<ExecutionReceipt>,
    budget: ResourceBudget,
}
impl std::fmt::Debug for SealedAttempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SealedAttempt")
            .field(&self.lifecycle.generation.hex())
            .finish()
    }
}
impl SealedAttempt {
    pub fn generation(&self) -> GenerationId {
        self.lifecycle.generation
    }
    /// Validate the stored contents and, for a facts attempt, admit them. Charged to the
    /// attempt's budget.
    pub async fn validate(self) -> Result<ValidatedAttempt, Error> {
        let budget = self.budget.clone();
        self.validate_charged(&budget).await
    }
    async fn validate_charged(self, budget: &ResourceBudget) -> Result<ValidatedAttempt, Error> {
        let Self {
            mut lifecycle,
            receipt,
            ..
        } = self;
        let Lifecycle {
            store,
            connection,
            generation,
            preflight,
        } = &mut lifecycle;
        let admitting = match (preflight.as_ref(), receipt.as_ref()) {
            (Some(preflight), Some(receipt)) => Some(Admitting { preflight, receipt }),
            (Some(_), None) => return Err(lifecycle.fail(Error::State).await),
            (None, _) => None,
        };
        let result = transaction_on(connection, async |tx| {
            store
                .validate_step(tx, *generation, budget, admitting)
                .await
        })
        .await;
        match result {
            Ok((content, admission)) => Ok(ValidatedAttempt {
                lifecycle,
                content,
                admission,
                budget: budget.clone(),
            }),
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    pub async fn fail(self, cause: &ModelError) -> Result<GenerationId, Error> {
        self.lifecycle.failed(Failure::of_model(cause)).await
    }
    pub async fn abort(self) -> Result<GenerationId, Error> {
        self.lifecycle.abort().await
    }
}

/// A validated attempt, carrying its content digest and, for facts, its admission.
pub struct ValidatedAttempt {
    lifecycle: Lifecycle,
    budget: ResourceBudget,
    content: ContentHash,
    admission: Option<FrontierAdmission>,
}
impl std::fmt::Debug for ValidatedAttempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatedAttempt")
            .field(&self.lifecycle.generation.hex())
            .finish()
    }
}
impl ValidatedAttempt {
    pub fn generation(&self) -> GenerationId {
        self.lifecycle.generation
    }
    pub fn content(&self) -> ContentHash {
        self.content
    }
    pub fn admission(&self) -> Option<&FrontierAdmission> {
        self.admission.as_ref()
    }
    /// Publish atomically: admission recorded, reader granted, state changed. The attempt ends
    /// and its lock is released; the generation is never selected here.
    pub async fn publish(self) -> Result<GenerationId, Error> {
        let Self {
            mut lifecycle,
            admission,
            budget,
            ..
        } = self;
        let Lifecycle {
            store,
            connection,
            generation,
            ..
        } = &mut lifecycle;
        let g = *generation;
        let result = transaction_on(connection, async |tx| {
            store.publish_step(tx, g, admission.as_ref(), &budget).await
        })
        .await;
        match result {
            Ok(()) => {
                lifecycle.end().await;
                Ok(g)
            }
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    pub async fn fail(self, cause: &ModelError) -> Result<GenerationId, Error> {
        self.lifecycle.failed(Failure::of_model(cause)).await
    }
    pub async fn abort(self) -> Result<GenerationId, Error> {
        self.lifecycle.abort().await
    }
}

/// Test harness attempts (feature `testing`): attempt-owned conformance generations that tests
/// write relation by relation. They have attempt semantics throughout: the lifecycle
/// connection's lock, and a refusal recorded as `failed`. Their planned outputs are the
/// relations written, fixed when they seal.
#[cfg(feature = "testing")]
mod harness {
    use super::*;
    use lctx_model::domain::stages::Profile;

    impl GenerationStore {
        pub async fn begin_harness(
            &self,
            writer: PgPool,
            profile: Profile,
            budget: ResourceBudget,
        ) -> Result<GenerationAttempt, Error> {
            let schedule = ContentHash::of(b"lctx-testing-harness");
            let lifecycle = self
                .register(
                    Frontier::Conformance,
                    schedule,
                    profile.name(),
                    None,
                    &BTreeSet::new(),
                    &[],
                    &[],
                )
                .await?;
            Ok(GenerationAttempt {
                generation: lifecycle.generation,
                frontier: Frontier::Conformance,
                store: self.clone(),
                lifecycle: tokio::sync::Mutex::new(lifecycle),
                poisoned: std::sync::atomic::AtomicBool::new(false),
                checkpoint: Mutex::new(None),
                reader: std::sync::Arc::default(),
                writer,
                identity: None,
                schedule,
                budget,
                expected: BTreeSet::new(),
                written: Mutex::new(BTreeSet::new()),
            })
        }
    }
    impl GenerationAttempt {
        /// Write rows of `R`, charged to `budget`.
        pub async fn put<R: Record>(
            &self,
            batch: &Batch<R>,
            budget: &ResourceBudget,
        ) -> Result<(), Error> {
            if self.identity.is_some() {
                return Err(Error::State);
            }
            self.store
                .copy_into(
                    if lctx_model::domain::stages::is_vocabulary(R::NAME) {
                        &self.store.owner
                    } else {
                        &self.writer
                    },
                    self.generation,
                    batch,
                    self.schedule,
                    budget,
                    R::NAME,
                )
                .await?;
            self.written
                .lock()
                .map_err(|_| Error::State)?
                .insert(("harness", R::NAME));
            Ok(())
        }
        /// Seal a harness attempt: its planned outputs become the relations it wrote.
        pub async fn seal_harness(self) -> Result<SealedAttempt, Error> {
            let Self {
                lifecycle,
                identity,
                schedule,
                budget,
                written,
                ..
            } = self;
            let mut lifecycle = lifecycle.into_inner();
            let result = async {
                if identity.is_some() {
                    return Err(Error::State);
                }
                let written = written.into_inner().map_err(|_| Error::State)?;
                let Lifecycle {
                    store,
                    connection,
                    generation,
                    ..
                } = &mut lifecycle;
                let g = *generation;
                transaction_on(connection, async |tx| {
                    for (stage, relation) in &written {
                        sqlx::query(
                            "INSERT INTO lctx_model_store.planned_outputs VALUES($1,$2,$3)",
                        )
                        .bind(g.0.to_vec())
                        .bind(stage)
                        .bind(relation)
                        .execute(&mut *tx)
                        .await?;
                    }
                    if !written.is_empty() {
                        store
                            .complete_stage_step(
                                tx,
                                g,
                                schedule,
                                "harness",
                                &written.iter().map(|(_, r)| *r).collect(),
                                lctx_model::domain::stages::ProviderOutcome::Complete,
                                None,
                                &budget,
                            )
                            .await?;
                    }
                    store.seal_step(tx, g, schedule, &written, None).await
                })
                .await
            }
            .await;
            match result {
                Ok(()) => Ok(SealedAttempt {
                    lifecycle,
                    receipt: None,
                    budget,
                }),
                Err(error) => Err(lifecycle.fail(error).await),
            }
        }
    }
    impl SealedAttempt {
        /// Validate charged to `budget` rather than the attempt's own.
        pub async fn validate_with(
            self,
            budget: &ResourceBudget,
        ) -> Result<ValidatedAttempt, Error> {
            self.validate_charged(budget).await
        }
    }
}
