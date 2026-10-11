//! Installation-local physical completion. Serialized data never grants freeze,
//! semantic admission or protected read authority; native factories earn those separately.
pub(crate) mod earning;
pub use earning::{ViewSelection,complete_frozen_content};
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Object, RecordId, SurrealValue, Value};

pub const VERSION: u32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealizationInventory {
    pub rows: u64,
    pub content: ContentHash,
}
/// Full completion data. Physical validity is installation-local, while `completed_state`
/// is the separate logical identity over the six complete normalized family roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenCompletion {
    pub version: u32,
    pub generation: ContentHash,
    pub attempt: ContentHash,
    pub attempt_epoch: i64,
    pub completed_revision: i64,
    pub validity_epoch: i64,
    pub completed_state: ContentHash,
    pub roots: d::completed_identity::CompletedFamilyRoots,
    pub bindings: ContentHash,
    pub realization_inventory: RealizationInventory,
}
impl FrozenCompletion {
    pub(crate) fn fence(&self)->FrozenContentFence{FrozenContentFence{generation:self.generation,attempt:self.attempt,attempt_epoch:self.attempt_epoch,completed_revision:self.completed_revision,validity_epoch:self.validity_epoch,bindings:self.bindings}}
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.version != VERSION || self.attempt_epoch <= 0
            || self.completed_revision <= 0 || self.validity_epoch <= 0
            || self.roots.identity()? != self.completed_state {
            return Err(ModelError::Conflict("frozen completion structure"));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut sink=KeySink::new("native-frozen-completion/v1");
        sink.part(b"canonical",&serde_json::to_vec(self).map_err(ModelError::codec)?);
        Ok(sink.finish())
    }
    pub fn record(&self) -> Result<RecordId, ModelError> {
        Ok(RecordId::new("native_completed_state",self.identity()?.hex()))
    }
    pub fn value(&self,completed:bool)->Result<Value,ModelError>{
        let mut row=Object::new();row.insert("id",self.record()?);
        row.insert("descriptor",surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?));
        row.insert("completed",completed);Ok(Value::Object(row))
    }
    pub fn from_row(value:&Value,require_completed:bool)->Result<Self,ModelError>{
        let Value::Object(row)=value else{return Err(ModelError::Schema("frozen completion row"));};
        let Some(Value::Bytes(bytes))=row.get("descriptor") else{return Err(ModelError::Schema("frozen completion descriptor"));};
        let result:Self=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        result.validate()?;
        let Some(Value::Bool(completed))=row.get("completed") else{return Err(ModelError::Schema("frozen completion state"));};
        if (require_completed && !*completed) || result.value(*completed)?!=*value {
            return Err(ModelError::Conflict("frozen completion exact persisted envelope"));
        }
        Ok(result)
    }
}
/// One row of the completed inventory; the count/digest lives in FrozenCompletion.
/// Rows are ordered by logical view and retain the exact installed namespace/incarnation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledViewRef {
    pub logical_view: ContentHash,
    pub realization: ContentHash,
    pub incarnation: i64,
    pub installation_completion: ContentHash,
}
impl InstalledViewRef {
    pub fn validate(&self)->Result<(),ModelError>{
        if self.incarnation<=0{return Err(ModelError::Conflict("completed view incarnation"));}Ok(())
    }
    pub fn record(&self,completion:ContentHash)->RecordId{
        let mut sink=KeySink::new("native-completed-view/v1");
        completion.encode(&mut sink);self.logical_view.encode(&mut sink);
        RecordId::new("native_completed_view",sink.finish().hex())
    }
    pub fn value(&self,completion:ContentHash)->Result<Value,ModelError>{
        self.validate()?;
        let mut row=Object::new();row.insert("id",self.record(completion));
        row.insert("completion",RecordId::new("native_completed_state",completion.hex()));
        row.insert("logical_view",self.logical_view.hex());row.insert("realization",self.realization.hex());
        row.insert("incarnation",self.incarnation);row.insert("installation_completion",self.installation_completion.hex());
        Ok(Value::Object(row))
    }
    pub fn from_row(value:&Value,completion:ContentHash)->Result<Self,ModelError>{
        let Value::Object(row)=value else{return Err(ModelError::Schema("completed view inventory row"));};
        let hash=|field:&str|->Result<ContentHash,ModelError>{
            let Some(Value::String(value))=row.get(field) else{return Err(ModelError::Schema("completed view inventory digest"));};
            Ok(ContentHash(hex::decode(value).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("completed view inventory digest length"))?))
        };
        let incarnation=i64::from_value(row.get("incarnation").cloned().ok_or(ModelError::Schema("completed view inventory incarnation"))?).map_err(ModelError::codec)?;
        let result=Self{logical_view:hash("logical_view")?,realization:hash("realization")?,incarnation,installation_completion:hash("installation_completion")?};
        if result.value(completion)?!=*value{return Err(ModelError::Conflict("completed view inventory exact envelope"));}Ok(result)
    }
}
/// Streaming inventory digest checks order and duplicates before accepting actual EOF.
pub struct RealizationHasher { rows:u64,last:Option<ContentHash>,sink:KeySink }
impl Default for RealizationHasher {fn default()->Self{Self{rows:0,last:None,sink:KeySink::new("native-completed-view-inventory/v1")}}}
impl RealizationHasher {
    pub fn row(&mut self,row:&InstalledViewRef)->Result<(),ModelError>{
        row.validate()?;
        if self.last.is_some_and(|last|last>=row.logical_view){return Err(ModelError::Conflict("completed realization inventory order/duplicate"));}
        self.rows=self.rows.checked_add(1).ok_or(ModelError::Schema("completed realization inventory count"))?;
        self.sink.part(b"row",&serde_json::to_vec(row).map_err(ModelError::codec)?);
        self.last=Some(row.logical_view);Ok(())
    }
    pub fn finish(mut self)->RealizationInventory{
        self.sink.part(b"rows",&self.rows.to_le_bytes());RealizationInventory{rows:self.rows,content:self.sink.finish()}
    }
}
/// Only a native checked constructor can create this fence. It admits no arbitrary SQL.
pub struct FrozenContentFence {
    pub(crate) generation:ContentHash,pub(crate) attempt:ContentHash,pub(crate) attempt_epoch:i64,
    pub(crate) completed_revision:i64,pub(crate) validity_epoch:i64,pub(crate) bindings:ContentHash,
}
impl FrozenContentFence {
    pub fn attempt(&self)->ContentHash{self.attempt}
    pub fn generation(&self)->ContentHash{self.generation}
    pub fn attempt_epoch(&self)->i64{self.attempt_epoch}
    pub fn completed_revision(&self)->i64{self.completed_revision}
    pub fn validity_epoch(&self)->i64{self.validity_epoch}
    pub fn bindings(&self)->ContentHash{self.bindings}
}
/// The native hold is on the exact independently read completion incarnation.
/// A DTO or matching hash cannot construct this owner.
pub struct HeldFrozenCompletedState {pub(crate) completion:FrozenCompletion,pub(crate) held:crate::control::HeldRecord,
    pub(crate) admission:crate::control::HeldRecord,pub(crate) inventory:std::sync::Arc<InstalledViewInventory>,pub(crate) _charge:Box<dyn d::resources::Reservation>}
