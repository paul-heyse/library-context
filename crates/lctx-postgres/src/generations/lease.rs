//! The lease protocol, independent of the driver that runs it (cutover plan P1.10, T13). A lease
//! is a session-level shared lock on one published generation, taken inside one transaction after
//! checking, under the shared installation lock and the shared generation lock, that the
//! installation, the generation's state and its frontier-scoped model and physical digests are
//! this binary's. `pin` runs it through SQLx; provider sessions run it through tokio-postgres on
//! every connection they bind. Neither ever reconnects to take a lease again.
use super::{Error, GenerationId, ddl};
use lctx_model::domain::{ContentHash, Infrastructure, ValidatedModel, admission::Frontier};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// A statement parameter: generation identities and digests are bytes, lock keys integers.
#[derive(Debug, Clone, Copy)]
pub enum LeaseParam<'a> {
    Bytes(&'a [u8]),
    Int(i64),
    Text(&'a str),
}

/// Runs one lease statement and returns its single text value, if a row was returned. Each
/// statement of the protocol yields at most one row of one text column.
pub trait LeaseDriver: Send {
    fn text(
        &mut self,
        sql: &str,
        params: &[LeaseParam<'_>],
    ) -> impl Future<Output = Result<Option<String>, Error>> + Send;
    fn batch(&mut self, sql: &str) -> impl Future<Output = Result<(), Error>> + Send;
}

/// What a lease on one generation must confirm: the installation, and each frontier's lowering
/// (its relations, physical digest and live columns).
#[derive(Debug, Clone)]
pub struct LeaseContract {
    generation: GenerationId,
    model: ContentHash,
    installation: ContentHash,
    scopes: Arc<BTreeMap<Frontier, ddl::Scope>>,
}
/// What a lease holds: the generation's frontier and the relations that frontier lowers. Readers
/// take their relation set from here rather than recomputing the frontier's scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub frontier: Frontier,
    pub relations: BTreeSet<&'static str>,
}
impl LeaseContract {
    pub(super) fn from_scopes(
        model: ContentHash,
        installation: ContentHash,
        scopes: Arc<BTreeMap<Frontier, ddl::Scope>>,
        generation: GenerationId,
    ) -> Self {
        Self {
            generation,
            model,
            installation,
            scopes,
        }
    }
    pub fn new(model: &ValidatedModel, generation: GenerationId) -> Self {
        let scopes = ddl::scopes(model);
        Self {
            generation,
            model: model.digest(),
            installation: scopes[&Frontier::Conformance].physical,
            scopes: Arc::new(scopes),
        }
    }
    pub fn generation(&self) -> GenerationId {
        self.generation
    }
    /// Take the lease; return what it holds. On any refusal the transaction rolls back and no lock
    /// is held.
    pub async fn acquire<D: LeaseDriver>(&self, driver: &mut D) -> Result<Held, Error> {
        driver.batch("BEGIN").await?;
        match self.checked(driver).await {
            Ok(frontier) => {
                driver.batch("COMMIT").await.map_err(|error| match error {
                    Error::Database(error) => Error::Commit(error),
                    Error::Driver { detail, .. } => Error::Driver {
                        class: Infrastructure::Unconfirmed,
                        detail,
                    },
                    other => other,
                })?;
                Ok(frontier)
            }
            Err(error) => {
                // A failed rollback leaves the session uncertain; the caller must discard it.
                let _ = driver.batch("ROLLBACK").await;
                Err(error)
            }
        }
    }
    async fn checked<D: LeaseDriver>(&self, driver: &mut D) -> Result<Held, Error> {
        let g = self.generation;
        driver
            .text(super::locks::INSTALLATION_SHARED_TEXT, &[])
            .await?;
        let compatible = driver.text("SELECT (model_digest=$1 AND physical_digest=$2)::text FROM lctx_model_store.installation WHERE singleton",
            &[LeaseParam::Bytes(&self.model.0), LeaseParam::Bytes(&self.installation.0)]).await?;
        if compatible.as_deref() != Some("true") {
            return Err(Error::Contract);
        }
        driver
            .text(
                "SELECT 'locked' FROM (SELECT pg_advisory_xact_lock_shared($1)) AS l",
                &[LeaseParam::Int(g.lock())],
            )
            .await?;
        let registered = driver.text("SELECT state || ' ' || frontier || ' ' || encode(model_digest,'hex') || ' ' || encode(physical_digest,'hex') \
            FROM lctx_model_store.generations WHERE id=$1", &[LeaseParam::Bytes(&g.0)]).await?.ok_or(Error::Absent)?;
        let fields: Vec<&str> = registered.split(' ').collect();
        let [state, frontier, model, physical] = fields.as_slice() else {
            return Err(Error::Contract);
        };
        let frontier = ddl::frontier(frontier).ok_or(Error::Contract)?;
        let Some(scope) = self.scopes.get(&frontier) else {
            return Err(Error::Contract);
        };
        if *model != self.model.hex() || scope.physical.hex() != *physical {
            return Err(Error::Contract);
        }
        if *state != "published" {
            return Err(Error::State);
        }
        // The registry's digests say what was lowered; the live catalog says what is there now.
        let live = driver
            .text(ddl::LIVE_COLUMNS, &[LeaseParam::Text(&g.schema())])
            .await?;
        if live.as_deref() != Some(scope.columns.as_str()) {
            return Err(Error::Contract);
        }
        driver
            .text(
                "SELECT 'locked' FROM (SELECT pg_advisory_lock_shared($1)) AS l",
                &[LeaseParam::Int(g.lock())],
            )
            .await?;
        Ok(Held {
            frontier,
            relations: scope.relations.clone(),
        })
    }
    /// Release the lease and confirm the server held it.
    pub async fn release<D: LeaseDriver>(&self, driver: &mut D) -> Result<(), Error> {
        let released = driver
            .text(
                "SELECT pg_advisory_unlock_shared($1)::text",
                &[LeaseParam::Int(self.generation.lock())],
            )
            .await?;
        if released.as_deref() != Some("true") {
            return Err(Error::State);
        }
        Ok(())
    }
}

impl Error {
    /// A driver failure outside SQLx, kept with its class.
    pub fn driver(class: Infrastructure, error: impl std::fmt::Display) -> Self {
        Self::Driver {
            class,
            detail: error.to_string(),
        }
    }
}

/// The SQLx driver `pin` uses.
pub(super) struct Sqlx<'c>(pub &'c mut sqlx::PgConnection);
impl LeaseDriver for Sqlx<'_> {
    async fn text(
        &mut self,
        sql: &str,
        params: &[LeaseParam<'_>],
    ) -> Result<Option<String>, Error> {
        let mut query = sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = match param {
                LeaseParam::Bytes(bytes) => query.bind(bytes.to_vec()),
                LeaseParam::Int(value) => query.bind(*value),
                LeaseParam::Text(text) => query.bind(text.to_string()),
            };
        }
        Ok(query.fetch_optional(&mut *self.0).await?)
    }
    async fn batch(&mut self, sql: &str) -> Result<(), Error> {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_owned()))
            .execute(&mut *self.0)
            .await?;
        Ok(())
    }
}
