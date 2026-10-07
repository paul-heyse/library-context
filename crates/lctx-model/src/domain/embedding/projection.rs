//! Deterministic representations derived from immutable full values, never fresh inference.
use super::{EmbeddingSpec, Spec, check_vector, value::{self, FullValue}};
use crate::{Domain, DomainCode};
use crate::domain::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Algorithm { PrefixL2F64F32 = 0 }

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="embedding_projection_definitions",validate=validate_definition)]
pub struct ProjectionDefinition {
    #[model(key)] pub dimensions: i64,
    #[model(key)] pub algorithm: Algorithm,
}
impl ProjectionDefinition {
    pub fn initial(spec: &Spec) -> Self {
        Self { dimensions: i64::from(spec.dimensions.min(1024)), algorithm: Algorithm::PrefixL2F64F32 }
    }
}
fn validate_definition(row: &ProjectionDefinition) -> Result<(), ModelError> {
    if !(1..=65536).contains(&row.dimensions) { return Err(ModelError::Invalid("invalid projection dimension".into())); }
    Ok(())
}

/// Independent F64 norm accumulation followed by exactly one F32 rounding per component.
pub fn project_prefix(full: &[f32], dimensions: u32) -> Result<Vec<f32>, String> {
    let prefix = full.get(..dimensions as usize).filter(|p| !p.is_empty())
        .ok_or_else(|| "projection exceeds full dimensions".to_owned())?;
    let norm = prefix.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>().sqrt();
    if !norm.is_finite() || norm == 0.0 { return Err("nonfinite or zero prefix norm".into()); }
    let projected = prefix.iter().map(|v| (f64::from(*v)/norm) as f32).collect::<Vec<_>>();
    check_vector(&projected, dimensions)?;
    Ok(projected)
}

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="embedding_projected_values",validate=validate_projection)]
pub struct ProjectedValue {
    #[model(key)] pub value: Id<FullValue>,
    #[model(key)] pub definition: Id<ProjectionDefinition>,
    pub source_digest: ContentHash,
    pub dimensions: i64,
    pub digest: ContentHash,
    pub bytes: EvidenceBytes,
}
impl ProjectedValue {
    pub fn new(full: &FullValue, policy: &ProjectionDefinition) -> Result<Self, ModelError> {
        full.validate()?; policy.validate()?;
        let dimensions = u32::try_from(policy.dimensions).map_err(ModelError::codec)?;
        let values = value::decode_vector(&full.bytes.0, full.dimensions as u32).map_err(ModelError::Invalid)?;
        let projection = project_prefix(&values, dimensions).map_err(ModelError::Invalid)?;
        Ok(Self {value:full.id(), definition:policy.id(),source_digest:full.digest,dimensions:policy.dimensions,digest:value::value_digest(&projection),bytes:EvidenceBytes(value::encode_vector(&projection))})
    }
    pub fn verify(&self, full: &FullValue, policy: &ProjectionDefinition) -> Result<(), ModelError> {
        if *self != Self::new(full,policy)? {return Err(ModelError::Invalid("projection differs from canonical full value/policy".into()));}
        Ok(())
    }
    pub fn values(&self) -> Result<Vec<f32>,ModelError> {
        self.validate()?;
        value::decode_vector(&self.bytes.0,self.dimensions as u32).map_err(ModelError::Invalid)
    }
}
fn validate_projection(row:&ProjectedValue)->Result<(),ModelError> {
    let dimensions = u32::try_from(row.dimensions).map_err(ModelError::codec)?;
    let values = value::decode_vector(&row.bytes.0,dimensions).map_err(ModelError::Invalid)?;
    check_vector(&values,dimensions).map_err(ModelError::Invalid)?;
    if value::value_digest(&values) != row.digest {return Err(ModelError::Invalid("projection byte digest mismatch".into()));}
    Ok(())
}

/// Reconstruct a full and its selected projection through the one shared value owner.
pub fn admit(encoder:&EmbeddingSpec, value:&value::AdmittedValue, policy:&ProjectionDefinition)->Result<(FullValue,ProjectedValue),ModelError> {
    let full=FullValue::new(encoder,value)?;
    let projection=ProjectedValue::new(&full,policy)?;
    Ok((full,projection))
}
