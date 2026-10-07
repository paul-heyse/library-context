//! Fixed independent decoding of the current final MCP structured output. No producer maps.
//! Supported meanings: exact original UTF-8 source, operation names, parameter names and
//! declared literal defaults. Missing effective/runtime/condition semantics stay missing.
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
    PublicEvidence { anchor, role: role.into(), text, qualifications: vec![], candidate_status: CandidateStatus::Supported }
}
fn literal(value: &Value) -> Result<Option<String>, String> {
    match text(field(value, "kind")?)? {
        "none" => Ok(Some("None".into())),
        "bool" => Ok(Some(if field(value, "value")?.as_bool().ok_or("public Boolean literal")? { "True".into() } else { "False".into() })),
        "integer" => Ok(Some(text(field(value, "decimal")?)?.into())),
        "string" => Ok(Some(text(field(value, "value")?)?.into())),
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
    Ok(Some(PublicQualification{id,text:meaning}))
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
                if !field(default,"qualification")?.is_null(){let q=identity(field(default,"qualification")?,16)?;if let Some(Some(q))=qualifications.get(&(q,analysis.clone())){meaning.qualifications.push(q.clone());}}
                evidence.push(meaning);
            }
        }
        groups.push(PublicGroup{context:assignment,evidence});
    }
    Ok(())
}
pub fn decode(bytes: &str, realization: &str) -> Result<DecodedPacket, String> {
    let UniqueJson(root) = serde_json::from_str(bytes).map_err(|e| e.to_string())?;
    let result = if root.get("jsonrpc").is_some() { field(&root, "result")? } else { &root };
    if field(result, "isError")?.as_bool() != Some(false) { return Err("MCP operation returned error/refusal".into()); }
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
        if !matches!(text(field(original, "encoding")?)?, "utf-8"|"raw_bytes") { return Err("unsupported source encoding".into()); }
        let source = String::from_utf8(body_bytes).map_err(|_| "source bytes are not UTF-8")?;
        let artifact = identity(field(original, "artifact")?, 16)?;
        let original_start = integer(field(original, "start")?)?;
        let original_end = integer(field(original, "end")?)?;
        if original_end < original_start || end > original_end - original_start { return Err("public source range mismatch".into()); }
        let anchor = serde_json::to_string(field(original, "source")?).map_err(|e| e.to_string())?;
        let context = Assignment::from([
            ("artifact_identity".into(), artifact),
            ("release_identity".into(), identity(field(original, "release")?,16)?),
            ("analysis_identity".into(), identity(field(original, "context")?,16)?),
            ("source_start".into(), original_start.to_string()),
            ("source_end".into(), original_end.to_string()),
        ]);
        let mut context=context;
        if let Some(release)=evidence.get("release") {
            if identity(field(release,"release")?,16)?!=context["release_identity"]{return Err("source readable release binding disagreement".into());}
            context.insert("release".into(),text(field(release,"version")?)?.into());context.insert("distribution".into(),text(field(release,"distribution")?)?.into());
        }
        groups.push(PublicGroup { context:context.clone(), evidence: vec![observed(anchor.clone(), "original_source", source)] });
        interpretation(evidence,&context,&mut groups)?;
        if let Some(cursor) = body.get("continuation") { references.push(serde_json::json!({"tool":"get_evidence","arguments":{"source":field(original,"source")?,"page":{"cursor":text(cursor)?,"expanded":true}}}).to_string()); }
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
                if default.get("kind").and_then(Value::as_str) == Some("literal") {
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
    } else { return Err("unsupported current MCP response class".into()); }
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
        serde_json::json!({"content":[{"type":"text","text":"get_operation: snapshot-bound result"}],"isError":false,"structuredContent":{"snapshot":{"semantic":vec![5;32],"realization":vec![6;32],"database":{"namespace":"control","database":"renderer"}},"operation":{"resolution":"unique","packet":{"core":{"member":vec![1;16],"name":"connect","release":{"distribution":"mini","version":"1","input":vec![10;16],"release":vec![11;16]},"literal_values":[{"literal":vec![9;16],"value":{"kind":"integer","decimal":"10"}}],"signatures":[{"signature":vec![12;16],"variant":vec![2;16],"analysis":vec![3;16],"parameters":[{"parameter":vec![4;16],"name":"timeout","default":{"kind":"unknown"}}]}],"interpretation":{"contexts":[{"analysis":vec![3;16],"python_version":"3.14","python_platform":"linux","search_path":["/captured/src"],"site_package_path":[]}],"defaults":[{"signature":vec![12;16],"variant":vec![2;16],"analysis":vec![3;16],"parameter":vec![4;16],"field":null,"subject_name":"timeout","option":vec![15;16],"value":{"kind":"literal","literal":vec![9;16]},"readable":"10","original":null,"qualification":vec![16;16]}],"qualifications":[{"qualification":vec![16;16],"analysis":vec![3;16],"constant":null,"truncated":false,"terms":[[{"analysis":vec![3;16],"predicate":"is truthy","value":true,"evaluation":{"text":"flag","original":{"source":{"kind":"occurrence","occurrence":vec![8;16]},"artifact":vec![7;16],"context":vec![3;16],"release":vec![11;16],"start":0,"end":4,"encoding":"raw_bytes"}}}]]}]}}}}}})
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
}
