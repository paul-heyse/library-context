//! Fixed independent decoding of the current final MCP structured output. No producer maps.
//! Supports captured UTF-8 originals, primary/context windows, operation navigation,
//! declared defaults and independently readable qualification/setup closure.
//! Runtime effective values and unreadable predicates remain unsupported.
use std::collections::BTreeMap;
use serde_json::Value;
use crate::contracts::{Assignment, CandidateStatus};
use crate::observer::{DecodedPacket, PublicEvidence, PublicGroup, PublicQualification};


// Final captures have unique object fields; reject ambiguous manual/corrupted captures.
struct UniqueJson(Value);
impl<'de> serde::Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self,D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("unambiguous JSON final object") }
            fn visit_bool<E:serde::de::Error>(self,v:bool)->Result<Self::Value,E> { Ok(UniqueJson(v.into())) }
            fn visit_i64<E:serde::de::Error>(self,v:i64)->Result<Self::Value,E> { Ok(UniqueJson(v.into())) }
            fn visit_u64<E:serde::de::Error>(self,v:u64)->Result<Self::Value,E> { Ok(UniqueJson(v.into())) }
            fn visit_f64<E:serde::de::Error>(self,v:f64)->Result<Self::Value,E> { Ok(UniqueJson(serde_json::Number::from_f64(v).ok_or_else(||E::custom("nonfinite JSON"))?.into())) }
            fn visit_str<E:serde::de::Error>(self,v:&str)->Result<Self::Value,E> { Ok(UniqueJson(v.into())) }
            fn visit_string<E:serde::de::Error>(self,v:String)->Result<Self::Value,E> { Ok(UniqueJson(v.into())) }
            fn visit_unit<E:serde::de::Error>(self)->Result<Self::Value,E> { Ok(UniqueJson(Value::Null)) }
            fn visit_seq<A:serde::de::SeqAccess<'de>>(self,mut seq:A)->Result<Self::Value,A::Error> {
                let mut values=vec![]; while let Some(UniqueJson(v))=seq.next_element()? { values.push(v); } Ok(UniqueJson(values.into()))
            }
            fn visit_map<A:serde::de::MapAccess<'de>>(self,mut map:A)->Result<Self::Value,A::Error> {
                let mut values=serde_json::Map::new();
                while let Some((key,UniqueJson(v)))=map.next_entry::<String,UniqueJson>()? {
                    if values.insert(key,v).is_some() { return Err(serde::de::Error::custom("duplicate captured public object field")); }
                }
                Ok(UniqueJson(values.into()))
            }
        }
        d.deserialize_any(Visitor)
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> { value.get(key).ok_or_else(|| format!("missing public field {key}")) }
fn text(value: &Value) -> Result<&str, String> { value.as_str().ok_or_else(|| "public field is not text".into()) }
fn integer(value: &Value) -> Result<u64, String> { value.as_u64().ok_or_else(|| "public field is not unsigned integer".into()) }
fn array(value: &Value) -> Result<&Vec<Value>, String> { value.as_array().ok_or_else(|| "public field is not array".into()) }
pub fn identity(value: &Value, length: usize) -> Result<String, String> {
    let bytes = array(value)?;
    if bytes.len() != length { return Err("public nominal identity length".into()); }
    bytes.iter().map(|value| integer(value).and_then(|n| u8::try_from(n).map(|b| format!("{b:02x}")).map_err(|_| "public identity byte range".into()))).collect()
}
fn observed(anchor: String, role: &str, text: String) -> PublicEvidence {
    PublicEvidence { anchor, provenance: Assignment::new(), role: role.into(), text, qualifications: vec![], candidate_status: CandidateStatus::Supported }
}
fn literal(value: &Value) -> Result<Option<String>, String> {
    match text(field(value, "kind")?)? {
        "none" => Ok(Some("None".into())),
        "bool" => Ok(Some(if field(value, "value")?.as_bool().ok_or("public Boolean literal")? { "True".into() } else { "False".into() })),
        "integer" => Ok(Some(text(field(value, "decimal")?)?.into())),
        "string" => Ok(Some(serde_json::to_string(text(field(value, "value")?)?).map_err(|e|e.to_string())?)),
        // Raw IEEE bits and byte arrays are not fabricated readable Python literals.
        "float" | "bytes" => Ok(None),
        _ => Err("unsupported public literal kind".into()),
    }
}
fn displayed_literal(value:&Value)->Result<Option<String>,String>{
    let kind=text(field(value,"kind")?)?;
    Ok(match kind {
        "string"=>Some(serde_json::to_string(text(field(value,"value")?)?).map_err(|e|e.to_string())?),
        "bytes"=>{let bytes=array(field(value,"value")?)?.iter().map(|v|integer(v).and_then(|n|u8::try_from(n).map_err(|_|"public byte literal".into()))).collect::<Result<Vec<_>,String>>()?;Some(format!("bytes(hex={})",bytes.iter().map(|b|format!("{b:02x}")).collect::<String>()))},
        "float"=>{let bits=field(value,"bits")?.as_i64().ok_or("public float bits")? as u64;let v=f64::from_bits(bits);v.is_finite().then(||format!("{v} (IEEE754 bits={bits:016x})"))},
        _=>literal(value)?,
    })
}
fn readable_qualification(value:&Value,analysis:&str,release:&str)->Result<Option<PublicQualification>,String>{
    let id=identity(field(value,"qualification")?,16)?;
    if identity(field(value,"analysis")?,16)?!=analysis{return Err("public qualification/context mismatch".into());}
    if field(value,"truncated")?.as_bool()!=Some(false){return Ok(None);}
    let terms=array(field(value,"terms")?)?;
    let meaning=if let Some(constant)=field(value,"constant")?.as_bool(){
        if (constant&&!(terms.len()==1&&array(&terms[0])?.is_empty()))||(!constant&&!terms.is_empty()){return Err("public constant/condition disagreement".into());}
        constant.to_string()
    }else{
        if terms.is_empty(){return Ok(None);}
        let mut meanings=vec![];
        for term in terms {
            let atoms=array(term)?;if atoms.is_empty(){return Err("unlabelled public constant term".into());}
            let mut conjunction=vec![];
            for atom in atoms {
                if identity(field(atom,"analysis")?,16)?!=analysis{return Err("public condition atom/context mismatch".into());}
                let Some(predicate)=field(atom,"predicate")?.as_str().filter(|v|!v.is_empty())else{return Ok(None);};
                let Some(evaluation)=field(atom,"evaluation")?.as_object()else{return Ok(None);};
                let Some(source)=evaluation.get("text").and_then(Value::as_str).filter(|v|!v.is_empty())else{return Ok(None);};
                let original=evaluation.get("original").ok_or("public atom original missing")?;
                if identity(field(original,"context")?,16)?!=analysis||identity(field(original,"release")?,16)?!=release{return Err("public atom original/context or release mismatch".into());}
                let start=integer(field(original,"start")?)?;let end=integer(field(original,"end")?)?;
                if text(field(original,"encoding")?)?!="raw_bytes"||end.checked_sub(start)!=Some(source.len() as u64){return Err("public atom original byte bounds/encoding".into());}
                let positive=field(atom,"value")?.as_bool().ok_or("public condition polarity")?;
                conjunction.push(format!("{}({source} {predicate})",if positive{""}else{"not "}));
            }
            meanings.push(conjunction.join(" and "));
        }
        meanings.join(" or ")
    };
    let attributes=["modality","approximation","claim_basis","availability"].into_iter().filter_map(|key|value.get(key).map(|v|Ok((key.into(),serde_json::to_string(v).map_err(|e|e.to_string())?)))).collect::<Result<Assignment,String>>()?;
    Ok(Some(PublicQualification{id,text:meaning,attributes}))
}
fn qualified_status(qualification:&PublicQualification)->CandidateStatus {
    if qualification.attributes.get("modality").map(String::as_str)==Some("\"definite\"") && qualification.attributes.get("approximation").map(String::as_str)==Some("\"exact\"") && qualification.attributes.get("availability").and_then(|v|serde_json::from_str::<Value>(v).ok()).is_some_and(|v|v["status"]=="available") {CandidateStatus::Supported}else{CandidateStatus::Unknown}
}
fn interpretation(core:&Value,common:&Assignment,groups:&mut Vec<PublicGroup>)->Result<(),String>{
    let Some(closure)=core.get("interpretation")else{return Ok(());};
    let mut contexts=BTreeMap::new();
    for context in array(field(closure,"contexts")?)? {
        let analysis=identity(field(context,"analysis")?,16)?;
        if contexts.insert(analysis.clone(),context).is_some(){return Err("duplicate public analysis context".into());}
        let mut assignment=common.clone();assignment.insert("analysis_identity".into(),analysis.clone());
        let mut evidence=vec![];
        for key in ["python_version","python_platform"] {evidence.push(observed(analysis.clone(),key,text(field(context,key)?)?.into()));}
        for key in ["search_path","site_package_path"] {for path in array(field(context,key)?)? {evidence.push(observed(analysis.clone(),key,text(path)?.into()));}}
        groups.push(PublicGroup{context:assignment,evidence});
    }
    let mut qualifications=BTreeMap::new();
    for q in array(field(closure,"qualifications")?)? {
        let analysis=identity(field(q,"analysis")?,16)?;
        if !contexts.contains_key(&analysis){return Err("public qualification missing captured context".into());}
        let id=identity(field(q,"qualification")?,16)?;
        if qualifications.insert((id,analysis.clone()),readable_qualification(q,&analysis,common.get("release_identity").ok_or("missing enclosing readable release")?)?).is_some(){return Err("duplicate public qualification definition".into());}
    }
    for default in array(field(closure,"defaults")?)? {
        let analysis=identity(field(default,"analysis")?,16)?;
        if !contexts.contains_key(&analysis){return Err("public default missing captured context".into());}
        let mut assignment=common.clone();assignment.insert("analysis_identity".into(),analysis.clone());
        let field_id=field(default,"field")?;
        let signature=field(default,"signature")?;let variant=field(default,"variant")?;let parameter=field(default,"parameter")?;
        if !field_id.is_null() {
            if !signature.is_null()||!variant.is_null()||!parameter.is_null(){return Err("field default invents callable binding".into());}
            assignment.insert("field_identity".into(),identity(field_id,16)?);
        }else{
            let sid=identity(signature,16)?;let vid=identity(variant,16)?;let pid=identity(parameter,16)?;
            if !array(field(core,"signatures")?)?.iter().any(|s|identity(&s["signature"],16).ok().as_ref()==Some(&sid)&&identity(&s["variant"],16).ok().as_ref()==Some(&vid)&&identity(&s["analysis"],16).ok().as_ref()==Some(&analysis)&&s.get("parameters").and_then(Value::as_array).is_some_and(|ps|ps.iter().any(|p|identity(&p["parameter"],16).ok().as_ref()==Some(&pid)))) {return Err("public default has foreign callable container".into());}
            assignment.insert("signature_identity".into(),sid);assignment.insert("variant_identity".into(),vid);assignment.insert("parameter_identity".into(),pid);
        }
        let anchor=identity(field(default,"option")?,16)?;
        let mut evidence=vec![];
        if let Some(name)=field(default,"subject_name")?.as_str(){evidence.push(observed(anchor.clone(),if field_id.is_null(){"parameter_name"}else{"field_name"},name.into()));}
        let kind=text(field(field(default,"value")?,"kind")?)?;
        let readable=field(default,"readable")?.as_str().filter(|s|!s.is_empty());
        if let Some(readable)=readable {
            let role=match kind {"literal"=>"declared_literal_default","expression"=>"declared_expression_default","factory"=>"declared_factory_default","absent"=>"declared_absent_default",_=>""};
            if !role.is_empty() {
                if kind=="literal" {
                    let id=identity(field(field(default,"value")?,"literal")?,16)?;
                    let values=array(field(core,"literal_values")?)?;
                    let literals=values.iter().filter(|v|identity(&v["literal"],16).ok().as_ref()==Some(&id)).collect::<Vec<_>>();
                    if literals.len()!=1||displayed_literal(field(literals[0],"value")?)?.as_deref()!=Some(readable){return Err("public declared literal/readable disagreement".into());}
                }
                if kind=="absent"&&readable!="no declared default" {return Err("public absent/readable disagreement".into());}
                if matches!(kind,"expression"|"factory") {
                    let excerpt=field(default,"original")?;let original=field(excerpt,"original")?;
                    if field(excerpt,"text")?.as_str()!=Some(readable)||identity(field(original,"source")?.get("occurrence").ok_or("expression original is not occurrence")?,16)?!=identity(field(field(default,"value")?,"expression")?,16)?||identity(field(original,"release")?,16)?!=*common.get("release_identity").ok_or("missing enclosing release")?||identity(field(original,"context")?,16)?!=analysis||text(field(original,"encoding")?)?!="raw_bytes"||integer(field(original,"end")?)?.checked_sub(integer(field(original,"start")?)?)!=Some(readable.len() as u64){return Err("public expression is not exact captured original".into());}
                }
                let mut meaning=observed(anchor,role,readable.into());
                if !field(default,"qualification")?.is_null(){let q=identity(field(default,"qualification")?,16)?;meaning.candidate_status=CandidateStatus::Unknown;if let Some(Some(q))=qualifications.get(&(q,analysis.clone())){meaning.candidate_status=qualified_status(q);meaning.qualifications.push(q.clone());}}
                evidence.push(meaning);
            }
        }
        groups.push(PublicGroup{context:assignment,evidence});
    }
    Ok(())
}
fn release_context(release:&Value)->Result<Assignment,String>{
    Ok(Assignment::from([("release".into(),text(field(release,"version")?)?.into()),("distribution".into(),text(field(release,"distribution")?)?.into()),("release_identity".into(),identity(field(release,"release")?,16)?),("input_identity".into(),identity(field(release,"input")?,16)?)]))
}
fn search_evidence(structured:&Value,groups:&mut Vec<PublicGroup>,references:&mut Vec<String>)->Result<(),String>{
    let items=array(field(field(structured,"results")?,"items")?)?;
    let rankings=array(field(structured,"ranking")?)?;
    if rankings.len()!=items.len(){return Err("public evidence rows/ranking mismatch".into());}
    for (hit,ranking) in items.iter().zip(rankings) {
        let unit=identity(field(hit,"unit")?,16)?;
        if text(field(field(ranking,"target")?,"kind")?)?!="unit"||identity(field(field(ranking,"target")?,"unit")?,16)?!=unit{return Err("public ranked evidence target mismatch".into());}
        let analysis=identity(field(ranking,"context")?,16)?;
        let release=field(hit,"release")?;
        if !array(field(structured,"domains")?)?.iter().any(|d|d.get("captures").and_then(Value::as_array).is_some_and(|cs|cs.iter().any(|c|c.get("release")==Some(release)))){return Err("public evidence release lacks capture".into());}
        let common=release_context(release)?;
        if !array(field(field(hit,"interpretation")?,"contexts")?)?.iter().any(|c|identity(&c["analysis"],16).ok().as_ref()==Some(&analysis)){return Err("public window lacks actual captured analysis context".into());}
        interpretation(hit,&common,groups)?;
        let mut quals=BTreeMap::new();
        for q in array(field(field(hit,"interpretation")?,"qualifications")?)? {
            let qanalysis=identity(field(q,"analysis")?,16)?;
            if qanalysis!=analysis{return Err("public window qualification has foreign analysis".into());}
            let id=identity(field(q,"qualification")?,16)?;
            quals.insert(id,readable_qualification(q,&analysis,&common["release_identity"])?);
        }
        let witnesses=array(field(ranking,"witnesses")?)?;
        for window in array(field(hit,"delivered_windows")?)? {
            let wid=identity(field(window,"window")?,16)?;let part=identity(field(window,"part")?,16)?;
            if identity(field(window,"analysis")?,16)?!=analysis{return Err("public delivered window analysis mismatch".into());}
            let purpose=integer(field(window,"purpose")?)?;
            if purpose>1{return Err("unsupported public window purpose".into());}
            if !witnesses.iter().any(|w|w.get("occurrence").is_some_and(|o|identity(&o["window"],16).ok().as_ref()==Some(&wid)&&identity(&o["context"],16).ok().as_ref()==Some(&analysis)&&(purpose==1||identity(&o["part"],16).ok().as_ref()==Some(&part)))){return Err("public window has no actual ranked witness".into());}
            if purpose==1&&["binding","subject","basis","member"].iter().any(|key|window.get(key).is_some_and(|v|!v.is_null())){return Err("context/setup part cannot nominate primary binding".into());}
            let content=text(field(window,"text")?)?;
            let mut context=common.clone();context.insert("analysis_identity".into(),analysis.clone());
            if let Some(member)=window.get("member").filter(|v|!v.is_null()){if purpose!=0{return Err("context member nomination".into());}context.insert("member_identity".into(),identity(member,16)?);}
            let mut meanings=vec![];let maps=array(field(window,"source_maps")?)?;
            let mut ranges=vec![];
            for map in maps {
                let start=integer(field(map,"start")?)?;let end=integer(field(map,"end")?)?;
                if end<=start{return Err("public window source map bounds".into());}
                let fragment=content.get(start as usize..end as usize).ok_or("public window source map UTF8 bounds")?;
                if ranges.iter().any(|(a,z)|start<*z&&end>*a){return Err("overlapping public window source map".into());}ranges.push((start,end));
                let original=field(map,"original")?;
                if original.is_null(){let mut synthetic=observed(wid.clone(),if purpose==0{"synthetic_primary_window"}else{"synthetic_context_window"},fragment.into());synthetic.candidate_status=CandidateStatus::Unknown;meanings.push(synthetic);continue;}
                if identity(field(original,"context")?,16)?!=analysis||identity(field(original,"release")?,16)?!=common["release_identity"]||text(field(original,"encoding")?)?!="raw_bytes"||integer(field(original,"end")?)?.checked_sub(integer(field(original,"start")?)?)!=Some(end-start){return Err("public window/original association mismatch".into());}
                let anchor=serde_json::to_string(field(original,"source")?).map_err(|e|e.to_string())?;
                let mut meaning=observed(anchor,if purpose==0{"primary_window_source"}else{"context_window_source"},fragment.into());
                if !field(window,"qualification")?.is_null(){let q=identity(field(window,"qualification")?,16)?;meaning.candidate_status=CandidateStatus::Unknown;if let Some(Some(readable))=quals.get(&q){meaning.candidate_status=qualified_status(readable);meaning.qualifications.push(readable.clone());}}
                meaning.provenance=Assignment::from([("window_identity".into(),wid.clone()),("part_identity".into(),part.clone()),("artifact_identity".into(),identity(field(original,"artifact")?,16)?),("source_start".into(),integer(field(original,"start")?)?.to_string()),("source_end".into(),integer(field(original,"end")?)?.to_string())]);
                groups.push(PublicGroup{context:context.clone(),evidence:vec![meaning]});
                references.push(serde_json::json!({"tool":"get_evidence","arguments":{"source":field(original,"source")?,"page":{"expanded":true,"evidence_demand":{"facets":["originals","conditions","setup"],"context":{"analysis":field(window,"analysis")?,"release":field(release,"version")?},"maximum_followups":0}}}}).to_string());
            }
            if maps.is_empty(){let mut synthetic=observed(wid,if purpose==0{"synthetic_primary_window"}else{"synthetic_context_window"},content.into());synthetic.candidate_status=CandidateStatus::Unknown;meanings.push(synthetic);}
            groups.push(PublicGroup{context,evidence:meanings});
        }
    }Ok(())
}
fn search_operations(structured:&Value,groups:&mut Vec<PublicGroup>,references:&mut Vec<String>)->Result<(),String>{
    for candidate in array(field(field(structured,"results")?,"items")?)? {
        let member=identity(field(candidate,"member")?,16)?;let analysis=identity(field(candidate,"analysis")?,16)?;
        for release in array(field(candidate,"releases")?)? {
            let mut context=release_context(release)?;context.insert("member_identity".into(),member.clone());context.insert("analysis_identity".into(),analysis.clone());
            groups.push(PublicGroup{context,evidence:vec![observed(member.clone(),"operation_navigation",text(field(candidate,"name")?)?.into())]});
            let domains=array(field(structured,"domains")?)?.iter().filter(|d|d.get("captures").and_then(Value::as_array).is_some_and(|cs|cs.iter().any(|c|c.get("release")==Some(release)))).collect::<Vec<_>>();
            if domains.is_empty(){return Err("public operation navigation lacks captured domain".into());}
            for domain in domains {
            references.push(serde_json::json!({"tool":"get_operation","arguments":{"library":field(domain,"name")?,"operation":{"kind":"member","member":field(candidate,"member")?},"page":{"evidence_demand":{"facets":["declaration","defaults","conditions","setup"],"context":{"analysis":field(candidate,"analysis")?,"release":field(release,"version")?},"maximum_followups":0}}}}).to_string());
            }
        }
    }Ok(())
}
fn continuation_expanded(cursor:&str)->Result<bool,String>{
    if cursor.len()%2!=0||!cursor.as_bytes().iter().all(u8::is_ascii_hexdigit){return Err("public cursor hex bounds".into());}
    let bytes=(0..cursor.len()).step_by(2).map(|i|u8::from_str_radix(&cursor[i..i+2],16).map_err(|_|"public cursor hex encoding".to_owned())).collect::<Result<Vec<_>,_>>()?;
    let UniqueJson(value)=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    let ordering=identity(field(field(&value,"binding")?,"ordering")?,32)?;
    if ordering==blake3::hash(b"serving-key-order/v2:default").to_hex().to_string(){Ok(false)}else if ordering==blake3::hash(b"serving-key-order/v2:expanded").to_hex().to_string(){Ok(true)}else{Err("unsupported public continuation ordering".into())}
}
pub fn decode(bytes: &str, realization: &str) -> Result<DecodedPacket, String> {
    let UniqueJson(root) = serde_json::from_str(bytes).map_err(|e| e.to_string())?;
    let result = if root.get("jsonrpc").is_some() { field(&root, "result")? } else { &root };
    if field(result, "isError")?.as_bool() != Some(false) { return Err("MCP operation returned error/refusal".into()); }
    crate::delivery_conformance::validate(result)?;
    let content = array(field(result, "content")?)?;
    if content.iter().any(|block| block.get("type").and_then(Value::as_str) != Some("text")) { return Err("unsupported MCP content kind".into()); }
    let structured = field(result, "structuredContent")?;
    let snapshot = field(structured, "snapshot")?;
    if identity(field(snapshot, "realization")?, 32)? != realization { return Err("captured packet realization mismatch".into()); }
    let semantic_snapshot = Some(identity(field(snapshot, "semantic")?, 32)?);
    let database = field(snapshot, "database")?;
    if text(field(database,"namespace")?)?.is_empty() || text(field(database,"database")?)?.is_empty() { return Err("empty public database identity".into()); }
    let database_fields = BTreeMap::from([("database", text(field(database,"database")?)?), ("namespace", text(field(database,"namespace")?)?)]);
    let database_identity = Some(serde_json::to_string(&database_fields).map_err(|e| e.to_string())?);
    let mut groups = Vec::new(); let mut references = Vec::new();
    let tool;
    if let Some(evidence) = structured.get("evidence") {
        tool = Some("get_evidence".into());
        if !content.iter().any(|block| block.get("text").and_then(Value::as_str) == Some("get_evidence: snapshot-bound result")) { return Err("MCP tool/result shape mismatch".into()); }
        let original = field(evidence, "original")?;
        let body = field(evidence, "body")?;
        let start = integer(field(body, "start")?)?; let end = integer(field(body, "end")?)?;
        let body_bytes = array(field(body, "bytes")?)?.iter().map(|v| integer(v).and_then(|n| u8::try_from(n).map_err(|_| "public source byte range".into()))).collect::<Result<Vec<_>, String>>()?;
        if end.checked_sub(start) != Some(body_bytes.len() as u64) { return Err("public body byte bounds mismatch".into()); }
        let encoding=text(field(original,"encoding")?)?;
        if !matches!(encoding,"utf-8"|"raw_bytes"|"native_literal_utf8_slice"){return Err("unsupported source encoding".into());}
        let source = String::from_utf8(body_bytes).map_err(|_| "source bytes are not UTF-8")?;
        let artifact = identity(field(original, "artifact")?, 16)?;
        let original_start = integer(field(original, "start")?)?;
        let original_end = integer(field(original, "end")?)?;
        if original_end < original_start || (encoding!="native_literal_utf8_slice"&&end > original_end - original_start) { return Err("public source range mismatch".into()); }
        let anchor = serde_json::to_string(field(original, "source")?).map_err(|e| e.to_string())?;
        let context = Assignment::from([
            ("release_identity".into(), identity(field(original, "release")?,16)?),
            ("analysis_identity".into(), identity(field(original, "context")?,16)?),
        ]);
        let mut context=context;
        if let Some(release)=evidence.get("release") {
            if identity(field(release,"release")?,16)?!=context["release_identity"]{return Err("source readable release binding disagreement".into());}
            context.insert("release".into(),text(field(release,"version")?)?.into());context.insert("distribution".into(),text(field(release,"distribution")?)?.into());
        }
        let mut meaning=observed(anchor.clone(),if encoding=="native_literal_utf8_slice"{"interpreted_prose"}else{"original_source"},source);
        meaning.provenance=Assignment::from([("artifact_identity".into(),artifact),("source_start".into(),original_start.to_string()),("source_end".into(),original_end.to_string())]);
        groups.push(PublicGroup { context:context.clone(), evidence: vec![meaning] });
        interpretation(evidence,&context,&mut groups)?;
        if let Some(cursor) = body.get("continuation") {let cursor=text(cursor)?;references.push(serde_json::json!({"tool":"get_evidence","arguments":{"source":field(original,"source")?,"page":{"cursor":cursor,"expanded":continuation_expanded(cursor)?}}}).to_string());}
    } else if let Some(operation) = structured.get("operation") {
        tool = Some("get_operation".into());
        if !content.iter().any(|block| block.get("text").and_then(Value::as_str) == Some("get_operation: snapshot-bound result")) { return Err("MCP tool/result shape mismatch".into()); }
        if text(field(operation, "resolution")?)? != "unique" { return Ok(DecodedPacket { tool, semantic_snapshot, database_identity, groups, references }); }
        let packet = field(operation, "packet")?;
        let core = field(packet, "core")?;
        let release = field(core, "release")?;
        let member = identity(field(core, "member")?, 16)?;
        let common = Assignment::from([("release".into(), text(field(release,"version")?)?.into()), ("distribution".into(), text(field(release,"distribution")?)?.into()), ("member_identity".into(), member.clone()), ("release_identity".into(), identity(field(release,"release")?,16)?), ("input_identity".into(), identity(field(release,"input")?,16)?)]);
        groups.push(PublicGroup { context: common.clone(), evidence: vec![observed(member, "operation_name", text(field(core,"name")?)?.into())] });
        let mut literals = BTreeMap::new();
        for entry in array(field(core, "literal_values")?)? {
            let id = identity(field(entry,"literal")?,16)?;
            if literals.insert(id, field(entry,"value")?).is_some() { return Err("duplicate public literal definition".into()); }
        }
        for signature in array(field(core, "signatures")?)? {
            let mut context = common.clone();
            context.insert("signature_identity".into(), identity(field(signature,"signature")?,16)?);
            context.insert("variant_identity".into(), identity(field(signature,"variant")?,16)?);
            context.insert("analysis_identity".into(), identity(field(signature,"analysis")?,16)?);
            for parameter in array(field(signature,"parameters")?)? {
                let id = identity(field(parameter,"parameter")?,16)?;
                let mut meanings = vec![];
                let mut parameter_context = context.clone();
                parameter_context.insert("parameter_identity".into(), id.clone());
                if let Some(name) = field(parameter,"name")?.as_str() { meanings.push(observed(id.clone(),"parameter_name", name.into())); }
                let default = field(parameter,"default")?;
                if core.get("interpretation").is_none() && default.get("kind").and_then(Value::as_str) == Some("literal") {
                    let literal_id = identity(field(default,"literal")?,16)?;
                    if let Some(value) = literals.get(&literal_id).and_then(|v| literal(v).transpose()).transpose()? { meanings.push(observed(id,"declared_literal_default",value)); }
                }
                groups.push(PublicGroup { context: parameter_context, evidence: meanings });
            }
        }
        interpretation(core,&common,&mut groups)?;
        // Only actual public OriginalRange fields can nominate get_evidence follow-ups.
        for section in ["scenarios", "deployment"] {
            for item in packet.get(section).and_then(|s| s.get("items")).and_then(Value::as_array).into_iter().flatten() {
                for original in item.get("spans").or_else(|| item.get("originals")).and_then(Value::as_array).into_iter().flatten() {
                    if let Some(source) = original.get("source") { references.push(serde_json::json!({"tool":"get_evidence","arguments":{"source":source,"page":{"expanded":true}}}).to_string()); }
                }
            }
        }
    } else if content.iter().any(|b|b.get("text").and_then(Value::as_str)==Some("search_evidence: snapshot-bound result")) {
        tool=Some("search_evidence".into());search_evidence(structured,&mut groups,&mut references)?;
    } else if content.iter().any(|b|b.get("text").and_then(Value::as_str)==Some("search_operations: snapshot-bound result")) {
        tool=Some("search_operations".into());search_operations(structured,&mut groups,&mut references)?;
    } else { return Err("unsupported current MCP response class".into()); }
    if matches!(tool.as_deref(),Some("search_evidence"|"search_operations")) {
        if let Some(cursor)=structured.get("results").and_then(|r|r.get("continuation")) {
            let cursor=text(cursor)?;references.push(serde_json::json!({"tool":tool.as_deref(),"arguments":{"page":{"cursor":cursor,"expanded":continuation_expanded(cursor)?}}}).to_string());
        }
    }
    if groups.len() > 256 || references.len() > 256 || groups.iter().map(|g| g.evidence.len()).sum::<usize>() > 256 { return Err("current public packet finite bound exceeded".into()); }
    Ok(DecodedPacket { tool, semantic_snapshot, database_identity, groups, references })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operation() -> Value {
        serde_json::json!({"content":[{"type":"text","text":"get_operation: snapshot-bound result"}],"isError":false,
            "structuredContent":{"snapshot":{"semantic":vec![5;32],"realization":vec![6;32],"database":{"namespace":"control","database":"renderer"}},
            "operation":{"resolution":"unique","packet":{"core":{"member":vec![1;16],"name":"connect",
                "release":{"distribution":"mini","version":"1","input":vec![10;16],"release":vec![11;16]},"literal_values":[{"literal":vec![9;16],"value":{"kind":"integer","decimal":"10"}}],
                "signatures":[{"signature":vec![12;16],"variant":vec![2;16],"analysis":vec![3;16],"parameters":[
                    {"parameter":vec![4;16],"name":"timeout","default":{"kind":"expression","expression":vec![8;16]}},
                    {"parameter":vec![7;16],"name":"other","default":{"kind":"literal","literal":vec![9;16]}}]},
                    {"signature":vec![13;16],"variant":vec![8;16],"analysis":vec![3;16],"parameters":[{"parameter":vec![4;16],"name":"timeout","default":{"kind":"literal","literal":vec![9;16]}}]}]}}}}})
    }
    #[test]
    fn current_public_container_keeps_foreign_variant_and_formal_separate() {
        let packet=decode(&operation().to_string(), &"06".repeat(32)).unwrap();
        let named=&packet.groups[1]; let other=&packet.groups[2]; let foreign=&packet.groups[3];
        assert_eq!(named.evidence.len(),1); // expression ID supplies no default meaning
        assert_ne!(named.context["parameter_identity"],other.context["parameter_identity"]);
        assert_ne!(named.context["variant_identity"],foreign.context["variant_identity"]);
        assert_ne!(named.context["signature_identity"],foreign.context["signature_identity"]);
        assert!(!crate::witness::compatible(&named.context,&other.context));
        assert!(!crate::witness::compatible(&named.context,&foreign.context));
        assert!(foreign.evidence.iter().all(|e| e.qualifications.is_empty()));
        let mut overload=operation();
        overload["structuredContent"]["operation"]["packet"]["core"]["signatures"][1]["variant"]=serde_json::json!(vec![2;16]);
        let same_variant=decode(&overload.to_string(), &"06".repeat(32)).unwrap();
        assert!(!crate::witness::compatible(&same_variant.groups[1].context,&same_variant.groups[3].context));
    }
    #[test]
    fn final_packet_snapshot_tool_and_duplicate_literal_conflicts_refuse() {
        assert!(decode(r#"{"isError":true,"isError":false}"#, &"06".repeat(32)).err().unwrap().contains("duplicate"));
        let mut value=operation();
        assert!(decode(&value.to_string(), &"ff".repeat(32)).is_err());
        value["content"][0]["text"]=serde_json::json!("get_evidence: snapshot-bound result");
        assert!(decode(&value.to_string(), &"06".repeat(32)).is_err());
        let mut value=operation();
        let entry=value["structuredContent"]["operation"]["packet"]["core"]["literal_values"][0].clone();
        value["structuredContent"]["operation"]["packet"]["core"]["literal_values"].as_array_mut().unwrap().push(entry);
        assert!(decode(&value.to_string(), &"06".repeat(32)).is_err());
    }
}
#[cfg(test)]
mod closure_tests {
    use super::*;
    fn packet()->Value{
        let original=serde_json::json!({"source":{"kind":"occurrence","occurrence":vec![8;16]},"artifact":vec![7;16],"context":vec![3;16],"release":vec![11;16],"start":0,"end":4,"encoding":"raw_bytes"});
        let atom=serde_json::json!({"analysis":vec![3;16],"predicate":"is truthy","value":true,"evaluation":{"text":"flag","original":original}});
        let q=serde_json::json!({"qualification":vec![16;16],"analysis":vec![3;16],"constant":null,"truncated":false,"terms":[[atom]],"modality":"definite","approximation":"exact","claim_basis":{"definitions":[]},"availability":{"status":"available"}});
        let context=serde_json::json!({"analysis":vec![3;16],"python_version":"3.14","python_platform":"linux","search_path":["/captured/src"],"site_package_path":[]});
        let default=serde_json::json!({"signature":vec![12;16],"variant":vec![2;16],"analysis":vec![3;16],"parameter":vec![4;16],"field":null,"subject_name":"timeout","option":vec![15;16],"value":{"kind":"literal","literal":vec![9;16]},"readable":"10","original":null,"qualification":vec![16;16]});
        let interpretation=serde_json::json!({"contexts":[context],"defaults":[default],"qualifications":[q]});
        let signature=serde_json::json!({"signature":vec![12;16],"variant":vec![2;16],"analysis":vec![3;16],"parameters":[{"parameter":vec![4;16],"name":"timeout","default":{"kind":"unknown"}}]});
        let core=serde_json::json!({"member":vec![1;16],"name":"connect","release":{"distribution":"mini","version":"1","input":vec![10;16],"release":vec![11;16]},"literal_values":[{"literal":vec![9;16],"value":{"kind":"integer","decimal":"10"}}],"signatures":[signature],"interpretation":interpretation});
        serde_json::json!({"content":[{"type":"text","text":"get_operation: snapshot-bound result"}],"isError":false,"structuredContent":{"snapshot":{"semantic":vec![5;32],"realization":vec![6;32],"database":{"namespace":"control","database":"renderer"}},"operation":{"resolution":"unique","packet":{"core":core}}}})
    }
    #[test]
    fn readable_qualification_and_setup_come_from_final_fields(){
        let p=decode(&packet().to_string(),&"06".repeat(32)).unwrap();
        let default=p.groups.last().unwrap();assert_eq!(default.evidence[1].text,"10");assert_eq!(default.evidence[1].qualifications[0].text,"(flag is truthy)");
        assert!(p.groups.iter().any(|g|g.evidence.iter().any(|e|e.role=="python_version"&&e.text=="3.14")));
        let mut missing=packet();missing["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["qualifications"][0]["terms"][0][0]["evaluation"]["text"]=Value::Null;
        let decoded=decode(&missing.to_string(),&"06".repeat(32)).unwrap();assert!(decoded.groups.last().unwrap().evidence[1].qualifications.is_empty());
    }
    #[test]
    fn foreign_callable_and_readable_literal_corruption_refuse(){
        let mut foreign=packet();foreign["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["defaults"][0]["variant"]=serde_json::json!(vec![99;16]);
        assert!(decode(&foreign.to_string(),&"06".repeat(32)).is_err());
        let mut corrupted=packet();corrupted["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["defaults"][0]["readable"]=serde_json::json!("11");
        assert!(decode(&corrupted.to_string(),&"06".repeat(32)).is_err());
    }
    #[test]
    fn field_defaults_preserve_noncallable_container(){
        let mut field=packet();let default=&mut field["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["defaults"][0];
        default["signature"]=Value::Null;default["variant"]=Value::Null;default["parameter"]=Value::Null;default["field"]=serde_json::json!(vec![17;16]);default["subject_name"]=serde_json::json!("limit");
        let decoded=decode(&field.to_string(),&"06".repeat(32)).unwrap();let group=decoded.groups.last().unwrap();assert_eq!(group.context["field_identity"],"11".repeat(16));assert!(!group.context.contains_key("signature_identity"));assert_eq!(group.evidence[0].role,"field_name");
        let mut unknown=field;unknown["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["qualifications"][0]["terms"][0][0]["predicate"]=Value::Null;
        assert!(decode(&unknown.to_string(),&"06".repeat(32)).unwrap().groups.last().unwrap().evidence[1].qualifications.is_empty());
    }
    #[test]
    fn readable_candidates_preserve_uncertainty_and_literal_identity(){
        assert_ne!(literal(&serde_json::json!({"kind":"none"})).unwrap(),literal(&serde_json::json!({"kind":"string","value":"None"})).unwrap());
        let mut source=packet();
        source["structuredContent"]["operation"]["packet"]["core"]["interpretation"]["qualifications"][0]["modality"]="candidate".into();
        let decoded=decode(&source.to_string(),&"06".repeat(32)).unwrap();
        let meaning=&decoded.groups.last().unwrap().evidence[1];
        assert_eq!(meaning.text,"10");assert_eq!(meaning.candidate_status,CandidateStatus::Unknown);
        assert_eq!(meaning.qualifications[0].attributes["modality"],"\"candidate\"");
        assert_eq!(meaning.qualifications[0].attributes["approximation"],"\"exact\"");
    }
}
#[cfg(test)]
mod originals_tests {
    use super::*;
    fn packet(source:Value,encoding:&str,bytes:Vec<u8>)->Value{
        serde_json::json!({"isError":false,"content":[{"type":"text","text":"get_evidence: snapshot-bound result"}],"structuredContent":{"snapshot":{"semantic":vec![5;32],"realization":vec![6;32],"database":{"namespace":"control","database":"renderer"}},"evidence":{"original":{"source":source,"artifact":vec![7;16],"release":vec![11;16],"context":vec![3;16],"start":0,"end":4,"encoding":encoding},"body":{"start":0,"end":bytes.len(),"bytes":bytes}}}})
    }
    #[test]
    fn raw_artifact_and_occurrence_preserve_exact_utf8_source(){
        for source in [serde_json::json!({"kind":"artifact","artifact":vec![7;16]}),serde_json::json!({"kind":"occurrence","occurrence":vec![8;16]})] {
            let p=decode(&packet(source,"raw_bytes",b"flag".to_vec()).to_string(),&"06".repeat(32)).unwrap();assert_eq!(p.groups[0].evidence[0].text,"flag");assert_eq!(p.groups[0].evidence[0].role,"original_source");
        }
        assert!(decode(&packet(serde_json::json!({"kind":"artifact","artifact":vec![7;16]}),"raw_bytes",vec![255,255,255,255]).to_string(),&"06".repeat(32)).is_err());
    }
    #[test]
    fn interpreted_prose_does_not_claim_original_source_bytes(){
        let p=decode(&packet(serde_json::json!({"kind":"prose","slice":vec![8;16]}),"native_literal_utf8_slice",b"interpreted text".to_vec()).to_string(),&"06".repeat(32)).unwrap();assert_eq!(p.groups[0].evidence[0].role,"interpreted_prose");
    }
}
#[cfg(test)]
mod search_tests {
    use super::*;
    fn packet()->Value {
        let release=serde_json::json!({"input":vec![10;16],"release":vec![11;16],"version":"1","distribution":"mini"});
        let original=serde_json::json!({"source":{"kind":"occurrence","occurrence":vec![7;16]},"artifact":vec![8;16],"release":vec![11;16],"context":vec![3;16],"start":20,"end":24,"encoding":"raw_bytes"});
        let primary=serde_json::json!({"window":vec![4;16],"part":vec![5;16],"purpose":0,"analysis":vec![3;16],"binding":vec![6;16],"subject":vec![7;16],"basis":0,"qualification":null,"text":"call","source_maps":[{"start":0,"end":4,"original":original}]});
        let setup=serde_json::json!({"window":vec![4;16],"part":vec![9;16],"purpose":1,"analysis":vec![3;16],"binding":null,"subject":null,"basis":null,"qualification":null,"text":"setup","source_maps":[{"start":0,"end":5,"original":null}]});
        serde_json::json!({"isError":false,"content":[{"type":"text","text":"search_evidence: snapshot-bound result"}],"structuredContent":{"snapshot":{"semantic":vec![5;32],"realization":vec![6;32],"database":{"namespace":"control","database":"renderer"}},"domains":[{"name":"mini","captures":[{"release":release}]}],"ranking":[{"target":{"kind":"unit","unit":vec![1;16]},"context":vec![3;16],"witnesses":[{"occurrence":{"window":vec![4;16],"part":vec![5;16],"context":vec![3;16]}}]}],"results":{"items":[{"unit":vec![1;16],"release":release,"delivered_windows":[primary,setup],"interpretation":{"contexts":[{"analysis":vec![3;16],"python_version":"3.14","python_platform":"linux","search_path":[],"site_package_path":[]}],"defaults":[],"qualifications":[]}}]}}})
    }
    #[test]
    fn actual_source_maps_and_context_roles_are_independently_decoded(){
        let source=packet();
        let decoded=decode(&source.to_string(),&"06".repeat(32)).unwrap();assert_eq!(decoded.tool.as_deref(),Some("search_evidence"));
        assert!(decoded.groups.iter().any(|g|g.evidence.iter().any(|e|e.role=="primary_window_source"&&e.text=="call")));
        let setup=decoded.groups.last().unwrap();assert_eq!(setup.evidence[0].role,"synthetic_context_window");assert_eq!(setup.evidence[0].candidate_status,CandidateStatus::Unknown);
        let mut corrupted=source;corrupted["structuredContent"]["results"]["items"][0]["delivered_windows"][0]["source_maps"][0]["original"]["context"]=serde_json::json!(vec![99;16]);assert!(decode(&corrupted.to_string(),&"06".repeat(32)).is_err());
    }
    #[test]
    fn setup_cannot_be_promoted_to_ranked_primary_or_binding(){
        let mut primary=packet();primary["structuredContent"]["results"]["items"][0]["delivered_windows"][1]["purpose"]=serde_json::json!(0);assert!(decode(&primary.to_string(),&"06".repeat(32)).is_err());
        let mut binding=packet();binding["structuredContent"]["results"]["items"][0]["delivered_windows"][1]["binding"]=serde_json::json!(vec![6;16]);assert!(decode(&binding.to_string(),&"06".repeat(32)).is_err());
        let mut map=packet();map["structuredContent"]["results"]["items"][0]["delivered_windows"][0]["source_maps"][0]["end"]=serde_json::json!(5);assert!(decode(&map.to_string(),&"06".repeat(32)).is_err());
    }
    #[test]
    fn ranked_continuation_preserves_public_ordering_mode(){
        let mut source=packet();let cursor=serde_json::json!({"binding":{"ordering":blake3::hash(b"serving-key-order/v2:default").as_bytes()}}).to_string();
        let encoded=cursor.as_bytes().iter().map(|b|format!("{b:02x}")).collect::<String>();source["structuredContent"]["results"]["continuation"]=serde_json::json!(encoded);
        let decoded=decode(&source.to_string(),&"06".repeat(32)).unwrap();let reference:Value=serde_json::from_str(decoded.references.last().unwrap()).unwrap();assert_eq!(reference["tool"],"search_evidence");assert_eq!(reference["arguments"]["page"]["expanded"],false);assert!(reference["arguments"].get("query").is_none());
        assert!(continuation_expanded("α").is_err());assert!(continuation_expanded("ab").is_err());
    }
    #[test]
    fn distinct_source_instances_join_only_on_semantic_context(){
        let mut source=packet();
        let mut second=source["structuredContent"]["results"]["items"][0]["delivered_windows"][0].clone();
        second["part"]=serde_json::json!(vec![12;16]);second["text"]="more".into();
        second["source_maps"][0]["original"]["start"]=30.into();second["source_maps"][0]["original"]["end"]=34.into();
        let mut witness=source["structuredContent"]["ranking"][0]["witnesses"][0].clone();witness["occurrence"]["part"]=second["part"].clone();
        source["structuredContent"]["ranking"][0]["witnesses"].as_array_mut().unwrap().push(witness);
        source["structuredContent"]["results"]["items"][0]["delivered_windows"].as_array_mut().unwrap().push(second);
        let decoded=decode(&source.to_string(),&"06".repeat(32)).unwrap();
        let leaves=decoded.groups.iter().filter(|g|g.evidence.iter().any(|e|e.role=="primary_window_source")).collect::<Vec<_>>();
        assert_eq!(leaves.len(),2);assert!(crate::witness::compatible(&leaves[0].context,&leaves[1].context));
        assert_ne!(leaves[0].evidence[0].provenance,leaves[1].evidence[0].provenance);
        let names=BTreeMap::from([("first".into(),vec![leaves[0].context.clone()]),("second".into(),vec![leaves[1].context.clone()])]);
        let all=crate::contracts::Witness::All{children:vec![crate::contracts::Witness::Leaf{predicate:"first".into()},crate::contracts::Witness::Leaf{predicate:"second".into()}]};
        assert_eq!(crate::witness::evaluate(&all,&names).len(),1);
        let mut foreign=leaves[1].context.clone();foreign.insert("analysis_identity".into(),"ff".repeat(16));assert!(!crate::witness::compatible(&leaves[0].context,&foreign));
    }
    #[test]
    fn operation_search_names_are_navigation_only(){
        let mut source=packet();source["content"][0]["text"]=serde_json::json!("search_operations: snapshot-bound result");
        let release=source["structuredContent"]["results"]["items"][0]["release"].clone();
        source["structuredContent"]["results"]["items"]=serde_json::json!([{"member":vec![1;16],"analysis":vec![3;16],"name":"call","releases":[release]}]);
        let decoded=decode(&source.to_string(),&"06".repeat(32)).unwrap();assert_eq!(decoded.groups[0].evidence[0].role,"operation_navigation");assert_eq!(decoded.references.len(),1);assert!(!decoded.groups[0].context.contains_key("signature_identity"));
    }
}
