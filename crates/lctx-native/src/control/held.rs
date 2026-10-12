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
    let reader=crate::NativeReader::private(std::sync::Arc::new(client.clone())).with_budget(budget);
    let result=inspect_expected_read(&reader,record,budget).await;
    let mut terminal=lctx_model::domain::completion::Completion::default();
    terminal.step("expected record owned reader close",reader.close().await);
    lctx_model::domain::completion::complete(result,terminal)
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

/// Private-reader inspection keeps every read under the caller's cancellation/session owner.
pub(crate) async fn inspect_expected_read(reader:&crate::NativeReader<()>,record:RecordId,budget:&ResourceBudget)->Result<Option<ExpectedRecord>,ModelError>{
    use surrealdb::types::SurrealValue;
    let mut charge=budget.reserve("native owned expected inspection",std::mem::size_of::<ExpectedRecord>())?;
    let mut bindings=Variables::new();bindings.insert("record",record.clone());bindings.insert("guard",guard_id(&Value::RecordId(record.clone()))?);
    let result:Object=reader.query_native("RETURN {rows:(SELECT * FROM $record),guards:(SELECT phase,incarnation FROM $guard)}",bindings).await?;
    // The finite reader charges delivery through conversion. Admit this owner's
    // retained/decode copies before extracting or cloning any returned container.
    let retained=result.iter().map(|(name,value)|name.len().saturating_add(crate::loader::native_bytes(value))).fold(std::mem::size_of::<Object>(),usize::saturating_add).saturating_mul(32).saturating_add(65536);
    if retained>MAX_ROW_BYTES{return Err(ModelError::Schema("native expected representation bound"));}
    charge.try_resize(retained)?;
    let rows=Vec::<Value>::from_value(result.get("rows").cloned().ok_or(ModelError::Schema("native expected rows"))?).map_err(ModelError::codec)?;
    let guards=Vec::<Object>::from_value(result.get("guards").cloned().ok_or(ModelError::Schema("native expected guards"))?).map_err(ModelError::codec)?;
    if rows.is_empty(){return Ok(None);}
    let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native expected row cardinality"));};
    let [guard]=guards.as_slice() else{return Err(ModelError::Conflict("native expected guard cardinality"));};
    let incarnation=authorization::int_field(guard,"incarnation")?;
    if incarnation<=0 || guard.get("phase")!=Some(&Value::String("active".into())){return Err(ModelError::Conflict("native expected object not active"));}
    let bytes=crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536);
    if bytes>MAX_ROW_BYTES{return Err(ModelError::Schema("native expected representation bound"));}charge.try_resize(bytes)?;
    Ok(Some(ExpectedRecord{record,incarnation,row:rows.into_iter().next().unwrap(),_charge:charge}))
}
/// Fixed attempt→completion acquisition. Neither a different owner nor arbitrary SQL is accepted.
pub(crate) async fn hold_frozen_completed_expected(client:&Surreal<Client>,completion:&crate::completed_state::FrozenCompletion,
    expected:ExpectedRecord,budget:&ResourceBudget)->Result<HeldRecord,ModelError>{
    if expected.record()!=&completion.record()? || expected.row()!=&completion.value(true)?{return Err(ModelError::Conflict("frozen completion hold exact expected bytes"));}
    let _charge=budget.reserve("frozen completion held transaction",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
    let owner=RecordId::new("native_attempt",completion.attempt.hex());
    let (_,hold_sql,mut bindings)=hold_plan(Some(completion.attempt),vec![(owner.clone(),expected.record.clone())])?.ok_or(ModelError::Schema("frozen completion hold plan"))?;
    let fence=completion.fence();frozen::bind_fence(&fence,&mut bindings);
    bindings.insert("expected_record",expected.record.clone());bindings.insert("expected_guard",guard_id(&Value::RecordId(expected.record.clone()))?);
    bindings.insert("expected_incarnation",expected.incarnation);bindings.insert("expected_bytes",expected.row.clone());
    bindings.insert("completed_identity",completion.identity()?.hex());bindings.insert("completed_logical",completion.completed_state.hex());
    let sql=format!("{} IF !$__owner.admitted OR $__owner.completed_identity!=$completed_identity OR $__owner.completed_state!=$completed_logical {{ THROW 'native frozen completion admission changed'; }}; LET $guard=SELECT * FROM ONLY $expected_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $expected_record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected_incarnation OR $actual!=$expected_bytes {{ THROW 'native frozen completion bytes/incarnation changed'; }}; {hold_sql}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(completion.attempt),&sql,bindings).await?;
    Ok(HeldRecord{expected,owner})
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

/// Re-hold the exact persisted admission for this currently admitted frozen attempt.
/// Admission bytes and incarnation are captured independently before this fixed effect.
pub(crate) async fn hold_frozen_admission_expected(client:&Surreal<Client>,completion:&crate::completed_state::FrozenCompletion,
    admission:&crate::completed_state::CompletedAdmission,expected:ExpectedRecord,budget:&ResourceBudget)->Result<HeldRecord,ModelError>{
    if expected.record()!=&admission.record()? || expected.row()!=&admission.value()? {return Err(ModelError::Conflict("semantic admission hold exact expected envelope"));}
    let _charge=budget.reserve("semantic admission hold construction",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
    let owner=RecordId::new("native_attempt",completion.attempt.hex());
    let (_,hold_sql,mut variables)=hold_plan(Some(completion.attempt),vec![(owner.clone(),expected.record.clone())])?.ok_or(ModelError::Schema("semantic admission hold plan"))?;
    frozen::bind_fence(&completion.fence(),&mut variables);
    variables.insert("admission",admission.record()?);variables.insert("admission_guard",guard_id(&Value::RecordId(admission.record()?))?);
    variables.insert("admission_incarnation",expected.incarnation);variables.insert("admission_bytes",expected.row.clone());
    variables.insert("completion_identity",completion.identity()?.hex());variables.insert("completion_guard",guard_id(&Value::RecordId(completion.record()?))?);
    variables.insert("completion_incarnation",admission.completion_incarnation);
    let sql=format!("{} IF !$__owner.admitted OR $__owner.completed_identity!=$completion_identity OR $__owner.completed_admission!=$admission {{ THROW 'frozen semantic admission owner changed'; }}; LET $semantic_guard=SELECT * FROM ONLY $admission_guard FOR UPDATE; LET $semantic=SELECT * FROM ONLY $admission FOR UPDATE; LET $physical_guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; IF $semantic_guard.phase!='active' OR $semantic_guard.incarnation!=$admission_incarnation OR $semantic!=$admission_bytes OR $physical_guard.phase!='active' OR $physical_guard.incarnation!=$completion_incarnation {{ THROW 'semantic admission bytes/incarnation changed'; }}; {hold_sql}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(completion.attempt),&sql,variables).await?;
    Ok(HeldRecord{expected,owner})
}
/// Effect-free physical CAS plan. This grants no semantic capability and executes no
/// request. Compiler storage consumes its private full artifact owner before submitting
/// this plan through the native frozen-attempt transaction machinery.
pub fn completed_admission_plan(physical:&crate::completed_state::NativeCompletedState,
    admission:&crate::completed_state::CompletedAdmission,budget:&ResourceBudget)->Result<(String,Variables,Box<dyn lctx_model::domain::resources::Reservation>),ModelError>{
    let data=physical.completion();
    let charge=budget.reserve("semantic admission durable transaction",crate::loader::native_bytes(&admission.value()?).saturating_mul(32).saturating_add(262144))?;
    let (row_sql,mut variables)=immutable_row_plan(None,None,vec![admission.value()?])?.ok_or(ModelError::Schema("semantic admission immutable plan"))?;
    let (_,hold_sql,holds)=hold_plan(Some(data.attempt),vec![(RecordId::new("native_attempt",data.attempt.hex()),admission.record()?),(admission.record()?,data.record()?)])?.ok_or(ModelError::Schema("semantic admission durable holds"))?;
    merge_plan_bindings(&mut variables,holds)?;frozen::bind_fence(&data.fence(),&mut variables);
    variables.insert("completion",data.record()?);variables.insert("completion_guard",guard_id(&Value::RecordId(data.record()?))?);variables.insert("completion_incarnation",physical.incarnation());variables.insert("completion_bytes",data.value(true)?);
    variables.insert("completion_identity",data.identity()?.hex());variables.insert("completion_logical",data.completed_state.hex());variables.insert("admission",admission.record()?);
    let sql=format!("{} LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes OR ($__owner.admitted AND $__owner.completed_admission!=$admission) {{ THROW 'semantic admission physical completion changed'; }}; {row_sql}; {hold_sql}; UPDATE $__attempt SET admitted=true,completed_identity=$completion_identity,completed_state=$completion_logical,completed_admission=$admission RETURN NONE",frozen::FENCE_GUARD);
    Ok((sql,variables,charge))
}

pub(crate) async fn hold_frozen_physical_expected(client:&Surreal<Client>,completion:&crate::completed_state::FrozenCompletion,
    expected:ExpectedRecord,budget:&ResourceBudget)->Result<HeldRecord,ModelError>{
    if expected.row()!=&completion.value(true)? || expected.record()!=&completion.record()?{return Err(ModelError::Conflict("physical completed hold exact envelope"));}
    let _charge=budget.reserve("physical completion hold",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let owner=RecordId::new("native_attempt",completion.attempt.hex());
    let (_,holds,mut variables)=hold_plan(Some(completion.attempt),vec![(owner.clone(),completion.record()?)])?.ok_or(ModelError::Schema("physical completed hold plan"))?;
    frozen::bind_fence(&completion.fence(),&mut variables);
    variables.insert("completion",completion.record()?);variables.insert("completion_guard",guard_id(&Value::RecordId(completion.record()?))?);variables.insert("completion_bytes",expected.row.clone());variables.insert("completion_incarnation",expected.incarnation);
    variables.insert("completion_identity",completion.identity()?.hex());
    let sql=format!("{} LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes OR $__owner.completed_identity!=$completion_identity {{ THROW 'physical completed hold changed'; }}; {holds}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(completion.attempt),&sql,variables).await?;Ok(HeldRecord{expected,owner})
}

/// Final acquisition compares the same live attempt epoch and exact completed view bytes.
pub(crate) async fn hold_live_view_expected(client:&Surreal<Client>,owner:&super::live_view::LiveViewOwner,
    expected:ExpectedRecord,budget:&ResourceBudget)->Result<HeldRecord,ModelError>{
    if expected.record().table.as_str()!="compiler_view_completion" {return Err(ModelError::Conflict("live held witness family"));}
    let _charge=budget.reserve("live exact installed view hold",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let holder=RecordId::new("native_attempt",owner.attempt().hex());
    let (_,holds,mut variables)=hold_plan(Some(owner.attempt()),vec![(holder.clone(),expected.record.clone())])?.ok_or(ModelError::Schema("live held view plan"))?;
    super::live_view::bind(owner,&mut variables);variables.insert("expected_record",expected.record.clone());variables.insert("expected_guard",guard_id(&Value::RecordId(expected.record.clone()))?);variables.insert("expected_incarnation",expected.incarnation);variables.insert("expected_bytes",expected.row.clone());
    let sql=format!("{} LET $guard=SELECT * FROM ONLY $expected_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $expected_record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected_incarnation OR $actual!=$expected_bytes {{ THROW 'live held completion changed'; }}; {holds}",super::live_view::GUARD);
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt()),&sql,variables).await?;Ok(HeldRecord{expected,owner:holder})
}

/// One receiving transaction compares every exact source incarnation before adding
/// any incoming edge. A multi-output product never leaves a committed prefix.
pub(crate) async fn hold_live_views_expected(client:&Surreal<Client>,owner:&super::live_view::LiveViewOwner,
    expected:Vec<ExpectedRecord>,contributors:Vec<ExpectedRecord>,budget:&ResourceBudget)->Result<Vec<HeldRecord>,ModelError>{
    if expected.is_empty() || expected.iter().any(|record|record.record().table.as_str()!="compiler_view_completion")
        || contributors.iter().any(|record|record.record().table.as_str()!="compiler_contribution") {
        return Err(ModelError::Conflict("atomic attached witness family/inventory"));
    }
    let _charge=budget.reserve("atomic exact view reference attachment",expected.iter().chain(&contributors).map(|row|crate::loader::native_bytes(row.row()).saturating_mul(32).saturating_add(4096)).fold(131072usize,usize::saturating_add))?;
    let holder=RecordId::new("native_attempt",owner.attempt().hex());
    let edges=expected.iter().chain(&contributors).map(|record|(holder.clone(),record.record().clone())).collect();
    let (_,holds,mut variables)=hold_plan(Some(owner.attempt()),edges)?.ok_or(ModelError::Schema("atomic attached views hold plan"))?;
    super::live_view::bind(owner,&mut variables);
    let checks=expected.iter().chain(&contributors).map(|expected|{
        let mut row=Object::new();row.insert("record",expected.record().clone());row.insert("guard",guard_id(&Value::RecordId(expected.record().clone()))?);row.insert("incarnation",expected.incarnation());row.insert("bytes",expected.row().clone());Ok(Value::Object(row))
    }).collect::<Result<Vec<_>,ModelError>>()?;
    variables.insert("attached_checks",checks);
    let sql=format!("{} FOR $check IN $attached_checks {{ LET $guard=SELECT * FROM ONLY $check.guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $check.record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$check.incarnation OR $actual!=$check.bytes {{ THROW 'atomic installed attachment source changed'; }}; }}; {holds}",super::live_view::GUARD);
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt()),&sql,variables).await?;
    Ok(expected.into_iter().map(|expected|HeldRecord{expected,owner:holder.clone()}).collect())
}
