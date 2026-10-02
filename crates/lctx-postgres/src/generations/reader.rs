//! Canonical owner-free runtime reader; provisioning authority never enters this factory.
use super::{Error,GenerationId,GenerationLease,LeaseContract};
use crate::roles::{Role,RoleConfig};
use lctx_model::domain::{ValidatedModel,resources::ResourceBudget};
use std::sync::Arc;
#[derive(Clone)]
pub struct GenerationReader { model:Arc<ValidatedModel>, pool:sqlx::PgPool }
impl GenerationReader {
    pub async fn connect(model:Arc<ValidatedModel>,config:&RoleConfig)->Result<Self,Error> {
        if config.role!=Role::Serving {return Err(Error::Contract);}
        let pool=config.connect().await.map_err(|e|Error::Codec(e.to_string()))?;
        Ok(Self{model,pool})
    }
    pub async fn pin(&self,g:GenerationId,budget:ResourceBudget)->Result<GenerationLease,Error> {
        GenerationLease::acquire(&self.pool,self.model.clone(),LeaseContract::new(&self.model,g),budget).await
    }
    pub async fn close(&self) {self.pool.close().await;}
}
