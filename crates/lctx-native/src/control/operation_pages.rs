//! One durable discovery root for a real producer, with direct atomic native pages.
//! Pages never allocate generic host recovery envelopes or reconstruct lost payloads.
use super::*;
use lctx_model::domain::{completion::{Completion,LocalState,RemoteState,complete},resources::ResourceBudget};
use tokio::sync::Mutex;

pub struct OperationWriter {
    owner: RecoveryOwner,
    segment: Option<RecordId>,
    state: Mutex<State>,
    budget:ResourceBudget,
    _retained:Box<dyn lctx_model::domain::resources::Reservation>,
}
enum State {
    Unopened,
    Open { request:NativeRequest,pending:PendingRecovery,scope:Arc<RecoveryScope>,next:i64,failed:bool },
    Terminal,
    FailedTerminal { cause:Arc<ModelError> },
}
impl OperationWriter {
    pub fn new(owner:RecoveryOwner,segment:Option<RecordId>,budget:&ResourceBudget)->Result<Self,ModelError> {
        if !matches!(owner,RecoveryOwner::Installation|RecoveryOwner::OpenAttempt(_)|RecoveryOwner::FrozenAttempt(_)){return Err(ModelError::Schema("native operation requires an actual producer owner"));}
        let retained=budget.reserve("native producer operation state",4096usize.saturating_add(segment.as_ref().map_or(0,crate::selection::node_bytes).saturating_mul(4)))?;
        Ok(Self{owner,segment,state:Mutex::new(State::Unopened),budget:budget.clone(),_retained:retained})
    }
    pub(super) fn workspace_bytes(&self)->Result<usize,ModelError>{
        let request=NativeRequest{issuance:IssuanceEra{generation:ContentHash([0;32]),era:i64::MAX},operation:ContentHash([0;32]),request:ContentHash([0;32])};
        let (query,bindings)=page_plan(request,request,i64::MAX,&self.owner,"",Variables::new())?;
        immutable_window::plan_bytes(&query,&bindings)
    }
    pub async fn apply(&self,client:&Surreal<Client>,sql:&str,mut variables:Variables,budget:&ResourceBudget)->Result<(),ModelError>{
        if !self.budget.shares_pool(budget){return Err(ModelError::Conflict("native operation allocation owner changed"));}
        let mut state=self.state.lock().await;
        match &*state {
            State::Terminal=>return Err(ModelError::Conflict("native operation is terminal")),
            State::FailedTerminal{cause}=>return Err(ModelError::SharedCause(cause.clone())),
            _=>{}
        }
        if matches!(*state,State::Unopened){
            let scope=capture_recovery_scope().ok_or(ModelError::Conflict("native operation requires actual recovery owner"))?;
            // Recovery bookkeeping belongs to the factory's independently admitted
            // metadata pool. Payload/state admission belongs to this operation's
            // caller pool (checked above); those legitimate pools need not match.
            if !std::ptr::eq(scope.client().as_ref(),client){return Err(ModelError::Conflict("native operation client owner"));}
            let owner=self.owner.effect_owner();
            let issuance=if matches!(self.owner,RecoveryOwner::Installation){IssuanceEra::capture(client).await?}else{authorization::owner_issuance(client,&owner).await?};
            let request=NativeRequest{issuance,operation:fresh_identity("native-producer-operation")?,request:ContentHash::of(&serde_json::to_vec(&("native-operation/v1",&self.owner,&self.segment,issuance)).map_err(ModelError::codec)?)};
            // This write precedes even root creation. An absent root is never
            // inferred safe: reconciliation installs a same-key native fence.
            let pending=scope.prepare_operation_root(request,self.owner.clone())?;
            *state=State::Open{request,pending,scope,next:0,failed:true};
            let result=create_root(client,request,&self.owner,self.segment.as_ref()).await;
            if let Err(error)=result{return Err(unknown(request,error));}
            if let State::Open{failed,..}=&mut *state{*failed=false;}
        }
        let State::Open{request,pending,scope,next,failed}=&mut *state else{unreachable!("opened native operation");};
        if !operation_recovery_context(scope,client){return Err(ModelError::Conflict("native operation changed execution context"));}
        if *failed{return Err(ModelError::Conflict("native operation suffix previously failed"));}
        *failed=true;
        let ordinal=*next;
        let payload=authorization::payload_digest(sql,&variables)?;
        let mut key=KeySink::new("native-operation-page/v1");request.operation.encode(&mut key);ordinal.encode(&mut key);
        let page=NativeRequest{issuance:request.issuance,operation:key.finish(),request:payload};
        let (query,bindings)=page_plan(*request,page,ordinal,&self.owner,sql,std::mem::take(&mut variables))?;
        super::receipts::accepted(page)?;
        let result=retry::transaction_named(client,&query,bindings,false,"operation-page").await;
        let disposition=match result {
            Ok(())=>EffectDisposition::Committed,
            Err(error)=>match reconcile_page(client,*request,page,ordinal,&self.owner).await {
                Ok(EffectDisposition::Committed)=>EffectDisposition::Committed,
                Ok(EffectDisposition::FencedUncommitted)=>{
                    super::receipts::observed(page,EffectDisposition::FencedUncommitted)?;
                    let refusal=Arc::new(page.error(error));
                    // This page's acknowledged fence also closes the producer's
                    // suffix. Settle its exact operation root before handing the
                    // known refusal back to the caller's operation admission.
                    // Earlier committed page receipts retain their true outcome.
                    let settlement=async{
                        pending.observed(EffectDisposition::FencedUncommitted)?;
                        release_operation_reference(client,*request,&self.owner,
                            &authorization::request_outcome_reference(*request,RecoveryReferencePolicy::ReleaseRecovery)?,
                            EffectDisposition::FencedUncommitted).await?;
                        pending.settle(EffectDisposition::FencedUncommitted)
                    }.await;
                    if let Err(settlement)=settlement {
                        return Err(unknown(*request,ModelError::SharedCause(refusal).alongside(settlement)));
                    }
                    *state=State::FailedTerminal{cause:refusal.clone()};
                    return Err(ModelError::SharedCause(refusal));
                }
                Ok(_)=>return Err(unknown(*request,page.error(error).alongside(ModelError::Schema("native operation page nonterminal reconciliation")))) ,
                Err(recovery)=>{return Err(unknown(page,error.alongside(recovery)));}
            },
        };
        super::receipts::observed(page,disposition)?;
        *next=next.checked_add(1).ok_or(ModelError::Schema("native operation page prefix overflow"))?;
        *failed=false;
        Ok(())
    }
    /// The actual producer calls this only after joining every accepted page.
    /// Scope finalization fences abandoned roots; it never declares successful production.
    pub async fn finish(&self,client:&Surreal<Client>,budget:&ResourceBudget)->Result<(),ModelError>{
        if !self.budget.shares_pool(budget){return Err(ModelError::Conflict("native operation terminal allocation owner"));}
        let mut state=self.state.lock().await;
        let State::Open{request,pending,scope,next,failed}=&*state else{
            return match &*state {
                State::Unopened=>{*state=State::Terminal;Ok(())},
                State::FailedTerminal{cause}=>Err(ModelError::SharedCause(cause.clone())),
                State::Terminal=>Ok(()),
                State::Open{..}=>unreachable!("open native operation"),
            };
        };
        if *failed{return Err(ModelError::Conflict("native failed operation cannot complete"));}
        if !operation_recovery_context(scope,client){return Err(ModelError::Conflict("native operation terminal owner"));}
        let mut variables=root_bindings(*request,&self.owner)?;
        variables.insert("operation_prefix",*next);
        let query=format!("BEGIN; {} LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; {} IF $root.state='open' {{ IF $root.committed_prefix!=$operation_prefix {{ THROW 'native operation terminal prefix changed'; }}; UPDATE $operation_root SET state='completed',revision+=1 RETURN NONE; }} ELSE IF $root.state!='completed' OR $root.committed_prefix!=$operation_prefix {{ THROW 'native operation terminal refused'; }}; COMMIT;",coordination::ERA_GUARD,ROOT_IDENTITY);
        coordination::preflight(&query,&variables)?;
        retry::transaction_named(client,&query,variables,false,"operation-terminal").await.map_err(|error|unknown(*request,error))?;
        // Knowledge is durable before the final reference disappears. This is
        // two operation-level host writes, never a write for every data page.
        pending.observed(EffectDisposition::Committed)?;
        release_operation_reference(client,*request,&self.owner,&authorization::request_outcome_reference(*request,RecoveryReferencePolicy::ReleaseRecovery)?,EffectDisposition::Committed).await?;
        pending.settle(EffectDisposition::Committed)?;
        *state=State::Terminal;
        Ok(())
    }
}
// A different active recovery owner cannot inherit this root's original host
// obligation merely because it happens to borrow the same SDK client.
fn operation_recovery_context(scope:&Arc<RecoveryScope>,client:&Surreal<Client>)->bool{
    std::ptr::eq(scope.client().as_ref(),client)
        && capture_recovery_scope().is_some_and(|active|Arc::ptr_eq(scope,&active))
}
fn unknown(request:NativeRequest,error:ModelError)->ModelError{
    complete::<()>(Err(request.error(error)),Completion{local:LocalState::Outstanding,remote:RemoteState::Unknown,..Completion::default()}).unwrap_err()
}
fn root_bindings(request:NativeRequest,owner:&RecoveryOwner)->Result<Variables,ModelError>{
    let effect=owner.effect_owner();
    let mut vars=Variables::new();request.bind(&mut vars);
    vars.insert("operation_root",RecordId::new("native_operation",request.operation.hex()));
    vars.insert("operation_digest",request.request.hex());vars.insert("operation_era",request.issuance.era);
    vars.insert("operation_owner",effect.record().map(Value::RecordId).unwrap_or(Value::None));
    vars.insert("operation_owner_kind",effect.kind());
    vars.insert("operation_reference",authorization::request_outcome_reference(request,RecoveryReferencePolicy::ReleaseRecovery)?);
    vars.insert("operation_reference_owner",request.operation.hex());
    Ok(vars)
}
const ROOT_IDENTITY:&str="IF $root=NONE OR $root.format!=1 OR $root.generation!=$__generation OR $root.era!=$operation_era OR $root.request!=$operation_digest OR $root.owner!=$operation_owner OR $root.owner_kind!=$operation_owner_kind OR $root.committed_prefix<0 OR $root.committed_prefix>=9223372036854775807 OR $root.revision<0 OR $root.revision>=9223372036854775807 { THROW 'native operation root identity collision'; };";
fn source_authorization(owner:&RecoveryOwner)->Result<String,ModelError>{
    let effect=owner.effect_owner();
    if matches!(owner,RecoveryOwner::Installation){return Ok("LET $__owner=NONE; LET $__epoch=1; LET $__attempt=NONE; LET $__attempt_slot=NONE;".into());}
    if !matches!(owner,RecoveryOwner::OpenAttempt(_)|RecoveryOwner::FrozenAttempt(_)){return Err(ModelError::Schema("native operation supported producer owner"));}
    Ok(format!("LET $__owner=SELECT * FROM ONLY $operation_owner FOR UPDATE; IF $__owner=NONE OR $__owner.generation!=$__generation OR $__owner.era!=$operation_era OR !({}) {{ THROW 'native operation source fenced'; }}; LET $__epoch=$__owner.epoch; LET $__attempt=$operation_owner; LET $__attempt_slot=$operation_slot;",effect.predicate()))
}
async fn create_root(client:&Surreal<Client>,request:NativeRequest,owner:&RecoveryOwner,segment:Option<&RecordId>)->Result<(),ModelError>{
    let mut vars=root_bindings(request,owner)?;
    vars.insert("operation_segment",segment.cloned().map(Value::RecordId).unwrap_or(Value::None));
    vars.insert("operation_slot",coordination::attempt_slot(request.operation));
    let auth=source_authorization(owner)?;
    let participation=if matches!(owner,RecoveryOwner::Installation){""}else{"fn::native_attempt_participate({attempt:$__attempt,effect:$operation_root,generation:$__generation,era:$operation_era,epoch:$__epoch,request:$operation_digest,slot:$operation_slot});"};
    let query=format!("BEGIN; {} LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; IF $root=NONE {{ {auth} fn::native_touch($operation_root); CREATE $operation_root SET format=1,generation=$__generation,era=$operation_era,request=$operation_digest,owner=$operation_owner,owner_kind=$operation_owner_kind,segment=$operation_segment,epoch=$__epoch,state='open',committed_prefix=0,revision=0 RETURN NONE; {participation} }} ELSE {{ {ROOT_IDENTITY} IF $root.state!='open' OR $root.segment!=$operation_segment {{ THROW 'native operation creation already fenced'; }}; }}; fn::native_touch($operation_reference); LET $reference=SELECT * FROM ONLY $operation_reference FOR UPDATE; IF $reference=NONE {{ CREATE $operation_reference SET object=$operation_root,owner=$operation_reference_owner,kind='recovery' RETURN NONE; }} ELSE IF $reference.object!=$operation_root OR $reference.effect!=NONE OR $reference.owner!=$operation_reference_owner OR $reference.kind!='recovery' {{ THROW 'native operation creation reference collision'; }}; COMMIT;",coordination::ERA_GUARD);
    coordination::preflight(&query,&vars)?;
    retry::transaction_named(client,&query,vars,false,"operation-root").await
}
fn page_plan(root:NativeRequest,page:NativeRequest,ordinal:i64,owner:&RecoveryOwner,sql:&str,mut vars:Variables)->Result<(String,Variables),ModelError>{
    super::merge_plan_bindings(&mut vars,root_bindings(root,owner)?)?;
    vars.insert("operation_page",RecordId::new("native_operation_page",page.operation.hex()));
    vars.insert("operation_ordinal",ordinal);vars.insert("page_digest",page.request.hex());
    vars.insert("__effect",RecordId::new("native_operation_page",page.operation.hex()));
    vars.insert("__request",page.request.hex());vars.insert("operation_slot",coordination::attempt_slot(page.operation));
    let auth=source_authorization(owner)?;
    let participation=if matches!(owner,RecoveryOwner::Installation){"LET $__attempt_context=NONE;"}else{"LET $__attempt_context={attempt:$__attempt,effect:$__effect,generation:$__generation,era:$__era,epoch:$__epoch,request:$__request,slot:$__attempt_slot}; fn::native_attempt_participate($__attempt_context);"};
    let query=format!("BEGIN; {} LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; {ROOT_IDENTITY} LET $prior=SELECT * FROM ONLY $operation_page FOR UPDATE; IF $prior!=NONE {{ IF $prior.operation!=$operation_root OR $prior.ordinal!=$operation_ordinal OR $prior.request!=$page_digest OR $prior.generation!=$__generation OR $prior.era!=$operation_era OR $prior.owner!=$operation_owner OR $prior.owner_kind!=$operation_owner_kind OR $prior.epoch!=$root.epoch OR $prior.ordinal>=$root.committed_prefix OR !$prior.committed OR !$prior.resolved {{ THROW 'native original page identity or outcome changed'; }}; }} ELSE {{ IF $root.state!='open' OR $root.committed_prefix!=$operation_ordinal {{ THROW 'native operation suffix fenced'; }}; {auth} IF $__epoch!=$root.epoch {{ THROW 'native operation source epoch changed'; }}; CREATE $operation_page SET operation=$operation_root,ordinal=$operation_ordinal,generation=$__generation,era=$operation_era,request=$page_digest,owner=$operation_owner,owner_kind=$operation_owner_kind,attempt=$__attempt,attempt_slot=$__attempt_slot,epoch=$__epoch,committed=false,resolved=false RETURN NONE; {participation} {} LET $__payload=|| {{ {sql}; }}; LET $__result=$__payload(); UPDATE $operation_page SET committed=true,resolved=true RETURN NONE; UPDATE $operation_root SET committed_prefix+=1,revision+=1 RETURN NONE; }}; COMMIT;",coordination::ERA_GUARD,coordination::expected_object_guard(&vars));
    coordination::preflight(&query,&vars)?;
    Ok((query,vars))
}
async fn reconcile_page(client:&Surreal<Client>,root:NativeRequest,page:NativeRequest,ordinal:i64,owner:&RecoveryOwner)->Result<EffectDisposition,ModelError>{
    let current=IssuanceEra::capture(client).await?;
    if current.generation!=root.issuance.generation{return Err(ModelError::Conflict("operation page recovery generation"));}
    let mut vars=root_bindings(root,owner)?;vars.insert("__era",current.era);
    vars.insert("operation_page",RecordId::new("native_operation_page",page.operation.hex()));vars.insert("operation_ordinal",ordinal);vars.insert("page_digest",page.request.hex());
    let query=format!("BEGIN; {} LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; {ROOT_IDENTITY} LET $prior=SELECT * FROM ONLY $operation_page FOR UPDATE; IF $prior=NONE {{ IF $root.state='completed' {{ THROW 'native completed operation missing committed page'; }}; CREATE $operation_page SET operation=$operation_root,ordinal=$operation_ordinal,generation=$__generation,era=$operation_era,request=$page_digest,owner=$operation_owner,owner_kind=$operation_owner_kind,attempt=NONE,epoch=0,committed=false,resolved=true RETURN NONE; UPDATE $operation_root SET state='fenced',revision+=1 RETURN NONE; }} ELSE {{ IF $prior.operation!=$operation_root OR $prior.ordinal!=$operation_ordinal OR $prior.request!=$page_digest OR $prior.generation!=$__generation OR $prior.era!=$operation_era OR $prior.owner!=$operation_owner OR $prior.owner_kind!=$operation_owner_kind OR ($prior.committed AND ($prior.epoch!=$root.epoch OR $prior.ordinal>=$root.committed_prefix)) OR !$prior.resolved {{ THROW 'native original page recovery identity'; }}; }}; COMMIT;",coordination::ERA_GUARD);
    coordination::preflight(&query,&vars)?;retry::transaction_named(client,&query,vars,false,"operation-page-recovery").await?;
    let mut response=client.query("SELECT committed FROM $page").bind(("page",RecordId::new("native_operation_page",page.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    match rows.as_slice(){[row] if row.get("committed")==Some(&Value::Bool(true))=>Ok(EffectDisposition::Committed),[row] if row.get("committed")==Some(&Value::Bool(false))=>Ok(EffectDisposition::FencedUncommitted),_=>Err(ModelError::Conflict("native page exact terminal response"))}
}
/// Exact host-loss recovery: no semantic resume, new request or production payload.
pub async fn reconcile_operation_root(client:&Surreal<Client>,request:NativeRequest,owner:&RecoveryOwner)->Result<EffectDisposition,ModelError>{
    request.validate()?;
    let current=IssuanceEra::capture(client).await?;
    if current.generation!=request.issuance.generation{return Err(ModelError::Conflict("native operation recovery generation"));}
    let mut vars=root_bindings(request,owner)?;vars.insert("__era",current.era);
    let query=format!("BEGIN; {} LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; IF $root=NONE {{ fn::native_touch($operation_root); CREATE $operation_root SET format=1,generation=$__generation,era=$operation_era,request=$operation_digest,owner=$operation_owner,owner_kind=$operation_owner_kind,epoch=0,state='fenced',committed_prefix=0,revision=1 RETURN NONE; }} ELSE {{ {ROOT_IDENTITY} IF $root.state='open' {{ UPDATE $operation_root SET state='fenced',revision+=1 RETURN NONE; }} ELSE IF $root.state NOT IN ['completed','fenced'] {{ THROW 'native operation recovery terminal state'; }}; }}; fn::native_touch($operation_reference); LET $reference=SELECT * FROM ONLY $operation_reference FOR UPDATE; IF $reference=NONE {{ CREATE $operation_reference SET object=$operation_root,owner=$operation_reference_owner,kind='recovery' RETURN NONE; }} ELSE IF $reference.object!=$operation_root OR $reference.effect!=NONE OR $reference.owner!=$operation_reference_owner OR $reference.kind!='recovery' {{ THROW 'native operation recovery reference collision'; }}; COMMIT;",coordination::ERA_GUARD);
    coordination::preflight(&query,&vars)?;retry::transaction_named(client,&query,vars,false,"operation-root-recovery").await?;
    let mut response=client.query("SELECT state FROM $root").bind(("root",RecordId::new("native_operation",request.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    match rows.as_slice(){[row] if row.get("state")==Some(&Value::String("completed".into()))=>Ok(EffectDisposition::Committed),[row] if row.get("state")==Some(&Value::String("fenced".into()))=>Ok(EffectDisposition::FencedUncommitted),_=>Err(ModelError::Conflict("native operation exact terminal response"))}
}
pub async fn release_operation_reference(client:&Surreal<Client>,request:NativeRequest,owner:&RecoveryOwner,reference:&RecordId,disposition:EffectDisposition)->Result<(),ModelError>{
    request.validate()?;
    if !matches!(disposition,EffectDisposition::Committed|EffectDisposition::FencedUncommitted){return Err(ModelError::Conflict("native operation terminal reference disposition"));}
    if reference!=&authorization::request_outcome_reference(request,RecoveryReferencePolicy::ReleaseRecovery)?{return Err(ModelError::Conflict("native operation exact reference"));}
    let current=IssuanceEra::capture(client).await?;
    if current.generation!=request.issuance.generation{return Err(ModelError::Conflict("native operation reference generation"));}
    let mut vars=root_bindings(request,owner)?;vars.insert("__era",current.era);vars.insert("terminal_state",if disposition==EffectDisposition::Committed{"completed"}else{"fenced"});
    // The host calls this phase only after durably recording terminal knowledge.
    // A crash between native release and host removal may leave no root after
    // legitimate history collection. Only an irreversible original-era cut and
    // exact reference absence discharge that already-terminal obligation.
    let query=format!("BEGIN; {} fn::native_touch($operation_root); LET $root=SELECT * FROM ONLY $operation_root FOR UPDATE; fn::native_touch($operation_reference); LET $reference=SELECT * FROM ONLY $operation_reference FOR UPDATE; IF $root=NONE {{ IF $__installation.closed_through<$operation_era OR $reference!=NONE {{ THROW 'native operation compacted terminal not proved'; }}; }} ELSE {{ {ROOT_IDENTITY} IF $root.state!=$terminal_state {{ THROW 'native operation release without terminal knowledge'; }}; IF $reference!=NONE {{ IF $reference.object!=$operation_root OR $reference.effect!=NONE OR $reference.owner!=$operation_reference_owner OR $reference.kind!='recovery' {{ THROW 'native operation reference changed'; }}; DELETE $operation_reference RETURN NONE; }}; }}; COMMIT;",coordination::TERMINAL_RELEASE_ERA_GUARD);
    coordination::preflight(&query,&vars)?;retry::transaction_named(client,&query,vars,false,"operation-reference-release").await
}

/// Maintenance observes/fences exact durable roots, retaining their existing host
/// recovery references. This does not settle recipients or prove executor drain.
pub(super) async fn reconcile_open_operations(client:&Surreal<Client>)->Result<(),ModelError>{
    loop{
        let mut response=client.query("SELECT * FROM native_operation WITH INDEX operation_history WHERE state='open' LIMIT 128").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;if rows.is_empty(){return Ok(());}
        for row in rows{
            let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native maintenance operation id"));};
            let surrealdb::types::RecordIdKey::String(key)=&id.key else{return Err(ModelError::Schema("native maintenance operation key"));};
            let operation=ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("native maintenance operation width"))?);
            let request=NativeRequest{issuance:IssuanceEra{generation:authorization::hash_field(&row,"generation")?,era:authorization::int_field(&row,"era")?},operation,request:authorization::hash_field(&row,"request")?};
            let owner=match (row.get("owner_kind"),row.get("owner")){
                (Some(Value::String(kind)),None|Some(Value::None)) if kind=="installation"=>RecoveryOwner::Installation,
                (Some(Value::String(kind)),Some(Value::RecordId(id))) if matches!(kind.as_str(),"attempt"|"frozen_attempt") && id.table.as_str()=="native_attempt"=>{
                    let surrealdb::types::RecordIdKey::String(key)=&id.key else{return Err(ModelError::Schema("native maintenance owner key"));};
                    let id=ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("native maintenance owner width"))?);
                    if kind=="attempt"{RecoveryOwner::OpenAttempt(id)}else{RecoveryOwner::FrozenAttempt(id)}
                },_=>return Err(ModelError::Schema("native maintenance operation owner")),
            };
            reconcile_operation_root(client,request,&owner).await?;
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use futures::FutureExt;
    #[test]
    fn native_pages_preserve_original_identity_scalar_locks_and_terminal_witness(){
        let request=NativeRequest{issuance:IssuanceEra{generation:ContentHash::of(b"generation"),era:1},operation:ContentHash::of(b"root"),request:ContentHash::of(b"rootpayload")};
        let owner=RecoveryOwner::OpenAttempt(ContentHash::of(b"attempt"));
        let (sql,_)=page_plan(request,request,0,&owner,"RETURN NONE",Variables::new()).unwrap();
        assert!(sql.contains("ONLY $operation_root FOR UPDATE"));
        assert!(sql.contains("ONLY $operation_page FOR UPDATE"));
        assert!(sql.contains("$root.committed_prefix!=$operation_ordinal"));
        assert!(sql.find("$prior!=NONE").unwrap()<sql.find("$root.state!='open'").unwrap());
        assert!(!sql.contains("CREATE $__effect"),"a page has its own receipt, never a generic effect");
        assert!(sql.contains("native_attempt_participate"));
    }
    #[tokio::test(flavor="multi_thread")]
    async fn original_native_pages_replay_once_fence_absence_and_retain_terminal_reference(){
        use lctx_model::domain as d;
        let config=lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"))).unwrap();
        let native_config=crate::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"))).unwrap();
        let budget=ResourceBudget::fixed(d::resources::DEFAULT_MEMORY_BYTES).unwrap();
        let factory=lctx_compiler_store::compiler::recovery_factory(&config,budget.clone()).unwrap();
        let store=lctx_compiler_store::compiler::NativeCompilerStore::begin(&config,d::admission::Frontier::Facts,&factory).await.unwrap();
        let client=lctx_compiler_store::reader::connect(&config.endpoint,&config.writer_credentials(),store.namespace().as_str(),store.database().as_str()).await.unwrap();
        let stop=RetryCancellation::default();let cancel=stop.clone();
        let signals=native_config.signal_owner("native original operation pages",move ||cancel.cancel()).unwrap();
        let scope=RecoveryScope::new(client.clone(),budget.clone(),stop,signals.pending_recipient().unwrap(),RecoveryTarget{generation:config.service_generation,namespace:config.namespace.as_str().into(),database:config.database.as_str().into(),endpoint:Some(config.endpoint.clone())}).unwrap();
        let owner=RecoveryOwner::OpenAttempt(store.attempt());
        let issuance=authorization::owner_issuance(&client,&owner.effect_owner()).await.unwrap();
        let root=NativeRequest{issuance,operation:fresh_identity("operation-page-control").unwrap(),request:ContentHash::of(b"exact native root control")};
        let pending=scope.prepare_operation_root(root,owner.clone()).unwrap();
        let result=with_recovery_scope(scope.clone(),async{
            create_root(&client,root,&owner,None).await?;
            let page=|ordinal:i64|{let mut key=KeySink::new("native-operation-page/v1");root.operation.encode(&mut key);ordinal.encode(&mut key);NativeRequest{issuance,operation:key.finish(),request:ContentHash::of(b"exact original control page")}};
            let (sql,vars)=page_plan(root,page(0),0,&owner,"UPDATE $operation_root SET revision+=7 RETURN NONE",Variables::new())?;
            retry::transaction_named(&client,&sql,vars.clone(),false,"control-operation-page").await?;
            // Simulate a lost acknowledgement: issue the same bytes/ordinal again.
            retry::transaction_named(&client,&sql,vars.clone(),false,"control-original-page-replay").await?;
            let mut response=client.query("SELECT committed_prefix,revision FROM ONLY $root").bind(("root",RecordId::new("native_operation",root.operation.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let observed:Option<Object>=response.take(0).map_err(ModelError::codec)?;
            let observed=observed.ok_or(ModelError::Conflict("native page control root"))?;
            assert_eq!(observed.get("committed_prefix"),Some(&Value::Number(surrealdb::types::Number::Int(1))));assert_eq!(observed.get("revision"),Some(&Value::Number(surrealdb::types::Number::Int(8))));
            assert_eq!(reconcile_page(&client,root,page(1),1,&owner).await?,EffectDisposition::FencedUncommitted);
            let (late,bindings)=page_plan(root,page(2),2,&owner,"UPDATE $operation_root SET revision+=100 RETURN NONE",Variables::new())?;
            assert!(retry::transaction_named(&client,&late,bindings,false,"control-late-page").await.is_err());
            retry::transaction_named(&client,&sql,vars,false,"control-earlier-committed-replay").await?;
            assert_eq!(reconcile_operation_root(&client,root,&owner).await?,EffectDisposition::FencedUncommitted);
            let reference=authorization::request_outcome_reference(root,RecoveryReferencePolicy::ReleaseRecovery)?;
            let mut response=client.query("SELECT object FROM ONLY $reference").bind(("reference",reference.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let actual:Option<Object>=response.take(0).map_err(ModelError::codec)?;assert!(actual.is_some(),"terminal recovery keeps its exact last witness until durable host knowledge");
            pending.observed(EffectDisposition::FencedUncommitted)?;
            release_operation_reference(&client,root,&owner,&reference,EffectDisposition::FencedUncommitted).await?;
            pending.settle(EffectDisposition::FencedUncommitted)?;
            // Discovery can win before delayed creation: same-key tombstone fences it.
            let absent=NativeRequest{operation:fresh_identity("absent-operation-control")?,..root};
            let absent_pending=scope.prepare_operation_root(absent,owner.clone())?;
            assert_eq!(reconcile_operation_root(&client,absent,&owner).await?,EffectDisposition::FencedUncommitted);
            assert!(create_root(&client,absent,&owner,None).await.is_err());
            absent_pending.observed(EffectDisposition::FencedUncommitted)?;
            release_operation_reference(&client,absent,&owner,&authorization::request_outcome_reference(absent,RecoveryReferencePolicy::ReleaseRecovery)?,EffectDisposition::FencedUncommitted).await?;
            absent_pending.settle(EffectDisposition::FencedUncommitted)?;
            // Exercise the real writer boundary as well as the native page
            // primitives: a confirmed refused suffix cannot remain pending or
            // be mistaken for successful producer completion.
            let writer=OperationWriter::new(owner.clone(),None,&budget)?;
            writer.apply(&client,"RETURN NONE",Variables::new(),&budget).await?;
            let refused=writer.apply(&client,"THROW 'confirmed operation writer refusal'",Variables::new(),&budget).await.unwrap_err();
            assert!(refused.permits_storage_cleanup(),"{refused:?}");
            assert_eq!(scope.pending_count()?,0,"known fenced root is durably settled before return");
            let ModelError::SharedCause(cause)=refused else{panic!("retained original writer cause");};
            for refused in [writer.finish(&client,&budget).await.unwrap_err(),
                writer.apply(&client,"RETURN NONE",Variables::new(),&budget).await.unwrap_err()]{
                let ModelError::SharedCause(actual)=refused else{panic!("failed writer retains original cause");};
                assert!(Arc::ptr_eq(&cause,&actual),"failed terminal writer cannot relabel or retry production");
            }
            Ok(())
        }).await;
        let mut terminal=d::completion::Completion::default();
        if scope.pending_count().unwrap()==0 && result.as_ref().err().is_none_or(ModelError::permits_storage_cleanup){
            terminal.step("native page control actual session close",client.invalidate().await.map_err(ModelError::codec));
            terminal.step("native page control source abandon",store.abandon().await);
        }else{terminal.local=d::completion::LocalState::Outstanding;terminal.remote=d::completion::RemoteState::Unknown;}
        let done=signals.complete(&terminal).await;terminal.step("native page control signal owner",done);
        d::completion::complete(result,terminal).unwrap();
    }
    #[tokio::test(flavor="multi_thread")]
    #[ignore="permanent era cut and history collection require exclusive owned validation maintenance"]
    async fn operation_roots_keep_replay_fences_until_actual_cut_and_history_release(){
        assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),"explicit exclusive campaign maintenance required");
        let path=std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("owned maintenance installer configuration");
        let config=crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        assert_eq!(config.authentication,crate::AuthenticationScope::Root);assert_eq!(config.database.as_str(),"validation");
        let inventory=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("separately qualified actual history inventory");
        let budget=ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap();
        let factory=RecoveryFactoryOwner::new(config.receipt_binding().unwrap(),budget.clone()).unwrap();
        let (client,lease)=crate::reader::connect_owned(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str(),&factory).await.unwrap();
        let connection=client.connection_completion().unwrap();
        let result=std::panic::AssertUnwindSafe(with_recovery_scope(lease.recovery_scope().clone(),async{
            crate::installation::check_connected_installation(&client,config.service_generation).await?;
            let mut response=client.query("SELECT admission_open,history_inventory FROM ONLY native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let installation:Option<Object>=response.take(0).map_err(ModelError::codec)?;
            let installation=installation.ok_or(ModelError::Schema("operation history installation"))?;
            assert_eq!(installation.get("admission_open"),Some(&Value::Bool(false)));assert_eq!(installation.get("history_inventory"),Some(&Value::String(inventory)));
            let issuance=IssuanceEra::capture(&client).await?;
            let root=NativeRequest{issuance,operation:fresh_identity("cut-operation-control")?,request:ContentHash::of(b"original root before actual era cut")};
            let owner=RecoveryOwner::Installation;
            let scope=lease.recovery_scope();
            let pending=scope.prepare_operation_root(root,owner.clone())?;
            create_root(&client,root,&owner,None).await?;
            let mut key=KeySink::new("native-operation-page/v1");root.operation.encode(&mut key);0i64.encode(&mut key);
            let page=NativeRequest{issuance,operation:key.finish(),request:ContentHash::of(b"original history control page")};
            let (sql,vars)=page_plan(root,page,0,&owner,"RETURN NONE",Variables::new())?;
            retry::transaction_named(&client,&sql,vars,false,"operation-history-page-control").await?;
            assert_eq!(reconcile_operation_root(&client,root,&owner).await?,EffectDisposition::FencedUncommitted);
            let absent=NativeRequest{operation:fresh_identity("cut-absent-root-control")?,..root};
            let absent_pending=scope.prepare_operation_root(absent,owner.clone())?;
            assert_eq!(reconcile_operation_root(&client,absent,&owner).await?,EffectDisposition::FencedUncommitted);
            let next=history::cut_era(&client).await?;assert_eq!(next.era,issuance.era+1);
            async fn collect(client:&Surreal<Client>)->Result<(),ModelError>{
                let mut progress=history::compact_history(client,128).await?;
                for _ in 0..128{if progress.state=="done"{return Ok(());}progress=history::resume_history_compaction(client,progress.identity,progress.revision,128).await?;}
                Err(ModelError::Conflict("bounded operation control history did not reach EOF"))
            }
            let records=vec![RecordId::new("native_operation",root.operation.hex()),RecordId::new("native_operation_page",page.operation.hex()),RecordId::new("native_operation",absent.operation.hex())];
            collect(&client).await?;
            let mut response=client.query("SELECT VALUE id FROM $records").bind(("records",records.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let protected:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;assert_eq!(protected.len(),3,"original root references protect committed pages and absent-root fences after the era cut");
            for (request,pending) in [(root,&pending),(absent,&absent_pending)]{
                pending.observed(EffectDisposition::FencedUncommitted)?;
                release_operation_reference(&client,request,&owner,&authorization::request_outcome_reference(request,RecoveryReferencePolicy::ReleaseRecovery)?,EffectDisposition::FencedUncommitted).await?;
            }
            collect(&client).await?;
            let mut response=client.query("SELECT VALUE id FROM $records").bind(("records",records)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let remaining:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;assert!(remaining.is_empty(),"closed-era pages collect before their released root");
            assert!(create_root(&client,absent,&owner,None).await.is_err(),"permanent original-era cut excludes delayed creation after tombstone collection");
            for (request,pending) in [(root,&pending),(absent,&absent_pending)]{
                // Crash-window replay after native release and legitimate compaction
                // retains the host's already-durable terminal interpretation.
                release_operation_reference(&client,request,&owner,&authorization::request_outcome_reference(request,RecoveryReferencePolicy::ReleaseRecovery)?,EffectDisposition::FencedUncommitted).await?;
                pending.settle(EffectDisposition::FencedUncommitted)?;
            }
            Ok(())
        })).catch_unwind().await.unwrap_or_else(|panic|Err(ModelError::Cause(Box::new(lctx_model::domain::completion::ThreadPanic::new("operation root history campaign control",panic)))));
        let mut terminal=Completion::default();
        if result.as_ref().is_err_and(|error|!error.permits_storage_cleanup()){terminal.remote=RemoteState::Unknown;}
        terminal.step("operation history fixture factory finalization",lease.complete_with(&terminal).await);
        let joined=terminal.is_satisfied();drop(client);drop(lease);drop(factory);
        if joined{terminal.step("operation history SDK completion",connection.wait().await.map_err(ModelError::codec));}drop(connection);
        complete(result,terminal).unwrap();
    }

}
