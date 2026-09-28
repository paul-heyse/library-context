//! The provider's driver and pool stay isolated here. PG15 owns admitted scans and federation.
use datafusion_table_providers_postgres::pool::PostgresConnectionPool;
use lctx_postgres::{
    Error,
    serving::{Role, RoleConfig},
};
use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

pub struct ProviderPool {
    pool: Arc<PostgresConnectionPool>,
    limit: u32,
}
impl ProviderPool {
    pub async fn open(config: &RoleConfig) -> Result<Self, Error> {
        config.validate()?;
        if config.role != Role::Serving || config.provider_connections == 0 {
            return Err(Error::Config("provider read-pool budget required"));
        }
        let mut url =
            url::Url::parse(&config.url).map_err(|_| Error::Config("invalid provider URL"))?;
        let mut ssl = "prefer".to_owned();
        let mut root = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "sslmode" => ssl = value.into_owned(),
                "sslrootcert" => root = Some(PathBuf::from(value.into_owned())),
                _ => return Err(Error::Config("unsupported provider option")),
            }
        }
        if !matches!(
            ssl.as_str(),
            "disable" | "prefer" | "require" | "verify-ca" | "verify-full"
        ) {
            return Err(Error::Config("unsupported provider TLS mode"));
        }
        url.set_query(None);
        let mut driver = tokio_postgres::Config::from_str(url.as_str())
            .map_err(|_| Error::Config("invalid provider configuration"))?;
        driver.application_name("lctx-provider").connect_timeout(Duration::from_secs(config.acquire_timeout_seconds))
   .ssl_mode(if ssl=="disable"{tokio_postgres::config::SslMode::Disable}else if ssl=="prefer"{tokio_postgres::config::SslMode::Prefer}else{tokio_postgres::config::SslMode::Require})
   .options(format!("-c search_path=pg_catalog,lctx_ext -c default_transaction_read_only=on -c statement_timeout={}s -c lock_timeout={}s -c idle_in_transaction_session_timeout=30s",config.statement_timeout_seconds,config.lock_timeout_seconds));
        let pool = PostgresConnectionPool::new_with_config(
            driver,
            &ssl,
            root,
            config.provider_connections,
            Duration::from_secs(config.acquire_timeout_seconds),
        )
        .await
        .map_err(|_| Error::Config("provider connection failed"))?;
        Ok(Self {
            pool: Arc::new(pool),
            limit: config.provider_connections,
        })
    }
    pub fn pool(&self) -> Arc<PostgresConnectionPool> {
        Arc::clone(&self.pool)
    }
    pub fn connection_limit(&self) -> u32 {
        self.limit
    }
}

use arrow_schema::{DataType, SchemaRef};
use async_trait::async_trait;
use cpg_schema::{id::Digest, postgres_report::View};
use datafusion::{
    catalog::Session,
    common::{
        DFSchema, ScalarValue, TableReference,
        tree_node::{Transformed, TreeNode, TreeNodeRecursion},
    },
    datasource::TableProvider,
    error::{DataFusionError, Result},
    execution::{TaskContext, context::SessionContext},
    logical_expr::{
        Expr, ExprSchemable, LogicalPlan, Operator, TableProviderFilterPushDown, TableType,
    },
    optimizer::{OptimizerConfig, OptimizerRule, optimizer::Optimizer},
    physical_plan::{
        DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
    },
    prelude::{col, lit},
};
use datafusion_federation::{
    FederatedTableProviderAdaptor,
    sql::{RemoteTableRef, SQLExecutor, SQLFederationProvider, SQLTableSource},
};
use std::fmt;

