//! Source-owned definitions and primary-evidence nomination, shared by construction/admission.
use super::*;
use build::{Data, Output, invalid, need};
use crate::domain::{normalized::entities::*, resources::ResourceBudget};

pub(super) fn definitions(d:&Data,member:Id<catalog::CatalogMember>,context:Id<AnalysisContext>)->Result<Vec<(Id<EntityRef>,Option<AnchorSource>,bool)>,ModelError>{
    let mut result=std::collections::BTreeMap::new();
    for candidate in d.source.catalog.candidates.iter(){
        let exposure=need(&d.source.catalog.exposures,candidate.exposure)?;
        if exposure.member!=member {continue;}
        let public=need(&d.source.core.exposures,exposure.exposure)?;
        if public.context!=context {continue;}
        let Some(entity)=candidate.entity else {continue;};
        let candidate=need(&d.source.core.entity_candidates,entity)?;
        let resolution=need(&d.source.core.resolutions,candidate.resolution)?;
        if resolution.context!=context{ return Err(invalid("defining-source resolution changed context")); }
        let reference=need(&d.source.core.refs,candidate.entity)?;
        let occurrence=match reference {
            EntityRef::Callable{callable}=>match need(&d.source.core.source_callables,*callable)?{CallableEntity::Source{declaration,..}=>Some(*declaration),_=>None},
            EntityRef::Class{class}=>match need(&d.source.core.source_classes,*class)?{ClassEntity::Source{declaration}=>Some(*declaration),_=>None},
            EntityRef::Occurrence{occurrence}=>Some(*occurrence),
            EntityRef::Parameter{parameter}=>match need(&d.source.core.parameters,*parameter)?{ParameterEntity::Source{declaration}=>Some(*declaration),_=>None},
            _=>None,
        };
        let anchor=if let Some(occurrence)=occurrence{Some(AnchorSource::Occurrence{occurrence})}else if let EntityRef::Module{module}=reference{Some(AnchorSource::Artifact{artifact:d.module_source(*module)?})}else{None};
        let exact=public.status==ResolutionStatus::Resolved && resolution.status==ResolutionStatus::Resolved && resolution.entity==Some(candidate.entity);
        result.entry(candidate.entity).and_modify(|(_,old):&mut(Option<AnchorSource>,bool)|*old&=exact).or_insert((anchor,exact));
    }
    Ok(result.into_iter().map(|(entity,(anchor,exact))|(entity,anchor,exact)).collect())
}
/// Supported ancestor headers and the selected branch are interpretation dependencies. Full
/// sibling bodies are not copied into the selected definition's primary evidence.
pub(super) fn enclosing(d:&Data,anchor:&AnchorSource,context:Id<AnalysisContext>)->Result<Vec<(AnchorSource,lexical::SyntaxField)>,ModelError>{
    let AnchorSource::Occurrence{occurrence}=anchor else{return Ok(vec![]);};
    let mut current=*occurrence;let mut seen=std::collections::BTreeSet::new();let mut result=vec![];
    loop {
        if !seen.insert(current){return Err(invalid("source interpretation parent cycle"));}
        let mut placements=d.source.core.placements.iter().filter(|p|p.occurrence==current&&d.source.core.qualifications.get(p.qualification).is_some_and(|q|q.context==context));
        let Some(placement)=placements.next()else{break;};
        if placements.next().is_some(){return Err(invalid("defining source parent placement is ambiguous"));}
        let Some(parent)=placement.parent else{break;};let occurrence=need(&d.source.core.occurrences,parent)?;
        if occurrence.syntax_kind==crate::domain::source::SyntaxKind::ModModule{break;}
        let first=d.source.core.placements.iter().filter(|p|p.parent==Some(parent)&&p.field==lexical::SyntaxField::Body&&d.source.core.qualifications.get(p.qualification).is_some_and(|q|q.context==context)).filter_map(|p|d.source.core.occurrences.get(p.occurrence).map(|o|o.start)).min();
        if let Some(end)=first && end>occurrence.start{result.push((AnchorSource::OccurrenceSlice{occurrence:parent,start:occurrence.start,end},placement.field));}
        current=parent;
    }
    result.reverse();Ok(result)
}
pub(super) fn option_anchors(d:&Data,row:&catalog::CatalogOption)->Result<Vec<AnchorSource>,ModelError>{
    let evidence=need(&d.source.catalog.evidence,row.evidence)?;
    let mut occurrences=vec![];
    match evidence {
        catalog::CatalogOptionEvidence::Parameter{syntax,..}|catalog::CatalogOptionEvidence::SourceParameter{syntax,..}=>{let syntax=need(&d.source.core.parameter_syntax,*syntax)?;occurrences.push(syntax.parameter);occurrences.extend(syntax.annotation);occurrences.extend(syntax.default);},
        catalog::CatalogOptionEvidence::DeclaredField{declaration,..}=>{let declaration=need(&d.source.core.field_declarations,*declaration)?;let syntax=need(&d.source.core.field_syntax,declaration.declaration)?;occurrences.push(syntax.target);occurrences.extend(syntax.annotation);occurrences.extend(syntax.value);},
        catalog::CatalogOptionEvidence::NativeParameter{..}|catalog::CatalogOptionEvidence::NativeField{..}=>{},
    };
    occurrences.sort_by_key(|id|d.source.core.occurrences.get(*id).map(|o|(o.start,o.end)));occurrences.dedup();
    Ok(occurrences.into_iter().map(|occurrence|AnchorSource::Occurrence{occurrence}).collect())
}
#[derive(Clone)]
pub(super) struct Nomination {pub subject:Id<Subject>,pub basis:BindingBasis,pub qualification:Option<Id<assertion::AssertionQualification>>,range:Option<(Id<crate::domain::source::SourceArtifact>,i64,i64)>}
pub(super) fn nominations(d:&Data,out:&Output,unit:Id<Unit>)->Result<Vec<Nomination>,ModelError>{
    let unit=need(&out.units,unit)?;
    let mut result=vec![];
    let mut push=|subject:Subject,basis,qualification,range|{result.push(Nomination{subject:subject.id(),basis,qualification,range});};
    match need(&out.origins,unit.origin)? {
        Origin::Api{..}|Origin::Brief{..}=>{
            let member=match need(&out.origins,unit.origin)?{Origin::Api{member}=>*member,_=>{
                let root=out.roots.iter().find(|r|r.unit==unit.id()).ok_or_else(||invalid("brief root absent"))?;
                match need(&d.evidence.subjects,need(&d.evidence.roots,root.root)?.subject)?{c1::RootSubject::Member{member}=>*member,_=>return Err(invalid("brief member owner absent"))}
            }};
            push(Subject::Member{member},BindingBasis::PublicContract,None,None);
        }
        Origin::Definition{member,entity}=>{
            for (candidate,anchor,exact) in definitions(d,*member,unit.context)?{
                if candidate==*entity && let Some(anchor)=anchor {
                    let range=Some(super::source::coordinates(d,&anchor)?);
                    push(Subject::Definition{entity:*entity},if exact{BindingBasis::DirectDefinition}else{BindingBasis::DefinitionCandidate},None,range);
                    push(Subject::Member{member:*member},if exact{BindingBasis::DirectDefinition}else{BindingBasis::DefinitionCandidate},None,range);
                }
            }
        }
        Origin::Scenario{scenario}=>for a in d.evidence.associations.iter().filter(|a|a.scenario==*scenario){
            let qualification=need(&d.source.core.qualifications,a.qualification)?;
            if qualification.context!=unit.context{continue;}
            let Some(alternative)=d.source.facts.alternatives.get(a.alternative)else{continue;};
            let event=need(&d.source.facts.events,alternative.event)?;
            if event.context!=unit.context{return Err(invalid("scenario call-site context differs"));}
            let occurrence=need(&d.source.core.occurrences,event.site)?;
            let exact=a.basis==c1::AssociationBasis::ResolvedTarget && qualification.approximation==assertion::Approximation::Exact && qualification.modality==attribution::Modality::Definite;
            push(Subject::Member{member:a.member},if exact{BindingBasis::ScenarioResolved}else{BindingBasis::ScenarioCandidate},Some(a.qualification),Some((occurrence.source,occurrence.start,occurrence.end)));
        },
        Origin::Passage{observation}=>{
            let passage=need(&d.source.facts.passages,*observation)?;
            for a in d.evidence.document_associations.iter(){
                let candidate=need(&d.source.facts.mention_candidates,a.candidate)?;
                let assessment=need(&d.source.facts.mention_assessments,candidate.assessment)?;
                let mention=need(&d.source.facts.mentions,assessment.observation)?;
                if mention.passage!=passage.passage {continue;}
                let q=need(&d.source.core.qualifications,mention.qualification)?;
                if q.context!=unit.context{continue;}
                let node=need(&d.source.facts.nodes,mention.mention.id())?;
                push(Subject::Member{member:a.member},BindingBasis::DocumentCandidate,Some(mention.qualification),Some(super::source::coordinates(d,&AnchorSource::Span{span:node.span()})?));
            }
        }
        Origin::Deployment{deployment}=>for r in d.evidence.release_deployments.iter().filter(|r|r.deployment==*deployment){push(Subject::Release{release:r.release},BindingBasis::DeclaredRelease,None,None);},
        Origin::Option{option}=>{let row=need(&d.source.catalog.options,*option)?;push(Subject::Option{option:*option},BindingBasis::Option,None,None);push(Subject::Member{member:row.member},BindingBasis::Option,None,None);},
        Origin::Source{artifact}=>push(Subject::Source{artifact:*artifact},BindingBasis::Source,None,None),
        Origin::Release{release}=>push(Subject::Release{release:*release},BindingBasis::DeclaredRelease,None,None),
        Origin::Original{..}|Origin::Document{..}|Origin::UnavailableDefinition{..}=>{},
    }
    Ok(result)
}
pub(super) fn nominates_part(d:&Data,out:&Output,part:&ContentPart,n:&Nomination)->Result<bool,ModelError>{
    if part.purpose!=PartPurpose::Primary{return Ok(false);}
    if let Some(q)=n.qualification{if need(&d.source.core.qualifications,q)?.context!=need(&out.units,part.unit)?.context{return Ok(false);}}
    let Some((artifact,start,end))=n.range else{return Ok(true);};
    for map in out.part_maps.iter().filter(|m|m.part==part.id()){
        if let Some(original)=map.original{
            let source=need(&out.anchor_sources,original)?;
            if super::source::coordinates(d,source)?.0==artifact {
                let a=map.original_start.unwrap();let z=map.original_end.unwrap();
                let defining=matches!(n.basis,BindingBasis::DirectDefinition|BindingBasis::DefinitionCandidate);
                if (defining&&a>=start&&z<=end)||(!defining&&a<=start&&z>=end){return Ok(true);}
            }
        }
    }
    Ok(false)
}

