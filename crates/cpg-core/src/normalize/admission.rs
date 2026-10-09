//! Required normalized owner predicates over one candidate/advertised domain at a time.
use super::{call_scope::CallScopes, receiver_scope::ReceiverScopes};
use crate::{
    consumed_rows::{ClosureTable, identifier},
    workspace::Cancellation,
};
use arrow_array::Array;
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::{binding_normalization, event_normalization, receiver},
    *,
};
use std::any::TypeId;

fn input_table<R: Record>(
    invariant: &Invariant,
    tables: &[ClosureTable],
) -> Result<String, ModelError> {
    let index = invariant
        .inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema(R::NAME))?;
    tables
        .get(index)
        .map(|table| identifier(&table.alias))
        .ok_or(ModelError::Schema(R::NAME))
}

pub async fn validate_receivers(
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &Cancellation,
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    let scopes = ReceiverScopes::from_tables(
        invariant.inputs.clone(),
        tables.clone(),
        session,
        budget,
        model,
    )
    .await?;
    // Stored assessment roots are independent of candidate eligibility, so dishonest existing
    // non-candidate targets cannot disappear merely because the producer would skip them.
    macro_rules! roots {
        ($ty:ty) => {{
            let table = input_table::<$ty>(invariant, &tables)?;
            let mut stream =
                crate::sql::query(session, &format!("SELECT id FROM {table} ORDER BY id"))
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    .ok_or(ModelError::Schema(<$ty>::NAME))?;
                for start in (0..ids.len()).step_by(32) {
                    let end = (start + 32).min(ids.len());
                    let _window_charge =
                        budget.reserve("receiver-admission-window", (end - start) * 128 + 4096)?;
                    let table = scopes.table_for::<$ty>()?;
                    let roots = (start..end)
                        .map(|index| {
                            Ok(crate::consumed_rows::PreparedRoot {
                                table,
                                key: ids.value(index).try_into().map_err(ModelError::codec)?,
                                kind: crate::consumed_rows::PreparedRootKind::Physical,
                            })
                        })
                        .collect::<Result<Vec<_>, ModelError>>()?;
                    let selected = scopes
                        .edges()
                        .batch_with_cancellation(&roots, budget, cancellation)
                        .await?;
                    let (data, stored) = scopes
                        .load_receiver_admission(&selected, budget, cancellation)
                        .await?;
                    for partition in 0..roots.len() {
                        let inputs = super::receiver_scope::DataSelection::new(
                            &selected,
                            partition,
                            scopes.inputs(),
                            &data,
                            budget,
                        )?;
                        let outputs = super::receiver_scope::ReceiverOutputSelection::new(
                            &selected,
                            partition,
                            scopes.inputs(),
                            &stored,
                            budget,
                        )?;
                        receiver::admit_view(&inputs.view()?, &outputs.view()?, budget)?;
                    }
                }
            }
        }};
    }
    roots!(calls::CallTarget);
    roots!(receiver::ReceiverAssessment);
    Ok(())
}

