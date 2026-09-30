//! Database discovery and role pools (cutover plan P1.11). `--database` names the protected
//! application configuration (`postgres.json`); otherwise `LCTX_DATABASE_CONFIG`, then
//! `~/.config/library-context/postgres.json`. Its sibling files select the roles:
//! `postgres-admin.json` the service owner, `postgres-serving.json` the reader.
use cpg_core::postgres::{Config, OwnerPool, Store, roles::RoleConfig};
use lctx_model::domain::ValidatedModel;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Database {
    path: PathBuf,
}
impl Database {
    pub fn discover(explicit: Option<&Path>) -> anyhow::Result<Self> {
        Ok(Self {
            path: Config::path(explicit)?,
        })
    }
    fn config(&self) -> anyhow::Result<Config> {
        Ok(Config::load(&self.path)?)
    }
    /// The verified service owner, through `postgres-admin.json`.
    pub async fn owner(&self) -> anyhow::Result<OwnerPool> {
        Ok(self.config()?.connect_migrator().await?.owner().await?)
    }
    /// The service baseline's migrator, through `postgres-admin.json`.
    pub async fn migrator(&self) -> anyhow::Result<cpg_core::postgres::MigrationStore> {
        Ok(self.config()?.connect_migrator().await?)
    }
    /// The application role for the retained services.
    pub async fn application(&self) -> anyhow::Result<Store> {
        Ok(self.config()?.connect_application().await?)
    }
    /// The reader's protected configuration, `postgres-serving.json`.
    pub fn serving(&self) -> anyhow::Result<RoleConfig> {
        Ok(RoleConfig::load(
            &self.path.with_file_name("postgres-serving.json"),
        )?)
    }
    pub fn importer(&self) -> anyhow::Result<RoleConfig> {
        let config = RoleConfig::load(&self.path.with_file_name("postgres-importer.json"))?;
        if config.role != cpg_core::postgres::roles::Role::Importer {
            anyhow::bail!("generation writer requires importer credentials");
        }
        Ok(config)
    }
    /// The generation writer's protected configuration.
    pub async fn writer(&self) -> anyhow::Result<sqlx::PgPool> {
        let config = self.importer()?;
        Ok(config.connect().await?)
    }
    pub async fn reader(&self) -> anyhow::Result<sqlx::PgPool> {
        Ok(self.serving()?.connect().await?)
    }
}

/// The model this binary lowers and reads.
pub fn model() -> anyhow::Result<Arc<ValidatedModel>> {
    Ok(Arc::new(lctx_model::domain::model()?))
}