fn grain_scope(d:&Data,origin:&Origin)->Result<Option<Subject>,ModelError>{
    Ok(match origin{
        Origin::Api{member}|Origin::UnavailableDefinition{member}=>Some(Subject::Member{member:*member}),
        Origin::Definition{entity,..}=>Some(Subject::Definition{entity:*entity}),
        Origin::Option{option}=>Some(Subject::Option{option:*option}),
        Origin::Release{release}=>Some(Subject::Release{release:*release}),
        Origin::Source{artifact}=>Some(Subject::Source{artifact:*artifact}),
        Origin::Document{observation}=>Some(Subject::Source{artifact:d.document_source(*observation)?}),
        Origin::Passage{observation}=>{let row=need(&d.source.facts.passages,*observation)?;let node=need(&d.source.facts.nodes,row.passage.id())?;Some(Subject::Source{artifact:super::source::coordinates(d,&AnchorSource::Span{span:node.span()})?.0})},
        // Scenario and brief grains can contain several qualified sources/targets. Their unit
        // origin and context remain the scope; no arbitrary single member is chosen.
        Origin::Scenario{..}|Origin::Brief{..}|Origin::Deployment{..}|Origin::Original{..}=>None,
    })
}

pub(super) fn parts(d:&Data,out:&mut Output,unit:Id<Unit>,b:&ResourceBudget)->Result<(),ModelError>{
    let owner=need(&out.units,unit)?.clone();
    let origin=need(&out.origins,owner.origin)?.clone();
    let text=need(&out.corpus,owner.corpus)?.text.as_str().to_owned();
    let scope=grain_scope(d,&origin)?.map(|s|out.subjects.insert(s)).transpose()?;
    let anchors:Vec<_>=out.anchors.iter().filter(|a|a.unit==unit).map(|a|a.original).collect();
    let mut segments:Vec<(usize,usize,Option<Id<AnchorSource>>,Option<i64>,PartPurpose)>=vec![];
    let mut cursor=0;
    for original in anchors {
        let anchor=need(&out.anchor_sources,original)?;
        // Interpreted literal bytes are not the captured source spelling: retain its anchor but
        // do not fabricate a byte-wise identity map.
        if matches!(anchor,AnchorSource::Prose{..}){continue;}
        let captured=super::source::read(d,anchor,b)?;
        if captured.value.is_empty(){continue;}
        if let Some(relative)=text[cursor..].find(&captured.value){
            let start=cursor+relative;
            if start>cursor{segments.push((cursor,start,None,None,PartPurpose::Context));}
            let purpose=if matches!(anchor,AnchorSource::OccurrenceSlice{..}){PartPurpose::Context}else if let Origin::Scenario{scenario}=origin{
                if d.evidence.spans.iter().any(|s|s.scenario==scenario && matches!(s.role,c1::SpanRole::Primary|c1::SpanRole::ExtractedPython) && matches!(anchor,AnchorSource::Original{source}if *source==s.source)){PartPurpose::Primary}else{PartPurpose::Context}
            }else{PartPurpose::Primary};
            segments.push((start,start+captured.value.len(),Some(original),Some(super::source::coordinates(d,anchor)?.1),purpose));
            cursor=start+captured.value.len();
        }
    }
    if cursor<text.len(){segments.push((cursor,text.len(),None,None,if matches!(origin,Origin::Api{..}|Origin::Brief{..}|Origin::Option{..}|Origin::Release{..}){PartPurpose::Primary}else{PartPurpose::Context}));}
    if segments.is_empty() && !text.is_empty(){segments.push((0,text.len(),None,None,PartPurpose::Context));}
    let mut ordinal=0;
    for (start,end,original,original_start,purpose) in segments{
        let ranges=if matches!(origin,Origin::Document{..}|Origin::Passage{..}) && purpose==PartPurpose::Primary{
            document_blocks(&text[start..end]).into_iter().map(|(a,z)|(a,z,purpose)).collect()
        }else if let Some(original)=original{
            source_blocks(d,out,original,&text[start..end],purpose,owner.context)?
        }else{vec![(0,end-start,purpose)]};
        for (a,z,purpose) in ranges{
            let value=&text[start+a..start+z];
            if value.is_empty(){continue;}
            // A heading supplies interpretation context; mentioning an API there does not bind it.
            let purpose=if matches!(origin,Origin::Document{..}|Origin::Passage{..}) && (value.trim_start().starts_with('#')||value.trim().is_empty()){PartPurpose::Context}else{purpose};
            let part=out.parts.insert(ContentPart{unit,ordinal,purpose,scope,qualification:match origin{Origin::Passage{observation}=>Some(need(&d.source.facts.passages,observation)?.qualification),_=>None},digest:ContentHash::of(value.as_bytes()),text:value.into()})?;
            out.part_maps.insert(PartSourceMap{part,ordinal:0,start:0,end:value.len() as i64,original,original_start:original_start.map(|v|v+a as i64),original_end:original_start.map(|v|v+z as i64)})?;
            ordinal+=1;
        }
    }
    Ok(())
}
/// The canonical Ruff-derived placement owner supplies body boundaries. Compound statements
/// remain intact, carrying enclosing predicates/control; no line/byte heuristic splits Python.
fn source_blocks(d:&Data,out:&Output,original:Id<AnchorSource>,text:&str,purpose:PartPurpose,context:Id<AnalysisContext>)->Result<Vec<(usize,usize,PartPurpose)>,ModelError>{
    let anchor=need(&out.anchor_sources,original)?;
    let occurrence=match anchor{
        AnchorSource::Occurrence{occurrence}=>*occurrence,
        AnchorSource::Artifact{artifact}=>{let Some(module)=d.source.core.occurrences.iter().find(|o|o.source==*artifact&&o.syntax_kind==crate::domain::source::SyntaxKind::ModModule&&o.start==0&&o.end==text.len() as i64)else{return Ok(vec![(0,text.len(),purpose)]);};module.id()},
        _=>return Ok(vec![(0,text.len(),purpose)]),
    };
    let parent=need(&d.source.core.occurrences,occurrence)?;
    if !matches!(parent.syntax_kind,crate::domain::source::SyntaxKind::StmtFunctionDef|crate::domain::source::SyntaxKind::StmtClassDef|crate::domain::source::SyntaxKind::ModModule){return Ok(vec![(0,text.len(),purpose)]);}
    let mut children=vec![];
    for placement in d.source.core.placements.iter().filter(|p|p.parent==Some(occurrence)&&p.field==lexical::SyntaxField::Body){
        let q=need(&d.source.core.qualifications,placement.qualification)?;
        if q.context!=context{continue;}
        let child=need(&d.source.core.occurrences,placement.occurrence)?;
        if child.source!=parent.source||child.start<parent.start||child.end>parent.end{return Err(invalid("canonical source body child exceeds definition"));}
        children.push((child.start-parent.start,child.end-parent.start,child.syntax_kind as i16));
    }
    children.sort_unstable();children.dedup();
    if children.is_empty(){return Ok(vec![(0,text.len(),purpose)]);}
    let mut result=vec![];let first=children[0].0 as usize;
    if first>0{result.push((0,first,PartPurpose::Context));}
    for (index,(start,end,kind)) in children.iter().enumerate(){
        let end=children.get(index+1).map_or(text.len(),|(next,_,_)|*next as usize).max(*end as usize);
        if end>text.len()||!text.is_char_boundary(*start as usize)||!text.is_char_boundary(end){return Err(invalid("canonical source body boundaries are not UTF8"));}
        let purpose=if parent.syntax_kind==crate::domain::source::SyntaxKind::ModModule && (*kind==crate::domain::source::SyntaxKind::StmtImport as i16||*kind==crate::domain::source::SyntaxKind::StmtImportFrom as i16){PartPurpose::Context}else{purpose};
        result.push((*start as usize,end,purpose));
    }
    Ok(result)
}
/// Markdown block boundary: fenced blocks and tables remain indivisible; blank lines separate
/// prose/list blocks. Captured offsets are retained, including all separators.
fn document_blocks(text:&str)->Vec<(usize,usize)>{
    let mut ranges=vec![];let mut start=0;let mut offset=0;let mut fence:Option<&str>=None;
    for line in text.split_inclusive('\n'){
        let trimmed=line.trim_start();
        if fence.is_none() && trimmed.starts_with('#'){if offset>start{ranges.push((start,offset));}offset+=line.len();ranges.push((offset-line.len(),offset));start=offset;continue;}
        if trimmed.starts_with("```")||trimmed.starts_with("~~~"){
            let marker=&trimmed[..3];if fence==Some(marker){fence=None;}else if fence.is_none(){fence=Some(marker);}
        }
        offset+=line.len();
        if fence.is_none() && line.trim().is_empty(){ranges.push((start,offset));start=offset;}
    }
    if start<text.len(){ranges.push((start,text.len()));}ranges
}

