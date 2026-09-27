//! Append-only attempt observations and reconstructible discovery indexes.

use super::{Error, Store};
use cpg_schema::id::{Digest, Id};
use serde::Serialize;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Run {
    pub attempt: String,
    pub library: Option<String>,
    pub store_path: String,
    pub started_at: Option<String>,
    pub registration: String,
    pub observed_at: String,
    pub outcome: String,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Event {
    pub event_key: String,
    pub kind: String,
    pub detail: String,
    pub recorded_at: String,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Snapshot {
    pub snapshot: String,
    pub content_digest: String,
    pub compiler_digest: String,
    pub store_path: String,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Generation {
    pub store_path: String,
    pub generation_key: String,
    pub snapshot: String,
    pub location: String,
    pub manifest_digest: String,
}

impl Store {
    pub async fn start_attempt(
        &self,
        id: Id,
        compiler: Digest,
        library: &str,
        store: &str,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO lctx_ops.attempts(attempt_id,compiler_digest,library,store_path) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(id.0.as_slice()).bind(compiler.0.as_slice()).bind(library).bind(store).execute(&self.pool).await?;
        let existing: (Vec<u8>, String, String) = sqlx::query_as(
            "SELECT compiler_digest,library,store_path FROM lctx_ops.attempts WHERE attempt_id=$1",
        )
        .bind(id.0.as_slice())
        .fetch_one(&self.pool)
        .await?;
        if existing != (compiler.0.to_vec(), library.to_owned(), store.to_owned()) {
            return Err(Error::Integrity(
                "attempt identity reused with different inputs",
            ));
        }
        self.event(id, "started", "started", "").await
    }

    pub async fn event(&self, id: Id, key: &str, kind: &str, detail: &str) -> Result<(), Error> {
        sqlx::query("INSERT INTO lctx_ops.events(attempt_id,event_key,kind,detail) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(id.0.as_slice()).bind(key).bind(kind).bind(detail).execute(&self.pool).await?;
        let prior: (String, String) = sqlx::query_as(
            "SELECT kind,detail FROM lctx_ops.events WHERE attempt_id=$1 AND event_key=$2",
        )
        .bind(id.0.as_slice())
        .bind(key)
        .fetch_one(&self.pool)
        .await?;
        if prior != (kind.to_owned(), detail.to_owned()) {
            return Err(Error::Integrity("event idempotency key conflict"));
        }
        Ok(())
    }

    pub async fn runs(&self, id: Option<Id>, limit: u32, offset: u32) -> Result<Vec<Run>, Error> {
        Ok(sqlx::query_as("SELECT encode(a.attempt_id,'hex') AS attempt,a.library,a.store_path,a.started_at::text,a.registration,a.observed_at::text, CASE WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='published') THEN 'published' WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='failed') THEN 'failed' WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='interrupted') THEN 'interrupted' ELSE 'unfinished' END AS outcome FROM lctx_ops.attempts a WHERE ($1::bytea IS NULL OR a.attempt_id=$1) ORDER BY a.observed_at DESC,a.attempt_id LIMIT $2 OFFSET $3")
            .bind(id.map(|i| i.0.to_vec())).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }

    pub async fn events(&self, id: Id, limit: u32, offset: u32) -> Result<Vec<Event>, Error> {
        Ok(sqlx::query_as("SELECT event_key,kind,detail,recorded_at::text FROM lctx_ops.events WHERE attempt_id=$1 ORDER BY recorded_at,event_key LIMIT $2 OFFSET $3")
            .bind(id.0.as_slice()).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }

    /// Explicit observation from verified canonical metadata; never reconstructs missing history.
    pub async fn observe_publication(
        &self,
        store: &str,
        id: Id,
        content: Digest,
        compiler: Digest,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO lctx_ops.attempts(attempt_id,compiler_digest,library,store_path,started_at,registration) VALUES($1,$2,NULL,$3,NULL,'reconciled') ON CONFLICT DO NOTHING")
            .bind(id.0.as_slice()).bind(compiler.0.as_slice()).bind(store).execute(&self.pool).await?;
        let prior: (Vec<u8>, String) = sqlx::query_as(
            "SELECT compiler_digest,store_path FROM lctx_ops.attempts WHERE attempt_id=$1",
        )
        .bind(id.0.as_slice())
        .fetch_one(&self.pool)
        .await?;
        // A moved store is a new discovery location, not a changed historical attempt location.
        if prior.0 != compiler.0 {
            return Err(Error::Integrity(
                "publication compiler conflicts with registered attempt",
            ));
        }
        self.event(
            id,
            "reconcile/published",
            "published",
            &format!("canonical publication observed: {}", content.hex()),
        )
        .await?;
        self.record_snapshot(store, id, content, compiler).await
    }

    /// Capture before enumerating canonical storage. Concurrent newer discoveries are protected.
    pub async fn reconciliation_started(&self) -> Result<String, Error> {
        Ok(sqlx::query_scalar("SELECT clock_timestamp()::text")
            .fetch_one(&self.pool)
            .await?)
    }

    /// Mark missing rebuildable entries unavailable; durable attempts/events are untouched.
    pub async fn finish_reconciliation(
        &self,
        store: &str,
        generations_root: &str,
        before: &str,
        snapshots: &[Id],
        locations: &[String],
    ) -> Result<(), Error> {
        let ids: Vec<Vec<u8>> = snapshots.iter().map(|id| id.0.to_vec()).collect();
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE lctx_ops.snapshots SET available=false WHERE store_path=$1 AND reconciled_at < $2::text::timestamptz AND NOT(snapshot_id=ANY($3))")
            .bind(store).bind(before).bind(ids).execute(&mut *tx).await?;
        sqlx::query("UPDATE lctx_ops.generations SET available=false WHERE (store_path=$1 OR store_path IS NULL) AND left(location,length($2)+1)=$2 || '/' AND reconciled_at < $3::text::timestamptz AND NOT(location=ANY($4))")
            .bind(store).bind(generations_root.trim_end_matches('/')).bind(before).bind(locations).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn record_snapshot(
        &self,
        store: &str,
        id: Id,
        content: Digest,
        compiler: Digest,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO lctx_ops.snapshots(store_path,snapshot_id,content_digest,compiler_digest) VALUES($1,$2,$3,$4) ON CONFLICT(store_path,snapshot_id) DO UPDATE SET content_digest=EXCLUDED.content_digest,compiler_digest=EXCLUDED.compiler_digest,reconciled_at=clock_timestamp(),available=true")
            .bind(store).bind(id.0.as_slice()).bind(content.0.as_slice()).bind(compiler.0.as_slice()).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn record_generation(
        &self,
        location: &str,
        store: &str,
        key: &str,
        snapshot: Id,
        manifest: Digest,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO lctx_ops.generations(location,generation_key,snapshot_id,manifest_digest,store_path,available) VALUES($1,$2,$3,$4,$5,true) ON CONFLICT(location) DO UPDATE SET generation_key=EXCLUDED.generation_key,snapshot_id=EXCLUDED.snapshot_id,manifest_digest=EXCLUDED.manifest_digest,store_path=EXCLUDED.store_path,available=true,reconciled_at=clock_timestamp()")
            .bind(location).bind(key).bind(snapshot.0.as_slice()).bind(manifest.0.as_slice()).bind(store).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn snapshots(
        &self,
        id: Option<Id>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Snapshot>, Error> {
        Ok(sqlx::query_as("SELECT encode(snapshot_id,'hex') AS snapshot,encode(content_digest,'hex') AS content_digest,encode(compiler_digest,'hex') AS compiler_digest,store_path FROM lctx_ops.snapshots WHERE available AND ($1::bytea IS NULL OR snapshot_id=$1) ORDER BY store_path,snapshot_id LIMIT $2 OFFSET $3")
            .bind(id.map(|i| i.0.to_vec())).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }

    pub async fn generations(
        &self,
        key: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Generation>, Error> {
        Ok(sqlx::query_as("SELECT store_path,generation_key,encode(snapshot_id,'hex') AS snapshot,location,encode(manifest_digest,'hex') AS manifest_digest FROM lctx_ops.generations WHERE available AND ($1::text IS NULL OR generation_key=$1) ORDER BY generation_key,location LIMIT $2 OFFSET $3")
            .bind(key).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }
}
