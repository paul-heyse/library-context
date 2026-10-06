//! Early analytic consumption: exact original text, selected effect and service-free replay.
use crate::{
    embedding_realization,
    embedding_service::Embedder,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, analytic_embedding::*},
    embedding::{
        analytic::{self, AnalysisEmbeddingUse, ConsumptionData, VectorAvailability},
        value,
    },
    normalized::Rows,
    stages::*,
    *,
};
use std::sync::Arc;

async fn load<R: Record>(
    session: &datafusion::prelude::SessionContext,
    rows: &mut Rows<R>,
    permit: &analysis::sources::CompletedInput<R>,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let query = crate::sql::query(&session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(permit, &batch)?;
        rows.decode(&batch)?;
    }
    Ok(())
}
/// Every expected text window gets an exact value or an explicit availability receipt. Runtime
/// corruption and resource failures abort; optional service failure never becomes a zero vector.
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
    let mut data = ConsumptionData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.$field,&permit,&mut admission).await?;)*};}
    lctx_model::analytic_consumption_inputs!(read);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    macro_rules! meta {
        ($ty:ty,$rows:ident,$admit:expr) => {{
            let permit = access.read::<$ty>()?;

            registered.insert(&mut registration, <$ty>::NAME)?;
            load(&session, &mut $rows, &permit, $admit).await?;
        }};
    }
    meta!(analysis::AnalysisDefinition, definitions, &mut admission);
    meta!(analysis::MethodParameters, parameters, &mut admission);
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$({if access.contains::<$ty>() && !registered.contains(<$ty>::NAME){let mut rows=Rows::<$ty>::new(runtime.budget());meta!($ty,rows,&mut admission);}})*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    let selected = data.selected()?;
    let (expected_parameters, definition) = analytic::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "analytic embedding requires its completed authored definition".into(),
        ));
    }
    let mut service = if selected {
        data.specification()?;
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
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "analytic-computation-frames");
    for run in data.runs.iter() {
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    let mut invocations = Rows::new(runtime.budget());
    let mut outcomes = Rows::new(runtime.budget());
    let mut uses = Rows::new(runtime.budget());
    let mut receipts = Rows::new(runtime.budget());
    for (input, context) in frames.iter() {
        let (invocation, parents, source_receipts, projections) = Invocation::admitted(
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
        for receipt in source_receipts {
            receipts.insert(receipt)?;
        }
        if let Some(service) = service.as_mut() {
            let specification = data.specification()?;
            let _batch = runtime.budget().reserve(
                "analytic-request-selection",
                data.windows.len() * size_of::<&str>(),
            )?;
            let mut documents = Vec::new();
            for window in data.windows.iter() {
                if data.owns(&invocation, window)? {
                    documents.push(window.text.as_str());
                }
            }
            service
                .prepare(documents)
                .await
                .map_err(ModelError::codec)?;
            for window in data.windows.iter() {
                if !data.owns(&invocation, window)? {
                    continue;
                }
                match service.realize(window.text.as_str()).await {
                    Ok(value) => {
                        AnalysisEmbeddingUse::admit_into(
                            &mut uses,
                            invocation.id(),
                            window.id(),
                            specification,
                            value,
                            runtime.budget(),
                        )?;
                    }
                    Err(error) => {
                        let (availability, admitted_tokens) =
                            if let embedding_realization::Error::TokenLimit { tokens, .. } = &error
                            {
                                (
                                    VectorAvailability::TokenLimit,
                                    Some(i64::try_from(*tokens).map_err(ModelError::codec)?),
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
                                ModelError::Invalid("analytic request allocation overflow".into())
                            })?;
                        let _request = runtime
                            .budget()
                            .reserve("analytic-unavailable-request", bound)?;
                        uses.insert(AnalysisEmbeddingUse {
                            invocation: invocation.id(),
                            window: window.id(),
                            specification: specification.id(),
                            input: value::input_hash(&spec.document_text(window.text.as_str())),
                            availability,
                            admitted_tokens,
                            codec: None,
                            value_digest: None,
                            bytes: None,
                        })?;
                    }
                }
            }
        }
        outcomes.insert(data.outcome(&invocation, &uses)?)?;
        invocations.insert(invocation)?;
    }
    drop(service);
    data.validate(&invocations, &outcomes, &uses, runtime.budget())?;
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::analytic_embedding::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(AnalysisEmbeddingUse);
    for receipt in receipts.iter() {
        output.push(receipt.clone()).await?;
    }
    for invocation in invocations.iter() {
        let outcome = outcomes
            .iter()
            .find(|o| o.invocation == invocation.id())
            .ok_or_else(|| ModelError::Invalid("analytic outcome absent".into()))?;
        let admitted = coverage::admit(
            invocation,
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
        output.push(outcome.clone()).await?;
        output.push(invocation.clone()).await?;
    }
    for row in uses.iter() {
        output.push(row.clone()).await?;
    }
    drop(data);
    drop(uses);
    drop(outcomes);
    drop(invocations);
    drop(receipts);
    drop(admission);
    drop(sources);
    drop(charge);
    output.finish(ProviderOutcome::Complete).await
}
