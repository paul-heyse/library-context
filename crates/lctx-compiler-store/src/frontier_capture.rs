//! Invocation-local successful native frontier captures. IPC is private preparation, never
//! completed-input authority. Coverage is published only after checked EOF and file finality.
use crate::{admission::Cancellation, consumed_rows::{PreparedEdges, coverage_index::CoverageIndex}, invocation_ipc::{InvocationIpc, InvocationRead, SealedStream, IpcStreamRead}};
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use arrow_schema::SchemaRef;
use datafusion::physical_plan::{SendableRecordBatchStream, RecordBatchStream};
use futures::{Stream, TryStreamExt};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use std::{collections::{BTreeMap, BTreeSet}, future::Future, sync::Arc};

struct Group {
    schema: SchemaRef,
    key_field: &'static str,
    windows: Vec<Window>,
    indexes: Vec<(usize, CoverageIndex)>,
}
struct Window { rows: Arc<SealedStream> }
/// Exact immutable source and budget ownership survive local membership/partition retries.
/// Failed source acquisition is terminal; successful windows are never reissued.
pub struct FrontierCapture {
    owner: PreparedEdges,
    ipc: InvocationIpc,
    budget: ResourceBudget,
    cancellation: Cancellation,
    groups: BTreeMap<String, Group>,
    charge: StateCharge,
    acquisition_failed: bool,
}
impl FrontierCapture {
    pub(crate) fn new(owner: PreparedEdges, budget: &ResourceBudget, cancellation: &Cancellation) -> Result<Self, ModelError> {
        let ipc = InvocationIpc::new(owner.session(), budget, cancellation, None)?;
        Ok(Self { owner, ipc, budget: budget.clone(), cancellation: cancellation.clone(),
            groups: BTreeMap::new(), charge: StateCharge::new(budget, "native-frontier-capture"),
            acquisition_failed: false })
    }
    pub(crate) fn require_owner(&self, owner: &PreparedEdges, budget: &ResourceBudget) -> Result<(), ModelError> {
        if self.acquisition_failed { return Err(ModelError::Conflict("failed native frontier acquisition")); }
        if !self.owner.shares_owner(owner) || !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict("native frontier capture owner"));
        }
        Ok(())
    }
    pub fn permits_local_retry(&self) -> bool { !self.acquisition_failed }
    pub(crate) fn begin_acquisition(&mut self) { self.acquisition_failed = true; }
    pub(crate) fn complete_acquisition(&mut self) { self.acquisition_failed = false; }
    async fn prepare_group(&mut self, name: &str) -> Result<(), ModelError> {
        let group = self.groups.get_mut(name).expect("capture group");
        // Compaction is optional pure work over successful immutable runs. A refusal
        // leaves every run routable, so a narrower membership retry need not repeat it
        // or reacquire any source. Other errors still retain their full meaning.
        loop {
            let pair = group.indexes.iter().enumerate().find_map(|(a, (level, _))| {
                group.indexes.iter().enumerate().skip(a + 1)
                    .find(|(_, (other, _))| other == level).map(|(b, _)| (a, b, *level))
            });
            let Some((a, b, level)) = pair else { break; };
            let merged = match group.indexes[a].1.merge(&group.indexes[b].1, &self.budget, &self.cancellation).await {
                Ok(merged) => merged,
                Err(ModelError::Resource { .. }) => break,
                Err(error) => return Err(error),
            };
            group.indexes[a] = (level + 1, merged);
            group.indexes.remove(b);
        }
        Ok(())
    }
    fn routes(&self, group: &str, keys: &[[u8;16]]) -> Result<BTreeMap<[u8;16], usize>, ModelError> {
        let mut routes = BTreeMap::new();
        for (_, index) in &self.groups[group].indexes {
            index.lookup(keys, &self.budget, |key, window| {
                if routes.insert(key, window).is_some() { return Err(ModelError::Conflict("duplicate native capture coverage")); }
                Ok(())
            })?;
        }
        Ok(routes)
    }
    pub(crate) async fn read<F, Fut, Prepared, Start>(
        &mut self, group: String, key_field: &'static str, keys: Arc<Vec<[u8; 16]>>,
        schema: SchemaRef, prepare: F, start: Start,
    ) -> Result<SendableRecordBatchStream, ModelError>
    where F: FnOnce(Arc<Vec<[u8; 16]>>) -> Fut,
        Fut: Future<Output = Result<Prepared, ModelError>>,
        Start: FnOnce(Prepared) -> Result<SendableRecordBatchStream, ModelError>,
    {
        self.cancellation.check()?;
        if self.acquisition_failed { return Err(ModelError::Conflict("failed native frontier acquisition")); }
        if keys.windows(2).any(|pair| pair[0] >= pair[1]) { return Err(ModelError::Schema("native capture key order")); }
        if let Some(old) = self.groups.get(&group) {
            if old.schema != schema || old.key_field != key_field { return Err(ModelError::Conflict("native capture query shape")); }
        } else {
            self.charge.grow(512 + group.len() + schema.fields().iter().map(|f| f.size()).sum::<usize>())?;
            self.groups.insert(group.clone(), Group { schema: schema.clone(), key_field, windows: Vec::new(),
                indexes: Vec::new() });
        }
        let _keys = self.budget.reserve("native-capture-key-window", keys.len() * 256 + 4096)?;
        self.prepare_group(&group).await?;
        let routes = self.routes(&group, &keys)?;
        let mut selected = routes.values().copied().collect::<BTreeSet<_>>();
        let missing = Arc::new(keys.iter().copied().filter(|key| !routes.contains_key(key)).collect::<Vec<_>>());
        // Empty demand still checks source authority once. A complete-empty physical result
        // earns the same exact input keys as a nonempty result.
        if !missing.is_empty() || self.groups[&group].windows.is_empty() {
            let window_ordinal = self.groups[&group].windows.len();
            let index = if missing.is_empty() { None } else { Some(CoverageIndex::new(&missing, window_ordinal, &self.budget)?) };
            let windows = &mut self.groups.get_mut(&group).expect("capture group").windows;
            if windows.len() == windows.capacity() {
                let additional = windows.capacity().max(4);
                self.charge.grow(additional * size_of::<Window>())?;
                windows.reserve_exact(additional);
            }
            if index.is_some() {
                let indexes = &mut self.groups.get_mut(&group).expect("capture group").indexes;
                if indexes.len() == indexes.capacity() {
                    let additional = indexes.capacity().max(4);
                    self.charge.grow(additional * size_of::<(usize, CoverageIndex)>())?;
                    indexes.reserve_exact(additional);
                }
            }
            let mut writer = self.ipc.begin_stream(schema.clone())?;
            let _buffer = self.budget.reserve("native-capture-nominal-buffer",
                lctx_model::domain::resources::TRANSFER_ROWS * schema.fields().len() * 64 + 4096)?;
            let mut buffer = NominalBuffer::new(schema.clone())?;
            // Only this closure's local plan construction may refuse without poisoning
            // acquisition. Provider earning is guarded by the caller before preparation;
            // start admits the producing stream and must never be replayed on failure.
            let prepared = prepare(missing.clone()).await?;
            self.begin_acquisition();
            let mut source = start(prepared)?;
            if source.schema() != schema { return Err(ModelError::Schema("native capture issued schema")); }
            let mut previous = None;
            while let Some(batch) = source.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                let column = batch.column_by_name(key_field).and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema("native capture nominal key"))?;
                let ids = batch.column_by_name("id").and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema("native capture nominal row ID"))?;
                for row in 0..batch.num_rows() {
                    if ids.is_null(row) { return Err(ModelError::Schema("native capture null row ID")); }
                    let id: [u8;16] = ids.value(row).try_into().map_err(ModelError::codec)?;
                    if previous.is_some_and(|old| old >= id) { return Err(ModelError::Conflict("native capture source row order")); }
                    previous = Some(id);
                    if column.is_null(row) && key_field != "id" { continue; }
                    if column.is_null(row) { return Err(ModelError::Schema("native capture null key")); }
                    let key: [u8;16] = column.value(row).try_into().map_err(ModelError::codec)?;
                    if missing.binary_search(&key).is_err() { return Err(ModelError::Conflict("native capture source demand")); }
                }
                buffer.push(&batch, |batch| writer.push(&batch))?;
                tokio::task::yield_now().await;
            }
            drop(source);
            if let Some(batch) = buffer.finish()? { writer.push(&batch)?; }
            drop(buffer);
            let rows = writer.seal()?;
            let state = self.groups.get_mut(&group).expect("capture group");
            state.windows.push(Window { rows });
            if let Some(index) = index { state.indexes.push((0, index)); }
            self.complete_acquisition();
            drop(_buffer);
            selected.insert(window_ordinal);
            self.prepare_group(&group).await?;
        }
        let windows = &self.groups[&group].windows;
        // Only exactly intersecting captures participate in this query. A disjoint frontier
        // neither scans old result files nor rebuilds a logical union over its history.
        let retained = self.budget.reserve("native-capture-local-query",
            selected.len().max(1) * size_of::<Arc<SealedStream>>() * 3 + keys.len() * 128 + 4096)?;
        let empty = selected.is_empty();
        let ordered = selected.len() <= 1;
        let inputs = if empty { vec![windows[0].rows.clone()] } else {
            selected.iter().map(|index| windows[*index].rows.clone()).collect()
        };
        let partition = Arc::new(CapturePartition { schema: schema.clone(), inputs });
        let mut provider = datafusion::catalog::streaming::StreamingTable::try_new(schema.clone(), vec![partition])
            .map_err(crate::sql::model_error)?;
        // Each issued native window was checked in strict ID order. Across distinct
        // windows we retain the ordinary spill-capable sort rather than claim ordering.
        if ordered { provider = provider.with_sort_order(vec![datafusion::prelude::col("id").sort(true, true)]); }
        let reader = self.ipc.reader()?;
        reader.session().register_table("captured", Arc::new(provider)).map_err(crate::sql::model_error)?;
        let sql = if empty { "SELECT * FROM captured WHERE FALSE".to_owned() } else {
            format!("SELECT * FROM captured WHERE {} ORDER BY id", key_predicate(key_field, &keys))
        };
        let frame = crate::sql::query(reader.session(), &sql).await.map_err(crate::sql::model_error)?;
        // Merely preparing a read must not start physical tasks. In particular, dropping
        // an unpolled stream releases its private carriers synchronously; actual execution
        // and its normal parallelism begin only when the consumer demands the first batch.
        let deferred = futures::stream::once(async move { frame.execute_stream().await }).try_flatten();
        let stream = Box::pin(datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(schema.clone(), deferred));
        Ok(Box::pin(CapturedRead { inner: Some(stream), reader: Some(reader), query: Some(retained), schema }))

    }
}
struct CapturePartition { schema: SchemaRef, inputs: Vec<Arc<SealedStream>> }
impl std::fmt::Debug for CapturePartition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("CapturePartition").field("windows", &self.inputs.len()).finish() }
}
impl datafusion::physical_plan::streaming::PartitionStream for CapturePartition {
    fn schema(&self) -> &SchemaRef { &self.schema }
    fn execute(&self, _context: Arc<datafusion::execution::TaskContext>) -> SendableRecordBatchStream {
        let inputs = self.inputs.clone();
        let stream = futures::stream::try_unfold((inputs, 0usize, None::<IpcStreamRead>), |(inputs, mut index, mut active)| async move {
            loop {
                if active.is_none() {
                    let Some(input) = inputs.get(index) else { return Ok::<_, datafusion::error::DataFusionError>(None); };
                    active = Some(input.open().map_err(|error| datafusion::error::DataFusionError::External(Box::new(error)))?);
                }
                match active.as_mut().expect("active stream reader").next()
                    .map_err(|error| datafusion::error::DataFusionError::External(Box::new(error)))? {
                    Some(batch) => {
                        tokio::task::yield_now().await;
                        return Ok(Some((batch, (inputs, index, active))));
                    }
                    None => { drop(active.take()); index += 1; }
                }
            }
        });
        Box::pin(datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(self.schema.clone(), stream))
    }
}
struct CapturedRead {
    inner: Option<SendableRecordBatchStream>,
    reader: Option<InvocationRead>,
    query: Option<Box<dyn lctx_model::domain::resources::Reservation>>,
    schema: SchemaRef,
}
impl CapturedRead {
    fn finish(&mut self) {
        drop(self.inner.take());
        drop(self.reader.take());
        drop(self.query.take());
    }
}
impl Unpin for CapturedRead {}
impl Stream for CapturedRead {
    type Item = Result<RecordBatch, datafusion::error::DataFusionError>;
    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let Some(inner) = &mut this.inner else { return std::task::Poll::Ready(None); };
        let result = inner.as_mut().poll_next(cx);
        if matches!(&result, std::task::Poll::Ready(None | Some(Err(_)))) { this.finish(); }
        result
    }
}
impl RecordBatchStream for CapturedRead { fn schema(&self) -> SchemaRef { self.schema.clone() } }
impl Drop for CapturedRead { fn drop(&mut self) { self.finish(); } }
fn key_predicate(field: &str, keys: &[[u8; 16]]) -> String {
    if keys.is_empty() { return "FALSE".into(); }
    format!("{} IN ({})", crate::consumed_rows::identifier(field),
        keys.iter().map(|key| format!("X'{}'", hex::encode(key))).collect::<Vec<_>>().join(","))
}
/// Projected native frontiers contain only nominal IDs and fixed namespace ordinals. Copy
/// tiny fragments into one fixed charged window rather than retaining one footer per fragment.
enum Column {
    Id(arrow_array::builder::FixedSizeBinaryBuilder),
    Kind(arrow_array::builder::Int64Builder),
}
struct NominalBuffer { schema: SchemaRef, columns: Vec<Column>, rows: usize }
impl NominalBuffer {
    fn new(schema: SchemaRef) -> Result<Self, ModelError> {
        let capacity = lctx_model::domain::resources::TRANSFER_ROWS;
        let columns = schema.fields().iter().map(|field| Ok(match field.data_type() {
            arrow_schema::DataType::FixedSizeBinary(16) => Column::Id(arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(capacity, 16)),
            arrow_schema::DataType::Int64 => Column::Kind(arrow_array::builder::Int64Builder::with_capacity(capacity)),
            _ => return Err(ModelError::Schema("native capture nominal projection")),
        })).collect::<Result<Vec<_>, ModelError>>()?;
        Ok(Self { schema, columns, rows: 0 })
    }
    fn push(&mut self, batch: &RecordBatch, mut emit: impl FnMut(RecordBatch) -> Result<(), ModelError>) -> Result<(), ModelError> {
        if batch.schema() != self.schema { return Err(ModelError::Schema("native capture batch shape")); }
        for row in 0..batch.num_rows() {
            for (column, values) in self.columns.iter_mut().zip(batch.columns()) {
                match column {
                    Column::Id(builder) => {
                        let values = values.as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("native capture nominal ID"))?;
                        if values.is_null(row) { builder.append_null(); }
                        else { builder.append_value(values.value(row)).map_err(ModelError::codec)?; }
                    }
                    Column::Kind(builder) => {
                        let values = values.as_any().downcast_ref::<arrow_array::Int64Array>().ok_or(ModelError::Schema("native capture namespace"))?;
                        if values.is_null(row) { builder.append_null(); } else { builder.append_value(values.value(row)); }
                    }
                }
            }
            self.rows += 1;
            if self.rows == lctx_model::domain::resources::TRANSFER_ROWS { emit(self.finish()?.expect("full nominal window"))?; }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<Option<RecordBatch>, ModelError> {
        if self.rows == 0 { return Ok(None); }
        let columns = self.columns.iter_mut().map(|column| match column {
            Column::Id(builder) => Arc::new(builder.finish()) as arrow_array::ArrayRef,
            Column::Kind(builder) => Arc::new(builder.finish()) as arrow_array::ArrayRef,
        }).collect();
        self.rows = 0;
        Ok(Some(RecordBatch::try_new(self.schema.clone(), columns).map_err(ModelError::codec)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumed_rows::{ClosureTable, NominalClosure};
    use datafusion::{error::DataFusionError, prelude::SessionContext, physical_plan::stream::RecordBatchStreamAdapter};
    use lctx_model::domain::{Relation, input::Package};
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn schema() -> SchemaRef {
        Arc::new(arrow_schema::Schema::new(vec![arrow_schema::Field::new(
            "id", arrow_schema::DataType::FixedSizeBinary(16), false)]))
    }
    fn batch(keys: &[[u8;16]]) -> RecordBatch {
        RecordBatch::try_new(schema(), vec![Arc::new(FixedSizeBinaryArray::try_from_iter(keys.iter().copied()).unwrap())]).unwrap()
    }
    fn stream(rows: Vec<Result<RecordBatch, DataFusionError>>) -> SendableRecordBatchStream {
        Box::pin(RecordBatchStreamAdapter::new(schema(), futures::stream::iter(rows)))
    }
    const START_STREAM: fn(SendableRecordBatchStream) -> Result<SendableRecordBatchStream, ModelError> = Ok;
    async fn owner(budget: &ResourceBudget) -> PreparedEdges {
        NominalClosure::new(vec![ClosureTable { alias: "unused".into(), relation: Relation::of::<Package>() }])
            .unwrap().prepare(&SessionContext::new(), budget).await.unwrap()
    }
    async fn ids(mut rows: SendableRecordBatchStream) -> Vec<[u8;16]> {
        let mut result = Vec::new();
        while let Some(batch) = rows.try_next().await.unwrap() {
            let ids = batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
            for row in 0..ids.len() { result.push(ids.value(row).try_into().unwrap()); }
        }
        result
    }
    #[tokio::test]
    async fn successful_windows_and_complete_empty_keys_survive_local_retry_without_reissue() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let issued = AtomicUsize::new(0);
            let first = capture.read("ids".into(), "id", Arc::new(vec![[1;16], [2;16], [3;16]]), schema(), |keys| {
                issued.fetch_add(1, Ordering::Relaxed);
                assert_eq!(&*keys, &[[1;16], [2;16], [3;16]]);
                async { Ok(stream(vec![Ok(batch(&[[1;16], [3;16]]))])) }
            }, START_STREAM).await.unwrap();
            assert_eq!(ids(first).await, [[1;16], [3;16]]);
            assert!(capture.permits_local_retry());
            // Simulate the later local membership refusal, preserving its typed cause.
            let refused = budget.reserve("local-membership-control", budget.limit() + 1).unwrap_err();
            assert!(matches!(refused, ModelError::Resource { .. }));
            let second = capture.read("ids".into(), "id", Arc::new(vec![[2;16], [3;16], [4;16]]), schema(), |keys| {
                issued.fetch_add(1, Ordering::Relaxed);
                assert_eq!(&*keys, &[[4;16]]);
                async { Ok(stream(vec![Ok(batch(&[[4;16]]))])) }
            }, START_STREAM).await.unwrap();
            assert_eq!(ids(second).await, [[3;16], [4;16]]);
            let empty = capture.read("ids".into(), "id", Arc::new(vec![[2;16]]), schema(), |_| async {
                panic!("complete-empty source key must not be reissued")
            }, START_STREAM).await.unwrap();
            assert!(ids(empty).await.is_empty());
            assert_eq!(issued.load(Ordering::Relaxed), 2);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn pure_plan_refusal_preserves_successful_coverage_and_permits_narrower_demand() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let started = AtomicUsize::new(0);
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                Ok(stream(vec![Ok(batch(&[[1;16]]))]))
            }, |source| { started.fetch_add(1, Ordering::Relaxed); Ok(source) }).await.unwrap();
            assert_eq!(ids(rows).await, [[1;16]]);
            let preparation_budget = &budget;
            let error = capture.read("ids".into(), "id", Arc::new(vec![[1;16],[2;16],[3;16]]), schema(), |keys| async move {
                assert_eq!(&*keys, &[[2;16],[3;16]]);
                let _preparation = preparation_budget.reserve("pure-plan-control", preparation_budget.limit() + 1)?;
                Ok(stream(vec![]))
            }, |_| { panic!("pure plan refusal cannot start the native stream") }).await.err().unwrap();
            assert!(matches!(error, ModelError::Resource { owner: "pure-plan-control", .. }));
            assert!(capture.permits_local_retry());
            capture.require_owner(&owner, &budget).unwrap();
            assert_eq!(capture.groups["ids"].windows.len(), 1);
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16],[2;16]]), schema(), |keys| async move {
                assert_eq!(&*keys, &[[2;16]], "successful input must not be reacquired");
                Ok(stream(vec![Ok(batch(&keys))]))
            }, |source| { started.fetch_add(1, Ordering::Relaxed); Ok(source) }).await.unwrap();
            assert_eq!(ids(rows).await, [[1;16],[2;16]]);
            assert_eq!(started.load(Ordering::Relaxed), 2);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn stream_start_refusal_and_unknown_are_terminal_before_first_batch() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        for error in [
            ModelError::Resource { owner: "producing-start-control", requested: 2, used: 1, limit: 1 },
            ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed, "source acknowledgement unknown"),
        ] {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let failure = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                Ok(stream(vec![]))
            }, |_| Err(error)).await.err().unwrap();
            assert!(failure.to_string().contains("producing-start-control") || failure.to_string().contains("source acknowledgement unknown"));
            assert!(!capture.permits_local_retry());
            assert!(capture.groups["ids"].windows.is_empty());
            assert!(capture.require_owner(&owner, &budget).is_err());
            assert!(capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("admitted or unknown acquisition cannot be reissued")
            }, START_STREAM).await.is_err());
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn late_acquisition_refusal_is_terminal_and_never_publishes_coverage() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let error = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                Ok(stream(vec![Ok(batch(&[[1;16]])), Err(DataFusionError::External(Box::new(
                    ModelError::Resource { owner: "active-native-control", requested: 2, used: 1, limit: 1 })))]))
            }, START_STREAM).await.err().unwrap();
            assert!(error.to_string().contains("active-native-control"));
            assert!(!capture.permits_local_retry());
            assert!(capture.groups["ids"].windows.is_empty());
            assert!(capture.require_owner(&owner, &budget).is_err());
            assert!(capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("failed acquisition cannot be retried")
            }, START_STREAM).await.is_err());
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn exact_owner_and_empty_authority_are_required() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let foreign = self::owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            assert!(capture.require_owner(&foreign, &budget).is_err());
            let issued = AtomicUsize::new(0);
            let rows = capture.read("empty".into(), "id", Arc::new(vec![]), schema(), |keys| {
                issued.fetch_add(1, Ordering::Relaxed);
                assert!(keys.is_empty());
                async { Ok(stream(vec![])) }
            }, START_STREAM).await.unwrap();
            assert!(ids(rows).await.is_empty());
            let rows = capture.read("empty".into(), "id", Arc::new(vec![]), schema(), |_| async {
                panic!("same earned empty demand does not reissue")
            }, START_STREAM).await.unwrap();
            assert!(ids(rows).await.is_empty());
            assert_eq!(issued.load(Ordering::Relaxed), 1);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn disjoint_history_routes_only_intersecting_windows_through_logarithmic_runs() {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            for n in 1..=32u8 {
                let rows = capture.read("ids".into(), "id", Arc::new(vec![[n;16]]), schema(), |keys| async move {
                    Ok(stream(vec![Ok(batch(&keys))]))
                }, START_STREAM).await.unwrap();
                assert_eq!(ids(rows).await, [[n;16]]);
            }
            let _keys = budget.reserve("index-route-control", 4096).unwrap();
            let routes = capture.routes("ids", &[[3;16],[29;16],[99;16]]).unwrap();
            assert_eq!(routes, BTreeMap::from([([3;16],2),([29;16],28)]));
            assert_eq!(capture.groups["ids"].indexes.len(), 1);
            assert_eq!(capture.groups["ids"].windows.len(), 32);
            // No data-window provider remains registered in the persistent writer owner.
            assert!(capture.ipc.session().catalog_names().iter().all(|name| {
                let catalog = capture.ipc.session().catalog(name).unwrap();
                catalog.schema_names().iter().all(|name| catalog.schema(name).unwrap().table_names().is_empty())
            }));
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[3;16],[29;16]]), schema(), |_| async {
                panic!("indexed historical keys cannot reissue native work")
            }, START_STREAM).await.unwrap();
            assert_eq!(ids(rows).await, [[3;16],[29;16]]);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn selected_reader_metadata_releases_at_eof_or_drop() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                Ok(stream(vec![Ok(batch(&[[1;16]]))]))
            }, START_STREAM).await.unwrap();
            assert_eq!(ids(rows).await, [[1;16]]);
            let retained = budget.reserved();
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("same input cannot reissue")
            }, START_STREAM).await.unwrap();
            assert!(budget.reserved() > retained);
            assert_eq!(ids(rows).await, [[1;16]]);
            assert_eq!(budget.reserved(), retained);
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("same input cannot reissue")
            }, START_STREAM).await.unwrap();
            assert!(budget.reserved() > retained);
            drop(rows);
            assert_eq!(budget.reserved(), retained);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn optional_compaction_refusal_keeps_exact_runs_readable_without_native_reissue() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            for n in 1..=2u8 {
                let rows = capture.read("ids".into(), "id", Arc::new(vec![[n;16]]), schema(), |keys| async move {
                    Ok(stream(vec![Ok(batch(&keys))]))
                }, START_STREAM).await.unwrap();
                assert_eq!(ids(rows).await, [[n;16]]);
            }
            // Represent two successful immutable runs before optional adoption. No source
            // acquisition is involved in this local owner-state fixture.
            let left = CoverageIndex::new(&[[1;16]], 0, &budget).unwrap();
            let right = CoverageIndex::new(&[[2;16]], 1, &budget).unwrap();
            capture.groups.get_mut("ids").unwrap().indexes = vec![(0,left),(0,right)];
            let available = lctx_model::domain::resources::TRANSFER_ROWS * 48 * 3;
            let pressure = budget.reserve("coverage-merge-pressure", budget.limit() - budget.reserved() - available).unwrap();
            for keys in [vec![[1;16],[2;16]], vec![[2;16]]] {
                let rows = capture.read("ids".into(), "id", Arc::new(keys.clone()), schema(), |_| async {
                    panic!("pure refusal must not reissue successful native acquisition")
                }, START_STREAM).await.unwrap();
                assert_eq!(ids(rows).await, keys);
                assert_eq!(capture.groups["ids"].indexes.len(), 2);
                assert!(capture.permits_local_retry());
            }
            drop(pressure);
            let duplicate = CoverageIndex::new(&[[1;16]], 0, &budget).unwrap();
            capture.groups.get_mut("ids").unwrap().indexes.push((0,duplicate));
            assert!(matches!(capture.routes("ids", &[[1;16]]), Err(ModelError::Conflict("duplicate native capture coverage"))));
            capture.groups.get_mut("ids").unwrap().indexes.pop();
            capture.prepare_group("ids").await.unwrap();
            assert_eq!(capture.groups["ids"].indexes.len(), 1);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn high_fanout_reverse_capture_narrows_with_one_bounded_reader_without_reissue() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let owner = owner(&budget).await;
            let mut capture = owner.capture_discovery(&budget, &Cancellation::default()).unwrap();
            let reverse = Arc::new(arrow_schema::Schema::new(vec![
                arrow_schema::Field::new("id", arrow_schema::DataType::FixedSizeBinary(16), false),
                arrow_schema::Field::new("owner", arrow_schema::DataType::FixedSizeBinary(16), false),
            ]));
            let shape = reverse.clone();
            let transfer = lctx_model::domain::resources::TRANSFER_ROWS;
            const BATCHES: usize = 1024;
            let rows = capture.read("reverse".into(), "owner", Arc::new(vec![[1;16],[2;16]]), reverse.clone(), |_| async move {
                let batches = futures::stream::iter((0..BATCHES).map(move |n| {
                    let ids = (0..transfer).map(|row| ((n * transfer + row) as u128).to_be_bytes());
                    let owners = std::iter::repeat_n(if n < BATCHES / 2 { [1;16] } else { [2;16] }, transfer);
                    Ok::<_, DataFusionError>(RecordBatch::try_new(shape.clone(), vec![
                        Arc::new(FixedSizeBinaryArray::try_from_iter(ids).unwrap()),
                        Arc::new(FixedSizeBinaryArray::try_from_iter(owners).unwrap()),
                    ]).unwrap())
                }));
                Ok(Box::pin(RecordBatchStreamAdapter::new(reverse.clone(), batches)) as SendableRecordBatchStream)
            }, START_STREAM).await.unwrap();
            async fn count(mut rows: SendableRecordBatchStream, expected: Option<[u8;16]>) -> usize {
                let mut count = 0;
                while let Some(batch) = rows.try_next().await.unwrap() {
                    if let Some(expected) = expected {
                        let owners = batch.column(1).as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
                        assert!((0..batch.num_rows()).all(|row| owners.value(row) == expected));
                    }
                    count += batch.num_rows();
                }
                count
            }
            assert_eq!(count(rows, None).await, BATCHES * transfer);
            let retained = budget.reserved();
            // Less space than the former whole-file footer allowance, but enough for
            // one exact nominal transfer and the active read/query metadata.
            let available = transfer * 32 * 3 + 65536;
            assert!(available < BATCHES * 256 * 2 + 4096);
            let pressure = budget.reserve("reverse-singleton-pressure", budget.limit() - retained - available).unwrap();
            let rows = capture.read("reverse".into(), "owner", Arc::new(vec![[2;16]]), shape_for_reverse(), |_| async {
                panic!("successful high-fanout native source must never be replayed")
            }, START_STREAM).await.unwrap();
            assert_eq!(count(rows, Some([2;16])).await, BATCHES / 2 * transfer);
            assert_eq!(budget.reserved(), retained + pressure.size());
            drop(pressure);
        }
        assert_eq!(budget.reserved(), 0);
    }
    fn shape_for_reverse() -> SchemaRef {
        Arc::new(arrow_schema::Schema::new(vec![
            arrow_schema::Field::new("id", arrow_schema::DataType::FixedSizeBinary(16), false),
            arrow_schema::Field::new("owner", arrow_schema::DataType::FixedSizeBinary(16), false),
        ]))
    }
    #[test]
    fn tiny_nominal_fragments_form_bounded_transfer_windows() {
        let mut buffer = NominalBuffer::new(schema()).unwrap();
        let mut counts = Vec::new();
        let input = batch(&[[1;16]]);
        for _ in 0..lctx_model::domain::resources::TRANSFER_ROWS + 1 {
            buffer.push(&input, |rows| { counts.push(rows.num_rows()); Ok(()) }).unwrap();
        }
        counts.push(buffer.finish().unwrap().unwrap().num_rows());
        assert_eq!(counts, [lctx_model::domain::resources::TRANSFER_ROWS, 1]);
    }
}
