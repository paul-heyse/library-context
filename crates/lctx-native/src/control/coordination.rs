//! Format6 write admission and exact physical expectations. No cross-owner epoch ordering.
use super::*;
use authorization::int_field;
use lctx_model::domain::resources::TRANSFER_BYTES;

pub const ADMISSION_STRIPES: usize = 64;
pub(super) const STRIPE_FORMAT: i64 = 1;
pub(super) fn stripe_id(operation: ContentHash) -> RecordId {
    let hash = ContentHash::of(&operation.0);
    RecordId::new("native_admission_stripe", i64::from(hash.0[0] & 63))
}
pub(super) fn stripe_ids() -> Vec<RecordId> {
    (0..ADMISSION_STRIPES).map(|index| RecordId::new("native_admission_stripe", index as i64)).collect()
}
pub(super) fn operation_owner(request: NativeRequest) -> RecordId {
    let mut sink = KeySink::new("native-installation-operation-owner/v6");
    request.issuance.generation.encode(&mut sink); request.operation.encode(&mut sink);
    RecordId::new("native_operation_owner", sink.finish().hex())
}
pub(super) fn bind_stripe(bindings: &mut Variables, operation: ContentHash) {
    bindings.insert("__stripe", stripe_id(operation));
}
pub(super) fn bind_all_stripes(bindings: &mut Variables) {
    bindings.insert("__stripes", stripe_ids());
}
// The marker read carries configuration only. The selected stripe WRITE is the conflict
// partner for close/open/cut, including an uncommitted intent that arrived before the cut.
pub(super) const ERA_GUARD: &str = "LET $__installation=SELECT * FROM ONLY native_installation:current; LET $__admission=SELECT * FROM ONLY $__stripe FOR UPDATE; IF $__installation=NONE OR $__installation.schema_version!=6 OR $__installation.generation!=$__generation OR $__admission=NONE OR $__admission.format!=1 OR $__admission.generation!=$__generation OR $__installation.era!=$__admission.era OR $__installation.closed_through!=$__admission.closed_through OR $__installation.admission_open!=$__admission.admission_open OR $__admission.era!=$__era OR $__era<=$__admission.closed_through OR $__admission.revision>=9223372036854775807 { THROW 'native issuance stripe fenced'; }; UPDATE $__stripe SET revision+=1 RETURN NONE;";
pub(super) const ALL_STRIPES_GUARD: &str = "IF array::len($__stripes)!=64 { THROW 'native stripe set width'; }; FOR $__stripe_id IN $__stripes { LET $__state=SELECT * FROM ONLY $__stripe_id FOR UPDATE; IF $__state=NONE OR $__state.format!=1 OR $__state.generation!=$__generation OR $__state.era!=$__era OR $__state.closed_through!=$__installation.closed_through OR $__state.admission_open!=$__installation.admission_open OR $__state.revision>=9223372036854775807 { THROW 'native all-stripe authority'; }; UPDATE $__stripe_id SET revision+=1 RETURN NONE; };";
pub(super) const DRAINED_PREDICATE: &str = "IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','frozen','closing'] LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_retirement WITH INDEX live_retirements WHERE state='registering' LIMIT 1)>0 { THROW 'native installation borrowers remain'; };";

