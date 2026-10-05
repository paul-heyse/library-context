//! Final E0: completed canonical catalog/S0 text, one nominal owner and immutable vector uses.
use datafusion::execution::context::SessionContext;
use crate::{
    embedding_realization,
    embedding_service::Embedder,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
    retrieval_preparation,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, retrieval::*},
    embedding::{analytic::VectorAvailability, value},
    normalized::Rows,
    retrieval::{
        build,
        consumption::{ConsumptionData, RetrievalEmbeddingUse},
    },
    stages::*,
    *,
};
use std::sync::Arc;
async fn load<R: Record>(
    session: &SessionContext,
    rows: &mut Rows<R>,
    permit: &analysis::sources::CompletedInput<R>,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let query = crate::sql::query(&session,&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(permit, &batch)?;
        rows.decode(&batch)?;
    }
    Ok(())
}
/// Rendering, optional brief inclusion and vector realization share this sole stage writer.
pub async fn produce(
    access: CompletedInputs,
    mut output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    embedder: Option<&dyn Embedder>,
    cache: Option<std::sync::Arc<dyn embedding_realization::EmbeddingCache>>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), runtime.budget())?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let (render, mandatory) =
        retrieval_preparation::mandatory(&access, runtime, model).await?;

    let mut data = ConsumptionData::new(runtime.budget());
    data.render = render;
    let session = access.session(runtime).await?;
    let mut registered = charged::ChargedSet::default();
    let mut registration =
        charged::StateCharge::new(runtime.budget(), "expected-input-registration");
    macro_rules! synthesis{($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.render.synthesis.$f,&permit,&mut admission).await?;)*};}
    lctx_model::retrieval_synthesis_inputs!(synthesis);
    macro_rules! meta {
        ($ty:ty,$rows:expr,$admit:expr) => {{
            let permit = access.read::<$ty>()?;
            
            registered.insert(&mut registration, <$ty>::NAME)?;
            load(&session, $rows, &permit, $admit).await?;
        }};
    }
    meta!(
        embedding::EmbeddingSpec,
        &mut data.specifications,
        &mut admission
    );
    meta!(
        embedding::configuration::ServiceConfiguration,
        &mut data.services,
        &mut admission
    );
    meta!(
        embedding::analytic::AnalysisEmbeddingUse,
        &mut data.analytic_uses,
        &mut admission
    );
    meta!(
        embedding::text::TextWindow,
        &mut data.windows,
        &mut admission
    );
    macro_rules! expected{($($field:ident:$ty:ty,)*)=>{$({if access.contains::<$ty>() && !registered.contains(<$ty>::NAME){let mut rows=Rows::<$ty>::new(runtime.budget());meta!($ty,&mut rows,&mut admission);}})*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    data.output = mandatory;
    build::extend_synthesis(&data.render, &mut data.output, runtime.budget())?;
    let selected = data.render.selected()?.embedding_requested;
    let (_, definition) = build::definition();
    let mut service = if selected {
        data.selected_spec()?;
        Some(
            embedding_realization::Session::open(
                &access,
                runtime,
                model,
                embedder.ok_or_else(|| {
                    build::invalid("requested retrieval service was not provided")
                })?,
                cache,
            )
            .await?,
        )
    } else {
        None
    };
    if let Some(service) = service.as_mut() {
        for row in data.analytic_uses.iter() {
            row.validate()?;
            if row.availability == VectorAvailability::Available {
                let spec = build::need(&data.specifications, row.specification)?;
                let window = build::need(&data.windows, row.window)?;
                service.seed_receipt(spec, window.text.as_str(), row.receipt()?)?;
            }
        }
    }
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "retrieval-native-frames");
    for run in data.render.source.facts.runs.iter() {
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    let mut invocations = Rows::new(runtime.budget());
    let mut outcomes = Rows::new(runtime.budget());
    let mut uses = Rows::new(runtime.budget());
    let mut receipts = Rows::new(runtime.budget());
    for (input, context) in frames.iter() {
        let _parents = runtime.budget().reserve(
            "retrieval-parent-ids",
            2 * size_of::<Id<InvocationSource>>(),
        )?;
        let mut parent_ids = Vec::with_capacity(2);
        for source in data.render.parents(*input, *context)? {
            parent_ids.push(data.sources.insert(source)?);
        }
        let (invocation, parents, source_receipts, projections) = Invocation::admitted(
            *input,
            *context,
            definition.id(),
            None,
            parent_ids,
            &sources,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(build::invalid("retrieval has no projection input"));
        }
        for row in parents {
            data.parents.insert(row)?;
        }
        for row in source_receipts {
            receipts.insert(row)?;
        }
        if let Some(service) = service.as_mut() {
            let specification = data.selected_spec()?;
            service.prepare(data.output.fragments.iter().filter(|f| data.owns(&invocation, f)).map(|f| f.text.as_str()))
                .await.map_err(ModelError::codec)?;
            for fragment in data
                .output
                .fragments
                .iter()
                .filter(|f| data.owns(&invocation, f))
            {
                match service.realize(fragment.text.as_str()).await {
                    Ok(value) => {
                        RetrievalEmbeddingUse::admit_into(
                            &mut uses,
                            invocation.id(),
                            fragment.id(),
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
                        let bound = fragment
                            .text
                            .len()
                            .checked_mul(spec.document_template.matches("{text}").count())
                            .and_then(|n| n.checked_add(spec.document_template.len()))
                            .and_then(|n| n.checked_mul(2))
                            .ok_or_else(|| {
                                build::invalid("retrieval request allocation overflow")
                            })?;
                        let _copy = runtime
                            .budget()
                            .reserve("retrieval-unavailable-request", bound)?;
                        uses.insert(RetrievalEmbeddingUse {
                            invocation: invocation.id(),
                            fragment: fragment.id(),
                            specification: specification.id(),
                            input: value::input_hash(&spec.document_text(fragment.text.as_str())),
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
    data.verify_completion(&invocations, &outcomes, &uses, runtime.budget())?;
    retrieval_preparation::publish_mandatory(&mut output, &data.output).await?;
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::retrieval::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(RetrievalEmbeddingUse);
    for row in data.sources.iter() {
        output.push(row.clone()).await?;
    }
    for row in data.parents.iter() {
        output.push(row.clone()).await?;
    }
    for row in receipts.iter() {
        output.push(row.clone()).await?;
    }
    for invocation in invocations.iter() {
        let outcome = outcomes
            .iter()
            .find(|r| r.invocation == invocation.id())
            .ok_or_else(|| build::invalid("retrieval outcome absent"))?;
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Retrieval,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in members {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                outcome.status,
                outcome.reason,
                runtime.budget(),
            )?;
            output.push(coverage).await?;
            for row in members {
                output.push(row).await?;
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
