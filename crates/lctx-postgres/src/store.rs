//! The canonical store kernel (DESIGN §15.11, ADR-0083; cutover plan §3.2, §4.1 D3–D5).
//!
//! The owner installs the generated canonical schema ([`MigrationStore::install`]), resets it on
//! a contract change ([`MigrationStore::reset`]) and retires generations
//! ([`MigrationStore::retire`]). The writer runs one generation's lifecycle through the store's
//! SECURITY DEFINER functions ([`Writer`]): create, COPY each relation in binary, validate each
//! relation, mark validated, publish or fail, select. Nothing here decides semantics: relation
//! contracts, DDL and templates come from `lctx-model`, and the attempt's DataFusion validators run
//! before [`Writer::mark_validated`].

use arrow_array::RecordBatch;
use bytes::BytesMut;
use lctx_model::ddl::{DdlConfig, Install};
use lctx_model::id::{Digest, Id};
use sqlx::PgPool;

use crate::serving::{Role, RoleConfig};
use crate::{Error, MigrationStore};

/// Where the generated canonical schema lives and who reads it (cutover plan §4.1 D5).
pub const CANONICAL: DdlConfig = DdlConfig {
    schema: "lctx",
    reader: "lctx_serving",
};

/// Rows per COPY statement: each statement stays well within the writer's statement timeout.
pub const COPY_ROWS: usize = 50_000;

/// Whether an install changed anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    AlreadyCurrent,
}

/// The installed canonical schema's digest, if any.
pub async fn installed_digest(pool: &PgPool) -> Result<Option<Digest>, Error> {
    let digest: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT ddl_digest FROM lctx_store.ddl_installs WHERE is_current")
            .fetch_optional(pool)
            .await?;
    digest
        .map(|d| {
            <[u8; 32]>::try_from(d.as_slice())
                .map(Digest)
                .map_err(|_| Error::Integrity("ddl digest width"))
        })
        .transpose()
}

impl MigrationStore {
    /// Install the generated canonical schema. The same digest is a no-op; a different installed
    /// digest is refused: a contract change is an explicit [`Self::reset`].
    pub async fn install(&self, install: &Install) -> Result<InstallOutcome, Error> {
        let pool = &self.inner.pool;
        match installed_digest(pool).await? {
            Some(current) if current == install.digest => return Ok(InstallOutcome::AlreadyCurrent),
            Some(_) => return Err(Error::CanonicalSchema),
            None => {}
        }
        let mut tx = pool.begin().await?;
        for statement in &install.statements {
            sqlx::raw_sql(sqlx::AssertSqlSafe(statement.clone()))
                .execute(&mut *tx)
                .await?;
        }
        for (ordinal, spec) in install.tables.iter().enumerate() {
            sqlx::query(
                "INSERT INTO lctx_store.relations (name, ordinal, partition_column, copy_columns, \
                 create_staging, create_indexes, attach, detach) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(&spec.name)
            .bind(i32::try_from(ordinal).map_err(|_| Error::Integrity("relation count"))?)
            .bind(&spec.partition.column)
            .bind(spec.copy_columns())
            .bind(spec.staging_table())
            .bind(spec.staging_indexes())
            .bind(spec.attach(&CANONICAL))
            .bind(spec.detach(&CANONICAL))
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("INSERT INTO lctx_store.ddl_installs (ddl_digest, is_current) VALUES ($1, true)")
            .bind(install.digest.0.as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(InstallOutcome::Installed)
    }

    /// Drop every generation and the whole canonical schema, then install `install`
    /// (current-only, ADR-0048/0078). Served projections are separate copies and are untouched.
    pub async fn reset(&self, install: &Install) -> Result<(), Error> {
        let pool = &self.inner.pool;
        let generations: Vec<Vec<u8>> =
            sqlx::query_scalar("SELECT generation_id FROM lctx_store.generations")
                .fetch_all(pool)
                .await?;
        let mut tx = pool.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA IF EXISTS {} CASCADE",
            lctx_model::ddl::quote(CANONICAL.schema)
        )))
        .execute(&mut *tx)
        .await?;
        for g in generations {
            let schema = generation_schema_of(&g)?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE")))
                .execute(&mut *tx)
                .await?;
        }
        sqlx::raw_sql(
            "DELETE FROM lctx_store.selections; DELETE FROM lctx_store.generations; \
             DELETE FROM lctx_store.relations; DELETE FROM lctx_store.ddl_installs;",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.install(install).await.map(|_| ())
    }

    /// Retire a generation that is not selected: detach its partitions without blocking readers
    /// of the parents (referrers first), then drop its schema. A failed or staging generation has
    /// nothing attached and is dropped directly.
    pub async fn retire(&self, generation: Id) -> Result<(), Error> {
        let pool = &self.inner.pool;
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM lctx_store.generations WHERE generation_id = $1")
                .bind(generation.0.as_slice())
                .fetch_optional(pool)
                .await?;
        let state = state.ok_or(Error::Request("no such generation".into()))?;
        let selected: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM lctx_store.selections WHERE generation_id = $1)",
        )
        .bind(generation.0.as_slice())
        .fetch_one(pool)
        .await?;
        if selected {
            return Err(Error::Request("a selected generation cannot be retired".into()));
        }
        if state == "retired" {
            return Ok(());
        }
        let schema = generation_schema_of(&generation.0)?;
        if state == "published" {
            let detach: Vec<(String, String)> = sqlx::query_as(
                "SELECT name, lctx_store.expand(detach, $1) FROM lctx_store.relations ORDER BY ordinal DESC",
            )
            .bind(generation.0.as_slice())
            .fetch_all(pool)
            .await?;
            for (name, statement) in detach {
                // DETACH … CONCURRENTLY cannot run inside a transaction block. A detached partition
                // keeps its reference constraints, so it is dropped before the partition it refers
                // to is detached (referrers first).
                sqlx::raw_sql(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                    "DROP TABLE \"{schema}\".{}",
                    lctx_model::ddl::quote(&name)
                )))
                .execute(pool)
                .await?;
            }
        }
        let mut tx = pool.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE")))
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE lctx_store.generations SET state = 'retired', retired_at = now() WHERE generation_id = $1",
        )
        .bind(generation.0.as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO lctx_store.generation_events (generation_id, event) VALUES ($1, 'retired')")
            .bind(generation.0.as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn generation_schema_of(id: &[u8]) -> Result<String, Error> {
    let id = <[u8; 16]>::try_from(id).map_err(|_| Error::Integrity("generation id width"))?;
    Ok(lctx_model::ddl::generation_schema(Id(id)))
}

/// One installed relation, as the writer loads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledRelation {
    pub name: String,
    pub copy_columns: Vec<String>,
}

/// One relation's receipt for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationReceipt {
    pub relation: String,
    pub row_count: u64,
    pub schema_digest: Digest,
}

