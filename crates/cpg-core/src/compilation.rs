//! One cumulative attempt through analysis/catalog. Semantic owners supply declarations and proofs.
use crate::{facts,analysis_graphs::PreparedGraphs,embedding_service::Embedder,model_runtime::AttemptRuntime,normalize::pipeline::Normalization,stage_runtime::{StageMeasurement,run_declared_stage_with_resources}};
use cpg_extract::bundle::{self,CapturedInputs,ProviderStage};
use lctx_model::domain::{*,admission::{Frontier,FrontierContract},analysis::{AnalysisDefinition,AnalysisMethod,preparation::Configuration,settings::AnalyticsConfiguration},stages::*};
use lctx_postgres::{generations::{GenerationAttempt,GenerationStore},roles::{Role,RoleConfig}};
use std::sync::Arc;

pub struct PreparedCompilation {
    frontier:Frontier,
    configuration:Configuration,
    embedding:Option<embedding::configuration::Configuration>,
    text:embedding::text::TextDefinition,
}
impl PreparedCompilation {
    pub fn new(frontier:Frontier,settings:AnalyticsConfiguration,catalog:&models::Catalog,embedder:Option<&dyn Embedder>,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
        if !matches!(frontier,Frontier::Analysis|Frontier::Catalog){return Err(ModelError::Invalid("upper compilation requires analysis or catalog".into()));}
        settings.validate()?;
        let mut definitions=vec![local_semantics::definition(),execution::configuration::base_evaluation(),execution::configuration::base_completion(),execution::configuration::source_calls(),execution::configuration::enriched_execution(catalog.declaration().id()),execution::configuration::models(catalog.declaration().id()),execution::configuration::summaries(catalog.declaration().id(),execution::configuration::SummaryLimits::default())?,catalog::build::definition(),catalog::evidence::build::definition(),embedding::analytic::definition()];
        for method in structural::build::methods(){definitions.push(structural::build::definition(&settings,method)?);}
        for method in analytics::build::METHODS {definitions.push(analytics::build::definition(&settings,method)?);}
        if frontier==Frontier::Catalog {definitions.extend([selection::build::definition(),synthesis::build::definition(),retrieval::build::definition()]);}
        let mut configuration=Configuration::new(catalog,definitions,budget)?.with_analytics(settings.clone())?;
        if frontier==Frontier::Catalog {configuration=configuration.with_retrieval(retrieval::RetrievalDefinition::builtin(embedder.is_some()))?;}
        let embedding=embedder.map(|e|embedding::configuration::Configuration::new(e.spec(),e.endpoint(),budget)).transpose()?;
        let text=embedding::text::TextDefinition {requested:embedder.is_some()&&(settings.knn||settings.knn_layer),..embedding::text::TextDefinition::builtin()};
        Ok(Self {frontier,configuration,embedding,text})
    }
    fn definition(&self,method:AnalysisMethod)->Result<&AnalysisDefinition,ModelError>{self.configuration.definitions().iter().find(|d|d.method==method).ok_or_else(||ModelError::Invalid("required compilation definition missing".into()))}
    fn settings(&self)->&AnalyticsConfiguration {self.configuration.analytics().iter().next().expect("admitted configuration")}
    pub fn schedule(&self,model:&ValidatedModel,providers:&[Box<dyn ProviderStage<GenerationAttempt>>],profile:Profile)->Result<Schedule,ModelError>{
        let mut declarations:Vec<_>=providers.iter().map(|p|p.declaration(profile)).collect();
        declarations.extend(Normalization::ALL.map(|s|s.declaration(profile)));
        declarations.extend([self.configuration.declaration(),analysis::preparation::native_stage(profile),embedding::configuration::stage(self.embedding.as_ref()),embedding::text::stage(profile,&self.text)?,embedding::analytic::stage(profile,self.text.requested),catalog::build::stage(profile),catalog::evidence::build::stage(profile,model)?,local_semantics::stage(profile,self.definition(AnalysisMethod::LocalTransfers)?,model),execution::production::stage(profile,self.definition(AnalysisMethod::Execution)?,model)?,execution::completion_production::stage(profile,self.definition(AnalysisMethod::Completion)?,model)?,execution::source_call::stage(profile,self.definition(AnalysisMethod::SourceCalls)?,model)?,execution::enriched_production::stage(profile,self.definition(AnalysisMethod::EnrichedExecution)?,model)?,execution::model_production::stage(profile,self.definition(AnalysisMethod::Models)?,model)?,execution::summary_replay::stage(profile,self.definition(AnalysisMethod::Summaries)?,model)?,structural::build::stage(profile,self.settings(),model)?,analytics::build::stage(profile,self.settings(),model)?]);
        if self.frontier==Frontier::Catalog {declarations.extend([selection::build::stage(profile,model)?,synthesis::build::stage(profile,self.settings(),model)?,retrieval::build::stage(profile,self.configuration.retrieval().iter().next().expect("catalog retrieval definition"),model)?]);}
        declarations.push(analysis::frontier::stage(profile,analysis::frontier::Target::Analysis,model)?);
        if self.frontier==Frontier::Catalog {declarations.push(analysis::frontier::stage(profile,analysis::frontier::Target::Catalog,model)?);}
        let facts=declarations.iter().filter(|s|s.profiles.contains(&profile)&&providers.iter().any(|p|p.declaration(profile).name==s.name)&&s.outputs.iter().any(|r|is_vocabulary(r.name()))).map(|s|s.name).collect();
        let mut groups=vec![PublicationGroup::new(PublicationBoundary::Facts,facts),PublicationGroup::new(PublicationBoundary::Local,vec!["analyze_local"])];
        // Remaining closes follow actual new vocabulary, never unused nominal boundaries.
        groups.extend([PublicationGroup::new(PublicationBoundary::Model,vec!["apply_models"]),PublicationGroup::new(PublicationBoundary::Summary,vec!["analyze_summaries"]),PublicationGroup::new(PublicationBoundary::Structural,vec!["analyze_structural"]),PublicationGroup::new(PublicationBoundary::Analytic,vec!["analyze_analytic"])]);
        if self.frontier==Frontier::Catalog {groups.push(PublicationGroup::new(PublicationBoundary::Synthesis,vec!["synthesis"]));}
        let schedule=Schedule::build_with_publications(model,declarations,&[],profile,groups)?;
        FrontierContract::for_frontier(model,profile,self.frontier)?.preflight(&schedule)?;
        for checkpoint in self.frontier.descriptor().checkpoints(){FrontierContract::for_frontier(model,profile,*checkpoint)?.checkpoint_preflight(&schedule)?;}
        Ok(schedule)
    }
}

