//! Canonical key continuations over exact request, representation and snapshot identities.
use lctx_model::domain::{ContentHash,serving::*};

pub fn binding(request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState,group:&str,section:&str,member:Option<lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>>)->Result<CursorBinding,WireError>{
    let ordering=ContentHash::of(if request.page().expanded {b"serving-key-order/v2:expanded"}else{b"serving-key-order/v2:default"});
    Ok(CursorBinding{snapshot:snapshot.clone(),request:request.canonical_identity()?,policy:ranking::RankingPolicy::default().identity().map_err(|e|WireError::Invalid(e.to_string()))?,wire:wire_identity(),channels:channels.identity(),group:Name::new(group)?,section:Name::new(section)?,member,ordering})
}
/// A response may expose several independently resumable sections. A cursor for one
/// section leaves the other sections on their first page; its full binding is still checked.
pub fn page<T>(mut values:Vec<(ContentHash,T)>,request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState,group:&str,section:&str,member:Option<lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>>,availability:Availability)->Result<SectionPage<T>,WireError>{
    values.sort_by_key(|(key,_)|*key);
    let expected=binding(request,snapshot,channels,group,section,member)?;
    let mut after=None;
    if let Some(token)=&request.page().cursor.0 {
        let bytes=hex::decode(token.as_str()).map_err(|_|WireError::Invalid("cursor encoding".into()))?;
        let cursor:Cursor=serde_json::from_slice(&bytes)?;
        // Inspecting the public group never authorizes reading. The selected group validates
        // every bound identity before applying its position.
        if cursor.binding.group==expected.group && cursor.binding.section==expected.section {
            let cursor=Cursor::decode(token,&expected)?;
            match cursor.after {CursorPosition::Key{key}=>after=Some(key),_=>return Err(WireError::Continuation("key continuation required".into()))}
        }
    }
    values.retain(|(key,_)|after.is_none_or(|after|*key>after));
    let omitted=values.len().saturating_sub(request.page().size as usize) as u64;
    values.truncate(request.page().size as usize);
    let continuation=if omitted>0 {Optional(Some(Cursor{binding:expected,after:CursorPosition::Key{key:values.last().expect("positive requested page size").0}}.encode()?))}else{Optional::default()};
    Ok(SectionPage{availability,items:values.into_iter().map(|(_,value)|value).collect(),continuation,omitted,truncated:omitted>0})
}

pub fn validate(request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState)->Result<(),WireError>{
    let Some(token)=&request.page().cursor.0 else{return Ok(())};
    let bytes=hex::decode(token.as_str()).map_err(|_|WireError::Invalid("cursor encoding".into()))?;
    let cursor:Cursor=serde_json::from_slice(&bytes)?;
    let tool=request.tool().name();
    let group=cursor.binding.group.as_str();let section=cursor.binding.section.as_str();
    let permitted=match request {
        Request::FindOperations(_)=>["supported","unresolved","conflicting"].contains(&group) && section=="results",
        Request::GetOperation(r)=>group==tool && match section {
            "access_routes"=>r.sections.contains(&OperationSection::AccessRoutes),
            "callable_comparison"=>r.sections.contains(&OperationSection::CallableComparison),
            "contextual_typing"=>r.sections.contains(&OperationSection::ContextualTyping),
            "incoming_references"=>r.sections.contains(&OperationSection::IncomingReferences),
            "scenarios"=>r.sections.contains(&OperationSection::Scenarios),
            "deployment"=>r.sections.contains(&OperationSection::Deployment),
            "relationships"=>r.sections.contains(&OperationSection::Relationships),
            "conflicts"=>r.sections.contains(&OperationSection::Conflicts),
            "briefs"=>r.sections.contains(&OperationSection::Briefs),
            "behavior"=>r.sections.contains(&OperationSection::Behavior),_=>false},
        Request::GetEvidence(_)=>group==tool && ["body","flow_inventory","source_characterization","derivation"].contains(&section),
        Request::BrowseLibrary(_)=>group==tool && section=="entries",
        Request::InspectValuePaths(r)=>group==tool && section=="paths" && cursor.binding.member==Some(r.member),
        Request::SearchOperations(_)|Request::SearchEvidence(_)|Request::SearchCapabilities(_)=>group==tool && section=="results",
        Request::CompareOperations(_)|Request::GetCapability(_)=>false,
    };
    if !permitted{return Err(WireError::Continuation("cursor group or section is not part of this request".into()))}
    let expected=binding(request,snapshot,channels,group,section,cursor.binding.member)?;
    Cursor::decode(token,&expected)?;
    Ok(())
}

pub fn target_key(target:lctx_model::domain::serving::ranking::Target)->ContentHash{
    use lctx_model::domain::graph;
    match target {ranking::Target::Member{member}=>graph::EntityId::of(member).0,ranking::Target::Unit{unit}=>graph::EntityId::of(unit).0}
}
pub fn ranked<T>(mut values:Vec<(ranking::RankedHit,ContentHash,T)>,request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState)->Result<(SectionPage<T>,Vec<ranking::RankedHit>),WireError>{
    let expected=binding(request,snapshot,channels,request.tool().name(),"results",None)?;
    if let Some(token)=&request.page().cursor.0 {
        let cursor=Cursor::decode(token,&expected)?;
        let CursorPosition::Ranked{key,score_bits}=cursor.after else{return Err(WireError::Continuation("ranked continuation required".into()))};
        let position=values.iter().position(|(hit,entry,_)|*entry==key && hit.score.to_bits()==score_bits).ok_or_else(||WireError::Continuation("ranked position absent from the pinned candidate result".into()))?;
        values.drain(..=position);
    }
    let omitted=values.len().saturating_sub(request.page().size as usize) as u64;
    values.truncate(request.page().size as usize);
    let continuation=if omitted>0 {let hit=&values.last().expect("positive requested page size").0;Optional(Some(Cursor{binding:expected,after:CursorPosition::Ranked{score_bits:hit.score.to_bits(),key:values.last().expect("positive requested page size").1}}.encode()?))}else{Optional::default()};
    let (ranking,items)=values.into_iter().map(|(ranking,_,item)|(ranking,item)).unzip();
    Ok((SectionPage{availability:Availability::Available{},items,continuation,omitted,truncated:omitted>0},ranking))
}
