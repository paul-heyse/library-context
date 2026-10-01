//! Shared entry-value producer and stored replay retain unsupported cases as obligations.
#[path="fixtures/stability.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,conditions::{*,entry::*,stability::*},flow::*,lexical::BindingEventKind,assertion::*,attribution::*,normalized::Rows,value::*};
fn replace_reaching(f:&mut Fixture,row:FlowReachingObservation){let mut support=f.data.reaching_supports.iter().next().unwrap().clone();support.assertion=row.id();f.data.reaching=Rows::new(&f.budget);f.data.reaching.insert(row).unwrap();f.data.reaching_supports=Rows::new(&f.budget);f.data.reaching_supports.insert(support).unwrap();}
#[test]
fn entry_value_and_guard_stability_share_one_replayed_access(){
 let f=Fixture::new();let entry=f.guard_entry().unwrap();f.validate(entry.witness()).unwrap();
 assert!(matches!(entry.root(),PlaceRoot::Entry{..}));let proof=StabilityWitness::derive(&f.data,f.guard.id(),&entry).unwrap();assert_eq!(proof.witness().entry,entry.witness().id());
 let inputs=stability_invariants().remove(0).inputs;assert!(!inputs.iter().any(|i|i.name()==GuardSubstitution::NAME || i.name().contains("control_influence")),"Local stability must not depend on Summary outputs");
}
#[test]
fn assignment_nested_unbound_and_loop_reaches_never_prove_entry_identity(){
 for mutation in 0..5 {
  let mut f=Fixture::new();let stored=f.derive().unwrap().witness().clone();
  match mutation {
   0=>{let mut row=f.definition.clone();row.kind=BindingEventKind::Assignment;let mut s=f.data.definition_supports.iter().next().unwrap().clone();s.assertion=row.id();f.data.definition_observations=Rows::new(&f.budget);f.data.definition_observations.insert(row).unwrap();f.data.definition_supports=Rows::new(&f.budget);f.data.definition_supports.insert(s).unwrap();},
   1|2=>{let target=if mutation==1{ReachingDefinition::Nested}else{ReachingDefinition::Unbound};f.data.targets.insert(target.clone()).unwrap();let row=FlowReachingObservation{target:target.id(),..f.reaching.clone()};replace_reaching(&mut f,row);},
   3=>{let row=FlowReachingObservation{loop_carried:true,..f.reaching.clone()};replace_reaching(&mut f,row);},
   _=>{let target=ReachingDefinition::Unbound;f.data.targets.insert(target.clone()).unwrap();let row=FlowReachingObservation{target:target.id(),..f.reaching.clone()};let mut support=f.data.reaching_supports.iter().next().unwrap().clone();support.assertion=row.id();f.data.reaching.insert(row).unwrap();f.data.reaching_supports.insert(support).unwrap();}
  }
  assert!(matches!(f.derive(),Err(ObligationKind::EntryValueUnknown)),"mutation {mutation}");assert!(f.validate(&stored).is_err());
 }
}
#[test]
fn provider_context_support_and_coverage_are_selected_together(){
 for mutation in 0..7 {
  let mut f=Fixture::new();let stored=f.derive().unwrap().witness().clone();
  match mutation {
   0=>{f.data.use_supports=Rows::new(&f.budget);},
   1=>{let mut c=f.coverage.clone();c.status=CoverageStatus::Partial;c.reason=Some(ObligationKind::MissingEvidence);f.data.coverage=Rows::new(&f.budget);f.data.coverage.insert(c).unwrap();},
   2=>{let mut q=f.q.clone();let mut context=AnalysisContext{python_version:"3.14.7".into(),python_platform:"foreign".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"foreign"),environment_digest:ContentHash::of(b"foreign"),lock_digest:None};q.context=context.id();context.python_platform="foreign".into();f.data.qualifications.insert(q.clone()).unwrap();let row=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,row);},
   3=>{let mut s=f.data.reaching_supports.iter().next().unwrap().clone();s.fidelity=Fidelity::ReportProjection;f.data.reaching_supports=Rows::new(&f.budget);f.data.reaching_supports.insert(s).unwrap();},
   4=>{f.data.declaration_supports=Rows::new(&f.budget);},
   5=>{let mut c=f.coverage.clone();c.provider=Some(Provider{tool:"foreign".into(),revision:"1".into(),build_digest:ContentHash::of(b"foreign")}.id());f.data.coverage=Rows::new(&f.budget);f.data.coverage.insert(c).unwrap();},
   _=>{let mut q=f.q.clone();q.modality=Modality::Candidate;f.data.qualifications.insert(q.clone()).unwrap();let row=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,row);}
  }
  assert!(f.derive().is_err(),"mutation {mutation}");assert!(f.validate(&stored).is_err());
 }
}
#[test]
fn binding_identity_never_becomes_mutable_predicate_stability(){
 let mut f=Fixture::new();let entry=f.guard_entry().unwrap();
 for predicate in [Predicate::Truthy,Predicate::Equals{value:Literal::None.id()},Predicate::Opaque{text:"field changed".into()}] {
  f.data.predicates.insert(predicate.clone()).unwrap();let atom=EvaluationAtom{predicate:predicate.id(),..f.guard.clone()};f.data.atoms.insert(atom.clone()).unwrap();assert_eq!(StabilityWitness::derive(&f.data,atom.id(),&entry).unwrap_err(),ObligationKind::ConditionTransferUnsupported);
 }
 assert!(StabilityBasis::ParameterOnlyReaching.eligible(&Predicate::IsValue{value:Literal::None.id()}));
 assert!(!StabilityBasis::ParameterOnlyReaching.eligible(&Predicate::Equals{value:Literal::None.id()}));
}
#[test]
fn stored_witness_cannot_choose_a_different_existing_coverage_premise(){
 let mut f=Fixture::new();let artifact=f.data.artifacts.iter().next().unwrap().id();let scope=lctx_model::domain::source::CoverageScope::Artifact{artifact};f.data.scopes.insert(scope.clone()).unwrap();let other=ProviderCoverage{scope:scope.id(),..f.coverage.clone()};f.data.coverage.insert(other.clone()).unwrap();let proof=f.derive().unwrap();f.validate(proof.witness()).unwrap();let mut forged=proof.witness().clone();forged.coverage=if forged.coverage==other.id(){f.coverage.id()}else{other.id()};assert!(f.validate(&forged).is_err());
}
#[test]
fn entry_allowance_follows_the_checked_guard_until_its_last_consumer_drops(){
 let f=Fixture::new();let before=f.budget.reserved();let entry=f.guard_entry().unwrap();let proof=StabilityWitness::derive(&f.data,f.guard.id(),&entry).unwrap();let retained=f.budget.reserved();assert!(retained>before);let clone=proof.clone();drop(entry);drop(proof);assert_eq!(f.budget.reserved(),retained);drop(clone);assert_eq!(f.budget.reserved(),before);
}


