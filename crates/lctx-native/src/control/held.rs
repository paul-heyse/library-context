//! Exact physical candidate inspection followed by an atomic expected-incarnation hold.
//! These capabilities remain installation-local; the durable owning attempt retains them.
use super::*;
use lctx_model::domain::resources::{MAX_ROW_BYTES, Reservation, ResourceBudget};

pub struct ExpectedRecord {
    record: RecordId,
    incarnation: i64,
    row: Value,
    _charge: Box<dyn Reservation>,
}
impl ExpectedRecord {
    pub fn record(&self) -> &RecordId {
        &self.record
    }
    pub fn incarnation(&self) -> i64 {
        self.incarnation
    }
    pub fn row(&self) -> &Value {
        &self.row
    }
    /// Consume the expected-record capability into data and its existing arena.
    /// No identity/incarnation or receiving authority survives this transfer.
    pub(crate) fn into_owned_row(self) -> (Value, Box<dyn Reservation>) {
        (self.row, self._charge)
    }
}
pub struct HeldRecord {
    expected: ExpectedRecord,
    owner: RecordId,
}
impl HeldRecord {
    pub fn record(&self) -> &RecordId {
        self.expected.record()
    }
    pub fn incarnation(&self) -> i64 {
        self.expected.incarnation()
    }
    pub fn row(&self) -> &Value {
        self.expected.row()
    }
    pub fn owner(&self) -> &RecordId {
        &self.owner
    }
}

/// Discovery grants no reuse authority. The subsequent hold compares both observed
/// bytes and incarnation in its write transaction, so intervening recreation refuses.
pub async fn inspect_expected(
    client: &Surreal<Client>,
    record: RecordId,
    budget: &ResourceBudget,
) -> Result<Option<ExpectedRecord>, ModelError> {
    let reader =
        crate::NativeReader::private(std::sync::Arc::new(client.clone())).with_budget(budget);
    let result = inspect_expected_read(&reader, record, budget).await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step("expected record owned reader close", reader.close().await);
    lctx_model::domain::completion::complete(result, terminal)
}

/// The expected identity is never refreshed by retry. Retirement and incoming holds
/// contend on the same guard; bytes or incarnation changes remain explicit conflicts.
pub async fn hold_expected(
    client: &Surreal<Client>,
    attempt: ContentHash,
    owner: RecordId,
    expected: ExpectedRecord,
) -> Result<HeldRecord, ModelError> {
    let Some((authority, hold_sql, mut bindings)) = hold_plan(
        Some(attempt),
        vec![(owner.clone(), expected.record.clone())],
    )?
    else {
        return Err(ModelError::Schema("expected native hold plan"));
    };
    let guard = guard_id(&Value::RecordId(expected.record.clone()))?;
    bindings.insert("expected_record", expected.record.clone());
    bindings.insert("expected_guard", guard);
    bindings.insert("expected_incarnation", expected.incarnation);
    bindings.insert("expected_bytes", expected.row.clone());
    let sql = format!(
        "LET $expected_guard_row=SELECT * FROM ONLY $expected_guard FOR UPDATE; LET $expected_row=SELECT * FROM ONLY $expected_record FOR UPDATE; IF $expected_guard_row.phase!='active' OR $expected_guard_row.incarnation!=$expected_incarnation OR $expected_row!=$expected_bytes {{ THROW 'native expected incarnation or bytes changed'; }}; {hold_sql}"
    );
    effect_for_owner(client, authority, &sql, bindings).await?;
    Ok(HeldRecord { expected, owner })
}