/// Whole generated transaction plus bindings, before its first remote effect. Atomic
/// control cuts never split. The conservative representation admission also covers SDK
/// serialization and request digest coexistence, as immutable windows already require.
pub(crate) fn preflight(sql: &str, bindings: &Variables) -> Result<(), ModelError> {
    let bytes = immutable_window::plan_bytes(sql, bindings)?;
    if bytes > TRANSFER_BYTES { return Err(ModelError::Limit { owner: "native transaction", limit: "complete control and payload bytes", observed: bytes, bound: TRANSFER_BYTES }); }
    Ok(())
}
pub(crate) fn initial_stripes(generation: ContentHash, era: i64, closed: i64, open: bool) -> Result<Vec<Value>, ModelError> {
    if era <= closed || closed < 0 || ADMISSION_STRIPES + 2 > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("native admission stripe initialization"));
    }
    let mut rows = Vec::with_capacity(ADMISSION_STRIPES);
    for id in stripe_ids() {
        let mut row = Object::new(); row.insert("id", id); row.insert("format", STRIPE_FORMAT);
        row.insert("generation", generation.hex()); row.insert("era", era);
        row.insert("closed_through", closed); row.insert("admission_open", open); row.insert("revision", 0i64);
        let value = Value::Object(row);
        if serde_json::to_vec(&value).map_err(ModelError::codec)?.len() > 1024 || crate::loader::native_bytes(&value) > 1024 {
            return Err(ModelError::Schema("native admission stripe row width"));
        }
        rows.push(value);
    }
    Ok(rows)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedObject {
    pub record: RecordId,
    pub incarnation: i64,
    pub phase: String,
    pub tombstone_revision_if_vacant: Option<i64>,
}
impl ExpectedObject {
    fn from_guard(record: RecordId, row: Option<&Object>) -> Result<Self, ModelError> {
        let Some(row) = row else { return Ok(Self { record, incarnation: 0, phase: "vacant".into(), tombstone_revision_if_vacant: Some(0) }); };
        let incarnation = int_field(row, "incarnation")?;
        let Some(Value::String(phase)) = row.get("phase") else { return Err(ModelError::Schema("native expected phase")); };
        if incarnation <= 0 || !matches!(phase.as_str(), "active" | "retired") || row.get("retired")!=Some(&Value::Bool(phase=="retired")) || int_field(row,"revision")?<0 || int_field(row,"tombstone_revision")?<0 { return Err(ModelError::Conflict("native expected object not writable")); }
        let tombstone = if phase == "retired" { Some(int_field(row, "tombstone_revision")?) } else { None };
        if tombstone.is_some_and(|revision| revision < 0) { return Err(ModelError::Schema("native expected tombstone")); }
        Ok(Self { record, incarnation, phase: phase.clone(), tombstone_revision_if_vacant: tombstone })
    }
    fn binding(&self, guard: RecordId, creation: Option<&Value>) -> Value {
        let mut row=Object::new(); row.insert("record",self.record.clone());row.insert("guard",guard);
        row.insert("incarnation",self.incarnation);row.insert("phase",self.phase.clone());
        row.insert("tombstone_revision",self.tombstone_revision_if_vacant.map(surrealdb::types::SurrealValue::into_value).unwrap_or(Value::None));
        row.insert("creation",creation.cloned().unwrap_or(Value::None));
        Value::Object(row)
    }
}
/// Internal plans declare physical guard IDs. Capture once before request hashing;
/// neither transaction retries nor execute_request refresh these observations.
pub(super) async fn prepare_expectations(client: &Surreal<Client>, bindings: &mut Variables) -> Result<(), ModelError> {
    if bindings.get("__object_expectations").is_some() { return Ok(()); }
    let mut guards = std::collections::BTreeSet::new();
    for key in ["guards", "hold_guards", "__write_guards"] {
        if let Some(Value::Array(ids)) = bindings.get(key) {
            for value in ids { match value { Value::RecordId(id) if id.table.as_str()=="native_guard"=>{guards.insert(id.clone());}, _=>return Err(ModelError::Schema("native expected physical guard")) } }
        }
    }
    if guards.is_empty() { bindings.insert("__object_expectations",Vec::<Value>::new()); return Ok(()); }
    // Guard IDs are exact physical identities; existing callers already own their record
    // mapping. Do not reconstruct IDs from debug/string representations.
    let mut actual=std::collections::BTreeMap::new();
    let ids=guards.iter().cloned().collect::<Vec<_>>();
    for window in ids.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        let mut response=client.query("SELECT * FROM $guards").bind(("guards",window.to_vec())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
        let mut admitted=crate::loader::NativeWindows::new(rows);
        let rows=admitted.next().transpose()?.unwrap_or_default();
        if admitted.next().is_some(){return Err(ModelError::Schema("native expected guard window width"));}
        for value in rows {let Value::Object(row)=value else{return Err(ModelError::Schema("native expected guard row"));};let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native expected guard identity"));};if !guards.contains(id) || actual.insert(id.clone(),row.clone()).is_some(){return Err(ModelError::Conflict("native expected guard cardinality"));}}
    }
    let Some(Value::Array(records))=bindings.get("__guard_records") else{return Err(ModelError::Schema("native expected record map missing"));};
    let mut mapping=std::collections::BTreeMap::new();
    for value in records {let Value::Object(row)=value else{return Err(ModelError::Schema("native expected record map"));};let (Some(Value::RecordId(guard)),Some(Value::RecordId(record)))=(row.get("guard"),row.get("record")) else{return Err(ModelError::Schema("native expected record identity"));};if guard_id(&Value::RecordId(record.clone()))?!=*guard{return Err(ModelError::Conflict("native expected guard correspondence"));}if let Some(prior)=mapping.insert(guard.clone(),record.clone()){if prior!=*record{return Err(ModelError::Conflict("native expected record collision"));}}}
    let creations=creation_indices(bindings)?;
    let expected=guards.into_iter().map(|guard|{let record=mapping.get(&guard).ok_or(ModelError::Schema("native expected record absent"))?;ExpectedObject::from_guard(record.clone(),actual.get(&guard)).map(|expected|expected.binding(guard,creations.get(record)))}).collect::<Result<Vec<_>,_>>()?;
    bindings.insert("__object_expectations",expected);Ok(())
}
// Only immutable plans declare these alternatives. Equal repeated declarations share
// one index; differing full values at an address never gain a creation alternative.
fn creation_indices(bindings:&Variables)->Result<std::collections::BTreeMap<RecordId,Value>,ModelError>{
    let mut indices=std::collections::BTreeMap::new();
    let Some(values)=bindings.get("__immutable_creations") else{return Ok(indices)};
    let Value::Array(values)=values else{return Err(ModelError::Schema("native immutable creations"))};
    for value in values.iter(){
        let Value::Object(creation)=value else{return Err(ModelError::Schema("native immutable creation"))};
        let (Some(Value::String(kind)),Some(Value::RecordId(record)))=(creation.get("kind"),creation.get("record")) else{return Err(ModelError::Schema("native immutable creation envelope"))};
        if kind!="immutable" && !(kind=="original_blob" && record.table.as_str()=="original_blob"){return Err(ModelError::Schema("native immutable creation kind"))}
        let source=if kind=="immutable" {"rows"}else{"original_blobs"};
        let Some(Value::Array(rows))=bindings.get(source) else{return Err(ModelError::Schema("native immutable creation payload"))};
        let mut selected:Option<(usize,&Value)>=None;
        for (index,row) in rows.iter().enumerate().filter(|(_,row)|row.as_object().and_then(|row|row.get("id"))==Some(&Value::RecordId(record.clone()))) {
            if let Some((_,previous))=selected {if previous!=row{return Err(ModelError::Conflict("native immutable creation collision"))}}else{selected=Some((index,row));}
        }
        let (index,_)=selected.ok_or(ModelError::Schema("native immutable creation row missing"))?;
        let mut reference=Object::new();reference.insert("kind",kind.clone());reference.insert("index",index as i64);
        let reference=Value::Object(reference);
        if let Some(previous)=indices.insert(record.clone(),reference.clone()){if previous!=reference{return Err(ModelError::Conflict("native immutable creation collision"))}}
    }Ok(indices)
}
pub(super) const EXPECTED_OBJECT_GUARD: &str = r#"
FOR $__expected IN ($__object_expectations ?? []) {
    LET $__guard_id=$__expected.guard;
    LET $__before=SELECT * FROM ONLY $__guard_id FOR UPDATE;
    IF $__before!=NONE AND $__before.revision>=9223372036854775807 { THROW 'native guard revision exhausted'; };
    IF $__expected.phase='active' {
        IF $__before=NONE OR $__before.phase!='active' OR $__before.incarnation!=$__expected.incarnation { THROW 'native expected active incarnation changed'; };
        UPDATE $__guard_id SET revision+=1 RETURN NONE;
    } ELSE {
        IF $__expected.phase='vacant' {
            IF $__expected.incarnation!=0 OR $__expected.tombstone_revision!=0 { THROW 'native expected vacant identity invalid'; };
        } ELSE IF $__expected.phase='retired' {
            IF $__expected.incarnation<=0 OR $__expected.tombstone_revision<0 { THROW 'native expected tombstone identity invalid'; };
        } ELSE { THROW 'native expected phase invalid'; };
        IF $__expected.incarnation>=9223372036854775807 { THROW 'native incarnation exhausted'; };
        IF $__expected.creation!=NONE AND $__before.phase='active' AND $__before.incarnation=$__expected.incarnation+1 AND $__before.tombstone_revision=$__expected.tombstone_revision AND $__before.retired=false {
            LET $__creation=$__expected.creation;
            LET $__record=$__expected.record;
            LET $__actual=SELECT * FROM ONLY $__record FOR UPDATE;
            LET $__wanted=IF $__creation.kind='immutable' { $rows[$__creation.index] } ELSE { $original_blobs[$__creation.index] };
            IF $__wanted.id!=$__record OR $__actual=NONE { THROW 'native immutable creation identity changed'; };
            IF $__creation.kind='immutable' {
                IF $__actual!=$__wanted { THROW 'native immutable address collision'; };
            } ELSE IF $__creation.kind='original_blob' AND record::table($__record)='original_blob' {
                IF $__actual.id!=$__wanted.id OR $__actual.content!=$__wanted.content OR $__actual.byte_len!=$__wanted.byte_len OR $__actual.format!=$__wanted.format OR type::is_bool($__actual.completed)=false OR array::len(object::keys($__actual))!=5 { THROW 'original blob declaration collision'; };
            } ELSE { THROW 'native immutable creation kind invalid'; };
            UPDATE $__guard_id SET revision+=1 RETURN NONE;
        } ELSE {
            IF $__expected.phase='vacant' {
                IF $__before!=NONE { THROW 'native expected vacant guard changed'; };
            } ELSE IF $__before=NONE OR $__before.phase!='retired' OR $__before.incarnation!=$__expected.incarnation OR $__before.tombstone_revision!=$__expected.tombstone_revision { THROW 'native expected tombstone changed'; };
            UPSERT $__guard_id SET phase='active',incarnation=$__expected.incarnation+1,tombstone_revision=($__before.tombstone_revision ?? 0),revision=($__before.revision ?? 0)+1,retired=false,retiring_item=NONE RETURN NONE;
        };
    };
};"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_stripe_set_is_fixed_bounded_and_operation_stable() {
        let rows=initial_stripes(ContentHash([255;32]),i64::MAX,i64::MAX-1,false).unwrap();
        assert_eq!(rows.len(),64);assert_eq!(stripe_ids().into_iter().collect::<std::collections::BTreeSet<_>>().len(),64);
        for byte in 0..=255 { let operation=ContentHash([byte;32]);assert_eq!(stripe_id(operation),stripe_id(operation));assert!(stripe_ids().contains(&stripe_id(operation))); }
        assert!(!ERA_GUARD.contains("UPDATE native_installation"));
        let mut bindings=Variables::new();bindings.insert("rows",rows);preflight(ALL_STRIPES_GUARD,&bindings).unwrap();
    }
    #[test]
    fn active_expectations_ignore_hold_revision_but_tombstones_bind_revision() {
        let id=RecordId::new("original","candidate");let mut guard=Object::new();guard.insert("phase","active");guard.insert("incarnation",4i64);guard.insert("revision",8i64);guard.insert("retired",false);guard.insert("tombstone_revision",0i64);
        let active=ExpectedObject::from_guard(id.clone(),Some(&guard)).unwrap();guard.insert("revision",9i64);assert_eq!(active,ExpectedObject::from_guard(id.clone(),Some(&guard)).unwrap());
        guard.insert("phase","retired");guard.insert("retired",true);guard.insert("tombstone_revision",2i64);let retired=ExpectedObject::from_guard(id.clone(),Some(&guard)).unwrap();guard.insert("tombstone_revision",3i64);assert_ne!(retired,ExpectedObject::from_guard(id,Some(&guard)).unwrap());
    }
    #[test]
    fn full_stripe_validation_refuses_missing_duplicate_and_divergent_control_rows() {
        let generation=ContentHash::of(b"stripe-generation");
        let mut marker=Object::new();marker.insert("generation",generation.hex());marker.insert("era",3i64);marker.insert("closed_through",2i64);marker.insert("admission_open",false);
        let rows=initial_stripes(generation,3,2,false).unwrap().into_iter().map(|row|match row{Value::Object(row)=>row,_=>unreachable!()}).collect::<Vec<_>>();
        let mut gate=Object::new();gate.insert("revision",0i64);gate.insert("active_count",0i64);
        validate_control_records(&marker,&rows,std::slice::from_ref(&gate)).unwrap();
        assert!(validate_control_records(&marker,&rows[..63],std::slice::from_ref(&gate)).is_err());
        let mut duplicate=rows.clone();duplicate[63]=duplicate[0].clone();assert!(validate_control_records(&marker,&duplicate,std::slice::from_ref(&gate)).is_err());
        for field in ["generation","era","closed_through","admission_open"] {
            let mut divergent=rows.clone();divergent[17].insert(field,Value::None);assert!(validate_control_records(&marker,&divergent,std::slice::from_ref(&gate)).is_err());
        }
        gate.insert("active_count",-1i64);assert!(validate_control_records(&marker,&rows,&[gate]).is_err());
    }
    #[test]
    fn target_control_declarations_and_fixed_transaction_guards_parse() {
        surrealdb_syn::parse(schema()).unwrap();
        for sql in [ERA_GUARD,ALL_STRIPES_GUARD,DRAINED_PREDICATE,EXPECTED_OBJECT_GUARD] {
            surrealdb_syn::parse(&format!("BEGIN; {sql} COMMIT;")).unwrap();
        }
    }
    #[test]
    fn oversized_complete_transaction_refuses_before_effect() {
        let mut bindings=Variables::new();bindings.insert("payload","x".repeat(TRANSFER_BYTES));assert!(preflight("BEGIN;COMMIT;",&bindings).is_err());
    }
}

