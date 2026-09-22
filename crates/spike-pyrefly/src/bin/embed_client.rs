//! Compile-time embedding client (reqwest + tokio) for the ADR-0010 spike.
//! Applies the same spec as the Python client, validates the response, writes JSON.
//! Usage: embed_client URL INPUTS.json OUT.json

use serde_json::{Value, json};

const MODEL: &str = "Qwen/Qwen3-Embedding-8B";
const DIMENSIONS: usize = 4096;
const TASK: &str = "Given a coding task, retrieve capability briefs of a Python library that solve it";

fn format_input(kind: &str, text: &str) -> String {
    match kind {
        "query" => format!("Instruct: {TASK}\nQuery:{text}"),
        _ => text.to_owned(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (url, inputs, dest) = (&args[1], &args[2], &args[3]);
    let inputs: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(inputs)?)?;
    let texts: Vec<String> = inputs
        .iter()
        .map(|i| format_input(i["kind"].as_str().unwrap(), i["text"].as_str().unwrap()))
        .collect();
    let body: Value = reqwest::Client::new()
        .post(format!("{url}/v1/embeddings"))
        .json(&json!({"model": MODEL, "input": texts, "encoding_format": "float"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if body["model"] != MODEL {
        return Err(format!("model mismatch: {}", body["model"]).into());
    }
    let data = body["data"].as_array().ok_or("no data")?;
    let mut out: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
    for d in data {
        let idx = d["index"].as_u64().ok_or("no index")? as usize;
        let v: Vec<f32> = d["embedding"]
            .as_array()
            .ok_or("no embedding")?
            .iter()
            .map(|x| x.as_f64().unwrap() as f32)
            .collect();
        if v.len() != DIMENSIONS || v.iter().any(|x| !x.is_finite()) {
            return Err("wrong length or non-finite value".into());
        }
        if out.get(idx).is_none_or(|slot| slot.is_some()) {
            return Err("wrong index mapping".into());
        }
        out[idx] = Some(v);
    }
    let mut vectors = serde_json::Map::new();
    let mut norms = serde_json::Map::new();
    for (i, v) in inputs.iter().zip(out) {
        let v = v.ok_or("missing index")?;
        let n = v.iter().map(|x| f64::from(*x) * f64::from(*x)).sum::<f64>().sqrt();
        let id = i["id"].as_str().unwrap().to_owned();
        norms.insert(id.clone(), json!(n));
        vectors.insert(id, json!(v));
    }
    std::fs::write(dest, serde_json::to_string(&json!({"vectors": vectors, "norms": norms, "texts": texts}))?)?;
    println!("{}", serde_json::to_string(&norms)?);
    Ok(())
}
