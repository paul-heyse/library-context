//! Immutable generation schemas lowered exclusively from a validated semantic model (ADR-0086).
mod codec;
mod ddl;
use std::{collections::BTreeSet, sync::{Arc,Mutex}};
use arrow_array::RecordBatch;
use bytes::BytesMut;
use lctx_model::domain::{Batch, ContentHash, KeySink, ModelError, Record, Relation, ValidatedModel};
use lctx_model::domain::stages::{AttemptIdentity, Execution, ExecutionReceipt, StageSink, WritePermit};
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
    #[error("conformance subset cannot be selected as a production facts generation")] Frontier,
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
    fn lock(self) -> i64 { i64::from_le_bytes(self.0[..8].try_into().expect("eight bytes")) }
}
#[derive(Clone)]
pub struct GenerationStore { owner: PgPool, model: Arc<ValidatedModel>, physical: ContentHash }
impl GenerationStore {
    /// Roles are provisioned by the existing PostgreSQL bootstrap, not by schema lowering.
    pub async fn install(owner: PgPool, model: Arc<ValidatedModel>) -> Result<Self, Error> {
        let physical = ddl::digest(&model);
        transaction(&owner,async |tx| {
        sqlx::query("SELECT pg_advisory_xact_lock(1279476824,0)").execute(&mut *tx).await?;
        sqlx::raw_sql(include_str!("control.sql")).execute(&mut *tx).await?;
        let existing: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation WHERE singleton FOR UPDATE").fetch_optional(&mut *tx).await?;
        if let Some((m, p)) = existing {
            if m != model.digest().0 || p != physical.0 { return Err(Error::Contract); }
        } else {
            sqlx::query("INSERT INTO lctx_model_store.installation VALUES (true,$1,$2)").bind(model.digest().0.to_vec()).bind(physical.0.to_vec()).execute(&mut *tx).await?;
        }
        Ok(())
        }).await?;
        Ok(Self { owner, model, physical })
    }
    pub fn model(&self) -> &ValidatedModel { &self.model }
    pub fn physical_digest(&self) -> ContentHash { self.physical }
    /// Disposable subset evidence. This method cannot manufacture production facts admission.
    /// Complete producer/model/schedule/coverage admission is added with the facts assembler.
    pub async fn create_conformance(&self, producer: ContentHash, profile: &str) -> Result<GenerationId, Error> {
        self.create_subset(producer, profile, None).await
    }
    /// Exercise the permanent stage-bound sink without claiming a production facts frontier.
    pub async fn begin_conformance(&self, writer: PgPool, execution: &mut Execution<'_>, budget: ResourceBudget) -> Result<GenerationAttempt, Error> {
        execution.bind_sink()?;
        let schedule = execution.schedule();
        if schedule.model() != self.model.digest() { return Err(Error::Contract); }
        let generation = self.create_subset(schedule.digest(), schedule.profile().name(), Some(schedule.digest())).await?;
        let expected = schedule.stages().iter().flat_map(|s| s.outputs.iter().map(move |r| (s.name,r.name()))).collect();
        Ok(GenerationAttempt { store: self.clone(), writer, generation, identity: execution.identity(), schedule: schedule.digest(), budget,
            expected, written: Mutex::new(BTreeSet::new()) })
    }
    async fn create_subset(&self, producer: ContentHash, profile: &str, schedule: Option<ContentHash>) -> Result<GenerationId, Error> {
        if !matches!(profile, "catalog" | "behavioral") { return Err(Error::State); }
        let g = GenerationId::new()?;
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        let compatible: bool = sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton FOR SHARE")
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).fetch_one(&mut *tx).await?;
        if !compatible { return Err(Error::Contract); }
        sqlx::query("INSERT INTO lctx_model_store.generations(id,state,model_digest,physical_digest,producer_digest,profile,frontier,schedule_digest) VALUES($1,'staging',$2,$3,$4,$5,'conformance',$6)")
            .bind(g.0.to_vec()).bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).bind(producer.0.to_vec()).bind(profile)
            .bind(schedule.map(|d| d.0.to_vec())).execute(&mut *tx).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {}", quoted(&g.schema())))).execute(&mut *tx).await?;
        let generated = ddl::generate(&self.model, g);
        for sql in generated.tables.into_iter().chain(generated.views) { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *tx).await?; }
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("GRANT USAGE ON SCHEMA {s} TO lctx_importer; GRANT INSERT ON ALL TABLES IN SCHEMA {s} TO lctx_importer", s=quoted(&g.schema())))).execute(&mut *tx).await?;
        Ok(g)
        }).await
    }
    /// Sealing takes the exclusive generation lock and table locks before revoking access.
    pub async fn seal(&self, g: GenerationId) -> Result<(), Error> {
        self.seal_attempt(g, None, &BTreeSet::new()).await
    }
    async fn seal_attempt(&self, g: GenerationId, schedule: Option<ContentHash>, outputs: &BTreeSet<(&str,&str)>) -> Result<(), Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "staging", &self.model, self.physical).await?;
        check_schedule(&mut tx, g, schedule).await?;
        for relation in self.model.relations() {
            sqlx::query(sqlx::AssertSqlSafe(format!("LOCK TABLE {} IN ACCESS EXCLUSIVE MODE", qualified(g, relation.name())))).execute(&mut *tx).await?;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("REVOKE ALL ON ALL TABLES IN SCHEMA {} FROM lctx_importer", quoted(&g.schema())))).execute(&mut *tx).await?;
        if let Some(schedule) = schedule {
            for (stage, relation) in outputs {
                sqlx::query("INSERT INTO lctx_model_store.stage_receipts VALUES($1,$2,$3,$4)")
                    .bind(g.0.to_vec()).bind(stage).bind(relation).bind(schedule.0.to_vec()).execute(&mut *tx).await?;
            }
        }
        transition(&mut tx, g, "sealed").await?;
        Ok(())
        }).await
    }
    /// Validate stored, sealed contents. Callers cannot submit a `valid=true` receipt.
    pub async fn validate(&self, g: GenerationId) -> Result<ContentHash, Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "sealed", &self.model, self.physical).await?;
        for sql in ddl::generate(&self.model, g).references { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *tx).await?; }
        let mut content = KeySink::new("generation-content");
        for relation in self.model.relations() {
            let mut rows = relation.content();
            visit_physical(&mut tx, g, relation, &["id"], |batch| {
                relation.hash_rows(&batch, &mut rows)?;
                Ok(())
            }).await?;
            let (row_count, digest) = rows.finish();
            content.part(relation.name().as_bytes(), &digest.0);
            sqlx::query("INSERT INTO lctx_model_store.receipts(generation_id,relation_name,row_count,content_digest) VALUES($1,$2,$3,$4)")
                .bind(g.0.to_vec()).bind(relation.name()).bind(i64::try_from(row_count).map_err(|_| Error::Codec("row count overflow".into()))?).bind(digest.0.to_vec()).execute(&mut *tx).await?;
        }
        let digest = content.finish();
        for invariant in self.model.invariants() {
            let mut check = (invariant.create)();
            for input in &invariant.inputs {
                let relation = self.model.relations().iter().find(|r| r.name() == input.name()).expect("validated invariant member");
                visit_physical(&mut tx, g, relation, input.order(), |batch| {
                    check.visit(input.name(), &batch)?;
                    Ok(())
                }).await?;
            }
            check.finish()?;
            sqlx::query("INSERT INTO lctx_model_store.validation_receipts(generation_id,validator_name,content_digest,model_digest,physical_digest) VALUES($1,$2,$3,$4,$5)")
                .bind(g.0.to_vec()).bind(invariant.name).bind(digest.0.to_vec())
                .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE lctx_model_store.generations SET content_digest=$2 WHERE id=$1").bind(g.0.to_vec()).bind(digest.0.to_vec()).execute(&mut *tx).await?;
        transition(&mut tx, g, "validated").await?;
        Ok(digest)
        }).await
    }
    pub async fn publish(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "validated", &self.model, self.physical).await?;
        let receipts = sqlx::query("SELECT relation_name,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 ORDER BY relation_name COLLATE \"C\"")
            .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
        if receipts.len() != self.model.relations().len() { return Err(Error::State); }
        let mut content = KeySink::new("generation-content");
        for (receipt, relation) in receipts.iter().zip(self.model.relations()) {
            if receipt.try_get::<String, _>("relation_name")? != relation.name() { return Err(Error::Contract); }
            content.part(relation.name().as_bytes(), &receipt.try_get::<Vec<u8>, _>("content_digest")?);
        }
        let digest = content.finish();
        let stored: Vec<u8> = sqlx::query_scalar("SELECT content_digest FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if stored != digest.0 { return Err(Error::Contract); }
        let validations = sqlx::query("SELECT validator_name,content_digest,model_digest,physical_digest FROM lctx_model_store.validation_receipts WHERE generation_id=$1 ORDER BY validator_name COLLATE \"C\"")
            .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
        if validations.len() != self.model.invariants().len() { return Err(Error::State); }
        for (receipt, invariant) in validations.iter().zip(self.model.invariants()) {
            if receipt.try_get::<String, _>("validator_name")? != invariant.name
                || receipt.try_get::<Vec<u8>, _>("content_digest")? != digest.0
                || receipt.try_get::<Vec<u8>, _>("model_digest")? != self.model.digest().0
                || receipt.try_get::<Vec<u8>, _>("physical_digest")? != self.physical.0 { return Err(Error::Contract); }
        }
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("GRANT USAGE ON SCHEMA {s} TO lctx_serving; GRANT SELECT ON ALL TABLES IN SCHEMA {s} TO lctx_serving", s=quoted(&g.schema())))).execute(&mut *tx).await?;
        transition(&mut tx, g, "published").await?;
        Ok(())
        }).await
    }
    pub async fn select(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        sqlx::query("SELECT singleton FROM lctx_model_store.selection FOR UPDATE").execute(&mut *tx).await?;
        lock(&mut tx, g, true).await?;
        state(&mut tx, g, "published", &self.model, self.physical).await?;
        let frontier: String = sqlx::query_scalar("SELECT frontier FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if frontier == "conformance" { return Err(Error::Frontier); }
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
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)").bind(g.lock()).fetch_one(&mut *tx).await?;
        if !acquired { return Err(Error::Busy); }
        let current: Option<String> = sqlx::query_scalar("SELECT state FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).fetch_optional(&mut *tx).await?;
        let schema: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)").bind(g.schema()).fetch_one(&mut *tx).await?;
        let residue: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.receipts WHERE generation_id=$1) OR EXISTS(SELECT 1 FROM lctx_model_store.validation_receipts WHERE generation_id=$1) OR EXISTS(SELECT 1 FROM lctx_model_store.events WHERE generation_id=$1) OR EXISTS(SELECT 1 FROM lctx_model_store.stage_receipts WHERE generation_id=$1)")
            .bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if current.is_none() && !schema && !residue { return Ok(CleanupOutcome::AlreadyAbsent); }
        let orphaned = current.is_none() || !schema;
        if orphaned && !repair { return Err(Error::Orphaned); }
        if repair && !orphaned { return Err(Error::State); }
        if let Some(current) = &current {
            if !repair && ((published && current != "published") || (!published && !matches!(current.as_str(), "staging" | "sealed" | "validated"))) { return Err(Error::State); }
            state(&mut tx,g,current,&self.model,self.physical).await?;
        }
        // DROP drains direct writer transactions as well as declared stage writers.
        if schema { sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(&g.schema())))).execute(&mut *tx).await?; }
        for table in ["receipts","validation_receipts","stage_receipts","events"] {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM lctx_model_store.{table} WHERE generation_id=$1"))).bind(g.0.to_vec()).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).execute(&mut *tx).await?;
        Ok(CleanupOutcome::Removed)
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
        transaction_on(&mut connection,async |mut tx| {
        self.lock_installation(&mut tx).await?;
        lock(&mut tx,g,true).await?;
        state(&mut tx, g, "published", &self.model, self.physical).await?;
        sqlx::query("SELECT pg_advisory_lock_shared($1)").bind(g.lock()).execute(&mut *tx).await?;
        Ok(())
        }).await?;
        Ok(GenerationLease { connection, generation: g, model: self.model.clone(), budget })
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
        state(&mut tx, g, "staging", &self.model, self.physical).await?;
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
            if size > MAX_ROW_BYTES - 20 { return Err(Error::Codec("COPY row exceeds storage admission limit".into())); }
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

/// An unpublished generation belongs to exactly one execution. Neither a copied schedule nor a
/// different invocation of that schedule can write or seal it. Dropping this handle never seals;
/// interrupted attempts are subsequently aborted by the lifecycle owner.
pub struct GenerationAttempt {
    store: GenerationStore, writer: PgPool, generation: GenerationId,
    identity: AttemptIdentity, schedule: ContentHash, budget: ResourceBudget,
    expected: BTreeSet<(&'static str,&'static str)>, written: Mutex<BTreeSet<(&'static str,&'static str)>>,
}
impl GenerationAttempt {
    pub fn generation(&self) -> GenerationId { self.generation }
    pub async fn copy<R: Record>(&self, permit: WritePermit<'_, R>, batch: &Batch<R>) -> Result<(), ModelError> {
        if permit.identity().attempt() != self.identity || permit.model() != self.store.model.digest() {
            return Err(ModelError::Invalid("write permit belongs to another generation attempt".into()));
        }
        self.store.copy_attempt(&self.writer, self.generation, batch, Some(self.schedule), &self.budget).await.map_err(|error| match error {
            Error::Model(error) => error, other => ModelError::codec(other),
        })?;
        self.written.lock().map_err(|_| ModelError::Invalid("generation output completion poisoned".into()))?.insert((permit.stage(),R::NAME));
        Ok(())
    }
    pub async fn seal(self, receipt: ExecutionReceipt) -> Result<GenerationId, Error> {
        if receipt.identity() != self.identity || receipt.model() != self.store.model.digest() || receipt.schedule() != self.schedule {
            return Err(Error::Contract);
        }
        let written = self.written.into_inner().map_err(|_| Error::State)?;
        if written != self.expected { return Err(Error::State); }
        self.store.seal_attempt(self.generation, Some(self.schedule), &written).await?;
        Ok(self.generation)
    }
}

impl StageSink for GenerationAttempt {
    fn copy<R: Record>(&self, permit: WritePermit<'_, R>, batch: &Batch<R>) -> impl Future<Output = Result<(), ModelError>> + Send {
        GenerationAttempt::copy(self, permit, batch)
    }
}

fn copy_size(row: &RecordBatch, builders: &[EncoderBuilder]) -> Result<usize, Error> {
    row.columns().iter().zip(builders).try_fold(2usize, |bytes, (column,builder)| {
        let encoder = builder.try_new(column.as_ref()).map_err(|e| Error::Codec(e.to_string()))?;
        let size = encoder.byte_size_hint().map_err(|e| Error::Codec(e.to_string()))?;
        bytes.checked_add(size).ok_or_else(|| Error::Codec("COPY row size overflow".into()))
    })
}

async fn check_schedule(connection: &mut PgConnection, g: GenerationId, expected: Option<ContentHash>) -> Result<(), Error> {
    let actual: Option<Vec<u8>> = sqlx::query_scalar("SELECT schedule_digest FROM lctx_model_store.generations WHERE id=$1")
        .bind(g.0.to_vec()).fetch_one(connection).await?;
    if actual.as_deref() != expected.as_ref().map(|d| d.0.as_slice()) { return Err(Error::Contract); }
    Ok(())
}

pub struct GenerationLease { connection: sqlx::pool::PoolConnection<sqlx::Postgres>, generation: GenerationId, model: Arc<ValidatedModel>, budget: ResourceBudget }
impl GenerationLease {
    pub fn generation(&self) -> GenerationId { self.generation }
    /// Consume the reader and wait for server acknowledgment that its session lease is released.
    /// Drop still closes the connection conservatively, but does not acknowledge lock release.
    /// The connection remains close-on-drop on error or cancellation and is never pooled again.
    pub async fn release(mut self) -> Result<(), Error> {
        let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock_shared($1)")
            .bind(self.generation.lock()).fetch_one(&mut *self.connection).await?;
        if !released { return Err(Error::State); }
        self.connection.close().await?;
        Ok(())
    }
    /// Visit bounded typed batches while borrowing the original leased connection.
    /// No connection reacquisition is permitted after a transport failure.
    pub async fn visit<R: Record>(&mut self, mut visitor: impl FnMut(Batch<R>) -> Result<(), Error>) -> Result<(), Error> {
        self.connection.ping().await?;
        let relation = self.model.require::<R>()?;
        visit_physical(&mut self.connection, self.generation, relation, &["id"], |batch| {
            visitor(Batch::read(&self.model, &batch, &self.budget)?)
        }).await
    }
    /// Convenience for small relations. Larger consumers must use `visit`.
    pub async fn read<R: Record>(&mut self) -> Result<Batch<R>, Error> {
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        self.visit::<R>(|batch| {
            bytes = bytes.checked_add(batch.arrow().get_array_memory_size()).ok_or_else(|| Error::Codec("read budget overflow".into()))?;
            if bytes > TRANSFER_BYTES { return Err(Error::Codec("small-relation read budget exceeded; use visit".into())); }
            rows.extend_from_slice(batch.rows());
            Ok(())
        }).await?;
        Ok(Batch::new(&self.model, rows, &self.budget)?)
    }
}

async fn visit_physical(connection: &mut PgConnection, g: GenerationId, relation: &Relation, order: &[&str],
    mut visitor: impl FnMut(RecordBatch) -> Result<(), Error>) -> Result<(), Error> {
    let columns = relation.schema().fields().iter().map(|f| quoted(f.name())).collect::<Vec<_>>().join(",");
    let order = order.iter().map(|name| {
        let text = relation.fields().iter().any(|field| field.name() == *name && field.scalar() == lctx_model::domain::Scalar::Text && !field.list());
        if text { format!("{} COLLATE \"C\"", quoted(name)) } else { quoted(name) }
    }).collect::<Vec<_>>().join(",");
    let query = format!("SELECT {columns} FROM {} ORDER BY {order}", qualified(g, relation.name()));
    let mut stream = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(connection);
    let mut rows = Vec::new();
    let mut bytes = 0usize;
    while let Some(row) = stream.try_next().await? {
        let mut size = 0usize;
        for index in 0..row.len() {
            let value = row.try_get_raw(index)?;
            if !value.is_null() {
                size = size.checked_add(value.as_bytes().map_err(|e| Error::Codec(e.to_string()))?.len())
                    .ok_or_else(|| Error::Codec("row size overflow".into()))?;
            }
        }
        if size > MAX_ROW_BYTES { return Err(Error::Codec("stored row exceeds read budget".into())); }
        if !rows.is_empty() && (rows.len() == TRANSFER_ROWS || bytes.saturating_add(size) > TRANSFER_BYTES) {
            visitor(codec::decode(relation, &rows)?)?;
            rows.clear(); bytes = 0;
        }
        bytes += size;
        rows.push(row);
    }
    if !rows.is_empty() { visitor(codec::decode(relation, &rows)?)?; }
    Ok(())
}
async fn state(connection: &mut PgConnection, g: GenerationId, expected: &str, model: &ValidatedModel, physical: ContentHash) -> Result<(), Error> {
    let row = sqlx::query("SELECT state,model_digest,physical_digest FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).fetch_optional(connection).await?.ok_or(Error::State)?;
    if row.try_get::<Vec<u8>, _>("model_digest")? != model.digest().0 || row.try_get::<Vec<u8>, _>("physical_digest")? != physical.0 { return Err(Error::Contract); }
    if row.try_get::<String, _>("state")? != expected { return Err(Error::State); }
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
