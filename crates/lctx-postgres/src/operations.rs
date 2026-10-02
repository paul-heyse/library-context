//! Append-only attempt observations. The Delta snapshot and generation discovery indexes were
//! removed with the Delta runtime (cutover plan P1.3); generations are listed by P1.8's catalog.

use super::{Error, Store};
use lctx_model::domain::ContentHash;
use serde::Serialize;

/// An operational attempt key; it never identifies a semantic row or generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AttemptId([u8; 16]);
impl AttemptId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub fn from_hex(text: &str) -> Option<Self> {
        if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0; 16];
        for (slot, pair) in bytes
            .iter_mut()
            .zip(text.as_bytes().as_chunks::<2>().0.iter())
        {
            *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
        }
        Some(Self(bytes))
    }
}

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

impl Store {
    pub async fn start_attempt(
        &self,
        id: AttemptId,
        compiler: ContentHash,
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

    pub async fn event(
        &self,
        id: AttemptId,
        key: &str,
        kind: &str,
        detail: &str,
    ) -> Result<(), Error> {
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

    pub async fn runs(
        &self,
        id: Option<AttemptId>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Run>, Error> {
        Ok(sqlx::query_as("SELECT encode(a.attempt_id,'hex') AS attempt,a.library,a.store_path,a.started_at::text,a.registration,a.observed_at::text, CASE WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='published') THEN 'published' WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='failed') THEN 'failed' WHEN EXISTS (SELECT FROM lctx_ops.events e WHERE e.attempt_id=a.attempt_id AND e.kind='interrupted') THEN 'interrupted' ELSE 'unfinished' END AS outcome FROM lctx_ops.attempts a WHERE ($1::bytea IS NULL OR a.attempt_id=$1) ORDER BY a.observed_at DESC,a.attempt_id LIMIT $2 OFFSET $3")
            .bind(id.map(|i| i.0.to_vec())).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }

    pub async fn events(
        &self,
        id: AttemptId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Event>, Error> {
        Ok(sqlx::query_as("SELECT event_key,kind,detail,recorded_at::text FROM lctx_ops.events WHERE attempt_id=$1 ORDER BY recorded_at,event_key LIMIT $2 OFFSET $3")
            .bind(id.0.as_slice()).bind(i64::from(limit.clamp(1,1000))).bind(i64::from(offset)).fetch_all(&self.pool).await?)
    }
}
