//! The PostgreSQL runtime roles and their protected connection configuration (cutover plan
//! P1.5). Role identity is shared by the generation store and the dormant serving code.
use crate::{Error, load_protected};
use serde::{Deserialize, Serialize};
use sqlx::{ConnectOptions, postgres::{PgConnectOptions, PgSslMode}};
use std::{path::Path, str::FromStr};

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
    pub(crate) fn options(&self) -> Result<PgConnectOptions, Error> {
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
}
