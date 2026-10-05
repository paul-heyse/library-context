//! Immutable generation schemas lowered exclusively from a validated semantic model (ADR-0086).
mod access_routes_service;
mod capability_service;
mod catalog;
mod codec;
mod ddl;
mod evidence_service;
mod failure;
mod flow_inventory_service;
mod guard;
mod install;
mod lease;
mod lifecycle;
pub(crate) mod locks;
mod native_service;
mod physical_columns;
mod publication_validation;
mod reader;
mod runtime;
mod serving_shape;
mod source_characterization_service;
mod source_usage_service;
mod vectors;
pub use native_service::PreparedNative;
mod service;
pub use service::ServingService;
mod retrieval_service;
pub use retrieval_service::{NumericalCorpus, NumericalQuery, RankingRequest, RetrievalService};
mod capture_packets;
mod catalog_service;
mod operation_sections;
mod packet_reads;
mod packet_service;
pub use catalog_service::CatalogService;
pub use guard::GenerationGuard;
pub use runtime::{GenerationService, PreparedReservation, RequestExecution};
pub use vectors::{ScoredVectors, VectorArtifact};
mod receipts;
mod selection;
pub use reader::GenerationReader;
pub use selection::{AdmittedSelection, SELECTION_PREPARATION_BYTES};
mod stage_validation;
mod validation_views;
mod audit;
pub use audit::AuditReport;
mod validation_session;
pub use validation_session::ValidationStats;
mod verify;
mod vocabulary;
use arrow_array::RecordBatch;
use bytes::BytesMut;
pub use catalog::{
    GenerationCatalog, GenerationDetail, GenerationState, GenerationSummary, ListFilter, Writer,
};
pub use failure::{Failure, FailureClass};
use futures::TryStreamExt;
pub use install::ResetInventory;
use lctx_model::domain::resources::{MAX_ROW_BYTES, ResourceBudget, TRANSFER_BYTES, TRANSFER_ROWS};
use lctx_model::domain::{
    Batch, ContentHash, Id, Infrastructure, ModelError, Record, Relation, ValidatedModel,
    admission::Frontier,
};
pub use lease::{AttemptReadContract, Held, LeaseContract, LeaseDriver, LeaseParam};
pub use lifecycle::{CompletedCheckpoint, GenerationAttempt, SealedAttempt, ValidatedAttempt};
use pgpq::encoders::{BuildEncoder, Encode, EncoderBuilder};
use sqlx::{Connection, PgConnection, PgPool, Row, ValueRef};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub use verify::{CheckReport, Finding, FindingKind};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LibraryAdmission(#[from] lctx_model::domain::serving::LibraryAdmissionError),
    #[error("{0}")]
    Model(#[from] ModelError),
    #[error("PostgreSQL operation failed")]
    Database(#[from] sqlx::Error),
    #[error("PostgreSQL commit failed; outcome is unconfirmed")]
    Commit(#[source] sqlx::Error),
    #[error("PostgreSQL rollback failed after {operation}; cleanup is unconfirmed")]
    Rollback {
        operation: Box<Error>,
        #[source]
        source: sqlx::Error,
    },
    #[error("PostgreSQL COPY abort failed after {operation}; cleanup is unconfirmed")]
    CopyAbort {
        operation: Box<Error>,
        #[source]
        source: sqlx::Error,
    },
    #[error("serving resource refused: {0}")]
    ResourceRefused(&'static str),
    #[error("generation state does not permit this operation")]
    State,
    /// No generation with this id is registered: never created, aborted or retired.
    #[error("no such generation")]
    Absent,
    /// The generation store is not installed in this database.
    #[error("the generation store is not installed; run `lctx store install`")]
    NotInstalled,
    #[error("generation model or physical schema differs from this binary")]
    Contract,
    #[error("generation has active readers or is selected")]
    Busy,
    #[error("codec: {0}")]
    Codec(String),
    #[error("secure generation randomness unavailable")]
    Random,
    #[error("generation has orphaned registry or schema objects; explicit repair is required")]
    Orphaned,
    /// A request outside the generation's frontier: a relation it does not hold, or selecting a
    /// generation that is not a facts generation (P0 exit F02).
    #[error("frontier: {0}")]
    Frontier(String),
    #[error("reset requires the database name as confirmation")]
    Confirmation,
    /// A failure of a driver other than SQLx (a provider session), with its class.
    #[error("{class:?} driver failure: {detail}")]
    Driver {
        class: Infrastructure,
        detail: String,
    },
}
impl Error {
    /// The infrastructure class a stage sink reports for a store failure (P0 exit F07).
    pub fn class(&self) -> Infrastructure {
        match self {
            Self::Database(error) => {
                let code = error
                    .as_database_error()
                    .and_then(|e| e.code())
                    .unwrap_or_default();
                match error {
                    sqlx::Error::Io(_)
                    | sqlx::Error::Tls(_)
                    | sqlx::Error::Protocol(_)
                    | sqlx::Error::PoolClosed
                    | sqlx::Error::WorkerCrashed => Infrastructure::Transport,
                    sqlx::Error::PoolTimedOut => Infrastructure::Contention,
                    _ => FailureClass::sqlstate(&code).infrastructure(),
                }
            }
            Self::Commit(_) | Self::Rollback { .. } | Self::CopyAbort { .. } => {
                Infrastructure::Unconfirmed
            }
            Self::ResourceRefused(_) => Infrastructure::Contention,
            Self::LibraryAdmission(_) => Infrastructure::Refused,
            Self::State
            | Self::Absent
            | Self::NotInstalled
            | Self::Busy
            | Self::Orphaned
            | Self::Confirmation => Infrastructure::State,
            Self::Contract | Self::Frontier(_) | Self::Model(_) | Self::Codec(_) => {
                Infrastructure::Contract
            }
            Self::Random => Infrastructure::Io,
            Self::Driver { class, .. } => *class,
        }
    }
}
/// A store failure keeps its class and safe detail (SQLSTATE, constraint, table) across the stage
/// sink boundary: a content violation the server found is invalid content, like one found here.
impl From<Error> for ModelError {
    fn from(error: Error) -> Self {
        match error {
            Error::Model(error) => error,
            Error::Frontier(message) => ModelError::Frontier(message),
            Error::Codec(message) => ModelError::Codec(message),
            other => {
                let failure = Failure::of(&other);
                if failure.class == FailureClass::Invalid {
                    ModelError::Invalid(failure.detail)
                } else {
                    ModelError::Infrastructure {
                        class: other.class(),
                        detail: failure.detail,
                    }
                }
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupOutcome {
    Removed,
    AlreadyAbsent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationId([u8; 16]);
impl GenerationId {
    fn new() -> Result<Self, Error> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|_| Error::Random)?;
        Ok(Self(bytes))
    }
    pub fn bytes(&self) -> &[u8; 16] {
        &self.0
    }
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub fn hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
    pub fn schema(self) -> String {
        format!("lctx_g{}", self.hex())
    }
    /// The generation a `lctx_g<32 hex>` schema name belongs to.
    pub fn from_schema(name: &str) -> Option<Self> {
        Self::from_hex(name.strip_prefix("lctx_g")?)
    }
    /// A generation id written as exactly 32 lowercase hex digits.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 32 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        let mut bytes = [0; 16];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }
}
#[derive(Clone)]
pub struct GenerationStore {
    owner: PgPool,
    model: Arc<ValidatedModel>,
    physical: ContentHash,
    scopes: Arc<BTreeMap<Frontier, ddl::Scope>>,
}
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
    pub async fn install(
        owner: crate::OwnerPool,
        model: Arc<ValidatedModel>,
    ) -> Result<Self, Error> {
        let store = Self::assemble(owner.pool().clone(), model);
        transaction(&store.owner, async |tx| {
            locks::installation_exclusive(tx).await?;
            install::control(tx, &store.model, store.physical).await
        })
        .await?;
        Ok(store)
    }
    /// Confirm an existing installation of this model and lowering, creating nothing.
    pub async fn open(owner: crate::OwnerPool, model: Arc<ValidatedModel>) -> Result<Self, Error> {
        let store = Self::assemble(owner.pool().clone(), model);
        let installed: bool =
            sqlx::query_scalar("SELECT to_regclass('lctx_model_store.installation') IS NOT NULL")
                .fetch_one(&store.owner)
                .await?;
        if !installed {
            return Err(Error::NotInstalled);
        }
        let mut connection = store.owner.acquire().await?;
        transaction_on(&mut connection, async |tx| {
            store.lock_installation(tx).await
        })
        .await?;
        Ok(store)
    }
    /// The installation's physical digest is the whole model's (conformance) lowering.
    fn assemble(owner: PgPool, model: Arc<ValidatedModel>) -> Self {
        let scopes = ddl::scopes(&model);
        Self {
            owner,
            physical: scopes[&Frontier::Conformance].physical,
            model,
            scopes: Arc::new(scopes),
        }
    }
    fn scope(&self, frontier: Frontier) -> Result<&ddl::Scope, Error> {
        self.scopes
            .get(&frontier)
            .ok_or_else(|| Error::Frontier(format!("the model has no {} scope", frontier.name())))
    }
    /// Compare the live store with a shadow install of this binary's lowering (`store check`).
    pub async fn check(
        owner: &crate::OwnerPool,
        model: &ValidatedModel,
    ) -> Result<CheckReport, Error> {
        verify::check(owner, model).await
    }
    /// What `reset` would drop, without changing anything.
    pub async fn reset_plan(owner: &crate::OwnerPool) -> Result<ResetInventory, Error> {
        install::plan(owner).await
    }
    /// Drop every inventoried generation schema and the control schema, then install `model`.
    /// `confirm` must name the database. Refused with `Busy` while any generation is leased, in
    /// a lifecycle transaction or owned by a live attempt. Resumable: it removes one generation
    /// per transaction, and a rerun finishes an interrupted reset.
    pub async fn reset(
        owner: crate::OwnerPool,
        model: Arc<ValidatedModel>,
        confirm: &str,
    ) -> Result<(ResetInventory, Self), Error> {
        install::reset(owner, model, confirm).await
    }
    pub fn model(&self) -> &ValidatedModel {
        &self.model
    }
    pub fn physical_digest(&self) -> ContentHash {
        self.physical
    }
    /// Register a generation and lower its staging phase in the caller's transaction. Every
    /// generation belongs to the attempt that registers it (store-lifecycle review F01).
    async fn create(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        frontier: Frontier,
        producer: ContentHash,
        profile: &str,
        schedule: ContentHash,
    ) -> Result<(), Error> {
        if !matches!(profile, "catalog" | "behavioral") {
            return Err(Error::State);
        }
        self.lock_installation(tx).await?;
        let physical = self.scope(frontier)?.physical;
        sqlx::query("INSERT INTO lctx_model_store.generations(id,state,model_digest,physical_digest,producer_digest,profile,frontier,schedule_digest) \
            VALUES($1,'staging',$2,$3,$4,$5,$6,$7)")
            .bind(g.0.to_vec()).bind(self.model.digest().0.to_vec()).bind(physical.0.to_vec()).bind(producer.0.to_vec()).bind(profile)
            .bind(frontier.name()).bind(schedule.0.to_vec()).execute(&mut *tx).await?;
        execute(tx, self.lowering(g, frontier)?.phase("staging")).await
    }
    pub async fn select(&self, g: GenerationId) -> Result<(), Error> {
        transaction(&self.owner, async |tx| {
            self.lock_installation(tx).await?;
            sqlx::query("SELECT singleton FROM lctx_model_store.selection FOR UPDATE")
                .execute(&mut *tx)
                .await?;
            lock(tx, g, true).await?;
            let registered = self.registered(tx, g).await?;
            registered.expect("published")?;
            if !registered.frontier.descriptor().selectable() {
                return Err(Error::Frontier(
                    "the generation's frontier is not selectable".into(),
                ));
            }
            sqlx::query("UPDATE lctx_model_store.selection SET generation_id=$1 WHERE singleton")
                .bind(g.0.to_vec())
                .execute(&mut *tx)
                .await?;
            Ok(())
        })
        .await
    }
    pub async fn clear_selection(&self) -> Result<(), Error> {
        transaction(&self.owner, async |tx| {
            self.lock_installation(tx).await?;
            sqlx::query("UPDATE lctx_model_store.selection SET generation_id=NULL WHERE singleton")
                .execute(&mut *tx)
                .await?;
            Ok(())
        })
        .await
    }
    pub async fn retire(&self, g: GenerationId) -> Result<CleanupOutcome, Error> {
        self.cleanup(g, true, false).await
    }
    /// Abandon an unpublished attempt. No registry tombstone or generation event survives.
    pub async fn abort(&self, g: GenerationId) -> Result<CleanupOutcome, Error> {
        self.cleanup(g, false, false).await
    }
    /// Explicitly repair orphan objects only; ordinary cleanup refuses them.
    pub async fn repair_orphan(&self, g: GenerationId) -> Result<CleanupOutcome, Error> {
        self.cleanup(g, false, true).await
    }
    async fn cleanup(
        &self,
        g: GenerationId,
        published: bool,
        repair: bool,
    ) -> Result<CleanupOutcome, Error> {
        transaction(&self.owner, async |tx| {
            self.cleanup_on(tx, g, published, repair).await
        })
        .await
    }
    /// Remove a generation's schema and every control record in the caller's transaction. An
    /// attempt aborts through its own lifecycle connection, which already holds its attempt lock.
    async fn cleanup_on(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        published: bool,
        repair: bool,
    ) -> Result<CleanupOutcome, Error> {
        self.lock_installation(tx).await?;
        let selected: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT generation_id FROM lctx_model_store.selection WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        if selected.as_deref() == Some(&g.0) {
            return Err(Error::Busy);
        }
        // A live attempt holds its attempt lock; a lease or transition holds the generation lock.
        if !locks::try_generation_exclusive(tx, g).await? {
            return Err(Error::Busy);
        }
        // The optional cache follows existing generation cleanup. It cannot hold a lease or
        // constrain generation deletion through a foreign key into the canonical store.
        let artifact_cache: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('lctx_cache.serving_vector_artifacts')::text")
                .fetch_one(&mut *tx)
                .await?;
        if artifact_cache.is_some() {
            sqlx::query("DELETE FROM lctx_cache.serving_vector_artifacts WHERE generation_id=$1")
                .bind(g.0.to_vec())
                .execute(&mut *tx)
                .await?;
        }
        let current: Option<String> =
            sqlx::query_scalar("SELECT state FROM lctx_model_store.generations WHERE id=$1")
                .bind(g.0.to_vec())
                .fetch_optional(&mut *tx)
                .await?;
        let schema: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)")
                .bind(g.schema())
                .fetch_one(&mut *tx)
                .await?;
        let residue: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT {}",
            CONTROL_RECORDS
                .iter()
                .map(|table| format!(
                    "EXISTS(SELECT 1 FROM lctx_model_store.{table} WHERE generation_id=$1)"
                ))
                .collect::<Vec<_>>()
                .join(" OR ")
        )))
        .bind(g.0.to_vec())
        .fetch_one(&mut *tx)
        .await?;
        if current.is_none() && !schema && !residue {
            return Ok(CleanupOutcome::AlreadyAbsent);
        }
        let orphaned = current.is_none() || !schema;
        if orphaned && !repair {
            return Err(Error::Orphaned);
        }
        if repair && !orphaned {
            return Err(Error::State);
        }
        if let Some(current) = &current {
            if !repair
                && ((published && current != "published")
                    || (!published
                        && !matches!(
                            current.as_str(),
                            "staging" | "sealed" | "validated" | ddl::FAILED
                        )))
            {
                return Err(Error::State);
            }
            self.registered(tx, g).await?;
        }
        // DROP drains direct writer transactions as well as declared stage writers.
        if schema {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP SCHEMA {} CASCADE",
                quoted(&g.schema())
            )))
            .execute(&mut *tx)
            .await?;
        }
        for table in CONTROL_RECORDS {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM lctx_model_store.{table} WHERE generation_id=$1"
            )))
            .bind(g.0.to_vec())
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=$1")
            .bind(g.0.to_vec())
            .execute(&mut *tx)
            .await?;
        Ok(CleanupOutcome::Removed)
    }
    fn lowering(&self, g: GenerationId, frontier: Frontier) -> Result<ddl::Lowering, Error> {
        Ok(ddl::lower(
            &self.model,
            &self.scope(frontier)?.relations,
            g,
            &g.schema(),
            ddl::CONTROL,
        ))
    }
    /// The relations and the invariants (all of whose inputs are held) of one frontier's scope.
    fn scoped(
        &self,
        frontier: Frontier,
    ) -> Result<(Vec<&Relation>, Vec<lctx_model::domain::Invariant>), Error> {
        let scope = &self.scope(frontier)?.relations;
        Ok((
            self.model
                .relations()
                .iter()
                .filter(|r| scope.contains(r.name()))
                .collect(),
            frontier.descriptor().invariants(&self.model)?,
        ))
    }
    /// A generation's registry row, after confirming its model and frontier-scoped lowering.
    async fn registered(
        &self,
        connection: &mut PgConnection,
        g: GenerationId,
    ) -> Result<Registered, Error> {
        let row = sqlx::query("SELECT state,model_digest,physical_digest,frontier FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec())
            .fetch_optional(connection).await?.ok_or(Error::Absent)?;
        let frontier =
            ddl::frontier(&row.try_get::<String, _>("frontier")?).ok_or(Error::Contract)?;
        if row.try_get::<Vec<u8>, _>("model_digest")? != self.model.digest().0
            || row.try_get::<Vec<u8>, _>("physical_digest")? != self.scope(frontier)?.physical.0
        {
            return Err(Error::Contract);
        }
        Ok(Registered {
            state: row.try_get("state")?,
            frontier,
        })
    }
    /// Generations whose attempt is gone: unpublished and not failed, with no lifecycle
    /// connection holding their attempt lock. They can only be aborted. Read from `pg_locks`
    /// without taking any lock.
    pub async fn interrupted(&self) -> Result<Vec<GenerationId>, Error> {
        transaction(&self.owner, async |tx| {
            self.lock_installation(tx).await?;
            let ids: Vec<Vec<u8>> = sqlx::query_scalar("SELECT id FROM lctx_model_store.generations WHERE state IN ('staging','sealed','validated') ORDER BY id")
                .fetch_all(&mut *tx).await?;
            let mut interrupted = Vec::new();
            for id in ids {
                let g = GenerationId(id.try_into().map_err(|_| Error::Codec("generation id length".into()))?);
                if !locks::attempt_live(&mut *tx, g).await? { interrupted.push(g); }
            }
            Ok(interrupted)
        }).await
    }
    async fn lock_installation(&self, connection: &mut PgConnection) -> Result<(), Error> {
        sqlx::query(locks::INSTALLATION_SHARED)
            .execute(&mut *connection)
            .await?;
        let compatible: Option<bool> = sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton")
            .bind(self.model.digest().0.to_vec()).bind(self.physical.0.to_vec()).fetch_optional(connection).await?;
        if compatible != Some(true) {
            return Err(Error::Contract);
        }
        Ok(())
    }
    /// Readers decode into the supplied budget for the lease's lifetime.
    pub async fn pin(
        &self,
        reader: &PgPool,
        g: GenerationId,
        budget: ResourceBudget,
    ) -> Result<GenerationLease, Error> {
        GenerationLease::acquire(reader, self.model.clone(), self.lease_contract(g), budget).await
    }
    /// The lease protocol for one generation of this store's model.
    pub fn lease_contract(&self, g: GenerationId) -> LeaseContract {
        LeaseContract::from_scopes(self.model.digest(), self.physical, self.scopes.clone(), g)
    }
    /// An attempt's COPY, charged to its budget.
    async fn copy_into<R: Record>(
        &self,
        writer: &PgPool,
        g: GenerationId,
        batch: &Batch<R>,
        schedule: ContentHash,
        budget: &ResourceBudget,
        physical: &str,
    ) -> Result<(), Error> {
        self.model.require::<R>()?;
        transaction(writer, async |tx| {
            self.lock_installation(tx).await?;
            lock(tx, g, true).await?;
            let registered = self.registered(tx, g).await?;
            registered.expect("staging")?;
            if !self.scope(registered.frontier)?.relations.contains(R::NAME) {
                return Err(Error::Frontier(format!(
                    "{} is outside this generation's frontier",
                    R::NAME
                )));
            }
            check_schedule(tx, g, schedule).await?;
            // Schema/encoder bookkeeping stays reserved for the COPY lifetime. The variable wire
            // buffer is admitted separately before pgpq allocates it, using pgpq's own size bound.
            let _metadata = budget.reserve(
                "postgres-copy-metadata",
                batch
                    .arrow()
                    .schema()
                    .fields()
                    .len()
                    .checked_mul(4096)
                    .ok_or_else(|| Error::Codec("COPY schema accounting overflow".into()))?,
            )?;
            let columns = batch
                .arrow()
                .schema()
                .fields()
                .iter()
                .map(|f| quoted(f.name()))
                .collect::<Vec<_>>()
                .join(",");
            let mut encoder = pgpq::ArrowToPostgresBinaryEncoder::try_new(&batch.arrow().schema())
                .map_err(|e| Error::Codec(e.to_string()))?;
            let builders = batch
                .arrow()
                .schema()
                .fields()
                .iter()
                .map(|f| EncoderBuilder::try_new(f.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::Codec(e.to_string()))?;
            let header_charge = budget.reserve("postgres-copy-wire", 64)?;
            let mut bytes = BytesMut::with_capacity(32);
            encoder
                .write_header(&mut bytes)
                .map_err(|e| Error::Codec(e.to_string()))?;
            let mut copy = tx
                .copy_in_raw(&format!(
                    "COPY {} ({columns}) FROM STDIN BINARY",
                    qualified(g, physical)
                ))
                .await?;
            let result: Result<(), Error> = async {
                copy.send(bytes.freeze()).await?;
                drop(header_charge);
                for index in 0..batch.arrow().num_rows() {
                    let row = batch.arrow().slice(index, 1);
                    let size = copy_size(&row, &builders)?;
                    // Final canonical rows add the generation column (20 wire bytes), and
                    // vocabulary rows also add the lifecycle-owned epoch (6 wire bytes).
                    let metadata_bytes = 20
                        + if lctx_model::domain::stages::is_vocabulary(R::NAME) {
                            6
                        } else {
                            0
                        };
                    if size > MAX_ROW_BYTES - metadata_bytes {
                        return Err(Error::Model(ModelError::Limit {
                            owner: R::NAME,
                            limit: "COPY row bytes",
                            observed: size,
                            bound: MAX_ROW_BYTES - metadata_bytes,
                        }));
                    }
                    let _wire = budget.reserve(
                        "postgres-copy-wire",
                        size.checked_mul(2)
                            .ok_or_else(|| Error::Codec("COPY wire accounting overflow".into()))?,
                    )?;
                    let mut bytes = BytesMut::with_capacity(size);
                    encoder
                        .write_batch(&row, &mut bytes)
                        .map_err(|e| Error::Codec(e.to_string()))?;
                    if bytes.len() > size {
                        return Err(Error::Codec(
                            "COPY encoder exceeded its admitted size".into(),
                        ));
                    }
                    copy.send(bytes.freeze()).await?;
                }
                let _footer = budget.reserve("postgres-copy-wire", 64)?;
                let mut bytes = BytesMut::with_capacity(32);
                encoder
                    .write_footer(&mut bytes)
                    .map_err(|e| Error::Codec(e.to_string()))?;
                copy.send(bytes.freeze()).await?;
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {
                    copy.finish().await?;
                }
                Err(operation) => {
                    if let Err(source) = copy.abort("typed generation COPY refused").await {
                        return Err(Error::CopyAbort {
                            operation: Box::new(operation),
                            source,
                        });
                    }
                    return Err(operation);
                }
            }
            Ok(())
        })
        .await
    }
}

fn copy_size(row: &RecordBatch, builders: &[EncoderBuilder]) -> Result<usize, Error> {
    row.columns()
        .iter()
        .zip(builders)
        .try_fold(2usize, |bytes, (column, builder)| {
            let encoder = builder
                .try_new(column.as_ref())
                .map_err(|e| Error::Codec(e.to_string()))?;
            let size = encoder
                .byte_size_hint()
                .map_err(|e| Error::Codec(e.to_string()))?;
            bytes
                .checked_add(size)
                .ok_or_else(|| Error::Codec("COPY row size overflow".into()))
        })
}

/// The control records a generation owns; cleanup removes them with its schema.
const CONTROL_RECORDS: [&str; 15] = [
    "publication_outputs",
    "epoch_receipts",
    "publication_groups",
    "receipts",
    "stage_read_checks",
    "validation_receipts",
    "stage_receipts",
    "planned_outputs",
    "stage_outcomes",
    "checkpoint_frame_receipts",
    "checkpoints",
    "admission_families",
    "admissions",
    "failures",
    "events",
];
/// A registry row's lifecycle facts.
struct Registered {
    state: String,
    frontier: Frontier,
}
impl Registered {
    /// The state a step requires.
    fn expect(&self, state: &str) -> Result<(), Error> {
        if self.state != state {
            return Err(Error::State);
        }
        Ok(())
    }
}

async fn check_schedule(
    connection: &mut PgConnection,
    g: GenerationId,
    expected: ContentHash,
) -> Result<(), Error> {
    let actual: Vec<u8> =
        sqlx::query_scalar("SELECT schedule_digest FROM lctx_model_store.generations WHERE id=$1")
            .bind(g.0.to_vec())
            .fetch_one(connection)
            .await?;
    if actual != expected.0 {
        return Err(Error::Contract);
    }
    Ok(())
}

pub struct GenerationLease {
    frontier: Frontier,
    connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
    contract: LeaseContract,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    /// The relations the generation's frontier holds; any other is refused before a scan.
    relations: BTreeSet<&'static str>,
}
impl GenerationLease {
    async fn acquire(
        reader: &PgPool,
        model: Arc<ValidatedModel>,
        contract: LeaseContract,
        budget: ResourceBudget,
    ) -> Result<Self, Error> {
        // A single leased session owns both its pool permit and generation lock until release.
        let mut connection = reader.acquire().await?;
        connection.close_on_drop();
        let held = contract.acquire(&mut lease::Sqlx(&mut connection)).await?;
        Ok(Self {
            connection,
            contract,
            model,
            budget,
            relations: held.relations,
            frontier: held.frontier,
        })
    }
    pub fn generation(&self) -> GenerationId {
        self.contract.generation()
    }
    /// Consume the reader and wait for server acknowledgment that its session lease is released.
    /// Drop still closes the connection conservatively, but does not acknowledge lock release.
    /// The connection remains close-on-drop on error or cancellation and is never pooled again.
    pub async fn release(mut self) -> Result<(), Error> {
        let released = self
            .contract
            .release(&mut lease::Sqlx(&mut self.connection))
            .await;
        let closed = self.connection.close().await;
        released?;
        closed?;
        Ok(())
    }
    /// Visit bounded typed batches while borrowing the original leased connection.
    /// No connection reacquisition is permitted after a transport failure.
    pub async fn visit<R: Record>(
        &mut self,
        mut visitor: impl FnMut(Batch<R>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if !self.relations.contains(R::NAME) {
            return Err(Error::Frontier(format!(
                "{} is outside this generation's frontier",
                R::NAME
            )));
        }
        self.connection.ping().await?;
        let relation = self.model.require::<R>()?;
        visit_physical(
            &mut self.connection,
            self.contract.generation(),
            relation,
            &["id"],
            &self.budget,
            |batch| visitor(Batch::read(&self.model, &batch, &self.budget)?),
        )
        .await
    }
    /// Convenience for small relations. Larger consumers must use `visit`.
    pub async fn read<R: Record>(&mut self) -> Result<Batch<R>, Error> {
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        let mut retained_bytes = 0usize;
        let mut retained = self.budget.reserve("generation_small_read", 0)?;
        self.visit::<R>(|batch| {
            bytes = bytes
                .checked_add(batch.arrow().get_array_memory_size())
                .ok_or_else(|| Error::Codec("read budget overflow".into()))?;
            if bytes > TRANSFER_BYTES {
                return Err(Error::Model(ModelError::Limit {
                    owner: R::NAME,
                    limit: "small-relation read bytes (use visit)",
                    observed: bytes,
                    bound: TRANSFER_BYTES,
                }));
            }
            for row in batch.rows() {
                retained_bytes = retained_bytes
                    .checked_add(std::mem::size_of::<R>().saturating_mul(2))
                    .and_then(|size| size.checked_add(row.heap_bytes()))
                    .ok_or_else(|| Error::Codec("typed read budget overflow".into()))?;
            }
            retained.try_resize(retained_bytes)?;
            rows.extend_from_slice(batch.rows());
            Ok(())
        })
        .await?;
        Ok(Batch::new(&self.model, rows, &self.budget)?)
    }
    /// Streaming set lookup for large original artifacts. The finite owner declares the
    /// nominal relation field and ordering; both are checked before building SQL.
    pub async fn visit_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
        order: &[&str],
        mut visitor: impl FnMut(Batch<R>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let relation = self.model.require::<R>()?.clone();
        if !self.relations.contains(R::NAME) {
            return Err(Error::Frontier(R::NAME.into()));
        }
        if field != "id" || std::any::TypeId::of::<R>() != std::any::TypeId::of::<T>() {
            let f = relation
                .fields()
                .iter()
                .find(|f| f.name() == field)
                .ok_or(Error::Contract)?;
            if f.list()
                || f.scalar() != lctx_model::domain::Scalar::Id
                || f.target().map(|t| t.0) != Some(std::any::TypeId::of::<T>())
            {
                return Err(Error::Contract);
            }
        }
        if order.is_empty()
            || order.iter().any(|name| {
                *name != "id"
                    && !relation
                        .fields()
                        .iter()
                        .any(|f| f.name() == *name && !f.list())
            })
        {
            return Err(Error::Contract);
        }
        let _keys_charge = self
            .budget
            .reserve("serving-set-keys", ids.len().saturating_mul(64))?;
        let keys: Vec<_> = ids.iter().map(|id| id.bytes().to_vec()).collect();
        let model = self.model.clone();
        let budget = self.budget.clone();
        let generation = self.generation();
        let result = visit_physical_inner(
            &mut self.connection,
            generation,
            &relation,
            R::NAME,
            order,
            &budget,
            Some((field, &keys)),
            |arrow| visitor(Batch::read(&model, &arrow, &budget)?),
        )
        .await;
        self.connection.shrink_buffers();
        result
    }
    /// Set-based packet hydration. Field names are checked against nominal declarations before
    /// SQL construction; identifiers never arrive from a serving request.
    pub async fn read_ids<R: Record>(&mut self, ids: &[Id<R>]) -> Result<Batch<R>, Error> {
        self.read_for::<R, R>("id", ids).await
    }
    pub async fn read_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
    ) -> Result<Batch<R>, Error> {
        let relation = self
            .model
            .relations()
            .iter()
            .find(|r| r.name() == R::NAME)
            .ok_or(Error::Contract)?
            .clone();
        if !self.relations.contains(R::NAME) {
            return Err(Error::Frontier(R::NAME.into()));
        }
        if field != "id" || std::any::TypeId::of::<R>() != std::any::TypeId::of::<T>() {
            let declaration = relation
                .fields()
                .iter()
                .find(|f| f.name() == field)
                .ok_or(Error::Contract)?;
            if declaration.list()
                || declaration.scalar() != lctx_model::domain::Scalar::Id
                || declaration.target().map(|t| t.0) != Some(std::any::TypeId::of::<T>())
            {
                return Err(Error::Contract);
            }
        }
        let mut retained = self
            .budget
            .reserve("serving-set-read", ids.len().saturating_mul(64))?;
        let keys: Vec<_> = ids.iter().map(|id| id.bytes().to_vec()).collect();
        let mut rows = Vec::new();
        let mut bytes = keys.len().saturating_mul(64);
        let model = self.model.clone();
        let budget = self.budget.clone();
        let generation = self.generation();
        let result = visit_physical_inner(
            &mut self.connection,
            generation,
            &relation,
            R::NAME,
            &["id"],
            &budget,
            Some((field, &keys)),
            |arrow| {
                let batch = Batch::<R>::read(&model, &arrow, &budget)?;
                for row in batch.rows() {
                    bytes = bytes
                        .checked_add(size_of::<R>().saturating_mul(2) + row.heap_bytes())
                        .ok_or(Error::Contract)?;
                }
                retained.try_resize(bytes)?;
                rows.extend_from_slice(batch.rows());
                Ok(())
            },
        )
        .await;
        self.connection.shrink_buffers();
        result?;
        Ok(Batch::new(&model, rows, &budget)?)
    }
}

/// Rows are buffered up to one transfer batch. Raw rows and their decoded form are admitted
/// before a row is retained, and released once the visitor has consumed the batch.
async fn visit_physical(
    connection: &mut PgConnection,
    g: GenerationId,
    relation: &Relation,
    order: &[&str],
    budget: &ResourceBudget,
    visitor: impl FnMut(RecordBatch) -> Result<(), Error>,
) -> Result<(), Error> {
    visit_named(
        connection,
        g,
        relation,
        relation.name(),
        order,
        budget,
        visitor,
    )
    .await
}
async fn visit_named(
    connection: &mut PgConnection,
    g: GenerationId,
    relation: &Relation,
    physical: &str,
    order: &[&str],
    budget: &ResourceBudget,
    visitor: impl FnMut(RecordBatch) -> Result<(), Error>,
) -> Result<(), Error> {
    let result = visit_physical_inner(
        connection, g, relation, physical, order, budget, None, visitor,
    )
    .await;
    // SQLx retains the largest protocol buffers unless explicitly shrunk. The stream is gone
    // before this call, including on a visitor/resource refusal.
    connection.shrink_buffers();
    result
}

#[allow(
    clippy::too_many_arguments,
    reason = "Validated physical read inputs, budget and visitor keep the SQL effect local"
)]
async fn visit_physical_inner(
    connection: &mut PgConnection,
    g: GenerationId,
    relation: &Relation,
    physical: &str,
    order: &[&str],
    budget: &ResourceBudget,
    filter: Option<(&str, &[Vec<u8>])>,
    mut visitor: impl FnMut(RecordBatch) -> Result<(), Error>,
) -> Result<(), Error> {
    let columns = relation
        .schema()
        .fields()
        .iter()
        .map(|f| quoted(f.name()))
        .collect::<Vec<_>>()
        .join(",");
    let order = order
        .iter()
        .map(|name| {
            let text = relation.fields().iter().any(|field| {
                field.name() == *name
                    && field.scalar() == lctx_model::domain::Scalar::Text
                    && !field.list()
            });
            if text {
                format!("{} COLLATE \"C\"", quoted(name))
            } else {
                quoted(name)
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT {}{columns} FROM {}{} ORDER BY {order}",
        if physical.starts_with("__delta_") {
            "DISTINCT "
        } else {
            ""
        },
        qualified(g, physical),
        filter.map_or(String::new(), |(field, _)| format!(
            " WHERE {}=ANY($1)",
            quoted(field)
        ))
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(query));
    if let Some((_, keys)) = filter {
        query = query.bind(keys);
    }
    let mut stream = query.fetch(connection);
    let mut rows = Vec::new();
    let mut bytes = 0usize;
    let mut held = budget.reserve("postgres-read", 0)?;
    while let Some(row) = stream.try_next().await? {
        let mut size = 0usize;
        for index in 0..row.len() {
            let value = row.try_get_raw(index)?;
            if !value.is_null() {
                size = size
                    .checked_add(
                        value
                            .as_bytes()
                            .map_err(|e| Error::Codec(e.to_string()))?
                            .len(),
                    )
                    .ok_or_else(|| Error::Codec("row size overflow".into()))?;
            }
        }
        if size > MAX_ROW_BYTES {
            return Err(Error::Model(ModelError::Limit {
                owner: relation.name(),
                limit: "stored row bytes",
                observed: size,
                bound: MAX_ROW_BYTES,
            }));
        }
        if !rows.is_empty()
            && (rows.len() == TRANSFER_ROWS || bytes.saturating_add(size) > TRANSFER_BYTES)
        {
            visitor(codec::decode(relation, &rows)?)?;
            rows.clear();
            bytes = 0;
            held.try_resize(0)?;
        }
        held.try_resize(bytes.saturating_add(size).saturating_mul(3))?;
        bytes += size;
        rows.push(row);
    }
    if !rows.is_empty() {
        visitor(codec::decode(relation, &rows)?)?;
    }
    Ok(())
}
async fn lock(connection: &mut PgConnection, g: GenerationId, shared: bool) -> Result<(), Error> {
    let query = if shared {
        "SELECT pg_advisory_xact_lock_shared($1)"
    } else {
        "SELECT pg_advisory_xact_lock($1)"
    };
    sqlx::query(query)
        .bind(g.lock())
        .execute(connection)
        .await?;
    Ok(())
}
async fn transition(
    connection: &mut PgConnection,
    g: GenerationId,
    next: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE lctx_model_store.generations SET state=$2 WHERE id=$1")
        .bind(g.0.to_vec())
        .bind(next)
        .execute(&mut *connection)
        .await?;
    sqlx::query("INSERT INTO lctx_model_store.events(generation_id,state) VALUES($1,$2)")
        .bind(g.0.to_vec())
        .bind(next)
        .execute(connection)
        .await?;
    Ok(())
}
async fn execute(connection: &mut PgConnection, statements: Vec<String>) -> Result<(), Error> {
    for sql in statements {
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn qualified(g: GenerationId, relation: &str) -> String {
    format!("{}.{}", quoted(&g.schema()), quoted(relation))
}

/// Completed domain refusals imply acknowledged rollback. Commit/rollback transport failures
/// remain explicitly unconfirmed and quarantine the connection. Cancellation uses SQLx drop cleanup.
async fn transaction<T>(
    pool: &PgPool,
    body: impl for<'a> AsyncFnOnce(&'a mut PgConnection) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut connection = pool.acquire().await?;
    let result = transaction_on(&mut connection, body).await;
    if matches!(&result, Err(Error::Commit(_) | Error::Rollback { .. })) {
        connection.close_on_drop();
    }
    result
}
async fn transaction_on<T>(
    connection: &mut PgConnection,
    body: impl for<'a> AsyncFnOnce(&'a mut PgConnection) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut tx = connection.begin().await?;
    let result = match body(&mut tx).await {
        Ok(value) => tx.commit().await.map_err(Error::Commit).map(|()| value),
        Err(operation) => match tx.rollback().await {
            Ok(()) => Err(operation),
            Err(source) => Err(Error::Rollback {
                operation: Box::new(operation),
                source,
            }),
        },
    };
    connection.shrink_buffers();
    result
}