/// A bounded, identity-correlated observation. Extraction transfers each row's
/// independent reservation; dropping the window cannot release an extracted row.
/// These separate reads do not promise an atomic snapshot. Receiving effects must
/// still compare the exact bytes and incarnation they consume.
pub struct ExpectedRecords {
    rows: std::collections::BTreeMap<RecordId, Option<ExpectedRecord>>,
    _charge: Box<dyn Reservation>,
}
impl ExpectedRecords {
    pub fn get(&self, record: &RecordId) -> Result<Option<&ExpectedRecord>, ModelError> {
        self.rows
            .get(record)
            .map(Option::as_ref)
            .ok_or(ModelError::Conflict("native expected unrequested record"))
    }
    pub fn contains(&self, record: &RecordId) -> bool {
        self.rows.get(record).is_some_and(Option::is_some)
    }
    pub fn take_optional(
        &mut self,
        record: &RecordId,
    ) -> Result<Option<ExpectedRecord>, ModelError> {
        self.rows
            .remove(record)
            .ok_or(ModelError::Conflict("native expected unrequested record"))
    }
    pub fn take_required(&mut self, record: &RecordId) -> Result<ExpectedRecord, ModelError> {
        self.take_optional(record)?.ok_or(ModelError::Conflict(
            "native expected required record missing",
        ))
    }
    pub fn finish(self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            Ok(())
        } else {
            Err(ModelError::Conflict(
                "native expected unconsumed observations",
            ))
        }
    }
    fn new(records: &[RecordId], budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = window_charge(records.iter(), records.len(), budget)?;
        let mut rows = std::collections::BTreeMap::new();
        for record in records {
            if rows.insert(record.clone(), None).is_some() {
                return Err(ModelError::Conflict(
                    "native expected duplicate requested record",
                ));
            }
        }
        Ok(Self {
            rows,
            _charge: charge,
        })
    }
    fn observe_record(&mut self, row: Value, budget: &ResourceBudget) -> Result<(), ModelError> {
        let record = row_id(&row)?;
        let observed = self
            .rows
            .get_mut(record)
            .ok_or(ModelError::Conflict("native expected foreign record"))?;
        if observed.is_some() {
            return Err(ModelError::Conflict("native expected duplicate record"));
        }
        let charge = retained_row_charge(&row, budget, "native owned expected inspection")?;
        *observed = Some(ExpectedRecord {
            record: record.clone(),
            incarnation: 0,
            row,
            _charge: charge,
        });
        Ok(())
    }
    fn guard_records(&self) -> Result<std::collections::BTreeMap<RecordId, RecordId>, ModelError> {
        self.rows
            .iter()
            .filter(|(_, row)| row.is_some())
            .map(|(record, _)| Ok((guard_id(&Value::RecordId(record.clone()))?, record.clone())))
            .collect()
    }
    fn guards_complete(
        &self,
        guards: &std::collections::BTreeMap<RecordId, RecordId>,
    ) -> Result<(), ModelError> {
        if !guards.is_empty() || self.rows.values().flatten().any(|row| row.incarnation <= 0) {
            return Err(ModelError::Conflict("native expected guard missing"));
        }
        Ok(())
    }
    fn observe_guard(
        &mut self,
        expected: &mut std::collections::BTreeMap<RecordId, RecordId>,
        row: Value,
    ) -> Result<(), ModelError> {
        let id = row_id(&row)?;
        let record = expected.remove(id).ok_or(ModelError::Conflict(
            "native expected duplicate or foreign guard",
        ))?;
        let guard = row
            .as_object()
            .ok_or(ModelError::Schema("native expected guard object"))?;
        let incarnation = authorization::int_field(guard, "incarnation")?;
        if incarnation <= 0 || guard.get("phase") != Some(&Value::String("active".into())) {
            return Err(ModelError::Conflict("native expected object not active"));
        }
        self.rows
            .get_mut(&record)
            .and_then(Option::as_mut)
            .ok_or(ModelError::Conflict("native expected guard correspondence"))?
            .incarnation = incarnation;
        Ok(())
    }
}

