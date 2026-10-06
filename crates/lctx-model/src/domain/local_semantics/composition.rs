//! Compact Local condition support and selection composition across independently released sources.
use super::*;
pub struct Composition {
 charge:charged::StateCharge,
 supports:charged::ChargedMap<Id<Condition>,Vec<Id<EvaluationAtom>>>,
 alternatives:charged::ChargedMap<Id<TransferAlternative>,(Id<TransferKey>,Id<AnalysisContext>,Id<CoverageScope>,Id<Condition>)>,
 influences:charged::ChargedMap<Id<ControlInfluence>,(Id<EvaluationAtom>,Id<AnalysisContext>,Id<CoverageScope>)>,
}
impl Composition {
 pub fn new(budget:&resources::ResourceBudget)->Self{Self{charge:charged::StateCharge::new(budget,"local-source-composition"),supports:Default::default(),alternatives:Default::default(),influences:Default::default()}}
 pub fn observe(&mut self,data:&LocalData,records:&LocalRecords,budget:&resources::ResourceBudget)->Result<(),ModelError>{
  if !self.charge.budget().is_some_and(|pool|pool.shares_pool(budget)){return Err(invalid("Local composition changes attempt budget"));}
  let mut nodes=Rows::<ConditionNode>::new(budget);for node in data.entry.condition_nodes.iter().chain(records.nodes.iter()){nodes.insert(node.clone())?;}
  let _decode=budget.reserve("local-source-condition-decode",nodes.len().checked_mul(2048).ok_or_else(||invalid("Local composition decode overflow"))?)?;
  let nodes=nodes.iter().cloned().collect::<Vec<_>>();
  for alternative in records.alternatives.iter(){
   let q=records.qualifications.get(alternative.qualification).or_else(||data.entry.qualifications.get(alternative.qualification)).ok_or_else(||invalid("Local composed alternative qualification absent"))?;
   if !self.supports.contains_key(&q.condition){
    let condition=records.conditions.get(q.condition).or_else(||data.entry.conditions.get(q.condition)).ok_or_else(||invalid("Local composed condition absent"))?;
    let diagram=Diagram::from_records(condition,&nodes)?;
    let _copy=budget.reserve("local-source-support-copy",diagram.support().len().checked_mul(size_of::<Id<EvaluationAtom>>()).ok_or_else(||invalid("Local support copy overflow"))?)?;
    self.supports.insert(&mut self.charge,q.condition,diagram.support().to_vec())?;
   }
   let value=(alternative.transfer,q.context,q.scope,q.condition);
   if self.alternatives.get(&alternative.id()).is_some_and(|old|old!=&value){return Err(invalid("Local composed alternative changed"));}
   self.alternatives.insert(&mut self.charge,alternative.id(),value)?;
  }
  for influence in records.influences.iter(){
   let q=records.qualifications.get(influence.qualification).or_else(||data.entry.qualifications.get(influence.qualification)).ok_or_else(||invalid("Local composed influence qualification absent"))?;
   let value=(influence.atom,q.context,q.scope);
   if self.influences.get(&influence.id()).is_some_and(|old|old!=&value){return Err(invalid("Local composed influence changed"));}
   self.influences.insert(&mut self.charge,influence.id(),value)?;
  }
  Ok(())
 }
 pub fn selections(&self)->impl Iterator<Item=Selection>+'_ {
  self.influences.iter().flat_map(move |(influence,(atom,context,scope))|self.alternatives.iter().filter_map(move |(alternative,(transfer,other_context,other_scope,condition))|{
   (context==other_context && scope==other_scope && self.supports.get(condition).is_some_and(|support|support.binary_search(atom).is_ok())).then_some(Selection{influence:*influence,atom:*atom,alternative:*alternative,transfer:*transfer})
  }))
 }
}
#[cfg(test)]
mod local_composition_controls{
 use super::*;
 fn nominal<R>(value:u8)->Id<R>{serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([value;16].into_iter())).unwrap()}
 #[test]
 fn released_source_conditions_still_select_matching_foreign_source_influence(){
  let budget=resources::ResourceBudget::fixed(128<<10).unwrap();let atom:Id<EvaluationAtom>=nominal(1);
  let diagram=Diagram::from_atom(atom);let(condition,nodes)=diagram.records();
  let qualification=AssertionQualification{context:nominal(2),scope:nominal(3),condition:condition.id(),modality:Modality::Definite,approximation:Approximation::Exact,assumptions:assumptions::AssumptionSet::empty_id()};
  let alternative=TransferAlternative{transfer:nominal(4),qualification:qualification.id(),scope:qualification.scope};
  let mut composition=Composition::new(&budget);
  {let data=LocalData::new(&budget);let mut records=LocalRecords::new(&budget);records.conditions.insert(condition).unwrap();for node in nodes{records.nodes.insert(node).unwrap();}records.qualifications.insert(qualification.clone()).unwrap();records.alternatives.insert(alternative.clone()).unwrap();composition.observe(&data,&records,&budget).unwrap();}
  assert_eq!(composition.selections().count(),0);
  let influence=ControlInfluence{qualification:qualification.id(),input:nominal(5),atom,evaluation:nominal(6)};
  {let data=LocalData::new(&budget);let mut records=LocalRecords::new(&budget);records.qualifications.insert(qualification.clone()).unwrap();records.influences.insert(influence.clone()).unwrap();let foreign=AssertionQualification{scope:nominal(99),..qualification};records.qualifications.insert(foreign.clone()).unwrap();records.influences.insert(ControlInfluence{qualification:foreign.id(),..influence.clone()}).unwrap();composition.observe(&data,&records,&budget).unwrap();}
  let selected:Vec<_>=composition.selections().collect();assert_eq!(selected,vec![Selection{influence:influence.id(),atom,alternative:alternative.id(),transfer:alternative.transfer}]);
  let foreign=resources::ResourceBudget::fixed(128<<10).unwrap();assert!(composition.observe(&LocalData::new(&foreign),&LocalRecords::new(&foreign),&foreign).is_err());
  drop(composition);assert_eq!(budget.reserved(),0);
 }
}
