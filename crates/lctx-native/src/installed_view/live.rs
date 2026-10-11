//! Per-view completion before aggregate attempt freeze. Every backing and owned alias
//! is checked to EOF; unrelated producers may continue under the same open attempt.
use super::*;
use crate::{control::live_view,completed_state::earning::{revision,root_members,verify_members,ViewSelection}};
use d::{charged::StateCharge,completion::{Completion,complete},resources::ResourceBudget};
use std::sync::Arc;
use surrealdb::{Surreal,engine::remote::grpc::Client,types::{Object,Variables}};

pub async fn install_live_view(client:Arc<Surreal<Client>>,attempt:ContentHash,view:&d::completed::CompletedView,
    mappings:&[ContributorRealization],budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let owner=live_view::capture(&reader,attempt,budget).await?;
        let mut charge=StateCharge::new(budget,"live installed view retained descriptor");
        charge.grow(view.relation.capacity().saturating_add(view.contributions.len().saturating_mul(256)).saturating_mul(16).saturating_add(mappings.len().saturating_mul(256)).saturating_add(65536))?;
        let selection=ViewSelection{view:view.clone(),mappings:mappings.to_vec()};
        let closure=super::audit::inspect_closure(&reader,view,mappings,budget).await?;
        super::audit::audit_memberships(&reader,&closure,budget).await?;
        let aliases=super::audit::audit_aliases(&reader,&closure,budget).await?;
        let roots=super::audit::logical_roots(&reader,&closure,&[],&aliases,budget).await?;
        let realization=realization_id(view,mappings)?;let root=RecordId::new("compiler_view_realization",realization.hex());
        let members=root_members(&reader,&selection,realization,budget).await?;
        let declaration=super::realization_declaration(view,mappings,false)?;
        let existing=crate::control::inspect_expected_read(&reader,root.clone(),budget).await?;
        let declaration=if let Some(existing)=&existing {
            if existing.row()!=&declaration && existing.row()!=&super::realization_declaration(view,mappings,true)? {return Err(ModelError::Conflict("live realization declaration collision"));}
            existing.row().clone()
        } else {declaration};
        live_view::install(&client,&owner,Some(declaration),vec![(RecordId::new("native_attempt",attempt.hex()),root.clone()),(root.clone(),RecordId::new("compiler_view",view.identity.hex()))],budget).await?;
        for mapping in mappings {live_view::hold_contributor(&client,&owner,&root,&closure.contributors[&mapping.logical],budget).await?;}
        let mut cursor=members.cursor_with_budget(budget)?;
        while let Some(member)=cursor.next_row()? {
            let Some(Value::RecordId(id))=member.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("live member id"));};
            let Some(Value::RecordId(node))=member.as_object().and_then(|row|row.get("node")) else{return Err(ModelError::Schema("live member node"));};
            live_view::install(&client,&owner,Some(member.clone()),vec![(root.clone(),id.clone()),(id.clone(),node.clone())],budget).await?;
        }
        let expected=crate::control::inspect_expected_read(&reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("live realization disappeared"))?;
        let before=revision(&reader,&root).await?;
        let membership_content=verify_members(&reader,realization,&members,view.rows,budget).await?;
        live_view::seal(&client,&owner,&expected,before,budget).await?;
        let completion=InstalledViewCompletion{generation:owner.generation(),validity_epoch:owner.validity_epoch(),realization_incarnation:expected.incarnation(),version:1,view:view.clone(),realization,member_codec:MEMBER_CODEC,mappings:mappings.to_vec(),closure:roots,membership_rows:view.rows,membership_content,frozen_revision:before.checked_add(1).ok_or(ModelError::Schema("live seal revision overflow"))?};
        let value=completion.value()?;charge.grow(crate::loader::native_bytes(&value).saturating_mul(32).saturating_add(65536))?;
        live_view::install(&client,&owner,Some(value),vec![(RecordId::new("native_attempt",attempt.hex()),completion.record()?),(completion.record()?,root)],budget).await?;
        let expected=crate::control::inspect_expected_read(&reader,completion.record()?,budget).await?.ok_or(ModelError::Conflict("live completed view disappeared"))?;
        if InstalledViewCompletion::from_row(expected.row())?!=completion{return Err(ModelError::Conflict("live completion exact readback"));}
        let held=crate::control::hold_live_view_expected(&client,&owner,expected,budget).await?;
        Ok(HeldInstalledView{client:client.clone(),completion,owner:InstalledViewOwner::Live{held,epoch:owner.epoch()},_charge:charge})
    }.await;
    let mut terminal=Completion::default();terminal.step("live view private reader close",reader.close().await);complete(result,terminal)
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct InstalledMember {pub key:[u8;16],pub node:RecordId,pub content:ContentHash}
/// Compact exact membership metadata. No canonical payload is decoded for key-only demand.
pub struct SelectedMembers {rows:Vec<InstalledMember>,_charge:StateCharge}
impl SelectedMembers {pub fn rows(&self)->&[InstalledMember]{&self.rows}}
impl HeldInstalledView {
    async fn ensure_current(&self,reader:&crate::NativeReader<()>,budget:&ResourceBudget)->Result<(),ModelError>{
        if !std::ptr::eq(reader.client(),self.client.as_ref()){return Err(ModelError::Conflict("held installed view foreign native session"));}
        let expected=crate::control::inspect_expected_read(reader,self.expected_record().clone(),budget).await?.ok_or(ModelError::Conflict("held live completion missing"))?;
        if expected.incarnation()!=self.completion_incarnation() || expected.row()!=self.expected_bytes(){return Err(ModelError::Conflict("held live completion incarnation/bytes"));}
        let root=RecordId::new("compiler_view_realization",self.completion.realization.hex());
        let expected=crate::control::inspect_expected_read(reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("held live realization missing"))?;
        if expected.incarnation()!=self.completion.realization_incarnation || expected.row()!=&self.completion.realization_value()? {return Err(ModelError::Conflict("held live realization incarnation/bytes"));}
        crate::completed_state::require_hold(reader,self.owner(),self.expected_record()).await?;
        crate::completed_state::require_hold(reader,self.expected_record(),&root).await?;
        crate::completed_state::require_hold(reader,&root,&RecordId::new("compiler_view",self.completion.view.identity.hex())).await?;
        let _charge=budget.reserve("held installed current owner validity",131072)?;
        match &self.owner {
            InstalledViewOwner::Published{completed,..}=>{
                let proof=completed.retained_reader().protected_publication_marker(budget).await?;
                if proof.row()!=completed.marker(){return Err(ModelError::Conflict("held published marker changed"));}
            }
            InstalledViewOwner::Live{epoch,..}=>{
                let mut variables=Variables::new();variables.insert("owner",self.owner().clone());
                let owners:Vec<Object>=reader.query_native("SELECT generation,epoch,state FROM $owner",variables).await?;
                let [owner]=owners.as_slice() else{return Err(ModelError::Conflict("held live owner missing"));};
                if owner.get("generation")!=Some(&Value::String(self.completion.generation.hex()))
                    || owner.get("epoch")!=Some(&Value::from(*epoch))
                    || !matches!(owner.get("state"),Some(Value::String(state)) if state=="open" || state=="frozen") {
                    return Err(ModelError::Conflict("held live current generation/epoch"));
                }
            }
        }
        let installations:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
        let [installation]=installations.as_slice() else{return Err(ModelError::Conflict("held installed installation missing"));};
        if installation.get("generation")!=Some(&Value::String(self.completion.generation.hex()))
            || installation.get("validity_epoch")!=Some(&Value::from(self.completion.validity_epoch)) {return Err(ModelError::Conflict("held installed current validity"));}
        Ok(())
    }
    /// Sorted distinct nominal demand is resolved through exact v2 addresses. Missing
    /// rows mean absence only after the held complete namespace is checked, even empty.
    pub async fn members(self:&Arc<Self>,reader:&crate::NativeReader<()>,keys:&[[u8;16]],budget:&ResourceBudget)->Result<SelectedMembers,ModelError>{
        if keys.len()>crate::loader::NATIVE_WINDOW_ROWS || keys.windows(2).any(|pair|pair[0]>=pair[1]) {return Err(ModelError::Conflict("installed member demand order/duplicate/bound"));}
        let setup=budget.reserve("owned compact member demand",keys.len().saturating_mul(64).saturating_add(4096))?;
        let owner=self.clone();let reader=reader.clone();let driver_budget=budget.clone();let keys=keys.to_vec();
        // The native driver owns the cap, reader, requested keys and reservations until
        // every accepted response is terminal, even when its caller drops delivery.
        let mut stream=crate::reader::NativeRows::owned(move |sender|async move {
            let _setup=setup;let selected=owner.members_inner(&reader,&keys,&driver_budget).await?;
            for member in selected.rows() {
                let mut value=Object::new();value.insert("semantic_key",hex::encode(member.key));value.insert("node",member.node.clone());value.insert("content",member.content.hex());
                if sender.send(Value::Object(value)).await.is_err(){break;}
            }Ok(())
        })?;
        let result=async {
            let mut charge=StateCharge::new(budget,"delivered compact installed members");let mut rows=Vec::new();
            while let Some(value)=stream.next_native().await? {
                let object=value.as_object().ok_or(ModelError::Schema("owned compact member delivery"))?;
                let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("owned compact member key"));};
                let Some(Value::RecordId(node))=object.get("node") else{return Err(ModelError::Schema("owned compact member node"));};
                let Some(Value::String(content))=object.get("content") else{return Err(ModelError::Schema("owned compact member content"));};
                charge.grow(crate::selection::node_bytes(node).saturating_mul(2).saturating_add(std::mem::size_of::<InstalledMember>()).saturating_add(128))?;
                rows.push(InstalledMember{key:crate::completed_rows::decode_nominal_key(key)?,node:node.clone(),content:ContentHash(hex::decode(content).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("owned compact content width"))?)});
            }Ok(SelectedMembers{rows,_charge:charge})
        }.await;
        let mut terminal=Completion::default();terminal.step("compact installed member driver drain",stream.drain_transport().await);complete(result,terminal)
    }
    async fn members_inner(&self,reader:&crate::NativeReader<()>,keys:&[[u8;16]],budget:&ResourceBudget)->Result<SelectedMembers,ModelError>{
        if keys.len()>crate::loader::NATIVE_WINDOW_ROWS || keys.windows(2).any(|pair|pair[0]>=pair[1]){return Err(ModelError::Conflict("installed member demand order/duplicate"));}
        self.ensure_current(reader,budget).await?;
        let _window=budget.reserve("installed direct member read window",keys.len().saturating_mul(4096).saturating_add(d::resources::MAX_ROW_BYTES))?;
        let mut variables=Variables::new();variables.insert("ids",keys.iter().map(|key|self.address(&self.completion.view.relation,key)).collect::<Result<Vec<_>,_>>()?);
        let rows=if keys.is_empty(){Vec::new()}else{reader.query_native("SELECT * FROM $ids ORDER BY semantic_key",variables).await?};
        let result=decode_window(&self.completion,keys,rows,budget)?;
        let root=RecordId::new("compiler_view_realization",self.completion.realization.hex());
        for member in result.rows() {
            let id=self.address(&self.completion.view.relation,&member.key)?;
            crate::completed_state::require_hold(reader,&root,&id).await?;crate::completed_state::require_hold(reader,&id,&member.node).await?;
        }
        self.ensure_current(reader,budget).await?;Ok(result)
    }
}
fn decode_member(completion:&InstalledViewCompletion,row:&Value)->Result<InstalledMember,ModelError>{
    let object=row.as_object().ok_or(ModelError::Schema("installed direct member object"))?;
    let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("installed direct member key"));};
    let key=crate::completed_rows::decode_nominal_key(key)?;
    let Some(Value::RecordId(node))=object.get("node") else{return Err(ModelError::Schema("installed direct member node"));};
    let Some(Value::String(content))=object.get("content") else{return Err(ModelError::Schema("installed direct member content"));};
    let content=ContentHash(hex::decode(content).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("installed direct member content width"))?);
    let result=InstalledMember{key,node:node.clone(),content};
    let mut expected=Object::new();expected.insert("id",member_id(completion.realization,&completion.view.relation,&key));expected.insert("realization",RecordId::new("compiler_view_realization",completion.realization.hex()));expected.insert("relation",completion.view.relation.clone());expected.insert("semantic_key",hex::encode(key));expected.insert("node",node.clone());expected.insert("content",content.hex());
    if row!=&Value::Object(expected) || !matches!(node.table.as_str(),"entity"|"assertion"|"compiler_record"){return Err(ModelError::Conflict("installed direct member exact envelope"));}
    Ok(result)
}

