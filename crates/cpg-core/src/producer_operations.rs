//! Borrowed operation boundaries for ordered typed publication.
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

pub(crate) fn emit<'a, R: Record>(
    rows: &'a Rows<R>,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for row in rows.iter() {
            output.push(row.clone()).await?;
        }
        Ok(())
    })
}
