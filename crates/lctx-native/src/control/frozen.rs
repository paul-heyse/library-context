//! Fixed native transitions for frozen canonical content. No SQL is accepted from callers.
use super::*;
use crate::completed_state::FrozenContentFence;
use lctx_model::domain::{completion::{Completion,complete},resources::{ResourceBudget,MAX_ROW_BYTES}};

/// Fence every ordinary attempt mutation before auditing complete content. This does not
/// establish physical completion or semantic admission. The supplied inventory is checked
/// against actual rows by the subsequent native completion constructor.
pub async fn freeze_content_native(client:Arc<Surreal<Client>>,attempt:ContentHash,
    expected_revision:i64,bindings:ContentHash,budget:&ResourceBudget)->Result<FrozenContentFence,ModelError>{
    if expected_revision<0{return Err(ModelError::Conflict("negative native freeze revision"));}
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let _scratch=budget.reserve("native content freeze readback",MAX_ROW_BYTES)?;
        let mut variables=Variables::new();variables.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
        let owner:Vec<Object>=reader.query_native("SELECT * FROM $attempt",variables.clone()).await?;
        let [owner]=owner.as_slice() else{return Err(ModelError::Conflict("native freeze attempt cardinality"));};
        let installation:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
        let [installation]=installation.as_slice() else{return Err(ModelError::Conflict("native freeze installation cardinality"));};
        let revision=expected_revision.checked_add(1).ok_or(ModelError::Schema("native freeze revision overflow"))?;
        let fence=FrozenContentFence{attempt,generation:authorization::hash_field(installation,"generation")?,
            attempt_epoch:authorization::int_field(owner,"epoch")?,completed_revision:revision,
            validity_epoch:authorization::int_field(installation,"validity_epoch")?,bindings};
        if fence.attempt_epoch<=0 || fence.validity_epoch<=0 || authorization::hash_field(owner,"generation")?!=fence.generation{
            return Err(ModelError::Conflict("native freeze generation/epoch"));
        }
        bind_fence(&fence,&mut variables);
        variables.insert("freeze_expected_revision",expected_revision);
        if owner.get("state")==Some(&Value::String("frozen".into())) {
            validate_fence(owner,installation,&fence,false)?;
            effect_for_owner(&client,EffectOwner::FrozenAttempt(attempt),FENCE_GUARD,variables.clone()).await?;
        } else {
            if owner.get("state")!=Some(&Value::String("open".into())) || authorization::int_field(owner,"revision")?!=expected_revision{
                return Err(ModelError::Conflict("native freeze revision/state changed"));
            }
            // The owner lock is shared by every accepted OpenAttempt payload. Effects
            // accepted before freeze must be terminal; intents racing after this CAS
            // can no longer execute a canonical payload under the frozen predicate.
            let sql="IF $__owner.revision!=$freeze_expected_revision OR $__owner.admitted OR $__installation.generation!=$freeze_generation OR $__installation.validity_epoch!=$freeze_validity_epoch OR $__owner.epoch!=$freeze_attempt_epoch { THROW 'native freeze inventory changed'; }; IF array::len(SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$__attempt AND completed=false LIMIT 1)>0 { THROW 'native freeze unfinished contribution'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX owner_effects WHERE owner=$__attempt AND resolved=false AND id!=$__effect LIMIT 1)>0 { THROW 'native freeze unresolved accepted effect'; }; UPDATE $__attempt SET state='frozen',revision=$freeze_completed_revision,completed_revision=$freeze_completed_revision,validity_epoch=$freeze_validity_epoch,completed_bindings=$freeze_bindings,completed_identity=NONE,completed_state=NONE RETURN NONE";
            effect_for_owner(&client,EffectOwner::OpenAttempt(attempt),sql,variables.clone()).await?;
        }
        let owners:Vec<Object>=reader.query_native("SELECT * FROM $attempt",variables).await?;
        let [owner]=owners.as_slice() else{return Err(ModelError::Conflict("native freeze readback cardinality"));};
        validate_fence(owner,installation,&fence,false)?;
        Ok(fence)
    }.await;
    let mut completion=Completion::default();completion.step("native freeze reader close",reader.close().await);
    complete(result,completion)
}

/// Resume from the persisted freeze revision rather than deriving a second CAS revision
/// from the current owner revision. This still grants no completed/admitted authority.
pub async fn resume_content_fence(client:Arc<Surreal<Client>>,attempt:ContentHash,budget:&ResourceBudget)->Result<FrozenContentFence,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let _scratch=budget.reserve("native persisted freeze readback",MAX_ROW_BYTES)?;
        let mut variables=Variables::new();variables.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
        let owners:Vec<Object>=reader.query_native("SELECT * FROM $attempt",variables.clone()).await?;
        let [owner]=owners.as_slice() else{return Err(ModelError::Conflict("native persisted freeze cardinality"));};
        let installations:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
        let [installation]=installations.as_slice() else{return Err(ModelError::Conflict("native persisted installation cardinality"));};
        let fence=FrozenContentFence{attempt,generation:authorization::hash_field(owner,"generation")?,attempt_epoch:authorization::int_field(owner,"epoch")?,completed_revision:authorization::int_field(owner,"completed_revision")?,validity_epoch:authorization::int_field(owner,"validity_epoch")?,bindings:authorization::hash_field(owner,"completed_bindings")?};
        if fence.attempt_epoch<=0 || fence.completed_revision<=0 || fence.validity_epoch<=0{return Err(ModelError::Conflict("native persisted freeze positive epoch/revision"));}
        validate_fence(owner,installation,&fence,false)?;bind_fence(&fence,&mut variables);
        effect_for_owner(&client,EffectOwner::FrozenAttempt(attempt),FENCE_GUARD,variables.clone()).await?;Ok(fence)
    }.await;
    let mut completion=Completion::default();completion.step("native persisted freeze reader close",reader.close().await);complete(result,completion)
}
/// A maintenance owner invalidates local trust without transporting authority to another
/// installation. Host borrower drain remains the caller's independently earned obligation.
pub async fn advance_validity_epoch_native(client:&Surreal<Client>,expected_epoch:i64,budget:&ResourceBudget)->Result<(),ModelError>{
    if expected_epoch<=0 || expected_epoch==i64::MAX{return Err(ModelError::Conflict("native validity epoch range"));}
    let _charge=budget.reserve("native validity epoch transition",65536)?;
    let mut bindings=Variables::new();bindings.insert("validity_epoch",expected_epoch);
    coordination::bind_all_stripes(&mut bindings);
    let sql=format!("{} {} {}",coordination::ALL_STRIPES_GUARD,coordination::DRAINED_PREDICATE,"IF $__installation.admission_open OR $__installation.validity_epoch!=$validity_epoch { THROW 'native validity maintenance identity/admission'; }; UPDATE native_installation:current SET validity_epoch=$validity_epoch+1 RETURN NONE");
    effect_for_owner(client,EffectOwner::Installation,&sql,bindings).await
}

