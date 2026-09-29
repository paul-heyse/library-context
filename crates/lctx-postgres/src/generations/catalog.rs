//! The generation catalog (cutover plan P1.8): what `lctx generation list|show` reports. It only
//! reads the control schema and the server's lock table, so the reader role can run it; it never
//! takes a lock, changes a registry row or reads a generation schema.
use lctx_model::domain::{ContentHash, admission::Frontier, stages::Profile};
use sqlx::{PgPool, Row, postgres::PgRow};
use super::{Error, GenerationId, ddl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenerationState { Staging, Sealed, Validated, Published, Failed }
impl GenerationState {
    pub fn name(self) -> &'static str {
        match self { Self::Staging => "staging", Self::Sealed => "sealed", Self::Validated => "validated", Self::Published => "published", Self::Failed => ddl::FAILED }
    }
    fn parse(name: &str) -> Result<Self, Error> {
        [Self::Staging, Self::Sealed, Self::Validated, Self::Published, Self::Failed].into_iter().find(|s| s.name() == name).ok_or(Error::Contract)
    }
}
/// Whether an attempt still owns an unpublished generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writer {
    /// A manual conformance generation: no attempt ever owned it.
    Manual,
    /// Its attempt holds the attempt lock.
    Live,
    /// Its attempt is gone before publication or failure; it can only be aborted.
    Interrupted,
    /// Its attempt ended: the generation is published or failed.
    Ended,
}
#[derive(Debug, Clone, Default)]
pub struct ListFilter { pub state: Option<GenerationState>, pub frontier: Option<Frontier> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationSummary {
    pub id: GenerationId, pub state: GenerationState, pub frontier: Frontier, pub profile: Profile, pub selected: bool,
    pub created_at: String, pub readers: u32, pub writer: Writer,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationDetail {
    pub summary: GenerationSummary,
    pub model: ContentHash, pub physical: ContentHash, pub producer: ContentHash,
    pub schedule: Option<ContentHash>, pub content: Option<ContentHash>,
    /// Receipted relations and their row counts, once validated.
    pub relations: Vec<(String, i64)>,
    /// A facts generation's admission: its contract digest and each family's availability.
    pub admission: Option<(ContentHash, Vec<(String, String)>)>,
    /// A failed generation's from-state, class and detail.
    pub failure: Option<(String, String, String)>,
}

pub struct GenerationCatalog { pool: PgPool }

const SUMMARY: &str = "SELECT g.id, g.state, g.frontier, g.profile, g.owned, g.created_at::text AS created_at, \
    s.generation_id IS NOT NULL AS selected FROM lctx_model_store.generations g \
    LEFT JOIN lctx_model_store.selection s ON s.generation_id = g.id";

impl GenerationCatalog {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    pub async fn list(&self, filter: &ListFilter) -> Result<Vec<GenerationSummary>, Error> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{SUMMARY} WHERE ($1::text IS NULL OR g.state = $1) AND ($2::text IS NULL OR g.frontier = $2) \
            ORDER BY g.created_at, g.id"))).bind(filter.state.map(GenerationState::name)).bind(filter.frontier.map(Frontier::name))
            .fetch_all(&self.pool).await?;
        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows { summaries.push(self.summary(&row).await?); }
        Ok(summaries)
    }
    pub async fn show(&self, g: GenerationId) -> Result<Option<GenerationDetail>, Error> {
        let Some(row) = sqlx::query(sqlx::AssertSqlSafe(format!("{SUMMARY} WHERE g.id = $1"))).bind(g.0.to_vec()).fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let summary = self.summary(&row).await?;
        let digest = |bytes: Vec<u8>| bytes.try_into().map(ContentHash).map_err(|_| Error::Codec("digest length".into()));
        let (model, physical, producer, schedule, content): (Vec<u8>, Vec<u8>, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT model_digest, physical_digest, producer_digest, schedule_digest, content_digest FROM lctx_model_store.generations WHERE id = $1")
            .bind(g.0.to_vec()).fetch_one(&self.pool).await?;
        let relations = sqlx::query_as("SELECT relation_name, row_count FROM lctx_model_store.receipts WHERE generation_id = $1 ORDER BY relation_name COLLATE \"C\"")
            .bind(g.0.to_vec()).fetch_all(&self.pool).await?;
        let admission: Option<(Vec<u8>, String)> = sqlx::query_as("SELECT contract_digest, availability FROM lctx_model_store.admissions WHERE generation_id = $1")
            .bind(g.0.to_vec()).fetch_optional(&self.pool).await?;
        let admission = admission.map(|(contract, availability)| -> Result<_, Error> {
            let families = availability.split(';').filter(|p| !p.is_empty())
                .map(|part| part.split_once('=').map(|(f, a)| (f.to_owned(), a.to_owned())).ok_or(Error::Contract)).collect::<Result<_, _>>()?;
            Ok((digest(contract)?, families))
        }).transpose()?;
        let failure = sqlx::query_as("SELECT from_state, class, detail FROM lctx_model_store.failures WHERE generation_id = $1")
            .bind(g.0.to_vec()).fetch_optional(&self.pool).await?;
        Ok(Some(GenerationDetail { summary, model: digest(model)?, physical: digest(physical)?, producer: digest(producer)?,
            schedule: schedule.map(digest).transpose()?, content: content.map(digest).transpose()?, relations, admission, failure }))
    }
    async fn summary(&self, row: &PgRow) -> Result<GenerationSummary, Error> {
        let id = GenerationId(row.try_get::<Vec<u8>, _>("id")?.try_into().map_err(|_| Error::Codec("generation id length".into()))?);
        let state = GenerationState::parse(&row.try_get::<String, _>("state")?)?;
        let frontier = ddl::frontier(&row.try_get::<String, _>("frontier")?).ok_or(Error::Contract)?;
        let profile = Profile::ALL.into_iter().find(|p| p.name() == row.try_get::<String, _>("profile").unwrap_or_default()).ok_or(Error::Contract)?;
        // A lease holds the generation lock shared for its lifetime; an attempt holds its attempt
        // lock exclusively. Both are bigint advisory locks: classid is the high half, objid the low.
        let holders = |key: i64, mode: &'static str| sqlx::query_scalar::<_, i64>("SELECT count(DISTINCT pid) FROM pg_locks WHERE locktype = 'advisory' \
            AND granted AND mode = $3 AND objsubid = 1 AND classid::bigint = $1 AND objid::bigint = $2")
            .bind((key as u64 >> 32) as i64).bind((key as u64 & 0xffff_ffff) as i64).bind(mode);
        let readers = holders(id.lock(), "ShareLock").fetch_one(&self.pool).await?;
        let owned: bool = row.try_get("owned")?;
        let writer = if !owned { Writer::Manual }
            else if matches!(state, GenerationState::Published | GenerationState::Failed) { Writer::Ended }
            else if holders(id.attempt_lock(), "ExclusiveLock").fetch_one(&self.pool).await? > 0 { Writer::Live }
            else { Writer::Interrupted };
        Ok(GenerationSummary { id, state, frontier, profile, selected: row.try_get("selected")?, created_at: row.try_get("created_at")?,
            readers: u32::try_from(readers).map_err(|_| Error::Codec("reader count".into()))?, writer })
    }
}
