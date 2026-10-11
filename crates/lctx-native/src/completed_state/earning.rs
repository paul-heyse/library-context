//! Native physical completion earning. All public inputs are data, independently audited
//! against exact held physical copies; semantic admission is a separate opaque transition.
use super::*;
use crate::{installed_view::{self,ContributorRealization,InstalledViewCompletion},ordered_rows::{SortedRows,PreparedRows}};
use d::{completion::{Completion,complete},resources::ResourceBudget,charged::StateCharge};
use std::sync::Arc;
use surrealdb::{Surreal,engine::remote::grpc::Client,types::Variables};

pub struct ViewSelection {pub view:d::completed::CompletedView,pub mappings:Vec<ContributorRealization>}
pub(crate) async fn revision(reader:&crate::NativeReader<()>,record:&RecordId)->Result<i64,ModelError>{
    let mut variables=Variables::new();variables.insert("record",record.clone());
    let value:Option<i64>=reader.query_native("RETURN (SELECT VALUE revision FROM ONLY type::record('native_predicate',[$record]))",variables).await?;
    Ok(value.unwrap_or(0))
}
fn member(realization:ContentHash,row:&Value)->Result<Value,ModelError>{
    let object=row.as_object().ok_or(ModelError::Schema("completed root member source"))?;
    let Some(Value::String(relation))=object.get("relation") else{return Err(ModelError::Schema("completed root member relation"));};
    let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("completed root member key"));};
    let key=crate::completed_rows::decode_nominal_key(key)?;
    let mut result=Object::new();result.insert("id",installed_view::member_id(realization,relation,&key));result.insert("realization",RecordId::new("compiler_view_realization",realization.hex()));
    for field in ["relation","semantic_key","node","content"] {result.insert(field,object.get(field).cloned().ok_or(ModelError::Schema("completed root member field"))?);}
    Ok(Value::Object(result))
}
pub(crate) async fn root_members(reader:&crate::NativeReader<()>,selection:&ViewSelection,realization:ContentHash,budget:&ResourceBudget)->Result<PreparedRows,ModelError>{
    let mut result=SortedRows::with_budget(budget)?;
    for logical in &selection.view.contributions {
        let mapping=&selection.mappings[selection.mappings.binary_search_by_key(logical,|mapping|mapping.logical).map_err(|_|ModelError::Conflict("root contribution missing exact mapping"))?];
        let mut variables=Variables::new();variables.insert("owner",RecordId::new("compiler_contribution",mapping.physical.hex()));variables.insert("relation",selection.view.relation.clone());
        let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation".into()])?)?;
        let outcome=async {while let Some(row)=rows.next_native().await? {
            let _scratch=budget.reserve("completed root member lowering",crate::loader::native_bytes(&row).saturating_mul(32).saturating_add(65536))?;
            crate::completed_rows::validate_state_row("compiler_membership",&row)?;result.push(member(realization,&row)?)?;
        }Ok::<_,ModelError>(())}.await;
        let mut completion=Completion::default();completion.step("completed root membership read drain",rows.drain_transport().await);complete(outcome,completion)?;
    }
    Ok(result.finish()?.into_prepared())
}
pub(crate) async fn verify_members(reader:&crate::NativeReader<()>,realization:ContentHash,expected:&PreparedRows,rows:u64,budget:&ResourceBudget)->Result<ContentHash,ModelError>{
    let root=RecordId::new("compiler_view_realization",realization.hex());let mut actual=SortedRows::with_budget(budget)?;
    let mut variables=Variables::new();variables.insert("realization",root.clone());
    let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_view_member WITH INDEX realization_nodes WHERE realization=$realization".into()])?)?;
    let outcome=async {let mut count=0u64;while let Some(row)=stream.next_native().await? {
        let _scratch=budget.reserve("completed actual member envelope",crate::loader::native_bytes(&row).saturating_mul(32).saturating_add(65536))?;
        let Some(Value::RecordId(id))=row.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("completed actual member id"));};
        let Some(Value::RecordId(node))=row.as_object().and_then(|row|row.get("node")) else{return Err(ModelError::Schema("completed actual member node"));};
        require_hold(reader,&root,id).await?;require_hold(reader,id,node).await?;
        actual.push(row)?;count=count.checked_add(1).ok_or(ModelError::Schema("completed actual member count"))?;
    }Ok(count)}.await;
    let mut completion=Completion::default();completion.step("completed actual member stream drain",stream.drain_transport().await);let count=complete(outcome,completion)?;
    let mut actual=actual.finish()?;let mut expected=expected.cursor_with_budget(budget)?;let mut sink=KeySink::new("native-view-membership/v2");let mut checked=0u64;
    loop {match(expected.next_row()?,actual.next_row()?){(None,None)=>break,(Some(expected),Some(actual)) if expected==actual=>{
        sink.part(b"member",&serde_json::to_vec(&actual).map_err(ModelError::codec)?);checked=checked.checked_add(1).ok_or(ModelError::Schema("completed member digest count"))?;
    },_=>return Err(ModelError::Conflict("completed exact root membership (including off-page EOF)"))}}
    if count!=rows || checked!=rows{return Err(ModelError::Conflict("completed root actual distinct membership count"));}
    sink.part(b"rows",&checked.to_le_bytes());Ok(sink.finish())
}
/// Freeze must already have quiesced canonical producers. This earns every exact selected
/// view's physical closure and all six complete logical roots; it grants no semantic admission.
pub async fn complete_frozen_content(client:Arc<Surreal<Client>>,fence:&FrozenContentFence,
    bindings:&[d::completed::CompletedBinding],selections:&[ViewSelection],budget:&ResourceBudget)->Result<NativeCompletedState,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        if d::completed::binding_inventory_identity(bindings)?!=fence.bindings()
            || selections.windows(2).any(|pair|pair[0].view.identity>=pair[1].view.identity){return Err(ModelError::Conflict("completed frozen binding/selection inventory"));}
        audit_attempt_bindings(&reader,fence,bindings,budget).await?;
        let mut charge=StateCharge::new(budget,"native complete selected physical audits");
        charge.grow(selections.len().saturating_mul(4096))?;
        let mut closures=Vec::with_capacity(selections.len());let mut references=Vec::with_capacity(selections.len());
        let mut all_aliases=SortedRows::with_budget(budget)?;
        // An existing earned completion reuses its original revision/install references.
        // Independent audit still runs; current revision must never become a new CAS basis.
        let mut variables=Variables::new();variables.insert("attempt",RecordId::new("native_attempt",fence.attempt().hex()));
        let identity:Option<String>=reader.query_native("RETURN (SELECT VALUE completed_identity FROM ONLY $attempt)",variables).await?;
        let previous=if let Some(identity)=identity {
            let hash=ContentHash(hex::decode(identity).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("completed replay digest width"))?);
            let actual=crate::control::inspect_expected_read(&reader,RecordId::new("native_completed_state",hash.hex()),budget).await?.ok_or(ModelError::Conflict("completed replay missing physical witness"))?;
            charge.grow(crate::loader::native_bytes(actual.row()).saturating_mul(32).saturating_add(65536))?;
            let data=FrozenCompletion::from_row(actual.row(),true)?;
            if data.attempt!=fence.attempt() || data.generation!=fence.generation() || data.attempt_epoch!=fence.attempt_epoch() || data.completed_revision!=fence.completed_revision() || data.validity_epoch!=fence.validity_epoch() || data.bindings!=fence.bindings(){return Err(ModelError::Conflict("completed replay exact original fence"));}
            let inventory=inspect_inventory(&reader,&data,budget).await?;Some((data,inventory))
        } else {None};
        for selection in selections {
            let closure=installed_view::audit::inspect_closure(&reader,&selection.view,&selection.mappings,budget).await?;
            installed_view::audit::audit_memberships(&reader,&closure,budget).await?;
            let aliases=installed_view::audit::audit_aliases(&reader,&closure,budget).await?;
            let mut aliases_cursor=aliases.cursor_with_budget(budget)?;while let Some(alias)=aliases_cursor.next_row()?{all_aliases.push(alias)?;}
            let realization=installed_view::realization_id(&selection.view,&selection.mappings)?;
            let root=RecordId::new("compiler_view_realization",realization.hex());
            let members=root_members(&reader,selection,realization,budget).await?;
            let closure_roots=installed_view::audit::logical_roots(&reader,&closure,&[],&aliases,budget).await?;
            if let Some((data,inventory))=&previous {
                let reference=inventory.view(selection.view.identity)?;let installed=inventory.installed(selection.view.identity)?;
                if installed.view!=selection.view || installed.mappings!=selection.mappings || installed.closure!=closure_roots || installed.realization!=realization
                    || verify_members(&reader,realization,&members,selection.view.rows,budget).await?!=installed.membership_content {
                    return Err(ModelError::Conflict("completed replay selected audit differs"));
                }
                verify_installed_ref(&reader,data,reference,budget).await?;references.push(reference.clone());closures.push(closure);continue;
            }
            let existing=crate::control::inspect_expected_read(&reader,root.clone(),budget).await?;
            let declaration=installed_view::realization_declaration(&selection.view,&selection.mappings,false)?;
            let declaration=if let Some(existing)=&existing {
                if existing.row()!=&declaration && existing.row()!=&installed_view::realization_declaration(&selection.view,&selection.mappings,true)?{return Err(ModelError::Conflict("realization declaration changed"));}
                existing.row().clone()
            }else{declaration};
            crate::control::completed::install_witness(&client,fence,Some(declaration),vec![(RecordId::new("native_attempt",fence.attempt().hex()),root.clone()),(root.clone(),RecordId::new("compiler_view",selection.view.identity.hex()))],budget).await?;
            for mapping in &selection.mappings {
                // Contribution predicate writes serialize this audit with every output/
                // alias incidence accepted before contribution completion.
                crate::control::completed::hold_selected_contributor(&client,fence,&root,&closure.contributors[&mapping.logical],budget).await?;
            }
            let mut cursor=members.cursor_with_budget(budget)?;
            while let Some(member)=cursor.next_row()? {
                let Some(Value::RecordId(id))=member.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("prepared member id"));};
                let Some(Value::RecordId(node))=member.as_object().and_then(|row|row.get("node")) else{return Err(ModelError::Schema("prepared member node"));};
                crate::control::completed::install_witness(&client,fence,Some(member.clone()),vec![(root.clone(),id.clone()),(id.clone(),node.clone())],budget).await?;
            }
            let expected=crate::control::inspect_expected_read(&reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("realization disappeared before seal"))?;
            let before=revision(&reader,&root).await?;
            let membership_content=verify_members(&reader,realization,&members,selection.view.rows,budget).await?;
            crate::control::completed::seal_realization(&client,fence,&expected,before,budget).await?;
            charge.grow(selection.mappings.capacity().saturating_mul(std::mem::size_of::<ContributorRealization>()).saturating_add(8192).saturating_add(selection.view.contributions.len().saturating_mul(128)))?;
            let installed=InstalledViewCompletion{generation:fence.generation(),validity_epoch:fence.validity_epoch(),realization_incarnation:expected.incarnation(),version:1,view:selection.view.clone(),realization,member_codec:installed_view::MEMBER_CODEC,mappings:selection.mappings.clone(),closure:closure_roots,membership_rows:selection.view.rows,membership_content,frozen_revision:before.checked_add(1).ok_or(ModelError::Schema("realization seal revision overflow"))?};
            charge.grow(crate::loader::native_bytes(&installed.value()?).saturating_mul(32).saturating_add(65536))?;
            crate::control::completed::install_witness(&client,fence,Some(installed.value()?),vec![(RecordId::new("native_attempt",fence.attempt().hex()),installed.record()?),(installed.record()?,root)],budget).await?;
            references.push(InstalledViewRef{logical_view:selection.view.identity,realization,incarnation:expected.incarnation(),installation_completion:installed.identity()?});closures.push(closure);
        }
        charge.grow(closures.iter().map(|closure|closure.logical.views().len().saturating_mul(128)).sum())?;
        let all_views=closures.iter().flat_map(|closure|closure.logical.views().keys().copied()).collect::<std::collections::BTreeSet<_>>();
        if !all_views.iter().copied().eq(selections.iter().map(|selection|selection.view.identity)){return Err(ModelError::Conflict("complete installed inventory must explicitly include entire dependency closure"));}
        let aliases=all_aliases.finish()?.into_prepared();
        let roots=installed_view::audit::logical_roots_many(&reader,&closures,bindings,&aliases,budget).await?;
        let mut hasher=RealizationHasher::default();for reference in &references{hasher.row(reference)?;}
        let data=FrozenCompletion{version:VERSION,generation:fence.generation(),attempt:fence.attempt(),attempt_epoch:fence.attempt_epoch(),completed_revision:fence.completed_revision(),validity_epoch:fence.validity_epoch(),completed_state:roots.identity()?,roots,bindings:fence.bindings(),realization_inventory:hasher.finish()};
        if let Some((previous,inventory))=previous {
            if previous!=data || inventory.references()!=references.as_slice(){return Err(ModelError::Conflict("completed replay six roots or exact full inventory differ"));}
        } else {
            crate::control::completed::install_witness(&client,fence,Some(data.value(false)?),vec![(RecordId::new("native_attempt",fence.attempt().hex()),data.record()?)],budget).await?;
            for reference in &references {
                crate::control::completed::install_witness(&client,fence,Some(reference.value(data.identity()?)?),vec![(data.record()?,reference.record(data.identity()?)),(data.record()?,RecordId::new("compiler_view_completion",reference.installation_completion.hex()))],budget).await?;
            }
            for binding in bindings {
                if selections.binary_search_by_key(&binding.view.identity,|selection|selection.view.identity).is_err(){return Err(ModelError::Conflict("binding view missing from complete installed inventory"));}
                let id=crate::binding_descriptor::record(binding)?;
                crate::control::completed::install_witness(&client,fence,Some(crate::binding_descriptor::value(binding)?),vec![(data.record()?,id.clone()),(id,RecordId::new("compiler_view",binding.view.identity.hex()))],budget).await?;
            }
            let expected=crate::control::inspect_expected_read(&reader,data.record()?,budget).await?.ok_or(ModelError::Conflict("physical completion disappeared"))?;
            let before=revision(&reader,&data.record()?).await?;
            let _inventory=inspect_inventory(&reader,&data,budget).await?;
            crate::control::completed::finish_physical_completion(&client,&data,&expected,before,budget).await?;
        }
        let actual=crate::control::inspect_expected_read(&reader,data.record()?,budget).await?.ok_or(ModelError::Conflict("physical completed readback missing"))?;
        let inventory=inspect_inventory(&reader,&data,budget).await?;
        let held=crate::control::hold_frozen_physical_expected(&client,&data,actual,budget).await?;
        Ok(NativeCompletedState{completion:data,held,inventory,_charge:charge})
    }.await;
    let mut completion=Completion::default();completion.step("physical completion earning private reader close",reader.close().await);complete(result,completion)
}

