//! N03's adjacent source5 translation component. The aggregate upgrader owns progress,
//! declaration installation, full format6 validation and the SINGLE publication transaction.
//! This component never publishes a format or grants host drainage authority by itself.
use super::*;
use authorization::{hash_field,int_field};

pub(crate) const SOURCE5_SCHEMA:&str="eec4dc767fe4477fbfc3f5a584b54a6c8855345af02c3390ad5f77ac326e0383";
pub(crate) const COMPONENT:&str="native-write-predicates-v6/source5-exact/64-stripes/local-owner-epochs/incarnation-tombstones/backup-gate/v1";
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum GuardPass { Preflight, Translate, Verify }
pub(crate) struct GuardPage {
    pub after:Option<RecordId>,
    pub finished:bool,
    pub body:String,
    pub bindings:Variables,
    pub observations:Vec<Object>,
}
/// The existing aggregate progress writer must compare observations and advance this
/// cursor with body in ONE transaction, under exact original source5/closed authority.
pub(crate) async fn prepare_guard_page(client:&Surreal<Client>,pass:GuardPass,after:Option<RecordId>)->Result<GuardPage,ModelError>{
    let cursor=after.clone().map(Value::RecordId).unwrap_or(Value::None);
    let source=keyset_source("native_guard",&cursor)?;
    let mut response=client.query(format!("SELECT * FROM {source} LIMIT {}",crate::loader::NATIVE_WINDOW_ROWS)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
    let mut windows=crate::loader::NativeWindows::new(rows);
    let rows=windows.next().transpose()?.unwrap_or_default();
    if windows.next().is_some(){return Err(ModelError::Conflict("native guard migration page width"));}
    let rows=rows.into_iter().map(|row|match row{Value::Object(row)=>Ok(row),_=>Err(ModelError::Schema("native guard migration row"))}).collect::<Result<Vec<_>,_>>()?;
    for row in &rows {validate_guard(row,pass)?;}
    let next=rows.last().map(|row|match row.get("id"){Some(Value::RecordId(id))=>Ok(id.clone()),_=>Err(ModelError::Schema("native guard migration identity"))}).transpose()?;
    let mut bindings=Variables::new();
    let body=if pass==GuardPass::Translate {
        bindings.insert("n03_guards",rows.clone());
        "FOR $n03_observed IN $n03_guards { LET $n03_id=$n03_observed.id; LET $n03_before=SELECT * FROM ONLY $n03_id FOR UPDATE; IF $n03_before!=$n03_observed { THROW 'native guard translation observation changed'; }; UPDATE $n03_id SET tombstone_revision=$n03_before.revision RETURN NONE; UPDATE $n03_id UNSET retired_through RETURN NONE; };".into()
    } else {String::new()};
    coordination::preflight(&body,&bindings)?;
    Ok(GuardPage{after:next.or(after),finished:rows.is_empty(),body,bindings,observations:rows})
}
fn validate_guard(row:&Object,pass:GuardPass)->Result<(),ModelError>{
    if !matches!(row.get("id"),Some(Value::RecordId(id)) if id.table.as_str()=="native_guard") || int_field(row,"revision")?<0 || int_field(row,"incarnation")?<=0 {return Err(ModelError::Conflict("source5 guard identity/revision/incarnation"));}
    let Some(Value::String(phase))=row.get("phase") else{return Err(ModelError::Schema("source5 guard phase"));};
    // Immutable source5 could reopen an already-retired incarnation through a second
    // queued nominee, leaving retiring+retired=true. Its original installer can finish
    // that exact retained claim. Require that completion before conversion; never clear
    // the flag or fabricate a fresh target claim during migration.
    if !matches!(phase.as_str(),"active"|"retiring"|"retired") || row.get("retired")!=Some(&Value::Bool(phase=="retired")) {return Err(ModelError::Conflict("source5 guard phase/retirement requires original installer completion"));}
    if phase=="retiring" && !matches!(row.get("retiring_item"),Some(Value::RecordId(id)) if id.table.as_str()=="native_retirement_item") {return Err(ModelError::Conflict("source5 claimed incarnation owner"));}
    if pass==GuardPass::Verify {
        if row.get("retired_through").is_some_and(|value|*value!=Value::None) || int_field(row,"tombstone_revision")?!=int_field(row,"revision")? {return Err(ModelError::Conflict("format6 translated guard verification"));}
    } else if let Some(value)=row.get("retired_through") {
        if *value!=Value::None && !matches!(value,Value::Number(surrealdb::types::Number::Int(value)) if *value>=0) {return Err(ModelError::Schema("source5 retired watermark"));}
    }
    Ok(())
}
/// An exact retained claim, not an authority to execute or rehome its predecessor.
/// A missing lineage uses the source5 nominal-job fallback; no rich lineage-root row
/// is invented, and rehomed item IDs are never recomputed under their current job.
#[derive(Clone,Debug)]
pub(crate) struct RetiringClaim {
    pub guard:Object,
    pub item:Object,
    pub job:Object,
}
/// Check-only transaction fragment. The aggregate must compose this before translating
/// these guards and again during its final Verify pass, atomically with its own progress
/// checkpoint. It binds exact source/guard/item/job bytes, never just an item pointer.
pub(crate) struct RetiringClaimAudit {
    pub claims:Vec<RetiringClaim>,
    pub body:String,
    pub bindings:Variables,
}
fn claim_source(marker:&Object)->Result<(ContentHash,i64,i64),ModelError>{
    let generation=hash_field(marker,"generation")?;
    if hash_field(marker,"schema")?.hex()!=SOURCE5_SCHEMA || int_field(marker,"schema_version")?!=5
        || marker.get("admission_open")!=Some(&Value::Bool(false)) {
        return Err(ModelError::Conflict("N03 retiring claim exact closed source5"));
    }
    let era=int_field(marker,"era")?;let closed=int_field(marker,"closed_through")?;
    let admissions=int_field(marker,"admission_revision")?;
    if era<=closed || closed<0 || admissions<0 {return Err(ModelError::Conflict("N03 retiring claim source horizon"));}
    Ok((generation,era,admissions))
}
fn optional_claim_record(row:&Object,field:&str)->Result<Option<RecordId>,ModelError>{
    match row.get(field) {
        None|Some(Value::None)=>Ok(None),
        Some(Value::RecordId(id)) if id.table.as_str()=="native_retirement"=>Ok(Some(id.clone())),
        _=>Err(ModelError::Schema("N03 retiring claim job reference")),
    }
}
/// Pure validator shared by prepare and independently replayed aggregate verification.
/// Finalize's exact object/incarnation/item/state/cutoff predicates come from immutable
/// source5 commit ebcf423c. Era, epoch and cutoff values are observed, never translated.
pub(crate) fn verify_retiring_claim(marker:&Object,guard:&Object,item:&Object,job:&Object,pass:GuardPass)->Result<RetiringClaim,ModelError>{
    let (generation,era,admissions)=claim_source(marker)?;
    validate_guard(guard,pass)?;
    let (Some(Value::RecordId(guard_id_actual)),Some(Value::RecordId(item_id)),Some(Value::RecordId(object)),Some(Value::RecordId(job_id)))=
        (guard.get("id"),item.get("id"),item.get("object"),item.get("job")) else {
        return Err(ModelError::Schema("N03 retiring claim exact records"));
    };
    if guard.get("phase")!=Some(&Value::String("retiring".into()))
        || guard.get("retiring_item")!=Some(&Value::RecordId(item_id.clone()))
        || item_id.table.as_str()!="native_retirement_item"
        || guard_id(&Value::RecordId(object.clone()))?!=*guard_id_actual
        || item.get("state")!=Some(&Value::String("retiring".into()))
        || int_field(item,"incarnation")?!=int_field(guard,"incarnation")?
        || item.get("successor").is_some_and(|value|*value!=Value::None)
        || job_id.table.as_str()!="native_retirement"
        || job.get("id")!=Some(&Value::RecordId(job_id.clone()))
        || hash_field(job,"generation")?!=generation {
        return Err(ModelError::Conflict("N03 retiring claim identity/incarnation/state"));
    }
    let cutoff=int_field(item,"cutoff")?;
    let job_era=int_field(job,"era")?;
    if cutoff<0 || cutoff>admissions || job_era<=0 || job_era>era || int_field(job,"epoch")?<=0 || int_field(job,"epoch")?>admissions
        || int_field(job,"root_count")?<0 || int_field(job,"roots_registered")?!=int_field(job,"root_count")? {
        return Err(ModelError::Conflict("N03 retiring claim original cutoff/job scope"));
    }
    let successor=optional_claim_record(job,"successor")?;
    let recovery=optional_claim_record(job,"recovery_source")?;
    optional_claim_record(job,"predecessor")?;
    optional_claim_record(job,"lineage")?;
    // Interrupted recovery can leave the claimed item on a superseded predecessor or
    // an incompletely rehomed successor. The aggregate's job pass authenticates the
    // actual predecessor/successor chain; this validator never refreshes its authority.
    match job.get("state") {
        Some(Value::String(state)) if state=="open" && successor.is_none() && recovery.is_none()=>{},
        Some(Value::String(state)) if state=="recovering" && successor.is_none() && recovery.is_some()=>{},
        Some(Value::String(state)) if state=="superseded" && successor.is_some()=>{},
        _=>return Err(ModelError::Conflict("N03 retiring claim recoverable job state")),
    }
    Ok(RetiringClaim{guard:guard.clone(),item:item.clone(),job:job.clone()})
}
fn claim_row_map(ids:&std::collections::BTreeSet<RecordId>,rows:Vec<Value>)->Result<std::collections::BTreeMap<RecordId,Object>,ModelError>{
    let mut windows=crate::loader::NativeWindows::new(rows);
    let rows=windows.next().transpose()?.unwrap_or_default();
    if windows.next().is_some(){return Err(ModelError::Conflict("N03 retiring claim native row window"));}
    let mut result=std::collections::BTreeMap::new();
    for row in rows {
        let Value::Object(row)=row else{return Err(ModelError::Schema("N03 retiring claim row"));};
        let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("N03 retiring claim row identity"));};
        if !ids.contains(id) || result.insert(id.clone(),row.clone()).is_some(){return Err(ModelError::Conflict("N03 retiring claim foreign/duplicate row"));}
    }
    if result.len()!=ids.len(){return Err(ModelError::Conflict("N03 retiring claim missing retained row"));}
    Ok(result)
}
const RETIRING_CLAIM_RECHECK:&str="LET $n03_claim_installed=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $n03_claim_installed!=$n03_claim_source { THROW 'N03 retiring claim source changed'; }; FOR $n03_claim IN $n03_claims { LET $n03_guard_id=$n03_claim.guard.id; LET $n03_item_id=$n03_claim.item.id; LET $n03_job_id=$n03_claim.job.id; LET $n03_guard_now=SELECT * FROM ONLY $n03_guard_id FOR UPDATE; LET $n03_item_now=SELECT * FROM ONLY $n03_item_id FOR UPDATE; LET $n03_job_now=SELECT * FROM ONLY $n03_job_id FOR UPDATE; IF $n03_guard_now!=$n03_claim.guard OR $n03_item_now!=$n03_claim.item OR $n03_job_now!=$n03_claim.job { THROW 'N03 retiring claim original bytes changed'; }; };";
/// Reads only fixed retained item/job IDs; there are no global scans or writes. The
/// caller supplies the original guard page and its exact closed source marker. All
/// returned check bindings remain charged by native row/complete-transfer admission.
pub(crate) async fn prepare_retiring_claim_audit(client:&Surreal<Client>,marker:&Object,guards:&[Object],pass:GuardPass)->Result<RetiringClaimAudit,ModelError>{
    if guards.len()>crate::loader::NATIVE_WINDOW_ROWS {return Err(ModelError::Conflict("N03 retiring claim guard page width"));}
    let retiring=guards.iter().filter(|guard|guard.get("phase")==Some(&Value::String("retiring".into()))).collect::<Vec<_>>();
    if retiring.is_empty(){return Ok(RetiringClaimAudit{claims:Vec::new(),body:String::new(),bindings:Variables::new()});}
    claim_source(marker)?;
    let ids=retiring.iter().map(|guard|match guard.get("retiring_item") {
        Some(Value::RecordId(id)) if id.table.as_str()=="native_retirement_item"=>Ok(id.clone()),
        _=>Err(ModelError::Schema("N03 retiring claim pointer")),
    }).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
    let mut response=client.query("SELECT * FROM native_installation:current; SELECT * FROM $items")
        .bind(("items",ids.iter().cloned().collect::<Vec<_>>())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let installed:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    if installed.as_slice()!=std::slice::from_ref(marker){return Err(ModelError::Conflict("N03 retiring claim original source changed"));}
    let items=claim_row_map(&ids,response.take::<Vec<Value>>(1).map_err(ModelError::codec)?)?;
    let job_ids=items.values().map(|item|match item.get("job") {
        Some(Value::RecordId(id)) if id.table.as_str()=="native_retirement"=>Ok(id.clone()),
        _=>Err(ModelError::Schema("N03 retiring claim job pointer")),
    }).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
    let mut response=client.query("SELECT * FROM $jobs").bind(("jobs",job_ids.iter().cloned().collect::<Vec<_>>())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let jobs=claim_row_map(&job_ids,response.take::<Vec<Value>>(0).map_err(ModelError::codec)?)?;
    let claims=retiring.into_iter().map(|guard|{
        let Some(Value::RecordId(id))=guard.get("retiring_item")else{unreachable!()};
        let item=items.get(id).ok_or(ModelError::Conflict("N03 retiring claim missing item"))?;
        let Some(Value::RecordId(job_id))=item.get("job")else{unreachable!()};
        let job=jobs.get(job_id).ok_or(ModelError::Conflict("N03 retiring claim missing job"))?;
        verify_retiring_claim(marker,guard,item,job,pass)
    }).collect::<Result<Vec<_>,_>>()?;
    let envelopes=claims.iter().map(|claim|{let mut row=Object::new();row.insert("guard",claim.guard.clone());row.insert("item",claim.item.clone());row.insert("job",claim.job.clone());Value::Object(row)}).collect();
    let mut windows=crate::loader::NativeWindows::new(envelopes);
    let envelopes=windows.next().transpose()?.unwrap_or_default();
    if windows.next().is_some(){return Err(ModelError::Conflict("N03 retiring claim proof window"));}
    let mut bindings=Variables::new();bindings.insert("n03_claim_source",marker.clone());bindings.insert("n03_claims",envelopes);
    let body=RETIRING_CLAIM_RECHECK.to_owned();
    coordination::preflight(&body,&bindings)?;
    Ok(RetiringClaimAudit{claims,body,bindings})
}

/// Build the N03 portion of the FINAL aggregate publication. Caller must insert this
/// body before publishing schema/version/progress in the SAME transaction. It is not
/// valid as a standalone command. It verifies drained native predicates under the old
/// global installation write and installs all64 stripes together with the era cut.
pub(crate) fn activation(marker:&Object,generation:ContentHash)->Result<(String,Variables),ModelError>{
    if hash_field(marker,"generation")?!=generation || hash_field(marker,"schema")?.hex()!=SOURCE5_SCHEMA || int_field(marker,"schema_version")?!=5 || marker.get("admission_open")!=Some(&Value::Bool(false)) {return Err(ModelError::Conflict("N03 exact source5 closed activation"));}
    let era=int_field(marker,"era")?;let closed=int_field(marker,"closed_through")?;
    if era<=closed || closed<0 || era==i64::MAX {return Err(ModelError::Conflict("N03 source5 era cut"));}
    let mut bindings=Variables::new();bindings.insert("n03_original",marker.clone());bindings.insert("n03_generation",generation.hex());
    bindings.insert("n03_stripes",coordination::initial_stripes(generation,era+1,era,false)?);
    bindings.insert("__effect",Value::None);
    let body=format!("LET $n03_installed=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $n03_installed!=$n03_original {{ THROW 'N03 original installation changed'; }}; {} IF array::len(SELECT VALUE id FROM native_cleanup WITH INDEX live_cleanup WHERE state='open' AND array::len(owners)>0 LIMIT 1)>0 {{ THROW 'N03 source5 cleanup scope needs original installer completion'; }}; IF array::len(SELECT VALUE id FROM native_admission_stripe LIMIT 1)>0 OR (SELECT VALUE id FROM ONLY native_backup_gate:current)!=NONE OR array::len(SELECT VALUE id FROM native_guard WHERE retired_through!=NONE LIMIT 1)>0 {{ THROW 'N03 activation residue'; }}; FOR $n03_stripe IN $n03_stripes {{ CREATE $n03_stripe.id CONTENT $n03_stripe RETURN NONE; }}; CREATE native_backup_gate:current SET revision=0,active_count=0 RETURN NONE; REMOVE FIELD retired_through ON native_guard; UPDATE native_installation:current UNSET admission_revision,backup_revision RETURN NONE; REMOVE FIELD admission_revision ON native_installation; REMOVE FIELD backup_revision ON native_installation; UPDATE native_installation:current SET closed_through=$n03_installed.era,era=$n03_installed.era+1,history_inventory=NONE RETURN NONE;",coordination::DRAINED_PREDICATE);
    coordination::preflight(&body,&bindings)?;Ok((body,bindings))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source()->Object{let mut row=Object::new();row.insert("generation",ContentHash::of(b"source5").hex());row.insert("schema",SOURCE5_SCHEMA);row.insert("schema_version",5i64);row.insert("admission_open",false);row.insert("era",9i64);row.insert("closed_through",8i64);row}
    #[test]
    fn activation_is_adjacent_atomic_and_does_not_publish_independently(){
        let row=source();let (sql,bindings)=activation(&row,ContentHash::of(b"source5")).unwrap();
        surrealdb_syn::parse(&format!("BEGIN; {sql} COMMIT;")).unwrap();
        assert!(!sql.contains("schema_version="));assert!(sql.contains("REMOVE FIELD retired_through"));
        let Some(Value::Array(stripes))=bindings.get("n03_stripes")else{panic!("stripes")};assert_eq!(stripes.len(),64);
        for value in stripes{let Value::Object(stripe)=value else{panic!("stripe")};assert_eq!(int_field(stripe,"era").unwrap(),10);assert_eq!(int_field(stripe,"closed_through").unwrap(),9);}
        let mut wrong=row.clone();wrong.insert("schema_version",4i64);assert!(activation(&wrong,ContentHash::of(b"source5")).is_err());
        wrong=row;wrong.insert("admission_open",true);assert!(activation(&wrong,ContentHash::of(b"source5")).is_err());
    }
    #[test]
    fn exact_guard_translation_preserves_incarnation_and_local_owner_values(){
        let mut guard=Object::new();guard.insert("id",RecordId::new("native_guard","exact"));guard.insert("phase","retired");guard.insert("retired",true);guard.insert("incarnation",7i64);guard.insert("revision",29i64);guard.insert("retired_through",9000i64);
        assert!(validate_guard(&guard,GuardPass::Preflight).is_ok());assert!(validate_guard(&guard,GuardPass::Verify).is_err());
        guard.remove("retired_through");guard.insert("tombstone_revision",29i64);assert!(validate_guard(&guard,GuardPass::Verify).is_ok());
        assert_eq!(int_field(&guard,"incarnation").unwrap(),7);
        guard.insert("tombstone_revision",28i64);assert!(validate_guard(&guard,GuardPass::Verify).is_err());
    }
}

/// Run before the aggregate converter's first declaration/data effect. This is a
/// native predicate check, not proof that remote/host owners have actually drained.
/// The aggregate publication repeats these predicates under the installation write.
pub(crate) async fn source_preflight(client:&Surreal<Client>,generation:ContentHash)->Result<Object,ModelError>{
    let mut response=checked_transaction(client.query("BEGIN; SELECT * FROM native_installation:current; SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false LIMIT 1; SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','frozen','closing'] LIMIT 1; SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1; SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1; SELECT VALUE id FROM native_retirement WITH INDEX live_retirements WHERE state='registering' LIMIT 1; SELECT VALUE id FROM native_cleanup WITH INDEX live_cleanup WHERE state='open' AND array::len(owners)>0 LIMIT 1; COMMIT;").await.map_err(ModelError::codec)?)?;
    let rows:Vec<Object>=response.take(1).map_err(ModelError::codec)?;let [marker]=rows.as_slice() else{return Err(ModelError::Conflict("N03 source installation cardinality"));};
    // activation() is pure and independently checks the exact source marker/era.
    let _=activation(marker,generation)?;
    for index in 2..=7 {let rows:Vec<RecordId>=response.take(index).map_err(ModelError::codec)?;if !rows.is_empty(){return Err(ModelError::Conflict("N03 source5 owners need original installer drainage"));}}
    Ok(marker.clone())
}

#[cfg(test)]
mod retiring_claim_controls {
    use super::*;
    fn source_claim()->(Object,Object,Object,Object){
        let generation=ContentHash::of(b"source5-claim");
        let mut source=Object::new();source.insert("schema",SOURCE5_SCHEMA);source.insert("schema_version",5i64);source.insert("generation",generation.hex());source.insert("admission_open",false);source.insert("era",9i64);source.insert("closed_through",8i64);source.insert("admission_revision",91i64);
        let object=RecordId::new("original","retained-source5-object");let id=RecordId::new("native_retirement_item","opaque-original-item");let job_id=RecordId::new("native_retirement","source5-owner");
        let mut guard=Object::new();guard.insert("id",guard_id(&Value::RecordId(object.clone())).unwrap());guard.insert("phase","retiring");guard.insert("retired",false);guard.insert("incarnation",7i64);guard.insert("revision",29i64);guard.insert("retiring_item",id.clone());guard.insert("retired_through",23i64);
        let mut item=Object::new();item.insert("id",id);item.insert("job",job_id.clone());item.insert("object",object);item.insert("state","retiring");item.insert("incarnation",7i64);item.insert("cutoff",72i64);
        let mut job=Object::new();job.insert("id",job_id);job.insert("generation",generation.hex());job.insert("era",9i64);job.insert("epoch",90i64);job.insert("state","open");job.insert("root_count",1i64);job.insert("roots_registered",1i64);
        (source,guard,item,job)
    }
    #[test]
    fn original_retiring_claim_preserves_cutoff_epoch_and_nominal_lineage(){
        let (source,mut guard,item,mut job)=source_claim();
        let original=verify_retiring_claim(&source,&guard,&item,&job,GuardPass::Preflight).unwrap();
        assert_eq!(original.guard,guard);assert_eq!(original.item,item);assert_eq!(original.job,job);
        // Recovery's newer owner epoch does not rewrite the original cutoff. Source5
        // finalize does not dereference a nominal lineage root or rehash the item ID.
        assert!(int_field(&item,"cutoff").unwrap()<int_field(&job,"epoch").unwrap());
        job.insert("lineage",RecordId::new("native_retirement","nominal-root-not-dereferenced"));
        verify_retiring_claim(&source,&guard,&item,&job,GuardPass::Preflight).unwrap();
        job.insert("state","recovering");job.insert("recovery_source",RecordId::new("native_retirement","retained-predecessor"));
        verify_retiring_claim(&source,&guard,&item,&job,GuardPass::Preflight).unwrap();
        job.insert("state","superseded");job.insert("successor",RecordId::new("native_retirement","retained-successor"));
        verify_retiring_claim(&source,&guard,&item,&job,GuardPass::Preflight).unwrap();
        guard.remove("retired_through");guard.insert("tombstone_revision",29i64);
        let translated=verify_retiring_claim(&source,&guard,&item,&job,GuardPass::Verify).unwrap();
        assert_eq!(translated.item,item);assert_eq!(translated.job,job);
    }
    #[test]
    fn claimed_guard_refuses_foreign_missing_or_changed_item_and_job(){
        let (source,guard,item,job)=source_claim();
        for (field,value) in [("id",Value::RecordId(RecordId::new("native_guard","unrelated"))),("phase",Value::String("active".into())),("incarnation",crate::upgrade::native(8i64)),("retiring_item",Value::RecordId(RecordId::new("native_retirement_item","other")))] {
            let mut changed=guard.clone();changed.insert(field,value);assert!(verify_retiring_claim(&source,&changed,&item,&job,GuardPass::Preflight).is_err(),"changed guard {field}");
        }
        for (field,value) in [("id",Value::RecordId(RecordId::new("native_retirement_item","other"))),("job",Value::RecordId(RecordId::new("native_retirement","other"))),("object",Value::RecordId(RecordId::new("original","recreated"))),("incarnation",crate::upgrade::native(8i64)),("state",Value::String("pending".into())),("cutoff",Value::None),("cutoff",crate::upgrade::native(-1i64)),("cutoff",crate::upgrade::native(92i64)),("successor",Value::RecordId(RecordId::new("native_retirement_item","new")))] {
            let mut changed=item.clone();changed.insert(field,value);assert!(verify_retiring_claim(&source,&guard,&changed,&job,GuardPass::Preflight).is_err(),"changed item {field}");
        }
        for (field,value) in [("id",Value::RecordId(RecordId::new("native_retirement","other"))),("generation",Value::String(ContentHash::of(b"foreign").hex())),("era",crate::upgrade::native(0i64)),("era",crate::upgrade::native(10i64)),("epoch",crate::upgrade::native(0i64)),("epoch",crate::upgrade::native(92i64)),("state",Value::String("done".into())),("state",Value::String("registering".into())),("state",Value::String("recovering".into())),("state",Value::String("superseded".into())),("roots_registered",crate::upgrade::native(0i64)),("lineage",Value::RecordId(RecordId::new("original","wrong-family")))] {
            let mut changed=job.clone();changed.insert(field,value);assert!(verify_retiring_claim(&source,&guard,&item,&changed,GuardPass::Preflight).is_err(),"changed job {field}");
        }
        let mut old_reopened=guard.clone();old_reopened.insert("retired",true);
        assert!(verify_retiring_claim(&source,&old_reopened,&item,&job,GuardPass::Preflight).is_err(),
            "source5 reopened retiring+retired=true requires its original installer to finish; migration never normalizes it");
        let mut changed=source.clone();changed.insert("admission_open",true);assert!(verify_retiring_claim(&changed,&guard,&item,&job,GuardPass::Preflight).is_err());
        changed=source;changed.insert("schema",ContentHash::of(b"unreviewed-source").hex());assert!(verify_retiring_claim(&changed,&guard,&item,&job,GuardPass::Preflight).is_err());
    }
    #[test]
    fn exact_claim_fetch_refuses_missing_duplicate_and_foreign_rows(){
        let (_,guard,item,job)=source_claim();
        let Some(Value::RecordId(id))=item.get("id")else{panic!("item")};let ids=[id.clone()].into_iter().collect();
        claim_row_map(&ids,vec![Value::Object(item.clone())]).unwrap();
        assert!(claim_row_map(&ids,vec![]).is_err());
        assert!(claim_row_map(&ids,vec![Value::Object(item.clone()),Value::Object(item)]).is_err());
        assert!(claim_row_map(&ids,vec![Value::Object(job)]).is_err());
        assert!(claim_row_map(&ids,vec![Value::Object(guard)]).is_err());
        surrealdb_syn::parse(&format!("BEGIN; {RETIRING_CLAIM_RECHECK} COMMIT;")).unwrap();
    }
}
