use std::io::{self, BufRead, Write};
use lctx_eval::{contracts::*, experiment::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_LINE: usize = 4 * 1024 * 1024;
const MAX_BATCH: usize = 32;
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Schema,
    Judge { cases: Vec<Case> },
    Freeze { experiment: Experiment },
    Admit { frozen: Freeze, experiment: Experiment },
    Feedback { proposal: FeedbackProposal, current_revision: String },
    Rejudge { input: Box<Rejudgment> },
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response { Completed { result: serde_json::Value }, Failed { reason: String } }
fn run(request: Request) -> Result<serde_json::Value, String> {
    let value = match request {
        Request::Schema => Ok(wire_schema()),
        Request::Judge { cases } => {
            if cases.is_empty() || cases.len() > MAX_BATCH { return Err("batch must contain 1..32 cases".into()); }
            serde_json::to_value(cases.iter().map(lctx_eval::judge).collect::<Vec<_>>())
        }
        Request::Freeze { experiment } => serde_json::to_value(freeze(experiment)?),
        Request::Admit { frozen, experiment } => { admit(&frozen, &experiment)?; Ok(serde_json::json!({"admitted": true})) }
        Request::Feedback { proposal, current_revision } => serde_json::to_value(triage(&proposal, &current_revision)?),
        Request::Rejudge { input } => serde_json::to_value(rejudge(&input)?),
    }; value.map_err(|e| e.to_string())
}
fn wire_schema() -> serde_json::Value {
    let mut source = blake3::Hasher::new();
    for input in [include_str!("../Cargo.toml"), include_str!("contracts.rs"), include_str!("witness.rs"), include_str!("experiment.rs"), include_str!("lib.rs"), include_str!("observer.rs"), include_str!("main.rs")] {
        source.update(&(input.len() as u64).to_le_bytes());
        source.update(input.as_bytes());
    }
    serde_json::json!({"protocol_version": 2, "kernel_source_revision": source.finalize().to_hex().to_string(), "request": schemars::schema_for!(Request), "response": schemars::schema_for!(Response), "case": schemars::schema_for!(Case), "finite_packet": schemars::schema_for!(lctx_eval::observer::FinitePacket), "judgment": schemars::schema_for!(Judgment), "experiment": schemars::schema_for!(Experiment)})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--schema") {
        let schema = wire_schema();
        println!("{}", serde_json::to_string_pretty(&schema)?); return Ok(());
    }
    let mut input = io::stdin().lock(); let mut output = io::stdout().lock();
    loop {
        // read_until alone can allocate an unbounded hostile line. Take a fixed-size reader first.
        let mut bytes = Vec::new();
        let read = std::io::Read::take(&mut input, (MAX_LINE + 1) as u64).read_until(b'\n', &mut bytes)?;
        if read == 0 { break; }
        let response = if bytes.len() > MAX_LINE {
            while !bytes.ends_with(b"\n") {
                let available = input.fill_buf()?;
                if available.is_empty() { break; }
                let end = available.iter().position(|b| *b == b'\n').map_or(available.len(), |i| i + 1);
                let found = available[..end].ends_with(b"\n");
                input.consume(end); if found { break; }
            }
            Response::Failed { reason: "JSONL line exceeds 4MiB".into() }
        } else { match serde_json::from_slice::<Request>(&bytes).map_err(|e| e.to_string()).and_then(run) {
            Ok(result) => Response::Completed { result }, Err(reason) => Response::Failed { reason },
        }};
        let mut encoded = serde_json::to_vec(&response)?;
        if encoded.len() >= MAX_LINE {
            encoded = serde_json::to_vec(&Response::Failed { reason: "JSONL response exceeds 4MiB finite output bound".into() })?;
        }
        output.write_all(&encoded)?; output.write_all(b"\n")?; output.flush()?;
    }
    Ok(())
}
