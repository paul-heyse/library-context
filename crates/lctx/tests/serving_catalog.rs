//! Actual complete Catalog publication and prepared finite catalog routes.
#[path="fixtures/serving_support.rs"] mod support;
use support::*;
use lctx_model::domain::{selection::*,serving::*};
#[tokio::test]
async fn complete_find_cursors_selection_browse_counts_and_caller_order(){
    let fixture=ServingFixture::start(SOURCE).await;
    let execution=fixture.service.execution().await.unwrap();let library=Name::new("demo").unwrap();
    let all=fixture.catalog.find(&execution,&FindOperationsRequest{library:library.clone(),selection:SelectionInput::default(),page:PageRequest{size:100,..Default::default()}}).await.unwrap();
    assert!(all.supported.items.iter().any(|r|r.name.as_str()=="demo.api"));assert!(all.unresolved.items.is_empty()&&all.conflicting.items.is_empty());
    assert!(all.supported.items.iter().all(|r|r.joint==JointApplicability::IndependentRecords));
    let mut request=FindOperationsRequest{library:library.clone(),selection:SelectionInput::default(),page:PageRequest{size:1,..Default::default()}};
    let first=fixture.catalog.find(&execution,&request).await.unwrap();let token=first.supported.continuation.0.clone().unwrap();request.page.cursor=Optional(Some(token));request.page.size=100;
    let next=fixture.catalog.find(&execution,&request).await.unwrap();let mut joined=first.supported.items;joined.extend(next.supported.items);assert_eq!(joined,all.supported.items);
    request.library=Name::new("foreign").unwrap();assert!(fixture.catalog.find(&execution,&request).await.is_err());
    let selection=Selection{requirements:vec![Requirement{predicate:Predicate::DeclaresParameter{name:"flag".into()},quantifier:Quantifier::AnyApplicable}],mode:Mode::Strict,joint:JointPolicy::IndependentRecords};
    let strict=fixture.catalog.find(&execution,&FindOperationsRequest{library:library.clone(),selection:SelectionInput(selection.clone()),page:PageRequest{size:100,..Default::default()}}).await.unwrap();
    let api=strict.supported.items.iter().find(|r|r.name.as_str()=="demo.api").expect("api supports flag");assert!(!api.requirements[0].claims.is_empty());assert!(api.requirements[0].claims.iter().all(|c|c.basis==EvidenceBasis::SourceDeclaration));assert!(api.requirements[0].claims.iter().any(|c|!c.positive.is_empty()));assert!(!strict.supported.items.iter().any(|r|r.name.as_str()=="demo.consume"));assert!(strict.unresolved.items.is_empty()&&strict.conflicting.items.is_empty());
    let browse=fixture.catalog.browse(&execution,&BrowseLibraryRequest{library:library.clone(),scope:BrowseScope::Library{},view:BrowseView::Modules,selection:SelectionInput(selection.clone()),page:PageRequest{size:100,..Default::default()}}).await.unwrap();
    let counted:u64=browse.entries.items.iter().map(|r|match r{BrowseEntry::Module{members,..}=>*members,_=>panic!("wrong browse view")}).sum();assert_eq!(counted,strict.supported.items.len() as u64);
    let compare=fixture.catalog.compare(&execution,&CompareOperationsRequest{library:library.clone(),operations:vec![path("demo.consume"),path("demo.api"),path("demo.absent")],selection:SelectionInput(selection),page:PageRequest::default()}).await.unwrap();
    assert_eq!(compare.operations.len(),3);assert_eq!(compare.operations[0].candidates[0].name.as_str(),"demo.consume");assert!(compare.operations[0].candidates[0].requirements.iter().any(|r|r.outcome==Outcome::Contradicted));assert!(compare.operations[2].candidates.is_empty());
    drop(execution);let memory=fixture.service.memory_reserved();for _ in 0..3{let request_execution=fixture.service.execution().await.unwrap();fixture.catalog.find(&request_execution,&FindOperationsRequest{library:library.clone(),selection:SelectionInput::default(),page:PageRequest::default()}).await.unwrap();drop(request_execution);}assert_eq!(fixture.service.memory_reserved(),memory,"prepared metadata is shared between requests");
    fixture.finish().await;
}
