//! Final native lexical metadata and source associations use actual retained providers.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{*, attribution::*, lexical::*, normalized::{entity_normalization,relation_normalization,entities::*,links::*}, resources::ResourceBudget, ruff::*, source::*};
use typed_driver::{files,rows};
inspector!(Facts,RuffBindingObservation,RuffDefinitionObservation,RuffContextObservation);
async fn fixture()->typed_driver::Tables {let tables=typed_driver::Tables::default();typed_driver::run(&files("native_lexical"),Facts(tables.clone())).await.unwrap();tables}
fn normalize(tables:&typed_driver::Tables,omit:Option<&str>)->(relation_normalization::RelationData,relation_normalization::RelationOutput){
    let budget=ResourceBudget::fixed(256<<20).unwrap();let mut data=relation_normalization::RelationData::new(&budget);
    macro_rules! facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(tables){data.facts.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);data.entities=entity_normalization::normalize(data.facts.inputs(),&budget).unwrap();
    macro_rules! rest {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(if omit!=Some(<$ty>::NAME){for row in rows::<$ty>(tables){data.$field.insert(row).unwrap();}})*};}
    lctx_model::normalized_relation_inputs!(rest);let out=relation_normalization::normalize(&data,&budget).unwrap();(data,out)
}
#[tokio::test]
async fn final_metadata_preserves_binding_kinds_rebinding_parent_and_unknown_scope(){
    let tables=fixture().await;let bindings=rows::<RuffBindingObservation>(&tables);let definitions=rows::<RuffDefinitionObservation>(&tables);let contexts=rows::<RuffContextObservation>(&tables);
    for kind in [RuffBindingKind::Argument,RuffBindingKind::FunctionDefinition,RuffBindingKind::ClassDefinition,RuffBindingKind::Assignment,RuffBindingKind::FromImport,RuffBindingKind::Deletion] {assert!(bindings.iter().any(|b|b.kind==kind),"{kind:?}");}
    assert!(bindings.iter().any(|b|b.native_name=="rebound"&&b.shadowed_location==NativeRelationLocation::Located));
    assert!(bindings.iter().any(|b|b.native_name=="item"&&b.scope_location==AttachmentStatus::Unlocated&&b.scope.is_none()));
    assert!(bindings.iter().any(|b|b.alias&&b.qualified_name.as_deref()==Some(&["collections".into(),"deque".into()])));
    assert!(definitions.iter().any(|d|d.name.as_deref()==Some("method")&&d.kind==RuffDefinitionKind::Method&&d.parent_location==NativeRelationLocation::Located));
    assert!(definitions.iter().any(|d|d.name.as_deref()==Some("nested")&&d.kind==RuffDefinitionKind::NestedFunction));
    assert!(contexts.iter().any(|c|c.phase==ContextPhase::FinalUnresolved&&c.unresolved_wildcard==Some(true)&&c.typing.is_none()));
    let providers=rows::<Provider>(&tables);let ruff=providers.iter().find(|p|p.tool=="ruff").unwrap();
    assert!(rows::<ProviderCoverage>(&tables).iter().any(|c|c.provider==Some(ruff.id())&&c.family==FactFamily::Lexical&&c.status==CoverageStatus::Partial));
    let (data,out)=normalize(&tables,None);
    assert!(out.reference_binding_characterizations.iter().any(|c|c.status==ResolutionStatus::Resolved&&c.support.is_some()&&c.context_support.is_some()));
    assert!(out.declaration_native_characterizations.iter().any(|c|c.status==ResolutionStatus::Resolved&&c.support.is_some()));
    // Native evidence never repairs a native unresolved answer merely from a custom candidate.
    for native in contexts.iter().filter(|c|c.phase==ContextPhase::FinalUnresolved){for reference in data.references.iter().filter(|r|r.read==native.subject){assert!(out.reference_entity_assessments.iter().filter(|a|a.reference==reference.id()).all(|a|a.status==ResolutionStatus::Unresolved));}}
    let runs=rows::<ProviderRun>(&tables);let occurrences=rows::<Occurrence>(&tables);
    let native_resolutions=rows::<LexicalResolutionSupport>(&tables).into_iter().filter(|s|runs.iter().any(|r|r.id()==s.run&&r.provider==ruff.id())).map(|s|s.assertion).collect::<Vec<_>>();
    let source=&files("native_lexical")["cases.py"];
    let captured_read=occurrences.iter().find(|o|o.role==OccurrenceRole::Read&&source.get(o.start as usize..o.end as usize)==Some(b"captured".as_slice())&&rows::<SourceArtifact>(&tables).iter().any(|a|a.id()==o.source&&a.path=="cases.py")).unwrap();
    assert!(!rows::<LexicalResolution>(&tables).iter().any(|r|r.read==captured_read.id()&&native_resolutions.contains(&r.id())),"different native scope must not invent capture=false");
}
#[tokio::test]
async fn native_correspondence_refuses_missing_support_and_shared_validator_checks_links(){
    let tables=fixture().await;
    for omit in [RuffBindingSupport::NAME,RuffContextSupport::NAME]{let (_,out)=normalize(&tables,Some(omit));assert!(!out.reference_binding_characterizations.is_empty());assert!(out.reference_binding_characterizations.iter().all(|c|c.status==ResolutionStatus::Unresolved));}
    let (_,out)=normalize(&tables,Some(RuffDefinitionSupport::NAME));assert!(!out.declaration_native_characterizations.is_empty());assert!(out.declaration_native_characterizations.iter().all(|c|c.status==ResolutionStatus::Unresolved));
    let (data,out)=normalize(&tables,None);let budget=ResourceBudget::fixed(256<<20).unwrap();
    for omit in [None,Some(ReferenceBindingCharacterization::NAME),Some(DeclarationNativeCharacterization::NAME)]{
        let mut check=(relation_normalization::invariants().remove(0).create)(&budget);
        macro_rules! facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.facts.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_entity_inputs!(facts);
        macro_rules! entity {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_entity_outputs!(entity);
        macro_rules! input {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_relation_inputs!(input);
        macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(if omit!=Some(<$ty>::NAME){check.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();})*};}
        lctx_model::normalized_relation_outputs!(output);assert_eq!(check.finish().is_ok(),omit.is_none(),"{omit:?}");
    }
}