pub(crate) async fn set_admission(client:&Surreal<Client>,open:bool)->Result<(),ModelError>{
    let mut bindings=Variables::new();bindings.insert("open",open);bind_all_stripes(&mut bindings);
    let sql=format!("{ALL_STRIPES_GUARD} IF $open {{ IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 {{ THROW 'native admission unresolved effects'; }}; }}; FOR $stripe IN $__stripes {{ UPDATE $stripe SET admission_open=$open RETURN NONE; }}; UPDATE native_installation:current SET admission_open=$open,control_revision+=1 RETURN NONE");
    effect(client,None,&sql,bindings).await
}

pub(super) fn validate_control_records(marker:&Object,stripes:&[Object],gates:&[Object])->Result<(),ModelError>{
    let expected=stripe_ids().into_iter().collect::<std::collections::BTreeSet<_>>();let mut seen=std::collections::BTreeSet::new();
    for stripe in stripes {
        let Some(Value::RecordId(id))=stripe.get("id") else{return Err(ModelError::Schema("native stripe identity"));};
        if !expected.contains(id) || !seen.insert(id.clone()) || int_field(stripe,"format")?!=STRIPE_FORMAT || int_field(stripe,"revision")?<0 || ["generation","era","closed_through","admission_open"].into_iter().any(|field|stripe.get(field)!=marker.get(field)) || serde_json::to_vec(stripe).map_err(ModelError::codec)?.len()>1024 {return Err(ModelError::Conflict("native stripe marker correspondence"));}
    }
    if seen!=expected {return Err(ModelError::Conflict("native stripe set incomplete"));}
    let [gate]=gates else{return Err(ModelError::Conflict("native backup gate cardinality"));};
    if int_field(gate,"revision")?<0 || int_field(gate,"active_count")?<0 {return Err(ModelError::Schema("native backup gate counters"));}Ok(())
}
