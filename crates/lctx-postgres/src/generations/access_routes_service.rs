//! Opt-in captured public-name route hydration; semantic traversal stays in the model owner.
use super::{CatalogService,Error,RequestExecution,packet_reads::PacketLease};
use super::catalog_service::{retain,name};
use lctx_model::domain::{*,catalog::access_routes::RouteData,serving::*};
use std::collections::BTreeSet;
impl CatalogService {
 pub(super) async fn access_route_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<AccessRoutePacket>,Error>{
  let this=self.clone();let retained=e.clone();let request=r.clone();
  let prepared=self.prepared();let d=prepared.data();
  let selected=d.source.catalog.members.get(member).ok_or(Error::Contract)?;
  let _metadata=e.budget().reserve("access-route-hydration-selectors",d.source.core.artifacts.len().saturating_add(d.source.core.occurrences.len()).saturating_add(d.source.core.binding_events.len()).saturating_add(d.source.core.exposures.len()).saturating_mul(256))?;
  let module_scope=catalog::access_routes::hydration_modules(d,member,e.budget())?;
  let artifacts=module_scope.value.iter().map(|id|d.source.core.modules.get(*id).map(|m|m.source).ok_or(Error::Contract)).collect::<Result<BTreeSet<_>,_>>()?;
  let _=selected;
  let aliases=d.source.core.occurrences.iter().filter(|o|artifacts.contains(&o.source)&&o.syntax_kind==source::SyntaxKind::Alias).map(Record::id).collect::<Vec<_>>();
  let events=d.source.core.binding_events.iter().filter(|event|d.source.core.occurrences.get(event.site).is_some_and(|o|artifacts.contains(&o.source))).map(Record::id).collect::<Vec<_>>();
  let names=d.source.core.exposures.iter().filter(|exposure|d.source.core.modules.get(exposure.access).is_some_and(|m|artifacts.contains(&m.source))).map(|exposure|exposure.observation).collect::<Vec<_>>();
  let classes=d.source.core.refs.iter().filter_map(|e|match e {normalized::entities::EntityRef::Class{class}=>Some(*class),_=>None}).collect::<Vec<_>>();
  let budget=e.budget().clone();
  let extra=e.query(move|lease|Box::pin(async move{
   let mut scope=PacketLease::new::<AccessRoutePacket>(lease);let mut extra=RouteData::new(&budget);
   macro_rules! take {($field:ident,$batch:expr)=>{{let batch=$batch;for row in batch.rows(){extra.$field.insert(row.clone())?;}}};}
   take!(classes,scope.read_ids::<normalized::entities::ClassEntity>(&classes).await?);
   take!(imports,scope.read_for::<syntax::ImportAliasObservation,source::Occurrence>("alias",&aliases).await?);
   let import_ids=extra.imports.iter().map(Record::id).collect::<Vec<_>>();
   take!(assessments,scope.read_for::<normalized::links::ImportModuleAssessment,syntax::ImportAliasObservation>("observation",&import_ids).await?);
   let assessment_ids=extra.assessments.iter().map(Record::id).collect::<Vec<_>>();
   take!(candidates,scope.read_for::<normalized::links::ImportModuleCandidate,normalized::links::ImportModuleAssessment>("assessment",&assessment_ids).await?);
   let resolution_ids=extra.candidates.iter().map(|c|c.observation).collect::<Vec<_>>();
   take!(resolutions,scope.read_ids::<symbols::ModuleResolutionObservation>(&resolution_ids).await?);
   take!(resolution_supports,scope.read_for::<symbols::ModuleResolutionSupport,symbols::ModuleResolutionObservation>("assertion",&resolution_ids).await?);
   take!(native_bindings,scope.read_for::<ruff::RuffBindingObservation,lexical::BindingEvent>("event",&events).await?);
   let binding_ids=extra.native_bindings.iter().map(Record::id).collect::<Vec<_>>();
   take!(native_supports,scope.read_for::<ruff::RuffBindingSupport,ruff::RuffBindingObservation>("assertion",&binding_ids).await?);
   take!(names,scope.read_ids::<symbols::PublicNameObservation>(&names).await?);
   take!(name_supports,scope.read_for::<symbols::PublicNameSupport,symbols::PublicNameObservation>("assertion",&names).await?);
   let origins=extra.names.iter().map(|n|n.origin).collect::<Vec<_>>();
   take!(origins,scope.read_ids::<symbols::ExportOrigin>(&origins).await?);
   let _support_ids=budget.reserve("access-route-support-selectors",extra.native_supports.len().saturating_add(extra.resolution_supports.len()).saturating_add(extra.name_supports.len()).saturating_mul(1024))?;
   let attributions=extra.native_supports.iter().filter_map(assertion::Support::attribution).chain(extra.resolution_supports.iter().filter_map(assertion::Support::attribution)).chain(extra.name_supports.iter().filter_map(assertion::Support::attribution)).collect::<Vec<_>>();
   let run_ids=attributions.iter().map(|a|a.run).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
   let surface_ids=attributions.iter().map(|a|a.surface).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
   let evidence_ids=attributions.iter().map(|a|a.evidence).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
   take!(runs,scope.read_ids::<attribution::ProviderRun>(&run_ids).await?);
   take!(surfaces,scope.read_ids::<assertion::ProviderSurface>(&surface_ids).await?);
   take!(evidence,scope.read_ids::<assertion::Evidence>(&evidence_ids).await?);
   let provider_ids=extra.runs.iter().map(|r|r.provider).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
   take!(providers,scope.read_ids::<attribution::Provider>(&provider_ids).await?);
   Ok(extra)
  })).await?;
  e.cpu(move|budget|{
   let result=catalog::access_routes::explain(this.prepared().data(),&extra,member,16,128,budget)?;
   let mut page=this.section_page(&request,member,"access_routes",20,result.routes,true)?;
   if result.partial{page.availability=Availability::Partial{reason:name("public name route correspondence or finite search incomplete")?};}
   retain(&retained,&page)?;Ok(page)
  }).await
 }
}
