//! Actual native target identities survive aliasing, re-export and reassignment.
#[path="typed_driver/mod.rs"] mod typed_driver;
use lctx_model::domain::{*,calls::*,types::*,source::*};
use typed_driver::{files,rows};
inspector!(Facts,ProviderCallSite);
#[tokio::test]
async fn aliases_reexports_rebinding_receivers_and_failed_overload_traces_remain_distinct(){
 let files=files("native_usage");let tables=typed_driver::Tables::default();typed_driver::run(&files,Facts(tables.clone())).await.unwrap();
 let occurrences=rows::<Occurrence>(&tables);let artifacts=rows::<SourceArtifact>(&tables);
 let source=|site|{let o=occurrences.iter().find(|o|o.id()==site).unwrap();let a=artifacts.iter().find(|a|a.id()==o.source).unwrap();std::str::from_utf8(&files[&a.path][o.start as usize..o.end as usize]).unwrap()};
 let targets=rows::<CallTarget>(&tables);let destinations=rows::<CallDestination>(&tables);let symbols=rows::<ProviderSymbol>(&tables);
 let names=|call:&str|targets.iter().filter(|t|source(t.site)==call).filter_map(|t|destinations.iter().find(|d|d.id()==t.destination).unwrap().symbol()).map(|id|symbols.iter().find(|s|s.id()==id).unwrap().name.clone()).collect::<Vec<_>>();
 for call in ["aliased(1)","forwarded('text')"]{assert!(names(call).iter().any(|n|n=="parse"),"{call}: {:?}",names(call));}
 assert!(targets.iter().filter(|t|source(t.site)=="rebound(2)").any(|t|destinations.iter().any(|d|d.id()==t.destination&&matches!(d,CallDestination::Unresolved{reason:obligation::ObligationKind::UnresolvedTarget,native:Some(PysaUnresolvedReason::UnexpectedDefiningClass)}))),"native overloaded local alias remains unresolved");
 assert!(names("simple(6)").iter().any(|n|n=="replacement"));
 assert!(names("rebound(3)").iter().any(|n|n=="replacement"));assert!(!names("rebound(3)").iter().any(|n|n=="parse"));
 assert!(targets.iter().filter(|t|source(t.site)=="box.run(4)").any(|t|rows::<Receiver>(&tables).iter().any(|r|r.id()==t.receiver&&matches!(r,Receiver::Bound{actual} if source(*actual)=="box"))));
 assert!(targets.iter().filter(|t|source(t.site)=="unknown(5)").any(|t|destinations.iter().any(|d|d.id()==t.destination&&matches!(d,CallDestination::Unresolved{..}))));
 assert!(targets.iter().filter(|t|source(t.site)=="apply(replacement, 7)").any(|t|rows::<CallChannel>(&tables).iter().any(|c|c.id()==t.channel&&matches!(c,CallChannel::HigherOrder{argument_index:0}))&&rows::<assertion::AssertionQualification>(&tables).iter().any(|q|q.id()==t.qualification&&q.modality==attribution::Modality::Potential)),"actual callback target stays potential");
 let types=rows::<TypeObservation>(&tables);assert!(types.iter().any(|t|source(t.subject)=="aliased(1)"&&t.role==TypeRole::ChosenOverload));
 assert!(types.iter().any(|t|source(t.subject)=="aliased(object())"&&t.role==TypeRole::OverloadCandidates));assert!(!types.iter().any(|t|source(t.subject)=="aliased(object())"&&t.role==TypeRole::ChosenOverload));
}

#[tokio::test]
async fn native_usage_reaches_c2_and_refuses_missing_support_or_event_correspondence(){
 use lctx_model::domain::{normalized::{entity_normalization,event_normalization},catalog::evidence::build,analysis::native::NativeInventory,resources::ResourceBudget};
 let tables=typed_driver::Tables::default();typed_driver::run(&files("native_usage"),Facts(tables.clone())).await.unwrap();let budget=ResourceBudget::fixed(512<<20).unwrap();
 let mut raw=entity_normalization::EntityData::new(&budget);
 macro_rules! facts{($($f:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables){raw.$f.insert(row).unwrap();})*};}lctx_model::normalized_entity_inputs!(facts);
 let entities=entity_normalization::normalize(raw.inputs(),&budget).unwrap();let mut data=event_normalization::EventData::new(&budget);
 macro_rules! inputs{($($f:ident:$ty:ty,)*)=>{$(for row in rows::<$ty>(&tables){data.$f.insert(row).unwrap();})*};}lctx_model::normalized_event_inputs!(inputs);
 macro_rules! entity_rows{($($f:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::normalized_entity_outputs!(entity_rows);
 let events=event_normalization::normalize(&data,&budget).unwrap();
 let prepare=|omit_support:bool|{let mut d=build::EvidenceData::new(&budget);
 macro_rules! entity_evidence{($($f:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::normalized_entity_outputs!(entity_evidence);
 for(name,batch)in tables.lock().unwrap().iter(){if !omit_support||*name!=ProviderCallSiteSupport::NAME{d.visit(name,batch).unwrap();}}
 macro_rules! events{($($f:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&events.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::normalized_event_outputs!(events);
 let mut native=NativeInventory::new(&budget);for input in NativeInventory::inputs(){if let Some(batch)=tables.lock().unwrap().get(input.name()){native.visit(input.name(),batch).unwrap();}}for row in native.collect().unwrap().premises.iter(){d.facts.characterization_native.insert(row.clone()).unwrap();}d};
 let d=prepare(false);let out=build::build(&d,&budget).unwrap();assert!(!out.source_usages.is_empty());assert!(out.source_usages.iter().all(|u|out.source_characterizations.get(u.characterization).is_some()));
 let without=prepare(true);assert!(build::build(&without,&budget).unwrap().source_usages.is_empty());
 let mut malformed=prepare(false);let source=malformed.facts.usage_event_sources.iter().next().unwrap().clone();let wrong=malformed.facts.events.iter().find(|e|e.id()!=source.event&&e.site!=malformed.facts.events.get(source.event).unwrap().site).unwrap().id();malformed.facts.usage_event_sources.insert(normalized::events::CallEventSource{event:wrong,observation:source.observation}).unwrap();assert!(build::build(&malformed,&budget).is_err(),"wrong actual event correspondence cannot become source usage");
}
