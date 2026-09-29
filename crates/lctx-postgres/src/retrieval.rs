//! Complete exact cosine ranks for capability briefs. Member ranks live in `selection`.
use crate::{
    Error,
    profiles::Route,
    repository::PinnedGeneration,
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
use sqlx::{PgConnection, Row};
use std::sync::Arc;

pub const RANK_BYTES: usize = 32 * 1024 * 1024;
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
        limit: u32,
    ) -> Result<RankResult, Error> {
        if generation.manifest.spec_hash.as_deref() != Some(spec) {
            return Err(Error::Request(
                "query embedding spec differs from the pinned generation".into(),
            ));
        }
        validate_query(query)?;
        if !(1..=100).contains(&limit) {
            return Err(Error::Request(
                "retrieval limit must be 1 through 100".into(),
            ));
        }
        // Exact filter ownership stays in the repository. This lease ends before vector work.
        let scope = self.search_scope(generation, "").await?;
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
        let rows = exact(&mut lease.connection, generation, query, &eligible).await?;
        let metadata = RankMetadata {
            profile: generation.profile.hex(),
            requested_route: Route::Exact,
            actual_route: Route::Exact,
            fallback: None,
            candidate_depth: 0,
            approximate: false,
            routing_reason: "complete_exact_ranks".into(),
            admission: None,
        };
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
async fn exact(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    query: &[f32],
    ids: &[Vec<u8>],
) -> Result<Vec<RankedEntity>, Error> {
    // Aggregate every eligible chunk before ordering; no ANN candidate truncation.
    let mut stream=sqlx::query("SELECT brief_id,1-min(vector OPERATOR(lctx_ext.<=>) $2) AS score FROM lctx_serving.vectors WHERE generation_digest=$1 AND brief_id=ANY($3) GROUP BY brief_id ORDER BY score DESC,brief_id")
        .bind(generation.id.0.as_slice()).bind(pgvector::Vector::from(query.to_vec())).bind(ids).fetch(&mut *conn);
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
            id: row.try_get("brief_id")?,
            view: "brief".into(),
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

/// Empty stream carries the authoritative brief rank transport schema to Python.
pub fn rank_schema_ipc() -> Result<Vec<u8>, Error> {
    encode(&[])
}
