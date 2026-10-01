//! Independently enumerate every C0/native frame and reconcile the two structural methods.
use super::{*,build::{Data,invalid,need}};
use crate::domain::{analysis::{self,structural as owner,settings::AnalyticsConfiguration},normalized::Rows,projection::normalization::ProjectionOutput,resources::ResourceBudget,*};
pub struct Context {
 pub settings:Rows<AnalyticsConfiguration>,pub definitions:Rows<analysis::AnalysisDefinition>,pub parameters:Rows<analysis::MethodParameters>,
 pub local:Rows<analysis::local::Invocation>,pub graphs:ProjectionOutput,
 pub invocations:Rows<owner::Invocation>,pub sources:Rows<owner::InvocationSource>,pub inputs:Rows<owner::AnalysisInput>,
}
impl Context {
 pub fn new(b:&ResourceBudget)->Self{Self{settings:Rows::new(b),definitions:Rows::new(b),parameters:Rows::new(b),local:Rows::new(b),graphs:ProjectionOutput::new(b),invocations:Rows::new(b),sources:Rows::new(b),inputs:Rows::new(b)}}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{
 if self.graphs.visit(n,b)? {return Ok(true);}
 macro_rules! rows {($($f:ident:$ty:ty,)*)=>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*};}
 rows!{settings:AnalyticsConfiguration,definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,local:analysis::local::Invocation,invocations:owner::Invocation,sources:owner::InvocationSource,inputs:owner::AnalysisInput,}Ok(false)
 }
 pub fn configuration(&self)->Result<&AnalyticsConfiguration,ModelError>{let mut settings=self.settings.iter();let row=settings.next().ok_or_else(||invalid("structural analysis requires selected settings"))?;if settings.next().is_some(){return Err(invalid("ambiguous selected analytics configuration"));}row.validate()?;Ok(row)}
 pub fn validation_inputs()->Vec<ValidationInput>{let mut v=ProjectionOutput::validation_inputs();v.extend([ValidationInput::of::<AnalyticsConfiguration>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"]),ValidationInput::of::<analysis::local::Invocation>(&["id"]),ValidationInput::of::<owner::Invocation>(&["id"]),ValidationInput::of::<owner::InvocationSource>(&["id"]),ValidationInput::of::<owner::AnalysisInput>(&["id"])]);v}
 pub fn local_parent(&self,core:&analysis::catalog_core::Invocation)->Result<&analysis::local::Invocation,ModelError>{let mut rows=self.local.iter().filter(|r|r.input==core.input&&r.context==core.context&&r.subject.is_none());let row=rows.next().ok_or_else(||invalid("structural native frame requires Local result, including explicit NotRequested"))?;if rows.next().is_some(){return Err(invalid("ambiguous Local parent"));}Ok(row)}
 pub fn graph(&self,core:&analysis::catalog_core::Invocation,name:projection::ProjectionName)->Result<&projection::ProjectionSourceAssessment,ModelError>{let mut rows=self.graphs.assessments.iter().filter(|r|r.input==core.input&&r.context==core.context&&r.projection==name);let row=rows.next().ok_or_else(||invalid("structural frame requires completed projection"))?;if rows.next().is_some(){return Err(invalid("ambiguous structural projection"));}Ok(row)}
}
pub fn parents(d:&Data,b:&ResourceBudget)->Result<Rows<analysis::catalog_core::Invocation>,ModelError>{catalog::evidence::frames::parents(&d.projection.runs,&d.core_invocations,b)}
pub fn frame(ctx:&Context,core:&analysis::catalog_core::Invocation)->Result<StructuralFrame,ModelError>{
 let settings=ctx.configuration()?;
 let select=|method|->Result<Id<owner::Invocation>,ModelError>{let (parameters,definition)=build::definition(settings,method)?;if ctx.parameters.get(parameters.id())!=Some(&parameters)||ctx.definitions.get(definition.id())!=Some(&definition){return Err(invalid("structural authored definition differs from canonical rules/settings"));}
 let mut rows=ctx.invocations.iter().filter(|r|r.input==core.input&&r.context==core.context&&r.definition==definition.id()&&r.subject.is_none());let row=rows.next().ok_or_else(||invalid("structural invocation domain incomplete"))?;if rows.next().is_some(){return Err(invalid("duplicate structural invocation"));}Ok(row.id())};
 Ok(StructuralFrame{invocation:select(analysis::AnalysisMethod::Delegation)?,usage_invocation:select(analysis::AnalysisMethod::DirectUsage)?,configuration:settings.id(),invocation_graph:ctx.graph(core,projection::ProjectionName::CallableInvocation)?.id(),definition_graph:ctx.graph(core,projection::ProjectionName::DefinitionContainment)?.id()})
}
pub fn verify(d:&Data,c:&Context,actual:&Output,b:&ResourceBudget)->Result<(),ModelError>{
 let parents=parents(d,b)?;
 // Unsheduled owner rows are empty during earlier scoped generations. Once C0 frames exist,
 // the structural stage must publish both methods; coupled removal does not hide the domain.
 if parents.is_empty()&&c.invocations.is_empty()&&actual.frames.is_empty(){return actual.matches(&Output::new(b));}
 let settings=c.configuration()?;let mut expected=Output::new(b);let mut sources=Rows::new(b);let mut inputs=Rows::new(b);let mut count=0;
 for core in parents.iter(){let local=c.local_parent(core)?;let f=frame(c,core)?;let invocation=need(&c.invocations,f.invocation)?;
  for invocation in [f.invocation,f.usage_invocation]{for parent in [owner::InvocationSource::Local{invocation:local.id()},owner::InvocationSource::CatalogCore{invocation:core.id()}]{let id=sources.insert(parent)?;inputs.insert(owner::AnalysisInput{invocation,parent:id})?;}count+=1;}
  let call=build::hydrate(&c.graphs,f.invocation_graph,b)?;let declaration=build::hydrate(&c.graphs,f.definition_graph,b)?;
  expected.extend(build::produce(d,&f,invocation,settings,&call,&declaration,b)?)?;
 }
 if count!=c.invocations.len()||!sources.same(&c.sources)||!inputs.same(&c.inputs){return Err(invalid("structural exact invocation/parent membership differs"));}actual.matches(&expected)
}
pub(super) fn invariants()->Vec<Invariant>{let mut inputs=Data::validation_inputs();inputs.extend(Context::validation_inputs());inputs.extend(Output::validation_inputs());inputs.sort_by_key(|i|(i.name(),i.prefix()));inputs.dedup_by_key(|i|(i.name(),i.prefix()));vec![Invariant{name:"structural_replay",inputs,create:std::sync::Arc::new(|b|Box::new(Check{data:Data::new(b),context:Context::new(b),output:Output::new(b),budget:b.clone()}))}]}
struct Check{data:Data,context:Context,output:Output,budget:ResourceBudget}
impl InvariantCheck for Check{fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{let data=self.data.visit(n,b)?;let context=self.context.visit(n,b)?;let output=self.output.visit(n,b)?;if data||context||output{Ok(())}else{Err(invalid("undeclared structural replay input"))}}fn finish(self:Box<Self>)->Result<(),ModelError>{verify(&self.data,&self.context,&self.output,&self.budget)}}
