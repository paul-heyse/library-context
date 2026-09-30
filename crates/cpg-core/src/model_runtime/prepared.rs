//! Read-only physical plans with whole-query scan admission and charged collected output.
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::{
    catalog::memory::MemorySourceConfig,
    datasource::source::DataSourceExec,
    error::{DataFusionError, Result},
    execution::{TaskContext, context::SessionContext},
    physical_plan::{
        ExecutionPlan, ExecutionPlanProperties, RecordBatchStream, SendableRecordBatchStream,
        empty::EmptyExec, memory::LazyMemoryExec, placeholder_row::PlaceholderRowExec,
    },
};
use datafusion_table_providers_postgres::pool::{PoolHealth, PostgresConnectionPool};
use futures::{Stream, TryStreamExt};
use lctx_model::domain::{charged::StateCharge, resources::ResourceBudget};
use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::{Mutex, OwnedMutexGuard};

struct Demand {
    pool: Arc<PostgresConnectionPool>,
    count: usize,
    capacity: usize,
}
/// No DataFrame or ExecutionPlan escapes this wrapper, so execution cannot bypass admission.
pub struct PreparedQuery {
    plan: Arc<dyn ExecutionPlan>,
    context: Arc<TaskContext>,
    demands: Vec<Demand>,
    gate: Arc<Mutex<()>>,
    budget: ResourceBudget,
    sources: Vec<Arc<dyn datafusion::datasource::TableProvider>>,
}
impl std::fmt::Debug for PreparedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedQuery")
            .field("plan", &self.plan.name())
            .field("scan_demand", &self.scan_demand())
            .finish_non_exhaustive()
    }
}
impl PreparedQuery {
    pub(crate) async fn prepare(
        context: &SessionContext,
        sql: &str,
        gate: Arc<Mutex<()>>,
        budget: ResourceBudget,
    ) -> Result<Self> {
        let frame = crate::sql::query(context, sql).await?;
        Self::from_frame(context, frame, gate, budget).await
    }
    pub(crate) async fn from_frame(
        context: &SessionContext,
        frame: datafusion::dataframe::DataFrame,
        gate: Arc<Mutex<()>>,
        budget: ResourceBudget,
    ) -> Result<Self> {
        let plan = frame.create_physical_plan().await?;
        let mut demands = BTreeMap::new();
        count_scans(&plan, &mut demands)?;
        for demand in demands.values() {
            if demand.count > demand.capacity {
                return Err(DataFusionError::ResourcesExhausted(format!(
                    "physical plan needs {} simultaneous remote scans; provider capacity is {}",
                    demand.count, demand.capacity
                )));
            }
        }
        Ok(Self {
            plan,
            context: context.task_ctx(),
            demands: demands.into_values().collect(),
            gate,
            budget,
            sources: Vec::new(),
        })
    }
    pub(crate) fn retain_sources(
        &mut self,
        sources: Vec<Arc<dyn datafusion::datasource::TableProvider>>,
    ) {
        self.sources = sources;
    }
    pub fn scan_demand(&self) -> usize {
        self.demands.iter().map(|d| d.count).sum()
    }
    pub fn schema(&self) -> SchemaRef {
        self.plan.schema()
    }
    pub async fn execute_stream(self) -> Result<SendableRecordBatchStream> {
        let guard = self.gate.try_lock_owned().map_err(|_| {
            DataFusionError::ResourcesExhausted("another query is active in this attempt".into())
        })?;
        // All demand is admitted before execute can acquire its first connection. A cancelled
        // predecessor still draining is unavailable capacity; no partial acquisitions queue.
        for demand in &self.demands {
            match demand.pool.health() {
                PoolHealth::Ready { idle, .. } if idle as usize >= demand.count => {}
                PoolHealth::Ready { .. } => {
                    return Err(DataFusionError::ResourcesExhausted(
                        "provider capacity is still in use or draining".into(),
                    ));
                }
                PoolHealth::Closing | PoolHealth::Closed => {
                    return Err(DataFusionError::External(Box::new(
                        crate::generation_read::ReadError::Closed,
                    )));
                }
                _ => {
                    return Err(DataFusionError::External(Box::new(
                        crate::generation_read::ReadError::Lost,
                    )));
                }
            }
        }
        let stream = datafusion::physical_plan::execute_stream(self.plan, self.context)?;
        Ok(Box::pin(QueryStream {
            stream,
            _guard: guard,
            _sources: self.sources,
        }))
    }
    pub async fn collect(self) -> Result<CollectedBatches> {
        let mut charge = StateCharge::new(&self.budget, "query-collected-output");
        let mut stream = self.execute_stream().await?;
        let mut batches = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            charge
                .grow(batch.get_array_memory_size())
                .map_err(|e| DataFusionError::External(Box::new(e)))?;
            batches.push(batch);
        }
        Ok(CollectedBatches {
            batches,
            _charge: charge,
        })
    }
}
pub struct CollectedBatches {
    batches: Vec<RecordBatch>,
    _charge: StateCharge,
}
impl std::ops::Deref for CollectedBatches {
    type Target = [RecordBatch];
    fn deref(&self) -> &Self::Target {
        &self.batches
    }
}
impl std::fmt::Debug for CollectedBatches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.batches.fmt(f)
    }
}
struct QueryStream {
    stream: SendableRecordBatchStream,
    _guard: OwnedMutexGuard<()>,
    _sources: Vec<Arc<dyn datafusion::datasource::TableProvider>>,
}
impl Stream for QueryStream {
    type Item = Result<RecordBatch>;
    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(context)
    }
}
impl RecordBatchStream for QueryStream {
    fn schema(&self) -> SchemaRef {
        self.stream.schema()
    }
}

fn count_scans(plan: &Arc<dyn ExecutionPlan>, demands: &mut BTreeMap<usize, Demand>) -> Result<()> {
    if let Some(scan) = plan.downcast_ref::<crate::generation_read::GenerationScan>() {
        let (pool, capacity) = scan.scan_pool();
        let count = plan.output_partitioning().partition_count();
        let demand = demands
            .entry(Arc::as_ptr(&pool) as usize)
            .or_insert(Demand {
                pool,
                count: 0,
                capacity,
            });
        demand.count = demand
            .count
            .checked_add(count)
            .ok_or_else(|| DataFusionError::Plan("remote scan demand overflow".into()))?;
        return Ok(());
    }
    let children = plan.children();
    if children.is_empty() {
        let memory = plan.downcast_ref::<DataSourceExec>().is_some_and(|p| {
            p.data_source()
                .downcast_ref::<MemorySourceConfig>()
                .is_some()
        });
        if !(memory
            || plan.downcast_ref::<EmptyExec>().is_some()
            || plan.downcast_ref::<PlaceholderRowExec>().is_some()
            || plan.downcast_ref::<LazyMemoryExec>().is_some())
        {
            return Err(DataFusionError::Plan(format!(
                "unadmitted scan source {}",
                plan.name()
            )));
        }
    }
    // Count each occurrence, including aliases and repeated shared plan pointers. Deduplicating
    // nodes would undercount simultaneously live scans of the same relation.
    for child in children {
        count_scans(child, demands)?;
    }
    Ok(())
}
