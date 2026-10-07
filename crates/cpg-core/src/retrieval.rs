//! Final E0: completed canonical catalog/S0 text, one nominal owner and immutable vector uses.
use crate::{
    embedding_realization,
    embedding_service::Embedder,
    retrieval_preparation,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use datafusion::execution::context::SessionContext;
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
    let query = crate::sql::query(session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(permit, &batch)?;
        rows.decode(&batch)?;
    }
    Ok(())
}
fn nominal<R>(bytes: &[u8]) -> Result<Id<R>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
async fn realize(
    output: &build::Output,
    invocation: &Invocation,
    selected: embedding::consumption::SelectedConsumption<'_>,
    publication: &ProducerOutput,
    service: &mut embedding_realization::Session<'_>,
    b: &resources::ResourceBudget,
) -> Result<Rows<RetrievalEmbeddingUse>, ModelError> {
    let specification = selected.encoder;
    let mut uses = Rows::new(b);
    service
        .prepare(
            output
                .windows
                .iter()
                .filter(|window| window.availability == retrieval::WindowAvailability::Ready)
                .map(|window| window.text.as_str()),
        )
        .await
        .map_err(ModelError::codec)?;
    for window in output.windows.iter() {
        if window.availability == retrieval::WindowAvailability::TokenizerUnavailable {
            return Err(build::invalid(
                "requested retrieval window lacks exact local tokenizer admission",
            ));
        }
        if window.availability == retrieval::WindowAvailability::LexicalOnly {
            uses.insert(RetrievalEmbeddingUse {
                invocation: invocation.id(),
                window: window.id(),
                specification: specification.id(),
                input: value::input_hash(window.input_text.as_str()),
                availability: VectorAvailability::TokenLimit,
                admitted_tokens: window.tokens,
                document: selected.document.id(),
                value: None,
                projection: None,
            })?;
            continue;
        }
        if service
            .configuration()
            .specification()
            .document_text(window.text.as_str())
            != window.input_text.as_str()
        {
            return Err(build::invalid(
                "selected tokenizer preprocessing differs from actual encoder input",
            ));
        }
        match service.publish(window.text.as_str(), publication).await {
            Ok(value) => {
                service.verify_published(&value)?;
                RetrievalEmbeddingUse::admit_into(
                    &mut uses,
                    invocation.id(),
                    window.id(),
                    embedding::consumption::SelectedConsumption {
                        encoder: selected.encoder,
                        document: selected.document,
                        projection: selected.projection,
                    },
                    &value,
                    b,
                )?;
            }
            Err(error) => {
                let (availability, admitted_tokens) =
                    if let embedding_realization::Error::TokenLimit { tokens, .. } = &error {
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
                    .ok_or_else(|| build::invalid("retrieval request allocation overflow"))?;
                let _copy = b.reserve("retrieval-unavailable-request", bound)?;
                uses.insert(RetrievalEmbeddingUse {
                    invocation: invocation.id(),
                    window: window.id(),
                    specification: specification.id(),
                    input: value::input_hash(&spec.document_text(window.text.as_str())),
                    availability,
                    admitted_tokens,
                    document: selected.document.id(),
                    value: None,
                    projection: None,
                })?;
            }
        }
    }
    Ok(uses)
}
/// Each actual C1 root or brief is rendered, realized, checked and published before the next.
/// Only exact fixed frame metadata and the private immutable receipt index survive a grain.
pub async fn produce(
    access: CompletedInputs,
    mut output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    embedder: Option<&dyn Embedder>,
    cache: Option<Arc<dyn embedding::cache::EmbeddingCache>>,
) -> Result<(), ModelError> {
    let b = runtime.budget();
    let sources =
        analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), b)?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, b)?;
    let mut preparation =
        retrieval_preparation::Preparation::prepare(&access, runtime, model, &mut admission)
            .await?;
    let selected = preparation.metadata.selected()?.embedding_requested;
    if selected {
        preparation.metadata.set_tokenizer(
            embedder
                .and_then(|e| e.document_tokenizer())
                .ok_or_else(|| {
                    build::invalid("requested retrieval requires acquired local tokenizer assets")
                })?,
        );
    }
    let session = preparation.session();
    let mut data = ConsumptionData::new(b);
    // Configuration is finite authored metadata. E1 text and use rows are read one at a time.
    let permit = access.read::<embedding::EmbeddingSpec>()?;
    load(session, &mut data.specifications, &permit, &mut admission).await?;
    let permit = access.read::<embedding::configuration::ServiceConfiguration>()?;
    load(session, &mut data.services, &permit, &mut admission).await?;
    let permit = access.read::<embedding::DocumentRecipe>()?;
    load(session, &mut data.documents, &permit, &mut admission).await?;
    let permit = access.read::<embedding::projection::ProjectionDefinition>()?;
    load(session, &mut data.projections, &permit, &mut admission).await?;
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
        let prefix = Some(PublicationBoundary::AnalyticEmbedding);
        let full = ValidationInput::of::<embedding::value::FullValue>(&["id"])
            .at_epoch(PublicationBoundary::AnalyticEmbedding);
        let permit = access.read_at::<embedding::value::FullValue>(prefix)?;
        let table = access.table_for(&full)?;
        crate::consumed_rows::stream_query_at(
            &permit,
            &full,
            session,
            &format!(
                "SELECT * FROM {} ORDER BY id",
                crate::consumed_rows::identifier(&table)
            ),
            |_, batch| {
                for row in embedding::value::FullValue::decode(batch)? {
                    service.seed_full_value(&row)?;
                }
                Ok(())
            },
        )
        .await?;
        let projection = ValidationInput::of::<embedding::projection::ProjectedValue>(&["id"])
            .at_epoch(PublicationBoundary::AnalyticEmbedding);
        let permit = access.read_at::<embedding::projection::ProjectedValue>(prefix)?;
        let table = access.table_for(&projection)?;
        crate::consumed_rows::stream_query_at(
            &permit,
            &projection,
            session,
            &format!(
                "SELECT * FROM {} ORDER BY id",
                crate::consumed_rows::identifier(&table)
            ),
            |_, batch| {
                for row in embedding::projection::ProjectedValue::decode(batch)? {
                    service.seed_projection(&row)?;
                }
                Ok(())
            },
        )
        .await?;
    }
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(b, "retrieval-native-frames");
    for run in preparation.metadata.source.facts.runs.iter() {
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    let mut invocations = Rows::new(b);
    let mut outcomes = Rows::new(b);
    let mut receipts = Rows::new(b);
    let (_, definition) = build::definition();
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::retrieval::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    output.declare::<RetrievalEmbeddingUse>()?;
    output.declare::<embedding::value::FullValue>()?;
    output.declare::<embedding::projection::ProjectedValue>()?;
    // Declare the complete owner output once, including genuinely empty families.
    // Each root then contributes rows to these same publications.
    macro_rules! mandatory_publication {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::retrieval_outputs!(mandatory_publication);
    for (input, context) in frames.iter() {
        let mut ids = Vec::with_capacity(2);
        let _ids = b.reserve(
            "retrieval-parent-ids",
            2 * size_of::<Id<InvocationSource>>(),
        )?;
        for source in preparation.metadata.parents(*input, *context)? {
            ids.push(data.sources.insert(source)?);
        }
        let (invocation, parents, source_receipts, projections) = Invocation::admitted(
            *input,
            *context,
            definition.id(),
            None,
            ids,
            &sources,
            [],
            b,
        )?;
        if !projections.is_empty() {
            return Err(build::invalid("retrieval has no projection input"));
        }
        for parent in parents {
            data.parents.insert(parent)?;
        }
        for receipt in source_receipts {
            receipts.insert(receipt)?;
        }
        let mut disposition = retrieval::consumption::Disposition::default();
        for brief in [false, true] {
            let mut roots = if brief {
                preparation.briefs(*input, *context).await?
            } else {
                preparation.roots(*input, *context).await?
            };
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let ids = batch
                    .column_by_name("id")
                    .and_then(|array| {
                        array
                            .as_any()
                            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    })
                    .ok_or(ModelError::Schema("E0 rendering root identity"))?;
                for index in 0..batch.num_rows() {
                    let (render, mandatory) = if brief {
                        preparation
                            .render_brief(&access, nominal(ids.value(index))?, b)
                            .await?
                    } else {
                        preparation
                            .render_root(&access, nominal(ids.value(index))?, b)
                            .await?
                    };
                    let uses = if let Some(service) = service.as_mut() {
                        realize(
                            &mandatory,
                            &invocation,
                            data.selected_consumption()?,
                            &output,
                            service,
                            b,
                        )
                        .await?
                    } else {
                        Rows::new(b)
                    };
                    retrieval::consumption::verify_uses(
                        &mandatory,
                        &invocation,
                        if selected {
                            Some(data.selected_consumption()?)
                        } else {
                            None
                        },
                        &uses,
                        b,
                    )?;
                    disposition.observe(&uses);
                    retrieval_preparation::publish_mandatory(&mut output, &mandatory).await?;
                    for use_ in uses.iter() {
                        output.push(use_.clone()).await?;
                    }
                    drop(uses);
                    drop(mandatory);
                    drop(render);
                }
            }
        }
        outcomes.insert(disposition.outcome(&invocation))?;
        invocations.insert(invocation)?;
    }
    drop(service);
    // Move only prepared metadata into the finite frame checker, never accumulate root payloads.
    data.render = preparation.metadata;
    data.verify_frames(&invocations, &outcomes, &outcomes, b)?;
    for source in data.sources.iter() {
        output.push(source.clone()).await?;
    }
    for parent in data.parents.iter() {
        output.push(parent.clone()).await?;
    }
    for receipt in receipts.iter() {
        output.push(receipt.clone()).await?;
    }
    for invocation in invocations.iter() {
        let outcome = outcomes
            .iter()
            .find(|row| row.invocation == invocation.id())
            .ok_or_else(|| build::invalid("retrieval outcome absent"))?;
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Retrieval,
            &admission,
            b,
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
                b,
            )?;
            output.push(coverage).await?;
            for row in members {
                output.push(row).await?;
            }
        }
        output.push(outcome.clone()).await?;
        output.push(invocation.clone()).await?;
    }
    drop(data);
    drop(receipts);
    drop(outcomes);
    drop(invocations);
    drop(admission);
    drop(sources);
    drop(charge);
    output.finish(ProviderOutcome::Complete).await
}
