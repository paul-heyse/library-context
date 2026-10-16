//! Invocation-local successful native frontier captures. IPC is private preparation, never
//! completed-input authority. Coverage is published only after checked EOF and file finality.
use crate::{admission::Cancellation, consumed_rows::{PreparedEdges, coverage_index::CoverageIndex}, invocation_ipc::{InvocationIpc, InvocationRead, SealedIpc}};
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
    indexes: Vec<Option<CoverageIndex>>,
    pending: Option<(usize, CoverageIndex)>,
}
struct Window { rows_name: String, rows: SealedIpc }
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
    next: usize,
}
impl FrontierCapture {
    pub(crate) fn new(owner: PreparedEdges, budget: &ResourceBudget, cancellation: &Cancellation) -> Result<Self, ModelError> {
        let ipc = InvocationIpc::new(owner.session(), budget, cancellation, None)?;
        Ok(Self { owner, ipc, budget: budget.clone(), cancellation: cancellation.clone(),
            groups: BTreeMap::new(), charge: StateCharge::new(budget, "native-frontier-capture"),
            acquisition_failed: false, next: 0 })
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
        // Successful capture owns its pending index before this pure compaction. Refusal
        // retains both immutable merge inputs; only a completed replacement retires them.
        while let Some((level, pending)) = group.pending.as_ref() {
            let level = *level;
            if level >= group.indexes.len() {
                if group.indexes.len() == group.indexes.capacity() {
                    let additional = group.indexes.capacity().max(4);
                    self.charge.grow(additional * size_of::<Option<CoverageIndex>>())?;
                    group.indexes.reserve_exact(additional);
                }
                group.indexes.push(None);
            }
            if let Some(old) = &group.indexes[level] {
                let merged = old.merge(pending, &self.budget, &self.cancellation).await?;
                group.indexes[level] = None;
                group.pending = Some((level + 1, merged));
            } else {
                let (_, pending) = group.pending.take().expect("pending capture index");
                group.indexes[level] = Some(pending);
            }
        }
        Ok(())
    }
    fn routes(&self, group: &str, keys: &[[u8;16]]) -> Result<BTreeMap<[u8;16], usize>, ModelError> {
        let mut routes = BTreeMap::new();
        for index in self.groups[group].indexes.iter().flatten() {
            index.lookup(keys, &self.budget, |key, window| {
                if routes.insert(key, window).is_some() { return Err(ModelError::Conflict("duplicate native capture coverage")); }
                Ok(())
            })?;
        }
        Ok(routes)
    }
    pub(crate) async fn read<F, Fut>(
        &mut self, group: String, key_field: &'static str, keys: Arc<Vec<[u8; 16]>>,
        schema: SchemaRef, issue: F,
    ) -> Result<SendableRecordBatchStream, ModelError>
    where F: FnOnce(Arc<Vec<[u8; 16]>>) -> Fut,
        Fut: Future<Output = Result<SendableRecordBatchStream, ModelError>>,
    {
        self.cancellation.check()?;
        if self.acquisition_failed { return Err(ModelError::Conflict("failed native frontier acquisition")); }
        if keys.windows(2).any(|pair| pair[0] >= pair[1]) { return Err(ModelError::Schema("native capture key order")); }
        if let Some(old) = self.groups.get(&group) {
            if old.schema != schema || old.key_field != key_field { return Err(ModelError::Conflict("native capture query shape")); }
        } else {
            self.charge.grow(512 + group.len() + schema.fields().iter().map(|f| f.size()).sum::<usize>())?;
            self.groups.insert(group.clone(), Group { schema: schema.clone(), key_field, windows: Vec::new(),
                indexes: Vec::new(), pending: None });
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
            let next = self.next; self.next += 1;
            let rows_name = format!("frontier_rows_{next}");
            self.charge.grow(256 + rows_name.len())?;
            let windows = &mut self.groups.get_mut(&group).expect("capture group").windows;
            if windows.len() == windows.capacity() {
                let additional = windows.capacity().max(4);
                self.charge.grow(additional * size_of::<Window>())?;
                windows.reserve_exact(additional);
            }
            let mut writer = self.ipc.begin(schema.clone())?;
            let _buffer = self.budget.reserve("native-capture-nominal-buffer",
                lctx_model::domain::resources::TRANSFER_ROWS * schema.fields().len() * 64 + 4096)?;
            let mut buffer = NominalBuffer::new(schema.clone())?;
            self.begin_acquisition();
            let mut source = issue(missing.clone()).await?;
            if source.schema() != schema { return Err(ModelError::Schema("native capture issued schema")); }
            let mut previous = None;
            while let Some(batch) = source.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                let column = batch.column_by_name(key_field).and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema("native capture nominal key"))?;
                let ids = batch.column_by_name("id").and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema("native capture nominal row ID"))?;
                for row in 0..batch.num_rows() {
                    if column.is_null(row) && key_field != "id" { continue; }
                    if column.is_null(row) || ids.is_null(row) { return Err(ModelError::Schema("native capture null key")); }
                    let key: [u8;16] = column.value(row).try_into().map_err(ModelError::codec)?;
                    let id: [u8;16] = ids.value(row).try_into().map_err(ModelError::codec)?;
                    if missing.binary_search(&key).is_err() { return Err(ModelError::Conflict("native capture source demand")); }
                    if previous.is_some_and(|old| old >= id) { return Err(ModelError::Conflict("native capture source row order")); }
                    previous = Some(id);
                }
                buffer.push(&batch, |batch| writer.push(&batch))?;
                tokio::task::yield_now().await;
            }
            drop(source);
            if let Some(batch) = buffer.finish()? { writer.push(&batch)?; }
            drop(buffer);
            let rows = writer.seal()?;
            let state = self.groups.get_mut(&group).expect("capture group");
            state.windows.push(Window { rows_name, rows });
            state.pending = index.map(|index| (0, index));
            self.complete_acquisition();
            selected.insert(window_ordinal);
            self.prepare_group(&group).await?;
        }
        let windows = &self.groups[&group].windows;
        // Only exactly intersecting captures participate in this query. A disjoint frontier
        // neither scans old result files nor rebuilds a logical union over its history.
        let retained = self.budget.reserve("native-capture-local-query",
            selected.iter().map(|index| windows[*index].rows_name.len() + 64).sum::<usize>() * 3 + keys.len() * 128 + 4096)?;
        let sql = if selected.is_empty() {
            format!("SELECT * FROM {} WHERE FALSE", windows[0].rows_name)
        } else {
            format!("SELECT * FROM ({}) captured WHERE {} ORDER BY id",
                selected.iter().map(|index| format!("SELECT * FROM {}", windows[*index].rows_name)).collect::<Vec<_>>().join(" UNION ALL "),
                key_predicate(key_field, &keys))
        };
        let mut reader = self.ipc.reader()?;
        if selected.is_empty() {
            reader.adopt(&windows[0].rows, &windows[0].rows_name).await?;
        } else {
            for index in selected { reader.adopt(&windows[index].rows, &windows[index].rows_name).await?; }
        }
        let stream = crate::sql::query(reader.session(), &sql).await.map_err(crate::sql::model_error)?
            .execute_stream().await.map_err(crate::sql::model_error)?;
        Ok(Box::pin(CapturedRead { inner: Some(stream), reader: Some(reader), query: Some(retained), schema }))

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
            }).await.unwrap();
            assert_eq!(ids(first).await, [[1;16], [3;16]]);
            assert!(capture.permits_local_retry());
            // Simulate the later local membership refusal, preserving its typed cause.
            let refused = budget.reserve("local-membership-control", budget.limit() + 1).unwrap_err();
            assert!(matches!(refused, ModelError::Resource { .. }));
            let second = capture.read("ids".into(), "id", Arc::new(vec![[2;16], [3;16], [4;16]]), schema(), |keys| {
                issued.fetch_add(1, Ordering::Relaxed);
                assert_eq!(&*keys, &[[4;16]]);
                async { Ok(stream(vec![Ok(batch(&[[4;16]]))])) }
            }).await.unwrap();
            assert_eq!(ids(second).await, [[3;16], [4;16]]);
            let empty = capture.read("ids".into(), "id", Arc::new(vec![[2;16]]), schema(), |_| async {
                panic!("complete-empty source key must not be reissued")
            }).await.unwrap();
            assert!(ids(empty).await.is_empty());
            assert_eq!(issued.load(Ordering::Relaxed), 2);
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
            }).await.err().unwrap();
            assert!(error.to_string().contains("active-native-control"));
            assert!(!capture.permits_local_retry());
            assert!(capture.groups["ids"].windows.is_empty());
            assert!(capture.require_owner(&owner, &budget).is_err());
            assert!(capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("failed acquisition cannot be retried")
            }).await.is_err());
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
            }).await.unwrap();
            assert!(ids(rows).await.is_empty());
            let rows = capture.read("empty".into(), "id", Arc::new(vec![]), schema(), |_| async {
                panic!("same earned empty demand does not reissue")
            }).await.unwrap();
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
                }).await.unwrap();
                assert_eq!(ids(rows).await, [[n;16]]);
            }
            let _keys = budget.reserve("index-route-control", 4096).unwrap();
            let routes = capture.routes("ids", &[[3;16],[29;16],[99;16]]).unwrap();
            assert_eq!(routes, BTreeMap::from([([3;16],2),([29;16],28)]));
            assert_eq!(capture.groups["ids"].indexes.iter().flatten().count(), 1);
            assert_eq!(capture.groups["ids"].windows.len(), 32);
            // No data-window provider remains registered in the persistent writer owner.
            assert!(capture.ipc.session().catalog_names().iter().all(|name| {
                let catalog = capture.ipc.session().catalog(name).unwrap();
                catalog.schema_names().iter().all(|name| catalog.schema(name).unwrap().table_names().is_empty())
            }));
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[3;16],[29;16]]), schema(), |_| async {
                panic!("indexed historical keys cannot reissue native work")
            }).await.unwrap();
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
            }).await.unwrap();
            assert_eq!(ids(rows).await, [[1;16]]);
            let retained = budget.reserved();
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("same input cannot reissue")
            }).await.unwrap();
            assert!(budget.reserved() > retained);
            assert_eq!(ids(rows).await, [[1;16]]);
            assert_eq!(budget.reserved(), retained);
            let rows = capture.read("ids".into(), "id", Arc::new(vec![[1;16]]), schema(), |_| async {
                panic!("same input cannot reissue")
            }).await.unwrap();
            assert!(budget.reserved() > retained);
            drop(rows);
            assert_eq!(budget.reserved(), retained);
        }
        assert_eq!(budget.reserved(), 0);
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
