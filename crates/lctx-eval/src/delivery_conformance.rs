//! Independently check delivery metadata against the exact final public object.
//! Successful metadata checks never supply facts to the semantic observer.
use crate::contracts::PublicCall;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

struct EvidenceRole {
    role: &'static str,
    original: Option<Value>,
    dependencies: BTreeSet<String>,
}
fn required_fields(
    value: &Value,
    path: &str,
    required: &mut BTreeMap<String, EvidenceRole>,
    omissions: &mut BTreeSet<String>,
) -> Result<(), String> {
    if let Some(values) = value.as_array() {
        for (index, child) in values.iter().enumerate() {
            required_fields(child, &format!("{path}/{index}"), required, omissions)?;
        }
        return Ok(());
    }
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    let mut add =
        |field: &str, role: &'static str, original: Option<Value>, dependencies: &[&str]| {
            required.insert(
                format!("{path}/{field}"),
                EvidenceRole {
                    role,
                    original,
                    dependencies: dependencies
                        .iter()
                        .map(|key| format!("{path}/{key}"))
                        .collect(),
                },
            );
        };
    if object.contains_key("window")
        && object.contains_key("part")
        && object.contains_key("source_maps")
    {
        let role = match value["purpose"].as_u64() {
            Some(0) => "primary",
            Some(1) => "interpretation",
            _ => return Err("delivery window purpose is invalid".into()),
        };
        if !value["text"].is_string() {
            return Err("delivery window text is absent".into());
        }
        add(
            "text",
            role,
            None,
            &["source_maps", "analysis", "qualification"],
        );
    }
    if let Some(original) = value.get("original").filter(|v| v.get("source").is_some()) {
        if value.get("text").is_some_and(Value::is_string) {
            add("text", "primary", Some(original.clone()), &[]);
        } else if value.get("text").is_some() {
            omissions.insert(format!("{path}/text"));
        }
        if let Some(body) = value.get("body") {
            let raw = matches!(original["encoding"].as_str(), Some("raw_bytes" | "utf-8"));
            let mapped = if raw {
                let (Some(base), Some(start), Some(end)) = (
                    original["start"].as_u64(),
                    body["start"].as_u64(),
                    body["end"].as_u64(),
                ) else {
                    return Err("delivery body source coordinates are absent".into());
                };
                let mut actual = original.clone();
                actual["start"] = base
                    .checked_add(start)
                    .ok_or("delivery body start overflow")?
                    .into();
                actual["end"] = base
                    .checked_add(end)
                    .ok_or("delivery body end overflow")?
                    .into();
                Some(actual)
            } else {
                None
            };
            add(
                "body/bytes",
                if raw { "primary" } else { "synthetic" },
                mapped,
                &["release", "interpretation"],
            );
            if body["truncated"].as_bool() == Some(true)
                || body["omitted"].as_u64().is_some_and(|v| v > 0)
            {
                omissions.insert(format!("{path}/body"));
            }
        }
    }
    if value.get("items").is_some_and(Value::is_array)
        && (value["truncated"].as_bool() == Some(true)
            || value["omitted"].as_u64().is_some_and(|v| v > 0)
            || value
                .get("availability")
                .is_some_and(|v| v["status"] != "available"))
    {
        omissions.insert(path.into());
    }
    if object.contains_key("option")
        && object.contains_key("analysis")
        && value.get("readable").is_some_and(Value::is_string)
    {
        let source = if matches!(
            value["value"]["kind"].as_str(),
            Some("expression" | "factory")
        ) {
            value
                .get("original")
                .and_then(|v| v.get("original"))
                .filter(|v| !v.is_null())
                .cloned()
        } else {
            None
        };
        let deps = if value.get("field").is_some_and(|v| !v.is_null()) {
            vec!["analysis", "field"]
        } else {
            vec!["analysis", "variant"]
        };
        add(
            "readable",
            if source.is_some() {
                "primary"
            } else {
                "synthetic"
            },
            source,
            &deps,
        );
    }
    for (key, child) in object {
        if key == "delivery" {
            continue;
        }
        let key = key.replace('~', "~0").replace('/', "~1");
        required_fields(child, &format!("{path}/{key}"), required, omissions)?;
    }
    Ok(())
}

