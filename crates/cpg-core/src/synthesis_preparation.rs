//! The single S0 writer publishes model-owned documentary records.
use crate::workspace::ProducerOutput;
use lctx_model::domain::{synthesis::documentary::Output, *};
pub async fn publish_documentary(
    output: &mut ProducerOutput,
    rows: &Output,
) -> Result<(), ModelError> {
    output.declare::<synthesis::documentary::DocumentaryConclusion>()?;
    output.declare::<synthesis::documentary::DocumentaryBoundary>()?;
    output.declare::<synthesis::documentary_templates::ComponentBoundary>()?;
    output.declare::<synthesis::documentary::ProseSlice>()?;
    output.declare::<synthesis::documentary::ProseSource>()?;
    output.declare::<synthesis::documentary::DocumentarySource>()?;
    output.declare::<assertion::AssertionQualification>()?;
    for row in rows.sources.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.prose_sources.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.slices.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.qualifications.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.conclusions.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.component_boundaries.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.boundaries.iter() {
        output.push(row.clone()).await?;
    }
    Ok(())
}
