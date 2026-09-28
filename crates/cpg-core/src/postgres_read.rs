//! The provider's driver and pool stay isolated here. PG15 owns admitted scans and federation.
use datafusion_table_providers_postgres::pool::PostgresConnectionPool;
use lctx_postgres::{
    Error,
    serving::{Role, RoleConfig},
};
use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

pub struct ProviderPool {
    pool: Arc<PostgresConnectionPool>,
    limit: u32,
}
impl ProviderPool {
    pub async fn open(config: &RoleConfig) -> Result<Self, Error> {
        config.validate()?;
        if config.role != Role::Serving || config.provider_connections == 0 {
            return Err(Error::Config("provider read-pool budget required"));
        }
        let mut url =
            url::Url::parse(&config.url).map_err(|_| Error::Config("invalid provider URL"))?;
        let mut ssl = "prefer".to_owned();
        let mut root = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "sslmode" => ssl = value.into_owned(),
                "sslrootcert" => root = Some(PathBuf::from(value.into_owned())),
                _ => return Err(Error::Config("unsupported provider option")),
            }
        }
        if !matches!(
            ssl.as_str(),
            "disable" | "prefer" | "require" | "verify-ca" | "verify-full"
        ) {
            return Err(Error::Config("unsupported provider TLS mode"));
        }
        url.set_query(None);
        let mut driver = tokio_postgres::Config::from_str(url.as_str())
            .map_err(|_| Error::Config("invalid provider configuration"))?;
        driver.application_name("lctx-provider").connect_timeout(Duration::from_secs(config.acquire_timeout_seconds))
   .ssl_mode(if ssl=="disable"{tokio_postgres::config::SslMode::Disable}else if ssl=="prefer"{tokio_postgres::config::SslMode::Prefer}else{tokio_postgres::config::SslMode::Require})
   .options(format!("-c search_path=pg_catalog,lctx_ext -c default_transaction_read_only=on -c statement_timeout={}s -c lock_timeout={}s -c idle_in_transaction_session_timeout=30s",config.statement_timeout_seconds,config.lock_timeout_seconds));
        let pool = PostgresConnectionPool::new_with_config(
            driver,
            &ssl,
            root,
            config.provider_connections,
            Duration::from_secs(config.acquire_timeout_seconds),
        )
        .await
        .map_err(|_| Error::Config("provider connection failed"))?;
        Ok(Self {
            pool: Arc::new(pool),
            limit: config.provider_connections,
        })
    }
    pub fn pool(&self) -> Arc<PostgresConnectionPool> {
        Arc::clone(&self.pool)
    }
    pub fn connection_limit(&self) -> u32 {
        self.limit
    }
}