/// Full exact hold bytes with independently retained allocation ownership. Holds
/// have id/owner/object incidence, and no separate physical guard of their own.
pub struct ExpectedHold {
    record: RecordId,
    owner: RecordId,
    object: RecordId,
    row: Value,
    _charge: Box<dyn Reservation>,
}
impl ExpectedHold {
    pub fn record(&self) -> &RecordId {
        &self.record
    }
    pub fn owner(&self) -> &RecordId {
        &self.owner
    }
    pub fn object(&self) -> &RecordId {
        &self.object
    }
    pub fn row(&self) -> &Value {
        &self.row
    }
}
pub struct ExpectedHolds {
    rows: std::collections::BTreeMap<RecordId, ((RecordId, RecordId), Option<ExpectedHold>)>,
    _charge: Box<dyn Reservation>,
}
impl ExpectedHolds {
    pub fn get(
        &self,
        owner: &RecordId,
        object: &RecordId,
    ) -> Result<Option<&ExpectedHold>, ModelError> {
        self.rows
            .get(&hold_record_id(owner, object)?)
            .map(|(_, row)| row.as_ref())
            .ok_or(ModelError::Conflict("native hold unrequested pair"))
    }
    pub fn contains(&self, owner: &RecordId, object: &RecordId) -> Result<bool, ModelError> {
        Ok(self.get(owner, object)?.is_some())
    }
    pub fn take_optional(
        &mut self,
        owner: &RecordId,
        object: &RecordId,
    ) -> Result<Option<ExpectedHold>, ModelError> {
        self.rows
            .remove(&hold_record_id(owner, object)?)
            .map(|(_, row)| row)
            .ok_or(ModelError::Conflict("native hold unrequested pair"))
    }
    pub fn take_required(
        &mut self,
        owner: &RecordId,
        object: &RecordId,
    ) -> Result<ExpectedHold, ModelError> {
        self.take_optional(owner, object)?
            .ok_or(ModelError::Conflict("native required hold missing"))
    }
    pub fn finish(self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            Ok(())
        } else {
            Err(ModelError::Conflict("native hold unconsumed observations"))
        }
    }
    fn new(pairs: &[(RecordId, RecordId)], budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = window_charge(
            pairs.iter().flat_map(|(owner, object)| [owner, object]),
            pairs.len(),
            budget,
        )?;
        let mut rows = std::collections::BTreeMap::new();
        for (owner, object) in pairs {
            if rows
                .insert(
                    hold_record_id(owner, object)?,
                    ((owner.clone(), object.clone()), None),
                )
                .is_some()
            {
                return Err(ModelError::Conflict("native hold duplicate requested pair"));
            }
        }
        Ok(Self {
            rows,
            _charge: charge,
        })
    }
    fn observe(&mut self, row: Value, budget: &ResourceBudget) -> Result<(), ModelError> {
        let record = row_id(&row)?;
        let ((owner, object), observed) = self.rows.get_mut(record).ok_or(ModelError::Conflict(
            "native hold foreign or noncanonical identity",
        ))?;
        let fields = row
            .as_object()
            .ok_or(ModelError::Schema("native hold object"))?;
        if observed.is_some()
            || fields.len() != 3
            || fields.get("owner") != Some(&Value::RecordId(owner.clone()))
            || fields.get("object") != Some(&Value::RecordId(object.clone()))
            || record != &hold_record_id(owner, object)?
        {
            return Err(ModelError::Conflict(
                "native hold duplicate or mismatched incidence",
            ));
        }
        // This fixed three-field envelope is retained without decoding nested
        // containers. Admit its row plus cloned IDs, rather than a decode arena.
        let row_bytes = crate::loader::native_bytes(&row);
        if row_bytes > MAX_ROW_BYTES {
            return Err(ModelError::Schema("native hold representation bound"));
        }
        let bytes = row_bytes.saturating_mul(8).saturating_add(4096);
        let charge = budget.reserve("native owned hold inspection", bytes)?;
        *observed = Some(ExpectedHold {
            record: record.clone(),
            owner: owner.clone(),
            object: object.clone(),
            row,
            _charge: charge,
        });
        Ok(())
    }
}
fn row_id(row: &Value) -> Result<&RecordId, ModelError> {
    match row.as_object().and_then(|row| row.get("id")) {
        Some(Value::RecordId(id)) => Ok(id),
        _ => Err(ModelError::Schema("native observation record identity")),
    }
}
fn window_charge<'a>(
    ids: impl Iterator<Item = &'a RecordId>,
    count: usize,
    budget: &ResourceBudget,
) -> Result<Box<dyn Reservation>, ModelError> {
    if count > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Limit {
            owner: "native-observation",
            limit: "window rows",
            observed: count,
            bound: crate::loader::NATIVE_WINDOW_ROWS,
        });
    }
    let bytes = ids
        .map(|id| {
            crate::selection::node_bytes(id)
                .saturating_mul(8)
                .saturating_add(512)
        })
        .fold(0usize, usize::saturating_add);
    budget.reserve("native observation correspondence", bytes)
}
fn retained_row_charge(
    row: &Value,
    budget: &ResourceBudget,
    owner: &'static str,
) -> Result<Box<dyn Reservation>, ModelError> {
    let row_bytes = crate::loader::native_bytes(row);
    if row_bytes > MAX_ROW_BYTES {
        return Err(ModelError::Schema("native expected representation bound"));
    }
    // A physical row limit and the workspace needed to retain its representation
    // have different units. Admit the unchanged conservative workspace separately.
    let bytes = row_bytes.saturating_mul(32).saturating_add(65536);
    budget.reserve(owner, bytes)
}
fn observation_query(
    ids: Vec<RecordId>,
    guards: bool,
) -> Result<crate::prepared::PreparedQuery, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("observation_ids", ids);
    // Record-ID point sets use the native primary-key access path. There is no
    // owner/object cross product and no per-record SELECT hidden in this query.
    crate::prepared::PreparedQuery::new(
        bindings,
        vec![],
        vec![if guards {
            "SELECT id,phase,incarnation FROM $observation_ids".into()
        } else {
            "SELECT * FROM $observation_ids".into()
        }],
    )
}
pub async fn inspect_expected_window(
    client: &Surreal<Client>,
    records: &[RecordId],
    budget: &ResourceBudget,
) -> Result<ExpectedRecords, ModelError> {
    let reader =
        crate::NativeReader::private(std::sync::Arc::new(client.clone())).with_budget(budget);
    let result = inspect_expected_window_read(&reader, records, budget).await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step(
        "expected record window owned reader close",
        reader.close().await,
    );
    lctx_model::domain::completion::complete(result, terminal)
}
/// Separate record/guard set reads are observations, never a snapshot or authority
/// for unchecked attachment. An absent record is optional; a present unguarded row refuses.
pub async fn inspect_expected_window_read<Context>(
    reader: &crate::NativeReader<Context>,
    records: &[RecordId],
    budget: &ResourceBudget,
) -> Result<ExpectedRecords, ModelError> {
    let mut observed = ExpectedRecords::new(records, budget)?;
    if records.is_empty() {
        return Ok(observed);
    }
    let mut stream = reader
        .stream_prepared(observation_query(records.to_vec(), false)?)?
        .with_row_bytes(lctx_model::domain::resources::MAX_ROW_BYTES);
    let result = async {
        while let Some(row) = stream.next().await? {
            observed.observe_record(row, budget)?;
        }
        Ok(())
    }
    .await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step(
        "expected record window stream drain",
        stream.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, terminal)?;
    let mut guards = observed.guard_records()?;
    if !guards.is_empty() {
        let mut stream =
            reader.stream_prepared(observation_query(guards.keys().cloned().collect(), true)?)?;
        let result = async {
            while let Some(row) = stream.next().await? {
                observed.observe_guard(&mut guards, row)?;
            }
            observed.guards_complete(&guards)
        }
        .await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "expected guard window stream drain",
            stream.drain_transport().await,
        );
        lctx_model::domain::completion::complete(result, terminal)?;
    }
    Ok(observed)
}
pub async fn inspect_hold_window(
    client: &Surreal<Client>,
    pairs: &[(RecordId, RecordId)],
    budget: &ResourceBudget,
) -> Result<ExpectedHolds, ModelError> {
    let reader =
        crate::NativeReader::private(std::sync::Arc::new(client.clone())).with_budget(budget);
    let result = inspect_hold_window_read(&reader, pairs, budget).await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step("hold window owned reader close", reader.close().await);
    lctx_model::domain::completion::complete(result, terminal)
}
pub async fn inspect_hold_window_read<Context>(
    reader: &crate::NativeReader<Context>,
    pairs: &[(RecordId, RecordId)],
    budget: &ResourceBudget,
) -> Result<ExpectedHolds, ModelError> {
    let mut observed = ExpectedHolds::new(pairs, budget)?;
    if pairs.is_empty() {
        return Ok(observed);
    }
    let mut stream = reader
        .stream_prepared(observation_query(
            observed.rows.keys().cloned().collect(),
            false,
        )?)?
        .with_row_bytes(lctx_model::domain::resources::MAX_ROW_BYTES);
    let result = async {
        while let Some(row) = stream.next().await? {
            observed.observe(row, budget)?;
        }
        Ok(())
    }
    .await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step("hold window stream drain", stream.drain_transport().await);
    lctx_model::domain::completion::complete(result, terminal)?;
    Ok(observed)
}
/// Private-reader singleton inspection retains the same optional discovery contract.
pub(crate) async fn inspect_expected_read(
    reader: &crate::NativeReader<()>,
    record: RecordId,
    budget: &ResourceBudget,
) -> Result<Option<ExpectedRecord>, ModelError> {
    let mut window =
        inspect_expected_window_read(reader, std::slice::from_ref(&record), budget).await?;
    let expected = window.take_optional(&record)?;
    window.finish()?;
    Ok(expected)
}
/// Fixed attempt→completion acquisition. Neither a different owner nor arbitrary SQL is accepted.
pub(crate) async fn hold_frozen_completed_expected(
    client: &Surreal<Client>,
    completion: &crate::completed_state::FrozenCompletion,
    expected: ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<HeldRecord, ModelError> {
    if expected.record() != &completion.record()? || expected.row() != completion.value(true)? {
        return Err(ModelError::Conflict(
            "frozen completion hold exact expected bytes",
        ));
    }
    let _charge = budget.reserve(
        "frozen completion held transaction",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let owner = RecordId::new("native_attempt", completion.attempt.hex());
    let (_, hold_sql, mut bindings) = hold_plan(
        Some(completion.attempt),
        vec![(owner.clone(), expected.record.clone())],
    )?
    .ok_or(ModelError::Schema("frozen completion hold plan"))?;
    let fence = completion.fence();
    frozen::bind_fence(&fence, &mut bindings);
    bindings.insert("expected_record", expected.record.clone());
    bindings.insert(
        "expected_guard",
        guard_id(&Value::RecordId(expected.record.clone()))?,
    );
    bindings.insert("expected_incarnation", expected.incarnation);
    bindings.insert("expected_bytes", expected.row.clone());
    bindings.insert("completed_identity", completion.identity()?.hex());
    bindings.insert("completed_logical", completion.completed_state.hex());
    let sql = format!(
        "{} IF !$__owner.admitted OR $__owner.completed_identity!=$completed_identity OR $__owner.completed_state!=$completed_logical {{ THROW 'native frozen completion admission changed'; }}; LET $guard=SELECT * FROM ONLY $expected_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $expected_record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected_incarnation OR $actual!=$expected_bytes {{ THROW 'native frozen completion bytes/incarnation changed'; }}; {hold_sql}",
        frozen::FENCE_GUARD
    );
    effect_for_owner(
        client,
        EffectOwner::FrozenAttempt(completion.attempt),
        &sql,
        bindings,
    )
    .await?;
    Ok(HeldRecord { expected, owner })
}

/// Re-hold the exact persisted admission for this currently admitted frozen attempt.
/// Admission bytes and incarnation are captured independently before this fixed effect.
pub(crate) async fn hold_frozen_admission_expected(
    client: &Surreal<Client>,
    completion: &crate::completed_state::FrozenCompletion,
    admission: &crate::completed_state::CompletedAdmission,
    expected: ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<HeldRecord, ModelError> {
    if expected.record() != &admission.record()? || expected.row() != admission.value()? {
        return Err(ModelError::Conflict(
            "semantic admission hold exact expected envelope",
        ));
    }
    let _charge = budget.reserve(
        "semantic admission hold construction",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let owner = RecordId::new("native_attempt", completion.attempt.hex());
    let (_, hold_sql, mut variables) = hold_plan(
        Some(completion.attempt),
        vec![(owner.clone(), expected.record.clone())],
    )?
    .ok_or(ModelError::Schema("semantic admission hold plan"))?;
    frozen::bind_fence(&completion.fence(), &mut variables);
    variables.insert("admission", admission.record()?);
    variables.insert(
        "admission_guard",
        guard_id(&Value::RecordId(admission.record()?))?,
    );
    variables.insert("admission_incarnation", expected.incarnation);
    variables.insert("admission_bytes", expected.row.clone());
    variables.insert("completion_identity", completion.identity()?.hex());
    variables.insert(
        "completion_guard",
        guard_id(&Value::RecordId(completion.record()?))?,
    );
    variables.insert("completion_incarnation", admission.completion_incarnation);
    let sql = format!(
        "{} IF !$__owner.admitted OR $__owner.completed_identity!=$completion_identity OR $__owner.completed_admission!=$admission {{ THROW 'frozen semantic admission owner changed'; }}; LET $semantic_guard=SELECT * FROM ONLY $admission_guard FOR UPDATE; LET $semantic=SELECT * FROM ONLY $admission FOR UPDATE; LET $physical_guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; IF $semantic_guard.phase!='active' OR $semantic_guard.incarnation!=$admission_incarnation OR $semantic!=$admission_bytes OR $physical_guard.phase!='active' OR $physical_guard.incarnation!=$completion_incarnation {{ THROW 'semantic admission bytes/incarnation changed'; }}; {hold_sql}",
        frozen::FENCE_GUARD
    );
    effect_for_owner(
        client,
        EffectOwner::FrozenAttempt(completion.attempt),
        &sql,
        variables,
    )
    .await?;
    Ok(HeldRecord { expected, owner })
}
/// Effect-free physical CAS plan. This grants no semantic capability and executes no
/// request. Compiler storage consumes its private full artifact owner before submitting
/// this plan through the native frozen-attempt transaction machinery.
pub fn completed_admission_plan(
    physical: &crate::completed_state::NativeCompletedState,
    admission: &crate::completed_state::CompletedAdmission,
    budget: &ResourceBudget,
) -> Result<
    (
        String,
        Variables,
        Box<dyn lctx_model::domain::resources::Reservation>,
    ),
    ModelError,
> {
    let data = physical.completion();
    let charge = budget.reserve(
        "semantic admission durable transaction",
        crate::loader::native_bytes(&admission.value()?)
            .saturating_mul(32)
            .saturating_add(262144),
    )?;
    let (row_sql, mut variables) = immutable_row_plan(None, None, vec![admission.value()?])?
        .ok_or(ModelError::Schema("semantic admission immutable plan"))?;
    let (_, hold_sql, holds) = hold_plan(
        Some(data.attempt),
        vec![
            (
                RecordId::new("native_attempt", data.attempt.hex()),
                admission.record()?,
            ),
            (admission.record()?, data.record()?),
        ],
    )?
    .ok_or(ModelError::Schema("semantic admission durable holds"))?;
    merge_plan_bindings(&mut variables, holds)?;
    frozen::bind_fence(&data.fence(), &mut variables);
    variables.insert("completion", data.record()?);
    variables.insert(
        "completion_guard",
        guard_id(&Value::RecordId(data.record()?))?,
    );
    variables.insert("completion_incarnation", physical.incarnation());
    variables.insert("completion_bytes", data.value(true)?);
    variables.insert("completion_identity", data.identity()?.hex());
    variables.insert("completion_logical", data.completed_state.hex());
    variables.insert("admission", admission.record()?);
    let sql = format!(
        "{} LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes OR ($__owner.admitted AND $__owner.completed_admission!=$admission) {{ THROW 'semantic admission physical completion changed'; }}; {row_sql}; {hold_sql}; UPDATE $__attempt SET admitted=true,completed_identity=$completion_identity,completed_state=$completion_logical,completed_admission=$admission RETURN NONE",
        frozen::FENCE_GUARD
    );
    Ok((sql, variables, charge))
}

pub(crate) async fn hold_frozen_physical_expected(
    client: &Surreal<Client>,
    completion: &crate::completed_state::FrozenCompletion,
    expected: ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<HeldRecord, ModelError> {
    if expected.row() != completion.value(true)? || expected.record() != &completion.record()? {
        return Err(ModelError::Conflict(
            "physical completed hold exact envelope",
        ));
    }
    let _charge = budget.reserve(
        "physical completion hold",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(131072),
    )?;
    let owner = RecordId::new("native_attempt", completion.attempt.hex());
    let (_, holds, mut variables) = hold_plan(
        Some(completion.attempt),
        vec![(owner.clone(), completion.record()?)],
    )?
    .ok_or(ModelError::Schema("physical completed hold plan"))?;
    frozen::bind_fence(&completion.fence(), &mut variables);
    variables.insert("completion", completion.record()?);
    variables.insert(
        "completion_guard",
        guard_id(&Value::RecordId(completion.record()?))?,
    );
    variables.insert("completion_bytes", expected.row.clone());
    variables.insert("completion_incarnation", expected.incarnation);
    variables.insert("completion_identity", completion.identity()?.hex());
    let sql = format!(
        "{} LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes OR $__owner.completed_identity!=$completion_identity {{ THROW 'physical completed hold changed'; }}; {holds}",
        frozen::FENCE_GUARD
    );
    effect_for_owner(
        client,
        EffectOwner::FrozenAttempt(completion.attempt),
        &sql,
        variables,
    )
    .await?;
    Ok(HeldRecord { expected, owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::sdk_error;
    fn record_row(id: &RecordId, bytes: i64) -> Value {
        let mut row = Object::new();
        row.insert("id", id.clone());
        row.insert("bytes", bytes);
        // The observed bytes retain native variants, not JSON-normalized surrogates.
        row.insert("target", RecordId::new("foreign_table", "nested"));
        Value::Object(row)
    }
    fn guard_row(id: &RecordId, incarnation: i64, phase: &str) -> Value {
        let mut row = Object::new();
        row.insert("id", guard_id(&Value::RecordId(id.clone())).unwrap());
        row.insert("incarnation", incarnation);
        row.insert("phase", phase.to_owned());
        Value::Object(row)
    }
    fn hold_row(owner: &RecordId, object: &RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", hold_record_id(owner, object).unwrap());
        row.insert("owner", owner.clone());
        row.insert("object", object.clone());
        Value::Object(row)
    }
    fn native_probe_row(id: &RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", id.clone());
        row.insert("semantic_type", "observation_probe");
        row.insert("semantic_key", "00000000000000000000000000000000");
        row.insert("content", ContentHash::of(b"observation probe").hex());
        row.insert(
            "canonical",
            surrealdb::types::Bytes::from(b"observation probe".to_vec()),
        );
        row.insert("body", Object::new());
        row.insert("scope_keys", Vec::<String>::new());
        Value::Object(row)
    }
    #[test]
    fn observation_window_correlates_shuffled_native_bytes_and_independent_lifetimes() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let a = RecordId::new("one", "a");
        let b = RecordId::new("two", "b");
        let absent = RecordId::new("one", "absent");
        let a_row = record_row(&a, 7);
        let b_row = record_row(&b, 29);
        let mut window =
            ExpectedRecords::new(&[a.clone(), absent.clone(), b.clone()], &budget).unwrap();
        window.observe_record(b_row.clone(), &budget).unwrap();
        window.observe_record(a_row.clone(), &budget).unwrap();
        let mut guards = window.guard_records().unwrap();
        window
            .observe_guard(&mut guards, guard_row(&b, 19, "active"))
            .unwrap();
        assert!(window.guards_complete(&guards).is_err());
        window
            .observe_guard(&mut guards, guard_row(&a, 4, "active"))
            .unwrap();
        window.guards_complete(&guards).unwrap();
        assert!(window.contains(&a));
        assert!(!window.contains(&absent));
        assert_eq!(window.get(&a).unwrap().unwrap().row(), &a_row);
        assert!(window.take_optional(&absent).unwrap().is_none());
        let expected_b = window.take_required(&b).unwrap();
        let expected_a = window.take_required(&a).unwrap();
        assert_eq!(expected_b.incarnation(), 19);
        assert_eq!(expected_a.incarnation(), 4);
        assert_eq!(expected_b.row(), &b_row);
        window.finish().unwrap();
        let both = budget.reserved();
        assert!(both > 0);
        drop(expected_a);
        assert!(budget.reserved() > 0 && budget.reserved() < both);
        drop(expected_b);
        assert_eq!(budget.reserved(), 0);
        let mut missing = ExpectedRecords::new(std::slice::from_ref(&absent), &budget).unwrap();
        assert!(missing.take_required(&absent).is_err());
        missing.finish().unwrap();
    }
    #[test]
    fn observation_window_refuses_foreign_duplicate_missing_and_inactive_guards() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let a = RecordId::new("one", "a");
        let b = RecordId::new("two", "b");
        assert!(ExpectedRecords::new(&[a.clone(), a.clone()], &budget).is_err());
        let mut window = ExpectedRecords::new(std::slice::from_ref(&a), &budget).unwrap();
        assert!(window.observe_record(record_row(&b, 0), &budget).is_err());
        window.observe_record(record_row(&a, 0), &budget).unwrap();
        assert!(window.observe_record(record_row(&a, 0), &budget).is_err());
        let mut guards = window.guard_records().unwrap();
        assert!(window.guards_complete(&guards).is_err());
        assert!(
            window
                .observe_guard(&mut guards, guard_row(&b, 1, "active"))
                .is_err()
        );
        window
            .observe_guard(&mut guards, guard_row(&a, 1, "active"))
            .unwrap();
        assert!(
            window
                .observe_guard(&mut guards, guard_row(&a, 1, "active"))
                .is_err()
        );
        assert!(window.take_optional(&b).is_err());
        assert!(window.finish().is_err());
        for (incarnation, phase) in [(0, "active"), (2, "retiring"), (3, "vacant")] {
            let mut window = ExpectedRecords::new(std::slice::from_ref(&a), &budget).unwrap();
            window.observe_record(record_row(&a, 0), &budget).unwrap();
            let mut guards = window.guard_records().unwrap();
            assert!(
                window
                    .observe_guard(&mut guards, guard_row(&a, incarnation, phase))
                    .is_err()
            );
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn hold_window_uses_exact_heterogeneous_incidence_and_transfers_reservations() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let a = RecordId::new("owner_a", "a");
        let b = RecordId::new("owner_b", "b");
        let x = RecordId::new("object_x", "x");
        let y = RecordId::new("object_y", "y");
        let pairs = [(a.clone(), x.clone()), (b.clone(), y.clone())];
        let mut window = ExpectedHolds::new(&pairs, &budget).unwrap();
        assert!(window.observe(hold_row(&a, &y), &budget).is_err()); // Cartesian expansion is foreign.
        window.observe(hold_row(&b, &y), &budget).unwrap();
        window.observe(hold_row(&a, &x), &budget).unwrap();
        assert!(window.observe(hold_row(&b, &y), &budget).is_err());
        assert!(window.contains(&a, &x).unwrap());
        let held = window.take_required(&a, &x).unwrap();
        assert_eq!(held.row(), &hold_row(&a, &x));
        assert_eq!(held.owner(), &a);
        assert_eq!(held.object(), &x);
        assert_eq!(held.record(), &hold_record_id(&a, &x).unwrap());
        drop(window);
        assert!(budget.reserved() > 0);
        drop(held);
        assert_eq!(budget.reserved(), 0);
        let mut missing = ExpectedHolds::new(&pairs, &budget).unwrap();
        missing.observe(hold_row(&a, &x), &budget).unwrap();
        drop(missing.take_required(&a, &x).unwrap());
        assert!(missing.take_required(&b, &y).is_err());
        missing.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn hold_window_refuses_noncanonical_identity_mismatched_or_extra_incidence() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let owner = RecordId::new("owner", "a");
        let object = RecordId::new("object", "x");
        let pairs = [(owner.clone(), object.clone())];
        let mut window = ExpectedHolds::new(&pairs, &budget).unwrap();
        assert!(ExpectedHolds::new(&[pairs[0].clone(), pairs[0].clone()], &budget).is_err());
        for (field, value) in [
            (
                "id",
                Value::RecordId(RecordId::new("native_hold", "noncanonical")),
            ),
            ("owner", Value::RecordId(RecordId::new("owner", "wrong"))),
            ("object", Value::RecordId(RecordId::new("object", "wrong"))),
            ("extra", Value::Bool(true)),
        ] {
            let Value::Object(mut row) = hold_row(&owner, &object) else {
                unreachable!()
            };
            row.insert(field, value);
            assert!(window.observe(Value::Object(row), &budget).is_err());
        }
        window.observe(hold_row(&owner, &object), &budget).unwrap();
        drop(window.take_required(&owner, &object).unwrap());
        window.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn observation_admission_refuses_window_and_retention_without_leaking() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let id = RecordId::new("one", "a");
        assert!(
            ExpectedRecords::new(
                &vec![id.clone(); crate::loader::NATIVE_WINDOW_ROWS + 1],
                &budget
            )
            .is_err()
        );
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(ExpectedRecords::new(std::slice::from_ref(&id), &tiny).is_err());
        let small = ResourceBudget::fixed(8192).unwrap();
        let mut window = ExpectedRecords::new(std::slice::from_ref(&id), &small).unwrap();
        let before = small.reserved();
        assert!(window.observe_record(record_row(&id, 0), &small).is_err());
        assert_eq!(small.reserved(), before);
        drop(window);
        assert_eq!(small.reserved(), 0);
        let query = observation_query(vec![id.clone()], false).unwrap();
        let (sql, bindings) = query.into_request();
        assert_eq!(sql, "SELECT * FROM $observation_ids;");
        assert_eq!(
            bindings.get("observation_ids"),
            Some(&Value::Array(vec![Value::RecordId(id)].into()))
        );
    }
    #[test]
    fn observation_composite_identity_has_bounded_correspondence_charge() {
        let budget = ResourceBudget::fixed(128 << 10).unwrap();
        let id = RecordId::new(
            "composite",
            surrealdb::types::RecordIdKey::Array(
                vec![
                    Value::String("scope".into()),
                    Value::Number(surrealdb::types::Number::Int(7)),
                ]
                .into(),
            ),
        );
        let mut window = ExpectedRecords::new(std::slice::from_ref(&id), &budget).unwrap();
        assert!(
            budget.reserved() < 8192,
            "small composite IDs must not reserve the maximum possible row per key"
        );
        window.observe_record(record_row(&id, 1), &budget).unwrap();
        let mut guards = window.guard_records().unwrap();
        window
            .observe_guard(&mut guards, guard_row(&id, 1, "active"))
            .unwrap();
        window.guards_complete(&guards).unwrap();
        drop(window.take_required(&id).unwrap());
        window.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn production_hold_plan_and_observation_share_the_exact_address() {
        let owner = RecordId::new("native_attempt", "a");
        let object = RecordId::new("compiler_contribution", "b");
        let (_, sql, bindings) = hold_plan(
            Some(ContentHash::of(b"attempt")),
            vec![(owner.clone(), object.clone())],
        )
        .unwrap()
        .unwrap();
        assert!(sql.contains("record::table($row.owner)='native_completed_state' { fn::native_completed_incidence($row.owner,$row); }"));
        assert!(!sql.contains("native_realization_incidence"));
        let Some(Value::Array(rows)) = bindings.get("holds") else {
            panic!("holds")
        };
        assert_eq!(rows.as_slice(), &[hold_row(&owner, &object)]);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_observation_windows_correlate_records_guards_and_heterogeneous_holds() {
        use futures::FutureExt;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"),
        ))
        .unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let attempt = store.attempt();
        let ids = ["first", "second", "receiver-a", "receiver-b"]
            .map(|key| RecordId::new("compiler_record", format!("{}-{key}", attempt.hex())));
        let [a, b, owner_a, owner_b] = &ids;
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let pairs = vec![(owner_a.clone(), a.clone()), (owner_b.clone(), b.clone())];
        let guard = guard_id(&Value::RecordId(b.clone())).unwrap();
        let mut restore_guard = None;
        let mut restore_hold = false;
        let outcome = std::panic::AssertUnwindSafe(async {
            ensure_rows(
                &client,
                Some(attempt),
                ids.iter().map(native_probe_row).collect(),
            )
            .await
            .unwrap();
            let absent = RecordId::new("compiler_record", format!("{}-absent", attempt.hex()));
            let mut window =
                inspect_expected_window(&client, &[b.clone(), absent.clone(), a.clone()], &budget)
                    .await
                    .unwrap();
            assert_eq!(window.get(a).unwrap().unwrap().row(), &native_probe_row(a));
            assert!(window.take_optional(&absent).unwrap().is_none());
            let first = window.take_required(a).unwrap();
            let second = window.take_required(b).unwrap();
            window.finish().unwrap();
            assert_eq!(first.record(), a);
            assert_eq!(second.record(), b);
            assert_eq!(second.incarnation(), 1);
            drop(first);
            assert!(budget.reserved() > 0);
            drop(second);
            assert_eq!(budget.reserved(), 0);
            hold_many(&client, Some(attempt), pairs.clone())
                .await
                .unwrap();
            let mut holds =
                inspect_hold_window(&client, &[pairs[1].clone(), pairs[0].clone()], &budget)
                    .await
                    .unwrap();
            let first = holds.take_required(owner_a, a).unwrap();
            let second = holds.take_required(owner_b, b).unwrap();
            holds.finish().unwrap();
            assert_eq!(first.row(), &hold_row(owner_a, a));
            assert_eq!(second.record(), &hold_record_id(owner_b, b).unwrap());
            drop(first);
            assert!(budget.reserved() > 0);
            drop(second);
            assert_eq!(budget.reserved(), 0);

            // Deliberately malformed owned fixture state distinguishes an absent optional
            // record from a present record whose guard is missing. Restore before asserting.
            let mut response = client
                .query("SELECT * FROM ONLY $guard")
                .bind(("guard", guard.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let prior: Option<Value> = response.take(0).unwrap();
            let prior = prior.unwrap();
            restore_guard = Some(prior.clone());
            client
                .query("DELETE $guard RETURN NONE")
                .bind(("guard", guard.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let missing_guard = inspect_expected_window(&client, &[a.clone(), b.clone()], &budget)
                .await
                .is_err();
            client
                .query("UPSERT $guard CONTENT $prior RETURN NONE")
                .bind(("guard", guard.clone()))
                .bind(("prior", prior))
                .await
                .unwrap()
                .check()
                .unwrap();
            restore_guard = None;
            let edge = hold_record_id(owner_b, b).unwrap();
            restore_hold = true;
            client
                .query("DELETE $edge RETURN NONE")
                .bind(("edge", edge))
                .await
                .unwrap()
                .check()
                .unwrap();
            let mut missing = inspect_hold_window(&client, &pairs, &budget).await.unwrap();
            drop(missing.take_required(owner_a, a).unwrap());
            let missing_hold = missing.take_required(owner_b, b).is_err();
            missing.finish().unwrap();
            hold_many(&client, Some(attempt), pairs.clone())
                .await
                .unwrap();
            restore_hold = false;
            assert!(missing_guard);
            assert!(missing_hold);
            assert_eq!(budget.reserved(), 0);
        })
        .catch_unwind()
        .await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        if let Some(prior) = restore_guard {
            terminal.step(
                "owned observation guard restoration",
                client
                    .query("UPSERT $guard CONTENT $prior RETURN NONE")
                    .bind(("guard", guard))
                    .bind(("prior", prior))
                    .await
                    .map_err(sdk_error)
                    .and_then(|response| response.check().map(|_| ()).map_err(sdk_error)),
            );
        }
        if restore_hold {
            terminal.step(
                "owned observation hold restoration",
                hold_many(&client, Some(attempt), pairs).await,
            );
        }
        terminal.step("owned observation source close", store.abandon().await);
        terminal.step(
            "owned observation client close",
            client.invalidate().await.map_err(sdk_error),
        );
        let cleanup = lctx_model::domain::completion::complete(Ok(()), terminal);
        if let Err(primary) = outcome {
            if let Err(error) = cleanup {
                eprintln!("observation control cleanup: {error:#?}");
            }
            std::panic::resume_unwind(primary);
        }
        cleanup.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn stale_expected_incarnation_or_bytes_never_creates_the_incoming_hold() {
        use futures::FutureExt;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"),
        ))
        .unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let attempt = store.attempt();
        let record = RecordId::new("compiler_record", format!("{}-candidate", attempt.hex()));
        let owner = RecordId::new("compiler_record", format!("{}-receiver", attempt.hex()));
        let outcome = std::panic::AssertUnwindSafe(async {
            let rows = [record.clone(), owner.clone()]
                .iter()
                .map(native_probe_row)
                .collect();
            ensure_rows(&client, Some(attempt), rows).await.unwrap();
            let budget = ResourceBudget::fixed(128 << 20).unwrap();
            let expected = inspect_expected(&client, record.clone(), &budget)
                .await
                .unwrap()
                .unwrap();
            let guard = guard_id(&Value::RecordId(record.clone())).unwrap();
            client
                .query("UPDATE $guard SET incarnation+=1 RETURN NONE")
                .bind(("guard", guard))
                .await
                .unwrap()
                .check()
                .unwrap();
            assert!(
                hold_expected(&client, attempt, owner.clone(), expected)
                    .await
                    .is_err()
            );
            let expected = inspect_expected(&client, record.clone(), &budget)
                .await
                .unwrap()
                .unwrap();
            client
                .query("UPDATE $record SET content=$changed RETURN NONE")
                .bind(("record", record.clone()))
                .bind(("changed", ContentHash::of(b"changed").hex()))
                .await
                .unwrap()
                .check()
                .unwrap();
            assert!(
                hold_expected(&client, attempt, owner.clone(), expected)
                    .await
                    .is_err()
            );
            let mut response = client
                .query("SELECT * FROM native_hold WHERE owner=$owner AND object=$record")
                .bind(("owner", owner.clone()))
                .bind(("record", record.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let holds: Vec<Value> = response.take(0).unwrap();
            assert!(holds.is_empty());
            let expected = inspect_expected(&client, record.clone(), &budget)
                .await
                .unwrap()
                .unwrap();
            let held = hold_expected(&client, attempt, owner, expected)
                .await
                .unwrap();
            assert_eq!(held.record(), &record);
            assert_eq!(held.incarnation(), 2);
            drop(held);
            assert_eq!(budget.reserved(), 0);
        })
        .catch_unwind()
        .await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step("owned expected CAS source close", store.abandon().await);
        terminal.step(
            "owned expected CAS client close",
            client.invalidate().await.map_err(sdk_error),
        );
        let cleanup = lctx_model::domain::completion::complete(Ok(()), terminal);
        if let Err(primary) = outcome {
            if let Err(error) = cleanup {
                eprintln!("expected CAS control cleanup: {error:#?}");
            }
            std::panic::resume_unwind(primary);
        }
        cleanup.unwrap();
    }
}
