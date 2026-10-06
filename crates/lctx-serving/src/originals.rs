//! Exact original source addresses and captured attribution; bytes remain in the native store.
use lctx_model::domain::{*,serving::*,};
use lctx_surrealdb::batches::CanonicalBatches;
use crate::records::{rows,need,wire};
pub fn target(source:&OriginalReference)->Result<graph::Target,ModelError>{
 Ok(match source{OriginalReference::Catalog{source}=>graph::Target::Entity(graph::EntityId::of(*source)),OriginalReference::Anchor{anchor}=>graph::target_for_row(derivation::RowRef::of(*anchor))?,OriginalReference::Prose{slice}=>graph::target_for_row(derivation::RowRef::of(*slice))?,OriginalReference::Artifact{artifact}=>graph::Target::Entity(graph::EntityId::of(*artifact)),OriginalReference::Occurrence{occurrence}=>graph::Target::Entity(graph::EntityId::of(*occurrence)),OriginalReference::Span{span}=>graph::Target::Entity(graph::EntityId::of(span.id()))})
}
pub fn range(data:&CanonicalBatches,source:&OriginalReference,context:Option<Id<attribution::AnalysisContext>>,release:Option<Id<input::Release>>)->Result<OriginalRange,ModelError>{
 let artifacts=rows::<source::SourceArtifact>(data)?;let occurrences=rows::<source::Occurrence>(data)?;let evidence=rows::<assertion::Evidence>(data)?;
 let mut original=source.clone();let mut chosen_context=context;
 if let OriginalReference::Anchor{anchor}=original {
  let anchors=rows::<retrieval::OriginalAnchor>(data)?;let a=need(&anchors,anchor)?;
  let units=rows::<retrieval::Unit>(data)?;let unit=need(&units,a.unit)?;chosen_context=Some(unit.context);
  let sources=rows::<retrieval::AnchorSource>(data)?;
  original=match need(&sources,a.original)?{retrieval::AnchorSource::Original{source}=>OriginalReference::Catalog{source:*source},retrieval::AnchorSource::Span{span}=>OriginalReference::Span{span:*span},retrieval::AnchorSource::Artifact{artifact}=>OriginalReference::Artifact{artifact:*artifact},retrieval::AnchorSource::Prose{slice}=>OriginalReference::Prose{slice:*slice}};
 }
 if let OriginalReference::Catalog{source}=original {
  let sources=rows::<catalog::evidence::OriginalSource>(data)?;
  original=match need(&sources,source)?{catalog::evidence::OriginalSource::Artifact{artifact}=>OriginalReference::Artifact{artifact:*artifact},catalog::evidence::OriginalSource::Occurrence{occurrence}=>OriginalReference::Occurrence{occurrence:*occurrence},catalog::evidence::OriginalSource::Span{span}=>OriginalReference::Span{span:*span}};
 }
 let prose=if let OriginalReference::Prose{slice}=original{Some(slice)}else{None};
 let mut encoding="raw_bytes";
 let (artifact,start,end)=if let Some(id)=prose{
  let slices=rows::<synthesis::documentary::ProseSlice>(data)?;let slice=need(&slices,id)?;let sources=rows::<synthesis::documentary::ProseSource>(data)?;
  match need(&sources,slice.source)?{
   synthesis::documentary::ProseSource::Occurrence{occurrence}=>{let o=need(&occurrences,*occurrence)?;if slice.end>o.end-o.start{return Err(ModelError::Schema("prose raw slice bounds"))}(o.source,o.start+slice.start,o.start+slice.end)},
   synthesis::documentary::ProseSource::Literal{occurrence,..}=>{let o=need(&occurrences,*occurrence)?;encoding="native_literal_utf8_slice";(o.source,o.start,o.end)},
   synthesis::documentary::ProseSource::Span{span}=>match need(&evidence,span.id())?{assertion::Evidence::SourceSpan{source,start,end}=>{if slice.end>end-start{return Err(ModelError::Schema("prose raw slice bounds"))}(*source,*start+slice.start,*start+slice.end)},_=>return Err(ModelError::Schema("prose span evidence"))}
  }
 }else{match original{
  OriginalReference::Artifact{artifact}=>{let a=need(&artifacts,artifact)?;(artifact,0,a.byte_len)},
  OriginalReference::Occurrence{occurrence}=>{let o=need(&occurrences,occurrence)?;(o.source,o.start,o.end)},
  OriginalReference::Span{span}=>match need(&evidence,span.id())?{assertion::Evidence::SourceSpan{source,start,end}=>(*source,*start,*end),_=>return Err(ModelError::Schema("source span evidence"))},_=>return Err(ModelError::Schema("original resolution"))
 }};
 let a=need(&artifacts,artifact)?;
 if start<0 || end<start || end>a.byte_len{return Err(ModelError::Schema("original byte bounds"))}
 let runs=rows::<attribution::ProviderRun>(data)?;
 let contexts=runs.iter().filter(|r|r.input==a.input).map(|r|r.context).collect::<std::collections::BTreeSet<_>>();
 let context=match chosen_context{Some(c)=>c,None=>{if contexts.len()!=1{return Err(ModelError::Conflict("original context attribution ambiguity"))}*contexts.first().expect("single context")}};
 let distributions=rows::<input::InputDistribution>(data)?;
 let corpora=rows::<input::CorpusLibrary>(data)?;
 let releases=distributions.iter().filter(|d|(d.input==a.input || corpora.iter().any(|c|c.corpus==a.input && c.library==d.input)) && d.role==input::DistributionRole::FirstParty).map(|d|d.release).collect::<std::collections::BTreeSet<_>>();
 let release=match release{Some(r)=>r,None=>{if releases.len()!=1{return Err(ModelError::Conflict("original release attribution ambiguity"))}*releases.first().expect("single release")}};
 Ok(OriginalRange{source:source.clone(),artifact,start:start as u64,end:end as u64,digest:a.content,encoding:Name::new(encoding).map_err(wire)?,release,context})
}
