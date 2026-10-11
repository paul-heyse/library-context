//! Independent exact selected mapping and dependency inspection for native installation.
//! This prepares a writer's audit; it does not grant semantic admission or completion.
use super::ContributorRealization;
use crate::{NativeReader, completed_rows::validate_state_row, control::ExpectedRecord};
use lctx_model::domain::{self as d, ContentHash, ModelError, completed::{CompletedContribution,CompletedView},
    charged::StateCharge, recovery_closure::{RecoveryClosure, RecoveryTraversal}, resources::ResourceBudget};
use std::collections::BTreeMap;
use surrealdb::types::{RecordId,Value,Variables};

/// Exact physical observations remain retained through the final checked installation
/// effect, which must compare these incarnations while touching each contributor.
pub(crate) struct InspectedClosure {
    pub(crate) logical: RecoveryClosure,
    pub(crate) contributors: BTreeMap<ContentHash, ExpectedRecord>,
    pub(crate) mappings: Vec<ContributorRealization>,
    _charge: StateCharge,
}
fn descriptor<T:serde::de::DeserializeOwned>(row:&Value)->Result<T,ModelError>{
    let Some(Value::Bytes(bytes))=row.as_object().and_then(|row|row.get("descriptor")) else {
        return Err(ModelError::Schema("installed audit descriptor"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
fn exact_mappings(closure:&RecoveryClosure,mappings:&[ContributorRealization])->Result<(),ModelError>{
    if mappings.windows(2).any(|pair|pair[0].logical>=pair[1].logical)
        || mappings.iter().any(|mapping|mapping.incarnation<=0)
        || mappings.iter().map(|mapping|mapping.physical).collect::<std::collections::BTreeSet<_>>().len()!=mappings.len()
        || !closure.contributions().keys().copied().eq(mappings.iter().map(|mapping|mapping.logical)) {
        return Err(ModelError::Conflict("installed exact dependency realization inventory"));
    }
    Ok(())
}
/// Reads only explicitly supplied physical contributors. A matching logical copy elsewhere
/// in the installation cannot satisfy a missing, replaced or malformed selected contributor.
pub(crate) async fn inspect_closure(reader:&NativeReader<()>,root:&CompletedView,
    mappings:&[ContributorRealization],budget:&ResourceBudget)->Result<InspectedClosure,ModelError>{
    root.validate()?;
    let mut charge=StateCharge::new(budget,"installed exact closure descriptors");
    charge.grow(mappings.len().saturating_mul(std::mem::size_of::<ContributorRealization>()+128))?;
    if mappings.windows(2).any(|pair|pair[0].logical>=pair[1].logical) {
        return Err(ModelError::Conflict("installed selected mapping order/duplicate"));
    }
    let mut contributors=BTreeMap::new();let mut descriptors=BTreeMap::new();
    for mapping in mappings {
        if mapping.incarnation<=0{return Err(ModelError::Conflict("installed selected incarnation"));}
        let expected=crate::control::inspect_expected_read(reader,
            RecordId::new("compiler_contribution",mapping.physical.hex()),budget).await?
            .ok_or(ModelError::Conflict("installed selected contributor missing"))?;
        if expected.incarnation()!=mapping.incarnation {
            return Err(ModelError::Conflict("installed selected contributor incarnation"));
        }
        charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
        validate_state_row("compiler_contribution",expected.row())?;
        let data:CompletedContribution=descriptor(expected.row())?;
        if data.identity()?!=mapping.logical{return Err(ModelError::Conflict("installed selected contributor logical identity"));}
        descriptors.insert(mapping.logical,data);contributors.insert(mapping.logical,expected);
    }
    let mut traversal=RecoveryTraversal::for_views(vec![root.clone()])?;
    while let Some(requested)=traversal.next_view()? {
        let mut variables=Variables::new();variables.insert("view",RecordId::new("compiler_view",requested.identity.hex()));
        // One bounded declared row. All payload/decode allocations remain covered until
        // the closure owner is consumed by the final effect.
        let _read=budget.reserve("installed exact view read",d::resources::MAX_ROW_BYTES)?;
        let rows:Vec<Value>=reader.query_native("SELECT * FROM $view",variables).await?;
        let [actual]=rows.as_slice() else{return Err(ModelError::Conflict("installed selected view cardinality"));};
        charge.grow(crate::loader::native_bytes(actual).saturating_mul(32).saturating_add(65536))?;
        validate_state_row("compiler_view",actual)?;
        let actual:CompletedView=descriptor(actual)?;
        let mut selected=Vec::new();let mut dependencies=BTreeMap::new();
        for logical in &requested.contributions {
            let contribution=descriptors.get(logical).ok_or(ModelError::Conflict("installed missing dependency mapping"))?;
            charge.grow(serde_json::to_vec(contribution).map_err(ModelError::codec)?.len().saturating_mul(4).saturating_add(1024))?;
            selected.push(contribution.clone());
            let mapping=&mappings[mappings.binary_search_by_key(logical,|mapping|mapping.logical)
                .map_err(|_|ModelError::Conflict("installed selected mapping disappeared"))?];
            crate::completed_state::require_hold(reader,&RecordId::new("compiler_view",requested.identity.hex()),
                &RecordId::new("compiler_contribution",mapping.physical.hex())).await?;
            for input in &contribution.spec.inputs {
                if dependencies.contains_key(&input.view()){continue;}
                let _page=budget.reserve("installed dependency view read",d::resources::MAX_ROW_BYTES)?;
                let mut variables=Variables::new();variables.insert("view",RecordId::new("compiler_view",input.view().hex()));
                let rows:Vec<Value>=reader.query_native("SELECT * FROM $view",variables).await?;
                let [row]=rows.as_slice() else{return Err(ModelError::Conflict("installed dependency view cardinality"));};
                charge.grow(crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536))?;
                validate_state_row("compiler_view",row)?;let dependency:CompletedView=descriptor(row)?;
                crate::completed_state::require_hold(reader,&RecordId::new("compiler_contribution",mapping.physical.hex()),
                    &RecordId::new("compiler_view",input.view().hex())).await?;
                dependencies.insert(input.view(),dependency);
            }
        }
        traversal.include_view(&requested,actual,selected,|identity|dependencies.get(&identity).cloned()
            .ok_or(ModelError::Conflict("installed undeclared dependency view")))?;
    }
    let logical=traversal.finish()?;exact_mappings(&logical,mappings)?;
    Ok(InspectedClosure{logical,contributors,mappings:mappings.to_vec(),_charge:charge})
}

/// Complete output EOF is checked for every selected contribution, including outputs
/// unrelated to the root's relation and explicit empty outputs. Root member preparation
/// below may select only the requested relation after this independent audit succeeds.
pub(crate) async fn audit_memberships(reader:&NativeReader<()>,closure:&InspectedClosure,
    budget:&ResourceBudget)->Result<(),ModelError>{
    use d::{Key,KeySink,completed::OutputContent};
    for mapping in &closure.mappings {
        let descriptor=&closure.logical.contributions()[&mapping.logical];
        let owner=RecordId::new("compiler_contribution",mapping.physical.hex());
        let _outputs=budget.reserve("installed output digest inventory",serde_json::to_vec(&descriptor.outputs)
            .map_err(ModelError::codec)?.len().saturating_mul(8).saturating_add(1024))?;
        let mut outputs=descriptor.outputs.keys().map(|name|(name.clone(),(0u64,KeySink::new("native-contribution-output/v1"))))
            .collect::<BTreeMap<_,_>>();
        let mut after_relation=String::new();let mut after_key=String::new();let mut after_id=Value::None;
        loop {
            let _page=budget.reserve("installed membership read",d::resources::MAX_ROW_BYTES)?;
            let mut variables=Variables::new();variables.insert("owner",owner.clone());
            variables.insert("after_relation",after_relation.clone());variables.insert("after_key",after_key.clone());variables.insert("after_id",after_id.clone());
            let rows:Vec<Value>=reader.query_native("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND (relation>$after_relation OR (relation=$after_relation AND (semantic_key>$after_key OR (semantic_key=$after_key AND id>$after_id)))) ORDER BY relation,semantic_key,id LIMIT 1",variables).await?;
            if rows.is_empty(){break;}
            let [row]=rows.as_slice() else{return Err(ModelError::Conflict("installed membership page bound"));};
            let _decode=budget.reserve("installed membership decode",crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536))?;
            validate_state_row("compiler_membership",row)?;
            let object=row.as_object().ok_or(ModelError::Schema("installed membership object"))?;
            let text=|field:&str|->Result<String,ModelError>{match object.get(field){Some(Value::String(value))=>Ok(value.clone()),_=>Err(ModelError::Schema("installed membership text"))}};
            let relation=text("relation")?;let key=text("semantic_key")?;let content=text("content")?;
            if object.get("contribution")!=Some(&Value::RecordId(owner.clone()))
                || (!after_relation.is_empty() && (relation.as_str(),key.as_str())<=(after_relation.as_str(),after_key.as_str())) {
                return Err(ModelError::Conflict("installed membership owner/order/duplicate"));
            }
            let (count,sink)=outputs.get_mut(&relation).ok_or(ModelError::Conflict("installed undeclared output membership"))?;
            key.encode(sink);content.encode(sink);*count=count.checked_add(1).ok_or(ModelError::Schema("installed membership count"))?;
            let Some(Value::RecordId(node))=object.get("node") else{return Err(ModelError::Schema("installed membership node"));};
            let expected=crate::control::inspect_expected_read(reader,node.clone(),budget).await?
                .ok_or(ModelError::Conflict("installed membership backing missing"))?;
            let backing=expected.row().as_object().ok_or(ModelError::Schema("installed backing object"))?;
            if backing.get("semantic_type")!=object.get("relation") || backing.get("semantic_key")!=object.get("semantic_key")
                || backing.get("content")!=object.get("content") {
                return Err(ModelError::Conflict("installed membership backing metadata"));
            }
            crate::completed_state::require_hold(reader,&owner,node).await?;
            audit_backing(reader,&owner,expected.row(),budget).await?;
            after_relation=relation;after_key=key;after_id=object.get("id").cloned().ok_or(ModelError::Schema("installed member id"))?;
        }
        let actual=outputs.into_iter().map(|(relation,(rows,sink))|(relation,OutputContent{rows,content:sink.finish()})).collect::<BTreeMap<_,_>>();
        if actual!=descriptor.outputs{return Err(ModelError::Conflict("installed complete output EOF/digest"));}
    }
    Ok(())
}
async fn audit_backing(reader:&NativeReader<()>,owner:&RecordId,row:&Value,budget:&ResourceBudget)->Result<(),ModelError>{
    use d::graph::{Entity,Assertion};
    let object=row.as_object().ok_or(ModelError::Schema("installed backing object"))?;
    let Some(Value::RecordId(node))=object.get("id") else{return Err(ModelError::Schema("installed backing id"));};
    let _decode=budget.reserve("installed backing canonical decode",crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536))?;
    let canonical=match object.get("canonical"){Some(Value::Bytes(bytes))=>bytes,_=>return Err(ModelError::Schema("installed canonical backing"))};
    let (expected,endpoints)=match node.table.as_str(){
        "compiler_record"=>{
            if let Some(original)=validate_state_row("compiler_record",row)? {
                audit_original(reader,&original,budget).await?;
            }
            crate::completed_rows::validate_backing_batch(std::slice::from_ref(row),budget)?;
            return Ok(());
        }
        "entity"=>{
            let entity:Entity=serde_json::from_slice(canonical).map_err(ModelError::codec)?;entity.validate()?;
            let view=crate::codec::entity_views(std::slice::from_ref(&entity))?.pop().ok_or(ModelError::Schema("installed entity view"))?;
            (crate::adapter::physical_row(crate::loader::entity_payload_id(&entity)?,entity.content(),
                serde_json::to_vec(&entity).map_err(ModelError::codec)?,Some((entity.kind() as i64,entity.subtype())),view)?,
             crate::loader::entity_endpoint_values(&entity)?)
        }
        "assertion"=>{
            let assertion:Assertion=serde_json::from_slice(canonical).map_err(ModelError::codec)?;assertion.validate()?;
            let view=crate::codec::assertion_views(std::slice::from_ref(&assertion))?.pop().ok_or(ModelError::Schema("installed assertion view"))?;
            (crate::adapter::physical_row(crate::loader::assertion_payload_id(&assertion)?,assertion.content(),
                serde_json::to_vec(&assertion).map_err(ModelError::codec)?,Some((assertion.kind as i64,None)),view)?,
             crate::loader::assertion_endpoint_values(&assertion)?)
        }
        _=>return Err(ModelError::Conflict("installed undeclared backing family")),
    };
    if expected!=*row{return Err(ModelError::Conflict("installed canonical graph backing projection"));}
    for endpoint in endpoints {
        let Some(Value::RecordId(id))=endpoint.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("installed canonical endpoint id"));};
        let actual=crate::control::inspect_expected_read(reader,id.clone(),budget).await?
            .ok_or(ModelError::Conflict("installed canonical endpoint missing before freeze"))?;
        if actual.row()!=&endpoint{return Err(ModelError::Conflict("installed canonical endpoint bytes"));}
        crate::completed_state::require_hold(reader,owner,id).await?;
    }
    Ok(())
}
async fn audit_original(reader:&NativeReader<()>,original:&crate::completed_rows::OriginalBacking,
    budget:&ResourceBudget)->Result<(),ModelError>{
    let source=d::graph::EntityId::of(original.artifact);
    let layout=crate::originals::layout(reader,&RecordId::new("original",source.0.hex())).await?
        .ok_or(ModelError::Conflict("installed original layout"))?;
    crate::originals::validate_layout(reader,&layout).await?;
    if !crate::originals::header(reader,&layout.blob).await?{return Err(ModelError::Conflict("installed original incomplete blob"));}
    let remaining=layout.blob.byte_len.checked_sub(original.start).ok_or(ModelError::Conflict("installed original coverage"))?;
    if original.len as u64!=remaining.min(d::artifact::ARTIFACT_CHUNK_BYTES as u64){return Err(ModelError::Conflict("installed original coverage"));}
    let mut hash=d::ContentHasher::default();let mut offset=0;
    while offset<original.len {
        let length=(original.len-offset).min(256<<10);
        let bytes=reader.original_bytes(source,original.start+offset as u64,length).await?;
        hash.update(&bytes);offset+=length;
    }
    if hash.finish()!=original.digest{return Err(ModelError::Conflict("installed original content digest"));}
    crate::originals::validate_blob(reader,&layout.blob,budget).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::{Relation, analysis::sources::SourceSnapshot, completed::{ContributionSpec,OutputContent,OutputProfile},input::Package,stages::Profile};
    fn contribution(name:&str,inputs:Vec<SourceSnapshot>)->CompletedContribution {
        CompletedContribution{spec:ContributionSpec{captured_binding:None,producer:name.into(),profile:OutputProfile::Specific(Profile::Catalog),
            model:ContentHash::of(b"model"),implementation:ContentHash::of(b"implementation"),configuration:None,inputs,outputs:["packages".into()].into()},
            outcome:0,outputs:[("packages".into(),OutputContent{rows:0,content:ContentHash::of(b"empty membership EOF")})].into()}
    }
    fn closure()->(RecoveryClosure,Vec<ContributorRealization>){
        let dependency=contribution("dependency",vec![]);let dep_id=dependency.identity().unwrap();
        let dependency_view=CompletedView::new("packages".into(),[dep_id].into(),0).unwrap();
        let input=SourceSnapshot::of_completed_view(&Relation::of::<Package>(),dependency.spec.model,&dependency_view).unwrap();
        let root=contribution("root",vec![input]);let root_id=root.identity().unwrap();
        let root_view=CompletedView::new("packages".into(),[root_id].into(),0).unwrap();
        let mut traversal=RecoveryTraversal::for_views(vec![root_view.clone()]).unwrap();
        let requested=traversal.next_view().unwrap().unwrap();
        traversal.include_view(&requested,root_view,vec![root],|_|Ok(dependency_view.clone())).unwrap();
        let requested=traversal.next_view().unwrap().unwrap();
        traversal.include_view(&requested,dependency_view,vec![dependency],|_|unreachable!()).unwrap();
        assert!(traversal.next_view().unwrap().is_none());
        let closure=traversal.finish().unwrap();
        let mappings=closure.contributions().keys().enumerate().map(|(i,logical)|ContributorRealization{
            logical:*logical,physical:ContentHash::of(format!("exact-copy-{i}").as_bytes()),incarnation:1}).collect();
        (closure,mappings)
    }
    #[test]
    fn exact_dependency_mapping_retains_empty_premise_and_refuses_missing_extra_duplicate_or_rebound_copy(){
        let (closure,mappings)=closure();assert_eq!(mappings.len(),2);exact_mappings(&closure,&mappings).unwrap();
        assert!(closure.views().values().all(|view|view.rows==0),"empty dependencies are still closure authority");
        for defect in ["missing","extra","duplicate","physical_alias","incarnation"] {
            let mut changed=mappings.clone();
            match defect {
                "missing"=>{changed.pop();},
                "extra"=>{changed.push(ContributorRealization{logical:ContentHash::of(b"unrequested logical owner"),physical:ContentHash::of(b"unrequested copy"),incarnation:1});changed.sort_by_key(|mapping|mapping.logical);},
                "duplicate"=>{changed.push(changed[0].clone());changed.sort_by_key(|mapping|mapping.logical);},
                "physical_alias"=>{changed[1].physical=changed[0].physical;},
                _=>{changed[0].incarnation=0;},
            }
            assert!(exact_mappings(&closure,&changed).is_err(),"exact closure rejects {defect}");
        }
        let root=closure.views().values().find(|view|view.contributions.iter().any(|logical|!closure.contributions()[logical].spec.inputs.is_empty())).unwrap();
        let before=super::super::realization_id(root,&mappings).unwrap();
        let mut alternate=mappings.clone();alternate[0].physical=ContentHash::of(b"another valid copy");
        exact_mappings(&closure,&alternate).unwrap();
        assert_ne!(before,super::super::realization_id(root,&alternate).unwrap(),"two exact valid copies have different physical namespaces");
    }
}

