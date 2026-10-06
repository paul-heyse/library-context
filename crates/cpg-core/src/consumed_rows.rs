//! Explicit completed-input dispatch for pure kernels. There are no persisted grants or epochs.
use crate::workspace::CompletedInputs;
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::sources::CompletedInput, charged, resources::ResourceBudget, *,
};
use std::{any::TypeId, collections::BTreeSet};
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: BTreeSet<usize>,
    _charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(
        mut declarations: Vec<ValidationInput>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        declarations.sort_by_key(|input| (input.name(), input.prefix(), input.order().to_vec()));
        declarations.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        let mut charge = charged::StateCharge::new(budget, "completed-consumer-inputs");
        charge.grow(
            declarations
                .capacity()
                .saturating_mul(size_of::<ValidationInput>()),
        )?;
        Ok(Self {
            declarations,
            dispatched: Default::default(),
            _charge: charge,
        })
    }
    pub fn next<R: Record>(
        &mut self,
        inputs: &CompletedInputs,
    ) -> Result<Option<(ValidationInput, CompletedInput<R>)>, ModelError> {
        let Some((index, declaration)) =
            self.declarations
                .iter()
                .enumerate()
                .find(|(index, declaration)| {
                    declaration.type_id() == TypeId::of::<R>() && !self.dispatched.contains(index)
                })
        else {
            return Ok(None);
        };
        let source = inputs.read_at::<R>(declaration.prefix())?;
        self.dispatched.insert(index);
        Ok(Some((declaration.clone(), source)))
    }
    pub fn finish(self, producer: &str) -> Result<(), ModelError> {
        if let Some((_, input)) = self
            .declarations
            .iter()
            .enumerate()
            .find(|(index, _)| !self.dispatched.contains(index))
        {
            return Err(ModelError::Invalid(format!(
                "producer {producer} has no typed loader for {}",
                input.name()
            )));
        }
        Ok(())
    }
}
pub async fn stream_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
    session: &SessionContext,
    mut consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let table = inputs.table_at::<R>(declaration.prefix())?;
    let order = declaration
        .order()
        .iter()
        .map(|column| format!("\"{column}\""))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT * FROM \"{table}\"{}",
        if order.is_empty() {
            String::new()
        } else {
            format!(" ORDER BY {order}")
        }
    );
    let mut batches = crate::sql::query(session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
        consume(input, &batch)?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
#[cfg(test)]
pub(crate) fn assert_decoder_reachability(
    declarations: Vec<ValidationInput>,
    decoders: &BTreeSet<TypeId>,
) {
    let missing: Vec<_> = declarations
        .iter()
        .filter(|input| !decoders.contains(&input.type_id()))
        .map(ValidationInput::name)
        .collect();
    assert!(
        missing.is_empty(),
        "completed kernel input lacks typed decoder: {missing:?}"
    );
}
