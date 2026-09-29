//! One coherent capture of mutable operational state, released before federated queries.
use crate::{
    Error, projection,
    serving::{QueryLease, ServingStore},
};
use arrow_array::RecordBatch;
use cpg_schema::{
    id::{Digest, Id},
    postgres_report::View,
    serving_projection::refused,
};
use futures::TryStreamExt;
use sqlx::{Connection, Row, ValueRef};
use std::collections::BTreeMap;
pub struct Capture {
    pub snapshot: String,
    pub captured_at: String,
    pub tables: BTreeMap<&'static str, Vec<RecordBatch>>,
    pub rows: usize,
    pub bytes: usize,
}
impl ServingStore {
    pub async fn capture_report(
        &self,
        store: &str,
        snapshot: Id,
        generation: Digest,
        compiler: Digest,
    ) -> Result<Capture, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut tx = lease.connection.begin().await?;
        sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL statement_timeout='30s'; SET LOCAL transaction_timeout='30s'; SET LOCAL idle_in_transaction_session_timeout='30s'").execute(&mut *tx).await?;
        let (snapshot_token, captured_at): (String, String) =
            sqlx::query_as("SELECT pg_current_snapshot()::text,clock_timestamp()::text")
                .fetch_one(&mut *tx)
                .await?;
        let mut capture = Capture {
            snapshot: snapshot_token,
            captured_at,
            tables: BTreeMap::new(),
            rows: 0,
            bytes: 0,
        };
        for view in View::MUTABLE {
            let sql = match view {
                View::Events => {
                    "SELECT a.attempt_id,a.compiler_digest,a.library,(extract(epoch FROM a.started_at)*1000000)::bigint started_at_us,e.event_key,e.kind,e.detail,(extract(epoch FROM e.recorded_at)*1000000)::bigint recorded_at_us FROM lctx_ops.attempts a LEFT JOIN lctx_ops.events e USING(attempt_id) WHERE a.store_path=$1 AND a.compiler_digest=$4 AND $2::bytea IS NOT NULL AND $3::bytea IS NOT NULL ORDER BY a.attempt_id,e.recorded_at,e.event_key LIMIT 100001"
                }
                View::Publications => {
                    "SELECT snapshot_id,content_digest,compiler_digest,available FROM lctx_ops.snapshots WHERE store_path=$1 AND snapshot_id=$2 AND $3::bytea IS NOT NULL AND compiler_digest=$4 LIMIT 100001"
                }
                View::Imports => {
                    "SELECT generation_digest,attempt_id,outcome,(extract(epoch FROM started_at)*1000000)::bigint started_at_us,(extract(epoch FROM finished_at)*1000000)::bigint finished_at_us,failure_code FROM lctx_serving.import_attempts WHERE generation_digest=$3 AND $1::text IS NOT NULL AND $2::bytea IS NOT NULL AND $4::bytea IS NOT NULL ORDER BY attempt_id LIMIT 100001"
                }
                View::ProjectionState => {
                    "SELECT * FROM lctx_report.projection_state($3) WHERE $1::text IS NOT NULL AND $2::bytea IS NOT NULL AND $4::bytea IS NOT NULL"
                }
                View::Profiles => {
                    "SELECT generation_digest,profile_digest,true ready,policy::text,qualification::text FROM lctx_serving.retrieval_profiles WHERE generation_digest=$3 AND $1::text IS NOT NULL AND $2::bytea IS NOT NULL AND $4::bytea IS NOT NULL LIMIT 100001"
                }
                View::Selections => {
                    "SELECT library,generation_digest,profile_digest FROM lctx_serving.selections WHERE library=(SELECT library FROM lctx_report.projection_state($3)) AND $1::text IS NOT NULL AND $2::bytea IS NOT NULL AND $4::bytea IS NOT NULL"
                }
                _ => unreachable!("finite mutable inventory"),
            };
            let mut stream = sqlx::query(sql)
                .bind(store)
                .bind(snapshot.0.as_slice())
                .bind(generation.0.as_slice())
                .bind(compiler.0.as_slice())
                .fetch(&mut *tx);
            let mut rows = Vec::new();
            let mut batches = Vec::new();
            while let Some(row) = stream.try_next().await? {
                capture.rows += 1;
                capture.bytes += 256 + row.columns().len() * 64;
                for i in 0..row.columns().len() {
                    let value = row.try_get_raw(i)?;
                    if !value.is_null() {
                        capture.bytes += value
                            .as_bytes()
                            .map_err(|_| Error::Integrity("report row transport"))?
                            .len()
                            * 2;
                    }
                }
                if capture.rows > 100000 || capture.bytes > 64 * 1024 * 1024 {
                    return Err(refused("coherent report capture row/byte budget").into());
                }
                rows.push(row);
                if rows.len() == 1000 {
                    batches.push(projection::decode_schema(view.schema(), &rows)?);
                    rows.clear();
                }
            }
            if !rows.is_empty() {
                batches.push(projection::decode_schema(view.schema(), &rows)?);
            }
            if batches.is_empty() {
                batches.push(RecordBatch::new_empty(view.schema()));
            }
            capture.tables.insert(view.name(), batches);
        }
        tx.commit().await?;
        lease.complete();
        Ok(capture)
    }
}