/// Only alias rows owned by the supplied completed contributor set are closure
/// authority. Global aliases with the same source are intentionally never queried.
pub(crate) async fn audit_aliases(reader:&NativeReader<()>,closure:&InspectedClosure,
    budget:&ResourceBudget)->Result<crate::ordered_rows::PreparedRows,ModelError>{
    use crate::ordered_rows::SortedRows;
    use d::{KeySink,completion::{Completion,complete}};
    let mut seeds=SortedRows::with_budget(budget)?;let mut aliases=SortedRows::with_budget(budget)?;
    for mapping in &closure.mappings {
        let owner=RecordId::new("compiler_contribution",mapping.physical.hex());
        let mut variables=Variables::new();variables.insert("owner",owner.clone());
        let mut members=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],
            vec!["SELECT node AS id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
        let result=async {while let Some(row)=members.next_native().await? {seeds.push(row)?;} Ok::<_,ModelError>(())}.await;
        let mut completion=Completion::default();completion.step("installed seed membership drainage",members.drain_transport().await);complete(result,completion)?;
        let mut variables=Variables::new();variables.insert("owner",owner.clone());
        let mut holds=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],
            vec!["SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner".into()])?)?;
        let result=async {
            while let Some(pointer)=holds.next_native().await? {
                let Some(Value::RecordId(id))=pointer.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("installed held alias pointer"));};
                if id.table.as_str()!="compiler_alias"{continue;}
                let expected=crate::control::inspect_expected_read(reader,id.clone(),budget).await?.ok_or(ModelError::Conflict("installed held alias missing"))?;
                let object=expected.row().as_object().ok_or(ModelError::Schema("installed alias object"))?;
                let Some(Value::RecordId(source))=object.get("source") else{return Err(ModelError::Schema("installed alias source"));};
                let Some(Value::RecordId(target))=object.get("target") else{return Err(ModelError::Schema("installed alias target"));};
                if source.table.as_str()!="entity" || target.table.as_str()!="entity"{return Err(ModelError::Conflict("installed alias entity family"));}
                let mut sink=KeySink::new("native-place-alias/v1");
                sink.part(b"source",&serde_json::to_vec(source).map_err(ModelError::codec)?);
                sink.part(b"target",&serde_json::to_vec(target).map_err(ModelError::codec)?);
                let mut canonical=surrealdb::types::Object::new();canonical.insert("id",RecordId::new("compiler_alias",sink.finish().hex()));canonical.insert("source",source.clone());canonical.insert("target",target.clone());
                if expected.row()!=&Value::Object(canonical){return Err(ModelError::Conflict("installed exact alias envelope"));}
                crate::completed_state::require_hold(reader,id,source).await?;
                let backing=crate::control::inspect_expected_read(reader,target.clone(),budget).await?.ok_or(ModelError::Conflict("installed alias target missing"))?;
                crate::completed_state::require_hold(reader,&owner,target).await?;
                audit_backing(reader,&owner,backing.row(),budget).await?;
                aliases.push(expected.row().clone())?;
            }
            Ok::<_,ModelError>(())
        }.await;
        let mut completion=Completion::default();completion.step("installed alias hold drainage",holds.drain_transport().await);complete(result,completion)?;
    }
    let aliases=aliases.finish()?.into_prepared();
    audit_alias_reachability(seeds.finish()?.into_prepared(),&aliases,budget)?;
    Ok(aliases)
}
fn audit_alias_reachability(mut reached:crate::ordered_rows::PreparedRows,aliases:&crate::ordered_rows::PreparedRows,
    budget:&ResourceBudget)->Result<(),ModelError>{
    use crate::ordered_rows::SortedRows;
    fn pointer(id:RecordId)->Value{let mut row=surrealdb::types::Object::new();row.insert("id",id);Value::Object(row)}
    loop {
        let index=reached.point_index(budget)?;let mut membership=index.cursor(budget)?;
        let mut next=SortedRows::with_budget(budget)?;let mut cursor=reached.cursor_with_budget(budget)?;
        while let Some(row)=cursor.next_row()? {next.push(row)?;}
        let mut aliases=aliases.cursor_with_budget(budget)?;let mut changed=false;let mut unreachable=false;
        while let Some(row)=aliases.next_row()? {
            let object=row.as_object().ok_or(ModelError::Schema("installed alias reachability row"))?;
            let Some(Value::RecordId(source))=object.get("source") else{return Err(ModelError::Schema("installed alias reachability source"));};
            let Some(Value::RecordId(target))=object.get("target") else{return Err(ModelError::Schema("installed alias reachability target"));};
            if membership.contains(source)? {
                if !membership.contains(target)? {next.push(pointer(target.clone()))?;changed=true;}
            } else {unreachable=true;}
        }
        if !changed {
            if unreachable{return Err(ModelError::Conflict("installed held alias outside selected reachable closure"));}
            return Ok(());
        }
        reached=next.finish()?.into_prepared();
    }
}

