//! SQLx owns PostgreSQL effects; neither connections nor operational rows are semantic inputs.

mod cache;
pub mod bootstrap;
pub mod diagnostics;
mod evidence;
mod evidence_search;
mod hydration;
pub mod import;
mod journey_cursor;
mod journeys;
pub mod operations;
mod packet;
pub mod profiles;
pub mod projection;
pub mod report;
pub mod repository;
pub mod retrieval;
pub mod roles;
#[cfg(feature = "testing")]
pub mod testing;
pub mod serving;
pub mod generations;

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
    #[error("PostgreSQL retrieval admission unavailable: {0}")]
    Admission(&'static str),
    #[error("PostgreSQL configuration: {0}")]
    Config(&'static str),
    #[error("invalid serving request: {0}")]
    Request(String),
    #[error("PostgreSQL {kind} (SQLSTATE {code})")]
    Database { kind: &'static str, code: String },
    #[error("PostgreSQL schema is incompatible; run lctx db status with the matching binary")]
    Schema,
    #[error("PostgreSQL data integrity: {0}")]
    Integrity(&'static str),
    #[error("PostgreSQL migration failed; inspect migration status using the migration identity")]
    Migration,
    /// The database carries migration history from before the service baseline. It is moved by the
    /// operator transition (cutover plan P1.13), never migrated in place.
    #[error("the database's migration history predates the service baseline; it is refused, not upgraded")]
    LegacyHistory,
    #[error("the configured store owner is not a verified service owner: {0}")]
    Owner(&'static str),
    #[error("the installed canonical schema differs from this binary; run `lctx store reset`")]
    CanonicalSchema,
    #[error("{0}")]
    Projection(#[from] cpg_schema::serving_projection::ProjectionError),
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

#[derive(Clone, Deserialize, schemars::JsonSchema)]
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

/// Read-only credential contract; schema export never serializes an actual credential.
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationConfig {
    pub migration_url: String,
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
            let admin: MigrationConfig = load_protected(path)?;
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

#[derive(Debug, Serialize, schemars::JsonSchema)]
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
        let mut connection = self.pool.acquire().await?;
        if !history_current(&mut connection).await? {
            return Err(Error::Schema);
        }
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

/// Whether the applied migration history is exactly the service baseline this binary declares.
pub(crate) async fn history_current(connection: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
    let present: Option<String> = sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations')::text")
        .fetch_one(&mut *connection).await?;
    if present.is_none() {
        return Ok(false);
    }
    let rows: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as("SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version")
        .fetch_all(&mut *connection).await?;
    let expected: Vec<_> = MIGRATOR.iter().filter(|m| !m.migration_type.is_down_migration()).collect();
    Ok(rows.len() == expected.len()
        && rows.iter().zip(expected).all(|((v, ok, digest), m)| *v == m.version && *ok && digest.as_slice() == m.checksum.as_ref()))
}

/// The runtime roles a service owner must never be joined to.
pub const RUNTIME_ROLES: [&str; 3] = ["lctx_app", "lctx_importer", "lctx_serving"];

/// A pool of the verified service owner: not a superuser, without CREATEROLE or BYPASSRLS, allowed
/// to CREATE in its database, and in no membership edge with a runtime role in either direction.
/// Only [`OwnerPool::verify`] constructs one, so installing the store requires the check.
#[derive(Clone)]
pub struct OwnerPool(sqlx::PgPool);
impl OwnerPool {
    pub async fn verify(pool: sqlx::PgPool) -> Result<Self, Error> {
        let (superuser, createrole, bypassrls, create, joined): (bool, bool, bool, bool, bool) = sqlx::query_as(
            "SELECT r.rolsuper, r.rolcreaterole, r.rolbypassrls, \
                    has_database_privilege(current_user, current_database(), 'CREATE'), \
                    EXISTS (SELECT FROM pg_catalog.pg_auth_members m \
                            JOIN pg_catalog.pg_roles granted ON granted.oid = m.roleid \
                            JOIN pg_catalog.pg_roles member ON member.oid = m.member \
                            WHERE (member.rolname = current_user AND granted.rolname = ANY($1)) \
                               OR (granted.rolname = current_user AND member.rolname = ANY($1))) \
             FROM pg_catalog.pg_roles r WHERE r.rolname = current_user")
            .bind(RUNTIME_ROLES.to_vec()).fetch_one(&pool).await?;
        let refusal = if superuser { Some("a superuser may not own the store") }
            else if createrole || bypassrls { Some("the owner may not hold CREATEROLE or BYPASSRLS") }
            else if !create { Some("the owner lacks CREATE on its database") }
            else if joined { Some("the owner shares a membership edge with a runtime role") }
            else { None };
        match refusal { Some(reason) => Err(Error::Owner(reason)), None => Ok(Self(pool)) }
    }
    pub fn pool(&self) -> &sqlx::PgPool { &self.0 }
}

/// Only the explicitly opened migration identity exposes DDL.
pub struct MigrationStore {
    inner: Store,
}
impl MigrationStore {
    /// Apply the service baseline as the verified owner. A history containing any migration this
    /// binary does not declare is refused before any change.
    pub async fn migrate(&self) -> Result<(), Error> {
        self.owner().await?;
        let present: Option<String> = sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations')::text")
            .fetch_one(&self.inner.pool).await?;
        if present.is_some() {
            let applied: Vec<i64> = sqlx::query_scalar("SELECT version FROM public._sqlx_migrations")
                .fetch_all(&self.inner.pool).await?;
            if applied.iter().any(|version| !MIGRATOR.iter().any(|m| m.version == *version)) {
                return Err(Error::LegacyHistory);
            }
        }
        MIGRATOR
            .run(&self.inner.pool)
            .await
            .map_err(|_| Error::Migration)
    }
    /// The verified owner of this database's store.
    pub async fn owner(&self) -> Result<OwnerPool, Error> {
        OwnerPool::verify(self.inner.pool.clone()).await
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

/// Shared generated schemas for the existing PostgreSQL format owners.
pub fn contract_schema(name: &str, output: bool) -> Result<serde_json::Value, Error> {
    use cpg_schema::wire::schema_for;
    Ok(match name {
        "PostgresConfig" if !output => schema_for::<Config>(false),
        "MigrationConfig" if !output => schema_for::<MigrationConfig>(false),
        "PostgresRoleConfig" => schema_for::<serving::RoleConfig>(output),
        "RetrievalPolicy" => schema_for::<profiles::Policy>(output),
        "Diagnostics" if output => schema_for::<diagnostics::Diagnostics>(true),
        "LiveDiagnostics" if output => schema_for::<diagnostics::LiveDiagnostics>(true),
        _ => return Err(Error::Config("unknown schema or unsupported direction")),
    })
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    #[test]
    fn existing_contracts_and_synthetic_credentials_conform() {
        for (name, directions) in [
            ("PostgresConfig", &[false][..]),
            ("MigrationConfig", &[false][..]),
            ("PostgresRoleConfig", &[false, true][..]),
            ("RetrievalPolicy", &[false, true][..]),
            ("Diagnostics", &[true][..]),
            ("LiveDiagnostics", &[true][..]),
        ] {
            for output in directions {
                assert!(
                    jsonschema::options()
                        .offline()
                        .build(&contract_schema(name, *output).unwrap())
                        .is_ok(),
                    "{name}"
                );
            }
        }
        assert!(contract_schema("PostgresConfig", true).is_err());
        let schema = contract_schema("MigrationConfig", false).unwrap();
        let validator = jsonschema::options().offline().build(&schema).unwrap();
        let synthetic =
            serde_json::json!({"migration_url":"postgresql://example:synthetic@localhost/example"});
        assert!(validator.is_valid(&synthetic));
        assert!(serde_json::from_value::<MigrationConfig>(synthetic).is_ok());
        assert!(!validator.is_valid(&serde_json::json!({"migration_url":3})));
        let policy = profiles::Policy::exact();
        let before = policy.canonical().unwrap();
        let digest = policy.digest().unwrap();
        let validator = jsonschema::options()
            .offline()
            .build(&contract_schema("RetrievalPolicy", false).unwrap())
            .unwrap();
        assert!(validator.is_valid(&serde_json::from_str(&before).unwrap()));
        let after: profiles::Policy = serde_json::from_str(&before).unwrap();
        assert_eq!(after.canonical().unwrap(), before);
        assert_eq!(after.digest().unwrap(), digest);
    }
}

pub mod selection;
