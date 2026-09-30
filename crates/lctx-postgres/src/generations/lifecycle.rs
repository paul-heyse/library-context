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
    AttemptIdentity, Execution, ExecutionReceipt, StageSink, WritePermit,
};
use lctx_model::domain::{
    Batch, ContentHash, ModelError, Record,
    admission::{FactsAdmission, Frontier, FrontierContract, Preflight},
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
        self.scope(Frontier::Facts)?;
        self.open_attempt(writer, execution, Frontier::Facts, Some(preflight), budget)
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
            )
            .await?;
        Ok(GenerationAttempt {
            lifecycle,
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
    async fn register(
        &self,
        frontier: Frontier,
        schedule: ContentHash,
        profile: &str,
        preflight: Option<Preflight>,
        planned: &BTreeSet<(&str, &str)>,
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
    lifecycle: Lifecycle,
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
            .field(&self.lifecycle.generation.hex())
            .finish()
    }
}
impl GenerationAttempt {
    pub fn generation(&self) -> GenerationId {
        self.lifecycle.generation
    }
    pub async fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> Result<(), ModelError> {
        if Some(permit.identity().attempt()) != self.identity
            || permit.model() != self.lifecycle.store.model.digest()
        {
            return Err(ModelError::Invalid(
                "write permit belongs to another generation attempt".into(),
            ));
        }
        self.lifecycle
            .store
            .copy_attempt(
                &self.writer,
                self.lifecycle.generation,
                batch,
                self.schedule,
                &self.budget,
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
            mut lifecycle,
            identity,
            schedule,
            budget,
            expected,
            written,
            ..
        } = self;
        let result = async {
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
        self.lifecycle.failed(Failure::of_model(cause)).await
    }
    /// Remove the generation and every record of it (T9: a required provider failed).
    pub async fn abort(self) -> Result<GenerationId, Error> {
        self.lifecycle.abort().await
    }
}
impl StageSink for GenerationAttempt {
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
    content: ContentHash,
    admission: Option<FactsAdmission>,
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
    pub fn admission(&self) -> Option<&FactsAdmission> {
        self.admission.as_ref()
    }
    /// Publish atomically: admission recorded, reader granted, state changed. The attempt ends
    /// and its lock is released; the generation is never selected here.
    pub async fn publish(self) -> Result<GenerationId, Error> {
        let Self {
            mut lifecycle,
            admission,
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
            store.publish_step(tx, g, admission.as_ref()).await
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
                )
                .await?;
            Ok(GenerationAttempt {
                lifecycle,
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
            self.lifecycle
                .store
                .copy_attempt(
                    &self.writer,
                    self.lifecycle.generation,
                    batch,
                    self.schedule,
                    budget,
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
                mut lifecycle,
                identity,
                schedule,
                budget,
                written,
                ..
            } = self;
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
