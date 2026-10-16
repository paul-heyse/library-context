//! Invocation-local, spillable Base preparation. Native reads finish once; adaptive assembly
//! only reads these private files and never retries a submitted source operation.
use super::*;
use arrow_array::{FixedSizeBinaryArray, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use lctx_compiler_store::invocation_ipc::{InvocationIpc, IpcCapture};
type Memberships = charged::ChargedSet<(usize, (usize, [u8; 16]))>;
const MEMBERS: &str = "base_root_memberships";
fn input_name(table: usize) -> String {
    format!("base_captured_input_{table}")
}
fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("root_ordinal", DataType::Int64, false),
        Field::new("kind", DataType::Int64, false),
        Field::new("id", DataType::FixedSizeBinary(16), false),
    ]))
}
fn encode_members(partition: usize, rows: &[(usize, [u8; 16])]) -> Result<RecordBatch, ModelError> {
    RecordBatch::try_new(
        schema(),
        vec![
            Arc::new(Int64Array::from(vec![partition as i64; rows.len()])),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.0 as i64).collect::<Vec<_>>(),
            )),
            Arc::new(
                FixedSizeBinaryArray::try_from_iter(rows.iter().map(|row| row.1))
                    .map_err(ModelError::codec)?,
            ),
        ],
    )
    .map_err(ModelError::codec)
}
pub(crate) struct CapturedBase<'a> {
    scopes: &'a BaseScopes,
    ipc: InvocationIpc,
    pub(super) keys: Vec<[u8; 16]>,
    opaque_literals: bool,
    kernel: Kernel,
    _charge: charged::StateCharge,
}
impl<'a> CapturedBase<'a> {
    pub(super) async fn prepare(
        scopes: &'a BaseScopes,
        kernel: Kernel,
        keys: Vec<[u8; 16]>,
        charge: charged::StateCharge,
        _context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<Self, ModelError> {
        cancellation.check()?;
        if keys.is_empty() || keys.len() > 32 {
            return Err(ModelError::Schema("Base capture root window"));
        }
        let root_table = scopes.root_index(kernel)?;
        let opaque_literals = matches!(kernel, Kernel::Expression);
        let root_kind = if opaque_literals {
            crate::consumed_rows::PreparedRootKind::Virtual
        } else {
            crate::consumed_rows::PreparedRootKind::Physical
        };
        tracing::info!(target:"lctx_phase", observation="base_capture_window", ?kernel,
            root_first=?keys.first(), root_count=keys.len());
        let source = scopes.edges.session();
        let membership_phase = lctx_native::phase::Phase::begin("base_cohort_membership_capture");
        let mut ipc = InvocationIpc::new(source, budget, cancellation, None)?;
        let mut members = ipc.begin(schema())?;
        for (partition, key) in keys.iter().enumerate() {
            cancellation.check()?;
            // Keep the existing single-root discovery bound; no32-way membership owner.
            let batch = scopes
                .edges
                .batch_with_cancellation(
                    &[crate::consumed_rows::PreparedRoot {
                        table: root_table,
                        key: *key,
                        kind: root_kind,
                    }],
                    budget,
                    cancellation,
                )
                .await?;
            let _transfer = budget.reserve(
                "Base-membership-write-window",
                resources::TRANSFER_ROWS * 64 + 4096,
            )?;
            let mut rows = Vec::with_capacity(resources::TRANSFER_ROWS);
            for row in batch.memberships(0)? {
                rows.push(row);
                if rows.len() == resources::TRANSFER_ROWS {
                    members.push(&encode_members(partition, &rows)?)?;
                    rows.clear();
                }
            }
            if !rows.is_empty() {
                members.push(&encode_members(partition, &rows)?)?;
            }
            drop(batch);
        }
        members.finish(&mut ipc, MEMBERS).await?;
        membership_phase.finish(lctx_native::phase::Terminal::Passed);
        let payload_phase = lctx_native::phase::Phase::begin("base_unique_payload_capture");
        let literals = scopes.index(TypeId::of::<value::Literal>())?;
        let literal_schema = scopes.tables[literals].relation.schema();
        let opaque_schema = Arc::new(
            literal_schema
                .project(&[
                    literal_schema.index_of("id").map_err(ModelError::codec)?,
                    literal_schema.index_of("kind").map_err(ModelError::codec)?,
                ])
                .map_err(ModelError::codec)?,
        );
        let mut opaque = None;
        for (table, _) in scopes.inputs.iter().enumerate() {
            cancellation.check()?;
            let schema = scopes.tables[table].relation.schema().clone();
            let mut writer = None;
            // Inventory primitive ordinals once per namespace, rather than planning an empty
            // point selection for every root/namespace pair.
            let _ordinals = budget.reserve("Base-active-root-ordinals", 32 + 4096)?;
            let mut active = [false; 32];
            let mut ordinals = crate::sql::query(
                ipc.session(),
                &format!("SELECT DISTINCT root_ordinal FROM {MEMBERS} WHERE kind={table}"),
            )
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
            while let Some(batch) = ordinals.try_next().await.map_err(crate::sql::model_error)? {
                cancellation.check()?;
                let values = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .ok_or(ModelError::Schema("Base active root ordinals"))?;
                for ordinal in values.values() {
                    let ordinal = usize::try_from(*ordinal)
                        .map_err(|_| ModelError::Schema("Base active root ordinal"))?;
                    if ordinal >= keys.len() {
                        return Err(ModelError::Schema("Base active root ordinal"));
                    }
                    active[ordinal] = true;
                }
            }
            drop(ordinals);
            for (partition, _) in active.iter().enumerate().filter(|(_, active)| **active) {
                // A native read never combines disjoint roots' rich payloads. Repeated supporting
                // IDs are captured by their first consuming root and reused only from local IPC.
                let sql = format!(
                    "SELECT DISTINCT id FROM {MEMBERS} WHERE kind={table} AND root_ordinal={partition} AND id NOT IN (SELECT id FROM {MEMBERS} WHERE kind={table} AND root_ordinal<{partition}) ORDER BY id"
                );
                let local = ipc.session().clone();
                let mut distinct = crate::sql::query(&local, &sql)
                    .await
                    .map_err(lctx_compiler_store::sql::model_error)?
                    .execute_stream()
                    .await
                    .map_err(lctx_compiler_store::sql::model_error)?;
                while let Some(batch) = distinct
                    .try_next()
                    .await
                    .map_err(lctx_compiler_store::sql::model_error)?
                {
                    cancellation.check()?;
                    let _transfer = budget
                        .reserve("Base-distinct-key-transfer", logical_batch_bytes(&batch)?)?;
                    for start in (0..batch.num_rows())
                        .step_by(lctx_compiler_store::compiler_provider::REFERENCE_KEYS)
                    {
                        let end = (start + lctx_compiler_store::compiler_provider::REFERENCE_KEYS)
                            .min(batch.num_rows());
                        let _keys =
                            budget.reserve("Base-native-point-keys", (end - start) * 160 + 4096)?;
                        let keys = (start..end)
                            .map(|row| {
                                crate::scoped_admission::column(&batch, "id", row)?
                                    .ok_or(ModelError::Schema("Base captured nominal key"))
                            })
                            .collect::<Result<Vec<_>, ModelError>>()?;
                        let point = scopes.edges.point_input(table, keys, budget).await?;
                        if opaque_literals && table == literals {
                            stream_capture(
                                point.session(),
                                &format!(
                                    "SELECT id,kind FROM ({}) literals WHERE kind IN (3,4)",
                                    point.sql()
                                ),
                                &mut opaque,
                                &mut ipc,
                                &opaque_schema,
                                cancellation,
                            )
                            .await?;
                            stream_capture(
                                point.session(),
                                &format!(
                                    "SELECT * FROM ({}) literals WHERE kind NOT IN (3,4)",
                                    point.sql()
                                ),
                                &mut writer,
                                &mut ipc,
                                &schema,
                                cancellation,
                            )
                            .await?;
                        } else {
                            stream_capture(
                                point.session(),
                                point.sql(),
                                &mut writer,
                                &mut ipc,
                                &schema,
                                cancellation,
                            )
                            .await?;
                        }
                    }
                }
                drop(distinct);
            }
            match writer {
                Some(writer) => writer.finish(&mut ipc, &input_name(table)).await?,
                None => ipc.empty(&input_name(table), schema)?,
            }
        }
        match opaque {
            Some(opaque) => opaque.finish(&mut ipc, "base_opaque_literals").await?,
            None => ipc.empty("base_opaque_literals", opaque_schema)?,
        }
        payload_phase.finish(lctx_native::phase::Terminal::Passed);
        Ok(Self {
            scopes,
            ipc,
            keys,
            opaque_literals,
            kernel,
            _charge: charge,
        })
    }
    #[cfg(test)]
    pub(super) async fn captured_rows<R: Record>(&self) -> Result<usize, ModelError> {
        let table = self.scopes.index(TypeId::of::<R>())?;
        let mut rows = crate::sql::query(
            self.ipc.session(),
            &format!("SELECT id FROM {}", input_name(table)),
        )
        .await
        .map_err(crate::sql::model_error)?
        .execute_stream()
        .await
        .map_err(crate::sql::model_error)?;
        let mut count = 0;
        while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
            count += batch.num_rows();
        }
        Ok(count)
    }
    pub(crate) async fn cohort(
        &self,
        start: usize,
        maximum: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<BaseWindow<'_>, ModelError> {
        let mut width = (self.keys.len() - start).min(maximum).min(32);
        loop {
            cancellation.check()?;
            match self.assemble(start, width, budget, cancellation).await {
                Ok(window) => return Ok(window),
                Err(error) if width > 1 && local_resource(&error) => width = (width / 2).max(1),
                Err(error) => return Err(error),
            }
        }
    }
    pub(super) async fn assemble(
        &self,
        start: usize,
        width: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<BaseWindow<'_>, ModelError> {
        tracing::info!(target:"lctx_phase", observation="base_local_assembly_window", kernel=?self.kernel,
            root_first=?self.keys.get(start), root_count=width, start, width);
        let assembly_phase = lctx_native::phase::Phase::begin("base_pure_local_assembly");
        let mut charge = charged::StateCharge::new(budget, "Base-local-cohort");
        charge.grow(size_of::<BaseData>())?;
        let mut data = Box::new(BaseData::new(budget));
        let mut memberships = Memberships::default();
        let select = format!(
            "SELECT root_ordinal,kind,id FROM {MEMBERS} WHERE root_ordinal>={start} AND root_ordinal<{} ORDER BY root_ordinal,kind,id",
            start + width
        );
        let mut stream = crate::sql::query(self.ipc.session(), &select)
            .await
            .map_err(lctx_compiler_store::sql::model_error)?
            .execute_stream()
            .await
            .map_err(lctx_compiler_store::sql::model_error)?;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(lctx_compiler_store::sql::model_error)?
        {
            cancellation.check()?;
            let _transfer = budget.reserve(
                "Base-local-membership-transfer",
                logical_batch_bytes(&batch)?,
            )?;
            let roots = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or(ModelError::Schema("Base captured root ordinal"))?;
            let kinds = batch
                .column(1)
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or(ModelError::Schema("Base captured namespace"))?;
            for row in 0..batch.num_rows() {
                let root = usize::try_from(roots.value(row)).map_err(ModelError::codec)?;
                let kind = usize::try_from(kinds.value(row)).map_err(ModelError::codec)?;
                let key = crate::scoped_admission::column(&batch, "id", row)?
                    .ok_or(ModelError::Schema("Base captured ID"))?;
                memberships.insert(&mut charge, (root - start, (kind, key)))?;
            }
        }
        drop(stream);
        for (table, input) in self.scopes.inputs.iter().enumerate() {
            cancellation.check()?;
            // Absence from complete captured membership establishes a known-empty namespace.
            // A demanded but missing payload remains demanded and follows the existing loader.
            if (0..width).all(|partition| {
                membership_keys(&memberships, partition, table)
                    .next()
                    .is_none()
            }) {
                continue;
            }
            let sql = format!(
                "SELECT source.* FROM {} source WHERE source.id IN (SELECT id FROM {MEMBERS} WHERE kind={table} AND root_ordinal>={start} AND root_ordinal<{}) ORDER BY source.id",
                input_name(table),
                start + width
            );
            let mut stream = crate::sql::query(self.ipc.session(), &sql)
                .await
                .map_err(lctx_compiler_store::sql::model_error)?
                .execute_stream()
                .await
                .map_err(lctx_compiler_store::sql::model_error)?;
            while let Some(batch) = stream
                .try_next()
                .await
                .map_err(lctx_compiler_store::sql::model_error)?
            {
                cancellation.check()?;
                let _transfer =
                    budget.reserve("Base-local-union-transfer", logical_batch_bytes(&batch)?)?;
                data.visit(input, &batch)?;
            }
        }
        let literals = self.scopes.index(TypeId::of::<value::Literal>())?;
        let opaque = format!(
            "SELECT opaque.* FROM base_opaque_literals opaque WHERE opaque.id IN (SELECT id FROM {MEMBERS} WHERE kind={literals} AND root_ordinal>={start} AND root_ordinal<{})",
            start + width
        );
        if self.opaque_literals
            && (0..width).any(|partition| {
                membership_keys(&memberships, partition, literals)
                    .next()
                    .is_some()
            })
        {
            project_literals(self.ipc.session(), &opaque, &mut data, budget).await?;
        }
        assembly_phase.finish(lctx_native::phase::Terminal::Passed);
        Ok(BaseWindow {
            scopes: self.scopes,
            memberships,
            data,
            keys: &self.keys[start..start + width],
            _charge: charge,
        })
    }
}
async fn stream_capture(
    session: &datafusion::prelude::SessionContext,
    sql: &str,
    writer: &mut Option<IpcCapture>,
    ipc: &mut InvocationIpc,
    schema: &Arc<Schema>,
    cancellation: &crate::workspace::Cancellation,
) -> Result<(), ModelError> {
    let mut stream = crate::sql::query(session, sql)
        .await
        .map_err(lctx_compiler_store::sql::model_error)?
        .execute_stream()
        .await
        .map_err(lctx_compiler_store::sql::model_error)?;
    while let Some(batch) = stream
        .try_next()
        .await
        .map_err(lctx_compiler_store::sql::model_error)?
    {
        cancellation.check()?;
        if writer.is_none() {
            *writer = Some(ipc.begin(schema.clone())?);
        }
        writer.as_mut().expect("capture opened").push(&batch)?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
pub(super) fn local_resource(error: &ModelError) -> bool {
    if error.has_operation_group()
        || !error.permits_storage_cleanup()
        || error.has_committed_effect()
    {
        return false;
    }
    let mut next: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = next {
        if matches!(
            error.downcast_ref::<ModelError>(),
            Some(ModelError::Resource { .. })
        ) {
            return true;
        }
        next = error.source();
    }
    false
}
