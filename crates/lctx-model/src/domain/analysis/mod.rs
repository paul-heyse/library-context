//! Immutable analysis publications have finite nominal owners. Native attribution stays separate.
mod config;
pub mod coverage;
pub mod delegation;
pub mod expected;
mod family;
pub mod findings;
pub mod native;
mod obligation_support;
pub mod policy;
pub mod preparation;
pub mod settings;
pub mod sources;
pub mod support;
pub mod usage;
use crate::domain::*;
pub use config::*;
use family::analysis_family;
analysis_family!(dispatch,"dispatch",[],[],[],[]);
// Local consumes normalized dispatch evidence; unused analysis predecessor code4 stays reserved.
analysis_family!(local,"local",[],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[LocalWitness:8=>crate::domain::local_semantics::LocalContribution,LocalGuard:9=>crate::domain::local_semantics::LocalGuardContribution]);
analysis_family!(base_evaluation,"base_evaluation",[Local:4=>local],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[BaseExpression:8=>crate::domain::execution::records::ExpressionEvaluation]);
analysis_family!(base_completion,"base_completion",[BaseEvaluation:4=>base_evaluation,Local:5=>local],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[BaseStatement:8=>crate::domain::execution::completion_records::StatementCompletion]);
analysis_family!(source_call,"source_call",[BaseCompletion:4=>base_completion,BaseEvaluation:5=>base_evaluation,Local:6=>local],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(enriched_execution,"enriched_execution",[SourceCallAnalysis:4=>source_call,BaseCompletion:5=>base_completion,Local:6=>local],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(model,"model",[EnrichedExecution:4=>enriched_execution,SourceCallAnalysis:5=>source_call,Local:6=>local],[crate::domain::transfer::local::TransferKey;ModelTransfer:10=>crate::domain::transfer::model::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(summary,"summary",[Model:4=>model,EnrichedExecution:5=>enriched_execution,SourceCallAnalysis:6=>source_call,Local:7=>local],[crate::domain::transfer::local::TransferKey;ModelTransfer:10=>crate::domain::transfer::model::TransferKey;SummaryTransfer:11=>crate::domain::transfer::summary::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[TransferWitness:8=>crate::domain::transfer::summary::SummaryWitness]);
analysis_family!(structural,"structural",[Local:4=>local,CatalogCore:5=>catalog_core],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(analytic_embedding,"analytic_embedding",[],[],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(analytic,"analytic",[Structural:4=>structural,AnalyticEmbedding:5=>analytic_embedding],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(catalog_core,"catalog_core",[],[],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(catalog_evidence,"catalog_evidence",[CatalogCore:4=>catalog_core],[],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
// Selection is declaration-owned. Unactivated predecessor codes 5/6 and transfer code 2 remain reserved; optional
// structural/analytic results feed synthesis, not catalog requirement closure.
analysis_family!(selection,"selection",[CatalogEvidence:4=>catalog_evidence],[],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(synthesis,"synthesis",[Selection:4=>selection,CatalogEvidence:5=>catalog_evidence,CatalogCore:6=>catalog_core,Structural:7=>structural,Analytic:8=>analytic,Summary:9=>summary],[crate::domain::transfer::local::TransferKey;ModelTransfer:10=>crate::domain::transfer::model::TransferKey;SummaryTransfer:11=>crate::domain::transfer::summary::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
analysis_family!(retrieval,"retrieval",[Synthesis:4=>synthesis,CatalogEvidence:5=>catalog_evidence,AnalyticEmbedding:6=>analytic_embedding],[crate::domain::transfer::local::TransferKey],[crate::domain::normalized::coverage::NormalizationCoverage],[]);
/// One-shot configuration and native inventory. No future result family is pulled into preflight.
pub fn early_relations() -> Vec<Relation> {
    let mut rows = vec![
        Relation::of::<AnalysisDefinition>(),
        Relation::of::<MethodParameters>(),
        Relation::of::<ProjectionDefinition>(),Relation::of::<settings::AnalyticsConfiguration>(),
    ];
    rows.extend(crate::domain::models::records::relations());
    rows.extend(native::relations());
    rows
}
pub fn relations() -> Vec<Relation> {
    let mut rows = early_relations();
    rows.extend(dispatch::relations());
    rows.extend(local::relations());
    rows.extend(base_evaluation::relations());
    rows.extend(base_completion::relations());
    rows.extend(source_call::relations());
    rows.extend(enriched_execution::relations());
    rows.extend(model::relations());
    rows.extend(summary::relations());
    rows.extend(structural::relations());
    rows.extend(analytic_embedding::relations());
    rows.extend(analytic::relations());
    rows.extend(catalog_core::relations());
    rows.extend(catalog_evidence::relations());
    rows.extend(selection::relations());
    rows.extend(synthesis::relations());
    rows.extend(retrieval::relations());
    rows.extend(findings::relations());
    rows
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
