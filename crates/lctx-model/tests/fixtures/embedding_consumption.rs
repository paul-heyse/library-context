#![allow(dead_code, reason="Shared fixture helpers serve separate consumption and streaming controls")]
use lctx_model::domain::{analysis::analytic_embedding::{AnalysisInvocation,AnalysisOutcome},embedding::{self,projection,analytic::*,consumption::*,text::*,value::*,*},normalized::Rows,resources::ResourceBudget,*};
pub fn id<T>(n:u8)->Id<T> {serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
pub struct Fixture {pub data:ConsumptionData,pub invocation:AnalysisInvocation,pub window:TextWindow,pub encoder:EmbeddingSpec,pub document:DocumentRecipe,pub policy:projection::ProjectionDefinition,pub spec:Spec}
pub fn fixture(b:&ResourceBudget,selected:bool)->Fixture {
 let mut spec=Spec::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../specs/embedding/qwen3-embedding-8b.json"))).unwrap();
 spec.document_template="prefix: {text}".into();let selection=configuration::Configuration::new(&spec,"fixture://service",b).unwrap();
 let encoder=selection.row().clone();let document=selection.document().clone();let policy=selection.projection().clone();
 let mut data=ConsumptionData::new(b);data.specifications.insert(encoder.clone()).unwrap();data.documents.insert(document.clone()).unwrap();data.projections.insert(policy.clone()).unwrap();data.services.insert(selection.service().clone()).unwrap();
 let definition=TextDefinition{requested:selected,..TextDefinition::builtin()};data.text_definitions.insert(definition.clone()).unwrap();
 let run=attribution::ProviderRun{provider:id(6),context:id(3),input:id(2),configuration:ContentHash::of(b"config"),requested_families:ContentHash::of(b"families")};data.runs.insert(run).unwrap();
 let assessment=TextAssessment{subject:id(1),definition:definition.id(),input:id(2),context:id(3),entity:Some(id(4)),source:id(5),availability:TextAvailability::Available,boundary:None};data.assessments.insert(assessment.clone()).unwrap();
 let text="original α bytes";let window=TextWindow{assessment:assessment.id(),ordinal:0,start:0,end:text.len() as i64,text:text.into(),content:ContentHash::of(text.as_bytes())};data.windows.insert(window.clone()).unwrap();
 let(invocation,_)=AnalysisInvocation::new(assessment.input,assessment.context,embedding::analytic::definition().1.id(),None,[]);
 Fixture{data,invocation,window,encoder,document,policy,spec}
}
pub fn winner(f:&Fixture,b:&ResourceBudget)->(FullValue,projection::ProjectedValue,PublishedValue) {
 let mut vector=vec![0.0;4096];vector[0]=0.6;vector[1]=0.8;vector[2]=-0.0;
 let v=AdmittedValue::new(&f.spec,&f.spec.document_text(f.window.text.as_str()),7,&vector,b).unwrap();
 let(full,projected)=projection::admit(&f.encoder,&v,&f.policy).unwrap();
 let published=PublishedValue{value:full.id(),projection:projected.id(),input:full.input,tokens:7};(full,projected,published)
}
pub fn consume(f:&Fixture,published:&PublishedValue,b:&ResourceBudget)->Rows<AnalysisEmbeddingUse> {
 let mut uses=Rows::new(b);AnalysisEmbeddingUse::admit_into(&mut uses,f.invocation.id(),f.window.id(),SelectedConsumption{encoder:&f.encoder,document:&f.document,projection:&f.policy},published,b).unwrap();uses
}
pub fn outcomes(f:&Fixture,uses:&Rows<AnalysisEmbeddingUse>,b:&ResourceBudget)->(Rows<AnalysisInvocation>,Rows<AnalysisOutcome>){let mut invocations=Rows::new(b);invocations.insert(f.invocation.clone()).unwrap();let mut outcomes=Rows::new(b);outcomes.insert(f.data.outcome(&f.invocation,uses).unwrap()).unwrap();(invocations,outcomes)}
pub fn visit<R:Record>(check:&mut dyn InvariantCheck,rows:&[R])->Result<(),ModelError>{check.visit(R::NAME,&R::encode(rows)?)}
pub fn admission(f:&Fixture,b:&ResourceBudget)->Box<dyn InvariantCheck>{
 let invariant=embedding::analytic::invariants().remove(0);let mut check=(invariant.create)(b);
 visit(&mut *check,&[f.encoder.clone()]).unwrap();visit(&mut *check,&[f.document.clone()]).unwrap();visit(&mut *check,&[f.policy.clone()]).unwrap();
 for s in f.data.services.iter(){visit(&mut *check,std::slice::from_ref(s)).unwrap();}
 for d in f.data.text_definitions.iter(){visit(&mut *check,std::slice::from_ref(d)).unwrap();}
 for r in f.data.runs.iter(){visit(&mut *check,std::slice::from_ref(r)).unwrap();}
 for a in f.data.assessments.iter(){visit(&mut *check,std::slice::from_ref(a)).unwrap();}
 visit(&mut *check,std::slice::from_ref(&f.invocation)).unwrap();visit(&mut *check,std::slice::from_ref(&f.window)).unwrap();check
}
