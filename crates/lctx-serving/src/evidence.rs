//! Original byte reads and graph derivations are independent of retrieval rank.
use lctx_model::domain::{*,serving::*,serving::mappings::PacketOutput,resources::ResourceBudget};
use lctx_surrealdb::{NativeReader,batches::{CanonicalBatches,CanonicalNode},reader::target_id};
use surrealdb::types::Variables;
use crate::records::{rows,need,wire};
pub async fn get(reader:&NativeReader,source:&OriginalReference,request:&Request,channels:&ChannelState,limits:&ResourceLimits,b:&ResourceBudget)->Result<EvidencePacket,ModelError>{
 let mut fields=crate::scope::OWNED_FIELDS.to_vec();fields.extend(["artifact","source","use_","inventory","view","characterization","event","diagnostic","entry","trace","attempt","call","arguments"]);
 let data=crate::scope::hydrate_with(reader,vec![target_id(crate::originals::target(source)?)],&crate::source_evidence::inputs(),&fields,b).await?;
 let original=crate::originals::range(&data,source,None,None)?;
 let body=body(reader,&data,&original,request,channels,limits,b).await?;
 let (flow_inventory,source_characterization)=crate::source_evidence::sections(&data,&original,request,reader.handle(),channels,b).await?;
 let derivation=derivations(reader,&original,request,channels,b).await?;
 Ok(EvidencePacket{original,body,flow_inventory,source_characterization,status:analysis::policy::EvidenceStatus::StructurallyObserved,derivation})
}
async fn body(reader:&NativeReader,data:&CanonicalBatches,range:&OriginalRange,request:&Request,channels:&ChannelState,limits:&ResourceLimits,b:&ResourceBudget)->Result<EvidenceBodyPage,ModelError>{
 let binding=crate::pagination::binding(request,reader.handle(),channels,request.tool().name(),"body",None).map_err(wire)?;
 let source_key=ContentHash::of(&serde_json::to_vec(&range.source).map_err(ModelError::codec)?);
 let mut offset=0;
 if let Some(token)=&request.page().cursor.0{let bytes=hex::decode(token.as_str()).map_err(ModelError::codec)?;let c:Cursor=serde_json::from_slice(&bytes).map_err(ModelError::codec)?;if c.binding.section.as_str()=="body"{let c=Cursor::decode(token,&binding).map_err(wire)?;let CursorPosition::Original{source,byte}=c.after else{return Err(ModelError::Schema("original continuation position"))};if source!=source_key{return Err(ModelError::Schema("original continuation source"))}offset=byte;}}
 let interpreted=if let OriginalReference::Prose{slice}=range.source{
  let slices=rows::<synthesis::documentary::ProseSlice>(data)?;let slice=need(&slices,slice)?;let sources=rows::<synthesis::documentary::ProseSource>(data)?;
  if let synthesis::documentary::ProseSource::Literal{literal,..}=need(&sources,slice.source)?{let literals=rows::<value::Literal>(data)?;let value::Literal::String{value}=need(&literals,*literal)? else{return Err(ModelError::Schema("prose literal string"))};Some(value.as_bytes().get(slice.start as usize..slice.end as usize).ok_or(ModelError::Schema("prose literal bounds"))?.to_vec())}else{None}
 }else{None};
 let length=interpreted.as_ref().map_or(range.end-range.start,|v|v.len() as u64);
 if offset>length{return Err(ModelError::Schema("original continuation bounds"))}
 // JSON bytes can expand to four decimal digits plus separators; leave room for provenance.
 let cap=(limits.response_bytes(request.page().expanded)/8).min(32*1024) as usize;
 let end=(offset+cap as u64).min(length);
 let _charge=b.reserve("native-original-page",(end-offset) as usize*6)?;
 let bytes=if let Some(value)=interpreted{value[offset as usize..end as usize].to_vec()}else{reader.original_bytes(graph::EntityId::of(range.artifact),range.start+offset,(end-offset) as usize).await?};
 let continuation=if end<length{Optional(Some(Cursor{binding,after:CursorPosition::Original{source:source_key,byte:end}}.encode().map_err(wire)?))}else{Optional::default()};
 Ok(EvidenceBodyPage{start:offset,end,bytes,continuation,omitted:length-end,truncated:end<length})
}
pub async fn hit(reader:&NativeReader,unit:Id<retrieval::Unit>,domains:&[LibraryDomainPacket],b:&ResourceBudget)->Result<EvidenceHit,ModelError>{
 let inputs=EvidenceHit::binding().lowered().sources.iter().map(|r|ValidationInput::of_relation(r,&["id"])).collect::<Vec<_>>();let mut fields=crate::scope::OWNED_FIELDS.to_vec();fields.push("unit");
 let data=crate::scope::hydrate_with(reader,vec![target_id(graph::Target::Entity(graph::EntityId::of(unit)))],&inputs,&fields,b).await?;
 let units=rows::<retrieval::Unit>(&data)?;let u=need(&units,unit)?;
 let captures=domains.iter().flat_map(|d|&d.captures).filter(|c|c.release.input==u.input || c.corpora.contains(&u.input)).collect::<Vec<_>>();
 let mut originals=Vec::new();for anchor in rows::<retrieval::OriginalAnchor>(&data)?.iter().filter(|a|a.unit==unit){for c in &captures{originals.push(crate::originals::range(&data,&OriginalReference::Anchor{anchor:anchor.id()},Some(u.context),Some(c.release.release))?);}}
 let subjects=rows::<retrieval::Subject>(&data)?;let mut members=Vec::new();for link in rows::<retrieval::UnitSubject>(&data)?.iter().filter(|s|s.unit==unit){if let retrieval::Subject::Member{member}=need(&subjects,link.subject)?{members.push(*member);}}members.sort();members.dedup();
 Ok(EvidenceHit{unit,family:u.family,title:Name::new(u.title.as_str()).map_err(wire)?,originals,associated_members:members})
}
async fn derivations(reader:&NativeReader,range:&OriginalRange,request:&Request,channels:&ChannelState,b:&ResourceBudget)->Result<SectionPage<DerivationStep>,ModelError>{
 let root=target_id(crate::originals::target(&range.source)?);let mut vars=Variables::new();vars.insert("root",root);
 let nodes:Vec<CanonicalNode>=reader.query("RETURN SELECT 'assertion' AS node_kind, canonical FROM assertion WHERE id IN array::distinct(array::concat((SELECT VALUE in FROM participant WHERE out=$root),(SELECT VALUE in FROM reference WHERE out=$root)));",vars).await?;
 let _charge=b.reserve("native-derivation-packets",nodes.iter().map(|n|n.canonical.len()*2+512).sum())?;let mut values=Vec::new();
 for node in nodes{let a:graph::Assertion=serde_json::from_slice(&node.canonical).map_err(ModelError::codec)?;if let Some(d)=&a.derivation{let mut premises=Vec::new();for (ordinal,target) in d.premises.iter().enumerate(){let role=a.participants.iter().find(|p|&p.target==target).map(|p|p.field.clone().unwrap_or_else(||format!("{:?}",p.role))).unwrap_or_else(||format!("premise_{ordinal}"));premises.push(PremisePacket{role:Name::new(role).map_err(wire)?,premise:ProofReference::from_target(target.clone())?});}values.push((a.id().0,DerivationStep{source:ProofReference::from_target(graph::Target::Assertion(a.id()))?,rule:Name::new(d.rule.clone()).map_err(wire)?,conclusion:ProofReference::from_target(d.conclusion.clone().unwrap_or(graph::Target::Assertion(a.id())))?,premises}));}}
 crate::pagination::page(values,request,reader.handle(),channels,request.tool().name(),"derivation",None,Availability::Available{}).map_err(wire)
}
