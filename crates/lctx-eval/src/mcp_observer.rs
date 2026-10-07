//! Fixed independent decoding of the current final MCP structured output. No producer maps.
//! Supported meanings: exact original UTF-8 source, operation names, parameter names and
//! declared literal defaults. Missing effective/runtime/condition semantics stay missing.
use std::collections::BTreeMap;
use serde_json::Value;
use crate::contracts::{Assignment, CandidateStatus};
use crate::observer::{DecodedPacket, PublicEvidence, PublicGroup};


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
        if text(field(original, "encoding")?)? != "utf-8" { return Err("unsupported source encoding".into()); }
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
        groups.push(PublicGroup { context, evidence: vec![observed(anchor.clone(), "original_source", source)] });
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
