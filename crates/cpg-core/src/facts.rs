//! Independent facts producers into exact persisted completed contribution views.
use crate::workspace::{ProducerOutput, Workspace, WorkspaceOptions};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    ContentHash, ModelError, ValidatedModel,
    batching::TransferLimits,
    stages::{Profile, Schedule},
};
use std::sync::Arc;

/// Run the production facts providers, accepting their enumeration in any order. Dependency
/// ordering is computed from declarations; runtime reads bind exact persisted completed inputs.
pub async fn compile_facts(
    workspace: &Arc<Workspace>,
    captured: &Arc<CapturedInputs>,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
) -> Result<(), ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    captured.config().check_profile(profile)?;
    captured.config().check_budget(workspace.budget())?;
    for input in captured.inputs() {
        input.captured().verify().map_err(ModelError::codec)?;
    }
    let mut offered: Vec<_> = providers
        .into_iter()
        .filter(|p| p.declaration(profile).profiles.contains(&profile))
        .collect();
    let schedule = Schedule::build(
        workspace.model(),
        offered.iter().map(|p| p.declaration(profile)).collect(),
        &[],
        profile,
    )?;
    for declaration in schedule.stages() {
        let position = offered
            .iter()
            .position(|p| p.declaration(profile).name == declaration.name)
            .ok_or_else(|| {
                ModelError::Invalid(format!("missing facts provider {}", declaration.name))
            })?;
        let provider = offered.swap_remove(position);
        let inputs = workspace.inputs(
            declaration.name,
            profile,
            declaration.inputs.iter().map(|r| r.name()),
        )?;
        let output = Arc::new(workspace.producer(declaration, profile, inputs));
        bundle::run_provider(
            provider,
            profile,
            output.clone(),
            workspace.model().clone(),
            captured.clone(),
            workspace.budget().clone(),
            limits,
        )
        .await?;
        Arc::try_unwrap(output)
            .map_err(|_| ModelError::Invalid("native provider retained output ownership".into()))?
            .complete()
            .await?;
    }
    Ok(())
}

pub fn providers(configuration: ContentHash) -> Vec<Box<dyn ProviderStage<ProducerOutput>>> {
    vec![
        Box::new(cpg_extract::acquisition::Acquire::new(configuration)),
        Box::new(cpg_extract::pyrefly_stage::Pyrefly::new(
            cpg_extract::typed_syntax::SyntaxLimits::default(),
        )),
        Box::new(cpg_extract::ty_flow::TyFlow::default()),
        Box::new(cpg_extract::document_parser::Documents),
        Box::new(cpg_extract::deployment::Deployment),
        Box::new(cpg_extract::assembly::Assemble),
    ]
}

pub async fn inspect(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    inspect_with(
        captured,
        options,
        profile,
        providers(ContentHash::of(b"facts-inspection")),
        TransferLimits::default(),
        native,
    )
    .await
}
pub async fn inspect_with(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace =
        Workspace::with_budget(model.clone(), options, captured.config().budget().clone(), native)?;
    if let Err(error)=compile_facts(&workspace,&captured,profile,providers,limits).await {
        let _=workspace.drain().await;return Err(error);
    }
    Ok((model, workspace))
}

pub async fn inspect_with_budget(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
    budget: lctx_model::domain::resources::ResourceBudget,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace = Workspace::with_budget(model.clone(), options, budget, native)?;
    let result=async{
        compile_facts(&workspace,&captured,profile,providers,limits).await?;
        workspace.validate().await?;Ok::<(),ModelError>(())
    }.await;
    if let Err(error)=result {let _=workspace.drain().await;return Err(error);}
    Ok((model, workspace))
}
