//! Captured immutable ports earn native authority on logical name resolution, before
//! optimization. Catalog construction and unrelated names acquire no remote authority.
use super::*;
use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion::error::{DataFusionError, Result as DataFusionResult};
use tracing::Instrument;

struct NativeInput {
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    view: lctx_model::domain::completed::CompletedView,
    relation: Relation,
    producing: Option<lctx_compiler_store::compiler::ProducingScope>,
    budget: ResourceBudget,
    batch_rows: usize,
    _files: Arc<WorkspaceFiles>,
}
pub(super) struct InputPort {
    native: Option<NativeInput>,
    private_cancellation: Option<Cancellation>,
    provider: tokio::sync::OnceCell<Arc<dyn TableProvider>>,
    // Fields and their containers drop before their reservation.
    _charge: Box<dyn Reservation>,
}
impl InputPort {
    pub(super) fn capture(
        workspace: &Workspace,
        source: &CompletedRelation,
        producing: Option<lctx_compiler_store::compiler::ProducingScope>,
    ) -> Result<Arc<Self>, ModelError> {
        let charge = workspace.budget.reserve(
            "completed-input-captured-port",
            size_of::<Self>()
                + source.view.relation.capacity()
                + source
                    .view
                    .contributions
                    .len()
                    .saturating_mul(size_of::<ContentHash>() + 64)
                + 512,
        )?;
        if source.private.is_some() {
            return Ok(Arc::new(Self {
                native: None,
                private_cancellation: Some(source.cancellation.clone()),
                provider: tokio::sync::OnceCell::new_with(Some(private_provider(source)?)),
                _charge: charge,
            }));
        }
        Ok(Arc::new(Self {
            private_cancellation: None,
            native: Some(NativeInput {
                native: workspace.native()?.clone(),
                view: source.view.clone(),
                relation: source.relation.clone(),
                producing,
                budget: workspace.budget.clone(),
                batch_rows: workspace.options.batch_rows,
                _files: workspace.files.clone(),
            }),
            provider: tokio::sync::OnceCell::new(),
            _charge: charge,
        }))
    }
    #[cfg(test)]
    pub(super) fn ready(
        provider: Arc<dyn TableProvider>,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let charge = budget.reserve("completed-input-control-port", size_of::<Self>() + 512)?;
        Ok(Arc::new(Self {
            native: None,
            private_cancellation: None,
            provider: tokio::sync::OnceCell::new_with(Some(provider)),
            _charge: charge,
        }))
    }
    pub(super) fn resolved(&self) -> Result<&Arc<dyn TableProvider>, ModelError> {
        self.provider
            .get()
            .ok_or(ModelError::Conflict("completed input port not resolved"))
    }
    async fn resolve(&self) -> Result<Arc<dyn TableProvider>, ModelError> {
        if let Some(cancellation) = &self.private_cancellation {
            cancellation.check()?;
        }
        self.provider
            .get_or_try_init(|| async {
                    let input = self.native.as_ref().ok_or(ModelError::Conflict(
                        "completed input port missing captured source",
                    ))?;
                    tracing::info!(target: "lctx_phase", observation="requested_input_binding",
                        relation=input.relation.name(), view=?input.view.identity, rows=input.view.rows);
                    let earning = lctx_native::phase::wait(
                        "completed_input_provider",
                        input.native.table_provider(
                            &input.view,
                            input.relation.clone(),
                            input.budget.clone(),
                            input.batch_rows,
                        ),
                    ).instrument(tracing::info_span!(target: "lctx_phase", "completed_input_port",
                        relation=input.relation.name(), view=?input.view.identity));
                    // Name resolution may run in another task. Re-enter this exact
                    // producing owner for earning as well as delayed native scans.
                    let provider = match &input.producing {
                        Some(scope) => scope.run(earning).await?,
                        None => earning.await?,
                    };
                    match &input.producing {
                        Some(scope) => lctx_compiler_store::compiler_provider::bind_producing(&provider, scope),
                        None => Ok(provider),
                    }
            })
            .await.cloned()
    }
}

