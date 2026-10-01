use lctx_analytics::native_concepts::*;
use lctx_model::domain::resources::ResourceBudget;
use std::collections::BTreeSet;
use bitvec::prelude::*;
#[test]
fn nominal_context_matches_independent_fcars_concept_sets_at_each_support_and_input_order() {
 let budget=ResourceBudget::fixed(1<<24).unwrap();
 for seed in 1..6u64 {
  let objects=(0..12).map(|n|n+100).collect::<Vec<u32>>();let attributes=(0..9).map(|n|n+500).collect::<Vec<u16>>();let mut state=seed;let mut incidence=vec![];
  for object in &objects {for attribute in &attributes {state^=state<<13;state^=state>>7;state^=state<<17;if state%3==0 {incidence.push((*object,*attribute));}}}
  let matrix=objects.iter().map(|g|attributes.iter().map(|m|incidence.contains(&(*g,*m))).collect::<BitVec>()).collect::<Vec<_>>();
  let oracle=fcars::FormalContext::new(objects.clone(),attributes.clone(),matrix);let expected=oracle.all_concepts_raw().into_iter().map(|c|(c.extent.iter_ones().map(|i|objects[i]).collect::<Vec<_>>(),c.intent.iter_ones().map(|i|attributes[i]).collect::<Vec<_>>())).collect::<Vec<_>>();
  let context=Context::new(&objects,&attributes,&incidence,&budget).unwrap();let reverse=Context::new(&objects.iter().rev().copied().collect::<Vec<_>>(),&attributes.iter().rev().copied().collect::<Vec<_>>(),&incidence.iter().rev().copied().collect::<Vec<_>>(),&budget).unwrap();
  for support in [0,2,4] {let lattice=context.analyse(support,100_000).unwrap();assert!(!lattice.budget_reached());let ours=lattice.concepts().iter().map(|c|(c.extent.clone(),c.intent.clone())).collect::<BTreeSet<_>>();assert_eq!(ours,expected.iter().filter(|(extent,_)|extent.len()>=support).cloned().collect());let reordered=reverse.analyse(support,100_000).unwrap();assert_eq!(lattice.concepts(),reordered.concepts());assert_eq!(lattice.implications(),reordered.implications());}
 }
 assert_eq!(budget.reserved(),0);
}
#[test]
fn implication_basis_is_sound_and_closes_all_attribute_sets() {
 let budget=ResourceBudget::fixed(1<<22).unwrap();let objects=[1u8,2,3,4];let attributes=[10u16,20,30];let incidence=[(1,10),(1,20),(2,10),(2,20),(3,10),(3,30)];let context=Context::new(&objects,&attributes,&incidence,&budget).unwrap();let lattice=context.analyse(0,1000).unwrap();assert!(!lattice.budget_reached());
 for mask in 0..8 {let mut set=attributes.iter().enumerate().filter(|(i,_)|mask&(1<<i)!=0).map(|(_,a)|*a).collect::<BTreeSet<_>>();let extent=objects.iter().filter(|g|set.iter().all(|a|incidence.contains(&(**g,*a)))).copied().collect::<Vec<_>>();let expected=attributes.iter().filter(|a|extent.iter().all(|g|incidence.contains(&(*g,**a)))).copied().collect::<BTreeSet<_>>();loop {let old=set.clone();for implication in lattice.implications() {if implication.premise.iter().all(|a|set.contains(a)) {set.extend(implication.conclusion.iter().copied());}}if set==old {break;}}assert_eq!(set,expected);}
 drop(context);assert!(budget.reserved()>0);drop(lattice);assert_eq!(budget.reserved(),0);
}
#[test]
fn empty_length_caps_foreign_incidence_and_reservation_limits_are_explicit() {
 let budget=ResourceBudget::fixed(1<<22).unwrap();let context=Context::<u8,u16>::new(&[],&[1,2],&[],&budget).unwrap();let all=context.analyse(0,100).unwrap();assert_eq!(all.concepts(),[Concept {extent:vec![],intent:vec![1,2]}]);assert!(context.analyse(1,100).unwrap().concepts().is_empty());assert!(context.analyse(0,0).unwrap().budget_reached());
 assert!(Context::new(&[1u8],&[2u16],&[(1,3)],&budget).is_err());assert!(Context::new(&[1u8],&[2u16],&[(3,2)],&budget).is_err());assert!(Context::new(&[1u8,1],&[2u16],&[],&budget).is_err());assert!(Context::new(&[1u8],&[2u16],&[],&ResourceBudget::fixed(1).unwrap()).is_err());drop(all);drop(context);assert_eq!(budget.reserved(),0);
}
