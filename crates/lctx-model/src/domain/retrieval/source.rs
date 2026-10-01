//! Exact captured byte ranges. No parsing, lossy decoding or alternative source-body store.
use super::{build::{Data,invalid,need},*};
use crate::domain::{artifact::{ARTIFACT_CHUNK_BYTES,ArtifactChunkKey},assertion::Evidence,resources::{ResourceBudget,Reservation},source::*};
pub struct Text {pub value:String,_reservation:Box<dyn Reservation>}
pub fn coordinates(d:&Data,original:&AnchorSource)->Result<(Id<SourceArtifact>,i64,i64),ModelError> {
 match original {
 AnchorSource::Prose {slice}=>{let slice=need(&d.synthesis.prose_slices,*slice)?;let (artifact,start,end)=match need(&d.synthesis.prose_sources,slice.source)? {crate::domain::synthesis::documentary::ProseSource::Occurrence {occurrence}=>{let r=need(&d.source.core.occurrences,*occurrence)?;(r.source,r.start,r.end)},crate::domain::synthesis::documentary::ProseSource::Span {span}=>match need(&d.source.facts.canonical_evidence,span.id())? {Evidence::SourceSpan {source,start,end}=>(*source,*start,*end),_=>return Err(invalid("brief prose anchor is not a source span"))}};if slice.start<0||slice.end<slice.start||slice.end>end-start{return Err(invalid("brief prose slice exceeds original source"));}Ok((artifact,start+slice.start,start+slice.end))},
 AnchorSource::Artifact {artifact}=>Ok((*artifact,0,need(&d.source.core.artifacts,*artifact)?.byte_len)),
 AnchorSource::Span {span}=>match need(&d.source.facts.canonical_evidence,span.id())? {Evidence::SourceSpan {source,start,end}=>Ok((*source,*start,*end)),_=>Err(invalid("retrieval anchor is not canonical source span"))},
 AnchorSource::Original {source}=>match need(&d.evidence.original_sources,*source)? {
 c1::OriginalSource::Artifact {artifact}=>Ok((*artifact,0,need(&d.source.core.artifacts,*artifact)?.byte_len)),
 c1::OriginalSource::Occurrence {occurrence}=>{let r=need(&d.source.core.occurrences,*occurrence)?;Ok((r.source,r.start,r.end))},
 c1::OriginalSource::Span {span}=>match need(&d.source.facts.canonical_evidence,span.id())? {Evidence::SourceSpan {source,start,end}=>Ok((*source,*start,*end)),_=>Err(invalid("retrieval anchor is not canonical source span"))},
 },}
}
pub fn read(d:&Data,original:&AnchorSource,b:&ResourceBudget)->Result<Text,ModelError> {
 let (id,start,end)=coordinates(d,original)?;let artifact=need(&d.source.core.artifacts,id)?;
 if start<0 || end<start || end>artifact.byte_len {return Err(invalid("retrieval original coordinates outside artifact"));}
 let len=usize::try_from(end-start).map_err(ModelError::codec)?;let reservation=b.reserve("retrieval-original-range",len+size_of::<Text>())?;let mut bytes=Vec::with_capacity(len);let mut offset=start as usize;
 while offset<end as usize {let id=Id::of(&ArtifactChunkKey {artifact:id,ordinal:(offset/ARTIFACT_CHUNK_BYTES) as i64});let chunk=need(&d.facts.chunks,id)?;let first=offset%ARTIFACT_CHUNK_BYTES;let count=(end as usize-offset).min(ARTIFACT_CHUNK_BYTES-first);let part=chunk.body.0.get(first..first+count).ok_or_else(||invalid("retrieval original chunk range missing"))?;bytes.extend_from_slice(part);offset+=count;}
 Ok(Text {value:String::from_utf8(bytes).map_err(|_|invalid("retrieval original is not UTF-8"))?,_reservation:reservation})
}
