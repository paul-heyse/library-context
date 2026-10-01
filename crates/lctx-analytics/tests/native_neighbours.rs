//! Analytically known unit vectors and adversarial canonical receipts; no embedding-quality claim.
use lctx_analytics::native_neighbours::*;
use lctx_model::domain::{*,embedding::{*,analytic::{self,ConsumptionData,AnalysisEmbeddingUse,VectorAvailability},text::*,value::*},analysis::analytic_embedding::{AnalysisInvocation,AnalysisOutcome},normalized::Rows,resources::ResourceBudget};
fn id<T>(n:u8)->Id<T>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
fn spec()->Spec {
    let mut spec=Spec::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../specs/embedding/qwen3-embedding-8b.json"))).unwrap();spec.dimensions=2;spec.source_dimensions=2;spec.reduction="none".into();spec.admission=None;spec
}
struct Fixture {data:ConsumptionData,invocations:Rows<AnalysisInvocation>,outcomes:Rows<AnalysisOutcome>,uses:Rows<AnalysisEmbeddingUse>,keys:Vec<ItemKey>}
impl Fixture {
    fn new(budget:&ResourceBudget,missing:bool)->Self {
        let mut data=ConsumptionData::new(budget);let spec=spec();let specification=EmbeddingSpec::new(&spec).unwrap();data.specifications.insert(specification.clone()).unwrap();data.services.insert(configuration::ServiceConfiguration {specification:specification.id(),endpoint:"fixture://unit-vectors".into()}).unwrap();let definition=TextDefinition {requested:true,..TextDefinition::builtin()};data.text_definitions.insert(definition.clone()).unwrap();data.runs.insert(attribution::ProviderRun {provider:id(1),context:id(2),input:id(3),configuration:ContentHash::of(b"fixture"),requested_families:ContentHash::of(b"family")}).unwrap();let (invocation,_)=AnalysisInvocation::new(id(3),id(2),analytic::definition().1.id(),None,[]);let mut invocations=Rows::new(budget);invocations.insert(invocation.clone()).unwrap();let mut uses=Rows::new(budget);let mut keys=vec![];
        // Query has two windows. Two targets tie at cosine 1; a third is orthogonal to its best
        // corresponding window but reaches 0.8 overall, so item similarity must use the maximum.
        for (n,vectors) in [(10,vec![[1.0,0.0],[0.6,0.8]]),(11,vec![[1.0,0.0]]),(12,vec![[0.0,1.0]]),(13,vec![[1.0,0.0]])] {
            let assessment=TextAssessment {subject:id(n),definition:definition.id(),input:invocation.input,context:invocation.context,entity:Some(id(n)),source:id(4),availability:TextAvailability::Available,boundary:None};data.assessments.insert(assessment.clone()).unwrap();keys.push(ItemKey {invocation:invocation.id(),subject:assessment.subject});
            for (ordinal,vector) in vectors.iter().enumerate() {
                let text=format!("item{n} window{ordinal}");let window=TextWindow {assessment:assessment.id(),ordinal:ordinal as i64,start:0,end:text.len() as i64,text:text.as_str().into(),content:ContentHash::of(text.as_bytes())};data.windows.insert(window.clone()).unwrap();
                if missing && n==10 && ordinal==1 {uses.insert(AnalysisEmbeddingUse {invocation:invocation.id(),window:window.id(),specification:specification.id(),input:input_hash(&spec.document_text(&text)),availability:VectorAvailability::ServiceUnavailable,admitted_tokens:None,codec:None,value_digest:None,bytes:None}).unwrap();}else {let value=AdmittedValue::new(&spec,&spec.document_text(&text),4,vector,budget).unwrap();AnalysisEmbeddingUse::admit_into(&mut uses,invocation.id(),window.id(),&specification,&value,budget).unwrap();}
            }
        }
        let mut outcomes=Rows::new(budget);outcomes.insert(data.outcome(&invocation,&uses).unwrap()).unwrap();Self {data,invocations,outcomes,uses,keys}
    }
    fn prepare(&self,budget:&ResourceBudget)->Result<Prepared,Error>{Prepared::admit(&self.data,&self.invocations,&self.outcomes,&self.uses,budget)}
}
fn parameters()->Parameters {Parameters {k:3,min_similarity:FiniteF64::new(0.0).unwrap(),max_window_pairs:100,exclude_self:true}}
#[test]
fn exact_window_maximum_ties_thresholds_and_self_exclusion_are_canonical() {
    let budget=ResourceBudget::fixed(1<<22).unwrap();let fixture=Fixture::new(&budget,false);let prepared=fixture.prepare(&budget).unwrap();let q=fixture.keys[0];let targets=&fixture.keys[1..];
    let result=prepared.nearest(&[q],targets,parameters()).unwrap();assert_eq!(result.window_pairs(),6);assert!(!result.incomplete());assert_eq!(result.rows().iter().map(|m|m.target).collect::<Vec<_>>(),[fixture.keys[1],fixture.keys[3],fixture.keys[2]]);assert_eq!(result.rows()[0].score.get(),1.0);assert!((result.rows()[2].score.get()-0.8).abs()<1e-7);
    let reverse:Vec<_>=targets.iter().rev().copied().collect();let shuffled=prepared.nearest(&[q],&reverse,parameters()).unwrap();assert_eq!(result.rows(),shuffled.rows());
    let threshold=prepared.nearest(&[q],targets,Parameters {min_similarity:FiniteF64::new(0.9).unwrap(),..parameters()}).unwrap();assert_eq!(threshold.rows().len(),2);
    let self_excluded=prepared.nearest(&[q],&fixture.keys,parameters()).unwrap();assert_eq!(self_excluded.rows(),result.rows());
    drop(fixture);drop(prepared);assert!(budget.reserved()>0,"result reservation survives input drop");drop(result);drop(shuffled);drop(threshold);drop(self_excluded);assert_eq!(budget.reserved(),0);
}
#[test]
fn work_bounds_missing_windows_and_invalid_universes_never_fabricate_zero_vectors() {
    let budget=ResourceBudget::fixed(1<<22).unwrap();let fixture=Fixture::new(&budget,true);let prepared=fixture.prepare(&budget).unwrap();assert_eq!(prepared.items().iter().find(|i|i.key==fixture.keys[0]).unwrap().availability,Availability::Partial);
    let result=prepared.nearest(&fixture.keys[..1],&fixture.keys[1..],parameters()).unwrap();assert!(result.incomplete());assert_eq!(result.window_pairs(),3);
    assert!(matches!(prepared.nearest(&fixture.keys[..1],&fixture.keys[1..],Parameters {max_window_pairs:2,..parameters()}),Err(Error::WorkLimit {required:3,limit:2})));
    assert!(prepared.nearest(&[fixture.keys[0],fixture.keys[0]],&[],parameters()).is_err());assert!(prepared.nearest(&[],&[ItemKey {subject:id(99),..fixture.keys[0]}],parameters()).is_err());
    drop(result);drop(prepared);drop(fixture);assert_eq!(budget.reserved(),0);
}
#[test]
fn malformed_shapes_and_payloads_refuse_before_any_similarity() {
    let budget=ResourceBudget::fixed(1<<22).unwrap();let fixture=Fixture::new(&budget,false);
    for corruption in 0..3 {
        let mut bad=Rows::new(&budget);for (n,row) in fixture.uses.iter().enumerate() {let mut row=row.clone();if n==0 {match corruption {0=>row.bytes.as_mut().unwrap().0.truncate(4),1=>{row.bytes.as_mut().unwrap().0[..4].copy_from_slice(&f32::NAN.to_le_bytes());},_=>row.value_digest=Some(ContentHash::of(b"bad"))}}bad.insert(row).unwrap();}
        assert!(Prepared::admit(&fixture.data,&fixture.invocations,&fixture.outcomes,&bad,&budget).is_err());
    }
    assert!(fixture.prepare(&ResourceBudget::fixed(1).unwrap()).is_err());drop(fixture);assert_eq!(budget.reserved(),0);
}