/// One closed capability algebra used for both scan predicates and whole federation subtrees.
/// Text comparisons/order are bytewise: every admitted database view declares COLLATE "C".
fn domain(ty: &DataType) -> bool {
    matches!(
        ty,
        DataType::Boolean
            | DataType::Int64
            | DataType::Utf8
            | DataType::Binary
            | DataType::FixedSizeBinary(16 | 32)
    )
}
pub fn admits_expr(expr: &Expr, schema: &DFSchema) -> bool {
    let ty = |expr: &Expr| expr.get_type(schema).ok();
    match expr {
        Expr::Column(_) | Expr::Literal(..) => ty(expr).is_some_and(|t| domain(&t)),
        Expr::Alias(a) => admits_expr(&a.expr, schema),
        Expr::IsNull(e) | Expr::IsNotNull(e) => admits_expr(e, schema),
        Expr::Not(e) => ty(e) == Some(DataType::Boolean) && admits_expr(e, schema),
        Expr::Cast(c) => {
            ty(&c.expr) == Some(c.field.data_type().clone()) && admits_expr(&c.expr, schema)
        }
        Expr::BinaryExpr(b) => {
            let compatible = ty(&b.left) == ty(&b.right)
                || matches!(
                    (ty(&b.left), ty(&b.right)),
                    (Some(DataType::Binary), Some(DataType::FixedSizeBinary(_)))
                        | (Some(DataType::FixedSizeBinary(_)), Some(DataType::Binary))
                );
            compatible
                && admits_expr(&b.left, schema)
                && admits_expr(&b.right, schema)
                && match b.op {
                    Operator::And | Operator::Or => ty(&b.left) == Some(DataType::Boolean),
                    Operator::Eq | Operator::NotEq => true,
                    Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq => {
                        ty(&b.left) == Some(DataType::Int64)
                    }
                    _ => false,
                }
        }
        _ => false,
    }
}
pub fn admits_plan(plan: &LogicalPlan) -> bool {
    let mut admitted = true;
    let inspected = plan.apply(|node| {
        let schema = match node.inputs().first() {
            Some(input) => input.schema().as_ref(),
            None => node.schema().as_ref(),
        };
        let own = match node {
            LogicalPlan::TableScan(scan) => {
                let remote = datafusion_federation::get_table_source(&scan.source)
                    .ok()
                    .flatten()
                    .and_then(|source| {
                        (source.as_ref() as &dyn std::any::Any)
                            .downcast_ref::<SQLTableSource>()
                            .map(SQLTableSource::table_reference)
                    });
                remote.is_some_and(|name| {
                    name.schema() == Some("lctx_report")
                        && matches!(name.table(), "generation_relations" | "operation_outline")
                }) && scan.filters.iter().all(|e| admits_expr(e, schema))
            }
            LogicalPlan::Projection(p) => p.expr.iter().all(|e| {
                admits_expr(e, schema)
                    && matches!(e, Expr::Column(_) | Expr::Alias(_))
                    && !binary_literal(e)
            }),
            LogicalPlan::Filter(f) => admits_expr(&f.predicate, schema),
            LogicalPlan::SubqueryAlias(_) => true,
            LogicalPlan::Limit(l) => l.skip.iter().chain(l.fetch.iter()).all(
                |e| matches!(e.as_ref(),Expr::Literal(ScalarValue::Int64(Some(n)),_) if *n>=0),
            ),
            LogicalPlan::Sort(s) => s
                .expr
                .iter()
                .all(|s| admits_expr(&s.expr, schema) && matches!(s.expr, Expr::Column(_))),
            LogicalPlan::Join(j) => {
                matches!(j.join_type, datafusion::logical_expr::JoinType::Inner)
                    && j.filter.is_none()
                    && declared_join_keys(&j.left, &j.right, &j.on)
                    && j.on.iter().all(|(l, r)| {
                        admits_expr(l, j.left.schema())
                            && admits_expr(r, j.right.schema())
                            && l.get_type(j.left.schema()).ok() == r.get_type(j.right.schema()).ok()
                    })
            }
            _ => false,
        };
        if !own {
            admitted = false;
            return Ok(TreeNodeRecursion::Stop);
        }
        Ok(TreeNodeRecursion::Continue)
    });
    inspected.is_ok() && admitted
}
fn binary_literal(expr: &Expr) -> bool {
    let mut found = false;
    let _ = expr.apply(|e| {
        if matches!(
            e,
            Expr::Literal(ScalarValue::Binary(_) | ScalarValue::FixedSizeBinary(..), _)
        ) {
            found = true;
        }
        Ok(TreeNodeRecursion::Continue)
    });
    found
}
// PostgreSQL X'..' is a bit string, while DF55 unparses Arrow binary literals that way.
// Use a typed bytea cast at this admitted boundary; all bytes remain generated literals.
fn postgres_literals(expr: Expr) -> Result<Expr> {
    Ok(expr
        .transform(|e| match e {
            Expr::Literal(
                ScalarValue::Binary(Some(bytes)) | ScalarValue::FixedSizeBinary(_, Some(bytes)),
                _,
            ) => {
                let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                Ok(Transformed::yes(datafusion::logical_expr::expr_fn::cast(
                    lit(format!("\\x{hex}")),
                    DataType::Binary,
                )))
            }
            e => Ok(Transformed::no(e)),
        })?
        .data)
}
// Only complete generation-qualified primary-key joins can leave DataFusion.
fn key_lineage(
    plan: &LogicalPlan,
    column: &datafusion::common::Column,
) -> Option<(String, String)> {
    let index = plan.schema().index_of_column(column).ok()?;
    match plan {
        LogicalPlan::TableScan(scan) => {
            let source = datafusion_federation::get_table_source(&scan.source).ok()??;
            let remote = (source.as_ref() as &dyn std::any::Any)
                .downcast_ref::<SQLTableSource>()?
                .table_reference();
            Some((
                remote.to_string(),
                plan.schema().field(index).name().clone(),
            ))
        }
        LogicalPlan::Projection(p) => {
            let mut expr = p.expr.get(index)?;
            while let Expr::Alias(a) = expr {
                expr = &a.expr;
            }
            if let Expr::Column(c) = expr {
                key_lineage(&p.input, c)
            } else {
                None
            }
        }
        LogicalPlan::SubqueryAlias(a) => {
            let (qualifier, field) = a.input.schema().qualified_field(index);
            key_lineage(
                &a.input,
                &datafusion::common::Column::new(qualifier.cloned(), field.name()),
            )
        }
        LogicalPlan::Filter(f) => key_lineage(&f.input, column),
        LogicalPlan::Sort(s) => key_lineage(&s.input, column),
        LogicalPlan::Limit(l) => key_lineage(&l.input, column),
        _ => None,
    }
}
fn declared_join_keys(left: &LogicalPlan, right: &LogicalPlan, on: &[(Expr, Expr)]) -> bool {
    let mut keys = std::collections::BTreeSet::new();
    let mut table = None;
    for (l, r) in on {
        let (Expr::Column(l), Expr::Column(r)) = (l, r) else {
            return false;
        };
        let (Some((lt, lk)), Some((rt, rk))) = (key_lineage(left, l), key_lineage(right, r)) else {
            return false;
        };
        if lt != rt || lk != rk || !keys.insert(lk) || table.as_ref().is_some_and(|t| t != &lt) {
            return false;
        }
        table = Some(lt);
    }
    (table.as_deref() == Some("lctx_report.generation_relations")
        && keys
            == std::collections::BTreeSet::from([
                "generation_digest".into(),
                "relation_name".into(),
            ]))
        || (table.as_deref() == Some("lctx_report.operation_outline")
            && keys
                == std::collections::BTreeSet::from(["generation_digest".into(), "node_id".into()]))
}
#[derive(Debug)]
struct SealedFederation(Arc<dyn datafusion_federation::FederationPlanner>);
#[async_trait]
impl datafusion_federation::FederationPlanner for SealedFederation {
    async fn plan_federation(
        &self,
        node: &datafusion_federation::FederatedPlanNode,
        state: &dyn Session,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(SealedScan(
            self.0.plan_federation(node, state).await?,
        )))
    }
}
#[derive(Debug)]
struct PinnedExecutor {
    inner: Arc<dyn SQLExecutor>,
    generation: Digest,
}
#[async_trait]
impl SQLExecutor for PinnedExecutor {
    fn name(&self) -> &str {
        "lctx_pg_admission_v1"
    }
    fn compute_context(&self) -> Option<String> {
        Some(format!(
            "{}:{}:admission-v1",
            self.inner.compute_context().unwrap_or_default(),
            self.generation.hex()
        ))
    }
    fn dialect(&self) -> Arc<dyn datafusion::sql::unparser::dialect::Dialect> {
        self.inner.dialect()
    }
    fn ast_analyzer(&self) -> Option<datafusion_federation::sql::AstAnalyzer> {
        Some(Box::new(|mut statement| {
            use datafusion::sql::sqlparser::ast::VisitMut;
            let _ = statement.visit(&mut AliasRepair::default());
            Ok(statement)
        }))
    }
    fn execute(
        &self,
        query: &str,
        schema: SchemaRef,
        filters: &[Arc<dyn datafusion::physical_plan::PhysicalExpr>],
    ) -> Result<SendableRecordBatchStream> {
        if !filters.is_empty() {
            return Err(DataFusionError::Plan(
                "unadmitted physical PostgreSQL predicate".into(),
            ));
        }
        self.inner.execute(query, schema, filters)
    }
    async fn table_names(&self) -> Result<Vec<String>> {
        Err(DataFusionError::Plan(
            "finite declared reporting inventory required".into(),
        ))
    }
    async fn get_table_schema(&self, _: &str) -> Result<SchemaRef> {
        Err(DataFusionError::Plan(
            "declared reporting schema required".into(),
        ))
    }
}
// DF55's unparser can leave the physical table qualifier inside an aliased scan.
// Repair only unambiguous declared table references, with a separate scope per SELECT.
#[derive(Default)]
struct AliasRepair {
    scopes: Vec<std::collections::BTreeMap<String, Option<datafusion::sql::sqlparser::ast::Ident>>>,
}
impl datafusion::sql::sqlparser::ast::VisitorMut for AliasRepair {
    type Break = ();
    fn pre_visit_query(
        &mut self,
        query: &mut datafusion::sql::sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<()> {
        use datafusion::sql::sqlparser::ast::{SetExpr, TableFactor};
        let mut scope = std::collections::BTreeMap::new();
        if let SetExpr::Select(select) = query.body.as_ref() {
            for factor in select.from.iter().flat_map(|from| {
                std::iter::once(&from.relation).chain(from.joins.iter().map(|j| &j.relation))
            }) {
                if let TableFactor::Table { name, alias, .. } = factor
                    && let Some(table) = name.0.last().and_then(|part| part.as_ident())
                {
                    if scope.contains_key(&table.value) {
                        scope.insert(table.value.clone(), None);
                    } else {
                        scope.insert(
                            table.value.clone(),
                            Some(
                                alias
                                    .as_ref()
                                    .map_or_else(|| table.clone(), |alias| alias.name.clone()),
                            ),
                        );
                    }
                }
            }
        }
        self.scopes.push(scope);
        std::ops::ControlFlow::Continue(())
    }
    fn post_visit_query(
        &mut self,
        _: &mut datafusion::sql::sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<()> {
        self.scopes.pop();
        std::ops::ControlFlow::Continue(())
    }
    fn post_visit_expr(
        &mut self,
        expr: &mut datafusion::sql::sqlparser::ast::Expr,
    ) -> std::ops::ControlFlow<()> {
        use datafusion::sql::sqlparser::ast::Expr;
        if let Expr::CompoundIdentifier(parts) = expr
            && parts.len() >= 2
        {
            let qualifier = &parts[parts.len() - 2].value;
            if matches!(
                qualifier.as_str(),
                "generation_relations" | "operation_outline"
            ) && let Some(Some(alias)) =
                self.scopes.last().and_then(|scope| scope.get(qualifier))
            {
                *parts = vec![alias.clone(), parts.last().expect("column").clone()];
            }
        }
        std::ops::ControlFlow::Continue(())
    }
}
#[derive(Debug)]
struct AdmitFederation {
    inner: Arc<Optimizer>,
}
impl OptimizerRule for AdmitFederation {
    fn name(&self) -> &str {
        "lctx_admitted_federation"
    }
    fn supports_rewrite(&self) -> bool {
        true
    }
    fn rewrite(
        &self,
        plan: LogicalPlan,
        config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        if !admits_plan(&plan) {
            return Ok(Transformed::no(plan));
        }
        // Scan predicates refer to the scan itself. Leaving their old qualifiers intact
        // across federation's table rewrite produces invalid SQL beneath aliases.
        let plan = plan
            .transform_up(|node| {
                if let LogicalPlan::SubqueryAlias(mut alias) = node {
                    while let LogicalPlan::SubqueryAlias(inner) = alias.input.as_ref() {
                        alias.input = inner.input.clone();
                    }
                    if let LogicalPlan::Filter(filter) = alias.input.as_ref() {
                        // Keep each join side's predicate bound to its visible alias before SQL
                        // generation; the unparser may lift this filter into the join's WHERE.
                        let predicate = filter
                            .predicate
                            .clone()
                            .transform(|e| match e {
                                Expr::Column(mut c) => {
                                    c.relation = Some(alias.alias.clone());
                                    Ok(Transformed::yes(Expr::Column(c)))
                                }
                                e => Ok(Transformed::no(e)),
                            })?
                            .data;
                        let mut input = filter.input.as_ref().clone();
                        while let LogicalPlan::SubqueryAlias(inner) = input {
                            input = inner.input.as_ref().clone();
                        }
                        let inner = datafusion::logical_expr::LogicalPlanBuilder::from(input)
                            .alias(alias.alias.clone())?
                            .build()?;
                        return Ok(Transformed::yes(
                            datafusion::logical_expr::LogicalPlanBuilder::from(inner)
                                .filter(predicate)?
                                .build()?,
                        ));
                    }
                    return Ok(Transformed::yes(LogicalPlan::SubqueryAlias(alias)));
                }
                if let LogicalPlan::TableScan(mut scan) = node {
                    scan.filters = scan
                        .filters
                        .into_iter()
                        .map(|e| {
                            e.transform(|e| match e {
                                Expr::Column(mut c) => {
                                    c.relation = None;
                                    Ok(Transformed::yes(Expr::Column(c)))
                                }
                                e => Ok(Transformed::no(e)),
                            })
                            .map(|r| r.data)
                        })
                        .collect::<Result<_>>()?;
                    Ok(Transformed::yes(LogicalPlan::TableScan(scan)))
                } else {
                    Ok(Transformed::no(node))
                }
            })?
            .data;
        let plan = plan
            .transform_up(|node| {
                node.map_expressions(|e| Ok(Transformed::yes(postgres_literals(e)?)))
            })?
            .data;
        let plan = self.inner.optimize(plan, config, |_, _| {})?;
        let plan = plan
            .transform_up(|node| {
                if let LogicalPlan::Extension(e) = &node
                    && let Some(f) = e
                        .node
                        .as_any()
                        .downcast_ref::<datafusion_federation::FederatedPlanNode>()
                {
                    return Ok(Transformed::yes(LogicalPlan::Extension(
                        datafusion::logical_expr::Extension {
                            node: Arc::new(datafusion_federation::FederatedPlanNode::new(
                                f.plan.clone(),
                                Arc::new(SealedFederation(f.planner.clone())),
                            )),
                        },
                    )));
                }
                Ok(Transformed::no(node))
            })?
            .data;
        Ok(Transformed::yes(plan))
    }
}
#[derive(Debug)]
struct AdmittedTable {
    inner: Arc<dyn TableProvider>,
    pushdown: bool,
}
#[async_trait]
impl TableProvider for AdmittedTable {
    fn schema(&self) -> SchemaRef {
        self.inner.schema()
    }
    fn table_type(&self) -> TableType {
        TableType::View
    }
    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        let schema = DFSchema::try_from(self.schema().as_ref().clone())?;
        Ok(filters
            .iter()
            .map(|e| {
                if self.pushdown && admits_expr(e, &schema) {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect())
    }
    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let schema = DFSchema::try_from(self.schema().as_ref().clone())?;
        if filters
            .iter()
            .any(|f| !self.pushdown || !admits_expr(f, &schema))
        {
            return Err(DataFusionError::Plan(
                "unadmitted PostgreSQL predicate".into(),
            ));
        }
        let filters = filters
            .iter()
            .cloned()
            .map(postgres_literals)
            .collect::<Result<Vec<_>>>()?;
        Ok(Arc::new(SealedScan(
            self.inner.scan(state, projection, &filters, limit).await?,
        )))
    }
}
/// Keep later physical optimizer rewrites from bypassing logical capability admission.
#[derive(Debug)]
struct SealedScan(Arc<dyn ExecutionPlan>);
impl DisplayAs for SealedScan {
    fn fmt_as(&self, _: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AdmittedPostgresScan")
    }
}
impl ExecutionPlan for SealedScan {
    fn name(&self) -> &str {
        "AdmittedPostgresScan"
    }
    fn properties(&self) -> &Arc<PlanProperties> {
        self.0.properties()
    }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !children.is_empty() {
            return Err(DataFusionError::Plan("sealed scan has no children".into()));
        }
        Ok(self)
    }
    fn apply_expressions(
        &self,
        _: &mut dyn FnMut(
            &Arc<dyn datafusion::physical_plan::PhysicalExpr>,
        ) -> Result<TreeNodeRecursion>,
    ) -> Result<TreeNodeRecursion> {
        Ok(TreeNodeRecursion::Continue)
    }
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        use futures::TryStreamExt;
        let mut rows = 0usize;
        let mut bytes = 0usize;
        let stream = self.0.execute(partition, context)?.and_then(move |batch| {
            rows += batch.num_rows();
            bytes += batch.get_array_memory_size();
            futures::future::ready(if rows > 200000 || bytes > 128 * 1024 * 1024 {
                Err(DataFusionError::ResourcesExhausted(
                    "PostgreSQL transfer row/byte budget".into(),
                ))
            } else {
                Ok(batch)
            })
        });
        Ok(Box::pin(
            datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(self.schema(), stream),
        ))
    }
}
impl ProviderPool {
    /// No inference, no mutable relations, and the pin is part of the expanded logical view.
    pub async fn register(
        &self,
        ctx: &SessionContext,
        pin: &lctx_postgres::repository::PinnedGeneration,
        view: View,
        alias: &str,
        pushdown: bool,
        federation: bool,
    ) -> Result<()> {
        if !view.immutable() {
            return Err(DataFusionError::Plan(
                "mutable PostgreSQL relations require coherent capture".into(),
            ));
        }
        let generation = Digest::from_hex(&pin.generation())
            .ok_or_else(|| DataFusionError::Plan("invalid pinned identity".into()))?;
        let connection = self
            .pool
            .connect_direct()
            .await
            .map_err(|_| DataFusionError::Execution("provider pin connection failed".into()))?;
        connection.conn.start_request();
        let row=connection.conn.query_opt("SELECT manifest::text FROM lctx_serving.generations WHERE generation_digest=$1 AND state='ready'", &[&generation.0.as_slice()]).await.map_err(|_|DataFusionError::Execution("provider pin verification failed".into()))?;
        let matches = row
            .and_then(|r| r.try_get::<_, String>(0).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            == serde_json::to_value(pin.manifest()).ok();
        connection.conn.finish_request();
        drop(connection);
        if !matches {
            return Err(DataFusionError::Plan(
                "provider generation is not the validated ready pin".into(),
            ));
        }
        let name = TableReference::partial("lctx_report", view.name());
        let table = datafusion_table_providers_postgres::PostgresTableFactory::new(self.pool())
            .declared_table(name.clone(), view.schema());
        let provider: Arc<dyn TableProvider> = Arc::new(AdmittedTable {
            inner: table.clone(),
            pushdown,
        });
        let provider = if federation {
            let mut owner = SQLFederationProvider::new(Arc::new(PinnedExecutor {
                inner: table,
                generation,
            }));
            owner.optimizer = Arc::new(Optimizer::with_rules(vec![Arc::new(AdmitFederation {
                inner: owner.optimizer.clone(),
            })]));
            let source = Arc::new(SQLTableSource::new_with_schema(
                Arc::new(owner),
                RemoteTableRef::from(name),
                view.schema(),
            ));
            Arc::new(FederatedTableProviderAdaptor::new_with_provider(
                source, provider,
            )) as Arc<dyn TableProvider>
        } else {
            provider
        };
        let pinned = ctx
            .read_table(provider)?
            .filter(
                col("generation_digest").eq(lit(ScalarValue::FixedSizeBinary(
                    32,
                    Some(generation.0.to_vec()),
                ))),
            )?
            .into_view();
        ctx.register_table(alias, pinned)?;
        Ok(())
    }
}

/// Preserve Delta's session rules while installing admitted federation planning.
pub fn session() -> SessionContext {
    use datafusion::execution::session_state::SessionStateBuilder;
    let base = crate::snapshot::empty_session().state();
    let mut rules = base.optimizers().to_vec();
    rules.push(Arc::new(
        datafusion_federation::FederationOptimizerRule::new(),
    ));
    SessionContext::new_with_state(
        SessionStateBuilder::new_from_existing(base)
            .with_optimizer_rules(rules)
            .with_query_planner(Arc::new(datafusion_federation::FederatedQueryPlanner::new()))
            .build(),
    )
}

pub struct Report {
    pub json: serde_json::Value,
    pub table: String,
}
pub async fn report(
    config: &RoleConfig,
    root: &std::path::Path,
    snapshot: cpg_schema::id::Id,
    generation: Digest,
) -> std::result::Result<Report, crate::CoreError> {
    use datafusion::datasource::MemTable;
    let root = fs_err::canonicalize(root)?;
    let catalog = crate::snapshot::catalog(&root).await?;
    let publication = catalog
        .iter()
        .find(|r| r.snapshot_id == snapshot)
        .ok_or_else(|| crate::CoreError::Bundle("report snapshot is not published".into()))?;
    let versions = crate::snapshot::resolve(&root, snapshot)
        .await?
        .ok_or_else(|| crate::CoreError::NoAttempt(snapshot.hex()))?;
    let reader = config.open_serving().await?;
    let result=async {
        // Resolve the immutable identity before the mutable capture; no cross-pool MVCC claim.
        let source=crate::snapshot::session(&root,snapshot,&versions).await?;
        let rows=crate::sql::query(&source,"SELECT coalesce(min(library),min(label)) library FROM releases").await?.collect().await?;
        if rows.len()!=1 || rows[0].num_rows()!=1{return Err(crate::CoreError::Bundle("report requires one canonical library summary".into()));}
        let library=match ScalarValue::try_from_array(rows[0].column(0),0)? {
            ScalarValue::Utf8(Some(s))|ScalarValue::Utf8View(Some(s))|ScalarValue::LargeUtf8(Some(s))=>s,
            _=>return Err(crate::CoreError::Bundle("report release identity".into())),
        };
        let started=std::time::Instant::now();
        let capture=reader.capture_report(&root.to_string_lossy(),snapshot,generation,publication.compiler_digest).await?;
        let capture_ms=started.elapsed().as_secs_f64()*1000.0;
        let states=&capture.tables[View::ProjectionState.name()];
        if states.iter().map(|b|b.num_rows()).sum::<usize>()!=1{return Err(crate::CoreError::Bundle("report projection identity does not exist".into()));}
        let state=&states[0];
        for (column,expected) in [("snapshot_id",snapshot.0.as_slice()),("content_digest",publication.content_digest.0.as_slice()),("compiler_digest",publication.compiler_digest.0.as_slice())] {
            let array=state.column_by_name(column).and_then(|a|a.as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>()).ok_or_else(||crate::CoreError::Bundle("report identity schema".into()))?;
            if array.value(0)!=expected{return Err(crate::CoreError::Bundle("report canonical/serving identity mismatch".into()));}
        }
        let ready=ScalarValue::try_from_array(state.column_by_name("state").expect("declared field"),0)?==ScalarValue::Utf8(Some("ready".into()));
        let ctx=session();
        crate::snapshot::register(&ctx,&root,"releases",*versions.get("releases").ok_or_else(||crate::CoreError::MissingTable("releases"))?,snapshot).await?;
        for view in View::MUTABLE {
            ctx.register_table(view.name(),Arc::new(MemTable::try_new(view.schema(),vec![capture.tables[view.name()].clone()])?))?;
        }
        let provider=ProviderPool::open(config).await?;
        if ready {
            let pinned=reader.pin(&library,Some(generation),None).await?;
            provider.register(&ctx,&pinned,View::GenerationRelations,"pg_relations",true,true).await?;
        } else {
            ctx.register_table("pg_relations",Arc::new(MemTable::try_new(View::GenerationRelations.schema(),vec![vec![arrow_array::RecordBatch::new_empty(View::GenerationRelations.schema())]])?))?;
        }
        let sql="SELECT encode(CAST(r.snapshot_id AS BYTEA),'hex') snapshot_id,r.library,encode(CAST(p.generation_digest AS BYTEA),'hex') generation,p.relation_name,p.rows serving_rows,o.snapshot_id IS NOT NULL observed_publication,o.available,counts.store_compiler_attempts,counts.store_compiler_events,imports.import_attempts FROM (SELECT snapshot_id,coalesce(min(library),min(label)) library FROM releases GROUP BY snapshot_id) r LEFT JOIN pg_relations p ON r.snapshot_id=p.snapshot_id LEFT JOIN captured_publications o ON r.snapshot_id=o.snapshot_id CROSS JOIN (SELECT count(DISTINCT attempt_id) store_compiler_attempts,count(event_key) store_compiler_events FROM captured_events) counts CROSS JOIN (SELECT count(*) import_attempts FROM captured_imports) imports ORDER BY p.relation_name";
        let started=std::time::Instant::now();
        let batches=crate::sql::query(&ctx,sql).await?.collect().await?;
        let query_ms=started.elapsed().as_secs_f64()*1000.0;
        let table=datafusion::arrow::util::pretty::pretty_format_batches(&batches)?.to_string();
        let mut writer=datafusion::arrow::json::ArrayWriter::new(Vec::new());
        writer.write_batches(&batches.iter().collect::<Vec<_>>())?;writer.finish()?;
        let rows:serde_json::Value=serde_json::from_slice(&writer.into_inner()).map_err(|_|crate::CoreError::Bundle("report JSON encoding".into()))?;
        let mut diagnostics=serde_json::Map::new();
        for view in [View::ProjectionState,View::Profiles,View::Selections,View::ProfileAttempts] {
            let mut writer=datafusion::arrow::json::ArrayWriter::new(Vec::new());
            writer.write_batches(&capture.tables[view.name()].iter().collect::<Vec<_>>())?;writer.finish()?;
            diagnostics.insert(view.name().into(),serde_json::from_slice(&writer.into_inner()).map_err(|_|crate::CoreError::Bundle("report diagnostics encoding".into()))?);
        }
        let table=format!("{table}\nDiagnostics captured together:\n{}",serde_json::to_string_pretty(&diagnostics).map_err(|_|crate::CoreError::Bundle("report diagnostic rendering".into()))?);
        Ok(Report{table,json:serde_json::json!({"generation":generation.hex(),"snapshot_id":snapshot.hex(),"captured_at":capture.captured_at,"mvcc_snapshot":capture.snapshot,"diagnostics":diagnostics,"attempt_scope":"store and compiler digest; may include other libraries","capture_rows":capture.rows,"capture_bytes":capture.bytes,"capture_ms":capture_ms,"query_and_transfer_ms":query_ms,"provider_query_start_ns":provider.pool.read_metrics().0,"provider_row_conversion_ns":provider.pool.read_metrics().1,"provider_rows":provider.pool.read_metrics().2,"rows":rows})})
    }.await;
    reader.close().await;
    result
}
