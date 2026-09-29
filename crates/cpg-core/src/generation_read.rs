//! Generation-bound provider sessions (cutover plan P1.10, T13; core review C02). A session reads
//! one published generation through a bound pool of provider connections, each holding its own
//! lease, taken by the store's lease protocol after checking the installation, the generation's
//! state, its frontier-scoped digests and its live columns. A session never reconnects: a closed
//! connection or a cancelled read that cannot be drained loses it, terminally. Tables expose
//! exactly their relation's declared schema and push down only a closed predicate algebra; they
//! discover nothing from the database.
use std::{collections::BTreeSet, fmt, str::FromStr, sync::{Arc, Mutex}, time::Duration};
use arrow_schema::{DataType, SchemaRef};
use async_trait::async_trait;
use datafusion::{
    catalog::Session,
    common::{Column, DFSchema, ScalarValue, tree_node::{Transformed, TreeNode, TreeNodeRecursion}},
    datasource::TableProvider,
    error::{DataFusionError, Result},
    execution::{TaskContext, context::{SQLOptions, SessionContext}, memory_pool::MemoryConsumer},
    logical_expr::{Expr, ExprSchemable, Operator, TableProviderFilterPushDown, TableType},
    physical_expr::EquivalenceProperties,
    physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties, SendableRecordBatchStream,
        execution_plan::{Boundedness, EmissionType}, stream::RecordBatchStreamAdapter},
    prelude::lit,
    sql::unparser::{Unparser, dialect::PostgreSqlDialect},
};
use datafusion_table_providers_postgres::{bounded::ChunkLimits, pool::{BoundLimits, PoolHealth, PostgresConnectionPool, SessionBinder}};
use lctx_model::domain::{Infrastructure, Record, Relation, ValidatedModel, admission::{Frontier, FrontierContract}, stages::Profile};
use lctx_postgres::{generations::{Error as StoreError, GenerationId, LeaseContract, LeaseDriver, LeaseParam}, roles::{Role, RoleConfig}};

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("provider configuration: {0}")]
    Config(&'static str),
    /// The lease protocol refused the generation: absent, unpublished, another model or
    /// lowering, or tampered columns.
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("provider pool: {0}")]
    Pool(String),
    /// A relation outside the generation's frontier.
    #[error("frontier: {0}")]
    Frontier(String),
}

/// A session's connections, timeouts and transfer bounds.
#[derive(Debug, Clone, Copy)]
pub struct ProviderOptions { pub connections: u32, pub acquire_timeout: Duration, pub drain_timeout: Duration, pub chunks: ChunkLimits }
impl Default for ProviderOptions {
    fn default() -> Self {
        Self { connections: 2, acquire_timeout: Duration::from_secs(10), drain_timeout: Duration::from_secs(2), chunks: ChunkLimits::default() }
    }
}

/// The lease driver over a provider connection.
struct Tokio<'c>(&'c tokio_postgres::Client);
fn classify(error: tokio_postgres::Error) -> StoreError {
    let transport = error.is_closed() || error.as_db_error().is_none()
        || error.code().is_some_and(|code| code.code().starts_with("08") || code.code().starts_with("57P"));
    StoreError::driver(if transport { Infrastructure::Transport } else { Infrastructure::Refused }, error)
}
impl LeaseDriver for Tokio<'_> {
    async fn text(&mut self, sql: &str, params: &[LeaseParam<'_>]) -> Result<Option<String>, StoreError> {
        let values: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = params.iter().map(|param| match param {
            LeaseParam::Bytes(bytes) => Box::new(bytes.to_vec()) as Box<dyn tokio_postgres::types::ToSql + Sync + Send>,
            LeaseParam::Int(value) => Box::new(*value),
            LeaseParam::Text(text) => Box::new(text.to_string()),
        }).collect();
        let refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = values.iter().map(|v| v.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        let row = self.0.query_opt(sql, &refs).await.map_err(classify)?;
        row.map(|row| row.try_get::<_, String>(0)).transpose().map_err(classify)
    }
    async fn batch(&mut self, sql: &str) -> Result<(), StoreError> {
        self.0.batch_execute(sql).await.map_err(classify)
    }
}

/// Takes the lease on every connection the pool opens and remembers the first refusal.
#[derive(Debug)]
struct Binder { contract: LeaseContract, frontier: Mutex<Option<Frontier>>, refusal: Mutex<Option<StoreError>> }
#[async_trait]
impl SessionBinder for Binder {
    async fn bind(&self, client: &tokio_postgres::Client) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.contract.acquire(&mut Tokio(client)).await {
            Ok(frontier) => {
                let mut bound = self.frontier.lock().map_err(|_| "binder poisoned")?;
                if bound.is_some_and(|f| f != frontier) { return Err("connections disagree on the frontier".into()); }
                *bound = Some(frontier);
                Ok(())
            },
            Err(error) => {
                let message = error.to_string();
                self.refusal.lock().map_err(|_| "binder poisoned")?.get_or_insert(error);
                Err(message.into())
            },
        }
    }
}

