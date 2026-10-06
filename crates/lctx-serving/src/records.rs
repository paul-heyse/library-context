//! Typed request-local access over the canonical batch adapter.
use lctx_model::domain::{Record, Id, ModelError};
use lctx_surrealdb::batches::CanonicalBatches;
pub fn rows<R:Record>(source:&CanonicalBatches)->Result<Vec<R>,ModelError> {
    source.batches.iter().filter(|(name,_)|*name==R::NAME).flat_map(|(_,batch)|R::decode(batch).map(|rows|rows.into_iter().map(Ok).collect::<Vec<_>>()).unwrap_or_else(|error|vec![Err(error)])) .collect()
}
pub fn need<R:Record>(rows:&[R],id:Id<R>)->Result<&R,ModelError> {
    rows.iter().find(|row|row.id()==id).ok_or_else(||ModelError::Invalid(format!("required native packet row missing: {}",R::NAME)))
}
pub fn wire(error:lctx_model::domain::serving::WireError)->ModelError {ModelError::Invalid(error.to_string())}
