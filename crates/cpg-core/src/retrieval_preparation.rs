//! E0 constructs mandatory retrieval content from completed C1 inputs; final realization extends it.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use lctx_model::domain::{
    retrieval::build::{self, Data, Output},
    *,
};
use std::sync::Arc;
/// Read through the declared finite source closure. Original bytes come solely from ArtifactChunk.
pub async fn mandatory(
    access: &CompletedInputs,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(Data, Output), ModelError> {
    let session = access.session(runtime).await?;
    let mut data = Data::new(runtime.budget());
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        Data::mandatory_consumed_inputs(access.profile()),
        runtime.budget(),
    )?;
    macro_rules! load {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(access)?{
        crate::consumed_rows::stream_at(&permit,&input,access,&session,|_,batch|{data.visit_input(&input,batch)?;Ok(())}).await?;
    })*};}
    lctx_model::catalog_inputs!(load);
    lctx_model::catalog_outputs!(load);
    lctx_model::catalog_evidence_inputs!(load);
    lctx_model::catalog_evidence_outputs!(load);
    lctx_model::retrieval_inputs!(load);
    lctx_model::catalog_runtime_inputs!(load);
    consumed.finish("mandatory-retrieval")?;
    drop(session);
    catalog::evidence::frames::verify(
        &data.source.facts.runs,
        &data.source.facts.core_invocations,
        &data.facts.evidence_invocations,
        &data.facts.evidence_sources,
        &data.facts.evidence_inputs,
        &data.source.runtime.lower(),
        runtime.budget(),
    )?;
    if !data
        .facts
        .evidence_links
        .same(&catalog::evidence::build::invocation_links(
            &data.evidence,
            &data.facts.evidence_invocations,
            runtime.budget(),
        )?)
    {
        return Err(build::invalid(
            "retrieval C1 root invocation closure differs",
        ));
    }
    let budget = runtime.budget().clone();
    tokio::task::spawn_blocking(move || {
        let output = build::build(&data, &budget)?;
        Ok((data, output))
    })
    .await
    .map_err(ModelError::codec)?
}
/// Same publication operation used by the final E0 producer. The scoped native control declares
/// only this mandatory boundary; it makes no completed S0/realization claim.
pub async fn publish_mandatory(
    output: &mut ProducerOutput,
    rows: &Output,
) -> Result<(), ModelError> {
    macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$f.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::retrieval_outputs!(write);
    Ok(())
}