pub(super) const FENCE_GUARD:&str="IF $__owner.state!='frozen' OR $__owner.generation!=$freeze_generation OR $__owner.epoch!=$freeze_attempt_epoch OR $__owner.completed_revision!=$freeze_completed_revision OR $__owner.validity_epoch!=$freeze_validity_epoch OR $__owner.completed_bindings!=$freeze_bindings OR $__installation.generation!=$freeze_generation OR $__installation.validity_epoch!=$freeze_validity_epoch { THROW 'native frozen content fence changed'; };";
pub(super) fn bind_fence(fence:&FrozenContentFence,bindings:&mut Variables){
    bindings.insert("freeze_generation",fence.generation.hex());bindings.insert("freeze_attempt_epoch",fence.attempt_epoch);
    bindings.insert("freeze_completed_revision",fence.completed_revision);bindings.insert("freeze_validity_epoch",fence.validity_epoch);
    bindings.insert("freeze_bindings",fence.bindings.hex());
}
pub(super) fn validate_fence(owner:&Object,installation:&Object,fence:&FrozenContentFence,require_admitted:bool)->Result<(),ModelError>{
    if owner.get("id")!=Some(&Value::RecordId(RecordId::new("native_attempt",fence.attempt.hex())))
        || owner.get("state")!=Some(&Value::String("frozen".into()))
        || authorization::hash_field(owner,"generation")?!=fence.generation
        || authorization::int_field(owner,"epoch")?!=fence.attempt_epoch
        || authorization::int_field(owner,"completed_revision")?!=fence.completed_revision
        || authorization::int_field(owner,"validity_epoch")?!=fence.validity_epoch
        || authorization::hash_field(owner,"completed_bindings")?!=fence.bindings
        || authorization::hash_field(installation,"generation")?!=fence.generation
        || authorization::int_field(installation,"validity_epoch")?!=fence.validity_epoch
        || (require_admitted && owner.get("admitted")!=Some(&Value::Bool(true))) {
        return Err(ModelError::Conflict("native frozen content fence changed"));
    }
    Ok(())
}

/// Format-six target only; the format coordinator installs this atomically with its converter.
pub fn schema_target()->&'static str{
    "DEFINE FIELD validity_epoch ON native_installation TYPE int DEFAULT 1; DEFINE FIELD completed_revision ON native_attempt TYPE option<int>; DEFINE FIELD completed_bindings ON native_attempt TYPE option<string>; DEFINE FIELD completed_identity ON native_attempt TYPE option<string>; DEFINE FIELD completed_state ON native_attempt TYPE option<string>; DEFINE FIELD validity_epoch ON native_attempt TYPE option<int>;"
}
#[cfg(test)]mod tests{
    use super::*;
    fn fixture()->(FrozenContentFence,Object,Object){
        let fence=FrozenContentFence{generation:ContentHash::of(b"generation"),attempt:ContentHash::of(b"attempt"),attempt_epoch:7,completed_revision:3,validity_epoch:4,bindings:ContentHash::of(b"bindings")};
        let mut owner=Object::new();owner.insert("id",RecordId::new("native_attempt",fence.attempt.hex()));owner.insert("state","frozen");owner.insert("generation",fence.generation.hex());owner.insert("epoch",fence.attempt_epoch);owner.insert("completed_revision",fence.completed_revision);owner.insert("validity_epoch",fence.validity_epoch);owner.insert("completed_bindings",fence.bindings.hex());owner.insert("admitted",false);
        let mut installation=Object::new();installation.insert("generation",fence.generation.hex());installation.insert("validity_epoch",fence.validity_epoch);(fence,owner,installation)
    }
    #[test]fn durable_freeze_is_not_admission_and_each_fence_component_is_required(){
        let (fence,owner,installation)=fixture();validate_fence(&owner,&installation,&fence,false).unwrap();assert!(validate_fence(&owner,&installation,&fence,true).is_err());
        for field in ["id","state","generation","epoch","completed_revision","validity_epoch","completed_bindings"]{
            let mut changed=owner.clone();changed.insert(field,Value::None);assert!(validate_fence(&changed,&installation,&fence,false).is_err(),"changed owner field {field}");
        }
        for field in ["generation","validity_epoch"]{let mut changed=installation.clone();changed.insert(field,Value::None);assert!(validate_fence(&owner,&changed,&fence,false).is_err(),"changed installation {field}");}
    }
}
