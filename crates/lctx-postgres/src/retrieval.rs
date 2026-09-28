//! PostgreSQL owns cosine evaluation; complete ranks and ANN candidates are distinct routes.
use crate::{
    Error,
    profiles::Route,
    repository::{PinnedGeneration, Where},
    serving::{QueryLease, ServingStore},
};
use arrow_array::{FixedSizeBinaryArray, Float64Array, RecordBatch, StringArray, UInt32Array};
use arrow_ipc::writer::StreamWriter;
use cpg_schema::{
    id::Id,
    serving_projection::{corrupt, refused},
};
use futures::TryStreamExt;
use serde::Serialize;
use sqlx::{Connection, PgConnection, Row};
use std::{collections::BTreeSet, sync::Arc};

pub const RANK_BYTES: usize = 32 * 1024 * 1024;
pub const RESCORE_ROWS: i64 = 200_000;
pub const RESCORE_BYTES: i64 = 128 * 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct RankMetadata {
    pub profile: String,
    pub requested_route: Route,
    pub actual_route: Route,
    pub fallback: Option<String>,
    pub candidate_depth: u32,
    pub approximate: bool,
    pub routing_reason: String,
    pub admission: Option<String>,
}
pub struct RankResult {
    pub metadata: RankMetadata,
    pub ipc: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct RankedEntity {
    pub id: Vec<u8>,
    pub view: String,
    pub score: f64,
    pub rank: u32,
}

impl ServingStore {
    pub async fn vector_ranks(
        &self,
        generation: &PinnedGeneration,
        query: &[f32],
        spec: &str,
        operations: bool,
        filter: Option<&Where>,
        limit: u32,
    ) -> Result<RankResult, Error> {
        if generation.manifest.spec_hash.as_deref() != Some(spec) {
            return Err(Error::Request(
                "query embedding spec differs from the pinned generation".into(),
            ));
        }
        validate_query(query)?;
        if !(1..=10).contains(&limit) {
            return Err(Error::Request(
                "retrieval limit must be 1 through 10".into(),
            ));
        }
        // Exact filter ownership stays in the repository. This lease ends before vector work.
        let scope = self
            .search_scope(generation, filter, "", operations)
            .await?;
        let eligible = scope["eligible"]
            .as_array()
            .ok_or_else(|| corrupt("eligibility result"))?
            .iter()
            .map(|id| {
                Id::from_hex(id.as_str().unwrap_or(""))
                    .map(|i| i.0.to_vec())
                    .ok_or_else(|| corrupt("eligible identity"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let (rows, metadata) = rank_on(
            &mut lease.connection,
            generation,
            query,
            operations,
            &eligible,
            limit,
            RankMode::Selected,
        )
        .await?;
        lease.complete();
        Ok(RankResult {
            metadata,
            ipc: encode(&rows)?,
        })
    }
}
pub(crate) fn validate_query(query: &[f32]) -> Result<(), Error> {
    let norm = query
        .iter()
        .map(|x| f64::from(*x).powi(2))
        .sum::<f64>()
        .sqrt();
    if query.len() != 1024 || !norm.is_finite() || (norm - 1.0).abs() > 0.001 {
        return Err(Error::Request(
            "query must contain 1024 finite normalized floats".into(),
        ));
    }
    Ok(())
}
pub(crate) enum RankMode {
    Selected,
    Qualification(Route),
}
pub(crate) async fn rank_on(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    query: &[f32],
    operations: bool,
    eligible: &[Vec<u8>],
    limit: u32,
    mode: RankMode,
) -> Result<(Vec<RankedEntity>, RankMetadata), Error> {
    let (route, enforce_admission) = match mode {
        RankMode::Selected => (generation.policy.route, true),
        RankMode::Qualification(route) => (route, false),
    };
    let mut metadata = RankMetadata {
        profile: generation.profile.hex(),
        requested_route: route,
        actual_route: route,
        fallback: None,
        candidate_depth: 0,
        approximate: route == Route::Hnsw,
        routing_reason: "explicit_profile".into(),
        admission: None,
    };
    let mut tx = conn.begin().await?;
    sqlx::raw_sql("SET TRANSACTION READ ONLY; SET LOCAL work_mem='8MB'; SET LOCAL statement_timeout='30s'; SET LOCAL hnsw.ef_search=100; SET LOCAL hnsw.iterative_scan='strict_order'; SET LOCAL hnsw.max_scan_tuples=20000; SET LOCAL hnsw.scan_mem_multiplier=2").execute(&mut *tx).await?;
    let route = if route == Route::Mixed {
        let (e, u) = if operations {
            crate::admission::population(
                &mut tx,
                generation.id,
                true,
                eligible,
                generation.vector_population,
            )
            .await?
        } else {
            (0, 0)
        };
        let (chosen, reason) = generation.policy.choose(operations, e, u);
        metadata.routing_reason = reason.into();
        chosen
    } else {
        route
    };
    metadata.actual_route = route;
    metadata.approximate = route == Route::Hnsw;
    if route == Route::Hnsw && (enforce_admission || generation.policy.route == Route::Mixed) {
        sqlx::query("SELECT pg_advisory_xact_lock_shared(lctx_serving.lock_key($1))")
            .bind(generation.id.0.as_slice())
            .execute(&mut *tx)
            .await?;
        metadata.admission = if enforce_admission {
            crate::admission::check(&mut tx, generation).await?
        } else {
            crate::admission::inspect(&mut tx, generation, false).await?
        };
    }
    let views: Vec<&str> = if operations {
        vec!["signature_doc", "source_body"]
    } else {
        vec!["brief"]
    };
    let mut candidate_union = BTreeSet::new();
    if route == Route::Hnsw {
        for view in &views {
            let available = entity_count(&mut tx, generation, operations, view, eligible).await?;
            let mut candidates = Vec::new();
            for depth in [200u32, 800, 3200] {
                candidates = nearest(
                    &mut tx, generation, query, operations, view, eligible, depth,
                )
                .await?;
                metadata.candidate_depth = metadata.candidate_depth.max(depth);
                if candidates.len() >= available.min(limit as i64) as usize {
                    break;
                }
            }
            if candidates.len() < available.min(limit as i64) as usize {
                metadata.actual_route = Route::Exact;
                metadata.approximate = false;
                metadata.fallback = Some("distinct-entity underfill; bounded exact ranks".into());
            }
            candidate_union.extend(candidates);
        }
    }
    let selected = if route == Route::Exact || metadata.fallback.is_some() {
        eligible.to_vec()
    } else {
        candidate_union.into_iter().collect()
    };
    // Candidate admission is per view; once admitted, every entity receives every available
    // view's best-chunk contribution. Truncating another view must not erase its RRF leg.
    if route == Route::Hnsw {
        let (table, key) = source(operations);
        let sql = format!(
            "SELECT count(*),coalesce(sum(pg_column_size(vector)),0)::bigint FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2)"
        );
        let (rows, bytes): (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(generation.id.0.as_slice())
            .bind(&selected)
            .fetch_one(&mut *tx)
            .await?;
        if rows > RESCORE_ROWS || bytes > RESCORE_BYTES {
            return Err(refused("aggregate candidate rescore/exact fallback budget").into());
        }
    }
    let mut all = Vec::new();
    for view in views {
        all.extend(
            exact(
                &mut tx,
                generation,
                query,
                operations,
                view,
                &selected,
                route == Route::Hnsw,
            )
            .await?,
        );
    }
    tx.commit().await?;
    Ok((all, metadata))
}
fn source(operations: bool) -> (&'static str, &'static str) {
    if operations {
        ("operation_vectors", "node_id")
    } else {
        ("vectors", "brief_id")
    }
}
fn view_predicate(operations: bool) -> &'static str {
    if operations {
        "AND embedding_view=$4"
    } else {
        "AND $4::text='brief'"
    }
}
async fn entity_count(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    operations: bool,
    view: &str,
    ids: &[Vec<u8>],
) -> Result<i64, Error> {
    let (table, key) = source(operations);
    let sql = format!(
        "SELECT count(DISTINCT {key}) FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2) AND ($3::text='brief' OR embedding_view=$3)"
    );
    let sql = if operations {
        sql
    } else {
        format!(
            "SELECT count(DISTINCT {key}) FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2) AND $3::text='brief'"
        )
    };
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(generation.id.0.as_slice())
        .bind(ids)
        .bind(view)
        .fetch_one(conn)
        .await?)
}
async fn nearest(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    query: &[f32],
    operations: bool,
    view: &str,
    ids: &[Vec<u8>],
    depth: u32,
) -> Result<Vec<Vec<u8>>, Error> {
    let (table, key) = source(operations);
    let sql = format!(
        "SELECT {key} FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($3) {} ORDER BY vector OPERATOR(lctx_ext.<=>) $2 LIMIT $5",
        view_predicate(operations)
    );
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(generation.id.0.as_slice())
        .bind(pgvector::Vector::from(query.to_vec()))
        .bind(ids)
        .bind(view)
        .bind(i64::from(depth))
        .fetch_all(conn)
        .await?;
    Ok(rows
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}
pub(crate) async fn exact(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    query: &[f32],
    operations: bool,
    view: &str,
    ids: &[Vec<u8>],
    bounded_rescore: bool,
) -> Result<Vec<RankedEntity>, Error> {
    let (table, key) = source(operations);
    if bounded_rescore {
        let sql = format!(
            "SELECT count(*),coalesce(sum(pg_column_size(vector)),0)::bigint FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2) {}",
            if operations {
                "AND embedding_view=$3"
            } else {
                "AND $3::text='brief'"
            }
        );
        let (rows, bytes): (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(generation.id.0.as_slice())
            .bind(ids)
            .bind(view)
            .fetch_one(&mut *conn)
            .await?;
        if rows > RESCORE_ROWS || bytes > RESCORE_BYTES {
            return Err(refused("candidate rescore/exact fallback budget").into());
        }
    }
    // Group every eligible chunk before ordering. No vector ORDER BY/LIMIT can invoke HNSW here.
    let sql = format!(
        "SELECT {key},1-min(vector OPERATOR(lctx_ext.<=>) $2) AS score FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($3) {} GROUP BY {key} ORDER BY score DESC,{key}",
        view_predicate(operations)
    );
    let mut stream = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(generation.id.0.as_slice())
        .bind(pgvector::Vector::from(query.to_vec()))
        .bind(ids)
        .bind(view)
        .fetch(&mut *conn);
    let mut out = Vec::new();
    while let Some(row) = stream.try_next().await? {
        if out.len() >= 200_000 {
            return Err(refused("rank row budget").into());
        }
        let score: f64 = row.try_get("score")?;
        if !score.is_finite() {
            return Err(corrupt("non-finite cosine").into());
        }
        out.push(RankedEntity {
            id: row.try_get(key)?,
            view: view.into(),
            score,
            rank: (out.len() + 1) as u32,
        });
    }
    Ok(out)
}
pub(crate) fn encode(rows: &[RankedEntity]) -> Result<Vec<u8>, Error> {
    let schema = cpg_schema::serving_projection::rank_schema();
    let mut bytes = Vec::new();
    {
        let mut writer =
            StreamWriter::try_new(&mut bytes, &schema).map_err(|_| corrupt("rank IPC schema"))?;
        for part in rows.chunks(1000) {
            let ids = FixedSizeBinaryArray::try_from_iter(part.iter().map(|r| r.id.as_slice()))
                .map_err(|_| corrupt("rank identity width"))?;
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(ids),
                    Arc::new(StringArray::from_iter_values(
                        part.iter().map(|r| r.view.as_str()),
                    )),
                    Arc::new(UInt32Array::from_iter_values(part.iter().map(|r| r.rank))),
                    Arc::new(Float64Array::from_iter_values(part.iter().map(|r| r.score))),
                ],
            )
            .map_err(|_| corrupt("rank batch"))?;
            writer.write(&batch).map_err(|_| corrupt("rank IPC"))?;
        }
        writer.finish().map_err(|_| corrupt("rank IPC finish"))?;
    }
    if bytes.len() > RANK_BYTES {
        return Err(refused("rank IPC byte budget").into());
    }
    Ok(bytes)
}