impl HeldFrozenCompletedState {
    pub fn completion(&self)->&FrozenCompletion{&self.completion}
    pub fn owner(&self)->&RecordId{self.held.owner()}
    pub fn admission_record(&self)->&RecordId{self.admission.record()}
    pub fn admission_incarnation(&self)->i64{self.admission.incarnation()}
    pub fn inventory(&self)->&InstalledViewInventory{&self.inventory}
    pub fn incarnation(&self)->i64{self.held.incarnation()}
}

/// Resolve the exact persisted physical completion selected by this admitted frozen
/// attempt. A logical manifest/completed-state digest is never a physical completion key.
pub async fn hold_admitted_completed_state_for_attempt(
    client:std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt:ContentHash,budget:&d::resources::ResourceBudget,
)->Result<HeldFrozenCompletedState,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let _page=budget.reserve("admitted attempt exact completion lookup",d::resources::MAX_ROW_BYTES)?;
        let mut variables=surrealdb::types::Variables::new();variables.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
        let rows:Vec<Object>=reader.query_native("SELECT id,state,admitted,completed_identity FROM $attempt",variables).await?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("admitted completed attempt cardinality"));};
        if row.get("id")!=Some(&Value::RecordId(RecordId::new("native_attempt",attempt.hex())))
            || row.get("state")!=Some(&Value::String("frozen".into())) || row.get("admitted")!=Some(&Value::Bool(true)) {
            return Err(ModelError::Conflict("admitted frozen completion attempt"));
        }
        let Some(Value::String(identity))=row.get("completed_identity") else{return Err(ModelError::Schema("admitted physical completion identity"));};
        let identity=ContentHash(hex::decode(identity).map_err(ModelError::codec)?.try_into()
            .map_err(|_|ModelError::Schema("admitted physical completion identity width"))?);
        // The exact hold factory independently re-reads owner/epoch/bytes and inventory;
        // a raced or stale first lookup cannot grant authority.
        hold_completed_state(client.clone(),attempt,identity,budget).await
    }.await;
    let mut completion=d::completion::Completion::default();completion.step("admitted completion lookup reader close",reader.close().await);
    d::completion::complete(result,completion)
}

