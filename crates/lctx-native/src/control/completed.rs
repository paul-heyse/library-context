//! Fixed physical witness construction after canonical attempt freeze. All helpers are
//! crate-private; callers cannot supply SQL or mutate canonical semantic content.
use super::*;
use crate::completed_state::{FrozenContentFence,FrozenCompletion};
use lctx_model::domain::resources::ResourceBudget;

pub(crate) async fn install_witness(client:&Surreal<Client>,fence:&FrozenContentFence,row:Option<Value>,
    edges:Vec<(RecordId,RecordId)>,budget:&ResourceBudget)->Result<(),ModelError>{
    let bytes=row.as_ref().map_or(0,crate::loader::native_bytes).saturating_add(edges.iter().map(|(a,b)|crate::loader::native_bytes(&Value::RecordId(a.clone())).saturating_add(crate::loader::native_bytes(&Value::RecordId(b.clone())))).sum::<usize>());
    let _charge=budget.reserve("frozen physical witness construction",bytes.saturating_mul(32).saturating_add(131072))?;
    if let Some(row)=&row {
        let Some(Value::RecordId(id))=row.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("physical witness id"));};
        if !matches!(id.table.as_str(),"compiler_view_realization"|"compiler_view_member"|"compiler_view_completion"|"native_completed_state"|"native_completed_view"|"compiler_binding_descriptor"){
            return Err(ModelError::Conflict("frozen physical witness family"));
        }
    }
    let mut variables=Variables::new();let mut sql=String::from(frozen::FENCE_GUARD);
    if let Some(row)=row {
        let (rows,bindings)=immutable_row_plan(None,None,vec![row])?.ok_or(ModelError::Schema("physical witness row plan"))?;
        sql.push_str(&rows);merge_plan_bindings(&mut variables,bindings)?;
    }
    if let Some((_,holds,bindings))=hold_plan(Some(fence.attempt()),edges)?{sql.push_str(&holds);merge_plan_bindings(&mut variables,bindings)?;}
    frozen::bind_fence(fence,&mut variables);
    effect_for_owner(client,EffectOwner::FrozenAttempt(fence.attempt()),&sql,variables).await
}
pub(crate) async fn install_members(client:&Surreal<Client>,fence:&FrozenContentFence,root:&RecordId,
    members:&crate::ordered_rows::PreparedRows,budget:&ResourceBudget)->Result<usize,ModelError>{
    let mut cursor=members.cursor_with_budget(budget)?;
    let rows=std::iter::from_fn(move||cursor.next_row().transpose());
    let mut variables=Variables::new();frozen::bind_fence(fence,&mut variables);
    let mut installed=0usize;
    for window in ImmutableWindows::for_view_members(rows,root.clone(),frozen::FENCE_GUARD,variables).with_budget(Some(budget)) {
        immutable_window::install_member_window(client,EffectOwner::FrozenAttempt(fence.attempt()),window?,budget).await?;
        installed=installed.checked_add(1).ok_or(ModelError::Schema("frozen member window count"))?;
    }Ok(installed)
}
pub(crate) async fn seal_realization(client:&Surreal<Client>,fence:&FrozenContentFence,expected:&ExpectedRecord,
    revision:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=budget.reserve("exact realization seal",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();frozen::bind_fence(fence,&mut variables);
    variables.insert("realization",expected.record().clone());variables.insert("realization_guard",guard_id(&Value::RecordId(expected.record().clone()))?);
    variables.insert("realization_incarnation",expected.incarnation());variables.insert("realization_bytes",expected.row().clone());variables.insert("realization_revision",revision);
    let sql=format!("{} fn::native_touch($realization); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$realization]); LET $guard=SELECT * FROM ONLY $realization_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $realization FOR UPDATE; IF $predicate!=$realization_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$realization_incarnation OR $actual!=$realization_bytes {{ THROW 'realization membership audit raced'; }}; IF type::is_bool($actual.sealed)=false {{ THROW 'realization missing seal state'; }}; IF !$actual.sealed {{ UPDATE $realization SET sealed=true RETURN NONE; }}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(fence.attempt()),&sql,variables).await
}
pub(crate) async fn finish_physical_completion(client:&Surreal<Client>,data:&FrozenCompletion,expected:&ExpectedRecord,revision:i64,
    budget:&ResourceBudget)->Result<(),ModelError>{
    if expected.row()!=&data.value(false)?{return Err(ModelError::Conflict("physical completion initial exact envelope"));}
    let _charge=budget.reserve("physical completion checked final effect",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();frozen::bind_fence(&data.fence(),&mut variables);
    variables.insert("completion",data.record()?);variables.insert("completion_guard",guard_id(&Value::RecordId(data.record()?))?);variables.insert("completion_incarnation",expected.incarnation());variables.insert("completion_bytes",expected.row().clone());variables.insert("completion_revision",revision);
    variables.insert("completion_identity",data.identity()?.hex());variables.insert("completion_logical",data.completed_state.hex());
    let sql=format!("{} fn::native_touch($completion); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$completion]); LET $guard=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $completion FOR UPDATE; IF $predicate!=$completion_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$completion_incarnation OR $actual!=$completion_bytes {{ THROW 'physical completion inventory audit raced'; }}; UPDATE $completion SET completed=true RETURN NONE; UPDATE $__attempt SET completed_identity=$completion_identity,completed_state=$completion_logical RETURN NONE",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(data.attempt),&sql,variables).await
}

pub(crate) async fn hold_selected_contributor(client:&Surreal<Client>,fence:&FrozenContentFence,root:&RecordId,expected:&ExpectedRecord,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=budget.reserve("exact selected contributor hold",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let (_,holds,mut variables)=hold_plan(Some(fence.attempt()),vec![(root.clone(),expected.record().clone())])?.ok_or(ModelError::Schema("selected contributor fixed hold plan"))?;
    frozen::bind_fence(fence,&mut variables);variables.insert("contributor",expected.record().clone());variables.insert("contributor_guard",guard_id(&Value::RecordId(expected.record().clone()))?);variables.insert("contributor_incarnation",expected.incarnation());variables.insert("contributor_bytes",expected.row().clone());
    let sql=format!("{} fn::native_touch($contributor); LET $guard=SELECT * FROM ONLY $contributor_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $contributor FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$contributor_incarnation OR $actual!=$contributor_bytes OR !$actual.completed {{ THROW 'selected contributor completion changed'; }}; {holds}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(fence.attempt()),&sql,variables).await
}

/// Preserve an already sealed namespace after independent EOF verification. The
/// fixed frozen-owner transaction compares the original physical premises without
/// touching its member predicate or rewriting any membership/hold incidence.
pub(crate) async fn retain_installed_view(client:&Surreal<Client>,fence:&FrozenContentFence,
    root:&ExpectedRecord,completion:&ExpectedRecord,revision:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    if root.record().table.as_str()!="compiler_view_realization" || completion.record().table.as_str()!="compiler_view_completion" {return Err(ModelError::Conflict("retained installed witness families"));}
    let _charge=budget.reserve("retained installed fixed comparison",crate::loader::native_bytes(root.row()).saturating_add(crate::loader::native_bytes(completion.row())).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();frozen::bind_fence(fence,&mut variables);
    for (prefix,expected) in [("root",root),("completion",completion)] {
        variables.insert(format!("{prefix}_record"),expected.record().clone());variables.insert(format!("{prefix}_guard"),guard_id(&Value::RecordId(expected.record().clone()))?);
        variables.insert(format!("{prefix}_incarnation"),expected.incarnation());variables.insert(format!("{prefix}_bytes"),expected.row().clone());
    }
    variables.insert("root_revision",revision);
    let sql=format!("{} LET $predicate=SELECT * FROM ONLY type::record('native_predicate',[$root_record]) FOR UPDATE; LET $rg=SELECT * FROM ONLY $root_guard FOR UPDATE; LET $root=SELECT * FROM ONLY $root_record FOR UPDATE; LET $cg=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $completion=SELECT * FROM ONLY $completion_record FOR UPDATE; IF $predicate.revision!=$root_revision OR $rg.phase!='active' OR $rg.incarnation!=$root_incarnation OR $root!=$root_bytes OR $root.sealed!=true OR $cg.phase!='active' OR $cg.incarnation!=$completion_incarnation OR $completion!=$completion_bytes OR $completion.completed!=true {{ THROW 'retained installed witness changed after EOF audit'; }}",frozen::FENCE_GUARD);
    effect_for_owner(client,EffectOwner::FrozenAttempt(fence.attempt()),&sql,variables).await
}

#[cfg(test)]
mod member_window_controls {
    use super::*;
    use futures::FutureExt;
    use lctx_model::domain::{Record,input::Package,graph::Entity,completion::{Completion,complete}};

    /// Exercises the fixed frozen writer directly: ordinary provider completion may
    /// already have sealed its live namespace and therefore skip this construction.
    /// This is physical member/owner evidence, never semantic artifact admission.
    #[tokio::test(flavor="multi_thread")]
    async fn frozen_member_installer_executes_multiple_windows_with_exact_holds_and_owner_fence(){
        let path=std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned frozen-member campaign fixture");
        let config=crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let store_config=lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let store=lctx_compiler_store::compiler::NativeCompilerStore::begin(&store_config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
        let client=crate::reader::connect(&config.endpoint,&config.writer_credentials(),store.namespace().as_str(),store.database().as_str()).await.unwrap();
        let budget=ResourceBudget::fixed(256<<20).unwrap();let attempt=store.attempt();let nonce=fresh_identity("frozen-member-window-control").unwrap();
        let outcome=std::panic::AssertUnwindSafe(async {
            let packages=(0..300).map(|index|Package{name:format!("frozen-member-{}-{index}",nonce.hex())}).collect::<Vec<_>>();
            let entities=packages.iter().cloned().map(Entity::from).collect::<Vec<_>>();
            crate::Loader::for_attempt_views(client.clone(),attempt,vec![]).with_budget(&budget).entities(&entities).await?;
            let root=RecordId::new("compiler_view_realization",nonce.hex());
            let mut declaration=Object::new();declaration.insert("id",root.clone());declaration.insert("sealed",false);declaration.insert("descriptor",surrealdb::types::Bytes::from(b"fixed frozen member control".to_vec()));
            let window=ImmutableWindows::new(vec![Value::Object(declaration)],Some(attempt),None,Some(RecordId::new("native_attempt",attempt.hex()))).with_budget(Some(&budget)).next().unwrap()?;
            install_window(&client,Some(attempt),window,Some(&budget)).await?;
            let mut prepared=crate::ordered_rows::SortedRows::with_budget(&budget)?;let mut expected=std::collections::BTreeMap::new();
            for (package,entity) in packages.iter().zip(&entities){
                let key=*package.id().bytes();let id=crate::installed_view::member_id(nonce,Package::NAME,&key);
                let mut row=Object::new();row.insert("id",id.clone());row.insert("realization",root.clone());row.insert("relation",Package::NAME);row.insert("semantic_key",hex::encode(key));row.insert("node",crate::loader::entity_payload_id(entity)?);row.insert("content",entity.content().hex());
                let row=Value::Object(row);prepared.push(row.clone())?;expected.insert(id,row);
            }
            let members=prepared.finish()?.into_prepared();
            let mut response=client.query("SELECT revision FROM $attempt").bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let owners:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let [owner]=owners.as_slice()else{return Err(ModelError::Schema("frozen member test owner cardinality"))};
            let fence=frozen::freeze_content_native(client.clone(),attempt,authorization::int_field(owner,"revision")?,ContentHash::of(b"empty physical test bindings"),&budget).await?;
            let installed=install_members(&client,&fence,&root,&members,&budget).await?;assert!(installed>1,"actual frozen installer must acknowledge more than one bounded window");
            // Read exact bounded windows independently of the prepared cursor. Both
            // outgoing hold levels and every full member value must be present.
            let ids=expected.keys().cloned().collect::<Vec<_>>();
            for ids in ids.chunks(crate::loader::NATIVE_WINDOW_ROWS){
                let mut response=client.query("SELECT * FROM $ids; SELECT owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$root AND object IN $ids; SELECT owner,object FROM native_hold WITH INDEX owner_holds WHERE owner IN $ids")
                    .bind(("ids",ids.to_vec())).bind(("root",root.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;let root_holds:Vec<Object>=response.take(1).map_err(ModelError::codec)?;let node_holds:Vec<Object>=response.take(2).map_err(ModelError::codec)?;
                assert_eq!(rows.len(),ids.len());assert_eq!(root_holds.len(),ids.len());assert_eq!(node_holds.len(),ids.len());
                let observed=rows.into_iter().map(|row|{let Some(Value::RecordId(id))=row.as_object().and_then(|row|row.get("id"))else{panic!("observed member identity")};(id.clone(),row)}).collect::<std::collections::BTreeMap<_,_>>();
                for id in ids {let row=&expected[id];assert_eq!(observed.get(id),Some(row));let node=row.as_object().unwrap().get("node").unwrap();assert!(root_holds.iter().any(|hold|hold.get("owner")==Some(&Value::RecordId(root.clone()))&&hold.get("object")==Some(&Value::RecordId(id.clone()))));assert!(node_holds.iter().any(|hold|hold.get("owner")==Some(&Value::RecordId(id.clone()))&&hold.get("object")==Some(node)));}
            }
            let wrong=FrozenContentFence{attempt_epoch:fence.attempt_epoch()+1,..fence};
            let error=install_members(&client,&wrong,&root,&members,&budget).await.unwrap_err();assert!(error.to_string().contains("native frozen content fence changed"),"{error}");assert!(error.permits_storage_cleanup(),"fixed owner refusal is acknowledged: {error}");
            let mut response=client.query("SELECT count() AS count FROM compiler_view_member WITH INDEX realization_nodes WHERE realization=$root GROUP ALL").bind(("root",root)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;let counts:Vec<Object>=response.take(0).map_err(ModelError::codec)?;assert_eq!(counts[0].get("count"),Some(&test_value(300i64)),"wrong owner cannot change the namespace");
            Ok::<_,ModelError>(())
        }).catch_unwind().await;
        let mut terminal=Completion::default();terminal.step("frozen member control owner abandon",store.abandon().await);terminal.step("frozen member control session close",client.invalidate().await.map_err(ModelError::codec));
        match outcome {Ok(result)=>complete(result,terminal).unwrap(),Err(panic)=>{if let Err(error)=complete(Ok(()),terminal){eprintln!("frozen member control cleanup: {error:#?}");}std::panic::resume_unwind(panic)}}
    }
}
