//! Installing and resetting the generated store (cutover plan P1.6). Both run as the verified
//! service owner under the exclusive installation lock, taken only by trying, so no lifecycle
//! transition, pin or copy (which take it shared) overlaps them and none waits behind them.
use super::{
    CONTROL_RECORDS, Error, GenerationId, GenerationStore, ddl, locks, quoted, transaction,
};
use crate::OwnerPool;
use lctx_model::domain::{ContentHash, ValidatedModel};
use sqlx::PgConnection;
use std::sync::Arc;

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
pub(super) async fn control(
    tx: &mut PgConnection,
    model: &ValidatedModel,
    physical: ContentHash,
) -> Result<(), Error> {
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('lctx_model_store.installation') IS NOT NULL")
            .fetch_one(&mut *tx)
            .await?;
    if present {
        let existing: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation WHERE singleton FOR UPDATE")
            .fetch_optional(&mut *tx).await?;
        return match existing {
            Some((m, p)) if m == model.digest().0 && p == physical.0 => Ok(()),
            _ => Err(Error::Contract),
        };
    }
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl::control(ddl::CONTROL)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO lctx_model_store.installation VALUES (true,$1,$2)")
        .bind(model.digest().0.to_vec())
        .bind(physical.0.to_vec())
        .execute(&mut *tx)
        .await?;
    Ok(())
}

async fn inventory(tx: &mut PgConnection) -> Result<(ResetInventory, Vec<GenerationId>), Error> {
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&mut *tx)
        .await?;
    let control: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1 AND nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user))")
        .bind(ddl::CONTROL).fetch_one(&mut *tx).await?;
    let schemas: Vec<String> = sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user) \
        AND nspname ~ '^lctx_g[0-9a-f]{32}$' ORDER BY 1").fetch_all(&mut *tx).await?;
    let mut generations: Vec<GenerationId> = schemas
        .iter()
        .filter_map(|s| GenerationId::from_schema(s))
        .collect();
    let registry: bool =
        sqlx::query_scalar("SELECT to_regclass('lctx_model_store.generations') IS NOT NULL")
            .fetch_one(&mut *tx)
            .await?;
    if control && registry {
        let registered: Vec<Vec<u8>> =
            sqlx::query_scalar("SELECT id FROM lctx_model_store.generations")
                .fetch_all(&mut *tx)
                .await?;
        for id in registered {
            generations.push(GenerationId(
                id.try_into()
                    .map_err(|_| Error::Codec("generation id length".into()))?,
            ));
        }
    }
    generations.sort_by_key(|g| g.0);
    generations.dedup();
    Ok((
        ResetInventory {
            database,
            schemas,
            control,
        },
        generations,
    ))
}

pub(super) async fn plan(owner: &OwnerPool) -> Result<ResetInventory, Error> {
    transaction(owner.pool(), async |tx| {
        sqlx::query(locks::INSTALLATION_SHARED)
            .execute(&mut *tx)
            .await?;
        Ok(inventory(tx).await?.0)
    })
    .await
}

/// Reset in phases (store-lifecycle review F02). A generation's cascade takes a lock per
/// dependent object, about 2,500 for a full lowering, so dropping every generation in one
/// transaction exhausts PostgreSQL's default lock table beyond a handful. Instead:
/// 1. under the exclusive installation lock, confirm, refuse any lease, transition or live
///    attempt, and withdraw the installation row, so every other step refuses until the reset
///    finishes;
/// 2. remove one generation, its schema and its control records per transaction;
/// 3. replace the control schema and install `model`.
///
/// Each phase is idempotent, so a rerun finishes an interrupted reset.
pub(super) async fn reset(
    owner: OwnerPool,
    model: Arc<ValidatedModel>,
    confirm: &str,
) -> Result<(ResetInventory, GenerationStore), Error> {
    let store = GenerationStore::assemble(owner.pool().clone(), model);
    let pool = store.owner.clone();
    let (inventory, generations) = transaction(&pool, async |tx| {
        locks::installation_exclusive(tx).await?;
        let (inventory, generations) = inventory(tx).await?;
        if confirm != inventory.database {
            return Err(Error::Confirmation);
        }
        for g in &generations {
            if !locks::try_generation_exclusive(tx, *g).await? {
                return Err(Error::Busy);
            }
        }
        if registry(tx).await? {
            sqlx::query("DELETE FROM lctx_model_store.installation")
                .execute(&mut *tx)
                .await?;
        }
        Ok((inventory, generations))
    })
    .await?;
    for g in generations {
        transaction(&pool, async |tx| {
            locks::installation_exclusive(tx).await?;
            if !locks::try_generation_exclusive(tx, g).await? { return Err(Error::Busy); }
            let owned: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1 AND nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user))")
                .bind(g.schema()).fetch_one(&mut *tx).await?;
            if owned { sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(&g.schema())))).execute(&mut *tx).await?; }
            if registry(tx).await? {
                sqlx::query("UPDATE lctx_model_store.selection SET generation_id = NULL WHERE generation_id = $1").bind(g.0.to_vec()).execute(&mut *tx).await?;
                for table in CONTROL_RECORDS {
                    sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM lctx_model_store.{table} WHERE generation_id=$1"))).bind(g.0.to_vec()).execute(&mut *tx).await?;
                }
                sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=$1").bind(g.0.to_vec()).execute(&mut *tx).await?;
            }
            Ok(())
        }).await?;
    }
    transaction(&pool, async |tx| {
        locks::installation_exclusive(tx).await?;
        let control: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1 AND nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user))")
            .bind(ddl::CONTROL).fetch_one(&mut *tx).await?;
        if control { sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE", quoted(ddl::CONTROL)))).execute(&mut *tx).await?; }
        self::control(tx, &store.model, store.physical).await
    }).await?;
    Ok((inventory, store))
}

async fn registry(tx: &mut PgConnection) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar("SELECT to_regclass('lctx_model_store.generations') IS NOT NULL")
            .fetch_one(&mut *tx)
            .await?,
    )
}
