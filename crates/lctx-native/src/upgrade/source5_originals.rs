//! Maintenance-only adjacent-format decoder. Ordinary original readers never accept source5.
//! This pure stream translation earns a byte correspondence, not native admission or mutation
//! authority. The aggregate converter retains source ownership and journals each effect separately.
use crate::originals::{Layout,CHUNK_BYTES};
use lctx_model::domain::{ContentHash,ContentHasher,KeySink,ModelError,source::SourceArtifact,
    resources::{Reservation,ResourceBudget}};
use surrealdb::types::{Bytes,Object,RecordId,RecordIdKey,Value};

pub(crate) struct Source5Original {
    layout:Layout,
    next:u64,
    pieces:u64,
    bytes:ContentHasher,
    inventory:KeySink,
    _charge:Box<dyn Reservation>,
}
/// Retain the target row's charge through its acknowledged migration effect.
pub(crate) struct TranslatedPiece {
    pub(crate) original:RecordId,
    pub(crate) target:Value,
    _charge:Box<dyn Reservation>,
}
/// Only checked EOF constructs this correspondence. Native completion must independently
/// read the target pieces and earn its current incarnation/ownership witness.
pub(crate) struct OriginalCorrespondence {
    layout:Layout,
    pieces:u64,
    source_inventory:ContentHash,
    _charge:Box<dyn Reservation>,
}
impl OriginalCorrespondence {
    pub(crate) fn layout(&self)->&Layout{&self.layout}
    pub(crate) fn pieces(&self)->u64{self.pieces}
    pub(crate) fn source_inventory(&self)->ContentHash{self.source_inventory}
}
impl Source5Original {
    /// `source` is the independently decoded exact selected SourceArtifact. Neither a
    /// nominal anchor nor the old original header alone establishes its canonical payload.
    pub(crate) fn begin(header:&Value,source:&SourceArtifact,budget:&ResourceBudget)->Result<Self,ModelError>{
        let mut charge=budget.reserve("source5 original translation state",
            source.path.len().saturating_mul(64).saturating_add(8192))?;
        let layout=Layout::source(source)?;
        let expected=old_header(&layout);
        if header!=&expected{return Err(ModelError::Conflict("source5 original exact header/source"));}
        let mut inventory=KeySink::new("source5-original-correspondence/v1");
        inventory.part(b"header",&serde_json::to_vec(header).map_err(ModelError::codec)?);
        charge.try_resize(8192)?;
        Ok(Self{layout,next:0,pieces:0,bytes:ContentHasher::default(),inventory,_charge:charge})
    }
    pub(crate) fn layout(&self)->&Layout{&self.layout}
    pub(crate) fn next_offset(&self)->u64{self.next}
    pub(crate) fn piece(&mut self,row:&Value,budget:&ResourceBudget)->Result<TranslatedPiece,ModelError>{
        // Bound admission before allocating translated bytes or serializing the source row.
        let charge=budget.reserve("source5 original translated piece",CHUNK_BYTES*16+8192)?;
        let Value::Object(object)=row else{return Err(ModelError::Schema("source5 original piece object"));};
        let Some(Value::Number(surrealdb::types::Number::Int(start)))=object.get("start") else{
            return Err(ModelError::Schema("source5 original piece start"));
        };
        let start=u64::try_from(*start).map_err(ModelError::codec)?;
        let Some(Value::Bytes(bytes))=object.get("bytes") else{return Err(ModelError::Schema("source5 original piece bytes"));};
        if start!=self.next || bytes.len()!=self.layout.blob.piece_len(start)? {
            return Err(ModelError::Conflict("source5 original continuity/piece length"));
        }
        let original=old_piece_id(&self.layout.id,start)?;
        let mut expected=Object::new();expected.insert("id",original.clone());
        expected.insert("source",self.layout.id.clone());expected.insert("start",start as i64);
        expected.insert("bytes",Bytes::from(bytes.to_vec()));expected.insert("content",ContentHash::of(bytes).hex());
        if row!=&Value::Object(expected){return Err(ModelError::Conflict("source5 original complete piece envelope"));}
        let next=self.next.checked_add(bytes.len() as u64).ok_or(ModelError::Schema("source5 original offset overflow"))?;
        let count=self.pieces.checked_add(1).ok_or(ModelError::Schema("source5 original count overflow"))?;
        let target=self.layout.blob.piece(start,bytes)?;
        // No fallible step follows state advancement, so a rejected piece never consumes an offset.
        let encoded=serde_json::to_vec(row).map_err(ModelError::codec)?;
        self.inventory.part(b"piece",&encoded);self.bytes.update(bytes);
        self.next=next;self.pieces=count;
        Ok(TranslatedPiece{original,target,_charge:charge})
    }
    /// Call only after the maintenance source cursor reaches actual checked EOF, including
    /// zero-piece originals. An arbitrary prefix or equal piece count is insufficient.
    pub(crate) fn finish(mut self)->Result<OriginalCorrespondence,ModelError>{
        if self.next!=self.layout.blob.byte_len || self.pieces!=self.layout.blob.piece_count()
            || self.bytes.finish()!=self.layout.blob.content {
            return Err(ModelError::Conflict("source5 original full EOF/content"));
        }
        self.inventory.part(b"pieces",&self.pieces.to_le_bytes());
        Ok(OriginalCorrespondence{layout:self.layout,pieces:self.pieces,
            source_inventory:self.inventory.finish(),_charge:self._charge})
    }
}
fn old_header(layout:&Layout)->Value{
    let mut row=Object::new();row.insert("id",layout.id.clone());row.insert("content",layout.blob.content.hex());
    row.insert("byte_len",layout.blob.byte_len as i64);Value::Object(row)
}
fn old_piece_id(source:&RecordId,start:u64)->Result<RecordId,ModelError>{
    let RecordIdKey::String(key)=&source.key else{return Err(ModelError::Schema("source5 original source key"));};
    if source.table.as_str()!="original" || key.len()!=64 || start>i64::MAX as u64 {
        return Err(ModelError::Schema("source5 original source address"));
    }
    Ok(RecordId::new("original_chunk",format!("{key}_{start}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::originals::Blob;
    use lctx_model::domain::Id;
    fn budget()->ResourceBudget{ResourceBudget::fixed(8<<20).unwrap()}
    fn source(path:&str,bytes:&[u8])->SourceArtifact{
        let input=serde_json::from_value::<Id<lctx_model::domain::input::InputRevision>>(
            serde_json::to_value([7u8;16]).unwrap()).unwrap();
        SourceArtifact::from_bytes(input,path.into(),bytes).unwrap()
    }
    fn old_piece(layout:&Layout,start:u64,bytes:&[u8])->Value{
        let mut row=Object::new();row.insert("id",old_piece_id(&layout.id,start).unwrap());
        row.insert("source",layout.id.clone());row.insert("start",start as i64);
        row.insert("bytes",Bytes::from(bytes.to_vec()));row.insert("content",ContentHash::of(bytes).hex());Value::Object(row)
    }
    fn translate(source:&SourceArtifact,body:&[u8])->OriginalCorrespondence{
        let budget=budget();let layout=Layout::source(source).unwrap();
        let mut stream=Source5Original::begin(&old_header(&layout),source,&budget).unwrap();
        for (index,bytes) in body.chunks(CHUNK_BYTES).enumerate(){
            let row=old_piece(&layout,(index*CHUNK_BYTES) as u64,bytes);
            let piece=stream.piece(&row,&budget).unwrap();
            crate::originals::decode_piece(piece.target.clone(),&layout.blob).unwrap();
            assert_ne!(piece.original,match piece.target {Value::Object(row)=>match row.get("id").unwrap(){Value::RecordId(id)=>id.clone(),_=>panic!()},_=>panic!()});
        }
        stream.finish().unwrap()
    }
    #[test]
    fn different_artifacts_share_only_verified_full_bytes_and_zero_has_real_eof(){
        let body=vec![23;CHUNK_BYTES*3+17];let a=translate(&source("a.py",&body),&body);
        let b=translate(&source("b.py",&body),&body);
        assert_ne!(a.layout().id,b.layout().id);assert_eq!(a.layout().blob.id(),b.layout().blob.id());
        assert_ne!(a.source_inventory(),b.source_inventory());assert_eq!(a.pieces(),4);
        let empty=translate(&source("empty.py",b""),b"");assert_eq!(empty.pieces(),0);
        assert_eq!(empty.layout().blob,Blob::new(ContentHash::of(b""),0).unwrap());
    }
    #[test]
    fn prefix_substitution_extra_and_foreign_source_refuse_without_consuming_position(){
        let budget=budget();let body=vec![9;CHUNK_BYTES+3];let source=source("a.py",&body);
        let layout=Layout::source(&source).unwrap();
        let mut stream=Source5Original::begin(&old_header(&layout),&source,&budget).unwrap();
        let good=old_piece(&layout,0,&body[..CHUNK_BYTES]);
        let mut changed=good.clone();if let Value::Object(row)=&mut changed {row.insert("source",RecordId::new("original","foreign"));}
        assert!(stream.piece(&changed,&budget).is_err());assert_eq!(stream.next_offset(),0);
        stream.piece(&good,&budget).unwrap();assert!(stream.finish().is_err());
        let mut stream=Source5Original::begin(&old_header(&layout),&source,&budget).unwrap();
        stream.piece(&good,&budget).unwrap();let wrong=old_piece(&layout,CHUNK_BYTES as u64,&[8;3]);
        stream.piece(&wrong,&budget).unwrap();assert!(stream.finish().is_err(),"piece digests alone cannot establish full content");
        let mut stream=Source5Original::begin(&old_header(&layout),&source,&budget).unwrap();
        stream.piece(&good,&budget).unwrap();stream.piece(&old_piece(&layout,CHUNK_BYTES as u64,&body[CHUNK_BYTES..]),&budget).unwrap();
        assert!(stream.piece(&old_piece(&layout,(CHUNK_BYTES*2) as u64,b"extra"),&budget).is_err());
        stream.finish().unwrap();
    }
}
