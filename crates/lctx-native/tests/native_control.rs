//! Focused controls for the shared content, exact-view and durable ownership cutover.
use lctx_model::domain::{
    ContentHash, FiniteF64, ModelError, Record, Relation,
    admission::Frontier,
    analysis::sources::SourceSnapshot,
    analytics::QualityStep,
    completed::{CompletedBinding, ContributionSpec},
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_native::{RuntimeConfig, control};
use lctx_compiler_store::compiler::NativeCompilerStore;
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::{Object, RecordId, Value, Variables};
fn config() -> RuntimeConfig {
    RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap()
}

// Build one deliberately explicit test request from the actual persisted guard. This
// uses the production request/digest boundary, rather than a hand-written legacy epoch.
async fn captured_guard_request(
    client: &surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,
    object: &RecordId,
    sql: &str,
    mut bindings: Variables,
) -> (control::NativeRequest, Variables) {
    let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex());
    let mut response=client.query("SELECT * FROM $guard").bind(("guard",guard.clone())).await.unwrap().check().unwrap();
    let rows:Vec<Object>=response.take(0).unwrap();
    let mut expected=Object::new();expected.insert("record",object.clone());expected.insert("guard",guard.clone());
    if let Some(row)=rows.first(){
        expected.insert("incarnation",row.get("incarnation").unwrap().clone());
        expected.insert("phase",row.get("phase").unwrap().clone());
        expected.insert("tombstone_revision",if row.get("phase")==Some(&Value::String("retired".into())) {row.get("tombstone_revision").unwrap().clone()}else{Value::None});
    }else{expected.insert("incarnation",0i64);expected.insert("phase","vacant");expected.insert("tombstone_revision",0i64);}
    let mut mapping=Object::new();mapping.insert("record",object.clone());mapping.insert("guard",guard.clone());
    bindings.insert("__write_guards",vec![guard]);bindings.insert("__guard_records",vec![Value::Object(mapping)]);
    bindings.insert("__object_expectations",vec![Value::Object(expected)]);
    let digest=ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).unwrap());
    (control::NativeRequest::issue(client,digest).await.unwrap(),bindings)
}

#[tokio::test(flavor = "multi_thread")]
async fn phased_retirement_charges_actual_edges_and_fences_both_hold_ends_until_finalization() {
    let cfg=config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let nonce=control::fresh_identity("phased-retirement-control").unwrap();
    let parent=RecordId::new("native_guard",format!("phased_parent_{}",nonce.hex()));
    let other=RecordId::new("native_guard",format!("phased_other_{}",nonce.hex()));
    let children=(0..17).map(|n|RecordId::new("native_guard",format!("phased_child_{}_{n}",nonce.hex()))).collect::<Vec<_>>();
    let values=std::iter::once(parent.clone()).chain(std::iter::once(other.clone())).chain(children.iter().cloned()).map(|id|{
        let mut row=Object::new();row.insert("id",id);row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)
    }).collect::<Vec<_>>();
    control::ensure_rows(&client,None,values.clone()).await.unwrap();
    control::hold(&client,None,parent.clone(),children.clone()).await.unwrap();
    let first=control::retire_reachable(&client,vec![parent.clone()],3).await.unwrap();
    assert_eq!(first.retired,0,"claim plus one nominated/deleted edge cannot finalize a high-degree parent");
    let mut response=client.query("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$parent; SELECT VALUE id FROM $parent")
        .bind(("parent",parent.clone())).await.unwrap().check().unwrap();
    let holds:Vec<RecordId>=response.take(0).unwrap();let payload:Vec<RecordId>=response.take(1).unwrap();
    assert_eq!(holds.len(),16,"effect budget includes child nomination plus exact hold deletion");
    assert_eq!(payload,[parent.clone()]);
    assert!(control::hold(&client,None,other.clone(),vec![parent.clone()]).await.is_err());
    assert!(control::hold(&client,None,parent.clone(),vec![other.clone()]).await.is_err());
    assert!(control::ensure_rows(&client,None,vec![values[0].clone()]).await.is_err());
    let mut late=Variables::new();late.insert("row",values[0].clone());
    assert!(control::guarded_effect(&client,None,vec![parent.clone()],"UPSERT $row.id CONTENT $row RETURN NONE",late).await.is_err(),"mutable compiler ingress must guard before its payload write");
    client.invalidate().await.unwrap();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let mut progress=first;
    while !progress.remaining.is_empty(){progress=control::resume_retirement(&client,progress.identity,16).await.unwrap();}
    assert_eq!(progress.retired,18);
    assert!(progress.retained.is_empty());
    let old=progress.identity;
    control::ensure_rows(&client,None,vec![values[0].clone()]).await.unwrap();
    let old_done=control::resume_retirement(&client,old,16).await.unwrap();
    assert_eq!(old_done.retired,18,"done items never become pending after content reactivation");
    let mut response=client.query("SELECT VALUE id FROM $parent").bind(("parent",parent.clone())).await.unwrap().check().unwrap();
    let payload:Vec<RecordId>=response.take(0).unwrap();assert_eq!(payload,[parent.clone()]);
    let fresh=control::retire_reachable(&client,vec![parent],4).await.unwrap();
    assert_ne!(fresh.identity,old);assert_eq!(fresh.retired,1);
    control::retire(&client,other).await.unwrap();client.invalidate().await.unwrap();
}


