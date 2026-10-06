//! Named analytical frames reuse compact edges; rich evidence belongs to one frame lifetime.
use crate::{consumed_rows::{ClosureTable,NominalClosure,PreparedEdges,PreparedClosure,identifier},workspace::CompletedInputs};
use lctx_model::domain::*;
use std::{any::TypeId,sync::Arc};
#[derive(Clone,Copy)]
pub(super) enum Kind {Structural,Analytic}
pub(super) struct FrameScopes {inputs:Vec<ValidationInput>,edges:PreparedEdges,root:usize,_charge:charged::StateCharge}
impl FrameScopes {
 pub(super) async fn prepare(access:&CompletedInputs,session:&datafusion::prelude::SessionContext,model:&Arc<ValidatedModel>,inputs:Vec<ValidationInput>,kind:Kind,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
  let tables=inputs.iter().map(|input|Ok(ClosureTable{relation:model.relation(input.name()).ok_or(ModelError::Schema(input.name()))?.clone(),alias:access.table_for(input)?})).collect::<Result<Vec<_>,ModelError>>()?;
  Self::bound(inputs,tables,session,kind,budget).await
 }
 async fn bound(inputs:Vec<ValidationInput>,mut tables:Vec<ClosureTable>,session:&datafusion::prelude::SessionContext,kind:Kind,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
  let mut structural_types=std::collections::BTreeSet::new();
  macro_rules! structural_types {($($field:ident:$ty:ty,)*)=>{$(structural_types.insert(TypeId::of::<$ty>());)*};}
  lctx_model::structural_outputs!(structural_types);
  let target_for=|from:usize,target:TypeId|->Result<Option<usize>,ModelError>{
   let candidates=inputs.iter().enumerate().filter(|(_,input)|input.type_id()==target).map(|(i,_)|i).collect::<Vec<_>>();
   if candidates.is_empty(){return Ok(None);} if let [only]=candidates.as_slice(){return Ok(Some(*only));}
   let epoch=inputs[from].prefix().unwrap_or_else(||{
    if [TypeId::of::<local_semantics::LocalContribution>(),TypeId::of::<local_semantics::LocalAssessment>()].contains(&inputs[from].type_id()){stages::PublicationBoundary::Local}
    else if structural_types.contains(&inputs[from].type_id()){stages::PublicationBoundary::Structural}
    else{stages::PublicationBoundary::Facts}
   });
   let matches=candidates.into_iter().filter(|i|inputs[*i].prefix()==Some(epoch)).collect::<Vec<_>>();
   match matches.as_slice(){[only]=>Ok(Some(*only)),[]=>Err(ModelError::Conflict("analytical dependency epoch absent")),_=>Err(ModelError::Conflict("analytical dependency epoch ambiguous"))}
  };
  let real=tables.len();let index=|kind:TypeId|tables[..real].iter().position(|table|table.relation.type_id()==kind);
  let owner=match kind {Kind::Structural=>TypeId::of::<analysis::catalog_core::Invocation>(),Kind::Analytic=>TypeId::of::<structural::StructuralFrame>()};
  let source=index(owner).ok_or(ModelError::Schema("analytical frame root"))?;
  let root=real;tables.push(tables[source].clone());let mut plan=NominalClosure::new(tables.clone())?;
  // Exact vocabulary epoch is chosen before following an edge. Dependency rows cannot become
  // publication roots merely because another frame shares their module, artifact or catalog.
  for (from,table) in tables[..real].iter().enumerate(){for field in table.relation.fields(){if let Some((target,_))=field.target() && let Some(to)=target_for(from,target)?{
   if field.list(){plan.pairs(from,to,format!("SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",identifier(field.name()),identifier(&table.alias)))?;}else{plan.follow(from,field.name(),to)?;}
  }}}
  let idx=|kind:TypeId|tables[..real].iter().position(|table|table.relation.type_id()==kind);
  let alias=|kind:TypeId|idx(kind).map(|i|identifier(&tables[i].alias));
  let roots=identifier(&tables[root].alias);
  plan.pairs(root,source,format!("SELECT id AS source_id,id AS target_id FROM {roots}"))?;
  macro_rules! own {($member:ty,$field:literal,$owner:ty)=>{for(member,input)in inputs.iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<$member>()){let _=input;if let Some(owner)=target_for(member,TypeId::of::<$owner>())?{plan.own(member,$field,owner)?;}}};}
  // Native supports and ordered list members are semantic ownership, not arbitrary incoming refs.
  for (member,table)in tables[..real].iter().enumerate(){if let Some(field)=table.relation.fields().iter().find(|field|field.name()=="assertion") && let Some((target,_))=field.target() && let Some(owner)=target_for(member,target)?{plan.own(member,field.name(),owner)?;}}
  own!(input::ArtifactUse,"artifact",source::SourceArtifact);
  own!(normalized::entities::OccurrenceOwnership,"occurrence",source::Occurrence);
  own!(normalized::entities::SymbolEntityResolution,"symbol",calls::ProviderSymbol);
  own!(normalized::entities::ParameterEntityLink,"entity",normalized::entities::ParameterEntity);
  own!(normalized::entities::ParameterEntityLink,"parameter",calls::SignatureParameter);
  own!(types::TypeSequenceMember,"sequence",types::TypeSequence);
  own!(types::TypedDictField,"list",types::TypedDictFieldList);
  own!(types::CallableParameter,"list",types::CallableParameterList);
  own!(symbols::SymbolSequenceMember,"sequence",symbols::SymbolSequence);
  own!(assumptions::AssumptionSetMember,"set",assumptions::AssumptionSet);
  own!(calls::SignatureParameter,"signature",calls::Signature);
  own!(normalized::callables::SignatureSlot,"variant",normalized::callables::SignatureVariant);
  own!(normalized::callables::SignatureSlotEntity,"slot",normalized::callables::SignatureSlot);
  own!(normalized::callables::EffectiveCallableEvidence,"assessment",normalized::callables::EffectiveCallableAssessment);
  own!(normalized::callables::EffectiveDecoratorMember,"assessment",normalized::callables::EffectiveCallableAssessment);
  own!(syntax::ParameterSyntaxObservation,"function",source::Occurrence);
  own!(declarations::ParameterDeclaration,"declaration",source::Occurrence);
  own!(declarations::ParameterDeclaration,"parameter",calls::SignatureParameter);
  own!(normalized::entities::EntityRef,"callable_callable",normalized::entities::CallableEntity);
  own!(syntax::DeclarationDecorator,"declaration",source::Occurrence);
  own!(types::TypeObservation,"subject",source::Occurrence);
  own!(types::FunctionBodyObservation,"declaration",source::Occurrence);
  own!(types::NativeSignatureObservation,"signature",calls::Signature);
  own!(types::SignatureTypeObservation,"subject",types::SignatureTypeSubject);
  own!(class_metadata::ClassMetadataObservation,"class",calls::ProviderSymbol);
  own!(class_metadata::ClassMemberObservation,"class",calls::ProviderSymbol);
  own!(captures::CaptureObservation,"function",calls::ProviderSymbol);
  own!(protocols::NativeExitObservation,"subject",source::Occurrence);
  own!(protocols::NativeTerminalObservation,"subject",source::Occurrence);
  own!(normalized::events::CallEventSource,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::events::CallEventSourceEvidence,"source",normalized::events::CallEventSource);
  own!(normalized::events::CallEventResolution,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::events::CallEventResolutionEvidence,"resolution",normalized::events::CallEventResolution);
  own!(normalized::events::NormalizedCallAlternative,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::events::CallAlternativeEvidence,"alternative",normalized::events::NormalizedCallAlternative);
  own!(normalized::events::EventAssessment,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::events::EventPhaseTarget,"assessment",normalized::events::EventAssessment);
  own!(normalized::events::CallPolicyAssessment,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::events::CallPolicyAdmission,"assessment",normalized::events::CallPolicyAssessment);
  own!(normalized::bindings::CallBindingAttempt,"event",normalized::events::NormalizedCallEvent);
  own!(normalized::bindings::CallBinding,"attempt",normalized::bindings::CallBindingAttempt);
  own!(flow::FlowUseObservation,"use_",flow::FlowUse);
  own!(flow::FlowReachingObservation,"use_",flow::FlowUse);
  own!(flow::FlowDefinitionObservation,"definition",flow::FlowDefinition);
  own!(flow_inventory::FlowUseInventoryMember,"inventory",flow_inventory::FlowUseInventoryObservation);
  own!(flow_inventory::FlowUseInventoryObservation,"use_",flow::FlowUse);
  own!(flow_inventory::FlowUseCandidate,"inventory",flow_inventory::FlowUseInventoryObservation);
  own!(conditions::entry::EntryValueWitness,"use_observation",flow::FlowUseObservation);
  own!(flow::FlowUse,"occurrence",source::Occurrence);
  own!(flow::FlowDefinition,"occurrence",source::Occurrence);
  own!(catalog::CatalogCallable,"member",catalog::CatalogMember);
  own!(calls::CallArgument,"call",calls::CallSyntax);
  own!(calls::CallResolutionMember,"resolution",calls::CallResolution);
  own!(syntax::DeclarationObservation,"declaration",source::Occurrence);
  own!(declarations::SymbolDeclaration,"declaration",source::Occurrence);
  own!(lexical::ReferenceObservation,"read",source::Occurrence);
  own!(normalized::links::ReferenceEntityAssessment,"reference",lexical::ReferenceObservation);
  own!(normalized::links::ReferenceEntityCandidate,"assessment",normalized::links::ReferenceEntityAssessment);
  own!(symbols::FunctionTraitObservation,"symbol",calls::ProviderSymbol);
  own!(symbols::ClassAncestryObservation,"class",calls::ProviderSymbol);
  own!(calls::SignatureEnumerationObservation,"symbol",calls::ProviderSymbol);
  own!(calls::SignatureEnumerationMember,"enumeration",calls::SignatureEnumerationObservation);
  own!(normalized::callables::SignatureSlotType,"slot",normalized::callables::SignatureSlot);
  for(member,table)in tables[..real].iter().enumerate().filter(|(_,table)|table.relation.type_id()==TypeId::of::<types::SignatureTypeSubject>()){for field in table.relation.fields(){if let Some((kind,_))=field.target() && let Some(owner)=idx(kind){plan.own(member,field.name(),owner)?;}}}

