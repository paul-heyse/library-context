//! Store the explicit authored configuration and actual native premise projection once.
use crate::workspace::{CheckedInputs, CompletedInputs, ProducerOutput, Workspace};
use futures::future::BoxFuture;
use lctx_model::domain::{
    analysis::{
        native::*,
        preparation::{Configuration, ConfigurationGroup},
        *,
    },
    assertion::AssertionQualification,
    models::{AuthoredContextProtocol, AuthoredModel, AuthoredTarget, ModelCatalog},
    stages::*,
    *,
};
use std::sync::Arc;

pub async fn configuration(
    _access: CompletedInputs,
    output: ProducerOutput,
    _model: &ValidatedModel,
    runtime: &Workspace,
    configuration: &Configuration,
    group: ConfigurationGroup,
) -> Result<(), ModelError> {
    configuration.check_budget(runtime.budget())?;
    macro_rules! write {
        ($ty:ty,$rows:expr) => {{
            output.declare_async::<$ty>().await?;
            for row in $rows.iter() {
                output.push(row.clone()).await?;
            }
        }};
    }
    match group {
        ConfigurationGroup::Authored => {
            write!(ModelCatalog, configuration.catalogs().catalogs);
            write!(AuthoredTarget, configuration.catalogs().targets);
            write!(AuthoredModel, configuration.catalogs().models);
            write!(AuthoredContextProtocol, configuration.catalogs().protocols);
        }
        ConfigurationGroup::Methods => {
            write!(MethodParameters, configuration.parameters());
            write!(AnalysisDefinition, configuration.definitions());
        }
        ConfigurationGroup::Projections => {
            write!(ProjectionDefinition, configuration.projections())
        }
        ConfigurationGroup::Analytics => {
            write!(settings::AnalyticsConfiguration, configuration.analytics())
        }
        ConfigurationGroup::Retrieval => write!(
            lctx_model::domain::retrieval::RetrievalDefinition,
            configuration.retrieval()
        ),
    }
    output.finish(ProviderOutcome::Complete).await
}

fn native_input<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declaration: &'a ValidationInput,
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let permit = access.read_at::<R>(declaration.prefix())?;
        let columns = NativeInventory::property_columns(R::NAME)?
            .iter()
            .map(|column| crate::consumed_rows::identifier(column))
            .collect::<Vec<_>>()
            .join(",");
        let selected = format!(
            "SELECT {columns} FROM {}",
            crate::consumed_rows::identifier(&access.table_for(declaration)?)
        );
        crate::consumed_rows::stream_query_at(
            &permit,
            declaration,
            access,
            session,
            &selected,
            |_, batch| inventory.visit_properties(R::NAME, batch),
        )
        .await
    })
}
fn native_declaration<R: Record>(
    declarations: &[ValidationInput],
) -> Result<&ValidationInput, ModelError> {
    declarations
        .iter()
        .find(|input| input.type_id() == std::any::TypeId::of::<R>())
        .ok_or(ModelError::Schema(
            "native inventory typed input declaration",
        ))
}
type PairLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a [ValidationInput],
    &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn native_pair<'a, A: Record, S: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declarations: &'a [ValidationInput],
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        // Presence is the existing assertion-family guard. A skipped pair neither resolves
        // its declarations nor acquires a support permit. Assertion failure precedes support.
        if access.contains::<A>() {
            native_input::<A>(
                access,
                session,
                native_declaration::<A>(declarations)?,
                inventory,
            )
            .await?;
            native_input::<S>(
                access,
                session,
                native_declaration::<S>(declarations)?,
                inventory,
            )
            .await?;
        }
        Ok(())
    })
}
fn load_native_inputs<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declarations: &'a [ValidationInput],
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    macro_rules! pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*) => {
        const PAIRS: &[PairLoader] = &[$(native_pair::<$assertion, $support>,)*];
    };}
    lctx_model::native_analysis_pairs!(pairs);
    Box::pin(async move {
        native_input::<AssertionQualification>(
            access,
            session,
            native_declaration::<AssertionQualification>(declarations)?,
            inventory,
        )
        .await?;
        for load in PAIRS {
            load(access, session, declarations, inventory).await?;
        }
        Ok(())
    })
}
pub async fn native_inventory(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    checked: &CheckedInputs,
) -> Result<(), ModelError> {
    // This is actual semantic admission authority over the complete model-owned native
    // dependency closure, not a grant reconstructed from nominal row/view identities.
    if model.digest() != runtime.model().digest() {
        return Err(ModelError::Conflict("native inventory model authority"));
    }
    let required = NativeInventory::checked_inputs(access.profile(), runtime.model())?;
    checked
        .select(&required)?
        .require_subset(runtime, &access)?;
    let phase = lctx_native::phase::Phase::begin("native_inventory_inputs");
    let mut inventory = NativeInventory::new(runtime.budget());
    let declarations = NativeInventory::inputs();
    let loaded = async {
        let session = access.session(runtime).await?;
        load_native_inputs(&access, &session, &declarations, &mut inventory).await?;
        // Preserve the former collect-before-write refusal boundary without keeping
        // another complete pair/qualification map beside the charged native inventory.
        inventory.for_each(|_, _| Ok(()))
    }
    .await;
    phase.finish_result(&loaded);
    loaded?;
    let phase = lctx_native::phase::Phase::begin("native_inventory_output");
    let emitted = async {
        output.declare_async::<NativeAssertionPremise>().await?;
        output.declare_async::<NativeQualification>().await?;
        for pair in inventory.pairs() {
            let (premise, qualification) = pair?;
            output.push(premise.clone()).await?;
            output.push(qualification).await?;
        }
        Ok::<_, ModelError>(())
    }
    .await;
    phase.finish_result(&emitted);
    emitted?;
    drop(inventory);
    let phase = lctx_native::phase::Phase::begin("native_inventory_completion");
    let completed = output.finish(ProviderOutcome::Complete).await;
    phase.finish_result(&completed);
    completed
}

