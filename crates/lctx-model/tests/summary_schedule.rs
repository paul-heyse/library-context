//! Static schedule contracts. No provider or store qualification is claimed here.
use lctx_model::domain::{*,analysis,execution,stages::*};
#[test]
fn summary_dependencies_use_only_published_nominal_evidence_routes(){
 let model=model().unwrap();let budget=resources::ResourceBudget::fixed(1<<28).unwrap();let catalog=models::Catalog::committed().unwrap();
 for profile in Profile::ALL{
 let pairs=vec![local_semantics::definition(),execution::configuration::base_evaluation(),execution::configuration::base_completion(),execution::configuration::source_calls(),execution::configuration::enriched_execution(catalog.declaration().id()),execution::configuration::models(catalog.declaration().id()),execution::configuration::summaries(catalog.declaration().id(),Default::default()).unwrap()];
 let configuration=analysis::preparation::Configuration::new(&catalog,pairs.clone(),&budget).unwrap();
 let facts=Stage{name:"fixture_facts_declaration",inputs:vec![],outputs:facts_relations().iter().map(RelationUse::of_relation).collect(),contributes:vec![],coverage:vec![],provider:None,profiles:vec![profile],effect:Effect::Pure,code:ContentHash::of(b"static-only"),configuration:ContentHash::of(b"static-only")};
 let mut stages=vec![facts,normalized::entity_normalization::stage(),normalized::relation_normalization::stage(profile),normalized::callable_normalization::stage(profile),normalized::callable_aspects::stage(profile),normalized::receiver::stage(profile),normalized::event_normalization::stage(profile),normalized::binding_normalization::stage(profile),projection::normalization::stage(profile),normalized::coverage::stage(profile),configuration.declaration(),analysis::preparation::native_stage(profile)];
 stages.extend([local_semantics::stage(profile,&pairs[0].1,&model),execution::production::stage(profile,&pairs[1].1,&model).unwrap(),execution::completion_production::stage(profile,&pairs[2].1,&model).unwrap(),execution::source_call::stage(profile,&pairs[3].1,&model).unwrap(),execution::enriched_production::stage(profile,&pairs[4].1,&model).unwrap(),execution::model_production::stage(profile,&pairs[5].1,&model).unwrap()]);
 let summary=execution::summary_replay::stage(profile,&pairs[6].1,&model).unwrap();
 for name in [analysis::base_completion::AnalysisDerivation::NAME,analysis::base_evaluation::AnalysisDerivation::NAME,analysis::enriched_execution::AnalysisDerivation::NAME,analysis::source_call::AnalysisDerivation::NAME]{assert!(!summary.inputs.iter().any(|i|i.name()==name),"invocation membership cannot invent a generic proof producer: {name}");}
 stages.push(summary);let schedule=Schedule::build_with_publications(&model,stages,&[],profile,vec![PublicationGroup::new(PublicationBoundary::Facts,vec!["fixture_facts_declaration"]),PublicationGroup::new(PublicationBoundary::Local,vec!["analyze_local"]),PublicationGroup::new(PublicationBoundary::Model,vec!["apply_models"]),PublicationGroup::new(PublicationBoundary::Summary,vec!["analyze_summaries"])]).unwrap();assert!(schedule.stages().iter().position(|s|s.name=="apply_models").unwrap()<schedule.stages().iter().position(|s|s.name=="analyze_summaries").unwrap());
 }
 assert_eq!(budget.reserved(),0);
}
