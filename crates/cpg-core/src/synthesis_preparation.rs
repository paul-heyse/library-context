//! The single S0 writer publishes model-owned documentary records.
use crate::{
    producer_operations::{self, Declaration},
    workspace::ProducerOutput,
};
use futures::future::BoxFuture;
use lctx_model::domain::{synthesis::documentary::Output, *};
pub fn publish_documentary<'a>(
    output: &'a mut ProducerOutput,
    rows: &'a Output,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        const DECLARATIONS: &[Declaration] = &[
            producer_operations::declare::<synthesis::documentary::DocumentaryConclusion>,
            producer_operations::declare::<synthesis::documentary::DocumentaryBoundary>,
            producer_operations::declare::<synthesis::documentary_templates::ComponentBoundary>,
            producer_operations::declare::<synthesis::documentary::ProseSlice>,
            producer_operations::declare::<synthesis::documentary::ProseSource>,
            producer_operations::declare::<synthesis::documentary::DocumentarySource>,
            producer_operations::declare::<assertion::AssertionQualification>,
        ];
        producer_operations::declare_ordered(output, DECLARATIONS).await?;
        publish_documentary_grain(output, rows).await
    })
}
/// Append one selected documentary grain after the producer has declared its outputs.
pub(crate) fn publish_documentary_grain<'a>(
    output: &'a ProducerOutput,
    rows: &'a Output,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit =
            for<'a> fn(&'a Output, &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(
            sources,
            prose_sources,
            slices,
            qualifications,
            conclusions,
            component_boundaries,
            boundaries
        );
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
