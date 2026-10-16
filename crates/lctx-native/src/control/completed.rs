//! Fixed physical witness construction after canonical attempt freeze. All helpers are
//! crate-private; callers cannot supply SQL or mutate canonical semantic content.
use super::*;
use crate::completed_state::{FrozenCompletion, FrozenContentFence};
use lctx_model::domain::resources::ResourceBudget;

/// Fixed immutable witness payloads and heterogeneous holds share one bounded
/// owned effect. The final parent CAS still follows independent full inventory EOF.
pub(super) fn validate_witness_window(
    rows: &[Value],
    edges: &[(RecordId, RecordId)],
) -> Result<(), ModelError> {
    if rows.len() > crate::loader::NATIVE_WINDOW_ROWS
        || edges.len() > crate::loader::NATIVE_WINDOW_ROWS
    {
        return Err(ModelError::Conflict(
            "compound witness row/hold window bound",
        ));
    }
    for row in rows {
        let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
            return Err(ModelError::Schema("physical witness id"));
        };
        if !matches!(
            id.table.as_str(),
            "native_completed_state"
                | "native_completed_view"
                | "native_completed_contributor"
                | "compiler_binding_descriptor"
        ) {
            return Err(ModelError::Conflict("physical witness family"));
        }
    }
    Ok(())
}
struct WitnessUnit {
    row: Value,
    edges: Vec<(RecordId, RecordId)>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
/// Boundary-local writer: only fixed immutable children and their heterogeneous holds.
/// Whole prepared effects are byte-checked before acceptance; no prefix seals a parent.
pub(crate) struct WitnessWriter<'a> {
    client: &'a Surreal<Client>,
    budget: &'a ResourceBudget,
    owner: EffectOwner,
    guard: &'static str,
    scope: Variables,
    issuance: Option<IssuanceEra>,
    pending: Vec<WitnessUnit>,
    pending_bytes: usize,
    pending_holds: usize,
    _handles: Box<dyn lctx_model::domain::resources::Reservation>,
    workspace: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl<'a> WitnessWriter<'a> {
    pub(super) fn new(
        client: &'a Surreal<Client>,
        budget: &'a ResourceBudget,
        owner: EffectOwner,
        guard: &'static str,
        scope: Variables,
    ) -> Result<Self, ModelError> {
        let handles = budget.reserve(
            "compound witness window handles",
            crate::loader::NATIVE_WINDOW_ROWS
                .saturating_mul(std::mem::size_of::<WitnessUnit>())
                .saturating_mul(4)
                .saturating_add(8192),
        )?;
        let workspace = budget.reserve("compound witness prepared complete request", 0)?;
        Ok(Self {
            client,
            budget,
            owner,
            guard,
            scope,
            issuance: None,
            pending: Vec::new(),
            pending_bytes: 0,
            pending_holds: 0,
            _handles: handles,
            workspace,
        })
    }
    fn plan(
        &self,
        rows: Vec<Value>,
        edges: Vec<(RecordId, RecordId)>,
    ) -> Result<(String, Variables), ModelError> {
        validate_witness_window(&rows, &edges)?;
        let mut sql = String::from(self.guard);
        let mut variables = self.scope.clone();
        if let Some((plan, bindings)) = immutable_row_plan(None, None, rows)? {
            sql.push_str(&plan);
            merge_plan_bindings(&mut variables, bindings)?;
        }
        let attempt = match self.owner {
            EffectOwner::OpenAttempt(id) | EffectOwner::FrozenAttempt(id) => id,
            _ => return Err(ModelError::Conflict("compound witness exact attempt owner")),
        };
        if let Some((_, plan, bindings)) = hold_plan(Some(attempt), edges)? {
            sql = join_immutable_holds(&sql, &plan);
            merge_plan_bindings(&mut variables, bindings)?;
        }
        Ok((sql, variables))
    }
    fn admit_workspace(&mut self, next_bytes: usize) -> Result<(), ModelError> {
        // Keep the complete preparation workspace while accepting a tail. Repeated
        // unit-plan overhead plus this protocol allowance bounds the fixed child
        // envelope (one row, at most two holds and three distinct physical guards).
        self.workspace.try_resize(
            self.pending_bytes
                .saturating_add(next_bytes)
                .saturating_add(131072),
        )
    }
    pub(crate) async fn push<T: serde::Serialize + Sync>(
        &mut self,
        descriptor: &T,
        build: impl FnOnce() -> Result<(Value, Vec<(RecordId, RecordId)>), ModelError> + Send,
    ) -> Result<(), ModelError> {
        struct Counter(usize);
        impl std::io::Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(bytes.len());
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut encoded = Counter(0);
        serde_json::to_writer(&mut encoded, descriptor).map_err(ModelError::codec)?;
        // Admit before descriptor serialization, native envelope allocation, and plan expansion.
        let admitted = encoded.0.saturating_mul(32).saturating_add(131072);
        let mut charge = match self.budget.reserve(
            "compound witness child serialization and expansion",
            admitted,
        ) {
            Ok(charge) => charge,
            Err(ModelError::Resource { .. }) if !self.pending.is_empty() => {
                self.flush().await?;
                self.budget.reserve(
                    "compound witness child serialization and expansion",
                    admitted,
                )?
            }
            Err(error) => return Err(error),
        };
        let (row, edges) = build()?;
        if edges.len() > 2 {
            return Err(ModelError::Conflict("compound witness child hold bound"));
        }
        let (sql, variables) = self.plan(vec![row.clone()], edges.clone())?;
        let bytes = immutable_window::plan_bytes(&sql, &variables)?;
        if bytes > admitted {
            return Err(ModelError::Schema(
                "compound witness child construction bound",
            ));
        }
        drop(variables);
        drop(sql);
        charge.try_resize(bytes)?;
        if !self.pending.is_empty()
            && (self.pending.len() + 1 > crate::loader::NATIVE_WINDOW_ROWS
                || self.pending_holds + edges.len() > crate::loader::NATIVE_WINDOW_ROWS
                || self.pending_bytes.saturating_add(bytes)
                    > lctx_model::domain::resources::TRANSFER_BYTES)
        {
            self.flush().await?;
        }
        if let Err(error) = self.admit_workspace(bytes) {
            if matches!(error, ModelError::Resource { .. }) && !self.pending.is_empty() {
                self.flush().await?;
                self.admit_workspace(bytes)?;
            } else {
                return Err(error);
            }
        }
        self.pending_bytes = self.pending_bytes.saturating_add(bytes);
        self.pending_holds += edges.len();
        self.pending.push(WitnessUnit {
            row,
            edges,
            _charge: charge,
        });
        if self.pending.len() == crate::loader::NATIVE_WINDOW_ROWS
            || self.pending_holds == crate::loader::NATIVE_WINDOW_ROWS
            || self.pending_bytes >= lctx_model::domain::resources::TRANSFER_BYTES
        {
            self.flush().await?;
        }
        Ok(())
    }
    fn split(units: Vec<WitnessUnit>, stack: &mut Vec<Vec<WitnessUnit>>) {
        let mut left = units;
        let right = left.split_off(left.len() / 2);
        stack.push(right);
        stack.push(left);
    }
    pub(crate) async fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        self.pending_bytes = 0;
        self.pending_holds = 0;
        if self.issuance.is_none() {
            self.issuance = Some(authorization::owner_issuance(self.client, &self.owner).await?);
        }
        let mut stack = vec![std::mem::take(&mut self.pending)];
        while let Some(units) = stack.pop() {
            let rows = units.iter().map(|unit| unit.row.clone()).collect();
            let edges = units
                .iter()
                .flat_map(|unit| unit.edges.iter().cloned())
                .collect();
            let (sql, variables) = self.plan(rows, edges)?;
            let prepared = match authorization::prepare_owner_effect(
                self.client,
                &self.owner,
                self.issuance
                    .expect("captured exact witness owner issuance"),
                &sql,
                variables,
            )
            .await
            {
                Ok(prepared) => prepared,
                Err(ModelError::Limit {
                    owner: "native transaction",
                    limit: "complete control and payload bytes",
                    ..
                }) if units.len() > 1 => {
                    Self::split(units, &mut stack);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if prepared.bytes()? > self.workspace.size() {
                return Err(ModelError::Schema(
                    "compound witness complete preparation workspace bound",
                ));
            }
            // Prepared SQL, controls and captured expectations are moved unchanged;
            // receipt acceptance and every native write begin only here.
            authorization::execute_prepared_owner_effect(self.client, prepared).await?;
            drop(units);
        }
        self.workspace.try_resize(0)?;
        Ok(())
    }
}
pub(crate) fn witness_writer<'a>(
    client: &'a Surreal<Client>,
    fence: &FrozenContentFence,
    budget: &'a ResourceBudget,
) -> Result<WitnessWriter<'a>, ModelError> {
    let mut scope = Variables::new();
    frozen::bind_fence(fence, &mut scope);
    WitnessWriter::new(
        client,
        budget,
        EffectOwner::FrozenAttempt(fence.attempt()),
        frozen::FENCE_GUARD,
        scope,
    )
}
pub(crate) async fn finish_physical_completion(
    client: &Surreal<Client>,
    data: &FrozenCompletion,
    expected: &ExpectedRecord,
    revision: i64,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if expected.row() != data.value(false)? {
        return Err(ModelError::Conflict(
            "physical completion initial exact envelope",
        ));
    }
    let _charge = budget.reserve(
        "physical completion checked final effect",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(131072),
    )?;
    let mut variables = Variables::new();
    frozen::bind_fence(&data.fence(), &mut variables);
    variables.insert("completion", data.record()?);
    variables.insert(
        "completion_guard",
        guard_id(&Value::RecordId(data.record()?))?,
    );
    variables.insert("completion_incarnation", expected.incarnation());
    variables.insert("completion_bytes", expected.row().clone());
    variables.insert("completion_revision", revision);
    variables.insert("completion_identity", data.identity()?.hex());
    variables.insert("completion_logical", data.completed_state.hex());
    let sql = format!(
        "{} fn::native_touch($completion); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$completion]); LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $predicate!=$completion_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes {{ THROW 'physical completion inventory audit raced'; }}; UPDATE $completion SET completed=true RETURN NONE; fn::native_attempt_cut($__attempt,$__generation,$__era,$__epoch,'completion'); UPDATE $__attempt SET completed_identity=$completion_identity,completed_state=$completion_logical RETURN NONE",
        frozen::FENCE_GUARD
    );
    effect_for_owner(
        client,
        EffectOwner::FrozenAttempt(data.attempt),
        &sql,
        variables,
    )
    .await
}

