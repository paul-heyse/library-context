//! Final E0: completed canonical catalog/S0 text, one nominal owner and immutable vector uses.
use crate::{
    embedding_realization, retrieval_preparation,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use datafusion::execution::context::SessionContext;
use futures::{FutureExt, TryStreamExt};
use lctx_model::domain::embedding::Embedder;
use lctx_model::domain::{
    analysis::{self, retrieval::*},
    embedding::analytic::VectorAvailability,
    normalized::Rows,
    retrieval::{
        build,
        consumption::{ConsumptionData, RetrievalEmbeddingUse},
    },
    stages::*,
    *,
};
use std::sync::Arc;
use tracing::Instrument;
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &SessionContext,
    rows: &mut Rows<R>,
    permit: &analysis::sources::CompletedInput<R>,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let declaration = ValidationInput::of::<R>(&["id"]);
    crate::consumed_rows::stream_at(permit, &declaration, access, session, |permit, batch| {
        admission.visit_if_expected(permit, batch)?;
        rows.decode(batch)
    })
    .await
}

pub(crate) fn nominal<R>(bytes: &[u8]) -> Result<Id<R>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
/// The deque allocation survives popped expectations, so its capacity has its own window lease.
struct PendingUses<T> {
    values: std::collections::VecDeque<T>,
    capacity: charged::StateCharge,
}
impl<T> PendingUses<T> {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            values: Default::default(),
            capacity: charged::StateCharge::new(budget, "retrieval-pending-use-capacity"),
        }
    }
    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    fn front(&self) -> Option<&T> {
        self.values.front()
    }
    fn pop_front(&mut self) -> Option<T> {
        self.values.pop_front()
    }
    fn push_back(&mut self, value: T) -> Result<(), ModelError> {
        if self.values.len() == self.values.capacity() {
            let target = self
                .values
                .capacity()
                .max(2)
                .checked_mul(2)
                .ok_or_else(|| build::invalid("retrieval pending use capacity overflow"))?;
            // Include allocation rounding and the temporary old/new buffers during growth.
            let allowance = target
                .checked_mul(2 * size_of::<T>())
                .ok_or_else(|| build::invalid("retrieval pending use allowance overflow"))?;
            self.capacity
                .grow(allowance.saturating_sub(self.capacity.reserved()))?;
            self.values.reserve_exact(target - self.values.len());
        }
        self.values.push_back(value);
        Ok(())
    }
}
async fn publish_ready(
    pending: &mut PendingUses<retrieval::consumption::PreparedWindowUse>,
    publication: &ProducerOutput,
    service: &mut embedding_realization::Session<'_>,
    disposition: &mut retrieval::consumption::Disposition,
) -> Result<(), ModelError> {
    while let Some(expected) = pending.front() {
        let row = match expected.availability() {
            retrieval::WindowAvailability::TokenizerUnavailable => {
                return Err(build::invalid(
                    "requested retrieval window lacks exact local tokenizer admission",
                ));
            }
            retrieval::WindowAvailability::LexicalOnly => {
                expected.refused(VectorAvailability::TokenLimit, expected.tokens())?
            }
            retrieval::WindowAvailability::Ready => {
                if !service.resolved(expected.input()) {
                    break;
                }
                match service.publish_input(expected.input(), publication).await {
                    Ok(published) => {
                        service.verify_published(&published)?;
                        expected.published(&published)?
                    }
                    Err(embedding_realization::Error::TokenLimit { tokens, .. }) => expected
                        .refused(
                            VectorAvailability::TokenLimit,
                            Some(i64::try_from(tokens).map_err(ModelError::codec)?),
                        )?,
                    Err(error) if error.unavailable() => {
                        expected.refused(VectorAvailability::ServiceUnavailable, None)?
                    }
                    Err(error) => return Err(ModelError::codec(error)),
                }
            }
        };
        disposition.observe_availability(row.availability);
        publication.push(row).await?;
        // The verified row transfers to the output owner's charged pending/native lifecycle.
        // Its durable acknowledgement and whole-stage completion remain with output.finish.
        pending.pop_front();
    }
    Ok(())
}
/// Each rich render is checked and published while live; only charged compact use expectations
/// and the Session's exact pending request chunk survive across roots in a render window.
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
    let mut admission = runtime.coverage_admission(&sources).await?;
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
    load(
        &access,
        session,
        &mut data.specifications,
        &permit,
        &mut admission,
    )
    .await?;
    let permit = access.read::<embedding::configuration::ServiceConfiguration>()?;
    load(
        &access,
        session,
        &mut data.services,
        &permit,
        &mut admission,
    )
    .await?;
    let permit = access.read::<embedding::DocumentRecipe>()?;
    load(
        &access,
        session,
        &mut data.documents,
        &permit,
        &mut admission,
    )
    .await?;
    let permit = access.read::<embedding::projection::ProjectionDefinition>()?;
    load(
        &access,
        session,
        &mut data.projections,
        &permit,
        &mut admission,
    )
    .await?;
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
            &access,
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
            &access,
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
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare_async::<analysis::retrieval::$record>().await?;)*};}
    lctx_model::analysis_publication!(common_publication);
    output.declare_async::<RetrievalEmbeddingUse>().await?;
    output
        .declare_async::<embedding::value::FullValue>()
        .await?;
    output
        .declare_async::<embedding::projection::ProjectedValue>()
        .await?;
    // Declare the complete owner output once, including genuinely empty families.
    // Each root then contributes rows to these same publications.
    macro_rules! mandatory_publication {($($field:ident:$ty:ty,)*)=>{$(output.declare_async::<$ty>().await?;)*};}
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
                for start in (0..batch.num_rows()).step_by(32) {
                    let end = (start + 32).min(batch.num_rows());
                    let _keys = b.reserve(
                        "retrieval-render-root-window",
                        (end - start) * size_of::<[u8; 16]>(),
                    )?;
                    let keys = (start..end)
                        .map(|index| ids.value(index).try_into().map_err(ModelError::codec))
                        .collect::<Result<Vec<[u8; 16]>, ModelError>>()?;
                    let window = preparation
                        .render_window(&access, &keys, brief, b, &runtime.cancellation())
                        .await?;
                    let mut pending = PendingUses::new(b);
                    for (partition, root) in keys.iter().enumerate() {
                        let rendered = window
                            .render(
                                &access,
                                partition,
                                b,
                                &runtime.cancellation(),
                                runtime,
                                &output,
                            )
                            .await;
                        let (render, mandatory) = match rendered {
                            Err(ModelError::Resource { .. })
                                if !pending.is_empty() && service.is_some() =>
                            {
                                let service = service.as_mut().expect("selected service");
                                service.flush_ready().await.map_err(ModelError::codec)?;
                                publish_ready(&mut pending, &output, service, &mut disposition)
                                    .await?;
                                window
                                    .render(
                                        &access,
                                        partition,
                                        b,
                                        &runtime.cancellation(),
                                        runtime,
                                        &output,
                                    )
                                    .await?
                            }
                            result => result?,
                        };
                        if let Some(service) = service.as_mut() {
                            for semantic in mandatory.windows.iter() {
                                if semantic.availability
                                    == retrieval::WindowAvailability::TokenizerUnavailable
                                {
                                    return Err(build::invalid(
                                        "requested retrieval window lacks exact local tokenizer admission",
                                    ));
                                }
                                let unit = mandatory.units.get(semantic.unit).ok_or_else(|| {
                                    build::invalid("retrieval rendered window has no exact unit")
                                })?;
                                let prepare = || {
                                    retrieval::consumption::PreparedWindowUse::new(
                                        semantic,
                                        unit,
                                        &invocation,
                                        &data.selected_consumption()?,
                                        b,
                                    )
                                };
                                let expected = match prepare() {
                                    Err(ModelError::Resource { .. }) if !pending.is_empty() => {
                                        service.flush_ready().await.map_err(ModelError::codec)?;
                                        publish_ready(
                                            &mut pending,
                                            &output,
                                            service,
                                            &mut disposition,
                                        )
                                        .await?;
                                        prepare()?
                                    }
                                    result => result?,
                                };
                                if semantic.availability == retrieval::WindowAvailability::Ready {
                                    let input = service
                                        .queue_ready(semantic.text.as_str())
                                        .boxed()
                                        .await
                                        .map_err(ModelError::codec)?;
                                    if input != expected.input() {
                                        return Err(build::invalid(
                                            "queued retrieval input differs from checked expectation",
                                        ));
                                    }
                                }
                                pending.push_back(expected)?;
                                publish_ready(&mut pending, &output, service, &mut disposition)
                                    .await?;
                            }
                        }
                        async {
                            let phase = lctx_native::phase::Phase::begin("retrieval_required_publication");
                            let result = retrieval_preparation::publish_mandatory(&mut output, &mandatory).await;
                            phase.finish_result(&result);
                            result
                        }.instrument(tracing::info_span!(target:"lctx_phase", "retrieval_publication", brief, ?root)).await?;
                        drop(mandatory);
                        drop(render);
                    }
                    if let Some(service) = service.as_mut() {
                        service.flush_ready().await.map_err(ModelError::codec)?;
                        publish_ready(&mut pending, &output, service, &mut disposition).await?;
                        if !pending.is_empty() {
                            return Err(build::invalid(
                                "retrieval render window retains unresolved uses",
                            ));
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_use_capacity_stays_charged_after_partial_and_complete_drains() {
        let budget = resources::ResourceBudget::fixed(8192).unwrap();
        let mut pending = PendingUses::new(&budget);
        for value in 0usize..64 {
            pending.push_back(value).unwrap();
        }
        let retained = budget.reserved();
        assert!(retained >= pending.values.capacity() * size_of::<usize>());
        for value in 0usize..63 {
            assert_eq!(pending.pop_front(), Some(value));
        }
        assert_eq!(budget.reserved(), retained);
        assert_eq!(pending.pop_front(), Some(63));
        assert!(pending.is_empty());
        assert_eq!(budget.reserved(), retained);
        pending.push_back(64).unwrap();
        assert_eq!(budget.reserved(), retained);
        drop(pending);
        assert_eq!(budget.reserved(), 0);
        let tiny = resources::ResourceBudget::fixed(32).unwrap();
        let mut refused = PendingUses::new(&tiny);
        assert!(matches!(
            refused.push_back(0usize),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(refused.values.capacity(), 0);
        assert_eq!(tiny.reserved(), 0);
    }
}