/// A read session over one published generation.
pub struct GenerationSession {
    pool: Arc<PostgresConnectionPool>, contract: LeaseContract, model: Arc<ValidatedModel>, frontier: Frontier,
    held: BTreeSet<&'static str>, options: ProviderOptions,
}
impl GenerationSession {
    /// Open `options.connections` leased connections as the serving role. Every refusal happens
    /// here, before any scan.
    pub async fn open(config: &RoleConfig, model: Arc<ValidatedModel>, generation: GenerationId, options: ProviderOptions) -> Result<Self, ReadError> {
        config.validate().map_err(|_| ReadError::Config("invalid serving configuration"))?;
        if config.role != Role::Serving { return Err(ReadError::Config("provider sessions read as the serving role")); }
        if options.connections == 0 || options.connections > config.provider_connections {
            return Err(ReadError::Config("provider connections exceed the configured provider budget"));
        }
        let (driver, ssl, root) = driver_config(config)?;
        let binder = Arc::new(Binder { contract: LeaseContract::new(&model, generation), frontier: Mutex::new(None), refusal: Mutex::new(None) });
        let limits = BoundLimits { connections: options.connections, acquire_timeout: options.acquire_timeout, drain_timeout: options.drain_timeout };
        let pool = match PostgresConnectionPool::new_bound(driver, &ssl, root, binder.clone(), limits).await {
            Ok(pool) => pool,
            Err(error) => {
                let refusal = binder.refusal.lock().ok().and_then(|mut r| r.take());
                return Err(refusal.map_or_else(|| ReadError::Pool(error.to_string()), ReadError::Store));
            },
        };
        let frontier = binder.frontier.lock().ok().and_then(|f| *f).ok_or(ReadError::Pool("no connection was bound".into()))?;
        let held = held(&model, frontier)?;
        Ok(Self { pool: Arc::new(pool), contract: binder.contract.clone(), model, frontier, held, options })
    }
    pub fn generation(&self) -> GenerationId { self.contract.generation() }
    pub fn frontier(&self) -> Frontier { self.frontier }
    pub fn health(&self) -> PoolHealth { self.pool.health() }
    /// The table of one relation the generation's frontier holds.
    pub fn table<R: Record>(&self) -> Result<Arc<GenerationTable>, ReadError> {
        let relation = self.model.require::<R>().map_err(|_| ReadError::Frontier(format!("{} is not in this model", R::NAME)))?;
        self.table_of(relation)
    }
    fn table_of(&self, relation: &Relation) -> Result<Arc<GenerationTable>, ReadError> {
        if !self.held.contains(relation.name()) {
            return Err(ReadError::Frontier(format!("{} is outside this generation's {} frontier", relation.name(), self.frontier.name())));
        }
        Ok(Arc::new(GenerationTable { pool: self.pool.clone(), schema: relation.schema().clone(), options: self.options,
            source: format!("{}.{}", quote(&self.generation().schema()), quote(relation.name())) }))
    }
    /// Release every lease and close the connections. Returns only after the server confirmed
    /// each release; a lost session releases by closing.
    pub async fn close(self) -> Result<(), ReadError> {
        if self.pool.health() == PoolHealth::Lost { return Err(ReadError::Pool("the session is lost".into())); }
        let mut released = Vec::new();
        for _ in 0..self.options.connections {
            let connection = self.pool.connect_direct().await.map_err(|e| ReadError::Pool(e.to_string()))?;
            self.contract.release(&mut Tokio(&connection.conn)).await?;
            released.push(connection);
        }
        drop(released);
        Ok(())
    }
}

