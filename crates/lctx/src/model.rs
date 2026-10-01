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
        Scalar::FiniteF64 => "finite_f64",
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
            "facts": facts.as_ref().is_some_and(|contract| contract.contains(relation.name())) });
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
            json!({ "name": invariant.name,
        "inputs": invariant.inputs.iter().map(|input| input.name()).collect::<Vec<_>>() })
        })
        .collect();
    json!({ "digest": model.digest().hex(), "relations": relations, "invariants": invariants })
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
