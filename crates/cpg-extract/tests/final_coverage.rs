//! Actual captured native frames cannot be replaced by an empty later owner inventory.
#[path="fixtures/transfer_composition.rs"]mod fixture;
use lctx_model::domain::{*,analysis::frontier::{self,FrontierData,Target},normalized::Rows,stages::Profile};
#[tokio::test]
async fn actual_native_frames_refuse_owner_and_input_erasure_and_release_read_charges(){
 let captured=fixture::native_from("phase4_summaries").await;let budget=&captured.budget;let baseline=budget.reserved();
 {
  let mut data=FrontierData::new(Profile::Behavioral,budget);let inputs=FrontierData::inputs(Target::Analysis);
  for(name,batch)in captured.tables.lock().unwrap().iter(){if inputs.iter().any(|i|i.name()==*name){data.visit(name,batch).unwrap();}}
  assert!(!data.runs.is_empty()&&!data.inputs.is_empty(),"independent actual native input/context universe");assert!(budget.reserved()>baseline,"captured rows retain read charges");
  assert!(frontier::derive(&data,Target::Analysis,budget).is_err(),"erased whole owner inventory cannot shrink captured native frames");
  assert!(frontier::derive(&data,Target::Analysis,&resources::ResourceBudget::fixed(1).unwrap()).is_err());
  data.inputs=Rows::new(budget);assert!(frontier::derive(&data,Target::Analysis,budget).is_err(),"erased input still referenced by native run refuses");
 }
 assert_eq!(budget.reserved(),baseline,"final frontier retained rows release with the captured input handle");
}