pub async fn validate_events(
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &Cancellation,
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    let scopes = CallScopes::from_tables(
        invariant.inputs.clone(),
        tables.clone(),
        session,
        budget,
        false,
        true,
        model,
    )
    .await?;
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
            let _window_charge=budget.reserve("event-admission-window",ids.len()*256+4096)?;
            let mut requests=Vec::new();
            for ordinal in 0..ids.len(){
                let key:event_normalization::EventKey=(super::callable_scope::nominal(sites.value(ordinal))?,super::callable_scope::nominal(origins.value(ordinal))?,super::callable_scope::nominal(contexts.value(ordinal))?);
                if seen.insert(&mut root_charge,key)?{requests.push((<[u8;16]>::try_from(ids.value(ordinal)).map_err(ModelError::codec)?,key));}
            }
            for window in requests.chunks(32){
                let root=scopes.root_for::<$ty>()?;
                let roots=window.iter().map(|(key,_)|crate::consumed_rows::PreparedRoot{table:root,key:*key,kind:crate::consumed_rows::PreparedRootKind::Virtual}).collect::<Vec<_>>();
                let selected=scopes.edges().batch_with_cancellation(&roots,budget,cancellation).await?;
                let (data,stored)=scopes.load_event_admission(&selected,budget,cancellation).await?;
                for (partition,(_,key)) in window.iter().enumerate(){
                    let inputs=super::call_scope::EventSelection::new(&selected,partition,scopes.inputs(),&data,budget)?;
                    let outputs=super::call_scope::EventOutputSelection::new(&selected,partition,scopes.inputs(),&stored,budget)?;
                    event_normalization::admit_event_view(&inputs.view()?,&outputs.view()?,*key,budget)?;
                }
            }
        }
    }};}
    roots!(calls::ProviderCallSite, false);
    roots!(calls::CallTarget, false);
    roots!(calls::CallResolution, false);
    roots!(normalized::events::NormalizedCallEvent, true);
    if let Ok(table) = input_table::<flow::FlowValuePathObservation>(invariant, &tables) {
        let mut stream = crate::sql::query(session, &format!("SELECT * FROM {table} ORDER BY id"))
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| {
                    c.as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .ok_or(ModelError::Schema("flow admission root"))?;
            for start in (0..ids.len()).step_by(32) {
                let end = (start + 32).min(ids.len());
                let _window_charge =
                    budget.reserve("flow-admission-window", (end - start) * 128 + 4096)?;
                let root = scopes.root_for::<flow::FlowValuePathObservation>()?;
                let roots = (start..end)
                    .map(|i| {
                        Ok(crate::consumed_rows::PreparedRoot {
                            table: root,
                            key: ids.value(i).try_into().map_err(ModelError::codec)?,
                            kind: crate::consumed_rows::PreparedRootKind::Virtual,
                        })
                    })
                    .collect::<Result<Vec<_>, ModelError>>()?;
                let selected = scopes
                    .edges()
                    .batch_with_cancellation(&roots, budget, cancellation)
                    .await?;
                let (data, stored) = scopes
                    .load_event_admission(&selected, budget, cancellation)
                    .await?;
                for (partition, i) in (start..end).enumerate() {
                    let inputs = super::call_scope::EventSelection::new(
                        &selected,
                        partition,
                        scopes.inputs(),
                        &data,
                        budget,
                    )?;
                    let outputs = super::call_scope::EventOutputSelection::new(
                        &selected,
                        partition,
                        scopes.inputs(),
                        &stored,
                        budget,
                    )?;
                    event_normalization::admit_flow_path_view(
                        &inputs.view()?,
                        &outputs.view()?,
                        super::callable_scope::nominal(ids.value(i))?,
                        budget,
                    )?;
                }
            }
        }
    }
    Ok(())
}

pub async fn validate_bindings(
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &Cancellation,
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    let scopes = CallScopes::from_tables(
        invariant.inputs.clone(),
        tables.clone(),
        session,
        budget,
        true,
        true,
        model,
    )
    .await?;
    let table = input_table::<normalized::events::NormalizedCallEvent>(invariant, &tables)?;
    let mut stream = crate::sql::query(session, &format!("SELECT * FROM {table} ORDER BY id"))
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column_by_name("id")
            .and_then(|c| {
                c.as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("binding admission root"))?;
        for start in (0..ids.len()).step_by(32) {
            let end = (start + 32).min(ids.len());
            let _window_charge =
                budget.reserve("binding-admission-window", (end - start) * 128 + 4096)?;
            let root = scopes.root_for::<normalized::events::NormalizedCallEvent>()?;
            let roots = (start..end)
                .map(|i| {
                    Ok(crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: ids.value(i).try_into().map_err(ModelError::codec)?,
                        kind: crate::consumed_rows::PreparedRootKind::Virtual,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let selected = scopes
                .edges()
                .batch_with_cancellation(&roots, budget, cancellation)
                .await?;
            let (data, stored) = scopes
                .load_binding_admission(&selected, budget, cancellation)
                .await?;
            for (partition, i) in (start..end).enumerate() {
                let inputs = super::call_scope::BindingSelection::new(
                    &selected,
                    partition,
                    scopes.inputs(),
                    &data,
                    budget,
                )?;
                let outputs = super::call_scope::BindingOutputSelection::new(
                    &selected,
                    partition,
                    scopes.inputs(),
                    &stored,
                    budget,
                )?;
                binding_normalization::admit_event_view(
                    &inputs.view()?,
                    &outputs.view()?,
                    super::callable_scope::nominal(ids.value(i))?,
                    budget,
                )?;
            }
        }
    }
    Ok(())
}