/// Acquires one exact installed view from a protected publication. The origin attempt
/// may already be closed or compacted; marker/pin reachability owns this capability.
pub async fn hold_published_view(completed:&crate::completed_state::PublishedCompletedState,view:ContentHash,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let installed=completed.inventory().installed(view)?;
    let mut charge=StateCharge::new(budget,"published held installed descriptor");
    charge.grow(installed.view.relation.capacity().saturating_add(installed.view.contributions.len().saturating_mul(256)).saturating_add(installed.mappings.len().saturating_mul(256)).saturating_add(8192))?;
    let proof=completed.retained_reader().protected_publication_marker(budget).await?;
    if proof.row()!=completed.marker(){return Err(ModelError::Conflict("published installed marker changed"));}
    let reader=completed.retained_reader().transport_reader().with_budget(budget);
    let expected=crate::control::inspect_expected_read(&reader,installed.record()?,budget).await?.ok_or(ModelError::Conflict("published installed completion missing"))?;
    if expected.row()!=&installed.value()? {return Err(ModelError::Conflict("published installed completion bytes"));}
    let owner=completed.completion().record()?;
    crate::completed_state::require_hold(&reader,&owner,expected.record()).await?;
    let result=HeldInstalledView{client:completed.retained_reader().shared_client(),completion:installed.clone(),owner:InstalledViewOwner::Published{expected,completed:completed.clone(),owner},_charge:charge};
    result.ensure_current(&reader,budget).await?;Ok(result)
}