fn role_matches(d:&Data,out:&Output,part:&ContentPart)->Result<bool,ModelError>{
    let unit=need(&out.units,part.unit)?;let origin=need(&out.origins,unit.origin)?;
    let maps:Vec<_>=out.part_maps.iter().filter(|m|m.part==part.id()).collect();
    let original=maps.iter().find_map(|m|m.original.map(|a|(a,m.original_start.unwrap(),m.original_end.unwrap())));
    let expected=match origin{
        Origin::Api{..}|Origin::Brief{..}|Origin::Release{..}=>PartPurpose::Primary,
        Origin::Option{..}=>if original.is_some()||out.anchors.iter().all(|a|a.unit!=unit.id()){PartPurpose::Primary}else{PartPurpose::Context},
        Origin::UnavailableDefinition{..}=>PartPurpose::Context,
        Origin::Scenario{scenario}=>if original.is_some_and(|(anchor,_,_)|matches!(out.anchor_sources.get(anchor),Some(AnchorSource::Original{source})if d.evidence.spans.iter().any(|s|s.scenario==*scenario&&s.source==*source&&matches!(s.role,c1::SpanRole::Primary|c1::SpanRole::ExtractedPython)))){PartPurpose::Primary}else{PartPurpose::Context},
        Origin::Document{..}|Origin::Passage{..}=>if original.is_none()||(part.text.as_str().trim_start().starts_with('#')||part.text.as_str().trim().is_empty()){PartPurpose::Context}else{PartPurpose::Primary},
        Origin::Deployment{..}=>if original.is_some(){PartPurpose::Primary}else{PartPurpose::Context},
        Origin::Definition{..}|Origin::Source{..}=>{
            let Some((anchor,start,end))=original else{return Ok(part.purpose==PartPurpose::Context);};
            let anchor=need(&out.anchor_sources,anchor)?;
            if matches!(anchor,AnchorSource::OccurrenceSlice{..}){return Ok(part.purpose==PartPurpose::Context);}
            let parent=match anchor{
                AnchorSource::Occurrence{occurrence}=>d.source.core.occurrences.get(*occurrence),
                AnchorSource::Artifact{artifact}=>d.source.core.occurrences.iter().find(|o|o.source==*artifact&&o.syntax_kind==crate::domain::source::SyntaxKind::ModModule),
                _=>None,
            };
            let context=if let Some(parent)=parent{
                let children:Vec<_>=d.source.core.placements.iter().filter(|p|p.parent==Some(parent.id())&&p.field==lexical::SyntaxField::Body&&d.source.core.qualifications.get(p.qualification).is_some_and(|q|q.context==unit.context)).filter_map(|p|d.source.core.occurrences.get(p.occurrence)).collect();
                let header=children.iter().map(|c|c.start).min().is_some_and(|first|end<=first);
                let import=parent.syntax_kind==crate::domain::source::SyntaxKind::ModModule&&children.iter().any(|c|matches!(c.syntax_kind,crate::domain::source::SyntaxKind::StmtImport|crate::domain::source::SyntaxKind::StmtImportFrom)&&start>=c.start&&start<c.end);
                header||import
            }else{false};
            if context{PartPurpose::Context}else{PartPurpose::Primary}
        },
        Origin::Original{..}=>return Ok(false),
    };
    Ok(part.purpose==expected)
}

