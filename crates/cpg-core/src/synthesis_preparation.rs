//! The single S0 writer publishes model-owned documentary records.
use crate::workspace::ProducerOutput;
use lctx_model::domain::{synthesis::documentary::Output, *};
pub async fn publish_documentary(
    output: &mut ProducerOutput,
    rows: &Output,
) -> Result<(), ModelError> {
    output.declare_async::<synthesis::documentary::DocumentaryConclusion>().await?;
    output.declare_async::<synthesis::documentary::DocumentaryBoundary>().await?;
    output.declare_async::<synthesis::documentary_templates::ComponentBoundary>().await?;
    output.declare_async::<synthesis::documentary::ProseSlice>().await?;
    output.declare_async::<synthesis::documentary::ProseSource>().await?;
    output.declare_async::<synthesis::documentary::DocumentarySource>().await?;
    output.declare_async::<assertion::AssertionQualification>().await?;
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
