//! Receipt-bound finite native inputs sharing the service's original canonical guard.
use super::{Error, GenerationGuard, GenerationService, RequestExecution};
use lctx_model::domain::{
    attribution::CoverageStatus,
    derivation::RowRef,
    native_requests::{self, NativeInventory},
    resources::{Reservation, ResourceBudget},
    serving::{self, InspectValuePathsRequest, InspectValuePathsResponse},
    *,
};
use std::{collections::BTreeSet, sync::Arc};

struct State {
    guard: GenerationGuard,
    semantics: native_requests::PreparedNativeSemantics,
    actual: BTreeSet<RowRef>,
    _charge: Box<dyn Reservation>,
}
#[derive(Clone)]
pub struct PreparedNative {
    state: Arc<State>,
}

/// Retains packet allocations until the adapter has encoded the typed result.
struct NativeResponse {
    response: InspectValuePathsResponse,
    _charge: Box<dyn Reservation>,
}

impl GenerationService {
    /// Hydration and admission retain the original guard; semantic work holds no lease mutex.
    pub async fn prepare_native(&self) -> Result<PreparedNative, Error> {
        let guard = self.guard();
        guard.check().await?;
        let selection = self.selection();
        let mut locked = guard.state.lease.lock().await;
        let lease = locked.as_mut().ok_or(Error::State)?;
        let budget = lease.budget.clone();
        let binding =
            serving::mappings::prepared_binding(serving::mappings::PreparedDependency::Native);
        let mut seen = BTreeSet::new();
        for invariant in native_requests::preparation_invariants() {
            if !seen.insert(invariant.name) {
                continue;
            }
            let mut check = (invariant.create)(&budget);
            for input in &invariant.inputs {
                if !binding.permits_relation(input.name()) {
                    return Err(Error::Contract);
                }
                lease
                    .selection_read(input, |batch| {
                        check.visit_input(input, &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            check.finish()?;
        }
        let mut inputs = native_requests::PreparationInputs::new(&budget);
        for input in NativeInventory::inputs() {
            if !binding.permits_relation(input.name()) {
                return Err(Error::Contract);
            }
            lease
                .selection_read(&input, |batch| {
                    inputs.native.visit(input.name(), &batch)?;
                    Ok(())
                })
                .await?;
        }
        for input in native_requests::PreparationRows::inputs() {
            if !binding.permits_relation(input.name()) {
                return Err(Error::Contract);
            }
            lease
                .selection_read(&input, |batch| {
                    inputs.rows.visit(input.name(), &batch)?;
                    Ok(())
                })
                .await?;
        }
        if inputs.needs_binding() {
            use lctx_model::domain::normalized::binding_normalization::{
                BindingData, BindingOutput,
            };
            let mut data = BindingData::new(&budget);
            let mut output = BindingOutput::new(&budget);
            for input in BindingData::validation_inputs() {
                if !binding.permits_relation(input.name()) {
                    return Err(Error::Contract);
                }
                lease
                    .selection_read(&input, |batch| {
                        data.visit(input.name(), &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            for input in BindingOutput::validation_inputs() {
                if !binding.permits_relation(input.name()) {
                    return Err(Error::Contract);
                }
                lease
                    .selection_read(&input, |batch| {
                        output.visit(input.name(), &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            inputs.binding = Some((data, output));
        }
        let charge = budget.reserve(
            "native-actual-membership",
            inputs
                .actual_row_bound()
                .saturating_mul(128)
                .saturating_add(size_of::<State>()),
        )?;
        let actual = inputs.actual_rows();
        drop(locked);
        let semantics = native_requests::PreparedNativeSemantics::prepare(
            inputs,
            selection.prepared().data(),
            &budget,
        )?;
        guard.check().await?;
        Ok(PreparedNative {
            state: Arc::new(State {
                guard,
                semantics,
                actual,
                _charge: charge,
            }),
        })
    }
}

impl PreparedNative {
    pub fn generation(&self) -> super::GenerationId {
        self.state.guard.generation()
    }
    pub async fn inspect(
        &self,
        execution: &RequestExecution,
        request: InspectValuePathsRequest,
    ) -> Result<InspectValuePathsResponse, Error> {
        if !execution.shares_guard(&self.state.guard) {
            return Err(Error::Contract);
        }
        let retained = self.clone();
        let grant = execution.clone();
        execution
            .cpu(move |budget| {
                let result = retained.inspect_pure(request, budget)?;
                let bytes = super::catalog_service::serialized_len(&result.response)?;
                grant.retain(
                    "native-owned-response",
                    bytes.saturating_mul(3).saturating_add(8192),
                )?;
                Ok(result.response)
            })
            .await
    }
    fn inspect_pure(
        &self,
        mut request: InspectValuePathsRequest,
        budget: &ResourceBudget,
    ) -> Result<NativeResponse, Error> {
        let input_bytes = request.inputs.iter().try_fold(
            size_of::<InspectValuePathsRequest>(),
            |sum, input| {
                let bytes = match &input.value {
                    native_requests::ExactScalar::String { value } => value.len(),
                    native_requests::ExactScalar::Integer { decimal } => decimal.len(),
                    native_requests::ExactScalar::None {}
                    | native_requests::ExactScalar::Bool { .. } => 0,
                };
                sum.checked_add(bytes.saturating_mul(4))
                    .and_then(|n| n.checked_add(256))
                    .ok_or(Error::ResourceRefused("native request inputs"))
            },
        )?;
        let _input_charge =
            budget.reserve("native-request-inputs", input_bytes.saturating_add(8192))?;
        let domain = self.state.semantics.domain();
        if !domain.contains_member(request.member) {
            return Err(Error::Absent);
        }
        let owner = domain.resolve(&mut request).map_err(|_| Error::Contract)?;
        let binding = serving::CursorBinding {
            generation: serving::GenerationKey(*self.generation().bytes()),
            request: serving::Request::InspectValuePaths(request.clone())
                .canonical_identity()
                .map_err(|e| Error::Codec(e.to_string()))?,
            policy: serving::identity::PolicyIdentity(native_requests::definition()),
            wire: serving::wire_identity(),
            channels: serving::ChannelState {
                lexical: false,
                vector: serving::VectorChannel::Disabled {},
            }
            .identity(),
            group: serving::Name::new("native").map_err(|e| Error::Codec(e.to_string()))?,
            section: serving::Name::new("paths").map_err(|e| Error::Codec(e.to_string()))?,
            member: Some(request.member),
            ordering: ContentHash::of(b"formal,path-relation,path-id/v1"),
        };
        let offset = request
            .page
            .cursor
            .0
            .as_ref()
            .map(|token| serving::Cursor::decode(token, &binding))
            .transpose()
            .map_err(|e| Error::Codec(e.to_string()))?
            .map_or(0, |c| c.offset as usize);
        let charge = budget.reserve(
            "native-response",
            request.page.size as usize * 512 * 1024 + 8192,
        )?;
        let mut packets = Vec::new();
        let mut total = 0usize;
        let mut unavailable = false;
        for input in &request.inputs {
            let Some(path_count) =
                self.state
                    .semantics
                    .path_count(owner, input.formal, request.analysis)
            else {
                unavailable = true;
                continue;
            };
            unavailable |= self
                .state
                .semantics
                .unexamined(owner, input.formal, request.analysis);
            for path_index in 0..path_count {
                let index = total;
                total += 1;
                if index < offset || packets.len() >= request.page.size as usize {
                    continue;
                }
                let exact = native_requests::ExactRequest {
                    generation: binding.generation,
                    owner,
                    formal: input.formal,
                    value: &input.value,
                    assumptions: request.assumptions,
                };
                let assessment = self.state.semantics.assess_path(
                    &exact,
                    request.analysis,
                    path_index,
                    budget,
                )?;
                if !self.state.actual.contains(&assessment.path)
                    || !self
                        .state
                        .actual
                        .contains(&RowRef::of(assessment.original_condition))
                    || assessment
                        .proof
                        .iter()
                        .any(|row| !self.state.actual.contains(row))
                {
                    return Err(Error::Contract);
                }
                packets.push(serving::NativeAssessmentPacket::from_canonical(&assessment));
            }
        }
        if offset > total {
            return Err(Error::Contract);
        }
        let end = offset + packets.len();
        let truncated = end < total;
        let continuation = if truncated {
            serving::Optional::supplied(
                serving::Cursor {
                    binding,
                    offset: end as u64,
                }
                .encode()
                .map_err(|e| Error::Codec(e.to_string()))?,
            )
        } else {
            serving::Optional::default()
        };
        let not_requested =
            self.state.semantics.coverage(request.analysis) == Some(CoverageStatus::NotRequested);
        let availability = if not_requested {
            serving::Availability::NotRequested {}
        } else if unavailable {
            serving::Availability::Partial {
                reason: serving::Name::new(native_requests::UNAVAILABLE_REASON)
                    .map_err(|e| Error::Codec(e.to_string()))?,
            }
        } else {
            serving::Availability::Available {}
        };
        Ok(NativeResponse {
            response: InspectValuePathsResponse {
                generation: serving::GenerationKey(*self.generation().bytes()),
                member: request.member,
                paths: serving::SectionPage {
                    availability,
                    items: packets,
                    continuation,
                    omitted: total.saturating_sub(end) as u64,
                    truncated,
                },
            },
            _charge: charge,
        })
    }
}
