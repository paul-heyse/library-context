//! Ordered declarations and consuming canonical output publication.
//! Semantic inventories own dispatch; the writer retains buffering and terminal ownership.
use crate::workspace::ProducerOutput;
use futures::future::BoxFuture;
use lctx_model::domain::{ModelError, Record, normalized::Rows};

pub(crate) type Declaration =
    for<'a> fn(&'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;

pub(crate) fn declare<R: Record>(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move { output.declare_async::<R>().await })
}

pub(crate) fn declare_ordered<'a>(
    output: &'a ProducerOutput,
    declarations: &'a [Declaration],
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for declare in declarations {
            declare(output).await?;
        }
        Ok(())
    })
}

/// Consume canonical results at their final semantic use without rich row clones.
pub(crate) fn emit_owned<'a, R: Record>(
    rows: &'a mut Rows<R>,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move { output.push_rows(rows).await })
}
