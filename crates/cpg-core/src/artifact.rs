//! Compiler capture attestation and lower independent artifact admission.
use crate::workspace::Workspace;
pub use lctx_compiler_store::artifact::{
    AdmittedArtifact, RestoredAdmission, VerifiedExport, verify_export, verify_restored,
};
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
use std::sync::Arc;
pub async fn admit(
    workspace: &Arc<Workspace>,
    captured: &cpg_extract::bundle::CapturedInputs,
    frontier: Frontier,
    profile: Profile,
    settings: ContentHash,
) -> Result<AdmittedArtifact, ModelError> {
    workspace
        .freeze_for_admission(Some((captured, frontier, profile, settings)))
        .await?;
    let runtime = workspace.admission_runtime(profile)?;
    lctx_compiler_store::artifact::admit(&runtime, frontier, profile, settings).await
}

/// Materialize the sealed private compilation into genuine native ownership, then run
/// independent native artifact admission. The target is explicit and has its own finality.
pub async fn materialize_and_admit(
    workspace: &Arc<Workspace>,
    captured: &cpg_extract::bundle::CapturedInputs,
    frontier: Frontier,
    profile: Profile,
    settings: ContentHash,
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
) -> Result<AdmittedArtifact, ModelError> {
    workspace
        .freeze_for_admission(Some((captured, frontier, profile, settings)))
        .await?;
    native.set_frontier(frontier)?;
    let runtime = workspace.materialize(native.clone(), profile).await?;
    workspace.retain_materialized_products(&native).await?;
    lctx_compiler_store::artifact::admit(&runtime, frontier, profile, settings).await
}
