//! Independent finite ER1 expectations: defining source, primary nomination and complete input maps.
use lctx_model::domain::{retrieval::{self,build::{Data,Output},partition::{Tokenizer,EncodedInput},*},catalog::{self,evidence as c1},normalized::{Rows,entities::*},source::*,assertion::*,attribution::*,resources::ResourceBudget,*};
use std::sync::Arc;
use lctx_model::domain::retrieval::{Origin,Subject};
fn id<T>(n:u8)->Id<T>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
fn artifact(d:&mut Data,bytes:&str,path:&str)->SourceArtifact{let a=SourceArtifact::from_bytes(id(1),path.into(),bytes.as_bytes()).unwrap();d.source.core.artifacts.insert(a.clone()).unwrap();for chunk in artifact::ArtifactChunk::split(&a,bytes.as_bytes()).unwrap(){d.facts.chunks.insert(chunk).unwrap();}a}
fn root(d:&mut Data,subject:c1::RootSubject){let subject=d.evidence.subjects.insert(subject).unwrap();d.evidence.roots.insert(c1::EvidenceRoot{input:id(1),context:id(2),subject}).unwrap();}
fn qualification(d:&mut Data,a:&SourceArtifact)->Id<AssertionQualification>{d.source.core.qualifications.insert(AssertionQualification{assumptions:assumptions::AssumptionSet::empty_id(),context:id(2),scope:CoverageScope::Artifact{artifact:a.id()}.id(),condition:conditions::Diagram::always().id(),modality:Modality::Definite,approximation:Approximation::Exact}).unwrap()}
fn base(selected:bool)->(ResourceBudget,Data){let b=ResourceBudget::fixed(32<<20).unwrap();let mut d=Data::new(&b);d.facts.definitions.insert(Definition::builtin(selected)).unwrap();(b,d)}
fn public_member(d:&mut Data,name:&str)->Id<catalog::CatalogMember>{let a=artifact(d,&format!("from implementation import {name}\n"),"public.py");let access=d.source.core.modules.insert(Module{source:a.id(),qualified_name:"public".into()}).unwrap();let member=d.source.catalog.members.insert(catalog::CatalogMember{input:id(1),access,path:vec![name.into()],name:name.into()}).unwrap();root(d,c1::RootSubject::Member{member});member}
fn definition(d:&mut Data,member:Id<catalog::CatalogMember>,source:&SourceArtifact,start:i64,end:i64,kind:SyntaxKind,status:ResolutionStatus)->Id<EntityRef>{
    let occurrence=d.source.core.occurrences.insert(Occurrence{source:source.id(),start,end,syntax_kind:kind,role:OccurrenceRole::Declaration,structural_path:vec![start as i32]}).unwrap();
    let entity=d.source.core.refs.insert(EntityRef::Occurrence{occurrence}).unwrap();
    let access=d.source.catalog.members.get(member).unwrap().access;
    let public=d.source.core.exposures.insert(PublicExposure{access,context:id(2),observation:id(start as u8+20),origin:id(10),enumeration:None,publicity:PublicPathKnowledge::Known,status,reason:EntityReason::DeclarationAgreement}).unwrap();
    let exposure=d.source.catalog.exposures.insert(catalog::CatalogExposure{member,exposure:public}).unwrap();
    let resolution=d.source.core.resolutions.insert(SymbolEntityResolution{symbol:id(start as u8+20),context:id(2),policy:ContentHash::of(b"finite-definition-policy"),status,entity:if status==ResolutionStatus::Resolved{Some(entity)}else{None},reason:EntityReason::DeclarationAgreement}).unwrap();
    let candidate=d.source.core.entity_candidates.insert(SymbolEntityCandidate{resolution,entity}).unwrap();
    d.source.catalog.candidates.insert(catalog::CatalogCandidate{exposure,candidate:None,entity:Some(candidate),path:None,alias:None}).unwrap();entity
}
struct Scalars;
impl Tokenizer for Scalars{
    fn identity(&self)->ContentHash{ContentHash::of(b"finite-complete-input-reference")}
    fn encode(&self,text:&str)->Result<EncodedInput,ModelError>{let text=format!("H:{text}:E");let offsets=text.char_indices().map(|(a,c)|(a,a+c.len_utf8())).chain(std::iter::once((0,0))).collect::<Vec<_>>();let mut specials=vec![false;offsets.len()];*specials.last_mut().unwrap()=true;let body_end=text.len()-2;Ok(EncodedInput{text,body_start:2,body_end,offsets,specials})}
}
#[test]
fn reexports_and_siblings_keep_defining_bytes_and_qualified_alternatives(){
    let (b,mut d)=base(false);let member=public_member(&mut d,"run");let source=artifact(&mut d,"def alpha():\n    return 1\n\ndef beta():\n    return 2\n","implementation.py");
    let split="def alpha():\n    return 1\n".len() as i64;
    let first=definition(&mut d,member,&source,0,split,SyntaxKind::StmtFunctionDef,ResolutionStatus::Ambiguous);
    let second=definition(&mut d,member,&source,split+1,source.byte_len,SyntaxKind::StmtFunctionDef,ResolutionStatus::Ambiguous);
    let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();
    let definitions=out.units.iter().filter(|u|matches!(out.origins.get(u.origin),Some(Origin::Definition{..}))).collect::<Vec<_>>();assert_eq!(definitions.len(),2);
    for unit in definitions{let text=out.corpus.get(unit.corpus).unwrap().text.as_str();assert!(!text.contains("from implementation"));assert!(!(text.contains("alpha")&&text.contains("beta")));}
    assert!(out.bindings.iter().any(|r|r.subject==(Subject::Definition{entity:first}).id()));assert!(out.bindings.iter().any(|r|r.subject==(Subject::Definition{entity:second}).id()));
    assert!(out.bindings.iter().filter(|r|matches!(out.subjects.get(r.subject),Some(Subject::Definition{..}))).all(|r|r.basis==BindingBasis::DefinitionCandidate));
}
#[test]
fn non_callable_and_unavailable_definitions_are_distinct_honest_roots(){
    let (b,mut d)=base(false);let member=public_member(&mut d,"constant");let source=artifact(&mut d,"VALUE = 17\n","implementation.py");definition(&mut d,member,&source,0,source.byte_len,SyntaxKind::StmtAssign,ResolutionStatus::Resolved);
    let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();assert!(out.units.iter().any(|u|out.corpus.get(u.corpus).unwrap().text.as_str()=="VALUE = 17\n"));
    let (_b,mut unknown)=base(false);public_member(&mut unknown,"native");let out=retrieval::build::build(&unknown,&b).unwrap();out.verify_completion(&unknown,&b).unwrap();assert!(out.origins.iter().any(|r|matches!(r,Origin::UnavailableDefinition{..})));assert!(out.bindings.iter().all(|r|r.basis!=BindingBasis::DirectDefinition));
}
#[test]
fn complete_input_headers_specials_semantic_splits_and_indivisible_refusal(){
    let (b,mut d)=base(true);d.set_tokenizer(Arc::new(Scalars));
    let text=format!("# Guide é🦀\n\n{}\n\n{}\n\n{}", "a".repeat(1010),"b".repeat(1010),"z".repeat(2050));let source=artifact(&mut d,&text,"guide.md");let q=qualification(&mut d,&source);let document=d.source.facts.documents.insert(documents::DocumentObservation{qualification:q,source:source.id(),title:Some("Guide".into()),parsed:true}).unwrap();root(&mut d,c1::RootSubject::Document{observation:document});
    let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();assert_eq!(out.windows.len(),3);assert_eq!(out.windows.iter().filter(|w|w.availability==WindowAvailability::LexicalOnly).count(),1);
    for window in out.windows.iter(){assert!(window.text.as_str().contains("# Guide é🦀"));assert_eq!(window.tokens,Some(window.input_text.as_str().chars().count()as i64+1));let maps=out.window_maps.iter().filter(|m|m.window==window.id()).cloned().collect::<Vec<_>>();let encoded=Scalars.encode(window.text.as_str()).unwrap();let sources=retrieval::partition::token_sources(&encoded,&maps).unwrap();assert!(sources.iter().all(|s|s.token+1<encoded.offsets.len()));assert!(!sources.iter().any(|s|s.token==0));}
    let mut omitted=retrieval::build::build(&d,&b).unwrap();omitted.parts=Rows::new(&b);assert!(omitted.verify_completion(&d,&b).is_err());
    let mut forged=retrieval::build::build(&d,&b).unwrap();let first=forged.window_maps.iter().find(|m|m.original.is_some()).unwrap().clone();let mut maps=Rows::new(&b);for row in forged.window_maps.iter(){let mut row=row.clone();if row.id()==first.id(){row.original=None;row.original_start=None;row.original_end=None;row.part=None;}maps.insert(row).unwrap();}forged.window_maps=maps;assert!(forged.verify_completion(&d,&b).is_err());
}
#[test]
fn source_roots_preserve_standalone_material_and_context_cannot_bind(){
    let (b,mut d)=base(false);let source=artifact(&mut d,"import helper\n","standalone.py");root(&mut d,c1::RootSubject::Source{artifact:source.id()});let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();assert_eq!(out.units.len(),1);assert!(out.bindings.iter().all(|r|matches!(out.subjects.get(r.subject),Some(Subject::Source{..}))));
    let mut injected=retrieval::build::build(&d,&b).unwrap();let part=injected.parts.iter().next().unwrap().clone();let window=injected.windows.iter().next().unwrap().id();let subject=injected.subjects.iter().next().unwrap().id();let mut parts=Rows::new(&b);parts.insert(ContentPart{purpose:PartPurpose::Context,..part.clone()}).unwrap();injected.parts=parts;injected.bindings.insert(WindowBinding{window,part:part.id(),subject,basis:BindingBasis::Source,qualification:None}).unwrap();assert!(injected.verify_completion(&d,&b).is_err());
    let mut erased=retrieval::build::build(&d,&b).unwrap();let mut parts=Rows::new(&b);for part in erased.parts.iter(){parts.insert(ContentPart{purpose:PartPurpose::Context,..part.clone()}).unwrap();}erased.parts=parts;
    let mut maps=Rows::new(&b);for map in erased.part_maps.iter(){maps.insert(PartSourceMap{original:None,original_start:None,original_end:None,..map.clone()}).unwrap();}erased.part_maps=maps;
    let mut maps=Rows::new(&b);for map in erased.window_maps.iter(){maps.insert(WindowSourceMap{original:None,original_start:None,original_end:None,..map.clone()}).unwrap();}erased.window_maps=maps;erased.bindings=Rows::new(&b);
    assert!(erased.verify_completion(&d,&b).is_err(),"joint source-map/context/binding erasure cannot relabel required original bytes synthetic");
    d.facts.chunks=Rows::new(&b);assert!(out.verify_completion(&d,&b).is_err(),"original maps require exact admitted source chunks without producer replay");
}
#[test]
fn encoder_requested_without_exact_assets_is_refused(){let(b,mut d)=base(true);let source=artifact(&mut d,"source","x.py");root(&mut d,c1::RootSubject::Source{artifact:source.id()});assert!(retrieval::build::build(&d,&b).err().unwrap().to_string().contains("local tokenizer assets"));}
#[test]
fn canonical_parts_windows_maps_and_bindings_roundtrip_without_producer_replay(){
    let(b,mut d)=base(false);let public=public_member(&mut d,"value");let source=artifact(&mut d,"value = 'é🦀'\n","implementation.py");definition(&mut d,public,&source,0,source.byte_len,SyntaxKind::StmtAssign,ResolutionStatus::Resolved);
    let out=retrieval::build::build(&d,&b).unwrap();let mut imported=Output::new(&b);
    macro_rules! roundtrip{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&out.$field.iter().cloned().collect::<Vec<_>>()).unwrap();assert!(imported.visit(<$ty>::NAME,&batch).unwrap());)*};}
    lctx_model::retrieval_outputs!(roundtrip);
    out.matches(&imported).unwrap();imported.verify_completion(&d,&b).unwrap();
    for omission in 0..4{let mut damaged=Output::new(&b);macro_rules! load{($($field:ident:$ty:ty,)*)=>{$(for row in out.$field.iter(){damaged.$field.insert(row.clone()).unwrap();})*};}lctx_model::retrieval_outputs!(load);
        match omission{0=>damaged.windows=Rows::new(&b),1=>damaged.bindings=Rows::new(&b),2=>damaged.part_maps=Rows::new(&b),_=>damaged.window_parts=Rows::new(&b)}
        assert!(damaged.verify_completion(&d,&b).is_err(),"canonical omission {omission}");
    }
    let model=lctx_model::domain::model().unwrap();for input in Output::inputs(){assert!(model.relation(input.name()).is_some(),"canonical registration {}",input.name());}
}
#[test]
fn setup_only_scenario_has_context_maps_and_navigation_without_primary_binding(){
    let(b,mut d)=base(false);let source=artifact(&mut d,"import api\nhelper()\n","setup.py");let q=qualification(&mut d,&source);
    let original=d.evidence.original_sources.insert(c1::OriginalSource::Artifact{artifact:source.id()}).unwrap();
    let scenario=d.evidence.scenarios.insert(c1::CatalogScenario{source:id(12),extraction:deployment::CheckStatus::Passed,parse:deployment::CheckStatus::Passed,binding:deployment::CheckStatus::Passed,environment:deployment::CheckStatus::NotRun,execution:deployment::CheckStatus::NotRun,intent:c1::Intent::Demonstration}).unwrap();
    d.evidence.spans.insert(c1::ScenarioSpan{scenario,ordinal:0,role:c1::SpanRole::EnclosingModule,source:original}).unwrap();
    let site=d.source.core.occurrences.insert(Occurrence{source:source.id(),start:11,end:19,syntax_kind:SyntaxKind::ExprCall,role:OccurrenceRole::Call,structural_path:vec![1]}).unwrap();
    let event=d.source.facts.events.insert(normalized::events::NormalizedCallEvent{site,origin:id(15),context:id(2),owner:id(16)}).unwrap();
    let alternative=d.source.facts.alternatives.insert(normalized::events::NormalizedCallAlternative{event,source:id(17),resolution:None,correspondence:None,entity:None,status:ResolutionStatus::Resolved,reason:normalized::links::LinkReason::ExplicitIdentity}).unwrap();
    d.evidence.associations.insert(c1::ScenarioAssociation{scenario,member:id(18),alternative,qualification:q,phase:calls::CallPhase::Call,basis:c1::AssociationBasis::ResolvedTarget,intent:c1::Intent::Demonstration}).unwrap();root(&mut d,c1::RootSubject::Scenario{scenario});
    let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();assert!(!out.unit_subjects.is_empty());assert!(out.parts.iter().all(|p|p.purpose==PartPurpose::Context));assert!(out.bindings.is_empty());assert!(out.part_maps.iter().any(|m|m.original.is_some()));
}

