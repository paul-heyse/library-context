//! Canonical owner-free runtime reader; provisioning authority never enters this factory.
use super::{Error, GenerationId, GenerationLease, LeaseContract};
use crate::roles::{Role, RoleConfig};
use lctx_model::domain::{ValidatedModel, resources::ResourceBudget};
use std::sync::Arc;
#[derive(Clone)]
pub struct GenerationReader {
    model: Arc<ValidatedModel>,
    pool: sqlx::PgPool,
}
impl GenerationReader {
    pub async fn connect(model: Arc<ValidatedModel>, config: &RoleConfig) -> Result<Self, Error> {
        if config.role != Role::Serving {
            return Err(Error::Contract);
        }
        let pool = config
            .connect()
            .await
            .map_err(|e| Error::Codec(e.to_string()))?;
        Ok(Self { model, pool })
    }
    pub async fn pin(
        &self,
        g: GenerationId,
        budget: ResourceBudget,
    ) -> Result<GenerationLease, Error> {
        GenerationLease::acquire(
            &self.pool,
            self.model.clone(),
            LeaseContract::new(&self.model, g),
            budget,
        )
        .await
    }
    pub async fn selected(&self) -> Result<GenerationId, Error> {
        let bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT generation_id FROM lctx_model_store.selection WHERE singleton",
        )
        .fetch_one(&self.pool)
        .await?;
        let bytes = bytes.ok_or(Error::Absent)?;
        Ok(GenerationId(bytes.try_into().map_err(|_| Error::Contract)?))
    }
    pub(super) async fn check_serving_capacity(&self, required: u32) -> Result<(), Error> {
        let (free, role_limit, role_used, database_limit, database_used): (i64,i64,i64,i64,i64) = sqlx::query_as(
            "SELECT current_setting('max_connections')::bigint-current_setting('superuser_reserved_connections')::bigint-current_setting('reserved_connections')::bigint-(SELECT count(*) FROM pg_stat_activity WHERE backend_type='client backend'), r.rolconnlimit::bigint,(SELECT count(*) FROM pg_stat_activity WHERE usename=current_user),d.datconnlimit::bigint,(SELECT count(*) FROM pg_stat_activity WHERE datname=current_database()) FROM pg_roles r,pg_database d WHERE r.rolname=current_user AND d.datname=current_database()")
            .fetch_one(&self.pool).await?;
        let remaining = i64::from(required.saturating_sub(self.pool.size()));
        if free < remaining
            || (role_limit >= 0 && role_limit - role_used < remaining)
            || (database_limit >= 0 && database_limit - database_used < remaining)
        {
            return Err(Error::Busy);
        }
        Ok(())
    }
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
