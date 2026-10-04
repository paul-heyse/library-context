//! Actual earlier Local and SourceCall producers for contextual catalog controls.
use lctx_model::domain::{stages::*, *};
pub fn relations() -> Vec<Relation> {
    let mut r = Vec::new();
    r.extend(analysis::local::relations());
    r.extend(transfer::local::relations());
    r.extend(local_semantics::relations());
    r.extend(local_theory::relations());
    r.extend(local_fields::relations());
    macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{$(r.push(Relation::of::<$ty>());)*};}
    lctx_model::local_semantic_outputs!(outputs);
    r.extend(analysis::base_evaluation::relations());
    r.extend(execution::records::relations());
    r.extend(execution::production::relations());
    r.extend(analysis::base_completion::relations());
    r.extend(execution::completion_records::relations());
    r.extend(execution::completion_production::relations());
    r.extend(execution::body_records::relations());
    r.extend(analysis::source_call::relations());
    r.extend(execution::source_call_records::relations());
    r
}
pub fn definitions() -> Vec<(analysis::MethodParameters, analysis::AnalysisDefinition)> {
    vec![
        local_semantics::definition(),
        execution::configuration::base_evaluation(),
        execution::configuration::base_completion(),
        execution::configuration::source_calls(),
    ]
}
pub fn stages(profile: Profile, model: &ValidatedModel) -> Vec<Stage> {
    vec![
        local_semantics::stage(profile, &local_semantics::definition().1, model, &alignment_publication_order()).unwrap(),
        execution::production::stage(
            profile,
            &execution::configuration::base_evaluation().1,
            model,
         &alignment_publication_order())
        .unwrap(),
        execution::completion_production::stage(
            profile,
            &execution::configuration::base_completion().1,
            model,
         &alignment_publication_order())
        .unwrap(),
        execution::source_call::stage(profile, &execution::configuration::source_calls().1, model, &alignment_publication_order())
            .unwrap(),
    ]
}
pub async fn run(
    name: &str,
    access: StageAccess<'_, '_>,
    attempt: &lctx_postgres::generations::GenerationAttempt,
    config: &lctx_postgres::roles::RoleConfig,
    runtime: &cpg_core::model_runtime::AttemptRuntime,
    model: &std::sync::Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    match name {
        "analyze_local" => {
            cpg_core::local_semantics::run(
                access,
                attempt,
                config,
                runtime,
                model,
                &local_semantics::definition().1,
            )
            .await
        }
        "evaluate_base" => {
            cpg_core::semantic_execution::evaluate_base(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::base_evaluation().1,
            )
            .await
        }
        "complete_base" => {
            cpg_core::semantic_execution::complete_base(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::base_completion().1,
            )
            .await
        }
        "prepare_source_calls" => {
            cpg_core::semantic_execution::prepare_source_calls(
                access,
                attempt,
                config,
                runtime,
                model,
                &execution::configuration::source_calls().1,
            )
            .await
        }
        _ => Err(ModelError::Invalid("not a catalog runtime parent".into())),
    }
}

fn alignment_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    use lctx_model::domain::stages::*;
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
        PublicationGroup::new(PublicationBoundary::Model, vec!["model"]),
        PublicationGroup::new(PublicationBoundary::Summary, vec!["summary"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Analytic, vec!["analytic"]),
        PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
    ]).unwrap()
}
