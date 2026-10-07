//! Incremental exact original-window consumption with one immutable selected effect Session.
use crate::{
    embedding_realization,
    embedding_service::Embedder,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, analytic_embedding::*},
    embedding::{
        analytic::{self, AnalysisEmbeddingUse, ConsumptionData, FrameOutcome, VectorAvailability},
        consumption::SelectedConsumption,
        projection::ProjectedValue,
        text::{TextAssessment, TextAvailability, TextWindow},
        value,
    },
    normalized::Rows,
    stages::*,
    *,
};
use std::sync::Arc;
async fn scan<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    mut consume: impl FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    if crate::consumed_rows::stream_artifact_admission(
        access,
        &ValidationInput::of::<R>(&["id"]),
        session,
        admission,
    )
    .await?
    {
        return Ok(());
    }
    let permit = access.read::<R>()?;
    let table = access.table_for(&ValidationInput::of::<R>(&["id"]))?;
    let mut stream = crate::sql::query(session, &format!("SELECT * FROM \"{table}\""))
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(&permit, &batch)?;
        consume(&batch)?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
fn key_literal(bytes: &[u8]) -> String {
    format!(
        "X'{}'",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
/// Metadata is shared once. Every input/context frame consumes bounded window transfers and
/// emits its values immediately; neither source text nor winning vectors accumulate as output.
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    embedder: Option<&dyn Embedder>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut registered = charged::ChargedSet::default();
    let mut registration =
        charged::StateCharge::new(runtime.budget(), "expected-input-registration");
    let mut metadata = ConsumptionData::new(runtime.budget());
    macro_rules! read {
        ($ty:ty,$rows:expr) => {{
            registered.insert(&mut registration, <$ty>::NAME)?;
            scan::<$ty>(&access, &session, &mut admission, |batch| {
                $rows.decode(batch)
            })
            .await?;
        }};
    }
    read!(embedding::text::TextDefinition, metadata.text_definitions);
    read!(embedding::EmbeddingSpec, metadata.specifications);
    read!(embedding::DocumentRecipe, metadata.documents);
    read!(
        embedding::projection::ProjectionDefinition,
        metadata.projections
    );
    read!(
        embedding::configuration::ServiceConfiguration,
        metadata.services
    );
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge =
        charged::StateCharge::new(runtime.budget(), "analytic-computation-frames");
    scan::<attribution::ProviderRun>(&access, &session, &mut admission, |batch| {
        let _decode = runtime.budget().reserve(
            "analytic-frame-decode",
            decode_allowance::<attribution::ProviderRun>(batch)?,
        )?;
        for row in attribution::ProviderRun::decode(batch)? {
            frames.insert(&mut frame_charge, (row.input, row.context))?;
        }
        Ok(())
    })
    .await?;
    registered.insert(&mut registration, attribution::ProviderRun::NAME)?;
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    read!(analysis::AnalysisDefinition, definitions);
    read!(analysis::MethodParameters, parameters);
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$(if access.contains::<$ty>() && !registered.contains(<$ty>::NAME) {
        scan::<$ty>(&access,&session,&mut admission,|_|Ok(())).await?;registered.insert(&mut registration,<$ty>::NAME)?;
    })*};}
    lctx_model::expected_domain_inputs!(expected);
    let selected = metadata.selected()?;
    let (expected_parameters, definition) = analytic::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "analytic embedding requires its completed authored definition".into(),
        ));
    }
    let mut service = if selected {
        metadata.specification()?;
        Some(
            embedding_realization::Session::open(
                &access,
                runtime,
                model,
                embedder.ok_or_else(|| {
                    ModelError::Invalid("requested embedding service was not provided".into())
                })?,
                cache,
            )
            .await?,
        )
    } else {
        None
    };
    macro_rules! common {($($record:ident,)*)=>{$(output.declare::<analysis::analytic_embedding::$record>()?;)*};}
    lctx_model::analysis_publication!(common);
    output.declare::<AnalysisEmbeddingUse>()?;
    output.declare::<value::FullValue>()?;
    output.declare::<ProjectedValue>()?;
    let _assessments = access.read::<TextAssessment>()?;
    let _windows = access.read::<TextWindow>()?;
    let assessments = access.table_for(&ValidationInput::of::<TextAssessment>(&["id"]))?;
    let windows = access.table_for(&ValidationInput::of::<TextWindow>(&["id"]))?;
    for (input, context) in frames.iter() {
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            *input,
            *context,
            definition.id(),
            None,
            [],
            &sources,
            [],
            runtime.budget(),
        )?;
        if !parents.is_empty() || !projections.is_empty() {
            return Err(ModelError::Invalid(
                "analytic embedding has no future analysis dependency".into(),
            ));
        }
        for receipt in receipts {
            output.push(receipt).await?;
        }
        let frame = format!(
            "input={} AND context={}",
            key_literal(input.bytes()),
            key_literal(context.bytes())
        );
        let mut disposition = FrameOutcome::new(selected);
        let unavailable = format!(
            "SELECT id FROM \"{assessments}\" WHERE {frame} AND availability={} LIMIT 1",
            TextAvailability::Unavailable as i16
        );
        let mut absent = crate::sql::query(&session, &unavailable)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = absent.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() > 0 {
                disposition.unavailable_text();
            }
        }
        drop(absent);
        if let Some(service) = service.as_mut() {
            let specification = metadata.specification()?;
            let document = metadata.document()?;
            let projection = metadata.policy()?;
            let query = format!(
                "SELECT window_row.* FROM \"{windows}\" window_row JOIN \"{assessments}\" assessment ON assessment.id=window_row.assessment WHERE assessment.input={} AND assessment.context={} ORDER BY window_row.id",
                key_literal(input.bytes()),
                key_literal(context.bytes())
            );
            let mut stream = crate::sql::query(&session, &query)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                for first in (0..batch.num_rows()).step_by(64) {
                    let batch = batch.slice(first, (batch.num_rows() - first).min(64));
                    let _decode = runtime.budget().reserve(
                        "analytic-window-decode",
                        decode_allowance::<TextWindow>(&batch)?,
                    )?;
                    let windows = TextWindow::decode(&batch)?;
                    service
                        .prepare(windows.iter().map(|window| window.text.as_str()))
                        .await
                        .map_err(ModelError::codec)?;
                    for window in &windows {
                        let mut uses = Rows::new(runtime.budget());
                        match service.publish(window.text.as_str(), &output).await {
                            Ok(published) => {
                                AnalysisEmbeddingUse::admit_into(
                                    &mut uses,
                                    invocation.id(),
                                    window.id(),
                                    SelectedConsumption {
                                        encoder: specification,
                                        document,
                                        projection,
                                    },
                                    &published,
                                    runtime.budget(),
                                )?;
                            }
                            Err(error) => {
                                let (availability, admitted_tokens) =
                                    if let embedding_realization::Error::TokenLimit {
                                        tokens, ..
                                    } = &error
                                    {
                                        (
                                            VectorAvailability::TokenLimit,
                                            Some(
                                                i64::try_from(*tokens)
                                                    .map_err(ModelError::codec)?,
                                            ),
                                        )
                                    } else if error.unavailable() {
                                        (VectorAvailability::ServiceUnavailable, None)
                                    } else {
                                        return Err(ModelError::codec(error));
                                    };
                                let spec = service.configuration().specification();
                                let bound = window
                                    .text
                                    .len()
                                    .checked_mul(spec.document_template.matches("{text}").count())
                                    .and_then(|n| n.checked_add(spec.document_template.len()))
                                    .and_then(|n| n.checked_mul(2))
                                    .ok_or_else(|| {
                                        ModelError::Invalid(
                                            "analytic request allocation overflow".into(),
                                        )
                                    })?;
                                let _request = runtime
                                    .budget()
                                    .reserve("analytic-unavailable-request", bound)?;
                                uses.insert(AnalysisEmbeddingUse {
                                    invocation: invocation.id(),
                                    window: window.id(),
                                    specification: specification.id(),
                                    document: document.id(),
                                    input: value::input_hash(
                                        &spec.document_text(window.text.as_str()),
                                    ),
                                    availability,
                                    admitted_tokens,
                                    value: None,
                                    projection: None,
                                })?;
                            }
                        }
                        for row in uses.iter() {
                            disposition.consumed(row);
                            output.push(row.clone()).await?;
                        }
                    }
                }
                tokio::task::yield_now().await;
            }
        }
        let outcome = disposition.outcome(invocation.id());
        let admitted = coverage::admit(
            &invocation,
            &definition,
            analysis::AnalysisCapability::AnalyticEmbedding,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for member in members {
                output.push(member).await?;
            }
            for observed in scope.observations() {
                output.push(observed.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                outcome.status,
                outcome.reason,
                runtime.budget(),
            )?;
            output.push(coverage).await?;
            for member in members {
                output.push(member).await?;
            }
        }
        output.push(outcome).await?;
        output.push(invocation).await?;
    }
    drop(service);
    drop(metadata);
    drop(session);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}
