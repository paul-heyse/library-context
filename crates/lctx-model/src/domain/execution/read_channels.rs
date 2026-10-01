//! Native read/dependency observations and complete-negative assessments. Read existence is
//! distinct from execution reachability and value availability. No heap state is inferred.
use crate::domain::{*,analysis::{base_evaluation as publication,native::NativeAssertionPremise,policy::{EvidenceStatus}},assertion::*,attribution::*,conditions::entry::EntryData,flow::*,normalized::{Rows,entities::*},source::*,value::*};
use crate::{Domain,DomainCode};
use super::evaluation::EvaluationData;
#[derive(Debug,Clone,Copy,PartialEq,Eq,DomainCode)]#[repr(i16)]
pub enum ReadLocation {NativePlace=0,DeclaredGlobal=1,UnresolvedGlobal=2}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="base_read_observations")]
pub struct ReadObservation {
 #[model(key)] pub invocation:Id<publication::AnalysisInvocation>,
 #[model(key)] pub observation:Id<FlowUseObservation>,
 pub owner:Id<EntityRef>,pub place:Id<Place>,pub location:ReadLocation,pub lexical_premise:Option<Id<NativeAssertionPremise>>,pub binding_premise:Option<Id<NativeAssertionPremise>>,pub qualification:Id<AssertionQualification>,
 pub premise:Option<Id<NativeAssertionPremise>>,pub status:EvidenceStatus,
 pub execution_region:Option<Id<FlowRegionObservation>>,pub region_premise:Option<Id<NativeAssertionPremise>>,
 pub reason:Option<obligation::ObligationKind>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="base_read_dependencies")]
pub struct ReadDependency {
 #[model(key)] pub read:Id<ReadObservation>,#[model(key)] pub observation:Id<FlowValueObservation>,
 pub premise:Option<Id<NativeAssertionPremise>>,pub qualification:Id<AssertionQualification>,pub status:EvidenceStatus,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="base_attribute_reads")]
pub struct AttributeRead {
 #[model(key)] pub invocation:Id<publication::AnalysisInvocation>,#[model(key)] pub observation:Id<FlowAttributeLoadObservation>,
 pub owner:Id<EntityRef>,pub premise:Option<Id<NativeAssertionPremise>>,pub qualification:Id<AssertionQualification>,pub status:EvidenceStatus,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,DomainCode)]