#[tokio::test(flavor="multi_thread")]
async fn retirement_high_degree_parent_spans_the_actual_128_edge_window(){
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let nonce=control::fresh_identity("retirement-129-edge-control").unwrap();
    let parent=RecordId::new("native_guard",format!("window_parent_{}",nonce.hex()));
    let children=(0..129).map(|index|RecordId::new("native_guard",format!("window_child_{}_{index:03}",nonce.hex()))).collect::<Vec<_>>();
    let values=std::iter::once(parent.clone()).chain(children.iter().cloned()).map(|id|{
        let mut row=Object::new();row.insert("id",id);row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)
    }).collect();
    control::ensure_rows(&client,None,values).await.unwrap();
    control::hold(&client,None,parent.clone(),children.clone()).await.unwrap();
    // Claim only the parent. The next pass has exactly one queued object before
    // nomination, so its first outgoing range must cross the physical row window.
    let first=control::retire_reachable(&client,vec![parent.clone()],1).await.unwrap();
    assert_eq!(first.retired,0);
    let progress=control::resume_retirement(&client,first.identity,257).await.unwrap();
    assert_eq!(progress.retired,0,"128 nominations plus 128 hold deletions exhaust the pass before finalization");
    let job=RecordId::new("native_retirement",first.identity.hex());
    let mut response=client.query("SELECT owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$parent; SELECT VALUE id FROM $payloads; SELECT id,object,state FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job")
        .bind(("parent",parent.clone())).bind(("payloads",std::iter::once(parent.clone()).chain(children.iter().cloned()).collect::<Vec<_>>())).bind(("job",job)).await.unwrap().check().unwrap();
    let holds:Vec<Object>=response.take(0).unwrap();let payloads:Vec<RecordId>=response.take(1).unwrap();let items:Vec<Object>=response.take(2).unwrap();
    assert_eq!(holds.len(),1,"the first actual batch admits at most 128 of 129 outgoing edges");
    assert_eq!(holds[0].get("owner"),Some(&Value::RecordId(parent.clone())));
    assert_eq!(holds[0].get("object"),Some(&Value::RecordId(children[128].clone())),"the edge after the bounded ordered range remains owned");
    assert_eq!(payloads.len(),130,"the remaining edge keeps the parent and every nominated child payload alive");
    assert_eq!(items.len(),129,"one root plus exactly 128 durable child nominations");
    client.invalidate().await.unwrap();
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let mut progress=control::resume_retirement(&client,first.identity,512).await.unwrap();
    while !progress.remaining.is_empty(){progress=control::resume_retirement(&client,first.identity,512).await.unwrap();}
    assert_eq!(progress.retired,130);assert!(progress.retained.is_empty());
    let mut response=client.query("SELECT VALUE id FROM $payloads; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$parent")
        .bind(("payloads",std::iter::once(parent.clone()).chain(children).collect::<Vec<_>>())).bind(("parent",parent)).await.unwrap().check().unwrap();
    let payloads:Vec<RecordId>=response.take(0).unwrap();let holds:Vec<RecordId>=response.take(1).unwrap();
    assert!(payloads.is_empty() && holds.is_empty(),"reconnected continuation retires the complete owned graph");
    client.invalidate().await.unwrap();
}