/// The digests a validated generation records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationDigests {
    pub compiler: Digest,
    pub producer: Digest,
    pub content: Digest,
}

/// A registry row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationRow {
    pub generation_id: Id,
    pub library: String,
    pub profile: String,
    pub state: String,
    pub ddl_digest: Digest,
    pub content_digest: Option<Digest>,
}

/// The writer's lifecycle handle (`lctx_importer`).
#[derive(Clone)]
pub struct Writer {
    pool: PgPool,
}

impl Writer {
    pub async fn open(config: &RoleConfig) -> Result<Self, Error> {
        if config.role != Role::Importer {
            return Err(Error::Config("writer (importer) credentials required"));
        }
        Ok(Self {
            pool: config.pool().await?,
        })
    }

    /// A writer over an existing writer-role pool.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn installed_digest(&self) -> Result<Option<Digest>, Error> {
        installed_digest(&self.pool).await
    }

    /// The installed relations in dependency order, with their COPY columns.
    pub async fn relations(&self) -> Result<Vec<InstalledRelation>, Error> {
        let rows: Vec<(String, Vec<String>)> = sqlx::query_as(
            "SELECT name, copy_columns FROM lctx_store.relations ORDER BY ordinal",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, copy_columns)| InstalledRelation { name, copy_columns })
            .collect())
    }

    pub async fn create(
        &self,
        generation: Id,
        library: &str,
        profile: &str,
        ddl_digest: Digest,
    ) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.create_generation($1, $2, $3, $4)")
            .bind(generation.0.as_slice())
            .bind(library)
            .bind(profile)
            .bind(ddl_digest.0.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Binary-COPY one relation's rows into the generation's staging table, `COPY_ROWS` per
    /// statement. The batch carries exactly the relation's COPY columns, by name.
    pub async fn copy(
        &self,
        generation: Id,
        relation: &InstalledRelation,
        batch: &RecordBatch,
    ) -> Result<u64, Error> {
        let projection = relation
            .copy_columns
            .iter()
            .map(|c| batch.schema().index_of(c).map_err(|_| Error::Integrity("COPY column missing")))
            .collect::<Result<Vec<_>, _>>()?;
        if projection.len() != batch.num_columns() {
            return Err(Error::Integrity("batch carries columns the relation does not"));
        }
        let batch = batch
            .project(&projection)
            .map_err(|_| Error::Integrity("COPY projection"))?;
        let batch = strip_metadata(&batch)?;
        let schema = lctx_model::ddl::generation_schema(generation);
        let columns = relation
            .copy_columns
            .iter()
            .map(|c| lctx_model::ddl::quote(c))
            .collect::<Vec<_>>()
            .join(", ");
        let statement = format!(
            "COPY \"{schema}\".{} ({columns}) FROM STDIN WITH (FORMAT BINARY)",
            lctx_model::ddl::quote(&relation.name)
        );
        let mut copied = 0u64;
        let mut offset = 0;
        let rows = batch.num_rows();
        while offset < rows || (rows == 0 && offset == 0) {
            let length = COPY_ROWS.min(rows - offset);
            let slice = batch.slice(offset, length);
            let encoded = encode(&slice)?;
            let mut conn = self.pool.acquire().await?;
            let mut copy = conn.copy_in_raw(&statement).await?;
            copy.send(encoded.as_ref()).await?;
            copied += copy.finish().await?;
            offset += length;
            if rows == 0 {
                break;
            }
        }
        if copied != rows as u64 {
            return Err(Error::Integrity("COPY row count"));
        }
        Ok(copied)
    }

    pub async fn validate_relation(&self, generation: Id, receipt: &RelationReceipt) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.validate_relation($1, $2, $3, $4)")
            .bind(generation.0.as_slice())
            .bind(&receipt.relation)
            .bind(i64::try_from(receipt.row_count).map_err(|_| Error::Integrity("row count"))?)
            .bind(receipt.schema_digest.0.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_validated(
        &self,
        generation: Id,
        digests: GenerationDigests,
        receipts: &serde_json::Value,
    ) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.mark_validated($1, $2, $3, $4, $5)")
            .bind(generation.0.as_slice())
            .bind(digests.compiler.0.as_slice())
            .bind(digests.producer.0.as_slice())
            .bind(digests.content.0.as_slice())
            .bind(receipts)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn publish(&self, generation: Id) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.publish_generation($1)")
            .bind(generation.0.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn fail(&self, generation: Id, reason: &str) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.fail_generation($1, $2)")
            .bind(generation.0.as_slice())
            .bind(reason)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn select(&self, generation: Id) -> Result<(), Error> {
        sqlx::query("SELECT lctx_store.select_generation($1)")
            .bind(generation.0.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn generation(&self, generation: Id) -> Result<Option<GenerationRow>, Error> {
        generation_row(&self.pool, generation).await
    }
}

/// One registry row, by id.
pub async fn generation_row(pool: &PgPool, generation: Id) -> Result<Option<GenerationRow>, Error> {
    let row: Option<(Vec<u8>, String, String, String, Vec<u8>, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT generation_id, library, profile, state, ddl_digest, content_digest \
         FROM lctx_store.generations WHERE generation_id = $1",
    )
    .bind(generation.0.as_slice())
    .fetch_optional(pool)
    .await?;
    row.map(|(id, library, profile, state, ddl, content)| {
        Ok(GenerationRow {
            generation_id: Id(id.try_into().map_err(|_| Error::Integrity("generation id width"))?),
            library,
            profile,
            state,
            ddl_digest: Digest(ddl.try_into().map_err(|_| Error::Integrity("ddl digest width"))?),
            content_digest: content
                .map(|c| c.try_into().map(Digest))
                .transpose()
                .map_err(|_| Error::Integrity("content digest width"))?,
        })
    })
    .transpose()
}

/// Every registry row, newest first.
pub async fn generation_rows(pool: &PgPool) -> Result<Vec<GenerationRow>, Error> {
    let ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT generation_id FROM lctx_store.generations ORDER BY created_at DESC, generation_id",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let id = Id(id.try_into().map_err(|_| Error::Integrity("generation id width"))?);
        if let Some(row) = generation_row(pool, id).await? {
            out.push(row);
        }
    }
    Ok(out)
}

/// A batch with schema and field metadata removed: the COPY encoder reads types only.
fn strip_metadata(batch: &RecordBatch) -> Result<RecordBatch, Error> {
    let fields: Vec<arrow_schema::Field> = batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.as_ref().clone().with_metadata(Default::default()))
        .collect();
    RecordBatch::try_new(
        std::sync::Arc::new(arrow_schema::Schema::new(fields)),
        batch.columns().to_vec(),
    )
    .map_err(|_| Error::Integrity("COPY batch schema"))
}

/// pgpq's binary COPY encoding of one batch.
pub fn encode(batch: &RecordBatch) -> Result<BytesMut, Error> {
    let mut encoder = pgpq::ArrowToPostgresBinaryEncoder::try_new(&batch.schema())
        .map_err(|_| Error::Integrity("COPY encoder rejects the batch schema"))?;
    let mut out = BytesMut::new();
    encoder.write_header(&mut out).map_err(|_| Error::Integrity("COPY header"))?;
    encoder.write_batch(batch, &mut out).map_err(|_| Error::Integrity("COPY batch"))?;
    encoder.write_footer(&mut out).map_err(|_| Error::Integrity("COPY footer"))?;
    Ok(out)
}

/// The PostgreSQL types pgpq encodes a schema's columns as, by name: the store asserts the
/// generated DDL declares exactly these.
pub fn copy_types(schema: &arrow_schema::Schema) -> Result<Vec<(String, String)>, Error> {
    let encoder = pgpq::ArrowToPostgresBinaryEncoder::try_new(schema)
        .map_err(|_| Error::Integrity("COPY encoder rejects the schema"))?;
    Ok(encoder
        .schema()
        .columns
        .iter()
        .map(|column| {
            (
                column.name.clone(),
                column.data_type.name().unwrap_or_else(|| "unnamed".to_owned()),
            )
        })
        .collect())
}