/// Read-only SQL over every relation of a pinned generation (`lctx query --generation`).
pub struct InspectionSession { session: GenerationSession, context: SessionContext }
impl InspectionSession {
    pub fn new(session: GenerationSession) -> Result<Self, ReadError> {
        let context = SessionContext::new();
        for relation in session.model.relations().iter().filter(|r| session.held.contains(r.name())) {
            context.register_table(relation.name(), session.table_of(relation)?).map_err(|e| ReadError::Pool(e.to_string()))?;
        }
        Ok(Self { session, context })
    }
    pub async fn sql(&self, sql: &str) -> Result<datafusion::dataframe::DataFrame> {
        self.context.sql_with_options(sql, SQLOptions::new().with_allow_ddl(false).with_allow_dml(false).with_allow_statements(false)).await
    }
    pub async fn close(self) -> Result<(), ReadError> { self.session.close().await }
}

fn held(model: &ValidatedModel, frontier: Frontier) -> Result<BTreeSet<&'static str>, ReadError> {
    let all = model.relations().iter().map(Relation::name);
    Ok(match frontier {
        Frontier::Conformance => all.collect(),
        Frontier::Facts => {
            let contract = FrontierContract::facts(model, Profile::Catalog).map_err(|e| ReadError::Frontier(e.to_string()))?;
            all.filter(|name| contract.contains(name)).collect()
        },
    })
}

fn driver_config(config: &RoleConfig) -> Result<(tokio_postgres::Config, String, Option<std::path::PathBuf>), ReadError> {
    let mut url = url::Url::parse(&config.url).map_err(|_| ReadError::Config("invalid provider URL"))?;
    let mut ssl = "prefer".to_owned();
    let mut root = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "sslmode" => ssl = value.into_owned(),
            "sslrootcert" => root = Some(std::path::PathBuf::from(value.into_owned())),
            _ => return Err(ReadError::Config("unsupported provider option")),
        }
    }
    if !matches!(ssl.as_str(), "disable" | "prefer" | "require" | "verify-ca" | "verify-full") { return Err(ReadError::Config("unsupported provider TLS mode")); }
    url.set_query(None);
    let mut driver = tokio_postgres::Config::from_str(url.as_str()).map_err(|_| ReadError::Config("invalid provider configuration"))?;
    driver.application_name("lctx-provider").connect_timeout(Duration::from_secs(config.acquire_timeout_seconds))
        .ssl_mode(match ssl.as_str() {
            "disable" => tokio_postgres::config::SslMode::Disable,
            "prefer" => tokio_postgres::config::SslMode::Prefer,
            _ => tokio_postgres::config::SslMode::Require,
        })
        .options(format!("-c search_path=pg_catalog -c default_transaction_read_only=on -c statement_timeout={}s -c lock_timeout={}s \
            -c idle_in_transaction_session_timeout=30s", config.statement_timeout_seconds, config.lock_timeout_seconds));
    Ok((driver, ssl, root))
}

fn quote(name: &str) -> String { format!("\"{}\"", name.replace('"', "\"\"")) }