#[tokio::test(flavor="multi_thread")]
async fn named_retirement_continues_exact_roots_prioritizes_pending_and_replays_terminal_job(){
    use futures::FutureExt;
    use lctx_model::domain::completion::{Completion,LocalState,RemoteState,complete};
    fn require(value:bool,message:&'static str)->Result<(),ModelError>{
        if value {Ok(())} else {Err(ModelError::Conflict(message))}
    }
    fn item(job:&RecordId,object:&RecordId)->RecordId{
        RecordId::new("native_retirement_item",ContentHash::of(&serde_json::to_vec(&(job,object,1i64)).unwrap()).hex())
    }
    async fn snapshot(client:&surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,job:&RecordId,objects:&[RecordId])->Result<Vec<Vec<Object>>,ModelError>{
        let mut response=client.query("SELECT * FROM $job; SELECT * FROM $objects ORDER BY id; SELECT * FROM native_hold WITH INDEX object_holds WHERE object IN $objects ORDER BY id; SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job ORDER BY id")
            .bind(("job",job.clone())).bind(("objects",objects.to_vec())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        (0..4).map(|index|response.take::<Vec<Object>>(index).map_err(ModelError::codec)).collect()
    }
    let nonce=control::fresh_identity("named-retirement-control").unwrap();
    // The caller chooses and keeps this address before the first native effect.
    let identity=control::fresh_identity("named-retirement-job").unwrap();
    let job=RecordId::new("native_retirement",identity.hex());
    // Arrange the physical queue so an earlier retained item would starve the
    // pending parent under an undifferentiated LIMIT 1 traversal.
    let mut candidates=[0,1].map(|index|RecordId::new("native_guard",format!("named_root_{}_{index}",nonce.hex())));
    candidates.sort_by_key(|object|item(&job,object));
    let [shared,parent]=candidates;
    let leaf=RecordId::new("native_guard",format!("named_leaf_{}",nonce.hex()));
    let refused=RecordId::new("native_guard",format!("named_unrelated_{}",nonce.hex()));
    let objects=vec![parent.clone(),shared.clone(),leaf.clone(),refused.clone()];
    let roots=vec![parent.clone(),shared.clone()];
    let cfg=config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let session=lctx_native::reader::SessionFinalizationGuard::new(client.clone());
    let mut pin=None;
    let outcome=std::panic::AssertUnwindSafe(async {
        let values=objects.iter().cloned().map(|id|{
            let mut row=Object::new();row.insert("id",id);row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)
        }).collect();
        control::ensure_rows(&client,None,values).await?;
        pin=Some(control::ReaderPin::acquire(client.clone(),&[]).await?);
        pin.as_ref().unwrap().protect(shared.clone()).await?;
        control::hold(&client,None,parent.clone(),vec![shared.clone(),leaf.clone()]).await?;
        let first=control::retire_reachable_named(&client,identity,cfg.service_generation,roots.clone(),1).await?;
        let shared_key=serde_json::to_string(&shared).map_err(ModelError::codec)?;
        require(first.identity==identity && first.retired==0 && first.retained==[shared_key.clone()] && !first.remaining.is_empty(),"first named pass must retain the shared root and leave pending work")?;
        let before=snapshot(&client,&job,&objects).await?;
        let refusal=control::retire_reachable_named(&client,identity,cfg.service_generation,vec![refused.clone()],1).await;
        let error=match refusal {Err(error)=>error,Ok(_)=>return Err(ModelError::Conflict("changed named roots must refuse"))};
        if !error.permits_storage_cleanup(){return Err(error);}
        require(error.to_string().contains("native retirement exact invocation changed"),"changed roots must fail the exact native invocation guard")?;
        require(snapshot(&client,&job,&objects).await?==before,"changed-root refusal must preserve the original job, all payloads and holds")?;
        // Reverse root order too: the contract is the exact canonical set.
        let second=control::retire_reachable_named(&client,identity,cfg.service_generation,vec![shared.clone(),parent.clone()],1).await?;
        require(second.identity==identity && second.retired==0,"same named job must continue rather than fork")?;
        let after=snapshot(&client,&job,&objects).await?;
        require(after[3].iter().any(|row|row.get("object")==Some(&Value::RecordId(parent.clone())) && row.get("state")==Some(&Value::String("retiring".into()))),"pending parent must advance ahead of earlier retained shared item")?;
        let mut progress=second;
        for _ in 0..4 {
            if progress.remaining.is_empty(){break;}
            progress=control::retire_reachable_named(&client,identity,cfg.service_generation,roots.clone(),5).await?;
        }
        require(progress.remaining.is_empty() && progress.retired==2 && progress.retained==[shared_key],"bounded continuation must retire parent and leaf while preserving independent shared content")?;
        let held=snapshot(&client,&job,&objects).await?;
        let pin_id=RecordId::new("native_pin",pin.as_ref().unwrap().identity.hex());
        require(held[1].len()==2 && held[1].iter().any(|row|row.get("id")==Some(&Value::RecordId(shared.clone()))) && held[1].iter().any(|row|row.get("id")==Some(&Value::RecordId(refused.clone()))),"shared and unrelated payloads must remain")?;
        require(held[2].len()==1 && held[2][0].get("owner")==Some(&Value::RecordId(pin_id)) && held[2][0].get("object")==Some(&Value::RecordId(shared.clone())),"independent pin hold must survive retirement traversal")?;
        pin.as_ref().unwrap().release().await?;
        let terminal=control::retire_reachable_named(&client,identity,cfg.service_generation,roots.clone(),2).await?;
        require(terminal.identity==identity && terminal.retired==3 && terminal.remaining.is_empty() && terminal.retained.is_empty(),"same job must become terminal after independent owner releases")?;
        let done=snapshot(&client,&job,&objects).await?;
        require(done[0].len()==1 && done[0][0].get("state")==Some(&Value::String("done".into())),"completed named job must persist terminal state")?;
        let replay=control::retire_reachable_named(&client,identity,cfg.service_generation,roots.clone(),1).await?;
        require(replay.identity==identity && replay.retired==terminal.retired && replay.examined==terminal.examined && replay.remaining.is_empty() && replay.retained.is_empty(),"terminal named replay must retain the original result")?;
        require(snapshot(&client,&job,&objects).await?==done,"terminal replay must not reset items, rewrite the job or touch unrelated payloads")
    }).catch_unwind().await;
    let cleanup_allowed=matches!(&outcome,Ok(result) if result.as_ref().err().is_none_or(ModelError::permits_storage_cleanup));
    let mut completion=Completion::default();
    if cleanup_allowed {
        let released=if let Some(pin)=&pin {pin.release().await} else {Ok(())};
        let release_ok=released.is_ok();completion.step("named retirement control independent pin release",released);
        if release_ok {
            let cleanup=async {
                let mut progress=control::retire_reachable_named(&client,identity,cfg.service_generation,roots,16).await?;
                for _ in 0..4 {if progress.remaining.is_empty(){break;}progress=control::resume_retirement(&client,identity,16).await?;}
                require(progress.remaining.is_empty() && progress.retained.is_empty(),"owned named retirement cleanup must finish")?;
                control::retire(&client,refused).await
            }.await;
            let safe=cleanup.as_ref().err().is_none_or(ModelError::permits_storage_cleanup);
            if cleanup.is_err(){completion.local=LocalState::Outstanding;}
            completion.step("named retirement control exact roots cleanup",cleanup);
            if safe {completion.step("named retirement control session invalidation",session.close().await);}
            else {completion.local=LocalState::Outstanding;completion.remote=RemoteState::Unknown;}
        } else {if let Some(pin)=&pin {pin.retain_unknown();}completion.local=LocalState::Outstanding;completion.remote=RemoteState::Unknown;}
    } else {
        if let Some(pin)=&pin {pin.retain_unknown();}
        completion.local=LocalState::Outstanding;completion.remote=RemoteState::Unknown;
    }
    match outcome {
        Ok(result)=>complete(result,completion).unwrap(),
        Err(panic)=>{let _=complete(Ok(()),completion);std::panic::resume_unwind(panic);}
    }
}

