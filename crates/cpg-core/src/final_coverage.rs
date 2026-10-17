//! Last immutable frontier assessment over actual captured native frames and owner results.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use lctx_model::domain::{
    analysis::frontier::{self, FrontierData, Target},
    stages::*,
    *,
};
use std::sync::Arc;

fn selected_inputs<R: Record>(
    inputs: &[ValidationInput],
) -> impl Iterator<Item = &ValidationInput> {
    // One type may have several exact selected epochs. A native Facts descriptor
    // cannot stand in for the compact result authored by a later analysis owner.
    inputs
        .iter()
        .filter(|input| input.type_id() == std::any::TypeId::of::<R>())
}

async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    declaration: &ValidationInput,
    data: &mut FrontierData,
) -> Result<(), ModelError> {
    let permit = access.read_at::<R>(declaration.prefix())?;
    crate::consumed_rows::stream_at(&permit, declaration, access, session, |_, batch| {
        data.visit(R::NAME, batch)
    })
    .await
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    target: Target,
) -> Result<(), ModelError> {
    let budget = runtime.budget();
    let _inputs = budget.reserve(
        "final-frontier-reader-inputs",
        FrontierData::inputs(target).len().saturating_mul(1024),
    )?;
    let inputs = FrontierData::inputs(target);
    let mut data = FrontierData::new(access.profile(), budget);
    // Frontier inputs include both native-family and analysis-owner declarations.
    // Facts availability deliberately admits only the native-family subset.
    let owners = runtime.uniform_owners(&access).await?;
    for owner in owners.owners() {
        data.admit_uniform_owner(owner)?;
    }
    let session = access.session(runtime).await?;
    macro_rules! read{($($ty:ty),* $(,)?)=>{$(for input in selected_inputs::<$ty>(&inputs){load::<$ty>(&access,&session,input,&mut data).await?;})*};}
    lctx_model::final_frontier_inputs!(read);
    let mut records = frontier::derive(&data, target, budget)?;
    match target {
        Target::Analysis => {
            output
                .declare_async::<frontier::AnalysisAssessment>()
                .await?;
            output.declare_async::<frontier::AnalysisMember>().await?;
            crate::producer_operations::emit_owned(&mut records.analysis, &output).await?;
            crate::producer_operations::emit_owned(&mut records.analysis_members, &output).await?;
        }
        Target::Catalog => {
            output
                .declare_async::<frontier::CatalogAssessment>()
                .await?;
            output.declare_async::<frontier::CatalogMember>().await?;
            crate::producer_operations::emit_owned(&mut records.catalog, &output).await?;
            crate::producer_operations::emit_owned(&mut records.catalog_members, &output).await?;
        }
    }
    drop(records);
    drop(data);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod controls {
    use super::*;

    #[test]
    fn frontier_reader_preserves_every_selected_uniform_epoch() {
        for target in [Target::Analysis, Target::Catalog] {
            let inputs = FrontierData::inputs(target);
            let selected = selected_inputs::<uniform_outcome::UniformNotRequested>(&inputs)
                .map(ValidationInput::prefix)
                .collect::<Vec<_>>();
            let mut expected = std::collections::BTreeSet::from([Some(PublicationBoundary::Facts)]);
            expected.extend(
                analysis::owner::AnalysisOwner::ALL
                    .into_iter()
                    .filter(|owner| owner.can_be_wholly_unrequested())
                    .map(|owner| owner.boundary()),
            );
            assert_eq!(selected.len(), expected.len());
            assert_eq!(
                selected
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>(),
                expected
            );
            assert_eq!(
                selected_inputs::<attribution::ProviderRun>(&inputs).count(),
                1,
                "ordinary inputs still select their one canonical role"
            );
            assert_eq!(
                selected_inputs::<analysis::frontier::CatalogAssessment>(&inputs).count(),
                0,
                "an undeclared successor cannot become a frontier read"
            );
        }
    }
}