#[test]
fn a_guarded_read_proves_entry_only_when_its_condition_is_covered_by_parameter_reaching(){
 let mut f=Fixture::new();let guarded=Diagram::from_atom(f.guard.id());let(always,nodes)=Diagram::always().records();f.data.conditions.insert(always).unwrap();for row in nodes{f.data.condition_nodes.insert(row).unwrap();}let(condition,nodes)=guarded.records();f.data.conditions.insert(condition).unwrap();for row in nodes{f.data.condition_nodes.insert(row).unwrap();}
 let q=AssertionQualification{condition:guarded.id(),..f.q.clone()};f.data.qualifications.insert(q.clone()).unwrap();let old=f.data.use_observations.iter().next().unwrap().clone();let changed=FlowUseObservation{qualification:q.id(),..old};let mut support=f.data.use_supports.iter().next().unwrap().clone();support.assertion=changed.id();f.data.use_observations=Rows::new(&f.budget);f.data.use_observations.insert(changed).unwrap();f.data.use_supports=Rows::new(&f.budget);f.data.use_supports.insert(support).unwrap();let proof=f.derive().unwrap();f.validate(proof.witness()).unwrap();
 let impossible=Diagram::never();let(condition,nodes)=impossible.records();f.data.conditions.insert(condition).unwrap();for row in nodes{f.data.condition_nodes.insert(row).unwrap();}let q=AssertionQualification{condition:impossible.id(),..f.q.clone()};f.data.qualifications.insert(q.clone()).unwrap();let row=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,row);assert!(f.derive().is_err());assert!(f.validate(proof.witness()).is_err());
}
#[test]
fn guard_identity_must_hold_before_both_truth_arms(){
 use lctx_model::domain::{source::*,normalized::entities::OccurrenceOwnership};
 let mut f=Fixture::new();let read=f.data.occurrences.get(f.request.access).unwrap();let owner=f.data.owners.iter().find(|o|o.occurrence==read.id()).unwrap().clone();
 let statement=Occurrence{source:read.source,start:17,end:read.end+20,syntax_kind:SyntaxKind::StmtIf,role:OccurrenceRole::Syntax,structural_path:vec![0,0,1]};f.data.occurrences.insert(statement.clone()).unwrap();f.data.owners.insert(OccurrenceOwnership{occurrence:statement.id(),..owner}).unwrap();let use_support=f.data.use_supports.iter().next().unwrap().clone();let scope=f.data.use_observations.iter().next().unwrap().scope;
 let region=FlowRegionObservation{qualification:f.q.id(),statement:statement.id(),scope};let region_support=FlowRegionSupport{assertion:region.id(),run:use_support.run,surface:use_support.surface,evidence:use_support.evidence,origin:use_support.origin,mode:use_support.mode,fidelity:use_support.fidelity};f.data.regions.insert(region).unwrap();f.data.region_supports.insert(region_support).unwrap();
 let leaf_condition=Diagram::from_atom(f.guard.id());let(c,nodes)=leaf_condition.records();f.data.conditions.insert(c).unwrap();for row in nodes{f.data.condition_nodes.insert(row).unwrap();}let q=AssertionQualification{condition:leaf_condition.id(),..f.q.clone()};f.data.qualifications.insert(q.clone()).unwrap();let leaf=FlowTestLeafObservation{qualification:q.id(),test:f.guard.evaluation,atom:f.guard.id(),operand:Some(f.request.access)};let support=FlowTestLeafSupport{assertion:leaf.id(),run:use_support.run,surface:use_support.surface,evidence:use_support.evidence,origin:use_support.origin,mode:use_support.mode,fidelity:use_support.fidelity};f.data.leaves.insert(leaf.clone()).unwrap();f.data.leaf_supports.insert(support.clone()).unwrap();
 let source=EntryAccessSource::guard(&f.data,f.request,leaf.id(),support.id()).unwrap();let entry=EntryValueWitness::derive_for(&f.data,f.request,&source,&f.budget).unwrap().unwrap();assert_eq!(entry.condition().id(),Diagram::always().id());StabilityWitness::derive(&f.data,f.guard.id(),&entry).unwrap();
 let old=f.data.use_observations.iter().next().unwrap().clone();let changed=FlowUseObservation{qualification:q.id(),..old};let mut use_support=f.data.use_supports.iter().next().unwrap().clone();use_support.assertion=changed.id();f.data.use_observations=Rows::new(&f.budget);f.data.use_observations.insert(changed).unwrap();f.data.use_supports=Rows::new(&f.budget);f.data.use_supports.insert(use_support).unwrap();
 let row=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,row);let use_entry=f.derive().unwrap();assert!(StabilityWitness::derive(&f.data,f.guard.id(),&use_entry).is_err(),"narrow Use entry cannot certify guard stability");assert!(EntryValueWitness::derive_for(&f.data,f.request,&source,&f.budget).unwrap().is_err(),"positive leaf formula cannot narrow an entry-identity proof");
}
