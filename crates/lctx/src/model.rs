//! `lctx model describe`: the typed model this binary lowers, with no database (cutover plan
//! P1.11). Its JSON form is the reviewable schema and codebook snapshot (P0 exit F06).
use clap::ValueEnum;
use lctx_model::domain::{Scalar, ValidatedModel, admission::FrontierContract, stages::Profile};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

fn scalar(scalar: Scalar) -> &'static str {
    match scalar {
        Scalar::Text => "text",
        Scalar::Bool => "bool",
        Scalar::Int16 => "int16",
        Scalar::Int32 => "int32",
        Scalar::Int64 => "int64",
        Scalar::FiniteF64 => "finite_f64",
        Scalar::Id => "id",
        Scalar::Digest => "digest",
        Scalar::Binary => "binary",
    }
}

/// Relations with their fields (type, list, nullability, key and provenance roles, reference
/// target, sum subtype and codebook), sums, the invariants and the facts frontier.
pub fn describe(model: &ValidatedModel) -> Value {
    let facts = FrontierContract::facts(model, Profile::Catalog).ok();
    let relations: Vec<Value> = model.relations().iter().map(|relation| {
        let fields: Vec<Value> = relation.fields().iter().map(|field| {
            let mut described = json!({ "name": field.name(), "type": scalar(field.scalar()), "list": field.list(), "nullable": field.nullable(),
                "key": field.is_key(), "provenance": field.is_provenance() });
            if let Some((_, target)) = field.target() { described["target"] = json!(target); }
            if let Some(subtype) = field.subtype() { described["subtype"] = json!(subtype); }
            if !field.codes().is_empty() { described["codes"] = json!(field.codes().iter().map(|(code, label)| json!([code, label])).collect::<Vec<_>>()); }
            described
        }).collect();
        let mut described = json!({ "name": relation.name(), "fields": fields,
            "facts": facts.as_ref().is_some_and(|contract| contract.contains(relation.name())),
            "invariant_refs": relation.invariant_refs(), "publication_refs": relation.publication_refs() });
        if let Some(family) = relation.family() { described["family"] = json!(format!("{family:?}")); }
        if let Some(sum) = relation.sum() {
            described["sum"] = json!({ "tag": sum.tag, "arms": sum.arms.iter().map(|arm| json!({ "code": arm.code,
                "fields": arm.fields.iter().map(|f| json!({ "name": f.name, "required": f.required })).collect::<Vec<_>>() })).collect::<Vec<_>>() });
        }
        described
    }).collect();
    let invariants: Vec<Value> = model
        .invariants()
        .iter()
        .map(|invariant| {
            json!({ "name": invariant.name, "revision": invariant.revision, "definition_digest": invariant.digest().hex(),
        "inputs": invariant.inputs.iter().map(|input| json!({"relation":input.name(), "order": input.order(),
            "prefix": input.prefix().map(|p| p.name())})).collect::<Vec<_>>() })
        })
        .collect();
    let publication: Vec<_> = model.publication_checks().iter().map(|check| json!({
        "name": check.name, "revision":check.revision, "definition_digest":check.digest().hex(),
        "inputs":check.inputs.iter().map(|input| json!({"relation":input.name(),"order":input.order(),
            "prefix":input.prefix().map(|p|p.name())})).collect::<Vec<_>>()
    })).collect();
    let mut graph_entities=Vec::<Value>::new();
    macro_rules! entities {($($variant:ident:$record:path),* $(,)?)=>{$(
        graph_entities.push(json!({"kind":stringify!($variant),"semantic_owner":<$record as lctx_model::domain::Record>::NAME}));
    )*};}
    lctx_model::graph_entity_records!(entities);
    let mut graph_assertions=Vec::<Value>::new();
    macro_rules! assertions {($($variant:ident:$record:path),* $(,)?)=>{$(
        graph_assertions.push(json!({"kind":stringify!($variant),"semantic_owner":<$record as lctx_model::domain::Record>::NAME,
            "participants":<$record as lctx_model::domain::Record>::fields().iter().filter(|field|field.target().is_some()).map(|field|json!({"role":field.name(),"ordered":field.list()})).collect::<Vec<_>>()}));
    )*};}
    lctx_model::graph_assertion_records!(assertions);
    json!({"graph":{"contract":lctx_model::domain::graph::semantic_contract(model).hex(),"format_version":lctx_model::domain::graph::ARTIFACT_FORMAT_VERSION,"entities":graph_entities,"assertions":graph_assertions}, "digest": model.digest().hex(), "relations": relations, "invariants": invariants, "publication_checks":publication })
}

pub fn text(description: &Value) -> String {
    let mut out = format!(
        "model {}\n",
        description["digest"].as_str().unwrap_or_default()
    );
    for relation in description["relations"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "\n{}{}\n",
            relation["name"].as_str().unwrap_or_default(),
            if relation["facts"] == json!(true) {
                "  [facts]"
            } else {
                ""
            }
        ));
        for field in relation["fields"].as_array().into_iter().flatten() {
            let ty = field["type"].as_str().unwrap_or_default();
            let mut line = format!(
                "  {} {}{}{}",
                field["name"].as_str().unwrap_or_default(),
                ty,
                if field["list"] == json!(true) {
                    "[]"
                } else {
                    ""
                },
                if field["nullable"] == json!(true) {
                    "?"
                } else {
                    ""
                }
            );
            if field["key"] == json!(true) {
                line.push_str(" key");
            }
            if let Some(target) = field["target"].as_str() {
                line.push_str(&format!(" -> {target}"));
            }
            if let Some(codes) = field["codes"].as_array() {
                line.push_str(&format!(" codes {}", codes.len()));
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.push_str(&format!(
        "\n{} invariants\n",
        description["invariants"].as_array().map_or(0, Vec::len)
    ));
    out
}
