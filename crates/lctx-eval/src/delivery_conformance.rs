//! Independently check delivery metadata against the exact final public object.
//! Successful metadata checks never supply facts to the semantic observer.
use serde_json::Value;

fn ancestors<'a>(root: &'a Value, path: &str) -> Result<Vec<&'a Value>, String> {
    if !path.starts_with('/') || path.starts_with("/structuredContent/delivery") {
        return Err("delivery map has an invalid public pointer".into());
    }
    let mut nodes = vec![root];
    for (index, _) in path.match_indices('/').skip(1) {
        nodes.push(root.pointer(&path[..index]).ok_or("delivery map parent pointer is absent")?);
    }
    Ok(nodes)
}
fn nearest<'a>(nodes: &[&'a Value], key: &str) -> Option<&'a Value> {
    nodes.iter().rev().find_map(|node| node.get(key).filter(|value| !value.is_null()))
}
fn nominal_nearest<'a>(nodes: &[&'a Value], key: &str) -> Option<&'a Value> {
    nodes.iter().rev().find_map(|node| node.get(key).filter(|value|value.as_array().is_some_and(|bytes|bytes.len()==16 && bytes.iter().all(|v|v.as_u64().is_some_and(|n|n<=255)))))
}
fn original<'a>(nodes: &[&'a Value], field: &'a Value) -> Option<&'a Value> {
    if field.get("source").is_some() && field.get("artifact").is_some() { return Some(field); }
    nodes.iter().rev().find_map(|node| node.get("original").and_then(|value| {
        if value.get("source").is_some() { Some(value) } else { value.get("original").filter(|v| v.get("source").is_some()) }
    }))
}
fn same_original(actual: &Value, mapped: &Value, body: Option<&Value>) -> bool {
    let mut expected = actual.clone();
    if let Some(body) = body {
        if !matches!(actual["encoding"].as_str(), Some("raw_bytes" | "utf-8")) { return false; }
        let (Some(base), Some(start), Some(end)) = (actual["start"].as_u64(), body["start"].as_u64(), body["end"].as_u64()) else {return false;};
        let (Some(start), Some(end)) = (base.checked_add(start), base.checked_add(end)) else {return false;};
        expected["start"] = start.into(); expected["end"] = end.into();
    }
    &expected == mapped
}
fn expansion(root: &Value, path: &str, item: &Value, nodes: &[&Value]) -> Result<(), String> {
    let Some(expand) = item.get("expand").filter(|v| !v.is_null()) else {return Ok(());};
    let tool = expand["tool"].as_str().ok_or("delivery expansion has no public tool")?;
    let args = expand["arguments"].as_object().ok_or("delivery expansion has no public arguments")?;
    if let Some(cursor) = args.get("page").and_then(|v| v.get("cursor")) {
        let actual = root.pointer(path).and_then(|v|v.get("continuation")).or_else(|| nearest(nodes,"continuation"));
        if actual != Some(cursor) {return Err("delivery expansion cursor differs from actual omitted page".into());}
    }
    if tool == "get_evidence" {
        let source = args.get("source").ok_or("delivery source expansion is missing source")?;
        if !nodes.iter().any(|node| original(&[node], node).is_some_and(|o|o.get("source")==Some(source))) {
            return Err("delivery expansion invents source attribution".into());
        }
    } else if tool == "get_operation" {
        let member = args.get("operation").and_then(|v|v.get("member")).ok_or("delivery operation expansion has no member")?;
        if nearest(nodes,"member") != Some(member) {return Err("delivery expansion invents member attribution".into());}
        if let Some(analysis) = args.get("page").and_then(|v|v.get("evidence_demand")).and_then(|v|v.get("context")).and_then(|v|v.get("analysis")) {
            if nearest(nodes,"analysis") != Some(analysis) {return Err("delivery expansion invents analysis attribution".into());}
        }
    }
    Ok(())
}
pub fn validate(result: &Value) -> Result<(), String> {
    let Some(map) = result.get("structuredContent").and_then(|v|v.get("delivery")) else {return Ok(());};
    let fields = map["fields"].as_array().ok_or("delivery map fields are absent")?;
    let omissions = map["omissions"].as_array().ok_or("delivery map omissions are absent")?;
    if fields.len() > 16384 || omissions.len() > 16384 {return Err("delivery map finite bound exceeded".into());}
    for item in fields {
        let path = item["field"].as_str().ok_or("delivery field is not a public pointer")?;
        let actual = result.pointer(path).ok_or("delivery map field pointer is absent")?;
        let nodes = ancestors(result,path)?;
        for key in ["member","signature","variant","analysis","parameter","field"] {
            let supplied = item.get("binding").and_then(|v|v.get(key)).filter(|v|!v.is_null());
            let expected = nominal_nearest(&nodes,key).or_else(|| (key=="analysis").then(||original(&nodes,actual).and_then(|o|o.get("context"))).flatten());
            if supplied != expected {return Err(format!("delivery map {key} binding disagrees with final field"));}
        }
        let role = item["role"].as_str().ok_or("delivery field role is absent")?;
        if !matches!(role,"primary"|"interpretation"|"synthetic"|"reference") {return Err("delivery map role is unknown".into());}
        if let Some(purpose) = nearest(&nodes,"purpose").and_then(Value::as_u64) {
            if path.ends_with("/text") && (purpose==1) != (role=="interpretation") {return Err("delivery map promotes context to primary".into());}
        }
        if role=="primary" && !actual.is_string() && !path.ends_with("/body/bytes") {return Err("delivery map reference is not primary readable evidence".into());}
        if let Some(mapped) = item.get("original").filter(|v|!v.is_null()) {
            let source = original(&nodes,actual).ok_or("delivery map invents original source range")?;
            let body = path.ends_with("/body/bytes").then(|| nodes.last().copied()).flatten();
            if !same_original(source,mapped,body) {return Err("delivery map original range disagrees with actual field".into());}
        } else if path.ends_with("/body/bytes") && role!="synthetic" {return Err("delivery map loses original body attribution".into());}
        for dependency in item["dependencies"].as_array().ok_or("delivery dependencies are absent")? {
            let pointer=dependency.as_str().ok_or("delivery dependency is not a pointer")?;
            if result.pointer(pointer).is_none() {return Err("delivery map dependency is absent".into());}
        }
        let qualification = nodes.last().and_then(|v|v.get("qualification")).filter(|v|!v.is_null());
        let qualifications = item["qualifications"].as_array().ok_or("delivery qualifications are absent")?;
        if qualifications.as_slice()!=qualification.into_iter().cloned().collect::<Vec<_>>().as_slice() {return Err("delivery map qualification differs from actual field".into());}
        if let Some(availability) = nodes.last().and_then(|v|v.get("availability")) {
            if availability["status"]!= "available" && item["availability"]["status"]=="available" {return Err("delivery map upgrades unavailable evidence".into());}
        }
    }
    for item in omissions {
        let path=item["field"].as_str().ok_or("delivery omission field is absent")?;
        let nodes=ancestors(result,path)?;
        let actual=result.pointer(path);
        let available=actual.is_some_and(|v|!v.is_null()&&v.get("omitted").and_then(Value::as_u64).unwrap_or(0)==0&&v.get("truncated").and_then(Value::as_bool)!=Some(true)&&v.get("availability").and_then(|a|a.get("status")).and_then(Value::as_str).is_none_or(|kind|kind=="available"));
        if available {return Err("delivery map omits an available complete field".into());}
        expansion(result,path,item,&nodes)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet()->Value {
        serde_json::json!({"content":[{"type":"text","text":"label"}],"structuredContent":{"item":{"member":vec![1;16],"analysis":vec![2;16],"text":"call"},"delivery":{"fields":[{"field":"/structuredContent/item/text","role":"primary","original":null,"binding":{"member":vec![1;16],"analysis":vec![2;16]},"qualifications":[],"dependencies":["/structuredContent/item/analysis"],"availability":{"status":"available"}}],"omissions":[]}}})
    }
    #[test]
    fn metadata_is_checked_separately_from_truth() {
        let good=packet(); validate(&good).unwrap();
        for key in ["member","analysis"] {let mut wrong=good.clone();wrong["structuredContent"]["delivery"]["fields"][0]["binding"][key]=serde_json::json!([9]);assert!(validate(&wrong).is_err());}
        let mut wrong=good.clone();wrong["structuredContent"]["delivery"]["fields"][0]["field"]="/structuredContent/absent".into();assert!(validate(&wrong).is_err());
        let mut wrong=good.clone();wrong["structuredContent"]["delivery"]["fields"][0]["dependencies"]=serde_json::json!(["/structuredContent/absent"]);assert!(validate(&wrong).is_err());
        let mut wrong=good;wrong["structuredContent"]["delivery"]["omissions"]=serde_json::json!([{"field":"/structuredContent/item/text","expand":null}]);assert!(validate(&wrong).is_err());
    }
}
