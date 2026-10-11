//! Explicit drained horizons and named outcome retention. No age-based deletion authority.
use super::*;
use authorization::{hash_field,int_field};

// A raw client cannot carry checked schema authority. Refuse older SCHEMAFULL declarations
// before any collector effect: dropping an unknown last_page field would lose its receipt.
const RUNTIME_SCHEMA_GUARD:&str="IF $__installation.schema_version!=$runtime_schema_version OR $__installation.schema!=$runtime_schema { THROW 'native history receipt schema requires explicit upgrade'; };";
fn bind_runtime_schema(bindings:&mut Variables){
    bindings.insert("runtime_schema_version",SCHEMA_VERSION);
    bindings.insert("runtime_schema",crate::compiler::base_schema_identity().hex());
}
const HISTORY_INSTALLATION_QUERY:&str="SELECT generation,closed_through,history_inventory,schema,schema_version FROM native_installation:current";
fn history_horizon(row:&Object,issuance:IssuanceEra)->Result<i64,ModelError>{
    // effect() first publishes its own durable intent. Refuse an older declaration here,
    // before entering that protocol; the body guard still protects checkpoint publication.
    // Exclusive host maintenance prevents a schema transition between this read and intent.
    if int_field(row,"schema_version")?!=SCHEMA_VERSION || hash_field(row,"schema")?!=crate::compiler::base_schema_identity() {return Err(ModelError::Conflict("native history receipt schema requires explicit upgrade"));}
    if hash_field(row,"generation")?!=issuance.generation || !matches!(row.get("history_inventory"),Some(Value::String(value)) if !value.is_empty()) {return Err(ModelError::Conflict("native history consumer inventory unqualified"));}
    int_field(row,"closed_through")
}

