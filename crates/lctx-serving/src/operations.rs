//! Native operation orchestration over scoped semantic owners.
use lctx_model::domain::{*,resources::ResourceBudget,serving::*,serving::ranking::{self,CandidateFusion,ChannelBinding,RankingPolicy}};
use lctx_surrealdb::{NativeReader,RecordSelection};
use crate::{records::{need,wire},candidates,service::QueryVector};
fn inputs(domains:&[LibraryDomainPacket])->Vec<[u8;16]>{domains.iter().flat_map(|d|&d.captures).flat_map(|c|std::iter::once(c.release.input).chain(c.corpora.iter().copied())).map(|i|*i.bytes()).collect::<std::collections::BTreeSet<_>>().into_iter().collect()}
fn quiet()->ChannelState{ChannelState{lexical:false,vector:VectorChannel::Disabled{}}}
fn page<T>(values:Vec<(ContentHash,T)>,request:&Request,reader:&NativeReader,group:&str,section:&str,member:Option<Id<catalog::CatalogMember>>,availability:Availability)->Result<SectionPage<T>,ModelError>{crate::pagination::page(values,request,reader.handle(),&quiet(),group,section,member,availability).map_err(wire)}
fn extent(selected:&selection::evaluate::Selected)->SelectionExtent{SelectionExtent::CompleteDomain{total:selected.candidates.len() as u64}}
async fn chosen(reader:&NativeReader,library:&Name,selector:Option<&OperationSelector>,selection:&selection::Selection,b:&ResourceBudget)->Result<crate::selection::MemberSelection,ModelError>{let members=crate::selection::members(reader,Some(library),selector).await?;crate::selection::classify(reader,&members,selection,b).await}
pub async fn dispatch(reader:&NativeReader,request:&Request,channels:&ChannelState,vector:Option<&QueryVector>,limits:&ResourceLimits,b:&ResourceBudget)->Result<Response,ModelError>{
 let snapshot=reader.handle().clone();
 match request{
  Request::GetCapability(r)=>Ok(Response::GetCapability(GetCapabilityResponse{snapshot,capability:crate::capability::get(reader,r.capability,b).await?})),
  Request::GetEvidence(r)=>Ok(Response::GetEvidence(GetEvidenceResponse{snapshot,evidence:crate::evidence::get(reader,&r.source,request,channels,limits,b).await?})),
  Request::InspectValuePaths(r)=>crate::inspection::get(reader,r,request,limits,b).await.map(Response::InspectValuePaths),
  Request::FindOperations(r)=>{
   let domains=crate::library::resolve(reader,Some(&r.library),b).await?;let data=chosen(reader,&r.library,None,&r.selection.0,b).await?;
   let group=|outcome,group|{let values=data.selected.group(outcome).filter(|_|r.selection.0.mode==selection::Mode::Discovery || outcome==selection::Outcome::Supported).map(|c|{let p=candidates::packet(c,data.prepared.data(),&domains)?;Ok((candidates::key(&p),p))}).collect::<Result<Vec<_>,ModelError>>()?;page(values,request,reader,group,"results",None,Availability::Available{})};
   Ok(Response::FindOperations(FindOperationsResponse{snapshot,domains:domains.clone(),supported:group(selection::Outcome::Supported,"supported")?,unresolved:group(selection::Outcome::Unresolved,"unresolved")?,conflicting:group(selection::Outcome::Conflicting,"conflicting")?,extent:extent(&data.selected)}))
  },
  Request::CompareOperations(r)=>{
   let domains=crate::library::resolve(reader,Some(&r.library),b).await?;let mut operations=Vec::new();
   for selector in &r.operations{let d=chosen(reader,&r.library,Some(selector),&r.selection.0,b).await?;let candidates=d.selected.eligible().map(|c|candidates::packet(c,d.prepared.data(),&domains)).collect::<Result<Vec<_>,_>>()?;let members=candidates.iter().map(|c|c.member).collect::<std::collections::BTreeSet<_>>();operations.push(ComparisonEntry{requested:selector.clone(),candidates,ambiguous:members.len()>1});}
   Ok(Response::CompareOperations(CompareOperationsResponse{snapshot,domains,operations}))
  },
  Request::GetOperation(r)=>{
   let domains=crate::library::resolve(reader,Some(&r.library),b).await?;let members=crate::selection::members(reader,Some(&r.library),Some(&r.operation)).await?;
   let operation=match members.as_slice(){[]=>OperationResolution::Missing{coverage:Availability::Partial{reason:Name::new("captured_public_domain_only").map_err(wire)?}},[member]=>OperationResolution::Unique{packet:crate::operation_packet::get(reader,member,&domains,r,request,limits,b).await?},_=>{let d=crate::selection::classify(reader,&members,&selection::Selection::default(),b).await?;OperationResolution::Ambiguous{candidates:d.selected.eligible().map(|c|candidates::packet(c,d.prepared.data(),&domains)).collect::<Result<Vec<_>,_>>()?}}};
   Ok(Response::GetOperation(GetOperationResponse{snapshot,domains,operation}))
  },
  Request::BrowseLibrary(r)=>browse(reader,r,request,b).await.map(Response::BrowseLibrary),
  Request::SearchOperations(r)=>search_operations(reader,r,request,channels,vector,b).await.map(Response::SearchOperations),
  Request::SearchEvidence(r)=>search_evidence(reader,r,request,channels,vector,b).await.map(Response::SearchEvidence),
  Request::SearchCapabilities(r)=>search_capabilities(reader,r,request,channels,vector,b).await.map(Response::SearchCapabilities),
 }
}
async fn scores(reader:&NativeReader,query:&str,families:&[retrieval::Family],domains:&[LibraryDomainPacket],pairs:Option<&[([u8;16],[u8;16])]>,member_mode:bool,units:Option<&[Id<retrieval::Unit>]>,vector:Option<&QueryVector>)->Result<Vec<ranking::CandidateScore>,ModelError>{
 let mut scores=Vec::new();let policy=RankingPolicy::default();let inputs=inputs(domains);
 for family in families{scores.extend(crate::search::lexical(reader,query,*family,&inputs,pairs,member_mode,units,100,&policy).await?);if let Some(v)=vector{scores.extend(crate::search::vector(reader,&v.vector,v.spec,embedding::value::value_digest(&v.vector),*family,&inputs,pairs,member_mode,units,100,&policy).await?);}}
 Ok(scores)
}
fn fusion(reader:&NativeReader,query:&str,vector:Option<&QueryVector>,scores:&[ranking::CandidateScore],promoted:&[Id<catalog::CatalogMember>],b:&ResourceBudget)->Result<Vec<ranking::RankedHit>,ModelError>{
 let policy=RankingPolicy::default();let mut channels=vec![ChannelBinding::lexical(&policy,query)?];if let Some(v)=vector{channels.push(ChannelBinding::vector(&policy,v.spec,embedding::value::value_digest(&v.vector))?);}
 let mut eligible=scores.iter().map(|s|s.occurrence.target).collect::<std::collections::BTreeSet<_>>();eligible.extend(promoted.iter().map(|m|ranking::Target::Member{member:*m}));
 let occurrences=scores.iter().map(|s|s.occurrence).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
 Ok(CandidateFusion::new(reader.handle().clone(),policy,&channels,&eligible.into_iter().collect::<Vec<_>>(),&occurrences,b)?.rank(scores,promoted)?.rows().to_vec())
}
async fn search_operations(reader:&NativeReader,r:&SearchOperationsRequest,request:&Request,channels:&ChannelState,vector:Option<&QueryVector>,b:&ResourceBudget)->Result<SearchOperationsResponse,ModelError>{
 let domains=crate::library::resolve(reader,r.library.0.as_ref(),b).await?;
 let members=crate::selection::members(reader,r.library.0.as_ref(),None).await?;let selected=crate::selection::classify(reader,&members,&r.selection.0,b).await?;
 let pairs=selected.selected.eligible().map(|c|(*c.member.bytes(),*c.analysis.bytes())).collect::<Vec<_>>();
 let scores=scores(reader,r.query.as_str(),&[retrieval::Family::ApiOptions,retrieval::Family::DocumentationDeployment,retrieval::Family::Scenario,retrieval::Family::Source],&domains,Some(&pairs),true,None,vector).await?;
 let promoted=selected.selected.eligible().filter(|c|c.path.join(".")==r.query.as_str() || selected.prepared.data().source.catalog.members.get(c.member).is_some_and(|m|m.name==r.query.as_str())).map(|c|c.member).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
 let ranked=fusion(reader,r.query.as_str(),vector,&scores,&promoted,b)?;let mut values=Vec::new();
 for hit in ranked{let ranking::Target::Member{member}=hit.target else{return Err(ModelError::Schema("member ranking target"))};let matched=selected.selected.eligible().filter(|c|c.member==member && (promoted.contains(&member)||hit.witnesses.iter().any(|w|w.occurrence.context==c.analysis))).collect::<Vec<_>>();for c in matched{let p=candidates::packet(c,selected.prepared.data(),&domains)?;values.push((hit.clone(),candidates::key(&p),p));}}
 let (results,ranking)=crate::pagination::ranked(values,request,reader.handle(),channels).map_err(wire)?;
 Ok(SearchOperationsResponse{snapshot:reader.handle().clone(),domains,extent:SelectionExtent::Ranked{returned:results.items.len() as u64},results,channels:channels.clone(),ranking})
}
async fn search_evidence(reader:&NativeReader,r:&SearchEvidenceRequest,request:&Request,channels:&ChannelState,vector:Option<&QueryVector>,b:&ResourceBudget)->Result<SearchEvidenceResponse,ModelError>{
 let domains=crate::library::resolve(reader,r.library.0.as_ref(),b).await?;let families=if r.families.is_empty(){[retrieval::Family::ApiOptions,retrieval::Family::DocumentationDeployment,retrieval::Family::Scenario,retrieval::Family::Source].to_vec()}else{r.families.clone()};
 let scores=scores(reader,r.query.as_str(),&families,&domains,None,false,None,vector).await?;let ranked=fusion(reader,r.query.as_str(),vector,&scores,&[],b)?;let mut values=Vec::new();
 for hit in ranked{let ranking::Target::Unit{unit}=hit.target else{return Err(ModelError::Schema("evidence ranking target"))};values.push((hit,crate::pagination::target_key(ranking::Target::Unit{unit}),crate::evidence::hit(reader,unit,&domains,b).await?));}
 let (results,ranking)=crate::pagination::ranked(values,request,reader.handle(),channels).map_err(wire)?;
 Ok(SearchEvidenceResponse{snapshot:reader.handle().clone(),domains,extent:SelectionExtent::Ranked{returned:results.items.len() as u64},results,channels:channels.clone(),ranking})
}
async fn search_capabilities(reader:&NativeReader,r:&SearchCapabilitiesRequest,request:&Request,channels:&ChannelState,vector:Option<&QueryVector>,b:&ResourceBudget)->Result<SearchCapabilitiesResponse,ModelError>{
 let domains=crate::library::resolve(reader,r.library.0.as_ref(),b).await?;
 let mut vars=surrealdb::types::Variables::new();vars.insert("inputs",surrealdb::types::SerdeWrapper(inputs(&domains)));
 let keys:Vec<String>=reader.query("RETURN { LET $origins=SELECT VALUE id FROM entity WHERE semantic_type='retrieval_origins' AND body.kind=6; RETURN SELECT VALUE semantic_key FROM entity WHERE semantic_type='retrieval_units' AND scope_keys CONTAINSANY $inputs.map(|$v|'retrieval_units|input|'+<string>$v) AND id IN (SELECT VALUE in FROM reference WHERE field='origin' AND out IN $origins); };",vars).await?;
 let units=keys.iter().map(|key|{let bytes=hex::decode(key).map_err(ModelError::codec)?;serde_json::from_value::<Id<retrieval::Unit>>(serde_json::to_value(bytes).map_err(ModelError::codec)?).map_err(ModelError::codec)}).collect::<Result<Vec<_>,_>>()?;
 let scores=scores(reader,r.query.as_str(),&[retrieval::Family::ApiOptions],&domains,None,false,Some(&units),vector).await?;let ranked=fusion(reader,r.query.as_str(),vector,&scores,&[],b)?;let mut values=Vec::new();let mut seen=std::collections::BTreeSet::new();
 for hit in ranked{let ranking::Target::Unit{unit}=hit.target else{return Err(ModelError::Schema("capability ranking target"))};let records=reader.records::<retrieval::Unit>(RecordSelection::Keys(vec![*unit.bytes()])).await?;let u=need(&records,unit)?;let origins=reader.records::<retrieval::Origin>(RecordSelection::Keys(vec![*u.origin.bytes()])).await?;if let retrieval::Origin::Brief{brief}=need(&origins,u.origin)?{if seen.insert(*brief){values.push((hit,match graph::target_for_row(derivation::RowRef::of(*brief))?{graph::Target::Assertion(id)=>id.0,graph::Target::Entity(id)=>id.0,_=>return Err(ModelError::Schema("brief graph key"))},crate::capability::get(reader,*brief,b).await?));}}}
 let (results,ranking)=crate::pagination::ranked(values,request,reader.handle(),channels).map_err(wire)?;
 Ok(SearchCapabilitiesResponse{snapshot:reader.handle().clone(),domains,extent:SelectionExtent::Ranked{returned:results.items.len() as u64},results,channels:channels.clone(),ranking})
}
async fn browse(reader:&NativeReader,r:&BrowseLibraryRequest,request:&Request,b:&ResourceBudget)->Result<BrowseLibraryResponse,ModelError>{
 let domains=crate::library::resolve(reader,Some(&r.library),b).await?;let d=chosen(reader,&r.library,None,&r.selection.0,b).await?;let data=d.prepared.data();
 let mut unknown=0;let mut values=Vec::new();let eligible=d.selected.eligible().collect::<Vec<_>>();
 for c in eligible{let member=data.source.catalog.members.get(c.member).ok_or(ModelError::Schema("browse member"))?;let include=match r.scope{BrowseScope::Library{}=>true,BrowseScope::Module{module}=>member.access==module,BrowseScope::Class{member:class}=>{let owners=data.source.catalog.paths.iter().filter(|p|data.source.catalog.candidates.iter().filter(|c|c.path==Some(p.id())).any(|candidate|data.source.catalog.exposures.get(candidate.exposure).is_some_and(|e|e.member==c.member))).collect::<Vec<_>>();if owners.is_empty(){unknown+=1;}owners.iter().any(|p|data.source.catalog.candidates.get(p.parent).and_then(|c|data.source.catalog.exposures.get(c.exposure)).is_some_and(|e|e.member==class))}};if !include{continue}
  let p=candidates::packet(c,data,&domains)?;
  match r.view{
   BrowseView::Members=>values.push((candidates::key(&p),BrowseEntry::Member{candidate:p,ownership:if matches!(r.scope,BrowseScope::Class{..}){Availability::Available{}}else{Availability::NotRequested{}}})),
   BrowseView::Classes=>if data.source.catalog.classes.iter().any(|cl|cl.member==member.id()){values.push((graph::EntityId::of(member.id()).0,BrowseEntry::Class{member:member.id(),name:p.name,members:data.source.catalog.paths.iter().filter(|p|data.source.catalog.candidates.get(p.parent).and_then(|c|data.source.catalog.exposures.get(c.exposure)).is_some_and(|e|e.member==member.id())).count() as u64}));},
   BrowseView::Modules=>{let module=data.source.core.modules.get(member.access).ok_or(ModelError::Schema("browse module"))?;let key=graph::EntityId::of(module.id()).0;if !values.iter().any(|(k,_)|k==&key){values.push((key,BrowseEntry::Module{module:module.id(),name:Name::new(module.qualified_name.clone()).map_err(wire)?,members:d.selected.eligible().filter(|c|data.source.catalog.members.get(c.member).is_some_and(|m|m.access==module.id())).count() as u64}));}},
   BrowseView::Vocabulary=>{},
  }
 }
 if r.view==BrowseView::Vocabulary{
  let mut facets=std::collections::BTreeMap::<selection::Facet,(std::collections::BTreeSet<Name>,std::collections::BTreeSet<Id<catalog::CatalogMember>>)>::new();
  for (facet,text,predicate) in crate::vocabulary::questions(data){let requirement=selection::Requirement{predicate,quantifier:selection::Quantifier::AnyApplicable};for c in d.selected.eligible(){let result=d.prepared.classify(c.member,c.analysis,&requirement,b)?;if result.outcome==selection::Outcome::Supported{let entry=facets.entry(facet).or_default();entry.0.insert(Name::new(text.clone()).map_err(wire)?);entry.1.insert(c.member);}}}
  for (facet,(names,members)) in facets{values.push((ContentHash::of(format!("native-vocabulary:{facet:?}").as_bytes()),BrowseEntry::Vocabulary{facet,values:names.into_iter().collect(),members:members.len() as u64}));}
 }
 Ok(BrowseLibraryResponse{snapshot:reader.handle().clone(),domains,scope:r.scope.clone(),view:r.view,entries:page(values,request,reader,request.tool().name(),"entries",None,Availability::Available{})?,extent:extent(&d.selected),unknown_ownership:unknown})
}
