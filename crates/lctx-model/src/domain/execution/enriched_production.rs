//! One finite enriched owner, with independently replayed SourceCall predecessors.
use crate::domain::{*,analysis::{self,enriched_execution as publication},normalized::Rows,resources::ResourceBudget,source::{Occurrence,SourceArtifact},input::ArtifactUse};
use crate::Domain;
use super::{enriched_records::*,source_call_records::{self,SourceCallData}};
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="execution_boundaries")]
pub struct ExecutionBoundary{#[model(key)]pub invocation:Id<publication::AnalysisInvocation>,#[model(key)]pub statement:Id<Occurrence>,pub owner:Option<Id<normalized::entities::EntityRef>>,pub reason:obligation::ObligationKind}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="execution_body_boundaries")]
pub struct BodyBoundary{#[model(key)]pub invocation:Id<publication::AnalysisInvocation>,#[model(key)]pub owner:Id<normalized::entities::EntityRef>,pub declaration:Id<Occurrence>,pub reason:obligation::ObligationKind}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="execution_runs",invariants=run_invariants,publication_checks=profile_checks)]
pub struct ExecutionRun{#[model(key)]pub invocation:Id<publication::AnalysisInvocation>,pub requested:bool,pub executed:i64,pub refused:i64,pub bodied:i64,pub body_refused:i64}
macro_rules! source_inputs{($apply:ident)=>{$apply!{
 source_invocations:analysis::source_call::AnalysisInvocation,
 source_runs:source_call_records::SourceCallRun,
 source_headers:source_call_records::SourceCallHeader,
 source_members:source_call_records::HeaderMember,
 source_boundaries:source_call_records::SourceCallBoundary,
 source_results:analysis::source_call::AnalysisOutcome,
 source_calls:source_call_records::SourceInvocation,
 source_releases:source_call_records::SourceFrameRelease,
 source_outcomes:source_call_records::SourceCallOutcome,
 source_invocation_boundaries:source_call_records::InvocationBoundary,
}};}
macro_rules! data{($($field:ident:$ty:ty,)*)=>{
 pub struct EnrichedData{pub source:SourceCallData,$(pub $field:Rows<$ty>,)*}
 impl EnrichedData{
  pub fn new(budget:&ResourceBudget)->Self{Self{source:SourceCallData::new(budget),$($field:Rows::new(budget),)*}}
  pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.source.visit(name,batch)?;$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*Ok(())}
  pub fn inputs()->Vec<ValidationInput>{let mut inputs=SourceCallData::inputs();inputs.extend(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]);inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
 }
};}
source_inputs!(data);
impl EnrichedData{
 fn check_source(&self,expected:&source_call_records::SourceCallRecords)->Result<(),ModelError>{
  let invalid=||ModelError::Invalid("enriched source predecessor differs from shared SourceCall replay".into());
  if self.source_runs.get(expected.run.id())!=Some(&expected.run)||self.source_results.get(expected.outcome.id())!=Some(&expected.outcome){return Err(invalid());}
  macro_rules! compare{($($earlier:ident:$field:ident,)*)=>{$(for row in expected.$field.iter(){if self.$earlier.get(row.id())!=Some(row){return Err(invalid());}})*};}
  compare!{source_headers:headers,source_members:members,source_boundaries:boundaries,source_calls:invocations,source_releases:releases,source_outcomes:call_outcomes,source_invocation_boundaries:invocation_boundaries,}
  Ok(())
 }
}
macro_rules! outputs{($apply:ident)=>{$apply!{
 executions:StatementExecution,outcomes:ExecutionOutcome,sources:ExecutionSource,members:ExecutionMember,entered:EnteredStatement,boundaries:ExecutionBoundary,
 bodies:BodyExecution,body_sources:BodySource,body_members:BodyMember,releases:BodyReleaseInput,body_boundaries:BodyBoundary,
}};}
macro_rules! records{($($field:ident:$ty:ty,)*)=>{
 pub struct ExecutionRecords{pub run:ExecutionRun,pub outcome:publication::AnalysisOutcome,$(pub $field:Rows<$ty>,)*}
 impl ExecutionRecords{fn new(invocation:Id<publication::AnalysisInvocation>,budget:&ResourceBudget)->Self{Self{run:ExecutionRun{invocation,requested:false,executed:0,refused:0,bodied:0,body_refused:0},outcome:publication::AnalysisOutcome{invocation,status:analysis::AnalysisStatus::Completed,reason:None},$($field:Rows::new(budget),)*}}}
};}
outputs!(records);
pub fn enrich_all(data:&EnrichedData,invocation:&publication::AnalysisInvocation,definition:&analysis::AnalysisDefinition,profile:stages::Profile,budget:&ResourceBudget)->Result<ExecutionRecords,ModelError>{
 let invalid=|message:&str|ModelError::Invalid(message.into());
 if *definition!=super::configuration::enriched_execution().1||invocation.definition!=definition.id()||invocation.subject.is_some(){return Err(invalid("enriched execution requires its whole-frame definition"));}
 let mut output=ExecutionRecords::new(invocation.id(),budget);if profile!=stages::Profile::Behavioral{output.outcome.status=analysis::AnalysisStatus::NotRequested;output.outcome.reason=Some(obligation::ObligationKind::NotRequested);return Ok(output);}output.run.requested=true;
 let mut parents=data.source_invocations.iter().filter(|row|(row.input,row.context)==(invocation.input,invocation.context));let source=parents.next().ok_or_else(||invalid("enriched SourceCall frame absent"))?;if parents.next().is_some(){return Err(invalid("enriched SourceCall frame ambiguous"));}let source_definition=data.source.definitions.get(source.definition).ok_or_else(||invalid("enriched SourceCall definition absent"))?;
 let facts=&data.source.evaluation;
 let bytes=facts.artifacts.iter().try_fold(0usize,|n,row|n.checked_add(size_of::<SourceArtifact>()+row.heap_bytes()+128)).and_then(|n|n.checked_add(facts.uses.len().checked_mul(size_of::<ArtifactUse>())?)).and_then(|n|n.checked_mul(2)).ok_or_else(||invalid("enriched root allowance overflow"))?;let _roots=budget.reserve("enriched_source_roots",bytes)?;
 let roots=admission::analysis_roots(&facts.artifacts.iter().cloned().collect::<Vec<_>>(),&facts.uses.iter().cloned().collect::<Vec<_>>())?;
 let selected=|source|roots.contains(&source)&&facts.artifacts.get(source).is_some_and(|artifact|artifact.input==invocation.input&&admission::ArtifactClass::of(&artifact.path)==Some(admission::ArtifactClass::PythonSource));
 let(_,expected)=super::enriched::with_frame(&data.source,source,source_definition,budget,|frame|{
  let mut statements=Vec::new();let mut charge=charged::StateCharge::new(budget,"enriched_retained_completions");
  for row in facts.occurrences.iter().filter(|row|super::completion_production::is_statement(row.syntax_kind)&&selected(row.source)){
   let mut owners=facts.owners.iter().filter(|owner|owner.occurrence==row.id());let first=owners.next();let owner=if owners.next().is_none(){first.map(|owner|owner.entity)}else{None};
   let result=if let Some(owner)=owner{frame.complete(super::completion::CompletionRequest{input:invocation.input,context:invocation.context,owner,statement:row.id()})?}else{Err(obligation::ObligationKind::MissingEvidence)};
   match result{Err(reason)=>{output.boundaries.insert(ExecutionBoundary{invocation:invocation.id(),statement:row.id(),owner,reason})?;},Ok(proof)=>{
    let records=emit_statement(frame,&proof,invocation,definition,budget)?;output.executions.insert(records.execution)?;output.outcomes.insert(records.outcome)?;for row in records.sources{output.sources.insert(row)?;}for row in records.members{output.members.insert(row)?;}for row in records.entered{output.entered.insert(row)?;}charge.grow(size_of::<super::completion::CheckedCompletion>()*2)?;statements.push(proof);
   }}
  }
  let _scratch=budget.reserve("enriched_body_statement_refs",statements.len().checked_mul(size_of::<&super::completion::CheckedCompletion>()*2).ok_or_else(||invalid("enriched body allowance overflow"))?)?;let refs=statements.iter().collect::<Vec<_>>();
  for callable in facts.callables.iter(){let normalized::entities::CallableEntity::Source{declaration,..}=callable else{continue};let occurrence=facts.occurrences.get(*declaration).ok_or_else(||invalid("enriched body declaration absent"))?;if !selected(occurrence.source){continue;}let owner=normalized::entities::EntityRef::Callable{callable:callable.id()}.id();
   match super::body::complete_body(facts,super::body::SourceBodyRequest{input:invocation.input,context:invocation.context,callee:owner},&refs,budget)?{
    Err(reason)=>{output.body_boundaries.insert(BodyBoundary{invocation:invocation.id(),owner,declaration:*declaration,reason})?;},Ok(proof)=>{let records=emit_body(&proof,invocation,&output.executions,budget)?;output.bodies.insert(records.body)?;output.outcomes.insert(records.outcome)?;for row in records.sources{output.body_sources.insert(row)?;}for row in records.members{output.body_members.insert(row)?;}for row in records.releases{output.releases.insert(row)?;}}
   }
  }
  Ok(())
 })?;data.check_source(&expected)?;
 output.run.executed=output.executions.len().try_into().map_err(|_|invalid("enriched count overflow"))?;output.run.refused=output.boundaries.len().try_into().map_err(|_|invalid("enriched count overflow"))?;output.run.bodied=output.bodies.len().try_into().map_err(|_|invalid("enriched count overflow"))?;output.run.body_refused=output.body_boundaries.len().try_into().map_err(|_|invalid("enriched count overflow"))?;
 if !output.boundaries.is_empty()||!output.body_boundaries.is_empty(){output.outcome.status=analysis::AnalysisStatus::Partial;output.outcome.reason=Some(obligation::ObligationKind::UnsupportedControlFlow);}Ok(output)
}
pub fn relations()->Vec<Relation>{vec![Relation::of::<ExecutionRun>(),Relation::of::<ExecutionBoundary>(),Relation::of::<BodyBoundary>()]}
fn invariant_inputs()->Vec<ValidationInput>{let mut inputs=EnrichedData::inputs();inputs.extend([ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),ValidationInput::of::<ExecutionRun>(&["id"]),ValidationInput::of::<publication::AnalysisOutcome>(&["id"])]);macro_rules! append{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}outputs!(append);inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));inputs}
fn run_invariants()->Vec<Invariant>{vec![Invariant{name:"enriched_execution_inventory",inputs:invariant_inputs(),create:std::sync::Arc::new(|budget|Box::new(ExecutionCheck::new(budget)))}]}
pub(crate) fn statement_invariants()->Vec<Invariant>{vec![Invariant{name:"enriched_statement_replay",inputs:invariant_inputs(),create:std::sync::Arc::new(|budget|Box::new(ExecutionCheck::new(budget)))}]}
macro_rules! checker{($($field:ident:$ty:ty,)*)=>{
 struct ExecutionCheck{data:EnrichedData,invocations:Rows<publication::AnalysisInvocation>,runs:Rows<ExecutionRun>,results:Rows<publication::AnalysisOutcome>,$($field:Rows<$ty>,)*budget:ResourceBudget}
 impl ExecutionCheck{fn new(budget:&ResourceBudget)->Self{Self{data:EnrichedData::new(budget),invocations:Rows::new(budget),runs:Rows::new(budget),results:Rows::new(budget),$($field:Rows::new(budget),)*budget:budget.clone()}}}
 impl InvariantCheck for ExecutionCheck{
  fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.data.visit(name,batch)?;if name==publication::AnalysisInvocation::NAME{self.invocations.decode(batch)?;}if name==ExecutionRun::NAME{self.runs.decode(batch)?;}if name==publication::AnalysisOutcome::NAME{self.results.decode(batch)?;}$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*Ok(())}
  fn visit_input(&mut self,input:&ValidationInput,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.visit(input.name(),batch)}
  fn finish(self:Box<Self>)->Result<(),ModelError>{
   let invalid=|message:&str|ModelError::Invalid(message.into());let mut frames=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(&self.budget,"enriched_frame_inventory");for row in self.data.source_invocations.iter(){frames.insert(&mut charge,(row.input,row.context))?;}if self.invocations.len()!=frames.len()||frames.iter().any(|frame|self.invocations.iter().filter(|row|(row.input,row.context)==*frame).count()!=1){return Err(invalid("enriched omitted/duplicated SourceCall frame"));}
   let mut runs=Rows::new(&self.budget);let mut results=Rows::new(&self.budget);$(let mut $field=Rows::new(&self.budget);)*
   for invocation in self.invocations.iter(){let mut parents=Rows::new(&self.budget);for source in self.data.source_invocations.iter().filter(|row|(row.input,row.context)==(invocation.input,invocation.context)){parents.insert(publication::InvocationSource::SourceCallAnalysis{invocation:source.id()})?;}let mut key=KeySink::new("analysis-invocation-inputs");for row in parents.iter(){row.id().encode(&mut key);}if invocation.inputs!=key.finish(){return Err(invalid("enriched changes exact SourceCall parents"));}
    let run=self.runs.iter().find(|row|row.invocation==invocation.id()).ok_or_else(||invalid("enriched run absent"))?;let definition=self.data.source.definitions.get(invocation.definition).ok_or_else(||invalid("enriched definition absent"))?;let expected=enrich_all(&self.data,invocation,definition,if run.requested{stages::Profile::Behavioral}else{stages::Profile::Catalog},&self.budget)?;runs.insert(expected.run)?;results.insert(expected.outcome)?;$(for row in expected.$field.iter(){$field.insert(row.clone())?;})*
   }
   if !self.runs.same(&runs)||!self.results.same(&results)$(||!self.$field.same(&$field))*{return Err(invalid("enriched inventory differs from exact shared ordered execution"));}Ok(())
  }
 }
};}
outputs!(checker);
fn profile_checks()->Vec<PublicationInvariant>{vec![PublicationInvariant{name:"enriched_execution_profile",inputs:vec![ValidationInput::of::<ExecutionRun>(&["id"]),ValidationInput::of::<publication::AnalysisInvocation>(&["id"])],create:std::sync::Arc::new(|budget|Box::new(ProfileCheck{runs:Rows::new(budget),invocations:Rows::new(budget)}))}]}
struct ProfileCheck{runs:Rows<ExecutionRun>,invocations:Rows<publication::AnalysisInvocation>}
impl PublicationCheck for ProfileCheck{
 fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{if name==ExecutionRun::NAME{self.runs.decode(batch)?;}else if name==publication::AnalysisInvocation::NAME{self.invocations.decode(batch)?;}else{return Err(ModelError::Invalid("undeclared enriched profile input".into()));}Ok(())}
 fn finish(self:Box<Self>,_sources:&[stages::CompletedRelation],profile:stages::Profile)->Result<(),ModelError>{if self.runs.len()!=self.invocations.len()||self.runs.iter().any(|row|self.invocations.get(row.invocation).is_none()||row.requested!=(profile==stages::Profile::Behavioral)){return Err(ModelError::Invalid("enriched request differs from actual publication profile".into()));}Ok(())}
}

