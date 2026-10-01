//! Immutable analysis publications have finite nominal owners. Native attribution stays separate.
pub mod native;
pub mod policy;
pub mod support;
pub mod coverage;
pub mod findings;
pub mod sources;
mod config;
mod family;
mod obligation_support;
pub use config::*;
use crate::domain::*;
use family::analysis_family;
analysis_family!(dispatch,"dispatch",[]);
analysis_family!(local,"local",[Dispatch:4=>dispatch],crate::domain::transfer::TransferKey);
analysis_family!(base_evaluation,"base_evaluation",[Local:4=>local],crate::domain::transfer::TransferKey);
analysis_family!(base_completion,"base_completion",[BaseEvaluation:4=>base_evaluation,Local:5=>local],crate::domain::transfer::TransferKey);
analysis_family!(source_call,"source_call",[BaseCompletion:4=>base_completion,BaseEvaluation:5=>base_evaluation,Local:6=>local],crate::domain::transfer::TransferKey);
analysis_family!(enriched_execution,"enriched_execution",[SourceCallAnalysis:4=>source_call,BaseCompletion:5=>base_completion,Local:6=>local],crate::domain::transfer::TransferKey);
analysis_family!(model,"model",[EnrichedExecution:4=>enriched_execution,SourceCallAnalysis:5=>source_call,Local:6=>local],crate::domain::transfer::TransferKey);
analysis_family!(summary,"summary",[Model:4=>model,EnrichedExecution:5=>enriched_execution,SourceCallAnalysis:6=>source_call,Local:7=>local],crate::domain::transfer::TransferKey);
analysis_family!(structural,"structural",[Local:4=>local],crate::domain::transfer::TransferKey);
analysis_family!(analytic_embedding,"analytic_embedding",[]);
analysis_family!(analytic,"analytic",[Structural:4=>structural,AnalyticEmbedding:5=>analytic_embedding],crate::domain::transfer::TransferKey);
analysis_family!(catalog_core,"catalog_core",[]);
analysis_family!(catalog_evidence,"catalog_evidence",[CatalogCore:4=>catalog_core]);
analysis_family!(selection,"selection",[CatalogEvidence:4=>catalog_evidence,Structural:5=>structural,Analytic:6=>analytic],crate::domain::transfer::TransferKey);
analysis_family!(synthesis,"synthesis",[Selection:4=>selection,CatalogEvidence:5=>catalog_evidence,CatalogCore:6=>catalog_core,Structural:7=>structural,Analytic:8=>analytic,Summary:9=>summary],crate::domain::transfer::TransferKey);
analysis_family!(retrieval,"retrieval",[Synthesis:4=>synthesis,CatalogEvidence:5=>catalog_evidence,AnalyticEmbedding:6=>analytic_embedding],crate::domain::transfer::TransferKey);
/// One-shot configuration and native inventory. No future result family is pulled into preflight.
pub fn early_relations()->Vec<Relation> {let mut rows=vec![Relation::of::<AnalysisDefinition>(),Relation::of::<MethodParameters>(),Relation::of::<ProjectionDefinition>()];rows.extend(native::relations());rows}
pub fn relations()->Vec<Relation> {let mut rows=early_relations();rows.extend(dispatch::relations());rows.extend(local::relations());rows.extend(base_evaluation::relations());rows.extend(base_completion::relations());rows.extend(source_call::relations());rows.extend(enriched_execution::relations());rows.extend(model::relations());rows.extend(summary::relations());rows.extend(structural::relations());rows.extend(analytic_embedding::relations());rows.extend(analytic::relations());rows.extend(catalog_core::relations());rows.extend(catalog_evidence::relations());rows.extend(selection::relations());rows.extend(synthesis::relations());rows.extend(retrieval::relations());rows.extend(findings::relations());rows}
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