/// Acquires protected authority only from admitted persisted native state, then checks
/// every exact installed witness and the complete inventory EOF under the same epoch.
/// A decoded DTO is never accepted as input authority to this constructor.
pub async fn hold_completed_state(client:std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt:ContentHash,identity:ContentHash,budget:&d::resources::ResourceBudget)->Result<HeldFrozenCompletedState,ModelError>{
    use surrealdb::types::Variables;
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let expected=crate::control::inspect_expected_read(&reader,RecordId::new("native_completed_state",identity.hex()),budget).await?
            .ok_or(ModelError::Conflict("native frozen completion missing"))?;
        let completion_charge=budget.reserve("native frozen completion decoding",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
        let data=FrozenCompletion::from_row(expected.row(),true)?;
        if data.identity()?!=identity || data.attempt!=attempt{return Err(ModelError::Conflict("native completed owner/identity"));}
        let held=crate::control::hold_frozen_completed_expected(&client,&data,expected,budget).await?;
        let inventory=inspect_inventory(&reader,&data,budget).await?;
        let mut vars=Variables::new();vars.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
        let admissions:Vec<RecordId>=reader.query_native("SELECT VALUE completed_admission FROM $attempt",vars).await?;
        let [address]=admissions.as_slice() else{return Err(ModelError::Conflict("frozen semantic admission pointer cardinality"));};
        let admission=crate::control::inspect_expected_read(&reader,address.clone(),budget).await?.ok_or(ModelError::Conflict("frozen semantic admission missing"))?;
        let _decode=budget.reserve("frozen semantic admission decoding",crate::loader::native_bytes(admission.row()).saturating_mul(32).saturating_add(65536))?;
        let semantic=CompletedAdmission::from_row(admission.row())?;let model=d::model()?;
        semantic.validate(&data,held.incarnation(),&model)?;
        require_hold(&reader,address,&data.record()?).await?;
        super_input_verification(&reader,&semantic.premises,&inventory,&model,budget).await?;
        let admission=crate::control::hold_frozen_admission_expected(&client,&data,&semantic,admission,budget).await?;
        // Recheck owner/installation after all reads. The first hold remains retained,
        // while this fixed exact effect confirms no epoch/admission change at return.
        let expected=crate::control::inspect_expected_read(&reader,data.record()?,budget).await?.ok_or(ModelError::Conflict("native completion disappeared"))?;
        if expected.incarnation()!=held.incarnation(){return Err(ModelError::Conflict("native completed incarnation changed"));}
        let repeated=crate::control::hold_frozen_completed_expected(&client,&data,expected,budget).await?;
        drop(held);Ok(HeldFrozenCompletedState{completion:data,held:repeated,admission,inventory,_charge:completion_charge})
    }.await;
    let mut completion=d::completion::Completion::default();completion.step("native completion reader close",reader.close().await);
    d::completion::complete(result,completion)
}
pub(crate) async fn require_hold(reader:&crate::NativeReader<()>,owner:&RecordId,object:&RecordId)->Result<(),ModelError>{
    let mut bindings=surrealdb::types::Variables::new();bindings.insert("owner",owner.clone());bindings.insert("object",object.clone());
    let rows:Vec<RecordId>=reader.query_native("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object LIMIT 2",bindings).await?;
    if rows.len()!=1{return Err(ModelError::Conflict("completed exact hold incidence missing/duplicate"));}Ok(())
}
pub(crate) async fn verify_installed_ref(reader:&crate::NativeReader<()>,data:&FrozenCompletion,reference:&InstalledViewRef,budget:&d::resources::ResourceBudget)->Result<crate::installed_view::InstalledViewCompletion,ModelError>{
    let address=RecordId::new("compiler_view_completion",reference.installation_completion.hex());
    let expected=crate::control::inspect_expected_read(reader,address.clone(),budget).await?.ok_or(ModelError::Conflict("installed completion missing"))?;
    let _decode=budget.reserve("native installed completion decoding",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
    let installed=crate::installed_view::InstalledViewCompletion::from_row(expected.row())?;
    if installed.record()?!=address || installed.view.identity!=reference.logical_view || installed.realization!=reference.realization
        || installed.realization_incarnation!=reference.incarnation || installed.generation!=data.generation || installed.validity_epoch!=data.validity_epoch {
        return Err(ModelError::Conflict("completed exact installed realization/epoch"));
    }
    require_hold(reader,&data.record()?,&address).await?;
    let root=RecordId::new("compiler_view_realization",reference.realization.hex());
    let actual=crate::control::inspect_expected_read(reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("installed realization missing"))?;
    if actual.incarnation()!=reference.incarnation || actual.row()!=&installed.realization_value()?{return Err(ModelError::Conflict("completed actual realization incarnation/declaration"));}
    require_hold(reader,&address,&root).await?;
    // Exact contributor selection is checked against this realization's incidence,
    // never against another process's first matching logical contributor.
    for mapping in &installed.mappings {
        let physical=RecordId::new("compiler_contribution",mapping.physical.hex());
        let actual=crate::control::inspect_expected_read(reader,physical.clone(),budget).await?.ok_or(ModelError::Conflict("completed mapped contributor missing"))?;
        if actual.incarnation()!=mapping.incarnation{return Err(ModelError::Conflict("completed mapped contributor incarnation"));}
        crate::completed_rows::validate_state_row("compiler_contribution",actual.row())?;
        let Value::Object(row)=actual.row() else{return Err(ModelError::Schema("mapped completed contributor"));};
        let Some(Value::Bytes(bytes))=row.get("descriptor") else{return Err(ModelError::Schema("mapped completed descriptor"));};
        let _decode=budget.reserve("mapped contributor decoding",bytes.len().saturating_mul(32).saturating_add(65536))?;
        let descriptor:d::completed::CompletedContribution=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if row.get("completed")!=Some(&Value::Bool(true)) || descriptor.identity()?!=mapping.logical || row.get("logical")!=Some(&Value::String(mapping.logical.hex())) {
            return Err(ModelError::Conflict("completed mapped contributor logical identity"));
        }
        require_hold(reader,&root,&physical).await?;
    }
    Ok(installed)
}
/// Installed only by the format coordinator together with target codecs and recovery.
pub fn schema_target()->&'static str{
    "DEFINE TABLE native_completed_admission SCHEMAFULL; DEFINE FIELD descriptor ON native_completed_admission TYPE bytes; DEFINE FIELD completed_admission ON native_attempt TYPE option<record<native_completed_admission>>; DEFINE TABLE native_completed_state SCHEMAFULL; DEFINE FIELD descriptor ON native_completed_state TYPE bytes; DEFINE FIELD completed ON native_completed_state TYPE bool; DEFINE TABLE native_completed_view SCHEMAFULL; DEFINE FIELD completion ON native_completed_view TYPE record<native_completed_state>; DEFINE FIELD logical_view ON native_completed_view TYPE string; DEFINE FIELD realization ON native_completed_view TYPE string; DEFINE FIELD incarnation ON native_completed_view TYPE int; DEFINE FIELD installation_completion ON native_completed_view TYPE string; DEFINE INDEX completion_view ON native_completed_view FIELDS completion,logical_view,id UNIQUE;"
}