/// Publish the one selected embedding configuration before analytic or retrieval work.
pub async fn embedding_configuration(
    _access: CompletedInputs,
    output: ProducerOutput,
    _model: &ValidatedModel,
    runtime: &Workspace,
    configuration: Option<&embedding::configuration::Configuration>,
) -> Result<(), ModelError> {
    use embedding::{
        DocumentRecipe, EmbeddingSpec, configuration::ServiceConfiguration,
        projection::ProjectionDefinition,
    };
    if let Some(configuration) = configuration {
        configuration.check_budget(runtime.budget())?;
    }
    output.declare_async::<EmbeddingSpec>().await?;
    output.declare_async::<ServiceConfiguration>().await?;
    output.declare_async::<DocumentRecipe>().await?;
    output.declare_async::<ProjectionDefinition>().await?;
    if let Some(configuration) = configuration {
        output.push(configuration.row().clone()).await?;
        output.push(configuration.service().clone()).await?;
        output.push(configuration.document().clone()).await?;
        output.push(configuration.projection().clone()).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod native_inventory_controls {
    use super::*;
    use crate::workspace::WorkspaceOptions;
    use futures::FutureExt;

    #[tokio::test]
    async fn admitted_native_inventory_empty_domain_uses_actual_checked_views_and_refuses_weakened_authority()
     {
        let model = Arc::new(lctx_model::domain::model().unwrap());
        let native = crate::test_native::store();
        let runtime =
            Workspace::new(model.clone(), WorkspaceOptions::default(), native.clone()).unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let profile = Profile::Catalog;
            let required = NativeInventory::checked_inputs(profile, &model).unwrap();
            let names = required
                .iter()
                .map(ValidationInput::name)
                .collect::<std::collections::BTreeSet<_>>();
            let output = runtime.output(
                "native-property-control-facts",
                profile,
                ContentHash::of(b"native-property-control-facts"),
                runtime
                    .inputs("native-property-control-facts", profile, [])
                    .unwrap(),
                names,
            );
            // This is a genuine empty domain on completed native views, not a nominal
            // CompletedInput authority or a mocked provider. No upper compiler journey runs.
            output.declare_empty_outputs().await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            runtime
                .freeze_inputs_async(PublicationBoundary::Facts)
                .await
                .unwrap();
            let order = PublicationOrder::planning(&[PublicationGroup::new(
                PublicationBoundary::Facts,
                vec!["native-property-control-facts"],
            )])
            .unwrap();
            let stage = analysis::preparation::native_stage(profile, &model, &order).unwrap();
            let access = runtime.stage_inputs(&stage, profile).unwrap();
            let checked = runtime.checked_inputs(&access).await.unwrap();
            let weak = checked
                .select(&[ValidationInput::of::<AssertionQualification>(&["id"])
                    .at_epoch(PublicationBoundary::Facts)])
                .unwrap();
            let refused = runtime.output(
                "native-property-control-refused",
                profile,
                ContentHash::of(b"native-property-control-refused"),
                access.clone(),
                [NativeAssertionPremise::NAME, NativeQualification::NAME],
            );
            assert!(
                native_inventory(access.clone(), refused, &runtime, &model, &weak)
                    .await
                    .is_err()
            );
            assert!(
                runtime.completed::<NativeAssertionPremise>().is_err(),
                "weakened authority cannot publish even an empty successful prefix"
            );
            let output = runtime.output(
                "native-property-control-inventory",
                profile,
                ContentHash::of(b"native-property-control-inventory"),
                access.clone(),
                [NativeAssertionPremise::NAME, NativeQualification::NAME],
            );
            native_inventory(access, output, &runtime, &model, &checked)
                .await
                .unwrap();
            assert_eq!(
                runtime
                    .completed::<NativeAssertionPremise>()
                    .unwrap()
                    .rows(),
                0
            );
            assert_eq!(
                runtime.completed::<NativeQualification>().unwrap().rows(),
                0
            );
        })
        .catch_unwind()
        .await;
        let mut terminal = completion::Completion::default();
        terminal.step(
            "native property control workspace drainage",
            runtime.drain().await,
        );
        terminal.merge(native.abandon_finality_report().await);
        let cleaned = completion::complete::<()>(Ok(()), terminal);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("native property control cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
}
