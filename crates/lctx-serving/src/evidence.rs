//! Original byte reads and graph derivations are independent of retrieval rank.
use crate::records::{need, rows, wire};
use lctx_model::domain::{
    resources::ResourceBudget, serving::mappings::PacketOutput, serving::*, *,
};
use lctx_surrealdb::{
    NativeReader,
    batches::{CanonicalBatches, CanonicalNode},
    reader::target_id,
};
use surrealdb::types::Variables;
pub async fn get(
    reader: &NativeReader,
    source: &OriginalReference,
    request: &Request,
    channels: &ChannelState,
    limits: &ResourceLimits,
    b: &ResourceBudget,
) -> Result<EvidencePacket, ModelError> {
    let mut fields = crate::scope::OWNED_FIELDS.to_vec();
    fields.extend([
        "artifact",
        "source",
        "use_",
        "inventory",
        "view",
        "characterization",
        "event",
        "diagnostic",
        "entry",
        "trace",
        "attempt",
        "call",
        "arguments",
        "site",
        "alternative",
        "link",
        "access",
    ]);
    let data = crate::scope::hydrate_with(
        reader,
        vec![target_id(crate::originals::target(source)?)],
        &crate::source_evidence::inputs(),
        &fields,
        b,
    )
    .await?;
    let demand = request.page().evidence_demand.0.as_ref();
    let context = demand.and_then(|d| d.context.analysis.0);
    let prepared = crate::originals::Prepared::new(&data)?;
    let original = if let Some(version) = demand.and_then(|d| d.context.release.0.as_ref()) {
        // Readable version selects only a release that the actual source resolver
        // admits. Matching a hydrated release label alone does not grant attribution.
        let mut admitted = Vec::new();
        for release in rows::<input::Release>(&data)?.iter().filter(|r| r.version == version.as_str()) {
            match prepared.range(source, context, Some(release.id())) {
                Ok(range) => admitted.push(range),
                Err(ModelError::Conflict("original release outside captured input")) => {},
                Err(error) => return Err(error),
            }
        }
        if admitted.len() != 1 { return Err(ModelError::Conflict("requested original release attribution ambiguity or absence")); }
        admitted.pop().expect("one admitted requested release")
    } else {
        prepared.range(source, context, None)?
    };
    let body = body(reader, &data, &original, request, channels, limits, b).await?;
    let (flow_inventory, source_characterization) =
        crate::source_evidence::sections(&data, &original, request, reader.handle(), channels, b)
            .await?;
    let derivation = derivations(reader, &original, request, channels, b).await?;
    let artifact=need(&rows::<source::SourceArtifact>(&data)?,original.artifact)?.clone();
    let release_row=need(&rows::<input::Release>(&data)?,original.release)?.clone();
    let package=need(&rows::<input::Package>(&data)?,release_row.package)?.clone();
    let corpora=rows::<input::CorpusLibrary>(&data)?;
    let captures=rows::<input::InputDistribution>(&data)?.into_iter().filter(|d|d.release==original.release&&d.role==input::DistributionRole::FirstParty&&(d.input==artifact.input||corpora.iter().any(|c|c.corpus==artifact.input&&c.library==d.input))).map(|d|d.input).collect::<std::collections::BTreeSet<_>>();
    if captures.len()!=1 {return Err(ModelError::Conflict("evidence readable release capture"));}
    let release=ReleaseIdentity{input:*captures.first().expect("single source release capture"),release:original.release,distribution:Name::new(package.name).map_err(wire)?,version:Name::new(release_row.version).map_err(wire)?};
    let analyses=std::collections::BTreeSet::from([original.context]);
    let qids=rows::<assertion::AssertionQualification>(&data)?.into_iter().filter(|q|q.context==original.context).map(|q|q.id()).collect();
    let mut interpretation=crate::defaults::qualified(&data,analyses,qids,original.release,b)?;
    crate::defaults::read_originals(reader,&mut interpretation,request,b).await?;
    Ok(EvidencePacket {
        release,
        interpretation,
        original,
        body,
        flow_inventory,
        source_characterization,
        status: analysis::policy::EvidenceStatus::StructurallyObserved,
        derivation,
    })
}
async fn body(
    reader: &NativeReader,
    data: &CanonicalBatches,
    range: &OriginalRange,
    request: &Request,
    channels: &ChannelState,
    limits: &ResourceLimits,
    b: &ResourceBudget,
) -> Result<EvidenceBodyPage, ModelError> {
    let binding = crate::pagination::binding(
        request,
        reader.handle(),
        channels,
        request.tool().name(),
        "body",
        None,
    )
    .map_err(wire)?;
    let source_key =
        ContentHash::of(&serde_json::to_vec(&range.source).map_err(ModelError::codec)?);
    let mut offset = 0;
    if let Some(token) = &request.page().cursor.0 {
        let bytes = hex::decode(token.as_str()).map_err(ModelError::codec)?;
        let c: Cursor = serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
        if c.binding.section.as_str() == "body" {
            let c = Cursor::decode(token, &binding).map_err(wire)?;
            let CursorPosition::Original { source, byte } = c.after else {
                return Err(ModelError::Schema("original continuation position"));
            };
            if source != source_key {
                return Err(ModelError::Schema("original continuation source"));
            }
            offset = byte;
        }
    }
    let interpreted = if let OriginalReference::Prose { slice } = range.source {
        let slices = rows::<synthesis::documentary::ProseSlice>(data)?;
        let slice = need(&slices, slice)?;
        let sources = rows::<synthesis::documentary::ProseSource>(data)?;
        if let synthesis::documentary::ProseSource::Literal { literal, .. } =
            need(&sources, slice.source)?
        {
            let literals = rows::<value::Literal>(data)?;
            let value::Literal::String { value } = need(&literals, *literal)? else {
                return Err(ModelError::Schema("prose literal string"));
            };
            Some(
                value
                    .as_bytes()
                    .get(slice.start as usize..slice.end as usize)
                    .ok_or(ModelError::Schema("prose literal bounds"))?
                    .to_vec(),
            )
        } else {
            None
        }
    } else {
        None
    };
    let length = interpreted
        .as_ref()
        .map_or(range.end - range.start, |v| v.len() as u64);
    if offset > length {
        return Err(ModelError::Schema("original continuation bounds"));
    }
    // JSON bytes can expand to four decimal digits plus separators; leave room for provenance.
    let cap = (limits.response_bytes(request.page().expanded) / 8).min(32 * 1024) as usize;
    let end = (offset + cap as u64).min(length);
    let _charge = b.reserve("native-original-page", (end - offset) as usize * 6)?;
    let bytes = if let Some(value) = interpreted {
        value[offset as usize..end as usize].to_vec()
    } else {
        reader
            .original_bytes(
                graph::EntityId::of(range.artifact),
                range.start + offset,
                (end - offset) as usize,
            )
            .await?
    };
    let continuation = if end < length {
        Optional(Some(
            Cursor {
                binding,
                after: CursorPosition::Original {
                    source: source_key,
                    byte: end,
                },
            }
            .encode()
            .map_err(wire)?,
        ))
    } else {
        Optional::default()
    };
    Ok(EvidenceBodyPage {
        start: offset,
        end,
        bytes,
        continuation,
        omitted: length - end,
        truncated: end < length,
    })
}
/// One scoped union for the retained primary witnesses. Unit-wide context anchors and
/// sibling subjects are never delivered as if they witnessed this ranked occurrence.
pub async fn hits(reader:&NativeReader, hits:&[ranking::RankedHit], domains:&[LibraryDomainPacket], b:&ResourceBudget)->Result<Vec<EvidenceHit>,ModelError>{
    if hits.is_empty(){return Ok(vec![]);}
    let inputs=EvidencePacket::binding().lowered().sources.iter().map(|r|ValidationInput::of_relation(r,&["id"])).collect::<Vec<_>>();
    let mut roots=vec![];
    for hit in hits {for witness in &hit.witnesses {
        let o=&witness.occurrence;
        roots.push(target_id(graph::Target::Entity(graph::EntityId::of(o.window))));
        roots.push(target_id(graph::Target::Entity(graph::EntityId::of(o.part))));
        if let Some(binding)=o.binding {roots.push(target_id(graph::target_for_row(derivation::RowRef::of(binding))?));}
    }}
    // Outgoing semantic dependencies remain complete; incoming ownership is window/part only.
    let data=crate::scope::hydrate_with(reader,roots,&inputs,&["window","part","set","universe"],b).await?;
    hits.iter().map(|hit|hit_packet(&data,hit,domains,b)).collect()
}
fn source_range(data:&CanonicalBatches, source:&retrieval::AnchorSource, analysis:Id<attribution::AnalysisContext>, release:Id<input::Release>)->Result<OriginalRange,ModelError>{
    let reference=match source {
        retrieval::AnchorSource::Original{source}=>OriginalReference::Catalog{source:*source},
        retrieval::AnchorSource::Span{span}|retrieval::AnchorSource::SpanSlice{span,..}=>OriginalReference::Span{span:*span},
        retrieval::AnchorSource::Artifact{artifact}=>OriginalReference::Artifact{artifact:*artifact},
        retrieval::AnchorSource::Prose{slice}=>OriginalReference::Prose{slice:*slice},
        retrieval::AnchorSource::Occurrence{occurrence}|retrieval::AnchorSource::OccurrenceSlice{occurrence,..}=>OriginalReference::Occurrence{occurrence:*occurrence},
    };
    let mut range=crate::originals::range(data,&reference,Some(analysis),Some(release))?;
    if let retrieval::AnchorSource::OccurrenceSlice{start,end,..}|retrieval::AnchorSource::SpanSlice{start,end,..}=source {
        restrict_source_range(&mut range,*start,*end)?;
    }
    Ok(range)
}
fn restrict_source_range(range:&mut OriginalRange,start:i64,end:i64)->Result<(),ModelError>{
    if start<0||end<start||(start as u64)<range.start||(end as u64)>range.end{return Err(ModelError::Schema("delivered captured source slice bounds"));}
    range.start=start as u64;range.end=end as u64;Ok(())
}
fn fragment<'a>(text:&'a str,start:i64,end:i64)->Result<&'a str,ModelError>{
    if start<0||end<=start{return Err(ModelError::Schema("delivered window fragment bounds"));}
    text.get(start as usize..end as usize).ok_or(ModelError::Schema("delivered window UTF8 fragment bounds"))
}
fn part_fragment<'a>(part:&'a retrieval::ContentPart,window_part:&retrieval::WindowPart)->Result<&'a str,ModelError>{
    if window_part.part!=part.id(){return Err(ModelError::Conflict("window part identity"));}
    fragment(part.text.as_str(),window_part.start,window_part.end)
}
fn hit_packet(data:&CanonicalBatches,hit:&ranking::RankedHit,domains:&[LibraryDomainPacket],b:&ResourceBudget)->Result<EvidenceHit,ModelError>{
    let ranking::Target::Unit{unit}=hit.target else{return Err(ModelError::Schema("evidence ranked target"));};
    let units=rows::<retrieval::Unit>(data)?;let u=need(&units,unit)?;
    if u.context!=hit.context{return Err(ModelError::Conflict("evidence hit analysis"));}
    let captures=domains.iter().flat_map(|d|&d.captures).filter(|c|c.release.input==u.input||c.corpora.contains(&u.input)).collect::<Vec<_>>();
    let releases=captures.iter().map(|c|c.release.release).collect::<std::collections::BTreeSet<_>>();
    if releases.len()!=1{return Err(ModelError::Conflict("window release capture ambiguity"));}
    let release=*releases.first().expect("one window release");
    let windows=rows::<retrieval::SearchWindow>(data)?;let parts=rows::<retrieval::ContentPart>(data)?;let window_parts=rows::<retrieval::WindowPart>(data)?;
    let bindings=rows::<retrieval::WindowBinding>(data)?;let subjects=rows::<retrieval::Subject>(data)?;let maps=rows::<retrieval::PartSourceMap>(data)?;let anchors=rows::<retrieval::AnchorSource>(data)?;
    let mut delivered=vec![];let mut originals=vec![];let mut members=std::collections::BTreeSet::new();let mut qids=std::collections::BTreeSet::new();let mut seen=std::collections::BTreeSet::new();
    for witness in &hit.witnesses {
        let o=&witness.occurrence;
        if o.unit!=unit||o.context!=hit.context||o.target!=hit.target {return Err(ModelError::Conflict("ranked window witness target/context"));}
        let window=need(&windows,o.window)?;let primary=need(&parts,o.part)?;
        if window.unit!=unit||primary.unit!=unit||primary.purpose!=retrieval::PartPurpose::Primary{return Err(ModelError::Conflict("context part cannot be primary evidence"));}
        if !window_parts.iter().any(|p|p.window==o.window&&p.part==o.part){return Err(ModelError::Schema("ranked part absent from window"));}
        for wp in window_parts.iter().filter(|p|p.window==o.window) {
        let part=need(&parts,wp.part)?;
        if wp.part!=o.part&&part.purpose!=retrieval::PartPurpose::Context {continue;}
        if part.unit!=unit{return Err(ModelError::Conflict("window context part unit"));}
        let nominated=if part.purpose==retrieval::PartPurpose::Primary{o.binding}else{None};
        if !seen.insert((o.window,wp.part,nominated)){continue;}
        if let Some(q)=part.qualification{qids.insert(q);}
        let text=part_fragment(part,wp)?;
        let binding=nominated.map(|id|need(&bindings,id)).transpose()?;
        if binding.is_some_and(|v|v.window!=o.window||v.part!=o.part){return Err(ModelError::Conflict("ranked window binding"));}
        if let Some(binding)=binding {if let retrieval::Subject::Member{member}=need(&subjects,binding.subject)? {members.insert(*member);}if let Some(q)=binding.qualification{qids.insert(q);}}
        let member=if let Some(binding)=binding {if let retrieval::Subject::Member{member}=need(&subjects,binding.subject)?{Some(*member)}else{None}}else{None};
        let mut source_maps=vec![];
        for map in maps.iter().filter(|m|m.part==wp.part&&m.start<wp.end&&m.end>wp.start) {
            let start=map.start.max(wp.start);let end=map.end.min(wp.end);
            fragment(part.text.as_str(),start,end)?;
            let original=match (map.original,map.original_start,map.original_end) {
                (None,None,None)=>None,
                (Some(id),Some(a),Some(z))=>{
                    if z-a!=map.end-map.start{return Err(ModelError::Schema("window source mapping length"));}
                    let mut range=source_range(data,need(&anchors,id)?,hit.context,release)?;
                    let actual_start=a+start-map.start;let actual_end=a+end-map.start;
                    if actual_start<0||actual_end<actual_start{return Err(ModelError::Schema("window source mapping bounds"));}
                    if range.encoding.as_str()!="raw_bytes" {None} else {
                        if (actual_start as u64)<range.start||(actual_end as u64)>range.end{return Err(ModelError::Schema("window source mapping source bounds"));}
                        range.start=actual_start as u64;range.end=actual_end as u64;originals.push(range.clone());Some(range)
                    }
                },
                _=>return Err(ModelError::Schema("partial original window mapping")),
            };
            source_maps.push(DeliveredWindowMap{start:(start-wp.start) as u64,end:(end-wp.start) as u64,availability:if original.is_some(){Availability::Available{}}else{Availability::Unavailable{reason:Name::new("synthetic_window_text").map_err(wire)?}},original:Nullable(original)});
        }
        delivered.push(DeliveredWindow{window:o.window,part:wp.part,purpose:part.purpose,member:Nullable(member),analysis:hit.context,binding:Nullable(nominated),subject:Nullable(binding.map(|v|v.subject)),basis:Nullable(binding.map(|v|v.basis)),qualification:Nullable(binding.and_then(|v|v.qualification).or(part.qualification)),text:Text::new(text).map_err(wire)?,source_maps});
        }
    }
    if delivered.is_empty(){return Err(ModelError::Schema("ranked evidence missing primary witnesses"));}
    let interpretation=crate::defaults::qualified(data,std::collections::BTreeSet::from([hit.context]),qids,release,b)?;
    let release_identity=captures.iter().find(|c|c.release.release==release).expect("selected captured release").release.clone();
    Ok(EvidenceHit{unit,release:release_identity,family:u.family,title:Name::new(u.title.as_str()).map_err(wire)?,originals,associated_members:members.into_iter().collect(),delivered_windows:delivered,interpretation})
}
async fn derivations(
    reader: &NativeReader,
    range: &OriginalRange,
    request: &Request,
    channels: &ChannelState,
    b: &ResourceBudget,
) -> Result<SectionPage<DerivationStep>, ModelError> {
    let root = target_id(crate::originals::target(&range.source)?);
    let mut vars = Variables::new();
    vars.insert("root", root);
    let nodes:Vec<CanonicalNode>=reader.query("RETURN SELECT 'assertion' AS node_kind, canonical FROM assertion WHERE id IN array::distinct(array::concat((SELECT VALUE in FROM participant WHERE out=$root),(SELECT VALUE in FROM reference WHERE out=$root)));",vars).await?;
    let _charge = b.reserve(
        "native-derivation-packets",
        nodes.iter().map(|n| n.canonical.len() * 2 + 512).sum(),
    )?;
    let mut values = Vec::new();
    for node in nodes {
        let a: graph::Assertion =
            serde_json::from_slice(&node.canonical).map_err(ModelError::codec)?;
        if let Some(d) = &a.derivation {
            let mut premises = Vec::new();
            for (ordinal, target) in d.premises.iter().enumerate() {
                let role = a
                    .participants
                    .iter()
                    .find(|p| &p.target == target)
                    .map(|p| p.field.clone().unwrap_or_else(|| format!("{:?}", p.role)))
                    .unwrap_or_else(|| format!("premise_{ordinal}"));
                premises.push(PremisePacket {
                    role: Name::new(role).map_err(wire)?,
                    premise: ProofReference::from_target(target.clone())?,
                });
            }
            values.push((
                a.id().0,
                DerivationStep {
                    source: ProofReference::from_target(graph::Target::Assertion(a.id()))?,
                    rule: Name::new(d.rule.clone()).map_err(wire)?,
                    conclusion: ProofReference::from_target(
                        d.conclusion
                            .clone()
                            .unwrap_or(graph::Target::Assertion(a.id())),
                    )?,
                    premises,
                },
            ));
        }
    }
    crate::pagination::page(
        values,
        request,
        reader.handle(),
        channels,
        request.tool().name(),
        "derivation",
        None,
        Availability::Available {},
    )
    .map_err(wire)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id<T>(n:u8)->Id<T>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
    #[test]
    fn delivered_heading_slice_stays_within_captured_span(){
        let span=assertion::Evidence::SourceSpan{source:id(2),start:7,end:29};
        let range=OriginalRange{source:OriginalReference::Span{span:assertion::EvidenceSourceSpanId::of(&span).unwrap()},artifact:id(2),start:7,end:29,digest:ContentHash::of(b"captured source"),encoding:Name::new("raw_bytes").unwrap(),release:id(3),context:id(4)};
        let mut exact=range.clone();restrict_source_range(&mut exact,7,16).unwrap();
        assert_eq!((exact.start,exact.end),(7,16));assert_eq!(exact.source,range.source);assert_eq!((exact.release,exact.context),(range.release,range.context));
        for (start,end) in [(-1,8),(6,16),(8,30),(16,8)] {let mut invalid=range.clone();assert!(restrict_source_range(&mut invalid,start,end).is_err());assert_eq!(invalid,range);}
    }
    #[test]
    fn unicode_context_and_second_primary_use_part_local_coordinates(){
        let unit=id::<retrieval::Unit>(1);let window=id::<retrieval::SearchWindow>(2);
        let mut fragments=vec![];
        for (ordinal,purpose,text) in [(0,retrieval::PartPurpose::Context,"α setup"),(1,retrieval::PartPurpose::Primary,"a much longer first primary"),(2,retrieval::PartPurpose::Primary,"β2")] {
            let part=retrieval::ContentPart{unit,ordinal,purpose,scope:None,qualification:None,digest:ContentHash::of(text.as_bytes()),text:Utf8Text::from(text)};
            let wp=retrieval::WindowPart{window,ordinal,part:part.id(),start:0,end:text.len() as i64};
            fragments.push(part_fragment(&part,&wp).unwrap().to_owned());
        }
        let complete_input=format!("nonempty query template\n{}",fragments.join("\n"));
        assert_eq!(fragments,vec!["α setup","a much longer first primary","β2"]);
        assert_ne!(&complete_input[..fragments[2].len()],fragments[2]);
        assert!(complete_input.starts_with("nonempty query template"));
    }
    #[test]
    fn primary_fragment_checks_utf8_and_exact_bounds(){
        assert_eq!(fragment("setup α primary",6,8).unwrap(),"α");
        assert!(fragment("setup α primary",7,8).is_err());
        assert!(fragment("short",-1,4).is_err());assert!(fragment("short",0,99).is_err());assert!(fragment("short",2,2).is_err());
    }
}