#[cfg(test)]
mod alias_tests {
    use super::*;
    use crate::ordered_rows::{SortedRows,PreparedRows};
    fn rows(budget:&ResourceBudget,ids:&[&str],edges:&[(&str,&str)])->(PreparedRows,PreparedRows){
        let mut nodes=SortedRows::with_budget(budget).unwrap();
        for id in ids {let mut row=surrealdb::types::Object::new();row.insert("id",RecordId::new("entity",*id));nodes.push(Value::Object(row)).unwrap();}
        let mut aliases=SortedRows::with_budget(budget).unwrap();
        for (ordinal,(source,target)) in edges.iter().enumerate(){
            let mut row=surrealdb::types::Object::new();row.insert("id",RecordId::new("compiler_alias",ordinal as i64));
            row.insert("source",RecordId::new("entity",*source));row.insert("target",RecordId::new("entity",*target));aliases.push(Value::Object(row)).unwrap();
        }
        (nodes.finish().unwrap().into_prepared(),aliases.finish().unwrap().into_prepared())
    }
    #[test]
    fn owned_alias_audit_reaches_full_chains_and_cycles_but_refuses_disconnected_owned_suffix(){
        let budget=ResourceBudget::fixed(16<<20).unwrap();
        for edges in [vec![("seed","b"),("b","c"),("c","b")],vec![("seed","c"),("c","b"),("b","seed")]] {
            let (seeds,aliases)=rows(&budget,&["seed"],&edges);audit_alias_reachability(seeds,&aliases,&budget).unwrap();
        }
        for edges in [vec![("seed","b"),("foreign","suffix")],vec![("foreign","suffix"),("suffix","foreign")]] {
            let (seeds,aliases)=rows(&budget,&["seed"],&edges);
            assert!(audit_alias_reachability(seeds,&aliases,&budget).is_err(),"unreachable owned aliases remain a refusal, including a disjoint cycle");
        }
        assert_eq!(budget.reserved(),0,"all external runs/index reservations finish with their owned audit");
    }
}

