//! One attempt runtime; each declared stage gets fresh DataFusion catalogs and the shared pool.
mod prepared;
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::{
    catalog::Session,
    datasource::{MemTable, TableProvider},
    logical_expr::{Expr, TableType},
    physical_plan::ExecutionPlan,
};
use datafusion::{
    execution::{
        context::SessionContext,
        disk_manager::{DiskManagerBuilder, DiskManagerMode},
        memory_pool::{MemoryConsumer, MemoryPool, MemoryReservation, PeakRecordingPool},
        runtime_env::{RuntimeEnv, RuntimeEnvBuilder},
        session_state::SessionStateBuilder,
    },
    prelude::SessionConfig,
};
use lctx_model::domain::{
    ModelError, Record,
    resources::{
        DEFAULT_MEMORY_BYTES, DEFAULT_PARTITIONS, Reservation, ResourceBudget, ResourcePool,
    },
    stages::{CompletedRelation, InputTransport, ReadPermit, StageAccess, StageIdentity},
};
pub use prepared::{CollectedBatches, PreparedQuery};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct RuntimeOptions {
    pub memory_bytes: usize,
    pub partitions: usize,
}
impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            memory_bytes: DEFAULT_MEMORY_BYTES,
            partitions: DEFAULT_PARTITIONS,
        }
    }
}
pub struct AttemptRuntime {
    runtime: Arc<RuntimeEnv>,
    budget: ResourceBudget,
    partitions: usize,
    gate: Arc<tokio::sync::Mutex<()>>,
}
impl AttemptRuntime {
    pub fn new(options: RuntimeOptions) -> Result<Self, ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 {
            return Err(ModelError::Invalid(
                "runtime memory and partitions must be positive".into(),
            ));
        }
        let mut builder = RuntimeEnvBuilder::new().with_memory_limit(options.memory_bytes, 1.0);
        let pool = Arc::new(PeakRecordingPool::new(
            builder.memory_pool.take().expect("configured limit"),
        ));
        let runtime = builder
            .with_memory_pool(pool)
            .with_disk_manager_builder(
                DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled),
            )
            .build_arc()
            .map_err(ModelError::codec)?;
        let budget = ResourceBudget::from_pool(Arc::new(ComputePool {
            pool: runtime.memory_pool.clone(),
            limit: options.memory_bytes,
        }))?;
        Ok(Self {
            runtime,
            budget,
            partitions: options.partitions,
            gate: Arc::default(),
        })
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub(crate) fn context(&self, config: SessionConfig) -> SessionContext {
        SessionContext::new_with_config_rt(
            config.with_target_partitions(self.partitions),
            self.runtime.clone(),
        )
    }
    pub(crate) fn query_gate(&self) -> Arc<tokio::sync::Mutex<()>> {
        self.gate.clone()
    }
    pub fn session(&self, stage: &StageAccess<'_, '_>) -> StageSession {
        let config = SessionConfig::default()
            .set_bool("datafusion.sql_parser.enable_ident_normalization", false)
            .set_usize("datafusion.optimizer.hash_join_inlist_pushdown_max_size", 0)
            .set_usize(
                "datafusion.optimizer.hash_join_inlist_pushdown_max_distinct_values",
                0,
            )
            .with_target_partitions(self.partitions);
        let state = SessionStateBuilder::new()
            .with_default_features()
            .with_config(config)
            .with_runtime_env(self.runtime.clone())
            .build();
        StageSession {
            context: SessionContext::new_with_state(state),
            identity: stage.identity(),
            gate: self.gate.clone(),
            budget: self.budget.clone(),
            sources: std::sync::Mutex::new(Vec::new()),
            completed_tables: Default::default(),
        }
    }
}
/// Registration is capability-gated. Callers can plan/execute SQL but cannot introduce ambient
/// tables through a public SessionContext or clone another stage's catalog.
pub struct StageSession {
    context: SessionContext,
    identity: StageIdentity,
    gate: Arc<tokio::sync::Mutex<()>>,
    budget: ResourceBudget,
    sources: std::sync::Mutex<Vec<Arc<dyn TableProvider>>>,
    completed_tables: std::sync::Mutex<std::collections::BTreeSet<&'static str>>,
}
impl StageSession {
    /// Register only the model's five membership views, over already admitted typed sources.
    /// Logical views inline their plans, so remote-scan admission and retained source lifetime
    /// remain owned by PreparedQuery; no materialization or arbitrary registration escapes.
    pub async fn register_call_policy_views(&self) -> Result<(), ModelError> {
        use lctx_model::domain::normalized::events::CallPolicy;
        for name in CallPolicy::view_relations() {
            if !self
                .completed_tables
                .lock()
                .map_err(|_| ModelError::Invalid("stage source ownership poisoned".into()))?
                .contains(name)
            {
                return Err(ModelError::Invalid(format!(
                    "call policy view requires admitted source {name}"
                )));
            }
        }
        for policy in CallPolicy::ALL {
            if self
                .context
                .table_exist(policy.view_name())
                .map_err(ModelError::codec)?
            {
                return Err(ModelError::Invalid(
                    "call policy view already registered".into(),
                ));
            }
            let sql = policy.select_sql(|name| format!("\"{name}\""));
            let view = crate::sql::query(&self.context, &sql)
                .await
                .map_err(ModelError::codec)?
                .into_view();
            self.context
                .register_table(policy.view_name(), view)
                .map_err(ModelError::codec)?;
        }
        Ok(())
    }
    pub fn register<R: Record>(
        &self,
        permit: &ReadPermit<'_, R>,
        table: StageTable<R>,
    ) -> Result<(), ModelError> {
        if permit.identity() != self.identity
            || table.consumer != self.identity
            || table.source.as_ref() != permit.source()
            || table.transport != permit.transport()
        {
            return Err(ModelError::Invalid(
                "read permit belongs to another stage or attempt".into(),
            ));
        }
        if table.provider.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        if let Some(requirement) = permit.requirement() {
            table
                .availability
                .as_ref()
                .ok_or_else(|| {
                    ModelError::Invalid("read lacks validated scoped availability".into())
                })?
                .admit(requirement)?;
        }
        if self
            .context
            .table_exist(permit.relation())
            .map_err(ModelError::codec)?
        {
            return Err(ModelError::Invalid(format!(
                "stage relation already registered: {}",
                permit.relation()
            )));
        }
        self.context
            .register_table(permit.relation(), table.provider.clone())
            .map_err(ModelError::codec)?;
        if permit.transport() == InputTransport::CompletedStore {
            self.completed_tables
                .lock()
                .map_err(|_| ModelError::Invalid("stage source ownership poisoned".into()))?
                .insert(permit.relation());
        }
        self.sources
            .lock()
            .map_err(|_| ModelError::Invalid("stage source ownership poisoned".into()))?
            .push(table.provider);
        Ok(())
    }
    pub async fn query(&self, sql: &str) -> datafusion::error::Result<PreparedQuery> {
        let mut query =
            PreparedQuery::prepare(&self.context, sql, self.gate.clone(), self.budget.clone())
                .await?;
        query.retain_sources(
            self.sources
                .lock()
                .map_err(|_| {
                    datafusion::error::DataFusionError::Internal("stage sources poisoned".into())
                })?
                .clone(),
        );
        Ok(query)
    }
}

