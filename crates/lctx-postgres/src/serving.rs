//! Role-specific, lifespan-owned serving/import connections. No compiler dependency.
use crate::{Error, MIGRATOR};
use serde::Serialize;
use sqlx::{Connection, PgConnection, PgPool, Postgres, pool::PoolConnection, postgres::PgPoolOptions};
use std::time::Duration;

pub const EXTENSION_VERSION: &str = "0.8.6";
pub const STAGING_BYTES: usize = 16 * 1024 * 1024;
pub const IMPORT_BUFFER_BYTES: usize = 128 * 1024 * 1024;
pub const TEST_IMAGE: &str = include_str!("../../../specs/postgres-vector-image.txt");

pub use crate::roles::{Role, RoleConfig};

impl RoleConfig {
    pub(crate) async fn pool(&self) -> Result<PgPool, Error> {
        self.validate()?;
        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections - self.provider_connections)
            .acquire_timeout(Duration::from_secs(self.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(60))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(self.options()?)
            .await?;
        let checked = async {
            let mut lease = QueryLease::acquire(&pool).await?;
            check_connection(&mut lease.connection).await?;
            let (role, version): (String, String) = sqlx::query_as("SELECT current_user::text, current_setting('server_version_num')").fetch_one(&mut *lease.connection).await?;
            if role != self.role.name() || version.parse::<u32>().unwrap_or(0) / 10000 != 18 { return Err(Error::Config("server or role mismatch")); }
            let elevated:bool=sqlx::query_scalar("SELECT rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls FROM pg_catalog.pg_roles WHERE rolname=current_user").fetch_one(&mut *lease.connection).await?;
            if elevated{return Err(Error::Config("runtime role has elevated privileges"));}
            lease.complete();
            Ok(())
        }.await;
        if let Err(error) = checked {
            pool.close().await;
            return Err(error);
        }
        Ok(pool)
    }
    pub async fn open_serving(&self) -> Result<ServingStore, Error> {
        if self.role != Role::Serving {
            return Err(Error::Config("serving credentials required"));
        }
        Ok(ServingStore {
            cpu_slots: std::sync::Arc::new(tokio::sync::Semaphore::new(2)),
            pool: self.pool().await?,
        })
    }
    pub async fn open_importer(&self) -> Result<ImportStore, Error> {
        if self.role != Role::Importer {
            return Err(Error::Config("importer credentials required"));
        }
        Ok(ImportStore {
            pool: self.pool().await?,
        })
    }
}

/// Keep pool capacity until server work has drained, including after caller cancellation.
/// SQLx 0.9 exposes no PostgreSQL CancelToken; work remains bounded by statement_timeout.
pub(crate) struct QueryLease {
    pub(crate) connection: LeaseConnection,
    pool: PgPool,
    complete: bool,
}
pub(crate) struct LeaseConnection(Option<PoolConnection<Postgres>>);
impl std::ops::Deref for LeaseConnection {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        self.0.as_ref().expect("owned lease")
    }
}
impl std::ops::DerefMut for LeaseConnection {
    fn deref_mut(&mut self) -> &mut PgConnection {
        self.0.as_mut().expect("owned lease")
    }
}
impl QueryLease {
    pub(crate) async fn acquire(pool: &PgPool) -> Result<Self, Error> {
        let started = std::time::Instant::now();
        let connection = pool.acquire().await?;
        tracing::debug!(target:"lctx::postgres", acquire_ms=started.elapsed().as_secs_f64()*1000.0, connections=pool.size(),idle=pool.num_idle(),"database lease acquired");
        Ok(Self {
            connection: LeaseConnection(Some(connection)),
            pool: pool.clone(),
            complete: false,
        })
    }
    pub(crate) fn complete(&mut self) {
        self.complete = true;
    }
}
impl Drop for QueryLease {
    fn drop(&mut self) {
        if self.complete {
            return;
        }
        let Some(mut connection) = self.connection.0.take() else {
            return;
        };
        let pool = self.pool.clone();
        tokio::spawn(async move {
            // Retain the pool permit while synchronizing the wire. A cancelled transaction
            // may first report its pending SQL error; a second ping consumes ReadyForQuery.
            // RoleConfig caps every server statement at 300 seconds.
            let drained = tokio::time::timeout(Duration::from_secs(305), async {
                match connection.ping().await {
                    Err(sqlx::Error::Database(_)) => connection.ping().await,
                    result => result,
                }
            })
            .await;
            if !matches!(drained, Ok(Ok(()))) {
                // Start shutdown before releasing uncertain capacity; no replacement query
                // may amplify work on an unresponsive backend. Later close() still awaits it.
                use futures::FutureExt;
                let _ = pool.close().now_or_never();
                tracing::warn!(target:"lctx::postgres","cancelled lease could not drain; serving pool closed");
            }
            let _ = connection.close().await;
        });
    }
}

pub(crate) async fn check_connection(conn: &mut PgConnection) -> Result<(), Error> {
    let rows: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
        "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut *conn)
    .await?;
    let expected: Vec<_> = MIGRATOR
        .iter()
        .filter(|m| !m.migration_type.is_down_migration())
        .collect();
    if rows.len() != expected.len()
        || rows.iter().zip(expected).any(|((v, ok, digest), m)| {
            *v != m.version || !ok || digest.as_slice() != m.checksum.as_ref()
        })
    {
        return Err(Error::Schema);
    }
    check_extension(conn).await
}

pub(crate) async fn check_extension(conn: &mut PgConnection) -> Result<(), Error> {
    let extension: Option<(String, String)> = sqlx::query_as("SELECT e.extversion, n.nspname::text FROM pg_catalog.pg_extension e JOIN pg_catalog.pg_namespace n ON n.oid=e.extnamespace WHERE e.extname='vector'").fetch_optional(conn).await?;
    if extension != Some((EXTENSION_VERSION.to_owned(), "lctx_ext".to_owned())) {
        return Err(Error::Config("pgvector extension version/schema mismatch"));
    }
    Ok(())
}

#[derive(Clone)]
pub struct ServingStore {
    pub(crate) pool: PgPool,
    cpu_slots: std::sync::Arc<tokio::sync::Semaphore>,
}
pub struct ImportStore {
    pub(crate) pool: PgPool,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ServingHealth {
    pub role: String,
    pub extension: &'static str,
    pub schema_current: bool,
    pub connections: u32,
    pub idle_connections: usize,
}
impl ServingStore {
    /// Two CPU jobs per repository lifetime. Cancellation retains admission until work ends.
    pub async fn run_cpu<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let permit = self
            .cpu_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| cpg_schema::serving_projection::refused("native CPU capacity"))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work()
        })
        .await
        .map_err(|_| Error::Integrity("native CPU task failed"))?
    }
    pub async fn check(&self) -> Result<ServingHealth, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        check_connection(&mut lease.connection).await?;
        let role: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&mut *lease.connection)
            .await?;
        lease.complete();
        Ok(ServingHealth {
            role,
            extension: EXTENSION_VERSION,
            schema_current: true,
            connections: self.pool.size(),
            idle_connections: self.pool.num_idle(),
        })
    }
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
impl ImportStore {
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
