//! Attempt-owned compact documentary outputs. Original rich inputs are never retained here.
use lctx_model::domain::{self as d, Record, ModelError, normalized::Rows, resources::ResourceBudget,
    charged::StateCharge, synthesis::documentary::Output};
use datafusion::arrow::ipc::{reader::StreamReader, writer::StreamWriter};
use std::{collections::BTreeMap, fs::File, io::{Read, Seek}, path::PathBuf};

struct Segment {relation:&'static str,start:u64,bytes:u64}
pub(crate) struct DocumentarySpool {
    file:File,
    _directory:tempfile::TempDir,
    members:BTreeMap<d::Id<d::catalog::CatalogMemberInvocation>,Vec<Segment>>,
    charge:StateCharge,
    budget:ResourceBudget,
}
impl DocumentarySpool {
    pub(crate) fn new(budget:&ResourceBudget)->Result<Self,ModelError>{
        let directory=tempfile::tempdir().map_err(ModelError::codec)?;
        let path:PathBuf=directory.path().join("documentary-results.arrow-streams");
        let file=std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(path).map_err(ModelError::codec)?;
        Ok(Self{file,_directory:directory,members:BTreeMap::new(),charge:StateCharge::new(budget,"synthesis-documentary-spool-index"),budget:budget.clone()})
    }
    pub(crate) fn append(&mut self,id:d::Id<d::catalog::CatalogMemberInvocation>,output:&Output)->Result<(),ModelError>{
        if self.members.contains_key(&id){return Err(ModelError::Conflict("duplicate documentary spool member"));}
        self.charge.grow(128+7*size_of::<Segment>())?;
        let mut segments=Vec::with_capacity(7);
        macro_rules! write {($($field:ident:$ty:ty),*)=>{$(if let Some(segment)=self.write::<$ty>(&output.$field)?{segments.push(segment);})*};}
        write!(sources:d::synthesis::documentary::DocumentarySource,
            prose_sources:d::synthesis::documentary::ProseSource,
            slices:d::synthesis::documentary::ProseSlice,
            qualifications:d::assertion::AssertionQualification,
            conclusions:d::synthesis::documentary::DocumentaryConclusion,
            component_boundaries:d::synthesis::documentary_templates::ComponentBoundary,
            boundaries:d::synthesis::documentary::DocumentaryBoundary);
        self.members.insert(id,segments);
        Ok(())
    }
    fn write<R:Record>(&mut self,rows:&Rows<R>)->Result<Option<Segment>,ModelError>{
        if rows.is_empty(){return Ok(None);}
        self.file.seek(std::io::SeekFrom::End(0)).map_err(ModelError::codec)?;
        let start=self.file.stream_position().map_err(ModelError::codec)?;
        let mut writer=StreamWriter::try_new(&mut self.file,&R::schema()).map_err(ModelError::codec)?;
        let mut transfer=StateCharge::new(&self.budget,"documentary-spool-transfer");
        let mut pending=Vec::new();let mut bytes=0;
        for row in rows.iter(){
            if !pending.is_empty() && (pending.len()>=4096 || bytes+row.row_bytes()>d::resources::TRANSFER_BYTES){
                writer.write(&R::encode(&pending)?).map_err(ModelError::codec)?;
                pending=Vec::new();transfer.release(transfer.reserved());bytes=0;
            }
            // Typed clone, Arrow encoding and writer scratch exist together in this bounded window.
            transfer.grow(row.row_bytes().saturating_mul(3)+size_of::<R>()+128)?;
            bytes+=row.row_bytes();pending.push(row.clone());
        }
        if !pending.is_empty(){writer.write(&R::encode(&pending)?).map_err(ModelError::codec)?;}
        writer.finish().map_err(ModelError::codec)?;drop(writer);
        let end=self.file.stream_position().map_err(ModelError::codec)?;
        Ok(Some(Segment{relation:R::NAME,start,bytes:end-start}))
    }
    pub(crate) fn read(&mut self,id:d::Id<d::catalog::CatalogMemberInvocation>)->Result<Output,ModelError>{
        let segments=self.members.get(&id).ok_or(ModelError::Schema("documentary spool member absent"))?;
        let mut output=Output::new(&self.budget);
        for segment in segments{
            self.file.seek(std::io::SeekFrom::Start(segment.start)).map_err(ModelError::codec)?;
            let file=self.file.try_clone().map_err(ModelError::codec)?.take(segment.bytes);
            let reader=StreamReader::try_new(file,None).map_err(ModelError::codec)?;
            for batch in reader{
                let batch=batch.map_err(ModelError::codec)?;
                let _transfer=self.budget.reserve("documentary-spool-decode",d::logical_batch_bytes(&batch)?.saturating_mul(2))?;
                if !output.visit(segment.relation,&batch)?{return Err(ModelError::Schema("documentary spool relation"));}
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::synthesis::documentary::{DocumentaryBoundary,DocumentaryBoundaryReason};
    fn member(byte:u8)->d::Id<d::catalog::CatalogMemberInvocation>{serde_json::from_value(serde_json::to_value([byte;16]).unwrap()).unwrap()}
    #[test]
    fn compact_spool_preserves_empty_and_multiple_selected_member_results() {
        let budget=ResourceBudget::fixed(1<<20).unwrap();
        let mut spool=DocumentarySpool::new(&budget).unwrap();
        let empty=Output::new(&budget);
        spool.append(member(1),&empty).unwrap();
        let mut output=Output::new(&budget);
        output.boundaries.insert(DocumentaryBoundary{member:member(2),candidate:None,association:None,reason:DocumentaryBoundaryReason::NoDocstring}).unwrap();
        spool.append(member(2),&output).unwrap();
        assert!(spool.read(member(1)).unwrap().boundaries.is_empty());
        let restored=spool.read(member(2)).unwrap();assert!(restored.boundaries.same(&output.boundaries));
        assert!(spool.append(member(2),&output).is_err());
        assert!(spool.read(member(3)).is_err());
    }
}
