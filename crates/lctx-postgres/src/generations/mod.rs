//! Immutable generation schemas lowered exclusively from a validated semantic model (ADR-0086).
mod codec;
mod ddl;
use std::sync::Arc;
use arrow_array::RecordBatch;
use bytes::BytesMut;
use lctx_model::domain::{Batch, ContentHash, KeySink, ModelError, Record, Relation, ValidatedModel};
use sqlx::{Connection, PgConnection, PgPool, Row};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")] Model(#[from] ModelError),
    #[error("PostgreSQL operation failed")] Database(#[from] sqlx::Error),
    #[error("generation state does not permit this operation")] State,
    #[error("generation model or physical schema differs from this binary")] Contract,
    #[error("generation has active readers or is selected")] Busy,
    #[error("codec: {0}")] Codec(String),
    #[error("secure generation randomness unavailable")] Random,
}
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
        let mut tx = owner.begin().await?;
        sqlx::raw_sql(include_str!("control.sql")).execute(&mut *tx).await?;
        let existing: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation WHERE singleton FOR UPDATE").fetch_optional(&mut *tx).await?;
        if let Some((m, p)) = existing {
            if m != model.digest().0 || p != physical.0 { return Err(Error::Contract); }
        } else {
            sqlx::query("INSERT INTO lctx_model_store.installation VALUES (true,$1,$2)").bind(model.digest().0.to_vec()).bind(physical.0.to_vec()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(Self { owner, model, physical })
    }
    pub fn model(&self) -> &ValidatedModel { &self.model }
    pub fn physical_digest(&self) -> ContentHash { self.physical }
    pub async fn create(&self, producer: ContentHash, profile: &str) -> Result<GenerationId, Error> {
        if !matches!(profile, "catalog" | "behavioral") { return Err(Error::State); }
        let g = GenerationId::new()?;
        let mut tx = self.owner.begin().await?;
        let compatible: bool = sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton FOR SHARE")
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).fetch_one(&mut *tx).await?;
        if !compatible { return Err(Error::Contract); }
        sqlx::query("INSERT INTO lctx_model_store.generations(id,state,model_digest,physical_digest,producer_digest,profile,frontier) VALUES($1,'staging',$2,$3,$4,$5,'facts')")
            .bind(g.0.to_vec()).bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).bind(producer.0.to_vec()).bind(profile).execute(&mut *tx).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {}", quoted(&g.schema())))).execute(&mut *tx).await?;
        let generated = ddl::generate(&self.model, g);
        for sql in generated.tables { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *tx).await?; }
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("GRANT USAGE ON SCHEMA {s} TO lctx_importer; GRANT INSERT ON ALL TABLES IN SCHEMA {s} TO lctx_importer", s=quoted(&g.schema())))).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(g)
    }
    /// Sealing takes the exclusive generation lock and table locks before revoking access.
    pub async fn seal(&self, g: GenerationId) -> Result<(), Error> {
        let mut tx = self.owner.begin().await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "staging", &self.model, self.physical).await?;
        for relation in self.model.relations() {
            sqlx::query(sqlx::AssertSqlSafe(format!("LOCK TABLE {} IN ACCESS EXCLUSIVE MODE", qualified(g, relation.name())))).execute(&mut *tx).await?;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("REVOKE ALL ON ALL TABLES IN SCHEMA {} FROM lctx_importer", quoted(&g.schema())))).execute(&mut *tx).await?;
        transition(&mut tx, g, "sealed").await?;
        tx.commit().await?; Ok(())
    }
    /// Validate stored, sealed contents. Callers cannot submit a `valid=true` receipt.
    pub async fn validate(&self, g: GenerationId) -> Result<ContentHash, Error> {
        let mut tx = self.owner.begin().await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "sealed", &self.model, self.physical).await?;
        for sql in ddl::generate(&self.model, g).references { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *tx).await?; }
        let mut content = KeySink::new("generation-content");
        for relation in self.model.relations() {
            let batch = read_physical(&mut tx, g, relation).await?;
            let canonical = relation.canonical(&batch)?;
            let mut bytes = Vec::new();
            {
                let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut bytes, &canonical.schema()).map_err(|e| Error::Codec(e.to_string()))?;
                writer.write(&canonical).map_err(|e| Error::Codec(e.to_string()))?;
                writer.finish().map_err(|e| Error::Codec(e.to_string()))?;
            }
            let digest = ContentHash::of(&bytes);
            content.part(relation.name().as_bytes(), &digest.0);
            sqlx::query("INSERT INTO lctx_model_store.receipts(generation_id,relation_name,row_count,content_digest) VALUES($1,$2,$3,$4)")
                .bind(g.0.to_vec()).bind(relation.name()).bind(canonical.num_rows() as i64).bind(digest.0.to_vec()).execute(&mut *tx).await?;
        }
        let digest = content.finish();
        sqlx::query("UPDATE lctx_model_store.generations SET content_digest=$2 WHERE id=$1").bind(g.0.to_vec()).bind(digest.0.to_vec()).execute(&mut *tx).await?;
        transition(&mut tx, g, "validated").await?;
        tx.commit().await?; Ok(digest)
    }
    pub async fn publish(&self, g: GenerationId) -> Result<(), Error> {
        let mut tx = self.owner.begin().await?;
        lock(&mut tx, g, false).await?;
        state(&mut tx, g, "validated", &self.model, self.physical).await?;
        let receipt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.receipts WHERE generation_id=$1").bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if receipt_count != self.model.relations().len() as i64 { return Err(Error::State); }
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("GRANT USAGE ON SCHEMA {s} TO lctx_serving; GRANT SELECT ON ALL TABLES IN SCHEMA {s} TO lctx_serving", s=quoted(&g.schema())))).execute(&mut *tx).await?;
        transition(&mut tx, g, "published").await?;
        tx.commit().await?; Ok(())
    }
    pub async fn select(&self, g: GenerationId) -> Result<(), Error> {
        let mut tx = self.owner.begin().await?;
        sqlx::query("SELECT singleton FROM lctx_model_store.selection FOR UPDATE").execute(&mut *tx).await?;
        lock(&mut tx, g, true).await?;
        state(&mut tx, g, "published", &self.model, self.physical).await?;
        sqlx::query("UPDATE lctx_model_store.selection SET generation_id=$1 WHERE singleton").bind(g.0.to_vec()).execute(&mut *tx).await?;
        tx.commit().await?; Ok(())
    }
    pub async fn retire(&self, g: GenerationId) -> Result<(), Error> {
        let mut tx = self.owner.begin().await?;
        let selected: Option<Vec<u8>> = sqlx::query_scalar("SELECT generation_id FROM lctx_model_store.selection WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
        if selected.as_deref() == Some(&g.0) { return Err(Error::Busy); }
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)").bind(g.lock()).fetch_one(&mut *tx).await?;
        if !acquired { return Err(Error::Busy); }
        state(&mut tx, g, "published", &self.model, self.physical).await?;
        // Only the owner can create dependencies. All generated dependencies live in this schema.
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(&g.schema())))).execute(&mut *tx).await?;
        transition(&mut tx, g, "retired").await?;
        tx.commit().await?; Ok(())
    }
    /// Abandon an unpublished attempt, draining writers before removing its schema atomically.
    pub async fn abort(&self, g: GenerationId) -> Result<(), Error> {
        let mut tx = self.owner.begin().await?;
        lock(&mut tx, g, false).await?;
        let current: String = sqlx::query_scalar("SELECT state FROM lctx_model_store.generations WHERE id=$1")
            .bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if current == "failed" { tx.commit().await?; return Ok(()); }
        if !matches!(current.as_str(), "staging" | "sealed" | "validated") { return Err(Error::State); }
        state(&mut tx, g, &current, &self.model, self.physical).await?;
        // DROP takes exclusive relation locks, including for direct writer transactions.
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(&g.schema())))).execute(&mut *tx).await?;
        transition(&mut tx, g, "failed").await?;
        tx.commit().await?; Ok(())
    }
    pub async fn pin(&self, reader: &PgPool, g: GenerationId) -> Result<GenerationLease, Error> {
        // Keep the pool permit; closing on drop releases both the advisory lock and pool capacity.
        let mut connection = reader.acquire().await?;
        connection.close_on_drop();
        sqlx::query("SELECT pg_advisory_lock_shared($1)").bind(g.lock()).execute(&mut *connection).await?;
        state(&mut connection, g, "published", &self.model, self.physical).await?;
        Ok(GenerationLease { connection, generation: g, model: self.model.clone() })
    }
    pub async fn copy<R: Record>(&self, writer: &PgPool, g: GenerationId, batch: &Batch<R>) -> Result<(), Error> {
        self.model.require::<R>()?;
        let mut tx = writer.begin().await?;
        lock(&mut tx, g, true).await?;
        state(&mut tx, g, "staging", &self.model, self.physical).await?;
        let columns = batch.arrow().schema().fields().iter().map(|f| quoted(f.name())).collect::<Vec<_>>().join(",");
        let mut encoder = pgpq::ArrowToPostgresBinaryEncoder::try_new(&batch.arrow().schema()).map_err(|e| Error::Codec(e.to_string()))?;
        let mut bytes = BytesMut::new();
        encoder.write_header(&mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
        encoder.write_batch(batch.arrow(), &mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
        encoder.write_footer(&mut bytes).map_err(|e| Error::Codec(e.to_string()))?;
        let mut copy = tx.copy_in_raw(&format!("COPY {} ({columns}) FROM STDIN BINARY", qualified(g, R::NAME))).await?;
        copy.send(bytes.freeze()).await?; copy.finish().await?;
        tx.commit().await?; Ok(())
    }
}

pub struct GenerationLease { connection: sqlx::pool::PoolConnection<sqlx::Postgres>, generation: GenerationId, model: Arc<ValidatedModel> }
impl GenerationLease {
    pub fn generation(&self) -> GenerationId { self.generation }
    pub async fn read<R: Record>(&mut self) -> Result<Batch<R>, Error> {
        // Every read uses the leased connection. A lost connection cannot silently reacquire a lease.
        self.connection.ping().await?;
        let relation = self.model.require::<R>()?;
        let batch = read_physical(&mut self.connection, self.generation, relation).await?;
        Ok(Batch::read(&self.model, &batch)?)
    }
}
async fn read_physical(connection: &mut PgConnection, g: GenerationId, relation: &Relation) -> Result<RecordBatch, Error> {
    let columns = relation.schema().fields().iter().map(|f| quoted(f.name())).collect::<Vec<_>>().join(",");
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {columns} FROM {} ORDER BY id", qualified(g, relation.name())))).fetch_all(connection).await?;
    codec::decode(relation, &rows)
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