#[test]
fn original_native_request_serialization_retains_issuance_and_exact_payload_digest() {
    let original=control::NativeRequest{issuance:control::IssuanceEra{generation:ContentHash::of(b"service"),era:7},operation:ContentHash::of(b"original-operation"),request:ContentHash::of(b"exact-payload")};
    let encoded=serde_json::to_vec(&original).unwrap();
    let retry:control::NativeRequest=serde_json::from_slice(&encoded).unwrap();
    assert_eq!(retry,original);
    let changed=control::NativeRequest{issuance:control::IssuanceEra{era:8,..original.issuance},..original};
    assert_ne!(changed,original,"a successor must be explicit; retry cannot borrow its era");
}

#[tokio::test(flavor = "multi_thread")]
async fn unresolved_epoch_zero_effect_cannot_execute_and_retains_original_disposition() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let nonce=control::fresh_identity("zero-epoch-control").unwrap();
    let object=RecordId::new("native_guard",format!("zero_epoch_{}",nonce.hex()));
    let sql="CREATE $object SET revision=0,retired=false,phase='active',incarnation=1 RETURN NONE";
    let mut bindings=Variables::new();bindings.insert("object",object.clone());
    let request=control::NativeRequest::issue(&client,ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).unwrap())).await.unwrap();
    client.query("CREATE $effect SET attempt=NONE,owner=NONE,owner_kind='installation',generation=$generation,era=$era,request=$digest,committed=false,resolved=false,revision=0,epoch=0 RETURN NONE")
        .bind(("effect",RecordId::new("native_effect",request.operation.hex()))).bind(("generation",request.issuance.generation.hex())).bind(("era",request.issuance.era)).bind(("digest",request.request.hex())).await.unwrap().check().unwrap();
    assert!(control::execute_request(&client,request,None,sql,bindings).await.is_err());
    assert_eq!(control::request_disposition(&client,request).await.unwrap(),control::EffectDisposition::FencedUncommitted);
    let mut response=client.query("SELECT VALUE id FROM $object").bind(("object",object)).await.unwrap().check().unwrap();
    let rows:Vec<RecordId>=response.take(0).unwrap();assert!(rows.is_empty());
    client.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_outcome_references_are_idempotent_and_release_only_the_named_consumer() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let sql="RETURN NONE";let bindings=Variables::new();
    let request=control::NativeRequest::issue(&client,ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).unwrap())).await.unwrap();
    control::execute_request(&client,request,None,sql,bindings).await.unwrap();
    let target=RecordId::new("native_effect",request.operation.hex());
    let one=control::retain_native_outcome(&client,target.clone(),"native-control-reader","evidence").await.unwrap();
    let repeat=control::retain_native_outcome(&client,target.clone(),"native-control-reader","evidence").await.unwrap();
    let two=control::retain_native_outcome(&client,target.clone(),"native-control-recovery","recovery").await.unwrap();
    assert_eq!(one,repeat);assert_ne!(one,two);
    assert!(control::release_outcome_reference(&client,one.clone(),"wrong-consumer","evidence").await.is_err());
    control::release_outcome_reference(&client,one.clone(),"native-control-reader","evidence").await.unwrap();
    control::release_outcome_reference(&client,one,"native-control-reader","evidence").await.unwrap();
    let mut response=client.query("SELECT VALUE id FROM native_outcome_ref WITH INDEX object_outcome_refs WHERE object=$object").bind(("object",target)).await.unwrap().check().unwrap();
    let refs:Vec<RecordId>=response.take(0).unwrap();assert_eq!(refs,[two.clone()]);
    assert_eq!(control::request_disposition(&client,request).await.unwrap(),control::EffectDisposition::Committed);
    control::release_outcome_reference(&client,two,"native-control-recovery","recovery").await.unwrap();
    control::release_outcome(&client,request,&request.operation.hex(),"caller").await.unwrap();
    client.invalidate().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn indexed_live_control_probes_select_declared_routes() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let budget=ResourceBudget::fixed(256<<20).unwrap();
    let reader=lctx_native::NativeReader::private(client.clone()).with_budget(&budget);
    for (index,sql) in [
        ("live_effects","SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false LIMIT 1"),
        ("live_attempts","SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state='open' LIMIT 1"),
        ("live_attempts","SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','frozen','closing'] LIMIT 1"),
        ("live_pins","SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1"),
        ("live_backups","SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1"),
        ("live_cleanup","SELECT VALUE id FROM native_cleanup WITH INDEX live_cleanup WHERE state='open' LIMIT 1"),
        ("live_retirements","SELECT VALUE id FROM native_retirement WITH INDEX live_retirements WHERE state='recovering' LIMIT 1"),
    ] {
        let plan:String=reader.query(format!("EXPLAIN {sql}"),Variables::new()).await.unwrap();
        assert!(plan.contains(index),"live probe must use {index}: {plan}");
    }
    for table in ["native_effect","native_cleanup","native_retirement","native_attempt"] {
        let plan:String=reader.query(format!("EXPLAIN SELECT VALUE id FROM type::record({table}:0>..) LIMIT 1"),Variables::new()).await.unwrap();
        assert!(plan.contains("DynamicScan") && plan.contains("limit: 1") && !plan.contains("RecordIdScan") && !plan.contains("Sort") && !plan.contains("TableScan"),"bounded primary-key page must push its limit into the scan: {plan}");
    }
    reader.close().await.unwrap();
    client.invalidate().await.unwrap();
}

