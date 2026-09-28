//! SQLx owns PostgreSQL effects; neither connections nor operational rows are semantic inputs.

mod cache;
pub mod operations;
pub mod projection;
pub mod serving;

use serde::{Deserialize, Serialize};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

pub use cache::CacheValue;

pub const TEST_IMAGE_TAG: &str = include_str!("../../../specs/postgres-image.txt");
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Deliberately omits driver/server text, which may include credentials or bound values.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("PostgreSQL configuration: {0}")]
    Config(&'static str),
    #[error("PostgreSQL {kind} (SQLSTATE {code})")]
    Database { kind: &'static str, code: String },
    #[error("PostgreSQL schema is incompatible; run lctx db status with the matching binary")]
    Schema,
    #[error("PostgreSQL data integrity: {0}")]
    Integrity(&'static str),
    #[error("PostgreSQL migration failed; inspect migration status using the migration identity")]
    Migration,
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        let code = error
            .as_database_error()
            .and_then(|e| e.code())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".to_owned());
        let kind = match error {
            sqlx::Error::PoolTimedOut => "pool acquisition timed out",
            sqlx::Error::PoolClosed => "pool closed",
            sqlx::Error::Io(_) | sqlx::Error::Tls(_) => "connection failed",
            _ => "query failed",
        };
        Self::Database { kind, code }
    }
}

impl Error {
    pub(crate) fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Database {
                kind: "connection failed",
                ..
            }
        ) || matches!(self, Self::Database { code, .. } if code == "40001" || code == "40P01" || code.starts_with("08") || code == "57P01")
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub application_url: String,
    #[serde(default)]
    pub migration_url: String,
    #[serde(skip)]
    pub migration_config: Option<PathBuf>,
    pub max_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub statement_timeout_seconds: u64,
    pub lock_timeout_seconds: u64,
    pub max_receipt_bytes: usize,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresConfig")
            .field("credentials", &"[redacted]")
            .field("max_connections", &self.max_connections)
            .finish()
    }
}

impl Config {
    pub fn path(explicit: Option<&Path>) -> Result<PathBuf, Error> {
        explicit
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("LCTX_DATABASE_CONFIG").map(PathBuf::from))
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".config/library-context/postgres.json"))
            })
            .ok_or(Error::Config("supply --database-config"))
    }

    pub fn load(path: &Path) -> Result<Self, Error> {
        let mut config: Self = load_protected(path)?;
        config.migration_config = Some(path.with_file_name("postgres-admin.json"));
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), Error> {
        if !(1..=32).contains(&self.max_connections)
            || !(1..=60).contains(&self.acquire_timeout_seconds)
            || !(1..=300).contains(&self.statement_timeout_seconds)
            || !(1..=60).contains(&self.lock_timeout_seconds)
            || !(16384..=1_073_741_824).contains(&self.max_receipt_bytes)
        {
            return Err(Error::Config(
                "pool, timeout or receipt limit is outside supported bounds",
            ));
        }
        Ok(())
    }

    pub async fn connect_application(&self) -> Result<Store, Error> {
        self.connect_role(false).await
    }
    pub async fn connect_migrator(&self) -> Result<MigrationStore, Error> {
        let mut config = self.clone();
        if config.migration_url.is_empty() {
            let path = config
                .migration_config
                .as_ref()
                .ok_or(Error::Config("missing migration config"))?;
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Admin {
                migration_url: String,
            }
            let admin: Admin = load_protected(path)?;
            config.migration_url = admin.migration_url;
        }
        Ok(MigrationStore {
            inner: config.connect_role(true).await?,
        })
    }

    async fn connect_role(&self, migration: bool) -> Result<Store, Error> {
        self.validate()?;
        let options = PgConnectOptions::from_str(if migration {
            &self.migration_url
        } else {
            &self.application_url
        })
        .map_err(|_| Error::Config("invalid connection URL"))?;
        let host = options.get_host();
        let local =
            host.starts_with('/') || host == "localhost" || host == "127.0.0.1" || host == "::1";
        if !local && !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
            return Err(Error::Config(
                "remote PostgreSQL requires sslmode=verify-full",
            ));
        }
        let options = options
            .disable_statement_logging()
            .application_name("lctx")
            .options([
                (
                    "statement_timeout",
                    format!("{}s", self.statement_timeout_seconds),
                ),
                ("lock_timeout", format!("{}s", self.lock_timeout_seconds)),
                ("idle_in_transaction_session_timeout", "30s".to_owned()),
                (
                    "search_path",
                    if migration {
                        "public,pg_catalog,lctx_ext"
                    } else {
                        "pg_catalog,lctx_ext"
                    }
                    .to_owned(),
                ),
            ]);
        let pool = PgPoolOptions::new()
            .max_connections(if migration { 2 } else { self.max_connections })
            .acquire_timeout(Duration::from_secs(self.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(60))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(options)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&pool)
            .await?;
        if version
            .parse::<u32>()
            .map_err(|_| Error::Integrity("server version"))?
            / 10000
            != 18
        {
            pool.close().await;
            return Err(Error::Config("PostgreSQL major version 18 is required"));
        }
        Ok(Store {
            pool,
            receipt_budget: self.max_receipt_bytes,
        })
    }
}

#[derive(Clone)]
pub struct Store {
    pub(crate) pool: sqlx::PgPool,
    receipt_budget: usize,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStore")
            .field("connections", &self.pool.size())
            .finish()
    }
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub server_version: String,
    pub role: String,
    pub schema_current: bool,
    pub connections: u32,
    pub idle_connections: usize,
}

impl Store {
    pub fn receipt_budget(&self) -> usize {
        self.receipt_budget
    }
    pub async fn close(&self) {
        self.pool.close().await;
    }
    pub async fn check(&self) -> Result<(), Error> {
        let present: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations')::text")
                .fetch_one(&self.pool)
                .await?;
        if present.is_none() {
            return Err(Error::Schema);
        }
        let rows: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
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
        let mut conn = self.pool.acquire().await?;
        serving::check_extension(&mut conn).await?;
        Ok(())
    }
    pub async fn health(&self) -> Result<Health, Error> {
        let (version, role): (String, String) =
            sqlx::query_as("SELECT current_setting('server_version_num'), current_user::text")
                .fetch_one(&self.pool)
                .await?;
        Ok(Health {
            server_version: version,
            role,
            schema_current: self.check().await.is_ok(),
            connections: self.pool.size(),
            idle_connections: self.pool.num_idle(),
        })
    }
}

/// Only the explicitly opened migration identity exposes DDL.
pub struct MigrationStore {
    inner: Store,
}
impl MigrationStore {
    pub async fn migrate(&self) -> Result<(), Error> {
        MIGRATOR
            .run(&self.inner.pool)
            .await
            .map_err(|_| Error::Migration)
    }
    pub async fn check(&self) -> Result<(), Error> {
        self.inner.check().await
    }
    pub async fn health(&self) -> Result<Health, Error> {
        self.inner.health().await
    }
    pub async fn close(&self) {
        self.inner.close().await;
    }
}

/// Read protected configuration without leaking file contents or parser diagnostics.
pub(crate) fn load_protected<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::symlink_metadata(path)
            .map_err(|_| Error::Config("unreadable protected config"))?;
        if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 {
            return Err(Error::Config(
                "protected config requires a regular mode-0600 file",
            ));
        }
    }
    let bytes = std::fs::read(path).map_err(|_| Error::Config("unreadable protected config"))?;
    serde_json::from_slice(&bytes).map_err(|_| Error::Config("invalid protected config"))
}
