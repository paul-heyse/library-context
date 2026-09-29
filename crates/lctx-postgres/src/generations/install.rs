//! Installing and resetting the generated store (cutover plan P1.6). Both run as the verified
//! service owner under the exclusive installation lock, so no lifecycle transition, pin or copy
//! (which take it shared) overlaps them.
use std::sync::Arc;
use lctx_model::domain::{ContentHash, ValidatedModel};
use sqlx::PgConnection;
use super::{Error, GenerationId, GenerationStore, ddl, quoted, transaction};
use crate::OwnerPool;

const INSTALLATION_LOCK: &str = "SELECT pg_advisory_xact_lock(1279476824,0)";

/// The objects a reset drops: owner-owned generation schemas (registered or orphaned) and the
/// control schema. Services, unrelated schemas and objects of other roles are never inventoried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetInventory {
    pub database: String,
    pub schemas: Vec<String>,
    pub control: bool,
}

/// Create the control schema for `model`, or confirm an installation of the same model and
/// lowering. Any other installation is a contract mismatch.
pub(super) async fn control(tx: &mut PgConnection, model: &ValidatedModel, physical: ContentHash) -> Result<(), Error> {
    let present: bool = sqlx::query_scalar("SELECT to_regclass('lctx_model_store.installation') IS NOT NULL").fetch_one(&mut *tx).await?;
    if present {
        let existing: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation WHERE singleton FOR UPDATE")
            .fetch_optional(&mut *tx).await?;
        return match existing {
            Some((m, p)) if m == model.digest().0 && p == physical.0 => Ok(()),
            _ => Err(Error::Contract),
        };
    }
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl::control(ddl::CONTROL))).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO lctx_model_store.installation VALUES (true,$1,$2)")
        .bind(model.digest().0.to_vec()).bind(physical.0.to_vec()).execute(&mut *tx).await?;
    Ok(())
}

async fn inventory(tx: &mut PgConnection) -> Result<(ResetInventory, Vec<GenerationId>), Error> {
    let database: String = sqlx::query_scalar("SELECT current_database()::text").fetch_one(&mut *tx).await?;
    let control: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1 AND nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user))")
        .bind(ddl::CONTROL).fetch_one(&mut *tx).await?;
    let schemas: Vec<String> = sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user) \
        AND nspname ~ '^lctx_g[0-9a-f]{32}$' ORDER BY 1").fetch_all(&mut *tx).await?;
    let mut generations: Vec<GenerationId> = schemas.iter().filter_map(|s| GenerationId::from_schema(s)).collect();
    let registry: bool = sqlx::query_scalar("SELECT to_regclass('lctx_model_store.generations') IS NOT NULL").fetch_one(&mut *tx).await?;
    if control && registry {
        let registered: Vec<Vec<u8>> = sqlx::query_scalar("SELECT id FROM lctx_model_store.generations").fetch_all(&mut *tx).await?;
        for id in registered {
            generations.push(GenerationId(id.try_into().map_err(|_| Error::Codec("generation id length".into()))?));
        }
    }
    generations.sort_by_key(|g| g.0);
    generations.dedup();
    Ok((ResetInventory { database, schemas, control }, generations))
}

pub(super) async fn plan(owner: &OwnerPool) -> Result<ResetInventory, Error> {
    transaction(owner.pool(), async |tx| {
        sqlx::query("SELECT pg_advisory_xact_lock_shared(1279476824,0)").execute(&mut *tx).await?;
        Ok(inventory(tx).await?.0)
    }).await
}

pub(super) async fn reset(owner: OwnerPool, model: Arc<ValidatedModel>, confirm: &str) -> Result<(ResetInventory, GenerationStore), Error> {
    let pool = owner.pool().clone();
    let physical = ddl::physical_digest(&model);
    let inventory = transaction(&pool, async |tx| {
        sqlx::query(INSTALLATION_LOCK).execute(&mut *tx).await?;
        let (inventory, generations) = inventory(tx).await?;
        if confirm != inventory.database { return Err(Error::Confirmation); }
        // A lease holds its generation lock for the reader's lifetime; a lifecycle transaction
        // holds it for the transition. Either keeps the store.
        for g in &generations {
            let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)").bind(g.lock()).fetch_one(&mut *tx).await?;
            if !acquired { return Err(Error::Busy); }
        }
        for schema in &inventory.schemas {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(schema)))).execute(&mut *tx).await?;
        }
        if inventory.control {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(ddl::CONTROL)))).execute(&mut *tx).await?;
        }
        control(tx, &model, physical).await?;
        Ok(inventory)
    }).await?;
    Ok((inventory, GenerationStore { owner: pool, model, physical }))
}
