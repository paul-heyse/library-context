//! Shared union of selected original byte ranges; native original_chunk remains the byte owner.
use lctx_model::domain::{self as d,charged,resources::ResourceBudget,ModelError,Id};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
pub(crate) fn union_range(keys:&mut charged::ChargedSet<Id<d::artifact::ArtifactChunk>>,charge:&mut charged::StateCharge,artifact:Id<d::source::SourceArtifact>,start:i64,end:i64)->Result<(),ModelError>{
    if start<0 || end<start {return Err(ModelError::Schema("original demand range"));}
    if start==end{return Ok(());}
    for ordinal in start/d::artifact::ARTIFACT_CHUNK_BYTES as i64..=(end-1)/d::artifact::ARTIFACT_CHUNK_BYTES as i64 {
        keys.insert(charge,Id::of(&d::artifact::ArtifactChunkKey{artifact,ordinal}))?;
    }
    Ok(())
}
pub(crate) async fn stream_selected(session:&SessionContext,table:&str,keys:&charged::ChargedSet<Id<d::artifact::ArtifactChunk>>,budget:&ResourceBudget,mut visit:impl FnMut(&arrow_array::RecordBatch)->Result<(),ModelError>)->Result<(),ModelError>{
    let mut remaining=keys.iter();
    loop {
        let _query=budget.reserve("original-demand-query",128*128)?;
        let selected=remaining.by_ref().take(128).map(|key|format!("X'{}'",key.hex())).collect::<Vec<_>>();
        if selected.is_empty(){break;}
        let sql=format!("SELECT * FROM {table} WHERE id IN ({})",selected.join(","));
        let mut stream=crate::sql::query(session,&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {visit(&batch)?;}
    }
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn overlapping_original_demands_union_chunks_and_skip_empty_ranges(){
        let budget=ResourceBudget::fixed(1<<20).unwrap();let mut charge=charged::StateCharge::new(&budget,"original-demands-control");let mut keys=charged::ChargedSet::default();
        let artifact=serde_json::from_value(serde_json::to_value([7u8;16]).unwrap()).unwrap();let width=d::artifact::ARTIFACT_CHUNK_BYTES as i64;
        union_range(&mut keys,&mut charge,artifact,0,width+1).unwrap();union_range(&mut keys,&mut charge,artifact,width-1,width*2).unwrap();union_range(&mut keys,&mut charge,artifact,width*9,width*9).unwrap();assert_eq!(keys.len(),2);
        assert!(union_range(&mut keys,&mut charge,artifact,-1,1).is_err());
    }
}