/// Compiler lifecycle stays at the exposure boundary; immutable storage remains reusable.
struct PrivateInput {
    source: Arc<private_source::PrivateRelation>,
    cancellation: Cancellation,
    execution: Arc<WorkspaceExecutionOwner>,
}
impl std::fmt::Debug for PrivateInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateInput")
            .field("relation", &self.source.relation().name())
            .finish_non_exhaustive()
    }
}
pub(super) fn private_provider(
    source: &CompletedRelation,
) -> Result<Arc<dyn TableProvider>, ModelError> {
    source.cancellation.check()?;
    Ok(Arc::new(PrivateInput {
        source: source.private_source()?.clone(),
        cancellation: source.cancellation.clone(),
        execution: source._execution.clone(),
    }))
}
#[async_trait::async_trait]
impl TableProvider for PrivateInput {
    fn schema(&self) -> arrow_schema::SchemaRef {
        self.source.relation().schema().clone()
    }
    fn table_type(&self) -> datafusion::logical_expr::TableType {
        datafusion::logical_expr::TableType::Base
    }
    fn statistics(&self) -> Option<datafusion::common::Statistics> {
        self.source.provider().statistics()
    }
    async fn scan(
        &self,
        _: &dyn datafusion::catalog::Session,
        projection: Option<&Vec<usize>>,
        _: &[datafusion::logical_expr::Expr],
        limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        self.cancellation
            .check()
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        Ok(Arc::new(
            datafusion::physical_plan::streaming::StreamingTableExec::try_new(
                self.schema(),
                vec![Arc::new(Self {
                    source: self.source.clone(),
                    cancellation: self.cancellation.clone(),
                    execution: self.execution.clone(),
                })],
                projection,
                [],
                false,
                limit,
            )?,
        ))
    }
}
impl datafusion::physical_plan::streaming::PartitionStream for PrivateInput {
    fn schema(&self) -> &arrow_schema::SchemaRef {
        self.source.relation().schema()
    }
    fn execute(
        &self,
        _: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::physical_plan::SendableRecordBatchStream {
        let batches = self
            .cancellation
            .check()
            .and_then(|()| self.source.read_batches());
        let state = (batches, self.cancellation.clone(), self.execution.clone());
        let stream =
            futures::stream::try_unfold(state, |(batches, cancellation, execution)| async move {
                cancellation
                    .check()
                    .map_err(|e| DataFusionError::External(Box::new(e)))?;
                let mut batches = batches.map_err(|e| DataFusionError::External(Box::new(e)))?;
                match batches.next() {
                    Some(batch) => {
                        let batch = batch.map_err(|e| DataFusionError::External(Box::new(e)))?;
                        tokio::task::yield_now().await;
                        cancellation
                            .check()
                            .map_err(|e| DataFusionError::External(Box::new(e)))?;
                        Ok(Some((batch, (Ok(batches), cancellation, execution))))
                    }
                    None => Ok(None),
                }
            });
        Box::pin(
            datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(
                self.source.relation().schema().clone(),
                stream,
            ),
        )
    }
}

pub(super) struct InputAliases {
    pub(super) ports: BTreeMap<String, Arc<InputPort>>,
    pub(super) _charge: StateCharge,
}
impl InputAliases {
    #[cfg(test)]
    pub(super) fn ready(
        providers: BTreeMap<String, Arc<dyn TableProvider>>,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let mut charge = StateCharge::new(budget, "completed-input-control-aliases");
        charge.grow(
            size_of::<Self>() + providers.keys().map(|name| name.len() + 128).sum::<usize>(),
        )?;
        let mut ports = BTreeMap::new();
        for (name, provider) in providers {
            ports.insert(name, InputPort::ready(provider, budget)?);
        }
        Ok(Arc::new(Self {
            ports,
            _charge: charge,
        }))
    }
}

pub(super) struct InputSchema {
    aliases: Arc<InputAliases>,
    candidates: Mutex<BTreeMap<String, Candidate>>,
    budget: ResourceBudget,
    _charge: Box<dyn Reservation>,
}
struct Candidate {
    provider: Arc<dyn TableProvider>,
    // Removal drops the retained key and map nodes before this allowance. The
    // schema's base allowance covers the empty BTree root retained after removal.
    _charge: Box<dyn Reservation>,
}
impl std::fmt::Debug for InputSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputSchema")
            .field("captured_ports", &self.aliases.ports.len())
            .finish_non_exhaustive()
    }
}
impl InputSchema {
    pub(super) fn install(
        context: &SessionContext,
        aliases: Arc<InputAliases>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let config = context.copied_config();
        let catalog_name = &config.options().catalog.default_catalog;
        let schema_name = &config.options().catalog.default_schema;
        let catalog = context.catalog(catalog_name).ok_or_else(|| {
            crate::sql::model_error(DataFusionError::Plan(format!(
                "failed to resolve catalog: {catalog_name}"
            )))
        })?;
        if catalog.schema(schema_name).is_none() {
            return Err(crate::sql::model_error(DataFusionError::Plan(format!(
                "failed to resolve schema: {schema_name}"
            ))));
        }
        let charge = budget.reserve("completed-input-private-schema", size_of::<Self>() + 8192)?;
        let schema = Arc::new(Self {
            aliases,
            candidates: Mutex::new(BTreeMap::new()),
            budget: budget.clone(),
            _charge: charge,
        });
        catalog
            .register_schema(schema_name, schema)
            .map_err(crate::sql::model_error)?;
        Ok(())
    }
    fn immutable_mutation() -> DataFusionError {
        DataFusionError::External(Box::new(ModelError::Conflict(
            "captured completed input aliases are immutable",
        )))
    }
}
#[async_trait::async_trait]
impl SchemaProvider for InputSchema {
    fn table_names(&self) -> Vec<String> {
        let candidates = self
            .candidates
            .lock()
            .expect("completed-input candidate catalog");
        self.aliases
            .ports
            .keys()
            .chain(candidates.keys())
            .cloned()
            .collect()
    }
    fn table_exist(&self, name: &str) -> bool {
        self.aliases.ports.contains_key(name)
            || self
                .candidates
                .lock()
                .expect("completed-input candidate catalog")
                .contains_key(name)
    }
    async fn table(&self, name: &str) -> DataFusionResult<Option<Arc<dyn TableProvider>>> {
        if let Some(port) = self.aliases.ports.get(name) {
            return port
                .resolve()
                .await
                .map(Some)
                .map_err(|error| DataFusionError::External(Box::new(error)));
        }
        let candidates = self
            .candidates
            .lock()
            .map_err(|_| DataFusionError::External(Box::new(poisoned())))?;
        Ok(candidates
            .get(name)
            .map(|candidate| candidate.provider.clone()))
    }
    async fn table_type(
        &self,
        name: &str,
    ) -> DataFusionResult<Option<datafusion::logical_expr::TableType>> {
        if self.aliases.ports.contains_key(name) {
            return Ok(Some(datafusion::logical_expr::TableType::Base));
        }
        let candidates = self
            .candidates
            .lock()
            .map_err(|_| DataFusionError::External(Box::new(poisoned())))?;
        Ok(candidates
            .get(name)
            .map(|candidate| candidate.provider.table_type()))
    }
    fn register_table(
        &self,
        name: String,
        provider: Arc<dyn TableProvider>,
    ) -> DataFusionResult<Option<Arc<dyn TableProvider>>> {
        if self.aliases.ports.contains_key(&name) {
            return Err(Self::immutable_mutation());
        }
        let mut candidates = self
            .candidates
            .lock()
            .map_err(|_| DataFusionError::External(Box::new(poisoned())))?;
        if candidates.contains_key(&name) {
            return Err(DataFusionError::Execution(format!(
                "The table {name} already exists"
            )));
        }
        let charge = self
            .budget
            .reserve(
                "completed-input-candidate-alias",
                name.capacity().saturating_add(512),
            )
            .map_err(|error| DataFusionError::External(Box::new(error)))?;
        candidates.insert(
            name,
            Candidate {
                provider,
                _charge: charge,
            },
        );
        Ok(None)
    }
    fn deregister_table(&self, name: &str) -> DataFusionResult<Option<Arc<dyn TableProvider>>> {
        if self.aliases.ports.contains_key(name) {
            return Err(Self::immutable_mutation());
        }
        let mut candidates = self
            .candidates
            .lock()
            .map_err(|_| DataFusionError::External(Box::new(poisoned())))?;
        Ok(candidates.remove(name).map(|candidate| candidate.provider))
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use lctx_model::domain::{completed::CompletedView, input::Package, serving::Name};

    #[test]
    fn missing_default_catalog_refuses_without_inventing_a_catalog() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let aliases = InputAliases::ready(BTreeMap::new(), &budget).unwrap();
        let context = SessionContext::new_with_config(
            SessionConfig::new().with_create_default_catalog_and_schema(false),
        );
        let baseline = budget.reserved();
        assert!(InputSchema::install(&context, aliases.clone(), &budget).is_err());
        assert_eq!(budget.reserved(), baseline);
        assert!(
            context
                .catalog(&context.copied_config().options().catalog.default_catalog)
                .is_none()
        );
        drop(context);
        drop(aliases);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn unused_unearned_port_is_not_acquired_but_false_queries_still_refuse() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let view = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(b"unearned")].into(),
            0,
        )
        .unwrap();
        let charge = budget
            .reserve("unearned-port-control", size_of::<InputPort>() + 1024)
            .unwrap();
        let native = lctx_compiler_store::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_native::surrealdb::Surreal::init()),
            Name::new("unearned").unwrap(),
            Name::new("unearned").unwrap(),
        );
        let port = Arc::new(InputPort {
            private_cancellation: None,
            native: Some(NativeInput {
                native: native.clone(),
                view,
                relation: Relation::of::<Package>(),
                producing: None,
                budget: budget.clone(),
                batch_rows: 7,
                _files: Arc::new(WorkspaceFiles {
                    _directory: Arc::new(tempfile::tempdir().unwrap()),
                }),
            }),
            provider: tokio::sync::OnceCell::new(),
            _charge: charge,
        });
        let mut charge = StateCharge::new(&budget, "unearned-alias-control");
        charge.grow(1024).unwrap();
        let aliases = Arc::new(InputAliases {
            ports: [("unearned".into(), port.clone())].into(),
            _charge: charge,
        });
        let context = SessionContext::new();
        InputSchema::install(&context, aliases.clone(), &budget).unwrap();
        assert!(context.table_exist("unearned").unwrap());
        assert!(context.deregister_table("unearned").is_err());
        assert!(
            port.provider.get().is_none(),
            "metadata and refused mutation do not earn a port"
        );
        let independent = crate::sql::query(&context, "SELECT 1 AS value")
            .await
            .unwrap();
        assert!(
            port.provider.get().is_none(),
            "an unrelated query does not touch captured authority"
        );
        drop(independent);
        for sql in [
            "SELECT name FROM unearned WHERE false",
            "SELECT NULL FROM unearned LIMIT 0",
        ] {
            assert!(
                crate::sql::query(&context, sql).await.is_err(),
                "{sql} must earn authority before optimization"
            );
        }
        assert!(
            port.provider.get().is_none(),
            "refusal never publishes an unearned provider; the native owner controls retries and finality"
        );
        drop(context);
        drop(aliases);
        drop(port);
        let finality = native.drain_finality_report().await;
        assert!(
            finality.is_satisfied(),
            "unearned-port owner must actually drain: {finality:?}"
        );
        lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
        drop(native);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn captured_aliases_share_identity_and_private_candidates_remain_mutable() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = Package::encode(&[Package {
            name: "captured".into(),
        }])
        .unwrap();
        let provider: Arc<dyn TableProvider> =
            Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap());
        let port = InputPort::ready(provider.clone(), &budget).unwrap();
        let mut charge = StateCharge::new(&budget, "shared-alias-control");
        charge.grow(1024).unwrap();
        let aliases = Arc::new(InputAliases {
            ports: [
                ("plain".into(), port.clone()),
                ("qualified".into(), port.clone()),
            ]
            .into(),
            _charge: charge,
        });
        let left = SessionContext::new();
        let right = SessionContext::new();
        let catalog_name = left
            .copied_config()
            .options()
            .catalog
            .default_catalog
            .clone();
        let catalog = left.catalog(&catalog_name).unwrap();
        InputSchema::install(&left, aliases.clone(), &budget).unwrap();
        InputSchema::install(&right, aliases.clone(), &budget).unwrap();
        assert!(
            Arc::ptr_eq(&catalog, &left.catalog(&catalog_name).unwrap()),
            "install replaces only the existing default schema"
        );
        let plain = left.table_provider("plain").await.unwrap();
        let qualified = right.table_provider("qualified").await.unwrap();
        assert!(Arc::ptr_eq(&plain, &qualified));
        assert!(left.deregister_table("plain").is_err());
        assert!(left.register_table("plain", provider.clone()).is_err());
        let candidate_baseline = budget.reserved();
        let mut candidate_name = String::with_capacity(16 * 1024);
        candidate_name.push_str("candidate");
        let candidate_capacity = candidate_name.capacity();
        // SessionContext copies TableReference::table() into a fresh String.
        // Pass the wide buffer to the actual schema to exercise retained capacity.
        left.state()
            .schema_for_ref("candidate")
            .unwrap()
            .register_table(candidate_name, provider.clone())
            .unwrap();
        assert!(
            budget.reserved() >= candidate_baseline + candidate_capacity,
            "the retained key capacity is admitted, including spare capacity"
        );
        assert!(left.table_exist("candidate").unwrap());
        assert!(!right.table_exist("candidate").unwrap());
        assert!(left.register_table("candidate", provider.clone()).is_err());
        assert!(Arc::ptr_eq(
            &left.deregister_table("candidate").unwrap().unwrap(),
            &provider
        ));
        assert!(!left.table_exist("candidate").unwrap());
        assert!(left.deregister_table("candidate").unwrap().is_none());
        assert_eq!(
            budget.reserved(),
            candidate_baseline,
            "removing the candidate releases its key/node allowance; the base still owns an empty BTree root"
        );
        let pressure = budget
            .reserve(
                "candidate-metadata-control",
                budget.limit() - budget.reserved() - 64,
            )
            .unwrap();
        assert!(
            left.register_table("refused_candidate", provider.clone())
                .is_err()
        );
        assert!(
            !left.table_exist("refused_candidate").unwrap(),
            "metadata refusal precedes catalog mutation"
        );
        drop(pressure);
        drop(plain);
        drop(qualified);
        drop(left);
        drop(right);
        drop(catalog);
        drop(aliases);
        drop(port);
        drop(provider);
        assert_eq!(budget.reserved(), 0);
    }
}