#[test]
fn centroid_retains_all_original_winners_and_exact_passage_choice(){
 let b=ResourceBudget::fixed(1<<22).unwrap();let f=Fixture::new(&b,false);let p=f.prepare(&b).unwrap();
 let m=p.centroid(&f.keys[..2],&f.keys[2..],FiniteF64::new(0.5).unwrap(),100).unwrap().unwrap();
 assert_eq!(m.target,f.keys[3]);assert_eq!(m.members.len(),3);assert!((m.score.get()-2.6f64/7.4f64.sqrt()).abs()<1e-7);
 let reversed=p.centroid(&f.keys[..2].iter().rev().copied().collect::<Vec<_>>(),&f.keys[2..].iter().rev().copied().collect::<Vec<_>>(),FiniteF64::new(0.5).unwrap(),100).unwrap().unwrap();assert_eq!(m.target,reversed.target);assert_eq!(m.score,reversed.score);assert_eq!(m.members,reversed.members);
 assert!(matches!(p.centroid(&f.keys[..2],&f.keys[2..],FiniteF64::new(0.5).unwrap(),1),Err(Error::WorkLimit{..})));
 assert!(p.centroid(&[f.keys[0],f.keys[0]],&f.keys[2..],FiniteF64::new(0.5).unwrap(),100).is_err());
 assert!(p.centroid(&[ItemKey{subject:id(99),..f.keys[0]}],&f.keys[2..],FiniteF64::new(0.5).unwrap(),100).is_err());
 assert!(p.centroid(&f.keys[..1],&f.keys[2..],FiniteF64::new(1.1).unwrap(),100).is_err());
 drop(p);drop(f);assert!(b.reserved()>0);drop(m);drop(reversed);assert_eq!(b.reserved(),0);
}