/// The closed predicate algebra a generation table pushes down. Text compares only for
/// (in)equality, because PostgreSQL's collation order need not be DataFusion's byte order.
fn domain(ty: &DataType) -> bool {
    matches!(ty, DataType::Boolean | DataType::Int16 | DataType::Int32 | DataType::Int64 | DataType::Utf8 | DataType::Utf8View | DataType::Binary
        | DataType::FixedSizeBinary(16 | 32))
}
/// Two operand types PostgreSQL compares with DataFusion's meaning: equal types, text in either
/// string representation (SQL literals are `Utf8View`), or a binary identity against bytes.
fn comparable(left: Option<DataType>, right: Option<DataType>) -> bool {
    use DataType::*;
    left == right || matches!((left, right), (Some(Utf8), Some(Utf8View)) | (Some(Utf8View), Some(Utf8))
        | (Some(Binary), Some(FixedSizeBinary(_))) | (Some(FixedSizeBinary(_)), Some(Binary)))
}
pub fn admits_expr(expr: &Expr, schema: &DFSchema) -> bool {
    let ty = |expr: &Expr| expr.get_type(schema).ok();
    match expr {
        Expr::Column(_) | Expr::Literal(..) => ty(expr).is_some_and(|t| domain(&t)),
        Expr::Alias(a) => admits_expr(&a.expr, schema),
        Expr::IsNull(e) | Expr::IsNotNull(e) => admits_expr(e, schema),
        Expr::Not(e) => ty(e) == Some(DataType::Boolean) && admits_expr(e, schema),
        Expr::Cast(c) => (ty(&c.expr) == Some(c.field.data_type().clone()) || view_cast(&c.expr, c.field.data_type(), schema)) && admits_expr(&c.expr, schema),
        Expr::BinaryExpr(b) => {
            comparable(ty(&b.left), ty(&b.right)) && admits_expr(&b.left, schema) && admits_expr(&b.right, schema) && match b.op {
                Operator::And | Operator::Or => ty(&b.left) == Some(DataType::Boolean),
                Operator::Eq | Operator::NotEq => true,
                Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq => matches!(ty(&b.left), Some(DataType::Int16 | DataType::Int32 | DataType::Int64)),
                _ => false,
            }
        },
        _ => false,
    }
}
/// A table's filters name its columns with the table's qualifier; the table's own schema, and its
/// SQL, name exactly one relation.
fn unqualified(expr: &Expr) -> Expr {
    expr.clone().transform(|e| match e {
        Expr::Column(column) if column.relation.is_some() => Ok(Transformed::yes(Expr::Column(Column::new_unqualified(column.name)))),
        e => Ok(Transformed::no(e)),
    }).map(|t| t.data).unwrap_or_else(|_| expr.clone())
}
/// DataFusion compares a text column with a SQL literal by casting the column to `Utf8View`: the
/// same text, so the cast is admitted and dropped before unparsing.
fn view_cast(inner: &Expr, target: &DataType, schema: &DFSchema) -> bool {
    target == &DataType::Utf8View && inner.get_type(schema).ok() == Some(DataType::Utf8)
}
/// PostgreSQL reads `X'..'` as a bit string, while DataFusion unparses binary literals that
/// way: bind bytes as a typed bytea cast of their hex text.
fn postgres_expr(expr: Expr, schema: &DFSchema) -> Result<Expr> {
    Ok(expr.transform(|e| match e {
        Expr::Cast(cast) if view_cast(&cast.expr, cast.field.data_type(), schema) => Ok(Transformed::yes(*cast.expr)),
        Expr::Literal(ScalarValue::Binary(Some(bytes)) | ScalarValue::FixedSizeBinary(_, Some(bytes)), _) => {
            let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            Ok(Transformed::yes(datafusion::logical_expr::expr_fn::cast(lit(format!("\\x{hex}")), DataType::Binary)))
        },
        e => Ok(Transformed::no(e)),
    })?.data)
}

