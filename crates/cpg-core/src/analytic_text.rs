//! Analytic source text is a pure, single-owner publication before vector consumption.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::stages::ProviderOutcome;
use lctx_model::domain::{
    embedding::text::{self, TextAssessment, TextData, TextDefinition, TextSubject, TextWindow},
    normalized::Rows,
    *,
};
use std::sync::Arc;

async fn load<R: Record>(session: &SessionContext, rows: &mut Rows<R>) -> Result<(), ModelError> {
    let query = crate::sql::query(session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        rows.decode(&batch)?;
    }
    Ok(())
}
pub async fn publish(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: TextDefinition,
) -> Result<(), ModelError> {
    let session = access.session(runtime).await?;
    let mut data = TextData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(
        load(&session,&mut data.$field).await?;
    )*};}
    lctx_model::analytic_text_inputs!(read);
    drop(session);
    let budget = runtime.budget().clone();
    let rows = tokio::task::spawn_blocking(move || text::prepare(&data, &definition, &budget))
        .await
        .map_err(ModelError::codec)??;
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
