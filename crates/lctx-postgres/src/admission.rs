//! Physical index admission is deployment state, never canonical generation identity.
use crate::{
    Error,
    profiles::{Route, hex},
    repository::PinnedGeneration,
};
use cpg_schema::Digest;
use serde_json::Value;
use sqlx::PgConnection;

pub const ENGINE: i64 = 2;

pub(crate) async fn realization(conn: &mut PgConnection, id: Digest) -> Result<Value, Error> {
    Ok(
        sqlx::query_scalar("SELECT lctx_serving.index_realization($1)")
            .bind(id.0.as_slice())
            .fetch_one(conn)
            .await?,
    )
}

pub(crate) async fn check(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
) -> Result<Option<String>, Error> {
    if generation.policy.route == Route::Exact {
        return Ok(None);
    }
    if generation.policy.route != Route::Mixed {
        return Err(Error::Admission(
            "legacy ANN policy has no physical admission",
        ));
    }
    inspect(conn, generation, true).await
}

// Qualification runs on the connection holding the exclusive generation lock. It pays
// the same physical/catalog lookup cost before admission exists, without requiring the row.
pub(crate) async fn inspect(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    required: bool,
) -> Result<Option<String>, Error> {
    let current = realization(conn, generation.id).await?;
    let admitted: Option<Vec<u8>> = sqlx::query_scalar("SELECT realization_digest FROM lctx_serving.profile_admissions WHERE generation_digest=$1 AND profile_digest=$2 AND realization=$3")
        .bind(generation.id.0.as_slice()).bind(generation.profile.0.as_slice()).bind(&current).fetch_optional(conn).await?;
    if (required && admitted.is_none()) || current["engine"] != ENGINE || current["valid"] != true {
        return Err(Error::Admission(
            "index admission missing or stale; select exact or requalify",
        ));
    }
    Ok(admitted.map(hex))
}

pub(crate) async fn universe(conn: &mut PgConnection, id: Digest) -> Result<u64, Error> {
    let n:i64=sqlx::query_scalar("SELECT count(DISTINCT node_id) FROM lctx_serving.operation_vectors WHERE generation_digest=$1").bind(id.0.as_slice()).fetch_one(conn).await?;
    Ok(n as u64)
}
pub(crate) async fn population(
    conn: &mut PgConnection,
    id: Digest,
    operations: bool,
    eligible: &[Vec<u8>],
    universe: Option<u64>,
) -> Result<(u64, u64), Error> {
    if operations && let Some(u) = universe {
        let e:i64=sqlx::query_scalar("SELECT count(DISTINCT node_id) FROM lctx_serving.operation_vectors WHERE generation_digest=$1 AND node_id=ANY($2)")
            .bind(id.0.as_slice()).bind(eligible).fetch_one(conn).await?;
        return Ok((e as u64, u));
    }
    let sql = if operations {
        "SELECT count(DISTINCT node_id) FILTER(WHERE node_id=ANY($2)),count(DISTINCT node_id) FROM lctx_serving.operation_vectors WHERE generation_digest=$1"
    } else {
        "SELECT count(DISTINCT brief_id) FILTER(WHERE brief_id=ANY($2)),count(DISTINCT brief_id) FROM lctx_serving.vectors WHERE generation_digest=$1"
    };
    let (e, u): (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(id.0.as_slice())
        .bind(eligible)
        .fetch_one(conn)
        .await?;
    Ok((e as u64, u as u64))
}
