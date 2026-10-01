use lctx_analytics::native_ranking::*;
use lctx_model::domain::{*,normalized::entities::EntityRef,resources::ResourceBudget};
use leiden_rs::graph::GraphDataBuilder;
fn id(n:u8)->Id<EntityRef> {serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
#[test]
fn weighted_direction_parallel_pairs_dangling_and_isolates_match_independent_flow_oracle() {
 let budget=ResourceBudget::fixed(1<<22).unwrap();let vertices=(0..6).map(id).collect::<Vec<_>>();let pairs=[(0,1,2),(0,1,1),(0,2,1),(1,2,2),(2,0,1),(3,2,1),(4,0,2),(4,3,1)].map(|(s,t,w)|Pair {source:id(s),target:id(t),weight:w});
 let p=Parameters::retained(1_000_000);let result=rank(&vertices,&pairs,p,&budget).unwrap();assert_eq!(result.stop(),Stop::Converged);assert!(result.residual().unwrap().get()<p.tolerance.get());
 let mut builder=GraphDataBuilder::new(6).directed();for pair in &pairs {builder.add_edge(vertices.iter().position(|v|*v==pair.source).unwrap(),vertices.iter().position(|v|*v==pair.target).unwrap(),pair.weight as f64).unwrap();}
 let oracle=leiden_rs::infomap::compute_flow(&builder.build().unwrap(),1.0-p.damping.get(),1e-14,1000);
 for (row,expected) in result.scores().iter().zip(&oracle) {assert!((row.value.get()-expected.flow).abs()<1e-9);}
 assert!((result.scores().iter().map(|r|r.value.get()).sum::<f64>()-1.0).abs()<1e-12);
 let reversed=rank(&vertices.iter().rev().copied().collect::<Vec<_>>(),&pairs.iter().rev().copied().collect::<Vec<_>>(),p,&budget).unwrap();assert_eq!(result.scores(),reversed.scores());assert_eq!(result.residual(),reversed.residual());drop(result);drop(reversed);assert_eq!(budget.reserved(),0);
}
#[test]
fn empty_iteration_work_and_parameter_limits_are_explicit_and_finite() {
 let budget=ResourceBudget::fixed(1<<22).unwrap();let p=Parameters::retained(1000);let vertices=[id(1),id(2)];let pairs=[Pair {source:id(1),target:id(2),weight:1}];
 let empty=rank(&[],&[],p,&budget).unwrap();assert_eq!(empty.stop(),Stop::Empty);assert!(empty.residual().is_none());
 let uniform=rank(&vertices,&[],p,&budget).unwrap();assert_eq!(uniform.stop(),Stop::Converged);assert!(uniform.scores().iter().all(|r|r.value.get()==0.5));
 let none=rank(&vertices,&pairs,Parameters {max_iterations:0,..p},&budget).unwrap();assert_eq!(none.stop(),Stop::IterationLimit);assert!(none.residual().is_none());
 let one=rank(&vertices,&pairs,Parameters {max_iterations:1,..p},&budget).unwrap();assert_eq!(one.stop(),Stop::IterationLimit);assert_eq!(one.iterations(),1);assert!(one.residual().is_some());
 let work=rank(&vertices,&pairs,Parameters {max_work:2,..p},&budget).unwrap();assert_eq!(work.stop(),Stop::WorkLimit);assert_eq!(work.iterations(),0);assert_eq!(work.work(),0);assert!(work.residual().is_none());
 assert!(rank(&vertices,&pairs,Parameters {damping:FiniteF64::new(1.1).unwrap(),..p},&budget).is_err());assert!(rank(&vertices,&pairs,Parameters {tolerance:FiniteF64::new(0.0).unwrap(),..p},&budget).is_err());
 drop(empty);drop(uniform);drop(none);drop(one);drop(work);assert_eq!(budget.reserved(),0);
}
#[test]
fn malformed_nominal_universe_and_checked_weights_refuse_before_iteration() {
 let budget=ResourceBudget::fixed(1<<22).unwrap();let p=Parameters::retained(1000);let vertices=[id(1),id(2)];
 for pairs in [vec![Pair {source:id(3),target:id(2),weight:1}],vec![Pair {source:id(1),target:id(2),weight:0}],vec![Pair {source:id(1),target:id(2),weight:u64::MAX},Pair {source:id(1),target:id(2),weight:1}],vec![Pair {source:id(1),target:id(2),weight:u64::MAX},Pair {source:id(1),target:id(1),weight:1}]] {assert!(rank(&vertices,&pairs,p,&budget).is_err());}
 assert!(rank(&[id(1),id(1)],&[],p,&budget).is_err());assert!(rank(&vertices,&[],p,&ResourceBudget::fixed(1).unwrap()).is_err());assert_eq!(budget.reserved(),0);
}
