//! Exact physical candidate inspection followed by an atomic expected-incarnation hold.
//! These capabilities remain installation-local; the durable owning attempt retains them.
use super::*;
use lctx_model::domain::resources::{Reservation, ResourceBudget, MAX_ROW_BYTES};

pub struct ExpectedRecord {
    record: RecordId,
    incarnation: i64,
    row: Value,
    _charge: Box<dyn Reservation>,
}
impl ExpectedRecord {
    pub fn record(&self) -> &RecordId { &self.record }
    pub fn incarnation(&self) -> i64 { self.incarnation }
    pub fn row(&self) -> &Value { &self.row }
}
pub struct HeldRecord {
    expected: ExpectedRecord,
    owner: RecordId,
}
impl HeldRecord {
    pub fn record(&self) -> &RecordId { self.expected.record() }
    pub fn incarnation(&self) -> i64 { self.expected.incarnation() }
    pub fn row(&self) -> &Value { self.expected.row() }
    pub fn owner(&self) -> &RecordId { &self.owner }
}

/// Discovery grants no reuse authority. The subsequent hold compares both observed
/// bytes and incarnation in its write transaction, so intervening recreation refuses.
pub async fn inspect_expected(client: &Surreal<Client>, record: RecordId, budget: &ResourceBudget)
    -> Result<Option<ExpectedRecord>, ModelError> {
    let mut charge = budget.reserve("native expected record inspection", MAX_ROW_BYTES)?;
    let guard = guard_id(&Value::RecordId(record.clone()))?;
    let mut variables = Variables::new(); variables.insert("record", record.clone()); variables.insert("guard", guard);
    let mut response = client.query("SELECT * FROM $record; SELECT phase,incarnation FROM $guard")
        .bind(variables).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
    let guards: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    if rows.is_empty() { return Ok(None); }
    let [row] = rows.as_slice() else { return Err(ModelError::Schema("expected native record cardinality")); };
    let [guard] = guards.as_slice() else { return Err(ModelError::Conflict("expected native record guard missing")); };
    let Some(Value::Number(surrealdb::types::Number::Int(incarnation))) = guard.get("incarnation") else {
        return Err(ModelError::Schema("expected native incarnation"));
    };
    if *incarnation <= 0 || guard.get("phase") != Some(&Value::String("active".into())) {
        return Err(ModelError::Conflict("expected native record not active"));
    }
    let bytes = crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536);
    if bytes > MAX_ROW_BYTES { return Err(ModelError::Schema("expected native representation bound")); }
    charge.try_resize(bytes)?;
    Ok(Some(ExpectedRecord { record, incarnation: *incarnation, row: rows.into_iter().next().unwrap(), _charge: charge }))
}

/// The expected identity is never refreshed by retry. Retirement and incoming holds
/// contend on the same guard; bytes or incarnation changes remain explicit conflicts.
pub async fn hold_expected(client: &Surreal<Client>, attempt: ContentHash, owner: RecordId,
    expected: ExpectedRecord) -> Result<HeldRecord, ModelError> {
    let Some((authority, hold_sql, mut bindings)) = hold_plan(Some(attempt), vec![(owner.clone(), expected.record.clone())])? else {
        return Err(ModelError::Schema("expected native hold plan"));
    };
    let guard = guard_id(&Value::RecordId(expected.record.clone()))?;
    bindings.insert("expected_record", expected.record.clone());
    bindings.insert("expected_guard", guard);
    bindings.insert("expected_incarnation", expected.incarnation);
    bindings.insert("expected_bytes", expected.row.clone());
    let sql = format!("LET $expected_guard_row=SELECT * FROM ONLY $expected_guard FOR UPDATE; LET $expected_row=SELECT * FROM ONLY $expected_record FOR UPDATE; IF $expected_guard_row.phase!='active' OR $expected_guard_row.incarnation!=$expected_incarnation OR $expected_row!=$expected_bytes {{ THROW 'native expected incarnation or bytes changed'; }}; {hold_sql}");
    effect_for_owner(client, authority, &sql, bindings).await?;
    Ok(HeldRecord { expected, owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn stale_expected_incarnation_or_bytes_never_creates_the_incoming_hold() {
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"))).unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(&config,
            lctx_model::domain::admission::Frontier::Facts).await.unwrap();
        let client = lctx_compiler_store::reader::connect(&config.endpoint, &config.writer_credentials(),
            store.namespace().as_str(), store.database().as_str()).await.unwrap();
        let attempt = store.attempt();
        let record = RecordId::new("original", format!("{}-candidate", attempt.hex()));
        let owner = RecordId::new("original", format!("{}-receiver", attempt.hex()));
        let rows = [record.clone(), owner.clone()].into_iter().map(|id| {
            let mut row = Object::new(); row.insert("id", id); row.insert("byte_len", 0i64);
            row.insert("content", ContentHash::of(b"").hex()); Value::Object(row)
        }).collect();
        ensure_rows(&client, Some(attempt), rows).await.unwrap();
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let expected = inspect_expected(&client, record.clone(), &budget).await.unwrap().unwrap();
        let guard = guard_id(&Value::RecordId(record.clone())).unwrap();
        client.query("UPDATE $guard SET incarnation+=1 RETURN NONE")
            .bind(("guard", guard)).await.unwrap().check().unwrap();
        assert!(hold_expected(&client, attempt, owner.clone(), expected).await.is_err());
        let expected = inspect_expected(&client, record.clone(), &budget).await.unwrap().unwrap();
        client.query("UPDATE $record SET byte_len=1 RETURN NONE")
            .bind(("record", record.clone())).await.unwrap().check().unwrap();
        assert!(hold_expected(&client, attempt, owner.clone(), expected).await.is_err());
        let mut response = client.query("SELECT * FROM native_hold WHERE owner=$owner AND object=$record")
            .bind(("owner", owner.clone())).bind(("record", record.clone())).await.unwrap().check().unwrap();
        let holds: Vec<Value> = response.take(0).unwrap(); assert!(holds.is_empty());
        let expected = inspect_expected(&client, record.clone(), &budget).await.unwrap().unwrap();
        let held = hold_expected(&client, attempt, owner, expected).await.unwrap();
        assert_eq!(held.record(), &record); assert_eq!(held.incarnation(), 2);
        drop(held); assert_eq!(budget.reserved(), 0);
        client.invalidate().await.unwrap(); store.abandon().await.unwrap();
    }
}