fn maintenance_config()->RuntimeConfig {
    assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),"run through explicit validation maintenance");
    let path=std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("explicit maintenance installer configuration");
    let cfg=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    assert_eq!(cfg.authentication,lctx_native::AuthenticationScope::Root);
    assert_eq!(cfg.database.as_str(),"validation");
    cfg
}


// Read fixture lifecycle state before any setup effects. Failed observation still
// invalidates the checked session; source4 refusal deliberately keeps its raw path.
async fn maintenance_snapshot(client:&surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,expected_inventory:Option<&str>)->(bool,Option<String>){
    let result=async {
        let mut response=client.query("SELECT admission_open,history_inventory FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [row]=rows.as_slice() else{return Err(ModelError::Schema("exact maintenance installation"));};
        let Some(Value::Bool(open))=row.get("admission_open") else{return Err(ModelError::Schema("maintenance admission state"));};
        let inventory=match row.get("history_inventory"){None|Some(Value::None)=>None,Some(Value::String(value))=>Some(value.clone()),_=>return Err(ModelError::Schema("maintenance inventory state"))};
        if expected_inventory.is_some_and(|expected|inventory.as_deref()!=Some(expected)){return Err(ModelError::Conflict("separately qualified maintenance inventory required"));}
        Ok((*open,inventory))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    if result.is_err(){completion.step("maintenance state observation session invalidation",client.invalidate().await.map_err(ModelError::codec));}
    lctx_model::domain::completion::complete(result,completion).unwrap()
}


async fn cleanup_owned_attempt(client:&surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,attempt:ContentHash)->Result<(),ModelError>{
    let current=control::IssuanceEra::capture(client).await?;
    let mut response=client.query("SELECT generation,era FROM $attempt").bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    match rows.as_slice(){
        []=>Ok(()),
        [row] if row.get("generation")==Some(&Value::String(current.generation.hex())) && row.get("era")==Some(&Value::Number(surrealdb::types::Number::Int(current.era)))=>control::close_attempt(client,attempt,"abandoned").await,
        [_]=>Ok(()), // A permanently fenced predecessor requires its named successor.
        _=>Err(ModelError::Schema("owned attempt cleanup identity")),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "transaction-timeout regression requires explicit validation maintenance and the owned patched server"]
async fn transaction_timeouts_preserve_executor_stack_and_following_queries() {
    let client=lctx_compiler_store::compiler::check_installation(&maintenance_config()).await.unwrap();
    // Both executor timeout branches must recover with the service's unchanged
    // ten-second transaction / twenty-second query deadlines. No durable rows.
    let result=async {
    for (sql,timeout_index) in [
        ("LET $value = sleep(11s); RETURN 42;",0usize),
        ("BEGIN; LET $value = sleep(11s); COMMIT; RETURN 42;",1usize),
    ] {
        let original=client.query(sql).await.map_err(ModelError::codec);
        // Always probe a fresh executor before inspecting the timed-out response,
        // including when that request itself returns an error.
        let health=async {
            let mut next=client.query("RETURN 73").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let value:Value=next.take(0).map_err(ModelError::codec)?;
            if value!=Value::Number(surrealdb::types::Number::Int(73)) {return Err(ModelError::Conflict("fresh query after transaction timeout must retain a healthy executor"));}
            Ok::<_,ModelError>(())
        }.await;
        let mut health_completion=lctx_model::domain::completion::Completion::default();
        health_completion.step("fresh checked query after transaction timeout",health);
        let mut response=lctx_model::domain::completion::complete(original,health_completion)?;
        let statements=response.num_statements();
        let errors=response.take_errors();
        assert_eq!(statements,2,"bare timeout continues to RETURN; explicit timeout stops after BEGIN and its error");
        assert_eq!(errors.len(),1,"exactly one unchanged transaction deadline must fire");
        assert!(errors.get(&timeout_index).is_some_and(|error|error.to_string().contains("exceeded the timeout")),"expected transaction timeout at {timeout_index}: {errors:?}");
        if timeout_index==0 {
            let value:Value=response.take(1).map_err(ModelError::codec)?;
            assert_eq!(value,Value::Number(surrealdb::types::Number::Int(42)),"a bare-statement timeout preserves subsequent statements in the same batch");
        } else {
            // The pinned explicit-BEGIN timeout returns from the whole batch.
            // Neither COMMIT nor RETURN 42 has a result; slot zero is BEGIN.
            let begin:Value=response.take(0).map_err(ModelError::codec)?;
            assert_eq!(begin,Value::None);
        }
        assert_eq!(response.num_statements(),0,"all actual statement results were inspected");
    }
    Ok::<_,ModelError>(())
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("timeout control session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "permanent era cuts require explicit exclusive validation maintenance"]
async fn delayed_preintent_and_original_attempt_retry_stay_fenced_after_era_cut() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let result=async {
    let object=RecordId::new("native_guard",control::fresh_identity("delayed-first-intent")?.hex());
    let sql="CREATE $object SET revision=0,retired=false,phase='active',incarnation=1 RETURN NONE";
    let mut bindings=Variables::new();bindings.insert("object",object.clone());
    let request=control::NativeRequest::issue(&client,ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).map_err(ModelError::codec)?)).await?;
    let attempt_era=control::IssuanceEra::capture(&client).await?;
    let attempt=control::fresh_identity("delayed-first-attempt")?;
    let registration_root=RecordId::new("native_guard",control::fresh_identity("partial-registration-root")?.hex());
    let mut row=Object::new();row.insert("id",registration_root.clone());row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);
    control::ensure_rows(&client,None,vec![Value::Object(row)]).await?;
    let registration=control::fresh_identity("partial-registration-invocation")?;
        lctx_compiler_store::compiler::close_admission(&cfg).await?;
        lctx_compiler_store::compiler::drain_installation(&cfg).await?;
        let mut registration_vars=Variables::new();let job=RecordId::new("native_retirement",registration.hex());
        registration_vars.insert("job",job);registration_vars.insert("digest",ContentHash::of(&serde_json::to_vec(&("native-retirement/v4",vec![registration_root.clone()])).map_err(ModelError::codec)?).hex());
        control::effect(&client,None,"CREATE $job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$digest,lineage=$job,state='registering',root_count=1,roots_registered=0,examined=0,retired=0,revision=0 RETURN NONE",registration_vars).await?;
        assert!(control::cut_era(&client).await.is_err(),"incomplete root registration must remain in its recoverable original era");
        control::register_retirement_roots(&client,registration,vec![registration_root]).await?;
        let finished=control::resume_retirement(&client,registration,8).await?;
        assert!(finished.remaining.is_empty() && finished.retained.is_empty());
        let next=control::cut_era(&client).await?;
        let first_refused=control::execute_request(&client,request,None,sql,bindings.clone()).await.is_err();
        let retry_refused=control::execute_request(&client,request,None,sql,bindings).await.is_err();
        let disposition=control::reconcile_request(&client,request).await?;
        let attempt_refused=match control::begin_attempt_in(&client,attempt,attempt_era).await {
            Err(ModelError::Cause(cause))=>cause.downcast_ref::<control::NativeAttemptError>().is_some_and(|error|error.attempt==attempt && error.issuance==attempt_era),
            _=>false,
        };
        let mut response=client.query("SELECT VALUE id FROM $object; SELECT VALUE id FROM $attempt; SELECT VALUE id FROM $effect").bind(("object",object)).bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).bind(("effect",RecordId::new("native_effect",request.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let payload:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;let attempts:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;let effects:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;
        Ok::<_,ModelError>((request,next,first_refused,retry_refused,disposition,attempt_refused,payload,attempts,effects))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("era control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (request,next,first,retry,disposition,attempt_refused,payload,attempts,effects)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert_eq!(next.era,request.issuance.era+1);assert!(first && retry && attempt_refused);
    assert_eq!(disposition,control::EffectDisposition::CompactedTerminal,"missing closed-era identity never implies known uncommitted");
    assert!(payload.is_empty() && attempts.is_empty() && effects.is_empty(),"a delayed first intent cannot materialize after the cut");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "closing crash recovery and era cuts require explicit validation maintenance"]
async fn closing_attempt_before_cleanup_obligation_blocks_cut_and_resumes_through_drain() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let mut acquired_attempts=Vec::new();
    let result=async {
        // The host lease is exclusive and initially closed. Admit only this owned
        // setup through the real attempt protocol, then close before crash recovery.
        lctx_compiler_store::compiler::open_admission(&cfg).await?;
    let generation=cfg.service_generation;
    let one=control::issue_attempt(&client,generation).await?;acquired_attempts.push(one);
    let two=control::issue_attempt(&client,generation).await?;acquired_attempts.push(two);
    let mut targets=Vec::new();
    for attempt in [one,two] {
        let target=RecordId::new("native_guard",control::fresh_identity("closing-target")?.hex());
        let mut row=Object::new();row.insert("id",target.clone());row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);
        control::ensure_rows(&client,Some(attempt),vec![Value::Object(row)]).await?;
        control::hold(&client,Some(attempt),RecordId::new("native_attempt",attempt.hex()),vec![target.clone()]).await?;
        targets.push(target);
        let mut vars=Variables::new();vars.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
        // Exact production fence, followed by simulated process death before inventory.
        control::effect(&client,Some(attempt),"UPDATE $attempt SET state='closing',revision+=1 RETURN NONE",vars).await?;
    }
    lctx_compiler_store::compiler::close_admission(&cfg).await?;
        assert!(control::cut_era(&client).await.is_err(),"pre-obligation scope must stay resumable in the original era");
        control::close_attempt(&client,one,"abandoned").await?;
        assert!(control::cut_era(&client).await.is_err(),"the other crashed close still fences the era");
        lctx_compiler_store::compiler::drain_installation(&cfg).await?;
        let attempts=[RecordId::new("native_attempt",one.hex()),RecordId::new("native_attempt",two.hex())];
        let mut response=client.query("SELECT VALUE state FROM $attempts; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner IN $attempts").bind(("attempts",attempts.to_vec())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let states:Vec<String>=response.take(0).map_err(ModelError::codec)?;
        let holds:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;
        assert_eq!(states,["abandoned","abandoned"]);assert!(holds.is_empty());
        for target in targets {control::retire(&client,target).await?;}
        control::cut_era(&client).await?;
        Ok::<_,ModelError>(())
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    if result.is_err(){
        completion.step("failed attempt setup admission closure",lctx_compiler_store::compiler::close_admission(&cfg).await);
        for attempt in acquired_attempts {completion.step("partially acquired owned attempt cleanup",cleanup_owned_attempt(&client,attempt).await);}
    }
    completion.step("closing control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "successor ownership across permanent cuts requires explicit validation maintenance"]
async fn post_cut_cleanup_and_retirement_preserve_scope_across_two_successors() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let nonce=control::fresh_identity("maintenance-successors").unwrap();
    let parent=RecordId::new("native_guard",format!("successor_parent_{}",nonce.hex()));
    let child=RecordId::new("native_guard",format!("successor_child_{}",nonce.hex()));
    let cleanup_target=RecordId::new("native_guard",format!("successor_cleanup_{}",nonce.hex()));
    let mut acquired_attempts=Vec::new();
    let result=async {
        let original=control::IssuanceEra::capture(&client).await?;
        // Fresh attempts need real admission even under the exclusive host lease.
        // Setup errors still flow through original-admission restoration below.
        lctx_compiler_store::compiler::open_admission(&cfg).await?;
    let values=[parent.clone(),child.clone(),cleanup_target.clone()].into_iter().map(|id|{let mut row=Object::new();row.insert("id",id);row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)}).collect();
    control::ensure_rows(&client,None,values).await?;
    control::hold(&client,None,parent.clone(),vec![child.clone()]).await?;
    let retirement=control::retire_reachable(&client,vec![parent.clone()],1).await?;
    let attempt=control::issue_attempt(&client,original.generation).await?;acquired_attempts.push(attempt);
    control::hold(&client,Some(attempt),RecordId::new("native_attempt",attempt.hex()),vec![cleanup_target.clone()]).await?;
    let cleanup_era=control::IssuanceEra::capture(&client).await?;
    let cleanup=control::fresh_identity("interrupted-terminal-cleanup")?;
    let mut vars=Variables::new();vars.insert("attempt",RecordId::new("native_attempt",attempt.hex()));vars.insert("cleanup",RecordId::new("native_cleanup",cleanup.hex()));
    let attempt_id=RecordId::new("native_attempt",attempt.hex());
    let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(attempt_id.clone())).map_err(ModelError::codec)?).hex());
    let mut observation=client.query("SELECT incarnation,phase FROM $guard").bind(("guard",guard.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let physical:Vec<Object>=observation.take(0).map_err(ModelError::codec)?;
    let [physical]=physical.as_slice()else{return Err(ModelError::Conflict("cleanup control exact owner guard"));};
    let mut expected=physical.clone();expected.insert("record",attempt_id);expected.insert("guard",guard);
    vars.insert("owner_incarnations",vec![Value::Object(expected)]);
    // Persist the production scope, then simulate interruption before its first page.
    control::effect(&client,Some(attempt),"fn::native_touch($cleanup); fn::native_touch($attempt); UPDATE $attempt SET state='closed',revision+=1 RETURN NONE; CREATE $cleanup SET attempt=$attempt,generation=$__generation,era=$__era,epoch=$__epoch,owners=[$attempt],owner_incarnations=$owner_incarnations,lineage=$cleanup,unadmitted=true,state='open' RETURN NONE",vars).await?;
    let mut response=client.query("SELECT epoch FROM $attempt; SELECT id,incarnation,cutoff FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retiring'").bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).bind(("job",RecordId::new("native_retirement",retirement.identity.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let epochs:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let claimed:Vec<Object>=response.take(1).map_err(ModelError::codec)?;
        lctx_compiler_store::compiler::close_admission(&cfg).await?;lctx_compiler_store::compiler::drain_installation(&cfg).await?;
        control::cut_era(&client).await?;
        let old_cleanup_fenced=control::resume_cleanup(&client,cleanup).await.is_err();
        let old_retirement_fenced=control::resume_retirement(&client,retirement.identity,8).await.is_err();
        let cleanup_one=control::recover_cleanup(&client,cleanup).await?;
        let retirement_one=control::recover_retirement(&client,retirement.identity).await?;
        let repeat_one=(control::recover_cleanup(&client,cleanup).await?==cleanup_one,control::recover_retirement(&client,retirement.identity).await?==retirement_one);
        // A second cut interrupts the first successors before any deletion page. Recovery
        // must name them, retain the original owner epoch/cutoff and never reopen predecessors.
        control::cut_era(&client).await?;
        let stale_predecessors=(control::recover_cleanup(&client,cleanup).await.is_err(),control::recover_retirement(&client,retirement.identity).await.is_err());
        let cleanup_two=control::recover_cleanup(&client,cleanup_one).await?;
        let retirement_two=control::recover_retirement(&client,retirement_one).await?;
        let repeat_two=(control::recover_cleanup(&client,cleanup_one).await?==cleanup_two,control::recover_retirement(&client,retirement_one).await?==retirement_two);
        control::resume_cleanup(&client,cleanup_two).await?;
        let mut progress=control::resume_retirement(&client,retirement_two,8).await?;
        while !progress.remaining.is_empty(){progress=control::resume_retirement(&client,retirement_two,8).await?;}
        let mut response=client.query("SELECT epoch,owners,state FROM $cleanup; SELECT id,incarnation,cutoff FROM $claimed; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt; SELECT VALUE id FROM $payloads").bind(("cleanup",RecordId::new("native_cleanup",cleanup_two.hex()))).bind(("claimed",claimed.iter().filter_map(|row|row.get("id")).cloned().collect::<Vec<_>>())).bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).bind(("payloads",vec![parent.clone(),child.clone(),cleanup_target.clone()])).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let cleaned:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let final_claim:Vec<Object>=response.take(1).map_err(ModelError::codec)?;let holds:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;let payloads:Vec<RecordId>=response.take(3).map_err(ModelError::codec)?;
        control::retire(&client,cleanup_target.clone()).await?;
        Ok::<_,ModelError>((old_cleanup_fenced,old_retirement_fenced,cleanup_one,retirement_one,cleanup_two,retirement_two,repeat_one,repeat_two,stale_predecessors,progress,cleaned,final_claim,holds,payloads,cleanup_era,epochs,claimed,retirement,cleanup,original))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    if result.is_err(){
        completion.step("failed attempt setup admission closure",lctx_compiler_store::compiler::close_admission(&cfg).await);
        for attempt in acquired_attempts {completion.step("partially acquired owned attempt cleanup",cleanup_owned_attempt(&client,attempt).await);}
    }
    completion.step("successor control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (cleanup_fenced,retirement_fenced,cleanup_one,retirement_one,cleanup_two,retirement_two,repeat_one,repeat_two,stale,progress,cleaned,final_claim,holds,payloads,cleanup_era,epochs,claimed,retirement,cleanup,original)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert_eq!(cleanup_era,original);assert!(cleanup_fenced && retirement_fenced && repeat_one.0 && repeat_one.1 && repeat_two.0 && repeat_two.1 && stale.0 && stale.1);
    assert_ne!(cleanup_one,cleanup);assert_ne!(cleanup_two,cleanup_one);assert_ne!(retirement_one,retirement.identity);assert_ne!(retirement_two,retirement_one);
    assert_eq!(progress.retired,2);assert!(progress.retained.is_empty() && holds.is_empty());assert_eq!(payloads,[cleanup_target]);
    assert_eq!(cleaned[0].get("epoch"),epochs[0].get("epoch"));assert_eq!(cleaned[0].get("owners"),Some(&Value::Array(Vec::<Value>::new().into())));assert_eq!(cleaned[0].get("state"),Some(&Value::String("done".into())));
    assert_eq!(final_claim,claimed,"successor preserves the originally claimed item identity/incarnation/cutoff");
}