#[cfg(test)]
mod tests{
 use super::*;
 fn completion()->FrozenCompletion{
  use d::{completed_identity::{CompletedFamilyHasher,CompletedFamilyRoots},recovery_closure::COMPLETED_FAMILIES};
  let roots=CompletedFamilyRoots::new(COMPLETED_FAMILIES.map(|family|CompletedFamilyHasher::new(family).unwrap().finish())).unwrap();
  FrozenCompletion{version:VERSION,generation:ContentHash::of(b"installed generation"),attempt:ContentHash::of(b"attempt"),attempt_epoch:1,completed_revision:2,validity_epoch:3,completed_state:roots.identity().unwrap(),roots,bindings:ContentHash::of(b"explicit empty bindings"),realization_inventory:RealizationHasher::default().finish()}
 }
 fn view(logical:u8)->InstalledViewRef{InstalledViewRef{logical_view:ContentHash([logical;32]),realization:ContentHash::of(b"physical namespace"),incarnation:1,installation_completion:ContentHash::of(b"earned installed completion")}}
 #[test]fn every_physical_validity_component_changes_completion_but_not_logical_content(){
  let one=completion();let identity=one.identity().unwrap();
  for field in ["generation","attempt","attempt_epoch","completed_revision","validity_epoch","bindings","realization_inventory"]{
   let mut changed=one.clone();match field{
    "generation"=>changed.generation=ContentHash::of(b"replacement installation"),"attempt"=>changed.attempt=ContentHash::of(b"replacement attempt"),"attempt_epoch"=>changed.attempt_epoch+=1,"completed_revision"=>changed.completed_revision+=1,"validity_epoch"=>changed.validity_epoch+=1,"bindings"=>changed.bindings=ContentHash::of(b"other selector inventory"),_=>{let mut h=RealizationHasher::default();h.row(&view(1)).unwrap();changed.realization_inventory=h.finish();}
   }
   assert_ne!(identity,changed.identity().unwrap(),"physical completion cannot survive changed {field}");assert_eq!(one.completed_state,changed.completed_state,"logical content remains separate from installation-local completion");
  }
 }
 #[test]fn persisted_completion_requires_exact_canonical_completed_envelope(){
  let one=completion();assert_eq!(FrozenCompletion::from_row(&one.value(true).unwrap(),true).unwrap(),one);
  assert!(FrozenCompletion::from_row(&one.value(false).unwrap(),true).is_err());
  for field in ["id","extra","descriptor"]{
   let mut row=one.value(true).unwrap();let Value::Object(object)=&mut row else{unreachable!()};
   match field{"id"=>{object.insert("id",RecordId::new("native_completed_state","wrong"));},"extra"=>{object.insert("extra",Value::Null);},_=>{let mut bytes=serde_json::to_vec(&one).unwrap();bytes.push(b' ');object.insert("descriptor",surrealdb::types::Bytes::from(bytes));}}
   assert!(FrozenCompletion::from_row(&row,true).is_err(),"exact completion envelope: {field}");
  }
 }
 #[test]fn inventory_digest_refuses_duplicates_and_commits_actual_prefix_and_incarnation(){
  let mut exact=RealizationHasher::default();exact.row(&view(1)).unwrap();exact.row(&view(2)).unwrap();let expected=exact.finish();
  let mut prefix=RealizationHasher::default();prefix.row(&view(1)).unwrap();assert_ne!(prefix.finish(),expected);
  let mut changed=RealizationHasher::default();changed.row(&view(1)).unwrap();let mut second=view(2);second.incarnation+=1;changed.row(&second).unwrap();assert_ne!(changed.finish(),expected);
  let mut duplicate=RealizationHasher::default();duplicate.row(&view(1)).unwrap();assert!(duplicate.row(&view(1)).is_err());
  let owner=completion().identity().unwrap();let row=view(1).value(owner).unwrap();assert_eq!(InstalledViewRef::from_row(&row,owner).unwrap(),view(1));assert!(InstalledViewRef::from_row(&row,ContentHash::of(b"another completion")).is_err());
 }
}

