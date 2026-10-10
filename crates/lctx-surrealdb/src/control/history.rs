//! Explicit drained horizons and named outcome retention. No age-based deletion authority.
use super::*;
use authorization::{hash_field,int_field};

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
#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct HistoryCompaction { pub identity:ContentHash,pub closed_through:i64,pub removed_effects:u64,pub removed_records:u64,pub state:String,pub protected:Vec<RecordId> }
/// Examine at most `limit` exact records behind the permanent era fence. Rich control
/// records are removed only after terminal state, exact references and actual provenance
/// consumers permit it. Slim incarnation guards and the era watermark remain permanent.
pub async fn compact_history(client:&Surreal<Client>,limit:usize)->Result<HistoryCompaction,ModelError>{
    if limit==0{return Err(ModelError::Invalid("native history page limit".into()));}
    let issuance=IssuanceEra::capture(client).await?;
    let identity=fresh_identity("history-collection")?;
    let mut response=client.query("SELECT generation,closed_through,history_inventory FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Schema("native history installation"));};
    if hash_field(row,"generation")?!=issuance.generation || !matches!(row.get("history_inventory"),Some(Value::String(value)) if !value.is_empty()) {return Err(ModelError::Conflict("native history consumer inventory unqualified"));}
    let closed=int_field(row,"closed_through")?;
    let mut bindings=Variables::new();bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",identity.hex()));bindings.insert("closed",closed);bindings.insert("issuance",issuance.era);
    effect(client,None,"IF $__era!=$issuance OR $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through!=$closed { THROW 'native history horizon changed'; }; CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=$__epoch,closed_through=$closed,revision=0,removed_effects=0,state='open' RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("history invocation",identity,issuance,error))?;
    resume_history_compaction(client,identity,limit).await
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
/// Continue the exact persisted horizon/phase/cursor. Deletion and cursor advance commit
/// together; an uncertain acknowledgement cannot skip or expand the original scope.
pub async fn resume_history_compaction(client:&Surreal<Client>,identity:ContentHash,limit:usize)->Result<HistoryCompaction,ModelError>{
    if limit==0{return Err(ModelError::Invalid("native history page limit".into()));}
    let current=IssuanceEra::capture(client).await?;
    let checkpoint=RecordId::new("native_history_checkpoint",identity.hex());
    let mut spent=0;let mut protected=Vec::new();
    loop {
        let mut response=client.query("SELECT * FROM $checkpoint").bind(("checkpoint",checkpoint.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native history checkpoint missing"));};
        if hash_field(row,"generation")?!=current.generation || int_field(row,"era")?!=current.era{return Err(ModelError::Conflict("native history checkpoint needs a fresh current-era invocation"));}
        let closed=int_field(row,"closed_through")?;
        if row.get("state")==Some(&Value::String("done".into())) || spent>=limit {
            let Some(Value::String(state))=row.get("state") else{return Err(ModelError::Schema("native history state"));};
            return Ok(HistoryCompaction{identity,closed_through:closed,removed_effects:u64::try_from(int_field(row,"removed_effects")?).map_err(ModelError::codec)?,removed_records:u64::try_from(int_field(row,"removed_records")?).map_err(ModelError::codec)?,state:state.clone(),protected});
        }
        let Some(Value::String(phase))=row.get("phase") else{return Err(ModelError::Schema("native history phase"));};
        let position=PHASES.iter().position(|(name,_)|name==phase).ok_or(ModelError::Schema("native history phase"))?;
        let after=row.get("after").cloned().unwrap_or(Value::None);
        // The primary record-id range is bounded even when the table holds live objects.
        // Eligibility and provenance checks occur against each exact candidate below.
        let selection=format!("SELECT VALUE id FROM {} LIMIT 1",keyset_source(PHASES[position].1,&after)?);
        let mut response=client.query(selection).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let candidates:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
        let mut bindings=Variables::new();bindings.insert("checkpoint",checkpoint.clone());bindings.insert("closed",closed);bindings.insert("after",after);bindings.insert("phase",phase.clone());bindings.insert("__generation",current.generation.hex());bindings.insert("__era",current.era);
        let validation="IF $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through<$closed { THROW 'native history horizon changed'; }; LET $progress=SELECT * FROM ONLY $checkpoint FOR UPDATE; IF $progress.era!=$__era OR ($progress.after ?? NONE)!=$after OR $progress.phase!=$phase OR $progress.state!='open' { THROW 'native history cursor changed'; };";
        if let Some(candidate)=candidates.first(){
            bindings.insert("candidate",candidate.clone());
            let eligibility=eligible(phase)?;
            let parent=if phase=="retirement_item" {"LET $parent_id=$candidate_state.job; LET $parent=SELECT * FROM ONLY $parent_id FOR UPDATE;"} else {"LET $parent=NONE;"};
            let generation=if phase=="retirement_item" {"$parent.generation=$__generation"} else {"$candidate_state.generation=$__generation"};
            let count=if phase=="effect"{"removed_effects+=1,removed_records+=1"}else{"removed_records+=1"};
            let sql=format!("{validation} LET $candidate_state=SELECT * FROM ONLY $candidate FOR UPDATE; {parent} IF $candidate_state!=NONE AND ({generation}) AND ({eligibility}) AND array::len(SELECT VALUE id FROM native_outcome_ref WITH INDEX object_outcome_refs WHERE object=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$candidate LIMIT 1)=0 AND array::len(SELECT VALUE id FROM native_effect WITH INDEX owner_effects WHERE owner=$candidate LIMIT 1)=0 {{ DELETE $candidate RETURN NONE; UPDATE $checkpoint SET {count} RETURN NONE; }}; UPDATE $checkpoint SET after=$candidate,revision+=1 RETURN NONE;");
            collection_page(client,identity,&sql,bindings).await?;
            let mut response=client.query("SELECT VALUE id FROM $candidate").bind(("candidate",candidate.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let remaining:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
            if !remaining.is_empty(){protected.push(candidate.clone());}spent+=1;
        } else {
            let next=PHASES.get(position+1).map(|(name,_)|*name);
            let update=if let Some(next)=next{bindings.insert("next",next);"UPDATE $checkpoint SET phase=$next,after=NONE,revision+=1 RETURN NONE"}else{"UPDATE $checkpoint SET state='done',revision+=1 RETURN NONE"};
            collection_page(client,identity,&format!("{validation} {update}"),bindings).await?;
            // Empty-phase transitions are small fixed setup/finalization costs (at most 8),
            // independent of the number of candidates or edges in the native universe.
        }
    }
}
// The checkpoint is the operation receipt. Recording another native_effect per candidate
// would replenish the very history this operation owns. Unknown results expose this exact
// checkpoint and can only retry its persisted original cursor under the same era.
async fn collection_page(client:&Surreal<Client>,identity:ContentHash,sql:&str,bindings:Variables)->Result<(),ModelError>{
    let query=format!("BEGIN; {} {sql}; COMMIT;",authorization::ERA_GUARD);
    match run_transaction(client,&query,bindings).await {
        Ok(())=>Ok(()),
        Err(error)=>{let mut completion=lctx_model::domain::completion::Completion::default();completion.committed("native history checkpoint",identity.hex());lctx_model::domain::completion::complete(Err(error),completion)}
    }
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
