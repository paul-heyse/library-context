//! Closed current-format maintenance sessions.
use crate::{AuthenticationScope, RuntimeConfig};
use lctx_model::domain::ModelError;
use std::sync::Arc;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::Object};

/// The actual maintenance command owns its session, stop policy and recovery
/// recipient together. An error can return the callable factory handback.
pub struct MaintenanceOwner {
    client: Arc<Surreal<Client>>,
    scope: Arc<crate::control::RecoveryScope>,
    lease: crate::control::FactoryLease,
    factory: Arc<crate::control::RecoveryFactoryOwner>,
}
impl MaintenanceOwner {
    pub async fn connect(
        config: &RuntimeConfig,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if config.authentication != AuthenticationScope::Root {
            return Err(ModelError::Conflict(
                "native maintenance installer authority",
            ));
        }
        let factory =
            crate::control::RecoveryFactoryOwner::new(config.receipt_binding()?, budget.clone())?;
        let (client, lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &factory,
        )
        .await?;
        let scope = lease.recovery_scope().clone();
        let owner = Self {
            client,
            scope,
            lease,
            factory,
        };
        if let Err(error) = crate::control::with_recovery_scope(
            owner.scope.clone(),
            crate::control::check_installation(&owner.client, config.service_generation),
        )
        .await
        {
            return owner.finish::<Self>(Err(error)).await;
        }
        Ok(owner)
    }
    pub fn client(&self) -> &Arc<Surreal<Client>> {
        &self.client
    }
    pub fn scope(&self) -> Arc<crate::control::RecoveryScope> {
        self.scope.clone()
    }
    pub async fn require_closed_admission(&self) -> Result<(), ModelError> {
        if marker(&self.client).await?.get("admission_open") != Some(&native(false)) {
            return Err(ModelError::Conflict("native maintenance closed admission"));
        }
        Ok(())
    }
    /// Check closed native admission and persisted outcomes. The enclosing
    /// service maintenance owner separately authenticates host/executor drainage.
    pub async fn require_closed_settled_effects(&self) -> Result<(), ModelError> {
        self.require_closed_admission().await?;
        crate::control::check_pending_effects(&self.client).await
    }
    pub async fn finish<T>(self, result: Result<T, ModelError>) -> Result<T, ModelError> {
        use lctx_model::domain::completion::{Completion, LocalState, RemoteState, complete};
        let mut completion = Completion::default();
        let settled = crate::control::resume_recovery_scope(&self.scope).await;
        let may_close = settled.is_ok()
            && self.scope.pending_count()? == 0
            && result
                .as_ref()
                .err()
                .is_none_or(ModelError::permits_storage_cleanup);
        completion.step("maintenance pending request settlement", settled);
        if may_close {
            completion.step(
                "maintenance session and signal finalization",
                self.lease.complete().await,
            );
        } else {
            completion.local = LocalState::Outstanding;
            completion.remote = RemoteState::Unknown;
        }
        complete(result, completion).map_err(|error| self.factory.handback(error))
    }
}
fn native(value: bool) -> surrealdb::types::Value {
    surrealdb::types::Value::Bool(value)
}
async fn marker(client: &Surreal<Client>) -> Result<Object, ModelError> {
    let mut response = client
        .query("SELECT * FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict(
            "native maintenance installation cardinality",
        ));
    };
    Ok(row.clone())
}
pub async fn maintenance_client(
    config: &RuntimeConfig,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    if config.authentication != AuthenticationScope::Root {
        return Err(ModelError::Conflict(
            "native maintenance installer authority",
        ));
    }
    let client = crate::installation::check_installation(config).await?;
    let session = crate::reader::SessionFinalizationGuard::new(client.clone());
    let result = async {
        if marker(&client).await?.get("admission_open") != Some(&native(false)) {
            return Err(ModelError::Conflict("native maintenance closed admission"));
        }
        crate::control::check_pending_effects(&client).await
    }
    .await;
    if let Err(error) = result {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("failed maintenance session close", session.close().await);
        return lctx_model::domain::completion::complete(Err(error), completion);
    }
    session.transfer();
    Ok(client)
}

/// Exact-request recovery must be able to fence an unresolved original effect.
/// Unlike installation-wide drainage, pending effects are its input. The caller
/// still holds explicit root authority, closed admission and an authenticated
/// dead/handed-back host receipt; this does not reopen ordinary maintenance APIs.
pub async fn request_recovery_client(
    config: &RuntimeConfig,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    if config.authentication != AuthenticationScope::Root {
        return Err(ModelError::Conflict(
            "native request recovery installer authority",
        ));
    }
    let client = crate::installation::check_installation(config).await?;
    let session = crate::reader::SessionFinalizationGuard::new(client.clone());
    let result = async {
        if marker(&client).await?.get("admission_open") != Some(&native(false)) {
            return Err(ModelError::Conflict(
                "native request recovery closed admission",
            ));
        }
        Ok(client)
    }
    .await;
    match result {
        Ok(client) => {
            session.transfer();
            Ok(client)
        }
        Err(error) => {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "failed request recovery session invalidation",
                session.close().await,
            );
            lctx_model::domain::completion::complete(Err(error), completion)
        }
    }
}