/// Charged exact per-view inventory. Equivalent logical contributors selected by two
/// views remain two explicit mappings; there is no installation-global first-copy map.
pub struct InstalledViewInventory {
    references:Vec<InstalledViewRef>,
    bindings:Vec<d::completed::CompletedBinding>,
    binding_incarnations:std::collections::BTreeMap<ContentHash,i64>,
    installed:std::collections::BTreeMap<ContentHash,crate::installed_view::InstalledViewCompletion>,
    _charge:d::charged::StateCharge,
}
impl InstalledViewInventory {
    pub fn references(&self)->&[InstalledViewRef]{&self.references}
    pub fn bindings(&self)->&[d::completed::CompletedBinding]{&self.bindings}
    pub fn binding_incarnation(&self,binding:ContentHash)->Result<i64,ModelError>{self.binding_incarnations.get(&binding).copied().ok_or(ModelError::Conflict("canonical binding absent from completed inventory"))}
    pub fn view(&self,logical:ContentHash)->Result<&InstalledViewRef,ModelError>{
        self.references.binary_search_by_key(&logical,|row|row.logical_view).map(|index|&self.references[index])
            .map_err(|_|ModelError::Conflict("view absent from exact completed inventory"))
    }
    pub fn contributors(&self,logical:ContentHash)->Result<&[crate::installed_view::ContributorRealization],ModelError>{
        Ok(&self.installed(logical)?.mappings)
    }
    pub fn installed(&self,logical:ContentHash)->Result<&crate::installed_view::InstalledViewCompletion,ModelError>{
        self.installed.get(&logical).ok_or(ModelError::Conflict("installed view absent from exact completed inventory"))
    }
}
pub(crate) async fn inspect_inventory(reader:&crate::NativeReader<()>,data:&FrozenCompletion,
    budget:&d::resources::ResourceBudget)->Result<std::sync::Arc<InstalledViewInventory>,ModelError>{
    let identity=data.identity()?;let mut hasher=RealizationHasher::default();let mut after=String::new();let mut after_id=Value::None;
    let mut result=InstalledViewInventory{references:Vec::new(),bindings:Vec::new(),binding_incarnations:std::collections::BTreeMap::new(),installed:std::collections::BTreeMap::new(),_charge:d::charged::StateCharge::new(budget,"exact completed installed inventory")};
    loop {
        let _page=budget.reserve("exact completed inventory page",d::resources::MAX_ROW_BYTES)?;
        let mut variables=surrealdb::types::Variables::new();variables.insert("completion",data.record()?);variables.insert("after",after.clone());variables.insert("after_id",after_id.clone());
        let rows:Vec<Value>=reader.query_native("SELECT * FROM native_completed_view WITH INDEX completion_view WHERE completion=$completion AND (logical_view>$after OR (logical_view=$after AND id>$after_id)) ORDER BY logical_view,id LIMIT 1",variables).await?;
        if rows.is_empty(){break;}
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("exact completed inventory page bound"));};
        let reference=InstalledViewRef::from_row(row,identity)?;hasher.row(&reference)?;
        require_hold(reader,&data.record()?,&reference.record(identity)).await?;
        let installed=verify_installed_ref(reader,data,&reference,budget).await?;
        // Admit both the geometric reference capacity and owned decoded map before
        // retaining it beyond the current page/decode reservation.
        result._charge.grow(crate::loader::native_bytes(&installed.value()?).saturating_mul(32).saturating_add(2048)
            .saturating_add(2*std::mem::size_of::<InstalledViewRef>()))?;
        after=reference.logical_view.hex();after_id=Value::RecordId(reference.record(identity));
        result.references.push(reference);result.installed.insert(installed.view.identity,installed);
    }
    if hasher.finish()!=data.realization_inventory{return Err(ModelError::Conflict("exact completed inventory actual EOF/digest"));}

    let mut after=Value::None;
    loop {
        let _page=budget.reserve("completed canonical binding incidence page",d::resources::MAX_ROW_BYTES)?;
        let mut variables=surrealdb::types::Variables::new();variables.insert("completion",data.record()?);variables.insert("after",after.clone());
        let holds:Vec<Object>=reader.query_native("SELECT id,object FROM native_hold WITH INDEX owner_holds WHERE owner=$completion AND record::table(object)='compiler_binding_descriptor' AND id>$after ORDER BY id LIMIT 1",variables).await?;
        if holds.is_empty(){break;}
        let [hold]=holds.as_slice() else{return Err(ModelError::Conflict("completed binding incidence page bound"));};
        let Some(Value::RecordId(address))=hold.get("object") else{return Err(ModelError::Schema("completed binding descriptor hold"));};
        let expected=crate::control::inspect_expected_read(reader,address.clone(),budget).await?.ok_or(ModelError::Conflict("completed canonical binding missing"))?;
        result._charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(2048))?;
        let binding=crate::binding_descriptor::decode(expected.row())?;
        if result.installed(binding.view.identity)?.view!=binding.view{return Err(ModelError::Conflict("completed canonical binding exact installed view"));}
        require_hold(reader,address,&RecordId::new("compiler_view",binding.view.identity.hex())).await?;
        let identity=binding.descriptor_identity()?;
        if result.binding_incarnations.insert(identity,expected.incarnation()).is_some(){return Err(ModelError::Conflict("completed duplicate canonical binding incidence"));}
        result.bindings.push(binding);
        after=hold.get("id").cloned().ok_or(ModelError::Schema("completed binding hold cursor"))?;
    }
    if d::completed::binding_inventory_identity(&result.bindings)?!=data.bindings{return Err(ModelError::Conflict("completed canonical binding actual EOF/inventory"));}
    result.bindings.sort_by_key(|binding|binding.key());
    Ok(std::sync::Arc::new(result))
}

