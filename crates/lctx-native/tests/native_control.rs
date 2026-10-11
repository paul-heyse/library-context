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
    let reader=lctx_native::NativeReader::private(client.clone());
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

#[tokio::test(flavor="multi_thread")]
#[ignore="requires an explicitly owned closed validation installation still at format5 before its format6 transition"]
async fn source5_history_refusal_creates_no_intent_reference_checkpoint_or_admission_revision(){
    let cfg=maintenance_config();
    // This control observes an actual predecessor installation. It never edits the marker,
    // applies declarations, opens admission or creates a synthetic source-format receipt.
    let client=lctx_native::reader::connect(&cfg.endpoint,&cfg.writer_credentials(),cfg.namespace.as_str(),cfg.database.as_str()).await.unwrap();
    let result=async {
        let snapshot="SELECT schema,schema_version,generation,era,closed_through,admission_open,history_inventory,control_revision,admission_revision FROM native_installation:current; SELECT count() FROM native_effect GROUP ALL; SELECT count() FROM native_outcome_ref GROUP ALL; SELECT count() FROM native_history_checkpoint GROUP ALL;";
        let mut response=client.query(snapshot).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let before=(0..4).map(|index|response.take::<Value>(index).map_err(ModelError::codec)).collect::<Result<Vec<_>,_>>()?;
        let Value::Array(markers)=&before[0] else{return Err(ModelError::Schema("source5 control marker array"));};
        let [Value::Object(marker)]=markers.as_slice() else{return Err(ModelError::Schema("source5 control exact marker"));};
        if marker.get("schema_version")!=Some(&Value::Number(surrealdb::types::Number::Int(5))) || marker.get("admission_open")!=Some(&Value::Bool(false)) || marker.get("generation")!=Some(&Value::String(cfg.service_generation.hex())) {return Err(ModelError::Conflict("source5 control requires existing exact closed predecessor installation"));}
        let refusal=control::compact_history(&client,1).await.err().ok_or(ModelError::Conflict("source5 history unexpectedly admitted"))?;
        let mut response=client.query(snapshot).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let after=(0..4).map(|index|response.take::<Value>(index).map_err(ModelError::codec)).collect::<Result<Vec<_>,_>>()?;
        Ok::<_,ModelError>((before,after,refusal.to_string()))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();completion.step("source5 refusal session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (before,after,refusal)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(refusal.contains("native history receipt schema requires explicit upgrade"),"{refusal}");
    assert_eq!(before,after,"source5 refusal must precede durable intent, outcome reference, checkpoint and admission/control revision changes");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "history deletion requires explicit maintenance and separately qualified consumer inventory"]
async fn qualified_history_pages_preserve_exact_references_and_collect_terminal_provenance() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let expected=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("separately reviewed history consumer inventory evidence");
    let (admission_was_open,inventory)=maintenance_snapshot(&client,Some(&expected)).await;
    assert_eq!(inventory.as_deref(),Some(expected.as_str()),"this control never qualifies source-only evidence for deletion");
    let result=async {
    let original=control::IssuanceEra::capture(&client).await?;
    let nonce=control::fresh_identity("history-eligibility-control")?;
    let prefix=format!("!history_{}",nonce.hex());
    let attempt_one=RecordId::new("native_attempt",format!("{prefix}_1"));
    let attempt_two=RecordId::new("native_attempt",format!("{prefix}_2"));
    let foreign=RecordId::new("native_attempt",format!("{prefix}_3"));
    let cleanup=RecordId::new("native_cleanup",format!("{prefix}_1"));
    let mut bindings=Variables::new();bindings.insert("one",attempt_one.clone());bindings.insert("two",attempt_two.clone());bindings.insert("foreign",foreign.clone());bindings.insert("foreign_generation",ContentHash::of(b"foreign-history-generation").hex());bindings.insert("cleanup",cleanup.clone());
    control::effect(&client,None,"CREATE $one SET generation=$__generation,era=$__era,epoch=$__epoch,state='closed',admitted=false,revision=0 RETURN NONE; CREATE $two SET generation=$__generation,era=$__era,epoch=$__epoch,state='closed',admitted=false,revision=0 RETURN NONE; CREATE $foreign SET generation=$foreign_generation,era=$__era,epoch=$__epoch,state='closed',admitted=false,revision=0 RETURN NONE; CREATE $cleanup SET attempt=$one,generation=$__generation,era=$__era,epoch=$__epoch,owners=[],unadmitted=true,state='done' RETURN NONE",bindings).await?;
    let reference=control::retain_native_outcome(&client,attempt_two.clone(),"history-control","evidence").await?;
    let sql="RETURN NONE";let bindings=Variables::new();
    let request=control::NativeRequest::issue(&client,ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).map_err(ModelError::codec)?)).await?;
    control::execute_request(&client,request,None,sql,bindings).await?;
        lctx_compiler_store::compiler::close_admission(&cfg).await?;lctx_compiler_store::compiler::drain_installation(&cfg).await?;control::cut_era(&client).await?;
        // Seed finite, exact checkpoints as native fixture state. Production creates the
        // same checkpoint fields; these cursors avoid walking unrelated retained history.
        let attempt_before=RecordId::new("native_attempt",format!("{prefix}_0"));
        let first=history_checkpoint(&client,original.era,"attempt",attempt_before.clone()).await?;
        let before=control::resume_history_compaction(&client,first,0,1).await?;
        let cleanup_before=RecordId::new("native_cleanup",format!("{prefix}_0"));
        let cleanup_page=history_checkpoint(&client,original.era,"cleanup",cleanup_before).await?;
        let cleaned=control::resume_history_compaction(&client,cleanup_page,0,1).await?;
        let attempts=history_checkpoint(&client,original.era,"attempt",attempt_before).await?;
        let after=control::resume_history_compaction(&client,attempts,0,2).await?;
        // Discarding/repeating the acknowledgement returns the same durable page, including
        // protected candidates, without re-running deletion or changing cumulative counts.
        let replayed=control::resume_history_compaction(&client,attempts,0,2).await?;
        assert_eq!(replayed,after);
        assert!(control::resume_history_compaction(&client,attempts,0,1).await.is_err(),"the acknowledged request limit is immutable");
        assert_eq!(after.examined,[attempt_one.clone(),attempt_two.clone()]);
        assert_eq!(after.removed,[attempt_one.clone()]);
        assert_eq!(after.revision,1);
        let advanced=control::resume_history_compaction(&client,attempts,after.revision,1).await?;
        assert_eq!(advanced.examined,[foreign.clone()]);assert_eq!(advanced.protected,[foreign.clone()]);assert_eq!(advanced.revision,2);
        assert!(control::resume_history_compaction(&client,attempts,0,2).await.is_err(),"an overwritten older receipt cannot be fabricated");
        assert_eq!(control::resume_history_compaction(&client,attempts,1,1).await?,advanced);
        let mut previous=request.operation.0;
        for byte in previous.iter_mut().rev(){if *byte>0{*byte-=1;break;}*byte=255;}
        let effect_before=RecordId::new("native_effect",ContentHash(previous).hex());
        let held_page=history_checkpoint(&client,original.era,"effect",effect_before.clone()).await?;
        let held=control::resume_history_compaction(&client,held_page,0,1).await?;
        let retained=control::request_disposition(&client,request).await?;
        control::release_outcome(&client,request,&request.operation.hex(),"caller").await?;
        let released_page=history_checkpoint(&client,original.era,"effect",effect_before).await?;
        let released=control::resume_history_compaction(&client,released_page,0,1).await?;
        let compacted=control::request_disposition(&client,request).await?;
        let mut response=client.query("SELECT VALUE id FROM $one; SELECT VALUE id FROM $two; SELECT VALUE id FROM $cleanup").bind(("one",attempt_one.clone())).bind(("two",attempt_two.clone())).bind(("cleanup",cleanup)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let one:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;let two:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;let cleanup:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;
        control::release_outcome_reference(&client,reference,"history-control","evidence").await?;
        let final_page=history_checkpoint(&client,original.era,"attempt",attempt_one.clone()).await?;
        let final_result=control::resume_history_compaction(&client,final_page,0,1).await?;
        let foreign_page=history_checkpoint(&client,original.era,"attempt",attempt_two.clone()).await?;
        let protected=control::resume_history_compaction(&client,foreign_page,0,1).await?;
        assert_eq!(protected.removed_records,0);assert_eq!(protected.protected,[foreign.clone()],"a closed era never grants collection authority over a different generation");
        // Remove only this freshly allocated corruption fixture after proving protection.
        client.query("DELETE $foreign RETURN NONE").bind(("foreign",foreign)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok::<_,ModelError>((before,cleaned,after,held,retained,released,compacted,one,two,cleanup,final_result,attempt_one,attempt_two))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();completion.step("history control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (before,cleaned,after,held,retained,released,compacted,one,two,cleanup,final_result,attempt_one,attempt_two)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert_eq!(before.removed_records,0);assert_eq!(before.protected,[attempt_one]);
    assert_eq!(cleaned.removed_records,1);assert_eq!(after.removed_records,1);assert_eq!(after.protected,[attempt_two.clone()]);
    assert_eq!(held.removed_effects,0);assert_eq!(retained,control::EffectDisposition::Committed);
    assert_eq!(released.removed_effects,1);assert_eq!(compacted,control::EffectDisposition::CompactedTerminal);
    assert!(one.is_empty() && cleanup.is_empty());assert_eq!(two,[attempt_two]);assert_eq!(final_result.removed_records,1);
}

#[tokio::test(flavor="multi_thread")]
#[ignore="lineage collection requires explicit maintenance and separately qualified consumer inventory"]
async fn qualified_history_preserves_live_retirement_lineage_between_successors(){
    let cfg=maintenance_config();let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let expected=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("separately qualified consumer inventory");
    let (admission_was_open,inventory)=maintenance_snapshot(&client,Some(&expected)).await;
    assert_eq!(inventory.as_deref(),Some(expected.as_str()));
    let result=async {
    let original=control::IssuanceEra::capture(&client).await?;let nonce=control::fresh_identity("lineage-history")?;
    let parent=RecordId::new("native_guard",format!("lineage_parent_{}",nonce.hex()));let child=RecordId::new("native_guard",format!("lineage_child_{}",nonce.hex()));
    let rows=[parent.clone(),child.clone()].into_iter().map(|id|{let mut row=Object::new();row.insert("id",id);row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)}).collect();
    control::ensure_rows(&client,None,rows).await?;control::hold(&client,None,parent.clone(),vec![child.clone()]).await?;
    let retired=control::retire_reachable(&client,vec![parent.clone()],1).await?;let origin=RecordId::new("native_retirement",retired.identity.hex());
        lctx_compiler_store::compiler::close_admission(&cfg).await?;lctx_compiler_store::compiler::drain_installation(&cfg).await?;control::cut_era(&client).await?;
        let one=control::recover_retirement(&client,retired.identity).await?;
        let mut response=client.query("SELECT VALUE id FROM native_effect WITH INDEX owner_effects WHERE owner=$origin AND resolved=true").bind(("origin",origin.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let effects:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
        for effect in effects {let checkpoint=history_checkpoint(&client,original.era,"effect",before_hash_record(&effect)?).await?;let receipt=control::resume_history_compaction(&client,checkpoint,0,1).await?;assert_eq!(receipt.removed_effects,1);}
        let checkpoint=history_checkpoint(&client,original.era,"retirement",before_hash_record(&origin)?).await?;
        let protected=control::resume_history_compaction(&client,checkpoint,0,1).await?;
        assert_eq!(protected.removed_records,0);assert_eq!(protected.protected,[origin.clone()],"live successor keeps its predecessor metadata after original effects are collected");
        control::cut_era(&client).await?;let two=control::recover_retirement(&client,one).await?;
        let mut progress=control::resume_retirement(&client,two,8).await?;while !progress.remaining.is_empty(){progress=control::resume_retirement(&client,two,8).await?;}
        assert_eq!(progress.retired,2);assert!(progress.retained.is_empty());
        let checkpoint=history_checkpoint(&client,original.era,"retirement",before_hash_record(&origin)?).await?;
        let collected=control::resume_history_compaction(&client,checkpoint,0,1).await?;assert_eq!(collected.removed_records,1,"completed lineage no longer needs predecessor metadata");
        let mut response=client.query("SELECT VALUE id FROM $payloads").bind(("payloads",vec![parent,child])).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let remaining:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;assert!(remaining.is_empty());Ok::<_,ModelError>(())
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();completion.step("lineage history admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion).unwrap();
}
#[tokio::test(flavor="multi_thread")]
#[ignore="history deletion requires explicit maintenance and separately qualified consumer inventory"]
async fn qualified_history_bounds_large_requests_and_preserves_interleaved_high_degree_records(){
    let cfg=maintenance_config();let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let expected=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("separately qualified consumer inventory");
    let (admission_was_open,inventory)=maintenance_snapshot(&client,Some(&expected)).await;
    assert_eq!(inventory.as_deref(),Some(expected.as_str()),"this test cannot manufacture deletion authority");
    let result=async {
    let nonce=control::fresh_identity("history-large-page")?;let prefix=format!("!history_large_{}",nonce.hex());
    let owner=RecordId::new("native_attempt",format!("{prefix}_owner"));
    let objects=(0..257).map(|index|RecordId::new("native_guard",format!("{prefix}_{index:03}"))).collect::<Vec<_>>();
    let guards=objects.iter().map(|id|{let mut row=Object::new();row.insert("id",id.clone());row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);Value::Object(row)}).collect();
    control::ensure_rows(&client,None,guards).await?;
    let candidates=(0..130).map(|index|RecordId::new("native_cleanup",format!("{prefix}_{index:03}"))).collect::<Vec<_>>();
    let original=control::IssuanceEra::capture(&client).await?;
    let mut vars=Variables::new();vars.insert("owner",owner.clone());
    control::effect(&client,None,"CREATE $owner SET generation=$__generation,era=$__era,epoch=$__epoch,state='closed',admitted=false,revision=0 RETURN NONE",vars).await?;
    for chunk in candidates.chunks(128){
        let rows=chunk.iter().enumerate().map(|(index,id)|{let mut row=Object::new();row.insert("id",id.clone());row.insert("owners",if index==0{objects.clone()}else if index%2==0{vec![objects[0].clone()]}else{Vec::new()});Value::Object(row)}).collect::<Vec<_>>();
        let mut vars=Variables::new();vars.insert("owner",owner.clone());vars.insert("rows",rows);
        control::effect(&client,None,"FOR $row IN $rows { LET $id=$row.id; CREATE $id SET attempt=$owner,generation=$__generation,era=$__era,epoch=$__epoch,owners=$row.owners,unadmitted=true,state='done' RETURN NONE; };",vars).await?;
    }
        lctx_compiler_store::compiler::close_admission(&cfg).await?;lctx_compiler_store::compiler::drain_installation(&cfg).await?;control::cut_era(&client).await?;
        let before=RecordId::new("native_cleanup",format!("{prefix}_"));
        let checkpoint=history_checkpoint(&client,original.era,"cleanup",before).await?;
        let page=control::resume_history_compaction(&client,checkpoint,0,10_000).await?;
        assert_eq!(page.examined,candidates[..128]);assert_eq!(page.revision,1);assert_eq!(page.removed_records,64);
        assert_eq!(page.removed,candidates[..128].iter().skip(1).step_by(2).cloned().collect::<Vec<_>>());
        assert_eq!(page.protected,candidates[..128].iter().step_by(2).cloned().collect::<Vec<_>>());
        assert_eq!(control::resume_history_compaction(&client,checkpoint,0,10_000).await?,page);
        assert!(control::resume_history_compaction(&client,checkpoint,0,128).await.is_err(),"physical clamping does not change original request identity");
        let next=control::resume_history_compaction(&client,checkpoint,page.revision,2).await?;
        assert_eq!(next.examined,candidates[128..]);assert_eq!(next.removed,[candidates[129].clone()]);assert_eq!(next.protected,[candidates[128].clone()]);assert_eq!(next.removed_records,65);
        let mut response=client.query("SELECT VALUE id FROM $objects; SELECT owners FROM $high_degree; SELECT VALUE id FROM $candidates").bind(("objects",objects.clone())).bind(("high_degree",candidates[0].clone())).bind(("candidates",candidates.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let remaining_objects:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;let protected:Vec<Object>=response.take(1).map_err(ModelError::codec)?;let remaining:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;
        assert_eq!(remaining_objects.len(),257);assert_eq!(protected[0].get("owners"),Some(&Value::Array(objects.clone().into_iter().map(Value::RecordId).collect::<Vec<_>>().into())));assert_eq!(remaining.len(),65);
        Ok::<_,ModelError>(())
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();completion.step("bounded history admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance fixture session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion).unwrap();
}

fn before_hash_record(id:&RecordId)->Result<RecordId,ModelError>{
    let surrealdb::types::RecordIdKey::String(key)=&id.key else{return Err(ModelError::Schema("fixture hash identity"));};
    let mut bytes=hex::decode(key).map_err(ModelError::codec)?;
    for byte in bytes.iter_mut().rev(){if *byte>0{*byte-=1;break;}*byte=255;}
    Ok(RecordId::new(id.table.as_str(),hex::encode(bytes)))
}

async fn history_checkpoint(client:&surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,closed:i64,phase:&str,after:RecordId)->Result<ContentHash,ModelError>{
    let issuance=control::IssuanceEra::capture(client).await?;
    let identity=control::fresh_identity("bounded-history-native-fixture")?;
    let mut bindings=Variables::new();bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",identity.hex()));bindings.insert("closed",closed);bindings.insert("phase",phase.to_owned());bindings.insert("after",after);bindings.insert("issuance",issuance.era);
    control::effect(client,None,"IF $__installation.admission_open OR $__installation.history_inventory=NONE OR $__era!=$issuance OR $__installation.closed_through<$closed { THROW 'history fixture requires qualified closed maintenance'; }; CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=$__epoch,closed_through=$closed,phase=$phase,after=$after,state='open',removed_effects=0,removed_records=0,revision=0 RETURN NONE",bindings).await?;
    Ok(identity)
}
fn row(run: [u8; 16], value: f64) -> QualityStep {
    QualityStep {
        run: serde_json::from_value(serde_json::json!(run)).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(value).unwrap(),
    }
}
fn specification(producer: String) -> ContributionSpec {
    ContributionSpec {
        captured_binding: None,
        producer,
        profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
        model: lctx_model::domain::model().unwrap().digest(),
        implementation: ContentHash::of(b"native-control/v1"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([QualityStep::NAME.into()]),
    }
}
async fn contribute(
    store: &std::sync::Arc<NativeCompilerStore>,
    spec: ContributionSpec,
    row: &QualityStep,
    previous: &BTreeMap<String, lctx_model::domain::completed::CompletedView>,
) -> (
    ContentHash,
    BTreeMap<String, lctx_model::domain::completed::CompletedView>,
) {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let relation = Relation::of::<QualityStep>();
    let id = store.begin_contribution(spec).await.unwrap();
    store
        .write_batch(
            &id,
            &relation,
            &QualityStep::encode(std::slice::from_ref(row)).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(id, ProviderOutcome::Complete, &[relation], previous, &native_operation_budget)
        .await
        .unwrap();
    (id, views)
}
#[tokio::test(flavor = "multi_thread")]
async fn indexed_abandonment_removes_only_unadmitted_products_and_owned_holds() {
    let cfg = config();
    let private = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let admitted = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("indexed-abandonment").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let (private_contribution, _) = contribute(
        &private,
        specification(format!("private-{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let (admitted_contribution, views) = contribute(
        &admitted,
        specification(format!("admitted-{}", nonce.hex())),
        &row(run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    private
        .retain_product_identity(nonce, private_contribution)
        .await
        .unwrap();
    admitted
        .retain_product_identity(nonce, admitted_contribution)
        .await
        .unwrap();
    let view = views[QualityStep::NAME].clone();
    admitted
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &Relation::of::<QualityStep>(),
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    admitted.mark_attempt_admitted().await.unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let unrelated = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    let result=async {
        unrelated.protect(RecordId::new("compiler_contribution",private_contribution.hex())).await?;
        let private_owner = RecordId::new("native_attempt", private.attempt().hex());
        let private_product = RecordId::new("native_product",format!("{}_{}",nonce.hex(),private.attempt().hex()));
        // Exercise many transaction windows for both attempt and unadmitted product roots.
        // The immutable targets survive cleanup; only these two owners lose their holds.
        let targets = (0..1025).map(|ordinal| RecordId::new("original",ContentHash::of(format!("cleanup_{}_{}",nonce.hex(),ordinal).as_bytes()).hex())).collect::<Vec<_>>();
        for window in targets.chunks(128) {
            let rows = window.iter().map(|id| {
                let mut row = Object::new();
                row.insert("id",id.clone());
                row.insert("content",ContentHash::of(b"").hex());
                row.insert("byte_len",0i64);
                Value::Object(row)
            }).collect();
            control::ensure_rows(&client,Some(private.attempt()),rows).await?;
            control::hold(&client,Some(private.attempt()),private_owner.clone(),window.to_vec()).await?;
            control::hold(&client,Some(private.attempt()),private_product.clone(),window.to_vec()).await?;
            control::hold(&client,Some(admitted.attempt()),RecordId::new("native_attempt",admitted.attempt().hex()),window.to_vec()).await?;
        }
        let mut partial = Variables::new();
        partial.insert("attempt",private_owner.clone());
        partial.insert("target",targets[0].clone());
        partial.insert("admitted_attempt",RecordId::new("native_attempt",admitted.attempt().hex()));
        let mut response = client.query("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND object=$target LIMIT 1").bind(partial.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let first:Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        partial.insert("first",first);
        // Simulate restart after a confirmed fence and one confirmed partial release.
        control::effect(&client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; UPDATE $attempt SET state='closing',revision+=1 RETURN NONE; UPDATE $admitted_attempt SET state='maintenance_fenced',revision+=1 RETURN NONE; LET $owned=SELECT VALUE id FROM $first WHERE owner=$attempt; DELETE $owned RETURN NONE",partial).await?;
        let post_fence_refused = control::effect(&client,Some(private.attempt()),"RETURN NONE",Variables::new()).await.is_err();
        let mut plans=Vec::new();
        for (sql,index,variable,value) in [
            ("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner","owner_holds","owner",RecordId::new("native_attempt",private.attempt().hex())),
            ("SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt","attempt_contributions","attempt",RecordId::new("native_attempt",private.attempt().hex())),
            ("SELECT VALUE id FROM native_product WITH INDEX contribution_products WHERE contribution=$contribution","contribution_products","contribution",RecordId::new("compiler_contribution",private_contribution.hex())),
        ] {let mut vars=Variables::new();vars.insert(variable,value);let plan:String=lctx_native::NativeReader::private(client.clone()).query(format!("EXPLAIN {sql}"),vars).await?;plans.push((index,plan));}
        let receipt_reader = lctx_native::NativeReader::private(client.clone());
        let mut receipt_scope = Variables::new();
        receipt_scope.insert("attempts", vec![private_owner.clone(), RecordId::new("native_attempt", admitted.attempt().hex())]);
        let original_epochs: Vec<Object> = receipt_reader.query_native("SELECT id,epoch FROM $attempts", receipt_scope.clone()).await?;
        let prior_receipts: Vec<RecordId> = receipt_reader.query_native("SELECT VALUE id FROM native_effect WHERE attempt IN $attempts", receipt_scope.clone()).await?;
        let prior_receipts = prior_receipts.into_iter().collect::<BTreeSet<_>>();
        private.abandon().await?;admitted.abandon().await?;
        let receipts: Vec<Object> = receipt_reader.query_native("SELECT id,request,attempt,epoch,revision,committed,resolved FROM native_effect WHERE attempt IN $attempts", receipt_scope).await?;
        let cleanup_receipts = receipts.into_iter().filter(|receipt| !matches!(receipt.get("id"), Some(Value::RecordId(id)) if prior_receipts.contains(id))).collect::<Vec<_>>();
        let private_product=RecordId::new("native_product",format!("{}_{}",nonce.hex(),private.attempt().hex()));let admitted_product=RecordId::new("native_product",format!("{}_{}",nonce.hex(),admitted.attempt().hex()));
        let mut response=client.query("SELECT VALUE id FROM $products; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$private; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$admitted; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$unrelated; SELECT VALUE state FROM $attempts; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt; SELECT VALUE id FROM $targets").bind(("products",vec![private_product.clone(),admitted_product.clone()])).bind(("private",private_product)).bind(("admitted",admitted_product.clone())).bind(("unrelated",RecordId::new("native_pin",unrelated.identity.hex()))).bind(("attempts",vec![private_owner.clone(),RecordId::new("native_attempt",admitted.attempt().hex())])).bind(("attempt",private_owner)).bind(("targets",targets)).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let products:Vec<RecordId>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;let private_holds:Vec<RecordId>=response.take(1).map_err(lctx_model::domain::ModelError::codec)?;let admitted_holds:Vec<RecordId>=response.take(2).map_err(lctx_model::domain::ModelError::codec)?;let unrelated_holds:Vec<RecordId>=response.take(3).map_err(lctx_model::domain::ModelError::codec)?;
        let states:Vec<String>=response.take(4).map_err(ModelError::codec)?;
        let attempt_holds:Vec<RecordId>=response.take(5).map_err(ModelError::codec)?;
        let targets:Vec<RecordId>=response.take(6).map_err(ModelError::codec)?;
        Ok::<_,lctx_model::domain::ModelError>((plans,products,admitted_product,private_holds,admitted_holds,unrelated_holds,post_fence_refused,states,attempt_holds,targets,original_epochs,cleanup_receipts))
    }.await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "indexed cleanup unrelated pin release",
        unrelated.release().await,
    );
    completion.step("indexed cleanup private local drain", private.drain().await);
    completion.step(
        "indexed cleanup private owner",
        control::close_attempt(&client, private.attempt(), "abandoned").await,
    );
    completion.step(
        "indexed cleanup admitted local drain",
        admitted.drain().await,
    );
    completion.step(
        "indexed cleanup admitted owner",
        control::close_attempt(&client, admitted.attempt(), "abandoned").await,
    );
    let (
        plans,
        products,
        admitted_product,
        private_holds,
        admitted_holds,
        unrelated_holds,
        post_fence_refused,
        states,
        attempt_holds,
        targets,
        original_epochs,
        cleanup_receipts,
    ) = lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(post_fence_refused);
    // Newly created cleanup receipts belong to the exact terminal owners and retain
    // their original epochs, independently of unrelated installation revision changes.
    let original_epochs = original_epochs
        .into_iter()
        .map(|row| {
            let Some(Value::RecordId(owner)) = row.get("id") else {
                panic!("original cleanup owner");
            };
            let Some(Value::Number(surrealdb::types::Number::Int(epoch))) = row.get("epoch") else {
                panic!("original cleanup epoch");
            };
            (owner.clone(), *epoch)
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(original_epochs.len(), 2);
    let mut scopes = BTreeMap::<RecordId, usize>::new();
    let mut requests = BTreeSet::new();
    for receipt in cleanup_receipts {
        let Some(Value::RecordId(owner)) = receipt.get("attempt") else {
            panic!("cleanup receipt must name its terminal owner");
        };
        let expected = original_epochs
            .get(owner)
            .expect("cleanup receipt owner is exact");
        assert_eq!(
            receipt.get("epoch"),
            Some(&Value::Number(surrealdb::types::Number::Int(*expected)))
        );
        assert_eq!(receipt.get("committed"), Some(&Value::Bool(true)));
        assert_eq!(receipt.get("resolved"), Some(&Value::Bool(true)));
        let Some(Value::String(request)) = receipt.get("request") else {
            panic!("cleanup request identity");
        };
        assert!(
            requests.insert((owner.clone(), request.clone())),
            "each completed cleanup scope has its own exact request"
        );
        if let Some(Value::Number(surrealdb::types::Number::Int(revision))) =
            receipt.get("revision")
        {
            if *revision >= 10 {
                *scopes.entry(owner.clone()).or_default() += 1;
            }
        } else {
            panic!("cleanup scope revision");
        }
    }
    assert!(
        scopes
            .get(&RecordId::new("native_attempt", private.attempt().hex()))
            .copied()
            .unwrap_or_default()
            >= 2,
        "both private product and attempt scopes must commit only after all nine pages and final empty proof"
    );
    assert!(
        scopes
            .get(&RecordId::new("native_attempt", admitted.attempt().hex()))
            .copied()
            .unwrap_or_default()
            >= 1,
        "large admitted cleanup also proves all nine pages and final empty completion"
    );
    assert_eq!(
        states.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from(["abandoned".to_string(), "maintenance_fenced".to_string()])
    );
    assert!(attempt_holds.is_empty());
    assert_eq!(targets.len(), 1025);
    for (index, plan) in plans {
        assert!(
            plan.contains("IndexScan ")
                && plan.contains(&format!("index: {index},"))
                && !plan.contains("TableScan"),
            "{index}: {plan}"
        );
    }
    assert_eq!(products, vec![admitted_product]);
    assert!(private_holds.is_empty());
    assert!(admitted_holds.contains(&RecordId::new(
        "compiler_contribution",
        admitted_contribution.hex()
    )));
    assert_eq!(
        unrelated_holds,
        vec![RecordId::new(
            "compiler_contribution",
            private_contribution.hex()
        )]
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn shared_revision_addresses_coexist_and_exact_views_select_their_payload() {
    let cfg = config();
    let a = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let b = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    assert_eq!(a.database(), b.database());
    assert_eq!(a.database(), &cfg.database);
    let nonce = control::fresh_identity("revision-control").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let (_, av) = contribute(
        &a,
        specification(format!("a-{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let (_, bv) = contribute(
        &b,
        specification(format!("b-{}", nonce.hex())),
        &row(run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let relation = Relation::of::<QualityStep>();
    let mut ar = a
        .scan_rows(&av[QualityStep::NAME], &relation, None, None, &budget)
        .await
        .unwrap();
    let ab = ar.next().await.unwrap().unwrap();
    assert_eq!(
        QualityStep::decode(
            &lctx_native::codec::decode_bodies(&relation, vec![ab], &budget).unwrap()
        )
        .unwrap()[0]
            .value,
        FiniteF64::new(1.0).unwrap()
    );
    assert!(ar.next().await.unwrap().is_none());
    drop(ar);
    let mut br = b
        .scan_rows(&bv[QualityStep::NAME], &relation, None, None, &budget)
        .await
        .unwrap();
    let bb = br.next().await.unwrap().unwrap();
    assert_eq!(
        QualityStep::decode(
            &lctx_native::codec::decode_bodies(&relation, vec![bb], &budget).unwrap()
        )
        .unwrap()[0]
            .value,
        FiniteF64::new(2.0).unwrap()
    );
    assert!(br.next().await.unwrap().is_none());
    drop(br);
    a.abandon().await.unwrap();
    b.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn atomic_field_reads_exclude_foreign_same_key_revisions_before_sorting() {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_model::domain::normalized::links::ImportModuleCandidate;
    use lctx_compiler_store::compiler::NativePredicate;
    let cfg = config();
    let selected = NativeCompilerStore::begin(&cfg, Frontier::Normalized)
        .await
        .unwrap();
    let foreign = NativeCompilerStore::begin(&cfg, Frontier::Normalized)
        .await
        .unwrap();
    let nonce = control::fresh_identity("atomic-revision-control").unwrap();
    let key = |tag: u8| {
        let mut bytes = [tag; 16];
        bytes[..8].copy_from_slice(&nonce.0[..8]);
        bytes
    };
    let a = ImportModuleCandidate {
        assessment: serde_json::from_value(serde_json::json!(key(1))).unwrap(),
        observation: serde_json::from_value(serde_json::json!(key(2))).unwrap(),
        module: serde_json::from_value(serde_json::json!(key(3))).unwrap(),
    };
    let mut b = a.clone();
    b.module = serde_json::from_value(serde_json::json!(key(4))).unwrap();
    assert_eq!(a.id(), b.id());
    let relation = Relation::of::<ImportModuleCandidate>();
    for field in ["assessment", "module"] {
        assert!(lctx_native::schema::atomic_scope_field(
            relation.name(),
            field
        ));
    }
    let mut selected_view = None;
    for (store, record, producer) in [(&selected, &a, "selected"), (&foreign, &b, "foreign")] {
        let mut spec = specification(format!("{producer}-{}", nonce.hex()));
        spec.outputs = BTreeSet::from([relation.name().into()]);
        let owner = store.begin_contribution(spec).await.unwrap();
        store
            .write_batch(
                &owner,
                &relation,
                &ImportModuleCandidate::encode(std::slice::from_ref(record)).unwrap(),
                &native_operation_budget,
            )
            .await
            .unwrap();
        let views = store
            .complete_contribution(owner,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(), &native_operation_budget)
            .await
            .unwrap();
        if producer == "selected" {
            selected_view = Some(views[relation.name()].clone());
        }
    }
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let result = async {
        // Predicates use the native body representation (reference byte arrays),
        // rather than the nominal key's display encoding.
        let bodies = lctx_native::codec::batch_bodies(
            &relation,
            &ImportModuleCandidate::encode(&[a.clone(), b.clone()])?,
        )?;
        let field = |row: usize, name: &str| {
            let Value::Object(body) = &bodies[row] else {
                panic!("native record body")
            };
            body.get(name).unwrap().clone()
        };
        let predicates = [
            NativePredicate::Field {
                field: "assessment".into(),
                values: vec![field(0, "assessment")],
            },
            NativePredicate::Field {
                field: "module".into(),
                values: vec![field(0, "module"), field(1, "module")],
            },
            NativePredicate::FieldSql {
                field: "module".into(),
                values: vec![field(1, "module")],
                sql: "$atomic_residual".into(),
                bindings: Variables::new(),
                preparation: vec!["LET $atomic_residual = true".into()],
            },
        ];
        let mut results = vec![];
        for predicate in predicates {
            let mut rows = selected
                .scan_rows(
                    selected_view.as_ref().unwrap(),
                    &relation,
                    None,
                    Some(predicate),
                    &budget,
                )
                .await?;
            let mut bodies = vec![];
            while let Some(body) = rows.next().await? {
                bodies.push(body);
            }
            results.push(ImportModuleCandidate::decode(
                &lctx_native::codec::decode_bodies(&relation, bodies, &budget)?,
            )?);
        }
        Ok::<_, lctx_model::domain::ModelError>(results)
    }
    .await;
    let selected_cleanup = selected.abandon().await;
    let foreign_cleanup = foreign.abandon().await;
    let results = result.unwrap();
    selected_cleanup.unwrap();
    foreign_cleanup.unwrap();
    assert_eq!(results, vec![vec![a.clone()], vec![a], vec![]]);
}
#[tokio::test(flavor = "multi_thread")]
async fn only_admitted_retention_attaches_to_fresh_attempt_without_payload_replay() {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("retention-control").unwrap();
    let spec = specification(nonce.hex());
    let run = nonce.0[..16].try_into().unwrap();
    let (id, views) = contribute(&source, spec.clone(), &row(run, 3.0), &BTreeMap::new()).await;
    source.retain_product_identity(nonce, id).await.unwrap();
    assert!(
        target
            .attach_retained_product(nonce, &spec, &native_operation_budget)
            .await
            .unwrap()
            .is_none()
    );
    let view = views[QualityStep::NAME].clone();
    let relation = Relation::of::<QualityStep>();
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let mut changed = spec.clone();
    changed.model = ContentHash::of(b"changed model");
    assert!(
        target
            .attach_retained_product(nonce, &changed, &native_operation_budget)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.implementation = ContentHash::of(b"changed implementation");
    assert!(
        target
            .attach_retained_product(nonce, &changed, &native_operation_budget)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.producer.push_str("changed source");
    assert!(
        target
            .attach_retained_product(nonce, &changed, &native_operation_budget)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.inputs.push(
        SourceSnapshot::of_completed_view(&relation, spec.model, &views[QualityStep::NAME])
            .unwrap(),
    );
    assert!(
        target
            .attach_retained_product(nonce, &changed, &native_operation_budget)
            .await
            .unwrap()
            .is_none()
    );
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let mut response=client.query("SELECT VALUE count() FROM compiler_record WHERE semantic_type=$relation AND semantic_key=$key GROUP ALL").bind(("relation",QualityStep::NAME)).bind(("key",row(run,3.0).id().hex())).await.unwrap().check().unwrap();
    let before: Vec<u64> = response.take(0).unwrap();
    let (attached, attached_views, descriptor) = target
        .attach_retained_product(nonce, &spec, &native_operation_budget)
        .await
        .unwrap()
        .unwrap();
    let mut response=client.query("SELECT VALUE count() FROM compiler_record WHERE semantic_type=$relation AND semantic_key=$key GROUP ALL").bind(("relation",QualityStep::NAME)).bind(("key",row(run,3.0).id().hex())).await.unwrap().check().unwrap();
    let after: Vec<u64> = response.take(0).unwrap();
    assert_eq!(before, vec![1]);
    assert_eq!(
        after, before,
        "attachment does not replay or duplicate backing payloads"
    );
    assert_ne!(id, attached);
    assert_eq!(attached_views[QualityStep::NAME], views[QualityStep::NAME]);
    assert_eq!(descriptor.spec, spec);
    let current = target
        .complete_contribution(attached,
            ProviderOutcome::Complete,
            &[relation],
            &BTreeMap::new(), &native_operation_budget)
        .await
        .unwrap();
    assert_eq!(current, attached_views);
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn guarded_reachability_retains_shared_children() {
    let client = lctx_compiler_store::compiler::check_installation(&config())
        .await
        .unwrap();
    let nonce = control::fresh_identity("retirement-control").unwrap();
    let parent = RecordId::new("native_guard", format!("p{}", nonce.hex()));
    let other = RecordId::new("native_guard", format!("o{}", nonce.hex()));
    let child = RecordId::new("native_guard", format!("c{}", nonce.hex()));
    let rows = [&parent, &other, &child]
        .into_iter()
        .map(|id| {
            let mut row = Object::new();
            row.insert("id", id.clone());
            row.insert("revision", 0i64);
            row.insert("retired", false);
    row.insert("phase", "active");
    row.insert("incarnation", 1i64);row.insert("tombstone_revision",0i64);
            Value::Object(row)
        })
        .collect();
    control::ensure_rows(&client, None, rows).await.unwrap();
    control::hold(&client, None, parent.clone(), vec![child.clone()])
        .await
        .unwrap();
    control::hold(&client, None, other.clone(), vec![child.clone()])
        .await
        .unwrap();
    let pass = control::retire_reachable(&client, vec![parent], 16)
        .await
        .unwrap();
    assert_eq!(pass.retired, 1);
    assert_eq!(pass.retained.len(), 1);
    assert!(
        !control::retirement_eligibility(&client, child.clone())
            .await
            .unwrap()
            .eligible
    );
    let pass = control::retire_reachable(&client, vec![other], 16)
        .await
        .unwrap();
    assert_eq!(pass.retired, 2);
    control::ensure_rows(
        &client,
        None,
        vec![{
            let mut row = Object::new();
            row.insert("id", child.clone());
            row.insert("revision", 0i64);
            row.insert("retired", false);
    row.insert("phase", "active");
    row.insert("incarnation", 1i64);row.insert("tombstone_revision",0i64);
            Value::Object(row)
        }],
    )
    .await
    .unwrap();
    control::retire(&client, child).await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn known_statement_abort_is_reconciled_and_attempt_fence_rejects_late_effect() {
    let cfg = config();
    let store = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    assert!(
        control::effect(
            &client,
            Some(store.attempt()),
            "THROW 'expected native control abort'",
            Variables::new()
        )
        .await
        .is_err()
    );
    {
        let mut response = client
            .query("SELECT VALUE id FROM native_effect WHERE attempt=$attempt AND resolved=false")
            .bind((
                "attempt",
                RecordId::new("native_attempt", store.attempt().hex()),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let pending: Vec<RecordId> = response.take(0).unwrap();
        assert!(pending.is_empty());
    }
    assert!(control::close_attempt(&client, store.attempt(), "frozen").await.is_err(),
        "durable frozen is active authority, never a terminal close argument");
    let mut frozen = Variables::new();
    frozen.insert("attempt", RecordId::new("native_attempt",store.attempt().hex()));
    control::effect(&client,Some(store.attempt()),"UPDATE $attempt SET state='frozen',revision+=1 RETURN NONE",frozen).await.unwrap();
    control::close_attempt(&client, store.attempt(), "abandoned").await.unwrap();
    let mut response=client.query("SELECT VALUE state FROM $attempt").bind(("attempt",RecordId::new("native_attempt",store.attempt().hex()))).await.unwrap().check().unwrap();
    let states:Vec<String>=response.take(0).unwrap();assert_eq!(states,["abandoned"]);
    assert!(
        control::effect(
            &client,
            Some(store.attempt()),
            "RETURN NONE",
            Variables::new()
        )
        .await
        .is_err()
    );
    {
        let mut response = client
            .query("SELECT VALUE id FROM native_effect WHERE attempt=$attempt AND resolved=false")
            .bind((
                "attempt",
                RecordId::new("native_attempt", store.attempt().hex()),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let pending: Vec<RecordId> = response.take(0).unwrap();
        assert!(pending.is_empty());
    }
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn retained_membership_deletion_refuses_attachment_and_same_view_revision_conflicts() {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("deleted-membership-control").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let spec = specification(nonce.hex());
    let (id, views) = contribute(&source, spec.clone(), &row(run, 1.0), &BTreeMap::new()).await;
    let relation = Relation::of::<QualityStep>();
    let view = views[QualityStep::NAME].clone();
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.retain_product_identity(nonce, id).await.unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    client
        .query("DELETE compiler_membership WHERE contribution=$owner")
        .bind(("owner", RecordId::new("compiler_contribution", id.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    assert!(
        target.attach_retained_product(nonce, &spec, &native_operation_budget).await.is_err(),
        "retained descriptor alone cannot admit missing compact membership"
    );
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
    let union = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let (_, prior) = contribute(
        &union,
        specification(format!("first{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let id = union
        .begin_contribution(specification(format!("second{}", nonce.hex())))
        .await
        .unwrap();
    union
        .write_batch(
            &id,
            &relation,
            &QualityStep::encode(&[row(run, 2.0)]).unwrap(),
            &native_operation_budget,
        )
        .await
        .unwrap();
    assert!(
        union
            .complete_contribution(id, ProviderOutcome::Complete, &[relation], &prior, &native_operation_budget)
            .await
            .is_err(),
        "one exact view cannot select two revisions of a nominal row"
    );
    let _ = union.abandon().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn retained_empty_output_keeps_exact_zero_cardinality() {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("empty-output-control").unwrap();
    let spec = specification(nonce.hex());
    let relation = Relation::of::<QualityStep>();
    let id = source.begin_contribution(spec.clone()).await.unwrap();
    let views = source
        .complete_contribution(id,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(), &native_operation_budget)
        .await
        .unwrap();
    let view = views[QualityStep::NAME].clone();
    assert_eq!(view.rows, 0);
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.retain_product_identity(nonce, id).await.unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let (_, attached, descriptor) = target
        .attach_retained_product(nonce, &spec, &native_operation_budget)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attached[QualityStep::NAME].rows, 0);
    assert_eq!(descriptor.outputs[QualityStep::NAME].rows, 0);
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn reconciliation_fence_excludes_late_remote_commit_and_retirement_resumes_after_unpin() {
    let cfg = config();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let operation = control::fresh_identity("unknown-effect-control").unwrap();
    let effect = RecordId::new("native_effect", operation.hex());
    let object = RecordId::new("native_guard", format!("unknown{}", operation.hex()));
    client.query("CREATE $effect SET attempt=NONE,request='control',committed=false,resolved=false,revision=0,epoch=1 RETURN NONE").bind(("effect",effect.clone())).await.unwrap().check().unwrap();
    let pending = client.as_ref().clone().begin().await.unwrap();
    pending.query("LET $fence=SELECT * FROM ONLY $effect FOR UPDATE; CREATE $object SET revision=0,retired=false RETURN NONE; UPDATE $effect SET committed=true,resolved=true,revision+=1 RETURN NONE").bind(("effect",effect)).bind(("object",object.clone())).await.unwrap().check().unwrap();
    assert!(!control::reconcile_effect(&client, operation).await.unwrap());
    assert!(
        pending.commit().await.is_err(),
        "fenced unknown transaction cannot commit after reconciliation"
    );
    let mut response = client
        .query("SELECT VALUE id FROM $object")
        .bind(("object", object.clone()))
        .await
        .unwrap()
        .check()
        .unwrap();
    let absent: Vec<RecordId> = response.take(0).unwrap();
    assert!(absent.is_empty());
    let mut row = Object::new();
    row.insert("id", object.clone());
    row.insert("revision", 0i64);
    row.insert("retired", false);
    row.insert("phase", "active");
    row.insert("incarnation", 1i64);row.insert("tombstone_revision",0i64);
    control::ensure_rows(&client, None, vec![Value::Object(row)])
        .await
        .unwrap();
    let pin = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    pin.protect(object.clone()).await.unwrap();
    let first = control::retire_reachable(&client, vec![object], 4)
        .await
        .unwrap();
    assert_eq!(first.retired, 0);
    assert_eq!(first.retained.len(), 1);
    pin.release().await.unwrap();
    let next = control::resume_retirement(&client, first.identity, 4)
        .await
        .unwrap();
    assert_eq!(next.retired, 1);
    assert!(next.remaining.is_empty());
    assert!(next.retained.is_empty());
    drop(pin);

    // Controlled application acknowledgment discard after a real guarded native COMMIT.
    // This exercises reconnect/reconciliation, without claiming network failure injection.
    let committed = control::fresh_identity("committed-effect-ack-discard").unwrap();
    let payload = RecordId::new("native_guard", format!("committed{}", committed.hex()));
    let mut expected = Object::new();
    expected.insert("id", payload.clone());expected.insert("revision",17i64);expected.insert("retired",false);
    expected.insert("phase","active");expected.insert("incarnation",1i64);expected.insert("tombstone_revision",0i64);
    let sql="fn::native_touch($object); CREATE $object CONTENT $row RETURN NONE";
    let mut bindings=Variables::new();bindings.insert("object",payload.clone());bindings.insert("row",expected.clone());
    let (request,bindings)=captured_guard_request(&client,&payload,sql,bindings).await;
    let receipt=RecordId::new("native_effect",request.operation.hex());
    let discarded=control::execute_request(&client,request,None,sql,bindings).await;
    drop(discarded); // Only fresh-session reconciliation below establishes the observed result.
    client.invalidate().await.unwrap();
    drop(client);
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let reader = lctx_native::NativeReader::private(client.clone());
    let result = async {
        let first = control::reconcile_request(&client, request).await?==control::EffectDisposition::Committed;
        let mut bindings = Variables::new();
        bindings.insert("object", payload.clone());
        bindings.insert("effect", receipt.clone());
        let payloads: Vec<Object> = reader
            .query_native("SELECT * FROM $object", bindings.clone())
            .await?;
        let before: Vec<Object> = reader
            .query_native("SELECT * FROM $effect", bindings.clone())
            .await?;
        let second = control::reconcile_request(&client, request).await?==control::EffectDisposition::Committed;
        let after: Vec<Object> = reader
            .query_native("SELECT * FROM $effect", bindings)
            .await?;
        Ok::<_, ModelError>((first, second, payloads, before, after))
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("committed acknowledgment control caller receipt release",control::release_outcome(&client,request,&request.operation.hex(),"caller").await);
    completion.step(
        "committed acknowledgment control payload retirement",
        control::retire(&client, payload).await,
    );
    completion.step(
        "committed acknowledgment control reader close",
        reader.close().await,
    );
    completion.step(
        "committed acknowledgment control session invalidation",
        client
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error))),
    );
    let (first, second, payloads, before, after) =
        lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(
        first && second,
        "committed acknowledgment reconciliation is repeatable after session loss"
    );
    assert_eq!(
        payloads,
        vec![expected],
        "one exact committed payload survives session loss"
    );
    assert_eq!(
        before, after,
        "receipt reconciliation cannot replay the committed decision"
    );
    let [receipt_row] = before.as_slice() else {
        panic!("one exact committed effect receipt");
    };
    assert_eq!(receipt_row.get("id"), Some(&Value::RecordId(receipt)));
    assert_eq!(
        receipt_row.get("request"),
        Some(&Value::String(committed.hex()))
    );
    assert_eq!(receipt_row.get("committed"), Some(&Value::Bool(true)));
    assert_eq!(receipt_row.get("resolved"), Some(&Value::Bool(true)));
    assert_eq!(
        receipt_row.get("revision"),
        Some(&Value::Number(surrealdb::types::Number::Int(1)))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn completed_state_is_exact_binding_closure_in_current_and_cold_owners() {
        let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let cfg = config();
    let store = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("state-closure-control").unwrap();
    let mut run = nonce.0[..16].try_into().unwrap();
    let (_, first) = contribute(
        &store,
        specification(format!("first{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    run[0] ^= 1;
    let (_, second) = contribute(
        &store,
        specification(format!("second{}", nonce.hex())),
        &row(run, 2.0),
        &first,
    )
    .await;
    let relation = Relation::of::<QualityStep>();
    let view = second[QualityStep::NAME].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &relation,
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    assert_eq!(
        store.views().await.unwrap().len(),
        1,
        "unbound singleton view is not portable state authority"
    );
    let identity = store.completed_state(&native_operation_budget).await.unwrap();
    let bindings = store.bindings().await.unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let cold = NativeCompilerStore::from_publication(client,
        cfg.namespace.clone(),
        cfg.database.clone(),
        bindings, &native_operation_budget)
    .await
    .unwrap();
    cold.verify_state(&native_operation_budget).await.unwrap();
    assert_eq!(cold.completed_state(&native_operation_budget).await.unwrap(), identity);
    run[0] ^= 2;
    let (_, unbound) = contribute(
        &store,
        specification(format!("unbound{}", nonce.hex())),
        &row(run, 3.0),
        &BTreeMap::new(),
    )
    .await;
    assert_eq!(
        store.completed_state(&native_operation_budget).await.unwrap(),
        identity,
        "unbound completed rows remain outside captured state"
    );
    let detached = tempfile::tempdir().unwrap();
    assert_eq!(
        store
            .export_state(&detached.path().join("before.jsonl"), &native_operation_budget)
            .await
            .unwrap(),
        identity
    );
    let view = unbound[QualityStep::NAME].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &relation,
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    let changed = store.completed_state(&native_operation_budget).await.unwrap();
    assert_ne!(
        changed, identity,
        "a fresh operation captures changed bindings"
    );
    assert_eq!(
        store
            .export_state(&detached.path().join("after.jsonl"), &native_operation_budget)
            .await
            .unwrap(),
        changed
    );
    assert_eq!(
        cold.completed_state(&native_operation_budget).await.unwrap(),
        identity,
        "published binding inventory remains exact"
    );
    store.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn retirement_uses_exact_guards_without_blocking_on_unrelated_live_effects() {
    let client = lctx_compiler_store::compiler::check_installation(&config())
        .await
        .unwrap();
    let nonce = control::fresh_identity("parallel-retirement-control").unwrap();
    let unrelated = RecordId::new("native_effect", nonce.hex());
    let object = RecordId::new("native_guard", format!("late{}", nonce.hex()));
    let guard = RecordId::new(
        "native_guard",
        ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex(),
    );
    client.query("CREATE $effect SET attempt=NONE,request='unrelated-active-control',committed=false,resolved=false,revision=0,epoch=1 RETURN NONE").bind(("effect",unrelated)).await.unwrap().check().unwrap();
    let pending = client.as_ref().clone().begin().await.unwrap();
    pending.query("UPSERT $guard SET revision=(revision ?? 0)+1,retired=(retired ?? false) RETURN NONE; CREATE $object SET revision=0,retired=false RETURN NONE").bind(("guard",guard)).bind(("object",object.clone())).await.unwrap().check().unwrap();
    assert!(
        control::retirement_eligibility(&client, object.clone())
            .await
            .unwrap()
            .eligible,
        "an unrelated active operation is not an object dependency"
    );
    control::retire(&client, object.clone()).await.unwrap();
    assert!(
        pending.commit().await.is_err(),
        "late immutable insertion must conflict with retirement's exact guard"
    );
    control::reconcile_effect(&client, nonce).await.unwrap();
    let mut response = client
        .query("SELECT VALUE id FROM $object")
        .bind(("object", object))
        .await
        .unwrap()
        .check()
        .unwrap();
    let rows: Vec<RecordId> = response.take(0).unwrap();
    assert!(rows.is_empty());
}

#[tokio::test(flavor="multi_thread")]
async fn completed_contribution_refuses_new_membership_and_outgoing_holds_but_accepts_exact_replay(){
    let cfg=config();let store=NativeCompilerStore::begin(&cfg,Frontier::Facts).await.unwrap();
    let nonce=control::fresh_identity("completed-incidence-control").unwrap();
    let (contribution,_)=contribute(&store,specification(format!("incidence{}",nonce.hex())),&row(nonce.0[..16].try_into().unwrap(),1.0),&BTreeMap::new()).await;
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let owner=RecordId::new("compiler_contribution",contribution.hex());
    let result=async{
        let mut response=client.query("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner LIMIT 1; SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner LIMIT 1").bind(("owner",owner.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let memberships:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let holds:Vec<Object>=response.take(1).map_err(ModelError::codec)?;
        let [membership]=memberships.as_slice()else{return Err(ModelError::Schema("completed incidence control membership"));};
        let [hold]=holds.as_slice()else{return Err(ModelError::Schema("completed incidence control hold"));};
        control::ensure_rows(&client,Some(store.attempt()),vec![Value::Object(membership.clone())]).await?;
        let Some(Value::RecordId(target))=hold.get("object")else{return Err(ModelError::Schema("completed incidence control hold target"));};
        control::hold(&client,Some(store.attempt()),owner.clone(),vec![target.clone()]).await?;
        let mut changed=membership.clone();changed.insert("id",RecordId::new("compiler_membership",format!("new{}",nonce.hex())));changed.insert("semantic_key",format!("new{}",nonce.hex()));
        let membership_error=control::ensure_rows(&client,Some(store.attempt()),vec![Value::Object(changed)]).await.unwrap_err();
        // The attempt exists and is not already an outgoing target of this contribution.
        let hold_error=control::hold(&client,Some(store.attempt()),owner.clone(),vec![RecordId::new("native_attempt",store.attempt().hex())]).await.unwrap_err();
        Ok::<_,ModelError>((membership_error.to_string(),hold_error.to_string()))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();completion.step("completed incidence owner abandonment",store.abandon().await);completion.step("completed incidence session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (membership,hold)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(membership.contains("native completed contribution refuses changed incidence"),"{membership}");
    assert!(hold.contains("native completed contribution refuses changed incidence"),"{hold}");
}

#[tokio::test(flavor = "multi_thread")]
async fn captured_incarnation_fences_original_request_while_new_plan_and_terminal_owners_are_distinct() {
    let cfg=config();
    let old=NativeCompilerStore::begin(&cfg,Frontier::Facts).await.unwrap();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let nonce=control::fresh_identity("reactivation-control").unwrap();
    let object=RecordId::new("native_guard",format!("reactivate{}",nonce.hex()));
    let mut row=Object::new();row.insert("id",object.clone());row.insert("revision",0i64);row.insert("retired",false);
    row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);
    let value=Value::Object(row);
    control::ensure_rows(&client,Some(old.attempt()),vec![value.clone()]).await.unwrap();
    let sql="IF (SELECT VALUE id FROM ONLY $object)=NONE { THROW 'captured object unavailable'; }; RETURN NONE";
    let mut vars=Variables::new();vars.insert("object",object.clone());
    let (original,bindings)=captured_guard_request(&client,&object,sql,vars).await;
    let first=control::retire_reachable(&client,vec![object.clone()],4).await.unwrap();assert_eq!(first.retired,1);
    let fresh=NativeCompilerStore::begin(&cfg,Frontier::Facts).await.unwrap();
    control::ensure_rows(&client,Some(fresh.attempt()),vec![value.clone()]).await.unwrap();
    for _ in 0..2 {
        assert!(control::execute_request(&client,original,Some(old.attempt()),sql,bindings.clone()).await.is_err(),"the original request never refreshes its captured incarnation");
        assert_eq!(control::request_disposition(&client,original).await.unwrap(),control::EffectDisposition::FencedUncommitted);
    }
    // The format6 contract replaces cross-owner retired_through ordering. A separate
    // plan can explicitly observe the new active incarnation while its owner is open.
    control::ensure_rows(&client,Some(old.attempt()),vec![value.clone()]).await.unwrap();
    let old_pin=control::ReaderPin::acquire(client.clone(),&[]).await.unwrap();
    old_pin.protect(object.clone()).await.unwrap();old_pin.release().await.unwrap();
    assert!(old_pin.protect(object.clone()).await.is_err(),"released pin authority stays terminal");
    control::release_outcome(&client,original,&original.operation.hex(),"caller").await.unwrap();
    let attempt=old.attempt();old.abandon().await.unwrap();
    assert!(control::ensure_rows(&client,Some(attempt),vec![value]).await.is_err(),"terminal attempt authority stays fenced");
    let second=control::retire_reachable(&client,vec![object],4).await.unwrap();
    assert_ne!(second.identity,first.identity);assert_eq!(second.retired,1);
    fresh.abandon().await.unwrap();client.invalidate().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn retained_product_keeps_exact_prerequisite_closure_after_origin_owner_retirement() {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let cfg = config();
    let origin = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("retained-prerequisite-control").unwrap();
    let relation = Relation::of::<QualityStep>();
    let run = nonce.0[..16].try_into().unwrap();
    let (_, base) = contribute(
        &origin,
        specification(format!("base{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let input = SourceSnapshot::of_completed_view(
        &relation,
        specification(String::new()).model,
        &base[QualityStep::NAME],
    )
    .unwrap();
    let mut derived_spec = specification(format!("derived{}", nonce.hex()));
    derived_spec.inputs = vec![input];
    let mut derived_run = run;
    derived_run[0] ^= 1;
    let (derived, derived_views) = contribute(
        &origin,
        derived_spec.clone(),
        &row(derived_run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    let view = derived_views[QualityStep::NAME].clone();
    origin
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, derived_spec.model, &view)
                .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    origin
        .retain_product_identity(nonce, derived)
        .await
        .unwrap();
    origin.mark_attempt_admitted().await.unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let owner = RecordId::new("native_guard", nonce.hex());
    let mut owner_row = Object::new();
    owner_row.insert("id", owner.clone());
    owner_row.insert("revision", 0i64);
    owner_row.insert("retired", false);
    owner_row.insert("phase", "active");
    owner_row.insert("incarnation", 1i64);owner_row.insert("tombstone_revision",0i64);
    control::ensure_rows(
        &client,
        Some(origin.attempt()),
        vec![Value::Object(owner_row)],
    )
    .await
    .unwrap();
    control::hold(
        &client,
        Some(origin.attempt()),
        owner.clone(),
        vec![
            RecordId::new(
                "compiler_view",
                derived_views[QualityStep::NAME].identity.hex(),
            ),
            RecordId::new("compiler_view", base[QualityStep::NAME].identity.hex()),
        ],
    )
    .await
    .unwrap();
    origin.abandon().await.unwrap();
    let retired = control::retire_reachable(&client, vec![owner], 64)
        .await
        .unwrap();
    assert!(retired.retired >= 1);
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let (attached, _, descriptor) = target
        .attach_retained_product(nonce, &derived_spec, &native_operation_budget)
        .await
        .unwrap()
        .expect(
            "admitted product retains its prerequisite closure independently of the origin owner",
        );
    assert_eq!(descriptor.spec.inputs, derived_spec.inputs);
    let mut response = client
        .query("SELECT VALUE object FROM native_hold WHERE owner=$owner")
        .bind((
            "owner",
            RecordId::new("compiler_contribution", attached.hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let targets: Vec<RecordId> = response.take(0).unwrap();
    assert!(targets.contains(&RecordId::new(
        "compiler_view",
        base[QualityStep::NAME].identity.hex()
    )));
    let mut rows = target
        .scan_rows(
            &base[QualityStep::NAME],
            &relation,
            None,
            None,
            &ResourceBudget::fixed(64 << 20).unwrap(),
        )
        .await
        .unwrap();
    assert!(rows.next().await.unwrap().is_some());
    assert!(rows.next().await.unwrap().is_none());
    drop(rows);
    target.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "database-wide backup exclusion requires explicit owned-service maintenance"]
async fn backup_hold_excludes_retirement_under_exclusive_maintenance() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let object=RecordId::new("native_guard",control::fresh_identity("backup-exclusion-control").unwrap().hex());
    let mut backup=None;
    let mut created=false;
    let mut registered=None;
    let result=async {
        // Only this fixture can acquire under the exclusive host lease. The
        // actual backup guard requires admission; exclusion runs after closing it.
        lctx_compiler_store::compiler::open_admission(&cfg).await?;
        let mut row=Object::new();row.insert("id",object.clone());row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);
        created=true;
        control::ensure_rows(&client,None,vec![Value::Object(row)]).await?;
        registered=Some(control::retire_reachable(&client,vec![object.clone()],1).await?.identity);
        backup=Some(control::acquire_backup_hold(&client).await?);
        lctx_compiler_store::compiler::close_admission(&cfg).await?;
        let eligibility=control::retirement_eligibility(&client,object.clone()).await?;
        let retirement=control::resume_retirement(&client,registered.expect("registered exact synthetic root"),4).await;
        Ok::<_,ModelError>((eligibility,retirement))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("backup control close after acquisition",lctx_compiler_store::compiler::close_admission(&cfg).await);
    if let Some(backup)=backup {completion.step("backup control hold release",control::release_backup_hold(&client,backup).await);}
    if let Some(job)=registered {completion.step("backup control synthetic object retirement",control::resume_retirement(&client,job,4).await.map(|_|()));} else if created {completion.step("backup control cleanup admission",lctx_compiler_store::compiler::open_admission(&cfg).await);completion.step("backup control synthetic object retirement",control::retire(&client,object).await);}
    completion.step("backup control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("backup control session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (eligibility,retirement)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(!eligibility.eligible);
    assert!(eligibility.reasons.contains(&"active native backup hold".to_string()));
    assert!(retirement.unwrap_err().to_string().contains("native retirement backup hold"),"the dedicated backup gate must cause refusal");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "abandoned reader recovery requires explicit owned-service maintenance"]
async fn maintenance_reconciliation_requires_drain_proof_and_fences_only_named_clients() {
    let cfg=maintenance_config();
    let path=std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("explicit validation maintenance installer configuration");
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let object=RecordId::new("native_guard",control::fresh_identity("maintenance-reader-target").unwrap().hex());
    let mut created=false;
    let mut first=None;
    let mut unrelated=None;
    let mut backup=None;
    let result=async {
        lctx_compiler_store::compiler::open_admission(&cfg).await?;
        let mut row=Object::new();row.insert("id",object.clone());row.insert("revision",0i64);row.insert("retired",false);row.insert("phase","active");row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);
        created=true;
        control::ensure_rows(&client,None,vec![Value::Object(row)]).await?;
        first=Some(control::ReaderPin::acquire(client.clone(),&[]).await?);
        unrelated=Some(control::ReaderPin::acquire(client.clone(),&[]).await?);
        backup=Some(control::acquire_backup_hold(&client).await?);
        let first=first.as_ref().expect("recorded first acquisition");
        let unrelated=unrelated.as_ref().expect("recorded unrelated acquisition");
        let backup=backup.expect("recorded backup acquisition");
        first.protect(object.clone()).await?;
        let pins=[first.identity];let holds=[backup];
        let open_refused=lctx_compiler_store::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await.is_err();
        let mut ordinary=RuntimeConfig::read(std::path::Path::new(&path)).map_err(lctx_model::domain::ModelError::codec)?;ordinary.authentication=lctx_native::AuthenticationScope::Database;
        let authority_refused=lctx_compiler_store::compiler::reconcile_maintenance(&ordinary,&pins,&holds,true).await.is_err();
        lctx_compiler_store::compiler::close_admission(&cfg).await?;
        let live_refused=lctx_compiler_store::compiler::reconcile_maintenance(&cfg,&pins,&holds,false).await.is_err();
        let mut response=client.query("SELECT VALUE released FROM ONLY $pin").bind(("pin",RecordId::new("native_pin",first.identity.hex()))).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let before:Option<bool>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;
        lctx_compiler_store::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await?;
        lctx_compiler_store::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await?;
        let unknown=control::fresh_identity("missing-maintenance-pin")?;
        let missing_refused=match lctx_compiler_store::compiler::reconcile_maintenance(&cfg,&[first.identity,unknown],&[],true).await {Err(lctx_model::domain::ModelError::Completion(outcome))=>outcome.completion.committed.iter().any(|effect|effect.kind=="reconciled native reader pin" && effect.identity==first.identity.hex()),_=>false};
        let fenced=first.protect(object.clone()).await.is_err();
        let mut response=client.query("SELECT VALUE released FROM ONLY $pin; SELECT VALUE active FROM ONLY $backup; SELECT VALUE released FROM ONLY $unrelated").bind(("pin",RecordId::new("native_pin",first.identity.hex()))).bind(("backup",RecordId::new("native_backup_hold",backup.hex()))).bind(("unrelated",RecordId::new("native_pin",unrelated.identity.hex()))).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let released:Option<bool>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;let active:Option<bool>=response.take(1).map_err(lctx_model::domain::ModelError::codec)?;let unrelated_released:Option<bool>=response.take(2).map_err(lctx_model::domain::ModelError::codec)?;
        Ok::<_,lctx_model::domain::ModelError>((open_refused,authority_refused,live_refused,before,missing_refused,fenced,released,active,unrelated_released))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("maintenance control close after acquisition",lctx_compiler_store::compiler::close_admission(&cfg).await);
    if let Some(first)=first {
        let released=first.release().await;
        if released.is_err(){first.retain_unknown();}
        completion.step("maintenance control first pin cleanup",released);
    }
    if let Some(unrelated)=unrelated {
        let released=unrelated.release().await;
        if released.is_err(){unrelated.retain_unknown();}
        completion.step("maintenance control unrelated pin cleanup",released);
    }
    if let Some(backup)=backup {completion.step("maintenance control backup cleanup",control::release_backup_hold(&client,backup).await);}
    if created {completion.step("maintenance control target retirement",control::retire(&client,object).await);}
    completion.step("maintenance control admission restoration",if admission_was_open {lctx_compiler_store::compiler::open_admission(&cfg).await} else {lctx_compiler_store::compiler::close_admission(&cfg).await});
    completion.step("maintenance control session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (open_refused,authority_refused,live_refused,before,missing_refused,fenced,released,active,unrelated_released)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(open_refused && authority_refused && live_refused && missing_refused && fenced);
    assert_eq!(before, Some(false));
    assert_eq!(released, Some(true));
    assert_eq!(active, Some(false));
    assert_eq!(unrelated_released, Some(false));
}

#[tokio::test(flavor="multi_thread")]
#[ignore = "all-stripe validity/drain transition requires explicit owned-service maintenance"]
async fn frozen_attempt_blocks_validity_and_cut_until_installer_fences_and_drains_it() {
    let cfg=maintenance_config();
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let (admission_was_open,_)=maintenance_snapshot(&client,None).await;
    let budget=ResourceBudget::fixed(256<<20).unwrap();
    let mut attempt=None;
    let result=async {
        lctx_native::installation::open_admission(&cfg).await?;
        let issuance=control::IssuanceEra::capture(&client).await?;
        let identity=control::issue_attempt(&client,issuance.generation).await?;attempt=Some(identity);
        let owner=RecordId::new("native_attempt",identity.hex());
        let mut variables=Variables::new();variables.insert("attempt",owner.clone());
        control::effect(&client,Some(identity),"UPDATE $attempt SET state='frozen',revision+=1 RETURN NONE",variables).await?;
        let mut response=client.query("SELECT VALUE validity_epoch FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let epochs:Vec<i64>=response.take(0).map_err(ModelError::codec)?;
        let [epoch]=epochs.as_slice()else{return Err(ModelError::Schema("validity control epoch"));};
        lctx_native::installation::close_admission(&cfg).await?;
        let validity_refused=control::advance_validity_epoch_native(&client,*epoch,&budget).await.is_err();
        let cut_refused=control::cut_era(&client).await.is_err();
        lctx_native::installation::drain_installation(&cfg).await?;
        let mut response=client.query("SELECT VALUE state FROM $attempt; SELECT VALUE validity_epoch FROM native_installation:current").bind(("attempt",owner)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let states:Vec<String>=response.take(0).map_err(ModelError::codec)?;
        let unchanged:Vec<i64>=response.take(1).map_err(ModelError::codec)?;
        control::advance_validity_epoch_native(&client,*epoch,&budget).await?;
        Ok::<_,ModelError>((validity_refused,cut_refused,states,unchanged,epochs))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    if let Some(attempt)=attempt {completion.step("frozen maintenance exact attempt close",cleanup_owned_attempt(&client,attempt).await);}
    completion.step("frozen maintenance admission restoration",if admission_was_open {lctx_native::installation::open_admission(&cfg).await}else{lctx_native::installation::close_admission(&cfg).await});
    completion.step("frozen maintenance session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (validity_refused,cut_refused,states,unchanged,epochs)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(validity_refused && cut_refused,"an active frozen owner blocks both all-stripe transitions");
    assert_eq!(states,["abandoned"]);assert_eq!(unchanged,epochs,"refused validity transition must not advance trust");
}

#[tokio::test(flavor="multi_thread")]
async fn committed_public_replay_retains_original_owner_without_requiring_live_owner() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let issuance=control::IssuanceEra::capture(&client).await.unwrap();
    let original=control::issue_attempt(&client,issuance.generation).await.unwrap();
    let other=control::issue_attempt(&client,issuance.generation).await.unwrap();
    let object=RecordId::new("native_guard",control::fresh_identity("replay-owner-payload").unwrap().hex());
    let sql="UPDATE $object SET revision+=1 RETURN NONE";
    let mut requests=Vec::new();
    let result=async {
        let mut row=Object::new();row.insert("id",object.clone());row.insert("phase","active");row.insert("retired",false);row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);row.insert("revision",0i64);
        control::ensure_rows(&client,Some(original),vec![Value::Object(row)]).await?;
        let mut input=Variables::new();input.insert("object",object.clone());
        let (request,bindings)=captured_guard_request(&client,&object,sql,input.clone()).await;
        requests.push(request);
        control::execute_request(&client,request,Some(original),sql,bindings.clone()).await?;
        let mut response=client.query("SELECT * FROM $effect").bind(("effect",RecordId::new("native_effect",request.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let before:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let other_refused=control::execute_request(&client,request,Some(other),sql,bindings.clone()).await.is_err();
        let installation_refused=control::execute_request(&client,request,None,sql,bindings.clone()).await.is_err();
        control::close_attempt(&client,original,"abandoned").await?;
        // Terminal owner state is irrelevant to replay of this original committed receipt.
        control::execute_request(&client,request,Some(original),sql,bindings).await?;
        let mut response=client.query("SELECT * FROM $effect; SELECT VALUE revision FROM $object").bind(("effect",RecordId::new("native_effect",request.operation.hex()))).bind(("object",object.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let after:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let revisions:Vec<i64>=response.take(1).map_err(ModelError::codec)?;
        let (installation,bindings)=captured_guard_request(&client,&object,sql,input).await;
        requests.push(installation);
        control::execute_request(&client,installation,None,sql,bindings.clone()).await?;
        let attempt_refused=control::execute_request(&client,installation,Some(other),sql,bindings.clone()).await.is_err();
        control::execute_request(&client,installation,None,sql,bindings).await?;
        let mut response=client.query("SELECT VALUE revision FROM $object").bind(("object",object.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let final_revisions:Vec<i64>=response.take(0).map_err(ModelError::codec)?;
        Ok::<_,ModelError>((other_refused,installation_refused,attempt_refused,before,after,revisions,final_revisions))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    for request in requests {completion.step("replay owner exact caller reference release",control::release_outcome(&client,request,&request.operation.hex(),"caller").await);}
    completion.step("replay exact original attempt close",cleanup_owned_attempt(&client,original).await);
    completion.step("replay exact alternate attempt close",cleanup_owned_attempt(&client,other).await);
    completion.step("replay exact payload retirement",control::retire(&client,object).await);
    completion.step("replay owner session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (other_refused,installation_refused,attempt_refused,before,after,revisions,final_revisions)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(other_refused && installation_refused && attempt_refused,"committed outcome does not transfer original owner authority");
    assert_eq!(before,after,"healthy outcome replay leaves original receipt bytes unchanged");
    assert_eq!(revisions,[1]);assert_eq!(final_revisions,[2],"each original operation mutates once regardless of replay");
}

#[tokio::test(flavor="multi_thread")]
async fn retained_committed_guarded_bytes_replay_without_granting_legacy_execution() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let issuance=control::IssuanceEra::capture(&client).await.unwrap();
    let attempt=control::issue_attempt(&client,issuance.generation).await.unwrap();
    let owner=RecordId::new("native_attempt",attempt.hex());
    let object=RecordId::new("native_guard",control::fresh_identity("retained-guarded-replay").unwrap().hex());
    let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex());
    let sql="UPDATE $object SET revision+=1 RETURN NONE";
    let mut bindings=Variables::new();bindings.insert("object",object.clone());bindings.insert("guards",vec![guard]);
    assert!(bindings.get("__object_expectations").is_none());
    let digest=ContentHash::of(&serde_json::to_vec(&(sql,&bindings)).unwrap());
    let committed=control::NativeRequest::issue(&client,digest).await.unwrap();
    let pending=control::NativeRequest::issue(&client,digest).await.unwrap();
    let unissued=control::NativeRequest::issue(&client,digest).await.unwrap();
    let mut seeded=Vec::new();
    let result=async {
        let mut row=Object::new();row.insert("id",object.clone());row.insert("phase","active");row.insert("retired",false);row.insert("incarnation",1i64);row.insert("tombstone_revision",0i64);row.insert("revision",0i64);
        control::ensure_rows(&client,Some(attempt),vec![Value::Object(row)]).await?;
        // Synthetic retained receipt shapes exercise the outcome-only boundary. No
        // format5 runtime, old scalar SQL, or source5 installation-NONE proof is claimed.
        for (request,is_committed) in [(committed,true),(pending,false)] {
            let mut receipt=Object::new();receipt.insert("id",RecordId::new("native_effect",request.operation.hex()));
            receipt.insert("attempt",owner.clone());receipt.insert("owner",owner.clone());receipt.insert("owner_kind","attempt");
            receipt.insert("generation",issuance.generation.hex());receipt.insert("era",issuance.era);receipt.insert("epoch",1i64);
            receipt.insert("request",request.request.hex());receipt.insert("committed",is_committed);receipt.insert("resolved",is_committed);receipt.insert("revision",if is_committed {1i64}else{0i64});
            let reference=RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(request.operation,request.operation.hex(),"caller")).unwrap()).hex());
            let mut seed=Variables::new();seed.insert("receipt",receipt);seed.insert("reference",reference);seed.insert("caller",request.operation.hex());seed.insert("object",object.clone());seed.insert("committed",is_committed);
            seeded.push(request); // Retain exact cleanup identity before the first possible seed effect.
            control::effect(&client,Some(attempt),"fn::native_incidence_insert($receipt); CREATE $receipt.id CONTENT $receipt RETURN NONE; fn::native_touch($reference); CREATE $reference SET effect=$receipt.id,owner=$caller,kind='caller' RETURN NONE; IF $committed { fn::native_touch($object); UPDATE $object SET revision=1 RETURN NONE; };",seed).await?;
        }
        let mut response=client.query("SELECT * FROM $effects").bind(("effects",vec![RecordId::new("native_effect",committed.operation.hex()),RecordId::new("native_effect",pending.operation.hex())])).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let before:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        control::execute_request(&client,committed,Some(attempt),sql,bindings.clone()).await?;
        let wrong_owner_refused=control::execute_request(&client,committed,None,sql,bindings.clone()).await.is_err();
        let pending_refused=control::execute_request(&client,pending,Some(attempt),sql,bindings.clone()).await.is_err();
        let unissued_refused=control::execute_request(&client,unissued,Some(attempt),sql,bindings.clone()).await.is_err();
        let changed_digest_refused=control::execute_request(&client,committed,Some(attempt),"RETURN NONE",bindings.clone()).await.is_err();
        let mut response=client.query("SELECT * FROM $effects; SELECT VALUE revision FROM $object; SELECT VALUE id FROM $unissued; SELECT VALUE id FROM native_outcome_ref WITH INDEX effect_outcome_refs WHERE effect=$unissued").bind(("effects",vec![RecordId::new("native_effect",committed.operation.hex()),RecordId::new("native_effect",pending.operation.hex())])).bind(("object",object.clone())).bind(("unissued",RecordId::new("native_effect",unissued.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let after:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let revisions:Vec<i64>=response.take(1).map_err(ModelError::codec)?;
        let intents:Vec<RecordId>=response.take(2).map_err(ModelError::codec)?;
        let references:Vec<RecordId>=response.take(3).map_err(ModelError::codec)?;
        Ok::<_,ModelError>((wrong_owner_refused,pending_refused,unissued_refused,changed_digest_refused,before,after,revisions,intents,references))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    for request in seeded {
        completion.step("guarded replay exact synthetic original reconciliation",control::reconcile_request(&client,request).await.map(|_|()));
        completion.step("guarded replay exact caller reference release",control::release_outcome(&client,request,&request.operation.hex(),"caller").await);
    }
    completion.step("guarded replay exact attempt close",cleanup_owned_attempt(&client,attempt).await);
    completion.step("guarded replay exact payload retirement",control::retire(&client,object).await);
    completion.step("guarded replay session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (wrong_owner_refused,pending_refused,unissued_refused,changed_digest_refused,before,after,revisions,intents,references)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert!(wrong_owner_refused && pending_refused && unissued_refused && changed_digest_refused);
    assert_eq!(before,after,"outcome replay and refused legacy execution preserve exact persisted receipts");
    assert_eq!(revisions,[1],"no old guarded payload executes");assert!(intents.is_empty() && references.is_empty(),"unissued refusal creates no original intent or retention reference");
}

#[tokio::test(flavor="multi_thread")]
async fn duplicate_same_incarnation_retirement_supersedes_without_second_retired_outcome() {
    let client=lctx_compiler_store::compiler::check_installation(&config()).await.unwrap();
    let object=RecordId::new("native_guard",control::fresh_identity("duplicate-retirement-nominee").unwrap().hex());
    let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex());
    let mut jobs=Vec::new();
    let result=async {
        let mut row=Object::new();row.insert("id",object.clone());row.insert("phase","active");row.insert("retired",false);row.insert("incarnation",1i64);row.insert("revision",0i64);row.insert("tombstone_revision",0i64);
        control::ensure_rows(&client,None,vec![Value::Object(row)]).await?;
        let first=control::retire_reachable(&client,vec![object.clone()],1).await?;jobs.push(first.identity);
        // Both invocations register the same actual incarnation. The first owns its
        // claim, while the second has an independently retained queued nomination.
        let second=control::retire_reachable(&client,vec![object.clone()],1).await?;jobs.push(second.identity);
        let completed=control::resume_retirement(&client,first.identity,1).await?;
        let mut response=client.query("SELECT * FROM $guard; SELECT examined,retired FROM $job").bind(("guard",guard.clone())).bind(("job",RecordId::new("native_retirement",second.identity.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let before:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let counts_before:Vec<Object>=response.take(1).map_err(ModelError::codec)?;
        let duplicate=control::resume_retirement(&client,second.identity,1).await?;
        let mut response=client.query("SELECT * FROM $guard; SELECT examined,retired FROM $job; SELECT state,incarnation FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job; SELECT VALUE id FROM $object").bind(("guard",guard)).bind(("job",RecordId::new("native_retirement",second.identity.hex()))).bind(("object",object.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let after:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let counts_after:Vec<Object>=response.take(1).map_err(ModelError::codec)?;
        let items:Vec<Object>=response.take(2).map_err(ModelError::codec)?;
        let payload:Vec<RecordId>=response.take(3).map_err(ModelError::codec)?;
        Ok::<_,ModelError>((first,second,completed,duplicate,before,after,counts_before,counts_after,items,payload))
    }.await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    for job in jobs {completion.step("duplicate nominee exact job completion",control::resume_retirement(&client,job,4).await.map(|_|()));}
    completion.step("duplicate nominee exact object cleanup",control::retire(&client,object).await);
    completion.step("duplicate nominee session invalidation",client.invalidate().await.map_err(ModelError::codec));
    let (first,second,completed,duplicate,before,after,counts_before,counts_after,items,payload)=lctx_model::domain::completion::complete(result,completion).unwrap();
    assert_eq!(first.retired,0);assert_eq!(second.retired,0);assert_eq!(second.retained.len(),1);
    assert_eq!(completed.retired,1);assert_eq!(duplicate.retired,0);assert!(duplicate.remaining.is_empty() && duplicate.retained.is_empty());
    assert_eq!(before,after,"duplicate nomination cannot reopen phase, mutate tombstone or change physical guard revision");
    assert_eq!(counts_before,counts_after,"duplicate nomination adds no examined/retired outcome");
    assert!(payload.is_empty());let [item]=items.as_slice()else{panic!("one exact nominee")};
    assert_eq!(item.get("state"),Some(&Value::String("superseded".into())));
    assert_eq!(item.get("incarnation"),Some(&Value::Number(surrealdb::types::Number::Int(1))));
}