/// Source-bound table; callers cannot substitute an arbitrary schema-compatible provider.
pub struct StageTable<R: Record> {
    provider: Arc<dyn TableProvider>,
    consumer: StageIdentity,
    source: Option<CompletedRelation>,
    transport: InputTransport,
    availability: Option<Arc<lctx_model::domain::admission::ScopedAvailability>>,
    marker: std::marker::PhantomData<R>,
}
impl<R: Record> StageTable<R> {
    pub fn handoff(stage: &StageAccess<'_, '_>) -> Result<Self, ModelError> {
        let permit = stage.read::<R>()?;
        if permit.transport() != InputTransport::Handoff {
            return Err(ModelError::Invalid("store input is not a handoff".into()));
        }
        let batches = stage.handoff::<R>()?;
        let table = MemTable::try_new(
            R::schema(),
            vec![batches.iter().map(|b| b.arrow().clone()).collect()],
        )
        .map_err(ModelError::codec)?;
        Ok(Self {
            provider: Arc::new(HandoffTable {
                table,
                _batches: batches,
            }),
            consumer: stage.identity(),
            source: permit.source().cloned(),
            transport: InputTransport::Handoff,
            availability: None,
            marker: Default::default(),
        })
    }
    pub fn availability(&self) -> Option<&lctx_model::domain::admission::ScopedAvailability> {
        self.availability.as_deref()
    }
    pub(crate) fn completed(
        permit: &ReadPermit<'_, R>,
        provider: Arc<dyn TableProvider>,
        availability: Option<Arc<lctx_model::domain::admission::ScopedAvailability>>,
    ) -> Self {
        Self {
            provider,
            consumer: permit.identity(),
            source: permit.source().cloned(),
            transport: InputTransport::CompletedStore,
            availability,
            marker: Default::default(),
        }
    }
}
struct HandoffTable<R: Record> {
    table: MemTable,
    _batches: Vec<Arc<lctx_model::domain::Batch<R>>>,
}
impl<R: Record> std::fmt::Debug for HandoffTable<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HandoffTable").field(&R::NAME).finish()
    }
}
#[async_trait]
impl<R: Record> TableProvider for HandoffTable<R> {
    fn schema(&self) -> SchemaRef {
        R::schema()
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        self.table.scan(state, projection, filters, limit).await
    }
}
#[derive(Debug)]
struct ComputePool {
    pool: Arc<dyn MemoryPool>,
    limit: usize,
}
#[derive(Debug)]
struct ComputeReservation {
    value: MemoryReservation,
    owner: &'static str,
    pool: Arc<dyn MemoryPool>,
    limit: usize,
}
impl ResourcePool for ComputePool {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reservation = ComputeReservation {
            value: MemoryConsumer::new(owner).register(&self.pool),
            owner,
            pool: self.pool.clone(),
            limit: self.limit,
        };
        reservation.try_resize(bytes)?;
        Ok(Box::new(reservation))
    }
    fn reserved(&self) -> usize {
        self.pool.reserved()
    }
    fn limit(&self) -> usize {
        self.limit
    }
    fn peak(&self) -> Option<usize> {
        PeakRecordingPool::from_pool(self.pool.as_ref()).map(PeakRecordingPool::max_reserved)
    }
    fn reset_stage_peak(&self) {
        if let Some(pool) = PeakRecordingPool::from_pool(self.pool.as_ref()) {
            pool.reset_peak();
        }
    }
    fn stage_peak(&self) -> Option<usize> {
        PeakRecordingPool::from_pool(self.pool.as_ref()).map(PeakRecordingPool::peak_reserved)
    }
}
impl Reservation for ComputeReservation {
    fn size(&self) -> usize {
        self.value.size()
    }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.value
            .try_resize(bytes)
            .map_err(|_| ModelError::Resource {
                owner: self.owner,
                requested: bytes.saturating_sub(self.value.size()),
                used: self.pool.reserved(),
                limit: self.limit,
            })
    }
}
