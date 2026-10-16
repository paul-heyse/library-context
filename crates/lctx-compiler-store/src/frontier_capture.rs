//! Invocation-local successful native frontier captures. IPC is private preparation, never
//! completed-input authority. Coverage is published only after checked EOF and file finality.
use crate::{admission::Cancellation, consumed_rows::PreparedEdges, invocation_ipc::{InvocationIpc, SealedIpc}};
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use arrow_schema::SchemaRef;
use datafusion::physical_plan::{SendableRecordBatchStream, stream::RecordBatchStreamAdapter};
use futures::{StreamExt, TryStreamExt};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use std::{collections::BTreeMap, future::Future, sync::Arc};

struct Group {
    schema: SchemaRef,
    key_field: &'static str,
    windows: Vec<Window>,
}
struct Window {
    coverage_name: String,
    rows_name: String,
    coverage: SealedIpc,
    rows: SealedIpc,
    coverage_adopted: bool,
    rows_adopted: bool,
}
/// An exact immutable source binding owns the protected inputs through every local retry.
/// A failed acquisition permanently prevents retry; successful windows remain reusable even
/// when later local membership/partition preparation refuses resource admission.
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
    /// Guard preparation that can itself issue native work (for example contextual adjacency).
    pub(crate) fn begin_acquisition(&mut self) { self.acquisition_failed = true; }
    pub(crate) fn complete_acquisition(&mut self) { self.acquisition_failed = false; }
    async fn adopt_group(&mut self, group: &str) -> Result<(), ModelError> {
        for window in &mut self.groups.get_mut(group).expect("capture group").windows {
            if !window.rows_adopted {
                self.ipc.adopt(&window.rows, &window.rows_name).await?;
                window.rows_adopted = true;
            }
            if !window.coverage_adopted {
                self.ipc.adopt(&window.coverage, &window.coverage_name).await?;
                window.coverage_adopted = true;
            }
        }
        Ok(())
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
        if keys.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ModelError::Schema("native capture key order"));
        }
        if let Some(old) = self.groups.get(&group) {
            if old.schema != schema || old.key_field != key_field {
                return Err(ModelError::Conflict("native capture query shape"));
            }
        } else {
            self.charge.grow(512 + group.len() + schema.fields().iter().map(|f| f.size()).sum::<usize>())?;
            self.groups.insert(group.clone(), Group { schema: schema.clone(), key_field, windows: Vec::new() });
        }
        self.adopt_group(&group).await?;
        let _keys = self.budget.reserve("native-capture-key-window", keys.len() * 256 + 4096)?;
        let predicate = key_predicate("id", &keys);
        let mut covered = std::collections::BTreeSet::new();
        let windows = &self.groups[&group].windows;
        if !windows.is_empty() && !keys.is_empty() {
            let _query = self.budget.reserve("native-capture-coverage-query",
                windows.iter().map(|window| window.coverage_name.len() + 64).sum::<usize>() * 3 + keys.len() * 128 + 4096)?;
            let sql = format!("SELECT DISTINCT id FROM ({}) covered WHERE {predicate} ORDER BY id",
                windows.iter().map(|window| format!("SELECT id FROM {}", window.coverage_name)).collect::<Vec<_>>().join(" UNION ALL "));
            let mut rows = crate::sql::query(self.ipc.session(), &sql).await.map_err(crate::sql::model_error)?
                .execute_stream().await.map_err(crate::sql::model_error)?;
            while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                let ids = batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>()
                    .ok_or(ModelError::Schema("native capture coverage IDs"))?;
                for row in 0..batch.num_rows() {
                    if ids.is_null(row) { return Err(ModelError::Schema("native capture coverage null")); }
                    let id = ids.value(row).try_into().map_err(ModelError::codec)?;
                    if keys.binary_search(&id).is_err() { return Err(ModelError::Conflict("native capture coverage demand")); }
                    covered.insert(id);
                }
            }
        }
        let missing = Arc::new(keys.iter().copied().filter(|key| !covered.contains(key)).collect::<Vec<_>>());
        // Empty demand still checks source authority once. No synthetic provider replaces it.
        if !missing.is_empty() || self.groups[&group].windows.is_empty() {
            let index = self.next; self.next += 1;
            let rows_name = format!("frontier_rows_{index}");
            let coverage_name = format!("frontier_coverage_{index}");
            self.charge.grow(256 + rows_name.len() + coverage_name.len())?;
            let windows = &mut self.groups.get_mut(&group).expect("capture group").windows;
            if windows.len() == windows.capacity() {
                let additional = windows.capacity().max(4);
                self.charge.grow(additional * size_of::<Window>())?;
                windows.reserve_exact(additional);
            }
            // Coverage is private until the source reaches complete EOF. Prepare and seal its
            // bounded key inventory before issue, so publication needs no further allocation.
            let coverage_schema = Arc::new(arrow_schema::Schema::new(vec![arrow_schema::Field::new(
                "id", arrow_schema::DataType::FixedSizeBinary(16), false)]));
            let mut coverage = self.ipc.begin(coverage_schema.clone())?;
            let mut ids = arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(missing.len(), 16);
            for key in missing.iter() { ids.append_value(key).map_err(ModelError::codec)?; }
            let ids = ids.finish();
            coverage.push(&RecordBatch::try_new(coverage_schema, vec![Arc::new(ids)]).map_err(ModelError::codec)?)?;
            let coverage = coverage.seal()?;
            let mut writer = self.ipc.begin(schema.clone())?;
            let _buffer = self.budget.reserve("native-capture-nominal-buffer",
                lctx_model::domain::resources::TRANSFER_ROWS * schema.fields().len() * 64 + 4096)?;
            let mut buffer = NominalBuffer::new(schema.clone())?;
            self.begin_acquisition();
            let mut source = issue(missing.clone()).await?;
            if source.schema() != schema { return Err(ModelError::Schema("native capture issued schema")); }
            while let Some(batch) = source.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                buffer.push(&batch, |batch| writer.push(&batch))?;
                tokio::task::yield_now().await;
            }
            drop(source);
            if let Some(batch) = buffer.finish()? { writer.push(&batch)?; }
            drop(buffer);
            let rows = writer.seal()?;
            self.groups.get_mut(&group).expect("capture group").windows.push(Window {
                coverage_name, rows_name, coverage, rows, coverage_adopted: false, rows_adopted: false,
            });
            self.complete_acquisition();
            self.adopt_group(&group).await?;
        }
        let windows = &self.groups[&group].windows;
        let retained = self.budget.reserve("native-capture-local-query",
            windows.iter().map(|window| window.rows_name.len() + 64).sum::<usize>() * 3 + keys.len() * 128 + 4096)?;
        let sql = format!("SELECT * FROM ({}) captured WHERE {} ORDER BY id",
            windows.iter().map(|window| format!("SELECT * FROM {}", window.rows_name)).collect::<Vec<_>>().join(" UNION ALL "),
            key_predicate(key_field, &keys));
        let stream = crate::sql::query(self.ipc.session(), &sql).await.map_err(crate::sql::model_error)?
            .execute_stream().await.map_err(crate::sql::model_error)?;
        let stream = stream.map(move |row| { let _hold = &retained; row });
        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
    }
}
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
    use datafusion::{error::DataFusionError, prelude::SessionContext};
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
