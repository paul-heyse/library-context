//! The PostgreSQL runtime roles and their protected connection configuration (cutover plan
//! P1.5). Role identity is shared by the generation store and the dormant serving code.
use crate::{Error, load_protected};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnectOptions;
use std::path::Path;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
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

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
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
        let pools_valid = match self.role {
            Role::Importer => {
                (1..=32).contains(&self.provider_connections)
                    && self.max_connections == self.provider_connections + 2
            }
            Role::Serving => {
                (1..=6).contains(&self.max_connections)
                    && self.provider_connections <= 2
                    && self.provider_connections < self.max_connections
            }
        };
        if self.format != 1
            || !pools_valid
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
    /// A pool of this runtime role for the generation store: the role, PostgreSQL 18 and no
    /// elevated attribute confirmed on the first connection.
    pub async fn connect(&self) -> Result<sqlx::PgPool, Error> {
        self.validate()?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.max_connections - self.provider_connections)
            .acquire_timeout(std::time::Duration::from_secs(self.acquire_timeout_seconds))
            .idle_timeout(std::time::Duration::from_secs(60))
            .connect_with(self.options()?)
            .await?;
        let (role, version, elevated): (String, String, bool) = sqlx::query_as("SELECT current_user::text, current_setting('server_version_num'), \
            rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls FROM pg_catalog.pg_roles WHERE rolname = current_user")
            .fetch_one(&pool).await?;
        if role != self.role.name() || version.parse::<u32>().unwrap_or(0) / 10000 != 18 || elevated
        {
            pool.close().await;
            return Err(Error::Config(
                "server, role or privileges differ from the role contract",
            ));
        }
        if self.role == Role::Importer {
            self.check_capacity(&pool).await?;
        }
        Ok(pool)
    }
    /// Preflight the configured combined importer budget before an attempt is registered.
    /// The two owner slots cover lifecycle and control work and are accounted separately.
    /// PostgreSQL still arbitrates races with other clients when the bound pool opens.
    pub async fn check_capacity(&self, pool: &sqlx::PgPool) -> Result<(), Error> {
        self.validate()?;
        let (server_free, role_limit, role_used, database_limit, database_used): (i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT current_setting('max_connections')::bigint-current_setting('superuser_reserved_connections')::bigint-current_setting('reserved_connections')::bigint-(SELECT count(*) FROM pg_stat_activity WHERE backend_type='client backend'),\
             r.rolconnlimit::bigint,(SELECT count(*) FROM pg_stat_activity WHERE usename=current_user),\
             d.datconnlimit::bigint,(SELECT count(*) FROM pg_stat_activity WHERE datname=current_database())\
             FROM pg_roles r, pg_database d WHERE r.rolname=current_user AND d.datname=current_database()")
            .fetch_one(pool).await?;
        let importer_remaining = i64::from(self.max_connections.saturating_sub(pool.size()));
        let with_owner = importer_remaining + 2;
        if server_free < with_owner
            || (role_limit >= 0 && role_limit - role_used < importer_remaining)
            || (database_limit >= 0 && database_limit - database_used < with_owner)
        {
            return Err(Error::Config(
                "insufficient PostgreSQL capacity for importer budget plus two owner connections",
            ));
        }
        Ok(())
    }
    pub(crate) fn options(&self) -> Result<PgConnectOptions, Error> {
        let options = crate::connection_options::parse(&self.url, "invalid role connection URL")?;
        if options.get_username() != self.role.name() {
            return Err(Error::Config("connection role mismatch"));
        }
        Ok(crate::connection_options::session(
            options,
            match self.role {
                Role::Importer => "lctx-import",
                Role::Serving => "lctx-serving",
            },
            self.statement_timeout_seconds,
            self.lock_timeout_seconds,
            "pg_catalog,lctx_ext",
        )
        .options([
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
}
