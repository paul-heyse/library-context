//! Per-view completion before aggregate attempt freeze. Every backing and owned alias
//! is checked to EOF; unrelated producers may continue under the same open attempt.
use super::*;
use crate::{control::live_view,completed_state::earning::{revision,root_members,verify_members,ViewSelection}};
use d::{charged::StateCharge,completion::{Completion,complete},resources::ResourceBudget};
use std::sync::Arc;
use surrealdb::{Surreal,engine::remote::grpc::Client,types::{Object,Variables}};

/// Parent-private constructor after terminal product recovery has independently
/// audited and pinned the exact persisted namespace and every selected contributor.
pub(crate) fn recovery_view(client:Arc<Surreal<Client>>,pin:Arc<crate::control::ReaderPin>,completion:InstalledViewCompletion,
    expected:crate::control::ExpectedRecord,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    pin.ensure_live()?;
    if expected.record()!=&completion.record()? || expected.row()!=&completion.value()? {return Err(ModelError::Conflict("recovery installed exact completion envelope"));}
    let mut charge=StateCharge::new(budget,"recovery installed view retained descriptor");
    charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
    let owner=RecordId::new("native_pin",pin.identity.hex());
    Ok(HeldInstalledView{client,completion,owner:InstalledViewOwner::Recovery{expected,pin,owner},_charge:charge})
}

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
        if let Some(existing)=&existing {
            if existing.row()==&super::realization_declaration(view,mappings,true)? {
                let current_revision=revision(&reader,&root).await?;
                let membership_content=verify_members(&reader,realization,&members,view.rows,budget).await?;
                let metadata=InstalledViewCompletion{generation:owner.generation(),validity_epoch:owner.validity_epoch(),realization_incarnation:existing.incarnation(),version:1,view:view.clone(),realization,member_codec:MEMBER_CODEC,mappings:mappings.to_vec(),closure:roots,membership_rows:view.rows,membership_content,frozen_revision:0};
                let original=super::audit::original_held_completion(&reader,&RecordId::new("native_attempt",attempt.hex()),&metadata,current_revision,budget).await?;
                crate::completed_state::require_hold(&reader,original.expected.record(),&root).await?;
                crate::completed_state::require_hold(&reader,&root,&RecordId::new("compiler_view",view.identity.hex())).await?;
                for mapping in mappings {crate::completed_state::require_hold(&reader,&root,&RecordId::new("compiler_contribution",mapping.physical.hex())).await?;}
                live_view::retain_installed_view(&client,&owner,existing,&original.expected,current_revision,budget).await?;
                charge.grow(crate::loader::native_bytes(original.expected.row()).saturating_mul(32).saturating_add(65536))?;
                let held=crate::control::hold_live_view_expected(&client,&owner,original.expected,budget).await?;
                return Ok(HeldInstalledView{client:client.clone(),completion:original.completion,owner:InstalledViewOwner::Live{held,epoch:owner.epoch()},_charge:charge});
            }
        }
        let declaration=if let Some(existing)=&existing {
            if existing.row()!=&declaration && existing.row()!=&super::realization_declaration(view,mappings,true)? {return Err(ModelError::Conflict("live realization declaration collision"));}
            existing.row().clone()
        } else {declaration};
        live_view::install(&client,&owner,Some(declaration),vec![(RecordId::new("native_attempt",attempt.hex()),root.clone()),(root.clone(),RecordId::new("compiler_view",view.identity.hex()))],budget).await?;
        for mapping in mappings {live_view::hold_contributor(&client,&owner,&root,&closure.contributors[&mapping.physical],budget).await?;}
        live_view::install_members(&client,&owner,&root,&members,budget).await?;
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
// A cancelled or panicking native driver must preserve the borrowed recovery pin.
// A confirmed logical refusal has no outstanding remote work and can release it.
struct InstalledMemberLifetime {owner:Arc<HeldInstalledView>,confirmed:bool}
impl Drop for InstalledMemberLifetime {fn drop(&mut self){if !self.confirmed {self.owner.retain_unknown();}}}
impl HeldInstalledView {
    async fn ensure_current(&self,reader:&crate::NativeReader<()>,budget:&ResourceBudget)->Result<(),ModelError>{
        // Session identity is not physical authority. The receiving session must
        // independently match the full protected rows, holds, incarnations, owner
        // epoch and installation below, including when borrowing another session.
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
            InstalledViewOwner::Recovery{pin,..}=>pin.ensure_live()?,
            InstalledViewOwner::Published{completed,..}=>{
                let proof=completed.retained_reader().protected_publication_marker(budget).await?;
                if proof.row()!=completed.marker(){return Err(ModelError::Conflict("held published marker changed"));}
            }
            InstalledViewOwner::Physical{completed,..}=>{
                let data=completed.completion();
                let expected=crate::control::inspect_expected_read(reader,data.record()?,budget).await?.ok_or(ModelError::Conflict("held source completion missing"))?;
                if expected.incarnation()!=completed.incarnation() || expected.row()!=&data.value(true)? {return Err(ModelError::Conflict("held source completion changed"));}
                crate::completed_state::require_hold(reader,completed.held.owner(),&data.record()?).await?;
                let mut variables=Variables::new();variables.insert("owner",completed.held.owner().clone());
                let rows:Vec<Object>=reader.query_native("SELECT generation,epoch,state,admitted,completed_identity FROM $owner",variables).await?;
                let [owner]=rows.as_slice() else{return Err(ModelError::Conflict("held source owner missing"));};
                if owner.get("generation")!=Some(&Value::String(data.generation.hex())) || owner.get("epoch")!=Some(&Value::Number(surrealdb::types::Number::Int(data.attempt_epoch)))
                    || owner.get("state")!=Some(&Value::String("frozen".into())) || owner.get("completed_identity")!=Some(&Value::String(data.identity()?.hex())) {return Err(ModelError::Conflict("held source owner validity changed"));}
            }
            InstalledViewOwner::Frozen{completed,..}=>{
                let data=completed.completion();
                let expected=crate::control::inspect_expected_read(reader,data.record()?,budget).await?.ok_or(ModelError::Conflict("held source completion missing"))?;
                if expected.incarnation()!=completed.incarnation() || expected.row()!=&data.value(true)? {return Err(ModelError::Conflict("held source completion changed"));}
                crate::completed_state::require_hold(reader,completed.admission_record(),&data.record()?).await?;
                let mut variables=Variables::new();variables.insert("owner",completed.owner().clone());
                let rows:Vec<Object>=reader.query_native("SELECT generation,epoch,state,admitted,completed_identity FROM $owner",variables).await?;
                let [owner]=rows.as_slice() else{return Err(ModelError::Conflict("held source owner missing"));};
                if owner.get("generation")!=Some(&Value::String(data.generation.hex())) || owner.get("epoch")!=Some(&Value::Number(surrealdb::types::Number::Int(data.attempt_epoch)))
                    || owner.get("state")!=Some(&Value::String("frozen".into())) || owner.get("admitted")!=Some(&Value::Bool(true)) || owner.get("completed_identity")!=Some(&Value::String(data.identity()?.hex())) {return Err(ModelError::Conflict("held source owner validity changed"));}
            }
            InstalledViewOwner::Live{epoch,..}=>{
                let mut variables=Variables::new();variables.insert("owner",self.owner().clone());
                let owners:Vec<Object>=reader.query_native("SELECT generation,epoch,state FROM $owner",variables).await?;
                let [owner]=owners.as_slice() else{return Err(ModelError::Conflict("held live owner missing"));};
                if owner.get("generation")!=Some(&Value::String(self.completion.generation.hex()))
                    || owner.get("epoch")!=Some(&Value::Number(surrealdb::types::Number::Int(*epoch)))
                    || !matches!(owner.get("state"),Some(Value::String(state)) if state=="open" || state=="frozen") {
                    return Err(ModelError::Conflict("held live current generation/epoch"));
                }
            }
        }
        let installations:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
        let [installation]=installations.as_slice() else{return Err(ModelError::Conflict("held installed installation missing"));};
        check_installation(&self.completion,installation)

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
            let _setup=setup;let mut lifetime=InstalledMemberLifetime{owner:owner.clone(),confirmed:false};
            let result=owner.members_inner(&reader,&keys,&driver_budget).await;
            lifetime.confirmed=result.as_ref().err().is_none_or(ModelError::permits_storage_cleanup);
            let selected=result?;
            for member in selected.rows() {
                let mut value=Object::new();value.insert("semantic_key",hex::encode(member.key));value.insert("node",member.node.clone());value.insert("content",member.content.hex());
                if sender.send(Value::Object(value)).await.is_err(){break;}
            }Ok(())
        })?;
        let result=async {
            let mut charge=StateCharge::new(budget,"delivered compact installed members");let mut rows=Vec::new();
            while let Some(value)=stream.next().await? {
                let object=value.as_object().ok_or(ModelError::Schema("owned compact member delivery"))?;
                let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("owned compact member key"));};
                let Some(Value::RecordId(node))=object.get("node") else{return Err(ModelError::Schema("owned compact member node"));};
                let Some(Value::String(content))=object.get("content") else{return Err(ModelError::Schema("owned compact member content"));};
                charge.grow(crate::selection::node_bytes(node).saturating_mul(2).saturating_add(std::mem::size_of::<InstalledMember>()).saturating_add(128))?;
                rows.push(InstalledMember{key:crate::completed_rows::decode_nominal_key(key)?,node:node.clone(),content:ContentHash(hex::decode(content).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("owned compact content width"))?)});
            }Ok(SelectedMembers{rows,_charge:charge})
        }.await;
        let mut terminal=Completion::default();terminal.step("compact installed member driver drain",stream.drain_transport().await);
        let result=complete(result,terminal);
        if result.as_ref().err().is_some_and(|error|!error.permits_storage_cleanup()) {self.retain_unknown();}
        result
    }
    async fn members_inner(&self,reader:&crate::NativeReader<()>,keys:&[[u8;16]],budget:&ResourceBudget)->Result<SelectedMembers,ModelError>{
        if keys.len()>crate::loader::NATIVE_WINDOW_ROWS || keys.windows(2).any(|pair|pair[0]>=pair[1]){return Err(ModelError::Conflict("installed member demand order/duplicate"));}
        self.ensure_current(reader,budget).await?;
        let _window=budget.reserve("installed direct member read window",keys.len().saturating_mul(4096).saturating_add(4096))?;
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

/// Source-owned exact view acquisition. Native frozen admission, exact installed
/// inventory and current physical witnesses are required before a capability exists.
pub async fn hold_frozen_view(client:Arc<Surreal<Client>>,completed:Arc<crate::completed_state::HeldFrozenCompletedState>,
    view:ContentHash,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let reference=completed.inventory().view(view)?;
        let installed=crate::completed_state::verify_installed_ref(&reader,completed.completion(),reference,budget).await?;
        let expected=crate::control::inspect_expected_read(&reader,installed.record()?,budget).await?.ok_or(ModelError::Conflict("source installed completion missing"))?;
        let mut charge=StateCharge::new(budget,"frozen selected view descriptor");
        charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(8192))?;
        let owner=completed.completion().record()?;
        let held=HeldInstalledView{client:client.clone(),completion:installed,owner:InstalledViewOwner::Frozen{expected,completed,owner},_charge:charge};
        held.ensure_current(&reader,budget).await?;Ok(held)
    }.await;
    let mut terminal=Completion::default();terminal.step("source held view reader close",reader.close().await);complete(result,terminal)
}
/// Frozen physical completion grants exact selected reads before semantic admission.
/// It is intentionally insufficient for publication or any admitted-product claim.
pub async fn hold_physical_view(client:Arc<Surreal<Client>>,completed:Arc<crate::completed_state::NativeCompletedState>,
    view:ContentHash,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let reference=completed.inventory().view(view)?;
        let installed=crate::completed_state::verify_installed_ref(&reader,completed.completion(),reference,budget).await?;
        let expected=crate::control::inspect_expected_read(&reader,installed.record()?,budget).await?.ok_or(ModelError::Conflict("physical installed completion missing"))?;
        let mut charge=StateCharge::new(budget,"physical selected view descriptor");
        charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(8192))?;
        let owner=completed.completion().record()?;
        let held=HeldInstalledView{client:client.clone(),completion:installed,owner:InstalledViewOwner::Physical{expected,completed,owner},_charge:charge};
        held.ensure_current(&reader,budget).await?;Ok(held)
    }.await;
    let mut terminal=Completion::default();terminal.step("physical held view reader close",reader.close().await);complete(result,terminal)
}

