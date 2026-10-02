//! Invocation-local consumed declarations and typed source-bound batch streaming.
use crate::{generation_read::AttemptSession, model_runtime::StageSession};
use futures::TryStreamExt;
use lctx_model::domain::{charged, resources::ResourceBudget, stages::*, *};
use std::{any::TypeId, collections::BTreeMap};

/// Resolve each owned declaration once and coalesce aliases only when the acknowledged source
/// is identical. The same relation at distinct immutable epochs remains two consumed sources.
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: charged::ChargedSet<(&'static str, Option<u8>)>,
    sources: BTreeMap<(&'static str, Option<u16>), CompletedRelation>,
    charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(mut declarations: Vec<ValidationInput>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        declarations.sort_by_key(|i| (i.name(), i.prefix()));
        declarations.dedup_by_key(|i| (i.name(), i.prefix()));
        let mut charge = charged::StateCharge::new(budget, "consumed-inputs");
        charge.grow(declarations.capacity().saturating_mul(size_of::<ValidationInput>())
            + declarations.iter().map(|i| i.order().len().saturating_mul(size_of::<&str>())).sum::<usize>())?;
        Ok(Self { declarations, dispatched: Default::default(), sources: BTreeMap::new(), charge })
    }
    /// Typed macros provide decoder reachability; they never select the semantic input inventory.
    pub fn next<'a, R: Record>(&mut self, access: &'a StageAccess<'_, '_>) -> Result<Option<(ValidationInput, ReadPermit<'a, R>)>, ModelError> {
        loop {
            let Some(input) = self.declarations.iter().find(|i| i.type_id() == TypeId::of::<R>()
                && !self.dispatched.contains(&(i.name(), i.prefix().map(|e| e as u8)))) else { return Ok(None); };
            let mut input = input.clone();
            self.dispatched.insert(&mut self.charge, (input.name(), input.prefix().map(|e| e as u8)))?;
            let permit = match input.prefix() {
                Some(epoch) => access.read_at_epoch::<R>(epoch)?,
                None => access.read::<R>()?,
            };
            let source = permit.source().ok_or_else(|| ModelError::Invalid("consumed rows require an acknowledged source".into()))?;
            let key = (R::NAME, source.prefix_ordinal().map(PrefixOrdinal::ordinal));
            if let Some(previous) = self.sources.get(&key) {
                if previous != source { return Err(ModelError::Invalid("consumed input changes acknowledged source within an epoch".into())); }
                continue;
            }
            self.charge.grow(size_of::<CompletedRelation>() + size_of::<(&str, Option<u16>)>() + 64)?;
            self.sources.insert(key, source.clone());
            if is_vocabulary(input.name()) {
                input = input.at_epoch(source.prefix().ok_or_else(|| ModelError::Invalid("consumed vocabulary has no immutable epoch".into()))?);
            }
            return Ok(Some((input, permit)));
        }
    }
    /// Refuse a typed dispatch that omitted an owned consumed declaration.
    pub fn finish(self) -> Result<(), ModelError> {
        if self.dispatched.len() != self.declarations.len() {
            return Err(ModelError::Invalid("consumed input has no typed loader".into()));
        }
        Ok(())
    }
}

/// Register an existing typed source in its stage session and stream to an explicit consumer.
/// On errors or cancellation the stream drops through the provider's existing drain contract.
/// Callers isolate sessions for distinct epochs because StageSession registers by relation name.
pub async fn stream<R: Record>(
    permit: &ReadPermit<'_, R>,
    reader: &AttemptSession,
    session: &StageSession,
    mut consume: impl FnMut(&ReadPermit<'_, R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    session.register(permit, reader.table(permit).map_err(ModelError::codec)?)?;
    let query = session.query(&format!("SELECT * FROM \"{}\"", R::NAME)).await.map_err(ModelError::codec)?;
    let mut batches = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
        consume(permit, &batch)?;
        // Ready batches must still offer cancellation and fair scheduling between consumers.
        tokio::task::yield_now().await;
    }
    Ok(())
}