fn ancestors<'a>(root: &'a Value, path: &str) -> Result<Vec<&'a Value>, String> {
    if !path.starts_with('/') || path.starts_with("/structuredContent/delivery") {
        return Err("delivery map has an invalid public pointer".into());
    }
    let mut nodes = vec![root];
    for (index, _) in path.match_indices('/').skip(1) {
        nodes.push(
            root.pointer(&path[..index])
                .ok_or("delivery map parent pointer is absent")?,
        );
    }
    Ok(nodes)
}
fn nearest<'a>(nodes: &[&'a Value], key: &str) -> Option<&'a Value> {
    nodes
        .iter()
        .rev()
        .find_map(|node| node.get(key).filter(|value| !value.is_null()))
}
fn nominal_nearest<'a>(nodes: &[&'a Value], key: &str) -> Option<&'a Value> {
    nodes.iter().rev().find_map(|node| {
        node.get(key).filter(|value| {
            value.as_array().is_some_and(|bytes| {
                bytes.len() == 16 && bytes.iter().all(|v| v.as_u64().is_some_and(|n| n <= 255))
            })
        })
    })
}
fn original<'a>(nodes: &[&'a Value], field: &'a Value) -> Option<&'a Value> {
    if field.get("source").is_some() && field.get("artifact").is_some() {
        return Some(field);
    }
    nodes.iter().rev().find_map(|node| {
        node.get("original").and_then(|value| {
            if value.get("source").is_some() {
                Some(value)
            } else {
                value.get("original").filter(|v| v.get("source").is_some())
            }
        })
    })
}
fn same_original(actual: &Value, mapped: &Value, body: Option<&Value>) -> bool {
    let mut expected = actual.clone();
    if let Some(body) = body {
        if !matches!(actual["encoding"].as_str(), Some("raw_bytes" | "utf-8")) {
            return false;
        }
        let (Some(base), Some(start), Some(end)) = (
            actual["start"].as_u64(),
            body["start"].as_u64(),
            body["end"].as_u64(),
        ) else {
            return false;
        };
        let (Some(start), Some(end)) = (base.checked_add(start), base.checked_add(end)) else {
            return false;
        };
        expected["start"] = start.into();
        expected["end"] = end.into();
    }
    &expected == mapped
}
fn expansion(
    root: &Value,
    path: &str,
    item: &Value,
    nodes: &[&Value],
    origin: Option<&PublicCall>,
) -> Result<(), String> {
    let Some(expand) = item.get("expand").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let tool = expand["tool"]
        .as_str()
        .ok_or("delivery expansion has no public tool")?;
    let args = expand["arguments"]
        .as_object()
        .ok_or("delivery expansion has no public arguments")?;
    let cursor = args.get("page").and_then(|v| v.get("cursor"));
    if let Some(cursor) = cursor {
        let actual = root
            .pointer(path)
            .and_then(|v| v.get("continuation"))
            .or_else(|| nearest(nodes, "continuation"));
        if actual != Some(cursor) {
            return Err("delivery expansion cursor differs from actual omitted page".into());
        }
        let expected = root["content"][0]["text"]
            .as_str()
            .and_then(|v| v.strip_suffix(": snapshot-bound result"));
        if expected != Some(tool) {
            return Err("delivery continuation changes the actual public tool".into());
        }
    }
    // A page-wide search continuation has no enclosing hit release. Its scope comes
    // from the actual call and named domain captures, never an arbitrary child hit.
    let continuation_origin = cursor.is_some()
        && matches!(tool, "search_evidence" | "search_operations")
        && origin.is_some_and(|call| {
            call.tool == tool
                && args.get("library").is_some()
                && call.arguments.get("library") == args.get("library")
        });
    let continuation_releases = root["structuredContent"]["domains"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|domain| continuation_origin && domain.get("name") == args.get("library"))
        .flat_map(|domain| {
            domain
                .get("captures")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|capture| capture.get("release"))
        .collect::<Vec<_>>();
    let demand = args
        .get("page")
        .and_then(|v| v.get("evidence_demand"))
        .and_then(|v| v.get("context"));
    let requested_analysis = demand.and_then(|v| v.get("analysis"));
    let requested_release = demand.and_then(|v| v.get("release"));
    let core = nodes.iter().rev().find_map(|v| v.get("core"));
    let captured_release = nodes
        .iter()
        .rev()
        .find_map(|v| v.get("release").filter(|r| r.get("version").is_some()))
        .or_else(|| core.and_then(|v| v.get("release")));
    if tool == "get_evidence" {
        let source = args
            .get("source")
            .ok_or("delivery source expansion is missing source")?;
        if !nodes
            .iter()
            .any(|node| original(&[node], node).is_some_and(|o| o.get("source") == Some(source)))
        {
            return Err("delivery expansion invents source attribution".into());
        }
        if let Some(analysis) = requested_analysis {
            let expected = nodes
                .iter()
                .rev()
                .find_map(|node| original(&[node], node).and_then(|v| v.get("context")));
            if expected != Some(analysis) {
                return Err("delivery source expansion invents analysis attribution".into());
            }
        }
    } else if tool == "get_operation" {
        let selector = args
            .get("operation")
            .ok_or("delivery operation expansion has no selector")?;
        if let Some(member) = selector.get("member") {
            if nominal_nearest(nodes, "member").or_else(|| core.and_then(|v| v.get("member")))
                != Some(member)
            {
                return Err("delivery expansion invents member attribution".into());
            }
        } else if let Some(path) = selector.get("path") {
            if core
                .and_then(|v| v.get("access"))
                .and_then(|v| v.get("path"))
                != Some(path)
            {
                return Err("delivery expansion invents public-path attribution".into());
            }
        } else {
            return Err("delivery operation expansion has no member/path".into());
        }
        if let Some(analysis) = args
            .get("page")
            .and_then(|v| v.get("evidence_demand"))
            .and_then(|v| v.get("context"))
            .and_then(|v| v.get("analysis"))
            && nominal_nearest(nodes, "analysis") != Some(analysis)
            && !core
                .and_then(|v| v.get("interpretation"))
                .and_then(|v| v.get("contexts"))
                .and_then(Value::as_array)
                .is_some_and(|cs| cs.iter().any(|c| c.get("analysis") == Some(analysis)))
        {
            return Err("delivery expansion invents analysis attribution".into());
        }
    } else if args.get("page").and_then(|v| v.get("cursor")).is_none() {
        return Err("delivery omission has no supported evidence expansion".into());
    }
    if let Some(release) = requested_release {
        let matched = captured_release.is_some_and(|v| v.get("version") == Some(release))
            || nodes.iter().any(|v| {
                v.get("releases")
                    .and_then(Value::as_array)
                    .is_some_and(|rs| rs.iter().any(|r| r.get("version") == Some(release)))
            })
            || (!continuation_releases.is_empty()
                && origin.is_some_and(|call| {
                    call.arguments
                        .get("page")
                        .and_then(|v| v.get("evidence_demand"))
                        .and_then(|v| v.get("context"))
                        .and_then(|v| v.get("release"))
                        == Some(release)
                })
                && continuation_releases
                    .iter()
                    .any(|r| r.get("version") == Some(release)));
        if !matched {
            return Err("delivery expansion invents release attribution".into());
        }
    }
    let requested_signature = demand.and_then(|v| v.get("signature"));
    let requested_variant = demand.and_then(|v| v.get("variant"));
    if requested_signature.is_some() || requested_variant.is_some() {
        if tool == "get_evidence" {
            return Err("delivery source expansion invents callable binding".into());
        }
        let matches = |value: &Value| {
            value.get("signature").is_some_and(|v| !v.is_null())
                && value.get("variant").is_some_and(|v| !v.is_null())
                && requested_signature.is_none_or(|v| value.get("signature") == Some(v))
                && requested_variant.is_none_or(|v| value.get("variant") == Some(v))
                && requested_analysis.is_none_or(|v| value.get("analysis") == Some(v))
        };
        if !nodes.iter().any(|node| matches(node))
            && !core
                .and_then(|v| v.get("signatures"))
                .and_then(Value::as_array)
                .is_some_and(|values| values.iter().any(matches))
        {
            return Err("delivery operation expansion invents callable tuple".into());
        }
    }
    if let Some(library) = args.get("library") {
        let releases = nodes
            .iter()
            .flat_map(|node| {
                node.get("releases")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .chain(captured_release)
            .collect::<Vec<_>>();
        let domain_match = root["structuredContent"]["domains"]
            .as_array()
            .is_some_and(|domains| {
                domains.iter().any(|domain| {
                    domain.get("name") == Some(library)
                        && domain
                            .get("captures")
                            .and_then(Value::as_array)
                            .is_some_and(|captures| {
                                captures.iter().any(|capture| {
                                    capture.get("release").is_some_and(|release| {
                                        (releases.contains(&release)
                                            || continuation_releases.contains(&release))
                                            && requested_release.is_none_or(|version| {
                                                release.get("version") == Some(version)
                                            })
                                    })
                                })
                            })
                })
            });
        // A controlled capture may lack a domain inventory. Its actual get_operation public
        // request supplies the admitted library scope independently of expansion metadata.
        let origin_match = origin.is_some_and(|call| {
            call.tool == "get_operation"
                && call.arguments.get("library") == Some(library)
                && core.is_some()
        });
        if !domain_match && !origin_match {
            return Err("delivery expansion invents library/release scope".into());
        }
    }
    Ok(())
}
pub fn validate(result: &Value) -> Result<(), String> {
    validate_for_call(result, None)
}
pub fn validate_for_call(result: &Value, origin: Option<&PublicCall>) -> Result<(), String> {
    let map = result
        .get("structuredContent")
        .and_then(|v| v.get("delivery"))
        .ok_or("current MCP delivery map is absent")?;
    let fields = map["fields"]
        .as_array()
        .ok_or("delivery map fields are absent")?;
    let omissions = map["omissions"]
        .as_array()
        .ok_or("delivery map omissions are absent")?;
    if fields.len() > 16384 || omissions.len() > 16384 {
        return Err("delivery map finite bound exceeded".into());
    }
    let mut required = BTreeMap::from([(
        "/content/0/text".into(),
        EvidenceRole {
            role: "synthetic",
            original: None,
            dependencies: BTreeSet::new(),
        },
    )]);
    let mut required_omissions = BTreeSet::new();
    required_fields(
        result
            .get("structuredContent")
            .ok_or("current MCP structured result is absent")?,
        "/structuredContent",
        &mut required,
        &mut required_omissions,
    )?;
    let mut seen = BTreeSet::new();
    for item in fields {
        let path = item["field"]
            .as_str()
            .ok_or("delivery field is not a public pointer")?;
        if !seen.insert(path) {
            return Err("delivery map has duplicate field pointers".into());
        }
        let actual = result
            .pointer(path)
            .ok_or("delivery map field pointer is absent")?;
        let nodes = ancestors(result, path)?;
        for key in [
            "member",
            "signature",
            "variant",
            "analysis",
            "parameter",
            "field",
        ] {
            let supplied = item
                .get("binding")
                .and_then(|v| v.get(key))
                .filter(|v| !v.is_null());
            let expected = nominal_nearest(&nodes, key).or_else(|| {
                (key == "analysis")
                    .then(|| original(&nodes, actual).and_then(|o| o.get("context")))
                    .flatten()
            });
            if supplied != expected {
                return Err(format!(
                    "delivery map {key} binding disagrees with final field"
                ));
            }
        }
        let role = item["role"]
            .as_str()
            .ok_or("delivery field role is absent")?;
        if let Some(expected) = required.remove(path) {
            if role != expected.role
                || item.get("original").filter(|v| !v.is_null()) != expected.original.as_ref()
            {
                return Err(
                    "delivery evidence role/source attribution disagrees with final field".into(),
                );
            }
            let dependencies = item["dependencies"]
                .as_array()
                .ok_or("delivery dependencies are absent")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or("delivery dependency is not a pointer".to_owned())
                })
                .collect::<Result<BTreeSet<_>, _>>()?;
            if dependencies != expected.dependencies
                || dependencies.len() != item["dependencies"].as_array().unwrap().len()
            {
                return Err(
                    "delivery evidence dependency closure disagrees with final field".into(),
                );
            }
        }
        if !matches!(
            role,
            "primary" | "interpretation" | "synthetic" | "reference"
        ) {
            return Err("delivery map role is unknown".into());
        }
        if let Some(purpose) = nearest(&nodes, "purpose").and_then(Value::as_u64)
            && path.ends_with("/text")
            && (purpose == 1) != (role == "interpretation")
        {
            return Err("delivery map promotes context to primary".into());
        }
        if role == "primary" && !actual.is_string() && !path.ends_with("/body/bytes") {
            return Err("delivery map reference is not primary readable evidence".into());
        }
        if let Some(mapped) = item.get("original").filter(|v| !v.is_null()) {
            let source =
                original(&nodes, actual).ok_or("delivery map invents original source range")?;
            let body = path
                .ends_with("/body/bytes")
                .then(|| nodes.last().copied())
                .flatten();
            if !same_original(source, mapped, body) {
                return Err("delivery map original range disagrees with actual field".into());
            }
        } else if path.ends_with("/body/bytes") && role != "synthetic" {
            return Err("delivery map loses original body attribution".into());
        }
        for dependency in item["dependencies"]
            .as_array()
            .ok_or("delivery dependencies are absent")?
        {
            let pointer = dependency
                .as_str()
                .ok_or("delivery dependency is not a pointer")?;
            if result.pointer(pointer).is_none() {
                return Err("delivery map dependency is absent".into());
            }
        }
        let qualification = nodes
            .last()
            .and_then(|v| v.get("qualification"))
            .filter(|v| !v.is_null());
        let qualifications = item["qualifications"]
            .as_array()
            .ok_or("delivery qualifications are absent")?;
        if qualifications.as_slice()
            != qualification
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
                .as_slice()
        {
            return Err("delivery map qualification differs from actual field".into());
        }
        if let Some(availability) = nodes.last().and_then(|v| v.get("availability"))
            && availability["status"] != "available"
            && item["availability"]["status"] == "available"
        {
            return Err("delivery map upgrades unavailable evidence".into());
        }
    }
    if !required.is_empty() {
        return Err("delivery map omits required readable evidence".into());
    }
    for item in omissions {
        let path = item["field"]
            .as_str()
            .ok_or("delivery omission field is absent")?;
        required_omissions.remove(path);
        let nodes = ancestors(result, path)?;
        let actual = result.pointer(path);
        let available = actual.is_some_and(|v| {
            !v.is_null()
                && v.get("omitted").and_then(Value::as_u64).unwrap_or(0) == 0
                && v.get("truncated").and_then(Value::as_bool) != Some(true)
                && v.get("availability")
                    .and_then(|a| a.get("status"))
                    .and_then(Value::as_str)
                    .is_none_or(|kind| kind == "available")
        });
        if available {
            return Err("delivery map omits an available complete field".into());
        }
        expansion(result, path, item, &nodes, origin)?;
    }
    if !required_omissions.is_empty() {
        return Err("delivery map omits actual unavailable or truncated evidence".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Value {
        serde_json::json!({"content":[{"type":"text","text":"label"}],"structuredContent":{"item":{"member":vec![1;16],"analysis":vec![2;16],"text":"call"},"delivery":{"fields":[{"field":"/structuredContent/item/text","role":"primary","original":null,"binding":{"member":vec![1;16],"analysis":vec![2;16]},"qualifications":[],"dependencies":["/structuredContent/item/analysis"],"availability":{"status":"available"}}, {"field":"/content/0/text","role":"synthetic","original":null,"binding":{},"qualifications":[],"dependencies":[],"availability":{"status":"available"}}],"omissions":[]}}})
    }
    #[test]
    fn metadata_is_checked_separately_from_truth() {
        let good = packet();
        validate(&good).unwrap();
        let mut scalar = good.clone();
        scalar["structuredContent"]["item"]["field"] = 0.into();
        validate(&scalar).unwrap(); // lexical field codes are not FieldEntity IDs
        for key in ["member", "analysis"] {
            let mut wrong = good.clone();
            wrong["structuredContent"]["delivery"]["fields"][0]["binding"][key] =
                serde_json::json!(vec![9; 16]);
            assert!(validate(&wrong).is_err());
        }
        let mut wrong = good.clone();
        wrong["structuredContent"]["delivery"]["fields"][0]["field"] =
            "/structuredContent/absent".into();
        assert!(validate(&wrong).is_err());
        let mut wrong = good.clone();
        wrong["structuredContent"]["delivery"]["fields"][0]["dependencies"] =
            serde_json::json!(["/structuredContent/absent"]);
        assert!(validate(&wrong).is_err());
        let mut wrong = good;
        wrong["structuredContent"]["delivery"]["omissions"] =
            serde_json::json!([{"field":"/structuredContent/item/text","expand":null}]);
        assert!(validate(&wrong).is_err());
    }
    #[test]
    fn required_window_inventory_and_dependency_roles_cannot_disappear() {
        let mut good = packet();
        good["structuredContent"]["item"] = serde_json::json!({"window":vec![3;16],"part":vec![4;16],"purpose":0,"member":vec![1;16],"analysis":vec![2;16],"text":"call","source_maps":[],"qualification":null});
        good["structuredContent"]["delivery"]["fields"][0]["dependencies"] = serde_json::json!([
            "/structuredContent/item/source_maps",
            "/structuredContent/item/analysis",
            "/structuredContent/item/qualification"
        ]);
        validate(&good).unwrap();
        let mut missing = good.clone();
        missing["structuredContent"]
            .as_object_mut()
            .unwrap()
            .remove("delivery");
        assert!(validate(&missing).is_err());
        let mut missing = good.clone();
        missing["structuredContent"]["delivery"]["fields"] = serde_json::json!([]);
        assert!(validate(&missing).is_err());
        for dependencies in [
            serde_json::json!([]),
            serde_json::json!(["/content/0/text"]),
        ] {
            let mut wrong = good.clone();
            wrong["structuredContent"]["delivery"]["fields"][0]["dependencies"] = dependencies;
            assert!(validate(&wrong).is_err());
        }
        for role in ["synthetic", "reference", "interpretation"] {
            let mut wrong = good.clone();
            wrong["structuredContent"]["delivery"]["fields"][0]["role"] = role.into();
            assert!(validate(&wrong).is_err());
        }
        let mut duplicate = good.clone();
        let row = duplicate["structuredContent"]["delivery"]["fields"][0].clone();
        duplicate["structuredContent"]["delivery"]["fields"]
            .as_array_mut()
            .unwrap()
            .push(row);
        assert!(validate(&duplicate).is_err());
    }
    #[test]
    fn expansion_keeps_one_callable_and_library_release_tuple() {
        let mut good = packet();
        let release = serde_json::json!({"input":vec![10;16],"release":vec![11;16],"distribution":"mini","version":"1"});
        good["structuredContent"]["item"]["core"] = serde_json::json!({"member":vec![1;16],"release":release,"signatures":[{"signature":vec![12;16],"variant":vec![13;16],"analysis":vec![2;16]},{"signature":vec![14;16],"variant":vec![15;16],"analysis":vec![2;16]}]});
        good["structuredContent"]["item"]["interpretation"] =
            serde_json::json!({"availability":{"status":"unavailable"}});
        good["structuredContent"]["domains"] = serde_json::json!([{"name":"mini","captures":[{"release":release}]},{"name":"other","captures":[{"release":{"input":vec![20;16],"release":vec![21;16],"distribution":"other","version":"2"}}]}]);
        good["structuredContent"]["delivery"]["omissions"] = serde_json::json!([{"field":"/structuredContent/item/interpretation","availability":{"status":"unavailable"},"expand":{"tool":"get_operation","arguments":{"library":"mini","operation":{"kind":"member","member":vec![1;16]},"page":{"evidence_demand":{"context":{"analysis":vec![2;16],"signature":vec![12;16],"variant":vec![13;16],"release":"1"}}}}}}]);
        validate(&good).unwrap();
        let mut wrong = good.clone();
        wrong["structuredContent"]["delivery"]["omissions"][0]["expand"]["arguments"]["page"]["evidence_demand"]
            ["context"]["variant"] = serde_json::json!(vec![15; 16]);
        assert!(validate(&wrong).is_err());
        let mut wrong = good.clone();
        wrong["structuredContent"]["delivery"]["omissions"][0]["expand"]["arguments"]["library"] =
            "other".into();
        assert!(validate(&wrong).is_err());
        good["structuredContent"]
            .as_object_mut()
            .unwrap()
            .remove("domains");
        assert!(validate(&good).is_err());
        let origin = PublicCall {
            tool: "get_operation".into(),
            arguments: serde_json::json!({"library":"mini"}),
        };
        validate_for_call(&good, Some(&origin)).unwrap();
    }
    #[test]
    fn search_page_continuation_keeps_actual_origin_and_named_capture_scope() {
        let mut good = packet();
        good["content"][0]["text"] = "search_evidence: snapshot-bound result".into();
        let release = serde_json::json!({"input":vec![10;16],"release":vec![11;16],"distribution":"mini","version":"1"});
        good["structuredContent"]["domains"] = serde_json::json!([
            {"name":"mini","captures":[{"release":release}]},
            {"name":"other","captures":[{"release":{"input":vec![20;16],"release":vec![21;16],"distribution":"other","version":"2"}}]}
        ]);
        good["structuredContent"]["results"] = serde_json::json!({"items":[],"continuation":"7b7d","omitted":1,"truncated":true,"availability":{"status":"available"}});
        let arguments = serde_json::json!({"library":"mini","query":"captured","page":{"cursor":"7b7d","expanded":true,"evidence_demand":{"context":{"release":"1"}}}});
        good["structuredContent"]["delivery"]["omissions"] = serde_json::json!([{"field":"/structuredContent/results","availability":{"status":"partial"},"expand":{"tool":"search_evidence","arguments":arguments}}]);
        let origin = PublicCall {
            tool: "search_evidence".into(),
            arguments: serde_json::json!({"library":"mini","query":"captured","page":{"evidence_demand":{"context":{"release":"1"}}}}),
        };
        validate_for_call(&good, Some(&origin)).unwrap();
        assert!(
            validate(&good).is_err(),
            "metadata alone cannot supply the originating library"
        );
        for (pointer, value) in [
            (
                "/structuredContent/delivery/omissions/0/expand/arguments/library",
                serde_json::json!("other"),
            ),
            (
                "/structuredContent/delivery/omissions/0/expand/arguments/page/cursor",
                serde_json::json!("ffff"),
            ),
            (
                "/structuredContent/delivery/omissions/0/expand/tool",
                serde_json::json!("search_operations"),
            ),
            (
                "/structuredContent/delivery/omissions/0/expand/arguments/page/evidence_demand/context/release",
                serde_json::json!("2"),
            ),
            (
                "/structuredContent/domains/0/name",
                serde_json::json!("foreign"),
            ),
            (
                "/structuredContent/domains/0/captures/0/release/version",
                serde_json::json!("foreign"),
            ),
        ] {
            let mut wrong = good.clone();
            *wrong.pointer_mut(pointer).unwrap() = value;
            assert!(
                validate_for_call(&wrong, Some(&origin)).is_err(),
                "{pointer}"
            );
        }
        let mut wrong = good.clone();
        wrong["structuredContent"]["domains"][0]["captures"] = serde_json::json!([]);
        assert!(validate_for_call(&wrong, Some(&origin)).is_err());
    }
}
