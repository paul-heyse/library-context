//! Explicit drained v3 cutover. Legacy receipts and ambiguous work stay named and protected.
use super::*;
use authorization::{hash_field,int_field};
use std::collections::{BTreeMap, btree_map::Entry};

async fn rows_after(client:&Surreal<Client>,table:&str,after:Value)->Result<Vec<Object>,ModelError>{
    let mut response=client.query(format!("SELECT * FROM {} LIMIT 128",keyset_source(table,&after)?))
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    response.take(0).map_err(ModelError::codec)
}
pub(crate) async fn migrate_legacy_state(client:&Surreal<Client>,migration:ContentHash)->Result<(),ModelError>{
    let mut response=client.query("SELECT * FROM native_installation:current; SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state='open' LIMIT 1; SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1; SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1; SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false LIMIT 1")
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let installations:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let [installation]=installations.as_slice() else{return Err(ModelError::Conflict("native migration installation"));};
    if installation.get("admission_open")!=Some(&Value::Bool(false)){return Err(ModelError::Conflict("native migration admission open"));}
    for index in 1..5 {let live:Vec<RecordId>=response.take(index).map_err(ModelError::codec)?;if !live.is_empty(){return Err(ModelError::Conflict("native migration unresolved borrowers"));}}
    let generation=hash_field(installation,"generation")?;
    // The root owns the exact source hash/journal preflight and publishes the new marker last.
    // Repeat only these identity-preserving migration pages after an interrupted invocation.
    for table in ["native_attempt","native_effect","native_pin","native_backup_hold","native_guard","native_retirement","native_retirement_item"] {
        let mut after=Value::None;
        loop {
            let rows=rows_after(client,table,after.clone()).await?;if rows.is_empty(){break;}
            let mut ids=Vec::new();for row in &rows {let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native migration record identity"));};ids.push(id.clone());}
            after=Value::RecordId(ids.last().expect("nonempty migration page").clone());
            let mut bindings=Variables::new();bindings.insert("rows",ids);bindings.insert("generation",generation.hex());
            let update=match table {
                "native_attempt"=>"UPDATE $rows SET era=0 RETURN NONE;",
                "native_effect"=>"UPDATE $rows SET generation=$generation,era=0,owner=attempt,owner_kind='legacy' RETURN NONE;",
                "native_pin"=>"UPDATE $rows SET generation=$generation,era=0 RETURN NONE;",
                "native_backup_hold"=>"UPDATE $rows SET generation=$generation,era=0,epoch=(epoch ?? 0) RETURN NONE;",
                "native_guard"=>"UPDATE $rows SET phase=IF retired { 'retired' } ELSE { 'active' },incarnation=1,retiring_item=NONE RETURN NONE;",
                "native_retirement"=>"UPDATE $rows SET generation=$generation,era=0,epoch=0,root_digest=<string>id,state='legacy_protected',successor=NONE RETURN NONE;",
                "native_retirement_item"=>"UPDATE $rows SET incarnation=0,cutoff=NONE,successor=NONE RETURN NONE;",
                _=>unreachable!(),
            };
            let mut refs=Vec::new();
            if table!="native_guard" {
                for row in &rows {
                    let Some(Value::RecordId(id))=row.get("id") else{unreachable!()};
                    let mut reference=Object::new();reference.insert("id",RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(migration,id)).map_err(ModelError::codec)?).hex()));
                    reference.insert("object",id.clone());reference.insert("owner",format!("schema4-migration:{}",migration.hex()));reference.insert("kind","migration");refs.push(Value::Object(reference));
                }
            }
            bindings.insert("references",refs);
            run_transaction(client,&format!("BEGIN; {update} FOR $reference IN $references {{ UPSERT $reference.id CONTENT $reference RETURN NONE; }}; COMMIT;"),bindings).await?;
        }
    }
    // An old item's scope cannot establish a current incarnation. Preserve the exact
    // currently extant object behind a named legacy-job hold instead of authorizing it.
    let mut after=Value::None;
    loop {
        let items=rows_after(client,"native_retirement_item",after.clone()).await?;if items.is_empty(){break;}
        for item in items {
            after=item.get("id").cloned().ok_or(ModelError::Schema("legacy retirement item identity"))?;
            if item.get("state")==Some(&Value::String("done".into())){continue;}
            let (Some(Value::RecordId(job)),Some(Value::RecordId(object)))=(item.get("job"),item.get("object")) else{return Err(ModelError::Schema("legacy retirement remaining scope"));};
            let guard=guard_id(&Value::RecordId(object.clone()))?;
            let mut sink=KeySink::new("native-hold/v1");
            sink.part(b"owner",&serde_json::to_vec(job).map_err(ModelError::codec)?);sink.part(b"object",&serde_json::to_vec(object).map_err(ModelError::codec)?);
            let hold=RecordId::new("native_hold",sink.finish().hex());
            let mut bindings=Variables::new();bindings.insert("job",job.clone());bindings.insert("object",object.clone());bindings.insert("hold",hold);bindings.insert("guard",guard);
            run_transaction(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.admission_open { THROW 'native migration admission changed'; }; UPDATE native_installation:current SET control_revision=(control_revision ?? 0)+1 RETURN NONE; IF (SELECT VALUE id FROM ONLY $object)!=NONE { LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $before=NONE { CREATE $guard SET revision=0,retired=false,retired_through=0,phase='active',incarnation=1 RETURN NONE; }; IF $before.phase='retired' { THROW 'legacy retirement extant object marked retired'; }; UPSERT $hold SET owner=$job,object=$object RETURN NONE; }; COMMIT;",bindings).await?;
        }
    }
    // Recover only actual remaining owner relations from a sufficient terminal attempt.
    let mut after=Value::None;
    loop {
        let attempts=rows_after(client,"native_attempt",after.clone()).await?;if attempts.is_empty(){break;}
        for attempt in attempts {
            let Some(Value::RecordId(id))=attempt.get("id") else{return Err(ModelError::Schema("legacy cleanup attempt"));};after=Value::RecordId(id.clone());
            if !matches!(attempt.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"closed"|"abandoned"|"frozen"|"maintenance_fenced")){return Err(ModelError::Conflict("legacy cleanup origin not terminal"));}
            let epoch=int_field(&attempt,"epoch")?;if epoch<=0{return Err(ModelError::Conflict("legacy cleanup origin epoch"));}
            let unadmitted=attempt.get("admitted")==Some(&Value::Bool(false));
            let mut owners=vec![id.clone()];
            if unadmitted {
                let mut contribution_after=Value::None;
                loop {
                    let mut response=client.query("SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt AND ($after=NONE OR id>$after) ORDER BY id LIMIT 128")
                        .bind(("attempt",id.clone())).bind(("after",contribution_after.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                    let contributions:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;if contributions.is_empty(){break;}
                    contribution_after=Value::RecordId(contributions.last().expect("nonempty contribution page").clone());
                    for contribution in contributions {
                        let mut product_after=Value::None;
                        loop {
                            let mut response=client.query("SELECT VALUE id FROM native_product WITH INDEX contribution_products WHERE contribution=$contribution AND ($after=NONE OR id>$after) ORDER BY id LIMIT 128")
                                .bind(("contribution",contribution.clone())).bind(("after",product_after.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                            let products:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;if products.is_empty(){break;}
                            product_after=Value::RecordId(products.last().expect("nonempty product page").clone());
                            for owner in products { create_cleanup(client,migration,generation,id,epoch,true,&owner).await?; }
                        }
                    }
                }
            }
            for owner in owners.drain(..){create_cleanup(client,migration,generation,id,epoch,unadmitted,&owner).await?;}
        }
    }
    verify_legacy_state(client,migration,generation).await?;
    run_transaction(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.admission_open { THROW 'native migration admission changed'; }; UPDATE native_installation:current SET era=1,closed_through=0,history_inventory=NONE,control_revision=(control_revision ?? 0)+1 RETURN NONE; COMMIT;",Variables::new()).await
}
async fn create_cleanup(client:&Surreal<Client>,migration:ContentHash,generation:ContentHash,attempt:&RecordId,epoch:i64,unadmitted:bool,owner:&RecordId)->Result<(),ModelError>{
    let identity=ContentHash::of(&serde_json::to_vec(&("legacy-cleanup",migration,attempt,owner)).map_err(ModelError::codec)?);
    let mut bindings=Variables::new();bindings.insert("cleanup",RecordId::new("native_cleanup",identity.hex()));bindings.insert("attempt",attempt.clone());bindings.insert("owner",owner.clone());bindings.insert("generation",generation.hex());bindings.insert("epoch",epoch);bindings.insert("unadmitted",unadmitted);
    bindings.insert("reference",RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(migration,RecordId::new("native_cleanup",identity.hex()))).map_err(ModelError::codec)?).hex()));
    bindings.insert("consumer",format!("schema4-migration:{}",migration.hex()));
    run_transaction(client,"BEGIN; LET $existing=SELECT * FROM ONLY $cleanup FOR UPDATE; IF $existing=NONE AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner LIMIT 1)>0 { CREATE $cleanup SET attempt=$attempt,generation=$generation,era=0,epoch=$epoch,owners=[$owner],unadmitted=$unadmitted,state='open' RETURN NONE; }; IF (SELECT VALUE id FROM ONLY $cleanup)!=NONE { UPSERT $reference SET object=$cleanup,owner=$consumer,kind='migration' RETURN NONE; }; COMMIT;",bindings).await
}

async fn cleanup_attempt_origin<'a>(client:&Surreal<Client>,origins:&'a mut BTreeMap<RecordId,Object>,attempt:&RecordId)->Result<&'a Object,ModelError>{
    match origins.entry(attempt.clone()) {
        Entry::Occupied(origin)=>Ok(origin.into_mut()),
        Entry::Vacant(origin)=>{
            let mut response=client.query("SELECT * FROM $attempt").bind(("attempt",attempt.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let mut rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
            if rows.len()!=1{return Err(ModelError::Conflict("native migration cleanup origin missing"));}
            Ok(origin.insert(rows.pop().expect("one cleanup origin")))
        }
    }
}
async fn cleanup_contribution_origins<'a>(client:&Surreal<Client>,origins:&'a mut BTreeMap<RecordId,Vec<RecordId>>,contribution:&RecordId)->Result<&'a [RecordId],ModelError>{
    match origins.entry(contribution.clone()) {
        Entry::Occupied(origin)=>Ok(origin.into_mut()),
        Entry::Vacant(origin)=>{
            let mut response=client.query("SELECT VALUE attempt FROM $contribution").bind(("contribution",contribution.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let rows:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
            Ok(origin.insert(rows))
        }
    }
}
// Publication may only follow a verified exact translation, including every legacy result
// consumer. Verification uses bounded primary-key pages and exact relation probes.
async fn verify_legacy_state(client:&Surreal<Client>,migration:ContentHash,generation:ContentHash)->Result<(),ModelError>{
    for table in ["native_attempt","native_effect","native_pin","native_backup_hold","native_retirement","native_retirement_item","native_cleanup","native_guard"] {
        let mut after=Value::None;
        loop {
            let rows=rows_after(client,table,after.clone()).await?;if rows.is_empty(){break;}
            // Admission is closed throughout cutover, so these exact origins cannot change.
            // Retain only this page's first native observations, not translated row copies.
            let mut cleanup_attempts=BTreeMap::new();
            let mut contribution_origins=BTreeMap::new();
            let mut references=std::collections::BTreeMap::new();
            if table!="native_guard" {
                let ids=rows.iter().map(|row|{let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native migration verification identity"));};Ok(RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(migration,id)).map_err(ModelError::codec)?).hex()))}).collect::<Result<Vec<_>,ModelError>>()?;
                let mut response=client.query("SELECT * FROM $references").bind(("references",ids)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let refs:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
                for reference in refs {let Some(Value::RecordId(id))=reference.get("id") else{return Err(ModelError::Schema("native migration reference identity"));};if references.insert(id.clone(),reference).is_some(){return Err(ModelError::Conflict("native migration duplicate reference"));}}
            }
            for row in rows {
                let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native migration verification identity"));};after=Value::RecordId(id.clone());
                if table=="native_guard" {
                    if int_field(&row,"incarnation")?!=1 || row.get("retiring_item").is_some_and(|value|*value!=Value::None) || row.get("phase")!=Some(&Value::String(if row.get("retired")==Some(&Value::Bool(true)){"retired"}else{"active"}.into())){return Err(ModelError::Conflict("native migration guard translation"));}
                    continue;
                }
                if table!="native_retirement_item" && (int_field(&row,"era")?!=0 || hash_field(&row,"generation")?!=generation){return Err(ModelError::Conflict("native migration original issuance"));}
                let reference=RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(migration,id)).map_err(ModelError::codec)?).hex());
                let reference=references.get(&reference).ok_or(ModelError::Conflict("native migration outcome reference missing"))?;
                if reference.get("object")!=Some(&Value::RecordId(id.clone())) || reference.get("owner")!=Some(&Value::String(format!("schema4-migration:{}",migration.hex()))) || reference.get("kind")!=Some(&Value::String("migration".into())){return Err(ModelError::Conflict("native migration exact outcome consumer"));}
                if table=="native_retirement" && (row.get("state")!=Some(&Value::String("legacy_protected".into())) || int_field(&row,"epoch")?!=0){return Err(ModelError::Conflict("native migration ambiguous job executable"));}
                if table=="native_retirement_item" {
                    if int_field(&row,"incarnation")?!=0 || row.get("cutoff").is_some_and(|value|*value!=Value::None){return Err(ModelError::Conflict("native migration legacy incarnation borrowed"));}
                    if row.get("state")!=Some(&Value::String("done".into())) {
                        let (Some(Value::RecordId(job)),Some(Value::RecordId(object)))=(row.get("job"),row.get("object")) else{return Err(ModelError::Schema("native migration legacy scope"));};
                        let mut response=client.query("SELECT VALUE id FROM $object; SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object AND owner=$job LIMIT 1").bind(("object",object.clone())).bind(("job",job.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                        let extant:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;let held:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;
                        if !extant.is_empty() && held.is_empty(){return Err(ModelError::Conflict("native migration ambiguous scope unprotected"));}
                    }
                }
                if table=="native_cleanup" {
                    let Some(Value::RecordId(attempt))=row.get("attempt") else{return Err(ModelError::Schema("native migration cleanup origin"));};
                    let origin=cleanup_attempt_origin(client,&mut cleanup_attempts,attempt).await?;
                    if int_field(&row,"epoch")?<=0 || row.get("epoch")!=origin.get("epoch") || !matches!(origin.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"closed"|"abandoned"|"frozen"|"maintenance_fenced")) || (row.get("unadmitted")==Some(&Value::Bool(true)) && origin.get("admitted")!=Some(&Value::Bool(false))){return Err(ModelError::Conflict("native migration cleanup original branch"));}
                    let Some(Value::Array(owners))=row.get("owners") else{return Err(ModelError::Schema("native migration cleanup owners"));};
                    for owner in owners {
                        let Value::RecordId(owner)=owner else{return Err(ModelError::Schema("native migration cleanup owner identity"));};
                        if owner==attempt {continue;}
                        if owner.table.as_str()!="native_product" || row.get("unadmitted")!=Some(&Value::Bool(true)){return Err(ModelError::Conflict("native migration cleanup unrelated owner"));}
                        let mut response=client.query("SELECT contribution FROM $product").bind(("product",owner.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                        let products:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let [product]=products.as_slice() else{return Err(ModelError::Conflict("native migration cleanup product missing"));};
                        let Some(Value::RecordId(contribution))=product.get("contribution") else{return Err(ModelError::Schema("native migration product contribution"));};
                        let origins=cleanup_contribution_origins(client,&mut contribution_origins,contribution).await?;
                        if origins!=std::slice::from_ref(attempt){return Err(ModelError::Conflict("native migration cleanup product origin"));}
                    }
                }
            }
        }
    }
    Ok(())
}