/// A single attempt owns every collection and lease. Lower checkpoints never publish or select it.
pub async fn publish(store:&GenerationStore,roles:&RoleConfig,writer:sqlx::PgPool,captured:Arc<CapturedInputs>,runtime:&AttemptRuntime,profile:Profile,configuration:ContentHash,prepared:&PreparedCompilation,embedder:Option<&dyn Embedder>,cache:Option<lctx_postgres::Store>)->Result<facts::PublishedGeneration,ModelError>{
    bundle::refuse_ambient(std::env::vars_os())?;
    captured.config().check_profile(profile)?;captured.config().check_budget(runtime.budget())?;
    prepared.configuration.check_budget(runtime.budget())?;
    if let Some(selected)=&prepared.embedding {selected.check_budget(runtime.budget())?;}
    roles.validate().map_err(ModelError::codec)?;
    if roles.role!=Role::Importer {return Err(ModelError::Invalid("upper compilation requires importer credentials".into()));}
    roles.check_capacity(&writer).await.map_err(ModelError::codec)?;
    let model=Arc::new(lctx_model::domain::model()?);
    if model.digest()!=store.model().digest(){return Err(ModelError::Invalid("compilation and store model differ".into()));}
    let mut providers=facts::providers::<GenerationAttempt>(configuration);
    let schedule=prepared.schedule(&model,&providers,profile)?;
    let fact_names:std::collections::BTreeSet<_>=providers.iter().map(|p|p.declaration(profile).name).collect();
    for input in captured.inputs(){input.captured().verify().map_err(ModelError::codec)?;}
    let contract=FrontierContract::for_frontier(&model,profile,prepared.frontier)?;
    let mut execution=schedule.execute();
    let attempt=store.begin(writer,&mut execution,&contract,runtime.budget().clone()).await.map_err(ModelError::from)?;
    let id=attempt.generation();
    let mut measurements=vec![];
    let mut observe=|measurement:StageMeasurement|{tracing::info!(stage=measurement.stage,elapsed_ms=measurement.elapsed_ms as u64,outcome=measurement.outcome,"cumulative compilation stage");measurements.push(measurement);};
    let work=async {
        for stage in schedule.stages(){
            let Some(index)=providers.iter().position(|p|p.declaration(profile).name==stage.name) else {continue;};
            let provider=providers.swap_remove(index);let declaration=provider.declaration(profile);
            run_declared_stage_with_resources(&mut execution,&declaration,runtime.budget(),async |access|bundle::run_stage(provider,access,&attempt,&model,&captured,runtime.budget(),Default::default()).await,&mut observe).await?;
        }
        if providers.iter().any(|p|p.declaration(profile).profiles.contains(&profile)){return Err(ModelError::Invalid("unscheduled facts provider".into()));}
        drop(captured);
        attempt.checkpoint(&execution,&FrontierContract::facts(&model,profile)?).await?;
        for stage in schedule.stages(){
            let Some(normalization)=Normalization::ALL.into_iter().find(|n|n.declaration(profile).name==stage.name) else {continue;};
            run_declared_stage_with_resources(&mut execution,stage,runtime.budget(),async |access|normalization.run(access,&attempt,roles,runtime,&model).await,&mut observe).await?;
        }
        attempt.checkpoint(&execution,&FrontierContract::for_frontier(&model,profile,Frontier::Normalized)?).await?;
        let mut graphs=None;
        for catalog_phase in [false,true] {
            if catalog_phase {
                if prepared.frontier!=Frontier::Catalog {break;}
                attempt.checkpoint(&execution,&FrontierContract::for_frontier(&model,profile,Frontier::Analysis)?).await?;
            }
            for stage in schedule.stages(){
                if Normalization::ALL.into_iter().any(|n|n.declaration(profile).name==stage.name)||fact_names.contains(stage.name){continue;}
                let is_catalog=matches!(stage.name,"catalog_selection"|"synthesis"|"retrieval"|"assess_catalog_frontier");
                if catalog_phase!=is_catalog {continue;}
                run_declared_stage_with_resources(&mut execution,stage,runtime.budget(),async |access|{
                    if matches!(stage.name,"analyze_summaries"|"analyze_structural"|"analyze_analytic")&&graphs.is_none(){graphs=Some(PreparedGraphs::load(&access,&attempt,roles,runtime,&model,&[projection::ProjectionName::CallableInvocation,projection::ProjectionName::DefinitionContainment].into_iter().collect()).await?);}
                    match stage.name {
                        "analysis_configuration"=>crate::analysis_prepare::configuration(access,&attempt,&model,runtime,&prepared.configuration).await,
                        "analysis_native_inventory"=>crate::analysis_prepare::native_inventory(access,&attempt,roles,runtime,&model).await,
                        "embedding_configuration"=>crate::analysis_prepare::embedding_configuration(access,&attempt,&model,runtime,prepared.embedding.as_ref()).await,
                        "analytic_text"=>crate::analytic_text::publish(access,&attempt,roles,runtime,&model,prepared.text.clone()).await,
                        "analytic_embedding"=>crate::analytic_embedding::produce(access,&attempt,roles,runtime,&model,embedder,cache.clone()).await,
                        "catalog_core"=>crate::catalog_core::produce(access,&attempt,roles,runtime,&model).await,
                        "catalog_evidence"=>crate::catalog_evidence::produce(access,&attempt,roles,runtime,&model).await,
                        "catalog_selection"=>crate::catalog_selection::produce(access,&attempt,roles,runtime,&model).await,
                        "analyze_local"=>crate::local_semantics::run(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::LocalTransfers)?).await,
                        "evaluate_base"=>crate::semantic_execution::evaluate_base(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::Execution)?).await,
                        "complete_base"=>crate::semantic_execution::complete_base(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::Completion)?).await,
                        "prepare_source_calls"=>crate::semantic_execution::prepare_source_calls(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::SourceCalls)?).await,
                        "enrich_execution"=>crate::semantic_execution::enrich(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::EnrichedExecution)?).await,
                        "apply_models"=>crate::semantic_models::apply(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::Models)?).await,
                        "analyze_summaries"=>crate::semantic_summaries::produce(access,&attempt,roles,runtime,&model,prepared.definition(AnalysisMethod::Summaries)?,graphs.as_ref().expect("loaded above")).await,
                        "analyze_structural"=>crate::structural::produce(access,&attempt,roles,runtime,&model,graphs.as_ref().expect("loaded above")).await,
                        "analyze_analytic"=>crate::analytic::produce(access,&attempt,roles,runtime,&model,graphs.as_ref().expect("loaded above")).await,
                        "assess_analysis_frontier"=>crate::final_coverage::produce(access,&attempt,roles,runtime,&model,analysis::frontier::Target::Analysis).await,
                        "assess_catalog_frontier"=>crate::final_coverage::produce(access,&attempt,roles,runtime,&model,analysis::frontier::Target::Catalog).await,
                        "synthesis"=>crate::synthesis::produce(access,&attempt,roles,runtime,&model).await,
                        "retrieval"=>crate::retrieval::produce(access,&attempt,roles,runtime,&model,embedder,cache.clone()).await,
                        _=>Err(ModelError::Invalid(format!("unscheduled upper stage {}",stage.name))),
                    }
                },&mut observe).await?;
            }
        }
        drop(graphs);
        execution.finish()
    }.await;
    let receipt=match work {Ok(receipt)=>receipt,Err(error)=>{if let Err(cleanup)=attempt.abort().await{return Err(facts::cleanup_error(id,"cumulative compilation",&error,&ModelError::from(cleanup)));}return Err(error);}};
    facts::finish_publication(store,attempt,receipt,measurements).await
}
