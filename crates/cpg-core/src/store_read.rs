//! Canonical reads over one pinned generation through the owned provider fork (DESIGN §15.11;
//! cutover plan §4.1 D14).
//!
//! A [`CanonicalReader`] registers each relation of one generation into a session as its declared
//! Arrow schema. The pin is checked against the registry once: the reader role reads a published
//! generation, the writer role may inspect a staging or failed one. Each relation scans exactly
//! the generation's own partition, so no generation filter is pushed down; a retired generation's
//! partitions are gone, and a query over them fails rather than answering empty. List columns are
//! read with nullable items (as the driver decodes arrays) and cast to the declared items, which
//! refuses a null the declaration forbids.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::common::TableReference;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::context::SessionContext;
use datafusion::logical_expr::{Expr, cast};
use datafusion::prelude::col;
use datafusion_table_providers_postgres::pool::PostgresConnectionPool;
use lctx_model::id::Id;
use lctx_postgres::Error;
use lctx_postgres::serving::{Role, RoleConfig};

/// Open a provider pool of `connections` read-only connections for a role config.
pub async fn connect(config: &RoleConfig, connections: u32) -> Result<PostgresConnectionPool, Error> {
    config.validate()?;
    let mut url = url::Url::parse(&config.url).map_err(|_| Error::Config("invalid provider URL"))?;
    let mut ssl = "prefer".to_owned();
    let mut root = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "sslmode" => ssl = value.into_owned(),
            "sslrootcert" => root = Some(PathBuf::from(value.into_owned())),
            _ => return Err(Error::Config("unsupported provider option")),
        }
    }
    if !matches!(ssl.as_str(), "disable" | "prefer" | "require" | "verify-ca" | "verify-full") {
        return Err(Error::Config("unsupported provider TLS mode"));
    }
    url.set_query(None);
    let mut driver = tokio_postgres::Config::from_str(url.as_str())
        .map_err(|_| Error::Config("invalid provider configuration"))?;
    driver
        .application_name("lctx-provider")
        .connect_timeout(Duration::from_secs(config.acquire_timeout_seconds))
        .ssl_mode(match ssl.as_str() {
            "disable" => tokio_postgres::config::SslMode::Disable,
            "prefer" => tokio_postgres::config::SslMode::Prefer,
            _ => tokio_postgres::config::SslMode::Require,
        })
        .options(format!(
            "-c search_path=pg_catalog,lctx_ext -c default_transaction_read_only=on \
             -c statement_timeout={}s -c lock_timeout={}s -c idle_in_transaction_session_timeout=30s",
            config.statement_timeout_seconds, config.lock_timeout_seconds
        ));
    PostgresConnectionPool::new_with_config(
        driver,
        &ssl,
        root,
        connections,
        Duration::from_secs(config.acquire_timeout_seconds),
    )
    .await
    .map_err(|_| Error::Config("provider connection failed"))
}

/// One relation to register: its name, its declared (batch) schema, and whether the store
/// supplies its partition column (a model relation) or the batch carries it (a legacy relation).
#[derive(Clone, Debug)]
pub struct ReadRelation {
    pub name: String,
    pub declared: SchemaRef,
    pub supplied_partition: Option<String>,
}

/// Canonical reads for one role.
pub struct CanonicalReader {
    pool: Arc<PostgresConnectionPool>,
    role: Role,
}

/// The schema the driver decodes: list items nullable, and the store-supplied partition column
/// first when there is one.
fn read_schema(relation: &ReadRelation) -> SchemaRef {
    let mut fields: Vec<Field> = Vec::new();
    if let Some(partition) = &relation.supplied_partition {
        fields.push(Field::new(partition, DataType::FixedSizeBinary(16), false));
    }
    fields.extend(relation.declared.fields().iter().map(|f| {
        let data_type = match f.data_type() {
            DataType::List(item) => DataType::List(Arc::new(item.as_ref().clone().with_nullable(true))),
            other => other.clone(),
        };
        Field::new(f.name(), data_type, f.is_nullable())
    }));
    Arc::new(Schema::new(fields))
}

impl CanonicalReader {
    pub async fn open(config: &RoleConfig, connections: u32) -> Result<Self, Error> {
        Ok(Self {
            pool: Arc::new(connect(config, connections).await?),
            role: config.role,
        })
    }

    /// The registry state of `generation`, if it exists.
    pub async fn state(&self, generation: Id) -> Result<Option<String>> {
        let connection = self
            .pool
            .connect_direct()
            .await
            .map_err(|_| DataFusionError::Execution("provider pin connection failed".into()))?;
        connection.conn.start_request();
        let row = connection
            .conn
            .query_opt(
                "SELECT state FROM lctx_store.generations WHERE generation_id = $1",
                &[&generation.0.as_slice()],
            )
            .await
            .map_err(|_| DataFusionError::Execution("provider pin query failed".into()))?;
        connection.conn.finish_request();
        Ok(row.map(|r| r.get::<_, String>(0)))
    }

    /// Register every relation of `generation` into `ctx` under its own name, as its declared
    /// schema. The reader requires a published generation; the writer may inspect a staging or
    /// failed one.
    pub async fn register(
        &self,
        ctx: &SessionContext,
        generation: Id,
        relations: &[ReadRelation],
    ) -> Result<()> {
        let state = self.state(generation).await?;
        let admitted = match (self.role, state.as_deref()) {
            (Role::Serving, Some("published")) => true,
            (Role::Importer, Some("staging" | "validated" | "failed")) => true,
            _ => false,
        };
        if !admitted {
            return Err(DataFusionError::Plan(format!(
                "generation {} is {state:?}, not readable by {}",
                generation.hex(),
                self.role.name()
            )));
        }
        let schema = lctx_model::ddl::generation_schema(generation);
        let factory = datafusion_table_providers_postgres::PostgresTableFactory::new(Arc::clone(&self.pool));
        for relation in relations {
            let table = factory.declared_table(
                TableReference::partial(schema.clone(), relation.name.clone()),
                read_schema(relation),
            );
            let columns: Vec<Expr> = relation
                .declared
                .fields()
                .iter()
                .map(|f| match f.data_type() {
                    DataType::List(_) => cast(col(f.name()), f.data_type().clone()).alias(f.name()),
                    _ => col(f.name()),
                })
                .collect();
            let view = ctx.read_table(table)?.select(columns)?.into_view();
            ctx.register_table(relation.name.as_str(), view)?;
        }
        Ok(())
    }
}