async fn audit_attempt_bindings(reader:&crate::NativeReader<()>,fence:&FrozenContentFence,bindings:&[d::completed::CompletedBinding],budget:&ResourceBudget)->Result<(),ModelError>{
    let mut variables=Variables::new();variables.insert("attempt",RecordId::new("native_attempt",fence.attempt().hex()));
    let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_binding WITH INDEX binding_attempt WHERE attempt=$attempt".into()])?)?;
    let outcome=async {
        let mut charge=StateCharge::new(budget,"frozen actual binding inventory audit");let mut actual=std::collections::BTreeMap::new();
        while let Some(row)=stream.next_native().await? {
            charge.grow(crate::loader::native_bytes(&row).saturating_mul(32).saturating_add(2048))?;
            crate::completed_rows::validate_state_row("compiler_binding",&row)?;
            let Some(Value::Bytes(bytes))=row.as_object().and_then(|row|row.get("descriptor")) else{return Err(ModelError::Schema("frozen actual binding descriptor"));};
            let binding:d::completed::CompletedBinding=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            if actual.insert(binding.key(),binding).is_some(){return Err(ModelError::Conflict("frozen duplicate actual binding slot"));}
        }
        if actual.len()!=bindings.len() || bindings.iter().any(|binding|actual.get(&binding.key())!=Some(binding))
            || d::completed::binding_inventory_identity(&actual.into_values().collect::<Vec<_>>())?!=fence.bindings(){
            return Err(ModelError::Conflict("frozen complete exact actual binding EOF"));
        }
        Ok(())
    }.await;
    let mut completion=Completion::default();completion.step("frozen binding inventory stream drain",stream.drain_transport().await);complete(outcome,completion)
}