/// Atomically transfer complete immutable namespaces and exact contributor ownership.
/// The whole source inventory is checked before the one receiving native effect.
pub async fn attach_live_views(client:Arc<Surreal<Client>>,attempt:ContentHash,sources:&[Arc<HeldInstalledView>],
    contributors:Vec<crate::control::ExpectedRecord>,budget:&ResourceBudget)->Result<Vec<HeldInstalledView>,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let owner=live_view::capture(&reader,attempt,budget).await?;
        let _inventory=budget.reserve("attached source witness inventory",sources.len().saturating_mul(std::mem::size_of::<crate::control::ExpectedRecord>()+std::mem::size_of::<StateCharge>()+std::mem::size_of::<HeldInstalledView>()).saturating_add(1024))?;
        let mut expected=Vec::with_capacity(sources.len());let mut charges=Vec::with_capacity(sources.len());
        let mut selected_contributors=std::collections::BTreeMap::new();
        for record in contributors {
            let id=record.record().clone();
            if !sources.iter().any(|source|source.completion.mappings.iter().any(|mapping|id==RecordId::new("compiler_contribution",mapping.physical.hex()))) {return Err(ModelError::Conflict("attached contributor outside selected inventory"));}
            if selected_contributors.insert(id,record).is_some(){return Err(ModelError::Conflict("attached duplicate exact contributor"));}
        }
        for source in sources {
            source.ensure_current(&reader,budget).await?;
            if source.completion.generation!=owner.generation() || source.completion.validity_epoch!=owner.validity_epoch(){return Err(ModelError::Conflict("attached source current installation"));}
            // Every selected dependency copy participates in the transaction's
            // fixed expected-incarnation checks, including shared physical copies.
            for mapping in &source.completion.mappings {
                let id=RecordId::new("compiler_contribution",mapping.physical.hex());
                if !selected_contributors.contains_key(&id) {
                    let selected=crate::control::inspect_expected_read(&reader,id.clone(),budget).await?.ok_or(ModelError::Conflict("attached selected contributor missing"))?;
                    selected_contributors.insert(id.clone(),selected);
                }
                let selected=&selected_contributors[&id];
                if selected.incarnation()!=mapping.incarnation{return Err(ModelError::Conflict("attached selected contributor incarnation"));}
                let _decode=budget.reserve("attached selected contributor decode",crate::loader::native_bytes(selected.row()).saturating_mul(32).saturating_add(65536))?;
                let descriptor:d::completed::CompletedContribution=super::audit::descriptor(selected.row())?;
                if descriptor.identity()?!=mapping.logical{return Err(ModelError::Conflict("attached selected contributor logical identity"));}
            }
            let record=crate::control::inspect_expected_read(&reader,source.completion.record()?,budget).await?.ok_or(ModelError::Conflict("attached source completion missing"))?;
            if record.incarnation()!=source.completion_incarnation() || record.row()!=source.expected_bytes(){return Err(ModelError::Conflict("attached source completion incarnation/bytes"));}
            let mut charge=StateCharge::new(budget,"attached installed view descriptor");charge.grow(crate::loader::native_bytes(record.row()).saturating_mul(32).saturating_add(8192))?;
            expected.push(record);charges.push(charge);
        }
        // No effect above mutates the receiver. Every incarnation/byte comparison
        // and incoming reference is now committed in one fixed native transaction.
        let held=crate::control::hold_live_views_expected(&client,&owner,expected,selected_contributors.into_values().collect(),budget).await?;
        Ok(held.into_iter().zip(sources).zip(charges).map(|((held,source),charge)|HeldInstalledView{client:client.clone(),completion:source.completion.clone(),owner:InstalledViewOwner::Live{held,epoch:owner.epoch()},_charge:charge}).collect())
    }.await;
    let mut terminal=Completion::default();terminal.step("atomic attached view reader close",reader.close().await);complete(result,terminal)
}
/// Single-view convenience uses the same atomic reference-only factory.
pub async fn attach_live_view(client:Arc<Surreal<Client>>,attempt:ContentHash,source:&Arc<HeldInstalledView>,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let mut held=attach_live_views(client,attempt,std::slice::from_ref(source),vec![],budget).await?;
    held.pop().ok_or(ModelError::Conflict("attached singleton witness cardinality"))
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

fn check_installation(completion:&InstalledViewCompletion,installation:&Object)->Result<(),ModelError>{
    if installation.get("generation")!=Some(&Value::String(completion.generation.hex()))
        || installation.get("validity_epoch")!=Some(&Value::Number(surrealdb::types::Number::Int(completion.validity_epoch))) {
        return Err(ModelError::Conflict("held installed current validity"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn completion()->InstalledViewCompletion {
        let view=d::completed::CompletedView::new("packages".into(),[ContentHash::of(b"owner")].into(),2).unwrap();
        let mappings=vec![ContributorRealization{view:view.identity,logical:*view.contributions.first().unwrap(),physical:ContentHash::of(b"physical"),incarnation:1}];
        InstalledViewCompletion{generation:ContentHash::of(b"generation"),validity_epoch:1,realization_incarnation:1,version:1,realization:realization_id(&view,&mappings).unwrap(),view,member_codec:MEMBER_CODEC,mappings,closure:d::completed_identity::CompletedFamilyRoots::new(d::recovery_closure::COMPLETED_FAMILIES.map(|family|d::completed_identity::CompletedFamilyHasher::new(family).unwrap().finish())).unwrap(),membership_rows:2,membership_content:ContentHash::of(b"membership"),frozen_revision:1}
    }
    #[test]
    fn receiving_installation_requires_exact_generation_and_validity_even_for_foreign_sessions(){
        let completion=completion();let mut installation=Object::new();
        installation.insert("generation",completion.generation.hex());installation.insert("validity_epoch",Value::Number(surrealdb::types::Number::Int(completion.validity_epoch)));
        check_installation(&completion,&installation).unwrap();
        installation.insert("generation",ContentHash::of(b"foreign installation").hex());
        assert!(check_installation(&completion,&installation).is_err());
        installation.insert("generation",completion.generation.hex());installation.insert("validity_epoch",Value::Number(surrealdb::types::Number::Int(completion.validity_epoch+1)));
        assert!(check_installation(&completion,&installation).is_err());
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