/// Physical completion precedes semantic admission. This private-field owner can only
/// be earned by the independent native completion writer under a durable frozen fence.
pub struct NativeCompletedState {
    pub(crate) completion:FrozenCompletion,
    pub(crate) held:crate::control::HeldRecord,
    pub(crate) inventory:std::sync::Arc<InstalledViewInventory>,
    pub(crate) _charge:d::charged::StateCharge,
}
impl NativeCompletedState {
    pub fn completion(&self)->&FrozenCompletion{&self.completion}
    pub fn incarnation(&self)->i64{self.held.incarnation()}
    pub fn inventory(&self)->&InstalledViewInventory{&self.inventory}
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedAdmission {
    pub version:u32,pub completion:ContentHash,pub completion_incarnation:i64,
    pub generation:ContentHash,pub validity_epoch:i64,
    pub premises:d::checked_admission::AdmissionPremises,
}
impl CompletedAdmission {
    fn validate(&self,data:&FrozenCompletion,incarnation:i64,model:&d::ValidatedModel)->Result<(),ModelError>{
        if self.version!=1 || self.completion!=data.identity()? || self.completion_incarnation!=incarnation
            || incarnation<=0 || self.generation!=data.generation || self.validity_epoch!=data.validity_epoch {
            return Err(ModelError::Conflict("semantic admission exact physical completion/epoch"));
        }
        self.premises.validate_current(model)
    }
    pub fn identity(&self)->Result<ContentHash,ModelError>{let mut sink=KeySink::new("native-completed-admission/v1");sink.part(b"canonical",&serde_json::to_vec(self).map_err(ModelError::codec)?);Ok(sink.finish())}
    pub fn record(&self)->Result<RecordId,ModelError>{Ok(RecordId::new("native_completed_admission",self.identity()?.hex()))}
    pub fn value(&self)->Result<Value,ModelError>{let mut row=Object::new();row.insert("id",self.record()?);row.insert("descriptor",surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?));Ok(Value::Object(row))}
    pub(crate) fn from_row(row:&Value)->Result<Self,ModelError>{
        let Some(Value::Bytes(bytes))=row.as_object().and_then(|row|row.get("descriptor")) else{return Err(ModelError::Schema("native completed admission descriptor"));};
        let data:Self=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if data.value()?!=*row{return Err(ModelError::Conflict("native completed admission canonical envelope"));}Ok(data)
    }
}

