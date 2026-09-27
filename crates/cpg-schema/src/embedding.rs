//! Exact consumed-vector receipts and their canonical byte recipes (ADR-0065/0067).
//! PostgreSQL reuses immutable winners across attempts. Ordinary snapshot-local Delta tables
//! preserve every consumed vector independently of that mutable service's availability.
//! The historical EmbeddingCache declaration is retained only for explicit legacy import.

use crate::hash::HashField;
use crate::id::Digest;
use crate::id::{Id, IdHasher};
use crate::table::table;

/// Versioned little-endian IEEE754 Float32 encoding, including the sign bit of zero.
pub const VALUE_CODEC: i16 = 1;

/// Persistent consumer bits are append-only. This is the single owner of their meaning.
#[derive(Clone, Copy)]
pub enum Usage {
    Operation = 1,
    Analytics = 2,
    Brief = 4,
}
impl Usage {
    pub const KNOWN_MASK: i64 =
        Self::Operation as i64 | Self::Analytics as i64 | Self::Brief as i64;
    pub fn valid_mask(mask: i64) -> bool {
        mask > 0 && mask & !Self::KNOWN_MASK == 0
    }
}

pub fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect()
}

pub fn decode_vector(bytes: &[u8], dimensions: u32) -> Result<Vec<f32>, String> {
    if dimensions == 0 || dimensions > 65536 || bytes.len() != dimensions as usize * 4 {
        return Err("invalid encoded vector dimensions".to_owned());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_bits(u32::from_le_bytes(*b)))
        .collect())
}

pub fn value_digest(vector: &[f32]) -> Digest {
    let mut h = IdHasher::new("embedding-value/v1");
    vector.to_vec().hash_into(&mut h);
    h.finish_digest()
}

/// Input order is immaterial; duplicate full keys are rejected by the table key validator.
pub fn receipt_digest(spec: Digest, rows: &[UsedEmbeddingsRow]) -> Digest {
    let mut keys: Vec<_> = rows
        .iter()
        .map(|r| (r.spec_hash, r.input_hash, r.value_digest))
        .collect();
    keys.sort();
    let mut h = IdHasher::new("embedding-receipt/v1");
    h.digest_field(spec).i64(keys.len() as i64);
    for (spec, input, value) in keys {
        h.digest_field(spec).digest_field(input).digest_field(value);
    }
    h.finish_digest()
}

table!(
    /// Exact vectors consumed by this attempt, frozen before publication.
    UsedEmbeddings, UsedEmbeddingsRow = "used_embeddings",
    family = Findings,
    key = [snapshot_id, spec_hash, input_hash],
    checks = [],
    {
        snapshot_id: Id,
        spec_hash: Digest,
        input_hash: Digest,
        vector: Vec<f32>,
        value_digest: Digest,
    }
);

table!(
    /// Recorded before returning a vector to a consumer; permits independent coverage checks.
    EmbeddingUses, EmbeddingUsesRow = "embedding_uses",
    family = Findings,
    key = [snapshot_id, spec_hash, input_hash],
    checks = [],
    {
        snapshot_id: Id,
        spec_hash: Digest,
        input_hash: Digest,
        /// Bitmask: operation documents = 1, E0 = 2, brief documents = 4.
        usage_mask: i64,
    }
);

table!(
    /// One cached vector: the spec and request text that produced it, and the served model.
    EmbeddingCache, EmbeddingCacheRow = "embedding_cache",
    family = EmbeddingCache,
    key = [spec_hash, input_hash],
    checks = [],
    {
        spec_hash: Digest,
        input_hash: Digest,
        vector: Vec<f32>,
        model: String,
    }
);
