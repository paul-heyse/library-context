//! Fixed per-view witness effects while unrelated attempt producers remain open.
//! This authority cannot create aggregate completion, admission or canonical content.
use super::*;
use lctx_model::domain::resources::ResourceBudget;

pub(crate) struct LiveViewOwner {
    pub(super) attempt:ContentHash,
    generation:ContentHash,
    epoch:i64,
    validity_epoch:i64,
}
impl LiveViewOwner {
    pub(crate) fn epoch(&self)->i64{self.epoch}
    pub(crate) fn attempt(&self)->ContentHash{self.attempt}
    pub(crate) fn generation(&self)->ContentHash{self.generation}
    pub(crate) fn validity_epoch(&self)->i64{self.validity_epoch}
}
pub(super) const GUARD:&str="IF $__owner.state!='open' OR $__owner.admitted OR $__owner.generation!=$live_generation OR $__owner.epoch!=$live_epoch OR $__installation.generation!=$live_generation OR $__installation.validity_epoch!=$live_validity_epoch { THROW 'live installed view owner changed'; };";
pub(super) fn bind(owner:&LiveViewOwner,bindings:&mut Variables){
    bindings.insert("live_generation",owner.generation.hex());bindings.insert("live_epoch",owner.epoch);bindings.insert("live_validity_epoch",owner.validity_epoch);
}
pub(crate) async fn capture(reader:&crate::NativeReader<()>,attempt:ContentHash,budget:&ResourceBudget)->Result<LiveViewOwner,ModelError>{
    let _charge=budget.reserve("live installed view owner inspection",131072)?;
    let mut bindings=Variables::new();bindings.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
    let owners:Vec<Object>=reader.query_native("SELECT * FROM $attempt",bindings).await?;
    let installations:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
    let [owner]=owners.as_slice() else{return Err(ModelError::Conflict("live view attempt cardinality"));};
    let [installation]=installations.as_slice() else{return Err(ModelError::Conflict("live view installation cardinality"));};
    let result=LiveViewOwner{attempt,generation:authorization::hash_field(installation,"generation")?,epoch:authorization::int_field(owner,"epoch")?,validity_epoch:authorization::int_field(installation,"validity_epoch")?};
    if owner.get("state")!=Some(&Value::String("open".into())) || owner.get("admitted")!=Some(&Value::Bool(false))
        || result.epoch<=0 || result.validity_epoch<=0 || authorization::hash_field(owner,"generation")?!=result.generation {
        return Err(ModelError::Conflict("live view requires actual open current attempt"));
    }Ok(result)
}
pub(crate) async fn install(client:&Surreal<Client>,owner:&LiveViewOwner,row:Option<Value>,edges:Vec<(RecordId,RecordId)>,budget:&ResourceBudget)->Result<(),ModelError>{
    if let Some(row)=&row {
        let Some(Value::RecordId(id))=row.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("live view witness id"));};
        if !matches!(id.table.as_str(),"compiler_view_realization"|"compiler_view_member"|"compiler_view_completion") {return Err(ModelError::Conflict("live view witness family"));}
    }
    let bytes=row.as_ref().map_or(0,crate::loader::native_bytes).saturating_add(edges.len().saturating_mul(1024));
    let _charge=budget.reserve("live view witness construction",bytes.saturating_mul(32).saturating_add(131072))?;
    let mut sql=String::from(GUARD);let mut variables=Variables::new();bind(owner,&mut variables);
    if let Some(row)=row {let (plan,bindings)=immutable_row_plan(None,None,vec![row])?.ok_or(ModelError::Schema("live view witness plan"))?;sql.push_str(&plan);merge_plan_bindings(&mut variables,bindings)?;}
    if let Some((_,plan,bindings))=hold_plan(Some(owner.attempt),edges)?{sql.push_str(&plan);merge_plan_bindings(&mut variables,bindings)?;}
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt),&sql,variables).await
}
pub(crate) async fn hold_contributor(client:&Surreal<Client>,owner:&LiveViewOwner,root:&RecordId,expected:&ExpectedRecord,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=budget.reserve("live view exact contributor hold",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let (_,holds,mut variables)=hold_plan(Some(owner.attempt),vec![(root.clone(),expected.record().clone())])?.ok_or(ModelError::Schema("live contributor hold plan"))?;
    bind(owner,&mut variables);variables.insert("contributor",expected.record().clone());variables.insert("contributor_guard",guard_id(&Value::RecordId(expected.record().clone()))?);variables.insert("contributor_incarnation",expected.incarnation());variables.insert("contributor_bytes",expected.row().clone());
    let sql=format!("{GUARD} fn::native_touch($contributor); LET $guard=SELECT * FROM ONLY $contributor_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $contributor FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$contributor_incarnation OR $actual!=$contributor_bytes OR !$actual.completed {{ THROW 'live selected contributor changed'; }}; {holds}");
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt),&sql,variables).await
}
pub(crate) async fn seal(client:&Surreal<Client>,owner:&LiveViewOwner,expected:&ExpectedRecord,revision:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=budget.reserve("live view exact member seal",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();bind(owner,&mut variables);variables.insert("realization",expected.record().clone());variables.insert("realization_guard",guard_id(&Value::RecordId(expected.record().clone()))?);variables.insert("realization_incarnation",expected.incarnation());variables.insert("realization_bytes",expected.row().clone());variables.insert("realization_revision",revision);
    let sql=format!("{GUARD} fn::native_touch($realization); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$realization]); LET $guard=SELECT * FROM ONLY $realization_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $realization FOR UPDATE; IF $predicate!=$realization_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$realization_incarnation OR $actual!=$realization_bytes {{ THROW 'live member EOF audit raced'; }}; IF type::is_bool($actual.sealed)=false {{ THROW 'live realization seal state'; }}; IF !$actual.sealed {{ UPDATE $realization SET sealed=true RETURN NONE; }}");
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt),&sql,variables).await
}
/// Authenticate the original sealed witness after a fresh full membership EOF audit.
/// Current incidence revision is a CAS premise, never replacement completion data.
pub(crate) async fn retain_installed_view(client:&Surreal<Client>,owner:&LiveViewOwner,root:&ExpectedRecord,completion:&ExpectedRecord,revision:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    if root.record().table.as_str()!="compiler_view_realization" || completion.record().table.as_str()!="compiler_view_completion" {return Err(ModelError::Conflict("live retained installed witness families"));}
    let _charge=budget.reserve("live retained installed fixed comparison",crate::loader::native_bytes(root.row()).saturating_add(crate::loader::native_bytes(completion.row())).saturating_mul(32).saturating_add(131072))?;
    let mut variables=Variables::new();bind(owner,&mut variables);
    for (prefix,expected) in [("root",root),("completion",completion)] {
        variables.insert(format!("{prefix}_record"),expected.record().clone());variables.insert(format!("{prefix}_guard"),guard_id(&Value::RecordId(expected.record().clone()))?);
        variables.insert(format!("{prefix}_incarnation"),expected.incarnation());variables.insert(format!("{prefix}_bytes"),expected.row().clone());
    }
    variables.insert("root_revision",revision);
    let sql=format!("{GUARD} LET $predicate=SELECT * FROM ONLY type::record('native_predicate',[$root_record]) FOR UPDATE; LET $rg=SELECT * FROM ONLY $root_guard FOR UPDATE; LET $root=SELECT * FROM ONLY $root_record FOR UPDATE; LET $cg=SELECT * FROM ONLY $completion_guard FOR UPDATE; LET $completion=SELECT * FROM ONLY $completion_record FOR UPDATE; IF $predicate.revision!=$root_revision OR $rg.phase!='active' OR $rg.incarnation!=$root_incarnation OR $root!=$root_bytes OR $root.sealed!=true OR $cg.phase!='active' OR $cg.incarnation!=$completion_incarnation OR $completion!=$completion_bytes OR $completion.completed!=true {{ THROW 'live retained installed witness changed after EOF audit'; }}");
    effect_for_owner(client,EffectOwner::OpenAttempt(owner.attempt),&sql,variables).await
}