  macro_rules! pair {($from:ty,$to:ty,$sql:expr)=>{if let(Some(from),Some(to))=(idx(TypeId::of::<$from>()),idx(TypeId::of::<$to>())){plan.pairs(from,to,$sql)?;}};}
  let frame_sql=match kind {Kind::Structural=>format!("SELECT id,input,context FROM {roots}"),Kind::Analytic=>format!("SELECT f.id,p.input,p.context FROM {roots} f JOIN {} p ON p.id=f.invocation",alias(TypeId::of::<analysis::structural::Invocation>()).ok_or(ModelError::Schema("analytic parent"))?)};
  let frames=format!("({frame_sql})");
  for ty in [TypeId::of::<analysis::settings::AnalyticsConfiguration>(),TypeId::of::<analysis::AnalysisDefinition>(),TypeId::of::<analysis::MethodParameters>(),TypeId::of::<embedding::text::TextDefinition>(),TypeId::of::<embedding::EmbeddingSpec>(),TypeId::of::<embedding::configuration::ServiceConfiguration>()]{if let Some(target)=idx(ty){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f CROSS JOIN {} t",identifier(&tables[target].alias)))?;}}
  for ty in [TypeId::of::<attribution::ProviderRun>(),TypeId::of::<analysis::local::Invocation>(),TypeId::of::<analysis::analytic_embedding::Invocation>()]{if let Some(target)=idx(ty){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.input=f.input AND t.context=f.context",identifier(&tables[target].alias)))?;}}
  for ty in [TypeId::of::<analysis::local::AnalysisOutcome>(),TypeId::of::<analysis::local::AnalysisCoverage>(),TypeId::of::<local_semantics::LocalContribution>(),TypeId::of::<local_semantics::LocalAssessment>()]{if let(Some(target),Some(local))=(idx(ty),alias(TypeId::of::<analysis::local::Invocation>())){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {local} l ON l.input=f.input AND l.context=f.context JOIN {} t ON t.invocation=l.id",identifier(&tables[target].alias)))?;}}
  if let Some(target)=idx(TypeId::of::<projection::ProjectionSourceAssessment>()){plan.pairs(root,target,format!("SELECT f.id AS source_id,p.id AS target_id FROM {frames} f JOIN {} p ON p.input=f.input AND p.context=f.context",identifier(&tables[target].alias)))?;}
  if let(Some(artifacts),Some(occurrences),Some(entities))=(alias(TypeId::of::<source::SourceArtifact>()),alias(TypeId::of::<source::Occurrence>()),alias(TypeId::of::<normalized::entities::CallableEntity>())){
   if matches!(kind,Kind::Structural){if let(Some(refs),Some(callables))=(idx(TypeId::of::<normalized::entities::EntityRef>()),idx(TypeId::of::<normalized::entities::CallableEntity>())){plan.pairs(root,refs,format!("SELECT f.id AS source_id,r.id AS target_id FROM {frames} f JOIN {artifacts} a ON a.input=f.input JOIN {occurrences} o ON o.source=a.id JOIN {entities} c ON c.source_declaration=o.id JOIN {} r ON r.callable_callable=c.id",identifier(&tables[refs].alias)))?;let _=callables;}}
   if let Some(events)=idx(TypeId::of::<normalized::events::NormalizedCallEvent>()){plan.pairs(root,events,format!("SELECT f.id AS source_id,e.id AS target_id FROM {frames} f JOIN {} e ON e.context=f.context JOIN {occurrences} o ON o.id=e.site JOIN {artifacts} a ON a.id=o.source AND a.input=f.input",identifier(&tables[events].alias)))?;}
   if let Some(modules)=idx(TypeId::of::<source::Module>()){pair!(source::SourceArtifact,source::Module,format!("SELECT a.id AS source_id,m.id AS target_id FROM {artifacts} a JOIN {} m ON m.source=a.id",identifier(&tables[modules].alias)));}
  }
  // Complete source/native callable alternatives are keyed by the selected frame context.
  if let(Some(refs),Some(callables),Some(occurrences),Some(artifacts))=(alias(TypeId::of::<normalized::entities::EntityRef>()),alias(TypeId::of::<normalized::entities::CallableEntity>()),alias(TypeId::of::<source::Occurrence>()),alias(TypeId::of::<source::SourceArtifact>())){
   let candidates=match kind {
    Kind::Structural=>format!("SELECT f.id AS frame,r.callable_callable FROM {frames} f JOIN {artifacts} a ON a.input=f.input JOIN {occurrences} o ON o.source=a.id JOIN {callables} c ON c.source_declaration=o.id JOIN {refs} r ON r.callable_callable=c.id"),
    Kind::Analytic=>format!("SELECT f.id AS frame,r.callable_callable FROM {roots} f JOIN {} m ON m.frame=f.id JOIN {refs} r ON r.id=m.entity",alias(TypeId::of::<structural::ScopeMember>()).ok_or(ModelError::Schema("analytic scope members"))?),
   };
   for ty in [TypeId::of::<normalized::callables::SignatureVariant>(),TypeId::of::<normalized::callables::EffectiveCallableAssessment>()]{if let Some(target)=idx(ty){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN ({candidates}) c ON c.frame=f.id JOIN {} t ON t.callable=c.callable AND t.context=f.context",identifier(&tables[target].alias)))?;}}
  }
  if matches!(kind,Kind::Structural){
   if let(Some(occurrences),Some(artifacts),Some(quals))=(alias(TypeId::of::<source::Occurrence>()),alias(TypeId::of::<source::SourceArtifact>()),alias(TypeId::of::<assertion::AssertionQualification>())){
    for(ty,field)in[(TypeId::of::<flow::FlowRegionObservation>(),"statement"),(TypeId::of::<flow::FlowValueObservation>(),"sink"),(TypeId::of::<flow::FlowTestLeafObservation>(),"test")]{if let Some(target)=idx(ty){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON TRUE JOIN {quals} q ON q.id=t.qualification AND q.context=f.context JOIN {occurrences} o ON o.id=t.{} JOIN {artifacts} a ON a.id=o.source AND a.input=f.input",identifier(&tables[target].alias),identifier(field)))?;}}
   }
   // The immutable native premise inventory joins selected assertions and actual support
   // alternatives to their qualification receipts. Qualification scope alone is no root.
   if let Some(premises)=idx(TypeId::of::<analysis::native::NativeAssertionPremise>()){
    for field in tables[premises].relation.fields().iter().filter(|field|!field.list()){
     if let Some((target,_))=field.target() && let Some(owner)=idx(target){plan.own(premises,field.name(),owner)?;}
    }
    own!(analysis::native::NativeQualification,"premise",analysis::native::NativeAssertionPremise);
   }
   if let(Some(occurrences),Some(details),Some(target),Some(from))=(alias(TypeId::of::<source::Occurrence>()),alias(TypeId::of::<syntax::SyntaxDetailObservation>()),idx(TypeId::of::<syntax::SyntaxDetailObservation>()),idx(TypeId::of::<source::Occurrence>())){
    // Independent roles can name the same argument coordinate. Keep all matching
    // observations and supports before the existing owner applies context and fidelity.
    plan.pairs(from,target,format!("SELECT o.id AS source_id,d.id AS target_id FROM {occurrences} o JOIN {occurrences} c ON c.source=o.source AND c.start=o.start AND c.end=o.end AND c.syntax_kind=o.syntax_kind AND c.structural_path=o.structural_path JOIN {details} d ON d.occurrence=c.id"))?;
   }
  }
  if matches!(kind,Kind::Structural){if let(Some(links),Some(members))=(idx(TypeId::of::<catalog::CatalogMemberInvocation>()),idx(TypeId::of::<catalog::CatalogMember>())){let link_rows=identifier(&tables[links].alias);plan.pairs(root,links,format!("SELECT f.id AS source_id,l.id AS target_id FROM {roots} f JOIN {link_rows} l ON l.invocation=f.id"))?;let _=members;}}
  if matches!(kind,Kind::Analytic){
   // All records owned by this StructuralFrame are the complete topology/attribute universe.
   for(member,table)in tables[..real].iter().enumerate(){for field in table.relation.fields().iter().filter(|field|field.target().map(|(target,_)|target)==Some(TypeId::of::<structural::StructuralFrame>()) && !field.list()){plan.own(member,field.name(),root)?;}}
   if let(Some(occurrences),Some(placements),Some(quals),Some(from),Some(target))=(alias(TypeId::of::<source::Occurrence>()),alias(TypeId::of::<syntax::SyntaxPlacement>()),alias(TypeId::of::<assertion::AssertionQualification>()),idx(TypeId::of::<source::Occurrence>()),idx(TypeId::of::<syntax::SyntaxPlacement>())){
    plan.pairs(from,target,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id JOIN {occurrences} c ON c.id=p.occurrence JOIN {quals} q ON q.id=p.qualification WHERE o.syntax_kind={} AND c.syntax_kind={}",source::SyntaxKind::ParameterWithDefault.code(),source::SyntaxKind::Parameter.code()))?;
   }
   if let(Some(scope),Some(ownership),Some(quals),Some(member),Some(owners))=(alias(TypeId::of::<structural::ScopeMember>()),alias(TypeId::of::<normalized::entities::OccurrenceOwnership>()),alias(TypeId::of::<assertion::AssertionQualification>()),idx(TypeId::of::<structural::ScopeMember>()),idx(TypeId::of::<normalized::entities::OccurrenceOwnership>())){
    // Selected entities own protocol observations even when their occurrence is absent
    // from event roots. Ordinary owned occurrence metadata remains outside the grain.
    for ty in [TypeId::of::<protocols::NativeExitObservation>(),TypeId::of::<protocols::NativeTerminalObservation>()]{if let Some(target)=idx(ty){
     let protocols=identifier(&tables[target].alias);
     let matched=format!("FROM {scope} s JOIN {frames} f ON f.id=s.frame JOIN {ownership} o ON o.entity=s.entity JOIN {protocols} p ON p.subject=o.occurrence JOIN {quals} q ON q.id=p.qualification AND q.context=f.context");
     plan.pairs(member,owners,format!("SELECT s.id AS source_id,o.id AS target_id {matched}"))?;
     plan.pairs(member,target,format!("SELECT s.id AS source_id,p.id AS target_id {matched}"))?;
    }}
   }
   own!(embedding::text::TextWindow,"assessment",embedding::text::TextAssessment);
   own!(normalized::links::MentionEntityCandidate,"assessment",normalized::links::MentionEntityAssessment);
   own!(normalized::entities::PublicExposureCandidate,"exposure",normalized::entities::PublicExposure);
   if let(Some(target),Some(candidates),Some(exposures),Some(resolutions),Some(scope))=(idx(TypeId::of::<normalized::links::MentionEntityAssessment>()),alias(TypeId::of::<normalized::links::MentionEntityCandidate>()),alias(TypeId::of::<normalized::entities::PublicExposureCandidate>()),alias(TypeId::of::<normalized::entities::SymbolEntityResolution>()),alias(TypeId::of::<structural::ScopeMember>())){
    plan.pairs(root,target,format!("SELECT f.id AS source_id,a.id AS target_id FROM {frames} f JOIN {scope} s ON s.frame=f.id JOIN {resolutions} r ON r.entity=s.entity AND r.context=f.context JOIN {exposures} e ON e.resolution=r.id JOIN {candidates} c ON c.exposure=e.exposure JOIN {} a ON a.id=c.assessment",identifier(&tables[target].alias)))?;
   }

   own!(embedding::analytic::AnalysisEmbeddingUse,"window",embedding::text::TextWindow);
   own!(analysis::analytic_embedding::AnalysisOutcome,"invocation",analysis::analytic_embedding::Invocation);
   if let Some(target)=idx(TypeId::of::<embedding::text::TextAssessment>()){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.input=f.input AND t.context=f.context",identifier(&tables[target].alias)))?;}
   if let Some(target)=idx(TypeId::of::<input::CorpusLibrary>()){plan.pairs(root,target,format!("SELECT f.id AS source_id,t.id AS target_id FROM {frames} f JOIN {} t ON t.library=f.input",identifier(&tables[target].alias)))?;}
  }
  let mut charge=charged::StateCharge::new(budget,"analytical-frame-descriptors");
  charge.grow(inputs.capacity()*size_of::<ValidationInput>()+tables.capacity()*size_of::<ClosureTable>()+tables.iter().map(|table|table.alias.capacity()).sum::<usize>())?;
  let edges=plan.prepare(session,budget).await?;Ok(Self{inputs,edges,root,_charge:charge})
 }
 pub(super) async fn grain<R:Record>(&self,id:Id<R>,budget:&resources::ResourceBudget)->Result<PreparedClosure,ModelError>{self.edges.grain(self.root,&crate::scoped_admission::root_predicate(&[*id.bytes()]),budget).await}
 pub(super) async fn read<R:Record>(&self,access:&CompletedInputs,grain:&PreparedClosure,mut visit:impl FnMut(&ValidationInput,&arrow_array::RecordBatch)->Result<(),ModelError>)->Result<(),ModelError>{for(table,input)in self.inputs.iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<R>()){let permit=access.read_at::<R>(input.prefix())?;crate::consumed_rows::stream_query_at(&permit,input,grain.session(),&grain.select(table)?,|_,batch|visit(input,batch)).await?;}Ok(())}
}

#[cfg(test)]
mod controls {
 use super::*;
 use lctx_model::domain::{normalized::entities::*,source::*};
 use datafusion::prelude::SessionContext;
 use futures::TryStreamExt;
 fn id<R>(n:u8)->Id<R>{serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([n;16].into_iter())).unwrap()}
 #[tokio::test]
 async fn structural_frame_keeps_complete_callable_roots_without_unrelated_source_payload(){
  let session=SessionContext::new();let budget=resources::ResourceBudget::fixed(4<<20).unwrap();
  let selected=SourceArtifact::from_bytes(id(1),"selected.py".into(),b"def f(): pass\ndef g(): pass").unwrap();
  let foreign=SourceArtifact::from_bytes(id(2),format!("{}.py","z".repeat(256<<10)),b"other").unwrap();
  let occurrence=|source,start|Occurrence{source,start,end:start+1,syntax_kind:SyntaxKind::StmtFunctionDef,role:OccurrenceRole::Declaration,structural_path:vec![start as i32]};
  let first=occurrence(selected.id(),0);let second=occurrence(selected.id(),14);let other=occurrence(foreign.id(),0);
  let unused=Occurrence{syntax_kind:SyntaxKind::StmtPass,role:OccurrenceRole::Syntax,structural_path:vec![4;65536],..first.clone()};
  let callables=[first.clone(),second.clone(),other.clone()].map(|row|CallableEntity::Source{declaration:row.id(),kind:CallableKind::Function});
  let refs=callables.iter().map(|row|EntityRef::Callable{callable:row.id()}).collect::<Vec<_>>();
  let frame=analysis::catalog_core::Invocation::new(selected.input,id(3),id(4),None,[]).0;
  let inputs=vec![ValidationInput::of::<SourceArtifact>(&["id"]),ValidationInput::of::<Occurrence>(&["id"]),ValidationInput::of::<CallableEntity>(&["id"]),ValidationInput::of::<EntityRef>(&["id"]),ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"])];
  let model=model().unwrap();let tables=inputs.iter().map(|input|ClosureTable{relation:model.relation(input.name()).unwrap().clone(),alias:input.name().into()}).collect::<Vec<_>>();
  for table in &tables{session.register_batch(&table.alias,arrow_array::RecordBatch::new_empty(table.relation.schema().clone())).unwrap();}
  fn put<R:Record>(session:&SessionContext,rows:&[R]){session.deregister_table(R::NAME).unwrap();session.register_batch(R::NAME,<R as Record>::encode(rows).unwrap()).unwrap();}
  put(&session,&[selected,foreign]);put(&session,&[first.clone(),second.clone(),other.clone(),unused.clone()]);put(&session,&callables);put(&session,&refs);put(&session,std::slice::from_ref(&frame));
  let prepared=FrameScopes::bound(inputs.clone(),tables,&session,Kind::Structural,&budget).await.unwrap();
  let tiny=resources::ResourceBudget::fixed(96<<10).unwrap();let grain=prepared.grain(frame.id(),&tiny).await.unwrap();
  let table=inputs.iter().position(|input|input.type_id()==TypeId::of::<Occurrence>()).unwrap();let mut stream=crate::sql::query(grain.session(),&grain.select(table).unwrap()).await.unwrap().execute_stream().await.unwrap();
  let mut rows=normalized::Rows::<Occurrence>::new(&tiny);while let Some(batch)=stream.try_next().await.unwrap(){rows.decode(&batch).unwrap();}
  assert_eq!(rows.len(),2);assert_eq!(rows.get(first.id()),Some(&first));assert_eq!(rows.get(second.id()),Some(&second));assert!(rows.get(other.id()).is_none());assert!(rows.get(unused.id()).is_none());
  drop(rows);drop(stream);drop(grain);assert_eq!(tiny.reserved(),0);drop(prepared);assert_eq!(budget.reserved(),0);
 }
}
