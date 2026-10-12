//! Whole-owner unrequested production. No per-method invocation or outcome is created.
use super::*;
use crate::workspace::CompletedInputs;
use lctx_model::domain::uniform_outcome::{AnalysisUniformInputs,UniformNotRequested};

pub(super) fn unrequested(binding:UpperStage,profile:Profile,prepared:&PreparedCompilation)->bool{
    match binding {
        UpperStage::Local|UpperStage::Base|UpperStage::Completion|UpperStage::SourceCalls
        |UpperStage::Enriched|UpperStage::Models|UpperStage::Summary=>profile==Profile::Catalog,
        UpperStage::Embedding=>!prepared.text.requested,
        UpperStage::Analytic=>!analytics::build::requested(prepared.settings()),
        _=>false,
    }
}
pub(super) async fn produce(access:CompletedInputs,output:ProducerOutput,workspace:&Workspace,
    declaration:&Stage)->Result<(),ModelError>{
    let mut inputs=AnalysisUniformInputs::new(access.profile(),workspace.budget());
    let required=AnalysisUniformInputs::inputs();
    let mut seen=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(workspace.budget(),"compact owner selected metadata");
    for source in access.relations(){
        if source.name()==UniformNotRequested::NAME || !required.iter().any(|input|input.name()==source.name())
            || seen.contains(&source.view_identity()){continue;}
        let mut rows=source.batches_async().await?;let mut count=0u64;
        while let Some(batch)=rows.next() {
            let batch=batch?;
            workspace.cancellation().check()?;
            count=count.checked_add(batch.num_rows() as u64).ok_or(ModelError::Conflict("compact owner metadata count overflow"))?;
            if count>source.rows(){return Err(ModelError::Conflict("compact owner metadata exceeds actual view"));}
            inputs.visit(source.name(),&batch)?;
        }
        if count!=source.rows(){return Err(ModelError::Conflict("compact owner metadata exact EOF count"));}
        seen.insert(&mut charge,source.view_identity())?;
    }
    let mut sources=charged::ChargedVec::default();let mut source_charge=charged::StateCharge::new(workspace.budget(),"compact owner ordered source snapshots");
    for source in access.snapshots(){sources.push(&mut source_charge,source)?;}
    let rows=inputs.declarations(declaration,&sources)?;
    output.declare_async::<UniformNotRequested>().await?;
    output.declare_empty_outputs().await?;
    for row in rows.iter(){output.push(row.clone()).await?;}
    // Production completed the exact compact declarations and every declared empty output.
    // Semantic NotRequested belongs to the admitted domains, not the physical disposition.
    output.finish(ProviderOutcome::Complete).await
}