pub fn stage(profile:stages::Profile,definition:&analysis::AnalysisDefinition,model:&ValidatedModel)->Result<stages::Stage,ModelError>{
 use stages::*;if *definition!=super::configuration::enriched_execution().1{return Err(ModelError::Invalid("enriched execution stage definition is unbound".into()));}
 let mut outputs=vec![Relation::of::<publication::AnalysisInvocation>(),Relation::of::<publication::AnalysisInput>(),Relation::of::<publication::SourceReceipt>(),Relation::of::<publication::ProjectionInput>(),Relation::of::<publication::InvocationSource>(),Relation::of::<publication::AnalysisOutcome>()];outputs.extend(publication::coverage::relations());outputs.extend(super::enriched_records::relations());outputs.extend(relations());outputs.sort_by_key(Relation::name);outputs.dedup_by_key(|r|r.name());
 let own=outputs.iter().map(Relation::name).collect::<std::collections::BTreeSet<_>>();let facts=crate::domain::facts_relations().iter().map(Relation::name).collect::<std::collections::BTreeSet<_>>();let mut inputs=std::collections::BTreeMap::new();
 let mut initial=if profile==Profile::Behavioral{invariant_inputs()}else{vec![ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"])]};
 for relation in &outputs{for check in relation.publication_checks(){initial.extend(check.inputs.iter().cloned());}}
 let mut pending=Vec::new();for input in initial{if own.contains(input.name())||inputs.contains_key(input.name()){continue;}let relation=model.relations().iter().find(|r|r.name()==input.name()).ok_or_else(||ModelError::Invalid(format!("enriched execution input missing {}",input.name())))?;inputs.insert(input.name(),RelationUse::of_relation(relation).completed_store());pending.push(input.name());}
 while let Some(name)=pending.pop(){let relation=model.relations().iter().find(|r|r.name()==name).ok_or_else(||ModelError::Invalid(format!("enriched execution closure missing {name}")))?;let references=relation.fields().iter().filter_map(|f|f.target().map(|(_,n)|n));let checks=relation.invariants().iter().flat_map(|i|i.inputs.iter()).map(ValidationInput::name);
 for required in references.chain(checks){if own.contains(required){return Err(ModelError::Invalid(format!("enriched execution predecessor {name} depends on unfinished output {required}")));}if !facts.contains(required)&&!inputs.contains_key(required){let relation=model.relations().iter().find(|r|r.name()==required).ok_or_else(||ModelError::Invalid(format!("enriched execution predecessor missing {required}")))?;inputs.insert(required,RelationUse::of_relation(relation).completed_store());pending.push(required);}}
 }
 let inputs=normalized::facts_stage_inputs(inputs.into_values().collect());let mut key=KeySink::new("enriched-execution-definition");definition.id().encode(&mut key);
 Ok(Stage{name:"enrich_execution",inputs,outputs:outputs.iter().map(RelationUse::of_relation).collect(),contributes:vec![],coverage:vec![],provider:None,profiles:vec![profile],effect:Effect::Pure,code:ContentHash::of(include_bytes!("enriched_production.rs")),configuration:key.finish()})
}