fn expected_originals(d:&Data,unit:&Unit,origin:&Origin)->Result<Option<Vec<AnchorSource>>,ModelError>{
    let anchors=match origin{
        Origin::Source{artifact}=>vec![AnchorSource::Artifact{artifact:*artifact}],
        Origin::Definition{member,entity}=>{let (_,anchor,_)=definitions(d,*member,unit.context)?.into_iter().find(|(candidate,_,_)|candidate==entity).ok_or_else(||invalid("defining source candidate absent"))?;let Some(anchor)=anchor else{return Ok(Some(vec![]));};let mut headers=enclosing(d,&anchor,unit.context)?.into_iter().map(|(anchor,_)|anchor).collect::<Vec<_>>();headers.push(anchor);headers},
        Origin::Scenario{scenario}=>{let mut spans=d.evidence.spans.iter().filter(|r|r.scenario==*scenario).collect::<Vec<_>>();spans.sort_by_key(|r|(r.ordinal,r.id()));spans.into_iter().map(|r|AnchorSource::Original{source:r.source}).collect()},
        Origin::Document{observation}=>vec![AnchorSource::Artifact{artifact:d.document_source(*observation)?}],
        Origin::Passage{observation}=>vec![AnchorSource::Span{span:need(&d.source.facts.nodes,need(&d.source.facts.passages,*observation)?.passage.id())?.span()}],
        Origin::Deployment{deployment}=>vec![AnchorSource::Span{span:need(&d.source.facts.deployment,need(&d.evidence.deployments,*deployment)?.observation)?.span}],
        Origin::Option{option}=>option_anchors(d,need(&d.source.catalog.options,*option)?)?,
        Origin::Api{..}|Origin::Release{..}|Origin::UnavailableDefinition{..}=>vec![],
        Origin::Brief{..}=>return Ok(None),
        Origin::Original{..}=>return Err(invalid("legacy original origin cannot own evidence")),
    };Ok(Some(anchors))
}
fn verify_original_domain(d:&Data,out:&Output,unit:&Unit)->Result<(),ModelError>{
    let Some(expected)=expected_originals(d,unit,need(&out.origins,unit.origin)?)?else{return Ok(());};
    let mut actual=out.anchors.iter().filter(|a|a.unit==unit.id()).collect::<Vec<_>>();actual.sort_by_key(|a|a.ordinal);
    if actual.len()!=expected.len()||actual.iter().zip(&expected).enumerate().any(|(ordinal,(a,e))|a.ordinal!=ordinal as i64||a.original!=e.id()){return Err(invalid("retrieval original anchor domain differs from source owner"));}
    for anchor in expected{let (_,start,end)=super::source::coordinates(d,&anchor)?;let mut ranges=vec![];for map in out.part_maps.iter().filter(|map|map.original==Some(anchor.id())&&out.parts.get(map.part).is_some_and(|p|p.unit==unit.id())){map.validate()?;ranges.push((map.original_start.ok_or_else(||invalid("original map start missing"))?,map.original_end.ok_or_else(||invalid("original map end missing"))?));}ranges.sort_unstable();let mut cursor=start;for(a,z)in ranges{if a>cursor||a<start||z>end{return Err(invalid("retrieval original map coverage differs from source owner"));}cursor=cursor.max(z);}if cursor!=end{return Err(invalid("retrieval required original content omitted or relabeled synthetic"));}}
    Ok(())
}
pub(super) fn verify(out:&Output,d:&Data,b:&ResourceBudget)->Result<(),ModelError>{
    let definition=d.selected()?;
    definition.validate()?;
    let _index=b.reserve("retrieval-semantic-admission",(out.parts.len()+out.windows.len()+out.part_maps.len()+out.window_maps.len()+out.bindings.len()).saturating_mul(128))?;
    let mut rooted=std::collections::BTreeSet::new();
    for root in out.roots.iter(){
        let unit=need(&out.units,root.unit)?;
        let evidence=need(&d.evidence.roots,root.root)?;
        if evidence.input!=unit.input || evidence.context!=unit.context{return Err(invalid("retrieval unit differs from contextual root"));}
        d.verify_origin(unit,need(&out.origins,unit.origin)?,evidence)?;rooted.insert(unit.id());
    }
    let mut used_corpus=std::collections::BTreeSet::new();
    for unit in out.units.iter(){
        if !rooted.contains(&unit.id()){return Err(invalid("retrieval unit lacks contextual root"));}
        verify_original_domain(d,out,unit)?;
        let corpus=need(&out.corpus,unit.corpus)?;used_corpus.insert(corpus.id());
        if unit.family!=corpus.family{return Err(invalid("retrieval unit family differs from corpus"));}
        let mut parts:Vec<_>=out.parts.iter().filter(|p|p.unit==unit.id()).collect();parts.sort_by_key(|p|p.ordinal);
        if parts.is_empty(){return Err(invalid("retrieval content parts omitted"));}
        let mut reconstructed=String::new();
        for (ordinal,part) in parts.iter().enumerate(){
            part.validate()?;
            if !role_matches(d,out,part)?{return Err(invalid("retrieval primary/context role differs from source authority"));}
            if part.ordinal!=ordinal as i64{return Err(invalid("retrieval content part order incomplete"));}
            if let Some(scope)=part.scope{need(&out.subjects,scope)?;}
            if part.scope!=grain_scope(d,need(&out.origins,unit.origin)?)?.map(|s|s.id()){return Err(invalid("retrieval content semantic scope differs from origin"));}
            if let Some(q)=part.qualification{if need(&d.source.core.qualifications,q)?.context!=unit.context{return Err(invalid("retrieval content qualification changed context"));}}
            reconstructed.push_str(part.text.as_str());
            let mut maps:Vec<_>=out.part_maps.iter().filter(|m|m.part==part.id()).collect();maps.sort_by_key(|m|m.ordinal);
            let mut cursor=0;
            for (index,map) in maps.iter().enumerate(){
                map.validate()?;
                if map.ordinal!=index as i64 || map.start!=cursor || part.text.as_str().get(map.start as usize..map.end as usize).is_none(){return Err(invalid("retrieval part maps incomplete or non-UTF8"));}
                if let Some(original)=map.original{
                    let anchor=need(&out.anchor_sources,original)?;
                    let (artifact,start,end)=super::source::coordinates(d,anchor)?;
                    let (input,len)=d.artifact_bounds(artifact)?;
                    if map.original_start.unwrap()<start || map.original_end.unwrap()>end || end>len || (!matches!(need(&out.origins,unit.origin)?,Origin::Brief{..}) && input!=unit.input){return Err(invalid("retrieval source map exceeds original contextual anchor"));}
                    // Exact selected original ranges are necessary evidence, not producer replay.
                    let original=super::source::read_range(d,artifact,map.original_start.unwrap(),map.original_end.unwrap(),b)?;
                    if Some(original.value.as_str())!=part.text.as_str().get(map.start as usize..map.end as usize){return Err(invalid("retrieval original map bytes differ"));}

                }
                cursor=map.end;
            }
            if cursor!=part.text.len() as i64{return Err(invalid("retrieval part source map domain omitted"));}
        }
        if reconstructed!=corpus.text.as_str(){return Err(invalid("retrieval content parts differ from completed corpus"));}
        let mut windows:Vec<_>=out.windows.iter().filter(|w|w.unit==unit.id()).collect();windows.sort_by_key(|w|w.ordinal);
        if windows.is_empty(){return Err(invalid("retrieval window domain omitted"));}
        let mut covered=std::collections::BTreeSet::new();
        let nominations=nominations(d,out,unit.id())?;
        for (ordinal,window) in windows.iter().enumerate(){
            window.validate()?;
            if definition.embedding_requested && window.availability==WindowAvailability::TokenizerUnavailable{return Err(invalid("requested retrieval window omitted local tokenizer admission"));}
            if window.definition!=definition.id()||window.ordinal!=ordinal as i64{return Err(invalid("retrieval window policy/order differs"));}
            let corpus=need(&out.corpus,window.corpus)?;used_corpus.insert(corpus.id());
            if corpus.family!=unit.family || corpus.text!=window.text{return Err(invalid("retrieval search corpus differs from window"));}
            let mut links:Vec<_>=out.window_parts.iter().filter(|p|p.window==window.id()).collect();links.sort_by_key(|p|p.ordinal);
            let mut render=vec![];
            let mut linked=std::collections::BTreeSet::new();
            let mut expected_bindings=std::collections::BTreeSet::new();
            for (index,link) in links.iter().enumerate(){
                let part=need(&out.parts,link.part)?;
                if link.ordinal!=index as i64 || part.unit!=unit.id() || link.start!=0 || link.end!=part.text.len() as i64 || !linked.insert(part.id()){return Err(invalid("retrieval window part interval differs"));}
                render.push(part.text.as_str());
                if part.purpose==PartPurpose::Primary{
                    if !covered.insert(part.id()){return Err(invalid("retrieval primary part duplicated across windows"));}
                    for n in &nominations{if nominates_part(d,out,part,n)?{
                        expected_bindings.insert(WindowBinding{window:window.id(),part:part.id(),subject:n.subject,basis:n.basis,qualification:n.qualification}.id());
                    }}
                }
            }
            for part in &parts{if part.purpose==PartPurpose::Context && !linked.contains(&part.id()){return Err(invalid("retrieval mandatory interpretation context omitted"));}}
            if render.join("\n")!=window.text.as_str(){return Err(invalid("retrieval window rendering differs from selected parts"));}
            let body_start=window.input_text.as_str().find(window.text.as_str()).ok_or_else(||invalid("retrieval complete input loses rendered body"))?;
            if let Some(tokenizer)=d.tokenizer(){
                let encoded=tokenizer.encode(window.text.as_str())?;
                if window.tokenizer!=Some(tokenizer.identity()) || window.input_text.as_str()!=encoded.text || window.tokens!=Some(encoded.offsets.len() as i64){return Err(invalid("retrieval tokenization differs from selected exact assets"));}
            }
            let mut maps:Vec<_>=out.window_maps.iter().filter(|m|m.window==window.id()).collect();maps.sort_by_key(|m|m.ordinal);
            let mut cursor=0;
            for (index,map) in maps.iter().enumerate(){
                map.validate()?;
                if map.ordinal!=index as i64||map.start!=cursor||window.input_text.as_str().get(map.start as usize..map.end as usize).is_none(){return Err(invalid("retrieval window source map incomplete/non-UTF8"));}
                if let Some(part)=map.part{
                    if !linked.contains(&part){return Err(invalid("retrieval window map references unselected part"));}
                    let part=need(&out.parts,part)?;
                    // The whole ordered map stream must exactly compose the canonical part maps.
                    let position=links.iter().position(|l|l.part==part.id()).unwrap();
                    let offset=body_start+links[..position].iter().map(|l|need(&out.parts,l.part).unwrap().text.len()+1).sum::<usize>();
                    if !out.part_maps.iter().any(|p|p.part==part.id() && p.start+offset as i64==map.start && p.end+offset as i64==map.end && p.original==map.original && p.original_start==map.original_start && p.original_end==map.original_end){return Err(invalid("retrieval window map is not canonical part composition"));}
                }else {
                    if map.original.is_some(){return Err(invalid("synthetic input has fabricated source"));}
                    let prefix=map.start==0&&map.end==body_start as i64;
                    let suffix=map.start==(body_start+window.text.len())as i64&&map.end==window.input_text.len()as i64;
                    let separator=links.iter().take(links.len().saturating_sub(1)).scan(body_start,|offset,link|{*offset+=need(&out.parts,link.part).unwrap().text.len();let start=*offset;*offset+=1;Some((start as i64,*offset as i64))}).any(|(a,z)|map.start==a&&map.end==z);
                    if !prefix&&!suffix&&!separator{return Err(invalid("synthetic map replaces primary source bytes"));}
                }
                cursor=map.end;
            }
            if cursor!=window.input_text.len() as i64{return Err(invalid("retrieval complete encoder input map omitted"));}
            let actual:std::collections::BTreeSet<_>=out.bindings.iter().filter(|r|r.window==window.id()).map(|r|r.id()).collect();
            if actual!=expected_bindings{return Err(invalid("retrieval primary binding nomination differs/omitted"));}
        }
        for part in parts{if part.purpose==PartPurpose::Primary && !covered.contains(&part.id()){return Err(invalid("retrieval primary part has no search window"));}}
        let mut anchors:Vec<_>=out.anchors.iter().filter(|a|a.unit==unit.id()).collect();anchors.sort_by_key(|a|a.ordinal);
        for (index,anchor) in anchors.iter().enumerate(){if anchor.ordinal!=index as i64{return Err(invalid("retrieval original anchors incomplete"));}super::source::coordinates(d,need(&out.anchor_sources,anchor.original)?)?;}
    }
    for corpus in out.corpus.iter(){corpus.validate()?;if corpus.rendering_version!=definition.rendering_version||!used_corpus.contains(&corpus.id()){return Err(invalid("retrieval corpus lacks contextual occurrence"));}}
    for part in out.parts.iter(){need(&out.units,part.unit)?;}
    for map in out.part_maps.iter(){need(&out.parts,map.part)?;}
    for link in out.window_parts.iter(){need(&out.windows,link.window)?;need(&out.parts,link.part)?;}
    for map in out.window_maps.iter(){need(&out.windows,map.window)?;}
    for binding in out.bindings.iter(){need(&out.windows,binding.window)?;let part=need(&out.parts,binding.part)?;need(&out.subjects,binding.subject)?;if part.purpose!=PartPurpose::Primary{return Err(invalid("context-only part cannot nominate applicability"));}}
    for link in out.unit_subjects.iter(){need(&out.units,link.unit)?;need(&out.subjects,link.subject)?;}
    for anchor in out.anchors.iter(){need(&out.units,anchor.unit)?;need(&out.anchor_sources,anchor.original)?;}
    Ok(())
}
