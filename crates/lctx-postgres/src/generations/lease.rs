//! The lease protocol, independent of the driver that runs it (cutover plan P1.10, T13). A lease
//! is a session-level shared lock on one published generation, taken inside one transaction after
//! checking, under the shared installation lock and the shared generation lock, that the
//! installation, the generation's state and its frontier-scoped model and physical digests are
//! this binary's. `pin` runs it through SQLx; provider sessions run it through tokio-postgres on
//! every connection they bind. Neither ever reconnects to take a lease again.
use super::{Error, GenerationId, ddl};
use lctx_model::domain::stages::{CompletedRelation, StageIdentity};
use lctx_model::domain::{ContentHash, Infrastructure, ValidatedModel, admission::Frontier};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// A statement parameter: generation identities and digests are bytes, lock keys integers.
#[derive(Debug, Clone, Copy)]
pub enum LeaseParam<'a> {
    Bytes(&'a [u8]),
    Int(i64),
    Text(&'a str),
}

/// Runs one lease statement and returns its single text value, if a row was returned. Each
/// statement of the protocol yields at most one row of one text column.
pub trait LeaseDriver: Send {
    fn text(
        &mut self,
        sql: &str,
        params: &[LeaseParam<'_>],
    ) -> impl Future<Output = Result<Option<String>, Error>> + Send;
    fn batch(&mut self, sql: &str) -> impl Future<Output = Result<(), Error>> + Send;
}

/// What a lease on one generation must confirm: the installation, and each frontier's lowering
/// (its relations, physical digest and live columns).
#[derive(Debug, Clone)]
pub struct LeaseContract {
    generation: GenerationId,
    model: ContentHash,
    installation: ContentHash,
    scopes: Arc<BTreeMap<Frontier, ddl::Scope>>,
    attempt: Option<AttemptSource>,
}
#[derive(Debug, Clone)]
struct AttemptSource {
    consumer: StageIdentity,
    schedule: ContentHash,
    sources: BTreeMap<&'static str, CompletedRelation>,
    checkpoint: Option<lctx_model::domain::admission::FrontierAdmission>,
    checks: BTreeMap<&'static str, ContentHash>,
}
/// A private capability minted by a live attempt, for exactly one consumer's frozen inputs.
#[derive(Debug, Clone)]
pub struct AttemptReadContract(LeaseContract);
impl AttemptReadContract {
    pub(super) fn new(
        mut lease: LeaseContract,
        consumer: StageIdentity,
        schedule: ContentHash,
        sources: Vec<CompletedRelation>,
        checkpoint: Option<lctx_model::domain::admission::FrontierAdmission>,
        checks: BTreeMap<&'static str, ContentHash>,
    ) -> Self {
        lease.attempt = Some(AttemptSource {
            consumer,
            schedule,
            sources: sources.into_iter().map(|s| (s.relation(), s)).collect(),
            checkpoint,
            checks,
        });
        Self(lease)
    }
    pub fn generation(&self) -> GenerationId {
        self.0.generation
    }
    pub fn model(&self) -> ContentHash {
        self.0.model
    }
    pub fn consumer(&self) -> StageIdentity {
        self.0.attempt.as_ref().expect("attempt contract").consumer
    }
    pub fn sources(&self) -> &BTreeMap<&'static str, CompletedRelation> {
        &self.0.attempt.as_ref().expect("attempt contract").sources
    }
    pub fn availability(&self) -> Option<&Arc<lctx_model::domain::admission::ScopedAvailability>> {
        self.0
            .attempt
            .as_ref()
            .expect("attempt contract")
            .checkpoint
            .as_ref()
            .map(|c| c.scoped())
    }
    pub async fn acquire<D: LeaseDriver>(&self, driver: &mut D) -> Result<Held, Error> {
        driver.batch("SET default_transaction_read_only=on").await?;
        self.0.acquire(driver).await
    }
    pub async fn release<D: LeaseDriver>(&self, driver: &mut D) -> Result<(), Error> {
        self.0.release(driver).await
    }
}
/// What a lease holds: the generation's frontier and the relations that frontier lowers. Readers
/// take their relation set from here rather than recomputing the frontier's scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub frontier: Frontier,
    pub relations: BTreeSet<&'static str>,
}
impl LeaseContract {
    pub(super) fn from_scopes(
        model: ContentHash,
        installation: ContentHash,
        scopes: Arc<BTreeMap<Frontier, ddl::Scope>>,
        generation: GenerationId,
    ) -> Self {
        Self {
            generation,
            model,
            installation,
            scopes,
            attempt: None,
        }
    }
    pub fn new(model: &ValidatedModel, generation: GenerationId) -> Self {
        let scopes = ddl::scopes(model);
        Self {
            generation,
            model: model.digest(),
            installation: scopes[&Frontier::Conformance].physical,
            scopes: Arc::new(scopes),
            attempt: None,
        }
    }
    pub fn generation(&self) -> GenerationId {
        self.generation
    }
    /// Take the lease; return what it holds. On any refusal the transaction rolls back and no lock
    /// is held.
    pub async fn acquire<D: LeaseDriver>(&self, driver: &mut D) -> Result<Held, Error> {
        driver.batch("BEGIN").await?;
        match self.checked(driver).await {
            Ok(frontier) => {
                driver.batch("COMMIT").await.map_err(|error| match error {
                    Error::Database(error) => Error::Commit(error),
                    Error::Driver { detail, .. } => Error::Driver {
                        class: Infrastructure::Unconfirmed,
                        detail,
                    },
                    other => other,
                })?;
                Ok(frontier)
            }
            Err(error) => {
                // A failed rollback leaves the session uncertain; the caller must discard it.
                let _ = driver.batch("ROLLBACK").await;
                Err(error)
            }
        }
    }
    async fn checked<D: LeaseDriver>(&self, driver: &mut D) -> Result<Held, Error> {
        let g = self.generation;
        driver
            .text(super::locks::INSTALLATION_SHARED_TEXT, &[])
            .await?;
        let compatible = driver.text("SELECT (model_digest=$1 AND physical_digest=$2)::text FROM lctx_model_store.installation WHERE singleton",
            &[LeaseParam::Bytes(&self.model.0), LeaseParam::Bytes(&self.installation.0)]).await?;
        if compatible.as_deref() != Some("true") {
            return Err(Error::Contract);
        }
        driver
            .text(
                "SELECT 'locked' FROM (SELECT pg_advisory_xact_lock_shared($1)) AS l",
                &[LeaseParam::Int(g.lock())],
            )
            .await?;
        let registered = driver.text("SELECT state || ' ' || frontier || ' ' || encode(model_digest,'hex') || ' ' || encode(physical_digest,'hex') \
            FROM lctx_model_store.generations WHERE id=$1", &[LeaseParam::Bytes(&g.0)]).await?.ok_or(Error::Absent)?;
        let fields: Vec<&str> = registered.split(' ').collect();
        let [state, frontier, model, physical] = fields.as_slice() else {
            return Err(Error::Contract);
        };
        let frontier = ddl::frontier(frontier).ok_or(Error::Contract)?;
        let Some(scope) = self.scopes.get(&frontier) else {
            return Err(Error::Contract);
        };
        if *model != self.model.hex() || scope.physical.hex() != *physical {
            return Err(Error::Contract);
        }
        if *state
            != if self.attempt.is_some() {
                "staging"
            } else {
                "published"
            }
        {
            return Err(Error::State);
        }
        if let Some(attempt) = &self.attempt {
            for (name, digest) in &attempt.checks {
                let same = driver.text("SELECT (input_digest=$4)::text FROM lctx_model_store.stage_read_checks WHERE generation_id=$1 AND consumer=$2 AND check_name=$3",
                    &[LeaseParam::Bytes(&g.0), LeaseParam::Text(attempt.consumer.stage()), LeaseParam::Text(name), LeaseParam::Bytes(&digest.0)]).await?;
                if same.as_deref() != Some("true") {
                    return Err(Error::Contract);
                }
            }
            if let Some(checkpoint) = &attempt.checkpoint {
                let same = driver.text("SELECT (contract_digest=$3 AND model_digest=$4 AND schedule_digest=$5 AND coverage_digest=$6 AND content_digest=$7)::text FROM lctx_model_store.checkpoints WHERE generation_id=$1 AND frontier=$2",
                    &[LeaseParam::Bytes(&g.0), LeaseParam::Text(checkpoint.frontier().name()), LeaseParam::Bytes(&checkpoint.contract().0), LeaseParam::Bytes(&checkpoint.model().0),
                      LeaseParam::Bytes(&checkpoint.schedule().0), LeaseParam::Bytes(&checkpoint.coverage().0), LeaseParam::Bytes(&checkpoint.content().0)]).await?;
                if same.as_deref() != Some("true") {
                    return Err(Error::Contract);
                }
            }
            let (high, low) = super::locks::halves(g.attempt_lock());
            let holders = driver
                .text(
                    &format!("SELECT ({})::text", super::locks::HOLDERS),
                    &[
                        LeaseParam::Int(high),
                        LeaseParam::Int(low),
                        LeaseParam::Text("ExclusiveLock"),
                    ],
                )
                .await?;
            if holders
                .as_deref()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                == 0
            {
                return Err(Error::State);
            }
            let same = driver.text("SELECT (schedule_digest=$2)::text FROM lctx_model_store.generations WHERE id=$1",
                &[LeaseParam::Bytes(&g.0), LeaseParam::Bytes(&attempt.schedule.0)]).await?;
            if same.as_deref() != Some("true") {
                return Err(Error::Contract);
            }
            for source in attempt.sources.values() {
                let receipt = source.receipt();
                let same = driver.text("SELECT (schedule_digest=$4 AND row_count=$5 AND content_digest=$6)::text FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3",
                    &[LeaseParam::Bytes(&g.0), LeaseParam::Text(source.producer()), LeaseParam::Text(source.relation()),
                      LeaseParam::Bytes(&source.schedule().0), LeaseParam::Int(i64::try_from(receipt.rows).map_err(|_| Error::Contract)?), LeaseParam::Bytes(&receipt.content.0)]).await?;
                if same.as_deref() != Some("true") || !scope.relations.contains(source.relation()) {
                    return Err(Error::Contract);
                }
                if let Some(prefix) = source
                    .prefix()
                    .filter(|_| lctx_model::domain::stages::is_vocabulary(source.relation()))
                {
                    let closed = driver.text("SELECT (g.closed AND r.row_count=$4 AND r.content_digest=$5)::text FROM lctx_model_store.publication_groups g JOIN lctx_model_store.epoch_receipts r USING(generation_id,epoch) WHERE r.generation_id=$1 AND r.epoch=$2::bigint AND r.relation_name=$3", &[LeaseParam::Bytes(&g.0), LeaseParam::Int(i64::from(prefix.code())), LeaseParam::Text(source.relation()), LeaseParam::Int(i64::try_from(receipt.rows).map_err(|_| Error::Contract)?), LeaseParam::Bytes(&receipt.content.0)]).await?;
                    if closed.as_deref() != Some("true") {
                        return Err(Error::Contract);
                    }
                }
            }
        }
        // The registry's digests say what was lowered; the live catalog says what is there now.
        let live = driver
            .text(ddl::LIVE_COLUMNS, &[LeaseParam::Text(&g.schema())])
            .await?;
        if live.as_deref() != Some(scope.columns.as_str()) {
            return Err(Error::Contract);
        }
        driver
            .text(
                "SELECT 'locked' FROM (SELECT pg_advisory_lock_shared($1)) AS l",
                &[LeaseParam::Int(g.lock())],
            )
            .await?;
        Ok(Held {
            frontier,
            relations: self.attempt.as_ref().map_or_else(
                || scope.relations.clone(),
                |a| a.sources.keys().copied().collect(),
            ),
        })
    }
    /// Release the lease and confirm the server held it.
    pub async fn release<D: LeaseDriver>(&self, driver: &mut D) -> Result<(), Error> {
        let released = driver
            .text(
                "SELECT pg_advisory_unlock_shared($1)::text",
                &[LeaseParam::Int(self.generation.lock())],
            )
            .await?;
        if released.as_deref() != Some("true") {
            return Err(Error::State);
        }
        Ok(())
    }
}

impl Error {
    /// A driver failure outside SQLx, kept with its class.
    pub fn driver(class: Infrastructure, error: impl std::fmt::Display) -> Self {
        Self::Driver {
            class,
            detail: error.to_string(),
        }
    }
}

/// The SQLx driver `pin` uses.
pub(super) struct Sqlx<'c>(pub &'c mut sqlx::PgConnection);
impl LeaseDriver for Sqlx<'_> {
    async fn text(
        &mut self,
        sql: &str,
        params: &[LeaseParam<'_>],
    ) -> Result<Option<String>, Error> {
        let mut query = sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = match param {
                LeaseParam::Bytes(bytes) => query.bind(bytes.to_vec()),
                LeaseParam::Int(value) => query.bind(*value),
                LeaseParam::Text(text) => query.bind(text.to_string()),
            };
        }
        Ok(query.fetch_optional(&mut *self.0).await?)
    }
    async fn batch(&mut self, sql: &str) -> Result<(), Error> {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_owned()))
            .execute(&mut *self.0)
            .await?;
        Ok(())
    }
}
