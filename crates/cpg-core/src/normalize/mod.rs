//! Computed normalization stages over admitted completed-stage inputs.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::{
        Rows,
        entity_normalization::{self, EntityData},
    },
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;

/// One transfer-bounded stream at a time. The typed collector admits every retained row; the
/// source-bound session and PreparedQuery own remote scan admission and stream lifetime.
async fn load<R: Record>(session: &StageSession, rows: &mut Rows<R>) -> Result<(), ModelError> {
    let query = session
        .query(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        rows.decode(&batch)?;
    }
    Ok(())
}
pub async fn entities(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = EntityData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        let permit = access.read::<$ty>()?;
        session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::normalized_entity_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), |data, budget| {
        entity_normalization::normalize(data.inputs(), budget)
    })
    .await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?;
        for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_entity_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn relations(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::relation_normalization::{self, RelationData};
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = RelationData::new(runtime.budget());
    macro_rules! read_facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.facts.$field).await?;
    )* }; }
    lctx_model::normalized_entity_inputs!(read_facts);
    macro_rules! read_entities { ($($field:ident: $ty:ty,)*) => { $(
        let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.entities.$field).await?;
    )* }; }
    lctx_model::normalized_entity_outputs!(read_entities);
    macro_rules! read_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        if access.stage().reads::<$ty>() {
            let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_relation_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), relation_normalization::normalize).await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_relation_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn callables(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::callable_normalization::{self, CallableData};
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = CallableData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::normalized_callable_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), callable_normalization::normalize).await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_callable_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn events(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::event_normalization::{self, EventData};
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = EventData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        if access.stage().reads::<$ty>() {
            let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_event_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), event_normalization::normalize).await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_event_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn bindings(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::binding_normalization::{self, BindingData};
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = BindingData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        if access.stage().reads::<$ty>() {
            let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_binding_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), binding_normalization::normalize).await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_binding_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

/// Generation-local computational snapshots are built once, after their canonical inputs finish.
pub async fn projections(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::projection::normalization::{self, ProjectionData};
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = ProjectionData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        let permit = access.read::<$ty>()?; session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::projection_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = compute(data, runtime.budget(), normalization::normalize).await?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::projection_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

mod pipeline;
pub use pipeline::{publish, schedule};

async fn compute<D: Send + 'static, O: Send + 'static>(
    data: D,
    budget: &lctx_model::domain::resources::ResourceBudget,
    operation: impl FnOnce(&D, &lctx_model::domain::resources::ResourceBudget) -> Result<O, ModelError>
    + Send
    + 'static,
) -> Result<O, ModelError> {
    let budget = budget.clone();
    tokio::task::spawn_blocking(move || operation(&data, &budget))
        .await
        .map_err(ModelError::codec)?
}

/// Assemble exact normalized scope outcomes over the private, admitted facts checkpoint.
pub async fn coverage(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{normalized::coverage::*, source::SourceArtifact};
    let sources = access.stored_sources()?;
    let profile = access.profile();
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let evidence = reader
        .availability()
        .cloned()
        .ok_or_else(|| ModelError::Invalid("coverage writer requires facts checkpoint".into()))?;
    let session = runtime.session(&access);
    let mut artifacts = Rows::<SourceArtifact>::new(runtime.budget());
    let permit = access.read::<SourceArtifact>()?;
    session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
    load(&session, &mut artifacts).await?;
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = assemble(profile, &evidence, &artifacts, &sources, runtime.budget())?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    output.declare::<NormalizationComputation>()?;
    for row in rows.computations.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NormalizationOutputReceipt>()?;
    for row in rows.receipts.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NormalizationCoverage>()?;
    for row in rows.outcomes.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NormalizationPremise>()?;
    for row in rows.premises.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NormalizationEvidenceSet>()?;
    for row in rows.sets.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NormalizationEvidenceMember>()?;
    for row in rows.members.iter() {
        output.push(row.clone()).await?;
    }
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}
