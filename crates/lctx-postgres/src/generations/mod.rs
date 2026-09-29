//! Immutable generation schemas lowered exclusively from a validated semantic model (ADR-0086).
mod catalog;
mod codec;
mod ddl;
mod install;
mod lease;
mod lifecycle;
mod receipts;
mod verify;
pub use catalog::{GenerationCatalog, GenerationDetail, GenerationState, GenerationSummary, ListFilter, Writer};
pub use install::ResetInventory;
pub use lease::{LeaseContract, LeaseDriver, LeaseParam};
pub use lifecycle::{GenerationAttempt, SealedAttempt, ValidatedAttempt};
pub use verify::{CheckReport, Finding, FindingKind};
use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};
use arrow_array::RecordBatch;
use bytes::BytesMut;
use lctx_model::domain::{Batch, ContentHash, Infrastructure, ModelError, Record, Relation, ValidatedModel, admission::Frontier};
use lctx_model::domain::resources::{ResourceBudget, MAX_ROW_BYTES, TRANSFER_BYTES, TRANSFER_ROWS};
use pgpq::encoders::{BuildEncoder, Encode, EncoderBuilder};
use sqlx::{Connection, PgConnection, PgPool, Row, ValueRef};
use futures::TryStreamExt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")] Model(#[from] ModelError),
    #[error("PostgreSQL operation failed")] Database(#[from] sqlx::Error),
    #[error("PostgreSQL commit failed; outcome is unconfirmed")] Commit(#[source] sqlx::Error),
    #[error("PostgreSQL rollback failed after {operation}; cleanup is unconfirmed")]
    Rollback { operation: Box<Error>, #[source] source: sqlx::Error },
    #[error("PostgreSQL COPY abort failed after {operation}; cleanup is unconfirmed")]
    CopyAbort { operation: Box<Error>, #[source] source: sqlx::Error },
    #[error("generation state does not permit this operation")] State,
    #[error("generation model or physical schema differs from this binary")] Contract,
    #[error("generation has active readers or is selected")] Busy,
    #[error("codec: {0}")] Codec(String),
    #[error("secure generation randomness unavailable")] Random,
    #[error("generation has orphaned registry or schema objects; explicit repair is required")] Orphaned,
    /// A request outside the generation's frontier: a relation it does not hold, or selecting a
    /// generation that is not a facts generation (P0 exit F02).
    #[error("frontier: {0}")] Frontier(String),
    #[error("reset requires the database name as confirmation")] Confirmation,
    /// A failure of a driver other than SQLx (a provider session), with its class.
    #[error("{class:?} driver failure: {detail}")] Driver { class: Infrastructure, detail: String },
}
impl Error {
    /// The infrastructure class a stage sink reports for a store failure (P0 exit F07).
    pub fn class(&self) -> Infrastructure {
        match self {
            Self::Database(error) => {
                let code = error.as_database_error().and_then(|e| e.code()).unwrap_or_default();
                match error {
                    sqlx::Error::Io(_) | sqlx::Error::Tls(_) | sqlx::Error::Protocol(_) | sqlx::Error::PoolClosed | sqlx::Error::PoolTimedOut
                        | sqlx::Error::WorkerCrashed => Infrastructure::Transport,
                    _ if code.starts_with("08") || code.starts_with("57P") => Infrastructure::Transport,
                    _ => Infrastructure::Refused,
                }
            },
            Self::Commit(_) | Self::Rollback { .. } | Self::CopyAbort { .. } => Infrastructure::Unconfirmed,
            Self::State | Self::Busy | Self::Orphaned | Self::Confirmation => Infrastructure::State,
            Self::Contract | Self::Frontier(_) | Self::Model(_) | Self::Codec(_) => Infrastructure::Contract,
            Self::Random => Infrastructure::Io,
            Self::Driver { class, .. } => *class,
        }
    }
}
/// A store failure keeps its class across the stage sink boundary.
impl From<Error> for ModelError {
    fn from(error: Error) -> Self {
        match error {
            Error::Model(error) => error,
            Error::Frontier(message) => ModelError::Frontier(message),
            Error::Codec(message) => ModelError::Codec(message),
            other => ModelError::infrastructure(other.class(), other),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupOutcome { Removed, AlreadyAbsent }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationId([u8;16]);
impl GenerationId {
    fn new() -> Result<Self, Error> {
        let mut bytes = [0;16]; getrandom::fill(&mut bytes).map_err(|_| Error::Random)?; Ok(Self(bytes))
    }
    pub fn hex(self) -> String { self.0.iter().map(|b| format!("{b:02x}")).collect() }
    pub fn schema(self) -> String { format!("lctx_g{}", self.hex()) }
    /// The generation a `lctx_g<32 hex>` schema name belongs to.
    pub fn from_schema(name: &str) -> Option<Self> {
        let hex = name.strip_prefix("lctx_g").filter(|h| h.len() == 32 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))?;
        let mut bytes = [0; 16];
        for (i, byte) in bytes.iter_mut().enumerate() { *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?; }
        Some(Self(bytes))
    }
    /// The lock a lifecycle transition holds exclusively and a lease or copy holds shared.
    fn lock(self) -> i64 { i64::from_le_bytes(self.0[..8].try_into().expect("eight bytes")) }
    /// The lock an attempt's lifecycle connection holds for the attempt's lifetime (T10).
    fn attempt_lock(self) -> i64 { i64::from_le_bytes(self.0[8..].try_into().expect("eight bytes")) }
}
#[derive(Clone)]
pub struct GenerationStore { owner: PgPool, model: Arc<ValidatedModel>, physical: ContentHash, scopes: Arc<BTreeMap<Frontier, ddl::Scope>> }
impl GenerationStore {
    /// Install or confirm the store as the verified service owner (cutover plan P1.5/P1.6). Roles
    /// are provisioned by the bootstrap, not by schema lowering. An installation made from a
    /// different model or lowering is refused, never altered; `reset` replaces it.
    ///
    /// Only a verified owner can install:
    /// ```compile_fail
    /// # async fn f(pool: sqlx::PgPool, model: std::sync::Arc<lctx_model::domain::ValidatedModel>) {
    /// lctx_postgres::generations::GenerationStore::install(pool, model).await;
    /// # }
    /// ```
    pub async fn install(owner: crate::OwnerPool, model: Arc<ValidatedModel>) -> Result<Self, Error> {
        let store = Self::assemble(owner.pool().clone(), model);
        transaction(&store.owner, async |tx| {
            sqlx::query("SELECT pg_advisory_xact_lock(1279476824,0)").execute(&mut *tx).await?;
            install::control(tx, &store.model, store.physical).await
        }).await?;
        Ok(store)
    }
    /// The installation's physical digest is the whole model's (conformance) lowering.
    fn assemble(owner: PgPool, model: Arc<ValidatedModel>) -> Self {
        let scopes = ddl::scopes(&model);
        Self { owner, physical: scopes[&Frontier::Conformance].physical, model, scopes: Arc::new(scopes) }
    }
    fn scope(&self, frontier: Frontier) -> Result<&ddl::Scope, Error> {
        self.scopes.get(&frontier).ok_or_else(|| Error::Frontier(format!("the model has no {} scope", frontier.name())))
    }
    /// Compare the live store with a shadow install of this binary's lowering (`store check`).
    pub async fn check(owner: &crate::OwnerPool, model: &ValidatedModel) -> Result<CheckReport, Error> { verify::check(owner, model).await }
    /// What `reset` would drop, without changing anything.
    pub async fn reset_plan(owner: &crate::OwnerPool) -> Result<ResetInventory, Error> { install::plan(owner).await }
    /// Drop every inventoried generation schema and the control schema, then install `model`.
    /// `confirm` must name the database. Refused with `Busy` while any generation is leased or
    /// in a lifecycle transaction.
    pub async fn reset(owner: crate::OwnerPool, model: Arc<ValidatedModel>, confirm: &str) -> Result<(ResetInventory, Self), Error> {
        install::reset(owner, model, confirm).await
    }
    pub fn model(&self) -> &ValidatedModel { &self.model }
    pub fn physical_digest(&self) -> ContentHash { self.physical }
    /// Disposable subset evidence, advanced manually by the conformance calls below. It never
    /// holds the facts frontier and is never selectable. Unlike an attempt, a refusal here rolls
    /// back to the prior state.
    pub async fn create_conformance(&self, producer: ContentHash, profile: &str) -> Result<GenerationId, Error> {
        let g = GenerationId::new()?;
        transaction(&self.owner, async |tx| self.create(tx, g, Frontier::Conformance, false, producer, profile, None).await).await?;
        Ok(g)
    }
    /// Register a generation and lower its staging phase in the caller's transaction.
    async fn create(&self, tx: &mut PgConnection, g: GenerationId, frontier: Frontier, owned: bool, producer: ContentHash, profile: &str,
        schedule: Option<ContentHash>) -> Result<(), Error> {
        if !matches!(profile, "catalog" | "behavioral") { return Err(Error::State); }
        self.lock_installation(tx).await?;
        let physical = self.scope(frontier)?.physical;
        sqlx::query("INSERT INTO lctx_model_store.generations(id,state,owned,model_digest,physical_digest,producer_digest,profile,frontier,schedule_digest) \
            VALUES($1,'staging',$2,$3,$4,$5,$6,$7,$8)")
            .bind(g.0.to_vec()).bind(owned).bind(self.model.digest().0.to_vec()).bind(physical.0.to_vec()).bind(producer.0.to_vec()).bind(profile)
            .bind(frontier.name()).bind(schedule.map(|d| d.0.to_vec())).execute(&mut *tx).await?;
        execute(tx, self.lowering(g, frontier)?.phase("staging")).await
    }
    /// Seal an unowned conformance generation: wait out writes, then revoke the writer.
    pub async fn seal(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner, async |tx| self.seal_step(tx, g, false, None, &BTreeSet::new(), None).await).await
    }
    /// Validate an unowned conformance generation's stored, sealed contents. Callers cannot
    /// submit a `valid=true` receipt. Read buffers and validator state are charged to `budget`.
    pub async fn validate(&self, g: GenerationId, budget: &ResourceBudget) -> Result<ContentHash, Error> {
        transaction(&self.owner, async |tx| self.validate_step(tx, g, false, budget, None).await).await.map(|(content, _)| content)
    }
    pub async fn publish(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner, async |tx| self.publish_step(tx, g, false, None).await).await
    }
    pub async fn select(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        sqlx::query("SELECT singleton FROM lctx_model_store.selection FOR UPDATE").execute(&mut *tx).await?;
        lock(&mut tx, g, true).await?;
        let registered = self.registered(&mut tx, g).await?;
        if registered.state != "published" { return Err(Error::State); }
        if registered.frontier != Frontier::Facts { return Err(Error::Frontier("only a facts generation can be selected".into())); }
        sqlx::query("UPDATE lctx_model_store.selection SET generation_id=$1 WHERE singleton").bind(g.0.to_vec()).execute(&mut *tx).await?;
        Ok(())
        }).await
    }
    pub async fn clear_selection(&self) -> Result<(), Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        sqlx::query("UPDATE lctx_model_store.selection SET generation_id=NULL WHERE singleton").execute(&mut *tx).await?;
        Ok(())
        }).await
    }
    pub async fn retire(&self, g: GenerationId) -> Result<CleanupOutcome, Error> { self.cleanup(g, true, false).await }
    /// Abandon an unpublished attempt. No registry tombstone or generation event survives.
    pub async fn abort(&self, g: GenerationId) -> Result<CleanupOutcome, Error> { self.cleanup(g, false, false).await }
    /// Explicitly repair orphan objects only; ordinary cleanup refuses them.
    pub async fn repair_orphan(&self, g: GenerationId) -> Result<CleanupOutcome, Error> { self.cleanup(g, false, true).await }
    async fn cleanup(&self, g: GenerationId, published: bool, repair: bool) -> Result<CleanupOutcome, Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        let selected: Option<Vec<u8>> = sqlx::query_scalar("SELECT generation_id FROM lctx_model_store.selection WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
        if selected.as_deref() == Some(&g.0) { return Err(Error::Busy); }
        // A live attempt holds its attempt lock; a lease or transition holds the generation lock.
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1) AND pg_try_advisory_xact_lock($2)")
            .bind(g.lock()).bind(g.attempt_lock()).fetch_one(&mut *tx).await?;
        if !acquired { return Err(Error::Busy); }
        let current: Option<String> = sqlx::query_scalar("SELECT state FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).fetch_optional(&mut *tx).await?;
        let schema: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)").bind(g.schema()).fetch_one(&mut *tx).await?;
        let residue: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT {}", CONTROL_RECORDS.iter()
            .map(|table| format!("EXISTS(SELECT 1 FROM lctx_model_store.{table} WHERE generation_id=$1)")).collect::<Vec<_>>().join(" OR "))))
            .bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if current.is_none() && !schema && !residue { return Ok(CleanupOutcome::AlreadyAbsent); }
        let orphaned = current.is_none() || !schema;
        if orphaned && !repair { return Err(Error::Orphaned); }
        if repair && !orphaned { return Err(Error::State); }
        if let Some(current) = &current {
            if !repair && ((published && current != "published") || (!published && !matches!(current.as_str(), "staging" | "sealed" | "validated" | ddl::FAILED))) { return Err(Error::State); }
            self.registered(&mut tx, g).await?;
        }
        // DROP drains direct writer transactions as well as declared stage writers.
        if schema { sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(&g.schema())))).execute(&mut *tx).await?; }
        for table in CONTROL_RECORDS {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM lctx_model_store.{table} WHERE generation_id=$1"))).bind(g.0.to_vec()).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).execute(&mut *tx).await?;
        Ok(CleanupOutcome::Removed)
        }).await
    }
    fn lowering(&self, g: GenerationId, frontier: Frontier) -> Result<ddl::Lowering, Error> {
        Ok(ddl::lower(&self.model, &self.scope(frontier)?.relations, g, &g.schema(), ddl::CONTROL))
    }
    /// The relations and the invariants (all of whose inputs are held) of one frontier's scope.
    fn scoped(&self, frontier: Frontier) -> Result<(Vec<&Relation>, Vec<&lctx_model::domain::Invariant>), Error> {
        let scope = &self.scope(frontier)?.relations;
        Ok((self.model.relations().iter().filter(|r| scope.contains(r.name())).collect(),
            self.model.invariants().iter().filter(|i| i.inputs.iter().all(|input| scope.contains(input.name()))).collect()))
    }
    /// A generation's registry row, after confirming its model and frontier-scoped lowering.
    async fn registered(&self, connection: &mut PgConnection, g: GenerationId) -> Result<Registered, Error> {
        let row = sqlx::query("SELECT state,owned,model_digest,physical_digest,frontier FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec())
            .fetch_optional(connection).await?.ok_or(Error::State)?;
        let frontier = ddl::frontier(&row.try_get::<String, _>("frontier")?).ok_or(Error::Contract)?;
        if row.try_get::<Vec<u8>, _>("model_digest")? != self.model.digest().0
            || row.try_get::<Vec<u8>, _>("physical_digest")? != self.scope(frontier)?.physical.0 { return Err(Error::Contract); }
        Ok(Registered { state: row.try_get("state")?, owned: row.try_get("owned")?, frontier })
    }
    /// Attempt-owned generations whose attempt is gone: unpublished and not failed, with no
    /// lifecycle connection holding their attempt lock. They can only be aborted.
    pub async fn interrupted(&self) -> Result<Vec<GenerationId>, Error> {
        transaction(&self.owner, async |tx| {
            self.lock_installation(tx).await?;
            let ids: Vec<Vec<u8>> = sqlx::query_scalar("SELECT id FROM lctx_model_store.generations WHERE owned AND state IN ('staging','sealed','validated') ORDER BY id")
                .fetch_all(&mut *tx).await?;
            let mut interrupted = Vec::new();
            for id in ids {
                let g = GenerationId(id.try_into().map_err(|_| Error::Codec("generation id length".into()))?);
                let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)").bind(g.attempt_lock()).fetch_one(&mut *tx).await?;
                if free { interrupted.push(g); }
            }
            Ok(interrupted)
        }).await
    }
    async fn lock_installation(&self, connection: &mut PgConnection) -> Result<(), Error> {
        sqlx::query("SELECT pg_advisory_xact_lock_shared(1279476824,0)").execute(&mut *connection).await?;
        let compatible: Option<bool> = sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton")
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).fetch_optional(connection).await?;
        if compatible != Some(true) { return Err(Error::Contract); } Ok(())
    }
    /// Readers decode into the supplied budget for the lease's lifetime.
    pub async fn pin(&self, reader: &PgPool, g: GenerationId, budget: ResourceBudget) -> Result<GenerationLease, Error> {
        // Keep the pool permit; closing on drop releases both the advisory lock and pool capacity.
        let mut connection = reader.acquire().await?;
        connection.close_on_drop();
        let contract = self.lease_contract(g);
        let frontier = contract.acquire(&mut lease::Sqlx(&mut connection)).await?;
        let relations = self.scope(frontier)?.relations.clone();
        Ok(GenerationLease { connection, contract, model: self.model.clone(), budget, relations })
    }
    /// The lease protocol for one generation of this store's model.
    pub fn lease_contract(&self, g: GenerationId) -> LeaseContract {
        LeaseContract::from_scopes(self.model.digest(), self.physical, self.scopes.iter().map(|(f, s)| (*f, (s.physical, s.columns.clone()))).collect(), g)
    }
    /// Conformance writes charge COPY buffers to the caller's attempt budget.
    pub async fn copy<R: Record>(&self, writer: &PgPool, g: GenerationId, batch: &Batch<R>, budget: &ResourceBudget) -> Result<(), Error> {
        self.copy_attempt(writer, g, batch, None, budget).await
    }
    async fn copy_attempt<R: Record>(&self, writer: &PgPool, g: GenerationId, batch: &Batch<R>, schedule: Option<ContentHash>, budget: &ResourceBudget) -> Result<(), Error> {
        self.model.require::<R>()?;
        transaction(writer,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        lock(&mut tx, g, true).await?;
        let registered = self.registered(&mut tx, g).await?;
        if registered.state != "staging" { return Err(Error::State); }
        if !self.scope(registered.frontier)?.relations.contains(R::NAME) { return Err(Error::Frontier(format!("{} is outside this generation's frontier", R::NAME))); }
        check_schedule(&mut tx, g, schedule).await?;
        // Schema/encoder bookkeeping stays reserved for the COPY lifetime. The variable wire
        // buffer is admitted separately before pgpq allocates it, using pgpq's own size bound.
        let _metadata = budget.reserve("postgres-copy-metadata", batch.arrow().schema().fields().len().checked_mul(4096)
            .ok_or_else(|| Error::Codec("COPY schema accounting overflow".into()))?)?;
        let columns = batch.arrow().schema().fields().iter().map(|f| quoted(f.name())).collect::<Vec<_>>().join(",");
        let mut encoder = pgpq::ArrowToPostgresBinaryEncoder::try_new(&batch.arrow().schema()).map_err(|e| Error::Codec(e.to_string()))?;
        let builders = batch.arrow().schema().fields().iter().map(|f| EncoderBuilder::try_new(f.clone())).collect::<Result<Vec<_>,_>>()
            .map_err(|e| Error::Codec(e.to_string()))?;
        let header_charge = budget.reserve("postgres-copy-wire", 64)?;
        let mut bytes = BytesMut::with_capacity(32);
        encoder.write_header(&mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
        let mut copy = tx.copy_in_raw(&format!("COPY {} ({columns}) FROM STDIN BINARY", qualified(g, R::NAME))).await?;
        let result: Result<(),Error> = async {
        copy.send(bytes.freeze()).await?;
        drop(header_charge);
        for index in 0..batch.arrow().num_rows() {
            let row = batch.arrow().slice(index, 1);
            let size = copy_size(&row, &builders)?;
            // The generated server-side row also carries the generation column (20 bytes).
            if size > MAX_ROW_BYTES - 20 { return Err(Error::Model(ModelError::Limit { owner: R::NAME, limit: "COPY row bytes", observed: size, bound: MAX_ROW_BYTES - 20 })); }
            let _wire = budget.reserve("postgres-copy-wire", size.checked_mul(2)
                .ok_or_else(|| Error::Codec("COPY wire accounting overflow".into()))?)?;
            let mut bytes = BytesMut::with_capacity(size);
            encoder.write_batch(&row, &mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
            if bytes.len() > size { return Err(Error::Codec("COPY encoder exceeded its admitted size".into())); }
            copy.send(bytes.freeze()).await?;
        }
        let _footer = budget.reserve("postgres-copy-wire", 64)?;
        let mut bytes = BytesMut::with_capacity(32);
        encoder.write_footer(&mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
        copy.send(bytes.freeze()).await?;
        Ok(())
        }.await;
        match result {
            Ok(()) => { copy.finish().await?; },
            Err(operation) => {
                if let Err(source) = copy.abort("typed generation COPY refused").await {
                    return Err(Error::CopyAbort { operation: Box::new(operation),source });
                }
                return Err(operation);
            },
        }
        Ok(())
        }).await
    }
}

fn copy_size(row: &RecordBatch, builders: &[EncoderBuilder]) -> Result<usize, Error> {
    row.columns().iter().zip(builders).try_fold(2usize, |bytes, (column,builder)| {
        let encoder = builder.try_new(column.as_ref()).map_err(|e| Error::Codec(e.to_string()))?;
        let size = encoder.byte_size_hint().map_err(|e| Error::Codec(e.to_string()))?;
        bytes.checked_add(size).ok_or_else(|| Error::Codec("COPY row size overflow".into()))
    })
}

/// The control records a generation owns; cleanup removes them with its schema.
const CONTROL_RECORDS: [&str; 8] = ["receipts", "validation_receipts", "stage_receipts", "planned_outputs", "stage_outcomes", "admissions", "failures", "events"];
/// A registry row's lifecycle facts.
struct Registered { state: String, owned: bool, frontier: Frontier }
impl Registered {
    /// The state a step requires, and whether an attempt or a manual conformance call owns it.
    fn expect(&self, state: &str, owned: bool) -> Result<(), Error> {
        if self.state != state || self.owned != owned { return Err(Error::State); }
        Ok(())
    }
}

async fn check_schedule(connection: &mut PgConnection, g: GenerationId, expected: Option<ContentHash>) -> Result<(), Error> {
    let actual: Option<Vec<u8>> = sqlx::query_scalar("SELECT schedule_digest FROM lctx_model_store.generations WHERE id=$1")
        .bind(g.0.to_vec()).fetch_one(connection).await?;
    if actual.as_deref() != expected.as_ref().map(|d| d.0.as_slice()) { return Err(Error::Contract); }
    Ok(())
}

pub struct GenerationLease {
    connection: sqlx::pool::PoolConnection<sqlx::Postgres>, contract: LeaseContract, model: Arc<ValidatedModel>, budget: ResourceBudget,
    /// The relations the generation's frontier holds; any other is refused before a scan.
    relations: BTreeSet<&'static str>,
}
impl GenerationLease {
    pub fn generation(&self) -> GenerationId { self.contract.generation() }
    /// Consume the reader and wait for server acknowledgment that its session lease is released.
    /// Drop still closes the connection conservatively, but does not acknowledge lock release.
    /// The connection remains close-on-drop on error or cancellation and is never pooled again.
    pub async fn release(mut self) -> Result<(), Error> {
        self.contract.release(&mut lease::Sqlx(&mut self.connection)).await?;
        self.connection.close().await?;
        Ok(())
    }
    /// Visit bounded typed batches while borrowing the original leased connection.
    /// No connection reacquisition is permitted after a transport failure.
    pub async fn visit<R: Record>(&mut self, mut visitor: impl FnMut(Batch<R>) -> Result<(), Error>) -> Result<(), Error> {
        if !self.relations.contains(R::NAME) { return Err(Error::Frontier(format!("{} is outside this generation's frontier", R::NAME))); }
        self.connection.ping().await?;
        let relation = self.model.require::<R>()?;
        visit_physical(&mut self.connection, self.contract.generation(), relation, &["id"], &self.budget, |batch| {
            visitor(Batch::read(&self.model, &batch, &self.budget)?)
        }).await
    }
    /// Convenience for small relations. Larger consumers must use `visit`.
    pub async fn read<R: Record>(&mut self) -> Result<Batch<R>, Error> {
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        self.visit::<R>(|batch| {
            bytes = bytes.checked_add(batch.arrow().get_array_memory_size()).ok_or_else(|| Error::Codec("read budget overflow".into()))?;
            if bytes > TRANSFER_BYTES { return Err(Error::Model(ModelError::Limit { owner: R::NAME, limit: "small-relation read bytes (use visit)", observed: bytes, bound: TRANSFER_BYTES })); }
            rows.extend_from_slice(batch.rows());
            Ok(())
        }).await?;
        Ok(Batch::new(&self.model, rows, &self.budget)?)
    }
}

/// Rows are buffered up to one transfer batch. Raw rows and their decoded form are admitted
/// before a row is retained, and released once the visitor has consumed the batch.
async fn visit_physical(connection: &mut PgConnection, g: GenerationId, relation: &Relation, order: &[&str],
    budget: &ResourceBudget, mut visitor: impl FnMut(RecordBatch) -> Result<(), Error>) -> Result<(), Error> {
    let columns = relation.schema().fields().iter().map(|f| quoted(f.name())).collect::<Vec<_>>().join(",");
    let order = order.iter().map(|name| {
        let text = relation.fields().iter().any(|field| field.name() == *name && field.scalar() == lctx_model::domain::Scalar::Text && !field.list());
        if text { format!("{} COLLATE \"C\"", quoted(name)) } else { quoted(name) }
    }).collect::<Vec<_>>().join(",");
    let query = format!("SELECT {columns} FROM {} ORDER BY {order}", qualified(g, relation.name()));
    let mut stream = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(connection);
    let mut rows = Vec::new();
    let mut bytes = 0usize;
    let mut held = budget.reserve("postgres-read", 0)?;
    while let Some(row) = stream.try_next().await? {
        let mut size = 0usize;
        for index in 0..row.len() {
            let value = row.try_get_raw(index)?;
            if !value.is_null() {
                size = size.checked_add(value.as_bytes().map_err(|e| Error::Codec(e.to_string()))?.len())
                    .ok_or_else(|| Error::Codec("row size overflow".into()))?;
            }
        }
        if size > MAX_ROW_BYTES { return Err(Error::Model(ModelError::Limit { owner: relation.name(), limit: "stored row bytes", observed: size, bound: MAX_ROW_BYTES })); }
        if !rows.is_empty() && (rows.len() == TRANSFER_ROWS || bytes.saturating_add(size) > TRANSFER_BYTES) {
            visitor(codec::decode(relation, &rows)?)?;
            rows.clear(); bytes = 0; held.try_resize(0)?;
        }
        held.try_resize(bytes.saturating_add(size).saturating_mul(3))?;
        bytes += size;
        rows.push(row);
    }
    if !rows.is_empty() { visitor(codec::decode(relation, &rows)?)?; }
    Ok(())
}
async fn lock(connection: &mut PgConnection, g: GenerationId, shared: bool) -> Result<(), Error> {
    let query = if shared { "SELECT pg_advisory_xact_lock_shared($1)" } else { "SELECT pg_advisory_xact_lock($1)" };
    sqlx::query(query).bind(g.lock()).execute(connection).await?; Ok(())
}
async fn transition(connection: &mut PgConnection, g: GenerationId, next: &str) -> Result<(), Error> {
    sqlx::query("UPDATE lctx_model_store.generations SET state=$2 WHERE id=$1").bind(g.0.to_vec()).bind(next).execute(&mut *connection).await?;
    sqlx::query("INSERT INTO lctx_model_store.events(generation_id,state) VALUES($1,$2)").bind(g.0.to_vec()).bind(next).execute(connection).await?; Ok(())
}
async fn execute(connection: &mut PgConnection, statements: Vec<String>) -> Result<(), Error> {
    for sql in statements { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *connection).await?; }
    Ok(())
}
fn quoted(name: &str) -> String { format!("\"{}\"", name.replace('"', "\"\"")) }
fn qualified(g: GenerationId, relation: &str) -> String { format!("{}.{}", quoted(&g.schema()), quoted(relation)) }

/// Completed domain refusals imply acknowledged rollback. Commit/rollback transport failures
/// remain explicitly unconfirmed and quarantine the connection. Cancellation uses SQLx drop cleanup.
async fn transaction<T>(pool: &PgPool,body: impl for<'a> AsyncFnOnce(&'a mut PgConnection) -> Result<T,Error>) -> Result<T,Error> {
    let mut connection = pool.acquire().await?;
    let result = transaction_on(&mut connection,body).await;
    if matches!(&result,Err(Error::Commit(_)|Error::Rollback { .. })) { connection.close_on_drop(); }
    result
}
async fn transaction_on<T>(connection: &mut PgConnection,body: impl for<'a> AsyncFnOnce(&'a mut PgConnection) -> Result<T,Error>) -> Result<T,Error> {
    let mut tx = connection.begin().await?;
    match body(&mut tx).await {
        Ok(value) => { tx.commit().await.map_err(Error::Commit)?; Ok(value) },
        Err(operation) => match tx.rollback().await {
            Ok(()) => Err(operation),
            Err(source) => Err(Error::Rollback { operation: Box::new(operation),source }),
        },
    }
}
