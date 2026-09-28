//! Role-specific, lifespan-owned serving/import connections. No compiler dependency.
use crate::{Error, MIGRATOR, load_protected};
use serde::{Deserialize, Serialize};
use sqlx::{
    ConnectOptions, Connection, PgConnection, PgPool, Postgres,
    pool::PoolConnection,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{path::Path, str::FromStr, time::Duration};

pub const EXTENSION_VERSION: &str = "0.8.6";
pub const STAGING_BYTES: usize = 16 * 1024 * 1024;
pub const IMPORT_BUFFER_BYTES: usize = 128 * 1024 * 1024;
pub const TEST_IMAGE: &str = include_str!("../../../specs/postgres-vector-image.txt");

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Importer,
    Serving,
}
impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Self::Importer => "lctx_importer",
            Self::Serving => "lctx_serving",
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    pub format: u32,
    pub role: Role,
    pub url: String,
    /// Combined SQLx + provider budget, not a per-pool allocation.
    pub max_connections: u32,
    pub provider_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub statement_timeout_seconds: u64,
    pub lock_timeout_seconds: u64,
}
impl std::fmt::Debug for RoleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoleConfig")
            .field("role", &self.role)
            .field("credentials", &"[redacted]")
            .field("max_connections", &self.max_connections)
            .finish()
    }
}
impl RoleConfig {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let config: Self = load_protected(path)?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), Error> {
        let max = if self.role == Role::Importer { 2 } else { 6 };
        if self.format != 1
            || !(1..=max).contains(&self.max_connections)
            || self.provider_connections >= self.max_connections
            || self.provider_connections > 2
            || self.role == Role::Importer && self.provider_connections != 0
            || !(1..=60).contains(&self.acquire_timeout_seconds)
            || !(1..=300).contains(&self.statement_timeout_seconds)
            || !(1..=60).contains(&self.lock_timeout_seconds)
        {
            return Err(Error::Config("invalid role, pool or timeout contract"));
        }
        let url =
            url::Url::parse(&self.url).map_err(|_| Error::Config("invalid role connection URL"))?;
        if !matches!(url.scheme(), "postgres" | "postgresql")
            || url.host_str().is_none()
            || url.path().len() < 2
            || url.fragment().is_some()
            || url
                .query_pairs()
                .any(|(k, _)| !matches!(k.as_ref(), "sslmode" | "sslrootcert"))
        {
            return Err(Error::Config("unsupported connection URL option"));
        }
        let mut keys = std::collections::BTreeSet::new();
        if url.query_pairs().any(|(k, _)| !keys.insert(k.into_owned())) {
            return Err(Error::Config("duplicate connection URL option"));
        }
        self.options()?;
        Ok(())
    }
    fn options(&self) -> Result<PgConnectOptions, Error> {
        let options = PgConnectOptions::from_str(&self.url)
            .map_err(|_| Error::Config("invalid role connection URL"))?;
        let host = options.get_host();
        let local = host.starts_with('/') || matches!(host, "localhost" | "127.0.0.1" | "::1");
        if !local && !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
            return Err(Error::Config(
                "remote PostgreSQL requires sslmode=verify-full",
            ));
        }
        if options.get_username() != self.role.name() {
            return Err(Error::Config("connection role mismatch"));
        }
        Ok(options
            .disable_statement_logging()
            .application_name(match self.role {
                Role::Importer => "lctx-import",
                Role::Serving => "lctx-serving",
            })
            .options([
                ("search_path", "pg_catalog,lctx_ext".to_owned()),
                (
                    "statement_timeout",
                    format!("{}s", self.statement_timeout_seconds),
                ),
                ("lock_timeout", format!("{}s", self.lock_timeout_seconds)),
                ("idle_in_transaction_session_timeout", "30s".to_owned()),
                ("transaction_timeout", "30s".to_owned()),
                (
                    "default_transaction_read_only",
                    if self.role == Role::Serving {
                        "on"
                    } else {
                        "off"
                    }
                    .to_owned(),
                ),
            ]))
    }
    async fn pool(&self) -> Result<PgPool, Error> {
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
        Ok(Self {
            connection: LeaseConnection(Some(pool.acquire().await?)),
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

async fn check_connection(conn: &mut PgConnection) -> Result<(), Error> {
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
}
pub struct ImportStore {
    pub(crate) pool: PgPool,
}
#[derive(Debug, Serialize)]
pub struct ServingHealth {
    pub role: String,
    pub extension: &'static str,
    pub schema_current: bool,
    pub connections: u32,
}
impl ServingStore {
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