#[derive(Clone)]
pub struct PublishedCompletedState {inner:std::sync::Arc<PublishedCompletedInner>}
struct PublishedCompletedInner {
    completion:FrozenCompletion,completion_incarnation:i64,
    admission:CompletedAdmission,admission_incarnation:i64,
    inventory:std::sync::Arc<InstalledViewInventory>,
    marker:crate::reader::ProtectedPublicationMarker,
    // This is a retained borrowed scope/pin, never a separately owned session to close.
    reader:crate::NativeReader<d::serving::SnapshotHandle>,
    _charge:Box<dyn d::resources::Reservation>,
}
impl PublishedCompletedState {
    pub fn completion(&self)->&FrozenCompletion{&self.inner.completion}
    pub fn incarnation(&self)->i64{self.inner.completion_incarnation}
    pub fn admission_record(&self)->Result<RecordId,ModelError>{self.inner.admission.record()}
    pub fn admission_incarnation(&self)->i64{self.inner.admission_incarnation}
    pub fn inventory(&self)->&InstalledViewInventory{&self.inner.inventory}
    pub(crate) fn marker(&self)->&Object{self.inner.marker.row()}
    pub(crate) fn admission(&self)->&CompletedAdmission{&self.inner.admission}
    pub(crate) fn retained_reader(&self)->&crate::NativeReader<d::serving::SnapshotHandle>{&self.inner.reader}
}
/// Marker/pin reachability and persisted semantic admission survive legitimate origin
/// history compaction. No origin attempt lookup is made on this publication read path.
pub async fn published_completed_state(reader:&crate::NativeReader<d::serving::SnapshotHandle>,
    budget:&d::resources::ResourceBudget)->Result<PublishedCompletedState,ModelError>{
    let marker=reader.protected_publication_marker(budget).await?;
    let record=|field:&str,table:&str|->Result<RecordId,ModelError>{match marker.row().get(field){Some(Value::RecordId(id)) if id.table.as_str()==table=>Ok(id.clone()),_=>Err(ModelError::Schema("published completed pointer"))}};
    let completion_id=record("completion","native_completed_state")?;let admission_id=record("admission","native_completed_admission")?;
    let transport=reader.transport_reader().with_budget(budget);
    let expected=crate::control::inspect_expected_read(&transport,completion_id.clone(),budget).await?.ok_or(ModelError::Conflict("published physical completion missing"))?;
    let _scratch=budget.reserve("published completion decode",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
    let data=FrozenCompletion::from_row(expected.row(),true)?;
    if data.record()?!=completion_id || data.generation!=reader.handle().service_generation
        || marker.row().get("completion_incarnation")!=Some(&Value::from(expected.incarnation())) {
        return Err(ModelError::Conflict("published physical completion identity/incarnation"));
    }
    let admission=crate::control::inspect_expected_read(&transport,admission_id.clone(),budget).await?.ok_or(ModelError::Conflict("published semantic admission missing"))?;
    let mut charge=budget.reserve("published completed retained data",crate::loader::native_bytes(expected.row()).saturating_add(crate::loader::native_bytes(admission.row())).saturating_mul(32).saturating_add(65536))?;
    let semantic=CompletedAdmission::from_row(admission.row())?;
    let model=d::model()?;semantic.validate(&data,expected.incarnation(),&model)?;
    if semantic.record()?!=admission_id || marker.row().get("admission_incarnation")!=Some(&Value::from(admission.incarnation())) {
        return Err(ModelError::Conflict("published semantic admission identity/incarnation"));
    }
    require_hold(&transport,&semantic.record()?,&data.record()?).await?;
    let inventory=inspect_inventory(&transport,&data,budget).await?;
    super_input_verification(&transport,&semantic.premises,&inventory,&model,budget).await?;
    let repeated=reader.protected_publication_marker(budget).await?;
    if repeated.row()!=marker.row(){return Err(ModelError::Conflict("published marker changed during completed factory"));}
    charge.try_resize(crate::loader::native_bytes(&data.value(true)?).saturating_add(crate::loader::native_bytes(&semantic.value()?)).saturating_mul(32).saturating_add(65536))?;
    Ok(PublishedCompletedState{inner:std::sync::Arc::new(PublishedCompletedInner{completion:data,completion_incarnation:expected.incarnation(),admission:semantic,admission_incarnation:admission.incarnation(),inventory,marker,reader:reader.clone(),_charge:charge})})
}

/// Independent native fingerprinting uses full typed rows in declared order. Native ID
/// order, batch framing and an asserted SourceSnapshot never replace this readback.
async fn super_input_verification(reader:&crate::NativeReader<()>,premises:&d::checked_admission::AdmissionPremises,
    inventory:&InstalledViewInventory,model:&d::ValidatedModel,budget:&d::resources::ResourceBudget)->Result<(),ModelError>{
    use d::{Record,completion::{Completion,complete}};
    use arrow_row::{RowConverter,SortField};use arrow_schema::SortOptions;
    premises.validate_current(model)?;
    for proof in &premises.validations {
        let declared=match proof.kind {0=>&model.invariant(&proof.name)?.inputs,1=>&model.publication_check(&proof.name)?.inputs,
            _=>return Err(ModelError::Conflict("native premise validation kind"))};
        if declared.len()!=proof.inputs.len(){return Err(ModelError::Conflict("native premise ordered role count"));}
        for (input,actual) in declared.iter().zip(&proof.inputs) {
            let installed=inventory.installed(actual.source.view())?;
            let relation=model.relation(input.name()).ok_or(ModelError::Schema(input.name()))?;
            let expected=d::analysis::sources::SourceSnapshot::of_completed_view(relation,model.digest(),&installed.view)?;
            let exact_binding=inventory.bindings().iter().find(|binding|binding.source==actual.source
                && binding.boundary.as_deref()==actual.resolved.map(|boundary|boundary.name()));
            if actual.source!=expected || exact_binding.is_none() || actual.rows!=installed.view.rows || actual.requested!=input.prefix(){
                return Err(ModelError::Conflict("native premise exact selected source metadata"));
            }
            let mut ordered=crate::ordered_rows::SortedValidationOrder::new(budget)?;
            let mut variables=surrealdb::types::Variables::new();variables.insert("realization",RecordId::new("compiler_view_realization",installed.realization.hex()));
            let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_view_member WITH INDEX realization_nodes WHERE realization=$realization".into()])?)?;
            let result=async {
                let mut count=0u64;
                while let Some(member)=rows.next_native().await? {
                    let _member_scratch=budget.reserve("native validation member scratch",crate::loader::native_bytes(&member).saturating_mul(32).saturating_add(65536))?;
                    let object=member.as_object().ok_or(ModelError::Schema("native validation member object"))?;
                    let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("native validation member key"));};
                    let key:[u8;16]=hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("native validation member key width"))?;
                    let Some(Value::RecordId(node))=object.get("node") else{return Err(ModelError::Schema("native validation member backing"));};
                    require_hold(reader,&RecordId::new("compiler_view_realization",installed.realization.hex()),&crate::installed_view::member_id(installed.realization,input.name(),&key)).await?;
                    require_hold(reader,&crate::installed_view::member_id(installed.realization,input.name(),&key),node).await?;
                    if object.get("id")!=Some(&Value::RecordId(crate::installed_view::member_id(installed.realization,input.name(),&key)))
                        || object.get("realization")!=Some(&Value::RecordId(RecordId::new("compiler_view_realization",installed.realization.hex())))
                        || object.get("relation")!=Some(&Value::String(input.name().into())) {
                        return Err(ModelError::Conflict("native validation exact member envelope"));
                    }
                    let expected=crate::control::inspect_expected_read(reader,node.clone(),budget).await?.ok_or(ModelError::Conflict("native validation backing missing"))?;
                    let object=expected.row().as_object().ok_or(ModelError::Schema("native validation backing object"))?;
                    if object.get("semantic_type")!=member.as_object().and_then(|row|row.get("relation"))
                        || object.get("semantic_key")!=member.as_object().and_then(|row|row.get("semantic_key"))
                        || object.get("content")!=member.as_object().and_then(|row|row.get("content")) {
                        return Err(ModelError::Conflict("native validation selected backing metadata"));
                    }
                    let original=if input.name()==d::artifact::ArtifactChunk::NAME {
                        Some(crate::completed_rows::validate_state_row("compiler_record",expected.row())?.ok_or(ModelError::Schema("native validation original metadata"))?)
                    } else {None};
                    // Keep the full typed original, its canonical Arrow copies and sort/hash
                    // construction charged together. Physical ingress pieces are smaller than
                    // the model's ArtifactChunk and must not set its semantic row boundary.
                    let original_scratch=original.as_ref().map_or(0,|original|original.len.saturating_mul(32));
                    let _typed=budget.reserve("native validation full typed row construction",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(original_scratch).saturating_add(65536))?;
                    let batch=if input.name()==d::artifact::ArtifactChunk::NAME {
                        let original=original.ok_or(ModelError::Schema("native validation original metadata"))?;
                        let source=RecordId::new("original",d::graph::EntityId::of(original.artifact).0.hex());
                        let mut bytes=Vec::with_capacity(original.len);
                        // The exact selected member, backing and holds above authorize this
                        // internal read. Re-entering public selection would add a legacy
                        // membership dependency while earning the new publication owner.
                        while bytes.len()<original.len {
                            let length=(original.len-bytes.len()).min(256*1024);
                            let start=original.start.checked_add(bytes.len() as u64).ok_or(ModelError::Schema("native validation original range"))?;
                            let part=crate::originals::read_ranges(reader,&[(source.clone(),start,length)],budget).await?.pop().ok_or(ModelError::Conflict("native validation original range missing"))?;
                            if part.len()!=length{return Err(ModelError::Conflict("native validation original range length"));}
                            bytes.extend_from_slice(&part);
                        }
                        if ContentHash::of(&bytes)!=original.digest{return Err(ModelError::Conflict("native validation original full bytes"));}
                        let ordinal=i64::try_from(original.start/(d::artifact::ARTIFACT_CHUNK_BYTES as u64)).map_err(ModelError::codec)?;
                        d::artifact::ArtifactChunk::encode(&[d::artifact::ArtifactChunk{artifact:original.artifact,ordinal,body:d::EvidenceBytes(bytes)}])?
                    } else {
                        let Some(Value::Object(body))=object.get("body") else{return Err(ModelError::Schema("native validation typed body"));};
                        let mut body=body.clone();body.insert("id",hex::encode(key));
                        let mut builder=crate::projected_arrow::ProjectedBuilder::new(relation.clone(),relation.schema().clone(),budget)?;
                        builder.push(Value::Object(body))?;builder.finish()?
                    };
                    let batch=relation.ordered_canonical(&batch)?;
                    let fingerprints=relation.row_fingerprints(&batch)?;
                    let [(nominal,content)]=fingerprints.as_slice() else{return Err(ModelError::Conflict("native validation one-row lowering"));};
                    if *nominal!=key{return Err(ModelError::Conflict("native validation typed nominal identity"));}
                    let columns=input.order().iter().map(|name|batch.column_by_name(name).cloned().ok_or(ModelError::Schema("native validation declared order field"))).collect::<Result<Vec<_>,_>>()?;
                    let converter=RowConverter::new(columns.iter().map(|column|SortField::new_with_options(column.data_type().clone(),SortOptions{descending:false,nulls_first:false})).collect()).map_err(ModelError::codec)?;
                    let order=converter.convert_columns(&columns).map_err(ModelError::codec)?;
                    let tuple=order.row(0);let _tuple=budget.reserve("native validation sort tuple copy",tuple.as_ref().len().saturating_mul(6).saturating_add(1024))?;
                    ordered.push(crate::ordered_rows::ValidationOrderRow{order:tuple.as_ref().to_vec(),nominal:*nominal,content:*content})?;
                    count=count.checked_add(1).ok_or(ModelError::Schema("native validation input count"))?;
                }
                if count!=actual.rows{return Err(ModelError::Conflict("native validation member actual EOF count"));}
                Ok(count)
            }.await;
            let mut completion=Completion::default();completion.step("native validation input stream drainage",rows.drain_transport().await);
            let count=complete(result,completion)?;
            let mut ordered=ordered.finish()?;let mut sink=KeySink::new("validation-ordered-typed-input/v1");input.encode_contract(&mut sink);
            let mut seen=0u64;
            while let Some(row)=ordered.next()? {sink.part(b"nominal",&row.nominal);row.content.encode(&mut sink);seen=seen.checked_add(1).ok_or(ModelError::Schema("native validation sorted count"))?;}
            sink.part(b"rows",&seen.to_le_bytes());
            if seen!=count || sink.finish()!=actual.ordered_content{return Err(ModelError::Conflict("native executed premise full typed content/order differs"));}
        }
        // Compact ownership is not inferred from source metadata or descriptor hashes.
        // Until exact member-owner proof is integrated, any such premise refuses.
        if !proof.uniform_owners.is_empty(){return Err(ModelError::Conflict("native uniform premise requires exact selected owner proof"));}
    }
    Ok(())
}

/// Integration seam, deliberately closed until executed evidence includes the exact
/// profile/frontier applicability and facts/producer/reference/graph admission gates.
/// Model checks alone are not full artifact admission. In particular this entrypoint
/// must not persist admitted=true while those owners and ordinary/restore consumers
/// are absent. No request or native effect is accepted by this incomplete transition.
pub async fn mark_completed_admitted(_client:std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    _physical:NativeCompletedState,_premises:&d::checked_admission::CheckedAdmissionPremises,_budget:&d::resources::ResourceBudget)->Result<HeldFrozenCompletedState,ModelError>{
    Err(ModelError::Conflict("complete artifact admission evidence integration is required"))
}