/// Compute all six normalized logical family roots from the already independently
/// audited exact closure. This never normalizes a physical row and then treats it as
/// a physical validator input; native codecs always run before logical lowering.
pub(crate) async fn logical_roots(reader:&NativeReader<()>,closure:&InspectedClosure,
    bindings:&[d::completed::CompletedBinding],aliases:&crate::ordered_rows::PreparedRows,
    budget:&ResourceBudget)->Result<d::completed_identity::CompletedFamilyRoots,ModelError>{
    logical_roots_many(reader,std::slice::from_ref(closure),bindings,aliases,budget).await
}

pub(crate) async fn logical_roots_many(reader:&NativeReader<()>,closures:&[InspectedClosure],
    bindings:&[d::completed::CompletedBinding],aliases:&crate::ordered_rows::PreparedRows,
    budget:&ResourceBudget)->Result<d::completed_identity::CompletedFamilyRoots,ModelError>{
    use crate::ordered_rows::SortedRows;
    use d::{completed_identity::CompletedFamilyHasher,recovery_closure::COMPLETED_FAMILIES,completion::{Completion,complete}};
    d::completed::binding_inventory_identity(bindings)?;
    let mut owner_charge=StateCharge::new(budget,"installed logical normalization owner descriptors");
    let mut owners=BTreeMap::new();
    for (closure,mapping) in closures.iter().flat_map(|closure|closure.mappings.iter().map(move |mapping|(closure,mapping))) {
        let descriptor=&closure.logical.contributions()[&mapping.logical];
        owner_charge.grow(serde_json::to_vec(descriptor).map_err(ModelError::codec)?.len().saturating_mul(16).saturating_add(1024))?;
        owners.insert(RecordId::new("compiler_contribution",mapping.physical.hex()),descriptor.clone());
    }
    let mut families=COMPLETED_FAMILIES.map(|_|SortedRows::with_budget_and_row_bytes(budget,d::resources::MAX_ROW_BYTES)).into_iter().collect::<Result<Vec<_>,_>>()?;
    let mut backing_ids=SortedRows::with_budget(budget)?;
    for (closure,mapping) in closures.iter().flat_map(|closure|closure.mappings.iter().map(move |mapping|(closure,mapping))) {
        let expected=&closure.contributors[&mapping.logical];
        families[0].push(crate::completed_logical_rows::normalize("compiler_contribution",expected.row(),&owners)?)?;
        let mut variables=Variables::new();variables.insert("owner",expected.record().clone());
        let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
        let result=async {
            while let Some(row)=rows.next_native().await? {
                validate_state_row("compiler_membership",&row)?;
                families[1].push(crate::completed_logical_rows::normalize("compiler_membership",&row,&owners)?)?;
                let Some(Value::RecordId(node))=row.as_object().and_then(|row|row.get("node")) else{return Err(ModelError::Schema("logical backing member node"));};
                if node.table.as_str()=="compiler_record" {let mut pointer=surrealdb::types::Object::new();pointer.insert("id",node.clone());backing_ids.push(Value::Object(pointer))?;}
            }
            Ok::<_,ModelError>(())
        }.await;
        let mut completion=Completion::default();completion.step("installed logical membership drainage",rows.drain_transport().await);complete(result,completion)?;
    }
    for view in closures.iter().flat_map(|closure|closure.logical.views().values()) {
        let mut variables=Variables::new();variables.insert("view",RecordId::new("compiler_view",view.identity.hex()));
        let _read=budget.reserve("logical declared view read",d::resources::MAX_ROW_BYTES)?;
        let rows:Vec<Value>=reader.query_native("SELECT * FROM $view",variables).await?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("logical declared view cardinality"));};
        let _decode=budget.reserve("logical declared view normalization",crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536))?;
        validate_state_row("compiler_view",row)?;
        if descriptor::<CompletedView>(row)?!=*view {return Err(ModelError::Conflict("logical declared view changed"));}
        families[2].push(crate::completed_logical_rows::normalize("compiler_view",row,&owners)?)?;
    }
    let mut backing_ids=backing_ids.finish()?;
    while let Some(pointer)=backing_ids.next_row()? {
        let Some(Value::RecordId(id))=pointer.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("logical backing identity"));};
        let actual=crate::control::inspect_expected_read(reader,id.clone(),budget).await?.ok_or(ModelError::Conflict("logical backing missing"))?;
        validate_state_row("compiler_record",actual.row())?;
        families[3].push(crate::completed_logical_rows::normalize("compiler_record",actual.row(),&owners)?)?;
    }
    for binding in bindings {
        if !closures.iter().any(|closure|closure.logical.views().get(&binding.view.identity)==Some(&binding.view)) {return Err(ModelError::Conflict("logical binding outside exact dependency closure"));}
        let _decode=budget.reserve("logical binding normalization",serde_json::to_vec(binding).map_err(ModelError::codec)?.len().saturating_mul(16).saturating_add(65536))?;
        let mut row=surrealdb::types::Object::new();row.insert("descriptor",surrealdb::types::Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?));
        families[4].push(crate::completed_logical_rows::normalize("compiler_binding",&Value::Object(row),&owners)?)?;
    }
    let mut aliases=aliases.cursor_with_budget(budget)?;
    while let Some(row)=aliases.next_row()? {families[5].push(crate::completed_logical_rows::normalize("compiler_alias",&row,&owners)?)?;}
    let mut roots=Vec::with_capacity(6);
    for (family,rows) in COMPLETED_FAMILIES.into_iter().zip(families) {
        let mut hasher=CompletedFamilyHasher::new(family)?;let mut rows=rows.finish()?;
        while let Some(row)=rows.next_row()? {hasher.row(&serde_json::to_vec(&row).map_err(ModelError::codec)?)?;}
        roots.push(hasher.finish());
    }
    d::completed_identity::CompletedFamilyRoots::new(roots.try_into().map_err(|_|ModelError::Schema("logical completed family cardinality"))?)
}
