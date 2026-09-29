//! Opaque positions bound to the complete normalized request and serving policy.
use crate::{Error, repository::PinnedGeneration};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    format: u32,
    generation: String,
    policy: String,
    request: String,
    offset: u64,
}
fn request_key(value: &Value) -> String {
    format!("{:x}", Sha256::digest(value.to_string().as_bytes()))
}
pub(crate) fn encode(g: &PinnedGeneration, target: &Value, offset: u64) -> Result<String, Error> {
    let value = Cursor {
        format: cpg_schema::wire::FORMAT,
        generation: g.generation(),
        policy: g.policy.digest()?,
        request: request_key(target),
        offset,
    };
    Ok(URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&value).map_err(|_| Error::Request("cursor encoding".into()))?))
}
pub(crate) fn position(
    g: &PinnedGeneration,
    target: &Value,
    raw: Option<&str>,
) -> Result<u64, Error> {
    let Some(raw) = raw else { return Ok(0) };
    let invalid =
        || Error::Request("cursor belongs to another generation, request or policy".into());
    if raw.len() > 2048 {
        return Err(invalid());
    }
    let c: Cursor = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    if c.format != cpg_schema::wire::FORMAT
        || c.generation != g.generation()
        || c.policy != g.policy.digest()?
        || c.request != request_key(target)
        || c.offset > i64::MAX as u64
    {
        return Err(invalid());
    }
    Ok(c.offset)
}
