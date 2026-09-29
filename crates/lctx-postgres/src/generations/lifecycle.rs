//! Attempt-owned generations (cutover plan P1.7, T10). An attempt is the only writer and the only
//! lifecycle owner of its generation: `GenerationAttempt` → `SealedAttempt` → `ValidatedAttempt`
//! → published. Its lifecycle connection holds the attempt lock from registration until the
//! attempt ends, so a crashed or dropped attempt leaves an *interrupted* generation that can only
//! be aborted. A refusal at any step records the generation `failed` (terminal, abort only) and
//! returns the refusal; nothing is repaired in place.
use std::{collections::BTreeSet, sync::Mutex};
use lctx_model::domain::{Batch, ContentHash, ModelError, Record, admission::{FactsAdmission, Frontier, FrontierContract, Preflight}};
use lctx_model::domain::resources::ResourceBudget;
use lctx_model::domain::stages::{AttemptIdentity, Execution, ExecutionReceipt, StageSink, WritePermit};
use sqlx::{PgPool, pool::PoolConnection, Postgres};
use super::{Error, GenerationId, GenerationStore, receipts::Admitting, transaction_on};

impl GenerationStore {
    /// Begin a facts attempt. Preflight refuses a schedule that cannot produce the contract's
    /// frontier before any store effect.
    pub async fn begin(&self, writer: PgPool, execution: &mut Execution<'_>, contract: &FrontierContract, budget: ResourceBudget) -> Result<GenerationAttempt, Error> {
        if contract.model() != self.model.digest() { return Err(Error::Contract); }
        let preflight = contract.preflight(execution.schedule()).map_err(|error| match error {
            ModelError::Frontier(message) => Error::Frontier(message), other => Error::Model(other),
        })?;
        self.scope(Frontier::Facts)?;
        self.open(writer, execution, Frontier::Facts, Some(preflight), budget).await
    }
    /// Begin an attempt over a subset schedule. It exercises the permanent stage-bound sink and
    /// lifecycle without claiming the facts frontier.
    pub async fn begin_conformance(&self, writer: PgPool, execution: &mut Execution<'_>, budget: ResourceBudget) -> Result<GenerationAttempt, Error> {
        self.open(writer, execution, Frontier::Conformance, None, budget).await
    }
    async fn open(&self, writer: PgPool, execution: &mut Execution<'_>, frontier: Frontier, preflight: Option<Preflight>, budget: ResourceBudget)
        -> Result<GenerationAttempt, Error> {
        if execution.schedule().model() != self.model.digest() { return Err(Error::Contract); }
        execution.bind_sink()?;
        let schedule = execution.schedule();
        let expected: BTreeSet<(&'static str, &'static str)> = schedule.stages().iter().flat_map(|s| s.outputs.iter().map(move |r| (s.name, r.name()))).collect();
        let g = GenerationId::new()?;
        let mut connection = self.owner.acquire().await?;
        // Closing, never pooling, releases the attempt lock with the attempt.
        connection.close_on_drop();
        transaction_on(&mut connection, async |tx| {
            // The session lock outlives this transaction and is taken before the registry row
            // is visible, so no one observes the generation without its attempt.
            sqlx::query("SELECT pg_advisory_lock($1)").bind(g.attempt_lock()).execute(&mut *tx).await?;
            self.create(tx, g, frontier, true, schedule.digest(), schedule.profile().name(), Some(schedule.digest())).await?;
            for (stage, relation) in &expected {
                sqlx::query("INSERT INTO lctx_model_store.planned_outputs VALUES($1,$2,$3)").bind(g.0.to_vec()).bind(stage).bind(relation).execute(&mut *tx).await?;
            }
            Ok(())
        }).await?;
        Ok(GenerationAttempt { lifecycle: Lifecycle { store: self.clone(), connection, generation: g, preflight }, writer, identity: execution.identity(),
            schedule: schedule.digest(), budget, expected, written: Mutex::new(BTreeSet::new()) })
    }
}

/// The attempt's hold on its generation: the store, the lock-holding lifecycle connection and,
/// for a facts attempt, its preflight.
struct Lifecycle { store: GenerationStore, connection: PoolConnection<Postgres>, generation: GenerationId, preflight: Option<Preflight> }
impl Lifecycle {
    /// Record `error` as the generation's failure, then end the attempt. If the record cannot be
    /// written (the connection is gone), the generation stays in its state and lists as
    /// interrupted once the connection closes.
    async fn fail(mut self, error: Error) -> Error {
        let (class, detail) = failure(&error);
        let (store, g) = (self.store.clone(), self.generation);
        let _ = transaction_on(&mut self.connection, async |tx| store.fail_step(tx, g, class, &detail).await).await;
        error
    }
    async fn end(self) {
        let _ = self.connection.close().await;
    }
}
/// The stored class and a bounded detail of a failure.
fn failure(error: &Error) -> (&'static str, String) {
    use lctx_model::domain::Infrastructure as I;
    let infrastructure = |class| match class {
        I::Transport => "transport", I::Unconfirmed => "unconfirmed", I::Refused => "refused", I::State => "state", I::Contract => "contract", I::Io => "io",
    };
    let class = match error {
        Error::Model(ModelError::Resource { .. }) => "resource",
        Error::Model(ModelError::Limit { .. }) => "limit",
        Error::Model(ModelError::Frontier(_)) | Error::Frontier(_) => "frontier",
        Error::Model(ModelError::Infrastructure { class, .. }) => infrastructure(*class),
        Error::Model(ModelError::Codec(_)) | Error::Codec(_) => "codec",
        Error::Model(_) => "invalid",
        other => infrastructure(other.class()),
    };
    let mut detail = error.to_string();
    if detail.len() > 4096 { let mut end = 4096; while !detail.is_char_boundary(end) { end -= 1; } detail.truncate(end); }
    (class, detail)
}

/// An unpublished generation belongs to exactly one execution. Neither a copied schedule nor a
/// different invocation of that schedule can write or seal it.
pub struct GenerationAttempt {
    lifecycle: Lifecycle, writer: PgPool, identity: AttemptIdentity, schedule: ContentHash, budget: ResourceBudget,
    expected: BTreeSet<(&'static str, &'static str)>, written: Mutex<BTreeSet<(&'static str, &'static str)>>,
}
impl GenerationAttempt {
    pub fn generation(&self) -> GenerationId { self.lifecycle.generation }
    pub async fn copy<R: Record>(&self, permit: WritePermit<'_, R>, batch: &Batch<R>) -> Result<(), ModelError> {
        if permit.identity().attempt() != self.identity || permit.model() != self.lifecycle.store.model.digest() {
            return Err(ModelError::Invalid("write permit belongs to another generation attempt".into()));
        }
        self.lifecycle.store.copy_attempt(&self.writer, self.lifecycle.generation, batch, Some(self.schedule), &self.budget).await.map_err(ModelError::from)?;
        self.written.lock().map_err(|_| ModelError::Invalid("generation output completion poisoned".into()))?.insert((permit.stage(), R::NAME));
        Ok(())
    }
    /// Seal with the execution's receipt: every planned output written, the receipt's stage
    /// outcomes recorded, the writer revoked.
    pub async fn seal(self, receipt: ExecutionReceipt) -> Result<SealedAttempt, Error> {
        let Self { mut lifecycle, identity, schedule, budget, expected, written, .. } = self;
        let result = async {
            if receipt.identity() != identity || receipt.model() != lifecycle.store.model.digest() || receipt.schedule() != schedule {
                return Err(Error::Contract);
            }
            let written = written.into_inner().map_err(|_| Error::State)?;
            if written != expected { return Err(Error::State); }
            let Lifecycle { store, connection, generation, .. } = &mut lifecycle;
            transaction_on(connection, async |tx| store.seal_step(tx, *generation, true, Some(schedule), &written, Some(receipt.outcomes())).await).await
        }.await;
        match result {
            Ok(()) => Ok(SealedAttempt { lifecycle, receipt, budget }),
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    /// The producer abandons the attempt: the generation is recorded failed with `detail`.
    pub async fn fail(self, detail: &str) -> Result<GenerationId, Error> { explicit_failure(self.lifecycle, detail).await }
}
impl StageSink for GenerationAttempt {
    fn copy<R: Record>(&self, permit: WritePermit<'_, R>, batch: &Batch<R>) -> impl Future<Output = Result<(), ModelError>> + Send {
        GenerationAttempt::copy(self, permit, batch)
    }
}

async fn explicit_failure(mut lifecycle: Lifecycle, detail: &str) -> Result<GenerationId, Error> {
    let (store, g) = (lifecycle.store.clone(), lifecycle.generation);
    transaction_on(&mut lifecycle.connection, async |tx| store.fail_step(tx, g, "producer", detail).await).await?;
    lifecycle.end().await;
    Ok(g)
}

/// A sealed attempt: its content is fixed and its writer revoked.
pub struct SealedAttempt { lifecycle: Lifecycle, receipt: ExecutionReceipt, budget: ResourceBudget }
impl SealedAttempt {
    pub fn generation(&self) -> GenerationId { self.lifecycle.generation }
    /// Validate the stored contents and, for a facts attempt, admit them. Charged to the
    /// attempt's budget.
    pub async fn validate(self) -> Result<ValidatedAttempt, Error> {
        let Self { mut lifecycle, receipt, budget } = self;
        let Lifecycle { store, connection, generation, preflight } = &mut lifecycle;
        let admitting = preflight.as_ref().map(|preflight| Admitting { preflight, receipt: &receipt });
        let result = transaction_on(connection, async |tx| store.validate_step(tx, *generation, true, &budget, admitting).await).await;
        match result {
            Ok((content, admission)) => Ok(ValidatedAttempt { lifecycle, content, admission }),
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    pub async fn fail(self, detail: &str) -> Result<GenerationId, Error> { explicit_failure(self.lifecycle, detail).await }
}

/// A validated attempt, carrying its content digest and, for facts, its admission.
pub struct ValidatedAttempt { lifecycle: Lifecycle, content: ContentHash, admission: Option<FactsAdmission> }
impl ValidatedAttempt {
    pub fn generation(&self) -> GenerationId { self.lifecycle.generation }
    pub fn content(&self) -> ContentHash { self.content }
    pub fn admission(&self) -> Option<&FactsAdmission> { self.admission.as_ref() }
    /// Publish atomically: admission recorded, reader granted, state changed. The attempt ends
    /// and its lock is released; the generation is never selected here.
    pub async fn publish(self) -> Result<GenerationId, Error> {
        let Self { mut lifecycle, admission, .. } = self;
        let Lifecycle { store, connection, generation, .. } = &mut lifecycle;
        let g = *generation;
        let result = transaction_on(connection, async |tx| store.publish_step(tx, g, true, admission.as_ref()).await).await;
        match result {
            Ok(()) => { lifecycle.end().await; Ok(g) },
            Err(error) => Err(lifecycle.fail(error).await),
        }
    }
    pub async fn fail(self, detail: &str) -> Result<GenerationId, Error> { explicit_failure(self.lifecycle, detail).await }
}
