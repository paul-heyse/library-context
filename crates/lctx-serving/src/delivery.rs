//! Actual-page delivery and greedy whole-bundle packing. Maps are observations, not proof.
use lctx_model::domain::{serving::*, *};
use serde_json::Value;
fn name(value:impl Into<String>)->Result<Name,WireError>{Name::new(value)}
fn unavailable(reason:&str)->Result<Availability,WireError>{Ok(Availability::Unavailable{reason:name(reason)?})}
fn partial(reason:&str)->Result<Availability,WireError>{Ok(Availability::Partial{reason:name(reason)?})}
fn parse<T:serde::de::DeserializeOwned>(value:&Value,key:&str)->Option<T>{serde_json::from_value(value.get(key)?.clone()).ok()}
fn binding(value:&Value,inherited:&DeliveryBinding)->DeliveryBinding {
    DeliveryBinding{member:Nullable(parse(value,"member").or(inherited.member.0)),signature:Nullable(parse(value,"signature").or(inherited.signature.0)),variant:Nullable(parse(value,"variant").or(inherited.variant.0)),analysis:Nullable(parse(value,"analysis").or(inherited.analysis.0)),parameter:Nullable(parse(value,"parameter").or(inherited.parameter.0))}
}
fn expansion(request:&Request,cursor:Option<CursorToken>)->Result<DeliveryExpansion,WireError> {
    let mut json:Value=serde_json::from_str(&request.to_json()?)?;
    json["page"]["expanded"]=Value::Bool(true);
    if let Some(cursor)=cursor{json["page"]["cursor"]=serde_json::to_value(cursor)?;}
    Ok(DeliveryExpansion{tool:request.tool(),arguments:json})
}
struct Scan<'a>{request:&'a Request,map:PacketEvidenceMap,followups:u32,maximum_followups:u32}
impl Scan<'_> {
    fn field(&mut self,path:String,role:DeliveryRole,original:Option<OriginalRange>,binding:DeliveryBinding,qualifications:Vec<Id<assertion::AssertionQualification>>,dependencies:Vec<String>,availability:Availability)->Result<(),WireError> {
        self.map.fields.push(DeliveredEvidence{field:name(path)?,role,original:Nullable(original),binding,qualifications,dependencies:dependencies.into_iter().map(name).collect::<Result<_,_>>()?,availability});Ok(())
    }
    fn omission(&mut self,path:String,availability:Availability,expand:Option<DeliveryExpansion>)->Result<(),WireError>{
        let expand=if self.followups<self.maximum_followups {if expand.is_some(){self.followups+=1;}expand}else{None};
        self.map.omissions.push(DeliveryOmission{field:name(path)?,availability,expand:Nullable(expand)});Ok(())
    }
    fn visit(&mut self,value:&Value,path:&str,inherited:&DeliveryBinding)->Result<(),WireError>{
        if let Some(values)=value.as_array(){for (i,v) in values.iter().enumerate(){self.visit(v,&format!("{path}/{i}"),inherited)?;}return Ok(());}
        let Some(object)=value.as_object() else{return Ok(());};
        let binding=binding(value,inherited);
        let qualification:Option<Id<assertion::AssertionQualification>>=parse(value,"qualification");
        let quals=qualification.into_iter().collect::<Vec<_>>();
        let original:Option<OriginalRange>=parse(value,"original");
        let availability=parse(value,"availability").unwrap_or(Availability::Available{});
        // An excerpt's text is the exact independently captured source; a null text is a reference.
        if let Some(original)=original.clone(){
            if value.get("text").is_some(){
                if value.get("text").is_some_and(Value::is_string){self.field(format!("{path}/text"),DeliveryRole::Primary,Some(original),binding.clone(),quals.clone(),vec![],availability.clone())?;}
                else {let expand=DeliveryExpansion{tool:Tool::GetEvidence,arguments:serde_json::json!({"source":original.source,"page":{"expanded":true}})};self.omission(format!("{path}/text"),availability.clone(),Some(expand))?;}
            }
            if let Some(body)=value.get("body"){
                let start=body.get("start").and_then(Value::as_u64).ok_or_else(||WireError::Invalid("delivered original body start".into()))?;
                let end=body.get("end").and_then(Value::as_u64).ok_or_else(||WireError::Invalid("delivered original body end".into()))?;
                if end<start||end>original.end-original.start{return Err(WireError::Invalid("delivered original body bounds".into()));}
                let mut actual=original;actual.start+=start;actual.end=actual.start+(end-start);
                self.field(format!("{path}/body/bytes"),DeliveryRole::Primary,Some(actual),binding.clone(),quals.clone(),vec![format!("{path}/release"),format!("{path}/interpretation")],Availability::Available{})?;
            }
        }
        if let Some(items)=value.get("items").and_then(Value::as_array){
            let omitted=value.get("omitted").and_then(Value::as_u64).unwrap_or(0);
            let truncated=value.get("truncated").and_then(Value::as_bool).unwrap_or(false);
            if omitted>0||truncated||!matches!(availability,Availability::Available{}){
                let cursor=parse::<CursorToken>(value,"continuation");
                let expand=if cursor.is_some(){Some(expansion(self.request,cursor)?)}else if matches!(availability,Availability::Partial{ref reason} if reason.as_str()=="delivery_envelope_expand"){Some(expansion(self.request,None)?)}else{None};
                self.omission(path.into(),if omitted>0||truncated{partial("page_or_delivery_omission")?}else{availability.clone()},expand)?;
            }
            // Items are scanned below, with bindings derived from each actual enclosing item.
            let _=items;
        }
        let mut dependencies=vec![];
        if object.contains_key("analysis")&&object.contains_key("variant"){dependencies.push(format!("{path}/analysis"));dependencies.push(format!("{path}/variant"));}
        for key in ["name","title","rendered","readable","predicate","constant","python_version","python_platform","search_path","site_package_path","distribution","version"]{
            if let Some(v)=object.get(key){if v.is_null(){continue;}
                let role=if matches!(key,"name"|"title"|"rendered"){DeliveryRole::Primary}else{DeliveryRole::Synthetic};
                let source=if key=="readable"&&matches!(value.get("value").and_then(|v|v.get("kind")).and_then(Value::as_str),Some("expression"|"factory")){value.get("original").and_then(|e|e.get("original")).and_then(|v|serde_json::from_value(v.clone()).ok())}else{None};
                self.field(format!("{path}/{key}"),if source.is_some(){DeliveryRole::Primary}else{role},source,binding.clone(),quals.clone(),dependencies.clone(),availability.clone())?;
            }}
        }
        if let Some(default)=object.get("default") {if default.is_object(){self.field(format!("{path}/default"),DeliveryRole::Reference,None,binding.clone(),quals.clone(),vec![],unavailable("default_reference_is_not_readable_value_or_condition")?)?;}}
        if object.contains_key("source")&&object.contains_key("artifact")&&object.contains_key("digest") {
            if let Ok(original)=serde_json::from_value::<OriginalRange>(value.clone()) {
                self.field(path.into(),DeliveryRole::Reference,Some(original.clone()),binding.clone(),quals.clone(),vec![],unavailable("original_reference_without_source_body")?)?;
            }
        }
        for (key,v) in object {
            if key=="delivery" {continue;}
            let key=key.replace('~',"~0").replace('/',"~1");
            self.visit(v,&format!("{path}/{key}"),&binding)?;
        }
        Ok(())
    }
}
fn evidence_map(request:&Request,response:&Response)->Result<PacketEvidenceMap,WireError>{
    let ranked=matches!(response,Response::SearchOperations(_)|Response::SearchEvidence(_)|Response::SearchCapabilities(_));
    let mut scan=Scan{request,map:PacketEvidenceMap{fields:vec![],omissions:vec![],ranked_continuation:Nullable(ranked.then(RankedContinuationPolicy::default)),packing_policy:name("exact_mcp_cost_whole_optional_bundles_with_transport_reserve")?},followups:0,maximum_followups:request.page().evidence_demand.0.as_ref().map_or(20,|d|d.maximum_followups)};
    scan.field("/content/0/text".into(),DeliveryRole::Synthetic,None,DeliveryBinding::default(),vec![],vec![],Availability::Available{})?;
    scan.visit(&serde_json::from_str::<Value>(&response.to_json()?)?,"/structuredContent",&DeliveryBinding::default())?;
    Ok(scan.map)
}
fn check_context(request:&Request,response:&Response)->Result<(),WireError>{
    let Some(demand)=&request.page().evidence_demand.0 else{return Ok(());};
    match response {
        Response::GetOperation(r)=>if let OperationResolution::Unique{packet}= &r.operation {
            let c=&demand.context;
            if c.release.0.as_ref().is_some_and(|v|v!=&packet.core.release.version){return Err(WireError::Invalid("demand release differs from selected operation".into()));}
            if c.signature.0.is_some()||c.variant.0.is_some()||c.analysis.0.is_some(){
                if !packet.core.signatures.iter().any(|s|c.signature.0.is_none_or(|id|id==s.signature)&&c.variant.0.is_none_or(|id|id==s.variant)&&c.analysis.0.is_none_or(|id|id==s.analysis)){return Err(WireError::Invalid("demand context has no compatible signature".into()));}
            }
        },
        Response::GetEvidence(r)=>{
            if demand.context.release.0.as_ref().is_some_and(|v|v!=&r.evidence.release.version)||demand.context.analysis.0.is_some_and(|id|id!=r.evidence.original.context){return Err(WireError::Invalid("demand differs from original context".into()));}
            if demand.context.signature.0.is_some()||demand.context.variant.0.is_some(){return Err(WireError::Invalid("original bytes do not establish a signature/variant binding".into()));}
        },
        _=>if demand.context.signature.0.is_some()||demand.context.variant.0.is_some(){return Err(WireError::Invalid("signature context requires get_operation".into()));},
    }Ok(())
}
fn protected(request:&Request,section:&str)->bool {
    request.page().evidence_demand.0.as_ref().is_some_and(|d|d.facets.iter().any(|facet|matches!((facet,section),(EvidenceFacet::Scenarios,"scenarios")|(EvidenceFacet::Deployment,"deployment")|(EvidenceFacet::Relationships,"relationships")|(EvidenceFacet::Behavior,"behavior"))))
}
fn omit<T:serde::Serialize>(page:&mut SectionPage<T>,expanded:bool)->Result<(),WireError>{
    page.omitted=page.omitted.checked_add(page.items.len() as u64).ok_or_else(||WireError::Invalid("delivery omission count".into()))?;
    page.items.clear();page.truncated=true;page.continuation=Optional::default();
    page.availability=if expanded{unavailable("optional_bundle_exceeds_expanded_envelope")?}else{partial("delivery_envelope_expand")?};Ok(())
}
fn optional_bundle(request:&Request,response:&mut Response)->Result<bool,WireError>{
    let Response::GetOperation(r)=response else{return Ok(false);};
    let OperationResolution::Unique{packet}= &mut r.operation else{return Ok(false);};
    let mut choices=vec![];
    macro_rules! choose {($($field:ident),*)=>{$(if !protected(request,stringify!($field))&&!packet.$field.items.is_empty(){choices.push((serde_json::to_vec(&packet.$field)?.len(),stringify!($field)));})*};}
    choose!(access_routes,callable_comparison,contextual_typing,incoming_references,scenarios,deployment,relationships,conflicts,briefs,behavior);
    choices.sort();let Some((_,selected))=choices.pop()else{return Ok(false);};
    macro_rules! remove {($($field:ident),*)=>{match selected{$(stringify!($field)=>omit(&mut packet.$field,request.page().expanded)?,)*_=>unreachable!()}};}
    remove!(access_routes,callable_comparison,contextual_typing,incoming_references,scenarios,deployment,relationships,conflicts,briefs,behavior);Ok(true)
}
/// Shared fresh/resumed-page entry point. Core/signatures/interpretation are never pruned.
/// The actual Python transport additionally admits the complete JSON-RPC envelope and metadata.
pub fn finalize(request:&Request,response:&mut Response)->Result<(),WireError>{
    if request.tool()!=response.tool(){return Err(WireError::Invalid("delivery request/response route".into()));}
    check_context(request,response)?;
    // Leave transport framing/IDs room; actual transport admission remains authoritative.
    let limit=ResourceLimits::default().response_bytes(request.page().expanded).saturating_sub(1024) as usize;
    loop {
        *response.delivery_mut()=Optional::default();
        let map=evidence_map(request,response)?;*response.delivery_mut()=Optional::supplied(map);
        if response.mcp_result_len()?<=limit{return Ok(());}
        if !optional_bundle(request,response)?{return Err(WireError::ResourceRefused("indivisible core/closure or demanded bundle exceeds final envelope".into()));}
    }
}
