//! Analytic source text is a pure, single-owner publication before vector consumption.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    embedding::text::{self, TextAssessment, TextData, TextDefinition, TextSubject, TextWindow},
    normalized::Rows,
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;

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
pub async fn publish(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: TextDefinition,
) -> Result<(), ModelError> {
    let declared = text::stage(access.profile(), &definition, model, access.publication_order())?;
    if access.stage().name != declared.name
        || access.stage().configuration != declared.configuration
        || access.stage().code != declared.code
    {
        return Err(ModelError::Invalid(
            "analytic text configuration differs from preflight".into(),
        ));
    }
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
    let mut data = TextData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(
        let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session,&mut data.$field).await?;
    )*};}
    lctx_model::analytic_text_inputs!(read);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let budget = runtime.budget().clone();
    let rows = tokio::task::spawn_blocking(move || text::prepare(&data, &definition, &budget))
        .await
        .map_err(ModelError::codec)??;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write {
        ($ty:ty,$field:ident) => {
            output.declare::<$ty>()?;
            for row in rows.$field.iter() {
                output.push(row.clone()).await?;
            }
        };
    }
    write!(TextDefinition, definitions);
    write!(TextSubject, subjects);
    write!(TextAssessment, assessments);
    write!(TextWindow, windows);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}
