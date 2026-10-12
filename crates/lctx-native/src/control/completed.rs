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
pub(crate) async fn seal_realization(client:&Surreal<Client>,fence:&FrozenContentFence,expected:&ExpectedRecord,
    revision:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=budget.reserve("exact realization seal",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();frozen::bind_fence(fence,&mut variables);
    variables.insert("realization",expected.record().clone());variables.insert("realization_guard",guard_id(&Value::RecordId(expected.record().clone()))?);
    variables.insert("realization_incarnation",expected.incarnation());variables.insert("realization_bytes",expected.row().clone());variables.insert("realization_revision",revision);
    let sql=format!("{} fn::native_touch($realization); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$realization]); LET $guard=SELECT * FROM ONLY $realization_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $realization FOR UPDATE; IF $predicate!=$realization_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$realization_incarnation OR $actual!=$realization_bytes {{ THROW 'realization membership audit raced'; }}; IF type::is::bool($actual.sealed)=false {{ THROW 'realization missing seal state'; }}; IF !$actual.sealed {{ UPDATE $realization SET sealed=true RETURN NONE; }}",frozen::FENCE_GUARD);
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