pub async fn retain_outcome(client:&Surreal<Client>,request:NativeRequest,owner:&str,kind:&str)->Result<(),ModelError>{
    if owner.is_empty() || !matches!(kind,"caller"|"recovery"|"evidence"|"migration") {return Err(ModelError::Invalid("native outcome retention consumer".into()));}
    let mut bindings=Variables::new();bindings.insert("target_effect",RecordId::new("native_effect",request.operation.hex()));bindings.insert("target_request",request.request.hex());bindings.insert("target_generation",request.issuance.generation.hex());bindings.insert("target_era",request.issuance.era);bindings.insert("owner",owner.to_owned());bindings.insert("kind",kind.to_owned());
    bindings.insert("reference",RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(request.operation,owner,kind)).map_err(ModelError::codec)?).hex()));
    effect(client,None,"LET $outcome=SELECT * FROM ONLY $target_effect FOR UPDATE; IF $outcome=NONE OR $outcome.request!=$target_request OR $outcome.generation!=$target_generation OR $outcome.era!=$target_era { THROW 'native retained outcome unavailable'; }; UPSERT $reference SET effect=$target_effect,owner=$owner,kind=$kind RETURN NONE",bindings).await
}
pub async fn release_outcome(client:&Surreal<Client>,request:NativeRequest,owner:&str,kind:&str)->Result<(),ModelError>{
    let mut bindings=Variables::new();bindings.insert("reference",RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(request.operation,owner,kind)).map_err(ModelError::codec)?).hex()));
    bindings.insert("effect",RecordId::new("native_effect",request.operation.hex()));bindings.insert("owner",owner.to_owned());bindings.insert("kind",kind.to_owned());
    effect(client,None,"LET $reference_state=SELECT * FROM ONLY $reference FOR UPDATE; IF $reference_state!=NONE { IF $reference_state.effect!=$effect OR $reference_state.owner!=$owner OR $reference_state.kind!=$kind { THROW 'native outcome reference collision'; }; DELETE $reference RETURN NONE; };",bindings).await
}
/// Record the explicit consumer inventory that permits maintenance collection. The caller
/// owns qualification evidence; installation authority alone never implies qualification.
pub async fn qualify_history_inventory(client:&Surreal<Client>,evidence:ContentHash)->Result<(),ModelError>{
    let mut bindings=Variables::new();bindings.insert("evidence",evidence.hex());
    effect(client,None,"IF $__installation.admission_open { THROW 'history inventory requires drained maintenance'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','closing'] LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 { THROW 'history inventory borrowers remain'; }; UPDATE native_installation:current SET history_inventory=$evidence RETURN NONE",bindings).await
}
pub async fn cut_era(client:&Surreal<Client>)->Result<IssuanceEra,ModelError>{
    let issuance=IssuanceEra::capture(client).await?;
    if issuance.era==i64::MAX{return Err(ModelError::Invalid("native issuance era exhausted".into()));}
    let mut bindings=Variables::new();bindings.insert("original_era",issuance.era);bindings.insert("original_generation",issuance.generation.hex());
    effect(client,None,"IF $__era!=$original_era OR $__generation!=$original_generation { THROW 'era closure issuance changed'; }; IF $__installation.admission_open { THROW 'era closure requires drained maintenance'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','closing'] LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 { THROW 'era closure borrowers remain'; }; IF array::len(SELECT VALUE id FROM native_retirement WITH INDEX live_retirements WHERE state='registering' LIMIT 1)>0 { THROW 'era closure has incomplete retirement registration'; }; UPDATE native_installation:current SET closed_through=$__era,era=$__era+1,control_revision+=1 RETURN NONE",bindings).await?;
    Ok(IssuanceEra{generation:issuance.generation,era:issuance.era+1})
}
#[derive(Clone,Debug,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub struct HistoryCompaction { pub identity:ContentHash,pub closed_through:i64,pub removed_effects:u64,pub removed_records:u64,pub state:String,pub revision:u64,pub examined:Vec<RecordId>,pub removed:Vec<RecordId>,pub protected:Vec<RecordId> }
/// Examine at most `limit` exact records behind the permanent era fence. Rich control
/// records are removed only after terminal state, exact references and actual provenance
/// consumers permit it. Slim incarnation guards and the era watermark remain permanent.
pub async fn compact_history(client:&Surreal<Client>,limit:usize)->Result<HistoryCompaction,ModelError>{
    if limit==0{return Err(ModelError::Invalid("native history page limit".into()));}
    let issuance=IssuanceEra::capture(client).await?;
    let identity=fresh_identity("history-collection")?;
    let mut response=client.query(HISTORY_INSTALLATION_QUERY).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Schema("native history installation"));};
    let closed=history_horizon(row,issuance)?;
    let mut bindings=Variables::new();bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",identity.hex()));bindings.insert("closed",closed);bindings.insert("issuance",issuance.era);
    bind_runtime_schema(&mut bindings);
    let creation=format!("{RUNTIME_SCHEMA_GUARD} IF $__era!=$issuance OR $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through!=$closed {{ THROW 'native history horizon changed'; }}; CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=$__epoch,closed_through=$closed,revision=0,removed_effects=0,state='open' RETURN NONE");
    effect(client,None,&creation,bindings).await.map_err(|error|authorization::lifecycle_error("history invocation",identity,issuance,error))?;
    resume_history_compaction(client,identity,0,limit).await
}
// Ordering releases child provenance before its terminal owner. Any reference that clears
// behind this invocation's cursor is reconsidered by a fresh invocation, never by rewinding.
const PHASES:&[(&str,&str)]=&[("effect","native_effect"),("cleanup","native_cleanup"),("retirement_item","native_retirement_item"),("retirement","native_retirement"),("pin","native_pin"),("backup","native_backup_hold"),("attempt","native_attempt"),("checkpoint","native_history_checkpoint")];
fn eligible(phase:&str)->Result<&'static str,ModelError>{
    Ok(match phase {
        "effect"=>"$candidate_state.resolved AND $candidate_state.era<=$closed AND array::len(SELECT VALUE id FROM native_outcome_ref WITH INDEX effect_outcome_refs WHERE effect=$candidate LIMIT 1)=0",
        "cleanup"=>"$candidate_state.era<=$closed AND $candidate_state.state IN ['done','superseded'] AND ($candidate_state.state='superseded' OR array::len($candidate_state.owners)=0)",
        "retirement_item"=>"$candidate_state.state IN ['done','superseded'] AND $parent!=NONE AND $parent.era<=$closed AND $parent.state!='legacy_protected' AND array::len(SELECT VALUE id FROM native_retirement WITH INDEX lineage_retirements WHERE lineage=$parent.lineage AND state='recovering' LIMIT 1)=0",
        "retirement"=>"$candidate_state.era<=$closed AND $candidate_state.state IN ['open','done','superseded'] AND $candidate_state.root_count=$candidate_state.roots_registered AND array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_retirement WITH INDEX lineage_retirements WHERE lineage=$candidate_state.lineage AND state IN ['registering','recovering','open'] AND id!=$candidate LIMIT 1)=0",
        "pin"=>"$candidate_state.era<=$closed AND $candidate_state.released",
        "backup"=>"$candidate_state.era<=$closed AND !$candidate_state.active",
        "attempt"=>"$candidate_state.era<=$closed AND $candidate_state.epoch>0 AND $candidate_state.state IN ['closed','abandoned','frozen','maintenance_fenced'] AND array::len(SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM compiler_binding WITH INDEX binding_attempt WHERE attempt=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_cleanup WITH INDEX attempt_cleanup WHERE attempt=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_effect WITH INDEX attempt_effects WHERE attempt=$candidate LIMIT 1)=0",
        "checkpoint"=>"$candidate_state.era<=$closed AND $candidate_state.state IN ['open','done'] AND $candidate!=$checkpoint",
        _=>return Err(ModelError::Schema("native history phase")),
    })
}
/// A request identifies the caller's original limit, even when the physical page is smaller.
fn page_request(identity:ContentHash,expected_revision:u64,limit:usize)->Result<ContentHash,ModelError>{
    if limit==0 || expected_revision>i64::MAX as u64 {return Err(ModelError::Invalid("native history page limit/revision".into()));}
    let mut sink=KeySink::new("native-history-page/v1");
    identity.encode(&mut sink);
    sink.part(b"expected-revision",&expected_revision.to_le_bytes());
    sink.part(b"requested-limit",&u64::try_from(limit).map_err(ModelError::codec)?.to_le_bytes());
    Ok(sink.finish())
}
fn native_ids(row:&Object,field:&str)->Result<Vec<RecordId>,ModelError>{
    let Some(Value::Array(values))=row.get(field) else{return Err(ModelError::Schema("native history outcome identities"));};
    values.iter().map(|value|match value {Value::RecordId(id)=>Ok(id.clone()),_=>Err(ModelError::Schema("native history outcome record"))}).collect()
}
fn outcome(identity:ContentHash,row:&Object)->Result<HistoryCompaction,ModelError>{
    if row.get("identity")!=Some(&Value::RecordId(RecordId::new("native_history_checkpoint",identity.hex()))) {return Err(ModelError::Conflict("native history outcome identity"));}
    let Some(Value::String(state))=row.get("state") else{return Err(ModelError::Schema("native history outcome state"));};
    if !matches!(state.as_str(),"open"|"done"){return Err(ModelError::Schema("native history outcome state"));}
    let unsigned=|field|u64::try_from(int_field(row,field)?).map_err(ModelError::codec);
    let result=HistoryCompaction{identity,closed_through:int_field(row,"closed_through")?,removed_effects:unsigned("removed_effects")?,removed_records:unsigned("removed_records")?,state:state.clone(),revision:unsigned("revision")?,examined:native_ids(row,"examined")?,removed:native_ids(row,"removed")?,protected:native_ids(row,"protected")?};
    let examined=result.examined.iter().collect::<std::collections::BTreeSet<_>>();
    let removed=result.removed.iter().collect::<std::collections::BTreeSet<_>>();
    let protected=result.protected.iter().collect::<std::collections::BTreeSet<_>>();
    if examined.len()!=result.examined.len() || removed.len()!=result.removed.len() || protected.len()!=result.protected.len() || !removed.is_disjoint(&protected) || removed.union(&protected).copied().collect::<std::collections::BTreeSet<_>>()!=examined || result.examined.len()>crate::loader::NATIVE_WINDOW_ROWS || result.removed_effects>result.removed_records || result.removed_records<result.removed.len() as u64 {return Err(ModelError::Schema("native history outcome partition"));}
    Ok(result)
}
/// Decode native values directly: record IDs remain record IDs throughout persistence/replay.
fn replay(identity:ContentHash,row:&Object,expected_revision:u64,request:ContentHash)->Result<Option<HistoryCompaction>,ModelError>{
    let Some(value)=row.get("last_page") else{return Ok(None);};
    if matches!(value,Value::None){return Ok(None);}
    let Value::Object(page)=value else{return Err(ModelError::Schema("native history last page"));};
    if int_field(page,"format")?!=1 {return Err(ModelError::Schema("native history page format"));}
    let expected=u64::try_from(int_field(page,"expected_revision")?).map_err(ModelError::codec)?;
    let revision=u64::try_from(int_field(page,"revision")?).map_err(ModelError::codec)?;
    let Some(Value::Object(result))=page.get("result") else{return Err(ModelError::Schema("native history page result"));};
    let result=outcome(identity,result)?;
    if expected.checked_add(1)!=Some(revision) || result.revision!=revision || int_field(row,"revision")?!=revision as i64 || int_field(row,"closed_through")?!=result.closed_through || int_field(row,"removed_effects")?!=result.removed_effects as i64 || int_field(row,"removed_records")?!=result.removed_records as i64 || row.get("state")!=Some(&Value::String(result.state.clone())) {return Err(ModelError::Schema("native history page checkpoint mismatch"));}
    let digest=hash_field(page,"request")?;
    if expected==expected_revision {
        if digest!=request {return Err(ModelError::Conflict("native history replay request changed"));}
        return Ok(Some(result));
    }
    Ok(None)
}
async fn checkpoint_row(client:&Surreal<Client>,checkpoint:&RecordId)->Result<Object,ModelError>{
    let mut response=client.query("SELECT * FROM $checkpoint").bind(("checkpoint",checkpoint.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native history checkpoint missing"));};
    Ok(row.clone())
}
/// Continue one bounded range or one finite empty-phase transition. The caller acknowledges
/// the last exact page by supplying its returned revision; older/changed requests refuse.
pub async fn resume_history_compaction(client:&Surreal<Client>,identity:ContentHash,expected_revision:u64,limit:usize)->Result<HistoryCompaction,ModelError>{
    let request=page_request(identity,expected_revision,limit)?;
    let current=IssuanceEra::capture(client).await?;
    let checkpoint=RecordId::new("native_history_checkpoint",identity.hex());
    let row=checkpoint_row(client,&checkpoint).await?;
    if hash_field(&row,"generation")?!=current.generation {return Err(ModelError::Conflict("native history checkpoint generation"));}
    // Reconciliation has no effects and can observe an exact acknowledged page after an era cut.
    if let Some(result)=replay(identity,&row,expected_revision,request)? {return Ok(result);}
    if int_field(&row,"revision")?!=expected_revision as i64 {return Err(ModelError::Conflict("native history stale expected revision"));}
    if row.get("state")==Some(&Value::String("done".into())) {
        let mut result=row.clone();result.insert("identity",checkpoint);result.insert("examined",Vec::<RecordId>::new());result.insert("removed",Vec::<RecordId>::new());result.insert("protected",Vec::<RecordId>::new());
        return outcome(identity,&result);
    }
    if expected_revision==i64::MAX as u64 {return Err(ModelError::Conflict("native history revision exhausted"));}
    if int_field(&row,"era")?!=current.era {return Err(ModelError::Conflict("native history checkpoint needs a fresh current-era invocation"));}
    let closed=int_field(&row,"closed_through")?;
    let Some(Value::String(phase))=row.get("phase") else{return Err(ModelError::Schema("native history phase"));};
    let position=PHASES.iter().position(|(name,_)|name==phase).ok_or(ModelError::Schema("native history phase"))?;
    let after=row.get("after").cloned().unwrap_or(Value::None);
    let selection=format!("SELECT * FROM {} LIMIT {}",keyset_source(PHASES[position].1,&after)?,limit.min(crate::loader::NATIVE_WINDOW_ROWS));
    let mut response=client.query(selection).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let candidates:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let rows=charge_candidates(client,phase,candidates).await?;
    let sql=page_sql(phase,position,rows.len())?;
    let mut bindings=Variables::new();bindings.insert("checkpoint",checkpoint.clone());bindings.insert("closed",closed);bindings.insert("after",after);bindings.insert("phase",phase.clone());bindings.insert("__generation",current.generation.hex());bindings.insert("__era",current.era);bindings.insert("expected_revision",expected_revision as i64);bindings.insert("request",request.hex());bindings.insert("rows",rows);
    bind_runtime_schema(&mut bindings);
    let result=collection_page(client,identity,&sql,bindings).await;
    resolve_collection_page(client,identity,current,expected_revision,request,result).await
}
// Shared production completion path. Qualification controls may discard a locally returned
// acknowledgement before invoking it; no public injection surface or network claim follows.
async fn resolve_collection_page(client:&Surreal<Client>,identity:ContentHash,current:IssuanceEra,expected_revision:u64,request:ContentHash,result:Result<Object,ModelError>)->Result<HistoryCompaction,ModelError>{
    let checkpoint=RecordId::new("native_history_checkpoint",identity.hex());
    match result {
        Ok(value)=>outcome(identity,&value),
        Err(error)=>{
            // Never repeat effects after an unknown acknowledgement. Read this request's
            // exact durable page, or retain the original failure and checkpoint identity.
            let reconciled=async {
                let row=checkpoint_row(client,&checkpoint).await?;
                if hash_field(&row,"generation")?!=current.generation{return Err(ModelError::Conflict("native history reconciliation generation"));}
                replay(identity,&row,expected_revision,request)
            }.await;
            match reconciled {
                Ok(Some(result))=>Ok(result),
                other=>{let mut completion=lctx_model::domain::completion::Completion::default();completion.committed("native history checkpoint",identity.hex());if let Err(reconciliation)=other{completion.step("native history exact page reconciliation",Err(reconciliation));}lctx_model::domain::completion::complete(Err(error),completion)}
            }
        }
    }
}
async fn charge_candidates(client:&Surreal<Client>,phase:&str,candidates:Vec<Object>)->Result<Vec<Value>,ModelError>{
    let mut parents=std::collections::BTreeMap::new();
    if phase=="retirement_item" && !candidates.is_empty(){
        let ids=candidates.iter().map(|row|match row.get("job"){Some(Value::RecordId(id)) if id.table.as_str()=="native_retirement"=>Ok(id.clone()),_=>Err(ModelError::Schema("native history retirement parent"))}).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
        let mut response=client.query("SELECT * FROM $parents").bind(("parents",ids.into_iter().collect::<Vec<_>>())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        for row in rows {let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native history parent identity"));};parents.insert(id.clone(),row.clone());}
    }
    charged_candidate_rows(candidates,&parents)
}
fn charged_candidate_rows(candidates:Vec<Object>,parents:&std::collections::BTreeMap<RecordId,Object>)->Result<Vec<Value>,ModelError>{
    let rows=candidates.into_iter().map(|candidate|{
        let Some(Value::RecordId(id))=candidate.get("id") else{return Err(ModelError::Schema("native history candidate identity"));};
        let mut charged=Object::new();
        // Include the possible receipt partitions in the same row/byte admission, plus
        // the actual parent body. Each candidate has at most one destructive effect.
        charged.insert("examined",id.clone());charged.insert("removed",id.clone());charged.insert("protected",id.clone());
        let parent=match candidate.get("job"){Some(Value::RecordId(id))=>parents.get(id).cloned().map(Value::Object).unwrap_or(Value::None),_=>Value::None};
        charged.insert("parent",parent);charged.insert("candidate",candidate);Ok(Value::Object(charged))
    }).collect::<Result<Vec<_>,ModelError>>()?;
    crate::loader::NativeWindows::new(rows).next().transpose().map(|rows|rows.unwrap_or_default())
}
fn page_sql(phase:&str,position:usize,count:usize)->Result<String,ModelError>{
    if count>crate::loader::NATIVE_WINDOW_ROWS || PHASES.get(position).map(|entry|entry.0)!=Some(phase){return Err(ModelError::Schema("native history bounded phase"));}
    let mut sql=format!("BEGIN; {} {RUNTIME_SCHEMA_GUARD} LET $progress=SELECT * FROM ONLY $checkpoint FOR UPDATE; IF $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through<$closed {{ THROW 'native history horizon changed'; }}; IF $progress=NONE OR $progress.generation!=$__generation OR $progress.era!=$__era OR $progress.closed_through!=$closed OR ($progress.after ?? NONE)!=$after OR $progress.phase!=$phase OR $progress.state!='open' OR $progress.revision!=$expected_revision {{ THROW 'native history cursor changed'; }};",authorization::ERA_GUARD);
    let mut removed=Vec::new();let mut protected=Vec::new();
    for index in 0..count {
        let parent=if phase=="retirement_item"{"LET $parent_id=$candidate_state.job; LET $parent=SELECT * FROM ONLY $parent_id FOR UPDATE; IF $parent!=$row.parent { THROW 'native history parent changed'; };"}else{"LET $parent=NONE;"};
        let generation=if phase=="retirement_item"{"$parent.generation=$__generation"}else{"$candidate_state.generation=$__generation"};
        sql.push_str(&format!(" LET $row=$rows[{index}]; LET $candidate=$row.candidate.id; LET $candidate_state=SELECT * FROM ONLY $candidate FOR UPDATE; IF $candidate_state!=$row.candidate {{ THROW 'native history candidate changed'; }}; {parent} LET $deleted_{index}=IF $candidate_state!=NONE AND ({generation}) AND ({}) AND array::len(SELECT VALUE id FROM native_outcome_ref WITH INDEX object_outcome_refs WHERE object=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_effect WITH INDEX owner_effects WHERE owner=$candidate LIMIT 1)=0 {{ DELETE $candidate RETURN NONE; true }} ELSE {{ false }};",eligible(phase)?));
        removed.push(format!("IF $deleted_{index} {{ [$rows[{index}].candidate.id] }} ELSE {{ [] }}"));
        protected.push(format!("IF $deleted_{index} {{ [] }} ELSE {{ [$rows[{index}].candidate.id] }}"));
    }
    sql.push_str(&format!(" LET $removed=array::flatten([{}]); LET $protected=array::flatten([{}]);",removed.join(","),protected.join(",")));
    let transition=if count>0 {format!("after=$rows[{}].candidate.id",count-1)}else if let Some((next,_))=PHASES.get(position+1){format!("phase='{next}',after=NONE")}else{"state='done'".into()};
    let examined=(0..count).map(|index|format!("$rows[{index}].candidate.id")).collect::<Vec<_>>().join(",");
    let effect_count=if phase=="effect"{"array::len($removed)"}else{"0"};
    let state=if count==0 && position+1==PHASES.len(){"'done'"}else{"'open'"};
    sql.push_str(&format!(" LET $result={{identity:$checkpoint,closed_through:$closed,removed_effects:$progress.removed_effects+{effect_count},removed_records:$progress.removed_records+array::len($removed),state:{state},revision:$expected_revision+1,examined:[{examined}],removed:$removed,protected:$protected}}; UPDATE $checkpoint SET {transition},revision=$result.revision,removed_effects=$result.removed_effects,removed_records=$result.removed_records,last_page={{format:1,expected_revision:$expected_revision,request:$request,revision:$result.revision,result:$result}} RETURN NONE; RETURN $result; COMMIT;"));
    Ok(sql)
}
async fn collection_page(client:&Surreal<Client>,_identity:ContentHash,sql:&str,bindings:Variables)->Result<Object,ModelError>{
    // SurrealDB 3.3 retains RETURN before COMMIT as the penultimate statement result.
    // Inspect every error before extracting it; an Ok response alone is not a commit.
    let response=client.query(sql.to_owned()).bind(bindings).await.map_err(crate::loader::write_failure)?;
    let mut response=checked_transaction(response)?;
    let index=response.num_statements().checked_sub(2).ok_or(ModelError::Schema("native history transaction return"))?;
    let value:Value=response.take(index).map_err(ModelError::codec)?;
    match value {Value::Object(row)=>Ok(row),_=>Err(ModelError::Schema("native history transaction outcome object"))}
}
/// Retain one exact lifecycle result under the same authority as effect outcomes. The
/// explicit inventory consumer releases this reference when its actual use ends.
pub async fn retain_native_outcome(client:&Surreal<Client>,object:RecordId,owner:&str,kind:&str)->Result<RecordId,ModelError>{
    if owner.is_empty() || !matches!(kind,"caller"|"recovery"|"evidence"|"migration") || !matches!(object.table.as_str(),"native_effect"|"native_attempt"|"native_pin"|"native_backup_hold"|"native_cleanup"|"native_retirement"|"native_retirement_item"|"native_history_checkpoint"){return Err(ModelError::Invalid("native exact outcome retention".into()));}
    let reference=RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&("native-lifecycle-outcome/v4",&object,owner,kind)).map_err(ModelError::codec)?).hex());
    let mut bindings=Variables::new();bindings.insert("object",object);bindings.insert("reference",reference.clone());bindings.insert("owner",owner.to_owned());bindings.insert("kind",kind.to_owned());
    effect(client,None,"LET $target=SELECT * FROM ONLY $object FOR UPDATE; IF $target=NONE { THROW 'native lifecycle outcome unavailable'; }; LET $prior=SELECT * FROM ONLY $reference FOR UPDATE; IF $prior!=NONE AND ($prior.object!=$object OR $prior.owner!=$owner OR $prior.kind!=$kind) { THROW 'native lifecycle outcome reference collision'; }; UPSERT $reference SET object=$object,owner=$owner,kind=$kind RETURN NONE",bindings).await?;
    Ok(reference)
}
/// Release one named legacy or lifecycle retention reference after its actual consumer ends.
/// This also covers migrated receipts whose historical request field was not a content hash.
pub async fn release_outcome_reference(client:&Surreal<Client>,reference:RecordId,owner:&str,kind:&str)->Result<(),ModelError>{
    if reference.table.as_str()!="native_outcome_ref" {return Err(ModelError::Invalid("native outcome reference table".into()));}
    let mut bindings=Variables::new();bindings.insert("reference",reference);bindings.insert("owner",owner.to_owned());bindings.insert("kind",kind.to_owned());
    effect(client,None,"LET $prior=SELECT * FROM ONLY $reference FOR UPDATE; IF $prior!=NONE { IF $prior.owner!=$owner OR $prior.kind!=$kind { THROW 'native outcome reference consumer mismatch'; }; DELETE $reference RETURN NONE; };",bindings).await
}

#[cfg(test)]
mod tests {
    use super::*;
    // One-off actual evidence acquisition, deliberately separate from qualification and
    // collector controls. The host/run owners supply exclusion and artifact retention.
    const INVENTORY_TABLES:[&str;18]=[
        "native_outcome_ref","native_effect","native_attempt","native_hold",
        "native_cleanup","native_retirement","native_retirement_item","native_guard",
        "native_pin","native_backup_hold","native_history_checkpoint","native_product",
        "compiler_contribution","compiler_binding","compiler_view","native_upgrade",
        "native_upgrade_progress_v1","native_installation",
    ];
    #[derive(serde::Serialize)]
    struct InventoryPage {
        sequence:u64,query:String,cursor_before:String,cursor_after:String,
        rows:usize,native_payload_bytes:usize,serialized_file_bytes:usize,
        file:String,blake3:String,native_ids_blake3:String,
    }
    #[derive(serde::Serialize)]
    struct InventoryTerminal {query:String,cursor:String,rows:usize}
    #[derive(serde::Serialize)]
    struct InventoryTable {
        table:String,present:bool,rows:u64,pages:Vec<InventoryPage>,
        terminal:Option<InventoryTerminal>,
    }
    #[derive(serde::Serialize)]
    struct InventoryManifest {
        format:u32,qualification:&'static str,codec:&'static str,
        namespace:String,database:String,generation:String,schema_version:i64,schema:String,
        executable:String,history_source_blake3:String,
        marker_before:String,marker_after:String,tables:Vec<InventoryTable>,
    }
    fn write_inventory_page(output:&std::path::Path,table:&str,sequence:u64,query:&str,after:&Value,rows:Vec<Value>)->Result<(InventoryPage,Value),ModelError>{
        use std::io::Write;
        use surrealdb::types::ToSql;
        let mut ids=Vec::with_capacity(rows.len());
        let mut next=after.clone();
        for row in &rows {
            let Value::Object(row)=row else{return Err(ModelError::Schema("history inventory native row"));};
            let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("history inventory native identity"));};
            if id.table.as_str()!=table || matches!(&next,Value::RecordId(previous) if id<=previous){return Err(ModelError::Conflict("history inventory native key order"));}
            next=Value::RecordId(id.clone());ids.push(next.clone());
        }
        if rows.is_empty(){return Err(ModelError::Schema("history inventory nonempty page"));}
        let native_payload_bytes=rows.iter().map(crate::loader::native_bytes).sum();
        let count=rows.len();
        let native_ids_blake3=ContentHash::of(Value::Array(ids.into()).to_sql().as_bytes()).hex();
        let mut encoded=Value::Array(rows.into()).to_sql();encoded.push('\n');
        let file=format!("{table}-{sequence:08}.surql");
        let mut destination=std::fs::OpenOptions::new().write(true).create_new(true).open(output.join(&file)).map_err(ModelError::codec)?;
        destination.write_all(encoded.as_bytes()).map_err(ModelError::codec)?;
        destination.sync_all().map_err(ModelError::codec)?;
        let page=InventoryPage{sequence,query:query.into(),cursor_before:after.to_sql(),cursor_after:next.to_sql(),rows:count,native_payload_bytes,serialized_file_bytes:encoded.len(),file,blake3:ContentHash::of(encoded.as_bytes()).hex(),native_ids_blake3};
        Ok((page,next))
    }
    async fn acquire_inventory_table(client:&Surreal<Client>,table:&str,output:&std::path::Path)->Result<InventoryTable,ModelError>{
        use surrealdb::types::ToSql;
        let mut result=InventoryTable{table:table.into(),present:true,rows:0,pages:Vec::new(),terminal:None};
        let mut after=Value::None;
        loop {
            let query=format!("SELECT * FROM {} LIMIT {}",keyset_source(table,&after)?,crate::loader::NATIVE_WINDOW_ROWS);
            let mut response=client.query(query.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
            if rows.is_empty(){result.terminal=Some(InventoryTerminal{query,cursor:after.to_sql(),rows:0});break;}
            // Every byte window from the fetched range is persisted; a narrow first
            // window must not cause the remaining fetched native identities to be skipped.
            for window in crate::loader::NativeWindows::new(rows) {
                let (page,next)=write_inventory_page(output,table,result.pages.len() as u64,&query,&after,window?)?;
                result.rows=result.rows.checked_add(page.rows as u64).ok_or(ModelError::Schema("history inventory row count"))?;
                result.pages.push(page);after=next;
            }
        }
        Ok(result)
    }
    #[tokio::test(flavor="multi_thread")]
    #[ignore="acquires actual HS6 evidence under owned closed Root validation maintenance into LCTX_RUN_DIR"]
    async fn acquire_actual_history_inventory_into_owned_run(){
        use std::io::Write;
        use surrealdb::types::ToSql;
        assert!(std::env::var("LCTX_SURREAL_MAINTENANCE_TOKEN").is_ok_and(|value|!value.is_empty()),"genuine host-owned maintenance required");
        let config_path=std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("maintenance installer configuration");
        let config=crate::RuntimeConfig::read(std::path::Path::new(&config_path)).unwrap();
        assert_eq!(config.authentication,crate::AuthenticationScope::Root);assert_eq!(config.database.as_str(),"validation");
        let run=std::path::PathBuf::from(std::env::var_os("LCTX_RUN_DIR").expect("existing managed run output required"));
        assert!(run.is_absolute() && run.is_dir(),"existing absolute managed run output required");
        let output=run.canonicalize().unwrap().join("hs6-history-inventory");
        let client=crate::upgrade::maintenance_client(&config).await.unwrap();
        let result=async {
            let marker_before=checkpoint_row(&client,&RecordId::new("native_installation","current")).await?;
            if marker_before.get("admission_open")!=Some(&Value::Bool(false)) || int_field(&marker_before,"schema_version")?!=SCHEMA_VERSION || hash_field(&marker_before,"schema")?!=crate::compiler::base_schema_identity() || hash_field(&marker_before,"generation")?!=config.service_generation{return Err(ModelError::Conflict("history inventory current closed installation"));}
            let catalog=crate::Loader::new(client.clone()).installation_catalog().await?;
            std::fs::create_dir(&output).map_err(ModelError::codec)?;
            let mut tables=Vec::with_capacity(INVENTORY_TABLES.len());
            for table in INVENTORY_TABLES {
                if !catalog.has_table(table) {
                    if table!="native_upgrade_progress_v1"{return Err(ModelError::Conflict("history inventory required native table"));}
                    tables.push(InventoryTable{table:table.into(),present:false,rows:0,pages:Vec::new(),terminal:None});
                }else{tables.push(acquire_inventory_table(&client,table,&output).await?);}
            }
            let marker_after=checkpoint_row(&client,&RecordId::new("native_installation","current")).await?;
            if marker_before!=marker_after{return Err(ModelError::Conflict("history inventory installation changed during exclusive read"));}
            let executable=std::env::current_exe().map_err(ModelError::codec)?;
            Ok::<_,ModelError>(InventoryManifest{format:1,qualification:"acquisition only; consumer completeness and qualification require owner review",codec:"surrealdb-3.3.0 native Value::to_sql; row files are array literals with trailing newline; native_ids_blake3 hashes the native ID array literal without newline; cursors and markers are native SurrealQL literals",namespace:config.namespace.as_str().into(),database:config.database.as_str().into(),generation:config.service_generation.hex(),schema_version:SCHEMA_VERSION,schema:crate::compiler::base_schema_identity().hex(),executable:executable.display().to_string(),history_source_blake3:ContentHash::of(include_bytes!("history.rs")).hex(),marker_before:Value::Object(marker_before).to_sql(),marker_after:Value::Object(marker_after).to_sql(),tables})
        }.await;
        let mut completion=lctx_model::domain::completion::Completion::default();
        completion.step("history inventory session invalidation",client.invalidate().await.map_err(ModelError::codec));
        let manifest=lctx_model::domain::completion::complete(result,completion).unwrap();
        // A partial scan or failed checked invalidation cannot publish a completion
        // manifest. Keep partial page files under the run owner for diagnosis.
        let mut encoded=serde_json::to_vec_pretty(&manifest).unwrap();encoded.push(b'\n');
        let mut destination=tempfile::NamedTempFile::new_in(&output).unwrap();
        destination.write_all(&encoded).unwrap();destination.as_file().sync_all().unwrap();
        let path=output.join("manifest.json");destination.persist_noclobber(&path).unwrap();
        std::fs::File::open(&output).unwrap().sync_all().unwrap();
        println!("history inventory acquisition only: {} blake3={}",path.display(),ContentHash::of(&encoded).hex());
    }
    fn receipt(identity:ContentHash,request:ContentHash)->Object {
        let a=RecordId::new("native_attempt","a");let b=RecordId::new("native_attempt","b");
        let mut result=Object::new();result.insert("identity",RecordId::new("native_history_checkpoint",identity.hex()));result.insert("closed_through",4i64);result.insert("revision",8i64);result.insert("removed_effects",1i64);result.insert("removed_records",3i64);result.insert("state","open");result.insert("examined",vec![a.clone(),b.clone()]);result.insert("removed",vec![a]);result.insert("protected",vec![b]);
        let mut page=Object::new();page.insert("format",1i64);page.insert("expected_revision",7i64);page.insert("revision",8i64);page.insert("request",request.hex());page.insert("result",result.clone());
        let mut checkpoint=result;checkpoint.insert("last_page",page);checkpoint
    }
    #[test]
    fn history_installation_projection_refuses_old_or_changed_schema_before_intent(){
        surrealdb_syn::parse(HISTORY_INSTALLATION_QUERY).unwrap();
        assert!(HISTORY_INSTALLATION_QUERY.contains("schema,schema_version"));
        let issuance=IssuanceEra{generation:ContentHash::of(b"installation"),era:5};
        let mut row=Object::new();row.insert("generation",issuance.generation.hex());row.insert("closed_through",4i64);row.insert("history_inventory",ContentHash::of(b"separately-qualified").hex());row.insert("schema",crate::compiler::base_schema_identity().hex());row.insert("schema_version",SCHEMA_VERSION);
        assert_eq!(history_horizon(&row,issuance).unwrap(),4);
        row.insert("schema_version",4i64);assert!(history_horizon(&row,issuance).is_err(),"current hash cannot authorize an older format");
        row.insert("schema_version",SCHEMA_VERSION);row.insert("schema",ContentHash::of(b"previous-runtime").hex());assert!(history_horizon(&row,issuance).is_err(),"current version cannot stamp changed declaration meaning");
        row.remove("schema_version");assert!(history_horizon(&row,issuance).is_err());
    }
    #[test]
    fn history_page_request_preserves_original_identity_revision_and_limit(){
        let identity=ContentHash::of(b"history");let request=page_request(identity,7,1024).unwrap();
        assert_eq!(request,page_request(identity,7,1024).unwrap());
        assert_ne!(request,page_request(identity,7,128).unwrap());
        assert_ne!(request,page_request(identity,8,1024).unwrap());
        assert_ne!(request,page_request(ContentHash::of(b"other"),7,1024).unwrap());
        assert!(page_request(identity,7,0).is_err());assert!(page_request(identity,i64::MAX as u64,1).is_ok());assert!(page_request(identity,i64::MAX as u64+1,1).is_err());
    }
    #[test]
    fn history_page_native_receipt_replays_exactly_and_refuses_changed_or_corrupt_outcomes(){
        let identity=ContentHash::of(b"history");let request=page_request(identity,7,1024).unwrap();let checkpoint=receipt(identity,request);
        let page=replay(identity,&checkpoint,7,request).unwrap().unwrap();
        assert_eq!(page.revision,8);assert_eq!(page.closed_through,4);assert_eq!(page.examined,[RecordId::new("native_attempt","a"),RecordId::new("native_attempt","b")]);assert_eq!(page.protected,[RecordId::new("native_attempt","b")]);
        assert!(replay(identity,&checkpoint,7,page_request(identity,7,128).unwrap()).is_err());
        assert!(replay(identity,&checkpoint,6,page_request(identity,6,1024).unwrap()).unwrap().is_none());
        assert!(replay(identity,&checkpoint,8,page_request(identity,8,1024).unwrap()).unwrap().is_none());
        let mut old=checkpoint.clone();old.remove("last_page");assert!(replay(identity,&old,7,request).unwrap().is_none(),"a pre-transition checkpoint has no historical page");
        let mut corrupt=checkpoint.clone();corrupt.insert("revision",9i64);assert!(replay(identity,&corrupt,7,request).is_err());
        let mut corrupt=checkpoint.clone();
        let Some(Value::Object(page))=corrupt.get_mut("last_page") else{panic!("page")};
        let Some(Value::Object(result))=page.get_mut("result") else{panic!("result")};
        result.insert("protected",vec![RecordId::new("native_attempt","a")]);assert!(replay(identity,&corrupt,7,request).is_err(),"removed and protected must partition examined");
        let mut corrupt=checkpoint;
        let Some(Value::Object(page))=corrupt.get_mut("last_page") else{panic!("page")};
        let Some(Value::Object(result))=page.get_mut("result") else{panic!("result")};
        result.insert("examined",vec!["native_attempt:a","native_attempt:b"]);assert!(replay(identity,&corrupt,7,request).is_err(),"record identity must never be replaced with a string codec");
    }
    #[test]
    fn history_candidate_window_charges_actual_parent_bodies_and_receipt_partitions(){
        let parent=RecordId::new("native_retirement","wide-parent");
        let candidates=(0..2).map(|index|{let mut row=Object::new();row.insert("id",RecordId::new("native_retirement_item",format!("item-{index}")));row.insert("job",parent.clone());row}).collect::<Vec<_>>();
        assert_eq!(charged_candidate_rows(candidates.clone(),&std::collections::BTreeMap::new()).unwrap().len(),2);
        let mut body=Object::new();body.insert("id",parent.clone());body.insert("root_digest","x".repeat(lctx_model::domain::resources::TRANSFER_BYTES/2+1));
        let parents=std::collections::BTreeMap::from([(parent,body)]);
        let window=charged_candidate_rows(candidates.clone(),&parents).unwrap();
        assert_eq!(window.len(),1,"parent metadata must consume the real native byte target even for two tiny candidates");
        assert!(crate::loader::native_bytes(&window[0])>lctx_model::domain::resources::TRANSFER_BYTES/2);
        let Value::Object(row)=&window[0] else{panic!("charged row")};
        assert_eq!(row.get("examined"),candidates[0].get("id"));assert_eq!(row.get("removed"),candidates[0].get("id"));assert_eq!(row.get("protected"),candidates[0].get("id"));
    }

    fn rollback_after_deletion_sql(sql:&str)->String {
        assert_eq!(sql.matches(" UPDATE $checkpoint SET").count(),1);
        sql.replacen(" UPDATE $checkpoint SET"," IF array::len(SELECT VALUE id FROM $candidate_first)>0 { THROW 'qualification candidate was not deleted'; }; THROW 'qualification rollback after actual deletion'; UPDATE $checkpoint SET",1)
    }
    fn check_preceding_deletion_sql(sql:&str)->String {
        assert_eq!(sql.matches(" LET $row=$rows[1]").count(),1);
        sql.replacen(" LET $row=$rows[1]"," IF array::len(SELECT VALUE id FROM $candidate_first)>0 { THROW 'qualification first retirement item was not deleted'; }; LET $row=$rows[1]",1)
    }
    #[test]
    fn history_private_qualification_fault_queries_parse_production_pages(){
        let attempt=PHASES.iter().position(|entry|entry.0=="attempt").unwrap();
        let retirement_item=PHASES.iter().position(|entry|entry.0=="retirement_item").unwrap();
        surrealdb_syn::parse(&rollback_after_deletion_sql(&page_sql("attempt",attempt,2).unwrap())).unwrap();
        surrealdb_syn::parse(&check_preceding_deletion_sql(&page_sql("retirement_item",retirement_item,2).unwrap())).unwrap();
    }
    struct QualifiedPage {
        identity:ContentHash,current:IssuanceEra,closed:i64,phase:&'static str,after:Value,candidates:Vec<RecordId>,parents:Vec<RecordId>,
    }
    async fn qualified_client()->Result<(Arc<Surreal<Client>>,IssuanceEra,i64),ModelError>{
        assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),"explicit owned validation maintenance required");
        let path=std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("maintenance installer configuration");
        let config=crate::RuntimeConfig::read(std::path::Path::new(&path))?;
        assert_eq!(config.authentication,crate::AuthenticationScope::Root);assert_eq!(config.database.as_str(),"validation");
        let expected=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("separately qualified existing history consumer inventory");
        let client=crate::compiler::check_installation(&config).await?;
        let checked=async {
            let current=IssuanceEra::capture(&client).await?;
            let installation=checkpoint_row(&client,&RecordId::new("native_installation","current")).await?;
            let closed=history_horizon(&installation,current)?;
            if installation.get("admission_open")!=Some(&Value::Bool(false)) || installation.get("history_inventory")!=Some(&Value::String(expected)) {return Err(ModelError::Conflict("history control requires closed maintenance and separately qualified existing inventory"));}
            Ok::<_,ModelError>((current,closed))
        }.await;
        match checked {Ok((current,closed))=>Ok((client,current,closed)),Err(error)=>{let mut completion=lctx_model::domain::completion::Completion::default();completion.step("failed history qualification session invalidation",client.invalidate().await.map_err(ModelError::codec));lctx_model::domain::completion::complete(Err(error),completion)}}
    }
    async fn seed_qualified_page(client:&Surreal<Client>,current:IssuanceEra,closed:i64,phase:&'static str)->Result<QualifiedPage,ModelError>{
        let identity=fresh_identity("history-atomic-page-qualification")?;let prefix=format!("!history_atomic_{}",identity.hex());
        let table=if phase=="retirement_item"{"native_retirement_item"}else{"native_attempt"};
        let candidates=(1..=2).map(|index|RecordId::new(table,format!("{prefix}_{index}"))).collect::<Vec<_>>();
        let parents=if phase=="retirement_item"{(1..=2).map(|index|RecordId::new("native_retirement",format!("{prefix}_{index}"))).collect::<Vec<_>>()}else{Vec::new()};
        let after=Value::RecordId(RecordId::new(table,format!("{prefix}_0")));
        let rows=candidates.iter().enumerate().map(|(index,id)|{let mut row=Object::new();row.insert("id",id.clone());if let Some(parent)=parents.get(index){row.insert("parent",parent.clone());row.insert("object",RecordId::new("native_guard",format!("{prefix}_object_{index}")));}Value::Object(row)}).collect::<Vec<_>>();
        let mut bindings=Variables::new();bindings.insert("__generation",current.generation.hex());bindings.insert("__era",current.era);bindings.insert("closed",closed);bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",identity.hex()));bindings.insert("phase",phase);bindings.insert("after",after.clone());bindings.insert("rows",rows);bindings.insert("inventory",std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("qualified existing inventory"));bindings.insert("root_digest",ContentHash::of(b"qualification-owned-single-root").hex());bind_runtime_schema(&mut bindings);
        let body=if phase=="retirement_item" {
            "FOR $row IN $rows { LET $parent=$row.parent; LET $object=$row.object; LET $id=$row.id; CREATE $parent SET generation=$__generation,era=$closed,epoch=1,revision=0,examined=1,retired=1,root_digest=$root_digest,lineage=$parent,state='done',root_count=1,roots_registered=1 RETURN NONE; CREATE $object SET revision=0,retired=true,retired_through=0,phase='retired',incarnation=1 RETURN NONE; CREATE $id SET job=$parent,object=$object,state='done',incarnation=1,cutoff=0 RETURN NONE; };"
        }else {
            "FOR $row IN $rows { LET $id=$row.id; CREATE $id SET generation=$__generation,era=$closed,epoch=1,state='closed',admitted=false,revision=0 RETURN NONE; };"
        };
        let sql=format!("BEGIN; {} {RUNTIME_SCHEMA_GUARD} IF $__installation.admission_open OR $__installation.history_inventory!=$inventory OR $__installation.closed_through!=$closed {{ THROW 'history qualification requires original qualified maintenance horizon'; }}; {body} CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=1,closed_through=$closed,phase=$phase,after=$after,state='open',removed_effects=0,removed_records=0,revision=0 RETURN NONE; COMMIT;",authorization::ERA_GUARD);
        checked_transaction(client.query(sql).bind(bindings).await.map_err(ModelError::codec)?)?;
        Ok(QualifiedPage{identity,current,closed,phase,after,candidates,parents})
    }
    async fn qualified_page_query(client:&Surreal<Client>,page:&QualifiedPage)->Result<(String,Variables,ContentHash),ModelError>{
        let mut response=client.query("SELECT * FROM $candidates").bind(("candidates",page.candidates.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let mut candidates:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        candidates.sort_by_key(|row|row.get("id").cloned());
        let rows=charge_candidates(client,page.phase,candidates).await?;
        let position=PHASES.iter().position(|entry|entry.0==page.phase).ok_or(ModelError::Schema("qualified history phase"))?;
        let sql=page_sql(page.phase,position,rows.len())?;let request=page_request(page.identity,0,2)?;
        let mut bindings=Variables::new();bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",page.identity.hex()));bindings.insert("closed",page.closed);bindings.insert("after",page.after.clone());bindings.insert("phase",page.phase);bindings.insert("__generation",page.current.generation.hex());bindings.insert("__era",page.current.era);bindings.insert("expected_revision",0i64);bindings.insert("request",request.hex());bindings.insert("rows",rows);bindings.insert("candidate_first",page.candidates[0].clone());bind_runtime_schema(&mut bindings);
        Ok((sql,bindings,request))
    }
    async fn qualified_snapshot(client:&Surreal<Client>,page:&QualifiedPage)->Result<Vec<Value>,ModelError>{
        let records=page.candidates.iter().chain(page.parents.iter()).cloned().chain(std::iter::once(RecordId::new("native_history_checkpoint",page.identity.hex()))).collect::<Vec<_>>();
        let mut response=client.query("SELECT * FROM $records; SELECT control_revision,admission_revision FROM native_installation:current; SELECT count() FROM native_effect GROUP ALL; SELECT count() FROM native_outcome_ref GROUP ALL;").bind(("records",records)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        (0..4).map(|index|response.take::<Value>(index).map_err(ModelError::codec)).collect()
    }
    #[tokio::test(flavor="multi_thread")]
    #[ignore="requires explicit closed validation maintenance and separately qualified existing history inventory"]
    async fn qualified_history_atomic_rollback_and_controlled_local_acknowledgement_reconciliation(){
        let (client,current,closed)=qualified_client().await.unwrap();
        let result=async {
            let page=seed_qualified_page(&client,current,closed,"attempt").await?;
            let (sql,bindings,request)=qualified_page_query(&client,&page).await?;
            let before=qualified_snapshot(&client,&page).await?;
            // Inject only into this private control's production SQL. The sentinel proves
            // deletion ran before THROW; executor rollback must restore it and the guard.
            let fault=rollback_after_deletion_sql(&sql);
            let failure=collection_page(&client,page.identity,&fault,bindings.clone()).await.expect_err("injected transaction fault");
            assert!(failure.to_string().contains("qualification rollback after actual deletion"),"{failure}");
            assert!(resolve_collection_page(&client,page.identity,current,0,request,Err(failure)).await.is_err(),"rollback has no last-page acknowledgement to recover");
            assert_eq!(qualified_snapshot(&client,&page).await?,before,"candidate deletion, checkpoint and installation revision roll back atomically");
            let committed=collection_page(&client,page.identity,&sql,bindings).await?;let expected=outcome(page.identity,&committed)?;
            assert_eq!(expected.removed,page.candidates);assert_eq!(expected.revision,1);
            let committed_snapshot=qualified_snapshot(&client,&page).await?;
            // A controlled local acknowledgement discard exercises the same Err-to-read
            // reconciliation branch as production. This is not a real network-failure test.
            let discarded=Err(ModelError::Conflict("qualification controlled local acknowledgement discarded"));
            assert_eq!(resolve_collection_page(&client,page.identity,current,0,request,discarded).await?,expected);
            assert_eq!(qualified_snapshot(&client,&page).await?,committed_snapshot,"exact reconciliation must perform no extra effect or revision change");
            Ok::<_,ModelError>(())
        }.await;
        let mut completion=lctx_model::domain::completion::Completion::default();completion.step("history atomic qualification session invalidation",client.invalidate().await.map_err(ModelError::codec));lctx_model::domain::completion::complete(result,completion).unwrap();
    }
    #[tokio::test(flavor="multi_thread")]
    #[ignore="requires explicit closed validation maintenance and separately qualified existing history inventory"]
    async fn qualified_history_changed_parent_rolls_back_preceding_candidate_deletion(){
        let (client,current,closed)=qualified_client().await.unwrap();
        let result=async {
            let page=seed_qualified_page(&client,current,closed,"retirement_item").await?;
            let (sql,bindings,request)=qualified_page_query(&client,&page).await?;
            let mut change=Variables::new();change.insert("__generation",current.generation.hex());change.insert("__era",current.era);change.insert("parent",page.parents[1].clone());bind_runtime_schema(&mut change);
            let change_sql=format!("BEGIN; {} {RUNTIME_SCHEMA_GUARD} IF $__installation.admission_open {{ THROW 'qualification maintenance ended'; }}; UPDATE $parent SET revision+=1 RETURN NONE; COMMIT;",authorization::ERA_GUARD);
            checked_transaction(client.query(change_sql).bind(change).await.map_err(ModelError::codec)?)?;
            let before=qualified_snapshot(&client,&page).await?;
            let guarded=check_preceding_deletion_sql(&sql);
            let failure=collection_page(&client,page.identity,&guarded,bindings).await.expect_err("second parent changed after nomination");
            assert!(failure.to_string().contains("native history parent changed"),"{failure}");
            assert!(resolve_collection_page(&client,page.identity,current,0,request,Err(failure)).await.is_err());
            assert_eq!(qualified_snapshot(&client,&page).await?,before,"changed parent refuses the entire page and restores the first deletion");
            Ok::<_,ModelError>(())
        }.await;
        let mut completion=lctx_model::domain::completion::Completion::default();completion.step("history parent qualification session invalidation",client.invalidate().await.map_err(ModelError::codec));lctx_model::domain::completion::complete(result,completion).unwrap();
    }

    #[test]
    fn history_page_sql_and_actual_runtime_receipt_field_parse_at_the_locked_version(){
        surrealdb_syn::parse(schema()).unwrap();
        let field=schema().lines().find(|line|line.starts_with("DEFINE FIELD last_page ON native_history_checkpoint ")).unwrap();
        surrealdb_syn::parse(field).unwrap();
        assert!(field.contains("TYPE option<object> FLEXIBLE"));
        for (position,(phase,_)) in PHASES.iter().enumerate(){
            for count in [0,1,crate::loader::NATIVE_WINDOW_ROWS]{
                let sql=page_sql(phase,position,count).unwrap();
                surrealdb_syn::parse(&sql).unwrap();
                assert!(sql.find(RUNTIME_SCHEMA_GUARD).unwrap()<sql.find("LET $progress").unwrap(),"schema guard must precede all checkpoint/candidate effects");
            }
        }
        assert!(page_sql("effect",0,crate::loader::NATIVE_WINDOW_ROWS+1).is_err());
    }
}
