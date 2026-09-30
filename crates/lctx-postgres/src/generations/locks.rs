//! The store's advisory locks (store-lifecycle review F07). Lock order is installation →
//! selection row → generation → attempt → relations. Every lifecycle step, copy, pin, lease and
//! cleanup takes the installation lock shared; install, `check` and `reset` take it exclusively,
//! and only by trying, so they never queue ahead of the store's work (review F03). A lease holds
//! the generation lock shared for its lifetime; a transition holds it exclusively; an attempt's
//! lifecycle connection holds its attempt lock exclusively until the attempt ends.
use super::{Error, GenerationId};
use sqlx::PgConnection;
use std::time::Duration;

/// Take the installation lock shared for this transaction.
pub(crate) const INSTALLATION_SHARED: &str = "SELECT pg_advisory_xact_lock_shared(1279476824,0)";
/// The same, as one text-valued lease statement.
pub(super) const INSTALLATION_SHARED_TEXT: &str =
    "SELECT 'locked' FROM (SELECT pg_advisory_xact_lock_shared(1279476824,0)) AS l";
const INSTALLATION_TRY_EXCLUSIVE: &str = "SELECT pg_try_advisory_xact_lock(1279476824,0)";
/// How long an exclusive installation request retries before refusing with `Busy`.
const EXCLUSIVE_PATIENCE: Duration = Duration::from_secs(1);

impl GenerationId {
    /// The generation lock's key.
    pub(super) fn lock(self) -> i64 {
        i64::from_le_bytes(self.0[..8].try_into().expect("eight bytes"))
    }
    /// The attempt lock's key.
    pub(super) fn attempt_lock(self) -> i64 {
        i64::from_le_bytes(self.0[8..].try_into().expect("eight bytes"))
    }
}

/// Take the installation lock exclusively for this transaction, or refuse with `Busy` within
/// about a second. Waiting in the lock queue would stall every pin, copy and step behind it.
pub(super) async fn installation_exclusive(connection: &mut PgConnection) -> Result<(), Error> {
    let started = std::time::Instant::now();
    loop {
        let acquired: bool = sqlx::query_scalar(INSTALLATION_TRY_EXCLUSIVE)
            .fetch_one(&mut *connection)
            .await?;
        if acquired {
            return Ok(());
        }
        if started.elapsed() >= EXCLUSIVE_PATIENCE {
            return Err(Error::Busy);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Try the generation and attempt locks of `g` exclusively for this transaction: a lease, a
/// transition or a live attempt keeps it.
pub(super) async fn try_generation_exclusive(
    connection: &mut PgConnection,
    g: GenerationId,
) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar(
            "SELECT pg_try_advisory_xact_lock($1) AND pg_try_advisory_xact_lock($2)",
        )
        .bind(g.lock())
        .bind(g.attempt_lock())
        .fetch_one(connection)
        .await?,
    )
}

/// Distinct sessions holding a bigint advisory key in `mode` (`ShareLock` or `ExclusiveLock`),
/// read from `pg_locks` without taking anything: `classid` is the key's high half, `objid` its low.
pub(super) const HOLDERS: &str = "SELECT count(DISTINCT pid) FROM pg_locks WHERE locktype = 'advisory' AND granted AND mode = $3 \
    AND objsubid = 1 AND classid::bigint = $1 AND objid::bigint = $2";
pub(super) fn halves(key: i64) -> (i64, i64) {
    ((key as u64 >> 32) as i64, (key as u64 & 0xffff_ffff) as i64)
}

/// Whether an attempt holds `g`'s attempt lock.
pub(super) async fn attempt_live<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    g: GenerationId,
) -> Result<bool, Error> {
    let (high, low) = halves(g.attempt_lock());
    let holders: i64 = sqlx::query_scalar(HOLDERS)
        .bind(high)
        .bind(low)
        .bind("ExclusiveLock")
        .fetch_one(executor)
        .await?;
    Ok(holders > 0)
}
/// How many leases hold `g`.
pub(super) async fn readers<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    g: GenerationId,
) -> Result<i64, Error> {
    let (high, low) = halves(g.lock());
    Ok(sqlx::query_scalar(HOLDERS)
        .bind(high)
        .bind(low)
        .bind("ShareLock")
        .fetch_one(executor)
        .await?)
}
