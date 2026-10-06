//! Computed normalization stages over admitted completed-stage inputs.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::{
        Rows,
        entity_normalization::{self, EntityData},
    },
    stages::*,
    *,
};
use std::sync::Arc;

/// One transfer-bounded stream at a time. The typed collector admits every retained row; the
/// source-bound session and PreparedQuery own remote scan admission and stream lifetime.
async fn load<R: Record>(
    session: &datafusion::prelude::SessionContext,
    rows: &mut Rows<R>,
) -> Result<(), ModelError> {
    let query = crate::sql::query(session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        rows.decode(&batch)?;
    }
    Ok(())
}
pub async fn entities(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let session = access.session(runtime).await?;
    let mut data = EntityData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        let _permit = access.read::<$ty>()?;

        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::normalized_entity_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), |data, budget| {
        entity_normalization::normalize(data.inputs(), budget)
    })
    .await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?;
        for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_entity_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn relations(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::relation_normalization::{self, RelationData};
    let session = access.session(runtime).await?;
    let mut data = RelationData::new(runtime.budget());
    let mut registered = charged::ChargedSet::default();
    let mut registration =
        charged::StateCharge::new(runtime.budget(), "relation-input-registration");
    macro_rules! read_facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        let _permit = access.read::<$ty>()?;
        registered.insert(&mut registration, <$ty>::NAME)?;
        load(&session, &mut data.facts.$field).await?;
    )* }; }
    lctx_model::normalized_entity_inputs!(read_facts);
    macro_rules! read_entities { ($($field:ident: $ty:ty,)*) => { $(
        let _permit = access.read::<$ty>()?;
        registered.insert(&mut registration, <$ty>::NAME)?;
        load(&session, &mut data.entities.$field).await?;
    )* }; }
    lctx_model::normalized_entity_outputs!(read_entities);
    macro_rules! read_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        if access.contains::<$ty>() {
            let _permit = access.read::<$ty>()?;
            registered.insert(&mut registration, <$ty>::NAME)?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_relation_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), relation_normalization::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_relation_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn callables(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::callable_normalization::{self, CallableData};
    let session = access.session(runtime).await?;
    let mut data = CallableData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        let _permit = access.read::<$ty>()?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::normalized_callable_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), callable_normalization::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_callable_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn receivers(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::receiver::{self, ReceiverData};
    let session = access.session(runtime).await?;
    let mut data = ReceiverData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        if access.contains::<$ty>() {
            let _permit = access.read::<$ty>()?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_receiver_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), receiver::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_receiver_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn events(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::event_normalization::{self, EventData};
    let session = access.session(runtime).await?;
    let mut data = EventData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        if access.contains::<$ty>() {
            let _permit = access.read::<$ty>()?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_event_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), event_normalization::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_event_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn bindings(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::binding_normalization::{self, BindingData};
    let session = access.session(runtime).await?;
    let mut data = BindingData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        if access.contains::<$ty>() {
            let _permit = access.read::<$ty>()?;
            load(&session, &mut data.$field).await?;
        }
    )* }; }
    lctx_model::normalized_binding_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), binding_normalization::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_binding_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

/// Generation-local computational snapshots are built once, after their canonical inputs finish.
pub async fn projections(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::projection::normalization::{self, ProjectionData};
    let session = access.session(runtime).await?;
    let mut data = ProjectionData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty,)*) => { $(
        let _permit = access.read::<$ty>()?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::projection_inputs!(read_inputs);
    drop(session);
    let rows = compute(data, runtime.budget(), normalization::normalize).await?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?; for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::projection_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

pub(crate) mod pipeline;

async fn compute<D: Send + 'static, O: Send + 'static>(
    data: D,
    budget: &lctx_model::domain::resources::ResourceBudget,
    operation: impl FnOnce(&D, &lctx_model::domain::resources::ResourceBudget) -> Result<O, ModelError>
    + Send
    + 'static,
) -> Result<O, ModelError> {
    crate::stage_runtime::borrowed_cpu("normalization", || operation(&data, budget))
}

/// Assemble exact normalized scope outcomes over the private, admitted facts checkpoint.
pub async fn coverage(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{normalized::coverage::*, source::SourceArtifact};
    let sources = access.snapshots().collect::<Vec<_>>();
    let profile = access.profile();
    let evidence = runtime.facts_availability(profile)?;
    let session = access.session(runtime).await?;
    let _permit = access.read::<SourceArtifact>()?;
    let table = access.table_at::<SourceArtifact>(None)?;
    let mut artifacts = ArtifactInputIndex::new(runtime.budget());
    // Coverage retains only the primitive artifact/input correspondence. Paths and artifact
    // payload metadata are excluded by the physical projection before any model decoding.
    let mut stream = crate::sql::query(&session, &format!("SELECT id,input FROM \"{table}\" ORDER BY id"))
        .await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        runtime.cancellation().check()?;
        artifacts.ingest(&batch)?;
        tokio::task::yield_now().await;
    }
    drop(stream);
    drop(session);
    output.declare::<NormalizationComputation>()?;
    output.declare::<NormalizationOutputReceipt>()?;
    output.declare::<NormalizationCoverage>()?;
    output.declare::<NormalizationPremise>()?;
    output.declare::<NormalizationEvidenceSet>()?;
    output.declare::<NormalizationEvidenceMember>()?;
    let prepared = CoveragePreparation::new(&evidence, &artifacts, runtime.budget())?;
    for record in prepared.evidence_records() {
        runtime.cancellation().check()?;
        let (set, member) = record?;
        output.push(set).await?;
        output.push(member).await?;
    }
    for capability in Capability::ALL {
        let stage = capability.producer(profile);
        let mut computation = NormalizationComputation {
            capability,
            policy: lctx_model::domain::normalized::policy_revision(),
            producer: stage.name.into(), declaration: stage.digest(), profile: profile.name().into(),
            availability: EvidenceAvailability::NoScope,
        };
        let mut aggregate = CoverageAggregate::default();
        for (scope, context) in prepared.scopes(capability) {
            runtime.cancellation().check()?;
            let scoped = prepared.outcome(capability, scope, context)?;
            aggregate.include(scoped.availability);
            let row = NormalizationCoverage {
                computation: computation.id(), scope, context, availability: scoped.availability,
            };
            let outcome = row.id();
            output.push(row).await?;
            for premise in &scoped.premises {
                output.push(NormalizationPremise { outcome, premise: premise.id() }).await?;
            }
        }
        computation.availability = aggregate.finish(capability, profile);
        let id = computation.id();
        output.push(computation).await?;
        for relation in &stage.outputs {
            let source = sources.iter().find(|source| source.relation() == relation.name() && source.producer() == stage.name)
                .ok_or_else(|| ModelError::Frontier("normalization output has no completed producer receipt".into()))?;
            output.push(NormalizationOutputReceipt {
                computation: id, relation: relation.name().into(), rows: source.rows(), content: source.content(),
            }).await?;
        }
    }
    output.finish(ProviderOutcome::Complete).await
}