#[repr(i16)]
pub enum ReadAssessment {ObservedRead=0,CompleteNoReadUnderModel=1,Unknown=2}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="base_formal_read_assessments")]
pub struct FormalReadAssessment {
 #[model(key)] pub invocation:Id<publication::AnalysisInvocation>,#[model(key)] pub formal:Id<ParameterEntity>,
 pub owner:Id<EntityRef>,pub scope:Option<Id<lexical::LexicalScope>>,pub coverage:Option<Id<ProviderCoverage>>,
 pub status:ReadAssessment,pub reason:Option<obligation::ObligationKind>,pub reads:ContentHash,
}
pub struct ReadRecords {pub fields:super::read_fields::FieldRecords,pub dynamic:Rows<super::read_dynamic::DynamicAccessObservation>,pub dynamic_premises:Rows<super::read_dynamic::DynamicAccessPremise>,pub reads:Rows<ReadObservation>,pub dependencies:Rows<ReadDependency>,pub attributes:Rows<AttributeRead>,pub formals:Rows<FormalReadAssessment>}
impl ReadRecords {pub fn new(budget:&resources::ResourceBudget)->Self{Self{fields:super::read_fields::FieldRecords::new(budget),dynamic:Rows::new(budget),dynamic_premises:Rows::new(budget),reads:Rows::new(budget),dependencies:Rows::new(budget),attributes:Rows::new(budget),formals:Rows::new(budget)}}}
pub fn relations()->Vec<Relation>{{let mut rows=super::read_dynamic::relations();rows.extend(super::read_fields::relations());rows.extend([Relation::of::<ReadObservation>(),Relation::of::<ReadDependency>(),Relation::of::<AttributeRead>(),Relation::of::<FormalReadAssessment>()]);rows}}
impl ReadRecords {
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{self.fields.visit(name,batch)?;macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}rows!{dynamic:super::read_dynamic::DynamicAccessObservation,dynamic_premises:super::read_dynamic::DynamicAccessPremise,reads:ReadObservation,dependencies:ReadDependency,attributes:AttributeRead,formals:FormalReadAssessment,}Ok(())}
 pub fn append(&mut self,other:&Self)->Result<(),ModelError>{self.fields.append(&other.fields)?;macro_rules! rows{($($field:ident,)*)=>{$(for row in other.$field.iter(){self.$field.insert(row.clone())?;})*};}rows!{dynamic,dynamic_premises,reads,dependencies,attributes,formals,}Ok(())}
 pub fn same(&self,other:&Self)->bool{self.fields.same(&other.fields)&&self.dynamic.same(&other.dynamic)&&self.dynamic_premises.same(&other.dynamic_premises)&&self.reads.same(&other.reads)&&self.dependencies.same(&other.dependencies)&&self.attributes.same(&other.attributes)&&self.formals.same(&other.formals)}
}
pub fn validation_inputs()->Vec<ValidationInput>{{let mut inputs=super::read_dynamic::validation_inputs();inputs.extend(super::read_fields::validation_inputs());inputs.extend([ValidationInput::of::<ReadObservation>(&["id"]),ValidationInput::of::<ReadDependency>(&["id"]),ValidationInput::of::<AttributeRead>(&["id"]),ValidationInput::of::<FormalReadAssessment>(&["id"])]);inputs}}
pub const WORK_LIMIT:usize=1<<24;
pub(super) struct Work {
 pub(super) dynamic_targets:charged::ChargedMap<Id<Occurrence>,Vec<Id<calls::CallTarget>>>,remaining:usize,native:charged::ChargedMap<(derivation::RowRef,derivation::RowRef),Id<analysis::native::NativeQualification>>,
 native_ids:charged::ChargedMap<Id<NativeAssertionPremise>,Id<analysis::native::NativeQualification>>,
 owners:charged::ChargedMap<Id<Occurrence>,Option<Id<EntityRef>>>,
 formal_reads:charged::ChargedMap<Id<Occurrence>,Vec<Id<FlowUse>>>,
 nested:charged::ChargedMap<Id<EntityRef>,Vec<Id<FlowReachingObservation>>>,
 calls:charged::ChargedMap<Id<EntityRef>,Vec<Id<Occurrence>>>,charge:charged::StateCharge,
}
impl Work {
 pub(super) fn scan(&mut self,n:usize)->Result<(),ModelError>{self.remaining=self.remaining.checked_sub(n).ok_or_else(||ModelError::Invalid("read channel work bound exhausted".into()))?;Ok(())}
 pub(super) fn tick(&mut self)->Result<(),ModelError>{self.scan(1)}
 pub(super) fn owner(&self,site:Id<Occurrence>)->Option<Id<EntityRef>>{self.owners.get(&site).copied().flatten()}
 pub(super) fn qualification<'a>(&self,data:&'a EvaluationData,premise:Id<NativeAssertionPremise>)->Option<&'a analysis::native::NativeQualification>{data.native.get(*self.native_ids.get(&premise)?)}
 fn new(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
  let mut index=Self{dynamic_targets:Default::default(),remaining:WORK_LIMIT,native:Default::default(),native_ids:Default::default(),owners:Default::default(),formal_reads:Default::default(),nested:Default::default(),calls:Default::default(),charge:charged::StateCharge::new(budget,"read-channel-index")};
  for row in data.native.iter(){index.tick()?;let premise=data.premises.get(row.premise).ok_or_else(||ModelError::Invalid("read native pair missing".into()))?;if index.native.insert(&mut index.charge,premise.assertion_and_support(),row.id())?.is_some(){return Err(ModelError::Invalid("duplicate read native pair".into()));}index.native_ids.insert(&mut index.charge,row.premise,row.id())?;}
  for row in entry.owners.iter(){index.tick()?;let value=if index.owners.contains_key(&row.occurrence){None}else{Some(row.entity)};index.owners.insert(&mut index.charge,row.occurrence,value)?;}
  let mut observed=charged::ChargedSet::default();for row in entry.use_observations.iter(){index.tick()?;if entry.qualifications.get(row.qualification).is_some_and(|q|q.context==invocation.context){observed.insert(&mut index.charge,row.use_)?;}}
  for row in entry.uses.iter(){index.tick()?;if !observed.contains(&row.id()){continue;}if let Some(PlaceRoot::Formal{declaration})=entry.places.get(row.place).and_then(|p|entry.roots.get(p.root)){index.formal_reads.update(&mut index.charge,*declaration,|rows|rows.push(row.id()))?;}}
  let mut parameters=charged::ChargedMap::default();for row in entry.placements.iter(){index.tick()?;if row.field==lexical::SyntaxField::Child&&row.ordinal==0{if let Some(parameter)=row.parent.filter(|p|entry.occurrences.get(*p).is_some_and(|o|o.syntax_kind==SyntaxKind::Parameter)){parameters.insert(&mut index.charge,row.occurrence,parameter)?;}}}
  for row in entry.reaching.iter(){index.tick()?;if entry.qualifications.get(row.qualification).is_none_or(|q|q.context!=invocation.context){continue;}if let Some(ReachingDefinition::Bound{definition})=entry.targets.get(row.target){if let Some(definition)=entry.definitions.get(*definition){let parameter=parameters.get(&definition.occurrence).copied().unwrap_or(definition.occurrence);index.formal_reads.update(&mut index.charge,parameter,|rows|rows.push(row.use_))?;}}
   if matches!(entry.targets.get(row.target),Some(ReachingDefinition::Nested)){if let Some(owner)=entry.uses.get(row.use_).and_then(|u|index.owner(u.occurrence)){index.nested.update(&mut index.charge,owner,|rows|rows.push(row.id()))?;}}
  }
  for target in data.call_targets.iter(){index.tick()?;if entry.qualifications.get(target.qualification).is_some_and(|q|q.context==invocation.context){index.dynamic_targets.update(&mut index.charge,target.site,|rows|rows.push(target.id()))?;}}
  for occurrence in entry.occurrences.iter(){index.tick()?;if occurrence.syntax_kind==SyntaxKind::ExprCall{if let Some(owner)=index.owner(occurrence.id()){index.calls.update(&mut index.charge,owner,|rows|rows.push(occurrence.id()))?;}}}
  Ok(index)
 }
}
pub(super) fn input_scope(entry:&EntryData,scope:Id<CoverageScope>,input:Id<input::InputRevision>,artifact:Id<SourceArtifact>)->bool{
 match entry.scopes.get(scope){Some(CoverageScope::Input{input:owner})=>*owner==input,Some(CoverageScope::Artifact{artifact:owner})=>*owner==artifact,Some(CoverageScope::Module{module})=>entry.modules.get(*module).is_some_and(|m|m.source==artifact),_=>false}
}
/// Calls may retain unresolved alternatives (Partial); that is not a runtime closure claim.
/// Missing/unavailable inventory cannot screen imported/qualified name-driven accesses.
pub(super) fn calls_available(entry:&EntryData,invocation:&publication::AnalysisInvocation,artifact:Id<SourceArtifact>)->bool{entry.coverage.iter().any(|c|c.family==FactFamily::Calls&&c.context==invocation.context&&input_scope(entry,c.scope,invocation.input,artifact)&&matches!(c.status,CoverageStatus::CompleteUnderStatedModel|CoverageStatus::Partial)&&c.run.is_some_and(|run|entry.runs.get(run).is_some_and(|r|r.input==invocation.input&&r.context==invocation.context)))}
pub(super) fn selected(entry:&EntryData,invocation:&publication::AnalysisInvocation,occurrence:Id<Occurrence>,roots:&std::collections::BTreeSet<Id<SourceArtifact>>)->bool{entry.occurrences.get(occurrence).is_some_and(|row|roots.contains(&row.source)&&entry.artifacts.get(row.source).is_some_and(|a|a.input==invocation.input))}
pub(super) fn native<S:Support>(data:&EvaluationData,entry:&EntryData,supports:&Rows<S>,assertion:Id<S::Assertion>,q:Id<AssertionQualification>,site:Id<Occurrence>,invocation:&publication::AnalysisInvocation,work:&mut Work)->Result<Option<(Id<NativeAssertionPremise>,Id<ProviderRun>,EvidenceStatus)>,ModelError>{
 let mut selected=None;work.scan(supports.len())?;
 for support in supports.iter().filter(|s|s.assertion()==assertion){work.tick()?;let Some(a)=support.attribution()else{continue};let Some(run)=entry.runs.get(a.run)else{continue};if run.input!=invocation.input||run.context!=invocation.context{continue;}
 let Some(qualification)=entry.qualifications.get(q)else{continue};let Some(occurrence)=entry.occurrences.get(site)else{continue};if qualification.context!=invocation.context||!input_scope(entry,qualification.scope,invocation.input,occurrence.source){continue;}
 let Some(n)=work.native.get(&(derivation::RowRef::of(assertion),derivation::RowRef::of(support.id()))).and_then(|id|data.native.get(*id)).filter(|n|n.qualification==q)else{continue};
 if selected.is_some(){return Err(ModelError::Invalid("ambiguous read provider support".into()));}selected=Some((n.premise,a.run,n.status));
 }Ok(selected)
}
/// The whole Base inventory checker reruns this operation over confirmed upstream rows. Roots
/// are selected by admission, never inferred from missing reads or chosen by the producer.
pub fn produce(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,budget:&resources::ResourceBudget)->Result<ReadRecords,ModelError>{
 let mut out=ReadRecords::new(budget);let mut work=Work::new(data,entry,invocation,budget)?;
 for observation in entry.use_observations.iter(){work.tick()?;if entry.qualifications.get(observation.qualification).is_none_or(|q|q.context!=invocation.context){continue;}let Some(use_)=entry.uses.get(observation.use_)else{return Err(ModelError::Invalid("native read use missing".into()));};if !selected(entry,invocation,use_.occurrence,roots){continue;}let Some(owner)=work.owner(use_.occurrence)else{continue;};
  let supported=native(data,entry,&entry.use_supports,observation.id(),observation.qualification,use_.occurrence,invocation,&mut work)?;
  let region=supported.and_then(|(_,run,_)|conditions::entry::region_for_access(entry,owner,use_.occurrence,invocation.context,run).ok());
  let region_premise=if let Some((region,support))=region{let row=entry.regions.get(region).ok_or_else(||ModelError::Invalid("read region missing".into()))?;let pair=NativeAssertionPremise::Region{assertion:region,support};work.qualification(data,pair.id()).filter(|n|n.qualification==row.qualification).map(|n|n.premise)}else{None};
  let mut global=None;let mut global_binding=None;let mut global_ambiguous=false;
  for resolution in data.lexical_resolutions.iter().filter(|r|r.read==use_.occurrence&&entry.qualifications.get(r.qualification).is_some_and(|q|q.context==invocation.context)){
   work.tick()?;let Some(lexical::LexicalTarget::Binding{event})=data.lexical_targets.get(resolution.target)else{continue};
   let mut bindings=data.bindings.iter().filter(|b|b.event==*event&&entry.lexical_scopes.get(b.scope).is_some_and(|s|s.kind==lexical::LexicalScopeKind::Module)&&entry.qualifications.get(b.qualification).is_some_and(|q|q.context==invocation.context));
   if let Some(binding)=bindings.next(){
    if bindings.next().is_some(){global_ambiguous=true;}
    let supported_binding=native(data,entry,&data.binding_supports,binding.id(),binding.qualification,data.binding_events.get(binding.event).ok_or_else(||ModelError::Invalid("global binding event missing".into()))?.site,invocation,&mut work)?;
    if let (Some(premise),Some(binding))=(native(data,entry,&data.lexical_resolution_supports,resolution.id(),resolution.qualification,use_.occurrence,invocation,&mut work)?,supported_binding){if global.is_some(){global_ambiguous=true;}global=Some(premise.0);global_binding=Some(binding.0);}
   }
  }
  let read=ReadObservation{invocation:invocation.id(),observation:observation.id(),owner,place:use_.place,location:if global_ambiguous{ReadLocation::UnresolvedGlobal}else if global.is_some(){ReadLocation::DeclaredGlobal}else{ReadLocation::NativePlace},lexical_premise:global.filter(|_|!global_ambiguous),binding_premise:global_binding.filter(|_|!global_ambiguous),qualification:observation.qualification,premise:supported.map(|p|p.0),status:supported.map_or(EvidenceStatus::Unresolved,|p|p.2),execution_region:region.filter(|_|region_premise.is_some()).map(|r|r.0),region_premise,reason:if supported.is_none(){Some(obligation::ObligationKind::MissingEvidence)}else if region_premise.is_none()&&!observation.annotation{Some(obligation::ObligationKind::MissingEvidence)}else{None}};
  let id=out.reads.insert(read)?;
  for value in entry.values.iter().filter(|v|v.use_==use_.id()&&entry.qualifications.get(v.qualification).is_some_and(|q|q.context==invocation.context)){work.tick()?;let supported=native(data,entry,&entry.value_supports,value.id(),value.qualification,use_.occurrence,invocation,&mut work)?;out.dependencies.insert(ReadDependency{read:id,observation:value.id(),premise:supported.map(|p|p.0),qualification:value.qualification,status:supported.map_or(EvidenceStatus::Unresolved,|p|p.2)})?;}
 }
 for row in data.attribute_loads.iter(){work.tick()?;if entry.qualifications.get(row.qualification).is_none_or(|q|q.context!=invocation.context)||!selected(entry,invocation,row.occurrence,roots){continue;}let Some(owner)=work.owner(row.occurrence)else{continue;};let supported=native(data,entry,&data.attribute_load_supports,row.id(),row.qualification,row.occurrence,invocation,&mut work)?;out.attributes.insert(AttributeRead{invocation:invocation.id(),observation:row.id(),owner,premise:supported.map(|p|p.0),qualification:row.qualification,status:supported.map_or(EvidenceStatus::Unresolved,|p|p.2)})?;}
 super::read_dynamic::produce(data,entry,invocation,roots,&mut out,budget,&mut work)?;
 // The admitted source parameter universe is independent of both emitted reads and assessments.
 for formal in entry.formals.iter(){work.tick()?;let ParameterEntity::Source{declaration}=formal else{continue};if !selected(entry,invocation,*declaration,roots){continue;}let Some(owner)=formal_owner(entry,*declaration)else{continue;};let EntityRef::Callable{callable}=entry.refs.get(owner).ok_or_else(||ModelError::Invalid("formal owner missing".into()))? else{continue};let CallableEntity::Source{declaration:function,..}=entry.callables.get(*callable).ok_or_else(||ModelError::Invalid("formal callable missing".into()))? else{continue};
  let mut scopes=entry.lexical_scopes.iter().filter(|s|s.owner==*function);let scope=scopes.next().filter(|_|scopes.next().is_none()).map(Record::id);
  let artifact=entry.occurrences.get(*declaration).unwrap().source;let mut coverages=entry.coverage.iter().filter(|c|c.family==FactFamily::Flow&&c.context==invocation.context&&input_scope(entry,c.scope,invocation.input,artifact));let coverage=coverages.next().filter(|_|coverages.next().is_none());
  let mut digest=KeySink::new("formal-native-read-universe");let mut seen=false;let mut unresolved=out.dynamic.iter().any(|d|matches!(d.kind,super::read_dynamic::DynamicKind::Exec|super::read_dynamic::DynamicKind::Eval));
  let reads=work.formal_reads.get(declaration);work.scan(reads.map_or(0,Vec::len))?;
  if let Some(reads)=work.formal_reads.get(declaration){let mut ids=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(budget,"formal-read-membership");for use_ in reads{ids.insert(&mut charge,*use_)?;}for use_ in ids.iter(){use_.encode(&mut digest);seen=true;}}
  // Nested/lazy bindings and arbitrary calls may inspect bindings beyond explicit native uses.
  if let Some(rows)=work.nested.get(&owner){for row in rows{row.encode(&mut digest);unresolved=true;}}
  if let Some(rows)=work.calls.get(&owner){for row in rows{row.encode(&mut digest);unresolved=true;}}
  let coverage_complete=coverage.is_some_and(|c|c.status==CoverageStatus::CompleteUnderStatedModel&&c.run.is_some_and(|run|entry.runs.get(run).is_some_and(|r|r.input==invocation.input&&r.context==invocation.context)));
  for c in entry.coverage.iter().filter(|c|c.family==FactFamily::Calls&&c.context==invocation.context&&input_scope(entry,c.scope,invocation.input,artifact)){c.id().encode(&mut digest);}if let Some(c)=coverage{c.id().encode(&mut digest);}if let Some(s)=scope{s.encode(&mut digest);}
  let (status,reason)=if seen{(ReadAssessment::ObservedRead,None)}else if unresolved{(ReadAssessment::Unknown,Some(obligation::ObligationKind::DynamicAccess))}else if !coverage_complete||!calls_available(entry,invocation,artifact)||scope.is_none(){(ReadAssessment::Unknown,Some(obligation::ObligationKind::IncompleteCoverage))}else{(ReadAssessment::CompleteNoReadUnderModel,None)};
  out.formals.insert(FormalReadAssessment{invocation:invocation.id(),formal:formal.id(),owner,scope,coverage:coverage.map(Record::id),status,reason,reads:digest.finish()})?;
 }super::read_fields::produce(data,entry,invocation,roots,&mut out,budget,&mut work)?;Ok(out)
}

// Parameter syntax is evaluated in its enclosing header owner. Its binding's callable owner
// is the unique nearest source callable anchor, independently of default evaluation ownership.
fn formal_owner(entry:&EntryData,parameter:Id<Occurrence>)->Option<Id<EntityRef>>{
 let parameter=entry.occurrences.get(parameter)?;let mut best=None;let mut depth=0;
 for callable in entry.callables.iter(){let CallableEntity::Source{declaration,..}=callable else{continue};let Some(function)=entry.occurrences.get(*declaration)else{continue};if function.source!=parameter.source||!parameter.structural_path.starts_with(&function.structural_path)||function.structural_path.len()>=parameter.structural_path.len(){continue;}
 if function.structural_path.len()>depth{depth=function.structural_path.len();best=Some(callable.id());}else if function.structural_path.len()==depth{return None;}}
 let reference=EntityRef::Callable{callable:best?};(entry.refs.get(reference.id())==Some(&reference)).then_some(reference.id())
}
