//! The retained f32 little-endian value recipe, independent of nominal generation row identity.
use super::{Spec, EmbeddingSpec, check_vector};
use crate::Domain;
use crate::domain::{
    ContentHash, ModelError,
    resources::{Reservation, ResourceBudget},
    Id, EvidenceBytes, Record,
};
use sha2::{Digest as _, Sha256};

pub const VALUE_CODEC: i16 = 1;

/// One immutable full winner. Digests are payload, so competing bytes cannot create legal keys.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="embedding_full_values",validate=validate_full)]
pub struct FullValue {
    #[model(key)] pub encoder: Id<EmbeddingSpec>,
    #[model(key)] pub input: ContentHash,
    pub dimensions: i64,
    pub tokens: i64,
    pub codec: i16,
    pub digest: ContentHash,
    pub bytes: EvidenceBytes,
}
impl FullValue {
    pub fn new(encoder:&EmbeddingSpec,value:&AdmittedValue)->Result<Self,ModelError> {
        encoder.validate()?;
        if value.spec()!=encoder.service_hash {return Err(ModelError::Invalid("full winner encoder mismatch".into()));}
        let row=Self {encoder:encoder.id(),input:value.input(),dimensions:encoder.dimensions,tokens:value.tokens().into(),codec:VALUE_CODEC,digest:value.digest(),bytes:EvidenceBytes(value.bytes().to_vec())};
        row.validate()?; Ok(row)
    }
    pub fn verify_encoder(&self,encoder:&EmbeddingSpec)->Result<(),ModelError> {
        self.validate()?; encoder.validate()?;
        if self.encoder!=encoder.id() || self.dimensions!=encoder.dimensions {return Err(ModelError::Invalid("full winner has foreign encoder/dimensions".into()));} Ok(())
    }
}
fn validate_full(row:&FullValue)->Result<(),ModelError> {
    if row.tokens<0 || row.tokens>u32::MAX.into() || row.codec!=VALUE_CODEC {return Err(ModelError::Invalid("invalid full value admission".into()));}
    let dimensions=u32::try_from(row.dimensions).map_err(ModelError::codec)?;
    let vector=decode_vector(&row.bytes.0,dimensions).map_err(ModelError::Invalid)?;
    check_vector(&vector,dimensions).map_err(ModelError::Invalid)?;
    if value_digest(&vector)!=row.digest {return Err(ModelError::Invalid("full winning digest differs".into()));} Ok(())
}
pub fn input_hash(request_text: &str) -> ContentHash {
    ContentHash(Sha256::digest(request_text.as_bytes()).into())
}

/// Codec operations are also used by the retained cache, whose caller admits its batch budget.
pub fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect()
}
pub fn decode_vector(bytes: &[u8], dimensions: u32) -> Result<Vec<f32>, String> {
    if dimensions == 0 || dimensions > 65536 || bytes.len() != dimensions as usize * 4 {
        return Err("invalid encoded vector dimensions".into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| f32::from_bits(u32::from_le_bytes(*v)))
        .collect())
}

/// This cache protocol digest predates nominal model rows. Preserve its complete byte recipe so
/// current immutable cache winners retain their meaning; it is never a semantic row identifier.
pub fn value_digest(vector: &[f32]) -> ContentHash {
    fn field(hash: &mut blake3::Hasher, bytes: &[u8]) {
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"lctx-id/v1");
    field(&mut hash, b"embedding-value/v1");
    field(&mut hash, &(vector.len() as i64).to_le_bytes());
    for value in vector {
        field(&mut hash, &value.to_bits().to_le_bytes());
    }
    ContentHash(*hash.finalize().as_bytes())
}

/// Exact consumed bytes with validated token, shape, finiteness and unit-norm admission.
/// No caller can mutate a component while retaining its validated digest.
pub struct AdmittedValue {
    spec: ContentHash,
    input: ContentHash,
    tokens: u32,
    bytes: Vec<u8>,
    digest: ContentHash,
    _reservation: Box<dyn Reservation>,
}
impl AdmittedValue {
    pub fn new(
        spec: &Spec,
        request_text: &str,
        tokens: u32,
        vector: &[f32],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        spec.validate().map_err(ModelError::Invalid)?;
        if tokens > spec.max_document_tokens {
            return Err(ModelError::Invalid(
                "embedding request exceeds its token cap".into(),
            ));
        }
        check_vector(vector, spec.dimensions).map_err(ModelError::Invalid)?;
        let reservation = budget.reserve(
            "embedding-consumed-value",
            size_of::<Self>() + vector.len() * 4,
        )?;
        let bytes = encode_vector(vector);
        Ok(Self {
            spec: spec.hash(),
            input: input_hash(request_text),
            tokens,
            bytes,
            digest: value_digest(vector),
            _reservation: reservation,
        })
    }
    pub fn spec(&self) -> ContentHash {
        self.spec
    }
    pub fn input(&self) -> ContentHash {
        self.input
    }
    pub fn tokens(&self) -> u32 {
        self.tokens
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> ContentHash {
        self.digest
    }
    pub fn decode(&self, spec: &Spec, budget: &ResourceBudget) -> Result<DecodedValue, ModelError> {
        if self.spec != spec.hash() {
            return Err(ModelError::Invalid(
                "consumed embedding has another specification".into(),
            ));
        }
        decode(spec, &self.bytes, self.digest, self.tokens, budget)
    }
}
pub struct DecodedValue {
    values: Vec<f32>,
    _reservation: Box<dyn Reservation>,
}
impl DecodedValue {
    pub fn values(&self) -> &[f32] {
        &self.values
    }
}
pub fn decode(
    spec: &Spec,
    bytes: &[u8],
    digest: ContentHash,
    tokens: u32,
    budget: &ResourceBudget,
) -> Result<DecodedValue, ModelError> {
    spec.validate().map_err(ModelError::Invalid)?;
    if tokens > spec.max_document_tokens || bytes.len() != spec.dimensions as usize * 4 {
        return Err(ModelError::Invalid(
            "stored embedding dimensions or token admission differ".into(),
        ));
    }
    let reservation = budget.reserve(
        "embedding-decoded-value",
        size_of::<DecodedValue>() + bytes.len(),
    )?;
    let values = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_bits(u32::from_le_bytes(*bytes)))
        .collect::<Vec<_>>();
    check_vector(&values, spec.dimensions).map_err(ModelError::Invalid)?;
    if value_digest(&values) != digest {
        return Err(ModelError::Invalid(
            "stored embedding value digest differs".into(),
        ));
    }
    Ok(DecodedValue {
        values,
        _reservation: reservation,
    })
}

/// Decode the deliberately selected analytical/search representation.
pub fn decode_projection(row:&super::projection::ProjectedValue,budget:&ResourceBudget)->Result<DecodedValue,ModelError> {
    row.validate()?;
    let reservation=budget.reserve("embedding-decoded-projection",size_of::<DecodedValue>()+row.bytes.0.len())?;
    let values=decode_vector(&row.bytes.0,row.dimensions as u32).map_err(ModelError::Invalid)?;
    Ok(DecodedValue {values,_reservation:reservation})
}
