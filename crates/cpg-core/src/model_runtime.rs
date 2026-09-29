//! One attempt runtime; each declared stage gets fresh DataFusion catalogs and the shared pool.
use std::sync::Arc;
use datafusion::{execution::{context::{SessionContext,SQLOptions}, memory_pool::{MemoryConsumer,MemoryPool,MemoryReservation},
    runtime_env::{RuntimeEnv,RuntimeEnvBuilder},session_state::SessionStateBuilder},prelude::SessionConfig};
use lctx_model::domain::{ModelError,Record,resources::{ResourceBudget,ResourcePool,Reservation,DEFAULT_MEMORY_BYTES,DEFAULT_PARTITIONS},stages::{ReadPermit,StageIdentity,StageAccess}};

#[derive(Debug,Clone,Copy)]
pub struct RuntimeOptions { pub memory_bytes: usize, pub partitions: usize }
impl Default for RuntimeOptions { fn default() -> Self { Self { memory_bytes: DEFAULT_MEMORY_BYTES,partitions: DEFAULT_PARTITIONS } } }
pub struct AttemptRuntime { runtime: Arc<RuntimeEnv>,budget: ResourceBudget,partitions: usize }
impl AttemptRuntime {
    pub fn new(options: RuntimeOptions) -> Result<Self,ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 { return Err(ModelError::Invalid("runtime memory and partitions must be positive".into())); }
        let runtime = RuntimeEnvBuilder::new().with_memory_limit(options.memory_bytes,1.0).build_arc().map_err(ModelError::codec)?;
        let budget = ResourceBudget::from_pool(Arc::new(ComputePool { pool: runtime.memory_pool.clone(),limit: options.memory_bytes }))?;
        Ok(Self { runtime,budget,partitions: options.partitions })
    }
    pub fn budget(&self) -> &ResourceBudget { &self.budget }
    pub fn session(&self, stage: &StageAccess<'_, '_>) -> StageSession {
        let config = SessionConfig::default().set_bool("datafusion.sql_parser.enable_ident_normalization",false)
            .set_usize("datafusion.optimizer.hash_join_inlist_pushdown_max_size",0)
            .set_usize("datafusion.optimizer.hash_join_inlist_pushdown_max_distinct_values",0)
            .with_target_partitions(self.partitions);
        let state = SessionStateBuilder::new().with_default_features().with_config(config).with_runtime_env(self.runtime.clone()).build();
        StageSession { context: SessionContext::new_with_state(state), identity: stage.identity() }
    }
}
/// Registration is capability-gated. Callers can plan/execute SQL but cannot introduce ambient
/// tables through a public SessionContext or clone another stage's catalog.
pub struct StageSession { context: SessionContext,identity: StageIdentity }
impl StageSession {
    pub fn register<R: Record>(&self, permit: &ReadPermit<'_,R>, table: Arc<dyn datafusion::datasource::TableProvider>) -> Result<(),ModelError> {
        if permit.identity() != self.identity { return Err(ModelError::Invalid("read permit belongs to another stage or attempt".into())); }
        if table.schema().as_ref() != R::schema().as_ref() { return Err(ModelError::Schema(R::NAME)); }
        if self.context.table_exist(permit.relation()).map_err(ModelError::codec)? { return Err(ModelError::Invalid("stage relation already registered".into())); }
        self.context.register_table(permit.relation(),table).map_err(ModelError::codec)?; Ok(())
    }
    pub async fn sql(&self, sql: &str) -> datafusion::error::Result<datafusion::dataframe::DataFrame> {
        self.context.sql_with_options(sql,SQLOptions::new().with_allow_ddl(false).with_allow_dml(false).with_allow_statements(false)).await
    }
}
#[derive(Debug)]
struct ComputePool { pool: Arc<dyn MemoryPool>,limit: usize }
#[derive(Debug)]
struct ComputeReservation { value: MemoryReservation,owner: &'static str,pool: Arc<dyn MemoryPool>,limit: usize }
impl ResourcePool for ComputePool {
    fn reserve(&self, owner: &'static str, bytes: usize) -> Result<Box<dyn Reservation>,ModelError> {
        let mut reservation = ComputeReservation { value: MemoryConsumer::new(owner).register(&self.pool),owner,pool: self.pool.clone(),limit: self.limit };
        reservation.try_resize(bytes)?; Ok(Box::new(reservation))
    }
    fn reserved(&self) -> usize { self.pool.reserved() }
    fn limit(&self) -> usize { self.limit }
}
impl Reservation for ComputeReservation {
    fn size(&self) -> usize { self.value.size() }
    fn try_resize(&mut self, bytes: usize) -> Result<(),ModelError> {
        self.value.try_resize(bytes).map_err(|_| ModelError::Resource { owner: self.owner,requested: bytes.saturating_sub(self.value.size()),used: self.pool.reserved(),limit: self.limit })
    }
}