/// One relation of a leased generation.
#[derive(Debug)]
pub struct GenerationTable { pool: Arc<PostgresConnectionPool>, schema: SchemaRef, source: String, options: ProviderOptions }
#[async_trait]
impl TableProvider for GenerationTable {
    fn schema(&self) -> SchemaRef { self.schema.clone() }
    fn table_type(&self) -> TableType { TableType::Base }
    fn supports_filters_pushdown(&self, filters: &[&Expr]) -> Result<Vec<TableProviderFilterPushDown>> {
        let schema = DFSchema::try_from(self.schema.as_ref().clone())?;
        Ok(filters.iter().map(|e| if admits_expr(&unqualified(e), &schema) { TableProviderFilterPushDown::Exact } else { TableProviderFilterPushDown::Unsupported }).collect())
    }
    async fn scan(&self, _: &dyn Session, projection: Option<&Vec<usize>>, filters: &[Expr], limit: Option<usize>) -> Result<Arc<dyn ExecutionPlan>> {
        let schema = DFSchema::try_from(self.schema.as_ref().clone())?;
        let filters: Vec<Expr> = filters.iter().map(unqualified).collect();
        if filters.iter().any(|f| !admits_expr(f, &schema)) { return Err(DataFusionError::Plan("unadmitted generation predicate".into())); }
        let projected = match projection { Some(indices) => Arc::new(self.schema.project(indices)?), None => self.schema.clone() };
        let columns = projected.fields().iter().map(|f| quote(f.name())).collect::<Vec<_>>().join(", ");
        let unparser = Unparser::new(&PostgreSqlDialect {});
        let predicates = filters.iter().cloned().map(|f| Ok(format!("({})", unparser.expr_to_sql(&postgres_expr(f, &schema)?)?))).collect::<Result<Vec<_>>>()?;
        let mut sql = format!("SELECT {columns} FROM {}", self.source);
        if !predicates.is_empty() { sql.push_str(&format!(" WHERE {}", predicates.join(" AND "))); }
        if let Some(limit) = limit { sql.push_str(&format!(" LIMIT {limit}")); }
        let properties = Arc::new(PlanProperties::new(EquivalenceProperties::new(projected.clone()), Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental, Boundedness::Bounded));
        Ok(Arc::new(GenerationScan { pool: self.pool.clone(), sql, schema: projected, properties, options: self.options }))
    }
}

/// A bounded, reserved scan of one relation on one leased connection.
#[derive(Debug)]
pub struct GenerationScan { pool: Arc<PostgresConnectionPool>, sql: String, schema: SchemaRef, properties: Arc<PlanProperties>, options: ProviderOptions }
impl GenerationScan {
    pub fn sql(&self) -> &str { &self.sql }
}
impl DisplayAs for GenerationScan {
    fn fmt_as(&self, _: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "GenerationScan: {}", self.sql) }
}
impl ExecutionPlan for GenerationScan {
    fn name(&self) -> &str { "GenerationScan" }
    fn properties(&self) -> &Arc<PlanProperties> { &self.properties }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> { vec![] }
    fn with_new_children(self: Arc<Self>, children: Vec<Arc<dyn ExecutionPlan>>) -> Result<Arc<dyn ExecutionPlan>> {
        if !children.is_empty() { return Err(DataFusionError::Plan("a generation scan has no children".into())); }
        Ok(self)
    }
    fn apply_expressions(&self, _: &mut dyn FnMut(&Arc<dyn datafusion::physical_plan::PhysicalExpr>) -> Result<TreeNodeRecursion>) -> Result<TreeNodeRecursion> {
        Ok(TreeNodeRecursion::Continue)
    }
    fn execute(&self, _: usize, context: Arc<TaskContext>) -> Result<SendableRecordBatchStream> {
        use futures::{StreamExt, TryStreamExt};
        let (pool, sql, schema, options) = (self.pool.clone(), self.sql.clone(), self.schema.clone(), self.options);
        let reservation = MemoryConsumer::new("generation_scan").register(context.memory_pool());
        let stream = futures::stream::once(async move {
            let connection = pool.connect_direct().await.map_err(|e| DataFusionError::External(Box::new(e)))?;
            connection.query_arrow_bounded(&sql, &[], schema, options.chunks, Some(reservation), pool.drain_timeout()).await
        }).try_flatten().boxed();
        Ok(Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), stream)))
    }
}
