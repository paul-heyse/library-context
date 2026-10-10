//! A claimed incarnation excludes hold insertion/reactivation while bounded pages drain.
use super::*;
use authorization::{hash_field,int_field};
use lctx_model::domain::completed::RetirementProgress;

fn item_id(job:&RecordId,object:&RecordId,incarnation:i64)->Result<RecordId,ModelError>{
    Ok(RecordId::new("native_retirement_item",ContentHash::of(&serde_json::to_vec(&(job,object,incarnation)).map_err(ModelError::codec)?).hex()))
}
async fn record(client:&Surreal<Client>,id:&RecordId)->Result<Object,ModelError>{
    let mut response=client.query("SELECT * FROM $id").bind(("id",id.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native retirement identity missing"));};Ok(row.clone())
}
async fn incarnation(client:&Surreal<Client>,object:&RecordId)->Result<i64,ModelError>{
    let guard=guard_id(&Value::RecordId(object.clone()))?;
    let mut response=client.query("SELECT VALUE incarnation FROM $guard").bind(("guard",guard)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let values:Vec<i64>=response.take(0).map_err(ModelError::codec)?;
    match values.as_slice(){[]=>Ok(1),[value] if *value>0=>Ok(*value),_=>Err(ModelError::Conflict("native object incarnation"))}
}
async fn enqueue_roots(client:&Surreal<Client>,job:&RecordId,objects:&[RecordId],registered:i64)->Result<(),ModelError>{
    let mut rows=Vec::new();
    for object in objects {
        let current=incarnation(client,object).await?;
        let mut row=Object::new();row.insert("id",item_id(job,object,current)?);row.insert("object",object.clone());
        row.insert("guard",guard_id(&Value::RecordId(object.clone()))?);row.insert("incarnation",current);rows.push(Value::Object(row));
    }
    let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("rows",rows);bindings.insert("registered",registered);
    effect_for_owner(client,EffectOwner::RetirementSetup(job.clone()),"IF $__owner.state!='registering' OR $__owner.roots_registered!=$registered { THROW 'native retirement registration raced'; }; FOR $row IN $rows { LET $guard_id=$row.guard; LET $before=SELECT * FROM ONLY $guard_id FOR UPDATE; IF ($before.incarnation ?? 1)!=$row.incarnation { THROW 'native retirement root incarnation changed'; }; IF $before=NONE { CREATE $guard_id SET revision=0,retired=false,retired_through=0,phase='active',incarnation=1 RETURN NONE; }; LET $item_id=$row.id; LET $item=SELECT * FROM ONLY $item_id FOR UPDATE; IF $item=NONE { CREATE $item_id SET job=$job,object=$row.object,incarnation=$row.incarnation,state='pending' RETURN NONE; }; }; UPDATE $job SET roots_registered+=array::len($rows),revision+=1 RETURN NONE;",bindings).await
}
/// Register the requested roots in bounded non-destructive setup transactions, then spend
/// `limit` on destructive advancement: one claim/finalize unit or two writes per outgoing
/// edge (durable child nomination plus exact hold deletion). Root registration is operation
/// setup, not permission to remove more edges than this pass admits.
pub async fn retire_reachable(client:&Surreal<Client>,mut roots:Vec<RecordId>,limit:usize)->Result<RetirementProgress,ModelError>{
    if limit==0{return Err(ModelError::Invalid("bounded retirement effect limit".into()));}
    roots.sort();roots.dedup();
    let issuance=IssuanceEra::capture(client).await?;
    let identity=fresh_identity("retirement-invocation")?;
    let job=RecordId::new("native_retirement",identity.hex());
    let digest=ContentHash::of(&serde_json::to_vec(&("native-retirement/v4",&roots)).map_err(ModelError::codec)?);
    let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("root_digest",digest.hex());bindings.insert("issuance",issuance.era);bindings.insert("root_count",roots.len());
    effect(client,None,"IF $__era!=$issuance { THROW 'native retirement issuance changed'; }; CREATE $job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$root_digest,lineage=$job,state='registering',root_count=$root_count,roots_registered=0,examined=0,retired=0,revision=0 RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("retirement invocation",identity,issuance,error))?;
    let result=async {register_retirement_roots(client,identity,roots).await?;resume_retirement(client,identity,limit).await}.await;
    if result.is_err(){let mut completion=lctx_model::domain::completion::Completion::default();completion.committed("native retirement invocation",identity.hex());return lctx_model::domain::completion::complete(result,completion);}result
}

/// Retry only the exact original canonical root set. Until all bounded registration pages
/// commit, resume_retirement cannot claim or delete any object in the partial invocation.
pub async fn register_retirement_roots(client:&Surreal<Client>,identity:ContentHash,mut roots:Vec<RecordId>)->Result<(),ModelError>{
    roots.sort();roots.dedup();
    let job=RecordId::new("native_retirement",identity.hex());
    let row=record(client,&job).await?;
    let digest=ContentHash::of(&serde_json::to_vec(&("native-retirement/v4",&roots)).map_err(ModelError::codec)?);
    if hash_field(&row,"root_digest")?!=digest || usize::try_from(int_field(&row,"root_count")?).map_err(ModelError::codec)?!=roots.len(){return Err(ModelError::Conflict("native retirement original roots changed"));}
    if row.get("state")==Some(&Value::String("open".into())){return Ok(());}
    if row.get("state")!=Some(&Value::String("registering".into())){return Err(ModelError::Conflict("native retirement registration fenced"));}
    let mut registered=usize::try_from(int_field(&row,"roots_registered")?).map_err(ModelError::codec)?;
    if registered>roots.len(){return Err(ModelError::Schema("native retirement registration count"));}
    for window in roots[registered..].chunks(crate::loader::NATIVE_WINDOW_ROWS){enqueue_roots(client,&job,window,i64::try_from(registered).map_err(ModelError::codec)?).await?;registered+=window.len();}
    let mut bindings=Variables::new();bindings.insert("job",job.clone());
    effect_for_owner(client,EffectOwner::RetirementSetup(job),"IF $__owner.state!='registering' OR $__owner.roots_registered!=$__owner.root_count { THROW 'native retirement roots incomplete'; }; UPDATE $job SET state='open',revision+=1 RETURN NONE",bindings).await
}

async fn claim(client:&Surreal<Client>,job:&RecordId,item:&Object)->Result<(),ModelError>{
    let Some(Value::RecordId(object))=item.get("object") else{return Err(ModelError::Schema("retirement object"));};
    let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("item",item.get("id").cloned().ok_or(ModelError::Schema("retirement item"))?);
    bindings.insert("object",object.clone());bindings.insert("guard",guard_id(&Value::RecordId(object.clone()))?);
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"IF array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.object!=$object OR $queued.successor!=NONE OR $queued.state NOT IN ['pending','retained'] { THROW 'native retirement item not claimable'; }; IF $before.incarnation!=$queued.incarnation { UPDATE $item SET state='superseded' RETURN NONE; } ELSE { IF $before.phase='retiring' OR array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0 { UPDATE $item SET state='retained' RETURN NONE; UPDATE $job SET examined+=1,revision+=1 RETURN NONE; } ELSE { UPDATE $guard SET phase='retiring',retiring_item=$item,revision+=1 RETURN NONE; UPDATE $item SET state='retiring',cutoff=$__installation.admission_revision RETURN NONE; UPDATE $job SET revision+=1 RETURN NONE; }; };",bindings).await
}

async fn outgoing_page(client:&Surreal<Client>,object:&RecordId,limit:usize)->Result<Vec<Object>,ModelError>{
    let mut response=client.query("SELECT id,owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$object ORDER BY object LIMIT $limit")
        .bind(("object",object.clone())).bind(("limit",limit)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    response.take(0).map_err(ModelError::codec)
}
async fn page(client:&Surreal<Client>,job:&RecordId,item:&Object,holds:Vec<Object>)->Result<(),ModelError>{
    let Some(Value::RecordId(object))=item.get("object") else{return Err(ModelError::Schema("retirement object"));};
    let job_row=record(client,job).await?;
    let lineage=match job_row.get("lineage"){Some(Value::RecordId(id))=>id.clone(),_=>job.clone()};
    let mut rows=Vec::new();
    for hold in holds {
        let Some(Value::RecordId(child))=hold.get("object") else{return Err(ModelError::Schema("retirement child"));};
        let incarnation=incarnation(client,child).await?;
        let mut row=hold.clone();row.insert("item",item_id(&lineage,child,incarnation)?);
        row.insert("guard",guard_id(&Value::RecordId(child.clone()))?);row.insert("incarnation",incarnation);rows.push(Value::Object(row));
    }
    let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("item",item.get("id").cloned().ok_or(ModelError::Schema("retirement item"))?);
    bindings.insert("object",object.clone());bindings.insert("guard",guard_id(&Value::RecordId(object.clone()))?);bindings.insert("rows",rows);
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"IF array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$item OR $before.incarnation!=$queued.incarnation { THROW 'native retirement page incarnation fenced'; }; FOR $row IN $rows { LET $hold_id=$row.id; LET $hold=SELECT * FROM ONLY $hold_id FOR UPDATE; IF $hold=NONE OR $hold.owner!=$object OR $hold.object!=$row.object { THROW 'native retirement page ownership changed'; }; LET $child_guard=$row.guard; LET $child=SELECT * FROM ONLY $child_guard FOR UPDATE; IF ($child.incarnation ?? 1)!=$row.incarnation { THROW 'native retirement child incarnation changed'; }; IF $child=NONE { CREATE $child_guard SET revision=0,retired=false,retired_through=0,phase='active',incarnation=1 RETURN NONE; }; LET $child_item=$row.item; LET $existing=SELECT * FROM ONLY $child_item FOR UPDATE; IF $existing=NONE { CREATE $child_item SET job=$job,object=$row.object,incarnation=$row.incarnation,state='pending' RETURN NONE; }; DELETE $hold_id RETURN NONE; }; UPDATE $job SET revision+=1 RETURN NONE;",bindings).await
}
async fn finalize(client:&Surreal<Client>,job:&RecordId,item:&Object)->Result<(),ModelError>{
    let Some(Value::RecordId(object))=item.get("object") else{return Err(ModelError::Schema("retirement object"));};
    let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("item",item.get("id").cloned().ok_or(ModelError::Schema("retirement item"))?);
    bindings.insert("object",object.clone());bindings.insert("guard",guard_id(&Value::RecordId(object.clone()))?);
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"IF array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$item OR $before.incarnation!=$queued.incarnation OR $queued.cutoff=NONE { THROW 'native retirement finalize incarnation fenced'; }; IF array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0 { THROW 'native retirement final holds remain'; }; DELETE $object RETURN NONE; UPDATE $guard SET phase='retired',retired=true,retiring_item=NONE,retired_through=math::max([retired_through ?? 0,$queued.cutoff]),revision+=1 RETURN NONE; UPDATE $item SET state='done' RETURN NONE; UPDATE $job SET examined+=1,retired+=1,revision+=1 RETURN NONE;",bindings).await
}
async fn progress(client:&Surreal<Client>,identity:ContentHash)->Result<RetirementProgress,ModelError>{
    let job=RecordId::new("native_retirement",identity.hex());
    let mut response=client.query("SELECT examined,retired,state FROM $job; SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state IN ['pending','retiring'] ORDER BY id LIMIT 128; SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retained' ORDER BY id LIMIT 128")
        .bind(("job",job)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native retirement progress"));};
    let identities=|values:Vec<RecordId>|values.into_iter().map(|value|serde_json::to_string(&value).map_err(ModelError::codec)).collect::<Result<Vec<_>,_>>();
    let remaining:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;
    let retained:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;
    if remaining.is_empty() && retained.is_empty() && row.get("state")==Some(&Value::String("open".into())) {
        let job=RecordId::new("native_retirement",identity.hex());let mut vars=Variables::new();vars.insert("job",job.clone());
        effect_for_owner(client,EffectOwner::Retirement(job),"IF array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state IN ['pending','retained','retiring'] LIMIT 1)>0 { THROW 'retirement completion raced'; }; UPDATE $job SET state='done',revision+=1 RETURN NONE",vars).await?;
    }
    Ok(RetirementProgress{identity,examined:u64::try_from(int_field(row,"examined")?).map_err(ModelError::codec)?,retired:u64::try_from(int_field(row,"retired")?).map_err(ModelError::codec)?,remaining:identities(remaining)?,retained:identities(retained)?})
}
pub async fn resume_retirement(client:&Surreal<Client>,identity:ContentHash,limit:usize)->Result<RetirementProgress,ModelError>{
    if limit==0{return Err(ModelError::Invalid("bounded retirement effect limit".into()));}
    let job=RecordId::new("native_retirement",identity.hex());
    let mut row=record(client,&job).await?;
    if row.get("state")==Some(&Value::String("recovering".into())) {
        finish_recovery(client,&job).await?;row=record(client,&job).await?;
    }
    if row.get("state")==Some(&Value::String("registering".into())){return Err(ModelError::Conflict("native retirement registration incomplete; retry exact roots"));}
    if row.get("state")==Some(&Value::String("done".into())) { return progress(client,identity).await; }
    let current=IssuanceEra::capture(client).await?;
    if hash_field(&row,"generation")?!=current.generation || int_field(&row,"era")?!=current.era || row.get("state")!=Some(&Value::String("open".into())) {
        return Err(ModelError::Conflict("native retirement needs explicit maintenance successor"));
    }
    let mut response=client.query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND owner=$job LIMIT 1").bind(("job",job.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let pending:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
    if !pending.is_empty(){return Err(ModelError::Conflict("native retirement previous page unresolved"));}
    let mut spent=0usize;let mut after=Value::None;
    while spent<limit {
        let mut response=client.query("SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state IN ['pending','retained','retiring'] AND ($after=NONE OR id>$after) ORDER BY id LIMIT 1")
            .bind(("job",job.clone())).bind(("after",after.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let Some(item)=rows.first() else{break;};
        let Some(Value::RecordId(id))=item.get("id") else{return Err(ModelError::Schema("retirement item identity"));};
        after=Value::RecordId(id.clone());
        let mut item=item.clone();
        if item.get("state")!=Some(&Value::String("retiring".into())){
            claim(client,&job,&item).await?;spent+=1;item=record(client,id).await?;
        }
        if item.get("state")!=Some(&Value::String("retiring".into())){continue;}
        let Some(Value::RecordId(object))=item.get("object") else{return Err(ModelError::Schema("retirement object"));};
        loop {
            if spent>=limit{break;}
            let allowance=((limit-spent)/2).min(crate::loader::NATIVE_WINDOW_ROWS);
            // One edge unit comprises a durable child nomination and one exact hold deletion.
            let holds=outgoing_page(client,object,allowance.max(1)).await?;
            if holds.is_empty(){finalize(client,&job,&item).await?;spent+=1;break;}
            if allowance==0{break;}
            spent+=holds.len()*2;page(client,&job,&item,holds).await?;
        }
    }
    progress(client,identity).await
}
pub async fn retire(client:&Surreal<Client>,object:RecordId)->Result<(),ModelError>{
    let mut progress=retire_reachable(client,vec![object],crate::loader::NATIVE_WINDOW_ROWS).await?;
    loop {
        if !progress.retained.is_empty(){return Err(ModelError::Conflict("native retirement reachable"));}
        if progress.remaining.is_empty(){return Ok(());}
        progress=resume_retirement(client,progress.identity,crate::loader::NATIVE_WINDOW_ROWS).await?;
    }
}
/// Claim only durable remaining items after the old era is permanently closed. An old
/// invocation cannot regain authority over a reactivated object or reset a done item.
pub async fn recover_retirement(client:&Surreal<Client>,identity:ContentHash)->Result<ContentHash,ModelError>{
    let issuance=IssuanceEra::capture(client).await?;
    let old=RecordId::new("native_retirement",identity.hex());
    let before=record(client,&old).await?;
    let existing=match before.get("successor"){Some(Value::RecordId(id))=>Some(id.clone()),_=>None};
    let next=if let Some(id)=&existing {
        let row=record(client,id).await?;
        if hash_field(&row,"generation")?!=issuance.generation || int_field(&row,"era")?!=issuance.era || !matches!(row.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"open"|"recovering")) {return Err(ModelError::Conflict("recover named retirement successor first"));}
        hash_record(id)?
    } else {fresh_identity("retirement-maintenance-successor")?};
    let successor=RecordId::new("native_retirement",next.hex());
    let mut bindings=Variables::new();bindings.insert("old",old.clone());bindings.insert("next",successor.clone());bindings.insert("era",issuance.era);
    if existing.is_none() { effect(client,None,"IF $__era!=$era OR $__installation.admission_open { THROW 'retirement recovery requires original current-era drained maintenance'; }; LET $before=SELECT * FROM ONLY $old FOR UPDATE; IF $before=NONE OR $before.state NOT IN ['open','recovering'] OR $before.generation!=$__generation OR $before.epoch<=0 OR $before.successor!=NONE OR $before.era>($__installation.closed_through ?? 0) { THROW 'retirement predecessor not fenced'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 { THROW 'retirement recovery has unresolved effects'; }; CREATE $next SET generation=$__generation,era=$era,epoch=$__epoch,root_digest=$before.root_digest,lineage=$before.lineage,state='recovering',predecessor=$old,recovery_source=$old,root_count=$before.root_count,roots_registered=$before.roots_registered,examined=$before.examined,retired=$before.retired,revision=0 RETURN NONE; UPDATE $old SET successor=$next,state='superseded' RETURN NONE",bindings).await?; }
    finish_recovery(client,&successor).await.map_err(|error|authorization::lifecycle_error("retirement successor",next,issuance,error))?;
    Ok(next)
}
async fn finish_recovery(client:&Surreal<Client>,successor:&RecordId)->Result<(),ModelError>{
    // Walk the durable predecessor chain one source at a time. A later era cut may fence
    // a half-rehomed successor; its successor still sees every older remaining source.
    loop {
        let row=record(client,successor).await?;
        if row.get("state")==Some(&Value::String("open".into())){return Ok(());}
        if row.get("state")!=Some(&Value::String("recovering".into())){return Err(ModelError::Conflict("native retirement recovery fenced"));}
        let Some(Value::RecordId(source))=row.get("recovery_source") else{return Err(ModelError::Schema("native retirement recovery source"));};
        let mut response=client.query("SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$source AND state IN ['pending','retained','retiring'] LIMIT 128").bind(("source",source.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let items:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
        let mut bindings=Variables::new();bindings.insert("source",source.clone());bindings.insert("next",successor.clone());bindings.insert("items",items);
        effect_for_owner(client,EffectOwner::RetirementSetup(successor.clone()),"IF $__owner.state!='recovering' OR $__owner.recovery_source!=$source { THROW 'native retirement recovery source raced'; }; LET $origin=SELECT * FROM ONLY $source FOR UPDATE; IF $origin=NONE OR $origin.state!='superseded' { THROW 'native retirement recovery source not fenced'; }; FOR $id IN $items { LET $item=SELECT * FROM ONLY $id FOR UPDATE; IF $item.job=$source AND $item.state IN ['pending','retained','retiring'] { UPDATE $id SET job=$next RETURN NONE; }; }; IF array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$source AND state IN ['pending','retained','retiring'] LIMIT 1)=0 { IF $origin.predecessor=NONE { UPDATE $next SET state='open',recovery_source=NONE,revision+=1 RETURN NONE; } ELSE { UPDATE $next SET recovery_source=$origin.predecessor,revision+=1 RETURN NONE; }; };",bindings).await?;
    }
}

fn hash_record(id:&RecordId)->Result<ContentHash,ModelError>{
    let surrealdb::types::RecordIdKey::String(key)=&id.key else{return Err(ModelError::Schema("retirement successor identity"));};
    Ok(ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("retirement successor width"))?))
}
