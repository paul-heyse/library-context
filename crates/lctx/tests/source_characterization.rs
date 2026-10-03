//! Actual compiler + PG18 + original-evidence packet, with no execution inference.
#[path="fixtures/serving_support.rs"] mod support;
use lctx_model::domain::{*,assertion::*,diagnostics::*,serving::*,source::*};
#[tokio::test]
async fn selected_diagnostics_are_cited_inside_the_same_original_source_grant(){
    let source=b"import os\n__all__ = ['api']\nlabel = 'caf\xc3\xa9'\nmissing_name\nsuppressed_name  # noqa: F821\ndef api(value: int) -> int:\n    unused_local = 'kept'\n    return 'wrong'\nimport pytest as ptest\n@ptest.fixture\ndef resource() -> int:\n    return 1\ndef test_resource(resource):\n    pass\n";
    let fixture=support::ServingFixture::start(source).await;let execution=fixture.service.execution().await.unwrap();
    let digest=ContentHash::of(source);let artifacts=execution.read::<SourceArtifact>().await.unwrap();let artifact=artifacts.rows().iter().find(|a|a.content==digest).unwrap().id();
    let full=fixture.service.evidence(&execution,&GetEvidenceRequest{source:OriginalReference::Artifact{artifact},page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
    assert_eq!(full.evidence.body.bytes,source);let items=&full.evidence.source_characterization.items;assert!(!items.is_empty());
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::RuffDiagnostic{native_code,channel,message,..} if native_code.as_str()=="F821" && *channel==DiagnosticChannel::RuffNoqaSuppressed && message.as_str().contains("suppressed_name"))));
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::PyreflyDiagnostic{category,channel,..} if category.as_str()=="bad-return"&&*channel==DiagnosticChannel::Emitted)));
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Ordinary)));
    for item in items{assert_eq!(item.source.artifact,artifact);assert!(item.source.start>=full.evidence.original.start&&item.source.end<=full.evidence.original.end);assert_eq!(item.support.context,full.evidence.original.context);assert!(item.proof.iter().any(|p|p==&item.support.support));}
    let spans=execution.read::<Evidence>().await.unwrap();let primary=items.iter().find_map(|i|matches!(&i.payload,SourceCharacterizationPayload::RuffDiagnostic{native_code,channel,..} if native_code.as_str()=="F821"&&*channel==DiagnosticChannel::Emitted).then_some(&i.source)).unwrap();
    let span=spans.rows().iter().find(|e|matches!(e,Evidence::SourceSpan{source,start,end} if *source==artifact&&*start==primary.start as i64&&*end==primary.end as i64)).unwrap();let id=EvidenceSourceSpanId::of(span).unwrap();
    let bounded=fixture.service.evidence(&execution,&GetEvidenceRequest{source:OriginalReference::Span{span:id},page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
    assert_eq!(bounded.evidence.body.bytes,&source[primary.start as usize..primary.end as usize]);
    assert!(bounded.evidence.source_characterization.items.iter().all(|i|i.source.artifact==artifact&&i.source.start>=primary.start&&i.source.end<=primary.end));
    assert!(!bounded.evidence.source_characterization.items.iter().any(|i|matches!(i.payload,SourceCharacterizationPayload::ParameterDefinition{..})));
    let fixture_answer=items.iter().find(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,target_location,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Fixture&&matches!(target_location,Availability::Available{}))).expect("actual fixture answer retains its native target");
    let occurrences=execution.read::<Occurrence>().await.unwrap();
    let parameter=occurrences.rows().iter().find(|o|o.source==artifact&&o.start==fixture_answer.source.start as i64&&o.end==fixture_answer.source.end as i64).unwrap();
    let parameter_packet=fixture.service.evidence(&execution,&GetEvidenceRequest{source:OriginalReference::Occurrence{occurrence:parameter.id()},page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
    assert_eq!(parameter_packet.evidence.body.bytes,&source[parameter.start as usize..parameter.end as usize]);
    assert!(parameter_packet.evidence.source_characterization.items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,target_location,target,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Fixture&&matches!(target_location,Availability::Unavailable{..})&&target.0.is_none())),"out-of-grant fixture definition remains unavailable without changing the native answer");
    drop(execution);fixture.finish().await;
}
