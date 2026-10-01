//! Finite statement closure for authored official source. This never proves execution.
use super::documentary;
use crate::domain::{*,analysis::native::NativeAssertionPremise,assertion::{AssertionQualification,Approximation},attribution::{AnalysisContext,Modality,Fidelity},lexical::*,normalized::Rows,source::{Occurrence,SyntaxKind,SourceArtifact},syntax::{SyntaxPlacement,ImportAliasObservation},resources::{ResourceBudget,Reservation},structural::handoffs};
#[macro_export]
macro_rules! synthesis_setup_inputs {($apply:ident)=>{$apply!{
 bindings:$crate::domain::lexical::BindingObservation,events:$crate::domain::lexical::BindingEvent,
 references:$crate::domain::lexical::ReferenceObservation,resolutions:$crate::domain::lexical::LexicalResolution,targets:$crate::domain::lexical::LexicalTarget,
 imports:$crate::domain::syntax::ImportAliasObservation,
}};}
macro_rules! data{($($f:ident:$ty:ty,)*)=>{pub struct Data{$(pub $f:Rows<$ty>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}}};}crate::synthesis_setup_inputs!(data);
pub const MAX_STATEMENTS:usize=12;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Boundary{UnsupportedBlock,MissingPlacement,UnresolvedRead,MissingNativeEvidence,UnsupportedSetup,StatementLimit,ForeignSource,FlowUnavailable}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum Dependency {
 Builtin{reference:Id<NativeAssertionPremise>,resolution:Id<NativeAssertionPremise>},
 Import{reference:Id<NativeAssertionPremise>,resolution:Id<NativeAssertionPremise>,binding:Id<NativeAssertionPremise>,alias:Id<NativeAssertionPremise>},
 Internal{reference:Id<NativeAssertionPremise>,resolution:Id<NativeAssertionPremise>,binding:Id<NativeAssertionPremise>},
 Named{reference:Id<NativeAssertionPremise>,value:handoffs::ValueSource},
}
/// Only this operation can admit a statement closure. Original bytes remain in canonical chunks.
pub struct Plan{statements:Vec<(Id<Occurrence>,Id<NativeAssertionPremise>)>,dependencies:Vec<Dependency>,_reservation:Box<dyn Reservation>}
impl Plan{pub fn statements(&self)->&[(Id<Occurrence>,Id<NativeAssertionPremise>)]{&self.statements}pub fn dependencies(&self)->&[Dependency]{&self.dependencies}}
fn invalid(s:&str)->ModelError{ModelError::Invalid(s.into())}
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ModelError>{rows.get(id).ok_or_else(||invalid("source setup input absent"))}
fn exact(q:&AssertionQualification,context:Id<AnalysisContext>,artifact:Id<SourceArtifact>)->bool{q.context==context&&q.scope==(source::CoverageScope::Artifact{artifact}).id()&&q.modality==Modality::Definite&&q.approximation==Approximation::Exact}
fn native(d:&documentary::Data,q:Id<AssertionQualification>,predicate:impl Fn(&NativeAssertionPremise)->bool)->Option<Id<NativeAssertionPremise>>{d.native.iter().filter(|p|predicate(p)&&d.native_qualifications.iter().any(|n|n.premise==p.id()&&n.qualification==q&&n.family==p.family()&&n.fidelity!=Fidelity::DisplayOnly)).map(Record::id).min()}
fn placed<'a>(d:&'a documentary::Data,id:Id<Occurrence>,context:Id<AnalysisContext>,source:Id<SourceArtifact>)->Result<Result<(&'a SyntaxPlacement,Id<NativeAssertionPremise>),Boundary>,ModelError>{let mut rows=d.placements.iter().filter(|p|p.occurrence==id&&d.qualifications.get(p.qualification).is_some_and(|q|exact(q,context,source)));let Some(row)=rows.next()else{return Ok(Err(Boundary::MissingPlacement))};if rows.next().is_some(){return Ok(Err(Boundary::MissingPlacement));}let Some(premise)=native(d,row.qualification,|p|matches!(p,NativeAssertionPremise::SyntaxPlacement{assertion,..}if *assertion==row.id()))else{return Ok(Err(Boundary::MissingNativeEvidence))};Ok(Ok((row,premise)))}
fn statement(d:&documentary::Data,mut id:Id<Occurrence>,context:Id<AnalysisContext>,source:Id<SourceArtifact>)->Result<Result<(Id<Occurrence>,Id<NativeAssertionPremise>,Id<Occurrence>,SyntaxField),Boundary>,ModelError>{for _ in 0..=d.placements.len(){let row=need(&d.occurrences,id)?;if row.source!=source{return Ok(Err(Boundary::ForeignSource));}let(p,premise)=match placed(d,id,context,source)?{Ok(p)=>p,Err(r)=>return Ok(Err(r))};if (2..=26).contains(&row.syntax_kind.code()){let Some(parent)=p.parent else{return Ok(Err(Boundary::UnsupportedBlock))};let parentrow=need(&d.occurrences,parent)?;if p.field!=SyntaxField::Body||!matches!(parentrow.syntax_kind,SyntaxKind::ModModule|SyntaxKind::StmtFunctionDef){return Ok(Err(Boundary::UnsupportedBlock));}return Ok(Ok((id,premise,parent,p.field)));}let Some(parent)=p.parent else{return Ok(Err(Boundary::UnsupportedBlock))};id=parent;}Err(invalid("source placement cycle"))}
fn inside(d:&documentary::Data,mut id:Id<Occurrence>,statement:Id<Occurrence>,context:Id<AnalysisContext>,source:Id<SourceArtifact>)->Result<Result<bool,Boundary>,ModelError>{for _ in 0..=d.placements.len(){if id==statement{return Ok(Ok(true));}let(p,_)=match placed(d,id,context,source)?{Ok(p)=>p,Err(r)=>return Ok(Err(r))};let Some(parent)=p.parent else{return Ok(Ok(false))};id=parent;}Err(invalid("source placement cycle"))}
/// `sites` are exact earlier source premises (the final consumer supplies the whole handoff).
/// Imports resolve by Alias/BindingEvent IDs; assignment setup reuses the qualified flow owner.
pub fn close(d:&documentary::Data,setup:&Data,flow:Option<&handoffs::Data>,sites:&[Id<Occurrence>],context:Id<AnalysisContext>,b:&ResourceBudget)->Result<Result<Plan,Boundary>,ModelError>{
 if sites.is_empty(){return Ok(Err(Boundary::UnsupportedSetup));}let source=need(&d.occurrences,sites[0])?.source;
 let allowance=setup.references.len().checked_mul(size_of::<Dependency>()+64).and_then(|n|n.checked_add(MAX_STATEMENTS*128)).ok_or_else(||invalid("source setup allowance overflow"))?;let reservation=b.reserve("synthesis-source-setup",allowance)?;
 let mut statements=Vec::with_capacity(MAX_STATEMENTS);let mut dependencies=Vec::new();let mut block=None;
 for site in sites{let(s,p,parent,field)=match statement(d,*site,context,source)?{Ok(r)=>r,Err(r)=>return Ok(Err(r))};if block.is_some_and(|v|v!=(parent,field)){return Ok(Err(Boundary::UnsupportedBlock));}block=Some((parent,field));if !statements.iter().any(|(id,_)|*id==s){if statements.len()==MAX_STATEMENTS{return Ok(Err(Boundary::StatementLimit));}statements.push((s,p));}}
 let root_start=statements.iter().map(|(id,_)|need(&d.occurrences,*id).map(|o|o.start)).collect::<Result<Vec<_>,_>>()?.into_iter().min().unwrap();let mut index=0;
 while index<statements.len(){let current=statements[index].0;index+=1;
  for reference in setup.references.iter(){let read=need(&d.occurrences,reference.read)?;if read.source!=source||!exact(need(&d.qualifications,reference.qualification)?,context,source){continue;}let belongs=match inside(d,reference.read,current,context,source)?{Ok(r)=>r,Err(r)=>return Ok(Err(r))};if !belongs{continue;}
   let Some(rp)=native(d,reference.qualification,|p|matches!(p,NativeAssertionPremise::ReferenceObservation{assertion,..}if *assertion==reference.id()))else{return Ok(Err(Boundary::MissingNativeEvidence))};
   let mut resolutions=setup.resolutions.iter().filter(|r|r.read==reference.read&&d.qualifications.get(r.qualification).is_some_and(|q|exact(q,context,source)));let Some(resolved)=resolutions.next()else{return Ok(Err(Boundary::UnresolvedRead))};if resolved.captured||resolutions.next().is_some(){return Ok(Err(Boundary::UnresolvedRead));}
   let Some(lp)=native(d,resolved.qualification,|p|matches!(p,NativeAssertionPremise::LexicalResolution{assertion,..}if *assertion==resolved.id()))else{return Ok(Err(Boundary::MissingNativeEvidence))};
   match need(&setup.targets,resolved.target)?{LexicalTarget::Builtin{..}=>{dependencies.push(Dependency::Builtin{reference:rp,resolution:lp});continue;},LexicalTarget::Unresolved{..}=>return Ok(Err(Boundary::UnresolvedRead)),LexicalTarget::Binding{event}=>{
    let event=need(&setup.events,*event)?;let mut bindings=setup.bindings.iter().filter(|r|r.event==event.id()&&d.qualifications.get(r.qualification).is_some_and(|q|exact(q,context,source)));let Some(binding)=bindings.next()else{return Ok(Err(Boundary::UnsupportedSetup))};if bindings.next().is_some(){return Ok(Err(Boundary::UnsupportedSetup));}let Some(bp)=native(d,binding.qualification,|p|matches!(p,NativeAssertionPremise::BindingObservation{assertion,..}if *assertion==binding.id()))else{return Ok(Err(Boundary::MissingNativeEvidence))};
    let mut internal=false;for(s,_)in &statements{if matches!(inside(d,event.site,*s,context,source)?,Ok(true)){internal=true;break;}}if internal||binding.kind==BindingEventKind::Implicit{dependencies.push(Dependency::Internal{reference:rp,resolution:lp,binding:bp});continue;}
    let(s,p,parent,field)=match statement(d,event.site,context,source)?{Ok(r)=>r,Err(r)=>return Ok(Err(r))};let owner=need(&d.occurrences,s)?;
    if matches!(binding.kind,BindingEventKind::Import|BindingEventKind::FromImport){let mut aliases=setup.imports.iter().filter(|a|a.alias==event.site&&a.statement==s&&d.qualifications.get(a.qualification).is_some_and(|q|exact(q,context,source)));let Some(alias)=aliases.next()else{return Ok(Err(Boundary::UnsupportedSetup))};if aliases.next().is_some(){return Ok(Err(Boundary::UnsupportedSetup));}let Some(ap)=native(d,alias.qualification,|p|matches!(p,NativeAssertionPremise::ImportAliasObservation{assertion,..}if *assertion==alias.id()))else{return Ok(Err(Boundary::MissingNativeEvidence))};dependencies.push(Dependency::Import{reference:rp,resolution:lp,binding:bp,alias:ap});
    }else{if block!=Some((parent,field))||owner.start>=root_start||!matches!(binding.kind,BindingEventKind::Assignment|BindingEventKind::AnnotationOnly){return Ok(Err(Boundary::UnsupportedSetup));}let Some(flow)=flow else{return Ok(Err(Boundary::FlowUnavailable))};let Some((origin,value))=handoffs::named_definition(flow,read,context,b)?else{return Ok(Err(Boundary::UnsupportedSetup))};if binding.value!=Some(origin){return Ok(Err(Boundary::UnsupportedSetup));}dependencies.push(Dependency::Named{reference:rp,value});}
    if !statements.iter().any(|(id,_)|*id==s){if statements.len()==MAX_STATEMENTS{return Ok(Err(Boundary::StatementLimit));}statements.push((s,p));}
   }}
  }
 }
 statements.sort_by_key(|(id,_)|{let row=d.occurrences.get(*id).unwrap();(row.start,row.end,*id)});Ok(Ok(Plan{statements,dependencies,_reservation:reservation}))
}
