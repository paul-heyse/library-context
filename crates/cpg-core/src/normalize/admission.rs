//! Required normalized owner predicates over one candidate/advertised domain at a time.
use super::{receiver_scope::ReceiverScopes, call_scope::CallScopes};
use crate::{consumed_rows::{ClosureTable, PreparedClosure, identifier}, workspace::Cancellation};
use futures::TryStreamExt;
use arrow_array::Array;
use lctx_model::domain::{*, normalized::{Rows, receiver, event_normalization, binding_normalization}};
use std::any::TypeId;

async fn feed(scope: &PreparedClosure, inputs: &[ValidationInput], cancellation: &Cancellation,
    mut visit: impl FnMut(&str, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    for (table, input) in inputs.iter().enumerate() {
        let mut stream = crate::sql::query(scope.session(), &format!("{} ORDER BY id", scope.select(table)?)).await.map_err(ModelError::codec)?
            .execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            visit(input.name(), &batch)?;
        }
    }
    Ok(())
}
fn input_table<R: Record>(invariant: &Invariant, tables: &[ClosureTable]) -> Result<String, ModelError> {
    let index = invariant.inputs.iter().position(|input| input.type_id() == TypeId::of::<R>()).ok_or(ModelError::Schema(R::NAME))?;
    tables.get(index).map(|table| identifier(&table.alias)).ok_or(ModelError::Schema(R::NAME))
}

pub async fn validate_receivers(invariant: &Invariant, tables: Vec<ClosureTable>, session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget, cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let scopes = ReceiverScopes::from_tables(invariant.inputs.clone(), tables.clone(), session, budget).await?;
    // Stored assessment roots are independent of candidate eligibility, so dishonest existing
    // non-candidate targets cannot disappear merely because the producer would skip them.
    macro_rules! roots {($ty:ty) => {{
        let table = input_table::<$ty>(invariant, &tables)?;
        let mut stream = crate::sql::query(session, &format!("SELECT id FROM {table} ORDER BY id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let ids = batch.column(0).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or(ModelError::Schema(<$ty>::NAME))?;
            for ordinal in 0..ids.len() {
                cancellation.check()?;
                let scope = scopes.root_grain(TypeId::of::<$ty>(), ids.value(ordinal), budget).await?;
                let mut data = receiver::ReceiverData::new(budget);
                let mut stored = receiver::ReceiverOutput::new(budget);
                feed(&scope, scopes.inputs(), cancellation, |name, batch| {
                    if !data.visit(name, batch)? && !stored.visit(name, batch)? { return Err(ModelError::Schema("receiver admission input")); }
                    Ok(())
                }).await?;
                receiver::admit(&data, &stored, budget)?;
                tokio::task::yield_now().await;
            }
        }
    }};}
    roots!(calls::CallTarget); roots!(receiver::ReceiverAssessment);
    Ok(())
}

pub async fn validate_events(invariant: &Invariant, tables: Vec<ClosureTable>, session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget, cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let scopes = CallScopes::from_tables(invariant.inputs.clone(), tables.clone(), session, budget, false, true).await?;
    let qualifications = input_table::<assertion::AssertionQualification>(invariant, &tables)?;
    let mut seen = charged::ChargedSet::default();
    let mut root_charge = charged::StateCharge::new(budget, "event-admission-root-keys");
    macro_rules! roots {($ty:ty, $stored:expr) => {{
        let table = input_table::<$ty>(invariant, &tables)?;
        let sql = if $stored { format!("SELECT r.id,r.site,r.origin,r.context FROM {table} r ORDER BY r.id") }
            else { format!("SELECT r.id,r.site,r.origin,q.context FROM {table} r JOIN {qualifications} q ON q.id=r.qualification ORDER BY r.id") };
        let mut stream = crate::sql::query(session, &sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let binary = |column: usize| batch.column(column).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or(ModelError::Schema(<$ty>::NAME));
            let ids = binary(0)?; let sites = binary(1)?; let origins = binary(2)?; let contexts = binary(3)?;
            for ordinal in 0..ids.len() {
                cancellation.check()?;
                let site = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(sites.value(ordinal).iter().copied())).map_err(ModelError::codec)?;
                let origin = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(origins.value(ordinal).iter().copied())).map_err(ModelError::codec)?;
                let context = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(contexts.value(ordinal).iter().copied())).map_err(ModelError::codec)?;
                let key: event_normalization::EventKey = (site,origin,context);
                if !seen.insert(&mut root_charge, key)? { continue; }
                let scope = scopes.root_grain(TypeId::of::<$ty>(), ids.value(ordinal), budget).await?;
                let mut data = event_normalization::EventData::new(budget);
                let mut stored = event_normalization::EventOutput::new(budget);
                feed(&scope, scopes.inputs(), cancellation, |name, batch| {
                    if !data.visit(name, batch)? && !stored.visit(name, batch)? { return Err(ModelError::Schema("event admission input")); }
                    Ok(())
                }).await?;
                event_normalization::admit_event(&data, &stored, key, budget)?;
                tokio::task::yield_now().await;
            }
        }
    }};}
    roots!(calls::ProviderCallSite, false); roots!(calls::CallTarget, false); roots!(calls::CallResolution, false);
    roots!(normalized::events::NormalizedCallEvent, true);
    if let Ok(table) = input_table::<flow::FlowValuePathObservation>(invariant, &tables) {
        let mut stream = crate::sql::query(session, &format!("SELECT * FROM {table} ORDER BY id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let mut roots = Rows::<flow::FlowValuePathObservation>::new(budget); roots.decode(&batch)?;
            for root in roots.iter() {
                let scope = scopes.root_grain(TypeId::of::<flow::FlowValuePathObservation>(), root.id().bytes(), budget).await?;
                let mut data = event_normalization::EventData::new(budget);
                let mut stored = event_normalization::EventOutput::new(budget);
                feed(&scope, scopes.inputs(), cancellation, |name, batch| { data.visit(name,batch)?; stored.visit(name,batch)?; Ok(()) }).await?;
                event_normalization::admit_flow_path(&data, &stored, root.id(), budget)?;
            }
        }
    }
    Ok(())
}

pub async fn validate_bindings(invariant: &Invariant, tables: Vec<ClosureTable>, session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget, cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let scopes = CallScopes::from_tables(invariant.inputs.clone(), tables.clone(), session, budget, true, true).await?;
    let table = input_table::<normalized::events::NormalizedCallEvent>(invariant, &tables)?;
    let mut stream = crate::sql::query(session, &format!("SELECT * FROM {table} ORDER BY id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let mut roots = Rows::<normalized::events::NormalizedCallEvent>::new(budget); roots.decode(&batch)?;
        for event in roots.iter() {
            cancellation.check()?;
            let scope = scopes.root_grain(TypeId::of::<normalized::events::NormalizedCallEvent>(), event.id().bytes(), budget).await?;
            let mut data = binding_normalization::BindingData::new(budget);
            let mut stored = binding_normalization::BindingOutput::new(budget);
            feed(&scope, scopes.inputs(), cancellation, |name,batch| { data.visit(name,batch)?; stored.visit(name,batch)?; Ok(()) }).await?;
            binding_normalization::admit_event(&data, &stored, event.id(), budget)?;
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}