pub(crate) async fn hold_selected_contributors(
    client: &Surreal<Client>,
    fence: &FrozenContentFence,
    root: &RecordId,
    expected: &[std::sync::Arc<ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut scope = Variables::new();
    frozen::bind_fence(fence, &mut scope);
    hold_boundary_contributors(
        client,
        EffectOwner::FrozenAttempt(fence.attempt()),
        frozen::FENCE_GUARD,
        scope,
        root,
        expected,
        budget,
    )
    .await
}
/// Fixed compound-parent copy ownership, independently of selector observations.
/// Every page compares all complete contributor rows before installing any hold.
pub(super) async fn hold_boundary_contributors(
    client: &Surreal<Client>,
    owner: EffectOwner,
    guard: &'static str,
    scope: Variables,
    root: &RecordId,
    records: &[std::sync::Arc<ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let attempt = match &owner {
        EffectOwner::FrozenAttempt(attempt) | EffectOwner::OpenAttempt(attempt) => *attempt,
        _ => return Err(ModelError::Conflict("compound contributor owner authority")),
    };
    if root.table.as_str() != "native_completed_state"
        || records
            .windows(2)
            .any(|pair| pair[0].record() >= pair[1].record())
        || records.iter().any(|row| {
            row.record().table.as_str() != "compiler_contribution" || row.incarnation() <= 0
        })
    {
        return Err(ModelError::Conflict("compound exact contributor inventory"));
    }
    if records.is_empty() {
        return Ok(());
    }
    let _scope = budget.reserve("compound contributor fixed owner context", 8192)?;
    let issuance = authorization::owner_issuance(client, &owner).await?;
    let mut offset = 0;
    while offset < records.len() {
        let mut count = (records.len() - offset).min(crate::loader::NATIVE_WINDOW_ROWS);
        loop {
            let page = &records[offset..offset + count];
            let bytes = page
                .iter()
                .map(|row| crate::loader::native_bytes(row.row()).saturating_add(2048))
                .fold(0usize, usize::saturating_add)
                .saturating_mul(32)
                .saturating_add(131072);
            let _workspace =
                match budget.reserve("compound exact contributor CAS construction", bytes) {
                    Ok(charge) => charge,
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        count = (count / 2).max(1);
                        continue;
                    }
                    Err(error) => return Err(error),
                };
            let mut selected = Vec::with_capacity(count);
            let mut edges = Vec::with_capacity(count);
            for expected in page {
                let mut row = Object::new();
                row.insert("record", expected.record().clone());
                row.insert(
                    "guard",
                    guard_id(&Value::RecordId(expected.record().clone()))?,
                );
                row.insert("incarnation", expected.incarnation());
                row.insert("bytes", expected.row().clone());
                selected.push(Value::Object(row));
                edges.push((root.clone(), expected.record().clone()));
            }
            let (_, holds, mut variables) = hold_plan(Some(attempt), edges)?.ok_or(
                ModelError::Schema("compound selected contributor hold page"),
            )?;
            variables.extend(scope.clone());
            variables.insert("selected_contributors", selected);
            let sql = format!(
                "{guard} FOR $expected IN $selected_contributors {{ LET $record=$expected.record; LET $guard_id=$expected.guard; fn::native_touch($record); LET $guard=SELECT * FROM ONLY $guard_id FOR UPDATE; LET $actual=SELECT * FROM ONLY $record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected.incarnation OR $actual!=$expected.bytes OR !$actual.completed {{ THROW 'selected contributor completion changed'; }}; }}; {holds}"
            );
            let prepared = match authorization::prepare_owner_effect(
                client, &owner, issuance, &sql, variables,
            )
            .await
            {
                Ok(prepared) => prepared,
                Err(ModelError::Limit {
                    owner: "native transaction",
                    limit: "complete control and payload bytes",
                    ..
                }) if count > 1 => {
                    count = (count / 2).max(1);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if prepared.bytes()? > _workspace.size() {
                return Err(ModelError::Schema(
                    "compound exact contributor CAS construction bound",
                ));
            }
            authorization::execute_prepared_owner_effect(client, prepared).await?;
            offset += count;
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod witness_window_controls {
    use super::*;
    #[test]
    fn held_headroom_keeps_large_oldest_preparable_when_tiny_tail_fills_pool() {
        let client = Surreal::init();
        let budget = ResourceBudget::fixed(2 << 20).unwrap();
        let mut writer = WitnessWriter::new(
            &client,
            &budget,
            EffectOwner::OpenAttempt(ContentHash([1; 32])),
            "",
            Variables::new(),
        )
        .unwrap();
        let large = 700_000;
        let oldest = budget.reserve("large oldest retained unit", large).unwrap();
        writer.admit_workspace(large).unwrap();
        writer.pending_bytes = large;
        let old_workspace = writer.workspace.size();
        assert_eq!(old_workspace, large + 131072);
        let tail = budget
            .reserve(
                "tiny trailing unit with construction buffers",
                budget.limit() - budget.reserved(),
            )
            .unwrap();
        assert!(matches!(
            writer.admit_workspace(1024),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(
            writer.workspace.size(),
            old_workspace,
            "failed tail admission preserves oldest preparation headroom"
        );
        assert_eq!(budget.reserved(), budget.limit());
        // Flush consumes the held workspace; it does not need a duplicate reservation.
        assert!(
            budget
                .reserve("old duplicate flush reservation", old_workspace)
                .is_err()
        );
        let mut row = Object::new();
        row.insert("id", RecordId::new("native_completed_view", "oldest"));
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(vec![b'x'; 32768]),
        );
        let (sql, bindings) = writer.plan(vec![Value::Object(row)], Vec::new()).unwrap();
        assert!(immutable_window::plan_bytes(&sql, &bindings).unwrap() < writer.workspace.size());
        drop(bindings);
        drop(sql);
        drop(tail);
        drop(oldest);
        drop(writer);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn child_serialization_is_admitted_before_building_native_payload() {
        let client = Surreal::init();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut writer = WitnessWriter::new(
            &client,
            &budget,
            EffectOwner::OpenAttempt(ContentHash([1; 32])),
            "",
            Variables::new(),
        )
        .unwrap();
        let built = std::sync::atomic::AtomicBool::new(false);
        let descriptor = "x".repeat(1 << 20);
        let result = writer
            .push(&descriptor, || {
                built.store(true, std::sync::atomic::Ordering::Relaxed);
                Err(ModelError::Schema("unadmitted builder ran"))
            })
            .await;
        assert!(matches!(result, Err(ModelError::Resource { .. })));
        assert!(!built.load(std::sync::atomic::Ordering::Relaxed));
        drop(writer);
        assert_eq!(budget.reserved(), 0);
    }
}