#[test]
fn empty_original_source_keeps_honest_context_and_no_primary_binding(){
    let(b,mut d)=base(false);let source=artifact(&mut d,"","empty.py");root(&mut d,c1::RootSubject::Source{artifact:source.id()});let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();assert_eq!(out.units.len(),1);assert!(out.parts.iter().all(|p|p.purpose==PartPurpose::Context));assert!(out.bindings.is_empty());assert!(out.windows.iter().all(|w|w.text.as_str().contains("empty (0 bytes)")));assert_eq!(out.anchors.len(),1);
}

#[test]
fn documentary_source_bindings_require_primary_originals_without_api_nomination(){
    for with_passage in [false,true]{
        let(b,mut d)=base(false);let source=artifact(&mut d,"# Unresolved API heading\n\nActual documentary body.\n","guide.md");let q=qualification(&mut d,&source);
        let document=d.source.facts.documents.insert(documents::DocumentObservation{qualification:q,source:source.id(),title:Some("Unresolved API title".into()),parsed:true}).unwrap();
        if with_passage{
            let span=Evidence::SourceSpan{source:source.id(),start:0,end:source.byte_len};let span_id=EvidenceSourceSpanId::of(&span).unwrap();d.source.facts.canonical_evidence.insert(span).unwrap();
            let node=documents::DocumentNode::Passage{span:span_id,ordinal:0};let passage=documents::DocumentNodePassageId::of(&node).unwrap();d.source.facts.nodes.insert(node).unwrap();
            d.source.facts.passages.insert(documents::PassageObservation{qualification:q,passage,level:1,heading:Some("Unresolved API heading".into()),heading_path:vec![],text:"Provider interpretation cannot nominate an API".into()}).unwrap();
        }
        root(&mut d,c1::RootSubject::Document{observation:document});let out=retrieval::build::build(&d,&b).unwrap();out.verify_completion(&d,&b).unwrap();
        assert_eq!(out.units.len(),if with_passage{2}else{1});assert!(!out.bindings.is_empty());
        for unit in out.units.iter(){
            assert_eq!(unit.context,id(2));assert!(out.bindings.iter().any(|r|out.parts.get(r.part).unwrap().unit==unit.id()));
        }
        for binding in out.bindings.iter(){
            assert_eq!(out.subjects.get(binding.subject),Some(&Subject::Source{artifact:source.id()}));assert_eq!(binding.basis,BindingBasis::Source);
            let part=out.parts.get(binding.part).unwrap();assert_eq!(part.purpose,PartPurpose::Primary);assert!(part.text.as_str().contains("Actual documentary body"));
            assert!(out.part_maps.iter().any(|m|m.part==part.id()&&m.original.is_some()));
            if matches!(out.origins.get(out.units.get(part.unit).unwrap().origin),Some(Origin::Passage{..})){assert_eq!(binding.qualification,Some(q));}
        }
        assert!(!out.subjects.iter().any(|s|matches!(s,Subject::Member{..})));
        let mut erased=retrieval::build::build(&d,&b).unwrap();erased.bindings=Rows::new(&b);assert!(erased.verify_completion(&d,&b).is_err(),"original documentary evidence requires its Source binding");
        let mut injected=retrieval::build::build(&d,&b).unwrap();let context=injected.parts.iter().find(|p|p.purpose==PartPurpose::Context).unwrap().clone();let window=injected.window_parts.iter().find(|w|w.part==context.id()).unwrap().window;
        injected.bindings.insert(WindowBinding{window,part:context.id(),subject:Subject::Source{artifact:source.id()}.id(),basis:BindingBasis::Source,qualification:None}).unwrap();
        assert!(injected.verify_completion(&d,&b).is_err(),"title and heading context cannot nominate Source or API evidence");
    }
}