fn decode_window(completion:&InstalledViewCompletion,keys:&[[u8;16]],rows:Vec<Value>,budget:&ResourceBudget)->Result<SelectedMembers,ModelError>{
    let mut charge=StateCharge::new(budget,"held installed compact members");let mut result=Vec::new();let mut previous=None;
    if rows.len()>keys.len(){return Err(ModelError::Conflict("installed direct member result bound"));}
    for row in rows {
        let _decode=budget.reserve("installed direct member decode",crate::loader::native_bytes(&row).saturating_mul(16).saturating_add(4096))?;
        let member=decode_member(completion,&row)?;
        if keys.binary_search(&member.key).is_err() || previous.is_some_and(|key|key>=member.key){return Err(ModelError::Conflict("installed direct member foreign/duplicate/order"));}
        charge.grow(crate::selection::node_bytes(&member.node).saturating_mul(2).saturating_add(std::mem::size_of::<InstalledMember>()).saturating_add(128))?;
        previous=Some(member.key);result.push(member);
    }Ok(SelectedMembers{rows:result,_charge:charge})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn completion()->InstalledViewCompletion {
        let view=d::completed::CompletedView::new("packages".into(),[ContentHash::of(b"owner")].into(),2).unwrap();
        let mappings=vec![ContributorRealization{logical:*view.contributions.first().unwrap(),physical:ContentHash::of(b"physical"),incarnation:1}];
        InstalledViewCompletion{generation:ContentHash::of(b"generation"),validity_epoch:1,realization_incarnation:1,version:1,realization:realization_id(&view,&mappings).unwrap(),view,member_codec:MEMBER_CODEC,mappings,closure:d::completed_identity::CompletedFamilyRoots::new(d::recovery_closure::COMPLETED_FAMILIES.map(|family|d::completed_identity::CompletedFamilyHasher::new(family).unwrap().finish())).unwrap(),membership_rows:2,membership_content:ContentHash::of(b"membership"),frozen_revision:1}
    }
    fn member(completion:&InstalledViewCompletion,key:[u8;16])->Value {
        let mut row=Object::new();row.insert("id",member_id(completion.realization,&completion.view.relation,&key));row.insert("realization",RecordId::new("compiler_view_realization",completion.realization.hex()));row.insert("relation",completion.view.relation.clone());row.insert("semantic_key",hex::encode(key));row.insert("node",RecordId::new("entity",ContentHash::of(&key).hex()));row.insert("content",ContentHash::of(&key).hex());Value::Object(row)
    }
    #[test]
    fn compact_member_delivery_keeps_charge_and_nominal_order_without_payloads(){
        let completion=completion();let budget=ResourceBudget::fixed(1<<20).unwrap();let keys=[[1;16],[2;16],[3;16]];
        let result=decode_window(&completion,&keys,vec![member(&completion,keys[0]),member(&completion,keys[2])],&budget).unwrap();
        assert_eq!(result.rows().iter().map(|row|row.key).collect::<Vec<_>>(),vec![keys[0],keys[2]],"exact absent key is omitted without renumbering nominal keys");
        assert!(budget.reserved()>0,"compact delivered allocations must retain their owner");drop(result);assert_eq!(budget.reserved(),0,"last consumer releases only its retained output allocation");
    }
    #[test]
    fn compact_member_decode_refuses_corrupt_suffix_duplicate_foreign_or_rebound_namespace(){
        let completion=completion();let budget=ResourceBudget::fixed(1<<20).unwrap();let keys=[[1;16],[2;16]];
        let mut suffix=member(&completion,keys[1]);if let Value::Object(row)=&mut suffix {row.insert("content","malformed");}
        assert!(decode_window(&completion,&keys,vec![member(&completion,keys[0]),suffix],&budget).is_err(),"valid prefix must not hide malformed later member");
        assert!(decode_window(&completion,&keys,vec![member(&completion,keys[0]),member(&completion,keys[0])],&budget).is_err(),"duplicate transport cannot substitute for another nominal key");
        assert!(decode_window(&completion,&keys,vec![member(&completion,[3;16])],&budget).is_err(),"foreign nominal key cannot enter selected demand");
        let mut rebound=member(&completion,keys[0]);if let Value::Object(row)=&mut rebound {row.insert("id",member_id(ContentHash::of(b"other realization"),&completion.view.relation,&keys[0]));}
        assert!(decode_window(&completion,&keys,vec![rebound],&budget).is_err(),"same-count namespace substitution must refuse");
        assert_eq!(budget.reserved(),0,"all failed decode allocation owners release after refusal");
    }
}
